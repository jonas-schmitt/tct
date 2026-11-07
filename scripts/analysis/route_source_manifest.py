#!/usr/bin/env python3
"""Build deterministic source manifests for non-JSON route evidence."""

from __future__ import annotations

import argparse
import fnmatch
import hashlib
import json
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Any


EXCLUDED_DIR_NAMES = {
    ".git",
    ".hg",
    ".svn",
    ".venv",
    "__pycache__",
    "bin",
    "build",
    "dist",
    "node_modules",
    "obj",
    "target",
    "vendor",
}


@dataclass(frozen=True)
class TargetConfig:
    target: str
    label: str
    extension: str
    marker: str


TARGETS = {
    "go-syntax": TargetConfig(
        target="go-syntax",
        label="Go Syntax",
        extension=".go",
        marker="tct-go-syntax-route-corpus-v1",
    ),
    "csharp": TargetConfig(
        target="csharp",
        label="C#",
        extension=".cs",
        marker="tct-csharp-route-corpus-v1",
    ),
}


def file_artifact(path: Path, display_path: str | None = None) -> dict[str, Any]:
    contents = path.read_bytes()
    return {
        "path": display_path or path.as_posix(),
        "sha256": hashlib.sha256(contents).hexdigest(),
        "bytes": len(contents),
    }


def corpus_fingerprint(entries: list[dict[str, Any]]) -> dict[str, Any] | None:
    if not entries:
        return None
    hasher = hashlib.sha256()
    for entry in entries:
        file = entry["file"]
        hasher.update(entry["manifest_path"].encode("utf-8"))
        hasher.update(b"\0")
        hasher.update(file["sha256"].encode("ascii"))
        hasher.update(b"\n")
    return {
        "fingerprint_kind": "path_sha256_list_v1",
        "sha256": hasher.hexdigest(),
        "entry_count": len(entries),
    }


def corpus_provenance(
    corpus_id: str | None,
    source: str | None,
    license_name: str | None,
    selection_note: str | None,
) -> dict[str, Any]:
    return {
        "provided": any((corpus_id, source, license_name, selection_note)),
        "corpus_id": corpus_id,
        "source": source,
        "license": license_name,
        "selection_note": selection_note,
    }


def render_markdown_report(report: dict[str, Any], max_entries: int = 20) -> str:
    fingerprint = report.get("corpus_fingerprint") or {}
    provenance = report.get("corpus_provenance") or {}
    minimum_entry_count = report["minimum_entry_count"]
    minimum_entry_count_text = (
        "none" if minimum_entry_count is None else str(minimum_entry_count)
    )
    lines = [
        f"# {report['target_label']} Route Source Manifest",
        "",
        "## Summary",
        "",
        f"- Target: `{report['target']}`",
        f"- Source root: `{report['source_root']}`",
        f"- Manifest: `{report['output']}`",
        f"- Entry count: {report['entry_count']}",
        f"- Minimum entry count: {minimum_entry_count_text}",
        f"- Fixture scope: {str(report['fixture_scope']).lower()}",
        f"- Evidence role: `{report['evidence_role']}`",
        f"- Corpus fingerprint kind: `{fingerprint.get('fingerprint_kind')}`",
        f"- Corpus fingerprint SHA-256: `{fingerprint.get('sha256')}`",
        f"- Corpus ID: `{provenance.get('corpus_id')}`",
        f"- Corpus source: `{provenance.get('source')}`",
        f"- Corpus license: `{provenance.get('license')}`",
        f"- Selection note: `{provenance.get('selection_note')}`",
        "",
        "## Selection Filters",
        "",
        "- Excluded directory names: "
        + ", ".join(f"`{name}`" for name in report["excluded_dir_names"]),
        "- Exclude globs: "
        + (
            ", ".join(f"`{pattern}`" for pattern in report["exclude_globs"])
            if report["exclude_globs"]
            else "none"
        ),
        "",
        "## Selected Entries",
        "",
    ]
    entries = report["entries"]
    for entry in entries[:max_entries]:
        file = entry["file"]
        lines.append(
            f"- `{entry['manifest_path']}` ({file['bytes']} bytes, "
            f"`{file['sha256']}`)"
        )
    omitted = len(entries) - max_entries
    if omitted > 0:
        lines.append(f"- ... {omitted} additional entries omitted from Markdown")
    lines.append("")
    return "\n".join(lines)


