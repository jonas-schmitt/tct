"""Fail-closed and isolation checks for the public-source scanner wrapper."""

import base64
import hashlib
import importlib.util
import json
import os
import subprocess
import tempfile
import unittest
from pathlib import Path
from unittest import mock


SCRIPT = Path(__file__).resolve().parents[2] / "scripts/release/scan-public-source.py"
SPEC = importlib.util.spec_from_file_location("public_source_scan", SCRIPT)
scan = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(scan)


class PublicSourceScanTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.repo = self.root / "repo"
        self.repo.mkdir()
        self.git("init", "-q")
        self.git("config", "user.name", "Scanner test")
        self.git("config", "user.email", "scanner@example.invalid")

    def git(self, *args):
        return subprocess.check_output(
            ["git", "-C", str(self.repo), *args], stderr=subprocess.PIPE
        )

    def commit(self):
        self.git("add", ".")
        self.git("commit", "-qm", "fixture")

    def test_source_must_be_repository_root_not_parent_discovery(self):
        (self.repo / "file").write_text("fixture")
        self.commit()
        child = self.repo / "nested"
        child.mkdir()
        with self.assertRaises(scan.ScanError):
            scan.repository_state(child)

    def test_snapshot_does_not_follow_symlink_or_export_ignore(self):
        outside = self.root / "outside"
        outside.write_text("outside marker")
        (self.repo / "link").symlink_to(outside)
        (self.repo / "hidden.txt").write_text("retained content")
        (self.repo / ".gitattributes").write_text("hidden.txt export-ignore\n")
        self.commit()
        tree, entries = scan.snapshot(self.repo, "HEAD", self.root / "snapshot")
        self.assertTrue(tree)
        self.assertEqual(len(entries), 3)
        self.assertFalse((self.root / "snapshot/link").is_symlink())
        self.assertEqual((self.root / "snapshot/link").read_text(), str(outside))
        self.assertEqual((self.root / "snapshot/hidden.txt").read_text(), "retained content")

    def test_external_git_object_store_is_rejected(self):
        (self.repo / "file").write_text("fixture")
        self.commit()
        alternates = self.repo / ".git/objects/info/alternates"
        alternates.write_text(str(self.root / "private-archive-objects") + "\n")
        with self.assertRaises(scan.ScanError):
            scan.repository_state(self.repo)

    def test_output_cannot_overwrite_source_or_git_metadata(self):
        for path in (self.repo / "README.md", self.repo / ".git/config"):
            with self.assertRaises(scan.ScanError):
                scan.validate_receipt_path(self.repo, path)

    def test_failed_scan_replaces_stale_success_receipt(self):
        receipt = self.root / "receipt.json"
        receipt.write_text('{"status":"no-unreviewed-findings"}')
        result = subprocess.run(
            ["python3", str(SCRIPT), "--source", str(self.repo), "--tool-dir", str(self.root / "missing-tools"), "--receipt", str(receipt)],
            capture_output=True, text=True,
        )
        self.assertEqual(result.returncode, 2)
        self.assertEqual(json.loads(receipt.read_text())["status"], "incomplete")

    def test_environment_cannot_replace_git_or_scanner_configuration(self):
        with mock.patch.dict(os.environ, {"GIT_DIR": "/private/archive", "GITLEAKS_CONFIG": "allow-everything", "GOMAXPROCS": "99"}):
            env = scan.clean_environment()
        self.assertNotIn("GIT_DIR", env)
        self.assertNotIn("GITLEAKS_CONFIG", env)
        self.assertEqual(env["GOMAXPROCS"], "2")

    def test_repository_fsmonitor_hook_cannot_execute(self):
        (self.repo / "file").write_text("fixture")
        self.commit()
        marker = self.root / "hook-executed"
        hook = self.repo / ".git/fsmonitor-test"
        hook.write_text(f"#!/usr/bin/python3\nfrom pathlib import Path\nPath({str(marker)!r}).write_text('executed')\nprint('0')\n")
        hook.chmod(0o755)
        self.git("config", "core.fsmonitor", str(hook))
        scan.repository_state(self.repo)
        self.assertFalse(marker.exists(), "repository fsmonitor hook executed")

    def test_status_cannot_execute_repository_clean_filter(self):
        (self.repo / "file").write_text("fixture")
        (self.repo / ".gitattributes").write_text("file filter=probe\n")
        self.commit()
        marker = self.root / "filter-executed"
        hook = self.repo / ".git/filter-test"
        hook.write_text(f"#!/usr/bin/python3\nimport sys\nfrom pathlib import Path\nPath({str(marker)!r}).write_text('executed')\nsys.stdout.buffer.write(sys.stdin.buffer.read())\n")
        hook.chmod(0o755)
        self.git("config", "filter.probe.clean", str(hook))
        (self.repo / "file").write_text("changed")
        scan.repository_state(self.repo)
        self.assertFalse(marker.exists(), "repository clean filter executed")

    def test_invalid_report_and_process_failure_never_pass(self):
        report = self.root / "report.json"
        for value, status in [("[]", 2), ("not json", 0), ("{}", 0), ("[]", 23), ('[{"Secret":"do not print"}]', 0)]:
            with self.subTest(status=status, value=value[:2]):
                report.write_text(value)
                with self.assertRaises(scan.ScanError) as caught:
                    scan.read_report(report, status)
                self.assertNotIn("do not print", str(caught.exception))

    def test_receipt_drops_secret_match_and_identity_fields(self):
        raw = {"RuleID": "generic-api-key", "File": "example.json", "StartLine": 2, "EndLine": 2, "StartColumn": 4, "EndColumn": 30, "Commit": "a" * 40, "Secret": "must stay private", "Match": "must stay private", "Message": "must stay private", "Author": "must stay private", "Email": "must stay private"}
        entry = {"sha256": "b" * 64, "blob": "c" * 40}
        safe = scan.redact_finding(raw, "history", entry)
        self.assertNotIn("must stay private", json.dumps(safe))
        self.assertEqual(safe["path"], "example.json")
        self.assertEqual(safe["blob_sha256"], "b" * 64)
        self.assertRegex(safe["finding_sha256"], "^[0-9a-f]{64}$")

    def test_review_requires_exact_contents_location_and_reason(self):
        finding = {"rule": "generic-api-key", "path": "fixture.json", "line": 3, "end_line": 3, "start_column": 2, "end_column": 15, "blob_sha256": "a" * 64}
        accepted = {**finding, "reason": "nonsecret_digest", "explanation": "Verified digest of fixture tokens"}
        path = self.root / "review.json"
        path.write_text(json.dumps({"version": 1, "reviews": [accepted]}))
        reviews = scan.load_reviews(path)
        self.assertIsNotNone(scan.review_for(finding, reviews))
        for key, value in [("line", 4), ("blob_sha256", "b" * 64), ("rule", "private-key")]:
            self.assertIsNone(scan.review_for({**finding, key: value}, reviews))
        path.write_text(json.dumps({"version": 1, "reviews": [{"path": "*", "reason": "nonsecret_digest"}]}))
        with self.assertRaises(scan.ScanError):
            scan.load_reviews(path)

    def test_kubernetes_payloads_decode_and_invalid_base64_blocks(self):
        payload = base64.b64encode(b"test-only payload").decode()
        data = json.dumps({"kind": "Secret", "data": {"credential": payload}}).encode()
        records = scan.kubernetes_payloads(data)
        self.assertEqual(records, [("/data/credential", b"test-only payload")])
        malformed = json.dumps({"kind": "Secret", "data": {"credential": "not base64!"}}).encode()
        with self.assertRaises(scan.ScanError):
            scan.kubernetes_payloads(malformed)

    def test_modified_scanner_binary_is_rejected_before_execution(self):
        directory = self.root / "tools"
        directory.mkdir()
        (directory / "gitleaks").write_text("not the official scanner")
        name, _ = scan.release_archive()
        (directory / name).write_bytes(b"unverified archive")
        with self.assertRaisesRegex(scan.ScanError, "scanner-checksum-mismatch"):
            scan.verified_scanner(directory)

    @unittest.skipUnless(os.environ.get("TCT_GITLEAKS_TOOL_DIR"), "set TCT_GITLEAKS_TOOL_DIR for pinned-scanner integration")
    def test_real_scanner_catches_deleted_encoded_and_metadata_secrets(self):
        # These deterministic, fabricated detector probes never authenticate.
        token = "ghp" + "_" + hashlib.sha256(b"synthetic-scanner-probe").hexdigest()[:36]
        encoded = base64.b64encode(token.encode()).decode()
        (self.repo / "removed.txt").write_text(token + " # gitleaks:allow\n")
        (self.repo / ".gitleaks.toml").write_text('[allowlist]\nregexes = [".*"]\n')
        self.commit()
        self.git("rm", "-q", "removed.txt")
        (self.repo / "secret.json").write_text(json.dumps({"kind": "Secret", "data": {"token": encoded}}))
        self.commit()
        self.git("tag", "-a", "metadata-probe", "-m", token)
        receipt = self.root / "result.json"
        result = scan.scan_repository(self.repo, Path(os.environ["TCT_GITLEAKS_TOOL_DIR"]), receipt)
        self.assertEqual(result["status"], "needs-review")
        scopes = {finding["scope"] for finding in result["findings"] if finding["rule"] == "github-pat"}
        self.assertTrue({"history", "tip", "history-blobs", "history-metadata", "kubernetes-decoded"} <= scopes)
        serialized = receipt.read_text()
        self.assertNotIn(token, serialized)
        self.assertNotIn(encoded, serialized)
        self.assertEqual(receipt.stat().st_mode & 0o777, 0o600)


if __name__ == "__main__":
    unittest.main()
