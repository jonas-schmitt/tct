#!/usr/bin/env python3
"""Retrain and compare checked-in BPE artifacts with provenance."""

from __future__ import annotations

import argparse
import glob
import hashlib
import json
import os
import re
import subprocess
import sys
import time
import tempfile
from datetime import datetime, timezone
from pathlib import Path
from typing import Any


sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "analysis"))
import public_bpe_source_manifest as public_source


REPORT_VERSION = 6
ARTIFACT_KIND = "tct_bpe_artifact_reproducibility"
TOOL_NAME = "test-bpe-artifact-reproducibility"
DEFAULT_REPORT = "target/tct-bpe-artifact-reproducibility-report.json"
DEFAULT_TARGET_VOCAB_CEILING = 2048
EXPLICIT_TARGET_VOCAB_CEILING = 4096
DEFAULT_QUEUE_SELECTED_SCOPES = {
    "current_single_schema_candidate",
}
TRAINER_ALGORITHM = {
    "name": "tct::bpe::trainer::train_bpe_incremental_with_options",
    "identity": "tct-bpe-trainer-incremental-frequency-lexicographic-v1",
    "tie_break": "highest_frequency_then_lexicographically_smallest_pair",
}
TRAINER_SOURCE_FILES = (
    "src/bpe/trainer.rs",
    "src/bpe/trainer/training_state.rs",
    "src/bpe/mod.rs",
    "src/bin/tct-train-bpe.rs",
)


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def read_json(path: Path) -> Any:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except json.JSONDecodeError as error:
        raise ValueError(f"{path}: invalid JSON: {error}") from error


def repo_relative(repo_root: Path, path: Path) -> str:
    try:
        return path.resolve().relative_to(repo_root.resolve()).as_posix()
    except ValueError:
        return path.as_posix()


def resolve_repo_path(repo_root: Path, value: str) -> Path:
    path = Path(value)
    if path.is_absolute():
        return path
    return repo_root / path


def file_record(repo_root: Path, path: Path) -> dict[str, Any]:
    return {
        "path": repo_relative(repo_root, path),
        "bytes": path.stat().st_size,
        "sha256": sha256_file(path),
    }


def optional_file_record(repo_root: Path, value: str | None) -> dict[str, Any] | None:
    if not value:
        return None
    path = resolve_repo_path(repo_root, value)
    if not path.is_file():
        return {
            "path": repo_relative(repo_root, path),
            "missing": True,
        }
    return file_record(repo_root, path)


def git_value(repo_root: Path, *args: str) -> str | None:
    try:
        completed = subprocess.run(
            ["git", "-C", str(repo_root), *args],
            check=True,
            capture_output=True,
            text=True,
        )
    except (OSError, subprocess.CalledProcessError):
        return None
    return completed.stdout.strip()


def git_metadata(repo_root: Path) -> dict[str, Any]:
    status = git_value(repo_root, "status", "--porcelain")
    return {
        "head": git_value(repo_root, "rev-parse", "HEAD"),
        "dirty": None if status is None else bool(status),
        "status_porcelain": status,
    }


def discover_bpe_artifact_paths(repo_root: Path, bpe_dir: str) -> list[Path]:
    root = resolve_repo_path(repo_root, bpe_dir)
    if not root.is_dir():
        return []
    return sorted(
        path
        for path in root.glob("*.json")
        if not path.name.endswith(".provenance.json")
    )


def infer_encoding_family(path: Path) -> str | None:
    stem = path.name.removesuffix(".json")
    if "-tct-bpe-" in stem:
        return "tct"
    if "-utf8-bpe-" in stem or "-utf8-base-" in stem:
        return "utf8"
    return None


def is_power_of_two(value: int) -> bool:
    return value > 0 and (value & (value - 1)) == 0


