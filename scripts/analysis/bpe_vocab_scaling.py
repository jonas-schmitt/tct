#!/usr/bin/env python3
"""Summarize BPE total-vocabulary scaling measurements."""

from __future__ import annotations

import argparse
import hashlib
import json
import sys
from pathlib import Path
from typing import Any


TOOL_NAME = "bpe_vocab_scaling"
MANIFEST_VERSION = 1
ARTIFACT_KIND = "tct_bpe_vocab_scaling_summary"
GRID_ORDER = ("base", "512", "1024", "2048", "4096")
AVERAGE_REDUCTION_THRESHOLD = 0.10
CONTEXT_PASS_THRESHOLD = 0.99
CONTEXT_WINDOW_TOKENS = 2048
REQUIRED_STATS = (
    "records",
    "total_tokens",
    "average_tokens",
    "p95_tokens",
    "p99_tokens",
    "context_window_tokens",
    "context_window_pass_rate",
)


def repo_root() -> Path:
    return Path(__file__).resolve().parents[2]


def relpath(path: Path, root: Path) -> str:
    try:
        return path.resolve().relative_to(root.resolve()).as_posix()
    except ValueError:
        return path.as_posix()


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def load_json(path: Path) -> dict[str, Any]:
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except FileNotFoundError as error:
        raise SystemExit(f"error: JSON file does not exist: {path}") from error
    except json.JSONDecodeError as error:
        raise SystemExit(f"error: invalid JSON in {path}: {error}") from error
    if not isinstance(data, dict):
        raise SystemExit(f"error: JSON file must contain an object: {path}")
    return data


def parse_entry(value: str) -> tuple[str, str, Path]:
    parts = value.split("=", 2)
    if len(parts) != 3:
        raise argparse.ArgumentTypeError(
            "--entry must use SCHEMA=GRID=MEASUREMENT_JSON"
        )
    schema, grid, path = parts
    if not schema:
        raise argparse.ArgumentTypeError("--entry schema must not be empty")
    if grid not in GRID_ORDER:
        allowed = ", ".join(GRID_ORDER)
        raise argparse.ArgumentTypeError(f"--entry grid must be one of: {allowed}")
    if not path:
        raise argparse.ArgumentTypeError("--entry path must not be empty")
    return schema, grid, Path(path)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description=(
            "Aggregate tct-measure-tokens JSON reports for the maintained "
            "base/512/1024/2048/4096 total-VOCAB_SIZE BPE investigation."
        )
    )
    parser.add_argument(
        "--entry",
        action="append",
        type=parse_entry,
        required=True,
        metavar="SCHEMA=GRID=MEASUREMENT_JSON",
        help="Measurement report for one schema and grid point.",
    )
    parser.add_argument(
        "--json-output",
        help="Write the summary JSON here. Defaults to stdout when omitted.",
    )
    parser.add_argument(
        "--markdown-output",
        help="Optional Markdown table output path.",
    )
    parser.add_argument(
        "--require-ready",
        action="store_true",
        help="Exit nonzero if any required grid, measurement, or provenance field is missing.",
    )
    return parser.parse_args()


def is_number(value: Any) -> bool:
    return isinstance(value, (int, float)) and not isinstance(value, bool)


def as_float(value: Any) -> float | None:
    return float(value) if is_number(value) else None


def sidecar_path_for_merge(path: Path) -> Path:
    if path.suffix == ".json":
        return path.with_suffix(".provenance.json")
    return Path(f"{path}.provenance.json")


def candidate_sidecar_paths_for_merge(path: Path) -> list[Path]:
    candidates = [sidecar_path_for_merge(path)]
    if path.suffix == ".json":
        prefix, separator, target = path.stem.rpartition("-")
        if separator and target.isdigit():
            candidates.append(path.with_name(f"{prefix}.provenance.json"))
    return list(dict.fromkeys(candidates))


def resolve_path(path_value: str, *, root: Path, report_path: Path) -> Path:
    candidate = Path(path_value)
    if candidate.is_absolute():
        return candidate
    report_relative = report_path.parent / candidate
    if report_relative.exists():
        return report_relative
    return root / candidate


