#!/usr/bin/env python3
"""Emit a deterministic manifest for checked-in BPE merge artifacts."""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path
from typing import Any


MANIFEST_VERSION = 4
ARTIFACT_KIND = "tct_bpe_artifact_inventory"
TOOL_NAME = "bpe_artifact_manifest"
DEFAULT_TARGET_VOCAB_CEILING = 2048
EXPLICIT_TARGET_VOCAB_CEILING = 4096
CLAIM_PROFILE_READY_ROADMAP_SCOPES = {
    "current_single_schema_candidate",
    "explicit_4096_candidate",
}
DEFAULT_REPRODUCIBILITY_QUEUE_SCOPE = "current_single_schema_candidate"
REQUIRED_PROVENANCE_FIELDS = (
    "command",
    "input_files",
    "input_corpus_fingerprint",
    "output_files",
    "base_vocab_size",
    "target_vocab",
    "trained_on_instances",
    "generated_merges",
)


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def repo_relative(repo_root: Path, path: Path) -> str:
    try:
        return path.resolve().relative_to(repo_root.resolve()).as_posix()
    except ValueError:
        return path.as_posix()


def infer_filename_metadata(path: Path) -> dict[str, str | None]:
    stem = path.name.removesuffix(".json")
    patterns = [
        ("-tct-bpe-", "tct"),
        ("-utf8-bpe-", "utf8"),
        ("-utf8-base-", "utf8"),
    ]
    for marker, family in patterns:
        if marker in stem:
            schema, target_label = stem.rsplit(marker, 1)
            return {
                "schema_from_filename": schema,
                "encoding_family": family,
                "target_label": target_label,
            }
    return {
        "schema_from_filename": None,
        "encoding_family": None,
        "target_label": None,
    }


def read_json(path: Path) -> Any:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except json.JSONDecodeError as error:
        raise ValueError(f"{path}: invalid JSON: {error}") from error


def corpus_fingerprint(records: list[dict[str, Any]]) -> str:
    digest = hashlib.sha256()
    digest.update(b"tct-bpe-artifact-inputs-v1\n")
    for record in records:
        digest.update(str(record["path"]).encode("utf-8"))
        digest.update(b"\0")
        digest.update(str(record["bytes"]).encode("ascii"))
        digest.update(b"\0")
        digest.update(str(record["sha256"]).encode("ascii"))
        digest.update(b"\n")
    return digest.hexdigest()


def input_files_match_current(repo_root: Path, input_files: Any) -> bool | None:
    if not isinstance(input_files, list):
        return None
    for record in input_files:
        if not isinstance(record, dict):
            return False
        path = record.get("path")
        if not isinstance(path, str):
            return False
        current_path = repo_root / path
        if not current_path.is_file():
            return False
        if record.get("bytes") != current_path.stat().st_size:
            return False
        if record.get("sha256") != sha256_file(current_path):
            return False
    return True


def is_power_of_two(value: int) -> bool:
    return value > 0 and (value & (value - 1)) == 0


def summarize_provenance(repo_root: Path, sidecar_path: Path) -> dict[str, Any] | None:
    if not sidecar_path.is_file():
        return None

    data = read_json(sidecar_path)
    if not isinstance(data, dict):
        raise ValueError(f"{sidecar_path}: provenance sidecar must be a JSON object")

    return {
        "path": repo_relative(repo_root, sidecar_path),
        "bytes": sidecar_path.stat().st_size,
        "sha256": sha256_file(sidecar_path),
        "created_at": data.get("created_at"),
        "tct_version": data.get("tct_version"),
        "repo_commit": data.get("repo_commit"),
        "repo_dirty": data.get("repo_dirty"),
        "command": data.get("command"),
        "schema_name": data.get("schema_name"),
        "instances_pattern": data.get("instances_pattern"),
        "encoded_input": data.get("encoded_input"),
        "schema_file": data.get("schema_file"),
        "encoded_input_file": data.get("encoded_input_file"),
        "prepare_data_metadata_file": data.get("prepare_data_metadata_file"),
        "base_vocab_size": data.get("base_vocab_size"),
        "base_vocab_source": data.get("base_vocab_source"),
        "optimization_level": data.get("optimization_level"),
        "target_vocab": data.get("target_vocab"),
        "checkpoints": data.get("checkpoints"),
        "sample_size": data.get("sample_size"),
        "cache_path": data.get("cache_path"),
        "cache_file": data.get("cache_file"),
        "trained_on_instances": data.get("trained_on_instances"),
        "generated_merges": data.get("generated_merges"),
        "input_file_count": (
            len(data["input_files"]) if isinstance(data.get("input_files"), list) else None
        ),
        "input_files": data.get("input_files"),
        "input_corpus_fingerprint": data.get("input_corpus_fingerprint"),
        "token_corpus": data.get("token_corpus"),
        "output_files": data.get("output_files"),
    }