def path_is_checked_in_fixture(path: Path) -> bool:
    parts = path.parts
    return "tests" in parts and "fixtures" in parts


def matches_exclude_glob(relative_path: str, exclude_globs: list[str]) -> bool:
    return any(fnmatch.fnmatchcase(relative_path, pattern) for pattern in exclude_globs)


def should_skip_path(source_root: Path, path: Path, exclude_globs: list[str]) -> bool:
    if path.name in EXCLUDED_DIR_NAMES:
        return True
    relative_path = path.relative_to(source_root).as_posix()
    return matches_exclude_glob(relative_path, exclude_globs)


def discover_sources(
    source_root: Path,
    extension: str,
    exclude_globs: list[str],
) -> list[Path]:
    discovered: list[Path] = []
    stack = [source_root]
    while stack:
        current = stack.pop()
        for child in sorted(current.iterdir(), key=lambda path: path.name):
            if should_skip_path(source_root, child, exclude_globs):
                continue
            if child.is_dir():
                stack.append(child)
                continue
            if child.is_file() and child.suffix == extension:
                discovered.append(child)
    return sorted(discovered, key=lambda path: path.relative_to(source_root).as_posix())


def build_manifest(
    config: TargetConfig,
    source_root: Path,
    output_path: Path,
    limit: int | None,
    min_files: int | None,
    exclude_globs: list[str],
    corpus_id: str | None = None,
    corpus_source: str | None = None,
    corpus_license: str | None = None,
    selection_note: str | None = None,
) -> dict[str, Any]:
    if not source_root.is_dir():
        raise ValueError(f"source root is not a directory: {source_root}")
    sources = discover_sources(source_root, config.extension, exclude_globs)
    if limit is not None:
        sources = sources[:limit]
    if not sources:
        raise ValueError(
            f"no {config.extension} files found under source root: {source_root}"
        )
    if min_files is not None and len(sources) < min_files:
        raise ValueError(
            f"only {len(sources)} {config.extension} file(s) selected under "
            f"{source_root}; minimum is {min_files}"
        )

    entries = []
    manifest_lines = [f"# {config.marker}"]
    for source in sources:
        relative = source.relative_to(source_root).as_posix()
        manifest_lines.append(relative)
        entries.append(
            {
                "manifest_path": relative,
                "file": file_artifact(source, relative),
            }
        )

    output_path.parent.mkdir(parents=True, exist_ok=True)
    output_path.write_text("\n".join(manifest_lines) + "\n", encoding="utf-8")
    manifest_artifact = file_artifact(output_path)

    return {
        "tool": "route_source_manifest",
        "version": 1,
        "target": config.target,
        "target_label": config.label,
        "source_root": source_root.as_posix(),
        "output": output_path.as_posix(),
        "extension": config.extension,
        "route_decision_marker": config.marker,
        "entry_count": len(entries),
        "minimum_entry_count": min_files,
        "fixture_scope": path_is_checked_in_fixture(source_root),
        "evidence_role": (
            "checked_in_fixture_smoke"
            if path_is_checked_in_fixture(source_root)
            else "route_intended_corpus"
        ),
        "excluded_dir_names": sorted(EXCLUDED_DIR_NAMES),
        "exclude_globs": sorted(exclude_globs),
        "manifest": manifest_artifact,
        "entries": entries,
        "corpus_fingerprint": corpus_fingerprint(entries),
        "corpus_provenance": corpus_provenance(
            corpus_id,
            corpus_source,
            corpus_license,
            selection_note,
        ),
    }


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--target", choices=sorted(TARGETS))
    parser.add_argument("--source-root", type=Path)
    parser.add_argument("--output", type=Path)
    parser.add_argument("--json-output", type=Path, help="Write JSON report")
    parser.add_argument("--markdown-output", type=Path, help="Write Markdown report")
    parser.add_argument(
        "--limit",
        type=int,
        help="Limit selected files after deterministic sorting",
    )
    parser.add_argument(
        "--min-files",
        type=int,
        help="Fail unless at least this many files remain after filtering and limit",
    )
    parser.add_argument(
        "--exclude-glob",
        action="append",
        default=[],
        help="Repo-relative glob to exclude before selecting source files",
    )
    parser.add_argument(
        "--corpus-id",
        help="Stable corpus identifier to record in the JSON/Markdown report",
    )
    parser.add_argument(
        "--corpus-source",
        help="Human-readable corpus source or URI to record as provenance",
    )
    parser.add_argument(
        "--corpus-license",
        help="Corpus license label to record as provenance",
    )
    parser.add_argument(
        "--selection-note",
        help="Short note describing filtering or selection policy",
    )
    parser.add_argument("--self-test", action="store_true", help="Run built-in checks")
    return parser.parse_args(argv)


