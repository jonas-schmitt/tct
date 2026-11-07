#!/usr/bin/env python3
"""Tests for the round-trip validity manifest helper."""

import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[2]
SCRIPT = REPO_ROOT / "scripts/analysis/round_trip_validity_manifest.py"


class TestRoundTripValidityManifest(unittest.TestCase):
    def test_manifest_links_schema_report_and_bpe_artifact(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            tmp = Path(tmpdir)
            schema_dir = tmp / "schemas" / "popular"
            schema_dir.mkdir(parents=True)
            schema_path = schema_dir / "package.json"
            schema_path.write_text(
                json.dumps(
                    {
                        "type": "object",
                        "required": ["name"],
                        "properties": {"name": {"type": "string"}},
                    }
                ),
                encoding="utf-8",
            )
            bpe_dir = tmp / "schemas" / "bpe"
            bpe_dir.mkdir(parents=True)
            (bpe_dir / "package-tct-bpe-500.json").write_text(
                json.dumps(
                    {
                        "schema": "package",
                        "base_vocab_size": 257,
                        "metadata": {"dict_size": 0},
                        "merges": [
                            {"pair": [1, 2], "new_token": 257, "frequency": 3}
                        ],
                    }
                ),
                encoding="utf-8",
            )
            report_path = tmp / "round-trip.json"
            report_path.write_text(
                json.dumps(
                    {
                        "mode": "smoke",
                        "default_smoke": True,
                        "fixture_selection": "default_smoke_manifest",
                        "max_instances": 3,
                        "build_profile": "debug",
                        "round_trip_scenarios_env": None,
                        "bpe_merges": None,
                        "bpe_preset": "legacy-tct-500",
                        "max_base_vocab": None,
                        "schemas": [
                            {
                                "schema_name": "package",
                                "schema_folder": "popular",
                                "instance_count": 1,
                                "instances": ["package/real/react"],
                                "scenarios": [
                                    {
                                        "id": "o3",
                                        "name": "O3",
                                        "total_instances": 1,
                                        "successful_instances": 1,
                                        "failed_instances": [],
                                        "schema_compilation_time_ms": 10,
                                        "code_generation_time_ms": 20,
                                        "rust_compilation_time_ms": 30,
                                        "generated_code_sha256": "a" * 64,
                                    }
                                ],
                            }
                        ],
                    }
                ),
                encoding="utf-8",
            )
            output_path = tmp / "validity.json"

            completed = subprocess.run(
                [
                    sys.executable,
                    str(SCRIPT),
                    "--repo-root",
                    str(tmp),
                    "--round-trip-report",
                    str(report_path),
                    "--output",
                    str(output_path),
                ],
                check=True,
                capture_output=True,
                text=True,
            )

            manifest = json.loads(completed.stdout)
            self.assertEqual(json.loads(output_path.read_text(encoding="utf-8")), manifest)
            self.assertEqual(manifest["manifest_version"], 1)
            self.assertEqual(
                manifest["artifact_kind"],
                "tct_round_trip_validity_manifest",
            )
            self.assertEqual(manifest["tool"], "round_trip_validity_manifest")
            self.assertEqual(manifest["overall_validity_status"], "passed")
            self.assertEqual(manifest["schema_count"], 1)
            schema = manifest["schemas"][0]
            self.assertEqual(schema["schema_name"], "package")
            self.assertEqual(schema["schema_file"]["path"], "schemas/popular/package.json")
            self.assertEqual(schema["round_trip"]["status"], "passed")
            self.assertEqual(schema["round_trip"]["successful_instances"], 1)
            self.assertIn("properties", schema["schema_features_present"])
            self.assertFalse(schema["bpe_policy"]["merge_file_missing"])
            self.assertEqual(schema["bpe_policy"]["merge_file"]["path"], "schemas/bpe/package-tct-bpe-500.json")
            self.assertEqual(schema["pad_bos_eos_policy"]["raw_bos_token"], None)
            self.assertEqual(schema["pad_bos_eos_policy"]["raw_eos_token"], None)
            self.assertNotIn("generated_code_sha256", schema["evidence_gaps"])
            self.assertEqual(
                schema["round_trip"]["scenarios"][0]["generated_code_sha256"],
                "a" * 64,
            )

    def test_manifest_marks_failed_schema(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            tmp = Path(tmpdir)
            schema_dir = tmp / "schemas" / "popular"
            schema_dir.mkdir(parents=True)
            (schema_dir / "broken.json").write_text(
                json.dumps({"type": "object"}),
                encoding="utf-8",
            )
            report_path = tmp / "round-trip.json"
            report_path.write_text(
                json.dumps(
                    {
                        "mode": "smoke",
                        "default_smoke": True,
                        "fixture_selection": "default_smoke_manifest",
                        "build_profile": "debug",
                        "bpe_preset": None,
                        "schemas": [
                            {
                                "schema_name": "broken",
                                "schema_folder": "popular",
                                "instance_count": 1,
                                "instances": ["broken/real/bad"],
                                "scenarios": [
                                    {
                                        "id": "o3",
                                        "name": "O3",
                                        "total_instances": 1,
                                        "successful_instances": 0,
                                        "failed_instances": [
                                            {"instance": "broken/real/bad", "error": "decode failed"}
                                        ],
                                    }
                                ],
                            }
                        ],
                    }
                ),
                encoding="utf-8",
            )

            completed = subprocess.run(
                [
                    sys.executable,
                    str(SCRIPT),
                    "--repo-root",
                    str(tmp),
                    "--round-trip-report",
                    str(report_path),
                ],
                check=True,
                capture_output=True,
                text=True,
            )

            manifest = json.loads(completed.stdout)
            self.assertEqual(manifest["overall_validity_status"], "failed")
            self.assertEqual(manifest["schemas"][0]["round_trip"]["status"], "failed")
            self.assertEqual(manifest["schemas"][0]["round_trip"]["failed_instances"], 1)
            self.assertIn("generated_code_sha256", manifest["schemas"][0]["evidence_gaps"])


if __name__ == "__main__":
    unittest.main()