def classify_provenance(
    repo_root: Path,
    artifact_path: str,
    artifact_sha256: str,
    artifact_base_vocab_size: int,
    artifact_merge_count: int,
    artifact_trained_on_instances: Any,
    provenance: dict[str, Any] | None,
) -> dict[str, Any]:
    if provenance is None:
        return {
            "provenance_status": "missing_sidecar",
            "provenance_gaps": ["missing_provenance_sidecar"],
            "current_claim_use": "blocked_without_fresh_provenance",
        }

    gaps = [
        f"missing_{field}"
        for field in REQUIRED_PROVENANCE_FIELDS
        if provenance.get(field) in (None, [], "")
    ]
    output_files = provenance.get("output_files")
    output_path_matches_artifact = False
    output_hash_matches_artifact = False
    output_record_matches_artifact = False
    if isinstance(output_files, list):
        for output_file in output_files:
            if not isinstance(output_file, dict):
                continue
            if output_file.get("path") == artifact_path:
                output_path_matches_artifact = True
            if output_file.get("sha256") == artifact_sha256:
                output_hash_matches_artifact = True
            if (
                output_file.get("path") == artifact_path
                and output_file.get("sha256") == artifact_sha256
            ):
                output_record_matches_artifact = True
    if not output_record_matches_artifact:
        if output_path_matches_artifact and output_hash_matches_artifact:
            gaps.append("output_file_record_does_not_match_artifact")
    if not output_path_matches_artifact:
        gaps.append("output_file_path_does_not_match_artifact")
    if not output_hash_matches_artifact:
        gaps.append("output_file_sha256_does_not_match_artifact")
    if provenance.get("base_vocab_size") != artifact_base_vocab_size:
        gaps.append("base_vocab_size_does_not_match_artifact")
    if provenance.get("generated_merges") != artifact_merge_count:
        gaps.append("generated_merges_does_not_match_artifact")
    if (
        artifact_trained_on_instances is not None
        and provenance.get("trained_on_instances") != artifact_trained_on_instances
    ):
        gaps.append("trained_on_instances_does_not_match_artifact")
    input_files = provenance.get("input_files")
    current_input_match = input_files_match_current(repo_root, input_files)
    if current_input_match is False:
        gaps.append("input_files_do_not_match_current_files")
    if isinstance(input_files, list) and provenance.get("input_corpus_fingerprint"):
        try:
            if provenance.get("input_corpus_fingerprint") != corpus_fingerprint(input_files):
                gaps.append("input_corpus_fingerprint_does_not_match_input_files")
        except (KeyError, TypeError, ValueError):
            gaps.append("input_corpus_fingerprint_does_not_match_input_files")
    token_corpus = provenance.get("token_corpus")
    token_corpus_required = True
    if token_corpus_required and not isinstance(token_corpus, dict):
        gaps.append("missing_token_corpus")
    if isinstance(token_corpus, dict):
        if not isinstance(token_corpus.get("source"), str):
            gaps.append("token_corpus_source")
        if not isinstance(token_corpus.get("sequence_fingerprint"), str):
            gaps.append("token_corpus_sequence_fingerprint")
        if token_corpus.get("sequence_count") != provenance.get("trained_on_instances"):
            gaps.append("token_corpus_sequence_count_does_not_match_trained_instances")
        if provenance.get("sample_size") is not None:
            if token_corpus.get("sample_seed") is None:
                gaps.append("token_corpus_sample_seed")
            if not isinstance(token_corpus.get("accepted_instance_files"), list):
                gaps.append("token_corpus_accepted_instance_files")
            if not isinstance(token_corpus.get("rejected_instance_files"), list):
                gaps.append("token_corpus_rejected_instance_files")
        if provenance.get("cache_path") and not (
            token_corpus.get("cache_hit") or token_corpus.get("cache_written")
        ):
            gaps.append("token_corpus_cache_identity")

    if gaps:
        return {
            "provenance_status": "incomplete_sidecar",
            "provenance_gaps": gaps,
            "current_claim_use": "blocked_without_fresh_provenance",
        }

    return {
        "provenance_status": "provenance_ready",
        "provenance_gaps": [],
        "current_claim_use": "candidate_artifact_evidence_not_paper_claim",
    }