def default_queue_policy(repo_root: Path, path: Path) -> dict[str, str]:
    encoding_family = infer_encoding_family(path)
    if encoding_family != "tct":
        return {
            "queue_scope": "comparison_artifact_not_local_queue",
            "reason": (
                "non-TCT BPE artifacts are comparison evidence, "
                "not the local TCT config queue"
            ),
        }

    provenance_path = path.with_suffix(".provenance.json")
    try:
        provenance = read_json(provenance_path)
    except (OSError, ValueError) as error:
        return {
            "queue_scope": "unclassified_explicit_candidate",
            "reason": (
                "sidecar could not be classified and requires explicit "
                f"artifact selection: {error}"
            ),
        }
    if not isinstance(provenance, dict):
        return {
            "queue_scope": "unclassified_explicit_candidate",
            "reason": (
                "sidecar is not a JSON object and requires explicit "
                "artifact selection"
            ),
        }

    target_vocab = provenance.get("target_vocab")
    if not isinstance(target_vocab, int):
        return {
            "queue_scope": "unclassified_explicit_candidate",
            "reason": (
                "sidecar target_vocab is missing or non-integer "
                "and requires explicit artifact selection"
            ),
        }
    if target_vocab > EXPLICIT_TARGET_VOCAB_CEILING:
        return {
            "queue_scope": "historical_excluded_by_vocab_policy",
            "reason": (
                "requested target vocabulary is larger than 4096 "
                "and stays historical unless a downstream profile reopens it"
            ),
        }
    if not is_power_of_two(target_vocab):
        return {
            "queue_scope": "historical_non_power_of_two_vocab_policy",
            "reason": (
                "requested total target vocabulary is not a power of two "
                "and stays historical unless a downstream profile reopens it"
            ),
        }
    if target_vocab <= DEFAULT_TARGET_VOCAB_CEILING:
        return {
            "queue_scope": "current_single_schema_candidate",
            "reason": (
                "requested power-of-two target vocabulary is within the "
                "conservative single-schema ceiling"
            ),
        }
    return {
        "queue_scope": "explicit_4096_candidate",
        "reason": (
            "requested 4096 target vocabulary requires explicit "
            "schema-specific or shared-corpus justification"
        ),
    }


def discover_artifacts(repo_root: Path, artifacts: list[str], bpe_dir: str) -> list[Path]:
    if artifacts:
        return [resolve_repo_path(repo_root, artifact) for artifact in artifacts]

    return [
        path
        for path in discover_bpe_artifact_paths(repo_root, bpe_dir)
        if path.with_suffix(".provenance.json").is_file()
        and (
            default_queue_policy(repo_root, path)["queue_scope"]
            in DEFAULT_QUEUE_SELECTED_SCOPES
        )
    ]


def discovery_report(
    repo_root: Path,
    bpe_dir: str,
    selected_artifacts: list[Path],
    explicit_artifacts: bool,
) -> dict[str, Any]:
    discovered = discover_bpe_artifact_paths(repo_root, bpe_dir)
    selected = {path.resolve() for path in selected_artifacts}
    missing_sidecar = [
        repo_relative(repo_root, path)
        for path in discovered
        if not path.with_suffix(".provenance.json").is_file()
    ]
    provenance_backed = [
        repo_relative(repo_root, path)
        for path in discovered
        if path.with_suffix(".provenance.json").is_file()
    ]
    default_queue_policy_counts: dict[str, int] = {}
    skipped_default_queue_artifacts = []
    for path in discovered:
        if not path.with_suffix(".provenance.json").is_file():
            continue
        policy = default_queue_policy(repo_root, path)
        queue_scope = policy["queue_scope"]
        default_queue_policy_counts[queue_scope] = (
            default_queue_policy_counts.get(queue_scope, 0) + 1
        )
        if not explicit_artifacts and path.resolve() not in selected:
            skipped_default_queue_artifacts.append(
                {
                    "path": repo_relative(repo_root, path),
                    "queue_scope": queue_scope,
                    "reason": policy["reason"],
                }
            )
    return {
        "bpe_dir": bpe_dir,
        "mode": "explicit_artifacts" if explicit_artifacts else "policy_scoped_default",
        "checked_in_artifact_count": len(discovered),
        "selected_artifact_count": len(selected_artifacts),
        "provenance_backed_artifact_count": len(provenance_backed),
        "missing_provenance_sidecar_count": len(missing_sidecar),
        "default_queue_policy_counts": default_queue_policy_counts,
        "provenance_backed_artifacts": provenance_backed,
        "skipped_missing_provenance_sidecars": missing_sidecar,
        "skipped_default_queue_artifacts": skipped_default_queue_artifacts,
        "selected_artifacts": [
            repo_relative(repo_root, path)
            for path in discovered
            if path.resolve() in selected
        ],
    }


