#!/usr/bin/env python3
"""Report Roslyn-backed syntax-metadata gaps for the checked C# corpus."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import shutil
import subprocess
import sys
import tempfile
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Iterable


TOOL = "csharp_real_source_metadata_gaps"
VERSION = 1
ROSLYN_PACKAGE = "Microsoft.CodeAnalysis.CSharp"
ROSLYN_PACKAGE_VERSION = "5.3.0"
TARGET_FRAMEWORK = "net10.0"
ROUTE_DECISION_MANIFEST_MARKER = "tct-csharp-route-corpus-v1"
DEFAULT_SOURCE_MANIFEST = Path("corpora/csharp/dotnet-samples-mini/sources.txt")
DEFAULT_SOURCE_ROOT = Path("corpora/csharp/dotnet-samples-mini/src")
DEFAULT_OUTPUT_DIR = Path("target/tct-csharp-real-source-metadata-gaps")

SUPPORTED_NODE_KINDS = {
    "AccessorDeclarationSyntax",
    "AccessorListSyntax",
    "ArgumentListSyntax",
    "ArgumentSyntax",
    "ArrayCreationExpressionSyntax",
    "ArrayRankSpecifierSyntax",
    "ArrayTypeSyntax",
    "ArrowExpressionClauseSyntax",
    "AttributeListSyntax",
    "AttributeSyntax",
    "AssignmentExpressionSyntax",
    "BaseListSyntax",
    "BinaryExpressionSyntax",
    "BlockSyntax",
    "BracketedArgumentListSyntax",
    "BreakStatementSyntax",
    "CaseSwitchLabelSyntax",
    "ClassDeclarationSyntax",
    "CompilationUnitSyntax",
    "ConditionalAccessExpressionSyntax",
    "ConstantPatternSyntax",
    "ConstructorDeclarationSyntax",
    "ElementAccessExpressionSyntax",
    "ElseClauseSyntax",
    "EnumDeclarationSyntax",
    "EnumMemberDeclarationSyntax",
    "EqualsValueClauseSyntax",
    "EventFieldDeclarationSyntax",
    "ExpressionStatementSyntax",
    "FieldDeclarationSyntax",
    "FileScopedNamespaceDeclarationSyntax",
    "ForEachStatementSyntax",
    "ForStatementSyntax",
    "GenericNameSyntax",
    "IdentifierNameSyntax",
    "IfStatementSyntax",
    "InitializerExpressionSyntax",
    "InterpolatedStringExpressionSyntax",
    "InterpolatedStringTextSyntax",
    "InterpolationSyntax",
    "InvocationExpressionSyntax",
    "IsPatternExpressionSyntax",
    "LiteralExpressionSyntax",
    "LocalDeclarationStatementSyntax",
    "MemberAccessExpressionSyntax",
    "MemberBindingExpressionSyntax",
    "MethodDeclarationSyntax",
    "NamespaceDeclarationSyntax",
    "NullableTypeSyntax",
    "ObjectCreationExpressionSyntax",
    "OmittedArraySizeExpressionSyntax",
    "ParameterListSyntax",
    "ParameterSyntax",
    "ParenthesizedExpressionSyntax",
    "PostfixUnaryExpressionSyntax",
    "PrefixUnaryExpressionSyntax",
    "PredefinedTypeSyntax",
    "PropertyDeclarationSyntax",
    "QualifiedNameSyntax",
    "RangeExpressionSyntax",
    "ReturnStatementSyntax",
    "SimpleBaseTypeSyntax",
    "StructDeclarationSyntax",
    "SwitchSectionSyntax",
    "SwitchStatementSyntax",
    "ThisExpressionSyntax",
    "TypeArgumentListSyntax",
    "UsingDirectiveSyntax",
    "VariableDeclarationSyntax",
    "VariableDeclaratorSyntax",
    "WhileStatementSyntax",
    "YieldStatementSyntax",
}

GAP_PRIORITIES = (
    {
        "gap_id": "csharp_block_scoped_namespace_syntax_metadata",
        "node_kinds": ("NamespaceDeclarationSyntax",),
        "next_tranche": "Add C# block-scoped namespace syntax-metadata route",
        "why": "Block-scoped namespace wrappers appear in the selected real-source corpus and currently sit before otherwise familiar class/member routes.",
    },
    {
        "gap_id": "csharp_using_directive_syntax_metadata",
        "node_kinds": ("UsingDirectiveSyntax",),
        "next_tranche": "Add C# using-directive syntax-metadata route",
        "why": "Using directives are common corpus front matter and should be handled before broader source-file claims.",
    },
    {
        "gap_id": "csharp_struct_declaration_syntax_metadata",
        "node_kinds": ("StructDeclarationSyntax",),
        "next_tranche": "Add C# struct declaration syntax-metadata route",
        "why": "Struct declarations appear in the selected corpus and are a narrow type-declaration extension.",
    },
    {
        "gap_id": "csharp_enum_declaration_syntax_metadata",
        "node_kinds": ("EnumDeclarationSyntax", "EnumMemberDeclarationSyntax"),
        "next_tranche": "Add C# enum declaration syntax-metadata route",
        "why": "Enums appear in the selected corpus and require finite member-list syntax evidence.",
    },
    {
        "gap_id": "csharp_event_declaration_syntax_metadata",
        "node_kinds": ("EventFieldDeclarationSyntax", "EventDeclarationSyntax"),
        "next_tranche": "Add C# event declaration syntax-metadata route",
        "why": "Event declarations appear in the selected corpus and need explicit member syntax handling.",
    },
    {
        "gap_id": "csharp_yield_statement_syntax_metadata",
        "node_kinds": ("YieldStatementSyntax",),
        "next_tranche": "Add C# yield statement syntax-metadata route",
        "why": "Iterator samples use yield statements, which are not part of the current route set.",
    },
    {
        "gap_id": "csharp_attribute_syntax_metadata",
        "node_kinds": ("AttributeListSyntax", "AttributeSyntax"),
        "next_tranche": "Add C# attribute syntax-metadata route",
        "why": "Attributes appear in the selected corpus and are a reusable member/type modifier surface.",
    },
    {
        "gap_id": "csharp_generic_name_syntax_metadata",
        "node_kinds": ("GenericNameSyntax", "TypeArgumentListSyntax"),
        "next_tranche": "Add C# generic name syntax-metadata route",
        "why": "Generic names appear in delegate/event and collection surfaces.",
    },
    {
        "gap_id": "csharp_qualified_name_syntax_metadata",
        "node_kinds": ("QualifiedNameSyntax",),
        "next_tranche": "Add C# qualified name syntax-metadata route",
        "why": (
            "Qualified names are the highest-count remaining unsupported syntax kind "
            "after completed assignment-expression and generic-name routes are counted "
            "as supported."
        ),
    },
    {
        "gap_id": "csharp_constructor_declaration_syntax_metadata",
        "node_kinds": ("ConstructorDeclarationSyntax",),
        "next_tranche": "Add C# constructor declaration syntax-metadata route",
        "why": "Constructor declarations are the highest-count remaining unsupported member syntax kind after qualified-name support.",
    },
    {
        "gap_id": "csharp_while_postfix_syntax_metadata",
        "node_kinds": ("WhileStatementSyntax", "PostfixUnaryExpressionSyntax"),
        "next_tranche": "Add C# while/postfix loop syntax-metadata route",
        "why": "Iterator corpus methods use while loops with postfix increments, making the statement and expression nodes the next coupled syntax-level gap after constructors.",
    },
    {
        "gap_id": "csharp_interpolation_syntax_metadata",
        "node_kinds": (
            "InterpolatedStringExpressionSyntax",
            "InterpolatedStringTextSyntax",
            "InterpolationSyntax",
        ),
        "next_tranche": "Add C# interpolated string syntax-metadata route",
        "why": "The selected corpus still uses interpolated string expressions after while/postfix support; these are the next coupled expression nodes in the real-source gap report.",
    },
    {
        "gap_id": "csharp_member_access_expression_syntax_metadata",
        "node_kinds": ("MemberAccessExpressionSyntax",),
        "next_tranche": "Add C# member access expression syntax-metadata route",
        "why": "Member access is the highest-count remaining concrete C# syntax node after interpolated-string support and appears across multiple selected corpus files.",
    },
    {
        "gap_id": "csharp_field_declaration_syntax_metadata",
        "node_kinds": ("FieldDeclarationSyntax",),
        "next_tranche": "Add C# field declaration syntax-metadata route",
        "why": "Field declarations are the next narrow reusable C# member-declaration gap after member-access expression support.",
    },
    {
        "gap_id": "csharp_base_list_syntax_metadata",
        "node_kinds": ("BaseListSyntax", "SimpleBaseTypeSyntax"),
        "next_tranche": "Add C# base list syntax-metadata route",
        "why": "Base type lists are the next narrow reusable C# type-declaration gap after field declaration support.",
    },
    {
        "gap_id": "csharp_conditional_access_syntax_metadata",
        "node_kinds": (
            "ConditionalAccessExpressionSyntax",
            "MemberBindingExpressionSyntax",
        ),
        "next_tranche": "Add C# conditional access syntax-metadata route",
        "why": "Conditional member access is the next narrow reusable C# expression gap after base-list support.",
    },
    {
        "gap_id": "csharp_array_rank_object_creation_syntax_metadata",
        "node_kinds": (
            "ObjectCreationExpressionSyntax",
            "ArrayRankSpecifierSyntax",
            "OmittedArraySizeExpressionSyntax",
        ),
        "next_tranche": "Add C# array-rank/object-creation syntax-metadata route",
        "why": (
            "The selected corpus now has no unprioritized higher-level route "
            "gaps; object creation plus array-rank syntax are the largest "
            "remaining coupled creation surface across event/property-change "
            "and iterator samples."
        ),
    },
    {
        "gap_id": "csharp_this_expression_syntax_metadata",
        "node_kinds": ("ThisExpressionSyntax",),
        "next_tranche": "Add C# this-expression syntax-metadata route",
        "why": (
            "`this` receiver expressions appear in the same real-source event "
            "and property-change call sites as object creation and are the next "
            "small reusable expression gap after creation/rank syntax."
        ),
    },
    {
        "gap_id": "csharp_pattern_expression_syntax_metadata",
        "node_kinds": ("IsPatternExpressionSyntax", "ConstantPatternSyntax"),
        "next_tranche": "Add C# pattern-expression syntax-metadata route",
        "why": (
            "Pattern matching remains in the logger sample after the creation "
            "and `this` expression gaps and should be handled as a coupled "
            "`is null` expression tranche."
        ),
    },
    {
        "gap_id": "csharp_arrow_expression_clause_syntax_metadata",
        "node_kinds": ("ArrowExpressionClauseSyntax",),
        "next_tranche": "Add C# arrow-expression-clause syntax-metadata route",
        "why": (
            "Arrow expression clauses remain as a narrow constructor/member "
            "body surface after the higher-count creation, `this`, and pattern "
            "expression gaps."
        ),
    },
)

NO_STABLE_CLAIMS = (
    "no_arena_ir_promotion",
    "no_source_program_validity_claim",
    "no_type_correctness_claim",
    "no_generated_codec_real_source_claim",
    "no_stable_backend_promotion",
)


@dataclass(frozen=True)
class SourceFile:
    path: Path
    display_path: str


def file_artifact(path: Path, display_path: str | None = None) -> dict[str, Any]:
    contents = path.read_bytes()
    return {
        "path": display_path or path.as_posix(),
        "sha256": hashlib.sha256(contents).hexdigest(),
        "bytes": len(contents),
    }


def trim_manifest_line(raw_line: str) -> str:
    without_comment = raw_line.split("#", maxsplit=1)[0]
    return without_comment.strip()


def read_source_manifest(
    manifest_path: Path,
    source_root: Path,
) -> tuple[list[SourceFile], dict[str, Any]]:
    manifest = file_artifact(manifest_path)
    marker_present = False
    sources: list[SourceFile] = []
    entries: list[dict[str, Any]] = []
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
                f"{manifest_path}:{line_number}: source paths must be relative"
            )
        if source_path.suffix != ".cs":
            raise ValueError(f"{manifest_path}:{line_number}: expected .cs entry")
        if source_spec in seen_entries:
            raise ValueError(f"{manifest_path}:{line_number}: duplicate entry")
        seen_entries.add(source_spec)
        absolute_source = source_root / source_path
        if not absolute_source.is_file():
            raise FileNotFoundError(f"{manifest_path}:{line_number}: {source_spec}")
        sources.append(SourceFile(absolute_source, source_path.as_posix()))
        entries.append(
            {
                "manifest_path": source_spec,
                "line": line_number,
                "file": file_artifact(absolute_source, source_path.as_posix()),
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


def corpus_fingerprint(files: Iterable[dict[str, Any]]) -> str:
    digest = hashlib.sha256()
    digest.update(b"tct-csharp-real-source-metadata-gap-corpus-v1\n")
    for file in sorted(files, key=lambda item: item["path"]):
        digest.update(file["path"].encode("utf-8"))
        digest.update(b"\0")
        digest.update(file["sha256"].encode("ascii"))
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
    return (completed.stdout or completed.stderr).strip() or None


def run_command(command: list[str], cwd: Path) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        command,
        cwd=cwd,
        check=False,
        capture_output=True,
        text=True,
    )


def write_project(project_dir: Path) -> Path:
    project_dir.mkdir(parents=True, exist_ok=True)
    project_path = project_dir / "TctCSharpRealSourceMetadataGaps.csproj"
    project_path.write_text(
        "\n".join(
            [
                '<Project Sdk="Microsoft.NET.Sdk">',
                "  <PropertyGroup>",
                "    <OutputType>Exe</OutputType>",
                f"    <TargetFramework>{TARGET_FRAMEWORK}</TargetFramework>",
                "    <LangVersion>latest</LangVersion>",
                "    <ImplicitUsings>enable</ImplicitUsings>",
                "    <Nullable>enable</Nullable>",
                "  </PropertyGroup>",
                "  <ItemGroup>",
                (
                    f'    <PackageReference Include="{ROSLYN_PACKAGE}" '
                    f'Version="{ROSLYN_PACKAGE_VERSION}" />'
                ),
                "  </ItemGroup>",
                "</Project>",
                "",
            ]
        ),
        encoding="utf-8",
    )
    (project_dir / "Program.cs").write_text(ROSLYN_PROGRAM, encoding="utf-8")
    return project_path


def restore_project(
    dotnet_bin: str,
    project_path: Path,
    repo_root: Path,
) -> tuple[dict[str, Any], dict[str, Any] | None]:
    command = [dotnet_bin, "restore", str(project_path)]
    completed = run_command(command, repo_root)
    command_record = {
        "command": command,
        "exit_code": completed.returncode,
    }
    if completed.returncode == 0:
        return command_record, None
    return command_record, {
        "kind": "dotnet_restore_failed",
        "command": command,
        "exit_code": completed.returncode,
        "stdout_tail": completed.stdout[-4000:],
        "stderr_tail": completed.stderr[-4000:],
    }


def run_roslyn_file(
    dotnet_bin: str,
    project_path: Path,
    source: SourceFile,
    dotnet_sdk_version: str,
    repo_root: Path,
) -> tuple[dict[str, Any] | None, dict[str, Any], dict[str, Any] | None]:
    command = [
        dotnet_bin,
        "run",
        "--no-restore",
        "--project",
        str(project_path),
        "--",
        str(source.path.resolve()),
        source.display_path,
        dotnet_sdk_version,
        ROSLYN_PACKAGE_VERSION,
    ]
    completed = run_command(command, repo_root)
    command_record = {"command": command, "exit_code": completed.returncode}
    if completed.returncode != 0:
        return None, command_record, {
            "kind": "dotnet_run_failed",
            "path": source.display_path,
            "stdout_tail": completed.stdout[-4000:],
            "stderr_tail": completed.stderr[-4000:],
        }
    try:
        parsed = json.loads(completed.stdout)
    except json.JSONDecodeError as error:
        return None, command_record, {
            "kind": "invalid_roslyn_json",
            "path": source.display_path,
            "message": str(error),
            "stdout_tail": completed.stdout[-4000:],
        }
    if not isinstance(parsed, dict):
        return None, command_record, {
            "kind": "invalid_roslyn_json",
            "path": source.display_path,
            "message": "expected object",
        }
    return parsed, command_record, None


def sorted_counts(data: dict[str, int]) -> list[dict[str, Any]]:
    return [
        {"kind": kind, "count": count}
        for kind, count in sorted(data.items(), key=lambda item: (-item[1], item[0]))
    ]


def merge_counts(reports: list[dict[str, Any]], field: str) -> dict[str, int]:
    merged: dict[str, int] = {}
    for report in reports:
        counts = report.get(field, {})
        if not isinstance(counts, dict):
            continue
        for kind, count in counts.items():
            merged[kind] = merged.get(kind, 0) + int(count)
    return merged


def classify_gaps(node_counts: dict[str, int]) -> list[dict[str, Any]]:
    gaps = []
    for priority, spec in enumerate(GAP_PRIORITIES):
        matched = [
            {"kind": kind, "count": node_counts[kind]}
            for kind in spec["node_kinds"]
            if kind in node_counts and kind not in SUPPORTED_NODE_KINDS
        ]
        if matched:
            gaps.append(
                {
                    "gap_id": spec["gap_id"],
                    "priority": priority,
                    "node_kinds": matched,
                    "next_tranche": spec["next_tranche"],
                    "why": spec["why"],
                }
            )
    supported = set(SUPPORTED_NODE_KINDS)
    prioritized = {
        kind for spec in GAP_PRIORITIES for kind in spec["node_kinds"]
    }
    other = [
        {"kind": kind, "count": count}
        for kind, count in sorted(
            node_counts.items(), key=lambda item: (-item[1], item[0])
        )
        if kind not in supported and kind not in prioritized
    ]
    if other:
        gaps.append(
            {
                "gap_id": "csharp_other_unsupported_syntax_metadata",
                "priority": len(GAP_PRIORITIES),
                "node_kinds": other,
                "next_tranche": "Classify remaining C# real-source unsupported syntax nodes",
                "why": "The selected corpus still contains node kinds outside the current syntax route table.",
            }
        )
    return gaps


def build_aggregate(
    source_files: list[SourceFile],
    manifest_summary: dict[str, Any],
    dotnet: dict[str, Any],
    file_reports: list[dict[str, Any]],
    blockers: list[dict[str, Any]],
    commands: dict[str, Any],
) -> dict[str, Any]:
    node_counts = merge_counts(file_reports, "node_kind_counts")
    token_counts = merge_counts(file_reports, "token_kind_counts")
    unsupported_counts = {
        kind: count
        for kind, count in node_counts.items()
        if kind not in SUPPORTED_NODE_KINDS
    }
    supported_counts = {
        kind: count
        for kind, count in node_counts.items()
        if kind in SUPPORTED_NODE_KINDS
    }
    parse_diagnostics = sum(
        len(report.get("parse_diagnostics", [])) for report in file_reports
    )
    parsed_files = sum(1 for report in file_reports if report.get("parsed") is True)
    parser_ready = (
        dotnet.get("available") is True
        and parsed_files == len(source_files)
        and parse_diagnostics == 0
        and not blockers
    )
    gaps = classify_gaps(node_counts)
    next_gap = gaps[0] if gaps else None
    file_artifacts = [
        file_artifact(source.path, source.display_path) for source in source_files
    ]
    return {
        "tool": TOOL,
        "version": VERSION,
        "status": "ready_with_gaps" if parser_ready else "blocked",
        "source_manifest": manifest_summary,
        "corpus": {
            "fingerprint_kind": "path_sha256_list_v1",
            "file_count": len(source_files),
            "sha256": corpus_fingerprint(file_artifacts),
        },
        "parser": {
            "tool": "Roslyn CSharpSyntaxTree",
            "package": ROSLYN_PACKAGE,
            "package_version": ROSLYN_PACKAGE_VERSION,
            "target_framework": TARGET_FRAMEWORK,
            "dotnet": dotnet,
        },
        "summary": {
            "files": len(source_files),
            "parsed_files": parsed_files,
            "parse_diagnostics": parse_diagnostics,
            "supported_node_kind_count": len(supported_counts),
            "unsupported_node_kind_count": len(unsupported_counts),
            "unsupported_gap_count": len(gaps),
        },
        "supported_route_fragments": sorted_counts(supported_counts),
        "unsupported_node_kinds": sorted_counts(unsupported_counts),
        "token_kinds": sorted_counts(token_counts),
        "unsupported_route_gaps": gaps,
        "next_smallest_tranche": next_gap,
        "route_readiness_boundary": {
            "parser_backed_gap_report_ready": parser_ready,
            "claim": "sidecar C# real-source syntax-metadata gap evidence only",
            "no_stable_claims": list(NO_STABLE_CLAIMS),
        },
        "files": file_reports,
        "commands": commands,
        "blockers": blockers,
    }


def build_report(args: argparse.Namespace, repo_root: Path) -> dict[str, Any]:
    source_manifest = repo_root / args.source_manifest
    source_root = repo_root / args.source_root
    sources, manifest_summary = read_source_manifest(source_manifest, source_root)
    dotnet_bin = find_dotnet(args.dotnet_bin)
    dotnet_sdk_version = dotnet_version(dotnet_bin) if dotnet_bin else None
    dotnet = {
        "available": bool(dotnet_bin and dotnet_sdk_version),
        "dotnet_bin": dotnet_bin,
        "version": dotnet_sdk_version,
        "missing_reason": None
        if dotnet_bin and dotnet_sdk_version
        else "dotnet not found or not executable",
    }
    file_reports: list[dict[str, Any]] = []
    blockers: list[dict[str, Any]] = []
    commands: dict[str, Any] = {}
    if dotnet_bin and dotnet_sdk_version:
        work_dir = repo_root / args.work_dir
        if work_dir.exists():
            shutil.rmtree(work_dir)
        project_path = write_project(work_dir / "project")
        restore_command, blocker = restore_project(dotnet_bin, project_path, repo_root)
        commands["restore"] = restore_command
        if blocker:
            blockers.append(blocker)
        else:
            run_commands = []
            for source in sources:
                parsed, command, file_blocker = run_roslyn_file(
                    dotnet_bin,
                    project_path,
                    source,
                    dotnet_sdk_version,
                    repo_root,
                )
                run_commands.append(command)
                if file_blocker:
                    blockers.append(file_blocker)
                if parsed is None:
                    parsed = {
                        "path": source.display_path,
                        "parsed": False,
                        "parse_diagnostics": [],
                        "node_kind_counts": {},
                        "token_kind_counts": {},
                    }
                parsed["source"] = file_artifact(source.path, source.display_path)
                file_reports.append(parsed)
            commands["run"] = run_commands
    else:
        blockers.append(
            {
                "kind": "dotnet_unavailable",
                "message": "dotnet was not found or did not report a version",
            }
        )
        for source in sources:
            file_reports.append(
                {
                    "path": source.display_path,
                    "parsed": False,
                    "parse_diagnostics": [],
                    "node_kind_counts": {},
                    "token_kind_counts": {},
                    "source": file_artifact(source.path, source.display_path),
                }
            )
    return build_aggregate(sources, manifest_summary, dotnet, file_reports, blockers, commands)


def render_markdown(report: dict[str, Any]) -> str:
    summary = report["summary"]
    parser = report["parser"]
    dotnet = parser["dotnet"]
    lines = [
        "# C# Real-Source Syntax Metadata Gap Report",
        "",
        "This report is sidecar parser-backed gap evidence only.",
        "",
        "| Metric | Value |",
        "| --- | ---: |",
        f"| Parser-backed gap report ready | {report['route_readiness_boundary']['parser_backed_gap_report_ready']} |",
        f"| Files | {summary['files']} |",
        f"| Parsed files | {summary['parsed_files']} |",
        f"| Parse diagnostics | {summary['parse_diagnostics']} |",
        f"| Supported node kinds | {summary['supported_node_kind_count']} |",
        f"| Unsupported node kinds | {summary['unsupported_node_kind_count']} |",
        f"| Unsupported gap classes | {summary['unsupported_gap_count']} |",
        "",
        f"dotnet: `{dotnet.get('version')}`",
        "",
    ]
    next_gap = report.get("next_smallest_tranche")
    if next_gap:
        lines.extend(
            [
                "Next smallest tranche:",
                "",
                f"- `{next_gap['gap_id']}`: {next_gap['next_tranche']}",
                "",
            ]
        )
    lines.append("Unsupported route gaps:")
    lines.append("")
    for gap in report["unsupported_route_gaps"]:
        kinds = ", ".join(f"`{item['kind']}`" for item in gap["node_kinds"])
        lines.append(f"- `{gap['gap_id']}`: {kinds}")
    lines.append("")
    return "\n".join(lines)


def write_report(report: dict[str, Any], json_output: Path | None, markdown_output: Path | None) -> None:
    text = json.dumps(report, indent=2, sort_keys=True) + "\n"
    if json_output:
        json_output.parent.mkdir(parents=True, exist_ok=True)
        json_output.write_text(text, encoding="utf-8")
    if markdown_output:
        markdown_output.parent.mkdir(parents=True, exist_ok=True)
        markdown_output.write_text(render_markdown(report), encoding="utf-8")
    if not json_output and not markdown_output:
        print(text, end="")


def self_test() -> None:
    with tempfile.TemporaryDirectory() as tmpdir:
        root = Path(tmpdir)
        source_root = root / "src"
        source_root.mkdir()
        source = source_root / "Sample.cs"
        source.write_text(
            "namespace Demo { public sealed class Sample { } }\n",
            encoding="utf-8",
        )
        manifest = root / "sources.txt"
        manifest.write_text(
            f"# {ROUTE_DECISION_MANIFEST_MARKER}\nSample.cs\n",
            encoding="utf-8",
        )
        sources, manifest_summary = read_source_manifest(manifest, source_root)
        fake_report = {
            "path": "Sample.cs",
            "parsed": True,
            "parse_diagnostics": [],
            "node_kind_counts": {
                "CompilationUnitSyntax": 1,
                "NamespaceDeclarationSyntax": 1,
                "ClassDeclarationSyntax": 1,
                "UsingDirectiveSyntax": 1,
                "StructDeclarationSyntax": 1,
                "EnumDeclarationSyntax": 1,
                "EventFieldDeclarationSyntax": 1,
                "YieldStatementSyntax": 1,
                "AttributeListSyntax": 1,
                "AttributeSyntax": 1,
                "ExpressionStatementSyntax": 1,
                "AssignmentExpressionSyntax": 1,
                "GenericNameSyntax": 1,
                "TypeArgumentListSyntax": 1,
                "QualifiedNameSyntax": 1,
                "ConstructorDeclarationSyntax": 1,
                "WhileStatementSyntax": 1,
                "PostfixUnaryExpressionSyntax": 1,
                "InterpolatedStringExpressionSyntax": 1,
                "InterpolatedStringTextSyntax": 1,
                "InterpolationSyntax": 1,
                "MemberAccessExpressionSyntax": 1,
                "FieldDeclarationSyntax": 1,
                "BaseListSyntax": 1,
                "SimpleBaseTypeSyntax": 1,
                "ConditionalAccessExpressionSyntax": 1,
                "MemberBindingExpressionSyntax": 1,
                "ObjectCreationExpressionSyntax": 2,
                "ArrayRankSpecifierSyntax": 2,
                "ThisExpressionSyntax": 2,
                "ArrowExpressionClauseSyntax": 1,
                "ConstantPatternSyntax": 1,
                "IsPatternExpressionSyntax": 1,
                "OmittedArraySizeExpressionSyntax": 1,
            },
            "token_kind_counts": {"IdentifierToken": 2},
            "source": file_artifact(source, "Sample.cs"),
        }
        report = build_aggregate(
            sources,
            manifest_summary,
            {
                "available": True,
                "dotnet_bin": "dotnet",
                "version": "10.0.100",
                "missing_reason": None,
            },
            [fake_report],
            [],
            {},
        )
    assert report["route_readiness_boundary"]["parser_backed_gap_report_ready"] is True
    assert report["next_smallest_tranche"] is None
    assert report["summary"]["unsupported_node_kind_count"] == 0
    assert report["summary"]["unsupported_gap_count"] == 0


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source-manifest", type=Path, default=DEFAULT_SOURCE_MANIFEST)
    parser.add_argument("--source-root", type=Path, default=DEFAULT_SOURCE_ROOT)
    parser.add_argument("--work-dir", type=Path, default=DEFAULT_OUTPUT_DIR / "work")
    parser.add_argument("--json-output", type=Path)
    parser.add_argument("--markdown-output", type=Path)
    parser.add_argument("--dotnet-bin")
    parser.add_argument(
        "--require-parser-ready",
        action="store_true",
        help="Exit nonzero unless every selected file parsed without diagnostics.",
    )
    parser.add_argument("--self-test", action="store_true")
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    args = parse_args(argv or sys.argv[1:])
    if args.self_test:
        self_test()
        print("csharp_real_source_metadata_gaps self-test passed")
        return 0
    repo_root = Path.cwd()
    report = build_report(args, repo_root)
    write_report(report, args.json_output, args.markdown_output)
    if (
        args.require_parser_ready
        and not report["route_readiness_boundary"]["parser_backed_gap_report_ready"]
    ):
        blockers = ", ".join(blocker.get("kind", "unknown") for blocker in report["blockers"])
        print(
            f"C# real-source parser-backed gap report is not ready: {blockers or 'parse diagnostics'}",
            file=sys.stderr,
        )
        return 1
    return 0


ROSLYN_PROGRAM = r'''
using System.Text.Json;
using Microsoft.CodeAnalysis;
using Microsoft.CodeAnalysis.CSharp;
using Microsoft.CodeAnalysis.CSharp.Syntax;

if (args.Length != 4)
{
    Console.Error.WriteLine("usage: <source-path> <source-display-path> <dotnet-version> <roslyn-package-version>");
    return 2;
}

var sourcePath = args[0];
var sourceDisplayPath = args[1];
var dotnetVersion = args[2];
var roslynPackageVersion = args[3];
var sourceText = File.ReadAllText(sourcePath);
var tree = CSharpSyntaxTree.ParseText(
    sourceText,
    CSharpParseOptions.Default.WithLanguageVersion(LanguageVersion.Preview),
    path: sourcePath);
var root = tree.GetCompilationUnitRoot();
var diagnostics = tree.GetDiagnostics()
    .Where(diagnostic => diagnostic.Severity == DiagnosticSeverity.Error)
    .Select(diagnostic => new
    {
        id = diagnostic.Id,
        message = diagnostic.GetMessage(),
        severity = diagnostic.Severity.ToString(),
        span = diagnostic.Location.GetLineSpan().ToString(),
    })
    .ToArray();

var nodeKindCounts = root.DescendantNodesAndSelf()
    .GroupBy(node => node.GetType().Name)
    .OrderBy(group => group.Key)
    .ToDictionary(group => group.Key, group => group.Count());
var tokenKindCounts = root.DescendantTokens(descendIntoTrivia: true)
    .GroupBy(token => token.Kind().ToString())
    .OrderBy(group => group.Key)
    .ToDictionary(group => group.Key, group => group.Count());
var topLevelMemberKinds = root.Members
    .Select(member => member.GetType().Name)
    .GroupBy(kind => kind)
    .OrderBy(group => group.Key)
    .ToDictionary(group => group.Key, group => group.Count());
var typeDeclarationKinds = root.DescendantNodes()
    .OfType<BaseTypeDeclarationSyntax>()
    .Select(node => node.GetType().Name)
    .GroupBy(kind => kind)
    .OrderBy(group => group.Key)
    .ToDictionary(group => group.Key, group => group.Count());
var memberDeclarationKinds = root.DescendantNodes()
    .OfType<MemberDeclarationSyntax>()
    .Select(node => node.GetType().Name)
    .GroupBy(kind => kind)
    .OrderBy(group => group.Key)
    .ToDictionary(group => group.Key, group => group.Count());
var statementKinds = root.DescendantNodes()
    .OfType<StatementSyntax>()
    .Select(node => node.GetType().Name)
    .GroupBy(kind => kind)
    .OrderBy(group => group.Key)
    .ToDictionary(group => group.Key, group => group.Count());

var report = new
{
    path = sourceDisplayPath,
    parsed = diagnostics.Length == 0,
    parser = new
    {
        tool = "Roslyn CSharpSyntaxTree",
        version = dotnetVersion,
        package = "Microsoft.CodeAnalysis.CSharp",
        package_version = roslynPackageVersion,
    },
    parse_diagnostics = diagnostics,
    node_kind_counts = nodeKindCounts,
    token_kind_counts = tokenKindCounts,
    top_level_member_kinds = topLevelMemberKinds,
    type_declaration_kinds = typeDeclarationKinds,
    member_declaration_kinds = memberDeclarationKinds,
    statement_kinds = statementKinds,
};
Console.WriteLine(JsonSerializer.Serialize(report));
return 0;
'''


if __name__ == "__main__":
    sys.exit(main())