def source_file_record(path: Path, root: Path) -> dict[str, Any]:
    return {
        "path": relpath(path, root),
        "bytes": path.stat().st_size,
        "sha256": sha256_file(path),
    }


def fingerprint_records(records: Any) -> str | None:
    if not isinstance(records, list):
        return None
    digest = hashlib.sha256()
    digest.update(b"tct-bpe-vocab-scaling-inputs-v1\n")
    for record in records:
        if not isinstance(record, dict):
            return None
        path = record.get("path")
        bytes_value = record.get("bytes")
        sha256 = record.get("sha256")
        if not isinstance(path, str) or not isinstance(bytes_value, int):
            return None
        if not isinstance(sha256, str):
            return None
        digest.update(path.encode("utf-8"))
        digest.update(b"\0")
        digest.update(str(bytes_value).encode("ascii"))
        digest.update(b"\0")
        digest.update(sha256.encode("ascii"))
        digest.update(b"\n")
    return digest.hexdigest()


def load_training_provenance(
    report: dict[str, Any],
    *,
    root: Path,
    report_path: Path,
    gaps: list[str],
) -> tuple[dict[str, Any] | None, dict[str, Any] | None]:
    provenance = report.get("provenance")
    if not isinstance(provenance, dict):
        gaps.append("missing measurement provenance")
        return None, None

    merge_file = provenance.get("bpe_merges_file")
    if not isinstance(merge_file, dict):
        return None, None
    merge_path_value = merge_file.get("path")
    if not isinstance(merge_path_value, str):
        gaps.append("BPE measurement provenance is missing bpe_merges_file.path")
        return merge_file, None

    merge_path = resolve_path(merge_path_value, root=root, report_path=report_path)
    candidate_sidecars = candidate_sidecar_paths_for_merge(merge_path)
    sidecar_path = next((path for path in candidate_sidecars if path.is_file()), None)
    if sidecar_path is None:
        expected = ", ".join(relpath(path, root) for path in candidate_sidecars)
        gaps.append(
            "missing training provenance sidecar for BPE merge table: "
            f"{expected}"
        )
        return merge_file, None
    return merge_file, {
        "path": relpath(sidecar_path, root),
        "bytes": sidecar_path.stat().st_size,
        "sha256": sha256_file(sidecar_path),
        "data": load_json(sidecar_path),
    }


def provenance_mode_for_requested_vocab(
    training: dict[str, Any],
    requested_vocab: int | None,
) -> str | None:
    if requested_vocab is None:
        return None
    if training.get("target_vocab") == requested_vocab:
        return "single_target"
    checkpoints = training.get("checkpoints")
    if isinstance(checkpoints, list):
        for checkpoint in checkpoints:
            if checkpoint == requested_vocab:
                return "checkpoint_set"
    return None