def expand_instance_pattern(repo_root: Path, pattern: str | None) -> list[Path]:
    if not pattern:
        return []
    search = pattern if Path(pattern).is_absolute() else str(repo_root / pattern)
    return sorted(Path(path) for path in glob.glob(search, recursive=True) if Path(path).is_file())


def corpus_fingerprint(records: list[dict[str, Any]]) -> str:
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


def stable_merge_table_hash(path: Path) -> str:
    data = read_json(path)
    if isinstance(data, dict) and isinstance(data.get("metadata"), dict):
        data = dict(data)
        metadata = dict(data["metadata"])
        metadata["training_date"] = "<normalized>"
        data["metadata"] = metadata
    canonical = json.dumps(data, sort_keys=True, separators=(",", ":")).encode("utf-8")
    return sha256_bytes(canonical)


def trainer_identity(repo_root: Path) -> dict[str, Any]:
    sources = []
    for relative in TRAINER_SOURCE_FILES:
        path = repo_root / relative
        if path.is_file():
            sources.append(file_record(repo_root, path))
        else:
            sources.append({"path": relative, "missing": True})
    return {
        **TRAINER_ALGORITHM,
        "source_files": sources,
        "source_fingerprint": corpus_fingerprint(
            [source for source in sources if not source.get("missing")]
        ),
    }


def cargo_run_prefix(profile: str, binary: str) -> list[str]:
    command = ["cargo", "run", "--quiet"]
    if profile == "release":
        command.append("--release")
    command.extend(["--bin", binary, "--"])
    return command


def compile_command(
    schema_path: str,
    schema_name: str,
    optimization: str | None,
    output_path: Path,
    profile: str,
) -> list[str]:
    command = cargo_run_prefix(profile, "tct")
    command.extend([schema_path, "--output", str(output_path), "--name", schema_name, "--quiet"])
    if optimization:
        command.extend(["--optimization", optimization])
    return command


def training_command(
    provenance: dict[str, Any],
    output_path: Path,
    profile: str,
) -> list[str]:
    command = cargo_run_prefix(profile, "tct-train-bpe")

    schema_file = provenance.get("schema_file")
    if isinstance(schema_file, dict) and schema_file.get("path"):
        command.extend(["--schema", str(schema_file["path"])])

    if provenance.get("instances_pattern"):
        command.extend(["--instances", str(provenance["instances_pattern"])])
    if provenance.get("encoded_input"):
        command.extend(["--encoded", str(provenance["encoded_input"])])

    if not (provenance.get("schema_file") or provenance.get("base_vocab_source") == "schema"):
        command.extend(["--base-vocab", str(provenance["base_vocab_size"])])

    if provenance.get("target_vocab") is not None:
        command.extend(["--target-vocab", str(provenance["target_vocab"])])
    if provenance.get("target_avg_length") is not None:
        command.extend(["--target-avg-length", str(provenance["target_avg_length"])])
    if provenance.get("max_merges_deprecated") is not None:
        command.extend(["--max-merges", str(provenance["max_merges_deprecated"])])
    if provenance.get("sample_size") is not None:
        command.extend(["--sample-size", str(provenance["sample_size"])])
    if provenance.get("cache_path"):
        command.extend(["--cache", str(provenance["cache_path"])])

    command.extend(["--output", str(output_path)])
    command.extend(["--name", str(provenance.get("schema_name", "schema"))])
    if provenance.get("optimization_level"):
        command.extend(["--optimization", str(provenance["optimization_level"])])
    return command


def command_record(
    repo_root: Path,
    command: list[str],
    stdout_path: Path,
    stderr_path: Path,
    timeout_seconds: int,
    env: dict[str, str] | None = None,
) -> dict[str, Any]:
    start = time.perf_counter()
    status = "completed"
    exit_code: int | None
    try:
        completed = subprocess.run(
            command,
            cwd=repo_root,
            capture_output=True,
            text=True,
            timeout=timeout_seconds,
            env=env,
        )
        stdout = completed.stdout
        stderr = completed.stderr
        exit_code = completed.returncode
    except subprocess.TimeoutExpired as error:
        status = "timeout"
        exit_code = None
        stdout = error.stdout if isinstance(error.stdout, str) else (error.stdout or b"").decode()
        stderr = error.stderr if isinstance(error.stderr, str) else (error.stderr or b"").decode()

    stdout_path.write_text(stdout, encoding="utf-8")
    stderr_path.write_text(stderr, encoding="utf-8")
    return {
        "command": command,
        "status": status,
        "exit_code": exit_code,
        "success": status == "completed" and exit_code == 0,
        "wall_time_seconds": round(time.perf_counter() - start, 3),
        "stdout": repo_relative(repo_root, stdout_path),
        "stderr": repo_relative(repo_root, stderr_path),
    }


