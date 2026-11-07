#!/usr/bin/env python3
"""Classify oversized Rust modules for the advisory quality gate."""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
import tomllib
from dataclasses import asdict, dataclass
from pathlib import Path
from typing import Iterable


DEFAULT_POLICY = Path("scripts/testing/module-size-policy.toml")
ARTIFACT_TYPE = "tct-module-size-report"
ARTIFACT_VERSION = 1
PRIORITY_RANK = {
    "P1": 1,
    "P2": 2,
    "P3": 3,
    "defer": 90,
}
CFG_TEST_RE = re.compile(r"#\s*\[\s*cfg\s*\(\s*test\s*\)")
ITEM_BLOCK_RE = re.compile(r"\b(?:mod|fn)\s+[A-Za-z_][A-Za-z0-9_]*")


@dataclass(frozen=True)
class Candidate:
    path: str
    priority: str
    owner: str
    rationale: str
    next_action: str


@dataclass(frozen=True)
class ModuleSize:
    path: str
    lines: int
    production_lines: int
    test_lines: int
    over_by: int
    priority: str | None = None
    owner: str | None = None
    rationale: str | None = None
    next_action: str | None = None


@dataclass
class ScrubState:
    block_comment: bool = False
    raw_string_hashes: int | None = None
    string: bool = False
    char: bool = False
    escaped: bool = False


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


def resolve_policy_path(policy: Path, repo_root: Path) -> Path:
    if policy.is_absolute():
        return policy
    return repo_root / policy


def iter_rust_files(root: Path) -> Iterable[Path]:
    for path in sorted(root.rglob("*.rs")):
        if path.is_file():
            yield path


def scrub_line(line: str, state: ScrubState) -> str:
    """Strip comments and string bodies while preserving code punctuation."""
    output: list[str] = []
    i = 0
    while i < len(line):
        char = line[i]
        nxt = line[i + 1] if i + 1 < len(line) else ""

        if state.block_comment:
            if char == "*" and nxt == "/":
                state.block_comment = False
                i += 2
            else:
                i += 1
            output.append(" ")
            continue

        if state.raw_string_hashes is not None:
            hashes = "#" * state.raw_string_hashes
            terminator = '"' + hashes
            if line.startswith(terminator, i):
                state.raw_string_hashes = None
                i += len(terminator)
            else:
                i += 1
            output.append(" ")
            continue

        if state.string:
            if state.escaped:
                state.escaped = False
            elif char == "\\":
                state.escaped = True
            elif char == '"':
                state.string = False
            i += 1
            output.append(" ")
            continue

        if state.char:
            if state.escaped:
                state.escaped = False
            elif char == "\\":
                state.escaped = True
            elif char == "'":
                state.char = False
            i += 1
            output.append(" ")
            continue

        if char == "/" and nxt == "/":
            output.extend(" " * (len(line) - i))
            break

        if char == "/" and nxt == "*":
            state.block_comment = True
            output.extend("  ")
            i += 2
            continue

        if char == "r":
            raw_match = re.match(r'r(#+)?"', line[i:])
            if raw_match is not None:
                hashes = raw_match.group(1) or ""
                state.raw_string_hashes = len(hashes)
                output.extend(" " * len(raw_match.group(0)))
                i += len(raw_match.group(0))
                continue

        if char == '"':
            state.string = True
            output.append(" ")
            i += 1
            continue

        if char == "'":
            state.char = True
            output.append(" ")
            i += 1
            continue

        output.append(char)
        i += 1

    return "".join(output)


def count_lines(path: Path) -> tuple[int, int]:
    with path.open("r", encoding="utf-8") as handle:
        lines = handle.readlines()

    total = len(lines)
    test_lines = 0
    brace_depth = 0
    test_block_depths: list[int] = []
    pending_cfg_test = False
    scrub_state = ScrubState()

    for raw_line in lines:
        code = scrub_line(raw_line, scrub_state)
        stripped = code.strip()
        if CFG_TEST_RE.search(code):
            pending_cfg_test = True

        opens = code.count("{")
        closes = code.count("}")
        starts_test_block = (
            pending_cfg_test
            and "{" in code
            and ITEM_BLOCK_RE.search(code) is not None
        )
        line_in_test = bool(test_block_depths) or starts_test_block
        if line_in_test:
            test_lines += 1

        if starts_test_block:
            test_block_depths.append(brace_depth + 1)
            pending_cfg_test = False
        elif stripped and not stripped.startswith("#["):
            pending_cfg_test = False

        brace_depth += opens - closes
        while test_block_depths and brace_depth < test_block_depths[-1]:
            test_block_depths.pop()
        if brace_depth < 0:
            brace_depth = 0

    return total, test_lines


def load_policy(path: Path) -> dict[str, Candidate]:
    if not path.exists():
        return {}

    data = tomllib.loads(path.read_text())
    candidates: dict[str, Candidate] = {}
    for index, raw_candidate in enumerate(data.get("candidate", []), start=1):
        try:
            candidate = Candidate(
                path=str(raw_candidate["path"]),
                priority=str(raw_candidate["priority"]),
                owner=str(raw_candidate["owner"]),
                rationale=str(raw_candidate["rationale"]),
                next_action=str(raw_candidate["next_action"]),
            )
        except KeyError as error:
            raise ValueError(f"{path}: candidate {index} missing {error}") from error

        if not candidate.path or not candidate.priority or not candidate.owner:
            raise ValueError(f"{path}: candidate {index} has an empty required field")
        if candidate.path in candidates:
            raise ValueError(f"{path}: duplicate candidate for {candidate.path}")
        candidates[candidate.path] = candidate

    return candidates