def summarize_entry(schema: str, grid: str, path: Path, root: Path) -> dict[str, Any]:
    report = load_json(path)
    gaps: list[str] = []
    stats = report.get("token_length_stats")
    if not isinstance(stats, dict):
        stats = {}
        gaps.append("missing token_length_stats")

    for field in REQUIRED_STATS:
        if field not in stats:
            gaps.append(f"missing token_length_stats.{field}")

    if stats.get("context_window_tokens") != CONTEXT_WINDOW_TOKENS:
        gaps.append(
            "token_length_stats.context_window_tokens is not "
            f"{CONTEXT_WINDOW_TOKENS}"
        )
    records = stats.get("records")
    if isinstance(records, int) and records <= 0:
        gaps.append("token_length_stats.records is zero")

    average_tokens = as_float(stats.get("average_tokens"))
    p95_tokens = as_float(stats.get("p95_tokens"))
    p99_tokens = as_float(stats.get("p99_tokens"))
    context_pass_rate = as_float(stats.get("context_window_pass_rate"))
    if average_tokens is None:
        gaps.append("token_length_stats.average_tokens is not numeric")
    if p95_tokens is None:
        gaps.append("token_length_stats.p95_tokens is not numeric")
    if p99_tokens is None:
        gaps.append("token_length_stats.p99_tokens is not numeric")
    if context_pass_rate is None:
        gaps.append("token_length_stats.context_window_pass_rate is not numeric")

    has_bpe = report.get("has_bpe")
    if grid == "base" and has_bpe is True:
        gaps.append("base grid entry is marked has_bpe=true")
    if grid != "base" and has_bpe is not True:
        gaps.append(f"{grid} grid entry is not marked has_bpe=true")

    measurement_provenance = (
        report.get("provenance") if isinstance(report.get("provenance"), dict) else {}
    )
    input_files = measurement_provenance.get("input_files")
    measurement_input_fingerprint = fingerprint_records(input_files)
    encoded_input_file = measurement_provenance.get("encoded_input_file")
    if measurement_input_fingerprint is None and isinstance(encoded_input_file, dict):
        encoded_sha = encoded_input_file.get("sha256")
        if isinstance(encoded_sha, str):
            measurement_input_fingerprint = encoded_sha

    merge_file, training_sidecar = load_training_provenance(
        report, root=root, report_path=path, gaps=gaps
    )
    training = (
        training_sidecar.get("data")
        if isinstance(training_sidecar, dict)
        and isinstance(training_sidecar.get("data"), dict)
        else {}
    )
    training_timings = training.get("timings") if isinstance(training, dict) else {}
    if not isinstance(training_timings, dict):
        training_timings = {}

    trainer_wall_ns = training_timings.get("training_wall_time_ns")
    if grid != "base" and not isinstance(trainer_wall_ns, int):
        gaps.append("missing training timings.training_wall_time_ns")
    trainer_wall_ms = trainer_wall_ns / 1_000_000 if isinstance(trainer_wall_ns, int) else None

    requested_vocab = None if grid == "base" else int(grid)
    training_provenance_mode = provenance_mode_for_requested_vocab(
        training, requested_vocab
    )
    if requested_vocab is not None and training_provenance_mode is None:
        gaps.append(
            "training provenance target_vocab/checkpoints does not cover grid: "
            f"{training.get('target_vocab')!r}/{training.get('checkpoints')!r} "
            f"!= {requested_vocab}"
        )

    merge_bytes = None
    if isinstance(merge_file, dict):
        bytes_value = merge_file.get("bytes")
        if isinstance(bytes_value, int):
            merge_bytes = bytes_value
        else:
            merge_path_value = merge_file.get("path")
            if isinstance(merge_path_value, str):
                merge_path = resolve_path(merge_path_value, root=root, report_path=path)
                if merge_path.is_file():
                    merge_bytes = merge_path.stat().st_size

    training_corpus = training.get("token_corpus") if isinstance(training, dict) else {}
    if not isinstance(training_corpus, dict):
        training_corpus = {}
    training_corpus_fingerprint = (
        training.get("input_corpus_fingerprint") if isinstance(training, dict) else None
    )
    if not isinstance(training_corpus_fingerprint, str):
        training_corpus_fingerprint = measurement_input_fingerprint

    if grid != "base" and not isinstance(training_corpus_fingerprint, str):
        gaps.append("missing training corpus fingerprint")

    source_training_provenance = None
    if isinstance(training_sidecar, dict):
        source_training_provenance = {
            key: training_sidecar[key]
            for key in ("path", "bytes", "sha256")
            if key in training_sidecar
        }

    return {
        "schema": schema,
        "grid": grid,
        "requested_total_vocab_size": requested_vocab,
        "actual_generated_vocab_size": report.get("generated_vocab_size"),
        "has_bpe": bool(has_bpe),
        "records": stats.get("records"),
        "total_tokens": stats.get("total_tokens"),
        "average_tokens": average_tokens,
        "p95_tokens": p95_tokens,
        "p99_tokens": p99_tokens,
        "context_window_tokens": stats.get("context_window_tokens"),
        "context_window_pass_rate": context_pass_rate,
        "final_merge_count": report.get("bpe_merges_count", training.get("generated_merges")),
        "artifact_bytes": merge_bytes,
        "training_corpus_size": training_corpus.get(
            "sequence_count", training.get("trained_on_instances")
        ),
        "training_corpus_fingerprint": training_corpus_fingerprint,
        "training_provenance_mode": training_provenance_mode,
        "trainer_wall_time_ns": trainer_wall_ns if isinstance(trainer_wall_ns, int) else None,
        "trainer_wall_time_ms": trainer_wall_ms,
        "source_report": source_file_record(path, root),
        "source_training_provenance": source_training_provenance,
        "gaps": gaps,
    }