def self_test() -> None:
    import tempfile

    with tempfile.TemporaryDirectory() as tmpdir:
        tmp = Path(tmpdir)
        source_root = tmp / "src"
        source_root.mkdir()
        (source_root / "b.cs").write_text("class B {}\n", encoding="utf-8")
        (source_root / "a.cs").write_text("class A {}\n", encoding="utf-8")
        (source_root / "obj").mkdir()
        (source_root / "obj" / "ignored.cs").write_text("", encoding="utf-8")
        output = tmp / "sources.txt"
        report = build_manifest(
            TARGETS["csharp"],
            source_root,
            output,
            None,
            2,
            ["generated/*"],
            corpus_id="self-test",
            corpus_source="temporary fixture",
            corpus_license="test-only",
            selection_note="self-test route manifest",
        )
        assert report["entry_count"] == 2
        assert output.read_text(encoding="utf-8").splitlines() == [
            "# tct-csharp-route-corpus-v1",
            "a.cs",
            "b.cs",
        ]
        assert report["corpus_fingerprint"]["fingerprint_kind"] == "path_sha256_list_v1"

        go_root = tmp / "go-src"
        go_root.mkdir()
        (go_root / "sample.go").write_text("package sample\n\nfunc F() {}\n", encoding="utf-8")
        go_output = tmp / "go-sources.txt"
        go_report = build_manifest(
            TARGETS["go-syntax"],
            go_root,
            go_output,
            None,
            1,
            [],
            corpus_id="self-test-go-syntax",
            corpus_source="temporary Go fixture",
            corpus_license="test-only",
            selection_note="go syntax route manifest",
        )
        assert go_report["target"] == "go-syntax"
        assert go_output.read_text(encoding="utf-8").splitlines() == [
            "# tct-go-syntax-route-corpus-v1",
            "sample.go",
        ]


def main(argv: list[str]) -> int:
    args = parse_args(argv)
    if args.self_test:
        self_test()
        print("route_source_manifest self-test passed")
        return 0
    if args.target is None:
        print("--target is required unless --self-test is used", file=sys.stderr)
        return 2
    if args.source_root is None:
        print("--source-root is required unless --self-test is used", file=sys.stderr)
        return 2
    if args.output is None:
        print("--output is required unless --self-test is used", file=sys.stderr)
        return 2
    if args.limit is not None and args.limit < 1:
        print("--limit must be at least 1", file=sys.stderr)
        return 2
    if args.min_files is not None and args.min_files < 1:
        print("--min-files must be at least 1", file=sys.stderr)
        return 2
    config = TARGETS[args.target]
    try:
        report = build_manifest(
            config,
            args.source_root.resolve(),
            args.output,
            args.limit,
            args.min_files,
            args.exclude_glob,
            corpus_id=args.corpus_id,
            corpus_source=args.corpus_source,
            corpus_license=args.corpus_license,
            selection_note=args.selection_note,
        )
    except (OSError, ValueError) as error:
        print(f"route source manifest failed: {error}", file=sys.stderr)
        return 1

    rendered_json = json.dumps(report, indent=2, sort_keys=True) + "\n"
    if args.json_output:
        args.json_output.parent.mkdir(parents=True, exist_ok=True)
        args.json_output.write_text(rendered_json, encoding="utf-8")
    else:
        print(rendered_json, end="")
    if args.markdown_output:
        args.markdown_output.parent.mkdir(parents=True, exist_ok=True)
        args.markdown_output.write_text(render_markdown_report(report), encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
