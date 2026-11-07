#!/usr/bin/env python3
"""Tests for the wheel packaging tool envelope helper."""

import json
import importlib.util
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[2]
SCRIPT = REPO_ROOT / "scripts/testing/wheel-tool-envelope.py"


def load_helper_module():
    spec = importlib.util.spec_from_file_location("wheel_tool_envelope", SCRIPT)
    module = importlib.util.module_from_spec(spec)
    self_loader = spec.loader
    assert self_loader is not None
    self_loader.exec_module(module)
    return module


class TestWheelToolEnvelope(unittest.TestCase):
    def test_reports_python_packaging_tool_envelope(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            output = Path(tmpdir) / "wheel-tool-envelope.json"

            completed = subprocess.run(
                [
                    sys.executable,
                    str(SCRIPT),
                    "--repo-root",
                    str(REPO_ROOT),
                    "--output",
                    str(output),
                ],
                check=True,
                capture_output=True,
                text=True,
            )

            report = json.loads(completed.stdout)
            self.assertEqual(json.loads(output.read_text(encoding="utf-8")), report)
            self.assertEqual(report["manifest_version"], 1)
            self.assertEqual(
                report["artifact_kind"], "tct_python_wheel_tool_envelope"
            )
            self.assertEqual(report["tool"], "wheel-tool-envelope")
            self.assertEqual(report["repo_commit"], self.git_head())
            self.assertTrue(report["python"]["executable"])
            self.assertIn("version", report["python"])
            self.assertIn("pip", report["tools"])
            self.assertIn("maturin", report["tools"])
            self.assertIn("setuptools", report["tools"])
            self.assertIn("wheel", report["tools"])
            self.assertEqual(
                report["tools"]["pip"]["command"],
                [sys.executable, "-m", "pip", "--version"],
            )
            self.assertEqual(
                report["build_inputs"]["source"], "src/wheel_builder.rs"
            )
            self.assertTrue(report["build_inputs"]["readable"])
            schema_wheel = report["build_inputs"]["templates"]["generated_schema_wheel"]
            self.assertEqual(schema_wheel["function"], "generate_pyproject_toml")
            self.assertEqual(schema_wheel["requires"], ["maturin>=1.0,<2.0"])
            self.assertEqual(schema_wheel["build_backend"], "maturin")
            self.assertEqual(
                schema_wheel["expected_requires"], ["maturin>=1.0,<2.0"]
            )
            self.assertEqual(schema_wheel["expected_build_backend"], "maturin")
            self.assertTrue(schema_wheel["requires_marker_present"])
            self.assertTrue(schema_wheel["build_backend_marker_present"])

            bundle_wheel = report["build_inputs"]["templates"]["bundle_wrapper_wheel"]
            self.assertEqual(
                bundle_wheel["function"], "generate_bundle_pyproject_toml"
            )
            self.assertEqual(bundle_wheel["requires"], ["setuptools>=61"])
            self.assertEqual(bundle_wheel["build_backend"], "setuptools.build_meta")
            self.assertEqual(bundle_wheel["expected_requires"], ["setuptools>=61"])
            self.assertEqual(
                bundle_wheel["expected_build_backend"], "setuptools.build_meta"
            )
            self.assertTrue(bundle_wheel["requires_marker_present"])
            self.assertTrue(bundle_wheel["build_backend_marker_present"])

    def test_required_wheel_build_tool_preflight_reports_missing_tools(self):
        helper = load_helper_module()
        report = {
            "tools": {
                "maturin": {"available": False},
                "setuptools": {"available": True},
                "wheel": {"available": False},
            }
        }

        missing = helper.missing_required_wheel_build_tools(report)
        self.assertEqual(
            missing,
            [
                {"tool": "maturin", "purpose": "generated schema wheel builds"},
                {"tool": "wheel", "purpose": "bundle wrapper wheel builds"},
            ],
        )
        message = helper.format_missing_required_wheel_build_tools(
            missing, "target/tct-wheel-tool-envelope.json"
        )
        self.assertIn("maturin (generated schema wheel builds)", message)
        self.assertIn("wheel (bundle wrapper wheel builds)", message)
        self.assertIn("target/tct-wheel-tool-envelope.json", message)
        self.assertIn("scripts/testing/test-wheel-builds.sh", message)

    def test_required_wheel_build_tool_preflight_accepts_available_tools(self):
        helper = load_helper_module()
        report = {
            "tools": {
                "maturin": {"available": True},
                "setuptools": {"available": True},
                "wheel": {"available": True},
            }
        }

        self.assertEqual(helper.missing_required_wheel_build_tools(report), [])

    @staticmethod
    def git_head() -> str:
        completed = subprocess.run(
            ["git", "-C", str(REPO_ROOT), "rev-parse", "HEAD"],
            check=True,
            capture_output=True,
            text=True,
        )
        return completed.stdout.strip()


if __name__ == "__main__":
    unittest.main()
