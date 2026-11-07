#!/usr/bin/env python3
"""Tests for the advisory module-size report helper."""

import json
import subprocess
import sys
import unittest
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[2]
MODULE_SIZE_REPORT = REPO_ROOT / "scripts/testing/module-size-report.py"


class TestModuleSizeReport(unittest.TestCase):
    def test_json_includes_tracked_queue(self):
        completed = subprocess.run(
            [
                sys.executable,
                str(MODULE_SIZE_REPORT),
                "--repo-root",
                str(REPO_ROOT),
                "--json",
                "--exit-zero",
            ],
            check=True,
            capture_output=True,
            text=True,
        )

        report = json.loads(completed.stdout)
        tracked_paths = {entry["path"] for entry in report["tracked_queue"]}
        self.assertEqual(report["artifact_type"], "tct-module-size-report")
        self.assertEqual(report["version"], 1)
        self.assertEqual(report["tool"], "module-size-report")
        self.assertTrue(report["report_only"])
        self.assertGreaterEqual(report["oversized_count"], report["tracked_count"])
        self.assertEqual(report["untracked_count"], 0)
        self.assertIn("src/generator/struct_generator.rs", tracked_paths)
        self.assertIn("src/generator/type_analyzer.rs", tracked_paths)
        self.assertIn(
            "src/generator/schema_walker/struct_discovery/allof.rs", tracked_paths
        )
        self.assertIn("src/generator/schema_walker/mod.rs", tracked_paths)
        self.assertIn("src/instance_encoder/process.rs", tracked_paths)
        self.assertTrue(all(entry["next_action"] for entry in report["tracked_queue"]))
        self.assertTrue(
            all(entry["production_lines"] >= 0 for entry in report["tracked_queue"])
        )
        self.assertTrue(all(entry["test_lines"] >= 0 for entry in report["tracked_queue"]))

    def test_json_output_writes_report_artifact(self):
        output = REPO_ROOT / "target" / "test-module-size-report.json"
        completed = subprocess.run(
            [
                sys.executable,
                str(MODULE_SIZE_REPORT),
                "--repo-root",
                str(REPO_ROOT),
                "--json-output",
                str(output),
                "--exit-zero",
            ],
            check=True,
            capture_output=True,
            text=True,
        )

        written = json.loads(output.read_text(encoding="utf-8"))
        self.assertIn("Module size report (advisory)", completed.stdout)
        self.assertTrue(written["report_only"])
        self.assertIn("tracked_queue", written)

    def test_text_marks_priority_and_deferred_entries(self):
        completed = subprocess.run(
            [
                sys.executable,
                str(MODULE_SIZE_REPORT),
                "--repo-root",
                str(REPO_ROOT),
                "--top",
                "200",
                "--exit-zero",
            ],
            check=True,
            capture_output=True,
            text=True,
        )

        self.assertIn("Module size report (advisory)", completed.stdout)
        self.assertIn("Tracked advisory queue:", completed.stdout)
        self.assertIn("prod", completed.stdout)
        self.assertIn("test", completed.stdout)
        self.assertIn(
            "defer src/generator/struct_generator/streaming_decode/bitpacked/zero_token.rs",
            completed.stdout,
        )
        self.assertIn(
            "defer src/generator/schema_walker/struct_discovery/allof.rs",
            completed.stdout,
        )
        self.assertIn("defer src/bin/tct-measure-tokens.rs", completed.stdout)


if __name__ == "__main__":
    unittest.main()