def grid_index(entry: dict[str, Any]) -> int:
    return GRID_ORDER.index(entry["grid"])


def annotate_schema_entries(entries: list[dict[str, Any]]) -> dict[str, Any]:
    entries = sorted(entries, key=grid_index)
    missing_grid = [grid for grid in GRID_ORDER if grid not in {entry["grid"] for entry in entries}]
    schema_gaps = [f"missing grid entry: {grid}" for grid in missing_grid]

    previous: dict[str, Any] | None = None
    for entry in entries:
        average = entry.get("average_tokens")
        pass_rate = entry.get("context_window_pass_rate")
        marginal = None
        context_recovery = None
        if previous is not None:
            previous_average = previous.get("average_tokens")
            previous_pass_rate = previous.get("context_window_pass_rate")
            if is_number(average) and is_number(previous_average) and previous_average > 0:
                marginal = (previous_average - average) / previous_average
            if is_number(pass_rate) and is_number(previous_pass_rate):
                context_recovery = (
                    previous_pass_rate < CONTEXT_PASS_THRESHOLD
                    and pass_rate >= CONTEXT_PASS_THRESHOLD
                )

        if entry["grid"] == "base":
            decision = "baseline"
            reasonable = True
        elif entry["gaps"]:
            decision = "incomplete"
            reasonable = False
        elif marginal is not None and marginal >= AVERAGE_REDUCTION_THRESHOLD:
            decision = "reasonable_average_reduction"
            reasonable = True
        elif context_recovery:
            decision = "reasonable_context_recovery"
            reasonable = True
        else:
            decision = "not_reasonable"
            reasonable = False

        entry["marginal_average_token_reduction"] = marginal
        entry["context_recovery_to_99_percent"] = context_recovery
        entry["reasonable"] = reasonable
        entry["decision"] = decision
        previous = entry

    reasonable_numeric = [
        int(entry["grid"])
        for entry in entries
        if entry["grid"] != "base" and entry.get("reasonable")
    ]
    default_ceiling_without_4096_review = (
        max([value for value in reasonable_numeric if value <= 2048], default=None)
    )
    entry_4096 = next((entry for entry in entries if entry["grid"] == "4096"), None)
    if entry_4096 and entry_4096.get("reasonable"):
        recommended_ceiling = 4096
        recommendation_note = (
            "4096 met the reasonableness rule; review before changing the "
            "single-schema default ceiling"
        )
    else:
        recommended_ceiling = default_ceiling_without_4096_review
        recommendation_note = "keep the single-schema ceiling at or below 2048"

    schema_gaps.extend(
        f"{entry['grid']}: {gap}" for entry in entries for gap in entry["gaps"]
    )
    return {
        "schema": entries[0]["schema"] if entries else None,
        "grid_complete": not missing_grid,
        "gaps": schema_gaps,
        "default_ceiling_without_4096_review": default_ceiling_without_4096_review,
        "recommended_ceiling_after_grid": recommended_ceiling,
        "recommendation_note": recommendation_note,
        "entries": entries,
    }


def build_summary(entries: list[tuple[str, str, Path]], root: Path) -> dict[str, Any]:
    grouped: dict[str, list[dict[str, Any]]] = {}
    duplicate_keys: list[str] = []
    seen: set[tuple[str, str]] = set()
    for schema, grid, path in entries:
        key = (schema, grid)
        if key in seen:
            duplicate_keys.append(f"{schema}={grid}")
        seen.add(key)
        grouped.setdefault(schema, []).append(summarize_entry(schema, grid, path, root))

    schemas = [annotate_schema_entries(grouped[schema]) for schema in sorted(grouped)]
    global_gaps = [f"duplicate entry: {key}" for key in duplicate_keys]
    for schema in schemas:
        global_gaps.extend(
            f"{schema['schema']}: {gap}" for gap in schema["gaps"]
        )

    return {
        "artifact_kind": ARTIFACT_KIND,
        "manifest_version": MANIFEST_VERSION,
        "tool": TOOL_NAME,
        "grid": list(GRID_ORDER),
        "policy": {
            "average_reduction_threshold": AVERAGE_REDUCTION_THRESHOLD,
            "context_pass_threshold": CONTEXT_PASS_THRESHOLD,
            "context_window_tokens": CONTEXT_WINDOW_TOKENS,
            "total_vocab_grid": True,
            "larger_single_schema_vocabularies": "blocked_without_multi_schema_scaling_evidence",
        },
        "summary": {
            "schema_count": len(schemas),
            "all_ready": not global_gaps,
            "gap_count": len(global_gaps),
        },
        "schemas": schemas,
        "gaps": global_gaps,
    }


