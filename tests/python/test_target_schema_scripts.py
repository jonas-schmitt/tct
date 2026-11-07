#!/usr/bin/env python3
"""Static checks for target-schema validation helpers."""

import unittest
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[2]


class TestTargetSchemaScripts(unittest.TestCase):
    def test_fast_target_schema_helper_pins_round_trip_environment(self):
        source = (
            REPO_ROOT / "scripts/testing/test-target-schemas.sh"
        ).read_text(encoding="utf-8")

        for variable in (
            "BPE_MERGES",
            "ROUND_TRIP_BPE_PRESET",
            "MAX_INSTANCES",
            "MAX_BASE_VOCAB",
        ):
            self.assertIn(f"-u {variable}", source)

        for assignment in (
            "ROUND_TRIP_FULL=0",
            "ROUND_TRIP_SCENARIOS=o3",
            "ROUND_TRIP_BUILD_PROFILE=debug",
        ):
            self.assertIn(assignment, source)

        self.assertIn("-- --exact", source)

    def test_depth_target_schema_helper_pins_round_trip_environment(self):
        source = (
            REPO_ROOT / "scripts/testing/test-target-schema-depth.sh"
        ).read_text(encoding="utf-8")

        for variable in (
            "BPE_MERGES",
            "ROUND_TRIP_BPE_PRESET",
            "MAX_INSTANCES",
            "MAX_BASE_VOCAB",
        ):
            self.assertIn(f"-u {variable}", source)

        for assignment in (
            "ROUND_TRIP_FULL=1",
            "ROUND_TRIP_SCENARIOS=all",
            "ROUND_TRIP_BUILD_PROFILE=release",
        ):
            self.assertIn(assignment, source)

        self.assertIn("FAIL (missing round-trip report)", source)
        self.assertIn("passed_round_trip_report_hash_gaps", source)
        self.assertIn("-- --exact", source)


if __name__ == "__main__":
    unittest.main()
