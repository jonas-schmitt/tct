#!/usr/bin/env python3
"""Inventory generated-source substring checks in Rust sources."""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
from dataclasses import asdict, dataclass
from pathlib import Path
from typing import Iterable


DEFAULT_PATHS = ("tests", "src")
ARTIFACT_TYPE = "tct-generated-source-inventory"
ARTIFACT_VERSION = 1

SOURCE_SHAPE_LITERALS = (
    "pub struct",
    "pub enum",
    "pub type",
    "unsafe impl",
    "serde",
    "JsonTokenEncoder",
    "decode_prefix",
    "BASE_VOCAB_SIZE",
    "VOCAB_SIZE",
    "PAD_TOKEN",
    "pub mod bpe",
    "pub fn encode",
    "pub fn decode",
    "fn token_count",
)

BROAD_SOURCE_SHAPE_PREFIXES = ("struct ", "enum ", "impl ")

PRODUCTION_CLASSIFIER_PATTERNS = (
    'contains("pub struct',
    'contains("pub enum',
    'contains("pub type',
    'contains("JsonTokenEncoder',
    'starts_with("pub struct',
    'starts_with("pub enum',
    'starts_with("pub type',
    "definition_tokens.to_string().contains",
    "implementation_tokens.to_string().contains",
)

RUST_STRING_RE = re.compile(r'"(?:\\.|[^"\\])*"')
RUST_RAW_STRING_RE = re.compile(r'r(?P<hashes>#*)"(.*?)"(?P=hashes)')
CARGO_DEPENDENCY_LITERAL_RE = re.compile(r"^[A-Za-z0-9_-]+\s*=\s*(?:\"|\{)")


@dataclass(frozen=True)
class Hit:
    path: str
    line: int
    kind: str
    text: str


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


def rust_files(repo_root: Path, paths: Iterable[str]) -> list[Path]:
    files: list[Path] = []
    for item in paths:
        path = (repo_root / item).resolve()
        if path.is_file() and path.suffix == ".rs":
            files.append(path)
        elif path.is_dir():
            files.extend(sorted(path.rglob("*.rs")))
    return sorted(set(files))


def relative(path: Path, repo_root: Path) -> str:
    try:
        return path.relative_to(repo_root).as_posix()
    except ValueError:
        return path.as_posix()


def strip_line_comment(line: str) -> str:
    in_string = False
    escaped = False
    for index, char in enumerate(line):
        if escaped:
            escaped = False
            continue
        if char == "\\" and in_string:
            escaped = True
            continue
        if char == '"':
            in_string = not in_string
            continue
        if not in_string and line[index : index + 2] == "//":
            return line[:index]
    return line


def string_literal_spans(code: str) -> list[tuple[int, int, str]]:
    spans: list[tuple[int, int, str]] = []

    for match in RUST_RAW_STRING_RE.finditer(code):
        spans.append((match.start(), match.end(), match.group(2)))

    for match in RUST_STRING_RE.finditer(code):
        if any(start <= match.start() < end for start, end, _ in spans):
            continue
        literal = match.group(0)[1:-1]
        try:
            literal = bytes(literal, "utf-8").decode("unicode_escape")
        except UnicodeDecodeError:
            pass
        spans.append((match.start(), match.end(), literal))

    return sorted(spans)


def mask_string_literals(code: str) -> str:
    masked = list(code)
    for start, end, _ in string_literal_spans(code):
        masked[start:end] = " " * (end - start)
    return "".join(masked)


def source_shape_string_literals(code: str) -> list[str]:
    return [literal for _, _, literal in string_literal_spans(code)]


def contains_argument_string_literals(code: str) -> list[str]:
    masked = mask_string_literals(code)
    literals: list[str] = []
    search_index = 0

    while True:
        call_index = masked.find(".contains(", search_index)
        if call_index < 0:
            return literals

        arg_start = call_index + len(".contains(")
        depth = 1
        index = arg_start
        while index < len(masked):
            char = masked[index]
            if char == "(":
                depth += 1
            elif char == ")":
                depth -= 1
                if depth == 0:
                    break
            index += 1

        if depth != 0:
            return literals

        literals.extend(source_shape_string_literals(code[arg_start:index]))
        search_index = index + 1


