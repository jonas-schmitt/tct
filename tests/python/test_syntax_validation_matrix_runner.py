#!/usr/bin/env python3
"""Tests for the syntax validation matrix execution runner."""

import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[2]
RUNNER = REPO_ROOT / "scripts/testing/run-syntax-validation-matrix.py"
TARGET = REPO_ROOT / "target"


class TestSyntaxValidationMatrixRunner(unittest.TestCase):
    def test_dry_run_writes_planned_report_with_matrix_identity(self):
        TARGET.mkdir(exist_ok=True)
        with tempfile.TemporaryDirectory(dir=TARGET) as tmp:
            output = Path(tmp) / "planned-report.json"
            completed = subprocess.run(
                [
                    sys.executable,
                    str(RUNNER),
                    "--dry-run",
                    "--output",
                    str(output),
                ],
                cwd=REPO_ROOT,
                text=True,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                check=False,
            )

            self.assertEqual(completed.returncode, 0, completed.stderr)
            report = json.loads(output.read_text(encoding="utf-8"))
            self.assertEqual(
                report["artifact_kind"],
                "tct_syntax_backend_validation_matrix_execution_report",
            )
            self.assertEqual(report["tool"], "syntax_backend_validation_matrix_runner")
            self.assertEqual(
                report["status"], "planned_release_depth_execution_report"
            )
            self.assertEqual(
                report["matrix_id"],
                "syntax-json-backend-grade-validation-matrix-v1",
            )
            self.assertFalse(report["arena_ir_proof_used"])
            self.assertTrue(report["json_schema_backend_unchanged"])
            self.assertEqual(len(report["category_results"]), 10)
            self.assertEqual(len(report["command_results"]), 6)

    def test_validate_report_accepts_passed_execution_shape(self):
        TARGET.mkdir(exist_ok=True)
        with tempfile.TemporaryDirectory(dir=TARGET) as tmp:
            output = Path(tmp) / "execution-report.json"
            planned = subprocess.run(
                [
                    sys.executable,
                    str(RUNNER),
                    "--dry-run",
                    "--output",
                    str(output),
                ],
                cwd=REPO_ROOT,
                text=True,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                check=False,
            )
            self.assertEqual(planned.returncode, 0, planned.stderr)

            report = json.loads(output.read_text(encoding="utf-8"))
            report["status"] = "release_depth_execution_report"
            report["all_passed"] = True
            for command in report["command_results"]:
                command["status"] = "passed"
                command["exit_code"] = 0
            for category in report["category_results"]:
                category["command_status"] = "passed"
            output.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")

            validation = subprocess.run(
                [
                    sys.executable,
                    str(RUNNER),
                    "--validate-report",
                    str(output),
                ],
                cwd=REPO_ROOT,
                text=True,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                check=False,
            )

            self.assertEqual(validation.returncode, 0, validation.stderr)
            self.assertIn("validated 10 matrix categories", validation.stdout)

    def test_validate_report_rejects_failed_category(self):
        TARGET.mkdir(exist_ok=True)
        with tempfile.TemporaryDirectory(dir=TARGET) as tmp:
            output = Path(tmp) / "execution-report.json"
            planned = subprocess.run(
                [
                    sys.executable,
                    str(RUNNER),
                    "--dry-run",
                    "--output",
                    str(output),
                ],
                cwd=REPO_ROOT,
                text=True,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                check=False,
            )
            self.assertEqual(planned.returncode, 0, planned.stderr)

            report = json.loads(output.read_text(encoding="utf-8"))
            report["status"] = "release_depth_execution_report"
            report["all_passed"] = True
            for command in report["command_results"]:
                command["status"] = "passed"
                command["exit_code"] = 0
            for category in report["category_results"]:
                category["command_status"] = "passed"
            report["category_results"][0]["command_status"] = "failed"
            output.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")

            validation = subprocess.run(
                [
                    sys.executable,
                    str(RUNNER),
                    "--validate-report",
                    str(output),
                ],
                cwd=REPO_ROOT,
                text=True,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                check=False,
            )

            self.assertNotEqual(validation.returncode, 0)
            self.assertIn("report category did not pass", validation.stderr)


if __name__ == "__main__":
    unittest.main()