def fmt_number(value: Any) -> str:
    if value is None:
        return "-"
    if isinstance(value, float):
        return f"{value:.4g}"
    return str(value)


def fmt_percent(value: Any) -> str:
    if value is None:
        return "-"
    if not is_number(value):
        return str(value)
    return f"{value * 100:.2f}%"


def render_markdown(summary: dict[str, Any]) -> str:
    lines = [
        "# BPE Vocabulary Scaling Summary",
        "",
        "Policy: total generated `VOCAB_SIZE` grid base, 512, 1024, 2048, 4096; "
        "a grid point is reasonable only with at least 10% marginal average-token "
        "reduction or recovery to at least 99% 2048-token context pass rate.",
        "",
        "| Schema | Grid | Avg | P95 | P99 | 2048 pass | Marginal avg reduction | Decision | Actual vocab | Merges | Artifact bytes | Trainer wall ms | Gaps |",
        "| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: | --- |",
    ]
    for schema in summary["schemas"]:
        for entry in schema["entries"]:
            lines.append(
                "| {schema} | {grid} | {avg} | {p95} | {p99} | {pass_rate} | "
                "{marginal} | {decision} | {actual_vocab} | {merges} | {bytes} | "
                "{wall_ms} | {gaps} |".format(
                    schema=entry["schema"],
                    grid=entry["grid"],
                    avg=fmt_number(entry["average_tokens"]),
                    p95=fmt_number(entry["p95_tokens"]),
                    p99=fmt_number(entry["p99_tokens"]),
                    pass_rate=fmt_percent(entry["context_window_pass_rate"]),
                    marginal=fmt_percent(entry["marginal_average_token_reduction"]),
                    decision=entry["decision"],
                    actual_vocab=fmt_number(entry["actual_generated_vocab_size"]),
                    merges=fmt_number(entry["final_merge_count"]),
                    bytes=fmt_number(entry["artifact_bytes"]),
                    wall_ms=fmt_number(entry["trainer_wall_time_ms"]),
                    gaps=", ".join(entry["gaps"]) if entry["gaps"] else "-",
                )
            )
        lines.extend(
            [
                "",
                f"- {schema['schema']} default ceiling without 4096 review: "
                f"{fmt_number(schema['default_ceiling_without_4096_review'])}",
                f"- {schema['schema']} recommended ceiling after grid: "
                f"{fmt_number(schema['recommended_ceiling_after_grid'])} "
                f"({schema['recommendation_note']})",
                "",
            ]
        )
    if summary["gaps"]:
        lines.append("## Gaps")
        lines.append("")
        lines.extend(f"- {gap}" for gap in summary["gaps"])
        lines.append("")
    return "\n".join(lines)


def write_json(summary: dict[str, Any], path: str | None) -> None:
    encoded = json.dumps(summary, indent=2, sort_keys=True) + "\n"
    if path:
        Path(path).parent.mkdir(parents=True, exist_ok=True)
        Path(path).write_text(encoded, encoding="utf-8")
    else:
        sys.stdout.write(encoded)


def main() -> int:
    args = parse_args()
    root = repo_root()
    summary = build_summary(args.entry, root)
    write_json(summary, args.json_output)
    if args.markdown_output:
        output = Path(args.markdown_output)
        output.parent.mkdir(parents=True, exist_ok=True)
        output.write_text(render_markdown(summary), encoding="utf-8")
    if args.require_ready and summary["gaps"]:
        print("BPE vocabulary scaling summary is incomplete:", file=sys.stderr)
        for gap in summary["gaps"]:
            print(f"  - {gap}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
