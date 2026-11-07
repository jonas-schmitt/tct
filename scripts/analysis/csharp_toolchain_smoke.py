#!/usr/bin/env python3
"""Optional C# dotnet smoke for non-JSON frontend planning."""

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
from dataclasses import asdict, dataclass
from pathlib import Path
from typing import Iterable


DIAGNOSTIC_RE = re.compile(
    r"^(?P<path>.+?)\((?P<line>\d+),(?P<column>\d+)\):\s+"
    r"(?P<severity>error|warning)\s+(?P<code>CS\d+):\s+"
    r"(?P<message>.*?)(?:\s+\[.*\])?$"
)
PARSE_DIAGNOSTIC_CODES = {
    "CS1001",
    "CS1002",
    "CS1003",
    "CS1009",
    "CS1010",
    "CS1011",
    "CS1012",
    "CS1013",
    "CS1014",
    "CS1015",
    "CS1018",
    "CS1022",
    "CS1031",
    "CS1039",
    "CS1040",
    "CS1513",
    "CS1514",
    "CS1519",
    "CS1525",
    "CS1526",
    "CS1527",
    "CS1528",
    "CS1529",
    "CS1530",
    "CS1585",
}
ROUTE_DECISION_MANIFEST_MARKER = "tct-csharp-route-corpus-v1"


@dataclass(frozen=True)
class SourceFile:
    path: Path
    display_path: str


@dataclass
class Diagnostic:
    code: str
    severity: str
    line: int
    column: int
    message: str
    category: str


@dataclass
class FileCheck:
    path: str
    bytes: int
    sha256: str
    checked: bool
    success: bool | None
    exit_code: int | None
    diagnostics: list[Diagnostic]


def discover_cs_files(paths: Iterable[Path]) -> list[SourceFile]:
    files: dict[str, SourceFile] = {}
    for path in paths:
        if path.is_file() and path.suffix == ".cs":
            files[str(path.resolve())] = SourceFile(path, path.as_posix())
        elif path.is_dir():
            for child in path.rglob("*"):
                if child.is_file() and child.suffix == ".cs":
                    files[str(child.resolve())] = SourceFile(child, child.as_posix())
    return sorted(files.values(), key=lambda item: item.display_path)


def trim_manifest_line(raw_line: str) -> str:
    without_comment = raw_line.split("#", maxsplit=1)[0]
    return without_comment.strip()


def file_artifact(path: Path, display_path: str | None = None) -> dict:
    contents = path.read_bytes()
    return {
        "path": display_path or path.as_posix(),
        "sha256": hashlib.sha256(contents).hexdigest(),
        "bytes": len(contents),
    }


def read_source_manifest(
    manifest_path: Path,
    source_root: Path,
) -> tuple[list[SourceFile], dict]:
    manifest = file_artifact(manifest_path)
    sources: list[SourceFile] = []
    entries = []
    marker_present = False
    seen_entries: set[str] = set()
    for line_number, raw_line in enumerate(
        manifest_path.read_text(encoding="utf-8").splitlines(), start=1
    ):
        stripped = raw_line.strip()
        if stripped.startswith("#") and ROUTE_DECISION_MANIFEST_MARKER in stripped:
            marker_present = True
        source_spec = trim_manifest_line(raw_line)
        if not source_spec:
            continue
        source_path = Path(source_spec)
        if source_path.is_absolute():
            raise ValueError(
                f"{manifest_path}:{line_number}: source paths must be relative "
                "to --source-root"
            )
        if source_path.suffix != ".cs":
            raise ValueError(
                f"{manifest_path}:{line_number}: source file is not a .cs file: "
                f"{source_spec}"
            )
        if source_spec in seen_entries:
            raise ValueError(
                f"{manifest_path}:{line_number}: duplicate source manifest entry: "
                f"{source_spec}"
            )
        seen_entries.add(source_spec)
        absolute_source = source_root / source_path
        if not absolute_source.is_file():
            raise FileNotFoundError(
                f"{manifest_path}:{line_number}: source file not found: "
                f"{source_spec}"
            )
        source = SourceFile(absolute_source, source_path.as_posix())
        sources.append(source)
        entries.append(
            {
                "manifest_path": source_spec,
                "line": line_number,
                "file": file_artifact(absolute_source, source.display_path),
            }
        )

    return sources, {
        "manifest": manifest,
        "source_root": source_root.as_posix(),
        "entry_count": len(entries),
        "route_decision_marker": ROUTE_DECISION_MANIFEST_MARKER,
        "route_decision_marker_present": marker_present,
        "entries": entries,
    }


def file_sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def corpus_fingerprint(files: list[FileCheck]) -> str:
    digest = hashlib.sha256()
    digest.update(b"tct-csharp-corpus-path-sha256-list-v1\n")
    for file in sorted(files, key=lambda item: item.path):
        digest.update(file.path.encode("utf-8"))
        digest.update(b"\0")
        digest.update(file.sha256.encode("ascii"))
        digest.update(b"\n")
    return digest.hexdigest()


