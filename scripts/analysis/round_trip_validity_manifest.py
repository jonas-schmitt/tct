#!/usr/bin/env python3
"""Emit per-schema validity manifests from a TCT round-trip report."""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path
from typing import Any


MANIFEST_VERSION = 1
ARTIFACT_KIND = "tct_round_trip_validity_manifest"
TOOL_NAME = "round_trip_validity_manifest"
SCHEMA_FEATURE_KEYS = {
    "$ref",
    "additionalProperties",
    "allOf",
    "anyOf",
    "const",
    "default",
    "enum",
    "format",
    "if",
    "items",
    "maxItems",
    "maxLength",
    "maximum",
    "minItems",
    "minLength",
    "minimum",
    "not",
    "oneOf",
    "pattern",
    "patternProperties",
    "properties",
    "required",
    "type",
}


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


def read_json(path: Path) -> Any:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except json.JSONDecodeError as error:
        raise ValueError(f"{path}: invalid JSON: {error}") from error


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


def file_provenance(repo_root: Path, path: Path) -> dict[str, Any]:
    return {
        "path": repo_relative(repo_root, path),
        "bytes": path.stat().st_size,
        "sha256": sha256_file(path),
    }


def collect_schema_features(value: Any, features: set[str] | None = None) -> list[str]:
    if features is None:
        features = set()
    if isinstance(value, dict):
        for key, child in value.items():
            if key in SCHEMA_FEATURE_KEYS:
                features.add(key)
            collect_schema_features(child, features)
    elif isinstance(value, list):
        for child in value:
            collect_schema_features(child, features)
    return sorted(features)


def bpe_path_for_schema(repo_root: Path, report: dict[str, Any], schema_name: str) -> Path | None:
    explicit = report.get("bpe_merges")
    if isinstance(explicit, str) and explicit:
        path = Path(explicit)
        return path if path.is_absolute() else repo_root / path

    preset = report.get("bpe_preset")
    if preset == "legacy-tct-500":
        return repo_root / "schemas" / "bpe" / f"{schema_name}-tct-bpe-500.json"
    return None


def summarize_bpe_policy(
    repo_root: Path, report: dict[str, Any], schema_name: str
) -> dict[str, Any]:
    merge_path = bpe_path_for_schema(repo_root, report, schema_name)
    enabled = merge_path is not None
    policy: dict[str, Any] = {
        "enabled": enabled,
        "preset": report.get("bpe_preset"),
        "explicit_merge_path": report.get("bpe_merges"),
        "boundary_policy": (
            "payload-local BPE inside the raw schema tokenizer; wrapper sequence "
            "PAD/BOS/EOS tokens are outside the raw BPE stream"
        ),
        "merge_file_missing": False,
        "merge_file": None,
        "base_vocab_size": None,
        "merge_count": None,
        "generated_vocab_size_with_pad": None,
        "legacy_dict_size": None,
    }
    if not enabled:
        return policy

    assert merge_path is not None
    if not merge_path.is_file():
        policy["merge_file_missing"] = True
        policy["merge_file"] = {"path": repo_relative(repo_root, merge_path)}
        return policy

    data = read_json(merge_path)
    if not isinstance(data, dict):
        raise ValueError(f"{merge_path}: BPE merge table must be a JSON object")
    merges = data.get("merges")
    if not isinstance(merges, list):
        raise ValueError(f"{merge_path}: BPE merges must be an array")
    base_vocab_size = data.get("base_vocab_size")
    if not isinstance(base_vocab_size, int):
        raise ValueError(f"{merge_path}: BPE base_vocab_size must be an integer")
    metadata = data.get("metadata") if isinstance(data.get("metadata"), dict) else {}

    policy.update(
        {
            "merge_file": file_provenance(repo_root, merge_path),
            "base_vocab_size": base_vocab_size,
            "merge_count": len(merges),
            "generated_vocab_size_with_pad": base_vocab_size + len(merges) + 1,
            "legacy_dict_size": metadata.get("dict_size"),
        }
    )
    return policy


def scenario_passed(scenario: dict[str, Any]) -> bool:
    return (
        scenario.get("successful_instances") == scenario.get("total_instances")
        and not scenario.get("failed_instances")
    )