def safe_stem(path: Path) -> str:
    return re.sub(r"[^A-Za-z0-9_.-]+", "-", path.stem)


def artifact_report(
    repo_root: Path,
    artifact_path: Path,
    work_dir: Path,
    profile: str,
    timeout_seconds: int,
    dry_run: bool,
    public_env: dict[str, str] | None = None,
) -> tuple[dict[str, Any], bool]:
    record: dict[str, Any] = {
        "artifact": repo_relative(repo_root, artifact_path),
        "status": "preflight",
    }

    if not artifact_path.is_file():
        record["status"] = "failed"
        record["error"] = "artifact does not exist"
        return record, False

    try:
        artifact_json = read_json(artifact_path)
    except ValueError as error:
        record["status"] = "failed"
        record["error"] = str(error)
        return record, False

    provenance_path = artifact_path.with_suffix(".provenance.json")
    if not provenance_path.is_file():
        record["status"] = "failed"
        record["error"] = "missing provenance sidecar"
        return record, False

    try:
        provenance = read_json(provenance_path)
    except ValueError as error:
        record["status"] = "failed"
        record["error"] = str(error)
        return record, False
    if not isinstance(provenance, dict):
        record["status"] = "failed"
        record["error"] = "provenance sidecar must be a JSON object"
        return record, False

    instance_paths = expand_instance_pattern(repo_root, provenance.get("instances_pattern"))
    input_files = []
    schema_record = None
    schema_file = provenance.get("schema_file")
    if isinstance(schema_file, dict) and schema_file.get("path"):
        schema_record = optional_file_record(repo_root, str(schema_file["path"]))
        if schema_record and not schema_record.get("missing"):
            input_files.append(schema_record)
    for path in instance_paths:
        input_files.append(file_record(repo_root, path))
    encoded_record = optional_file_record(repo_root, provenance.get("encoded_input"))
    if encoded_record and not encoded_record.get("missing"):
        input_files.append(encoded_record)
    input_files = sorted(input_files, key=lambda item: item["path"])
    input_corpus_hash = corpus_fingerprint(input_files)

    output_files = provenance.get("output_files")
    expected_output = output_files[0] if isinstance(output_files, list) and output_files else None
    expected_input_files = provenance.get("input_files")
    expected_input_corpus_hash = provenance.get("input_corpus_fingerprint")
    expected_token_corpus = provenance.get("token_corpus")
    artifact_hash = sha256_file(artifact_path)
    artifact_stable_hash = stable_merge_table_hash(artifact_path)
    artifact_relative_path = repo_relative(repo_root, artifact_path)
    expected_output_path_matches = (
        isinstance(expected_output, dict)
        and expected_output.get("path") == artifact_relative_path
    )
    expected_output_hash_matches = (
        isinstance(expected_output, dict)
        and expected_output.get("sha256") == artifact_hash
    )
    training_date = None
    if isinstance(artifact_json, dict) and isinstance(artifact_json.get("metadata"), dict):
        training_date = artifact_json["metadata"].get("training_date")

    record.update(
        {
            "provenance": file_record(repo_root, provenance_path),
            "schema_name": provenance.get("schema_name"),
            "schema_file": schema_record,
            "instance_pattern": provenance.get("instances_pattern"),
            "instance_file_count": len(instance_paths),
            "input_files": input_files,
            "input_corpus_fingerprint": input_corpus_hash,
            "provenance_input_corpus_fingerprint": expected_input_corpus_hash,
            "matches_provenance_input_files": expected_input_files == input_files,
            "matches_provenance_input_corpus_fingerprint": (
                expected_input_corpus_hash == input_corpus_hash
            ),
            "provenance_token_corpus": expected_token_corpus,
            "trainer_algorithm_identity": trainer_identity(repo_root),
            "checked_in_merge_table": {
                "path": artifact_relative_path,
                "bytes": artifact_path.stat().st_size,
                "sha256": artifact_hash,
                "stable_sha256_without_training_date": artifact_stable_hash,
                "training_date": training_date,
                "matches_provenance_output": (
                    expected_output_path_matches and expected_output_hash_matches
                ),
                "matches_provenance_output_path": expected_output_path_matches,
                "matches_provenance_output_hash": expected_output_hash_matches,
            },
            "provenance_command": provenance.get("command"),
        }
    )

    gaps = []
    if not schema_record or schema_record.get("missing"):
        gaps.append("schema_file")
    if provenance.get("instances_pattern") and not instance_paths:
        gaps.append("instance_files")
    if not isinstance(expected_input_files, list):
        gaps.append("provenance_input_files")
    elif expected_input_files != input_files:
        gaps.append("input_files")
    if not isinstance(expected_input_corpus_hash, str):
        gaps.append("provenance_input_corpus_fingerprint")
    elif expected_input_corpus_hash != input_corpus_hash:
        gaps.append("input_corpus_fingerprint")
    if not isinstance(expected_output, dict):
        gaps.append("provenance_output_files")
    else:
        if not expected_output_path_matches:
            gaps.append("provenance_output_path")
        if not expected_output_hash_matches:
            gaps.append("provenance_output_hash")
    if provenance.get("checkpoints"):
        gaps.append("checkpoint_artifacts_not_supported")
    token_corpus_required = True
    if token_corpus_required and not isinstance(expected_token_corpus, dict):
        gaps.append("provenance_token_corpus")
    if isinstance(expected_token_corpus, dict):
        if not isinstance(expected_token_corpus.get("source"), str):
            gaps.append("token_corpus_source")
        if not isinstance(expected_token_corpus.get("sequence_fingerprint"), str):
            gaps.append("token_corpus_sequence_fingerprint")
        if expected_token_corpus.get("sequence_count") != provenance.get(
            "trained_on_instances"
        ):
            gaps.append("token_corpus_sequence_count")
        if provenance.get("sample_size") is not None:
            if expected_token_corpus.get("sample_seed") is None:
                gaps.append("token_corpus_sample_seed")
            if not isinstance(expected_token_corpus.get("accepted_instance_files"), list):
                gaps.append("token_corpus_accepted_instance_files")
            if not isinstance(expected_token_corpus.get("rejected_instance_files"), list):
                gaps.append("token_corpus_rejected_instance_files")
        if provenance.get("cache_path") and not (
            expected_token_corpus.get("cache_hit")
            or expected_token_corpus.get("cache_written")
        ):
            gaps.append("token_corpus_cache_identity")
    if gaps:
        record["status"] = "failed"
        record["error"] = "preflight gaps"
        record["gaps"] = gaps
        return record, False

    artifact_work_dir = work_dir / safe_stem(artifact_path)
    artifact_work_dir.mkdir(parents=True, exist_ok=True)
    generated_code_path = artifact_work_dir / "generated.rs"
    candidate_path = artifact_work_dir / artifact_path.name

    generated_command = compile_command(
        str(schema_file["path"]),
        str(provenance.get("schema_name", "schema")),
        provenance.get("optimization_level"),
        generated_code_path,
        profile,
    )
    train_command = training_command(provenance, candidate_path, profile)
    if public_env is not None:
        generated_command.insert(2, "--locked")
        train_command.insert(2, "--locked")
    record["generated_code"] = {
        "command": generated_command,
        "path": repo_relative(repo_root, generated_code_path),
    }
    record["candidate_training"] = {
        "command": train_command,
        "path": repo_relative(repo_root, candidate_path),
    }

    if dry_run:
        record["status"] = "dry_run"
        return record, True

    generated = command_record(
        repo_root,
        generated_command,
        artifact_work_dir / "generate-code.stdout",
        artifact_work_dir / "generate-code.stderr",
        timeout_seconds,
        env=public_env,
    )
    record["generated_code"]["run"] = generated
    if not generated["success"]:
        record["status"] = "failed"
        record["error"] = "generated code command failed"
        return record, False
    record["generated_code"]["bytes"] = generated_code_path.stat().st_size
    record["generated_code"]["sha256"] = sha256_file(generated_code_path)

    env = dict(public_env) if public_env is not None else os.environ.copy()
    if isinstance(training_date, str) and training_date:
        env["TCT_BPE_TRAINING_DATE"] = training_date

    trained = command_record(
        repo_root,
        train_command,
        artifact_work_dir / "train.stdout",
        artifact_work_dir / "train.stderr",
        timeout_seconds,
        env=env,
    )
    record["candidate_training"]["run"] = trained
    if not trained["success"]:
        record["status"] = "failed"
        record["error"] = "training command failed"
        return record, False
    candidate_hash = sha256_file(candidate_path)
    candidate_stable_hash = stable_merge_table_hash(candidate_path)
    candidate_provenance_path = candidate_path.with_suffix(".provenance.json")
    candidate_token_corpus = None
    token_corpus_matches = expected_token_corpus is None
    if candidate_provenance_path.is_file():
        candidate_provenance = read_json(candidate_provenance_path)
        if isinstance(candidate_provenance, dict):
            candidate_token_corpus = candidate_provenance.get("token_corpus")
            token_corpus_matches = candidate_token_corpus == expected_token_corpus
    record["candidate_training"]["bytes"] = candidate_path.stat().st_size
    record["candidate_training"]["sha256"] = candidate_hash
    record["candidate_training"]["stable_sha256_without_training_date"] = candidate_stable_hash
    record["candidate_training"]["provenance"] = (
        file_record(repo_root, candidate_provenance_path)
        if candidate_provenance_path.is_file()
        else {"path": repo_relative(repo_root, candidate_provenance_path), "missing": True}
    )
    record["candidate_training"]["token_corpus"] = candidate_token_corpus
    record["comparison"] = {
        "raw_hash_matches": candidate_hash == artifact_hash,
        "stable_hash_matches": candidate_stable_hash == artifact_stable_hash,
        "provenance_output_hash_matches_checked_in": record["checked_in_merge_table"][
            "matches_provenance_output"
        ],
        "token_corpus_matches": token_corpus_matches,
    }
    record["status"] = (
        "passed"
        if record["comparison"]["raw_hash_matches"]
        and record["comparison"]["token_corpus_matches"]
        else "mismatch"
    )
    return record, record["status"] == "passed"


