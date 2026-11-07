#!/usr/bin/env python3
"""Classify Rust unwrap()/expect() sites for the advisory quality gate."""

from __future__ import annotations

import argparse
import json
import re
import sys
import tomllib
from collections import Counter
from dataclasses import dataclass
from pathlib import Path
from typing import Iterable


DEFAULT_POLICY = Path("scripts/testing/unwrap-expect-policy.toml")
ARTIFACT_TYPE = "tct-unwrap-expect-report"
ARTIFACT_VERSION = 1
SITE_RE = re.compile(r"\.(unwrap\(\)|expect\()")
CFG_TEST_RE = re.compile(r"#\s*\[\s*cfg\s*\(\s*test\s*\)")
TEST_ATTR_RE = re.compile(r"#\s*\[\s*(?:[\w:]+::)?test\b")
ITEM_BLOCK_RE = re.compile(r"\b(?:mod|fn)\s+[A-Za-z_][A-Za-z0-9_]*")
IDENT_START_RE = re.compile(r"[A-Za-z_]")
IDENT_CONTINUE_RE = re.compile(r"[A-Za-z0-9_]")


@dataclass(frozen=True)
class Site:
    path: Path
    line: int
    kind: str
    in_test: bool
    text: str


@dataclass(frozen=True)
class PolicyEntry:
    path: str
    pattern: str
    reason: str
    category: str


@dataclass
class ScrubState:
    block_comment: bool = False
    raw_string_hashes: int | None = None
    string: bool = False
    char: bool = False
    escaped: bool = False


def apostrophe_starts_lifetime(line: str, index: int) -> bool:
    next_index = index + 1
    if next_index >= len(line):
        return False
    if IDENT_START_RE.fullmatch(line[next_index]) is None:
        return False

    cursor = next_index + 1
    while cursor < len(line) and IDENT_CONTINUE_RE.fullmatch(line[cursor]) is not None:
        cursor += 1

    return cursor >= len(line) or line[cursor] != "'"


def expect_call_uses_try_operator(code: str, match: re.Match[str]) -> bool:
    depth = 1
    cursor = match.end()
    while cursor < len(code):
        char = code[cursor]
        if char == "(":
            depth += 1
        elif char == ")":
            depth -= 1
            if depth == 0:
                cursor += 1
                while cursor < len(code) and code[cursor].isspace():
                    cursor += 1
                return cursor < len(code) and code[cursor] == "?"
        cursor += 1
    return False


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

        if char == "'" and apostrophe_starts_lifetime(line, i):
            output.append(char)
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


def scan_lines(path: Path, lines: Iterable[str]) -> list[Site]:
    sites: list[Site] = []
    scrub_state = ScrubState()
    brace_depth = 0
    test_block_depths: list[int] = []
    pending_cfg_test = False
    pending_test_attr = False

    for line_number, raw_line in enumerate(lines, start=1):
        code = scrub_line(raw_line, scrub_state)
        stripped = code.strip()

        if CFG_TEST_RE.search(code):
            pending_cfg_test = True
        if TEST_ATTR_RE.search(code):
            pending_test_attr = True

        opens = code.count("{")
        closes = code.count("}")
        starts_test_block = (
            (pending_cfg_test or pending_test_attr)
            and "{" in code
            and ITEM_BLOCK_RE.search(code) is not None
        )
        line_in_test = bool(test_block_depths) or starts_test_block

        for match in SITE_RE.finditer(code):
            kind = "unwrap" if match.group(1) == "unwrap()" else "expect"
            if kind == "expect" and expect_call_uses_try_operator(code, match):
                continue
            sites.append(
                Site(
                    path=path,
                    line=line_number,
                    kind=kind,
                    in_test=line_in_test,
                    text=raw_line.strip(),
                )
            )

        if starts_test_block:
            test_block_depths.append(brace_depth + 1)
            pending_cfg_test = False
            pending_test_attr = False
        elif stripped and not stripped.startswith("#[") and "{" not in code:
            pending_cfg_test = False
            pending_test_attr = False

        brace_depth += opens - closes
        while test_block_depths and brace_depth < test_block_depths[-1]:
            test_block_depths.pop()
        if brace_depth < 0:
            brace_depth = 0

    return sites


def scan_root(root: Path) -> list[Site]:
    sites: list[Site] = []
    for path in iter_rust_files(root):
        sites.extend(scan_lines(path, path.read_text().splitlines()))
    return sites


def repo_relative(path: Path) -> Path:
    try:
        return path.relative_to(Path.cwd())
    except ValueError:
        return path


