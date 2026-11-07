#!/usr/bin/env python3
"""Print touched-file coverage from a cargo llvm-cov summary JSON file."""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Iterable


METRICS = ("lines", "functions", "regions")


@dataclass(frozen=True)
class FileCoverage:
    path: Path
    summary: dict


def resolve_repo_root(repo_root: str | None) -> Path:
    if repo_root:
        return Path(repo_root).resolve()

    try:
        completed = subprocess.run(
            ["git", "rev-parse", "--show-toplevel"],
            check=True,
            capture_output=True,
            text=True,
        )
    except (OSError, subprocess.CalledProcessError):
        return Path.cwd().resolve()

    return Path(completed.stdout.strip()).resolve()


def relative(path: Path, repo_root: Path) -> str:
    try:
        return path.relative_to(repo_root).as_posix()
    except ValueError:
        return path.as_posix()


def changed_rust_paths(ref: str, repo_root: Path) -> list[Path]:
    try:
        completed = subprocess.run(
            ["git", "diff", "--name-only", "--diff-filter=ACMR", ref],
            cwd=repo_root,
            check=True,
            capture_output=True,
            text=True,
        )
        untracked = subprocess.run(
            ["git", "ls-files", "--others", "--exclude-standard"],
            cwd=repo_root,
            check=True,
            capture_output=True,
            text=True,
        )
    except (OSError, subprocess.CalledProcessError) as error:
        raise SystemExit(f"error: failed to read changed files from git: {error}")

    paths = []
    for line in [*completed.stdout.splitlines(), *untracked.stdout.splitlines()]:
        if line.endswith(".rs"):
            paths.append((repo_root / line).resolve())
    return paths


def load_file_coverage(summary_json: Path) -> list[FileCoverage]:
    data = json.loads(summary_json.read_text())
    try:
        files = data["data"][0]["files"]
    except (KeyError, IndexError, TypeError) as error:
        raise SystemExit(f"error: unsupported llvm-cov summary format: {error}")

    coverage = []
    for item in files:
        try:
            coverage.append(
                FileCoverage(
                    path=Path(item["filename"]).resolve(),
                    summary=item["summary"],
                )
            )
        except KeyError as error:
            raise SystemExit(f"error: malformed file coverage entry: {error}")
    return coverage


def is_under(path: Path, directory: Path) -> bool:
    try:
        path.relative_to(directory)
        return True
    except ValueError:
        return False


def select_files(
    coverage: Iterable[FileCoverage],
    requested_paths: Iterable[Path],
) -> tuple[list[FileCoverage], list[Path]]:
    coverage_by_path = {item.path: item for item in coverage}
    selected: dict[Path, FileCoverage] = {}
    missing: list[Path] = []

    for requested in requested_paths:
        if requested in coverage_by_path:
            selected[requested] = coverage_by_path[requested]
            continue

        matches = [
            item
            for item in coverage_by_path.values()
            if requested.is_dir() and is_under(item.path, requested)
        ]
        if matches:
            for item in matches:
                selected[item.path] = item
            continue

        missing.append(requested)

    return sorted(selected.values(), key=lambda item: item.path.as_posix()), missing


def metric_counts(summary: dict, metric: str) -> tuple[int, int]:
    item = summary.get(metric, {})
    return int(item.get("covered", 0)), int(item.get("count", 0))


def format_metric(covered: int, count: int) -> str:
    if count == 0:
        return "n/a"
    return f"{covered}/{count} ({covered * 100 / count:.2f}%)"


def render_report(
    selected: list[FileCoverage],
    missing: list[Path],
    repo_root: Path,
    summary_json: Path,
) -> str:
    lines = [f"Coverage focus from {relative(summary_json.resolve(), repo_root)}"]

    if not selected and not missing:
        lines.append("No Rust source files selected. Pass paths or --changed-from REF.")
        return "\n".join(lines) + "\n"

    if selected:
        path_width = max(
            4,
            min(80, max(len(relative(item.path, repo_root)) for item in selected)),
        )
        header = (
            f"{'path':<{path_width}}  "
            f"{'lines':>20}  {'functions':>20}  {'regions':>20}"
        )
        lines.extend(["", header, "-" * len(header)])

        totals = {metric: [0, 0] for metric in METRICS}
        for item in selected:
            row_counts = {}
            for metric in METRICS:
                covered, count = metric_counts(item.summary, metric)
                totals[metric][0] += covered
                totals[metric][1] += count
                row_counts[metric] = format_metric(covered, count)

            rel_path = relative(item.path, repo_root)
            lines.append(
                f"{rel_path:<{path_width}}  "
                f"{row_counts['lines']:>20}  "
                f"{row_counts['functions']:>20}  "
                f"{row_counts['regions']:>20}"
            )

        total_counts = {
            metric: format_metric(*counts) for metric, counts in totals.items()
        }
        lines.append("-" * len(header))
        lines.append(
            f"{'TOTAL':<{path_width}}  "
            f"{total_counts['lines']:>20}  "
            f"{total_counts['functions']:>20}  "
            f"{total_counts['regions']:>20}"
        )

    if missing:
        lines.extend(
            [
                "",
                "Paths without coverage rows:",
                *[f"  {relative(path, repo_root)}" for path in missing],
                "This is expected for tests, docs-only changes, or files not reached by the selected coverage mode.",
            ]
        )

    return "\n".join(lines) + "\n"


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description=(
            "Summarize selected files from cargo llvm-cov summary JSON. "
            "This is report-only and does not enforce thresholds."
        )
    )
    parser.add_argument("summary_json", help="Path to cargo llvm-cov summary.json")
    parser.add_argument("paths", nargs="*", help="Repo-relative files or directories")
    parser.add_argument(
        "--changed-from",
        help="Add changed Rust files from git diff REF plus untracked Rust files",
    )
    parser.add_argument("--repo-root", help="Repository root. Defaults to git root.")
    parser.add_argument("--output", help="Optional file to write the same report.")
    if hasattr(parser, "parse_intermixed_args"):
        return parser.parse_intermixed_args()
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    repo_root = resolve_repo_root(args.repo_root)
    summary_json = Path(args.summary_json)
    if not summary_json.is_absolute():
        summary_json = (repo_root / summary_json).resolve()

    requested = [(repo_root / path).resolve() for path in args.paths]
    if args.changed_from:
        requested.extend(changed_rust_paths(args.changed_from, repo_root))

    coverage = load_file_coverage(summary_json)
    selected, missing = select_files(coverage, requested)
    report = render_report(selected, missing, repo_root, summary_json)

    if args.output:
        output = Path(args.output)
        if not output.is_absolute():
            output = (repo_root / output).resolve()
        output.parent.mkdir(parents=True, exist_ok=True)
        output.write_text(report)

    sys.stdout.write(report)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