def public_cache_directory(repo_root: Path, value: str) -> Path:
    path = resolve_repo_path(repo_root, value).resolve()
    public_source.require(not path.is_relative_to(repo_root) or path.is_relative_to(repo_root / "target"),
                          "public cache inside source must be under target/: " + str(path))
    for ancestor in (path, *path.parents):
        public_source.require(not ancestor.exists() or ancestor.is_dir(),
                              "public cache path has a non-directory component: " + str(ancestor))
    return path


def public_environment(repo_root: Path, cargo_home: str | None = None,
                       cargo_target_dir: str | None = None) -> dict[str, str]:
    blocked = ("TCT_", "CARGO_", "GIT_", "RUSTC", "RUSTFLAGS", "RUSTDOC", "RUSTUP_TOOLCHAIN")
    env = {key: value for key, value in os.environ.items()
           if not key.startswith(blocked) and key not in
           {"MAX_BASE_VOCAB", "MAX_INSTANCES", "BPE_MERGES", "ROUND_TRIP_BPE_PRESET"}}
    target = public_cache_directory(repo_root, cargo_target_dir or "target/public-bpe-cargo")
    generated_target = public_cache_directory(repo_root, str(target / "generated"))
    env.update(TCT_SOURCE_DIR=str(repo_root), CARGO_TARGET_DIR=str(target),
               TCT_GENERATED_CARGO_TARGET_DIR=str(generated_target),
               CARGO_BUILD_JOBS="2", RAYON_NUM_THREADS="2", RUST_TEST_THREADS="2",
               CARGO_NET_OFFLINE="true", CARGO_PROFILE_DEV_DEBUG="0", CARGO_PROFILE_TEST_DEBUG="0",
               GIT_CONFIG_NOSYSTEM="1", GIT_CONFIG_GLOBAL=os.devnull, GIT_NO_REPLACE_OBJECTS="1",
               GIT_TERMINAL_PROMPT="0", GIT_NO_LAZY_FETCH="1", GIT_OPTIONAL_LOCKS="0",
               GIT_CONFIG_COUNT="3", GIT_CONFIG_KEY_0="core.fsmonitor", GIT_CONFIG_VALUE_0="false",
               GIT_CONFIG_KEY_1="core.hooksPath", GIT_CONFIG_VALUE_1=os.devnull,
               GIT_CONFIG_KEY_2="log.showSignature", GIT_CONFIG_VALUE_2="false")
    if cargo_home:
        env["CARGO_HOME"] = str(public_cache_directory(repo_root, cargo_home))
    return env