def classify_roadmap_policy(
    encoding_family: str | None,
    generated_vocab_size_with_pad: int,
    provenance: dict[str, Any] | None,
) -> dict[str, Any]:
    if encoding_family != "tct":
        return {
            "queue_scope": "comparison_artifact_not_local_queue",
            "vocab_source": "not_applicable",
            "policy_vocab": None,
            "reason": (
                "non-TCT BPE artifacts are comparison evidence, "
                "not the local TCT config queue"
            ),
        }

    if provenance is not None:
        target_vocab = provenance.get("target_vocab")
        if not isinstance(target_vocab, int):
            return {
                "queue_scope": "unclassified_explicit_candidate",
                "vocab_source": "provenance_target_vocab",
                "policy_vocab": None,
                "reason": (
                    "sidecar target_vocab is missing or non-integer and "
                    "requires explicit artifact selection"
                ),
            }
        policy_vocab = target_vocab
        vocab_source = "provenance_target_vocab"
    else:
        policy_vocab = generated_vocab_size_with_pad
        vocab_source = "generated_vocab_size_with_pad"

    if policy_vocab > EXPLICIT_TARGET_VOCAB_CEILING:
        return {
            "queue_scope": "historical_excluded_by_vocab_policy",
            "vocab_source": vocab_source,
            "policy_vocab": policy_vocab,
            "reason": (
                "larger-than-4096 total vocabulary is historical unless a new "
                "downstream profile reopens it"
            ),
        }
    if not is_power_of_two(policy_vocab):
        return {
            "queue_scope": "historical_non_power_of_two_vocab_policy",
            "vocab_source": vocab_source,
            "policy_vocab": policy_vocab,
            "reason": (
                "non-power-of-two total vocabulary is historical unless a new "
                "downstream profile reopens it"
            ),
        }
    if policy_vocab <= DEFAULT_TARGET_VOCAB_CEILING:
        return {
            "queue_scope": "current_single_schema_candidate",
            "vocab_source": vocab_source,
            "policy_vocab": policy_vocab,
            "reason": (
                "power-of-two policy vocabulary is within the conservative "
                "single-schema ceiling"
            ),
        }
    return {
        "queue_scope": "explicit_4096_candidate",
        "vocab_source": vocab_source,
        "policy_vocab": policy_vocab,
        "reason": (
            "4096 total vocabulary requires schema-specific or shared-corpus "
            "justification"
        ),
    }


def cross_repo_handoff(artifact: dict[str, Any]) -> dict[str, Any]:
    provenance = artifact.get("provenance")
    token_corpus = provenance.get("token_corpus") if isinstance(provenance, dict) else None
    handoff_gaps = handoff_readiness_gaps(artifact, provenance, token_corpus)
    handoff: dict[str, Any] = {
        "ready_for_claim_profile": not handoff_gaps,
        "current_claim_use": artifact["current_claim_use"],
        "artifact": {
            "path": artifact["path"],
            "sha256": artifact["sha256"],
            "base_vocab_size": artifact["base_vocab_size"],
            "merge_count": artifact["merge_count"],
            "generated_vocab_size_with_pad": artifact["generated_vocab_size_with_pad"],
            "encoding_family": artifact["encoding_family"],
            "schema_from_filename": artifact["schema_from_filename"],
        },
        "provenance_status": artifact["provenance_status"],
        "provenance_gaps": artifact["provenance_gaps"],
        "handoff_gaps": handoff_gaps,
        "provenance_sidecar": None,
        "training": None,
    }
    if isinstance(provenance, dict):
        handoff["provenance_sidecar"] = {
            "path": provenance.get("path"),
            "sha256": provenance.get("sha256"),
        }
        handoff["training"] = {
            "command": provenance.get("command"),
            "created_at": provenance.get("created_at"),
            "tct_version": provenance.get("tct_version"),
            "repo_commit": provenance.get("repo_commit"),
            "repo_dirty": provenance.get("repo_dirty"),
            "schema_name": provenance.get("schema_name"),
            "schema_file": provenance.get("schema_file"),
            "instances_pattern": provenance.get("instances_pattern"),
            "encoded_input": provenance.get("encoded_input"),
            "encoded_input_file": provenance.get("encoded_input_file"),
            "prepare_data_metadata_file": provenance.get("prepare_data_metadata_file"),
            "base_vocab_source": provenance.get("base_vocab_source"),
            "optimization_level": provenance.get("optimization_level"),
            "target_vocab": provenance.get("target_vocab"),
            "trained_on_instances": provenance.get("trained_on_instances"),
            "input_file_count": provenance.get("input_file_count"),
            "input_corpus_fingerprint": provenance.get("input_corpus_fingerprint"),
            "token_corpus": (
                {
                    "source": token_corpus.get("source"),
                    "sequence_count": token_corpus.get("sequence_count"),
                    "sequence_fingerprint": token_corpus.get("sequence_fingerprint"),
                    "sample_size": token_corpus.get("sample_size"),
                    "sample_seed": token_corpus.get("sample_seed"),
                    "cache_path": token_corpus.get("cache_path"),
                    "cache_hit": token_corpus.get("cache_hit"),
                    "cache_written": token_corpus.get("cache_written"),
                    "accepted_instance_count": (
                        len(token_corpus["accepted_instance_files"])
                        if isinstance(token_corpus.get("accepted_instance_files"), list)
                        else None
                    ),
                    "rejected_instance_count": (
                        len(token_corpus["rejected_instance_files"])
                        if isinstance(token_corpus.get("rejected_instance_files"), list)
                        else None
                    ),
                }
                if isinstance(token_corpus, dict)
                else None
            ),
        }
    return handoff


