#!/usr/bin/env python3
"""Scan the exact committed source and reachable history with pinned Gitleaks.

Install: python3 scripts/release/scan-public-source.py --install
Scan:    python3 scripts/release/scan-public-source.py --source . --receipt target/secret-scan.json

Exit codes: 0 = no unreviewed findings, 1 = findings need review, 2 = incomplete.
Only committed contents are covered. Dirty/untracked work is reported separately
and requires another scan after committing. No credentials are tested online.
The receipt retains every finding, including precisely reviewed false positives;
it is scanner evidence, not a guarantee of absence or permission to publish.

An optional --review JSON has version=1 and reviews=[...]. Each review requires
rule, path, line, end_line, start_column, end_column, blob_sha256, reason, and
explanation. Reasons are nonsecret_digest, nonsecret_reference, nonsecret_hostname.
Reviews bind complete blob contents and exact finding locations. No wildcard,
credential, blanket fixture, baseline, or scanner-rule exclusions are supported.
Gitleaks' upstream built-in rules and built-in exclusions remain in effect.
"""

from __future__ import annotations

import argparse
import base64
import binascii
import hashlib
import io
import json
import os
from pathlib import Path, PurePosixPath
import platform
import re
import subprocess
import tarfile
import tempfile
import urllib.request


VERSION = "8.30.1"
RELEASE = f"https://github.com/gitleaks/gitleaks/releases/download/v{VERSION}/"
ARCHIVES = {
    "aarch64": ("linux_arm64", "e4a487ee7ccd7d3a7f7ec08657610aa3606637dab924210b3aee62570fb4b080"),
    "x86_64": ("linux_x64", "551f6fc83ea457d62a0d98237cbad105af8d557003051f41f3e7ca7b3f2470eb"),
}
ROOT = Path(__file__).resolve().parents[2]
REVIEW_FIELDS = ("rule", "path", "line", "end_line", "start_column", "end_column", "blob_sha256")
REASONS = {"nonsecret_digest", "nonsecret_reference", "nonsecret_hostname"}


class ScanError(Exception):
    """Only fixed, non-sensitive diagnostics belong in this exception."""


def sha256(data):
    return hashlib.sha256(data).hexdigest()


def clean_environment():
    env = {k: v for k, v in os.environ.items() if not k.startswith(("GIT_", "GITLEAKS_"))}
    env.update(GOMAXPROCS="2", GIT_CONFIG_NOSYSTEM="1", GIT_CONFIG_GLOBAL=os.devnull, GIT_TERMINAL_PROMPT="0", GIT_NO_REPLACE_OBJECTS="1", GIT_OPTIONAL_LOCKS="0", GIT_NO_LAZY_FETCH="1")
    # Also protect Git subprocesses launched inside the verified scanner.
    settings = {"core.fsmonitor": "false", "core.hooksPath": os.devnull, "log.showSignature": "false"}
    env["GIT_CONFIG_COUNT"] = str(len(settings))
    for index, (key, value) in enumerate(settings.items()):
        env[f"GIT_CONFIG_KEY_{index}"] = key
        env[f"GIT_CONFIG_VALUE_{index}"] = value
    return env


def git(source, *args):
    env = clean_environment()
    command = ["git", "-c", "core.fsmonitor=false", "-c", f"core.hooksPath={os.devnull}", "-c", "log.showSignature=false", "-C", str(source)]
    if args and args[0] == "status":
        # Status can run clean/process filters when checking same-size changes.
        # Reading config does not execute them; override every configured driver.
        filters = subprocess.run([*command, "config", "--includes", "--null", "--name-only", "--get-regexp", r"^filter\..*\.(clean|smudge|process|required)$"], env=env, capture_output=True)
        if filters.returncode not in (0, 1):
            raise ScanError("git-filter-config-read-failed")
        for raw in sorted(set(filters.stdout.split(b"\0")) - {b""}):
            key = raw.decode("utf-8")
            command += ["-c", key + ("=false" if key.endswith(".required") else "=")]
    result = subprocess.run([*command, *args], env=env, capture_output=True)
    if result.returncode:
        raise ScanError("git-read-failed")
    return result.stdout


def release_archive():
    if platform.system() != "Linux" or platform.machine() not in ARCHIVES:
        raise ScanError("unsupported-scanner-platform")
    suffix, expected = ARCHIVES[platform.machine()]
    return f"gitleaks_{VERSION}_{suffix}.tar.gz", expected