def load_policy(path: Path) -> list[PolicyEntry]:
    if not path.exists():
        return []

    data = tomllib.loads(path.read_text())
    entries = []
    for index, raw_entry in enumerate(data.get("allow", []), start=1):
        try:
            entry = PolicyEntry(
                path=str(raw_entry["path"]),
                pattern=str(raw_entry["pattern"]),
                reason=str(raw_entry["reason"]),
                category=str(raw_entry.get("category", "documented_invariant")),
            )
        except KeyError as error:
            raise ValueError(f"{path}: allow entry {index} missing {error}") from error
        if not entry.path or not entry.pattern or not entry.reason:
            raise ValueError(f"{path}: allow entry {index} has an empty required field")
        entries.append(entry)
    return entries


def policy_match(site: Site, policy: list[PolicyEntry]) -> PolicyEntry | None:
    site_path = str(repo_relative(site.path))
    for entry in policy:
        if entry.path == site_path and entry.pattern in site.text:
            return entry
    return None


def print_sample(title: str, sites: list[Site], limit: int) -> None:
    print(f"\n{title}:")
    if not sites:
        print("  none")
        return
    for site in sites[:limit]:
        print(f"  {site.path}:{site.line}: {site.text}")
    if len(sites) > limit:
        print(f"  ... {len(sites) - limit} more")


def print_policy_sample(
    title: str, matches: list[tuple[Site, PolicyEntry]], limit: int
) -> None:
    print(f"\n{title}:")
    if not matches:
        print("  none")
        return
    for site, entry in matches[:limit]:
        print(f"  {site.path}:{site.line}: {entry.category}: {entry.reason}")
    if len(matches) > limit:
        print(f"  ... {len(matches) - limit} more")


def site_record(site: Site) -> dict:
    return {
        "path": str(site.path),
        "line": site.line,
        "kind": site.kind,
        "in_test": site.in_test,
        "text": site.text,
    }


def policy_match_record(site: Site, entry: PolicyEntry) -> dict:
    return {
        "site": site_record(site),
        "policy": {
            "path": entry.path,
            "pattern": entry.pattern,
            "reason": entry.reason,
            "category": entry.category,
        },
    }


def classify_sites(sites: list[Site], policy: list[PolicyEntry]) -> dict:
    non_test = [site for site in sites if not site.in_test]
    test_only = [site for site in sites if site.in_test]
    allowed = [
        (site, match)
        for site in non_test
        if (match := policy_match(site, policy)) is not None
    ]
    allowed_ids = {(site.path, site.line) for site, _ in allowed}
    unclassified = [
        site for site in non_test if (site.path, site.line) not in allowed_ids
    ]

    return {
        "non_test": non_test,
        "test_only": test_only,
        "allowed": allowed,
        "unclassified": unclassified,
        "non_test_counts": Counter(site.kind for site in non_test),
        "allowed_counts": Counter(match.category for _, match in allowed),
        "unclassified_counts": Counter(site.kind for site in unclassified),
        "test_counts": Counter(site.kind for site in test_only),
        "unclassified_by_file": Counter(str(site.path) for site in unclassified),
    }


def build_report(
    root: Path,
    policy_path: Path,
    sites: list[Site],
    policy: list[PolicyEntry],
) -> dict:
    classified = classify_sites(sites, policy)
    return {
        "artifact_type": ARTIFACT_TYPE,
        "version": ARTIFACT_VERSION,
        "tool": "unwrap-expect-report",
        "root": str(root),
        "policy": str(policy_path) if policy else None,
        "total_sites": len(sites),
        "non_test_sites": len(classified["non_test"]),
        "non_test_counts": dict(classified["non_test_counts"]),
        "documented_invariant_sites": len(classified["allowed"]),
        "documented_counts": dict(classified["allowed_counts"]),
        "unclassified_non_test_sites": len(classified["unclassified"]),
        "unclassified_counts": dict(classified["unclassified_counts"]),
        "inline_test_sites": len(classified["test_only"]),
        "inline_test_counts": dict(classified["test_counts"]),
        "top_unclassified_non_test_files": [
            {"path": path, "count": count}
            for path, count in classified["unclassified_by_file"].most_common(10)
        ],
        "unclassified": [site_record(site) for site in classified["unclassified"]],
        "documented": [
            policy_match_record(site, entry)
            for site, entry in classified["allowed"]
        ],
        "inline_test": [site_record(site) for site in classified["test_only"]],
    }


