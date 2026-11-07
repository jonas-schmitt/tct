#!/usr/bin/env python3
"""Execute and validate the route-owned syntax backend validation matrix."""

from __future__ import annotations

import argparse
import json
import os
import re
import subprocess
import sys
import time
from pathlib import Path
from typing import Any


REPO_ROOT = Path(__file__).resolve().parents[2]
DEFAULT_FIXTURE = (
    "tests/fixtures/syntax_backend/validation-matrix-gate-v1.json"
)
DEFAULT_OUTPUT = (
    "target/tct-syntax-validation-matrix/execution-report-v1.json"
)
DEFAULT_LOG_DIR = "target/tct-syntax-validation-matrix/logs"
ARTIFACT_KIND = "tct_syntax_backend_validation_matrix_execution_report"
VERSION = 1
TOOL = "syntax_backend_validation_matrix_runner"
STATUS = "release_depth_execution_report"
PLANNED_STATUS = "planned_release_depth_execution_report"
COMPLETED_PROMOTION_CLASS = (
    "syntax backend validation matrix execution report helper gate"
)
NEXT_MISSING_STABLE_PROMOTION_CLASS = (
    "syntax backend validation matrix execution evidence preservation gate"
)


def main() -> int:
    args = parse_args()
    fixture_path = repo_path(args.fixture)
    output_path = repo_path(args.output)
    log_dir = repo_path(args.log_dir)
    gate = load_json(fixture_path)

    if args.validate_report:
        report_path = repo_path(args.validate_report)
        report = load_json(report_path)
        gaps = validate_report(gate, report, fixture_path, report_path)
        if gaps:
            for gap in gaps:
                print(gap, file=sys.stderr)
            return 1
        print(
            f"validated {len(report['category_results'])} matrix categories "
            f"from {report_path}"
        )
        return 0

    report = build_report(
        gate=gate,
        fixture_path=fixture_path,
        output_path=output_path,
        log_dir=log_dir,
        timeout_seconds=args.timeout_seconds,
        dry_run=args.dry_run,
    )
    write_json(output_path, report)
    print(
        f"wrote {len(report['category_results'])} matrix category results "
        f"to {output_path}"
    )
    return 0 if report["all_passed"] or args.dry_run else 1


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description=(
            "Execute the checked syntax backend validation matrix and write a "
            "volatile report under target/."
        )
    )
    parser.add_argument(
        "--fixture",
        default=DEFAULT_FIXTURE,
        help="checked syntax validation matrix fixture",
    )
    parser.add_argument(
        "--output",
        default=DEFAULT_OUTPUT,
        help="execution report path to write under target/",
    )
    parser.add_argument(
        "--log-dir",
        default=DEFAULT_LOG_DIR,
        help="directory for per-command logs under target/",
    )
    parser.add_argument(
        "--timeout-seconds",
        type=int,
        default=int(os.environ.get("TCT_SYNTAX_VALIDATION_MATRIX_TIMEOUT_SECONDS", "1800")),
        help="per-command timeout",
    )
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="write a planned report without executing matrix commands",
    )
    parser.add_argument(
        "--validate-report",
        help="validate an existing execution report against the current fixture",
    )
    return parser.parse_args()


def repo_path(value: str | Path) -> Path:
    path = Path(value)
    if path.is_absolute():
        return path
    return REPO_ROOT / path


def repo_relative(path: Path) -> str:
    try:
        return path.relative_to(REPO_ROOT).as_posix()
    except ValueError:
        return path.as_posix()


def load_json(path: Path) -> dict[str, Any]:
    with path.open(encoding="utf-8") as handle:
        data = json.load(handle)
    if not isinstance(data, dict):
        raise ValueError(f"{path} must contain a JSON object")
    return data


