#!/usr/bin/env python3
"""Batch Go current-plus generated-codec intake from checker-ready evidence.

Default mode plans or applies intake of the next checker-ready MISSING records
(`--selection-report`, `--checker-report`, `--max-new-records`). The
`--refresh-existing` mode is mutually exclusive with those intake flags: it
iterates the report-ingestion manifest's already-ingested records in order and
re-runs the same per-record metadata/projection/tokenizer-generator pipeline
for every one of them. Without `--apply` it prints the refresh plan (record
count and rewrite targets) and exits 0; with `--apply` it rewrites both the
current-plus report-ingestion manifest records and the checked per-record
projection reports, so one command regenerates the whole corpus consistently
after a token-layout change.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
from dataclasses import dataclass
from pathlib import Path
from typing import Any


TOOL = "go_syntax_current_plus_batch_intake"
ARTIFACT_KIND = "tct_go_syntax_current_plus_batch_intake_v1"
VERSION = 1

DEFAULT_MAX_NEW_RECORDS = 8

DEFAULT_REPORT_INGESTION_MANIFEST = (
    "tests/fixtures/go_syntax_corpus/training-report-ingestion-current-plus-v1.json"
)
DEFAULT_SOURCE_FIXTURE_ROOT = "tests/fixtures/go_syntax_corpus/src"
DEFAULT_METADATA_ROOT = "tests/fixtures/go_syntax_metadata"
DEFAULT_CHECKED_REPORT_ROOT = "tests/fixtures/go_syntax_corpus"
DEFAULT_PROJECTION_OUTPUT_ROOT = "target/tct-analysis/go-syntax-source-instance"
DEFAULT_TOKENIZER_OUTPUT_ROOT = "target/tct-syntax-real-source-demo"
DEFAULT_METADATA_SMOKE_FIXTURE = "generic-source-file"
GENERATED_CODEC_ROUTE = "go_source_instance_projection_contextual_raw_tokens"
GENERATED_CODEC_REPORT_ARTIFACT_KIND = (
    "tct_go_syntax_source_instance_generated_codec_report_v1"
)
CURRENT_PLUS_ROLE = "current_plus_generated_codec_round_trip"
CURRENT_PLUS_EVIDENCE_SCOPE = "source_instance_generated_codec"

# Unified pinned inventory-derived Go grammar artifact (M1): every
# current-plus record projects against the one checked go/ast schema
# inventory grammar instead of a per-file observed grammar.
GO_UNIFIED_GRAMMAR_ID = "go_ast_schema_inventory_go1_24_3"
# Pinned unified artifact id; the self-test cross-checks this constant
# against the checked grammar-artifact pin fixture.
GO_GRAMMAR_ARTIFACT_ID = "tct-go-grammar-artifact-v1"
# Kind recorded when --per-file-grammar-blocker deliberately mints a retired
# per-file candidate grammar id through the escape hatch.
PER_FILE_GRAMMAR_BLOCKER_KIND = "per_file_grammar_intake_override"
GO_AST_SCHEMA_INVENTORY_FIXTURE = (
    "tests/fixtures/go_ast_schema_inventory/go1.24.3-ast-schema-inventory-v1.json"
)
GO_GRAMMAR_ARTIFACT_PIN_FIXTURE = (
    "tests/fixtures/syntax_structural_model/go-grammar-artifact-pin-v1.json"
)
RAW_SYNTAX_STREAM_CONTRACT_ELIDED_V1 = "raw-syntax-stream-elided-v1"


@dataclass(frozen=True)
class CandidatePlan:
    manifest_path: str
    source_path: str
    source_sha256: str
    source_bytes: int
    package_name: str
    selection_order: int
    source_fixture_path: str
    metadata_path: str
    checked_projection_report_path: str
    projection_output_path: str
    tokenizer_report_path: str
    candidate_grammar_id: str


def repo_relative(path: Path) -> str:
    return path.as_posix()


def read_json(path: Path) -> Any:
    with path.open("r", encoding="utf-8") as handle:
        return json.load(handle)


def require_report_field(
    report: dict[str, Any], key: str, *, report_label: str
) -> Any:
    """Return a required report field or raise an explicit contract error.

    Tokenizer-generator reports after the elided-stream cut
    (raw-syntax-stream-elided-v1) must carry the canonical token fields used by
    current-plus ingestion. A bare KeyError is easy to misread as a missing
    dict; name the report, the missing key, and the present keys instead.
    """
    if key not in report:
        present = ", ".join(sorted(str(name) for name in report.keys())) or "(none)"
        raise ValueError(
            f"{report_label} is missing required field {key!r} "
            f"(elided-stream tokenizer report contract); present keys: {present}"
        )
    return report[key]


def write_json(path: Path, value: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2, sort_keys=False) + "\n", encoding="utf-8")


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def command_string(command: list[str]) -> str:
    return " ".join(command)


def run_command(command: list[str], env: dict[str, str] | None = None) -> None:
    result = subprocess.run(command, text=True, capture_output=True, env=env)
    if result.returncode != 0:
        stderr = result.stderr.strip()
        stdout = result.stdout.strip()
        detail = stderr or stdout or f"exit status {result.returncode}"
        raise RuntimeError(f"{command_string(command)} failed: {detail}")


def slug_parts(manifest_path: str) -> list[str]:
    without_ext = re.sub(r"\.go$", "", manifest_path)
    return [part for part in re.split(r"[^A-Za-z0-9]+", without_ext) if part]


def underscore_slug(manifest_path: str) -> str:
    return "_".join(part.lower() for part in slug_parts(manifest_path))


def dash_slug(manifest_path: str) -> str:
    return "-".join(part.lower() for part in slug_parts(manifest_path))


def grammar_id_for_manifest_path(
    manifest_path: str, per_file_grammar_blocker: str | None = None
) -> str:
    # M1 unified artifact: one inventory-derived grammar id covers every
    # current-plus record; per-file go_*_source_file_syntax_v0 ids are
    # retired from the forward gate (historical evidence only). Minting one
    # requires the explicit --per-file-grammar-blocker escape hatch.
    if per_file_grammar_blocker is None:
        return GO_UNIFIED_GRAMMAR_ID
    if not per_file_grammar_blocker.strip():
        raise ValueError(
            "--per-file-grammar-blocker requires a non-empty reason naming why the "
            f"unified pinned grammar artifact {GO_GRAMMAR_ARTIFACT_ID} "
            f"(grammar id {GO_UNIFIED_GRAMMAR_ID}) cannot cover the record"
        )
    return f"go_{underscore_slug(manifest_path)}_source_file_syntax_v0"


def validate_forward_candidate_grammar_id(
    candidate_grammar_id: str, per_file_grammar_blocker: str | None = None
) -> None:
    """Forward gate (M1): refuse per-file candidate grammar ids.

    The unified inventory grammar id is the only accepted forward id; any
    other candidate grammar id is a retired per-file registration and is
    refused unless an explicit --per-file-grammar-blocker reason records the
    exception.
    """
    if candidate_grammar_id == GO_UNIFIED_GRAMMAR_ID:
        if per_file_grammar_blocker is not None:
            raise ValueError(
                "--per-file-grammar-blocker must not be recorded for the unified "
                f"inventory grammar id {GO_UNIFIED_GRAMMAR_ID}"
            )
        return
    if per_file_grammar_blocker and per_file_grammar_blocker.strip():
        return
    raise ValueError(
        f"per-file Go grammar registration {candidate_grammar_id!r} is retired from "
        "the forward intake: every current-plus record must project against the "
        f"unified pinned grammar artifact {GO_GRAMMAR_ARTIFACT_ID} "
        f"(grammar id {GO_UNIFIED_GRAMMAR_ID}); pass --per-file-grammar-blocker "
        '"<reason>" to record an explicit blocker for a deliberate exception'
    )


def metadata_path_for_manifest_path(manifest_path: str, metadata_root: Path) -> Path:
    # Inventory-as-metadata: the projector consumes the checked go/ast schema
    # inventory fixture directly; per-file metadata fixtures under
    # tests/fixtures/go_syntax_metadata/ stay checked in as historical
    # parser-provenance evidence only.
    del manifest_path, metadata_root
    return Path(GO_AST_SCHEMA_INVENTORY_FIXTURE)


def checked_report_path_for_manifest_path(manifest_path: str, checked_report_root: Path) -> Path:
    return (
        checked_report_root
        / f"source-instance-projection-report-{dash_slug(manifest_path)}-v1.json"
    )


def projection_output_for_manifest_path(manifest_path: str, output_root: Path) -> Path:
    return output_root / f"{dash_slug(manifest_path)}-source-instance-projection-v1.json"


def tokenizer_report_for_manifest_path(manifest_path: str, output_root: Path) -> Path:
    return output_root / f"go-{dash_slug(manifest_path)}-tokenizer-generator-report-v1.json"


def source_fixture_path_for_manifest_path(manifest_path: str, source_fixture_root: Path) -> Path:
    return source_fixture_root / manifest_path


def selected_candidates(selection: dict[str, Any]) -> list[dict[str, Any]]:
    selected = [candidate for candidate in selection.get("selected_candidates", []) if candidate.get("selected")]
    return sorted(selected, key=lambda candidate: candidate.get("selection_order", 0))


def checker_ready_by_sha(checker: dict[str, Any]) -> dict[str, dict[str, Any]]:
    if checker.get("status") != "ready":
        raise ValueError(f"checker report status must be ready, got {checker.get('status')!r}")
    if checker.get("unsupported_constructs") or checker.get("unsupported_grammar_gaps"):
        raise ValueError("checker report still has unsupported constructs or grammar gaps")
    if checker.get("next_missing_construct") or checker.get("next_missing_grammar_gap"):
        raise ValueError("checker report still names a missing construct or grammar gap")

    ready: dict[str, dict[str, Any]] = {}
    for file_summary in checker.get("files", []):
        if file_summary.get("unsupported_constructs") or file_summary.get("unsupported_grammar_gaps"):
            continue
        sha = file_summary.get("sha256")
        if sha:
            ready[sha] = file_summary
    return ready


def ingested_keys(manifest: dict[str, Any]) -> tuple[set[str], set[str]]:
    paths: set[str] = set()
    shas: set[str] = set()
    for record in manifest.get("records", []):
        if record.get("manifest_path"):
            paths.add(record["manifest_path"])
        if record.get("source_sha256"):
            shas.add(record["source_sha256"])
    return paths, shas


def candidate_source_path(candidate: dict[str, Any], go_bin: str) -> Path:
    for key in ("source_path",):
        value = candidate.get(key)
        if value and Path(value).exists():
            return Path(value)
    file_path = candidate.get("file", {}).get("path")
    if file_path and Path(file_path).exists():
        return Path(file_path)
    goroot = subprocess.check_output([go_bin, "env", "GOROOT"], text=True).strip()
    return Path(goroot) / "src" / candidate["manifest_path"]


def build_candidate_plan(
    candidate: dict[str, Any],
    *,
    go_bin: str,
    source_fixture_root: Path,
    metadata_root: Path,
    checked_report_root: Path,
    projection_output_root: Path,
    tokenizer_output_root: Path,
    per_file_grammar_blocker: str | None = None,
) -> CandidatePlan:
    file_ref = candidate.get("file", {})
    manifest_path = candidate["manifest_path"]
    source_path = candidate_source_path(candidate, go_bin)
    candidate_grammar_id = grammar_id_for_manifest_path(
        manifest_path, per_file_grammar_blocker
    )
    validate_forward_candidate_grammar_id(candidate_grammar_id, per_file_grammar_blocker)
    return CandidatePlan(
        manifest_path=manifest_path,
        source_path=repo_relative(source_path),
        source_sha256=file_ref["sha256"],
        source_bytes=int(file_ref["bytes"]),
        package_name=candidate.get("package", ""),
        selection_order=int(candidate.get("selection_order", 0)),
        source_fixture_path=repo_relative(
            source_fixture_path_for_manifest_path(manifest_path, source_fixture_root)
        ),
        metadata_path=repo_relative(metadata_path_for_manifest_path(manifest_path, metadata_root)),
        checked_projection_report_path=repo_relative(
            checked_report_path_for_manifest_path(manifest_path, checked_report_root)
        ),
        projection_output_path=repo_relative(
            projection_output_for_manifest_path(manifest_path, projection_output_root)
        ),
        tokenizer_report_path=repo_relative(
            tokenizer_report_for_manifest_path(manifest_path, tokenizer_output_root)
        ),
        candidate_grammar_id=candidate_grammar_id,
    )


def build_batch_plan(
    selection: dict[str, Any],
    checker: dict[str, Any],
    report_ingestion_manifest: dict[str, Any],
    *,
    max_new_records: int,
    go_bin: str,
    source_fixture_root: Path,
    metadata_root: Path,
    checked_report_root: Path,
    projection_output_root: Path,
    tokenizer_output_root: Path,
    per_file_grammar_blocker: str | None = None,
) -> list[CandidatePlan]:
    ready_by_sha = checker_ready_by_sha(checker)
    ingested_paths, ingested_shas = ingested_keys(report_ingestion_manifest)
    plans: list[CandidatePlan] = []
    for candidate in selected_candidates(selection):
        file_ref = candidate.get("file", {})
        manifest_path = candidate.get("manifest_path")
        sha = file_ref.get("sha256")
        if not manifest_path or not sha:
            continue
        if manifest_path in ingested_paths or sha in ingested_shas:
            continue
        if sha not in ready_by_sha:
            continue
        plans.append(
            build_candidate_plan(
                candidate,
                go_bin=go_bin,
                source_fixture_root=source_fixture_root,
                metadata_root=metadata_root,
                checked_report_root=checked_report_root,
                projection_output_root=projection_output_root,
                tokenizer_output_root=tokenizer_output_root,
                per_file_grammar_blocker=per_file_grammar_blocker,
            )
        )
        if len(plans) >= max_new_records:
            break
    return plans


def build_refresh_plans(
    manifest: dict[str, Any],
    *,
    source_fixture_root: Path,
    metadata_root: Path,
    checked_report_root: Path,
    projection_output_root: Path,
    tokenizer_output_root: Path,
) -> list[CandidatePlan]:
    """Plan one pipeline re-run per already-ingested manifest record, in record order."""
    plans: list[CandidatePlan] = []
    for order, record in enumerate(manifest.get("records", []), start=1):
        manifest_path = record.get("manifest_path")
        sha = record.get("source_sha256")
        if not manifest_path or not sha:
            raise ValueError(
                f"report-ingestion record {order} is missing manifest_path or source_sha256"
            )
        candidate_grammar_id = grammar_id_for_manifest_path(manifest_path)
        validate_forward_candidate_grammar_id(candidate_grammar_id)
        source_fixture = source_fixture_path_for_manifest_path(manifest_path, source_fixture_root)
        plans.append(
            CandidatePlan(
                manifest_path=manifest_path,
                source_path=repo_relative(source_fixture),
                source_sha256=sha,
                source_bytes=int(record.get("source_bytes", 0)),
                package_name=record.get("package", ""),
                selection_order=order,
                source_fixture_path=repo_relative(source_fixture),
                metadata_path=repo_relative(
                    metadata_path_for_manifest_path(manifest_path, metadata_root)
                ),
                checked_projection_report_path=repo_relative(
                    checked_report_path_for_manifest_path(manifest_path, checked_report_root)
                ),
                projection_output_path=repo_relative(
                    projection_output_for_manifest_path(manifest_path, projection_output_root)
                ),
                tokenizer_report_path=repo_relative(
                    tokenizer_report_for_manifest_path(manifest_path, tokenizer_output_root)
                ),
                candidate_grammar_id=candidate_grammar_id,
            )
        )
    return plans


def default_go_bin() -> str:
    configured = os.environ.get("TCT_GO_BIN")
    if configured:
        return configured
    repo_local = Path("target/tct-go-bin/go")
    if repo_local.exists():
        return repo_relative(repo_local)
    return "go"


def copy_source(plan: CandidatePlan, created_paths: list[Path]) -> None:
    source = Path(plan.source_path)
    target = Path(plan.source_fixture_path)
    if not source.exists():
        raise RuntimeError(f"selected source does not exist: {source}")
    if sha256_file(source) != plan.source_sha256:
        raise RuntimeError(f"selected source hash mismatch for {plan.manifest_path}")
    if target.exists():
        if sha256_file(target) != plan.source_sha256:
            raise RuntimeError(f"checked source fixture hash mismatch: {target}")
        return
    target.parent.mkdir(parents=True, exist_ok=True)
    shutil.copyfile(source, target)
    created_paths.append(target)


def run_metadata(
    plan: CandidatePlan,
    go_bin: str,
    metadata_smoke_fixture: str,
    created_paths: list[Path],
    *,
    refresh_stale_metadata: bool = False,
) -> None:
    # Inventory-as-metadata: validate the shared checked inventory fixture
    # instead of generating/validating a per-file metadata fixture.
    del metadata_smoke_fixture, created_paths, refresh_stale_metadata
    metadata_path = Path(plan.metadata_path)
    if not metadata_path.exists():
        raise RuntimeError(
            f"checked Go AST schema inventory fixture does not exist: {metadata_path}"
        )
    run_command(
        [
            go_bin,
            "run",
            "scripts/analysis/go_ast_schema_inventory.go",
            "--validate",
            plan.metadata_path,
            "--self-test",
        ]
    )


def run_projection(
    plan: CandidatePlan,
    go_bin: str,
    report_ingestion_manifest_path: Path,
    metadata_smoke_fixture: str,
    created_paths: list[Path],
    *,
    force_report_rewrite: bool = False,
) -> None:
    del metadata_smoke_fixture
    checked_report_path = Path(plan.checked_projection_report_path)
    base_command = [
        go_bin,
        "run",
        "scripts/analysis/go_syntax_source_instance_project.go",
        "--source",
        plan.source_fixture_path,
        "--metadata",
        plan.metadata_path,
        "--manifest-path",
        plan.manifest_path,
        "--candidate-grammar-id",
        plan.candidate_grammar_id,
        "--report-ingestion-manifest",
        repo_relative(report_ingestion_manifest_path),
        "--projection-output",
        plan.projection_output_path,
    ]
    existed_before = checked_report_path.exists()
    if force_report_rewrite or not existed_before:
        run_command(base_command + ["--report-output", plan.checked_projection_report_path])
        if not existed_before:
            created_paths.append(checked_report_path)
    run_command(base_command + ["--validate-report", plan.checked_projection_report_path])


def run_tokenizer_generator(plan: CandidatePlan, go_bin: str) -> tuple[dict[str, Any], dict[str, Any]]:
    run_command(
        [
            "cargo",
            "run",
            "--features",
            "experimental",
            "--bin",
            "tct-syntax-go-tokenizer-generator",
            "--",
            "--go-bin",
            go_bin,
            "--checked-report",
            plan.checked_projection_report_path,
            "--output",
            plan.tokenizer_report_path,
        ]
    )
    tokenizer_report = read_json(Path(plan.tokenizer_report_path))
    generated_report_path = tokenizer_report["generated_codec_report_path"]
    generated_report = read_json(Path(generated_report_path))
    return tokenizer_report, generated_report


def manifest_record_from_reports(
    plan: CandidatePlan,
    tokenizer_report: dict[str, Any],
    generated_report: dict[str, Any],
    per_file_grammar_blocker: str | None = None,
) -> dict[str, Any]:
    source = generated_report["source"]
    canonical = generated_report["canonical_render"]
    parser = generated_report["parser"]
    # Canonical (elided) stream fields are the model-facing contract after
    # fe5cd5379; raw_token_* remain expanded evidence on the same report.
    record = {
        "role": CURRENT_PLUS_ROLE,
        "evidence_scope": CURRENT_PLUS_EVIDENCE_SCOPE,
        "manifest_path": plan.manifest_path,
        "report_path": generated_report["report_path"],
        "generated_codec_route": generated_report["route"],
        "source_path": source["path"],
        "source_sha256": source["sha256"],
        "source_bytes": source["bytes"],
        "package": plan.package_name,
        "source_function": "source_file",
        "expected_tct_token_count": require_report_field(
            tokenizer_report, "canonical_token_count", report_label="tokenizer_report"
        ),
        "expected_tct_tokens_sha256": require_report_field(
            tokenizer_report, "canonical_tokens_sha256", report_label="tokenizer_report"
        ),
        "expected_max_raw_token_id": require_report_field(
            tokenizer_report, "canonical_max_token_id", report_label="tokenizer_report"
        ),
        "expected_vocab_size": require_report_field(
            tokenizer_report, "token_layout_vocab_size", report_label="tokenizer_report"
        ),
        "expected_pad_token": require_report_field(
            tokenizer_report, "token_layout_pad_token", report_label="tokenizer_report"
        ),
        "expected_pad_token_absent": require_report_field(
            tokenizer_report, "pad_token_absent", report_label="tokenizer_report"
        ),
        "go_parser_version": parser["runtime_go_version"],
        "generated_codec_report_artifact_kind": generated_report["artifact_kind"],
        "canonical_render": {
            "bytes": canonical["canonical_byte_count"],
            "sha256": canonical["canonical_sha256"],
        },
        "expected_claim_boundary": generated_report["claim_boundary"],
        "source_program_validity_claim_allowed": False,
        "stable_promotion_allowed": False,
    }
    if plan.candidate_grammar_id != GO_UNIFIED_GRAMMAR_ID:
        # Escape-hatch record: the deliberate per-file registration carries
        # its recorded blocker so the manifest evidence shows the exception.
        validate_forward_candidate_grammar_id(
            plan.candidate_grammar_id, per_file_grammar_blocker
        )
        record["per_file_grammar_blocker"] = {
            "kind": PER_FILE_GRAMMAR_BLOCKER_KIND,
            "reason": (per_file_grammar_blocker or "").strip(),
        }
    return record


def refresh_package_metadata(manifest: dict[str, Any], next_missing: str | None) -> None:
    record_count = len(manifest.get("records", []))
    last_manifest_path = manifest["records"][-1]["manifest_path"] if record_count else "none"
    status = f"current_plus_{record_count}_record_generated_codec_gate_sidecar"
    completed = (
        f"syntax backend Go current-plus {record_count}-record generated-codec round-trip manifest"
    )
    if next_missing:
        blocker = (
            f"current-plus report ingestion retains {record_count} parser-backed Go "
            f"source-instance generated-codec records through {last_manifest_path} without "
            "promoting source-program validity, type correctness, arena IR, BPE, generated "
            f"hot paths, paper-facing claims, or stable backend status; the refreshed "
            f"checker-ready generated-codec gate blocks at {next_missing}."
        )
    else:
        blocker = (
            f"current-plus report ingestion retains {record_count} parser-backed Go "
            "source-instance generated-codec records and the current checker-ready "
            "selection has no remaining missing generated-codec ingestion record."
        )
    manifest.setdefault("package_metadata", {})
    manifest["package_metadata"]["status"] = status
    manifest["package_metadata"]["completed_promotion_class"] = completed
    manifest["package_metadata"]["next_missing_workstream_blocker"] = blocker
    stamp_package_metadata_identity(manifest)


def stamp_package_metadata_identity(manifest: dict[str, Any]) -> None:
    """Stamp the unified-artifact and stream-contract identity from the
    checked grammar-artifact pin fixture so the manifest can never drift from
    the pinned identity without the pin gate seeing it first."""
    artifact_pin = read_json(Path(GO_GRAMMAR_ARTIFACT_PIN_FIXTURE))
    manifest.setdefault("package_metadata", {})
    manifest["package_metadata"]["stream_contract"] = RAW_SYNTAX_STREAM_CONTRACT_ELIDED_V1
    manifest["package_metadata"]["grammar_artifact_id"] = artifact_pin["artifact_id"]
    manifest["package_metadata"]["grammar_artifact_content_sha256"] = artifact_pin[
        "content_sha256"
    ]


def next_missing_after(selection: dict[str, Any], manifest: dict[str, Any]) -> str | None:
    ingested_paths, ingested_shas = ingested_keys(manifest)
    for candidate in selected_candidates(selection):
        manifest_path = candidate.get("manifest_path")
        sha = candidate.get("file", {}).get("sha256")
        if manifest_path not in ingested_paths and sha not in ingested_shas:
            return manifest_path
    return None


def rollback_created(created_paths: list[Path]) -> None:
    for path in reversed(created_paths):
        try:
            if path.exists():
                path.unlink()
        except OSError:
            pass


def run_record_pipeline(
    plan: CandidatePlan,
    *,
    go_bin: str,
    metadata_smoke_fixture: str,
    report_ingestion_manifest_path: Path,
    created_paths: list[Path],
    force_report_rewrite: bool = False,
    per_file_grammar_blocker: str | None = None,
) -> dict[str, Any]:
    validate_forward_candidate_grammar_id(
        plan.candidate_grammar_id, per_file_grammar_blocker
    )
    copy_source(plan, created_paths)
    run_metadata(
        plan,
        go_bin,
        metadata_smoke_fixture,
        created_paths,
        refresh_stale_metadata=force_report_rewrite,
    )
    run_projection(
        plan,
        go_bin,
        report_ingestion_manifest_path,
        metadata_smoke_fixture,
        created_paths,
        force_report_rewrite=force_report_rewrite,
    )
    tokenizer_report, generated_report = run_tokenizer_generator(plan, go_bin)
    record = manifest_record_from_reports(
        plan, tokenizer_report, generated_report, per_file_grammar_blocker
    )
    if record["generated_codec_route"] != GENERATED_CODEC_ROUTE:
        raise RuntimeError(f"unexpected generated codec route for {plan.manifest_path}")
    if record["generated_codec_report_artifact_kind"] != GENERATED_CODEC_REPORT_ARTIFACT_KIND:
        raise RuntimeError(f"unexpected generated codec artifact kind for {plan.manifest_path}")
    return record


def apply_plan(
    plans: list[CandidatePlan],
    *,
    selection: dict[str, Any],
    report_ingestion_manifest: dict[str, Any],
    report_ingestion_manifest_path: Path,
    go_bin: str,
    metadata_smoke_fixture: str,
    per_file_grammar_blocker: str | None = None,
) -> list[dict[str, Any]]:
    created_paths: list[Path] = []
    new_records: list[dict[str, Any]] = []
    try:
        for plan in plans:
            new_records.append(
                run_record_pipeline(
                    plan,
                    go_bin=go_bin,
                    metadata_smoke_fixture=metadata_smoke_fixture,
                    report_ingestion_manifest_path=report_ingestion_manifest_path,
                    created_paths=created_paths,
                    per_file_grammar_blocker=per_file_grammar_blocker,
                )
            )
    except Exception:
        rollback_created(created_paths)
        raise

    report_ingestion_manifest.setdefault("records", []).extend(new_records)
    refresh_package_metadata(report_ingestion_manifest, next_missing_after(selection, report_ingestion_manifest))
    write_json(report_ingestion_manifest_path, report_ingestion_manifest)
    return new_records


def apply_refresh(
    plans: list[CandidatePlan],
    *,
    report_ingestion_manifest: dict[str, Any],
    report_ingestion_manifest_path: Path,
    go_bin: str,
    metadata_smoke_fixture: str,
) -> list[dict[str, Any]]:
    records = report_ingestion_manifest.get("records", [])
    if len(plans) != len(records):
        raise RuntimeError(
            f"refresh plan covers {len(plans)} records but the manifest has {len(records)}"
        )
    created_paths: list[Path] = []
    refreshed_records: list[dict[str, Any]] = []
    try:
        for index, plan in enumerate(plans):
            record = run_record_pipeline(
                plan,
                go_bin=go_bin,
                metadata_smoke_fixture=metadata_smoke_fixture,
                report_ingestion_manifest_path=report_ingestion_manifest_path,
                created_paths=created_paths,
                force_report_rewrite=True,
            )
            # Preserve maintained sidecar keys the pipeline does not regenerate
            # (for example raw_tree_render_attempt selection markers).
            refreshed = dict(records[index])
            refreshed.update(record)
            records[index] = refreshed
            refreshed_records.append(refreshed)
    except Exception:
        rollback_created(created_paths)
        raise

    stamp_package_metadata_identity(report_ingestion_manifest)
    write_json(report_ingestion_manifest_path, report_ingestion_manifest)
    return refreshed_records


def plan_to_json(plan: CandidatePlan) -> dict[str, Any]:
    return {
        "manifest_path": plan.manifest_path,
        "source_path": plan.source_path,
        "source_sha256": plan.source_sha256,
        "source_bytes": plan.source_bytes,
        "package": plan.package_name,
        "selection_order": plan.selection_order,
        "source_fixture_path": plan.source_fixture_path,
        "metadata_path": plan.metadata_path,
        "checked_projection_report_path": plan.checked_projection_report_path,
        "projection_output_path": plan.projection_output_path,
        "tokenizer_report_path": plan.tokenizer_report_path,
        "candidate_grammar_id": plan.candidate_grammar_id,
    }


def build_report(
    *,
    selection_path: Path,
    checker_path: Path,
    manifest_path: Path,
    selection: dict[str, Any],
    checker: dict[str, Any],
    manifest: dict[str, Any],
    plans: list[CandidatePlan],
    apply: bool,
    applied_records: list[dict[str, Any]],
    blocker: dict[str, str] | None,
    max_new_records: int,
    per_file_grammar_blocker: str | None = None,
) -> dict[str, Any]:
    if blocker:
        status = "blocked"
    elif apply:
        status = "applied"
    elif plans:
        status = "ready"
    else:
        status = "no_new_records"
    return {
        "artifact_kind": ARTIFACT_KIND,
        "version": VERSION,
        "tool": TOOL,
        "mode": "missing_record_intake",
        "status": status,
        "apply": apply,
        "max_new_records": max_new_records,
        "selection_report": {
            "path": repo_relative(selection_path),
            "sha256": sha256_file(selection_path),
            "selected_count": selection.get("selected_count"),
        },
        "checker_report": {
            "path": repo_relative(checker_path),
            "sha256": sha256_file(checker_path),
            "status": checker.get("status"),
        },
        "report_ingestion_manifest": {
            "path": repo_relative(manifest_path),
            "sha256": sha256_file(manifest_path),
            "record_count": len(manifest.get("records", [])),
        },
        "planned_record_count": len(plans),
        "applied_record_count": len(applied_records),
        "first_blocker": blocker,
        "per_file_grammar_blocker": (
            {
                "kind": PER_FILE_GRAMMAR_BLOCKER_KIND,
                "reason": per_file_grammar_blocker.strip(),
            }
            if per_file_grammar_blocker and per_file_grammar_blocker.strip()
            else None
        ),
        "planned_records": [plan_to_json(plan) for plan in plans],
        "applied_records": applied_records,
        "next_concrete_task": (
            "run this helper with --apply for the planned checker-ready records"
            if plans and not apply
            else "fix the first reusable generated-codec intake blocker"
            if blocker
            else "run the generated-codec round-trip gate and use this helper for the next batch or blocker"
        ),
        "claim_boundary": (
            "batch intake automates parser-backed Go source fixtures, checked metadata, "
            "source-instance projection reports, target/ generated-codec reports, and "
            "current-plus report-ingestion records only; it does not promote source "
            "validity, type correctness, ArenaIR, BPE, generated hot paths, paper claims, "
            "or stable backend status"
        ),
    }


def build_refresh_report(
    *,
    manifest_path: Path,
    manifest: dict[str, Any],
    plans: list[CandidatePlan],
    apply: bool,
    refreshed_records: list[dict[str, Any]],
    blocker: dict[str, str] | None,
) -> dict[str, Any]:
    if blocker:
        status = "blocked"
    elif apply:
        status = "applied"
    elif plans:
        status = "ready"
    else:
        status = "no_existing_records"
    return {
        "artifact_kind": ARTIFACT_KIND,
        "version": VERSION,
        "tool": TOOL,
        "mode": "refresh_existing",
        "status": status,
        "apply": apply,
        "report_ingestion_manifest": {
            "path": repo_relative(manifest_path),
            "sha256": sha256_file(manifest_path),
            "record_count": len(manifest.get("records", [])),
        },
        "planned_record_count": len(plans),
        "refreshed_record_count": len(refreshed_records),
        "first_blocker": blocker,
        "rewrite_targets": {
            "report_ingestion_manifest_path": repo_relative(manifest_path),
            "checked_projection_report_paths": [
                plan.checked_projection_report_path for plan in plans
            ],
        },
        "planned_records": [plan_to_json(plan) for plan in plans],
        "next_concrete_task": (
            "run this helper with --refresh-existing --apply to rewrite every ingested record"
            if plans and not apply
            else "fix the first reusable generated-codec refresh blocker"
            if blocker
            else "run the generated-codec round-trip gate against the refreshed corpus"
        ),
        "claim_boundary": (
            "refresh re-runs parser-backed Go source fixtures, checked metadata, "
            "source-instance projection reports, target/ generated-codec reports, and "
            "current-plus report-ingestion records only; it does not promote source "
            "validity, type correctness, ArenaIR, BPE, generated hot paths, paper claims, "
            "or stable backend status"
        ),
    }


def emit_report(report: dict[str, Any], output: Path | None) -> None:
    if output:
        write_json(output, report)
    else:
        print(json.dumps(report, indent=2))


def run_self_test() -> None:
    assert underscore_slug("debug/pe/symbol.go") == "debug_pe_symbol"
    assert dash_slug("encoding/gob/decoder.go") == "encoding-gob-decoder"
    # M1 unified artifact: every record carries the one inventory grammar id
    # and projects against the checked inventory fixture.
    assert grammar_id_for_manifest_path("encoding/gob/decoder.go") == GO_UNIFIED_GRAMMAR_ID
    checked_manifest = read_json(Path(DEFAULT_REPORT_INGESTION_MANIFEST))
    package_metadata = checked_manifest.get("package_metadata", {})
    assert package_metadata.get("stream_contract") == RAW_SYNTAX_STREAM_CONTRACT_ELIDED_V1
    artifact_pin = read_json(Path(GO_GRAMMAR_ARTIFACT_PIN_FIXTURE))
    assert GO_GRAMMAR_ARTIFACT_ID == artifact_pin["artifact_id"]
    assert package_metadata.get("grammar_artifact_id") == artifact_pin["artifact_id"]
    assert (
        package_metadata.get("grammar_artifact_content_sha256")
        == artifact_pin["content_sha256"]
    )

    # Forward gate (M1): per-file candidate grammar ids are refused without a
    # recorded blocker, and the refusal names the unified artifact.
    validate_forward_candidate_grammar_id(GO_UNIFIED_GRAMMAR_ID)
    try:
        validate_forward_candidate_grammar_id("go_synthetic_example_source_file_syntax_v0")
    except ValueError as error:
        message = str(error)
        assert GO_GRAMMAR_ARTIFACT_ID in message
        assert GO_UNIFIED_GRAMMAR_ID in message
        assert "--per-file-grammar-blocker" in message
    else:
        raise AssertionError(
            "per-file grammar id without a recorded blocker must be refused"
        )
    try:
        validate_forward_candidate_grammar_id(
            GO_UNIFIED_GRAMMAR_ID, "blocked: spurious blocker on the unified id"
        )
    except ValueError as error:
        assert GO_UNIFIED_GRAMMAR_ID in str(error)
    else:
        raise AssertionError(
            "a per-file grammar blocker on the unified grammar id must be refused"
        )
    # Escape hatch: an explicit recorded blocker mints the historical
    # per-file id shape and passes the forward validator.
    blocker_reason = "blocked: deliberate historical per-file regression witness"
    minted = grammar_id_for_manifest_path("x/y.go", blocker_reason)
    assert minted == "go_x_y_source_file_syntax_v0"
    validate_forward_candidate_grammar_id(minted, blocker_reason)
    try:
        grammar_id_for_manifest_path("x/y.go", "   ")
    except ValueError as error:
        assert GO_GRAMMAR_ARTIFACT_ID in str(error)
    else:
        raise AssertionError("a blank per-file grammar blocker reason must be refused")
    selection = {
        "selected_candidates": [
            {
                "manifest_path": "already/in.go",
                "source_path": __file__,
                "selected": True,
                "selection_order": 1,
                "file": {"sha256": "a" * 64, "bytes": 10},
                "package": "already",
            },
            {
                "manifest_path": "encoding/gob/decoder.go",
                "source_path": __file__,
                "selected": True,
                "selection_order": 2,
                "file": {"sha256": "b" * 64, "bytes": 20},
                "package": "gob",
            },
        ]
    }
    checker = {
        "status": "ready",
        "unsupported_constructs": [],
        "unsupported_grammar_gaps": [],
        "next_missing_construct": None,
        "next_missing_grammar_gap": None,
        "files": [
            {"sha256": "a" * 64, "unsupported_constructs": [], "unsupported_grammar_gaps": []},
            {"sha256": "b" * 64, "unsupported_constructs": [], "unsupported_grammar_gaps": []},
        ],
    }
    manifest = {"records": [{"manifest_path": "already/in.go", "source_sha256": "a" * 64}]}
    plans = build_batch_plan(
        selection,
        checker,
        manifest,
        max_new_records=8,
        go_bin="go",
        source_fixture_root=Path(DEFAULT_SOURCE_FIXTURE_ROOT),
        metadata_root=Path(DEFAULT_METADATA_ROOT),
        checked_report_root=Path(DEFAULT_CHECKED_REPORT_ROOT),
        projection_output_root=Path(DEFAULT_PROJECTION_OUTPUT_ROOT),
        tokenizer_output_root=Path(DEFAULT_TOKENIZER_OUTPUT_ROOT),
    )
    assert len(plans) == 1
    assert plans[0].manifest_path == "encoding/gob/decoder.go"
    assert plans[0].metadata_path == GO_AST_SCHEMA_INVENTORY_FIXTURE
    assert plans[0].candidate_grammar_id == GO_UNIFIED_GRAMMAR_ID

    with tempfile.TemporaryDirectory(prefix="tct-current-plus-refresh-self-test-") as tempdir:
        synthetic_manifest_path = Path(tempdir) / "training-report-ingestion-current-plus-v1.json"
        write_json(
            synthetic_manifest_path,
            {
                "records": [
                    {
                        "manifest_path": "already/in.go",
                        "source_path": "tests/fixtures/go_syntax_corpus/src/already/in.go",
                        "source_sha256": "a" * 64,
                        "source_bytes": 10,
                        "package": "already",
                        "raw_tree_render_attempt": True,
                    },
                    {
                        "manifest_path": "encoding/gob/decoder.go",
                        "source_path": "tests/fixtures/go_syntax_corpus/src/encoding/gob/decoder.go",
                        "source_sha256": "b" * 64,
                        "source_bytes": 20,
                        "package": "gob",
                    },
                ]
            },
        )
        manifest_sha_before = sha256_file(synthetic_manifest_path)
        refresh_report_path = Path(tempdir) / "refresh-plan-report.json"
        exit_code = main(
            [
                "--refresh-existing",
                "--report-ingestion-manifest",
                str(synthetic_manifest_path),
                "--output",
                str(refresh_report_path),
            ]
        )
        assert exit_code == 0
        assert sha256_file(synthetic_manifest_path) == manifest_sha_before
        refresh_report = read_json(refresh_report_path)
        assert refresh_report["mode"] == "refresh_existing"
        assert refresh_report["status"] == "ready"
        assert refresh_report["apply"] is False
        assert refresh_report["planned_record_count"] == 2
        assert refresh_report["refreshed_record_count"] == 0
        assert [record["manifest_path"] for record in refresh_report["planned_records"]] == [
            "already/in.go",
            "encoding/gob/decoder.go",
        ]
        assert refresh_report["rewrite_targets"]["checked_projection_report_paths"] == [
            "tests/fixtures/go_syntax_corpus/source-instance-projection-report-already-in-v1.json",
            "tests/fixtures/go_syntax_corpus/source-instance-projection-report-encoding-gob-decoder-v1.json",
        ]


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--selection-report", type=Path)
    parser.add_argument("--checker-report", type=Path)
    parser.add_argument(
        "--report-ingestion-manifest",
        type=Path,
        default=Path(DEFAULT_REPORT_INGESTION_MANIFEST),
    )
    parser.add_argument("--source-fixture-root", type=Path, default=Path(DEFAULT_SOURCE_FIXTURE_ROOT))
    parser.add_argument("--metadata-root", type=Path, default=Path(DEFAULT_METADATA_ROOT))
    parser.add_argument("--checked-report-root", type=Path, default=Path(DEFAULT_CHECKED_REPORT_ROOT))
    parser.add_argument("--projection-output-root", type=Path, default=Path(DEFAULT_PROJECTION_OUTPUT_ROOT))
    parser.add_argument("--tokenizer-output-root", type=Path, default=Path(DEFAULT_TOKENIZER_OUTPUT_ROOT))
    parser.add_argument("--metadata-smoke-fixture", default=DEFAULT_METADATA_SMOKE_FIXTURE)
    parser.add_argument("--go-bin", default=default_go_bin())
    parser.add_argument(
        "--max-new-records",
        type=int,
        default=None,
        help=f"missing-record intake batch size (default {DEFAULT_MAX_NEW_RECORDS})",
    )
    parser.add_argument(
        "--refresh-existing",
        action="store_true",
        help=(
            "re-run the metadata/projection/tokenizer-generator pipeline for every "
            "already-ingested manifest record instead of intaking missing records; "
            "plan-only without --apply"
        ),
    )
    parser.add_argument(
        "--per-file-grammar-blocker",
        help=(
            "explicit recorded-blocker escape hatch: mint retired per-file "
            "go_*_source_file_syntax_v0 candidate grammar ids for this intake batch "
            "instead of the unified pinned grammar artifact "
            f"{GO_GRAMMAR_ARTIFACT_ID} (grammar id {GO_UNIFIED_GRAMMAR_ID}), recording "
            "the given reason in the intake report and the minted manifest records; "
            "without this flag per-file grammar registrations are refused"
        ),
    )
    parser.add_argument("--apply", action="store_true")
    parser.add_argument("--output", type=Path)
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args(argv)
    if args.self_test:
        return args
    if args.refresh_existing:
        if args.per_file_grammar_blocker is not None:
            parser.error(
                "--refresh-existing re-validates already-ingested records against the "
                "unified grammar artifact and cannot be combined with the per-file "
                "registration escape hatch --per-file-grammar-blocker"
            )
        if args.max_new_records is not None:
            parser.error(
                "--refresh-existing re-runs every already-ingested record and cannot be "
                "combined with the missing-record intake flag --max-new-records; "
                "please drop one of the two flags"
            )
        if args.selection_report is not None or args.checker_report is not None:
            parser.error(
                "--refresh-existing iterates the report-ingestion manifest records and "
                "cannot be combined with the missing-record intake flags "
                "--selection-report/--checker-report; please drop one mode"
            )
        return args
    if args.selection_report is None:
        parser.error("--selection-report is required unless --self-test or --refresh-existing is set")
    if args.checker_report is None:
        parser.error("--checker-report is required unless --self-test or --refresh-existing is set")
    if args.max_new_records is None:
        args.max_new_records = DEFAULT_MAX_NEW_RECORDS
    if args.max_new_records <= 0:
        parser.error("--max-new-records must be positive")
    return args


def main(argv: list[str]) -> int:
    args = parse_args(argv)
    if args.self_test:
        run_self_test()
        print("go syntax current-plus batch intake self-test passed")
        return 0

    if args.refresh_existing:
        manifest = read_json(args.report_ingestion_manifest)
        blocker = None
        refreshed_records: list[dict[str, Any]] = []
        try:
            plans = build_refresh_plans(
                manifest,
                source_fixture_root=args.source_fixture_root,
                metadata_root=args.metadata_root,
                checked_report_root=args.checked_report_root,
                projection_output_root=args.projection_output_root,
                tokenizer_output_root=args.tokenizer_output_root,
            )
            if args.apply and plans:
                refreshed_records = apply_refresh(
                    plans,
                    report_ingestion_manifest=manifest,
                    report_ingestion_manifest_path=args.report_ingestion_manifest,
                    go_bin=args.go_bin,
                    metadata_smoke_fixture=args.metadata_smoke_fixture,
                )
        except Exception as error:  # noqa: BLE001 - report precise reusable blocker to JSON.
            plans = []
            blocker = {
                "kind": "go_current_plus_refresh_existing_blocked",
                "path": repo_relative(args.report_ingestion_manifest),
                "reason": str(error),
            }
        report = build_refresh_report(
            manifest_path=args.report_ingestion_manifest,
            manifest=manifest,
            plans=plans,
            apply=args.apply,
            refreshed_records=refreshed_records,
            blocker=blocker,
        )
        emit_report(report, args.output)
        return 1 if blocker else 0

    selection = read_json(args.selection_report)
    checker = read_json(args.checker_report)
    manifest = read_json(args.report_ingestion_manifest)
    blocker = None
    applied_records: list[dict[str, Any]] = []
    try:
        plans = build_batch_plan(
            selection,
            checker,
            manifest,
            max_new_records=args.max_new_records,
            go_bin=args.go_bin,
            source_fixture_root=args.source_fixture_root,
            metadata_root=args.metadata_root,
            checked_report_root=args.checked_report_root,
            projection_output_root=args.projection_output_root,
            tokenizer_output_root=args.tokenizer_output_root,
            per_file_grammar_blocker=args.per_file_grammar_blocker,
        )
        if args.apply and plans:
            applied_records = apply_plan(
                plans,
                selection=selection,
                report_ingestion_manifest=manifest,
                report_ingestion_manifest_path=args.report_ingestion_manifest,
                go_bin=args.go_bin,
                metadata_smoke_fixture=args.metadata_smoke_fixture,
                per_file_grammar_blocker=args.per_file_grammar_blocker,
            )
    except Exception as error:  # noqa: BLE001 - report precise reusable blocker to JSON.
        plans = []
        blocker = {
            "kind": "go_current_plus_batch_intake_blocked",
            "path": repo_relative(args.report_ingestion_manifest),
            "reason": str(error),
        }

    report = build_report(
        selection_path=args.selection_report,
        checker_path=args.checker_report,
        manifest_path=args.report_ingestion_manifest,
        selection=selection,
        checker=checker,
        manifest=manifest,
        plans=plans,
        apply=args.apply,
        applied_records=applied_records,
        blocker=blocker,
        max_new_records=args.max_new_records,
        per_file_grammar_blocker=args.per_file_grammar_blocker,
    )
    emit_report(report, args.output)
    return 1 if blocker else 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