def priority_key(module: ModuleSize) -> tuple[int, int, str]:
    priority = module.priority or "untracked"
    return (
        PRIORITY_RANK.get(priority, 80),
        -module.production_lines,
        module.path,
    )


def collect_report(repo_root: Path, limit: int, policy: dict[str, Candidate]) -> dict:
    src_root = repo_root / "src"
    modules: list[ModuleSize] = []

    for path in iter_rust_files(src_root):
        lines, test_lines = count_lines(path)
        if lines <= limit:
            continue

        production_lines = lines - test_lines
        rel_path = repo_relative(path, repo_root)
        candidate = policy.get(rel_path)
        if candidate is None:
            modules.append(
                ModuleSize(
                    path=rel_path,
                    lines=lines,
                    production_lines=production_lines,
                    test_lines=test_lines,
                    over_by=lines - limit,
                )
            )
        else:
            modules.append(
                ModuleSize(
                    path=rel_path,
                    lines=lines,
                    production_lines=production_lines,
                    test_lines=test_lines,
                    over_by=lines - limit,
                    priority=candidate.priority,
                    owner=candidate.owner,
                    rationale=candidate.rationale,
                    next_action=candidate.next_action,
                )
            )

    modules.sort(key=lambda module: (-module.production_lines, -module.lines, module.path))
    tracked = sorted(
        [module for module in modules if module.priority is not None],
        key=priority_key,
    )
    untracked = [module for module in modules if module.priority is None]

    return {
        "artifact_type": ARTIFACT_TYPE,
        "version": ARTIFACT_VERSION,
        "tool": "module-size-report",
        "report_only": True,
        "root": repo_root.as_posix(),
        "limit": limit,
        "oversized_count": len(modules),
        "tracked_count": len(tracked),
        "untracked_count": len(untracked),
        "oversized": [asdict(module) for module in modules],
        "tracked_queue": [asdict(module) for module in tracked],
        "untracked_oversized": [asdict(module) for module in untracked],
    }


def render_text(report: dict, top: int) -> str:
    lines = [
        "Module size report (advisory)",
        f"Root: {report['root']}",
        f"Line limit: {report['limit']}",
        f"Oversized modules: {report['oversized_count']}",
        f"Tracked queue entries: {report['tracked_count']}",
        f"Untracked oversized modules: {report['untracked_count']}",
        "",
    ]

    tracked = report["tracked_queue"]
    if tracked:
        lines.append("Tracked advisory queue:")
        for module in tracked[:top]:
            lines.append(
                f"  {module['priority']} {module['path']} "
                f"({module['lines']} lines, {module['production_lines']} prod, "
                f"{module['test_lines']} test, +{module['over_by']})"
            )
            lines.append(f"    owner: {module['owner']}")
            lines.append(f"    next: {module['next_action']}")
            lines.append(f"    why: {module['rationale']}")
    else:
        lines.append("Tracked advisory queue: none")

    untracked = report["untracked_oversized"]
    if untracked:
        lines.append("")
        lines.append("Largest untracked oversized modules:")
        for module in untracked[:top]:
            lines.append(
                f"  {module['path']} ({module['lines']} lines, "
                f"{module['production_lines']} prod, {module['test_lines']} test, "
                f"+{module['over_by']})"
            )

    return "\n".join(lines) + "\n"


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo-root", help="Repository root; defaults to git root")
    parser.add_argument(
        "--policy",
        type=Path,
        default=DEFAULT_POLICY,
        help=f"Module-size policy TOML, default: {DEFAULT_POLICY}",
    )
    parser.add_argument(
        "--limit",
        type=int,
        default=600,
        help="Line-count threshold for oversized Rust modules",
    )
    parser.add_argument(
        "--top",
        type=int,
        default=20,
        help="Maximum entries per text-report section",
    )
    parser.add_argument("--json", action="store_true", help="Emit JSON report")
    parser.add_argument("--json-output", help="Write JSON report to this path")
    parser.add_argument(
        "--exit-zero",
        action="store_true",
        help="Return success even when oversized modules exist",
    )
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    repo_root = resolve_repo_root(args.repo_root)
    policy_path = resolve_policy_path(args.policy, repo_root)

    try:
        policy = load_policy(policy_path)
    except ValueError as error:
        print(error, file=sys.stderr)
        return 2

    report = collect_report(repo_root, args.limit, policy)
    if args.json_output:
        output_path = Path(args.json_output)
        output_path.parent.mkdir(parents=True, exist_ok=True)
        output_path.write_text(
            json.dumps(report, indent=2, sort_keys=True) + "\n",
            encoding="utf-8",
        )

    if args.json:
        print(json.dumps(report, indent=2, sort_keys=True))
    else:
        print(render_text(report, args.top), end="")

    if report["oversized_count"] and not args.exit_zero:
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