def handoff_readiness_gaps(
    artifact: dict[str, Any],
    provenance: Any,
    token_corpus: Any,
) -> list[str]:
    gaps: list[str] = []
    if artifact["provenance_status"] != "provenance_ready":
        gaps.extend(artifact.get("provenance_gaps", []))
    if not isinstance(provenance, dict):
        return gaps or ["missing_provenance_sidecar"]
    queue_scope = artifact.get("roadmap_policy", {}).get("queue_scope")
    if queue_scope not in CLAIM_PROFILE_READY_ROADMAP_SCOPES:
        scope = queue_scope if isinstance(queue_scope, str) and queue_scope else "missing"
        gaps.append(f"roadmap_policy_{scope}_not_claim_profile_ready")
    if not provenance.get("path") or not provenance.get("sha256"):
        gaps.append("missing_provenance_sidecar_fingerprint")
    if not provenance.get("command"):
        gaps.append("missing_training_command")
    if not provenance.get("input_corpus_fingerprint"):
        gaps.append("missing_input_corpus_fingerprint")
    if not isinstance(token_corpus, dict) or not token_corpus.get("sequence_fingerprint"):
        gaps.append("missing_token_corpus_sequence_fingerprint")
    if not provenance.get("repo_commit"):
        gaps.append("missing_repo_commit")
    if provenance.get("repo_dirty") is None:
        gaps.append("missing_repo_dirty")
    elif provenance.get("repo_dirty") is not False:
        gaps.append("repo_dirty")
    return gaps