def print_report(
    root: Path, policy_path: Path, sites: list[Site], policy: list[PolicyEntry], limit: int
) -> int:
    classified = classify_sites(sites, policy)
    non_test = classified["non_test"]
    test_only = classified["test_only"]
    allowed = classified["allowed"]
    unclassified = classified["unclassified"]
    non_test_counts = classified["non_test_counts"]
    allowed_counts = classified["allowed_counts"]
    unclassified_counts = classified["unclassified_counts"]
    test_counts = classified["test_counts"]
    by_file = classified["unclassified_by_file"]

    print("unwrap()/expect() policy report")
    print(f"Root: {root}")
    print(f"Policy: {policy_path if policy else 'none'}")
    print(f"Total sites: {len(sites)}")
    print(
        "Non-test sites: "
        f"{len(non_test)} "
        f"(unwrap: {non_test_counts['unwrap']}, expect: {non_test_counts['expect']})"
    )
    print(
        "Documented invariant sites: "
        f"{len(allowed)} "
        f"({', '.join(f'{name}: {count}' for name, count in sorted(allowed_counts.items())) or 'none'})"
    )
    print(
        "Unclassified non-test sites: "
        f"{len(unclassified)} "
        f"(unwrap: {unclassified_counts['unwrap']}, expect: {unclassified_counts['expect']})"
    )
    print(
        "Inline test sites: "
        f"{len(test_only)} "
        f"(unwrap: {test_counts['unwrap']}, expect: {test_counts['expect']})"
    )

    print("\nTop unclassified non-test files:")
    if by_file:
        for path, count in by_file.most_common(10):
            print(f"  {count:4d} {path}")
    else:
        print("  none")

    print_sample("Highest-priority unclassified candidates", unclassified, limit)
    print_policy_sample("Documented invariant sample", allowed, min(limit, 10))
    print_sample("Inline test sample", test_only, min(limit, 10))

    if unclassified:
        print(
            "\nPolicy: inline test sites are reported separately; non-test sites "
            "need either typed error handling, a documented impossible invariant, "
            "or a local cleanup issue."
        )
    return len(unclassified)


def self_test() -> None:
    sample = [
        'pub fn prod() { value.unwrap(); }\n',
        'pub fn prod_expect() { value.expect("valid invariant"); }\n',
        '#[cfg(test)]\n',
        'mod tests {\n',
        '    fn helper() { value.unwrap(); }\n',
        '    #[test]\n',
        '    fn case() { value.expect("test fixture"); }\n',
        '}\n',
        '#[test]\n',
        'fn outer_case() { value.unwrap(); }\n',
        "pub fn lifetime<'a>(value: &'a str, label: &'static str) { let _ = (value, label); }\n",
        '#[test]\n',
        'fn after_lifetime() { value.unwrap(); }\n',
        'pub fn custom_expect(cursor: &mut Cursor) -> Result<(), Error> { cursor.expect(TOKEN, "ctx")?; Ok(()) }\n',
        'pub fn ignore_string() { let _ = "{ not a brace } .unwrap()"; }\n',
    ]
    sites = scan_lines(Path("sample.rs"), sample)
    non_test = [site for site in sites if not site.in_test]
    test_only = [site for site in sites if site.in_test]
    assert len(sites) == 6, sites
    assert len(non_test) == 2, non_test
    assert len(test_only) == 4, test_only
    policy = [
        PolicyEntry(
            path="sample.rs",
            pattern='expect("valid invariant")',
            reason="self-test invariant",
            category="documented_invariant",
        )
    ]
    assert policy_match(non_test[1], policy) == policy[0]


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path("src"))
    parser.add_argument("--policy", type=Path, default=DEFAULT_POLICY)
    parser.add_argument("--limit", type=int, default=40)
    parser.add_argument("--json", action="store_true", help="Emit JSON output.")
    parser.add_argument("--json-output", help="Write JSON output to this path.")
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()

    if args.self_test:
        self_test()
        return 0

    if not args.root.exists():
        print(f"{args.root}: path does not exist", file=sys.stderr)
        return 2

    sites = scan_root(args.root)
    try:
        policy = load_policy(args.policy)
    except ValueError as error:
        print(error, file=sys.stderr)
        return 2
    report = build_report(args.root, args.policy, sites, policy)

    if args.json_output:
        output_path = Path(args.json_output)
        output_path.parent.mkdir(parents=True, exist_ok=True)
        output_path.write_text(
            json.dumps(report, indent=2, sort_keys=True) + "\n",
            encoding="utf-8",
        )

    if args.json:
        print(json.dumps(report, indent=2, sort_keys=True))
        unclassified = report["unclassified_non_test_sites"]
    else:
        unclassified = print_report(args.root, args.policy, sites, policy, args.limit)
    return 1 if unclassified else 0


if __name__ == "__main__":
    raise SystemExit(main())