def is_cfg_test_attribute(code: str) -> bool:
    match = re.search(r"#\s*\[\s*cfg\s*\((.*)\)\s*\]", code)
    if not match:
        return False

    cfg_expr = match.group(1)
    if re.search(r"\bnot\s*\(\s*test\s*\)", cfg_expr):
        return False
    return re.search(r"\btest\b", cfg_expr) is not None


def test_line_flags(path: Path, repo_root: Path, lines: list[str]) -> list[bool]:
    rel = relative(path, repo_root)
    if (
        rel.startswith("tests/")
        or path.name == "tests.rs"
        or path.stem.endswith("_tests")
    ):
        return [True] * len(lines)

    flags = [False] * len(lines)
    pending_cfg_test = False
    pending_test_function = False
    test_depth: int | None = None
    brace_depth = 0

    for index, line in enumerate(lines):
        code = strip_line_comment(line)

        if test_depth is not None:
            flags[index] = True

        if is_cfg_test_attribute(code):
            pending_cfg_test = True

        if re.search(r"#\s*\[\s*(?:test|[\w:]+::test)\b", code):
            pending_test_function = True

        opens = code.count("{")
        closes = code.count("}")
        new_depth = brace_depth + opens - closes

        if pending_cfg_test and re.search(r"\b(?:mod|fn)\s+[A-Za-z_]\w*", code):
            flags[index] = True
            pending_cfg_test = False
            if new_depth > brace_depth:
                test_depth = new_depth

        if pending_test_function and re.search(r"\bfn\s+[A-Za-z_]\w*", code):
            flags[index] = True
            pending_test_function = False
            if new_depth > brace_depth:
                test_depth = new_depth

        brace_depth = new_depth

        if test_depth is not None and brace_depth < test_depth:
            test_depth = None

    return flags


def is_source_shape_assertion(code: str) -> bool:
    source_literals = contains_argument_string_literals(code)
    return any(is_source_shape_literal(literal) for literal in source_literals)


def is_source_shape_literal(literal: str) -> bool:
    stripped = literal.lstrip()
    if CARGO_DEPENDENCY_LITERAL_RE.match(stripped):
        return False

    if any(shape_literal in literal for shape_literal in SOURCE_SHAPE_LITERALS):
        return True

    return any(
        stripped.startswith(shape_prefix)
        for shape_prefix in BROAD_SOURCE_SHAPE_PREFIXES
    )


def is_production_classifier(code: str) -> bool:
    return any(pattern in code for pattern in PRODUCTION_CLASSIFIER_PATTERNS)


def collect_hits(repo_root: Path, paths: Iterable[str]) -> list[Hit]:
    hits: list[Hit] = []

    for path in rust_files(repo_root, paths):
        lines = path.read_text().splitlines()
        test_flags = test_line_flags(path, repo_root, lines)
        rel = relative(path, repo_root)

        for index, line in enumerate(lines, start=1):
            code = strip_line_comment(line).strip()
            if not code:
                continue

            if test_flags[index - 1] and is_source_shape_assertion(code):
                hits.append(Hit(rel, index, "test-source-shape", code))
            elif not test_flags[index - 1] and is_production_classifier(code):
                hits.append(Hit(rel, index, "production-classifier", code))

    return hits


def render_text(hits: list[Hit], mode: str) -> str:
    filtered = filter_hits(hits, mode)
    lines = [f"Generated-source substring inventory ({mode})"]
    if not filtered:
        lines.append("No matches.")
        return "\n".join(lines) + "\n"

    for hit in filtered:
        lines.append(f"{hit.path}:{hit.line}: [{hit.kind}] {hit.text}")
    return "\n".join(lines) + "\n"


def filter_hits(hits: list[Hit], mode: str) -> list[Hit]:
    if mode == "all":
        return hits
    if mode == "test-assertions":
        return [hit for hit in hits if hit.kind == "test-source-shape"]
    if mode == "production-classifiers":
        return [hit for hit in hits if hit.kind == "production-classifier"]
    raise ValueError(f"unsupported mode: {mode}")