def find_dotnet(explicit: str | None) -> str | None:
    if explicit:
        return explicit
    env_dotnet = os.environ.get("TCT_DOTNET_BIN")
    if env_dotnet:
        return env_dotnet
    return shutil.which("dotnet")


def dotnet_version(dotnet_bin: str) -> str | None:
    try:
        completed = subprocess.run(
            [dotnet_bin, "--version"],
            check=False,
            capture_output=True,
            text=True,
        )
    except OSError:
        return None
    text = (completed.stdout or completed.stderr).strip()
    return text or None


def classify_diagnostic(code: str) -> str:
    return "parse" if code in PARSE_DIAGNOSTIC_CODES else "other"


def parse_diagnostics(output: str) -> list[Diagnostic]:
    diagnostics: list[Diagnostic] = []
    for line in output.splitlines():
        match = DIAGNOSTIC_RE.match(line.strip())
        if not match:
            continue
        code = match.group("code")
        diagnostics.append(
            Diagnostic(
                code=code,
                severity=match.group("severity"),
                line=int(match.group("line")),
                column=int(match.group("column")),
                message=match.group("message"),
                category=classify_diagnostic(code),
            )
        )
    return diagnostics


def write_smoke_project(project_dir: Path, source: Path, target_framework: str) -> Path:
    project = project_dir / "TctCSharpSmoke.csproj"
    project.write_text(
        "\n".join(
            [
                '<Project Sdk="Microsoft.NET.Sdk">',
                "  <PropertyGroup>",
                "    <OutputType>Library</OutputType>",
                f"    <TargetFramework>{target_framework}</TargetFramework>",
                "    <LangVersion>latest</LangVersion>",
                "    <ImplicitUsings>enable</ImplicitUsings>",
                "    <Nullable>enable</Nullable>",
                "  </PropertyGroup>",
                "</Project>",
                "",
            ]
        ),
        encoding="utf-8",
    )
    (project_dir / "Input.cs").write_bytes(source.read_bytes())
    return project


def run_dotnet_file(
    dotnet_bin: str, path: Path, target_framework: str
) -> tuple[int, list[Diagnostic]]:
    with tempfile.TemporaryDirectory() as tmpdir:
        project_dir = Path(tmpdir)
        project = write_smoke_project(project_dir, path, target_framework)
        completed = subprocess.run(
            [
                dotnet_bin,
                "build",
                str(project),
                "--nologo",
                "-v:minimal",
            ],
            check=False,
            capture_output=True,
            text=True,
        )
    diagnostics = parse_diagnostics("\n".join([completed.stdout, completed.stderr]))
    return completed.returncode, diagnostics


def build_report(
    paths: list[Path],
    dotnet_bin: str | None,
    target_framework: str,
    source_manifest: Path | None = None,
    source_root: Path | None = None,
) -> dict:
    manifest_summary = None
    if source_manifest is not None:
        files, manifest_summary = read_source_manifest(
            source_manifest,
            source_root or Path("."),
        )
    else:
        files = discover_cs_files(paths)
    resolved_dotnet = find_dotnet(dotnet_bin)
    version = dotnet_version(resolved_dotnet) if resolved_dotnet else None
    available = bool(resolved_dotnet and version)
    checks: list[FileCheck] = []

    for source in files:
        if available and resolved_dotnet:
            exit_code, diagnostics = run_dotnet_file(
                resolved_dotnet, source.path, target_framework
            )
            checks.append(
                FileCheck(
                    path=source.display_path,
                    bytes=source.path.stat().st_size,
                    sha256=file_sha256(source.path),
                    checked=True,
                    success=exit_code == 0,
                    exit_code=exit_code,
                    diagnostics=diagnostics,
                )
            )
        else:
            checks.append(
                FileCheck(
                    path=source.display_path,
                    bytes=source.path.stat().st_size,
                    sha256=file_sha256(source.path),
                    checked=False,
                    success=None,
                    exit_code=None,
                    diagnostics=[],
                )
            )

    checked = [check for check in checks if check.checked]
    diagnostics = [diag for check in checks for diag in check.diagnostics]
    summary = {
        "files": len(checks),
        "checked": len(checked),
        "successes": sum(1 for check in checked if check.success),
        "failures": sum(1 for check in checked if check.success is False),
        "diagnostics": len(diagnostics),
        "parse_diagnostics": sum(1 for diag in diagnostics if diag.category == "parse"),
    }
    if manifest_summary is not None:
        scope = {
            "input_kind": "source_manifest",
            "source_manifest": source_manifest.as_posix(),
            "source_root": (source_root or Path(".")).as_posix(),
            "target_framework": target_framework,
        }
    else:
        scope = {
            "input_kind": "paths",
            "inputs": [path.as_posix() for path in paths],
            "target_framework": target_framework,
        }
    return {
        "tool": "csharp_toolchain_smoke",
        "version": 1,
        "scope": scope,
        "source_manifest": manifest_summary,
        "corpus": {
            "fingerprint_kind": "path_sha256_list_v1",
            "file_count": len(checks),
            "sha256": corpus_fingerprint(checks),
        },
        "toolchain": {
            "available": available,
            "dotnet_bin": resolved_dotnet,
            "version": version,
            "missing_reason": None if available else "dotnet not found or not executable",
        },
        "summary": summary,
        "files": [asdict(check) for check in checks],
        "notes": [
            "This is parser/compiler smoke evidence, not a TCT frontend implementation.",
            "The smoke builds each file as an isolated library project; project files, partial types, and external references remain out of scope.",
            "Use a pinned corpus before claiming frontend feasibility.",
            "Route-decision C# corpus manifests should include the tct-csharp-route-corpus-v1 marker.",
        ],
    }


