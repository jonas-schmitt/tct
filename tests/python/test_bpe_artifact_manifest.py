#!/usr/bin/env python3
"""Tests for the checked-in BPE artifact manifest helper."""

import hashlib
import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[2]
SCRIPT = REPO_ROOT / "scripts/analysis/bpe_artifact_manifest.py"


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def file_record(root: Path, relative: str) -> dict:
    path = root / relative
    return {
        "path": relative,
        "bytes": path.stat().st_size,
        "sha256": sha256_file(path),
    }


def corpus_fingerprint(records: list[dict]) -> str:
    digest = hashlib.sha256()
    digest.update(b"tct-bpe-artifact-inputs-v1\n")
    for record in records:
        digest.update(record["path"].encode("utf-8"))
        digest.update(b"\0")
        digest.update(str(record["bytes"]).encode("ascii"))
        digest.update(b"\0")
        digest.update(record["sha256"].encode("ascii"))
        digest.update(b"\n")
    return digest.hexdigest()


def token_corpus(sequence_count: int, source: str = "instance_encoding") -> dict:
    return {
        "source": source,
        "sequence_count": sequence_count,
        "sequence_fingerprint": "a" * 64,
        "sample_size": None,
        "sample_seed": None,
        "cache_path": None,
        "cache_hit": False,
        "cache_written": False,
        "parallel_early_stop": False,
        "truncated_to_sample_size": False,
        "selected_instance_files": [],
        "accepted_instance_files": [],
        "rejected_instance_files": [],
    }