def summarize_schema(
    repo_root: Path, report: dict[str, Any], schema_report: dict[str, Any]
) -> dict[str, Any]:
    schema_name = schema_report.get("schema_name")
    schema_folder = schema_report.get("schema_folder")
    if not isinstance(schema_name, str) or not isinstance(schema_folder, str):
        raise ValueError("round-trip schema entries must include schema_name and schema_folder")

    schema_path = repo_root / "schemas" / schema_folder / f"{schema_name}.json"
    evidence_gaps: list[str] = []
    schema_file: dict[str, Any] | None = None
    schema_features: list[str] = []
    if schema_path.is_file():
        schema_file = file_provenance(repo_root, schema_path)
        schema_features = collect_schema_features(read_json(schema_path))
    else:
        evidence_gaps.append("schema_file_missing")

    scenarios = schema_report.get("scenarios", [])
    if not isinstance(scenarios, list):
        raise ValueError(f"{schema_name}: scenarios must be an array")
    passed = bool(scenarios) and all(
        isinstance(scenario, dict) and scenario_passed(scenario) for scenario in scenarios
    )
    if not scenarios:
        evidence_gaps.append("round_trip_scenarios_missing")

    if not scenarios or any(
        not isinstance(scenario, dict) or not scenario.get("generated_code_sha256")
        for scenario in scenarios
    ):
        evidence_gaps.append("generated_code_sha256")

    bpe_policy = summarize_bpe_policy(repo_root, report, schema_name)
    if bpe_policy["merge_file_missing"]:
        evidence_gaps.append("bpe_merge_file_missing")

    scenario_summaries = []
    for scenario in scenarios:
        if not isinstance(scenario, dict):
            continue
        scenario_summaries.append(
            {
                "id": scenario.get("id"),
                "name": scenario.get("name"),
                "status": "passed" if scenario_passed(scenario) else "failed",
                "total_instances": scenario.get("total_instances"),
                "successful_instances": scenario.get("successful_instances"),
                "failed_instances": scenario.get("failed_instances", []),
                "schema_compilation_time_ms": scenario.get("schema_compilation_time_ms"),
                "code_generation_time_ms": scenario.get("code_generation_time_ms"),
                "rust_compilation_time_ms": scenario.get("rust_compilation_time_ms"),
                "generated_code_sha256": scenario.get("generated_code_sha256"),
            }
        )

    total_instances = sum(
        scenario.get("total_instances", 0)
        for scenario in scenarios
        if isinstance(scenario, dict) and isinstance(scenario.get("total_instances"), int)
    )
    successful_instances = sum(
        scenario.get("successful_instances", 0)
        for scenario in scenarios
        if isinstance(scenario, dict) and isinstance(scenario.get("successful_instances"), int)
    )
    failed_instances = sum(
        len(scenario.get("failed_instances", []))
        for scenario in scenarios
        if isinstance(scenario, dict) and isinstance(scenario.get("failed_instances"), list)
    )

    return {
        "schema_name": schema_name,
        "schema_folder": schema_folder,
        "schema_file": schema_file or {"path": repo_relative(repo_root, schema_path)},
        "schema_features_present": schema_features,
        "round_trip": {
            "status": "passed" if passed else "failed",
            "fixture_selection": report.get("fixture_selection"),
            "build_profile": report.get("build_profile"),
            "scenarios_env": report.get("round_trip_scenarios_env"),
            "instances": schema_report.get("instances", []),
            "reported_instance_count": schema_report.get("instance_count"),
            "total_instances": total_instances,
            "successful_instances": successful_instances,
            "failed_instances": failed_instances,
            "scenarios": scenario_summaries,
            "focused_reproduction_command": focused_reproduction_command(
                report, schema_name, schema_folder
            ),
        },
        "decode_contract": {
            "strict_or_recovery": (
                "generated decode may normalize supported recovery tokens, while the "
                "round-trip helper requires complete token consumption"
            ),
            "source_validation": (
                "round-trip helpers validate source JSON before encode and decoded JSON "
                "after decode against the source schema"
            ),
            "consumed_token_policy": "no unconsumed tokens are allowed in this gate",
        },
        "bpe_policy": bpe_policy,
        "pad_bos_eos_policy": {
            "raw_pad_token": "generated PAD_TOKEN; numeric value is not recorded in round-trip report",
            "raw_bos_token": None,
            "raw_eos_token": None,
            "wrapper_policy": (
                "BOS/EOS and schema-decision tokens are sequence-wrapper or downstream "
                "profile concerns, not raw schema-local round-trip tokens"
            ),
        },
        "evidence_gaps": sorted(evidence_gaps),
    }