def render_markdown(report: dict) -> str:
    summary = report["summary"]
    toolchain = report["toolchain"]
    lines = [
        "# C# Toolchain Smoke Report",
        "",
        "This report records optional dotnet smoke evidence for C# frontend planning.",
        "",
        "| Metric | Value |",
        "| --- | ---: |",
        f"| Toolchain available | {toolchain['available']} |",
        f"| Files | {summary['files']} |",
        f"| Checked | {summary['checked']} |",
        f"| Successes | {summary['successes']} |",
        f"| Failures | {summary['failures']} |",
        f"| Diagnostics | {summary['diagnostics']} |",
        f"| Parse diagnostics | {summary['parse_diagnostics']} |",
        "",
    ]
    if toolchain["version"]:
        lines.append(f"Toolchain: `{toolchain['version']}`")
        lines.append("")
    source_manifest = report.get("source_manifest")
    if source_manifest:
        lines.extend(
            [
                f"Source manifest: `{source_manifest['manifest']['path']}`",
                "",
                (
                    "Route-decision marker present: "
                    f"`{source_manifest['route_decision_marker_present']}`"
                ),
                "",
            ]
        )
    return "\n".join(lines)


def self_test() -> None:
    with tempfile.TemporaryDirectory() as tmpdir:
        source = Path(tmpdir) / "Sample.cs"
        source.write_text("public sealed class Sample { }\n", encoding="utf-8")
        report = build_report([source], "/definitely/missing/dotnet", "net8.0")
    assert report["toolchain"]["available"] is False
    assert report["summary"]["files"] == 1
    assert report["summary"]["checked"] == 0


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Run optional dotnet smoke checks for C# frontend planning."
    )
    parser.add_argument("paths", nargs="*", help="C# files or directories")
    parser.add_argument(
        "--source-manifest",
        help=(
            "Pinned newline-delimited C# source manifest. Entries must be "
            "relative to --source-root."
        ),
    )
    parser.add_argument(
        "--source-root",
        default=".",
        help="Base directory for relative --source-manifest entries",
    )
    parser.add_argument(
        "--dotnet-bin", help="Path to dotnet; defaults to TCT_DOTNET_BIN or PATH"
    )
    parser.add_argument(
        "--target-framework",
        default="net8.0",
        help="Target framework for temporary smoke projects",
    )
    parser.add_argument("--json-output", help="Write JSON report to this path")
    parser.add_argument("--markdown-output", help="Write Markdown report to this path")
    parser.add_argument("--self-test", action="store_true", help="Run built-in checks")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    if args.self_test:
        self_test()
        print("csharp_toolchain_smoke self-test passed")
        return 0
    if args.source_manifest and args.paths:
        raise SystemExit("error: provide either paths or --source-manifest, not both")
    if not args.paths and not args.source_manifest:
        raise SystemExit(
            "error: provide at least one path, --source-manifest, or use --self-test"
        )

    report = build_report(
        [Path(path) for path in args.paths],
        args.dotnet_bin,
        args.target_framework,
        Path(args.source_manifest) if args.source_manifest else None,
        Path(args.source_root),
    )
    rendered_json = json.dumps(report, indent=2, sort_keys=True) + "\n"
    if args.json_output:
        output = Path(args.json_output)
        output.parent.mkdir(parents=True, exist_ok=True)
        output.write_text(rendered_json, encoding="utf-8")
    if args.markdown_output:
        output = Path(args.markdown_output)
        output.parent.mkdir(parents=True, exist_ok=True)
        output.write_text(render_markdown(report), encoding="utf-8")
    if not args.json_output and not args.markdown_output:
        print(rendered_json, end="")
    return 0


if __name__ == "__main__":
    sys.exit(main())
