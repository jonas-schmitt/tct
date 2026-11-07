#!/usr/bin/env python3
"""Tests for the advisory documentation coverage report helper."""

import json
import subprocess
import sys
import tempfile
import textwrap
import unittest
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[2]
HELPER = REPO_ROOT / "scripts/testing/documentation-coverage-report.py"


class TestDocumentationCoverageReport(unittest.TestCase):
    def test_json_reports_undocumented_modules(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            root = Path(tmpdir)
            src = root / "src"
            src.mkdir()
            (src / "documented.rs").write_text(
                "//! module docs\npub fn f() {}\n",
                encoding="utf-8",
            )
            (src / "undocumented.rs").write_text("pub fn g() {}\n", encoding="utf-8")

            completed = subprocess.run(
                [
                    sys.executable,
                    str(HELPER),
                    "--repo-root",
                    str(root),
                    "--json",
                    "--exit-zero",
                ],
                check=True,
                capture_output=True,
                text=True,
            )

        report = json.loads(completed.stdout)
        self.assertEqual(report["artifact_type"], "tct-documentation-coverage")
        self.assertEqual(report["version"], 1)
        self.assertEqual(report["tool"], "documentation-coverage-report")
        self.assertTrue(report["report_only"])
        self.assertEqual(report["root"], "src")
        self.assertEqual(report["total_modules"], 2)
        self.assertEqual(report["documented_modules"], 1)
        self.assertEqual(report["undocumented_count"], 1)
        self.assertEqual(report["percent"], 50)
        self.assertEqual(report["undocumented_modules"], ["src/undocumented.rs"])
        self.assertIsNone(report["error"])

    def test_json_output_writes_report_artifact(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            root = Path(tmpdir)
            src = root / "src"
            src.mkdir()
            output = root / "target" / "documentation-coverage-report.json"
            (src / "documented.rs").write_text(
                textwrap.dedent(
                    """\
                    //! module docs

                    pub fn f() {}
                    """
                ),
                encoding="utf-8",
            )

            completed = subprocess.run(
                [
                    sys.executable,
                    str(HELPER),
                    "--repo-root",
                    str(root),
                    "--json-output",
                    str(output),
                ],
                check=True,
                capture_output=True,
                text=True,
            )

            written = json.loads(output.read_text(encoding="utf-8"))

        self.assertIn("Documentation coverage report (advisory)", completed.stdout)
        self.assertEqual(written["documented_modules"], 1)
        self.assertEqual(written["undocumented_modules"], [])

    def test_threshold_failure_still_writes_json_output(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            root = Path(tmpdir)
            src = root / "src"
            src.mkdir()
            output = root / "target" / "documentation-coverage-report.json"
            (src / "undocumented.rs").write_text("pub fn g() {}\n", encoding="utf-8")

            completed = subprocess.run(
                [
                    sys.executable,
                    str(HELPER),
                    "--repo-root",
                    str(root),
                    "--threshold",
                    "100",
                    "--json-output",
                    str(output),
                ],
                check=False,
                capture_output=True,
                text=True,
            )

            written = json.loads(output.read_text(encoding="utf-8"))

        self.assertEqual(completed.returncode, 1)
        self.assertIn("Documented Rust modules: 0/1 = 0%", completed.stdout)
        self.assertEqual(written["undocumented_modules"], ["src/undocumented.rs"])


if __name__ == "__main__":
    unittest.main()