def write_json(path: Path, value: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def build_report(
    *,
    gate: dict[str, Any],
    fixture_path: Path,
    output_path: Path,
    log_dir: Path,
    timeout_seconds: int,
    dry_run: bool,
) -> dict[str, Any]:
    contract = expect_dict(gate, "matrix_contract")
    required_commands = expect_string_list(contract, "required_validation_commands")
    categories = expect_list(contract, "categories")
    categories_by_command = categories_for_commands(categories)
    command_results: list[dict[str, Any]] = []

    for index, command in enumerate(required_commands, start=1):
        log_path = log_dir / f"{index:02d}-{command_slug(command)}.log"
        result = (
            planned_command_result(command, log_path, categories_by_command)
            if dry_run
            else execute_command(command, log_path, timeout_seconds, categories_by_command)
        )
        command_results.append(result)

    command_status = {
        str(result["command"]): str(result["status"]) for result in command_results
    }
    category_results = []
    for raw_category in categories:
        category = expect_dict_value(raw_category, "category record")
        command = str(category.get("validation_command", ""))
        category_results.append(
            {
                "category_id": str(category.get("category_id", "")),
                "validation_command": command,
                "command_status": command_status.get(command, "missing"),
                "route_metadata_field": str(category.get("route_metadata_field", "")),
                "route_metadata_key": str(category.get("route_metadata_key", "")),
                "route_owned_evidence": list(category.get("route_owned_evidence", [])),
                "sidecar_control_gate": category.get("sidecar_control_gate", {}),
                "stable_promotion_required": category.get("stable_promotion_required"),
            }
        )

    all_passed = all(result["status"] == "passed" for result in command_results)
    return {
        "artifact_kind": ARTIFACT_KIND,
        "version": VERSION,
        "tool": TOOL,
        "status": PLANNED_STATUS if dry_run else STATUS,
        "fixture_path": repo_relative(fixture_path),
        "output_path": repo_relative(output_path),
        "log_dir": repo_relative(log_dir),
        "matrix_artifact_kind": gate.get("artifact_kind"),
        "matrix_tool": gate.get("tool"),
        "matrix_id": contract.get("matrix_id"),
        "matrix_status": gate.get("status"),
        "source_production_route": gate.get("source_production_route"),
        "compiler_boundary": gate.get("compiler_boundary"),
        "json_schema_backend_unchanged": True,
        "arena_ir_proof_used": False,
        "all_passed": all_passed,
        "timeout_seconds": timeout_seconds,
        "command_results": command_results,
        "category_results": category_results,
        "stable_metadata_claims": contract.get("stable_metadata_claims", []),
        "completed_promotion_class": COMPLETED_PROMOTION_CLASS,
        "next_missing_stable_promotion_class": NEXT_MISSING_STABLE_PROMOTION_CLASS,
    }


def planned_command_result(
    command: str,
    log_path: Path,
    categories_by_command: dict[str, list[str]],
) -> dict[str, Any]:
    return {
        "command": command,
        "status": "planned",
        "exit_code": None,
        "duration_ms": 0,
        "log_path": repo_relative(log_path),
        "category_ids": categories_by_command.get(command, []),
    }


def execute_command(
    command: str,
    log_path: Path,
    timeout_seconds: int,
    categories_by_command: dict[str, list[str]],
) -> dict[str, Any]:
    log_path.parent.mkdir(parents=True, exist_ok=True)
    started = time.monotonic()
    status = "passed"
    exit_code: int | None = 0
    try:
        completed = subprocess.run(
            ["bash", "-lc", command],
            cwd=REPO_ROOT,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            timeout=timeout_seconds,
            check=False,
        )
        exit_code = completed.returncode
        if completed.returncode != 0:
            status = "failed"
        log_text = completed.stdout
    except subprocess.TimeoutExpired as error:
        status = "timeout"
        exit_code = None
        output = error.stdout or ""
        if isinstance(output, bytes):
            output = output.decode("utf-8", errors="replace")
        log_text = output + f"\nTIMEOUT after {timeout_seconds} seconds\n"
    duration_ms = int((time.monotonic() - started) * 1000)
    log_path.write_text(log_text, encoding="utf-8", errors="replace")
    return {
        "command": command,
        "status": status,
        "exit_code": exit_code,
        "duration_ms": duration_ms,
        "log_path": repo_relative(log_path),
        "category_ids": categories_by_command.get(command, []),
    }


def categories_for_commands(categories: list[Any]) -> dict[str, list[str]]:
    grouped: dict[str, list[str]] = {}
    for raw_category in categories:
        category = expect_dict_value(raw_category, "category record")
        command = str(category.get("validation_command", ""))
        grouped.setdefault(command, []).append(str(category.get("category_id", "")))
    return grouped


def validate_report(
    gate: dict[str, Any],
    report: dict[str, Any],
    fixture_path: Path,
    report_path: Path,
) -> list[str]:
    gaps: list[str] = []
    contract = expect_dict(gate, "matrix_contract")
    required_commands = expect_string_list(contract, "required_validation_commands")
    categories = expect_list(contract, "categories")
    required_categories = [
        str(category.get("category_id", ""))
        for category in categories
        if isinstance(category, dict)
    ]

    expected_scalars = {
        "artifact_kind": ARTIFACT_KIND,
        "version": VERSION,
        "tool": TOOL,
        "status": STATUS,
        "fixture_path": repo_relative(fixture_path),
        "matrix_artifact_kind": gate.get("artifact_kind"),
        "matrix_tool": gate.get("tool"),
        "matrix_id": contract.get("matrix_id"),
        "matrix_status": gate.get("status"),
        "completed_promotion_class": COMPLETED_PROMOTION_CLASS,
        "next_missing_stable_promotion_class": NEXT_MISSING_STABLE_PROMOTION_CLASS,
    }
    for field, expected in expected_scalars.items():
        if report.get(field) != expected:
            gaps.append(f"report has wrong {field}: {report.get(field)!r}")

    if report.get("json_schema_backend_unchanged") is not True:
        gaps.append("report must preserve the JSON Schema backend")
    if report.get("arena_ir_proof_used") is not False:
        gaps.append("report must not use arena IR as syntax-level proof")
    if report.get("all_passed") is not True:
        gaps.append("report does not record a passed matrix execution")
    if not str(report.get("output_path", "")).startswith("target/"):
        gaps.append("report output_path must stay under target/")
    if repo_relative(report_path) != report.get("output_path"):
        gaps.append("report output_path does not match --validate-report path")

    command_results = report.get("command_results")
    if not isinstance(command_results, list):
        gaps.append("report command_results must be a list")
        command_results = []
    commands = {
        str(result.get("command", "")): result
        for result in command_results
        if isinstance(result, dict)
    }
    for command in required_commands:
        result = commands.get(command)
        if not isinstance(result, dict):
            gaps.append(f"report is missing command result: {command}")
            continue
        if result.get("status") != "passed":
            gaps.append(f"report command did not pass: {command}")
        if result.get("exit_code") != 0:
            gaps.append(f"report command has wrong exit code: {command}")
        if not str(result.get("log_path", "")).startswith(DEFAULT_LOG_DIR):
            gaps.append(f"report command log must stay under target/: {command}")

    category_results = report.get("category_results")
    if not isinstance(category_results, list):
        gaps.append("report category_results must be a list")
        category_results = []
    category_ids = {
        str(result.get("category_id", ""))
        for result in category_results
        if isinstance(result, dict)
    }
    for category_id in required_categories:
        if category_id not in category_ids:
            gaps.append(f"report is missing category result: {category_id}")
    for result in category_results:
        if not isinstance(result, dict):
            gaps.append("report category result must be an object")
            continue
        category_id = str(result.get("category_id", ""))
        if result.get("command_status") != "passed":
            gaps.append(f"report category did not pass: {category_id}")
        sidecar = result.get("sidecar_control_gate")
        if not isinstance(sidecar, dict):
            gaps.append(f"report category lacks sidecar control: {category_id}")
            continue
        if sidecar.get("control_only") is not True:
            gaps.append(f"report category sidecar is not control-only: {category_id}")
        if sidecar.get("arena_ir_not_proof") is not True:
            gaps.append(f"report category allows arena IR proof: {category_id}")

    stable_metadata_claims = set(
        str(value)
        for value in report.get("stable_metadata_claims", [])
        if isinstance(value, str)
    )
    for claim in expect_string_list(contract, "stable_metadata_claims"):
        if claim not in stable_metadata_claims:
            gaps.append(f"report is missing stable metadata claim: {claim}")

    return gaps


def expect_dict(data: dict[str, Any], field: str) -> dict[str, Any]:
    value = data.get(field)
    if not isinstance(value, dict):
        raise ValueError(f"{field} must be a JSON object")
    return value


def expect_dict_value(value: Any, context: str) -> dict[str, Any]:
    if not isinstance(value, dict):
        raise ValueError(f"{context} must be a JSON object")
    return value


def expect_list(data: dict[str, Any], field: str) -> list[Any]:
    value = data.get(field)
    if not isinstance(value, list):
        raise ValueError(f"{field} must be a JSON array")
    return value


def expect_string_list(data: dict[str, Any], field: str) -> list[str]:
    return [
        str(value)
        for value in expect_list(data, field)
        if isinstance(value, str)
    ]


def command_slug(command: str) -> str:
    slug = re.sub(r"[^a-zA-Z0-9]+", "-", command).strip("-").lower()
    return slug[:80] or "command"


if __name__ == "__main__":
    raise SystemExit(main())