def archive_binary(data):
    try:
        with tarfile.open(fileobj=io.BytesIO(data), mode="r:gz") as archive:
            entry = archive.getmember("gitleaks")
            if not entry.isfile():
                raise ScanError("invalid-scanner-archive")
            return archive.extractfile(entry).read()
    except (KeyError, tarfile.TarError):
        raise ScanError("invalid-scanner-archive") from None


def install_scanner(directory):
    """Fetch only official HTTPS release assets and verify the pinned checksum."""
    name, expected = release_archive()
    manifest = urllib.request.urlopen(RELEASE + f"gitleaks_{VERSION}_checksums.txt", timeout=60).read().decode("ascii")
    entries = {parts[1]: parts[0] for line in manifest.splitlines() if len(parts := line.split()) == 2}
    if entries.get(name) != expected:
        raise ScanError("official-checksum-disagrees-with-pin")
    data = urllib.request.urlopen(RELEASE + name, timeout=60).read()
    if sha256(data) != expected:
        raise ScanError("scanner-archive-checksum-mismatch")
    directory.mkdir(parents=True, exist_ok=True)
    if any((directory / filename).is_symlink() for filename in (name, "gitleaks")):
        raise ScanError("scanner-install-symlink")
    (directory / name).write_bytes(data)
    binary = directory / "gitleaks"
    binary.write_bytes(archive_binary(data))
    binary.chmod(0o700)
    return verified_scanner(directory)


def verified_scanner(directory):
    name, expected = release_archive()
    archive = directory / name
    binary = directory / "gitleaks"
    if not archive.is_file() or not binary.is_file() or archive.is_symlink() or binary.is_symlink():
        raise ScanError("verified-scanner-missing-run-install")
    data = archive.read_bytes()
    if sha256(data) != expected or binary.read_bytes() != archive_binary(data):
        raise ScanError("scanner-checksum-mismatch")
    result = subprocess.run([str(binary), "version"], capture_output=True, env=clean_environment())
    if result.returncode or result.stdout.decode().strip() != VERSION:
        raise ScanError("scanner-version-mismatch")
    return binary.resolve(), {"name": "gitleaks", "version": VERSION, "release": RELEASE, "archive_sha256": expected, "binary_sha256": sha256(binary.read_bytes())}


def repository_state(source):
    source = source.resolve()
    # Explicit local root prevents Git searching upward into a private archive.
    git_dir = source / ".git"
    if not git_dir.is_dir() or git_dir.is_symlink():
        raise ScanError("source-needs-independent-git-directory")
    if any(path.is_symlink() for path in git_dir.rglob("*")):
        raise ScanError("external-git-metadata-symlink")
    if Path(os.fsdecode(git(source, "rev-parse", "--show-toplevel")).strip()).resolve() != source:
        raise ScanError("source-is-not-repository-root")
    if Path(os.fsdecode(git(source, "rev-parse", "--absolute-git-dir")).strip()).resolve() != git_dir:
        raise ScanError("external-git-directory")
    for path in ("objects/info/alternates", "objects/info/http-alternates", "info/grafts", "shallow", "commondir"):
        if (git_dir / path).exists():
            raise ScanError("external-or-incomplete-git-history")
    if git(source, "for-each-ref", "refs/replace").strip():
        raise ScanError("replacement-history-not-supported")
    refs = os.fsdecode(git(source, "for-each-ref", "--format=%(refname) %(objectname)")).splitlines()
    commits = git(source, "rev-list", "--all", "HEAD").decode().splitlines()
    return {
        "head": git(source, "rev-parse", "HEAD").decode().strip(),
        "refs": refs,
        "commits": sorted(commits),
        "working_tree_dirty": bool(git(source, "status", "--porcelain", "--untracked-files=normal")),
    }


def tree_entries(source, revision):
    entries = []
    for record in git(source, "ls-tree", "-rz", revision).split(b"\0"):
        if not record:
            continue
        metadata, raw_path = record.split(b"\t", 1)
        mode, kind, oid = metadata.decode().split()
        path = os.fsdecode(raw_path)
        parts = PurePosixPath(path).parts
        if not parts or PurePosixPath(path).is_absolute() or any(p in ("..", ".git") for p in parts):
            raise ScanError("unsafe-tracked-path")
        if kind != "blob" or mode not in ("100644", "100755", "120000"):
            raise ScanError("submodules-or-special-entries-not-covered")
        entries.append({"path": path, "blob": oid, "mode": mode})
    return entries