def git_output(repo_root: Path, args: list[str]) -> str | None:
    try:
        completed = subprocess.run(
            ["git", *args],
            cwd=repo_root,
            check=True,
            capture_output=True,
            text=True,
        )
    except (OSError, subprocess.CalledProcessError):
        return None
    return completed.stdout.strip()


def repo_provenance(repo_root: Path) -> dict:
    status = git_output(repo_root, ["status", "--porcelain=v1"])
    return {
        "root": repo_root.as_posix(),
        "head": git_output(repo_root, ["rev-parse", "HEAD"]),
        "dirty": None if status is None else bool(status),
    }


def count_by_kind(hits: list[Hit]) -> dict[str, int]:
    counts = {"test-source-shape": 0, "production-classifier": 0}
    for hit in hits:
        counts[hit.kind] = counts.get(hit.kind, 0) + 1
    return counts


def count_by_path_and_kind(hits: list[Hit]) -> dict[str, dict[str, int]]:
    counts: dict[str, dict[str, int]] = {
        "test-source-shape": {},
        "production-classifier": {},
    }
    for hit in hits:
        path_counts = counts.setdefault(hit.kind, {})
        path_counts[hit.path] = path_counts.get(hit.path, 0) + 1
    return {
        kind: dict(sorted(path_counts.items()))
        for kind, path_counts in sorted(counts.items())
    }


def artifact_payload(
    repo_root: Path,
    paths: Iterable[str],
    mode: str,
    hits: list[Hit],
) -> dict:
    filtered = filter_hits(hits, mode)
    return {
        "artifact_type": ARTIFACT_TYPE,
        "version": ARTIFACT_VERSION,
        "tool": "generated-source-inventory",
        "mode": mode,
        "source_paths": list(paths),
        "repo": repo_provenance(repo_root),
        "counts": {
            "total_hits": len(hits),
            "filtered_hits": len(filtered),
            "by_kind": count_by_kind(hits),
            "filtered_by_kind": count_by_kind(filtered),
            "path_counts_by_kind": count_by_path_and_kind(hits),
            "filtered_path_counts_by_kind": count_by_path_and_kind(filtered),
        },
        "hits": [asdict(hit) for hit in filtered],
    }


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description=(
            "Inventory generated-source substring checks. The output is "
            "report-only unless a --fail-on-* option is set."
        )
    )
    parser.add_argument("paths", nargs="*", default=list(DEFAULT_PATHS))
    parser.add_argument("--repo-root", help="Repository root. Defaults to git root.")
    parser.add_argument(
        "--mode",
        choices=("all", "test-assertions", "production-classifiers"),
        default="all",
    )
    parser.add_argument("--json", action="store_true", help="Emit JSON output.")
    parser.add_argument("--json-output", help="Write JSON output to this path.")
    parser.add_argument(
        "--json-format",
        choices=("hits", "artifact"),
        default="hits",
        help=(
            "JSON payload shape for --json and --json-output. 'hits' is the "
            "legacy bare list; 'artifact' adds versioned provenance metadata."
        ),
    )
    parser.add_argument(
        "--fail-on-production",
        action="store_true",
        help="Exit non-zero when production generated-source classifiers are found.",
    )
    parser.add_argument(
        "--fail-on-test-assertions",
        action="store_true",
        help="Exit non-zero when generated-source test assertions are found.",
    )
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    repo_root = resolve_repo_root(args.repo_root)
    hits = collect_hits(repo_root, args.paths)
    filtered = filter_hits(hits, args.mode)
    if args.json_format == "artifact":
        json_payload = artifact_payload(repo_root, args.paths, args.mode, hits)
    else:
        json_payload = [asdict(hit) for hit in filtered]

    if args.json_output:
        output_path = Path(args.json_output)
        output_path.parent.mkdir(parents=True, exist_ok=True)
        output_path.write_text(
            json.dumps(json_payload, indent=2) + "\n",
            encoding="utf-8",
        )

    if args.json:
        print(json.dumps(json_payload, indent=2))
    else:
        print(render_text(hits, args.mode), end="")

    if args.fail_on_production and any(
        hit.kind == "production-classifier" for hit in hits
    ):
        return 1
    if args.fail_on_test_assertions and any(
        hit.kind == "test-source-shape" for hit in hits
    ):
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