def focused_reproduction_command(report: dict[str, Any], schema_name: str, schema_folder: str) -> str:
    env_parts = [f"SCHEMA_NAME={schema_name}", f"SCHEMA_FOLDER={schema_folder}"]
    if report.get("bpe_preset"):
        env_parts.append(f"ROUND_TRIP_BPE_PRESET={report['bpe_preset']}")
    if report.get("bpe_merges"):
        env_parts.append(f"BPE_MERGES={report['bpe_merges']}")
    if not report.get("default_smoke", True):
        env_parts.append("ROUND_TRIP_FULL=1")
    if report.get("round_trip_scenarios_env"):
        env_parts.append(f"ROUND_TRIP_SCENARIOS={report['round_trip_scenarios_env']}")
    if report.get("build_profile") == "release":
        env_parts.append("ROUND_TRIP_BUILD_PROFILE=release")
    if report.get("max_base_vocab"):
        env_parts.append(f"MAX_BASE_VOCAB={report['max_base_vocab']}")
    return "env " + " ".join(env_parts) + " cargo test --test round_trip_test test_round_trip"


def build_manifest(repo_root: Path, report_path: Path) -> dict[str, Any]:
    report = read_json(report_path)
    if not isinstance(report, dict):
        raise ValueError(f"{report_path}: round-trip report must be a JSON object")
    schemas = report.get("schemas")
    if not isinstance(schemas, list):
        raise ValueError(f"{report_path}: round-trip report must include schemas array")

    schema_manifests = [
        summarize_schema(repo_root, report, schema)
        for schema in schemas
        if isinstance(schema, dict)
    ]
    repo_status = git_value(repo_root, "status", "--porcelain")

    return {
        "manifest_version": MANIFEST_VERSION,
        "artifact_kind": ARTIFACT_KIND,
        "tool": TOOL_NAME,
        "created_at": datetime.now(timezone.utc).isoformat(),
        "repo_root": str(repo_root),
        "repo_commit": git_value(repo_root, "rev-parse", "HEAD"),
        "repo_dirty": None if repo_status is None else bool(repo_status),
        "source_round_trip_report": file_provenance(repo_root, report_path),
        "round_trip_run": {
            "mode": report.get("mode"),
            "default_smoke": report.get("default_smoke"),
            "fixture_selection": report.get("fixture_selection"),
            "max_instances": report.get("max_instances"),
            "build_profile": report.get("build_profile"),
            "round_trip_scenarios_env": report.get("round_trip_scenarios_env"),
            "bpe_merges": report.get("bpe_merges"),
            "bpe_preset": report.get("bpe_preset"),
            "max_base_vocab": report.get("max_base_vocab"),
        },
        "schema_count": len(schema_manifests),
        "overall_validity_status": (
            "passed"
            if schema_manifests
            and all(schema["round_trip"]["status"] == "passed" for schema in schema_manifests)
            else "failed"
        ),
        "schemas": schema_manifests,
    }


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Emit per-schema validity manifests from a TCT round-trip report."
    )
    parser.add_argument(
        "--repo-root",
        default=".",
        help="Repository root used for schema paths and optional git metadata.",
    )
    parser.add_argument(
        "--round-trip-report",
        default="target/tct-round-trip-report.json",
        help="Round-trip report JSON path, relative to --repo-root unless absolute.",
    )
    parser.add_argument(
        "--output",
        help="Optional JSON output path. The report is always printed to stdout.",
    )
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    repo_root = Path(args.repo_root).resolve()
    report_path = Path(args.round_trip_report)
    if not report_path.is_absolute():
        report_path = repo_root / report_path
    if not report_path.is_file():
        print(f"Round-trip report does not exist: {report_path}", file=sys.stderr)
        return 2

    try:
        manifest = build_manifest(repo_root, report_path)
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