def summarize_artifact(repo_root: Path, path: Path) -> dict[str, Any]:
    data = read_json(path)
    if not isinstance(data, dict):
        raise ValueError(f"{path}: merge table must be a JSON object")

    merges = data.get("merges")
    if not isinstance(merges, list):
        raise ValueError(f"{path}: merges must be an array")

    base_vocab_size = data.get("base_vocab_size")
    if not isinstance(base_vocab_size, int):
        raise ValueError(f"{path}: base_vocab_size must be an integer")

    metadata = data.get("metadata")
    if metadata is None:
        metadata = {}
    if not isinstance(metadata, dict):
        raise ValueError(f"{path}: metadata must be an object when present")

    new_tokens = [
        merge.get("new_token")
        for merge in merges
        if isinstance(merge, dict) and isinstance(merge.get("new_token"), int)
    ]
    frequencies = [
        merge.get("frequency")
        for merge in merges
        if isinstance(merge, dict) and isinstance(merge.get("frequency"), int)
    ]
    pair_tokens = [
        token
        for merge in merges
        if isinstance(merge, dict) and isinstance(merge.get("pair"), list)
        for token in merge["pair"]
        if isinstance(token, int)
    ]

    first_new_token = new_tokens[0] if new_tokens else None
    last_new_token = new_tokens[-1] if new_tokens else None
    expected_last = base_vocab_size + len(merges) - 1 if merges else None
    generated_vocab_size_with_pad = base_vocab_size + len(merges) + 1

    artifact_path = repo_relative(repo_root, path)
    artifact_sha256 = sha256_file(path)
    provenance = summarize_provenance(repo_root, path.with_suffix(".provenance.json"))
    filename_metadata = infer_filename_metadata(path)
    artifact = {
        "path": artifact_path,
        "bytes": path.stat().st_size,
        "sha256": artifact_sha256,
        "schema_field": data.get("schema"),
        **filename_metadata,
        "base_vocab_size": base_vocab_size,
        "merge_count": len(merges),
        "first_new_token": first_new_token,
        "last_new_token": last_new_token,
        "new_tokens_are_contiguous_from_base": (
            first_new_token == base_vocab_size and last_new_token == expected_last
        ),
        "generated_vocab_size_with_pad": generated_vocab_size_with_pad,
        "roadmap_policy": classify_roadmap_policy(
            filename_metadata["encoding_family"],
            generated_vocab_size_with_pad,
            provenance,
        ),
        "max_pair_token": max(pair_tokens) if pair_tokens else None,
        "max_frequency": max(frequencies) if frequencies else None,
        "metadata": {
            "trained_on_instances": metadata.get("trained_on_instances"),
            "training_date": metadata.get("training_date"),
            "max_merges": metadata.get("max_merges"),
            "dict_size": metadata.get("dict_size"),
        },
        "provenance": provenance,
        **classify_provenance(
            repo_root,
            artifact_path,
            artifact_sha256,
            base_vocab_size,
            len(merges),
            metadata.get("trained_on_instances"),
            provenance,
        ),
    }
    artifact["cross_repo_handoff"] = cross_repo_handoff(artifact)
    return artifact


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


def build_manifest(repo_root: Path, bpe_dir: Path) -> dict[str, Any]:
    artifact_paths = sorted(
        path
        for path in bpe_dir.glob("*.json")
        if not path.name.endswith(".provenance.json")
    )
    artifacts = [summarize_artifact(repo_root, path) for path in artifact_paths]
    repo_status = git_value(repo_root, "status", "--porcelain")

    return {
        "manifest_version": MANIFEST_VERSION,
        "artifact_kind": ARTIFACT_KIND,
        "tool": TOOL_NAME,
        "created_at": datetime.now(timezone.utc).isoformat(),
        "repo_root": str(repo_root),
        "repo_commit": git_value(repo_root, "rev-parse", "HEAD"),
        "repo_dirty": None if repo_status is None else bool(repo_status),
        "bpe_dir": repo_relative(repo_root, bpe_dir),
        "artifact_count": len(artifacts),
        "provenance_summary": provenance_summary(artifacts),
        "artifacts": artifacts,
    }