def snapshot(source, revision, destination):
    """Read Git blobs directly: no export-ignore, filters, or followed symlinks."""
    destination.mkdir(parents=True, exist_ok=False)
    entries = tree_entries(source, revision)
    for entry in entries:
        data = git(source, "cat-file", "blob", entry["blob"])
        path = destination / entry["path"]
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(data)
        entry.update(sha256=sha256(data), bytes=len(data))
    return git(source, "rev-parse", f"{revision}^{{tree}}").decode().strip(), entries


def kubernetes_payloads(data):
    """Supplement Gitleaks decoding with explicit JSON Secret.data decoding."""
    try:
        value = json.loads(data)
    except (ValueError, UnicodeError):
        return []
    result = []

    def walk(node, pointer):
        if isinstance(node, dict):
            if node.get("kind") == "Secret":
                for field in ("data", "stringData"):
                    values = node.get(field, {})
                    if not isinstance(values, dict):
                        raise ScanError("malformed-kubernetes-secret-payload")
                    for key, value in values.items():
                        if not isinstance(value, str):
                            raise ScanError("malformed-kubernetes-secret-payload")
                        try:
                            decoded = base64.b64decode(value, validate=True) if field == "data" else value.encode()
                        except (ValueError, binascii.Error):
                            raise ScanError("invalid-kubernetes-secret-base64") from None
                        result.append((pointer + "/" + field + "/" + key.replace("~", "~0").replace("/", "~1"), decoded))
            for key, child in node.items():
                walk(child, pointer + "/" + key.replace("~", "~0").replace("/", "~1"))
        elif isinstance(node, list):
            for i, child in enumerate(node):
                walk(child, pointer + "/" + str(i))

    walk(value, "")
    return result


def read_report(path, returncode):
    if returncode not in (0, 23):
        raise ScanError("scanner-process-failed")
    try:
        findings = json.loads(path.read_text())
    except (OSError, ValueError):
        raise ScanError("scanner-report-missing-or-invalid") from None
    if not isinstance(findings, list) or bool(findings) != (returncode == 23):
        raise ScanError("scanner-report-exit-mismatch")
    for finding in findings:
        if not isinstance(finding, dict) or not isinstance(finding.get("RuleID"), str) or not isinstance(finding.get("File"), str):
            raise ScanError("invalid-scanner-finding")
        for field in ("StartLine", "EndLine", "StartColumn", "EndColumn"):
            if type(finding.get(field)) is not int or finding[field] < 0:
                raise ScanError("invalid-scanner-location")
    return findings


def redact_finding(raw, scope, entry):
    result = {
        "scope": scope, "rule": raw["RuleID"], "path": raw["File"],
        "line": raw["StartLine"], "end_line": raw["EndLine"],
        "start_column": raw["StartColumn"], "end_column": raw["EndColumn"],
        "commit": raw.get("Commit", ""), "blob_sha256": entry["sha256"],
        "blob_oid": entry["blob"],
    }
    result["finding_sha256"] = sha256(json.dumps(result, sort_keys=True).encode())
    return result


def review_key(finding):
    return tuple(finding[field] for field in REVIEW_FIELDS)


def load_reviews(path):
    if path is None:
        return {}
    try:
        value = json.loads(path.read_text())
        if set(value) != {"version", "reviews"} or value["version"] != 1 or not isinstance(value["reviews"], list):
            raise ValueError
        reviews = {}
        for item in value["reviews"]:
            if set(item) != set(REVIEW_FIELDS) | {"reason", "explanation"}:
                raise ValueError
            if item["reason"] not in REASONS or not isinstance(item["explanation"], str) or not item["explanation"].strip():
                raise ValueError
            if not re.fullmatch(r"[0-9a-f]{64}", item["blob_sha256"]):
                raise ValueError
            if any(type(item[k]) is not int or item[k] < 0 for k in REVIEW_FIELDS[2:6]):
                raise ValueError
            if not isinstance(item["rule"], str) or not isinstance(item["path"], str) or any(c in item["path"] for c in "*?[]"):
                raise ValueError
            key = review_key(item)
            if key in reviews:
                raise ValueError
            reviews[key] = item
        return reviews
    except (OSError, ValueError, KeyError, TypeError):
        raise ScanError("invalid-exact-review-manifest") from None


def review_for(finding, reviews):
    return reviews.get(review_key(finding))


