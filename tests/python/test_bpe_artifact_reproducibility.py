#!/usr/bin/env python3
"""Tests for the checked-in BPE artifact reproducibility gate."""

import hashlib
import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[2]
SCRIPT = REPO_ROOT / "scripts/testing/test-bpe-artifact-reproducibility.py"


def sha256_file(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


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


class TestBpeArtifactReproducibility(unittest.TestCase):
    def populate_repo(self, root: Path) -> Path:
        for relative in [
            "src/bpe",
            "src/bpe/trainer",
            "src/bin",
            "schemas/bpe",
            "schemas/popular",
            "tests/data/instances/package/nested",
        ]:
            (root / relative).mkdir(parents=True, exist_ok=True)

        for relative in [
            "src/bpe/trainer.rs",
            "src/bpe/trainer/training_state.rs",
            "src/bpe/mod.rs",
            "src/bin/tct-train-bpe.rs",
        ]:
            (root / relative).write_text(f"// {relative}\n", encoding="utf-8")

        (root / "schemas/popular/package.json").write_text(
            json.dumps({"type": "object"}),
            encoding="utf-8",
        )
        (root / "tests/data/instances/package/z.json").write_text(
            json.dumps({"z": 1}),
            encoding="utf-8",
        )
        (root / "tests/data/instances/package/nested/a.json").write_text(
            json.dumps({"a": 1}),
            encoding="utf-8",
        )

        artifact_path = root / "schemas/bpe/package-tct-bpe-2048.json"
        artifact_path.write_text(
            json.dumps(
                {
                    "schema": "package",
                    "base_vocab_size": 257,
                    "merges": [{"pair": [1, 2], "new_token": 257, "frequency": 3}],
                    "metadata": {
                        "trained_on_instances": 2,
                        "training_date": "2026-05-01",
                        "max_merges": 1,
                        "dict_size": None,
                    },
                },
                indent=2,
            ),
            encoding="utf-8",
        )
        sidecar_path = root / "schemas/bpe/package-tct-bpe-2048.provenance.json"
        input_files = [
            file_record(root, "schemas/popular/package.json"),
            file_record(root, "tests/data/instances/package/nested/a.json"),
            file_record(root, "tests/data/instances/package/z.json"),
        ]
        sidecar_path.write_text(
            json.dumps(
                {
                    "command": ["target/release/tct-train-bpe"],
                    "schema_name": "package",
                    "instances_pattern": "tests/data/instances/package/**/*.json",
                    "encoded_input": None,
                    "schema_file": {"path": "schemas/popular/package.json"},
                    "input_files": input_files,
                    "input_corpus_fingerprint": corpus_fingerprint(input_files),
                    "output_files": [
                        {
                            "path": "schemas/bpe/package-tct-bpe-2048.json",
                            "sha256": sha256_file(artifact_path),
                        }
                    ],
                    "base_vocab_size": 257,
                    "base_vocab_source": "schema",
                    "optimization_level": "O3",
                    "target_vocab": 2048,
                    "sample_size": None,
                    "trained_on_instances": 2,
                    "generated_merges": 1,
                    "token_corpus": token_corpus(2),
                },
                indent=2,
            ),
            encoding="utf-8",
        )
        return artifact_path

    def test_dry_run_reports_inputs_commands_and_hashes(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            tmp = Path(tmpdir)
            artifact_path = self.populate_repo(tmp)
            output_path = tmp / "target/report.json"

            completed = subprocess.run(
                [
                    sys.executable,
                    str(SCRIPT),
                    "--repo-root",
                    str(tmp),
                    "--artifact",
                    "schemas/bpe/package-tct-bpe-2048.json",
                    "--dry-run",
                    "--output",
                    str(output_path),
                ],
                check=True,
                capture_output=True,
                text=True,
            )

            report = json.loads(completed.stdout)
            self.assertEqual(json.loads(output_path.read_text(encoding="utf-8")), report)
            self.assertEqual(report["report_version"], 6)
            self.assertEqual(
                report["artifact_kind"],
                "tct_bpe_artifact_reproducibility",
            )
            self.assertEqual(report["tool"], "test-bpe-artifact-reproducibility")
            self.assertTrue(report["dry_run"])
            self.assertEqual(report["summary"], {"dry_run": 1})
            self.assertEqual(report["discovery"]["mode"], "explicit_artifacts")
            artifact = report["artifacts"][0]
            self.assertEqual(artifact["artifact"], "schemas/bpe/package-tct-bpe-2048.json")
            self.assertEqual(artifact["instance_file_count"], 2)
            self.assertEqual(
                [item["path"] for item in artifact["input_files"]],
                [
                    "schemas/popular/package.json",
                    "tests/data/instances/package/nested/a.json",
                    "tests/data/instances/package/z.json",
                ],
            )
            self.assertEqual(len(artifact["input_corpus_fingerprint"]), 64)
            self.assertEqual(
                artifact["input_corpus_fingerprint"],
                artifact["provenance_input_corpus_fingerprint"],
            )
            self.assertTrue(artifact["matches_provenance_input_files"])
            self.assertTrue(artifact["matches_provenance_input_corpus_fingerprint"])
            self.assertEqual(
                artifact["checked_in_merge_table"]["sha256"],
                sha256_file(artifact_path),
            )
            self.assertTrue(artifact["checked_in_merge_table"]["matches_provenance_output"])
            self.assertIn("--bin", artifact["generated_code"]["command"])
            self.assertIn("tct-train-bpe", artifact["candidate_training"]["command"])
            self.assertEqual(
                artifact["trainer_algorithm_identity"]["name"],
                "tct::bpe::trainer::train_bpe_incremental_with_options",
            )
            self.assertEqual(len(artifact["trainer_algorithm_identity"]["source_fingerprint"]), 64)

    def test_default_discovery_reports_missing_sidecars_without_failing(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            tmp = Path(tmpdir)
            self.populate_repo(tmp)
            legacy_path = tmp / "schemas/bpe/legacy-tct-bpe-500.json"
            legacy_path.write_text(
                json.dumps(
                    {
                        "schema": "legacy",
                        "base_vocab_size": 257,
                        "merges": [],
                        "metadata": {"dict_size": None},
                    }
                ),
                encoding="utf-8",
            )
            historical_path = tmp / "schemas/bpe/package-tct-bpe-5k.json"
            historical_path.write_text(
                json.dumps(
                    {
                        "schema": "package",
                        "base_vocab_size": 257,
                        "merges": [{"pair": [1, 2], "new_token": 257, "frequency": 3}],
                        "metadata": {
                            "trained_on_instances": 2,
                            "training_date": "2026-05-01",
                            "max_merges": 1,
                            "dict_size": None,
                        },
                    }
                ),
                encoding="utf-8",
            )
            historical_sidecar = json.loads(
                (tmp / "schemas/bpe/package-tct-bpe-2048.provenance.json").read_text(
                    encoding="utf-8"
                )
            )
            historical_sidecar["target_vocab"] = 5000
            historical_sidecar["output_files"] = [
                {
                    "path": "schemas/bpe/package-tct-bpe-5k.json",
                    "sha256": sha256_file(historical_path),
                }
            ]
            (tmp / "schemas/bpe/package-tct-bpe-5k.provenance.json").write_text(
                json.dumps(historical_sidecar),
                encoding="utf-8",
            )
            unclassified_path = tmp / "schemas/bpe/package-tct-bpe-unclassified.json"
            unclassified_path.write_text(
                json.dumps(
                    {
                        "schema": "package",
                        "base_vocab_size": 257,
                        "merges": [{"pair": [1, 2], "new_token": 257, "frequency": 3}],
                        "metadata": {
                            "trained_on_instances": 2,
                            "training_date": "2026-05-01",
                            "max_merges": 1,
                            "dict_size": None,
                        },
                    }
                ),
                encoding="utf-8",
            )
            unclassified_sidecar = json.loads(
                (tmp / "schemas/bpe/package-tct-bpe-2048.provenance.json").read_text(
                    encoding="utf-8"
                )
            )
            del unclassified_sidecar["target_vocab"]
            unclassified_sidecar["output_files"] = [
                {
                    "path": "schemas/bpe/package-tct-bpe-unclassified.json",
                    "sha256": sha256_file(unclassified_path),
                }
            ]
            (
                tmp / "schemas/bpe/package-tct-bpe-unclassified.provenance.json"
            ).write_text(
                json.dumps(unclassified_sidecar),
                encoding="utf-8",
            )

            completed = subprocess.run(
                [
                    sys.executable,
                    str(SCRIPT),
                    "--repo-root",
                    str(tmp),
                    "--dry-run",
                ],
                check=True,
                capture_output=True,
                text=True,
            )

            report = json.loads(completed.stdout)
            self.assertEqual(report["summary"], {"dry_run": 1})
            self.assertEqual(report["artifact_count"], 1)
            self.assertEqual(report["discovery"]["mode"], "policy_scoped_default")
            self.assertEqual(report["discovery"]["checked_in_artifact_count"], 4)
            self.assertEqual(report["discovery"]["selected_artifact_count"], 1)
            self.assertEqual(report["discovery"]["provenance_backed_artifact_count"], 3)
            self.assertEqual(report["discovery"]["missing_provenance_sidecar_count"], 1)
            self.assertEqual(
                report["discovery"]["default_queue_policy_counts"],
                {
                    "current_single_schema_candidate": 1,
                    "historical_excluded_by_vocab_policy": 1,
                    "unclassified_explicit_candidate": 1,
                },
            )
            self.assertEqual(
                report["discovery"]["skipped_missing_provenance_sidecars"],
                ["schemas/bpe/legacy-tct-bpe-500.json"],
            )
            self.assertEqual(
                report["discovery"]["skipped_default_queue_artifacts"],
                [
                    {
                        "path": "schemas/bpe/package-tct-bpe-5k.json",
                        "queue_scope": "historical_excluded_by_vocab_policy",
                        "reason": (
                            "requested target vocabulary is larger than 4096 "
                            "and stays historical unless a downstream profile reopens it"
                        ),
                    },
                    {
                        "path": "schemas/bpe/package-tct-bpe-unclassified.json",
                        "queue_scope": "unclassified_explicit_candidate",
                        "reason": (
                            "sidecar target_vocab is missing or non-integer "
                            "and requires explicit artifact selection"
                        ),
                    },
                ],
            )
            self.assertEqual(
                report["discovery"]["provenance_backed_artifacts"],
                [
                    "schemas/bpe/package-tct-bpe-2048.json",
                    "schemas/bpe/package-tct-bpe-5k.json",
                    "schemas/bpe/package-tct-bpe-unclassified.json",
                ],
            )
            self.assertEqual(
                report["discovery"]["selected_artifacts"],
                ["schemas/bpe/package-tct-bpe-2048.json"],
            )

    def test_missing_instance_files_fail_preflight(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            tmp = Path(tmpdir)
            self.populate_repo(tmp)
            sidecar_path = tmp / "schemas/bpe/package-tct-bpe-2048.provenance.json"
            sidecar = json.loads(sidecar_path.read_text(encoding="utf-8"))
            sidecar["instances_pattern"] = "tests/data/instances/package/missing/*.json"
            sidecar_path.write_text(json.dumps(sidecar), encoding="utf-8")

            completed = subprocess.run(
                [
                    sys.executable,
                    str(SCRIPT),
                    "--repo-root",
                    str(tmp),
                    "--artifact",
                    "schemas/bpe/package-tct-bpe-2048.json",
                    "--dry-run",
                ],
                capture_output=True,
                text=True,
            )

            self.assertNotEqual(completed.returncode, 0)
            report = json.loads(completed.stdout)
            self.assertEqual(report["summary"], {"failed": 1})
            self.assertIn("instance_files", report["artifacts"][0]["gaps"])

    def test_stale_provenance_output_hash_fails_preflight(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            tmp = Path(tmpdir)
            self.populate_repo(tmp)
            sidecar_path = tmp / "schemas/bpe/package-tct-bpe-2048.provenance.json"
            sidecar = json.loads(sidecar_path.read_text(encoding="utf-8"))
            sidecar["output_files"][0]["sha256"] = "0" * 64
            sidecar_path.write_text(json.dumps(sidecar), encoding="utf-8")

            completed = subprocess.run(
                [
                    sys.executable,
                    str(SCRIPT),
                    "--repo-root",
                    str(tmp),
                    "--artifact",
                    "schemas/bpe/package-tct-bpe-2048.json",
                    "--dry-run",
                ],
                capture_output=True,
                text=True,
            )

            self.assertNotEqual(completed.returncode, 0)
            report = json.loads(completed.stdout)
            self.assertEqual(report["summary"], {"failed": 1})
            artifact = report["artifacts"][0]
            self.assertFalse(
                artifact["checked_in_merge_table"]["matches_provenance_output"]
            )
            self.assertFalse(
                artifact["checked_in_merge_table"]["matches_provenance_output_hash"]
            )
            self.assertTrue(
                artifact["checked_in_merge_table"]["matches_provenance_output_path"]
            )
            self.assertIn("provenance_output_hash", artifact["gaps"])

    def test_stale_provenance_output_path_fails_preflight(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            tmp = Path(tmpdir)
            self.populate_repo(tmp)
            sidecar_path = tmp / "schemas/bpe/package-tct-bpe-2048.provenance.json"
            sidecar = json.loads(sidecar_path.read_text(encoding="utf-8"))
            sidecar["output_files"][0]["path"] = "schemas/bpe/other.json"
            sidecar_path.write_text(json.dumps(sidecar), encoding="utf-8")

            completed = subprocess.run(
                [
                    sys.executable,
                    str(SCRIPT),
                    "--repo-root",
                    str(tmp),
                    "--artifact",
                    "schemas/bpe/package-tct-bpe-2048.json",
                    "--dry-run",
                ],
                capture_output=True,
                text=True,
            )

            self.assertNotEqual(completed.returncode, 0)
            report = json.loads(completed.stdout)
            self.assertEqual(report["summary"], {"failed": 1})
            artifact = report["artifacts"][0]
            self.assertFalse(
                artifact["checked_in_merge_table"]["matches_provenance_output"]
            )
            self.assertTrue(
                artifact["checked_in_merge_table"]["matches_provenance_output_hash"]
            )
            self.assertFalse(
                artifact["checked_in_merge_table"]["matches_provenance_output_path"]
            )
            self.assertIn("provenance_output_path", artifact["gaps"])

    def test_input_corpus_drift_fails_preflight(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            tmp = Path(tmpdir)
            self.populate_repo(tmp)
            (tmp / "tests/data/instances/package/z.json").write_text(
                json.dumps({"z": 2}),
                encoding="utf-8",
            )

            completed = subprocess.run(
                [
                    sys.executable,
                    str(SCRIPT),
                    "--repo-root",
                    str(tmp),
                    "--artifact",
                    "schemas/bpe/package-tct-bpe-2048.json",
                    "--dry-run",
                ],
                capture_output=True,
                text=True,
            )

            self.assertNotEqual(completed.returncode, 0)
            report = json.loads(completed.stdout)
            self.assertEqual(report["summary"], {"failed": 1})
            artifact = report["artifacts"][0]
            self.assertFalse(artifact["matches_provenance_input_files"])
            self.assertFalse(
                artifact["matches_provenance_input_corpus_fingerprint"]
            )
            self.assertIn("input_files", artifact["gaps"])
            self.assertIn("input_corpus_fingerprint", artifact["gaps"])

    def test_sidecar_without_token_corpus_fails_preflight(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            tmp = Path(tmpdir)
            self.populate_repo(tmp)
            sidecar_path = tmp / "schemas/bpe/package-tct-bpe-2048.provenance.json"
            sidecar = json.loads(sidecar_path.read_text(encoding="utf-8"))
            del sidecar["token_corpus"]
            sidecar_path.write_text(json.dumps(sidecar), encoding="utf-8")

            completed = subprocess.run(
                [
                    sys.executable,
                    str(SCRIPT),
                    "--repo-root",
                    str(tmp),
                    "--artifact",
                    "schemas/bpe/package-tct-bpe-2048.json",
                    "--dry-run",
                ],
                capture_output=True,
                text=True,
            )

            self.assertNotEqual(completed.returncode, 0)
            report = json.loads(completed.stdout)
            artifact = report["artifacts"][0]
            self.assertIn("provenance_token_corpus", artifact["gaps"])

    def test_token_corpus_count_mismatch_fails_preflight(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            tmp = Path(tmpdir)
            self.populate_repo(tmp)
            sidecar_path = tmp / "schemas/bpe/package-tct-bpe-2048.provenance.json"
            sidecar = json.loads(sidecar_path.read_text(encoding="utf-8"))
            sidecar["sample_size"] = 1
            sidecar["token_corpus"] = {
                "source": "instance_encoding",
                "sequence_count": 1,
                "sequence_fingerprint": "a" * 64,
                "sample_size": 1,
                "sample_seed": 42,
                "cache_path": None,
                "cache_hit": False,
                "cache_written": False,
                "parallel_early_stop": True,
                "truncated_to_sample_size": False,
                "selected_instance_files": sidecar["input_files"][1:],
                "accepted_instance_files": sidecar["input_files"][1:2],
                "rejected_instance_files": [],
            }
            sidecar_path.write_text(json.dumps(sidecar), encoding="utf-8")

            completed = subprocess.run(
                [
                    sys.executable,
                    str(SCRIPT),
                    "--repo-root",
                    str(tmp),
                    "--artifact",
                    "schemas/bpe/package-tct-bpe-2048.json",
                    "--dry-run",
                ],
                capture_output=True,
                text=True,
            )

            self.assertNotEqual(completed.returncode, 0)
            report = json.loads(completed.stdout)
            artifact = report["artifacts"][0]
            self.assertIn("token_corpus_sequence_count", artifact["gaps"])




class TestPublicArtifactsMode(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        import importlib.util
        spec = importlib.util.spec_from_file_location("public_repro_tests", SCRIPT)
        cls.module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(cls.module)

    def args(self, **changes):
        from argparse import Namespace
        values = dict(dry_run=False, artifact=[], bpe_dir="schemas/bpe", cargo_profile="dev",
                      work_dir="target/repro", output="target/report.json", timeout_seconds=5,
                      cargo_home=None, cargo_target_dir=None)
        values.update(changes)
        return Namespace(**values)

    def test_environment_isolates_source_and_build_overrides(self):
        from unittest.mock import patch
        poisoned = {"TCT_SOURCE_DIR": "/other/source", "TCT_GENERATED_CARGO_TARGET_DIR": "/other/cache",
                    "CARGO_ENCODED_RUSTFLAGS": "--cfg=other", "RUSTFLAGS": "--cfg=other",
                    "RUSTC_WRAPPER": "/other/wrapper", "RUSTUP_TOOLCHAIN": "other",
                    "CARGO_HOME": "/other/cargo", "MAX_BASE_VOCAB": "512", "TCT_DEBUG": "1"}
        with patch.dict("os.environ", poisoned):
            env = self.module.public_environment(Path("/current/source"), "/explicit/cache", "/explicit/target")
        self.assertEqual(env["TCT_SOURCE_DIR"], "/current/source")
        self.assertEqual(env["CARGO_TARGET_DIR"], "/explicit/target")
        self.assertEqual(env["CARGO_HOME"], "/explicit/cache")
        self.assertEqual(env["CARGO_NET_OFFLINE"], "true")
        self.assertEqual(env["TCT_GENERATED_CARGO_TARGET_DIR"], "/explicit/target/generated")
        for key in ("CARGO_ENCODED_RUSTFLAGS", "RUSTFLAGS", "RUSTC_WRAPPER", "RUSTUP_TOOLCHAIN", "MAX_BASE_VOCAB", "TCT_DEBUG"):
            self.assertNotIn(key, env)

    def test_public_cache_paths_reject_source_files_and_symlink_aliases(self):
        from unittest.mock import patch
        from test_public_bpe_source_manifest import public_fixture
        for option in ("cargo_home", "cargo_target_dir"):
            for kind in ("source", "file", "file_parent", "source_alias", "generated_alias"):
                if kind == "generated_alias" and option != "cargo_target_dir":
                    continue
                with self.subTest(option=option, kind=kind), tempfile.TemporaryDirectory() as tmp:
                    outer = Path(tmp)
                    root = outer / "source"
                    root.mkdir()
                    public_fixture(root)
                    (root / "src").mkdir()
                    cache = root / "src/build-cache"
                    if kind in ("file", "file_parent"):
                        cache = outer / "cache-file"
                        cache.write_text("not a directory")
                        if kind == "file_parent":
                            cache /= "child"
                    elif kind == "source_alias":
                        cache = outer / "alias"
                        cache.symlink_to(root / "src", target_is_directory=True)
                    elif kind == "generated_alias":
                        cache = outer / "cache"
                        cache.mkdir()
                        (cache / "generated").symlink_to(root / "src", target_is_directory=True)
                    with patch.object(self.module.subprocess, "run") as run, patch.object(self.module, "artifact_report") as artifact:
                        report, ok = self.module.public_report(root, self.args(**{option: str(cache)}))
                    self.assertFalse(ok)
                    self.assertIn("cache", report.get("error", ""))
                    run.assert_not_called()
                    artifact.assert_not_called()
                    self.assertFalse((root / "target/repro").exists())

    def test_public_cache_paths_accept_target_and_external_directories(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp) / "source"
            root.mkdir()
            for home, target in (("target/deps", "target/build"), (str(Path(tmp) / "deps"), str(Path(tmp) / "build"))):
                env = self.module.public_environment(root, home, target)
                self.assertEqual(Path(env["CARGO_HOME"]), (root / home).resolve())
                self.assertEqual(Path(env["CARGO_TARGET_DIR"]), (root / target).resolve())

    def test_git_metadata_environment_disables_local_hooks_and_lazy_fetch(self):
        env = self.module.public_environment(Path("/current/source"))
        self.assertEqual(env["GIT_NO_LAZY_FETCH"], "1")
        self.assertEqual(env["GIT_OPTIONAL_LOCKS"], "0")
        self.assertEqual(env["GIT_CONFIG_COUNT"], "3")
        config = {env[f"GIT_CONFIG_KEY_{i}"]: env[f"GIT_CONFIG_VALUE_{i}"] for i in range(3)}
        self.assertEqual(config, {"core.fsmonitor": "false", "core.hooksPath": "/dev/null", "log.showSignature": "false"})

    def test_dry_or_partial_run_never_qualifies(self):
        for args in (self.args(dry_run=True), self.args(artifact=["some.json"])):
            report, ok = self.module.public_report(REPO_ROOT, args)
            self.assertFalse(ok)
            self.assertFalse(report["qualified"])
            self.assertEqual(report["artifact_count"], 0)
            self.assertIn("error", report)

    def test_report_cannot_overwrite_source_on_preflight_failure(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            source = root / "Cargo.lock"
            source.write_text("original lockfile")
            result = subprocess.run([sys.executable, str(SCRIPT), "--public-artifacts", "--dry-run",
                                     "--repo-root", str(root), "--output", "Cargo.lock"], capture_output=True)
            self.assertNotEqual(result.returncode, 0)
            self.assertEqual(source.read_text(), "original lockfile")

    def test_all_six_required_and_source_rechecked(self):
        from types import SimpleNamespace
        from unittest.mock import patch
        from test_public_bpe_source_manifest import public_fixture
        for failure in (None, "raw_hash_matches", "token_corpus_matches", "source_drift"):
            with self.subTest(failure=failure), tempfile.TemporaryDirectory() as tmp:
                root = Path(tmp)
                public_fixture(root)
                selected = []
                def artifact(repo, path, work, profile, timeout, dry, public_env):
                    selected.append(path.relative_to(repo).as_posix())
                    self.assertEqual(public_env["TMPDIR"], str(work / "tmp"))
                    self.assertTrue((work / "tmp").is_dir())
                    row = {"artifact": selected[-1], "status": "passed", "comparison": {
                        "raw_hash_matches": True, "token_corpus_matches": True}}
                    if failure in row["comparison"]:
                        row["comparison"][failure] = False
                    if failure == "source_drift":
                        (root / "injected.rs").write_text("changed during execution")
                    return row, True
                with patch.dict("os.environ", {"TMPDIR": str(root / "src/bad-temp")}), patch.object(self.module, "artifact_report", side_effect=artifact), patch.object(
                        self.module.subprocess, "run", return_value=SimpleNamespace(stdout="version", returncode=0)):
                    report, ok = self.module.public_report(root, self.args())
                self.assertEqual(set(selected), set(self.module.public_source.EXPECTED_OUTPUTS))
                self.assertEqual(report["artifact_count"], 6)
                self.assertEqual(ok, failure is None)
                if failure is None:
                    self.assertEqual(report["source_snapshot_before"], report["source_snapshot_after"])
                    self.assertEqual(set(report["tool_versions"]), {"python", "cargo", "rustc", "git"})

    def test_actual_comparisons_and_locked_command_environment(self):
        from unittest.mock import patch
        from test_public_bpe_source_manifest import public_fixture
        for failure in (None, "table_bytes", "token_corpus"):
            with self.subTest(failure=failure), tempfile.TemporaryDirectory() as tmp:
                root = Path(tmp)
                public_fixture(root)
                output = root / "schemas/bpe/package-tct-bpe-500.json"
                env = self.module.public_environment(root)
                def execute(repo, command, stdout, stderr, timeout, env=None):
                    self.assertIn("--locked", command)
                    self.assertEqual(env["TCT_SOURCE_DIR"], str(root))
                    generated = Path(command[command.index("--output") + 1])
                    if "tct-train-bpe" not in command:
                        generated.write_text("// generated")
                    else:
                        generated.write_bytes(output.read_bytes() + (b"\n" if failure == "table_bytes" else b""))
                        sidecar = json.loads(output.with_suffix(".provenance.json").read_text())
                        if failure == "token_corpus":
                            sidecar["token_corpus"]["sequence_fingerprint"] = "b" * 64
                        generated.with_suffix(".provenance.json").write_text(json.dumps(sidecar))
                    return {"success": True}
                with patch.object(self.module, "command_record", side_effect=execute):
                    row, ok = self.module.artifact_report(root, output, root / "target/run", "dev", 5, False, public_env=env)
                self.assertEqual(ok, failure is None)
                self.assertEqual(row["comparison"]["raw_hash_matches"], failure != "table_bytes")
                self.assertEqual(row["comparison"]["token_corpus_matches"], failure != "token_corpus")


if __name__ == "__main__":
    unittest.main()
