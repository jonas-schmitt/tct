#!/usr/bin/env python3
"""Tests for the optional C# dotnet smoke helper."""

import json
import os
import stat
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[2]
SCRIPT = REPO_ROOT / "scripts/analysis/csharp_toolchain_smoke.py"


class TestCSharpToolchainSmoke(unittest.TestCase):
    def test_self_test(self):
        completed = subprocess.run(
            [sys.executable, str(SCRIPT), "--self-test"],
            cwd=REPO_ROOT,
            check=True,
            capture_output=True,
            text=True,
        )
        self.assertIn("self-test passed", completed.stdout)

    def test_reports_unavailable_toolchain_without_failing(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            tmp = Path(tmpdir)
            source = tmp / "Sample.cs"
            report_path = tmp / "report.json"
            source.write_text("public sealed class Sample { }\n")

            completed = subprocess.run(
                [
                    sys.executable,
                    str(SCRIPT),
                    str(tmp),
                    "--dotnet-bin",
                    str(tmp / "missing-dotnet"),
                    "--json-output",
                    str(report_path),
                ],
                cwd=REPO_ROOT,
                check=True,
                capture_output=True,
                text=True,
            )

            self.assertEqual(completed.stdout, "")
            report = json.loads(report_path.read_text())

        self.assertEqual(report["tool"], "csharp_toolchain_smoke")
        self.assertEqual(report["version"], 1)
        self.assertEqual(report["summary"]["files"], 1)
        self.assertEqual(report["corpus"]["file_count"], 1)
        self.assertEqual(len(report["corpus"]["sha256"]), 64)
        self.assertEqual(report["summary"]["checked"], 0)
        self.assertFalse(report["toolchain"]["available"])
        self.assertEqual(report["files"][0]["checked"], False)
        self.assertIsNone(report["files"][0]["success"])

    def test_collects_fake_dotnet_diagnostics(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            tmp = Path(tmpdir)
            fake_dotnet = tmp / "fake-dotnet"
            good_source = tmp / "Good.cs"
            bad_source = tmp / "Bad.cs"
            report_path = tmp / "report.json"
            markdown_path = tmp / "report.md"
            good_source.write_text("public sealed class Good { }\n")
            bad_source.write_text("public sealed class Bad { public void M() { } }\n")
            fake_dotnet.write_text(
                "\n".join(
                    [
                        f"#!{sys.executable}",
                        "import pathlib",
                        "import sys",
                        "if '--version' in sys.argv:",
                        "    print('8.0.100')",
                        "    raise SystemExit(0)",
                        "project = pathlib.Path(sys.argv[2])",
                        "source = project.parent / 'Input.cs'",
                        "if 'Bad' in source.read_text():",
                        "    print(f'{source}(1,39): error CS1002: ; expected [{project}]')",
                        "    raise SystemExit(1)",
                        "raise SystemExit(0)",
                        "",
                    ]
                )
            )
            fake_dotnet.chmod(fake_dotnet.stat().st_mode | stat.S_IXUSR)

            env = os.environ.copy()
            env["PATH"] = tmpdir
            subprocess.run(
                [
                    sys.executable,
                    str(SCRIPT),
                    str(tmp),
                    "--dotnet-bin",
                    str(fake_dotnet),
                    "--json-output",
                    str(report_path),
                    "--markdown-output",
                    str(markdown_path),
                ],
                cwd=REPO_ROOT,
                check=True,
                capture_output=True,
                text=True,
                env=env,
            )

            report = json.loads(report_path.read_text())
            markdown = markdown_path.read_text()

        self.assertTrue(report["toolchain"]["available"])
        self.assertEqual(report["toolchain"]["version"], "8.0.100")
        self.assertEqual(report["summary"]["files"], 2)
        self.assertEqual(report["corpus"]["file_count"], 2)
        self.assertEqual(len(report["corpus"]["sha256"]), 64)
        self.assertEqual(report["summary"]["checked"], 2)
        self.assertEqual(report["summary"]["successes"], 1)
        self.assertEqual(report["summary"]["failures"], 1)
        self.assertEqual(report["summary"]["diagnostics"], 1)
        self.assertEqual(report["summary"]["parse_diagnostics"], 1)
        failing_file = next(file for file in report["files"] if file["path"].endswith("Bad.cs"))
        self.assertFalse(failing_file["success"])
        self.assertEqual(failing_file["diagnostics"][0]["code"], "CS1002")
        self.assertEqual(failing_file["diagnostics"][0]["category"], "parse")
        self.assertIn("C# Toolchain Smoke Report", markdown)

    def test_source_manifest_records_marker_and_relative_paths(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            tmp = Path(tmpdir)
            source_dir = tmp / "src"
            source_dir.mkdir()
            source = source_dir / "Sample.cs"
            manifest = tmp / "csharp-sources.txt"
            report_path = tmp / "report.json"
            source.write_text("public sealed class Sample { }\n")
            manifest.write_text(
                "# tct-csharp-route-corpus-v1\n"
                "# pinned test corpus\n"
                "src/Sample.cs\n"
            )

            subprocess.run(
                [
                    sys.executable,
                    str(SCRIPT),
                    "--source-manifest",
                    str(manifest),
                    "--source-root",
                    str(tmp),
                    "--dotnet-bin",
                    str(tmp / "missing-dotnet"),
                    "--json-output",
                    str(report_path),
                ],
                cwd=REPO_ROOT,
                check=True,
                capture_output=True,
                text=True,
            )

            report = json.loads(report_path.read_text())

        self.assertEqual(report["scope"]["input_kind"], "source_manifest")
        self.assertEqual(report["corpus"]["file_count"], 1)
        self.assertEqual(report["files"][0]["path"], "src/Sample.cs")
        self.assertEqual(report["source_manifest"]["entry_count"], 1)
        self.assertTrue(report["source_manifest"]["route_decision_marker_present"])
        self.assertEqual(
            report["source_manifest"]["route_decision_marker"],
            "tct-csharp-route-corpus-v1",
        )
        self.assertEqual(
            report["source_manifest"]["entries"][0]["manifest_path"],
            "src/Sample.cs",
        )
        self.assertFalse(report["toolchain"]["available"])


if __name__ == "__main__":
    unittest.main()