def run_scanner(binary, command, work, label):
    report = work / f"{label}.json"
    args = [str(binary), *command, "--config", str(work / "rules.toml"), "--gitleaks-ignore-path", str(work / "empty-ignore"), "--ignore-gitleaks-allow", "--redact=100", "--max-decode-depth=12", "--max-archive-depth=8", "--max-target-megabytes=0", "--no-banner", "--no-color", "--report-format=json", "--report-path", str(report), "--exit-code=23"]
    # Never relay scanner stdout/stderr: matched context may contain other secrets.
    result = subprocess.run(args, cwd=work, env=clean_environment(), capture_output=True)
    if re.search(rb"\b(?:ERR|FTL)\b", result.stderr):
        raise ScanError("scanner-reported-an-error")
    return read_report(report, result.returncode)


def validate_receipt_path(source, receipt):
    source, receipt = source.resolve(), receipt.resolve()
    if receipt.is_relative_to(source):
        relative = receipt.relative_to(source)
        if relative.parts[0] != "target":
            raise ScanError("receipt-must-be-outside-source-or-under-target")
        tracked = {os.fsdecode(path) for path in git(source, "ls-files", "-z").split(b"\0") if path}
        if str(relative) in tracked:
            raise ScanError("receipt-would-overwrite-tracked-source")
    return receipt