class TestBpeArtifactManifest(unittest.TestCase):
    def test_manifest_reports_bpe_layout_and_sidecar_provenance(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            tmp = Path(tmpdir)
            bpe_dir = tmp / "schemas" / "bpe"
            bpe_dir.mkdir(parents=True)
            input_path = tmp / "tests/data/instances/package/one.json"
            input_path.parent.mkdir(parents=True)
            input_path.write_text("{}", encoding="utf-8")
            schema_path = tmp / "schemas/popular/package.json"
            schema_path.parent.mkdir(parents=True)
            schema_path.write_text("{}", encoding="utf-8")
            merge_path = bpe_dir / "package-tct-bpe-2048.json"
            sidecar_path = bpe_dir / "package-tct-bpe-2048.provenance.json"
            output_path = tmp / "manifest.json"

            merge_path.write_text(
                json.dumps(
                    {
                        "schema": "package",
                        "base_vocab_size": 257,
                        "metadata": {
                            "trained_on_instances": 3,
                            "max_merges": 2,
                            "dict_size": 0,
                        },
                        "merges": [
                            {"pair": [1, 2], "new_token": 257, "frequency": 9},
                            {"pair": [257, 3], "new_token": 258, "frequency": 4},
                        ],
                    }
                ),
                encoding="utf-8",
            )
            merge_sha256 = sha256_file(merge_path)
            input_files = [file_record(tmp, "tests/data/instances/package/one.json")]
            sidecar_path.write_text(
                json.dumps(
                    {
                        "created_at": "2026-05-03T00:00:00Z",
                        "tct_version": "0.1.0",
                        "repo_commit": "1" * 40,
                        "repo_dirty": False,
                        "command": ["tct-train-bpe", "--target-vocab", "2048"],
                        "base_vocab_size": 257,
                        "instances_pattern": "tests/data/instances/package/*.json",
                        "schema_file": file_record(tmp, "schemas/popular/package.json"),
                        "input_files": input_files,
                        "input_corpus_fingerprint": corpus_fingerprint(input_files),
                        "target_vocab": 2048,
                        "trained_on_instances": 3,
                        "generated_merges": 2,
                        "token_corpus": token_corpus(3),
                        "output_files": [
                            {
                                "path": "schemas/bpe/package-tct-bpe-2048.json",
                                "bytes": merge_path.stat().st_size,
                                "sha256": merge_sha256,
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
                    "--bpe-dir",
                    "schemas/bpe",
                    "--output",
                    str(output_path),
                ],
                check=True,
                capture_output=True,
                text=True,
            )

            report = json.loads(completed.stdout)
            self.assertEqual(json.loads(output_path.read_text(encoding="utf-8")), report)
            self.assertEqual(report["manifest_version"], 4)
            self.assertEqual(report["artifact_kind"], "tct_bpe_artifact_inventory")
            self.assertEqual(report["tool"], "bpe_artifact_manifest")
            self.assertEqual(report["artifact_count"], 1)
            self.assertEqual(
                report["provenance_summary"]["status_counts"],
                {"provenance_ready": 1},
            )
            self.assertEqual(
                report["provenance_summary"]["claim_use_counts"],
                {"candidate_artifact_evidence_not_paper_claim": 1},
            )
            self.assertEqual(
                report["provenance_summary"]["roadmap_policy_counts"],
                {"current_single_schema_candidate": 1},
            )
            self.assertEqual(
                report["provenance_summary"][
                    "default_reproducibility_queue_scope"
                ],
                "current_single_schema_candidate",
            )
            self.assertEqual(
                report["provenance_summary"][
                    "default_reproducibility_queue_policy_artifacts"
                ],
                ["schemas/bpe/package-tct-bpe-2048.json"],
            )
            self.assertEqual(
                report["provenance_summary"][
                    "default_reproducibility_queue_policy_artifact_count"
                ],
                1,
            )
            self.assertEqual(
                report["provenance_summary"][
                    "default_reproducibility_queue_artifacts"
                ],
                ["schemas/bpe/package-tct-bpe-2048.json"],
            )
            self.assertEqual(
                report["provenance_summary"][
                    "default_reproducibility_queue_artifact_count"
                ],
                1,
            )
            self.assertEqual(
                report["provenance_summary"][
                    "default_reproducibility_queue_blocked_artifacts"
                ],
                [],
            )
            self.assertEqual(
                report["provenance_summary"][
                    "default_reproducibility_queue_blocked_artifact_count"
                ],
                0,
            )
            self.assertEqual(
                report["provenance_summary"]["provenance_ready_artifacts"],
                ["schemas/bpe/package-tct-bpe-2048.json"],
            )
            artifact = report["artifacts"][0]
            self.assertEqual(artifact["path"], "schemas/bpe/package-tct-bpe-2048.json")
            self.assertEqual(artifact["sha256"], merge_sha256)
            self.assertEqual(artifact["schema_from_filename"], "package")
            self.assertEqual(artifact["encoding_family"], "tct")
            self.assertEqual(artifact["target_label"], "2048")
            self.assertEqual(artifact["base_vocab_size"], 257)
            self.assertEqual(artifact["merge_count"], 2)
            self.assertEqual(artifact["first_new_token"], 257)
            self.assertEqual(artifact["last_new_token"], 258)
            self.assertEqual(artifact["generated_vocab_size_with_pad"], 260)
            self.assertEqual(
                artifact["roadmap_policy"]["queue_scope"],
                "current_single_schema_candidate",
            )
            self.assertEqual(
                artifact["roadmap_policy"]["vocab_source"],
                "provenance_target_vocab",
            )
            self.assertEqual(artifact["roadmap_policy"]["policy_vocab"], 2048)
            self.assertEqual(artifact["metadata"]["dict_size"], 0)
            self.assertEqual(
                artifact["provenance"]["path"],
                "schemas/bpe/package-tct-bpe-2048.provenance.json",
            )
            self.assertEqual(artifact["provenance"]["target_vocab"], 2048)
            self.assertEqual(artifact["provenance"]["trained_on_instances"], 3)
            self.assertEqual(artifact["provenance"]["input_file_count"], 1)
            self.assertEqual(
                artifact["provenance"]["input_corpus_fingerprint"],
                corpus_fingerprint(input_files),
            )
            self.assertEqual(artifact["provenance_status"], "provenance_ready")
            self.assertEqual(artifact["provenance_gaps"], [])
            self.assertEqual(
                artifact["current_claim_use"],
                "candidate_artifact_evidence_not_paper_claim",
            )
            handoff = artifact["cross_repo_handoff"]
            self.assertTrue(handoff["ready_for_claim_profile"])
            self.assertEqual(handoff["handoff_gaps"], [])
            self.assertEqual(
                handoff["artifact"]["path"],
                "schemas/bpe/package-tct-bpe-2048.json",
            )
            self.assertEqual(handoff["artifact"]["sha256"], merge_sha256)
            self.assertEqual(
                handoff["provenance_sidecar"]["path"],
                "schemas/bpe/package-tct-bpe-2048.provenance.json",
            )
            self.assertEqual(
                handoff["provenance_sidecar"]["sha256"],
                sha256_file(sidecar_path),
            )
            self.assertEqual(handoff["training"]["tct_version"], "0.1.0")
            self.assertEqual(handoff["training"]["repo_commit"], "1" * 40)
            self.assertFalse(handoff["training"]["repo_dirty"])
            self.assertEqual(
                handoff["training"]["schema_file"]["path"],
                "schemas/popular/package.json",
            )
            self.assertEqual(
                handoff["training"]["input_corpus_fingerprint"],
                corpus_fingerprint(input_files),
            )
            self.assertEqual(
                handoff["training"]["token_corpus"]["sequence_fingerprint"],
                "a" * 64,
            )
            self.assertEqual(
                report["provenance_summary"]["cross_repo_handoff_ready_artifacts"],
                ["schemas/bpe/package-tct-bpe-2048.json"],
            )

    def test_manifest_classifies_bpe_roadmap_policy_scopes(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            tmp = Path(tmpdir)
            bpe_dir = tmp / "schemas" / "bpe"
            bpe_dir.mkdir(parents=True)

            artifacts = {
                "small-tct-bpe-512.json": {"base_vocab_size": 511, "merges": []},
                "odd-tct-bpe-1000.json": {"base_vocab_size": 999, "merges": []},
                "wide-tct-bpe-4096.json": {"base_vocab_size": 4095, "merges": []},
                "large-tct-bpe-5k.json": {
                    "base_vocab_size": 4096,
                    "merges": [{"pair": [1, 2], "new_token": 4096, "frequency": 1}],
                },
                "saturated-tct-bpe-5k.json": {
                    "base_vocab_size": 257,
                    "merges": [{"pair": [1, 2], "new_token": 257, "frequency": 1}],
                },
                "small-utf8-bpe-500.json": {"base_vocab_size": 257, "merges": []},
            }
            for name, merge_table in artifacts.items():
                (bpe_dir / name).write_text(
                    json.dumps(
                        {
                            "schema": name.split("-", 1)[0],
                            "metadata": {},
                            **merge_table,
                        }
                    ),
                    encoding="utf-8",
                )
            (bpe_dir / "saturated-tct-bpe-5k.provenance.json").write_text(
                json.dumps({"target_vocab": 5000}),
                encoding="utf-8",
            )

            completed = subprocess.run(
                [
                    sys.executable,
                    str(SCRIPT),
                    "--repo-root",
                    str(tmp),
                    "--bpe-dir",
                    "schemas/bpe",
                ],
                check=True,
                capture_output=True,
                text=True,
            )

            report = json.loads(completed.stdout)
            policy_by_path = {
                artifact["path"]: artifact["roadmap_policy"]
                for artifact in report["artifacts"]
            }
            artifacts_by_path = {
                path: policy["queue_scope"] for path, policy in policy_by_path.items()
            }
            self.assertEqual(
                report["provenance_summary"]["roadmap_policy_counts"],
                {
                    "comparison_artifact_not_local_queue": 1,
                    "current_single_schema_candidate": 1,
                    "explicit_4096_candidate": 1,
                    "historical_non_power_of_two_vocab_policy": 1,
                    "historical_excluded_by_vocab_policy": 2,
                },
            )
            self.assertEqual(
                artifacts_by_path["schemas/bpe/small-tct-bpe-512.json"],
                "current_single_schema_candidate",
            )
            self.assertEqual(
                artifacts_by_path["schemas/bpe/odd-tct-bpe-1000.json"],
                "historical_non_power_of_two_vocab_policy",
            )
            self.assertEqual(
                artifacts_by_path["schemas/bpe/wide-tct-bpe-4096.json"],
                "explicit_4096_candidate",
            )
            self.assertEqual(
                artifacts_by_path["schemas/bpe/large-tct-bpe-5k.json"],
                "historical_excluded_by_vocab_policy",
            )
            self.assertEqual(
                artifacts_by_path["schemas/bpe/saturated-tct-bpe-5k.json"],
                "historical_excluded_by_vocab_policy",
            )
            self.assertEqual(
                policy_by_path["schemas/bpe/saturated-tct-bpe-5k.json"][
                    "vocab_source"
                ],
                "provenance_target_vocab",
            )
            self.assertEqual(
                policy_by_path["schemas/bpe/saturated-tct-bpe-5k.json"][
                    "policy_vocab"
                ],
                5000,
            )
            self.assertEqual(
                policy_by_path["schemas/bpe/large-tct-bpe-5k.json"][
                    "vocab_source"
                ],
                "generated_vocab_size_with_pad",
            )
            self.assertEqual(
                artifacts_by_path["schemas/bpe/small-utf8-bpe-500.json"],
                "comparison_artifact_not_local_queue",
            )

    def test_handoff_requires_repo_git_state_for_claim_profile(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            tmp = Path(tmpdir)
            bpe_dir = tmp / "schemas" / "bpe"
            bpe_dir.mkdir(parents=True)
            input_path = tmp / "tests/data/instances/package/one.json"
            input_path.parent.mkdir(parents=True)
            input_path.write_text("{}", encoding="utf-8")
            merge_path = bpe_dir / "package-tct-bpe-500.json"
            merge_path.write_text(
                json.dumps(
                    {
                        "schema": "package",
                        "base_vocab_size": 257,
                        "metadata": {"trained_on_instances": 1, "max_merges": 1},
                        "merges": [
                            {"pair": [1, 2], "new_token": 257, "frequency": 9}
                        ],
                    }
                ),
                encoding="utf-8",
            )
            input_files = [file_record(tmp, "tests/data/instances/package/one.json")]
            merge_path.with_suffix(".provenance.json").write_text(
                json.dumps(
                    {
                        "command": ["tct-train-bpe", "--target-vocab", "500"],
                        "base_vocab_size": 257,
                        "instances_pattern": "tests/data/instances/package/*.json",
                        "input_files": input_files,
                        "input_corpus_fingerprint": corpus_fingerprint(input_files),
                        "target_vocab": 500,
                        "trained_on_instances": 1,
                        "generated_merges": 1,
                        "token_corpus": token_corpus(1),
                        "output_files": [
                            {
                                "path": "schemas/bpe/package-tct-bpe-500.json",
                                "bytes": merge_path.stat().st_size,
                                "sha256": sha256_file(merge_path),
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
                    "--bpe-dir",
                    "schemas/bpe",
                ],
                check=True,
                capture_output=True,
                text=True,
            )

            artifact = json.loads(completed.stdout)["artifacts"][0]
            handoff = artifact["cross_repo_handoff"]
            self.assertEqual(artifact["provenance_status"], "provenance_ready")
            self.assertFalse(handoff["ready_for_claim_profile"])
            self.assertIn("missing_repo_commit", handoff["handoff_gaps"])
            self.assertIn("missing_repo_dirty", handoff["handoff_gaps"])

    def test_handoff_blocks_historical_vocab_policy_for_claim_profile(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            tmp = Path(tmpdir)
            bpe_dir = tmp / "schemas" / "bpe"
            bpe_dir.mkdir(parents=True)
            input_path = tmp / "tests/data/instances/package/one.json"
            input_path.parent.mkdir(parents=True)
            input_path.write_text("{}", encoding="utf-8")
            merge_path = bpe_dir / "package-tct-bpe-5k.json"
            merge_path.write_text(
                json.dumps(
                    {
                        "schema": "package",
                        "base_vocab_size": 257,
                        "metadata": {"trained_on_instances": 1, "max_merges": 1},
                        "merges": [
                            {"pair": [1, 2], "new_token": 257, "frequency": 9}
                        ],
                    }
                ),
                encoding="utf-8",
            )
            input_files = [file_record(tmp, "tests/data/instances/package/one.json")]
            merge_path.with_suffix(".provenance.json").write_text(
                json.dumps(
                    {
                        "command": ["tct-train-bpe", "--target-vocab", "5000"],
                        "base_vocab_size": 257,
                        "instances_pattern": "tests/data/instances/package/*.json",
                        "input_files": input_files,
                        "input_corpus_fingerprint": corpus_fingerprint(input_files),
                        "target_vocab": 5000,
                        "trained_on_instances": 1,
                        "generated_merges": 1,
                        "repo_commit": "1" * 40,
                        "repo_dirty": False,
                        "token_corpus": token_corpus(1),
                        "output_files": [
                            {
                                "path": "schemas/bpe/package-tct-bpe-5k.json",
                                "bytes": merge_path.stat().st_size,
                                "sha256": sha256_file(merge_path),
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
                    "--bpe-dir",
                    "schemas/bpe",
                ],
                check=True,
                capture_output=True,
                text=True,
            )

            report = json.loads(completed.stdout)
            artifact = report["artifacts"][0]
            handoff = artifact["cross_repo_handoff"]
            self.assertEqual(artifact["provenance_status"], "provenance_ready")
            self.assertEqual(
                artifact["roadmap_policy"]["queue_scope"],
                "historical_excluded_by_vocab_policy",
            )
            self.assertFalse(handoff["ready_for_claim_profile"])
            self.assertIn(
                "roadmap_policy_historical_excluded_by_vocab_policy_not_claim_profile_ready",
                handoff["handoff_gaps"],
            )
            self.assertEqual(
                report["provenance_summary"]["cross_repo_handoff_ready_artifacts"],
                [],
            )
            self.assertEqual(
                report["provenance_summary"]["cross_repo_handoff_blocked_artifacts"],
                ["schemas/bpe/package-tct-bpe-5k.json"],
            )
            self.assertEqual(
                report["provenance_summary"]["cross_repo_handoff_gap_counts"],
                {
                    "roadmap_policy_historical_excluded_by_vocab_policy_not_claim_profile_ready": 1
                },
            )
            self.assertEqual(
                report["provenance_summary"]["cross_repo_handoff_blocked_artifacts_by_gap"],
                {
                    "roadmap_policy_historical_excluded_by_vocab_policy_not_claim_profile_ready": [
                        "schemas/bpe/package-tct-bpe-5k.json"
                    ]
                },
            )

    def test_manifest_classifies_missing_and_incomplete_sidecars(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            tmp = Path(tmpdir)
            bpe_dir = tmp / "schemas" / "bpe"
            bpe_dir.mkdir(parents=True)
            missing_sidecar_path = bpe_dir / "missing-tct-bpe-500.json"
            incomplete_sidecar_path = bpe_dir / "incomplete-tct-bpe-500.json"

            merge_table = {
                "schema": "test",
                "base_vocab_size": 257,
                "metadata": {
                    "trained_on_instances": 1,
                    "max_merges": 1,
                    "dict_size": 0,
                },
                "merges": [{"pair": [1, 2], "new_token": 257, "frequency": 9}],
            }
            missing_sidecar_path.write_text(json.dumps(merge_table), encoding="utf-8")
            incomplete_sidecar_path.write_text(json.dumps(merge_table), encoding="utf-8")
            incomplete_sidecar_path.with_suffix(".provenance.json").write_text(
                json.dumps(
                    {
                        "command": ["tct-train-bpe"],
                        "target_vocab": 2048,
                        "output_files": [
                            {
                                "path": "schemas/bpe/incomplete-tct-bpe-500.json",
                                "bytes": 1,
                                "sha256": "f" * 64,
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
                    "--bpe-dir",
                    "schemas/bpe",
                ],
                check=True,
                capture_output=True,
                text=True,
            )

            report = json.loads(completed.stdout)
            artifacts = {artifact["path"]: artifact for artifact in report["artifacts"]}
            summary = report["provenance_summary"]
            missing = artifacts["schemas/bpe/missing-tct-bpe-500.json"]
            incomplete = artifacts["schemas/bpe/incomplete-tct-bpe-500.json"]

            self.assertEqual(
                summary["status_counts"],
                {"incomplete_sidecar": 1, "missing_sidecar": 1},
            )
            self.assertEqual(
                summary["roadmap_policy_counts"],
                {
                    "current_single_schema_candidate": 1,
                    "historical_non_power_of_two_vocab_policy": 1,
                },
            )
            self.assertEqual(
                summary["default_reproducibility_queue_policy_artifacts"],
                ["schemas/bpe/incomplete-tct-bpe-500.json"],
            )
            self.assertEqual(
                summary["default_reproducibility_queue_policy_artifact_count"], 1
            )
            self.assertEqual(summary["default_reproducibility_queue_artifacts"], [])
            self.assertEqual(summary["default_reproducibility_queue_artifact_count"], 0)
            self.assertEqual(
                summary["default_reproducibility_queue_blocked_artifacts"],
                ["schemas/bpe/incomplete-tct-bpe-500.json"],
            )
            self.assertEqual(
                summary["default_reproducibility_queue_blocked_artifact_count"], 1
            )
            self.assertEqual(
                summary["blocked_artifacts"],
                [
                    "schemas/bpe/incomplete-tct-bpe-500.json",
                    "schemas/bpe/missing-tct-bpe-500.json",
                ],
            )
            self.assertEqual(summary["blocked_artifact_count"], 2)
            self.assertEqual(summary["gap_counts"]["missing_provenance_sidecar"], 1)
            self.assertEqual(summary["gap_counts"]["missing_token_corpus"], 1)
            self.assertEqual(
                summary["blocked_artifacts_by_gap"]["missing_provenance_sidecar"],
                ["schemas/bpe/missing-tct-bpe-500.json"],
            )
            self.assertEqual(missing["provenance_status"], "missing_sidecar")
            self.assertIn("missing_provenance_sidecar", missing["provenance_gaps"])
            self.assertEqual(
                missing["current_claim_use"], "blocked_without_fresh_provenance"
            )
            self.assertEqual(
                missing["roadmap_policy"]["queue_scope"],
                "historical_non_power_of_two_vocab_policy",
            )
            self.assertFalse(
                missing["cross_repo_handoff"]["ready_for_claim_profile"]
            )
            self.assertIn(
                "missing_provenance_sidecar",
                missing["cross_repo_handoff"]["handoff_gaps"],
            )
            self.assertIsNone(missing["cross_repo_handoff"]["provenance_sidecar"])

            self.assertEqual(incomplete["provenance_status"], "incomplete_sidecar")
            self.assertEqual(
                incomplete["roadmap_policy"]["queue_scope"],
                "current_single_schema_candidate",
            )
            self.assertIn("missing_input_files", incomplete["provenance_gaps"])
            self.assertIn(
                "missing_input_corpus_fingerprint", incomplete["provenance_gaps"]
            )
            self.assertIn(
                "output_file_sha256_does_not_match_artifact",
                incomplete["provenance_gaps"],
            )
            self.assertIn("missing_token_corpus", incomplete["provenance_gaps"])

    def test_manifest_classifies_mismatched_sidecar_fields(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            tmp = Path(tmpdir)
            bpe_dir = tmp / "schemas" / "bpe"
            bpe_dir.mkdir(parents=True)
            input_path = tmp / "tests/data/instances/package/one.json"
            input_path.parent.mkdir(parents=True)
            input_path.write_text("{}", encoding="utf-8")
            merge_path = bpe_dir / "package-tct-bpe-500.json"

            merge_table = {
                "schema": "package",
                "base_vocab_size": 257,
                "metadata": {
                    "trained_on_instances": 3,
                    "max_merges": 2,
                    "dict_size": 0,
                },
                "merges": [
                    {"pair": [1, 2], "new_token": 257, "frequency": 9},
                    {"pair": [257, 3], "new_token": 258, "frequency": 4},
                ],
            }
            merge_path.write_text(json.dumps(merge_table), encoding="utf-8")
            input_files = [file_record(tmp, "tests/data/instances/package/one.json")]
            sidecar = {
                "command": ["tct-train-bpe", "--target-vocab", "500"],
                "base_vocab_size": 999,
                "instances_pattern": "tests/data/instances/package/*.json",
                "input_files": [
                    {
                        **input_files[0],
                        "sha256": "0" * 64,
                    }
                ],
                "input_corpus_fingerprint": corpus_fingerprint(input_files),
                "target_vocab": 500,
                "trained_on_instances": 4,
                "generated_merges": 1,
                "token_corpus": token_corpus(4),
                "output_files": [
                    {
                        "path": "schemas/bpe/other.json",
                        "bytes": merge_path.stat().st_size,
                        "sha256": sha256_file(merge_path),
                    }
                ],
            }
            merge_path.with_suffix(".provenance.json").write_text(
                json.dumps(sidecar),
                encoding="utf-8",
            )

            completed = subprocess.run(
                [
                    sys.executable,
                    str(SCRIPT),
                    "--repo-root",
                    str(tmp),
                    "--bpe-dir",
                    "schemas/bpe",
                ],
                check=True,
                capture_output=True,
                text=True,
            )

            artifact = json.loads(completed.stdout)["artifacts"][0]
            self.assertEqual(artifact["provenance_status"], "incomplete_sidecar")
            gaps = artifact["provenance_gaps"]
            self.assertIn("output_file_path_does_not_match_artifact", gaps)
            self.assertIn("base_vocab_size_does_not_match_artifact", gaps)
            self.assertIn("generated_merges_does_not_match_artifact", gaps)
            self.assertIn("trained_on_instances_does_not_match_artifact", gaps)
            self.assertIn("input_files_do_not_match_current_files", gaps)
            self.assertIn(
                "input_corpus_fingerprint_does_not_match_input_files", gaps
            )

    def test_manifest_rejects_split_output_path_and_hash_records(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            tmp = Path(tmpdir)
            bpe_dir = tmp / "schemas" / "bpe"
            bpe_dir.mkdir(parents=True)
            input_path = tmp / "tests/data/instances/package/one.json"
            input_path.parent.mkdir(parents=True)
            input_path.write_text("{}", encoding="utf-8")
            merge_path = bpe_dir / "package-tct-bpe-500.json"
            merge_path.write_text(
                json.dumps(
                    {
                        "schema": "package",
                        "base_vocab_size": 257,
                        "metadata": {
                            "trained_on_instances": 3,
                            "max_merges": 1,
                            "dict_size": 0,
                        },
                        "merges": [
                            {"pair": [1, 2], "new_token": 257, "frequency": 9}
                        ],
                    }
                ),
                encoding="utf-8",
            )
            input_files = [file_record(tmp, "tests/data/instances/package/one.json")]
            merge_path.with_suffix(".provenance.json").write_text(
                json.dumps(
                    {
                        "command": ["tct-train-bpe", "--target-vocab", "500"],
                        "base_vocab_size": 257,
                        "instances_pattern": "tests/data/instances/package/*.json",
                        "input_files": input_files,
                        "input_corpus_fingerprint": corpus_fingerprint(input_files),
                        "target_vocab": 500,
                        "trained_on_instances": 3,
                        "generated_merges": 1,
                        "token_corpus": token_corpus(3),
                        "output_files": [
                            {
                                "path": "schemas/bpe/package-tct-bpe-500.json",
                                "bytes": merge_path.stat().st_size,
                                "sha256": "0" * 64,
                            },
                            {
                                "path": "schemas/bpe/other.json",
                                "bytes": merge_path.stat().st_size,
                                "sha256": sha256_file(merge_path),
                            },
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
                    "--bpe-dir",
                    "schemas/bpe",
                ],
                check=True,
                capture_output=True,
                text=True,
            )

            artifact = json.loads(completed.stdout)["artifacts"][0]
            self.assertEqual(artifact["provenance_status"], "incomplete_sidecar")
            self.assertIn(
                "output_file_record_does_not_match_artifact",
                artifact["provenance_gaps"],
            )

    def test_sampled_sidecar_requires_token_corpus_provenance(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            tmp = Path(tmpdir)
            bpe_dir = tmp / "schemas" / "bpe"
            bpe_dir.mkdir(parents=True)
            input_path = tmp / "tests/data/instances/package/one.json"
            input_path.parent.mkdir(parents=True)
            input_path.write_text("{}", encoding="utf-8")
            merge_path = bpe_dir / "package-tct-bpe-500.json"
            merge_path.write_text(
                json.dumps(
                    {
                        "schema": "package",
                        "base_vocab_size": 257,
                        "metadata": {"trained_on_instances": 1, "max_merges": 1},
                        "merges": [
                            {"pair": [1, 2], "new_token": 257, "frequency": 9}
                        ],
                    }
                ),
                encoding="utf-8",
            )
            input_files = [file_record(tmp, "tests/data/instances/package/one.json")]
            merge_sha256 = sha256_file(merge_path)
            merge_path.with_suffix(".provenance.json").write_text(
                json.dumps(
                    {
                        "command": ["tct-train-bpe", "--sample-size", "1"],
                        "base_vocab_size": 257,
                        "instances_pattern": "tests/data/instances/package/*.json",
                        "input_files": input_files,
                        "input_corpus_fingerprint": corpus_fingerprint(input_files),
                        "target_vocab": 500,
                        "sample_size": 1,
                        "trained_on_instances": 1,
                        "generated_merges": 1,
                        "output_files": [
                            {
                                "path": "schemas/bpe/package-tct-bpe-500.json",
                                "bytes": merge_path.stat().st_size,
                                "sha256": merge_sha256,
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
                    "--bpe-dir",
                    "schemas/bpe",
                ],
                check=True,
                capture_output=True,
                text=True,
            )

            artifact = json.loads(completed.stdout)["artifacts"][0]
            self.assertEqual(artifact["provenance_status"], "incomplete_sidecar")
            self.assertIn("missing_token_corpus", artifact["provenance_gaps"])

    def test_manifest_rejects_invalid_merge_table(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            tmp = Path(tmpdir)
            bpe_dir = tmp / "schemas" / "bpe"
            bpe_dir.mkdir(parents=True)
            (bpe_dir / "bad-tct-bpe-500.json").write_text(
                json.dumps({"schema": "bad", "base_vocab_size": 257, "merges": {}}),
                encoding="utf-8",
            )

            completed = subprocess.run(
                [
                    sys.executable,
                    str(SCRIPT),
                    "--repo-root",
                    str(tmp),
                    "--bpe-dir",
                    "schemas/bpe",
                ],
                capture_output=True,
                text=True,
            )

            self.assertNotEqual(completed.returncode, 0)
            self.assertIn("merges must be an array", completed.stderr)


if __name__ == "__main__":
    unittest.main()