def public_report(repo_root: Path, args: argparse.Namespace) -> tuple[dict[str, Any], bool]:
    report: dict[str, Any] = {
        "report_version": REPORT_VERSION, "artifact_kind": ARTIFACT_KIND, "tool": TOOL_NAME,
        "created_at": datetime.now(timezone.utc).isoformat(), "repo_root": str(repo_root),
        "mode": "public_artifacts", "qualified": False, "dry_run": args.dry_run,
        "cargo_profile": args.cargo_profile, "artifact_count": 0, "artifacts": [],
    }
    try:
        public_source.require(not args.dry_run, "--public-artifacts cannot qualify a dry run")
        public_source.require(not args.artifact and args.bpe_dir == "schemas/bpe",
                              "--public-artifacts requires exactly the six spec outputs; partial selection is forbidden")
        manifest = public_source.validate_manifest(repo_root)
        before = manifest["source_snapshot"]
        report["source_snapshot_before"] = {"sha256": before["sha256"], "file_count": len(before["files"])}
        report["training_spec"] = file_record(repo_root, repo_root / public_source.SPEC_PATH)
        artifacts = [repo_root / job["output"] for job in public_source.load_spec(repo_root)["artifacts"]]
        public_source.require(len(artifacts) == 6 and len(set(artifacts)) == 6, "all six public artifacts are required")
        work_parent = resolve_repo_path(repo_root, args.work_dir).resolve()
        output = resolve_repo_path(repo_root, args.output).resolve()
        for path in (work_parent, output):
            public_source.require(not path.is_relative_to(repo_root) or path.is_relative_to(repo_root / "target"),
                                  "public run output inside source must be under target/")
        env = public_environment(repo_root, args.cargo_home, args.cargo_target_dir)
        work_parent.mkdir(parents=True, exist_ok=True)
        work_dir = Path(tempfile.mkdtemp(prefix="public-", dir=work_parent))
        temp_dir = work_dir / "tmp"
        temp_dir.mkdir()
        env["TMPDIR"] = str(temp_dir)
        report["work_dir"] = str(work_dir)
        report["environment"] = {key: value for key, value in env.items()
                                 if key.startswith(("TCT_", "CARGO_", "GIT_", "RAYON_", "RUST_TEST_")) or key == "TMPDIR"}
        report["tool_versions"] = {"python": sys.version}
        for name in ("cargo", "rustc", "git"):
            completed = subprocess.run([name, "--version"], cwd=repo_root, env=env,
                                       capture_output=True, text=True, check=True)
            report["tool_versions"][name] = completed.stdout.strip()
        for artifact in artifacts:
            row, _ = artifact_report(repo_root, artifact, work_dir, args.cargo_profile,
                                     args.timeout_seconds, False, public_env=env)
            report["artifacts"].append(row)
        # Rederive the manifest, fixture identities and complete filesystem closure.
        final_manifest = public_source.validate_manifest(repo_root)
        public_source.require(manifest == final_manifest, "public artifact manifest changed during reproduction")
        after = final_manifest["source_snapshot"]
        public_source.require(before == after, "source snapshot changed during public reproduction")
        report["source_snapshot_after"] = {"sha256": after["sha256"], "file_count": len(after["files"])}
        report["qualified"] = (
            len(report["artifacts"]) == 6
            and {row["artifact"] for row in report["artifacts"]} == set(public_source.EXPECTED_OUTPUTS)
            and all(row.get("status") == "passed"
                    and row.get("comparison", {}).get("raw_hash_matches") is True
                    and row.get("comparison", {}).get("token_corpus_matches") is True
                    for row in report["artifacts"])
        )
    except (ValueError, OSError, KeyError, TypeError, subprocess.SubprocessError) as error:
        report["error"] = str(error)
    report["artifact_count"] = len(report["artifacts"])
    report["summary"] = {status: sum(row.get("status") == status for row in report["artifacts"])
                         for status in sorted({row.get("status", "failed") for row in report["artifacts"]})}
    return report, report["qualified"]


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Retrain checked-in BPE artifacts and compare their hashes."
    )
    parser.add_argument("--public-artifacts", action="store_true",
                        help="Qualify all six public recipes against their complete source manifest (no dry/partial runs).")
    parser.add_argument("--cargo-home", help="Explicit cached Cargo home for public mode; inherited Cargo overrides are cleared.")
    parser.add_argument("--cargo-target-dir", help="Explicit Cargo build cache for public mode.")
    parser.add_argument("--repo-root", default=".", help="Repository root.")
    parser.add_argument("--bpe-dir", default="schemas/bpe", help="BPE artifact directory.")
    parser.add_argument(
        "--artifact",
        action="append",
        default=[],
        help=(
            "Specific artifact to check; may be repeated. Defaults to "
            "sidecar-backed TCT artifacts with power-of-two target_vocab <= 2048."
        ),
    )
    parser.add_argument("--output", default=DEFAULT_REPORT, help="JSON report path.")
    parser.add_argument(
        "--work-dir",
        default="target/tct-bpe-artifact-reproducibility",
        help="Directory for regenerated artifacts and logs.",
    )
    parser.add_argument(
        "--cargo-profile",
        choices=("dev", "release"),
        default="release",
        help="Cargo profile used for helper binaries.",
    )
    parser.add_argument(
        "--timeout-seconds",
        type=int,
        default=1800,
        help="Per-command timeout for generated-code and training runs.",
    )
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="Only validate inputs and emit the command/report shape.",
    )
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    repo_root = Path(args.repo_root).resolve()
    if args.public_artifacts:
        output_path = resolve_repo_path(repo_root, args.output).resolve()
        if output_path.is_relative_to(repo_root) and not output_path.is_relative_to(repo_root / "target"):
            sys.stderr.write("public report output inside source must be under target/\n")
            return 1
        report, all_ok = public_report(repo_root, args)
        rendered = json.dumps(report, indent=2, sort_keys=True) + "\n"
        output_path.parent.mkdir(parents=True, exist_ok=True)
        output_path.write_text(rendered, encoding="utf-8")
        try:
            sys.stdout.write(rendered)
        except BrokenPipeError:
            pass
        return 0 if all_ok else 1
    artifacts = discover_artifacts(repo_root, args.artifact, args.bpe_dir)
    discovery = discovery_report(
        repo_root,
        args.bpe_dir,
        artifacts,
        bool(args.artifact),
    )
    work_dir = resolve_repo_path(repo_root, args.work_dir)
    work_dir.mkdir(parents=True, exist_ok=True)

    report_artifacts = []
    all_ok = True
    for artifact in artifacts:
        artifact_data, ok = artifact_report(
            repo_root,
            artifact,
            work_dir,
            args.cargo_profile,
            args.timeout_seconds,
            args.dry_run,
        )
        report_artifacts.append(artifact_data)
        all_ok = all_ok and ok

    statuses: dict[str, int] = {}
    for artifact in report_artifacts:
        statuses[artifact["status"]] = statuses.get(artifact["status"], 0) + 1

    report = {
        "report_version": REPORT_VERSION,
        "artifact_kind": ARTIFACT_KIND,
        "tool": TOOL_NAME,
        "created_at": datetime.now(timezone.utc).isoformat(),
        "repo_root": str(repo_root),
        "git": git_metadata(repo_root),
        "dry_run": args.dry_run,
        "cargo_profile": args.cargo_profile,
        "discovery": discovery,
        "artifact_count": len(report_artifacts),
        "summary": statuses,
        "artifacts": report_artifacts,
    }

    rendered = json.dumps(report, indent=2, sort_keys=True)
    output_path = resolve_repo_path(repo_root, args.output)
    output_path.parent.mkdir(parents=True, exist_ok=True)
    output_path.write_text(rendered + "\n", encoding="utf-8")
    try:
        sys.stdout.write(rendered + "\n")
    except BrokenPipeError:
        return 0 if all_ok else 1
    return 0 if all_ok else 1


if __name__ == "__main__":
    raise SystemExit(main())
