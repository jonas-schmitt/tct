#!/usr/bin/env python3
"""Tests for retained C# route input preflight."""

import json
import os
import stat
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[2]
SCRIPT = REPO_ROOT / "scripts/analysis/frontend_route_inputs.py"
ROUTE_SOURCE_MANIFEST_SCRIPT = REPO_ROOT / "scripts/analysis/route_source_manifest.py"
ROUTE_PROVENANCE_ARGS = [
    "--corpus-id",
    "frontend-route-test-corpus",
    "--corpus-source",
    "temporary unit-test corpus",
    "--corpus-license",
    "test-only",
    "--selection-note",
    "unit-test route manifest",
]


def write_fake_tool(path: Path, output: str) -> None:
    path.write_text(f"#!/usr/bin/env sh\necho '{output}'\n", encoding="utf-8")
    path.chmod(path.stat().st_mode | stat.S_IXUSR)


class TestFrontendRouteInputs(unittest.TestCase):
    def test_self_test(self):
        completed = subprocess.run(
            [sys.executable, str(SCRIPT), "--self-test"],
            cwd=REPO_ROOT,
            check=True,
            capture_output=True,
            text=True,
        )
        self.assertIn("self-test passed", completed.stdout)

    def test_reports_missing_csharp_inputs_without_failing_by_default(self):
        completed = subprocess.run(
            [
                sys.executable,
                str(SCRIPT),
                "--target",
                "csharp",
                "--tool-bin",
                "/definitely/missing/dotnet",
            ],
            cwd=REPO_ROOT,
            check=True,
            capture_output=True,
            text=True,
        )
        report = json.loads(completed.stdout)

        self.assertEqual(report["tool"], "frontend_route_inputs")
        self.assertEqual(report["version"], 1)
        self.assertFalse(report["inputs_ready"])
        missing = {item["kind"] for item in report["missing_inputs"]}
        self.assertEqual(
            missing,
            {"dotnet_toolchain_available", "pinned_csharp_source_manifest"},
        )
        self.assertFalse(report["toolchain"]["available"])

    def test_require_inputs_ready_fails_with_context(self):
        completed = subprocess.run(
            [
                sys.executable,
                str(SCRIPT),
                "--target",
                "csharp",
                "--tool-bin",
                "/definitely/missing/dotnet",
                "--require-inputs-ready",
            ],
            cwd=REPO_ROOT,
            capture_output=True,
            text=True,
        )

        self.assertEqual(completed.returncode, 1)
        self.assertIn("C# route inputs are incomplete", completed.stderr)
        self.assertIn("dotnet_toolchain_available", completed.stderr)
        self.assertIn("pinned_csharp_source_manifest", completed.stderr)

    def test_reports_ready_csharp_inputs_with_fake_dotnet(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            tmp = Path(tmpdir)
            source = tmp / "Sample.cs"
            second_source = tmp / "Second.cs"
            manifest = tmp / "sources.txt"
            manifest_report = tmp / "sources.manifest.json"
            fake_dotnet = tmp / "dotnet"
            report_path = tmp / "report.json"
            markdown_path = tmp / "report.md"
            source.write_text("class Sample {}\n", encoding="utf-8")
            second_source.write_text("class Second {}\n", encoding="utf-8")
            subprocess.run(
                [
                    sys.executable,
                    str(ROUTE_SOURCE_MANIFEST_SCRIPT),
                    "--target",
                    "csharp",
                    "--source-root",
                    str(tmp),
                    "--output",
                    str(manifest),
                    "--json-output",
                    str(manifest_report),
                    "--min-files",
                    "2",
                    *ROUTE_PROVENANCE_ARGS,
                ],
                cwd=REPO_ROOT,
                check=True,
                capture_output=True,
                text=True,
            )
            write_fake_tool(fake_dotnet, "8.0.100")

            subprocess.run(
                [
                    sys.executable,
                    str(SCRIPT),
                    "--target",
                    "csharp",
                    "--source-manifest",
                    str(manifest),
                    "--source-manifest-report",
                    str(manifest_report),
                    "--source-root",
                    str(tmp),
                    "--tool-bin",
                    str(fake_dotnet),
                    "--json-output",
                    str(report_path),
                    "--markdown-output",
                    str(markdown_path),
                    "--require-inputs-ready",
                ],
                cwd=REPO_ROOT,
                check=True,
                capture_output=True,
                text=True,
            )
            report = json.loads(report_path.read_text())
            markdown = markdown_path.read_text()

        self.assertTrue(report["inputs_ready"])
        self.assertEqual(report["missing_inputs"], [])
        self.assertEqual(report["target"], "csharp")
        self.assertTrue(report["toolchain"]["available"])
        self.assertEqual(report["source_manifest"]["entry_count"], 2)
        self.assertTrue(
            report["source_manifest"]["representative_entry_count_ready"]
        )
        self.assertEqual(
            report["source_manifest"]["corpus_fingerprint"]["fingerprint_kind"],
            "path_sha256_list_v1",
        )
        self.assertTrue(report["source_manifest_report"]["valid"])
        self.assertEqual(
            report["source_manifest"]["corpus_fingerprint"]["entry_count"],
            2,
        )
        self.assertEqual(
            len(report["source_manifest"]["corpus_fingerprint"]["sha256"]),
            64,
        )
        self.assertTrue(report["source_manifest"]["route_decision_marker_present"])
        self.assertEqual(
            report["source_manifest"]["entries"][0]["manifest_path"],
            "Sample.cs",
        )
        self.assertIn("C# Route Input Preflight", markdown)

    def test_route_intended_manifest_requires_manifest_report(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            tmp = Path(tmpdir)
            first = tmp / "Alpha.cs"
            second = tmp / "Beta.cs"
            manifest = tmp / "sources.txt"
            fake_dotnet = tmp / "dotnet"
            first.write_text("class Alpha {}\n", encoding="utf-8")
            second.write_text("class Beta {}\n", encoding="utf-8")
            manifest.write_text(
                "# tct-csharp-route-corpus-v1\nAlpha.cs\nBeta.cs\n",
                encoding="utf-8",
            )
            write_fake_tool(fake_dotnet, "8.0.100")

            completed = subprocess.run(
                [
                    sys.executable,
                    str(SCRIPT),
                    "--target",
                    "csharp",
                    "--source-manifest",
                    str(manifest),
                    "--source-root",
                    str(tmp),
                    "--tool-bin",
                    str(fake_dotnet),
                ],
                cwd=REPO_ROOT,
                check=True,
                capture_output=True,
                text=True,
            )
            report = json.loads(completed.stdout)

        self.assertFalse(report["inputs_ready"])
        self.assertEqual(
            [item["kind"] for item in report["missing_inputs"]],
            ["csharp_source_manifest_report"],
        )
        self.assertFalse(report["source_manifest_report"]["present"])

    def test_manifest_report_requires_explicit_corpus_provenance(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            tmp = Path(tmpdir)
            first = tmp / "Alpha.cs"
            second = tmp / "Beta.cs"
            manifest = tmp / "sources.txt"
            manifest_report = tmp / "sources.manifest.json"
            fake_dotnet = tmp / "dotnet"
            first.write_text("class Alpha {}\n", encoding="utf-8")
            second.write_text("class Beta {}\n", encoding="utf-8")
            write_fake_tool(fake_dotnet, "8.0.100")

            subprocess.run(
                [
                    sys.executable,
                    str(ROUTE_SOURCE_MANIFEST_SCRIPT),
                    "--target",
                    "csharp",
                    "--source-root",
                    str(tmp),
                    "--output",
                    str(manifest),
                    "--json-output",
                    str(manifest_report),
                    "--min-files",
                    "2",
                ],
                cwd=REPO_ROOT,
                check=True,
                capture_output=True,
                text=True,
            )
            completed = subprocess.run(
                [
                    sys.executable,
                    str(SCRIPT),
                    "--target",
                    "csharp",
                    "--source-manifest",
                    str(manifest),
                    "--source-manifest-report",
                    str(manifest_report),
                    "--source-root",
                    str(tmp),
                    "--tool-bin",
                    str(fake_dotnet),
                ],
                cwd=REPO_ROOT,
                check=True,
                capture_output=True,
                text=True,
            )
            report = json.loads(completed.stdout)

        self.assertFalse(report["inputs_ready"])
        self.assertEqual(
            [item["kind"] for item in report["missing_inputs"]],
            ["csharp_source_manifest_report_validity"],
        )
        self.assertTrue(
            any(
                "source manifest report corpus provenance is incomplete" in error
                for error in report["source_manifest_report"]["errors"]
            )
        )

    def test_route_source_manifest_output_feeds_csharp_preflight(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            tmp = Path(tmpdir)
            generated = tmp / "generated"
            generated.mkdir()
            (tmp / "Alpha.cs").write_text(
                "class Alpha {}\n",
                encoding="utf-8",
            )
            (tmp / "Beta.cs").write_text(
                "class Beta {}\n",
                encoding="utf-8",
            )
            (generated / "Ignored.cs").write_text(
                "class Ignored {}\n",
                encoding="utf-8",
            )
            manifest = tmp / "sources.txt"
            manifest_report = tmp / "sources.json"
            manifest_markdown = tmp / "sources.md"
            fake_dotnet = tmp / "dotnet"
            write_fake_tool(fake_dotnet, "8.0.100")

            subprocess.run(
                [
                    sys.executable,
                    str(ROUTE_SOURCE_MANIFEST_SCRIPT),
                    "--target",
                    "csharp",
                    "--source-root",
                    str(tmp),
                    "--output",
                    str(manifest),
                    "--json-output",
                    str(manifest_report),
                    "--markdown-output",
                    str(manifest_markdown),
                    "--min-files",
                    "2",
                    "--exclude-glob",
                    "generated/*",
                    *ROUTE_PROVENANCE_ARGS,
                ],
                cwd=REPO_ROOT,
                check=True,
                capture_output=True,
                text=True,
            )
            completed = subprocess.run(
                [
                    sys.executable,
                    str(SCRIPT),
                    "--target",
                    "csharp",
                    "--source-manifest",
                    str(manifest),
                    "--source-manifest-report",
                    str(manifest_report),
                    "--source-root",
                    str(tmp),
                    "--tool-bin",
                    str(fake_dotnet),
                    "--require-inputs-ready",
                ],
                cwd=REPO_ROOT,
                check=True,
                capture_output=True,
                text=True,
            )
            builder_report = json.loads(manifest_report.read_text(encoding="utf-8"))
            preflight_report = json.loads(completed.stdout)
            manifest_markdown_text = manifest_markdown.read_text(encoding="utf-8")
            manifest_text = manifest.read_text(encoding="utf-8")

        self.assertTrue(preflight_report["inputs_ready"])
        self.assertEqual(preflight_report["missing_inputs"], [])
        self.assertEqual(builder_report["entry_count"], 2)
        self.assertEqual(
            builder_report["corpus_fingerprint"],
            preflight_report["source_manifest"]["corpus_fingerprint"],
        )
        self.assertTrue(preflight_report["source_manifest_report"]["valid"])
        self.assertEqual(
            builder_report["corpus_fingerprint"],
            preflight_report["source_manifest_report"]["corpus_fingerprint"],
        )
        self.assertIn(
            "corpus_provenance",
            preflight_report["source_manifest_report"],
        )
        self.assertEqual(
            [
                entry["manifest_path"]
                for entry in preflight_report["source_manifest"]["entries"]
            ],
            ["Alpha.cs", "Beta.cs"],
        )
        self.assertIn("C# Route Source Manifest", manifest_markdown_text)
        self.assertNotIn("Ignored.cs", manifest_text)

    def test_single_file_manifest_reports_representative_corpus_gap(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            tmp = Path(tmpdir)
            source = tmp / "Sample.cs"
            manifest = tmp / "sources.txt"
            fake_dotnet = tmp / "dotnet"
            source.write_text("class Sample {}\n", encoding="utf-8")
            manifest.write_text(
                "# tct-csharp-route-corpus-v1\nSample.cs\n",
                encoding="utf-8",
            )
            write_fake_tool(fake_dotnet, "8.0.100")

            completed = subprocess.run(
                [
                    sys.executable,
                    str(SCRIPT),
                    "--target",
                    "csharp",
                    "--source-manifest",
                    str(manifest),
                    "--source-root",
                    str(tmp),
                    "--tool-bin",
                    str(fake_dotnet),
                ],
                cwd=REPO_ROOT,
                check=True,
                capture_output=True,
                text=True,
            )
            report = json.loads(completed.stdout)

        self.assertFalse(report["inputs_ready"])
        self.assertFalse(
            report["source_manifest"]["representative_entry_count_ready"]
        )
        self.assertEqual(
            report["source_manifest"]["corpus_fingerprint"]["entry_count"],
            1,
        )
        self.assertEqual(
            report["missing_inputs"],
            [
                {
                    "kind": "representative_csharp_source_corpus",
                    "required_before": "frontend_route_decision_evidence",
                    "status": "insufficient_entries",
                }
            ],
        )

    def test_checked_in_fixture_manifest_keeps_inputs_not_ready(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            tmp = Path(tmpdir)
            fake_dotnet = tmp / "dotnet"
            write_fake_tool(fake_dotnet, "8.0.100")

            completed = subprocess.run(
                [
                    sys.executable,
                    str(SCRIPT),
                    "--target",
                    "csharp",
                    "--source-manifest",
                    "tests/fixtures/csharp_syntax_sources/sources.txt",
                    "--source-root",
                    "tests/fixtures/csharp_syntax_sources",
                    "--tool-bin",
                    str(fake_dotnet),
                ],
                cwd=REPO_ROOT,
                check=True,
                capture_output=True,
                text=True,
            )
            report = json.loads(completed.stdout)

        self.assertFalse(report["inputs_ready"])
        self.assertEqual(
            report["source_manifest"]["evidence_role"],
            "checked_in_fixture_smoke",
        )
        self.assertTrue(report["source_manifest"]["fixture_scope"])
        self.assertEqual(
            [item["kind"] for item in report["missing_inputs"]],
            ["representative_csharp_source_corpus"],
        )

    def test_manifest_report_fingerprint_mismatch_is_actionable(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            tmp = Path(tmpdir)
            (tmp / "Sample.cs").write_text("class Sample {}\n", encoding="utf-8")
            (tmp / "Second.cs").write_text("class Second {}\n", encoding="utf-8")
            manifest = tmp / "sources.txt"
            manifest_report = tmp / "sources.json"
            fake_dotnet = tmp / "dotnet"
            write_fake_tool(fake_dotnet, "8.0.100")

            subprocess.run(
                [
                    sys.executable,
                    str(ROUTE_SOURCE_MANIFEST_SCRIPT),
                    "--target",
                    "csharp",
                    "--source-root",
                    str(tmp),
                    "--output",
                    str(manifest),
                    "--json-output",
                    str(manifest_report),
                    "--min-files",
                    "2",
                    *ROUTE_PROVENANCE_ARGS,
                ],
                cwd=REPO_ROOT,
                check=True,
                capture_output=True,
                text=True,
            )
            manifest_data = json.loads(manifest_report.read_text(encoding="utf-8"))
            manifest_data["corpus_fingerprint"]["sha256"] = "0" * 64
            manifest_report.write_text(json.dumps(manifest_data), encoding="utf-8")

            completed = subprocess.run(
                [
                    sys.executable,
                    str(SCRIPT),
                    "--target",
                    "csharp",
                    "--source-manifest",
                    str(manifest),
                    "--source-manifest-report",
                    str(manifest_report),
                    "--source-root",
                    str(tmp),
                    "--tool-bin",
                    str(fake_dotnet),
                ],
                cwd=REPO_ROOT,
                check=True,
                capture_output=True,
                text=True,
            )
            report = json.loads(completed.stdout)

        self.assertFalse(report["inputs_ready"])
        self.assertFalse(report["source_manifest_report"]["valid"])
        self.assertIn(
            "source manifest report corpus fingerprint mismatch",
            report["source_manifest_report"]["errors"],
        )
        self.assertEqual(
            [item["kind"] for item in report["missing_inputs"]],
            ["csharp_source_manifest_report_validity"],
        )

    def test_invalid_csharp_manifest_reports_marker_and_validity_gaps(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            tmp = Path(tmpdir)
            manifest = tmp / "sources.txt"
            fake_dotnet = tmp / "dotnet"
            manifest.write_text(
                "missing.cs\n"
                "notes.txt\n",
                encoding="utf-8",
            )
            write_fake_tool(fake_dotnet, "8.0.100")

            completed = subprocess.run(
                [
                    sys.executable,
                    str(SCRIPT),
                    "--target",
                    "csharp",
                    "--source-manifest",
                    str(manifest),
                    "--source-root",
                    str(tmp),
                    "--tool-bin",
                    str(fake_dotnet),
                ],
                cwd=REPO_ROOT,
                check=True,
                capture_output=True,
                text=True,
            )
            report = json.loads(completed.stdout)

        self.assertFalse(report["inputs_ready"])
        missing = [item["kind"] for item in report["missing_inputs"]]
        self.assertEqual(
            missing,
            [
                "csharp_source_manifest_validity",
                "csharp_route_decision_manifest_marker",
                "csharp_source_manifest_entries",
            ],
        )
        self.assertIn("source file not found", report["source_manifest"]["errors"][0])
        self.assertIn("expected .cs file", report["source_manifest"]["errors"][1])


if __name__ == "__main__":
    unittest.main()
