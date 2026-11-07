#!/usr/bin/env python3
"""Report Rust module documentation coverage for the advisory quality gate."""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
from pathlib import Path
from typing import Iterable


ARTIFACT_TYPE = "tct-documentation-coverage"
ARTIFACT_VERSION = 1


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


def repo_relative(path: Path, repo_root: Path) -> str:
    try:
        return path.relative_to(repo_root).as_posix()
    except ValueError:
        return path.as_posix()


def resolve_root(root: Path, repo_root: Path) -> Path:
    if root.is_absolute():
        return root
    return repo_root / root


def iter_rust_modules(root: Path) -> Iterable[Path]:
    for path in sorted(root.rglob("*.rs")):
        if path.is_file():
            yield path


def has_module_docs(path: Path) -> bool:
    with path.open("r", encoding="utf-8") as handle:
        return any(line.lstrip().startswith("//!") for line in handle)


def collect_report(repo_root: Path, root: Path, threshold: int) -> dict:
    rust_modules = list(iter_rust_modules(root))
    documented = [path for path in rust_modules if has_module_docs(path)]
    documented_set = set(documented)
    undocumented = [path for path in rust_modules if path not in documented_set]
    total = len(rust_modules)
    documented_count = len(documented)
    if total:
        percent = documented_count * 100 // total
        error = None
    else:
        percent = 0
        error = f"No Rust source files found under {repo_relative(root, repo_root)}."

    return {
        "artifact_type": ARTIFACT_TYPE,
        "version": ARTIFACT_VERSION,
        "tool": "documentation-coverage-report",
        "report_only": True,
        "root": repo_relative(root, repo_root),
        "threshold_percent": threshold,
        "total_modules": total,
        "documented_modules": documented_count,
        "undocumented_count": len(undocumented),
        "percent": percent,
        "undocumented_modules": [repo_relative(path, repo_root) for path in undocumented],
        "error": error,
    }


def print_report(report: dict, top: int) -> None:
    print("Documentation coverage report (advisory)")
    if report["error"] is not None:
        print(report["error"])
        return

    print(f"Rust module root: {report['root']}")
    print(
        "Documented Rust modules: "
        f"{report['documented_modules']}/{report['total_modules']} = "
        f"{report['percent']}%; advisory threshold: "
        f"{report['threshold_percent']}%."
    )

    if report["undocumented_modules"]:
        print("Undocumented modules:")
        for path in report["undocumented_modules"][:top]:
            print(f"  - {path}")
        remaining = len(report["undocumented_modules"]) - top
        if remaining > 0:
            print(f"  ... {remaining} more")
    else:
        print("Undocumented modules: none")


def write_json_output(report: dict, output: Path) -> None:
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(
        json.dumps(report, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo-root", help="Repository root. Defaults to git root.")
    parser.add_argument(
        "--root",
        default="src",
        type=Path,
        help="Rust source root to inspect, relative to the repo root by default.",
    )
    parser.add_argument(
        "--threshold",
        type=int,
        default=90,
        help="Minimum documented module percentage before the advisory passes.",
    )
    parser.add_argument("--top", type=int, default=40, help="Number of missing modules to print.")
    parser.add_argument("--json", action="store_true", help="Write the report JSON to stdout.")
    parser.add_argument("--json-output", type=Path, help="Write the report JSON to this path.")
    parser.add_argument(
        "--exit-zero",
        action="store_true",
        help="Always exit successfully after writing the advisory report.",
    )
    return parser.parse_args(argv)


def main(argv: list[str]) -> int:
    args = parse_args(argv)
    if args.threshold < 0 or args.threshold > 100:
        raise SystemExit("--threshold must be between 0 and 100")
    if args.top < 0:
        raise SystemExit("--top must be non-negative")

    repo_root = resolve_repo_root(args.repo_root)
    root = resolve_root(args.root, repo_root)
    report = collect_report(repo_root, root, args.threshold)

    if args.json:
        print(json.dumps(report, indent=2, sort_keys=True))
    else:
        print_report(report, args.top)

    if args.json_output is not None:
        write_json_output(report, args.json_output)

    if args.exit_zero:
        return 0
    if report["error"] is not None:
        return 1
    return 0 if report["percent"] >= args.threshold else 1


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