def write_receipt(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.NamedTemporaryFile(mode="w", dir=path.parent, prefix=".scan-receipt-", delete=False) as stream:
        temporary = Path(stream.name)
        json.dump(value, stream, indent=2, sort_keys=True)
        stream.write("\n")
    temporary.chmod(0o600)
    temporary.replace(path)


def scan_repository(source, directory, receipt, review_path=None):
    source, directory, receipt = source.resolve(), directory.resolve(), receipt.resolve()
    validate_receipt_path(source, receipt)
    binary, scanner = verified_scanner(directory)
    before = repository_state(source)
    reviews = load_reviews(review_path)
    receipt.parent.mkdir(parents=True, exist_ok=True)
    # Temporary contents and scanner reports are private and removed on completion.
    with tempfile.TemporaryDirectory(prefix="tct-secret-scan-", dir=receipt.parent) as temporary:
        work = Path(temporary)
        (work / "rules.toml").write_text("[extend]\nuseDefault = true\n")
        (work / "empty-ignore").write_text("")
        tree, tip = snapshot(source, before["head"], work / "tip")
        tip_entries = {entry["path"]: entry for entry in tip}
        history_entries = {}
        blob_entries = {}
        decoded_entries = {}
        metadata_entries = {}
        for dirname in ("blobs", "decoded", "metadata"):
            (work / dirname).mkdir()
        for commit in before["commits"]:
            history_entries[commit] = {}
            for entry in tree_entries(source, commit):
                oid, path = entry["blob"], entry["path"]
                alias = oid + "/" + path
                if alias not in blob_entries:
                    data = git(source, "cat-file", "blob", oid)
                    entry.update(sha256=sha256(data), bytes=len(data))
                    destination = work / "blobs" / alias
                    destination.parent.mkdir(parents=True, exist_ok=True)
                    destination.write_bytes(data)
                    blob_entries[alias] = entry
                    if path.endswith(".json"):
                        for pointer, payload in kubernetes_payloads(data):
                            name = sha256((alias + pointer).encode()) + ".txt"
                            (work / "decoded" / name).write_bytes(payload)
                            decoded_entries[name] = {**entry, "json_pointer": pointer}
                history_entries[commit][path] = blob_entries[alias]
            # git-mode scans diffs; scan commit objects separately for messages.
            data = git(source, "cat-file", "commit", commit)
            (work / "metadata" / commit).write_bytes(data)
            metadata_entries[commit] = {"path": "git-commit/" + commit, "blob": commit, "sha256": sha256(data)}
        for ref in before["refs"]:
            oid = ref.rsplit(" ", 1)[-1]
            if git(source, "cat-file", "-t", oid).strip() == b"tag":
                data = git(source, "cat-file", "tag", oid)
                (work / "metadata" / oid).write_bytes(data)
                metadata_entries[oid] = {"path": "git-tag/" + oid, "blob": oid, "sha256": sha256(data)}
        findings = []
        commands = [
            ("history", ["git", str(source), "--log-opts=--all HEAD --full-history -m --no-ext-diff --no-textconv"]),
            ("tip", ["dir", str(work / "tip")]),
            ("history-blobs", ["dir", str(work / "blobs")]),
            ("history-metadata", ["dir", str(work / "metadata")]),
        ]
        if decoded_entries:
            commands.append(("kubernetes-decoded", ["dir", str(work / "decoded")]))
        for scope, command in commands:
            for raw in run_scanner(binary, command, work, scope):
                if scope == "history":
                    entry = history_entries.get(raw.get("Commit"), {}).get(raw["File"])
                else:
                    prefix = {"tip": "tip", "history-blobs": "blobs", "kubernetes-decoded": "decoded", "history-metadata": "metadata"}[scope]
                    file = raw["File"]
                    if file.startswith(str(work / prefix) + "/"):
                        file = file[len(str(work / prefix)) + 1:]
                    mapping = {"tip": tip_entries, "history-blobs": blob_entries, "kubernetes-decoded": decoded_entries, "history-metadata": metadata_entries}[scope]
                    entry = mapping.get(file)
                if entry is None:
                    raise ScanError("scanner-finding-outside-bound-snapshot")
                raw["File"] = entry["path"]
                safe = redact_finding(raw, scope, entry)
                if scope == "kubernetes-decoded":
                    safe["json_pointer"] = entry["json_pointer"]
                    # Decoded locations are not source locations; no review matching.
                    review = None
                elif scope == "history-metadata":
                    review = None
                else:
                    review = review_for(safe, reviews)
                safe["reviewed"] = review is not None
                if review:
                    safe["review_reason"] = review["reason"]
                    # Free-form review prose is private; bind it without echoing it.
                    safe["review_explanation_sha256"] = sha256(review["explanation"].encode())
                findings.append(safe)
        after = repository_state(source)
        if before != after:
            raise ScanError("source-state-changed-during-scan")
        unresolved = sum(not finding["reviewed"] for finding in findings)
        result = {
            "version": 1, "status": "needs-review" if unresolved else "no-unreviewed-findings",
            "scanner": scanner, "source": str(source), "source_state": before,
            "head_tree": tree,
            "tip_blob_manifest_sha256": sha256(json.dumps(tip, sort_keys=True).encode()),
            "tip_files": len(tip), "history_path_blob_pairs": len(blob_entries),
            "kubernetes_decoded_payloads": len(decoded_entries),
            "completed_scopes": [scope for scope, _ in commands],
            "max_decode_depth": 12, "max_archive_depth": 8,
            "review_manifest_sha256": sha256(review_path.read_bytes()) if review_path else None,
            "findings": findings, "unreviewed_findings": unresolved,
            "limitations": ["Committed source only; uncommitted work requires a new committed scan.", "Upstream default detection rules and built-in exclusions apply.", "Decoding/archive recursion is bounded; encrypted formats are not covered.", "Explicit Kubernetes supplement covers JSON Secret data/stringData; general recursive decoding also runs.", "Local reachable refs only; hidden remote refs, unreachable objects, hosting caches and external artifacts are outside scope.", "No credential validity, ownership, revocation, or corpus redistribution rights are established."],
        }
        write_receipt(receipt, result)
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--source", type=Path, default=ROOT)
    parser.add_argument("--tool-dir", type=Path, default=ROOT / "target/publication-tools" / f"gitleaks-{VERSION}")
    parser.add_argument("--receipt", type=Path)
    parser.add_argument("--review", type=Path)
    parser.add_argument("--install", action="store_true")
    args = parser.parse_args()
    os.umask(0o077)
    receipt = None
    try:
        if args.install:
            _, metadata = install_scanner(args.tool_dir.resolve())
            print(json.dumps(metadata, sort_keys=True))
            return 0
        if args.receipt is None:
            parser.error("--receipt is required for scanning")
        receipt = validate_receipt_path(args.source, args.receipt)
        result = scan_repository(args.source, args.tool_dir, args.receipt, args.review)
        print(json.dumps({"status": result["status"], "receipt": str(args.receipt), "findings": len(result["findings"]), "unreviewed": result["unreviewed_findings"]}))
        return 1 if result["unreviewed_findings"] else 0
    except Exception as exc:
        # Arbitrary parser/network/subprocess errors can contain source values.
        code = str(exc) if isinstance(exc, ScanError) else type(exc).__name__
        if receipt is not None:
            try:
                write_receipt(receipt, {"version": 1, "status": "incomplete", "error": code})
            except OSError:
                pass
        print(json.dumps({"status": "incomplete", "error": code}))
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