def provenance_summary(artifacts: list[dict[str, Any]]) -> dict[str, Any]:
    status_counts: dict[str, int] = {}
    claim_use_counts: dict[str, int] = {}
    roadmap_policy_counts: dict[str, int] = {}
    artifacts_by_status: dict[str, list[str]] = {}
    artifacts_by_roadmap_policy: dict[str, list[str]] = {}
    blocked_artifacts: list[str] = []
    blocked_artifacts_by_gap: dict[str, list[str]] = {}
    gap_counts: dict[str, int] = {}
    handoff_blocked_artifacts: list[str] = []
    handoff_blocked_artifacts_by_gap: dict[str, list[str]] = {}
    handoff_gap_counts: dict[str, int] = {}

    for artifact in artifacts:
        path = artifact["path"]
        status = artifact["provenance_status"]
        claim_use = artifact["current_claim_use"]
        queue_scope = artifact.get("roadmap_policy", {}).get("queue_scope", "unknown")
        status_counts[status] = status_counts.get(status, 0) + 1
        claim_use_counts[claim_use] = claim_use_counts.get(claim_use, 0) + 1
        roadmap_policy_counts[queue_scope] = (
            roadmap_policy_counts.get(queue_scope, 0) + 1
        )
        artifacts_by_status.setdefault(status, []).append(path)
        artifacts_by_roadmap_policy.setdefault(queue_scope, []).append(path)
        if claim_use == "blocked_without_fresh_provenance":
            blocked_artifacts.append(path)
        for gap in artifact.get("provenance_gaps", []):
            gap_counts[gap] = gap_counts.get(gap, 0) + 1
            blocked_artifacts_by_gap.setdefault(gap, []).append(path)
        handoff = artifact.get("cross_repo_handoff", {})
        if not handoff.get("ready_for_claim_profile"):
            handoff_blocked_artifacts.append(path)
            for gap in handoff.get("handoff_gaps", []):
                if not isinstance(gap, str):
                    continue
                handoff_gap_counts[gap] = handoff_gap_counts.get(gap, 0) + 1
                handoff_blocked_artifacts_by_gap.setdefault(gap, []).append(path)

    default_policy_artifacts = sorted(
        artifacts_by_roadmap_policy.get(DEFAULT_REPRODUCIBILITY_QUEUE_SCOPE, [])
    )
    provenance_ready_artifacts = set(artifacts_by_status.get("provenance_ready", []))
    default_queue_artifacts = sorted(
        path for path in default_policy_artifacts if path in provenance_ready_artifacts
    )
    default_queue_blocked_artifacts = sorted(
        path
        for path in default_policy_artifacts
        if path not in provenance_ready_artifacts
    )

    return {
        "status_counts": status_counts,
        "claim_use_counts": claim_use_counts,
        "roadmap_policy_counts": roadmap_policy_counts,
        "artifacts_by_roadmap_policy": artifacts_by_roadmap_policy,
        "artifacts_by_status": artifacts_by_status,
        "default_reproducibility_queue_scope": (
            DEFAULT_REPRODUCIBILITY_QUEUE_SCOPE
        ),
        "default_reproducibility_queue_policy_artifacts": default_policy_artifacts,
        "default_reproducibility_queue_policy_artifact_count": len(
            default_policy_artifacts
        ),
        "default_reproducibility_queue_artifacts": default_queue_artifacts,
        "default_reproducibility_queue_artifact_count": len(
            default_queue_artifacts
        ),
        "default_reproducibility_queue_blocked_artifacts": (
            default_queue_blocked_artifacts
        ),
        "default_reproducibility_queue_blocked_artifact_count": len(
            default_queue_blocked_artifacts
        ),
        "blocked_artifacts": blocked_artifacts,
        "blocked_artifact_count": len(blocked_artifacts),
        "gap_counts": gap_counts,
        "blocked_artifacts_by_gap": blocked_artifacts_by_gap,
        "cross_repo_handoff_blocked_artifacts": handoff_blocked_artifacts,
        "cross_repo_handoff_blocked_artifact_count": len(handoff_blocked_artifacts),
        "cross_repo_handoff_gap_counts": handoff_gap_counts,
        "cross_repo_handoff_blocked_artifacts_by_gap": (
            handoff_blocked_artifacts_by_gap
        ),
        "provenance_ready_artifacts": artifacts_by_status.get("provenance_ready", []),
        "cross_repo_handoff_ready_artifacts": [
            artifact["path"]
            for artifact in artifacts
            if artifact.get("cross_repo_handoff", {}).get("ready_for_claim_profile")
        ],
    }


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Emit a deterministic manifest for checked-in BPE merge artifacts."
    )
    parser.add_argument(
        "--repo-root",
        default=".",
        help="Repository root used for relative paths and optional git metadata.",
    )
    parser.add_argument(
        "--bpe-dir",
        default="schemas/bpe",
        help="Directory containing BPE merge JSON files, relative to --repo-root unless absolute.",
    )
    parser.add_argument(
        "--output",
        help="Optional JSON output path. The report is always printed to stdout.",
    )
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    repo_root = Path(args.repo_root).resolve()
    bpe_dir = Path(args.bpe_dir)
    if not bpe_dir.is_absolute():
        bpe_dir = repo_root / bpe_dir
    if not bpe_dir.is_dir():
        print(f"BPE artifact directory does not exist: {bpe_dir}", file=sys.stderr)
        return 2

    try:
        manifest = build_manifest(repo_root, bpe_dir)
    except ValueError as error:
        print(str(error), file=sys.stderr)
        return 2

    rendered = json.dumps(manifest, indent=2, sort_keys=True)
    if args.output:
        output_path = Path(args.output)
        output_path.parent.mkdir(parents=True, exist_ok=True)
        output_path.write_text(rendered + "\n", encoding="utf-8")
    try:
        sys.stdout.write(rendered + "\n")
    except BrokenPipeError:
        return 0
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
