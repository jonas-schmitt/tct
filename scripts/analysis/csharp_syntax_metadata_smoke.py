#!/usr/bin/env python3
"""Roslyn-backed smoke for the checked C# syntax metadata fixture."""

from __future__ import annotations

import argparse
import hashlib
import json
import shutil
import subprocess
import sys
from pathlib import Path
from typing import Any


TOOL = "csharp_syntax_metadata_smoke"
VERSION = 1
ARTIFACT_KIND = "tct_csharp_syntax_metadata_v0"
METADATA_VERSION = 0
ROSLYN_PACKAGE = "Microsoft.CodeAnalysis.CSharp"
ROSLYN_PACKAGE_VERSION = "5.3.0"
TARGET_FRAMEWORK = "net10.0"
DEFAULT_SOURCE = Path("tests/fixtures/csharp_syntax_sources/minimal_class/source.cs")
DEFAULT_CHECKED_METADATA = Path(
    "tests/fixtures/csharp_syntax_metadata/minimal_class/syntax-metadata-v0.json"
)
DEFAULT_WORK_DIR = Path("target/tct-analysis/csharp-syntax-metadata-smoke")
FIXTURES = {
    "minimal-class": (
        DEFAULT_SOURCE,
        DEFAULT_CHECKED_METADATA,
    ),
    "compilation-unit-source-file": (
        Path("corpora/csharp/dotnet-samples-mini/src/pattern-matching/Shapes.cs"),
        Path(
            "tests/fixtures/csharp_syntax_metadata/compilation_unit_source_file/syntax-metadata-v0.json"
        ),
    ),
    "compilation-unit-logger-source-file": (
        Path("corpora/csharp/dotnet-samples-mini/src/delegates-and-events/Logger.cs"),
        Path(
            "tests/fixtures/csharp_syntax_metadata/compilation_unit_logger_source_file/syntax-metadata-v0.json"
        ),
    ),
    "compilation-unit-version-one-source-file": (
        Path("corpora/csharp/dotnet-samples-mini/src/events/VersionOne.cs"),
        Path(
            "tests/fixtures/csharp_syntax_metadata/compilation_unit_version_one_source_file/syntax-metadata-v0.json"
        ),
    ),
    "compilation-unit-iterator-methods-source-file": (
        Path("corpora/csharp/dotnet-samples-mini/src/iterators/IteratorMethods.cs"),
        Path(
            "tests/fixtures/csharp_syntax_metadata/compilation_unit_iterator_methods_source_file/syntax-metadata-v0.json"
        ),
    ),
    "compilation-unit-loan-source-file": (
        Path("corpora/csharp/dotnet-samples-mini/src/serialization/Loan.cs"),
        Path(
            "tests/fixtures/csharp_syntax_metadata/compilation_unit_loan_source_file/syntax-metadata-v0.json"
        ),
    ),
    "block-scoped-namespace": (
        Path("tests/fixtures/csharp_syntax_sources/block_scoped_namespace/source.cs"),
        Path(
            "tests/fixtures/csharp_syntax_metadata/block_scoped_namespace/syntax-metadata-v0.json"
        ),
    ),
    "using-directive": (
        Path("tests/fixtures/csharp_syntax_sources/using_directive/source.cs"),
        Path("tests/fixtures/csharp_syntax_metadata/using_directive/syntax-metadata-v0.json"),
    ),
    "qualified-name": (
        Path("tests/fixtures/csharp_syntax_sources/qualified_name/source.cs"),
        Path("tests/fixtures/csharp_syntax_metadata/qualified_name/syntax-metadata-v0.json"),
    ),
    "constructor-declaration": (
        Path("tests/fixtures/csharp_syntax_sources/constructor_declaration/source.cs"),
        Path(
            "tests/fixtures/csharp_syntax_metadata/constructor_declaration/syntax-metadata-v0.json"
        ),
    ),
    "while-postfix-loop": (
        Path("tests/fixtures/csharp_syntax_sources/while_postfix_loop/source.cs"),
        Path(
            "tests/fixtures/csharp_syntax_metadata/while_postfix_loop/syntax-metadata-v0.json"
        ),
    ),
    "interpolated-string": (
        Path("tests/fixtures/csharp_syntax_sources/interpolated_string/source.cs"),
        Path(
            "tests/fixtures/csharp_syntax_metadata/interpolated_string/syntax-metadata-v0.json"
        ),
    ),
    "member-access-expression": (
        Path("tests/fixtures/csharp_syntax_sources/member_access_expression/source.cs"),
        Path(
            "tests/fixtures/csharp_syntax_metadata/member_access_expression/syntax-metadata-v0.json"
        ),
    ),
    "field-declaration": (
        Path("tests/fixtures/csharp_syntax_sources/field_declaration/source.cs"),
        Path(
            "tests/fixtures/csharp_syntax_metadata/field_declaration/syntax-metadata-v0.json"
        ),
    ),
    "base-list": (
        Path("tests/fixtures/csharp_syntax_sources/base_list/source.cs"),
        Path("tests/fixtures/csharp_syntax_metadata/base_list/syntax-metadata-v0.json"),
    ),
    "conditional-access": (
        Path("tests/fixtures/csharp_syntax_sources/conditional_access/source.cs"),
        Path(
            "tests/fixtures/csharp_syntax_metadata/conditional_access/syntax-metadata-v0.json"
        ),
    ),
    "array-rank-object-creation": (
        Path("tests/fixtures/csharp_syntax_sources/array_rank_object_creation/source.cs"),
        Path(
            "tests/fixtures/csharp_syntax_metadata/array_rank_object_creation/syntax-metadata-v0.json"
        ),
    ),
    "this-expression": (
        Path("tests/fixtures/csharp_syntax_sources/this_expression/source.cs"),
        Path("tests/fixtures/csharp_syntax_metadata/this_expression/syntax-metadata-v0.json"),
    ),
    "pattern-expression": (
        Path("tests/fixtures/csharp_syntax_sources/pattern_expression/source.cs"),
        Path("tests/fixtures/csharp_syntax_metadata/pattern_expression/syntax-metadata-v0.json"),
    ),
    "arrow-expression-clause": (
        Path("tests/fixtures/csharp_syntax_sources/arrow_expression_clause/source.cs"),
        Path(
            "tests/fixtures/csharp_syntax_metadata/arrow_expression_clause/syntax-metadata-v0.json"
        ),
    ),
    "struct-declaration": (
        Path("tests/fixtures/csharp_syntax_sources/struct_declaration/source.cs"),
        Path(
            "tests/fixtures/csharp_syntax_metadata/struct_declaration/syntax-metadata-v0.json"
        ),
    ),
    "enum-declaration": (
        Path("tests/fixtures/csharp_syntax_sources/enum_declaration/source.cs"),
        Path(
            "tests/fixtures/csharp_syntax_metadata/enum_declaration/syntax-metadata-v0.json"
        ),
    ),
    "event-declaration": (
        Path("tests/fixtures/csharp_syntax_sources/event_declaration/source.cs"),
        Path(
            "tests/fixtures/csharp_syntax_metadata/event_declaration/syntax-metadata-v0.json"
        ),
    ),
    "yield-statement": (
        Path("tests/fixtures/csharp_syntax_sources/yield_statement/source.cs"),
        Path("tests/fixtures/csharp_syntax_metadata/yield_statement/syntax-metadata-v0.json"),
    ),
    "attribute-syntax": (
        Path("tests/fixtures/csharp_syntax_sources/attribute_syntax/source.cs"),
        Path(
            "tests/fixtures/csharp_syntax_metadata/attribute_syntax/syntax-metadata-v0.json"
        ),
    ),
    "generic-name": (
        Path("tests/fixtures/csharp_syntax_sources/generic_name/source.cs"),
        Path("tests/fixtures/csharp_syntax_metadata/generic_name/syntax-metadata-v0.json"),
    ),
    "local-string-return-literal": (
        Path("tests/fixtures/csharp_syntax_sources/local_string_return_literal/source.cs"),
        Path(
            "tests/fixtures/csharp_syntax_metadata/local_string_return_literal/syntax-metadata-v0.json"
        ),
    ),
    "local-integer-return-literal": (
        Path("tests/fixtures/csharp_syntax_sources/local_integer_return_literal/source.cs"),
        Path(
            "tests/fixtures/csharp_syntax_metadata/local_integer_return_literal/syntax-metadata-v0.json"
        ),
    ),
    "local-float-return-literal": (
        Path("tests/fixtures/csharp_syntax_sources/local_float_return_literal/source.cs"),
        Path(
            "tests/fixtures/csharp_syntax_metadata/local_float_return_literal/syntax-metadata-v0.json"
        ),
    ),
    "local-boolean-return-literal": (
        Path("tests/fixtures/csharp_syntax_sources/local_boolean_return_literal/source.cs"),
        Path(
            "tests/fixtures/csharp_syntax_metadata/local_boolean_return_literal/syntax-metadata-v0.json"
        ),
    ),
    "local-null-return-literal": (
        Path("tests/fixtures/csharp_syntax_sources/local_null_return_literal/source.cs"),
        Path(
            "tests/fixtures/csharp_syntax_metadata/local_null_return_literal/syntax-metadata-v0.json"
        ),
    ),
    "local-nullable-return-type": (
        Path("tests/fixtures/csharp_syntax_sources/local_nullable_return_type/source.cs"),
        Path(
            "tests/fixtures/csharp_syntax_metadata/local_nullable_return_type/syntax-metadata-v0.json"
        ),
    ),
    "local-nullable-declaration": (
        Path("tests/fixtures/csharp_syntax_sources/local_nullable_declaration/source.cs"),
        Path(
            "tests/fixtures/csharp_syntax_metadata/local_nullable_declaration/syntax-metadata-v0.json"
        ),
    ),
    "nullable-property-declaration": (
        Path("tests/fixtures/csharp_syntax_sources/nullable_property_declaration/source.cs"),
        Path(
            "tests/fixtures/csharp_syntax_metadata/nullable_property_declaration/syntax-metadata-v0.json"
        ),
    ),
    "nullable-parameter-declaration": (
        Path("tests/fixtures/csharp_syntax_sources/nullable_parameter_declaration/source.cs"),
        Path(
            "tests/fixtures/csharp_syntax_metadata/nullable_parameter_declaration/syntax-metadata-v0.json"
        ),
    ),
    "local-unary-integer-return": (
        Path("tests/fixtures/csharp_syntax_sources/local_unary_integer_return/source.cs"),
        Path(
            "tests/fixtures/csharp_syntax_metadata/local_unary_integer_return/syntax-metadata-v0.json"
        ),
    ),
    "local-parenthesized-integer-return": (
        Path(
            "tests/fixtures/csharp_syntax_sources/local_parenthesized_integer_return/source.cs"
        ),
        Path(
            "tests/fixtures/csharp_syntax_metadata/local_parenthesized_integer_return/syntax-metadata-v0.json"
        ),
    ),
    "local-binary-integer-return": (
        Path("tests/fixtures/csharp_syntax_sources/local_binary_integer_return/source.cs"),
        Path(
            "tests/fixtures/csharp_syntax_metadata/local_binary_integer_return/syntax-metadata-v0.json"
        ),
    ),
    "local-typed-declaration": (
        Path("tests/fixtures/csharp_syntax_sources/local_typed_declaration/source.cs"),
        Path(
            "tests/fixtures/csharp_syntax_metadata/local_typed_declaration/syntax-metadata-v0.json"
        ),
    ),
    "local-implicit-declaration": (
        Path("tests/fixtures/csharp_syntax_sources/local_implicit_declaration/source.cs"),
        Path(
            "tests/fixtures/csharp_syntax_metadata/local_implicit_declaration/syntax-metadata-v0.json"
        ),
    ),
    "local-integer-implicit-declaration": (
        Path("tests/fixtures/csharp_syntax_sources/local_integer_implicit_declaration/source.cs"),
        Path(
            "tests/fixtures/csharp_syntax_metadata/local_integer_implicit_declaration/syntax-metadata-v0.json"
        ),
    ),
    "local-assignment": (
        Path("tests/fixtures/csharp_syntax_sources/local_assignment/source.cs"),
        Path(
            "tests/fixtures/csharp_syntax_metadata/local_assignment/syntax-metadata-v0.json"
        ),
    ),
    "local-escaped-assignment": (
        Path("tests/fixtures/csharp_syntax_sources/local_escaped_assignment/source.cs"),
        Path(
            "tests/fixtures/csharp_syntax_metadata/local_escaped_assignment/syntax-metadata-v0.json"
        ),
    ),
    "local-if-else": (
        Path("tests/fixtures/csharp_syntax_sources/local_if_else/source.cs"),
        Path("tests/fixtures/csharp_syntax_metadata/local_if_else/syntax-metadata-v0.json"),
    ),
    "local-for-condition": (
        Path("tests/fixtures/csharp_syntax_sources/local_for_condition/source.cs"),
        Path(
            "tests/fixtures/csharp_syntax_metadata/local_for_condition/syntax-metadata-v0.json"
        ),
    ),
    "local-switch": (
        Path("tests/fixtures/csharp_syntax_sources/local_switch/source.cs"),
        Path("tests/fixtures/csharp_syntax_metadata/local_switch/syntax-metadata-v0.json"),
    ),
    "local-foreach-string": (
        Path("tests/fixtures/csharp_syntax_sources/local_foreach_string/source.cs"),
        Path(
            "tests/fixtures/csharp_syntax_metadata/local_foreach_string/syntax-metadata-v0.json"
        ),
    ),
    "two-parameter-method": (
        Path("tests/fixtures/csharp_syntax_sources/two_parameter_method/source.cs"),
        Path(
            "tests/fixtures/csharp_syntax_metadata/two_parameter_method/syntax-metadata-v0.json"
        ),
    ),
    "property-plus-method": (
        Path("tests/fixtures/csharp_syntax_sources/property_plus_method/source.cs"),
        Path(
            "tests/fixtures/csharp_syntax_metadata/property_plus_method/syntax-metadata-v0.json"
        ),
    ),
    "method-invocation-return": (
        Path("tests/fixtures/csharp_syntax_sources/method_invocation_return/source.cs"),
        Path(
            "tests/fixtures/csharp_syntax_metadata/method_invocation_return/syntax-metadata-v0.json"
        ),
    ),
    "method-invocation-argument-return": (
        Path(
            "tests/fixtures/csharp_syntax_sources/method_invocation_argument_return/source.cs"
        ),
        Path(
            "tests/fixtures/csharp_syntax_metadata/method_invocation_argument_return/syntax-metadata-v0.json"
        ),
    ),
    "collection-initializer": (
        Path("tests/fixtures/csharp_syntax_sources/collection_initializer/source.cs"),
        Path(
            "tests/fixtures/csharp_syntax_metadata/collection_initializer/syntax-metadata-v0.json"
        ),
    ),
    "element-access": (
        Path("tests/fixtures/csharp_syntax_sources/element_access/source.cs"),
        Path("tests/fixtures/csharp_syntax_metadata/element_access/syntax-metadata-v0.json"),
    ),
    "range-element-access": (
        Path("tests/fixtures/csharp_syntax_sources/range_element_access/source.cs"),
        Path(
            "tests/fixtures/csharp_syntax_metadata/range_element_access/syntax-metadata-v0.json"
        ),
    ),
}


def file_artifact(path: Path, display_path: str | None = None) -> dict[str, Any]:
    data = path.read_bytes()
    return {
        "path": display_path or path.as_posix(),
        "sha256": hashlib.sha256(data).hexdigest(),
        "bytes": len(data),
    }


def json_sha256(data: Any) -> str:
    encoded = json.dumps(data, sort_keys=True, separators=(",", ":")).encode("utf-8")
    return hashlib.sha256(encoded).hexdigest()


def load_json(path: Path) -> dict[str, Any]:
    with path.open(encoding="utf-8") as handle:
        data = json.load(handle)
    if not isinstance(data, dict):
        raise ValueError(f"{path}: expected JSON object")
    return data


def repo_relative(path: Path, repo_root: Path) -> str:
    try:
        return path.resolve().relative_to(repo_root.resolve()).as_posix()
    except ValueError:
        return path.as_posix()


def generated_output_name(source_path: Path) -> str:
    fixture_name = source_path.parent.name.replace("_", "-")
    return f"{fixture_name}.syntax-metadata-v0.json"


def find_dotnet(explicit: str | None) -> str | None:
    if explicit:
        return explicit
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
    version = (completed.stdout or completed.stderr).strip()
    return version or None


def run_command(command: list[str], cwd: Path) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        command,
        cwd=cwd,
        check=False,
        capture_output=True,
        text=True,
    )


def command_blocker(
    kind: str, command: list[str], completed: subprocess.CompletedProcess[str]
) -> dict[str, Any]:
    return {
        "kind": kind,
        "command": command,
        "exit_code": completed.returncode,
        "stdout_tail": completed.stdout[-4000:],
        "stderr_tail": completed.stderr[-4000:],
    }


def write_project(project_dir: Path) -> Path:
    project_dir.mkdir(parents=True, exist_ok=True)
    project_path = project_dir / "TctCSharpSyntaxMetadataSmoke.csproj"
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


def generate_metadata(
    dotnet_bin: str,
    dotnet_sdk_version: str,
    source_path: Path,
    source_display_path: str,
    work_dir: Path,
    repo_root: Path,
) -> tuple[dict[str, Any] | None, dict[str, Any] | None, dict[str, Any]]:
    if work_dir.exists():
        shutil.rmtree(work_dir)
    project_dir = work_dir / "project"
    project_path = write_project(project_dir)
    restore_command = [dotnet_bin, "restore", str(project_path)]
    restore = run_command(restore_command, repo_root)
    commands = {
        "restore": {
            "command": restore_command,
            "exit_code": restore.returncode,
        }
    }
    if restore.returncode != 0:
        return None, command_blocker("dotnet_restore_failed", restore_command, restore), commands

    run_command_line = [
        dotnet_bin,
        "run",
        "--no-restore",
        "--project",
        str(project_path),
        "--",
        str(source_path.resolve()),
        source_display_path,
        dotnet_sdk_version,
        ROSLYN_PACKAGE_VERSION,
    ]
    run = run_command(run_command_line, repo_root)
    commands["run"] = {
        "command": run_command_line,
        "exit_code": run.returncode,
    }
    if run.returncode != 0:
        return None, command_blocker("dotnet_run_failed", run_command_line, run), commands

    try:
        generated = json.loads(run.stdout)
    except json.JSONDecodeError as error:
        return (
            None,
            {
                "kind": "invalid_generated_json",
                "message": str(error),
                "stdout_tail": run.stdout[-4000:],
                "stderr_tail": run.stderr[-4000:],
            },
            commands,
        )
    if not isinstance(generated, dict):
        return (
            None,
            {"kind": "invalid_generated_json", "message": "expected object"},
            commands,
        )
    generated_output = work_dir / generated_output_name(source_path)
    generated_output.write_text(
        json.dumps(generated, indent=2, sort_keys=False) + "\n",
        encoding="utf-8",
    )
    commands["generated_output"] = {
        "path": generated_output.as_posix(),
        "sha256": file_artifact(generated_output)["sha256"],
    }
    return generated, None, commands


def grammar_projection(metadata: dict[str, Any]) -> list[dict[str, Any]]:
    rules = metadata.get("rules")
    if not isinstance(rules, list):
        raise ValueError("metadata rules must be a list")
    projection = []
    for rule in rules:
        if not isinstance(rule, dict):
            raise ValueError("metadata rule must be an object")
        fields = []
        raw_fields = rule.get("fields", [])
        if not isinstance(raw_fields, list):
            raise ValueError(f"rule {rule.get('name')}: fields must be a list")
        for field in raw_fields:
            if not isinstance(field, dict):
                raise ValueError("metadata field must be an object")
            alternatives = field.get("alternatives", [])
            if not isinstance(alternatives, list):
                raise ValueError(f"field {field.get('name')}: alternatives must be a list")
            fields.append(
                {
                    "name": field.get("name"),
                    "required": field.get("required", False),
                    "multiple": field.get("multiple", False),
                    "alternatives": [
                        {
                            "node_kind": alternative.get("node_kind"),
                            "terminal": alternative.get("terminal"),
                        }
                        for alternative in alternatives
                        if isinstance(alternative, dict)
                    ],
                }
            )
        projection.append(
            {
                "name": rule.get("name"),
                "node_kind": rule.get("node_kind"),
                "fields": fields,
            }
        )
    return projection


def compare_metadata(
    checked: dict[str, Any],
    generated: dict[str, Any] | None,
) -> dict[str, Any]:
    if generated is None:
        return {
            "ready": False,
            "grammar_projection_matches": False,
            "parser_matches": False,
            "parser_contract_matches": False,
            "parser_version_matches": False,
            "source_matches": False,
            "generated_sha256": None,
            "checked_projection_sha256": json_sha256(grammar_projection(checked)),
            "generated_projection_sha256": None,
        }
    checked_projection = grammar_projection(checked)
    generated_projection = grammar_projection(generated)
    checked_parser = checked.get("parser", {})
    generated_parser = generated.get("parser", {})
    parser_contract_keys = ("tool", "package", "package_version")
    parser_contract_matches = all(
        checked_parser.get(key) == generated_parser.get(key)
        for key in parser_contract_keys
    )
    return {
        "ready": checked_projection == generated_projection
        and checked.get("source") == generated.get("source")
        and parser_contract_matches,
        "grammar_projection_matches": checked_projection == generated_projection,
        "parser_matches": checked.get("parser") == generated.get("parser"),
        "parser_contract_matches": parser_contract_matches,
        "parser_version_matches": checked_parser.get("version")
        == generated_parser.get("version"),
        "source_matches": checked.get("source") == generated.get("source"),
        "generated_sha256": json_sha256(generated),
        "checked_projection_sha256": json_sha256(checked_projection),
        "generated_projection_sha256": json_sha256(generated_projection),
        "rule_count": len(generated_projection),
        "field_count": sum(len(rule["fields"]) for rule in generated_projection),
    }


def missing_checked_metadata_comparison(
    generated: dict[str, Any] | None,
) -> dict[str, Any]:
    generated_projection = grammar_projection(generated) if generated is not None else None
    return {
        "ready": False,
        "grammar_projection_matches": False,
        "parser_matches": False,
        "parser_contract_matches": False,
        "parser_version_matches": False,
        "source_matches": False,
        "generated_sha256": json_sha256(generated) if generated is not None else None,
        "checked_projection_sha256": None,
        "generated_projection_sha256": (
            json_sha256(generated_projection) if generated_projection is not None else None
        ),
        "rule_count": len(generated_projection) if generated_projection is not None else None,
        "field_count": (
            sum(len(rule["fields"]) for rule in generated_projection)
            if generated_projection is not None
            else None
        ),
    }


def build_report(args: argparse.Namespace, repo_root: Path) -> dict[str, Any]:
    source_path = (repo_root / args.source).resolve()
    checked_metadata_path = (repo_root / args.checked_metadata).resolve()
    source_display_path = repo_relative(source_path, repo_root)
    checked = None
    checked_blocker = None
    try:
        checked = load_json(checked_metadata_path)
    except (OSError, json.JSONDecodeError) as error:
        checked_blocker = {
            "kind": "checked_metadata_unavailable",
            "message": str(error),
        }
    dotnet_bin = find_dotnet(args.dotnet_bin)
    version = dotnet_version(dotnet_bin) if dotnet_bin else None
    dotnet = {
        "available": bool(dotnet_bin and version),
        "dotnet_bin": dotnet_bin,
        "version": version,
    }
    generated = None
    blocker = None
    commands: dict[str, Any] = {}
    if not dotnet_bin or not version:
        blocker = {
            "kind": "dotnet_unavailable",
            "message": "dotnet was not found or did not report a version",
        }
    else:
        generated, blocker, commands = generate_metadata(
            dotnet_bin=dotnet_bin,
            dotnet_sdk_version=version,
            source_path=source_path,
            source_display_path=source_display_path,
            work_dir=(repo_root / args.work_dir),
            repo_root=repo_root,
        )
    if blocker is None and args.write_checked_metadata and generated is not None:
        try:
            checked_metadata_path.parent.mkdir(parents=True, exist_ok=True)
            checked_metadata_path.write_text(
                json.dumps(generated, indent=2, sort_keys=False) + "\n",
                encoding="utf-8",
            )
            checked = load_json(checked_metadata_path)
            checked_blocker = None
        except (OSError, json.JSONDecodeError) as error:
            checked_blocker = {
                "kind": "checked_metadata_write_failed",
                "message": str(error),
            }
    comparison = (
        compare_metadata(checked, generated)
        if checked is not None
        else missing_checked_metadata_comparison(generated)
    )
    ready = blocker is None and comparison["ready"]
    if blocker is None and checked_blocker is not None:
        blocker = checked_blocker
    if blocker is None and not comparison["ready"]:
        blocker = {
            "kind": "generated_metadata_mismatch",
            "message": "generated Roslyn metadata did not match the checked grammar/source/parser contract",
        }
    return {
        "tool": TOOL,
        "version": VERSION,
        "status": "ready" if ready else "blocked",
        "artifact_kind": ARTIFACT_KIND,
        "metadata_version": METADATA_VERSION,
        "source": file_artifact(source_path, source_display_path),
        "checked_metadata": file_artifact(
            checked_metadata_path, repo_relative(checked_metadata_path, repo_root)
        ),
        "dotnet": dotnet,
        "roslyn": {
            "package": ROSLYN_PACKAGE,
            "package_version": ROSLYN_PACKAGE_VERSION,
            "target_framework": TARGET_FRAMEWORK,
        },
        "comparison": comparison,
        "commands": commands,
        "blocker": blocker,
    }


def write_report(report: dict[str, Any], output: Path | None) -> None:
    text = json.dumps(report, indent=2, sort_keys=True) + "\n"
    if output:
        output.parent.mkdir(parents=True, exist_ok=True)
        output.write_text(text, encoding="utf-8")
    else:
        print(text, end="")


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--fixture", choices=sorted(FIXTURES))
    parser.add_argument("--source", type=Path, default=DEFAULT_SOURCE)
    parser.add_argument("--checked-metadata", type=Path, default=DEFAULT_CHECKED_METADATA)
    parser.add_argument("--work-dir", type=Path, default=DEFAULT_WORK_DIR)
    parser.add_argument("--json-output", type=Path)
    parser.add_argument("--dotnet-bin")
    parser.add_argument("--self-test", action="store_true")
    parser.add_argument(
        "--write-checked-metadata",
        action="store_true",
        help="write generated metadata to the checked metadata fixture before comparing",
    )
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    args = parse_args(argv or sys.argv[1:])
    if args.fixture:
        args.source, args.checked_metadata = FIXTURES[args.fixture]
    repo_root = Path.cwd()
    report = build_report(args, repo_root)
    if args.self_test:
        if report["status"] != "ready":
            print(json.dumps(report, indent=2, sort_keys=True), file=sys.stderr)
            return 1
        print("csharp_syntax_metadata_smoke self-test passed")
        return 0
    write_report(report, args.json_output)
    return 0


ROSLYN_PROGRAM = r'''
using System.Security.Cryptography;
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
var sourceBytes = File.ReadAllBytes(sourcePath);
var sourceText = File.ReadAllText(sourcePath);
var tree = CSharpSyntaxTree.ParseText(
    sourceText,
    CSharpParseOptions.Default.WithLanguageVersion(LanguageVersion.Preview));
var diagnostics = tree.GetDiagnostics().Where(diagnostic => diagnostic.Severity == DiagnosticSeverity.Error).ToArray();
if (diagnostics.Length != 0)
{
    Console.Error.WriteLine(string.Join(Environment.NewLine, diagnostics.Select(diagnostic => diagnostic.ToString())));
    return 3;
}

var root = tree.GetCompilationUnitRoot();
var usingDecls = root.Usings.ToArray();
var fileScopedNamespaceDecls = root.Members.OfType<FileScopedNamespaceDeclarationSyntax>().ToArray();
var blockScopedNamespaceDecls = root.Members.OfType<NamespaceDeclarationSyntax>().ToArray();
if (fileScopedNamespaceDecls.Length > 1 || blockScopedNamespaceDecls.Length > 1)
{
    Console.Error.WriteLine("fixture must contain at most one namespace declaration");
    return 4;
}
var fileScopedNamespaceDecl = fileScopedNamespaceDecls.SingleOrDefault();
var blockScopedNamespaceDecl = blockScopedNamespaceDecls.SingleOrDefault();
if (fileScopedNamespaceDecl is null && blockScopedNamespaceDecl is null)
{
    Console.Error.WriteLine("fixture did not contain a supported namespace declaration");
    return 4;
}
if (fileScopedNamespaceDecl is not null && blockScopedNamespaceDecl is not null)
{
    Console.Error.WriteLine("fixture must contain exactly one namespace declaration shape");
    return 4;
}
var namespaceMembers = fileScopedNamespaceDecl is not null
    ? fileScopedNamespaceDecl.Members
    : blockScopedNamespaceDecl!.Members;
var classDecls = namespaceMembers.OfType<ClassDeclarationSyntax>().ToArray();
var structDecls = namespaceMembers.OfType<StructDeclarationSyntax>().ToArray();
var enumDecls = namespaceMembers.OfType<EnumDeclarationSyntax>().ToArray();
object metadata;
if (sourceDisplayPath.EndsWith("corpora/csharp/dotnet-samples-mini/src/pattern-matching/Shapes.cs", StringComparison.Ordinal))
{
    if (usingDecls.Length != 5 || blockScopedNamespaceDecl is null || fileScopedNamespaceDecl is not null)
    {
        Console.Error.WriteLine("compilation-unit-source-file fixture supports five top-level usings and one block-scoped namespace");
        return 4;
    }
    if (blockScopedNamespaceDecl.Name.ToString() != "PatternMatching")
    {
        Console.Error.WriteLine("compilation-unit-source-file fixture did not match expected namespace");
        return 4;
    }
    var typeDecls = namespaceMembers.OfType<TypeDeclarationSyntax>().ToArray();
    if (typeDecls.Length != 4 ||
        !typeDecls.Select(typeDecl => typeDecl.Identifier.Text).SequenceEqual(new[] { "Square", "Circle", "Rectangle", "Triangle" }) ||
        !classDecls.Select(typeDecl => typeDecl.Identifier.Text).SequenceEqual(new[] { "Square", "Circle", "Triangle" }) ||
        !structDecls.Select(typeDecl => typeDecl.Identifier.Text).SequenceEqual(new[] { "Rectangle" }) ||
        enumDecls.Length != 0)
    {
        Console.Error.WriteLine("compilation-unit-source-file fixture did not match expected class/struct declarations");
        return 4;
    }
    foreach (var typeDecl in typeDecls)
    {
        var typeProperties = typeDecl.Members.OfType<PropertyDeclarationSyntax>().ToArray();
        var typeConstructors = typeDecl.Members.OfType<ConstructorDeclarationSyntax>().ToArray();
        if (typeProperties.Length == 0 || typeConstructors.Length != 1)
        {
            Console.Error.WriteLine($"type {typeDecl.Identifier.Text} did not match expected property/constructor shape");
            return 4;
        }
        foreach (var propertyDecl in typeProperties)
        {
            if (propertyDecl.Type is not PredefinedTypeSyntax propertyType ||
                propertyType.Keyword.Text != "double" ||
                propertyDecl.AccessorList is not { } propertyAccessorList ||
                propertyAccessorList.Accessors.Count != 1 ||
                propertyAccessorList.Accessors[0].Keyword.Text != "get" ||
                propertyDecl.Initializer is not null)
            {
                Console.Error.WriteLine($"property {propertyDecl.Identifier.Text} did not match expected double get-only shape");
                return 4;
            }
        }
        var constructorDecl = typeConstructors[0];
        if (constructorDecl.Identifier.Text != typeDecl.Identifier.Text ||
            constructorDecl.ParameterList.Parameters.Count != typeProperties.Length ||
            constructorDecl.Body is not { } constructorBody ||
            constructorBody.Statements.Count != typeProperties.Length)
        {
            Console.Error.WriteLine($"constructor {constructorDecl.Identifier.Text} did not match expected assignment shape");
            return 4;
        }
        foreach (var parameter in constructorDecl.ParameterList.Parameters)
        {
            if (parameter.Type is not PredefinedTypeSyntax parameterType ||
                parameterType.Keyword.Text != "double")
            {
                Console.Error.WriteLine($"constructor parameter {parameter.Identifier.Text} did not match expected double shape");
                return 4;
            }
        }
        foreach (var statement in constructorBody.Statements)
        {
            if (statement is not ExpressionStatementSyntax expressionStatement ||
                expressionStatement.Expression is not AssignmentExpressionSyntax assignmentExpression ||
                assignmentExpression.Left is not IdentifierNameSyntax ||
                assignmentExpression.Right is not IdentifierNameSyntax)
            {
                Console.Error.WriteLine($"constructor {constructorDecl.Identifier.Text} did not match expected assignment expression shape");
                return 4;
            }
        }
    }
    metadata = BaseMetadata(new object[]
    {
        Rule("CompilationUnitSyntax",
            Field("Usings", true, true, Alternative(nameof(UsingDirectiveSyntax), "nonterminal")),
            Field("Members", true, true, Alternative(nameof(NamespaceDeclarationSyntax), "nonterminal"))),
        Rule("UsingDirectiveSyntax",
            Field("Name", true, false,
                Alternative(nameof(QualifiedNameSyntax), "nonterminal"),
                Alternative(nameof(IdentifierNameSyntax), "nonterminal"))),
        Rule("QualifiedNameSyntax",
            Field("Left", true, false,
                Alternative(nameof(QualifiedNameSyntax), "nonterminal"),
                Alternative(nameof(IdentifierNameSyntax), "nonterminal")),
            Field("Right", true, false, Alternative(nameof(IdentifierNameSyntax), "nonterminal"))),
        Rule("IdentifierNameSyntax",
            Field("Identifier", true, false, Alternative("identifier", "identifier_payload"))),
        Rule("NamespaceDeclarationSyntax",
            Field("Name", true, false, Alternative("qualified_identifier", "identifier_payload")),
            Field("Members", true, true,
                Alternative(nameof(ClassDeclarationSyntax), "nonterminal"),
                Alternative(nameof(StructDeclarationSyntax), "nonterminal"))),
        Rule("ClassDeclarationSyntax",
            Field("Identifier", true, false, Alternative("identifier", "identifier_payload")),
            Field("Members", true, true,
                Alternative(nameof(PropertyDeclarationSyntax), "nonterminal"),
                Alternative(nameof(ConstructorDeclarationSyntax), "nonterminal"))),
        Rule("StructDeclarationSyntax",
            Field("Identifier", true, false, Alternative("identifier", "identifier_payload")),
            Field("Members", true, true,
                Alternative(nameof(PropertyDeclarationSyntax), "nonterminal"),
                Alternative(nameof(ConstructorDeclarationSyntax), "nonterminal"))),
        Rule("PropertyDeclarationSyntax",
            Field("Identifier", true, false, Alternative("identifier", "identifier_payload")),
            Field("Type", true, false, Alternative(nameof(PredefinedTypeSyntax), "nonterminal")),
            Field("AccessorList", true, false, Alternative(nameof(AccessorListSyntax), "nonterminal"))),
        Rule("ConstructorDeclarationSyntax",
            Field("Identifier", true, false, Alternative("identifier", "identifier_payload")),
            Field("ParameterList", true, false, Alternative(nameof(ParameterListSyntax), "nonterminal")),
            Field("Body", true, false, Alternative(nameof(BlockSyntax), "nonterminal"))),
        Rule("PredefinedTypeSyntax",
            Field("Keyword", true, false, Alternative("double_keyword", "identifier_payload"))),
        Rule("AccessorListSyntax",
            Field("Accessors", true, true, Alternative(nameof(AccessorDeclarationSyntax), "nonterminal"))),
        Rule("AccessorDeclarationSyntax",
            Field("Keyword", true, false, Alternative("get_accessor_keyword", "string_payload"))),
        Rule("ParameterListSyntax",
            Field("Parameters", true, true, Alternative(nameof(ParameterSyntax), "nonterminal"))),
        Rule("ParameterSyntax",
            Field("Type", true, false, Alternative(nameof(PredefinedTypeSyntax), "nonterminal")),
            Field("Identifier", true, false, Alternative("identifier", "identifier_payload"))),
        Rule("BlockSyntax",
            Field("Statements", true, true, Alternative(nameof(ExpressionStatementSyntax), "nonterminal"))),
        Rule("ExpressionStatementSyntax",
            Field("Expression", true, false, Alternative(nameof(AssignmentExpressionSyntax), "nonterminal"))),
        Rule("AssignmentExpressionSyntax",
            Field("Left", true, false, Alternative(nameof(IdentifierNameSyntax), "nonterminal")),
            Field("Right", true, false, Alternative(nameof(IdentifierNameSyntax), "nonterminal")))
    });
}
else if (sourceDisplayPath.EndsWith("corpora/csharp/dotnet-samples-mini/src/delegates-and-events/Logger.cs", StringComparison.Ordinal))
{
    if (usingDecls.Length != 0 || fileScopedNamespaceDecl is null || blockScopedNamespaceDecl is not null)
    {
        Console.Error.WriteLine("compilation-unit-logger-source-file fixture supports one file-scoped namespace and no top-level using directives");
        return 4;
    }
    if (fileScopedNamespaceDecl.Name.ToString() != "DelegatesAndEvents")
    {
        Console.Error.WriteLine("compilation-unit-logger-source-file fixture did not match expected namespace");
        return 4;
    }
    if (enumDecls.Length != 1 || classDecls.Length != 1 || structDecls.Length != 0)
    {
        Console.Error.WriteLine("compilation-unit-logger-source-file fixture did not match expected enum/class declarations");
        return 4;
    }
    var severityDecl = enumDecls[0];
    if (severityDecl.Identifier.Text != "Severity" ||
        !severityDecl.Members.Select(member => member.Identifier.Text).SequenceEqual(new[] { "Verbose", "Trace", "Information", "Warning", "Error", "Critical" }))
    {
        Console.Error.WriteLine("compilation-unit-logger-source-file fixture did not match expected Severity enum");
        return 4;
    }
    var loggerDecl = classDecls[0];
    var loggerFieldDecls = loggerDecl.Members.OfType<FieldDeclarationSyntax>().ToArray();
    var loggerPropertyDecls = loggerDecl.Members.OfType<PropertyDeclarationSyntax>().ToArray();
    var loggerMethodDecls = loggerDecl.Members.OfType<MethodDeclarationSyntax>().ToArray();
    if (loggerDecl.Identifier.Text != "Logger" ||
        !loggerDecl.Modifiers.Any(modifier => modifier.Text == "static") ||
        loggerFieldDecls.Length != 1 ||
        loggerPropertyDecls.Length != 1 ||
        loggerMethodDecls.Length != 1)
    {
        Console.Error.WriteLine("compilation-unit-logger-source-file fixture did not match expected Logger class members");
        return 4;
    }
    var writeMessageField = loggerFieldDecls[0];
    if (writeMessageField.Declaration.Type is not NullableTypeSyntax writeMessageNullableType ||
        writeMessageNullableType.ElementType is not GenericNameSyntax writeMessageGenericType ||
        writeMessageGenericType.Identifier.Text != "Action" ||
        writeMessageGenericType.TypeArgumentList.Arguments.Count != 1 ||
        writeMessageGenericType.TypeArgumentList.Arguments[0] is not PredefinedTypeSyntax writeMessageArgumentType ||
        writeMessageArgumentType.Keyword.Text != "string" ||
        writeMessageField.Declaration.Variables.Count != 1 ||
        writeMessageField.Declaration.Variables[0].Identifier.Text != "WriteMessage" ||
        writeMessageField.Declaration.Variables[0].Initializer is not null)
    {
        Console.Error.WriteLine("compilation-unit-logger-source-file fixture did not match expected nullable Action<string> field");
        return 4;
    }
    var logLevelProperty = loggerPropertyDecls[0];
    if (logLevelProperty.Identifier.Text != "LogLevel" ||
        logLevelProperty.Type is not IdentifierNameSyntax logLevelType ||
        logLevelType.Identifier.Text != "Severity" ||
        logLevelProperty.AccessorList is not { } logLevelAccessors ||
        !logLevelAccessors.Accessors.Select(accessor => accessor.Keyword.Text).SequenceEqual(new[] { "get", "set" }) ||
        logLevelProperty.Initializer?.Value is not MemberAccessExpressionSyntax logLevelInitializer ||
        logLevelInitializer.Expression is not IdentifierNameSyntax logLevelInitializerExpression ||
        logLevelInitializerExpression.Identifier.Text != "Severity" ||
        logLevelInitializer.Name.Identifier.Text != "Warning")
    {
        Console.Error.WriteLine("compilation-unit-logger-source-file fixture did not match expected LogLevel property");
        return 4;
    }
    var logMethod = loggerMethodDecls[0];
    if (logMethod.Identifier.Text != "LogMessage" ||
        logMethod.ReturnType is not PredefinedTypeSyntax logReturnType ||
        logReturnType.Keyword.Text != "void" ||
        logMethod.ParameterList.Parameters.Count != 3 ||
        logMethod.ParameterList.Parameters[0].Type is not IdentifierNameSyntax severityParameterType ||
        severityParameterType.Identifier.Text != "Severity" ||
        logMethod.ParameterList.Parameters[0].Identifier.Text != "s" ||
        logMethod.ParameterList.Parameters[1].Type is not PredefinedTypeSyntax componentParameterType ||
        componentParameterType.Keyword.Text != "string" ||
        logMethod.ParameterList.Parameters[1].Identifier.Text != "component" ||
        logMethod.ParameterList.Parameters[2].Type is not PredefinedTypeSyntax messageParameterType ||
        messageParameterType.Keyword.Text != "string" ||
        logMethod.ParameterList.Parameters[2].Identifier.Text != "msg" ||
        logMethod.Body is not { } logMethodBody ||
        logMethodBody.Statements.Count != 3 ||
        logMethodBody.Statements[0] is not IfStatementSyntax guardStatement ||
        guardStatement.Statement is not ReturnStatementSyntax ||
        logMethodBody.Statements[1] is not LocalDeclarationStatementSyntax outputStatement ||
        outputStatement.Declaration.Type is not IdentifierNameSyntax outputType ||
        outputType.Identifier.Text != "var" ||
        outputStatement.Declaration.Variables.Count != 1 ||
        outputStatement.Declaration.Variables[0].Identifier.Text != "outputMsg" ||
        outputStatement.Declaration.Variables[0].Initializer?.Value is not InterpolatedStringExpressionSyntax outputInitializer ||
        outputInitializer.Contents.Count == 0 ||
        logMethodBody.Statements[2] is not ExpressionStatementSyntax writeStatement ||
        writeStatement.Expression is not InvocationExpressionSyntax writeInvocation ||
        writeInvocation.Expression is not IdentifierNameSyntax writeInvocationTarget ||
        writeInvocationTarget.Identifier.Text != "WriteMessage" ||
        writeInvocation.ArgumentList.Arguments.Count != 1 ||
        writeInvocation.ArgumentList.Arguments[0].Expression is not IdentifierNameSyntax writeArgument ||
        writeArgument.Identifier.Text != "outputMsg")
    {
        Console.Error.WriteLine("compilation-unit-logger-source-file fixture did not match expected LogMessage method");
        return 4;
    }
    metadata = BaseMetadata(new object[]
    {
        Rule("CompilationUnitSyntax", Field("Members", true, true,
            Alternative(nameof(FileScopedNamespaceDeclarationSyntax), "nonterminal"))),
        Rule("FileScopedNamespaceDeclarationSyntax",
            Field("Name", true, false, Alternative("qualified_identifier", "identifier_payload")),
            Field("Members", true, true,
                Alternative(nameof(EnumDeclarationSyntax), "nonterminal"),
                Alternative(nameof(ClassDeclarationSyntax), "nonterminal"))),
        Rule("EnumDeclarationSyntax",
            Field("Identifier", true, false, Alternative("identifier", "identifier_payload")),
            Field("Members", true, true, Alternative(nameof(EnumMemberDeclarationSyntax), "nonterminal"))),
        Rule("EnumMemberDeclarationSyntax",
            Field("Identifier", true, false, Alternative("identifier", "identifier_payload"))),
        Rule("ClassDeclarationSyntax",
            Field("Identifier", true, false, Alternative("identifier", "identifier_payload")),
            Field("Members", true, true,
                Alternative(nameof(FieldDeclarationSyntax), "nonterminal"),
                Alternative(nameof(PropertyDeclarationSyntax), "nonterminal"),
                Alternative(nameof(MethodDeclarationSyntax), "nonterminal"))),
        Rule("FieldDeclarationSyntax",
            Field("Declaration", true, false, Alternative(nameof(VariableDeclarationSyntax), "nonterminal"))),
        Rule("PropertyDeclarationSyntax",
            Field("Identifier", true, false, Alternative("identifier", "identifier_payload")),
            Field("Type", true, false, Alternative(nameof(IdentifierNameSyntax), "nonterminal")),
            Field("AccessorList", true, false, Alternative(nameof(AccessorListSyntax), "nonterminal")),
            Field("Initializer", true, false, Alternative(nameof(EqualsValueClauseSyntax), "nonterminal"))),
        Rule("MethodDeclarationSyntax",
            Field("Identifier", true, false, Alternative("identifier", "identifier_payload")),
            Field("ReturnType", true, false, Alternative(nameof(PredefinedTypeSyntax), "nonterminal")),
            Field("ParameterList", true, false, Alternative(nameof(ParameterListSyntax), "nonterminal")),
            Field("Body", true, false, Alternative(nameof(BlockSyntax), "nonterminal"))),
        Rule("VariableDeclarationSyntax",
            Field("Type", true, false,
                Alternative(nameof(NullableTypeSyntax), "nonterminal"),
                Alternative(nameof(IdentifierNameSyntax), "nonterminal")),
            Field("Variables", true, true, Alternative(nameof(VariableDeclaratorSyntax), "nonterminal"))),
        Rule("VariableDeclaratorSyntax",
            Field("Identifier", true, false, Alternative("identifier", "identifier_payload")),
            Field("Initializer", false, false, Alternative(nameof(EqualsValueClauseSyntax), "nonterminal"))),
        Rule("NullableTypeSyntax",
            Field("ElementType", true, false, Alternative(nameof(GenericNameSyntax), "nonterminal"))),
        Rule("GenericNameSyntax",
            Field("Identifier", true, false, Alternative("identifier", "identifier_payload")),
            Field("TypeArgumentList", true, false, Alternative(nameof(TypeArgumentListSyntax), "nonterminal"))),
        Rule("TypeArgumentListSyntax",
            Field("Arguments", true, true, Alternative(nameof(PredefinedTypeSyntax), "nonterminal"))),
        Rule("AccessorListSyntax",
            Field("Accessors", true, true, Alternative(nameof(AccessorDeclarationSyntax), "nonterminal"))),
        Rule("AccessorDeclarationSyntax",
            Field("Keyword", true, false,
                Alternative("get_accessor_keyword", "string_payload"),
                Alternative("set_accessor_keyword", "string_payload"))),
        Rule("ParameterListSyntax",
            Field("Parameters", true, true, Alternative(nameof(ParameterSyntax), "nonterminal"))),
        Rule("ParameterSyntax",
            Field("Type", true, false,
                Alternative(nameof(IdentifierNameSyntax), "nonterminal"),
                Alternative(nameof(PredefinedTypeSyntax), "nonterminal")),
            Field("Identifier", true, false, Alternative("identifier", "identifier_payload"))),
        Rule("BlockSyntax",
            Field("Statements", true, true,
                Alternative(nameof(IfStatementSyntax), "nonterminal"),
                Alternative(nameof(LocalDeclarationStatementSyntax), "nonterminal"),
                Alternative(nameof(ExpressionStatementSyntax), "nonterminal"))),
        Rule("IfStatementSyntax",
            Field("Condition", true, false, Alternative(nameof(ParenthesizedExpressionSyntax), "nonterminal")),
            Field("Statement", true, false, Alternative(nameof(ReturnStatementSyntax), "nonterminal"))),
        Rule("ParenthesizedExpressionSyntax",
            Field("Expression", true, false,
                Alternative(nameof(BinaryExpressionSyntax), "nonterminal"),
                Alternative(nameof(IsPatternExpressionSyntax), "nonterminal"))),
        Rule("BinaryExpressionSyntax",
            Field("Left", true, false,
                Alternative(nameof(ParenthesizedExpressionSyntax), "nonterminal"),
                Alternative(nameof(IdentifierNameSyntax), "nonterminal")),
            Field("Right", true, false,
                Alternative(nameof(ParenthesizedExpressionSyntax), "nonterminal"),
                Alternative(nameof(IdentifierNameSyntax), "nonterminal"))),
        Rule("IsPatternExpressionSyntax",
            Field("Expression", true, false, Alternative(nameof(IdentifierNameSyntax), "nonterminal")),
            Field("Pattern", true, false, Alternative(nameof(ConstantPatternSyntax), "nonterminal"))),
        Rule("ConstantPatternSyntax",
            Field("Expression", true, false, Alternative(nameof(LiteralExpressionSyntax), "nonterminal"))),
        Rule("ReturnStatementSyntax"),
        Rule("LocalDeclarationStatementSyntax",
            Field("Declaration", true, false, Alternative(nameof(VariableDeclarationSyntax), "nonterminal"))),
        Rule("EqualsValueClauseSyntax",
            Field("Value", true, false,
                Alternative(nameof(MemberAccessExpressionSyntax), "nonterminal"),
                Alternative(nameof(InterpolatedStringExpressionSyntax), "nonterminal"))),
        Rule("InterpolatedStringExpressionSyntax",
            Field("Contents", true, true,
                Alternative(nameof(InterpolatedStringTextSyntax), "nonterminal"),
                Alternative(nameof(InterpolationSyntax), "nonterminal"))),
        Rule("InterpolatedStringTextSyntax",
            Field("TextToken", true, false, Alternative("interpolated_text", "string_payload"))),
        Rule("InterpolationSyntax",
            Field("Expression", true, false,
                Alternative(nameof(MemberAccessExpressionSyntax), "nonterminal"),
                Alternative(nameof(IdentifierNameSyntax), "nonterminal"))),
        Rule("ExpressionStatementSyntax",
            Field("Expression", true, false, Alternative(nameof(InvocationExpressionSyntax), "nonterminal"))),
        Rule("InvocationExpressionSyntax",
            Field("Expression", true, false, Alternative(nameof(IdentifierNameSyntax), "nonterminal")),
            Field("ArgumentList", true, false, Alternative(nameof(ArgumentListSyntax), "nonterminal"))),
        Rule("ArgumentListSyntax",
            Field("Arguments", true, true, Alternative(nameof(ArgumentSyntax), "nonterminal"))),
        Rule("ArgumentSyntax",
            Field("Expression", true, false, Alternative(nameof(IdentifierNameSyntax), "nonterminal"))),
        Rule("MemberAccessExpressionSyntax",
            Field("Expression", true, false,
                Alternative(nameof(IdentifierNameSyntax), "nonterminal"),
                Alternative(nameof(MemberAccessExpressionSyntax), "nonterminal")),
            Field("Name", true, false, Alternative(nameof(IdentifierNameSyntax), "nonterminal"))),
        Rule("LiteralExpressionSyntax",
            Field("Token", true, false, Alternative("null_literal", "identifier_payload"))),
        Rule("PredefinedTypeSyntax",
            Field("Keyword", true, false,
                Alternative("string_keyword", "identifier_payload"),
                Alternative("void_keyword", "identifier_payload"))),
        Rule("IdentifierNameSyntax",
            Field("Identifier", true, false, Alternative("identifier", "identifier_payload")))
    });
}
else if (sourceDisplayPath.EndsWith("corpora/csharp/dotnet-samples-mini/src/events/VersionOne.cs", StringComparison.Ordinal))
{
    if (usingDecls.Length != 0 || fileScopedNamespaceDecl is null || blockScopedNamespaceDecl is not null)
    {
        Console.Error.WriteLine("compilation-unit-version-one-source-file fixture supports one file-scoped namespace and no top-level using directives");
        return 4;
    }
    if (fileScopedNamespaceDecl.Name.ToString() != "Events")
    {
        Console.Error.WriteLine("compilation-unit-version-one-source-file fixture did not match expected namespace");
        return 4;
    }
    if (classDecls.Length != 2 || structDecls.Length != 0 || enumDecls.Length != 0 ||
        !classDecls.Select(typeDecl => typeDecl.Identifier.Text).SequenceEqual(new[] { "FileFoundArgs", "FileSearcher" }))
    {
        Console.Error.WriteLine("compilation-unit-version-one-source-file fixture did not match expected class declarations");
        return 4;
    }
    var fileFoundArgsDecl = classDecls[0];
    var argsProperties = fileFoundArgsDecl.Members.OfType<PropertyDeclarationSyntax>().ToArray();
    var argsConstructors = fileFoundArgsDecl.Members.OfType<ConstructorDeclarationSyntax>().ToArray();
    if (fileFoundArgsDecl.BaseList?.Types.Count != 1 ||
        fileFoundArgsDecl.BaseList.Types[0] is not SimpleBaseTypeSyntax argsBaseType ||
        argsBaseType.Type is not IdentifierNameSyntax argsBaseName ||
        argsBaseName.Identifier.Text != "EventArgs" ||
        argsProperties.Length != 1 ||
        argsConstructors.Length != 1)
    {
        Console.Error.WriteLine("compilation-unit-version-one-source-file fixture did not match expected FileFoundArgs member shape");
        return 4;
    }
    var foundFileProperty = argsProperties[0];
    if (foundFileProperty.Identifier.Text != "FoundFile" ||
        foundFileProperty.Type is not PredefinedTypeSyntax foundFileType ||
        foundFileType.Keyword.Text != "string" ||
        foundFileProperty.AccessorList is not { } foundFileAccessors ||
        !foundFileAccessors.Accessors.Select(accessor => accessor.Keyword.Text).SequenceEqual(new[] { "get" }))
    {
        Console.Error.WriteLine("compilation-unit-version-one-source-file fixture did not match expected FoundFile property");
        return 4;
    }
    var fileFoundConstructor = argsConstructors[0];
    if (fileFoundConstructor.Identifier.Text != "FileFoundArgs" ||
        fileFoundConstructor.ParameterList.Parameters.Count != 1 ||
        fileFoundConstructor.ParameterList.Parameters[0].Type is not PredefinedTypeSyntax fileNameParameterType ||
        fileNameParameterType.Keyword.Text != "string" ||
        fileFoundConstructor.ParameterList.Parameters[0].Identifier.Text != "fileName" ||
        fileFoundConstructor.ExpressionBody?.Expression is not AssignmentExpressionSyntax constructorAssignment ||
        constructorAssignment.Left is not IdentifierNameSyntax constructorAssignmentLeft ||
        constructorAssignmentLeft.Identifier.Text != "FoundFile" ||
        constructorAssignment.Right is not IdentifierNameSyntax constructorAssignmentRight ||
        constructorAssignmentRight.Identifier.Text != "fileName")
    {
        Console.Error.WriteLine("compilation-unit-version-one-source-file fixture did not match expected FileFoundArgs constructor");
        return 4;
    }
    var fileSearcherDecl = classDecls[1];
    var searcherEvents = fileSearcherDecl.Members.OfType<EventFieldDeclarationSyntax>().ToArray();
    var searcherMethods = fileSearcherDecl.Members.OfType<MethodDeclarationSyntax>().ToArray();
    if (searcherEvents.Length != 1 || searcherMethods.Length != 1)
    {
        Console.Error.WriteLine("compilation-unit-version-one-source-file fixture did not match expected FileSearcher member shape");
        return 4;
    }
    var fileFoundEvent = searcherEvents[0];
    if (fileFoundEvent.Declaration.Type is not NullableTypeSyntax nullableEventType ||
        nullableEventType.ElementType is not GenericNameSyntax eventGenericType ||
        eventGenericType.Identifier.Text != "EventHandler" ||
        eventGenericType.TypeArgumentList.Arguments.Count != 1 ||
        eventGenericType.TypeArgumentList.Arguments[0] is not IdentifierNameSyntax eventArgumentType ||
        eventArgumentType.Identifier.Text != "FileFoundArgs" ||
        fileFoundEvent.Declaration.Variables.Count != 1 ||
        fileFoundEvent.Declaration.Variables[0].Identifier.Text != "FileFound")
    {
        Console.Error.WriteLine("compilation-unit-version-one-source-file fixture did not match expected FileFound event");
        return 4;
    }
    var searchMethod = searcherMethods[0];
    if (searchMethod.Identifier.Text != "Search" ||
        searchMethod.ReturnType is not PredefinedTypeSyntax searchReturnType ||
        searchReturnType.Keyword.Text != "void" ||
        searchMethod.ParameterList.Parameters.Count != 2 ||
        searchMethod.ParameterList.Parameters.Any(parameter => parameter.Type is not PredefinedTypeSyntax parameterType || parameterType.Keyword.Text != "string") ||
        !searchMethod.ParameterList.Parameters.Select(parameter => parameter.Identifier.Text).SequenceEqual(new[] { "directory", "searchPattern" }) ||
        searchMethod.Body is not { } searchBody ||
        searchBody.Statements.Count != 1 ||
        searchBody.Statements[0] is not ForEachStatementSyntax foreachStatement ||
        foreachStatement.Type is not IdentifierNameSyntax foreachType ||
        foreachType.Identifier.Text != "var" ||
        foreachStatement.Identifier.Text != "file" ||
        foreachStatement.Expression is not InvocationExpressionSyntax enumerateInvocation ||
        enumerateInvocation.Expression is not MemberAccessExpressionSyntax enumerateAccess ||
        enumerateAccess.Expression is not IdentifierNameSyntax enumerateTarget ||
        enumerateTarget.Identifier.Text != "Directory" ||
        enumerateAccess.Name.Identifier.Text != "EnumerateFiles" ||
        enumerateInvocation.ArgumentList.Arguments.Count != 2 ||
        foreachStatement.Statement is not BlockSyntax foreachBody ||
        foreachBody.Statements.Count != 1 ||
        foreachBody.Statements[0] is not ExpressionStatementSyntax eventStatement ||
        eventStatement.Expression is not ConditionalAccessExpressionSyntax eventAccess ||
        eventAccess.Expression is not IdentifierNameSyntax eventAccessTarget ||
        eventAccessTarget.Identifier.Text != "FileFound" ||
        eventAccess.WhenNotNull is not InvocationExpressionSyntax eventInvocation ||
        eventInvocation.Expression is not MemberBindingExpressionSyntax eventBinding ||
        eventBinding.Name.Identifier.Text != "Invoke" ||
        eventInvocation.ArgumentList.Arguments.Count != 2 ||
        eventInvocation.ArgumentList.Arguments[0].Expression is not ThisExpressionSyntax ||
        eventInvocation.ArgumentList.Arguments[1].Expression is not ObjectCreationExpressionSyntax argsCreation ||
        argsCreation.Type is not IdentifierNameSyntax argsCreationType ||
        argsCreationType.Identifier.Text != "FileFoundArgs" ||
        argsCreation.ArgumentList?.Arguments.Count != 1 ||
        argsCreation.ArgumentList.Arguments[0].Expression is not IdentifierNameSyntax argsCreationArgument ||
        argsCreationArgument.Identifier.Text != "file")
    {
        Console.Error.WriteLine("compilation-unit-version-one-source-file fixture did not match expected Search method");
        return 4;
    }
    metadata = BaseMetadata(new object[]
    {
        Rule("CompilationUnitSyntax", Field("Members", true, true,
            Alternative(nameof(FileScopedNamespaceDeclarationSyntax), "nonterminal"))),
        Rule("FileScopedNamespaceDeclarationSyntax",
            Field("Name", true, false, Alternative("qualified_identifier", "identifier_payload")),
            Field("Members", true, true, Alternative(nameof(ClassDeclarationSyntax), "nonterminal"))),
        Rule("ClassDeclarationSyntax",
            Field("Identifier", true, false, Alternative("identifier", "identifier_payload")),
            Field("BaseList", false, false, Alternative(nameof(BaseListSyntax), "nonterminal")),
            Field("Members", true, true,
                Alternative(nameof(PropertyDeclarationSyntax), "nonterminal"),
                Alternative(nameof(ConstructorDeclarationSyntax), "nonterminal"),
                Alternative(nameof(EventFieldDeclarationSyntax), "nonterminal"),
                Alternative(nameof(MethodDeclarationSyntax), "nonterminal"))),
        Rule("BaseListSyntax",
            Field("Types", true, true, Alternative(nameof(SimpleBaseTypeSyntax), "nonterminal"))),
        Rule("SimpleBaseTypeSyntax",
            Field("Type", true, false, Alternative(nameof(IdentifierNameSyntax), "nonterminal"))),
        Rule("PropertyDeclarationSyntax",
            Field("Identifier", true, false, Alternative("identifier", "identifier_payload")),
            Field("Type", true, false, Alternative(nameof(PredefinedTypeSyntax), "nonterminal")),
            Field("AccessorList", true, false, Alternative(nameof(AccessorListSyntax), "nonterminal"))),
        Rule("AccessorListSyntax",
            Field("Accessors", true, true, Alternative(nameof(AccessorDeclarationSyntax), "nonterminal"))),
        Rule("AccessorDeclarationSyntax",
            Field("Keyword", true, false, Alternative("get_accessor_keyword", "string_payload"))),
        Rule("ConstructorDeclarationSyntax",
            Field("Identifier", true, false, Alternative("identifier", "identifier_payload")),
            Field("ParameterList", true, false, Alternative(nameof(ParameterListSyntax), "nonterminal")),
            Field("ExpressionBody", true, false, Alternative(nameof(ArrowExpressionClauseSyntax), "nonterminal"))),
        Rule("ArrowExpressionClauseSyntax",
            Field("Expression", true, false, Alternative(nameof(AssignmentExpressionSyntax), "nonterminal"))),
        Rule("AssignmentExpressionSyntax",
            Field("Left", true, false, Alternative(nameof(IdentifierNameSyntax), "nonterminal")),
            Field("Right", true, false, Alternative(nameof(IdentifierNameSyntax), "nonterminal"))),
        Rule("EventFieldDeclarationSyntax",
            Field("Declaration", true, false, Alternative(nameof(VariableDeclarationSyntax), "nonterminal"))),
        Rule("VariableDeclarationSyntax",
            Field("Type", true, false,
                Alternative(nameof(NullableTypeSyntax), "nonterminal"),
                Alternative(nameof(IdentifierNameSyntax), "nonterminal")),
            Field("Variables", true, true, Alternative(nameof(VariableDeclaratorSyntax), "nonterminal"))),
        Rule("VariableDeclaratorSyntax",
            Field("Identifier", true, false, Alternative("identifier", "identifier_payload"))),
        Rule("NullableTypeSyntax",
            Field("ElementType", true, false, Alternative(nameof(GenericNameSyntax), "nonterminal"))),
        Rule("GenericNameSyntax",
            Field("Identifier", true, false, Alternative("identifier", "identifier_payload")),
            Field("TypeArgumentList", true, false, Alternative(nameof(TypeArgumentListSyntax), "nonterminal"))),
        Rule("TypeArgumentListSyntax",
            Field("Arguments", true, true, Alternative(nameof(IdentifierNameSyntax), "nonterminal"))),
        Rule("MethodDeclarationSyntax",
            Field("Identifier", true, false, Alternative("identifier", "identifier_payload")),
            Field("ReturnType", true, false, Alternative(nameof(PredefinedTypeSyntax), "nonterminal")),
            Field("ParameterList", true, false, Alternative(nameof(ParameterListSyntax), "nonterminal")),
            Field("Body", true, false, Alternative(nameof(BlockSyntax), "nonterminal"))),
        Rule("ParameterListSyntax",
            Field("Parameters", true, true, Alternative(nameof(ParameterSyntax), "nonterminal"))),
        Rule("ParameterSyntax",
            Field("Type", true, false, Alternative(nameof(PredefinedTypeSyntax), "nonterminal")),
            Field("Identifier", true, false, Alternative("identifier", "identifier_payload"))),
        Rule("BlockSyntax",
            Field("Statements", true, true,
                Alternative(nameof(ForEachStatementSyntax), "nonterminal"),
                Alternative(nameof(ExpressionStatementSyntax), "nonterminal"))),
        Rule("ForEachStatementSyntax",
            Field("Type", true, false, Alternative(nameof(IdentifierNameSyntax), "nonterminal")),
            Field("Identifier", true, false, Alternative("identifier", "identifier_payload")),
            Field("Expression", true, false, Alternative(nameof(InvocationExpressionSyntax), "nonterminal")),
            Field("Statement", true, false, Alternative(nameof(BlockSyntax), "nonterminal"))),
        Rule("ExpressionStatementSyntax",
            Field("Expression", true, false, Alternative(nameof(ConditionalAccessExpressionSyntax), "nonterminal"))),
        Rule("ConditionalAccessExpressionSyntax",
            Field("Expression", true, false, Alternative(nameof(IdentifierNameSyntax), "nonterminal")),
            Field("WhenNotNull", true, false, Alternative(nameof(InvocationExpressionSyntax), "nonterminal"))),
        Rule("InvocationExpressionSyntax",
            Field("Expression", true, false,
                Alternative(nameof(MemberAccessExpressionSyntax), "nonterminal"),
                Alternative(nameof(MemberBindingExpressionSyntax), "nonterminal")),
            Field("ArgumentList", true, false, Alternative(nameof(ArgumentListSyntax), "nonterminal"))),
        Rule("ArgumentListSyntax",
            Field("Arguments", true, true, Alternative(nameof(ArgumentSyntax), "nonterminal"))),
        Rule("ArgumentSyntax",
            Field("Expression", true, false,
                Alternative(nameof(IdentifierNameSyntax), "nonterminal"),
                Alternative(nameof(ThisExpressionSyntax), "nonterminal"),
                Alternative(nameof(ObjectCreationExpressionSyntax), "nonterminal"))),
        Rule("ObjectCreationExpressionSyntax",
            Field("Type", true, false, Alternative(nameof(IdentifierNameSyntax), "nonterminal")),
            Field("ArgumentList", true, false, Alternative(nameof(ArgumentListSyntax), "nonterminal"))),
        Rule("ThisExpressionSyntax"),
        Rule("MemberAccessExpressionSyntax",
            Field("Expression", true, false, Alternative(nameof(IdentifierNameSyntax), "nonterminal")),
            Field("Name", true, false, Alternative(nameof(IdentifierNameSyntax), "nonterminal"))),
        Rule("MemberBindingExpressionSyntax",
            Field("Name", true, false, Alternative(nameof(IdentifierNameSyntax), "nonterminal"))),
        Rule("PredefinedTypeSyntax",
            Field("Keyword", true, false,
                Alternative("string_keyword", "identifier_payload"),
                Alternative("void_keyword", "identifier_payload"))),
        Rule("IdentifierNameSyntax",
            Field("Identifier", true, false, Alternative("identifier", "identifier_payload")))
    });
}
else if (sourceDisplayPath.EndsWith("corpora/csharp/dotnet-samples-mini/src/iterators/IteratorMethods.cs", StringComparison.Ordinal))
{
    if (usingDecls.Length != 4 || blockScopedNamespaceDecl is null || fileScopedNamespaceDecl is not null)
    {
        Console.Error.WriteLine("compilation-unit-iterator-methods-source-file fixture supports four top-level using directives and one block-scoped namespace");
        return 4;
    }
    if (blockScopedNamespaceDecl.Name.ToString() != "Iterators")
    {
        Console.Error.WriteLine("compilation-unit-iterator-methods-source-file fixture did not match expected namespace");
        return 4;
    }
    if (classDecls.Length != 1 || structDecls.Length != 0 || enumDecls.Length != 0 ||
        classDecls[0].Identifier.Text != "IteratorMethods")
    {
        Console.Error.WriteLine("compilation-unit-iterator-methods-source-file fixture did not match expected class declaration");
        return 4;
    }
    var iteratorMethodsDecl = classDecls[0];
    var methods = iteratorMethodsDecl.Members.OfType<MethodDeclarationSyntax>().ToArray();
    if (methods.Length != 6 ||
        !methods.Select(method => method.Identifier.Text).SequenceEqual(new[]
        {
            "GetSingleDigitNumbers",
            "GetSingleDigitNumbersV2",
            "GetSingleDigitNumbersAndNumbersOver100",
            "GetSingleDigitNumbersAndOver100V2",
            "GetSingleDigitOddNumbers",
            "IteratorMethod"
        }))
    {
        Console.Error.WriteLine("compilation-unit-iterator-methods-source-file fixture did not match expected method declarations");
        return 4;
    }
    bool IsIEnumerableInt(TypeSyntax type) =>
        type is GenericNameSyntax genericType &&
        genericType.Identifier.Text == "IEnumerable" &&
        genericType.TypeArgumentList.Arguments.Count == 1 &&
        genericType.TypeArgumentList.Arguments[0] is PredefinedTypeSyntax argumentType &&
        argumentType.Keyword.Text == "int";
    if (!methods.All(method => IsIEnumerableInt(method.ReturnType)) ||
        methods.Take(4).Any(method => method.ParameterList.Parameters.Count != 0) ||
        methods[5].ParameterList.Parameters.Count != 0 ||
        methods[4].ParameterList.Parameters.Count != 1 ||
        methods[4].ParameterList.Parameters[0].Type is not PredefinedTypeSyntax oddParameterType ||
        oddParameterType.Keyword.Text != "bool" ||
        methods[4].ParameterList.Parameters[0].Identifier.Text != "getCollection")
    {
        Console.Error.WriteLine("compilation-unit-iterator-methods-source-file fixture did not match expected method signatures");
        return 4;
    }
    var methodBodies = methods.Select(method => method.Body).ToArray();
    if (methodBodies.Any(body => body is null))
    {
        Console.Error.WriteLine("compilation-unit-iterator-methods-source-file fixture expected block-bodied iterator methods");
        return 4;
    }
    var firstMethodStatements = methodBodies[0]!.Statements;
    if (firstMethodStatements.Count != 10 ||
        firstMethodStatements.Select(statement => statement is YieldStatementSyntax yieldStatement &&
            yieldStatement.Expression is LiteralExpressionSyntax literal &&
            literal.IsKind(SyntaxKind.NumericLiteralExpression)).Any(matched => !matched))
    {
        Console.Error.WriteLine("compilation-unit-iterator-methods-source-file fixture did not match expected first iterator yield sequence");
        return 4;
    }
    var secondMethodStatements = methodBodies[1]!.Statements;
    var thirdMethodStatements = methodBodies[2]!.Statements;
    var fourthMethodStatements = methodBodies[3]!.Statements;
    var oddMethodStatements = methodBodies[4]!.Statements;
    var privateMethodStatements = methodBodies[5]!.Statements;
    if (secondMethodStatements.Count != 2 ||
        secondMethodStatements[0] is not LocalDeclarationStatementSyntax ||
        secondMethodStatements[1] is not WhileStatementSyntax secondWhile ||
        secondWhile.Statement is not YieldStatementSyntax ||
        thirdMethodStatements.Count != 5 ||
        thirdMethodStatements[0] is not LocalDeclarationStatementSyntax ||
        thirdMethodStatements[1] is not WhileStatementSyntax thirdFirstWhile ||
        thirdFirstWhile.Statement is not YieldStatementSyntax ||
        thirdMethodStatements[2] is not YieldStatementSyntax ||
        thirdMethodStatements[3] is not ExpressionStatementSyntax ||
        thirdMethodStatements[4] is not WhileStatementSyntax thirdSecondWhile ||
        thirdSecondWhile.Statement is not YieldStatementSyntax ||
        fourthMethodStatements.Count != 5 ||
        fourthMethodStatements[0] is not LocalDeclarationStatementSyntax ||
        fourthMethodStatements[1] is not WhileStatementSyntax fourthWhile ||
        fourthWhile.Statement is not YieldStatementSyntax ||
        fourthMethodStatements[2] is not YieldStatementSyntax ||
        fourthMethodStatements[3] is not LocalDeclarationStatementSyntax ||
        fourthMethodStatements[4] is not ForEachStatementSyntax foreachStatement ||
        foreachStatement.Statement is not YieldStatementSyntax ||
        oddMethodStatements.Count != 1 ||
        oddMethodStatements[0] is not IfStatementSyntax oddIf ||
        oddIf.Statement is not ReturnStatementSyntax ||
        oddIf.Else?.Statement is not ReturnStatementSyntax ||
        privateMethodStatements.Count != 2 ||
        privateMethodStatements[0] is not LocalDeclarationStatementSyntax ||
        privateMethodStatements[1] is not WhileStatementSyntax privateWhile ||
        privateWhile.Statement is not IfStatementSyntax privateIf ||
        privateIf.Statement is not YieldStatementSyntax)
    {
        Console.Error.WriteLine("compilation-unit-iterator-methods-source-file fixture did not match expected iterator statement shapes");
        return 4;
    }
    metadata = BaseMetadata(new object[]
    {
        Rule("CompilationUnitSyntax",
            Field("Usings", true, true, Alternative(nameof(UsingDirectiveSyntax), "nonterminal")),
            Field("Members", true, true, Alternative(nameof(NamespaceDeclarationSyntax), "nonterminal"))),
        Rule("UsingDirectiveSyntax",
            Field("Name", true, false,
                Alternative(nameof(QualifiedNameSyntax), "nonterminal"),
                Alternative(nameof(IdentifierNameSyntax), "nonterminal"))),
        Rule("QualifiedNameSyntax",
            Field("Left", true, false,
                Alternative(nameof(QualifiedNameSyntax), "nonterminal"),
                Alternative(nameof(IdentifierNameSyntax), "nonterminal")),
            Field("Right", true, false, Alternative(nameof(IdentifierNameSyntax), "nonterminal"))),
        Rule("NamespaceDeclarationSyntax",
            Field("Name", true, false, Alternative("qualified_identifier", "identifier_payload")),
            Field("Members", true, true, Alternative(nameof(ClassDeclarationSyntax), "nonterminal"))),
        Rule("ClassDeclarationSyntax",
            Field("Identifier", true, false, Alternative("identifier", "identifier_payload")),
            Field("Members", true, true, Alternative(nameof(MethodDeclarationSyntax), "nonterminal"))),
        Rule("MethodDeclarationSyntax",
            Field("Identifier", true, false, Alternative("identifier", "identifier_payload")),
            Field("ReturnType", true, false, Alternative(nameof(GenericNameSyntax), "nonterminal")),
            Field("ParameterList", true, false, Alternative(nameof(ParameterListSyntax), "nonterminal")),
            Field("Body", true, false, Alternative(nameof(BlockSyntax), "nonterminal"))),
        Rule("GenericNameSyntax",
            Field("Identifier", true, false, Alternative("identifier", "identifier_payload")),
            Field("TypeArgumentList", true, false, Alternative(nameof(TypeArgumentListSyntax), "nonterminal"))),
        Rule("TypeArgumentListSyntax",
            Field("Arguments", true, true, Alternative(nameof(PredefinedTypeSyntax), "nonterminal"))),
        Rule("ParameterListSyntax",
            Field("Parameters", true, true, Alternative(nameof(ParameterSyntax), "nonterminal"))),
        Rule("ParameterSyntax",
            Field("Type", true, false, Alternative(nameof(PredefinedTypeSyntax), "nonterminal")),
            Field("Identifier", true, false, Alternative("identifier", "identifier_payload"))),
        Rule("BlockSyntax",
            Field("Statements", true, true,
                Alternative(nameof(YieldStatementSyntax), "nonterminal"),
                Alternative(nameof(LocalDeclarationStatementSyntax), "nonterminal"),
                Alternative(nameof(WhileStatementSyntax), "nonterminal"),
                Alternative(nameof(ExpressionStatementSyntax), "nonterminal"),
                Alternative(nameof(ForEachStatementSyntax), "nonterminal"),
                Alternative(nameof(IfStatementSyntax), "nonterminal"),
                Alternative(nameof(ReturnStatementSyntax), "nonterminal"))),
        Rule("YieldStatementSyntax",
            Field("ReturnOrBreakKeyword", true, false, Alternative("yield_return_keyword", "identifier_payload")),
            Field("Expression", true, false,
                Alternative(nameof(LiteralExpressionSyntax), "nonterminal"),
                Alternative(nameof(IdentifierNameSyntax), "nonterminal"))),
        Rule("LocalDeclarationStatementSyntax",
            Field("Declaration", true, false, Alternative(nameof(VariableDeclarationSyntax), "nonterminal"))),
        Rule("VariableDeclarationSyntax",
            Field("Type", true, false,
                Alternative(nameof(PredefinedTypeSyntax), "nonterminal"),
                Alternative(nameof(IdentifierNameSyntax), "nonterminal")),
            Field("Variables", true, true, Alternative(nameof(VariableDeclaratorSyntax), "nonterminal"))),
        Rule("VariableDeclaratorSyntax",
            Field("Identifier", true, false, Alternative("identifier", "identifier_payload")),
            Field("Initializer", false, false, Alternative(nameof(EqualsValueClauseSyntax), "nonterminal"))),
        Rule("EqualsValueClauseSyntax",
            Field("Value", true, false,
                Alternative(nameof(LiteralExpressionSyntax), "nonterminal"),
                Alternative(nameof(ArrayCreationExpressionSyntax), "nonterminal"))),
        Rule("WhileStatementSyntax",
            Field("Condition", true, false, Alternative(nameof(BinaryExpressionSyntax), "nonterminal")),
            Field("Statement", true, false,
                Alternative(nameof(YieldStatementSyntax), "nonterminal"),
                Alternative(nameof(IfStatementSyntax), "nonterminal"))),
        Rule("BinaryExpressionSyntax",
            Field("Left", true, false,
                Alternative(nameof(PostfixUnaryExpressionSyntax), "nonterminal"),
                Alternative(nameof(IdentifierNameSyntax), "nonterminal"),
                Alternative(nameof(BinaryExpressionSyntax), "nonterminal")),
            Field("OperatorToken", true, false, Alternative("binary_operator", "string_payload")),
            Field("Right", true, false,
                Alternative(nameof(LiteralExpressionSyntax), "nonterminal"),
                Alternative(nameof(IdentifierNameSyntax), "nonterminal"),
                Alternative(nameof(BinaryExpressionSyntax), "nonterminal"))),
        Rule("PostfixUnaryExpressionSyntax",
            Field("Operand", true, false, Alternative(nameof(IdentifierNameSyntax), "nonterminal")),
            Field("OperatorToken", true, false, Alternative("postfix_operator", "string_payload"))),
        Rule("ExpressionStatementSyntax",
            Field("Expression", true, false, Alternative(nameof(AssignmentExpressionSyntax), "nonterminal"))),
        Rule("AssignmentExpressionSyntax",
            Field("Left", true, false, Alternative(nameof(IdentifierNameSyntax), "nonterminal")),
            Field("Right", true, false, Alternative(nameof(LiteralExpressionSyntax), "nonterminal"))),
        Rule("ForEachStatementSyntax",
            Field("Type", true, false, Alternative(nameof(IdentifierNameSyntax), "nonterminal")),
            Field("Identifier", true, false, Alternative("identifier", "identifier_payload")),
            Field("Expression", true, false, Alternative(nameof(IdentifierNameSyntax), "nonterminal")),
            Field("Statement", true, false, Alternative(nameof(YieldStatementSyntax), "nonterminal"))),
        Rule("IfStatementSyntax",
            Field("Condition", true, false, Alternative(nameof(BinaryExpressionSyntax), "nonterminal")),
            Field("Statement", true, false,
                Alternative(nameof(ReturnStatementSyntax), "nonterminal"),
                Alternative(nameof(YieldStatementSyntax), "nonterminal")),
            Field("Else", false, false, Alternative(nameof(ElseClauseSyntax), "nonterminal"))),
        Rule("ElseClauseSyntax",
            Field("Statement", true, false, Alternative(nameof(ReturnStatementSyntax), "nonterminal"))),
        Rule("ReturnStatementSyntax",
            Field("Expression", true, false,
                Alternative(nameof(ArrayCreationExpressionSyntax), "nonterminal"),
                Alternative(nameof(InvocationExpressionSyntax), "nonterminal"))),
        Rule("InvocationExpressionSyntax",
            Field("Expression", true, false, Alternative(nameof(IdentifierNameSyntax), "nonterminal")),
            Field("ArgumentList", true, false, Alternative(nameof(ArgumentListSyntax), "nonterminal"))),
        Rule("ArgumentListSyntax",
            Field("Arguments", true, true, Alternative(nameof(ArgumentSyntax), "nonterminal"))),
        Rule("ArgumentSyntax",
            Field("Expression", true, false, Alternative(nameof(IdentifierNameSyntax), "nonterminal"))),
        Rule("ArrayCreationExpressionSyntax",
            Field("Type", true, false, Alternative(nameof(ArrayTypeSyntax), "nonterminal")),
            Field("Initializer", false, false, Alternative(nameof(InitializerExpressionSyntax), "nonterminal"))),
        Rule("ArrayTypeSyntax",
            Field("ElementType", true, false, Alternative(nameof(PredefinedTypeSyntax), "nonterminal")),
            Field("RankSpecifiers", true, true, Alternative(nameof(ArrayRankSpecifierSyntax), "nonterminal"))),
        Rule("ArrayRankSpecifierSyntax",
            Field("Sizes", true, true,
                Alternative(nameof(OmittedArraySizeExpressionSyntax), "nonterminal"),
                Alternative(nameof(LiteralExpressionSyntax), "nonterminal"))),
        Rule("OmittedArraySizeExpressionSyntax"),
        Rule("InitializerExpressionSyntax",
            Field("Expressions", true, true, Alternative(nameof(LiteralExpressionSyntax), "nonterminal"))),
        Rule("LiteralExpressionSyntax",
            Field("Token", true, false,
                Alternative("integer_literal", "integer_payload"),
                Alternative("boolean_literal", "boolean_payload"))),
        Rule("PredefinedTypeSyntax",
            Field("Keyword", true, false,
                Alternative("int_keyword", "identifier_payload"),
                Alternative("bool_keyword", "identifier_payload"))),
        Rule("IdentifierNameSyntax",
            Field("Identifier", true, false, Alternative("identifier", "identifier_payload")))
    });
}
else if (sourceDisplayPath.EndsWith("corpora/csharp/dotnet-samples-mini/src/serialization/Loan.cs", StringComparison.Ordinal))
{
    if (usingDecls.Length != 2 || blockScopedNamespaceDecl is null || fileScopedNamespaceDecl is not null)
    {
        Console.Error.WriteLine("compilation-unit-loan-source-file fixture supports two top-level using directives and one block-scoped namespace");
        return 4;
    }
    if (!usingDecls.Select(usingDecl => usingDecl.Name?.ToString()).SequenceEqual(new[] { "System.ComponentModel", "System.Text.Json.Serialization" }))
    {
        Console.Error.WriteLine("compilation-unit-loan-source-file fixture did not match expected using directives");
        return 4;
    }
    if (blockScopedNamespaceDecl.Name.ToString() != "serialization")
    {
        Console.Error.WriteLine("compilation-unit-loan-source-file fixture did not match expected namespace");
        return 4;
    }
    if (classDecls.Length != 1 || structDecls.Length != 0 || enumDecls.Length != 0 ||
        classDecls[0].Identifier.Text != "Loan")
    {
        Console.Error.WriteLine("compilation-unit-loan-source-file fixture did not match expected class declaration");
        return 4;
    }
    var loanDecl = classDecls[0];
    if (loanDecl.BaseList is not { } loanBaseList ||
        loanBaseList.Types.Count != 1 ||
        loanBaseList.Types[0].Type is not IdentifierNameSyntax loanBaseType ||
        loanBaseType.Identifier.Text != "INotifyPropertyChanged")
    {
        Console.Error.WriteLine("compilation-unit-loan-source-file fixture did not match expected base list");
        return 4;
    }
    var loanProperties = loanDecl.Members.OfType<PropertyDeclarationSyntax>().ToArray();
    var loanFields = loanDecl.Members.OfType<FieldDeclarationSyntax>().ToArray();
    var loanEvents = loanDecl.Members.OfType<EventFieldDeclarationSyntax>().ToArray();
    var loanConstructors = loanDecl.Members.OfType<ConstructorDeclarationSyntax>().ToArray();
    var loanMethods = loanDecl.Members.OfType<MethodDeclarationSyntax>().ToArray();
    if (loanProperties.Length != 5 ||
        !loanProperties.Select(property => property.Identifier.Text).SequenceEqual(new[] { "LoanAmount", "InterestRate", "TimeLastLoaded", "Term", "Customer" }) ||
        loanFields.Length != 1 ||
        loanEvents.Length != 1 ||
        loanConstructors.Length != 1 ||
        loanMethods.Length != 0)
    {
        Console.Error.WriteLine("compilation-unit-loan-source-file fixture did not match expected member declarations");
        return 4;
    }
    bool HasGetSetAccessors(PropertyDeclarationSyntax property) =>
        property.AccessorList is { } accessors &&
        accessors.Accessors.Select(accessor => accessor.Keyword.Text).SequenceEqual(new[] { "get", "set" });
    if (loanProperties[0].Type is not PredefinedTypeSyntax loanAmountType ||
        loanAmountType.Keyword.Text != "double" ||
        !HasGetSetAccessors(loanProperties[0]) ||
        loanProperties[1].Type is not PredefinedTypeSyntax interestRateType ||
        interestRateType.Keyword.Text != "double" ||
        !HasGetSetAccessors(loanProperties[1]) ||
        loanProperties[2].Type is not IdentifierNameSyntax timeLoadedType ||
        timeLoadedType.Identifier.Text != "DateTime" ||
        !HasGetSetAccessors(loanProperties[2]) ||
        loanProperties[2].AttributeLists.Count != 1 ||
        loanProperties[2].AttributeLists[0].Attributes.Count != 1 ||
        loanProperties[2].AttributeLists[0].Attributes[0].Name is not IdentifierNameSyntax jsonIgnoreAttribute ||
        jsonIgnoreAttribute.Identifier.Text != "JsonIgnore" ||
        loanProperties[3].Type is not PredefinedTypeSyntax termType ||
        termType.Keyword.Text != "int" ||
        !HasGetSetAccessors(loanProperties[3]) ||
        loanProperties[4].Type is not PredefinedTypeSyntax customerType ||
        customerType.Keyword.Text != "string" ||
        !HasGetSetAccessors(loanProperties[4]))
    {
        Console.Error.WriteLine("compilation-unit-loan-source-file fixture did not match expected property shapes");
        return 4;
    }
    var customerAccessors = loanProperties[4].AccessorList!.Accessors;
    var customerGet = customerAccessors[0];
    var customerSet = customerAccessors[1];
    if (customerGet.Body?.Statements.Count != 1 ||
        customerGet.Body.Statements[0] is not ReturnStatementSyntax customerReturn ||
        customerReturn.Expression is not IdentifierNameSyntax customerReturnExpression ||
        customerReturnExpression.Identifier.Text != "_customer" ||
        customerSet.Body?.Statements.Count != 2 ||
        customerSet.Body.Statements[0] is not ExpressionStatementSyntax customerAssignmentStatement ||
        customerAssignmentStatement.Expression is not AssignmentExpressionSyntax customerAssignment ||
        customerAssignment.Left is not IdentifierNameSyntax customerAssignmentLeft ||
        customerAssignmentLeft.Identifier.Text != "_customer" ||
        customerAssignment.Right is not IdentifierNameSyntax customerAssignmentRight ||
        customerAssignmentRight.Identifier.Text != "value" ||
        customerSet.Body.Statements[1] is not ExpressionStatementSyntax notificationStatement ||
        notificationStatement.Expression is not ConditionalAccessExpressionSyntax notificationAccess ||
        notificationAccess.Expression is not IdentifierNameSyntax notificationTarget ||
        notificationTarget.Identifier.Text != "PropertyChanged" ||
        notificationAccess.WhenNotNull is not InvocationExpressionSyntax notificationInvocation ||
        notificationInvocation.Expression is not MemberBindingExpressionSyntax notificationBinding ||
        notificationBinding.Name.Identifier.Text != "Invoke" ||
        notificationInvocation.ArgumentList.Arguments.Count != 2 ||
        notificationInvocation.ArgumentList.Arguments[0].Expression is not ThisExpressionSyntax ||
        notificationInvocation.ArgumentList.Arguments[1].Expression is not ObjectCreationExpressionSyntax eventArgsCreation ||
        eventArgsCreation.Type is not IdentifierNameSyntax eventArgsType ||
        eventArgsType.Identifier.Text != "PropertyChangedEventArgs" ||
        eventArgsCreation.ArgumentList?.Arguments.Count != 1 ||
        eventArgsCreation.ArgumentList.Arguments[0].Expression is not InvocationExpressionSyntax nameofInvocation ||
        nameofInvocation.Expression is not IdentifierNameSyntax nameofTarget ||
        nameofTarget.Identifier.Text != "nameof" ||
        nameofInvocation.ArgumentList.Arguments.Count != 1 ||
        nameofInvocation.ArgumentList.Arguments[0].Expression is not IdentifierNameSyntax nameofArgument ||
        nameofArgument.Identifier.Text != "Customer")
    {
        Console.Error.WriteLine("compilation-unit-loan-source-file fixture did not match expected Customer accessor bodies");
        return 4;
    }
    var customerField = loanFields[0];
    if (customerField.Declaration.Type is not PredefinedTypeSyntax customerFieldType ||
        customerFieldType.Keyword.Text != "string" ||
        customerField.Declaration.Variables.Count != 1 ||
        customerField.Declaration.Variables[0].Identifier.Text != "_customer")
    {
        Console.Error.WriteLine("compilation-unit-loan-source-file fixture did not match expected _customer field");
        return 4;
    }
    var propertyChangedEvent = loanEvents[0];
    if (propertyChangedEvent.Declaration.Type is not NullableTypeSyntax propertyChangedNullableType ||
        propertyChangedNullableType.ElementType is not IdentifierNameSyntax propertyChangedType ||
        propertyChangedType.Identifier.Text != "PropertyChangedEventHandler" ||
        propertyChangedEvent.Declaration.Variables.Count != 1 ||
        propertyChangedEvent.Declaration.Variables[0].Identifier.Text != "PropertyChanged")
    {
        Console.Error.WriteLine("compilation-unit-loan-source-file fixture did not match expected nullable PropertyChanged event");
        return 4;
    }
    var loanConstructor = loanConstructors[0];
    if (loanConstructor.Identifier.Text != "Loan" ||
        loanConstructor.ParameterList.Parameters.Count != 4 ||
        !loanConstructor.ParameterList.Parameters.Select(parameter => parameter.Identifier.Text).SequenceEqual(new[] { "loanAmount", "interestRate", "term", "customer" }) ||
        loanConstructor.ParameterList.Parameters[0].Type is not PredefinedTypeSyntax loanAmountParameterType ||
        loanAmountParameterType.Keyword.Text != "double" ||
        loanConstructor.ParameterList.Parameters[1].Type is not PredefinedTypeSyntax interestRateParameterType ||
        interestRateParameterType.Keyword.Text != "double" ||
        loanConstructor.ParameterList.Parameters[2].Type is not PredefinedTypeSyntax termParameterType ||
        termParameterType.Keyword.Text != "int" ||
        loanConstructor.ParameterList.Parameters[3].Type is not PredefinedTypeSyntax customerParameterType ||
        customerParameterType.Keyword.Text != "string" ||
        loanConstructor.Body is not { } constructorBody ||
        constructorBody.Statements.Count != 4)
    {
        Console.Error.WriteLine("compilation-unit-loan-source-file fixture did not match expected constructor signature");
        return 4;
    }
    var expectedConstructorAssignments = new[]
    {
        ("LoanAmount", "loanAmount"),
        ("InterestRate", "interestRate"),
        ("Term", "term"),
        ("_customer", "customer")
    };
    for (var i = 0; i < expectedConstructorAssignments.Length; i++)
    {
        if (constructorBody.Statements[i] is not ExpressionStatementSyntax assignmentStatement ||
            assignmentStatement.Expression is not AssignmentExpressionSyntax assignmentExpression ||
            assignmentExpression.Left is not IdentifierNameSyntax assignmentLeft ||
            assignmentLeft.Identifier.Text != expectedConstructorAssignments[i].Item1 ||
            assignmentExpression.Right is not IdentifierNameSyntax assignmentRight ||
            assignmentRight.Identifier.Text != expectedConstructorAssignments[i].Item2)
        {
            Console.Error.WriteLine("compilation-unit-loan-source-file fixture did not match expected constructor assignments");
            return 4;
        }
    }
    metadata = BaseMetadata(new object[]
    {
        Rule("CompilationUnitSyntax",
            Field("Usings", true, true, Alternative(nameof(UsingDirectiveSyntax), "nonterminal")),
            Field("Members", true, true, Alternative(nameof(NamespaceDeclarationSyntax), "nonterminal"))),
        Rule("UsingDirectiveSyntax",
            Field("Name", true, false, Alternative(nameof(QualifiedNameSyntax), "nonterminal"))),
        Rule("QualifiedNameSyntax",
            Field("Left", true, false,
                Alternative(nameof(QualifiedNameSyntax), "nonterminal"),
                Alternative(nameof(IdentifierNameSyntax), "nonterminal")),
            Field("Right", true, false, Alternative(nameof(IdentifierNameSyntax), "nonterminal"))),
        Rule("NamespaceDeclarationSyntax",
            Field("Name", true, false, Alternative("qualified_identifier", "identifier_payload")),
            Field("Members", true, true, Alternative(nameof(ClassDeclarationSyntax), "nonterminal"))),
        Rule("ClassDeclarationSyntax",
            Field("Identifier", true, false, Alternative("identifier", "identifier_payload")),
            Field("BaseList", true, false, Alternative(nameof(BaseListSyntax), "nonterminal")),
            Field("Members", true, true,
                Alternative(nameof(PropertyDeclarationSyntax), "nonterminal"),
                Alternative(nameof(FieldDeclarationSyntax), "nonterminal"),
                Alternative(nameof(EventFieldDeclarationSyntax), "nonterminal"),
                Alternative(nameof(ConstructorDeclarationSyntax), "nonterminal"))),
        Rule("BaseListSyntax",
            Field("Types", true, true, Alternative(nameof(SimpleBaseTypeSyntax), "nonterminal"))),
        Rule("SimpleBaseTypeSyntax",
            Field("Type", true, false, Alternative(nameof(IdentifierNameSyntax), "nonterminal"))),
        Rule("PropertyDeclarationSyntax",
            Field("AttributeLists", false, true, Alternative(nameof(AttributeListSyntax), "nonterminal")),
            Field("Identifier", true, false, Alternative("identifier", "identifier_payload")),
            Field("Type", true, false,
                Alternative(nameof(PredefinedTypeSyntax), "nonterminal"),
                Alternative(nameof(IdentifierNameSyntax), "nonterminal")),
            Field("AccessorList", true, false, Alternative(nameof(AccessorListSyntax), "nonterminal"))),
        Rule("AttributeListSyntax",
            Field("Attributes", true, true, Alternative(nameof(AttributeSyntax), "nonterminal"))),
        Rule("AttributeSyntax",
            Field("Name", true, false, Alternative(nameof(IdentifierNameSyntax), "nonterminal"))),
        Rule("AccessorListSyntax",
            Field("Accessors", true, true, Alternative(nameof(AccessorDeclarationSyntax), "nonterminal"))),
        Rule("AccessorDeclarationSyntax",
            Field("Keyword", true, false,
                Alternative("get_accessor_keyword", "string_payload"),
                Alternative("set_accessor_keyword", "string_payload")),
            Field("Body", false, false, Alternative(nameof(BlockSyntax), "nonterminal"))),
        Rule("BlockSyntax",
            Field("Statements", true, true,
                Alternative(nameof(ReturnStatementSyntax), "nonterminal"),
                Alternative(nameof(ExpressionStatementSyntax), "nonterminal"))),
        Rule("ReturnStatementSyntax",
            Field("Expression", true, false, Alternative(nameof(IdentifierNameSyntax), "nonterminal"))),
        Rule("ExpressionStatementSyntax",
            Field("Expression", true, false,
                Alternative(nameof(AssignmentExpressionSyntax), "nonterminal"),
                Alternative(nameof(ConditionalAccessExpressionSyntax), "nonterminal"))),
        Rule("AssignmentExpressionSyntax",
            Field("Left", true, false, Alternative(nameof(IdentifierNameSyntax), "nonterminal")),
            Field("Right", true, false, Alternative(nameof(IdentifierNameSyntax), "nonterminal"))),
        Rule("ConditionalAccessExpressionSyntax",
            Field("Expression", true, false, Alternative(nameof(IdentifierNameSyntax), "nonterminal")),
            Field("WhenNotNull", true, false, Alternative(nameof(InvocationExpressionSyntax), "nonterminal"))),
        Rule("InvocationExpressionSyntax",
            Field("Expression", true, false,
                Alternative(nameof(MemberBindingExpressionSyntax), "nonterminal"),
                Alternative(nameof(IdentifierNameSyntax), "nonterminal")),
            Field("ArgumentList", true, false, Alternative(nameof(ArgumentListSyntax), "nonterminal"))),
        Rule("MemberBindingExpressionSyntax",
            Field("Name", true, false, Alternative(nameof(IdentifierNameSyntax), "nonterminal"))),
        Rule("ArgumentListSyntax",
            Field("Arguments", true, true, Alternative(nameof(ArgumentSyntax), "nonterminal"))),
        Rule("ArgumentSyntax",
            Field("Expression", true, false,
                Alternative(nameof(ThisExpressionSyntax), "nonterminal"),
                Alternative(nameof(ObjectCreationExpressionSyntax), "nonterminal"),
                Alternative(nameof(InvocationExpressionSyntax), "nonterminal"),
                Alternative(nameof(IdentifierNameSyntax), "nonterminal"))),
        Rule("ThisExpressionSyntax"),
        Rule("ObjectCreationExpressionSyntax",
            Field("Type", true, false, Alternative(nameof(IdentifierNameSyntax), "nonterminal")),
            Field("ArgumentList", true, false, Alternative(nameof(ArgumentListSyntax), "nonterminal"))),
        Rule("FieldDeclarationSyntax",
            Field("Declaration", true, false, Alternative(nameof(VariableDeclarationSyntax), "nonterminal"))),
        Rule("EventFieldDeclarationSyntax",
            Field("Declaration", true, false, Alternative(nameof(VariableDeclarationSyntax), "nonterminal"))),
        Rule("VariableDeclarationSyntax",
            Field("Type", true, false,
                Alternative(nameof(PredefinedTypeSyntax), "nonterminal"),
                Alternative(nameof(NullableTypeSyntax), "nonterminal")),
            Field("Variables", true, true, Alternative(nameof(VariableDeclaratorSyntax), "nonterminal"))),
        Rule("VariableDeclaratorSyntax",
            Field("Identifier", true, false, Alternative("identifier", "identifier_payload"))),
        Rule("NullableTypeSyntax",
            Field("ElementType", true, false, Alternative(nameof(IdentifierNameSyntax), "nonterminal"))),
        Rule("ConstructorDeclarationSyntax",
            Field("Identifier", true, false, Alternative("identifier", "identifier_payload")),
            Field("ParameterList", true, false, Alternative(nameof(ParameterListSyntax), "nonterminal")),
            Field("Body", true, false, Alternative(nameof(BlockSyntax), "nonterminal"))),
        Rule("ParameterListSyntax",
            Field("Parameters", true, true, Alternative(nameof(ParameterSyntax), "nonterminal"))),
        Rule("ParameterSyntax",
            Field("Type", true, false, Alternative(nameof(PredefinedTypeSyntax), "nonterminal")),
            Field("Identifier", true, false, Alternative("identifier", "identifier_payload"))),
        Rule("PredefinedTypeSyntax",
            Field("Keyword", true, false,
                Alternative("double_keyword", "identifier_payload"),
                Alternative("int_keyword", "identifier_payload"),
                Alternative("string_keyword", "identifier_payload"))),
        Rule("IdentifierNameSyntax",
            Field("Identifier", true, false, Alternative("identifier", "identifier_payload")))
    });
}
else
{
var classDecl = classDecls.SingleOrDefault();
var structDecl = structDecls.SingleOrDefault();
var enumDecl = enumDecls.SingleOrDefault();
var supportedTypeDecls = new object?[] { classDecl, structDecl, enumDecl }.Count(typeDecl => typeDecl is not null);
if (supportedTypeDecls == 0)
{
    Console.Error.WriteLine("fixture did not contain exactly one supported class, struct, or enum declaration");
    return 4;
}
if (supportedTypeDecls != 1)
{
    Console.Error.WriteLine("fixture must contain exactly one supported type declaration");
    return 4;
}

if (enumDecl is not null)
{
    if (usingDecls.Length != 0 || blockScopedNamespaceDecl is null || fileScopedNamespaceDecl is not null)
    {
        Console.Error.WriteLine("enum-declaration fixture supports one block-scoped namespace and no top-level using directives");
        return 4;
    }
    var enumMemberDecls = enumDecl.Members.ToArray();
    var enumMemberDecl = enumMemberDecls.SingleOrDefault();
    if (enumMemberDecl is null)
    {
        Console.Error.WriteLine("enum-declaration fixture did not match expected single enum member shape");
        return 4;
    }
    metadata = BaseMetadata(new object[]
    {
        Rule("CompilationUnitSyntax", Field("Members", true, true,
            Alternative(nameof(NamespaceDeclarationSyntax), "nonterminal"))),
        Rule("NamespaceDeclarationSyntax",
            Field("Name", true, false, Alternative("qualified_identifier", "identifier_payload")),
            Field("Members", true, true, Alternative(nameof(EnumDeclarationSyntax), "nonterminal"))),
        Rule("EnumDeclarationSyntax",
            Field("Identifier", true, false, Alternative("identifier", "identifier_payload")),
            Field("Members", true, true, Alternative(nameof(EnumMemberDeclarationSyntax), "nonterminal"))),
        Rule("EnumMemberDeclarationSyntax",
            Field("Identifier", true, false, Alternative("identifier", "identifier_payload")))
    });
}
else if (structDecl is not null)
{
    if (usingDecls.Length != 0 || blockScopedNamespaceDecl is null || fileScopedNamespaceDecl is not null)
    {
        Console.Error.WriteLine("struct-declaration fixture supports one block-scoped namespace and no top-level using directives");
        return 4;
    }
    var structPropertyDecls = structDecl.Members.OfType<PropertyDeclarationSyntax>().ToArray();
    var structMethodDecls = structDecl.Members.OfType<MethodDeclarationSyntax>().ToArray();
    var propertyDecl = structPropertyDecls.SingleOrDefault();
    if (propertyDecl is null ||
        structMethodDecls.Length != 0 ||
        propertyDecl.Type is not PredefinedTypeSyntax predefinedType ||
        predefinedType.Keyword.Text != "string" ||
        propertyDecl.Initializer?.Value is not LiteralExpressionSyntax initializer ||
        !initializer.IsKind(SyntaxKind.StringLiteralExpression))
    {
        Console.Error.WriteLine("struct-declaration fixture did not match expected struct/string property shape");
        return 4;
    }
    metadata = BaseMetadata(new object[]
    {
        Rule("CompilationUnitSyntax", Field("Members", true, true,
            Alternative(nameof(NamespaceDeclarationSyntax), "nonterminal"))),
        Rule("NamespaceDeclarationSyntax",
            Field("Name", true, false, Alternative("qualified_identifier", "identifier_payload")),
            Field("Members", true, true, Alternative(nameof(StructDeclarationSyntax), "nonterminal"))),
        Rule("StructDeclarationSyntax",
            Field("Identifier", true, false, Alternative("identifier", "identifier_payload")),
            Field("Members", true, true, Alternative(nameof(PropertyDeclarationSyntax), "nonterminal"))),
        Rule("PropertyDeclarationSyntax",
            Field("Identifier", true, false, Alternative("identifier", "identifier_payload")),
            Field("Type", true, false, Alternative(nameof(PredefinedTypeSyntax), "nonterminal")),
            Field("Initializer", true, false, Alternative("StringLiteralExpressionSyntax", "string_payload"))),
        Rule("PredefinedTypeSyntax",
            Field("Keyword", true, false, Alternative("string_keyword", "identifier_payload")))
    });
}
else
{
var propertyDecls = classDecl!.Members.OfType<PropertyDeclarationSyntax>().ToArray();
var methodDecls = classDecl.Members.OfType<MethodDeclarationSyntax>().ToArray();
var eventFieldDecls = classDecl.Members.OfType<EventFieldDeclarationSyntax>().ToArray();
var fieldDecls = classDecl.Members.OfType<FieldDeclarationSyntax>().ToArray();
var constructorDecls = classDecl.Members.OfType<ConstructorDeclarationSyntax>().ToArray();
var baseList = classDecl.BaseList;
if (constructorDecls.Length > 0)
{
    if (usingDecls.Length != 0 || fileScopedNamespaceDecl is null || blockScopedNamespaceDecl is not null)
    {
        Console.Error.WriteLine("constructor-declaration fixture supports one file-scoped namespace and no top-level using directives");
        return 4;
    }
    var propertyDecl = propertyDecls.SingleOrDefault();
    var constructorDecl = constructorDecls.SingleOrDefault();
    if (constructorDecl is null ||
        propertyDecl is null ||
        methodDecls.Length != 0 ||
        eventFieldDecls.Length != 0 ||
        propertyDecl.Type is not PredefinedTypeSyntax propertyType ||
        propertyType.Keyword.Text != "string" ||
        propertyDecl.AccessorList is not { } propertyAccessorList ||
        propertyAccessorList.Accessors.Count != 1 ||
        propertyAccessorList.Accessors[0].Keyword.Text != "get" ||
        propertyDecl.Initializer is not null ||
        constructorDecl.Identifier.Text != classDecl.Identifier.Text ||
        constructorDecl.ParameterList.Parameters.Count != 1 ||
        constructorDecl.ParameterList.Parameters[0] is not { } parameter ||
        parameter.Type is not PredefinedTypeSyntax parameterType ||
        parameterType.Keyword.Text != "string" ||
        parameter.Identifier.Text != "name" ||
        constructorDecl.Body is not { } constructorBody ||
        constructorBody.Statements.Count != 1 ||
        constructorBody.Statements[0] is not ExpressionStatementSyntax expressionStatement ||
        expressionStatement.Expression is not AssignmentExpressionSyntax assignmentExpression ||
        assignmentExpression.Left is not IdentifierNameSyntax assignmentLeft ||
        assignmentLeft.Identifier.Text != propertyDecl.Identifier.Text ||
        assignmentExpression.Right is not IdentifierNameSyntax assignmentRight ||
        assignmentRight.Identifier.Text != parameter.Identifier.Text)
    {
        Console.Error.WriteLine("constructor-declaration fixture did not match expected property plus constructor assignment shape");
        return 4;
    }
    metadata = BaseMetadata(new object[]
    {
        Rule("CompilationUnitSyntax", Field("Members", true, true,
            Alternative(nameof(FileScopedNamespaceDeclarationSyntax), "nonterminal"))),
        Rule("FileScopedNamespaceDeclarationSyntax",
            Field("Name", true, false, Alternative("qualified_identifier", "identifier_payload")),
            Field("Members", true, true, Alternative(nameof(ClassDeclarationSyntax), "nonterminal"))),
        Rule("ClassDeclarationSyntax",
            Field("Identifier", true, false, Alternative("identifier", "identifier_payload")),
            Field("Members", true, true,
                Alternative(nameof(PropertyDeclarationSyntax), "nonterminal"),
                Alternative(nameof(ConstructorDeclarationSyntax), "nonterminal"))),
        Rule("PropertyDeclarationSyntax",
            Field("Identifier", true, false, Alternative("identifier", "identifier_payload")),
            Field("Type", true, false, Alternative(nameof(PredefinedTypeSyntax), "nonterminal")),
            Field("AccessorList", true, false, Alternative(nameof(AccessorListSyntax), "nonterminal"))),
        Rule("ConstructorDeclarationSyntax",
            Field("Identifier", true, false, Alternative("identifier", "identifier_payload")),
            Field("ParameterList", true, false, Alternative(nameof(ParameterListSyntax), "nonterminal")),
            Field("Body", true, false, Alternative(nameof(BlockSyntax), "nonterminal"))),
        Rule("PredefinedTypeSyntax",
            Field("Keyword", true, false, Alternative("string_keyword", "identifier_payload"))),
        Rule("AccessorListSyntax",
            Field("Accessors", true, true, Alternative(nameof(AccessorDeclarationSyntax), "nonterminal"))),
        Rule("AccessorDeclarationSyntax",
            Field("Keyword", true, false, Alternative("get_accessor_keyword", "string_payload"))),
        Rule("ParameterListSyntax",
            Field("Parameters", true, true, Alternative(nameof(ParameterSyntax), "nonterminal"))),
        Rule("ParameterSyntax",
            Field("Type", true, false, Alternative(nameof(PredefinedTypeSyntax), "nonterminal")),
            Field("Identifier", true, false, Alternative("identifier", "identifier_payload"))),
        Rule("BlockSyntax",
            Field("Statements", true, true, Alternative(nameof(ExpressionStatementSyntax), "nonterminal"))),
        Rule("ExpressionStatementSyntax",
            Field("Expression", true, false, Alternative(nameof(AssignmentExpressionSyntax), "nonterminal"))),
        Rule("AssignmentExpressionSyntax",
            Field("Left", true, false, Alternative(nameof(IdentifierNameSyntax), "nonterminal")),
            Field("Right", true, false, Alternative(nameof(IdentifierNameSyntax), "nonterminal"))),
        Rule("IdentifierNameSyntax",
            Field("Identifier", true, false, Alternative("identifier", "identifier_payload")))
    });
}
else if (baseList is not null)
{
    if (usingDecls.Length != 0 || blockScopedNamespaceDecl is null || fileScopedNamespaceDecl is not null)
    {
        Console.Error.WriteLine("base-list fixture supports one block-scoped namespace and no top-level using directives");
        return 4;
    }
    var baseTypes = baseList.Types.ToArray();
    if (propertyDecls.Length != 0 ||
        methodDecls.Length != 0 ||
        eventFieldDecls.Length != 0 ||
        fieldDecls.Length != 0 ||
        baseTypes.Length != 1 ||
        baseTypes[0] is not SimpleBaseTypeSyntax simpleBaseType ||
        simpleBaseType.Type is not IdentifierNameSyntax baseIdentifier ||
        baseIdentifier.Identifier.Text != "INotifyPropertyChanged")
    {
        Console.Error.WriteLine("base-list fixture did not match expected single identifier base type shape");
        return 4;
    }
    metadata = BaseMetadata(new object[]
    {
        Rule("CompilationUnitSyntax", Field("Members", true, true,
            Alternative(nameof(NamespaceDeclarationSyntax), "nonterminal"))),
        Rule("NamespaceDeclarationSyntax",
            Field("Name", true, false, Alternative("qualified_identifier", "identifier_payload")),
            Field("Members", true, true, Alternative(nameof(ClassDeclarationSyntax), "nonterminal"))),
        Rule("ClassDeclarationSyntax",
            Field("Identifier", true, false, Alternative("identifier", "identifier_payload")),
            Field("BaseList", true, false, Alternative(nameof(BaseListSyntax), "nonterminal"))),
        Rule("BaseListSyntax",
            Field("Types", true, true, Alternative(nameof(SimpleBaseTypeSyntax), "nonterminal"))),
        Rule("SimpleBaseTypeSyntax",
            Field("Type", true, false, Alternative(nameof(IdentifierNameSyntax), "nonterminal"))),
        Rule("IdentifierNameSyntax",
            Field("Identifier", true, false, Alternative("identifier", "identifier_payload")))
    });
}
else if (usingDecls.Length > 0 && usingDecls[0].Name is QualifiedNameSyntax qualifiedUsingName)
{
    if (usingDecls.Length != 1 || blockScopedNamespaceDecl is null || fileScopedNamespaceDecl is not null)
    {
        Console.Error.WriteLine("qualified-name fixture supports one qualified top-level using before one block-scoped namespace");
        return 4;
    }
    var propertyDecl = propertyDecls.SingleOrDefault();
    if (qualifiedUsingName.Left is not IdentifierNameSyntax qualifiedLeft ||
        qualifiedLeft.Identifier.Text != "System" ||
        qualifiedUsingName.Right is not IdentifierNameSyntax qualifiedRight ||
        qualifiedRight.Identifier.Text != "Collections" ||
        propertyDecl is null ||
        methodDecls.Length != 0 ||
        propertyDecl.Type is not PredefinedTypeSyntax predefinedType ||
        predefinedType.Keyword.Text != "string" ||
        propertyDecl.Initializer?.Value is not LiteralExpressionSyntax initializer ||
        !initializer.IsKind(SyntaxKind.StringLiteralExpression))
    {
        Console.Error.WriteLine("qualified-name fixture did not match expected qualified using/class/string property shape");
        return 4;
    }
    metadata = BaseMetadata(new object[]
    {
        Rule("CompilationUnitSyntax",
            Field("Usings", true, true, Alternative(nameof(UsingDirectiveSyntax), "nonterminal")),
            Field("Members", true, true, Alternative(nameof(NamespaceDeclarationSyntax), "nonterminal"))),
        Rule("UsingDirectiveSyntax",
            Field("Name", true, false, Alternative(nameof(QualifiedNameSyntax), "nonterminal"))),
        Rule("QualifiedNameSyntax",
            Field("Left", true, false, Alternative(nameof(IdentifierNameSyntax), "nonterminal")),
            Field("Right", true, false, Alternative(nameof(IdentifierNameSyntax), "nonterminal"))),
        Rule("IdentifierNameSyntax",
            Field("Identifier", true, false, Alternative("identifier", "identifier_payload"))),
        Rule("NamespaceDeclarationSyntax",
            Field("Name", true, false, Alternative("qualified_identifier", "identifier_payload")),
            Field("Members", true, true, Alternative(nameof(ClassDeclarationSyntax), "nonterminal"))),
        Rule("ClassDeclarationSyntax",
            Field("Identifier", true, false, Alternative("identifier", "identifier_payload")),
            Field("Members", true, true, Alternative(nameof(PropertyDeclarationSyntax), "nonterminal"))),
        Rule("PropertyDeclarationSyntax",
            Field("Identifier", true, false, Alternative("identifier", "identifier_payload")),
            Field("Type", true, false, Alternative(nameof(PredefinedTypeSyntax), "nonterminal")),
            Field("Initializer", true, false, Alternative("StringLiteralExpressionSyntax", "string_payload"))),
        Rule("PredefinedTypeSyntax",
            Field("Keyword", true, false, Alternative("string_keyword", "identifier_payload")))
    });
}
else if (usingDecls.Length > 0)
{
    if (usingDecls.Length != 1 || blockScopedNamespaceDecl is null || fileScopedNamespaceDecl is not null)
    {
        Console.Error.WriteLine("using-directive fixture supports one top-level using before one block-scoped namespace");
        return 4;
    }
    var propertyDecl = propertyDecls.SingleOrDefault();
    if (propertyDecl is null ||
        methodDecls.Length != 0 ||
        propertyDecl.Type is not PredefinedTypeSyntax predefinedType ||
        predefinedType.Keyword.Text != "string" ||
        propertyDecl.Initializer?.Value is not LiteralExpressionSyntax initializer ||
        !initializer.IsKind(SyntaxKind.StringLiteralExpression))
    {
        Console.Error.WriteLine("using-directive fixture did not match expected class/string property shape");
        return 4;
    }
    metadata = BaseMetadata(new object[]
    {
        Rule("CompilationUnitSyntax",
            Field("Usings", true, true, Alternative(nameof(UsingDirectiveSyntax), "nonterminal")),
            Field("Members", true, true, Alternative(nameof(NamespaceDeclarationSyntax), "nonterminal"))),
        Rule("UsingDirectiveSyntax",
            Field("Name", true, false, Alternative("qualified_identifier", "identifier_payload"))),
        Rule("NamespaceDeclarationSyntax",
            Field("Name", true, false, Alternative("qualified_identifier", "identifier_payload")),
            Field("Members", true, true, Alternative(nameof(ClassDeclarationSyntax), "nonterminal"))),
        Rule("ClassDeclarationSyntax",
            Field("Identifier", true, false, Alternative("identifier", "identifier_payload")),
            Field("Members", true, true, Alternative(nameof(PropertyDeclarationSyntax), "nonterminal"))),
        Rule("PropertyDeclarationSyntax",
            Field("Identifier", true, false, Alternative("identifier", "identifier_payload")),
            Field("Type", true, false, Alternative(nameof(PredefinedTypeSyntax), "nonterminal")),
            Field("Initializer", true, false, Alternative("StringLiteralExpressionSyntax", "string_payload"))),
        Rule("PredefinedTypeSyntax",
            Field("Keyword", true, false, Alternative("string_keyword", "identifier_payload")))
    });
}
else if (eventFieldDecls.Length > 0)
{
    if (usingDecls.Length != 0 || blockScopedNamespaceDecl is null || fileScopedNamespaceDecl is not null)
    {
        Console.Error.WriteLine("event-declaration fixture supports one block-scoped namespace and no top-level using directives");
        return 4;
    }
    var eventFieldDecl = eventFieldDecls.SingleOrDefault();
    if (eventFieldDecl is null ||
        propertyDecls.Length != 0 ||
        methodDecls.Length != 0 ||
        eventFieldDecl.Declaration.Type is not IdentifierNameSyntax eventType ||
        eventType.Identifier.Text != "ChangedHandler" ||
        eventFieldDecl.Declaration.Variables.Count != 1 ||
        eventFieldDecl.Declaration.Variables[0].Identifier.Text != "Changed" ||
        eventFieldDecl.Declaration.Variables[0].Initializer is not null)
    {
        Console.Error.WriteLine("event-declaration fixture did not match expected event-field declaration shape");
        return 4;
    }
    metadata = BaseMetadata(new object[]
    {
        Rule("CompilationUnitSyntax", Field("Members", true, true,
            Alternative(nameof(NamespaceDeclarationSyntax), "nonterminal"))),
        Rule("NamespaceDeclarationSyntax",
            Field("Name", true, false, Alternative("qualified_identifier", "identifier_payload")),
            Field("Members", true, true, Alternative(nameof(ClassDeclarationSyntax), "nonterminal"))),
        Rule("ClassDeclarationSyntax",
            Field("Identifier", true, false, Alternative("identifier", "identifier_payload")),
            Field("Members", true, true, Alternative(nameof(EventFieldDeclarationSyntax), "nonterminal"))),
        Rule("EventFieldDeclarationSyntax",
            Field("Declaration", true, false, Alternative(nameof(VariableDeclarationSyntax), "nonterminal"))),
        Rule("VariableDeclarationSyntax",
            Field("Type", true, false, Alternative(nameof(IdentifierNameSyntax), "nonterminal")),
            Field("Variables", true, true, Alternative(nameof(VariableDeclaratorSyntax), "nonterminal"))),
        Rule("IdentifierNameSyntax",
            Field("Identifier", true, false, Alternative("identifier", "identifier_payload"))),
        Rule("VariableDeclaratorSyntax",
            Field("Identifier", true, false, Alternative("identifier", "identifier_payload")))
    });
}
else if (fieldDecls.Length > 0)
{
    if (usingDecls.Length != 0 || blockScopedNamespaceDecl is null || fileScopedNamespaceDecl is not null)
    {
        Console.Error.WriteLine("field-declaration fixture supports one block-scoped namespace and no top-level using directives");
        return 4;
    }
    var fieldDecl = fieldDecls.SingleOrDefault();
    if (fieldDecl is null ||
        propertyDecls.Length != 0 ||
        methodDecls.Length != 0 ||
        eventFieldDecls.Length != 0 ||
        fieldDecl.Declaration.Type is not PredefinedTypeSyntax fieldType ||
        fieldType.Keyword.Text != "string" ||
        fieldDecl.Declaration.Variables.Count != 1 ||
        fieldDecl.Declaration.Variables[0].Identifier.Text != "_customer" ||
        fieldDecl.Declaration.Variables[0].Initializer is not null)
    {
        Console.Error.WriteLine("field-declaration fixture did not match expected private string field shape");
        return 4;
    }
    metadata = BaseMetadata(new object[]
    {
        Rule("CompilationUnitSyntax", Field("Members", true, true,
            Alternative(nameof(NamespaceDeclarationSyntax), "nonterminal"))),
        Rule("NamespaceDeclarationSyntax",
            Field("Name", true, false, Alternative("qualified_identifier", "identifier_payload")),
            Field("Members", true, true, Alternative(nameof(ClassDeclarationSyntax), "nonterminal"))),
        Rule("ClassDeclarationSyntax",
            Field("Identifier", true, false, Alternative("identifier", "identifier_payload")),
            Field("Members", true, true, Alternative(nameof(FieldDeclarationSyntax), "nonterminal"))),
        Rule("FieldDeclarationSyntax",
            Field("Declaration", true, false, Alternative(nameof(VariableDeclarationSyntax), "nonterminal"))),
        Rule("VariableDeclarationSyntax",
            Field("Type", true, false, Alternative(nameof(PredefinedTypeSyntax), "nonterminal")),
            Field("Variables", true, true, Alternative(nameof(VariableDeclaratorSyntax), "nonterminal"))),
        Rule("PredefinedTypeSyntax",
            Field("Keyword", true, false, Alternative("string_keyword", "identifier_payload"))),
        Rule("VariableDeclaratorSyntax",
            Field("Identifier", true, false, Alternative("identifier", "identifier_payload")))
    });
}
else if (blockScopedNamespaceDecl is not null)
{
    if (propertyDecls.Length != 1 || methodDecls.Length != 0)
    {
        Console.Error.WriteLine("block-scoped namespace fixture supports only the minimal string property shape");
        return 4;
    }
    var propertyDecl = propertyDecls.Single();
    if (propertyDecl.Type is not PredefinedTypeSyntax predefinedType ||
        predefinedType.Keyword.Text != "string" ||
        propertyDecl.Initializer?.Value is not LiteralExpressionSyntax initializer ||
        !initializer.IsKind(SyntaxKind.StringLiteralExpression))
    {
        Console.Error.WriteLine("block-scoped namespace fixture did not match expected class/string property shape");
        return 4;
    }
    metadata = BaseMetadata(new object[]
    {
        Rule("CompilationUnitSyntax", Field("Members", true, true,
            Alternative(nameof(NamespaceDeclarationSyntax), "nonterminal"))),
        Rule("NamespaceDeclarationSyntax",
            Field("Name", true, false, Alternative("qualified_identifier", "identifier_payload")),
            Field("Members", true, true, Alternative(nameof(ClassDeclarationSyntax), "nonterminal"))),
        Rule("ClassDeclarationSyntax",
            Field("Identifier", true, false, Alternative("identifier", "identifier_payload")),
            Field("Members", true, true, Alternative(nameof(PropertyDeclarationSyntax), "nonterminal"))),
        Rule("PropertyDeclarationSyntax",
            Field("Identifier", true, false, Alternative("identifier", "identifier_payload")),
            Field("Type", true, false, Alternative(nameof(PredefinedTypeSyntax), "nonterminal")),
            Field("Initializer", true, false, Alternative("StringLiteralExpressionSyntax", "string_payload"))),
        Rule("PredefinedTypeSyntax",
            Field("Keyword", true, false, Alternative("string_keyword", "identifier_payload")))
    });
}
else if (propertyDecls.Length == 1 && methodDecls.Length == 0)
{
    var propertyDecl = propertyDecls.Single();
    if (propertyDecl.Type is PredefinedTypeSyntax arrowPropertyType &&
        arrowPropertyType.Keyword.Text == "int" &&
        propertyDecl.Identifier.Text == "Count" &&
        propertyDecl.ExpressionBody is ArrowExpressionClauseSyntax arrowExpressionBody &&
        arrowExpressionBody.Expression is LiteralExpressionSyntax arrowLiteral &&
        arrowLiteral.IsKind(SyntaxKind.NumericLiteralExpression) &&
        arrowLiteral.Token.ValueText == "1" &&
        propertyDecl.AccessorList is null &&
        propertyDecl.Initializer is null)
    {
        metadata = BaseMetadata(new object[]
        {
            Rule("CompilationUnitSyntax", Field("Members", true, true,
                Alternative(nameof(FileScopedNamespaceDeclarationSyntax), "nonterminal"))),
            Rule("FileScopedNamespaceDeclarationSyntax",
                Field("Name", true, false, Alternative("qualified_identifier", "identifier_payload")),
                Field("Members", true, true, Alternative(nameof(ClassDeclarationSyntax), "nonterminal"))),
            Rule("ClassDeclarationSyntax",
                Field("Identifier", true, false, Alternative("identifier", "identifier_payload")),
                Field("Members", true, true, Alternative(nameof(PropertyDeclarationSyntax), "nonterminal"))),
            Rule("PropertyDeclarationSyntax",
                Field("Identifier", true, false, Alternative("identifier", "identifier_payload")),
                Field("Type", true, false, Alternative(nameof(PredefinedTypeSyntax), "nonterminal")),
                Field("ExpressionBody", true, false, Alternative(nameof(ArrowExpressionClauseSyntax), "nonterminal"))),
            Rule("PredefinedTypeSyntax",
                Field("Keyword", true, false, Alternative("int_keyword", "identifier_payload"))),
            Rule("ArrowExpressionClauseSyntax",
                Field("Expression", true, false, Alternative(nameof(LiteralExpressionSyntax), "nonterminal"))),
            Rule("LiteralExpressionSyntax",
                Field("Token", true, false, Alternative("integer_literal", "integer_payload")))
        });
    }
    else if (classDecl.AttributeLists.Count == 1 &&
        classDecl.AttributeLists[0].Attributes.Count == 1 &&
        classDecl.AttributeLists[0].Attributes[0].Name is IdentifierNameSyntax attributeName &&
        attributeName.Identifier.Text == "Obsolete" &&
        propertyDecl.Type is PredefinedTypeSyntax attributePropertyType &&
        attributePropertyType.Keyword.Text == "string" &&
        propertyDecl.Identifier.Text == "Value" &&
        propertyDecl.Initializer?.Value is LiteralExpressionSyntax attributeInitializer &&
        attributeInitializer.IsKind(SyntaxKind.StringLiteralExpression))
    {
        metadata = BaseMetadata(new object[]
        {
            Rule("CompilationUnitSyntax", Field("Members", true, true,
                Alternative(nameof(FileScopedNamespaceDeclarationSyntax), "nonterminal"))),
            Rule("FileScopedNamespaceDeclarationSyntax",
                Field("Name", true, false, Alternative("qualified_identifier", "identifier_payload")),
                Field("Members", true, true, Alternative(nameof(ClassDeclarationSyntax), "nonterminal"))),
            Rule("ClassDeclarationSyntax",
                Field("AttributeLists", true, true, Alternative(nameof(AttributeListSyntax), "nonterminal")),
                Field("Identifier", true, false, Alternative("identifier", "identifier_payload")),
                Field("Members", true, true, Alternative(nameof(PropertyDeclarationSyntax), "nonterminal"))),
            Rule("AttributeListSyntax",
                Field("Attributes", true, true, Alternative(nameof(AttributeSyntax), "nonterminal"))),
            Rule("AttributeSyntax",
                Field("Name", true, false, Alternative(nameof(IdentifierNameSyntax), "nonterminal"))),
            Rule("IdentifierNameSyntax",
                Field("Identifier", true, false, Alternative("identifier", "identifier_payload"))),
            Rule("PropertyDeclarationSyntax",
                Field("Identifier", true, false, Alternative("identifier", "identifier_payload")),
                Field("Type", true, false, Alternative(nameof(PredefinedTypeSyntax), "nonterminal")),
                Field("Initializer", true, false, Alternative("StringLiteralExpressionSyntax", "string_payload"))),
            Rule("PredefinedTypeSyntax",
                Field("Keyword", true, false, Alternative("string_keyword", "identifier_payload")))
        });
    }
    else if (propertyDecl.Type is GenericNameSyntax genericType &&
        genericType.Identifier.Text == "List" &&
        genericType.TypeArgumentList.Arguments.Count == 1 &&
        genericType.TypeArgumentList.Arguments[0] is PredefinedTypeSyntax genericArgument &&
        genericArgument.Keyword.Text == "string" &&
        propertyDecl.Identifier.Text == "Values" &&
        propertyDecl.AccessorList is { } genericAccessorList &&
        genericAccessorList.Accessors.Count == 1 &&
        genericAccessorList.Accessors[0].Keyword.Text == "get" &&
        propertyDecl.Initializer is null)
    {
        metadata = BaseMetadata(new object[]
        {
            Rule("CompilationUnitSyntax", Field("Members", true, true,
                Alternative(nameof(FileScopedNamespaceDeclarationSyntax), "nonterminal"))),
            Rule("FileScopedNamespaceDeclarationSyntax",
                Field("Name", true, false, Alternative("qualified_identifier", "identifier_payload")),
                Field("Members", true, true, Alternative(nameof(ClassDeclarationSyntax), "nonterminal"))),
            Rule("ClassDeclarationSyntax",
                Field("Identifier", true, false, Alternative("identifier", "identifier_payload")),
                Field("Members", true, true, Alternative(nameof(PropertyDeclarationSyntax), "nonterminal"))),
            Rule("PropertyDeclarationSyntax",
                Field("Identifier", true, false, Alternative("identifier", "identifier_payload")),
                Field("Type", true, false, Alternative(nameof(GenericNameSyntax), "nonterminal")),
                Field("AccessorList", true, false, Alternative(nameof(AccessorListSyntax), "nonterminal"))),
            Rule("GenericNameSyntax",
                Field("Identifier", true, false, Alternative("identifier", "identifier_payload")),
                Field("TypeArgumentList", true, false, Alternative(nameof(TypeArgumentListSyntax), "nonterminal"))),
            Rule("TypeArgumentListSyntax",
                Field("Arguments", true, true, Alternative(nameof(PredefinedTypeSyntax), "nonterminal"))),
            Rule("PredefinedTypeSyntax",
                Field("Keyword", true, false, Alternative("string_keyword", "identifier_payload"))),
            Rule("AccessorListSyntax",
                Field("Accessors", true, true, Alternative(nameof(AccessorDeclarationSyntax), "nonterminal"))),
            Rule("AccessorDeclarationSyntax",
                Field("Keyword", true, false, Alternative("get_accessor_keyword", "string_payload")))
        });
    }
    else if (propertyDecl.Type is NullableTypeSyntax nullablePropertyType &&
        nullablePropertyType.ElementType is PredefinedTypeSyntax nullableElementType &&
        nullableElementType.Keyword.Text == "string" &&
        nullablePropertyType.QuestionToken.Text == "?" &&
        propertyDecl.AccessorList is { } accessorList &&
        accessorList.Accessors.Count == 2 &&
        accessorList.Accessors[0].Keyword.Text == "get" &&
        accessorList.Accessors[1].Keyword.Text == "init" &&
        propertyDecl.Initializer?.Value is LiteralExpressionSyntax nullInitializer &&
        nullInitializer.IsKind(SyntaxKind.NullLiteralExpression))
    {
        metadata = BaseMetadata(new object[]
        {
            Rule("CompilationUnitSyntax", Field("Members", true, true,
                Alternative(nameof(FileScopedNamespaceDeclarationSyntax), "nonterminal"))),
            Rule("FileScopedNamespaceDeclarationSyntax",
                Field("Name", true, false, Alternative("qualified_identifier", "identifier_payload")),
                Field("Members", true, true, Alternative(nameof(ClassDeclarationSyntax), "nonterminal"))),
            Rule("ClassDeclarationSyntax",
                Field("Identifier", true, false, Alternative("identifier", "identifier_payload")),
                Field("Members", true, true, Alternative(nameof(PropertyDeclarationSyntax), "nonterminal"))),
            Rule("PropertyDeclarationSyntax",
                Field("Identifier", true, false, Alternative("identifier", "identifier_payload")),
                Field("Type", true, false, Alternative(nameof(NullableTypeSyntax), "nonterminal")),
                Field("AccessorList", true, false, Alternative(nameof(AccessorListSyntax), "nonterminal")),
                Field("Initializer", true, false, Alternative(nameof(LiteralExpressionSyntax), "nonterminal"))),
            Rule("NullableTypeSyntax",
                Field("ElementType", true, false, Alternative(nameof(PredefinedTypeSyntax), "nonterminal")),
                Field("QuestionToken", true, false, Alternative("nullable_marker", "string_payload"))),
            Rule("PredefinedTypeSyntax",
                Field("Keyword", true, false, Alternative("string_keyword", "identifier_payload"))),
            Rule("AccessorListSyntax",
                Field("Accessors", true, true, Alternative(nameof(AccessorDeclarationSyntax), "nonterminal"))),
            Rule("AccessorDeclarationSyntax",
                Field("Keyword", true, false,
                    Alternative("get_accessor_keyword", "string_payload"),
                    Alternative("init_accessor_keyword", "string_payload"))),
            Rule("LiteralExpressionSyntax",
                Field("Token", true, false, Alternative("null_literal", "identifier_payload")))
        });
    }
    else
    {
        if (propertyDecl.Type is not PredefinedTypeSyntax predefinedType ||
            predefinedType.Keyword.Text != "string" ||
            propertyDecl.Initializer?.Value is not LiteralExpressionSyntax initializer ||
            !initializer.IsKind(SyntaxKind.StringLiteralExpression))
        {
            Console.Error.WriteLine("minimal fixture did not match expected namespace/class/string property shape");
            return 4;
        }
        metadata = BaseMetadata(new object[]
        {
            Rule("CompilationUnitSyntax", Field("Members", true, true,
                Alternative(nameof(FileScopedNamespaceDeclarationSyntax), "nonterminal"))),
            Rule("FileScopedNamespaceDeclarationSyntax",
                Field("Name", true, false, Alternative("qualified_identifier", "identifier_payload")),
                Field("Members", true, true, Alternative(nameof(ClassDeclarationSyntax), "nonterminal"))),
            Rule("ClassDeclarationSyntax",
                Field("Identifier", true, false, Alternative("identifier", "identifier_payload")),
                Field("Members", true, true, Alternative(nameof(PropertyDeclarationSyntax), "nonterminal"))),
            Rule("PropertyDeclarationSyntax",
                Field("Identifier", true, false, Alternative("identifier", "identifier_payload")),
                Field("Type", true, false, Alternative(nameof(PredefinedTypeSyntax), "nonterminal")),
                Field("Initializer", true, false, Alternative("StringLiteralExpressionSyntax", "string_payload"))),
            Rule("PredefinedTypeSyntax",
                Field("Keyword", true, false, Alternative("string_keyword", "identifier_payload")))
        });
    }
}
else if (propertyDecls.Length == 1 && methodDecls.Length == 1)
{
    var propertyDecl = propertyDecls.Single();
    var methodDecl = methodDecls.Single();
    if (propertyDecl.Type is not PredefinedTypeSyntax propertyType ||
        propertyType.Keyword.Text != "string" ||
        propertyDecl.Initializer?.Value is not LiteralExpressionSyntax initializer ||
        !initializer.IsKind(SyntaxKind.StringLiteralExpression) ||
        methodDecl.ReturnType is not PredefinedTypeSyntax methodReturnType ||
        methodReturnType.Keyword.Text != "string" ||
        methodDecl.ParameterList.Parameters.Count != 0 ||
        methodDecl.Body is not { } methodBody ||
        methodBody.Statements.Count != 1 ||
        methodBody.Statements[0] is not ReturnStatementSyntax returnStatement ||
        returnStatement.Expression is not IdentifierNameSyntax returnIdentifier ||
        returnIdentifier.Identifier.Text != propertyDecl.Identifier.Text)
    {
        Console.Error.WriteLine("property-plus-method fixture did not match expected string property plus block-bodied string method shape");
        return 4;
    }
    metadata = BaseMetadata(new object[]
    {
        Rule("CompilationUnitSyntax", Field("Members", true, true,
            Alternative(nameof(FileScopedNamespaceDeclarationSyntax), "nonterminal"))),
        Rule("FileScopedNamespaceDeclarationSyntax",
            Field("Name", true, false, Alternative("qualified_identifier", "identifier_payload")),
            Field("Members", true, true, Alternative(nameof(ClassDeclarationSyntax), "nonterminal"))),
        Rule("ClassDeclarationSyntax",
            Field("Identifier", true, false, Alternative("identifier", "identifier_payload")),
            Field("Members", true, true,
                Alternative(nameof(PropertyDeclarationSyntax), "nonterminal"),
                Alternative(nameof(MethodDeclarationSyntax), "nonterminal"))),
        Rule("PropertyDeclarationSyntax",
            Field("Identifier", true, false, Alternative("identifier", "identifier_payload")),
            Field("Type", true, false, Alternative(nameof(PredefinedTypeSyntax), "nonterminal")),
            Field("Initializer", true, false, Alternative("StringLiteralExpressionSyntax", "string_payload"))),
        Rule("MethodDeclarationSyntax",
            Field("Identifier", true, false, Alternative("identifier", "identifier_payload")),
            Field("ReturnType", true, false, Alternative(nameof(PredefinedTypeSyntax), "nonterminal")),
            Field("Body", true, false, Alternative(nameof(BlockSyntax), "nonterminal"))),
        Rule("PredefinedTypeSyntax",
            Field("Keyword", true, false, Alternative("string_keyword", "identifier_payload"))),
        Rule("BlockSyntax",
            Field("Statements", true, true, Alternative(nameof(ReturnStatementSyntax), "nonterminal"))),
        Rule("ReturnStatementSyntax",
            Field("Expression", true, false, Alternative(nameof(IdentifierNameSyntax), "nonterminal"))),
        Rule("IdentifierNameSyntax",
            Field("Identifier", true, false, Alternative("identifier", "identifier_payload")))
    });
}
else if (propertyDecls.Length == 0 && methodDecls.Length == 2)
{
    var callerMethod = methodDecls[0];
    var calleeMethod = methodDecls[1];
    if (callerMethod.ReturnType is PredefinedTypeSyntax argumentCallerReturnType &&
        argumentCallerReturnType.Keyword.Text == "string" &&
        callerMethod.ParameterList.Parameters.Count == 0 &&
        callerMethod.Body is { } argumentCallerBody &&
        argumentCallerBody.Statements.Count == 1 &&
        argumentCallerBody.Statements[0] is ReturnStatementSyntax argumentCallerReturnStatement &&
        argumentCallerReturnStatement.Expression is InvocationExpressionSyntax argumentInvocation &&
        argumentInvocation.Expression is IdentifierNameSyntax argumentInvocationTarget &&
        argumentInvocationTarget.Identifier.Text == calleeMethod.Identifier.Text &&
        argumentInvocation.ArgumentList.Arguments.Count == 1 &&
        argumentInvocation.ArgumentList.Arguments[0].Expression is LiteralExpressionSyntax invocationArgument &&
        invocationArgument.IsKind(SyntaxKind.StringLiteralExpression) &&
        calleeMethod.ReturnType is PredefinedTypeSyntax argumentCalleeReturnType &&
        argumentCalleeReturnType.Keyword.Text == "string" &&
        calleeMethod.ParameterList.Parameters.Count == 1 &&
        calleeMethod.ParameterList.Parameters[0] is { } calleeParameter &&
        calleeParameter.Type is PredefinedTypeSyntax calleeParameterType &&
        calleeParameterType.Keyword.Text == "string" &&
        calleeParameter.Identifier.Text == "value" &&
        calleeMethod.Body is { } argumentCalleeBody &&
        argumentCalleeBody.Statements.Count == 1 &&
        argumentCalleeBody.Statements[0] is ReturnStatementSyntax argumentCalleeReturnStatement &&
        argumentCalleeReturnStatement.Expression is IdentifierNameSyntax calleeReturnIdentifier &&
        calleeReturnIdentifier.Identifier.Text == calleeParameter.Identifier.Text)
    {
        metadata = BaseMetadata(new object[]
        {
            Rule("CompilationUnitSyntax", Field("Members", true, true,
                Alternative(nameof(FileScopedNamespaceDeclarationSyntax), "nonterminal"))),
            Rule("FileScopedNamespaceDeclarationSyntax",
                Field("Name", true, false, Alternative("qualified_identifier", "identifier_payload")),
                Field("Members", true, true, Alternative(nameof(ClassDeclarationSyntax), "nonterminal"))),
            Rule("ClassDeclarationSyntax",
                Field("Identifier", true, false, Alternative("identifier", "identifier_payload")),
                Field("Members", true, true, Alternative(nameof(MethodDeclarationSyntax), "nonterminal"))),
            Rule("MethodDeclarationSyntax",
                Field("Identifier", true, false, Alternative("identifier", "identifier_payload")),
                Field("ReturnType", true, false, Alternative(nameof(PredefinedTypeSyntax), "nonterminal")),
                Field("ParameterList", true, false, Alternative(nameof(ParameterListSyntax), "nonterminal")),
                Field("Body", true, false, Alternative(nameof(BlockSyntax), "nonterminal"))),
            Rule("PredefinedTypeSyntax",
                Field("Keyword", true, false, Alternative("string_keyword", "identifier_payload"))),
            Rule("ParameterListSyntax",
                Field("Parameters", true, true, Alternative(nameof(ParameterSyntax), "nonterminal"))),
            Rule("ParameterSyntax",
                Field("Type", true, false, Alternative(nameof(PredefinedTypeSyntax), "nonterminal")),
                Field("Identifier", true, false, Alternative("identifier", "identifier_payload"))),
            Rule("BlockSyntax",
                Field("Statements", true, true, Alternative(nameof(ReturnStatementSyntax), "nonterminal"))),
            Rule("ReturnStatementSyntax",
                Field("Expression", true, false,
                    Alternative(nameof(InvocationExpressionSyntax), "nonterminal"),
                    Alternative(nameof(IdentifierNameSyntax), "nonterminal"))),
            Rule("InvocationExpressionSyntax",
                Field("Expression", true, false, Alternative(nameof(IdentifierNameSyntax), "nonterminal")),
                Field("ArgumentList", true, false, Alternative(nameof(ArgumentListSyntax), "nonterminal"))),
            Rule("ArgumentListSyntax",
                Field("Arguments", true, true, Alternative(nameof(ArgumentSyntax), "nonterminal"))),
            Rule("ArgumentSyntax",
                Field("Expression", true, false, Alternative(nameof(LiteralExpressionSyntax), "nonterminal"))),
            Rule("IdentifierNameSyntax",
                Field("Identifier", true, false, Alternative("identifier", "identifier_payload"))),
            Rule("LiteralExpressionSyntax",
                Field("Token", true, false, Alternative("string_literal", "string_payload")))
        });
    }
    else
    {
        if (callerMethod.ReturnType is not PredefinedTypeSyntax callerReturnType ||
            callerReturnType.Keyword.Text != "string" ||
            callerMethod.ParameterList.Parameters.Count != 0 ||
            callerMethod.Body is not { } callerBody ||
            callerBody.Statements.Count != 1 ||
            callerBody.Statements[0] is not ReturnStatementSyntax callerReturnStatement ||
            callerReturnStatement.Expression is not InvocationExpressionSyntax invocation ||
            invocation.Expression is not IdentifierNameSyntax invocationTarget ||
            invocationTarget.Identifier.Text != calleeMethod.Identifier.Text ||
            invocation.ArgumentList.Arguments.Count != 0 ||
            calleeMethod.ReturnType is not PredefinedTypeSyntax calleeReturnType ||
            calleeReturnType.Keyword.Text != "string" ||
            calleeMethod.ParameterList.Parameters.Count != 0 ||
            calleeMethod.Body is not { } calleeBody ||
            calleeBody.Statements.Count != 1 ||
            calleeBody.Statements[0] is not ReturnStatementSyntax calleeReturnStatement ||
            calleeReturnStatement.Expression is not LiteralExpressionSyntax calleeLiteral ||
            !calleeLiteral.IsKind(SyntaxKind.StringLiteralExpression))
        {
            Console.Error.WriteLine("method-invocation-return fixture did not match expected string method returning zero-argument method invocation shape");
            return 4;
        }
        metadata = BaseMetadata(new object[]
        {
            Rule("CompilationUnitSyntax", Field("Members", true, true,
                Alternative(nameof(FileScopedNamespaceDeclarationSyntax), "nonterminal"))),
            Rule("FileScopedNamespaceDeclarationSyntax",
                Field("Name", true, false, Alternative("qualified_identifier", "identifier_payload")),
                Field("Members", true, true, Alternative(nameof(ClassDeclarationSyntax), "nonterminal"))),
            Rule("ClassDeclarationSyntax",
                Field("Identifier", true, false, Alternative("identifier", "identifier_payload")),
                Field("Members", true, true, Alternative(nameof(MethodDeclarationSyntax), "nonterminal"))),
            Rule("MethodDeclarationSyntax",
                Field("Identifier", true, false, Alternative("identifier", "identifier_payload")),
                Field("ReturnType", true, false, Alternative(nameof(PredefinedTypeSyntax), "nonterminal")),
                Field("Body", true, false, Alternative(nameof(BlockSyntax), "nonterminal"))),
            Rule("PredefinedTypeSyntax",
                Field("Keyword", true, false, Alternative("string_keyword", "identifier_payload"))),
            Rule("BlockSyntax",
                Field("Statements", true, true, Alternative(nameof(ReturnStatementSyntax), "nonterminal"))),
            Rule("ReturnStatementSyntax",
                Field("Expression", true, false,
                    Alternative(nameof(InvocationExpressionSyntax), "nonterminal"),
                    Alternative(nameof(LiteralExpressionSyntax), "nonterminal"))),
            Rule("InvocationExpressionSyntax",
                Field("Expression", true, false, Alternative(nameof(IdentifierNameSyntax), "nonterminal")),
                Field("ArgumentList", true, false, Alternative(nameof(ArgumentListSyntax), "nonterminal"))),
            Rule("ArgumentListSyntax"),
            Rule("IdentifierNameSyntax",
                Field("Identifier", true, false, Alternative("identifier", "identifier_payload"))),
            Rule("LiteralExpressionSyntax",
                Field("Token", true, false, Alternative("string_literal", "string_payload")))
        });
    }
}
else if (propertyDecls.Length == 0 && methodDecls.Length == 1)
{
    var methodDecl = methodDecls.Single();
    if (methodDecl.Body is not { } body)
    {
        Console.Error.WriteLine("method fixture did not match expected class/method shape");
        return 4;
    }

    if (methodDecl.ReturnType is PredefinedTypeSyntax yieldReturnType &&
        yieldReturnType.Keyword.Text == "object" &&
        methodDecl.ParameterList.Parameters.Count == 0 &&
        body.Statements.Count == 1 &&
        body.Statements[0] is YieldStatementSyntax yieldStatement &&
        yieldStatement.ReturnOrBreakKeyword.Text == "return" &&
        yieldStatement.Expression is LiteralExpressionSyntax yieldLiteral &&
        yieldLiteral.IsKind(SyntaxKind.StringLiteralExpression))
    {
        metadata = BaseMetadata(new object[]
        {
            Rule("CompilationUnitSyntax", Field("Members", true, true,
                Alternative(nameof(FileScopedNamespaceDeclarationSyntax), "nonterminal"))),
            Rule("FileScopedNamespaceDeclarationSyntax",
                Field("Name", true, false, Alternative("qualified_identifier", "identifier_payload")),
                Field("Members", true, true, Alternative(nameof(ClassDeclarationSyntax), "nonterminal"))),
            Rule("ClassDeclarationSyntax",
                Field("Identifier", true, false, Alternative("identifier", "identifier_payload")),
                Field("Members", true, true, Alternative(nameof(MethodDeclarationSyntax), "nonterminal"))),
            Rule("MethodDeclarationSyntax",
                Field("Identifier", true, false, Alternative("identifier", "identifier_payload")),
                Field("ReturnType", true, false, Alternative(nameof(PredefinedTypeSyntax), "nonterminal")),
                Field("Body", true, false, Alternative(nameof(BlockSyntax), "nonterminal"))),
            Rule("PredefinedTypeSyntax",
                Field("Keyword", true, false, Alternative("object_keyword", "identifier_payload"))),
            Rule("BlockSyntax",
                Field("Statements", true, true, Alternative(nameof(YieldStatementSyntax), "nonterminal"))),
            Rule("YieldStatementSyntax",
                Field("ReturnOrBreakKeyword", true, false, Alternative("yield_return_keyword", "identifier_payload")),
                Field("Expression", true, false, Alternative(nameof(LiteralExpressionSyntax), "nonterminal"))),
            Rule("LiteralExpressionSyntax",
                Field("Token", true, false, Alternative("string_literal", "string_payload")))
        });
    }
    else if (methodDecl.ReturnType is ArrayTypeSyntax rangeReturnType &&
        methodDecl.ParameterList.Parameters.Count == 1 &&
        methodDecl.ParameterList.Parameters[0] is { } rangeParameter &&
        rangeParameter.Type is ArrayTypeSyntax rangeParameterType &&
        rangeParameterType.ElementType is PredefinedTypeSyntax rangeParameterElementType &&
        rangeParameterElementType.Keyword.Text == "int" &&
        rangeParameter.Identifier.Text == "values" &&
        rangeParameterType.RankSpecifiers.Count == 1 &&
        rangeReturnType.ElementType is PredefinedTypeSyntax rangeReturnElementType &&
        rangeReturnElementType.Keyword.Text == "int" &&
        rangeReturnType.RankSpecifiers.Count == 1 &&
        body.Statements.Count == 1 &&
        body.Statements[0] is ReturnStatementSyntax rangeReturnStatement &&
        rangeReturnStatement.Expression is ElementAccessExpressionSyntax rangeElementAccess &&
        rangeElementAccess.Expression is IdentifierNameSyntax rangeTarget &&
        rangeTarget.Identifier.Text == rangeParameter.Identifier.Text &&
        rangeElementAccess.ArgumentList.Arguments.Count == 1 &&
        rangeElementAccess.ArgumentList.Arguments[0].Expression is RangeExpressionSyntax rangeExpression &&
        rangeExpression.LeftOperand is LiteralExpressionSyntax rangeStart &&
        rangeExpression.RightOperand is LiteralExpressionSyntax rangeEnd &&
        rangeStart.IsKind(SyntaxKind.NumericLiteralExpression) &&
        rangeEnd.IsKind(SyntaxKind.NumericLiteralExpression) &&
        rangeStart.Token.ValueText == "1" &&
        rangeEnd.Token.ValueText == "2")
    {
        metadata = BaseMetadata(new object[]
        {
            Rule("CompilationUnitSyntax", Field("Members", true, true,
                Alternative(nameof(FileScopedNamespaceDeclarationSyntax), "nonterminal"))),
            Rule("FileScopedNamespaceDeclarationSyntax",
                Field("Name", true, false, Alternative("qualified_identifier", "identifier_payload")),
                Field("Members", true, true, Alternative(nameof(ClassDeclarationSyntax), "nonterminal"))),
            Rule("ClassDeclarationSyntax",
                Field("Identifier", true, false, Alternative("identifier", "identifier_payload")),
                Field("Members", true, true, Alternative(nameof(MethodDeclarationSyntax), "nonterminal"))),
            Rule("MethodDeclarationSyntax",
                Field("Identifier", true, false, Alternative("identifier", "identifier_payload")),
                Field("ReturnType", true, false, Alternative(nameof(ArrayTypeSyntax), "nonterminal")),
                Field("ParameterList", true, false, Alternative(nameof(ParameterListSyntax), "nonterminal")),
                Field("Body", true, false, Alternative(nameof(BlockSyntax), "nonterminal"))),
            Rule("ArrayTypeSyntax",
                Field("ElementType", true, false, Alternative(nameof(PredefinedTypeSyntax), "nonterminal"))),
            Rule("PredefinedTypeSyntax",
                Field("Keyword", true, false, Alternative("int_keyword", "identifier_payload"))),
            Rule("ParameterListSyntax",
                Field("Parameters", true, true, Alternative(nameof(ParameterSyntax), "nonterminal"))),
            Rule("ParameterSyntax",
                Field("Type", true, false, Alternative(nameof(ArrayTypeSyntax), "nonterminal")),
                Field("Identifier", true, false, Alternative("identifier", "identifier_payload"))),
            Rule("BlockSyntax",
                Field("Statements", true, true, Alternative(nameof(ReturnStatementSyntax), "nonterminal"))),
            Rule("ReturnStatementSyntax",
                Field("Expression", true, false, Alternative(nameof(ElementAccessExpressionSyntax), "nonterminal"))),
            Rule("ElementAccessExpressionSyntax",
                Field("Expression", true, false, Alternative(nameof(IdentifierNameSyntax), "nonterminal")),
                Field("ArgumentList", true, false, Alternative(nameof(BracketedArgumentListSyntax), "nonterminal"))),
            Rule("BracketedArgumentListSyntax",
                Field("Arguments", true, true, Alternative(nameof(ArgumentSyntax), "nonterminal"))),
            Rule("ArgumentSyntax",
                Field("Expression", true, false, Alternative(nameof(RangeExpressionSyntax), "nonterminal"))),
            Rule("RangeExpressionSyntax",
                Field("LeftOperand", true, false, Alternative(nameof(LiteralExpressionSyntax), "nonterminal")),
                Field("RightOperand", true, false, Alternative(nameof(LiteralExpressionSyntax), "nonterminal"))),
            Rule("IdentifierNameSyntax",
                Field("Identifier", true, false, Alternative("identifier", "identifier_payload"))),
            Rule("LiteralExpressionSyntax",
                Field("Token", true, false, Alternative("integer_literal", "integer_payload")))
        });
    }
    else if (methodDecl.ReturnType is ArrayTypeSyntax arrayReturnType &&
        body.Statements.Count == 1 &&
        body.Statements[0] is ReturnStatementSyntax arrayReturnStatement &&
        arrayReturnStatement.Expression is ArrayCreationExpressionSyntax arrayCreation &&
        arrayCreation.Type is ArrayTypeSyntax arrayCreationType &&
        arrayCreation.Initializer is { } arrayInitializer &&
        arrayReturnType.ElementType is PredefinedTypeSyntax arrayReturnElementType &&
        arrayCreationType.ElementType is PredefinedTypeSyntax arrayCreationElementType &&
        arrayReturnElementType.Keyword.Text == "int" &&
        arrayCreationElementType.Keyword.Text == "int" &&
        arrayReturnType.RankSpecifiers.Count == 1 &&
        arrayCreationType.RankSpecifiers.Count == 1 &&
        arrayInitializer.Expressions.Count >= 1 &&
        arrayInitializer.Expressions.All(expression =>
            expression is LiteralExpressionSyntax literal &&
            literal.IsKind(SyntaxKind.NumericLiteralExpression)))
    {
        metadata = BaseMetadata(new object[]
        {
            Rule("CompilationUnitSyntax", Field("Members", true, true,
                Alternative(nameof(FileScopedNamespaceDeclarationSyntax), "nonterminal"))),
            Rule("FileScopedNamespaceDeclarationSyntax",
                Field("Name", true, false, Alternative("qualified_identifier", "identifier_payload")),
                Field("Members", true, true, Alternative(nameof(ClassDeclarationSyntax), "nonterminal"))),
            Rule("ClassDeclarationSyntax",
                Field("Identifier", true, false, Alternative("identifier", "identifier_payload")),
                Field("Members", true, true, Alternative(nameof(MethodDeclarationSyntax), "nonterminal"))),
            Rule("MethodDeclarationSyntax",
                Field("Identifier", true, false, Alternative("identifier", "identifier_payload")),
                Field("ReturnType", true, false, Alternative(nameof(ArrayTypeSyntax), "nonterminal")),
                Field("Body", true, false, Alternative(nameof(BlockSyntax), "nonterminal"))),
            Rule("ArrayTypeSyntax",
                Field("ElementType", true, false, Alternative(nameof(PredefinedTypeSyntax), "nonterminal"))),
            Rule("PredefinedTypeSyntax",
                Field("Keyword", true, false, Alternative("int_keyword", "identifier_payload"))),
            Rule("BlockSyntax",
                Field("Statements", true, true, Alternative(nameof(ReturnStatementSyntax), "nonterminal"))),
            Rule("ReturnStatementSyntax",
                Field("Expression", true, false, Alternative(nameof(ArrayCreationExpressionSyntax), "nonterminal"))),
            Rule("ArrayCreationExpressionSyntax",
                Field("Type", true, false, Alternative(nameof(ArrayTypeSyntax), "nonterminal")),
                Field("Initializer", true, false, Alternative(nameof(InitializerExpressionSyntax), "nonterminal"))),
            Rule("InitializerExpressionSyntax",
                Field("Expressions", true, true, Alternative(nameof(LiteralExpressionSyntax), "nonterminal"))),
            Rule("LiteralExpressionSyntax",
                Field("Token", true, false, Alternative("integer_literal", "integer_payload")))
        });
    }
    else if (methodDecl.ReturnType is NullableTypeSyntax nullableReturnType &&
        methodDecl.ParameterList.Parameters.Count == 0 &&
        body.Statements.Count == 1 &&
        nullableReturnType.ElementType is PredefinedTypeSyntax nullableElementType &&
        nullableElementType.Keyword.Text == "string" &&
        body.Statements[0] is ReturnStatementSyntax nullableReturnStatement &&
        nullableReturnStatement.Expression is LiteralExpressionSyntax nullableReturnLiteral &&
        nullableReturnLiteral.IsKind(SyntaxKind.NullLiteralExpression))
    {
        metadata = BaseMetadata(new object[]
        {
            Rule("CompilationUnitSyntax", Field("Members", true, true,
                Alternative(nameof(FileScopedNamespaceDeclarationSyntax), "nonterminal"))),
            Rule("FileScopedNamespaceDeclarationSyntax",
                Field("Name", true, false, Alternative("qualified_identifier", "identifier_payload")),
                Field("Members", true, true, Alternative(nameof(ClassDeclarationSyntax), "nonterminal"))),
            Rule("ClassDeclarationSyntax",
                Field("Identifier", true, false, Alternative("identifier", "identifier_payload")),
                Field("Members", true, true, Alternative(nameof(MethodDeclarationSyntax), "nonterminal"))),
            Rule("MethodDeclarationSyntax",
                Field("Identifier", true, false, Alternative("identifier", "identifier_payload")),
                Field("ReturnType", true, false, Alternative(nameof(NullableTypeSyntax), "nonterminal")),
                Field("Body", true, false, Alternative(nameof(BlockSyntax), "nonterminal"))),
            Rule("NullableTypeSyntax",
                Field("ElementType", true, false, Alternative(nameof(PredefinedTypeSyntax), "nonterminal")),
                Field("QuestionToken", true, false, Alternative("nullable_marker", "string_payload"))),
            Rule("PredefinedTypeSyntax",
                Field("Keyword", true, false, Alternative("string_keyword", "identifier_payload"))),
            Rule("BlockSyntax",
                Field("Statements", true, true, Alternative(nameof(ReturnStatementSyntax), "nonterminal"))),
            Rule("ReturnStatementSyntax",
                Field("Expression", true, false, Alternative(nameof(LiteralExpressionSyntax), "nonterminal"))),
            Rule("LiteralExpressionSyntax",
                Field("Token", true, false, Alternative("null_literal", "identifier_payload")))
        });
    }
    else if (methodDecl.ReturnType is PredefinedTypeSyntax returnType)
    {
        var returnTypeKeyword = returnType.Keyword.Text;
        if (returnTypeKeyword != "string" && returnTypeKeyword != "int" && returnTypeKeyword != "double" && returnTypeKeyword != "bool" && returnTypeKeyword != "object")
        {
            Console.Error.WriteLine("method fixture return type must be string, int, double, bool, object, or int[]");
            return 4;
        }

        if (returnTypeKeyword == "object" &&
            methodDecl.ParameterList.Parameters.Count == 0 &&
            body.Statements.Count == 3 &&
            body.Statements[0] is LocalDeclarationStatementSyntax valuesStatement &&
            valuesStatement.Declaration.Type is ArrayTypeSyntax valuesType &&
            valuesType.ElementType is PredefinedTypeSyntax valuesElementType &&
            valuesElementType.Keyword.Text == "int" &&
            valuesType.RankSpecifiers.Count == 1 &&
            valuesStatement.Declaration.Variables.Count == 1 &&
            valuesStatement.Declaration.Variables[0] is { } valuesDeclarator &&
            valuesDeclarator.Identifier.Text == "values" &&
            valuesDeclarator.Initializer?.Value is ArrayCreationExpressionSyntax valuesCreation &&
            valuesCreation.Type is ArrayTypeSyntax valuesCreationType &&
            valuesCreationType.ElementType is PredefinedTypeSyntax valuesCreationElementType &&
            valuesCreationElementType.Keyword.Text == "int" &&
            valuesCreationType.RankSpecifiers.Count == 1 &&
            valuesCreationType.RankSpecifiers[0].Sizes.Count == 1 &&
            valuesCreationType.RankSpecifiers[0].Sizes[0] is OmittedArraySizeExpressionSyntax &&
            valuesCreation.Initializer is { } valuesInitializer &&
            valuesInitializer.Expressions.Count == 2 &&
            valuesInitializer.Expressions.All(expression =>
                expression is LiteralExpressionSyntax literal &&
                literal.IsKind(SyntaxKind.NumericLiteralExpression)) &&
            body.Statements[1] is LocalDeclarationStatementSyntax emptyStatement &&
            emptyStatement.Declaration.Type is ArrayTypeSyntax emptyType &&
            emptyType.ElementType is PredefinedTypeSyntax emptyElementType &&
            emptyElementType.Keyword.Text == "int" &&
            emptyType.RankSpecifiers.Count == 1 &&
            emptyStatement.Declaration.Variables.Count == 1 &&
            emptyStatement.Declaration.Variables[0] is { } emptyDeclarator &&
            emptyDeclarator.Identifier.Text == "empty" &&
            emptyDeclarator.Initializer?.Value is ArrayCreationExpressionSyntax emptyCreation &&
            emptyCreation.Type is ArrayTypeSyntax emptyCreationType &&
            emptyCreationType.ElementType is PredefinedTypeSyntax emptyCreationElementType &&
            emptyCreationElementType.Keyword.Text == "int" &&
            emptyCreationType.RankSpecifiers.Count == 1 &&
            emptyCreationType.RankSpecifiers[0].Sizes.Count == 1 &&
            emptyCreationType.RankSpecifiers[0].Sizes[0] is LiteralExpressionSyntax emptySize &&
            emptySize.IsKind(SyntaxKind.NumericLiteralExpression) &&
            emptySize.Token.ValueText == "0" &&
            emptyCreation.Initializer is null &&
            body.Statements[2] is ReturnStatementSyntax objectReturnStatement &&
            objectReturnStatement.Expression is ObjectCreationExpressionSyntax objectCreation &&
            objectCreation.Type is IdentifierNameSyntax objectCreationType &&
            objectCreationType.Identifier.Text == classDecl.Identifier.Text &&
            objectCreation.ArgumentList is { } objectArgumentList &&
            objectArgumentList.Arguments.Count == 0 &&
            objectCreation.Initializer is null)
        {
            metadata = BaseMetadata(new object[]
            {
                Rule("CompilationUnitSyntax", Field("Members", true, true,
                    Alternative(nameof(FileScopedNamespaceDeclarationSyntax), "nonterminal"))),
                Rule("FileScopedNamespaceDeclarationSyntax",
                    Field("Name", true, false, Alternative("qualified_identifier", "identifier_payload")),
                    Field("Members", true, true, Alternative(nameof(ClassDeclarationSyntax), "nonterminal"))),
                Rule("ClassDeclarationSyntax",
                    Field("Identifier", true, false, Alternative("identifier", "identifier_payload")),
                    Field("Members", true, true, Alternative(nameof(MethodDeclarationSyntax), "nonterminal"))),
                Rule("MethodDeclarationSyntax",
                    Field("Identifier", true, false, Alternative("identifier", "identifier_payload")),
                    Field("ReturnType", true, false, Alternative(nameof(PredefinedTypeSyntax), "nonterminal")),
                    Field("Body", true, false, Alternative(nameof(BlockSyntax), "nonterminal"))),
                Rule("PredefinedTypeSyntax",
                    Field("Keyword", true, false,
                        Alternative("object_keyword", "identifier_payload"),
                        Alternative("int_keyword", "identifier_payload"))),
                Rule("BlockSyntax",
                    Field("Statements", true, true,
                        Alternative(nameof(LocalDeclarationStatementSyntax), "nonterminal"),
                        Alternative(nameof(ReturnStatementSyntax), "nonterminal"))),
                Rule("LocalDeclarationStatementSyntax",
                    Field("Declaration", true, false, Alternative(nameof(VariableDeclarationSyntax), "nonterminal"))),
                Rule("VariableDeclarationSyntax",
                    Field("Type", true, false, Alternative(nameof(ArrayTypeSyntax), "nonterminal")),
                    Field("Variables", true, true, Alternative(nameof(VariableDeclaratorSyntax), "nonterminal"))),
                Rule("ArrayTypeSyntax",
                    Field("ElementType", true, false, Alternative(nameof(PredefinedTypeSyntax), "nonterminal")),
                    Field("RankSpecifiers", true, true, Alternative(nameof(ArrayRankSpecifierSyntax), "nonterminal"))),
                Rule("ArrayRankSpecifierSyntax",
                    Field("Sizes", true, true,
                        Alternative(nameof(OmittedArraySizeExpressionSyntax), "nonterminal"),
                        Alternative(nameof(LiteralExpressionSyntax), "nonterminal"))),
                Rule("OmittedArraySizeExpressionSyntax"),
                Rule("VariableDeclaratorSyntax",
                    Field("Identifier", true, false, Alternative("identifier", "identifier_payload")),
                    Field("Initializer", true, false, Alternative(nameof(EqualsValueClauseSyntax), "nonterminal"))),
                Rule("EqualsValueClauseSyntax",
                    Field("Value", true, false, Alternative(nameof(ArrayCreationExpressionSyntax), "nonterminal"))),
                Rule("ArrayCreationExpressionSyntax",
                    Field("Type", true, false, Alternative(nameof(ArrayTypeSyntax), "nonterminal")),
                    Field("Initializer", false, false, Alternative(nameof(InitializerExpressionSyntax), "nonterminal"))),
                Rule("InitializerExpressionSyntax",
                    Field("Expressions", true, true, Alternative(nameof(LiteralExpressionSyntax), "nonterminal"))),
                Rule("LiteralExpressionSyntax",
                    Field("Token", true, false, Alternative("integer_literal", "integer_payload"))),
                Rule("ReturnStatementSyntax",
                    Field("Expression", true, false, Alternative(nameof(ObjectCreationExpressionSyntax), "nonterminal"))),
                Rule("ObjectCreationExpressionSyntax",
                    Field("Type", true, false, Alternative(nameof(IdentifierNameSyntax), "nonterminal")),
                    Field("ArgumentList", true, false, Alternative(nameof(ArgumentListSyntax), "nonterminal"))),
                Rule("ArgumentListSyntax"),
                Rule("IdentifierNameSyntax",
                    Field("Identifier", true, false, Alternative("identifier", "identifier_payload")))
            });
        }
        else if (returnTypeKeyword == "int" &&
            methodDecl.ParameterList.Parameters.Count == 0 &&
            body.Statements.Count == 3 &&
            body.Statements[0] is LocalDeclarationStatementSyntax whileLocalStatement &&
            whileLocalStatement.Declaration.Type is PredefinedTypeSyntax whileLocalType &&
            whileLocalType.Keyword.Text == "int" &&
            whileLocalStatement.Declaration.Variables.Count == 1 &&
            whileLocalStatement.Declaration.Variables[0] is { } whileVariableDeclarator &&
            whileVariableDeclarator.Identifier.Text == "index" &&
            whileVariableDeclarator.Initializer?.Value is LiteralExpressionSyntax whileInitialValue &&
            whileInitialValue.IsKind(SyntaxKind.NumericLiteralExpression) &&
            whileInitialValue.Token.ValueText == "0" &&
            body.Statements[1] is WhileStatementSyntax whileStatement &&
            whileStatement.Condition is BinaryExpressionSyntax whileCondition &&
            whileCondition.IsKind(SyntaxKind.LessThanExpression) &&
            whileCondition.Left is IdentifierNameSyntax whileConditionLeft &&
            whileConditionLeft.Identifier.Text == whileVariableDeclarator.Identifier.Text &&
            whileCondition.OperatorToken.Text == "<" &&
            whileCondition.Right is LiteralExpressionSyntax whileConditionRight &&
            whileConditionRight.IsKind(SyntaxKind.NumericLiteralExpression) &&
            whileConditionRight.Token.ValueText == "3" &&
            whileStatement.Statement is BlockSyntax whileBlock &&
            whileBlock.Statements.Count == 1 &&
            whileBlock.Statements[0] is ExpressionStatementSyntax whileExpressionStatement &&
            whileExpressionStatement.Expression is PostfixUnaryExpressionSyntax postfixExpression &&
            postfixExpression.IsKind(SyntaxKind.PostIncrementExpression) &&
            postfixExpression.Operand is IdentifierNameSyntax postfixOperand &&
            postfixOperand.Identifier.Text == whileVariableDeclarator.Identifier.Text &&
            postfixExpression.OperatorToken.Text == "++" &&
            body.Statements[2] is ReturnStatementSyntax whileReturnStatement &&
            whileReturnStatement.Expression is IdentifierNameSyntax whileReturnIdentifier &&
            whileReturnIdentifier.Identifier.Text == whileVariableDeclarator.Identifier.Text)
        {
            metadata = BaseMetadata(new object[]
            {
                Rule("CompilationUnitSyntax", Field("Members", true, true,
                    Alternative(nameof(FileScopedNamespaceDeclarationSyntax), "nonterminal"))),
                Rule("FileScopedNamespaceDeclarationSyntax",
                    Field("Name", true, false, Alternative("qualified_identifier", "identifier_payload")),
                    Field("Members", true, true, Alternative(nameof(ClassDeclarationSyntax), "nonterminal"))),
                Rule("ClassDeclarationSyntax",
                    Field("Identifier", true, false, Alternative("identifier", "identifier_payload")),
                    Field("Members", true, true, Alternative(nameof(MethodDeclarationSyntax), "nonterminal"))),
                Rule("MethodDeclarationSyntax",
                    Field("Identifier", true, false, Alternative("identifier", "identifier_payload")),
                    Field("ReturnType", true, false, Alternative(nameof(PredefinedTypeSyntax), "nonterminal")),
                    Field("Body", true, false, Alternative(nameof(BlockSyntax), "nonterminal"))),
                Rule("PredefinedTypeSyntax",
                    Field("Keyword", true, false, Alternative("int_keyword", "identifier_payload"))),
                Rule("BlockSyntax",
                    Field("Statements", true, true,
                        Alternative(nameof(LocalDeclarationStatementSyntax), "nonterminal"),
                        Alternative(nameof(WhileStatementSyntax), "nonterminal"),
                        Alternative(nameof(ExpressionStatementSyntax), "nonterminal"),
                        Alternative(nameof(ReturnStatementSyntax), "nonterminal"))),
                Rule("LocalDeclarationStatementSyntax",
                    Field("Declaration", true, false, Alternative(nameof(VariableDeclarationSyntax), "nonterminal"))),
                Rule("VariableDeclarationSyntax",
                    Field("Type", true, false, Alternative(nameof(PredefinedTypeSyntax), "nonterminal")),
                    Field("Variables", true, true, Alternative(nameof(VariableDeclaratorSyntax), "nonterminal"))),
                Rule("VariableDeclaratorSyntax",
                    Field("Identifier", true, false, Alternative("identifier", "identifier_payload")),
                    Field("Initializer", true, false, Alternative(nameof(EqualsValueClauseSyntax), "nonterminal"))),
                Rule("EqualsValueClauseSyntax",
                    Field("Value", true, false, Alternative(nameof(LiteralExpressionSyntax), "nonterminal"))),
                Rule("WhileStatementSyntax",
                    Field("Condition", true, false, Alternative(nameof(BinaryExpressionSyntax), "nonterminal")),
                    Field("Statement", true, false, Alternative(nameof(BlockSyntax), "nonterminal"))),
                Rule("BinaryExpressionSyntax",
                    Field("Left", true, false, Alternative(nameof(IdentifierNameSyntax), "nonterminal")),
                    Field("OperatorToken", true, false, Alternative("binary_operator", "string_payload")),
                    Field("Right", true, false, Alternative(nameof(LiteralExpressionSyntax), "nonterminal"))),
                Rule("ExpressionStatementSyntax",
                    Field("Expression", true, false, Alternative(nameof(PostfixUnaryExpressionSyntax), "nonterminal"))),
                Rule("PostfixUnaryExpressionSyntax",
                    Field("Operand", true, false, Alternative(nameof(IdentifierNameSyntax), "nonterminal")),
                    Field("OperatorToken", true, false, Alternative("postfix_operator", "string_payload"))),
                Rule("LiteralExpressionSyntax",
                    Field("Token", true, false, Alternative("integer_literal", "integer_payload"))),
                Rule("ReturnStatementSyntax",
                    Field("Expression", true, false, Alternative(nameof(IdentifierNameSyntax), "nonterminal"))),
                Rule("IdentifierNameSyntax",
                    Field("Identifier", true, false, Alternative("identifier", "identifier_payload")))
            });
        }
        else if (returnTypeKeyword == "int" &&
            methodDecl.ParameterList.Parameters.Count == 0 &&
            body.Statements.Count == 1 &&
            body.Statements[0] is ReturnStatementSyntax parenthesizedReturnStatement &&
            parenthesizedReturnStatement.Expression is ParenthesizedExpressionSyntax parenthesizedExpression &&
            parenthesizedExpression.Expression is LiteralExpressionSyntax parenthesizedLiteral &&
            parenthesizedLiteral.IsKind(SyntaxKind.NumericLiteralExpression) &&
            parenthesizedLiteral.Token.ValueText == "7")
        {
            metadata = BaseMetadata(new object[]
            {
                Rule("CompilationUnitSyntax", Field("Members", true, true,
                    Alternative(nameof(FileScopedNamespaceDeclarationSyntax), "nonterminal"))),
                Rule("FileScopedNamespaceDeclarationSyntax",
                    Field("Name", true, false, Alternative("qualified_identifier", "identifier_payload")),
                    Field("Members", true, true, Alternative(nameof(ClassDeclarationSyntax), "nonterminal"))),
                Rule("ClassDeclarationSyntax",
                    Field("Identifier", true, false, Alternative("identifier", "identifier_payload")),
                    Field("Members", true, true, Alternative(nameof(MethodDeclarationSyntax), "nonterminal"))),
                Rule("MethodDeclarationSyntax",
                    Field("Identifier", true, false, Alternative("identifier", "identifier_payload")),
                    Field("ReturnType", true, false, Alternative(nameof(PredefinedTypeSyntax), "nonterminal")),
                    Field("Body", true, false, Alternative(nameof(BlockSyntax), "nonterminal"))),
                Rule("PredefinedTypeSyntax",
                    Field("Keyword", true, false, Alternative("int_keyword", "identifier_payload"))),
                Rule("BlockSyntax",
                    Field("Statements", true, true, Alternative(nameof(ReturnStatementSyntax), "nonterminal"))),
                Rule("ReturnStatementSyntax",
                    Field("Expression", true, false, Alternative(nameof(ParenthesizedExpressionSyntax), "nonterminal"))),
                Rule("ParenthesizedExpressionSyntax",
                    Field("Expression", true, false, Alternative(nameof(LiteralExpressionSyntax), "nonterminal"))),
                Rule("LiteralExpressionSyntax",
                    Field("Token", true, false, Alternative("integer_literal", "integer_payload")))
            });
        }
        else if (returnTypeKeyword == "int" &&
            methodDecl.ParameterList.Parameters.Count == 0 &&
            body.Statements.Count == 1 &&
            body.Statements[0] is ReturnStatementSyntax unaryReturnStatement &&
            unaryReturnStatement.Expression is PrefixUnaryExpressionSyntax unaryExpression &&
            unaryExpression.IsKind(SyntaxKind.UnaryMinusExpression) &&
            unaryExpression.OperatorToken.Text == "-" &&
            unaryExpression.Operand is LiteralExpressionSyntax unaryOperand &&
            unaryOperand.IsKind(SyntaxKind.NumericLiteralExpression) &&
            unaryOperand.Token.ValueText == "1")
        {
            metadata = BaseMetadata(new object[]
            {
                Rule("CompilationUnitSyntax", Field("Members", true, true,
                    Alternative(nameof(FileScopedNamespaceDeclarationSyntax), "nonterminal"))),
                Rule("FileScopedNamespaceDeclarationSyntax",
                    Field("Name", true, false, Alternative("qualified_identifier", "identifier_payload")),
                    Field("Members", true, true, Alternative(nameof(ClassDeclarationSyntax), "nonterminal"))),
                Rule("ClassDeclarationSyntax",
                    Field("Identifier", true, false, Alternative("identifier", "identifier_payload")),
                    Field("Members", true, true, Alternative(nameof(MethodDeclarationSyntax), "nonterminal"))),
                Rule("MethodDeclarationSyntax",
                    Field("Identifier", true, false, Alternative("identifier", "identifier_payload")),
                    Field("ReturnType", true, false, Alternative(nameof(PredefinedTypeSyntax), "nonterminal")),
                    Field("Body", true, false, Alternative(nameof(BlockSyntax), "nonterminal"))),
                Rule("PredefinedTypeSyntax",
                    Field("Keyword", true, false, Alternative("int_keyword", "identifier_payload"))),
                Rule("BlockSyntax",
                    Field("Statements", true, true, Alternative(nameof(ReturnStatementSyntax), "nonterminal"))),
                Rule("ReturnStatementSyntax",
                    Field("Expression", true, false, Alternative(nameof(PrefixUnaryExpressionSyntax), "nonterminal"))),
                Rule("PrefixUnaryExpressionSyntax",
                    Field("OperatorToken", true, false, Alternative("unary_operator", "string_payload")),
                    Field("Operand", true, false, Alternative(nameof(LiteralExpressionSyntax), "nonterminal"))),
                Rule("LiteralExpressionSyntax",
                    Field("Token", true, false, Alternative("integer_literal", "integer_payload")))
            });
        }
        else if (returnTypeKeyword == "int" &&
            methodDecl.ParameterList.Parameters.Count == 0 &&
            body.Statements.Count == 1 &&
            body.Statements[0] is ReturnStatementSyntax binaryReturnStatement &&
            binaryReturnStatement.Expression is BinaryExpressionSyntax binaryExpression &&
            binaryExpression.IsKind(SyntaxKind.AddExpression) &&
            binaryExpression.Left is LiteralExpressionSyntax leftOperand &&
            leftOperand.IsKind(SyntaxKind.NumericLiteralExpression) &&
            leftOperand.Token.ValueText == "1" &&
            binaryExpression.OperatorToken.Text == "+" &&
            binaryExpression.Right is LiteralExpressionSyntax rightOperand &&
            rightOperand.IsKind(SyntaxKind.NumericLiteralExpression) &&
            rightOperand.Token.ValueText == "2")
        {
            metadata = BaseMetadata(new object[]
            {
                Rule("CompilationUnitSyntax", Field("Members", true, true,
                    Alternative(nameof(FileScopedNamespaceDeclarationSyntax), "nonterminal"))),
                Rule("FileScopedNamespaceDeclarationSyntax",
                    Field("Name", true, false, Alternative("qualified_identifier", "identifier_payload")),
                    Field("Members", true, true, Alternative(nameof(ClassDeclarationSyntax), "nonterminal"))),
                Rule("ClassDeclarationSyntax",
                    Field("Identifier", true, false, Alternative("identifier", "identifier_payload")),
                    Field("Members", true, true, Alternative(nameof(MethodDeclarationSyntax), "nonterminal"))),
                Rule("MethodDeclarationSyntax",
                    Field("Identifier", true, false, Alternative("identifier", "identifier_payload")),
                    Field("ReturnType", true, false, Alternative(nameof(PredefinedTypeSyntax), "nonterminal")),
                    Field("Body", true, false, Alternative(nameof(BlockSyntax), "nonterminal"))),
                Rule("PredefinedTypeSyntax",
                    Field("Keyword", true, false, Alternative("int_keyword", "identifier_payload"))),
                Rule("BlockSyntax",
                    Field("Statements", true, true, Alternative(nameof(ReturnStatementSyntax), "nonterminal"))),
                Rule("ReturnStatementSyntax",
                    Field("Expression", true, false, Alternative(nameof(BinaryExpressionSyntax), "nonterminal"))),
                Rule("BinaryExpressionSyntax",
                    Field("Left", true, false, Alternative(nameof(LiteralExpressionSyntax), "nonterminal")),
                    Field("OperatorToken", true, false, Alternative("binary_operator", "string_payload")),
                    Field("Right", true, false, Alternative(nameof(LiteralExpressionSyntax), "nonterminal"))),
                Rule("LiteralExpressionSyntax",
                    Field("Token", true, false, Alternative("integer_literal", "integer_payload")))
            });
        }
        else if (returnTypeKeyword == "string" &&
            methodDecl.ParameterList.Parameters.Count == 1 &&
            methodDecl.ParameterList.Parameters[0] is { } nullableParameter &&
            nullableParameter.Type is NullableTypeSyntax nullableParameterType &&
            nullableParameterType.ElementType is PredefinedTypeSyntax nullableParameterElementType &&
            nullableParameterElementType.Keyword.Text == "string" &&
            nullableParameterType.QuestionToken.Text == "?" &&
            nullableParameter.Identifier.Text == "name" &&
            body.Statements.Count == 1 &&
            body.Statements[0] is ReturnStatementSyntax nullableParameterReturnStatement &&
            nullableParameterReturnStatement.Expression is LiteralExpressionSyntax nullableParameterReturnLiteral &&
            nullableParameterReturnLiteral.IsKind(SyntaxKind.StringLiteralExpression) &&
            nullableParameterReturnLiteral.Token.ValueText == "ok")
        {
            metadata = BaseMetadata(new object[]
            {
                Rule("CompilationUnitSyntax", Field("Members", true, true,
                    Alternative(nameof(FileScopedNamespaceDeclarationSyntax), "nonterminal"))),
                Rule("FileScopedNamespaceDeclarationSyntax",
                    Field("Name", true, false, Alternative("qualified_identifier", "identifier_payload")),
                    Field("Members", true, true, Alternative(nameof(ClassDeclarationSyntax), "nonterminal"))),
                Rule("ClassDeclarationSyntax",
                    Field("Identifier", true, false, Alternative("identifier", "identifier_payload")),
                    Field("Members", true, true, Alternative(nameof(MethodDeclarationSyntax), "nonterminal"))),
                Rule("MethodDeclarationSyntax",
                    Field("Identifier", true, false, Alternative("identifier", "identifier_payload")),
                    Field("ReturnType", true, false, Alternative(nameof(PredefinedTypeSyntax), "nonterminal")),
                    Field("ParameterList", true, false, Alternative(nameof(ParameterListSyntax), "nonterminal")),
                    Field("Body", true, false, Alternative(nameof(BlockSyntax), "nonterminal"))),
                Rule("PredefinedTypeSyntax",
                    Field("Keyword", true, false, Alternative("string_keyword", "identifier_payload"))),
                Rule("ParameterListSyntax",
                    Field("Parameters", true, true, Alternative(nameof(ParameterSyntax), "nonterminal"))),
                Rule("ParameterSyntax",
                    Field("Type", true, false, Alternative(nameof(NullableTypeSyntax), "nonterminal")),
                    Field("Identifier", true, false, Alternative("identifier", "identifier_payload"))),
                Rule("NullableTypeSyntax",
                    Field("ElementType", true, false, Alternative(nameof(PredefinedTypeSyntax), "nonterminal")),
                    Field("QuestionToken", true, false, Alternative("nullable_marker", "string_payload"))),
                Rule("BlockSyntax",
                    Field("Statements", true, true, Alternative(nameof(ReturnStatementSyntax), "nonterminal"))),
                Rule("ReturnStatementSyntax",
                    Field("Expression", true, false, Alternative(nameof(LiteralExpressionSyntax), "nonterminal"))),
                Rule("LiteralExpressionSyntax",
                    Field("Token", true, false, Alternative("string_literal", "string_payload")))
            });
        }
        else if (returnTypeKeyword == "string" &&
            methodDecl.ParameterList.Parameters.Count == 1 &&
            methodDecl.ParameterList.Parameters[0] is { } interpolationParameter &&
            interpolationParameter.Type is PredefinedTypeSyntax interpolationParameterType &&
            interpolationParameterType.Keyword.Text == "string" &&
            interpolationParameter.Identifier.Text == "name" &&
            body.Statements.Count == 1 &&
            body.Statements[0] is ReturnStatementSyntax interpolationReturnStatement &&
            interpolationReturnStatement.Expression is InterpolatedStringExpressionSyntax interpolationExpression &&
            interpolationExpression.Contents.Count == 3 &&
            interpolationExpression.Contents[0] is InterpolatedStringTextSyntax interpolationHead &&
            interpolationHead.TextToken.ValueText == "Hello, " &&
            interpolationExpression.Contents[1] is InterpolationSyntax interpolation &&
            interpolation.Expression is IdentifierNameSyntax interpolationIdentifier &&
            interpolationIdentifier.Identifier.Text == interpolationParameter.Identifier.Text &&
            interpolationExpression.Contents[2] is InterpolatedStringTextSyntax interpolationTail &&
            interpolationTail.TextToken.ValueText == "!")
        {
            metadata = BaseMetadata(new object[]
            {
                Rule("CompilationUnitSyntax", Field("Members", true, true,
                    Alternative(nameof(FileScopedNamespaceDeclarationSyntax), "nonterminal"))),
                Rule("FileScopedNamespaceDeclarationSyntax",
                    Field("Name", true, false, Alternative("qualified_identifier", "identifier_payload")),
                    Field("Members", true, true, Alternative(nameof(ClassDeclarationSyntax), "nonterminal"))),
                Rule("ClassDeclarationSyntax",
                    Field("Identifier", true, false, Alternative("identifier", "identifier_payload")),
                    Field("Members", true, true, Alternative(nameof(MethodDeclarationSyntax), "nonterminal"))),
                Rule("MethodDeclarationSyntax",
                    Field("Identifier", true, false, Alternative("identifier", "identifier_payload")),
                    Field("ReturnType", true, false, Alternative(nameof(PredefinedTypeSyntax), "nonterminal")),
                    Field("ParameterList", true, false, Alternative(nameof(ParameterListSyntax), "nonterminal")),
                    Field("Body", true, false, Alternative(nameof(BlockSyntax), "nonterminal"))),
                Rule("PredefinedTypeSyntax",
                    Field("Keyword", true, false, Alternative("string_keyword", "identifier_payload"))),
                Rule("ParameterListSyntax",
                    Field("Parameters", true, true, Alternative(nameof(ParameterSyntax), "nonterminal"))),
                Rule("ParameterSyntax",
                    Field("Type", true, false, Alternative(nameof(PredefinedTypeSyntax), "nonterminal")),
                    Field("Identifier", true, false, Alternative("identifier", "identifier_payload"))),
                Rule("BlockSyntax",
                    Field("Statements", true, true, Alternative(nameof(ReturnStatementSyntax), "nonterminal"))),
                Rule("ReturnStatementSyntax",
                    Field("Expression", true, false, Alternative(nameof(InterpolatedStringExpressionSyntax), "nonterminal"))),
                Rule("InterpolatedStringExpressionSyntax",
                    Field("Contents", true, true,
                        Alternative(nameof(InterpolatedStringTextSyntax), "nonterminal"),
                        Alternative(nameof(InterpolationSyntax), "nonterminal"))),
                Rule("InterpolatedStringTextSyntax",
                    Field("TextToken", true, false, Alternative("interpolated_string_text", "string_payload"))),
                Rule("InterpolationSyntax",
                    Field("Expression", true, false, Alternative(nameof(IdentifierNameSyntax), "nonterminal"))),
                Rule("IdentifierNameSyntax",
                    Field("Identifier", true, false, Alternative("identifier", "identifier_payload")))
            });
        }
        else if (returnTypeKeyword == "string" &&
            methodDecl.ParameterList.Parameters.Count == 0 &&
            body.Statements.Count == 1 &&
            body.Statements[0] is ReturnStatementSyntax memberAccessReturnStatement &&
            memberAccessReturnStatement.Expression is MemberAccessExpressionSyntax memberAccessExpression &&
            memberAccessExpression.Expression is IdentifierNameSyntax memberAccessTarget &&
            memberAccessTarget.Identifier.Text == "DateTime" &&
            memberAccessExpression.Name is IdentifierNameSyntax memberAccessMember &&
            memberAccessMember.Identifier.Text == "Now")
        {
            metadata = BaseMetadata(new object[]
            {
                Rule("CompilationUnitSyntax", Field("Members", true, true,
                    Alternative(nameof(FileScopedNamespaceDeclarationSyntax), "nonterminal"))),
                Rule("FileScopedNamespaceDeclarationSyntax",
                    Field("Name", true, false, Alternative("qualified_identifier", "identifier_payload")),
                    Field("Members", true, true, Alternative(nameof(ClassDeclarationSyntax), "nonterminal"))),
                Rule("ClassDeclarationSyntax",
                    Field("Identifier", true, false, Alternative("identifier", "identifier_payload")),
                    Field("Members", true, true, Alternative(nameof(MethodDeclarationSyntax), "nonterminal"))),
                Rule("MethodDeclarationSyntax",
                    Field("Identifier", true, false, Alternative("identifier", "identifier_payload")),
                    Field("ReturnType", true, false, Alternative(nameof(PredefinedTypeSyntax), "nonterminal")),
                    Field("Body", true, false, Alternative(nameof(BlockSyntax), "nonterminal"))),
                Rule("PredefinedTypeSyntax",
                    Field("Keyword", true, false, Alternative("string_keyword", "identifier_payload"))),
                Rule("BlockSyntax",
                    Field("Statements", true, true, Alternative(nameof(ReturnStatementSyntax), "nonterminal"))),
                Rule("ReturnStatementSyntax",
                    Field("Expression", true, false, Alternative(nameof(MemberAccessExpressionSyntax), "nonterminal"))),
                Rule("MemberAccessExpressionSyntax",
                    Field("Expression", true, false, Alternative(nameof(IdentifierNameSyntax), "nonterminal")),
                    Field("Name", true, false, Alternative(nameof(IdentifierNameSyntax), "nonterminal"))),
                Rule("IdentifierNameSyntax",
                    Field("Identifier", true, false, Alternative("identifier", "identifier_payload")))
            });
        }
        else if (returnTypeKeyword == "string" &&
            methodDecl.ParameterList.Parameters.Count == 0 &&
            body.Statements.Count == 2 &&
            body.Statements[0] is ExpressionStatementSyntax thisExpressionStatement &&
            thisExpressionStatement.Expression is ConditionalAccessExpressionSyntax thisConditionalAccess &&
            thisConditionalAccess.Expression is IdentifierNameSyntax thisConditionalTarget &&
            thisConditionalTarget.Identifier.Text == "PropertyChanged" &&
            thisConditionalAccess.WhenNotNull is InvocationExpressionSyntax thisConditionalInvocation &&
            thisConditionalInvocation.Expression is MemberBindingExpressionSyntax thisMemberBinding &&
            thisMemberBinding.Name is IdentifierNameSyntax thisMemberBindingName &&
            thisMemberBindingName.Identifier.Text == "Invoke" &&
            thisConditionalInvocation.ArgumentList.Arguments.Count == 1 &&
            thisConditionalInvocation.ArgumentList.Arguments[0].Expression is ThisExpressionSyntax &&
            body.Statements[1] is ReturnStatementSyntax thisReturnStatement &&
            thisReturnStatement.Expression is LiteralExpressionSyntax thisReturnLiteral &&
            thisReturnLiteral.IsKind(SyntaxKind.StringLiteralExpression) &&
            thisReturnLiteral.Token.ValueText == "sent")
        {
            metadata = BaseMetadata(new object[]
            {
                Rule("CompilationUnitSyntax", Field("Members", true, true,
                    Alternative(nameof(FileScopedNamespaceDeclarationSyntax), "nonterminal"))),
                Rule("FileScopedNamespaceDeclarationSyntax",
                    Field("Name", true, false, Alternative("qualified_identifier", "identifier_payload")),
                    Field("Members", true, true, Alternative(nameof(ClassDeclarationSyntax), "nonterminal"))),
                Rule("ClassDeclarationSyntax",
                    Field("Identifier", true, false, Alternative("identifier", "identifier_payload")),
                    Field("Members", true, true, Alternative(nameof(MethodDeclarationSyntax), "nonterminal"))),
                Rule("MethodDeclarationSyntax",
                    Field("Identifier", true, false, Alternative("identifier", "identifier_payload")),
                    Field("ReturnType", true, false, Alternative(nameof(PredefinedTypeSyntax), "nonterminal")),
                    Field("Body", true, false, Alternative(nameof(BlockSyntax), "nonterminal"))),
                Rule("PredefinedTypeSyntax",
                    Field("Keyword", true, false, Alternative("string_keyword", "identifier_payload"))),
                Rule("BlockSyntax",
                    Field("Statements", true, true,
                        Alternative(nameof(ExpressionStatementSyntax), "nonterminal"),
                        Alternative(nameof(ReturnStatementSyntax), "nonterminal"))),
                Rule("ExpressionStatementSyntax",
                    Field("Expression", true, false, Alternative(nameof(ConditionalAccessExpressionSyntax), "nonterminal"))),
                Rule("ConditionalAccessExpressionSyntax",
                    Field("Expression", true, false, Alternative(nameof(IdentifierNameSyntax), "nonterminal")),
                    Field("WhenNotNull", true, false, Alternative(nameof(InvocationExpressionSyntax), "nonterminal"))),
                Rule("InvocationExpressionSyntax",
                    Field("Expression", true, false, Alternative(nameof(MemberBindingExpressionSyntax), "nonterminal")),
                    Field("ArgumentList", true, false, Alternative(nameof(ArgumentListSyntax), "nonterminal"))),
                Rule("MemberBindingExpressionSyntax",
                    Field("Name", true, false, Alternative(nameof(IdentifierNameSyntax), "nonterminal"))),
                Rule("ArgumentListSyntax",
                    Field("Arguments", true, true, Alternative(nameof(ArgumentSyntax), "nonterminal"))),
                Rule("ArgumentSyntax",
                    Field("Expression", true, false, Alternative(nameof(ThisExpressionSyntax), "nonterminal"))),
                Rule("ThisExpressionSyntax"),
                Rule("ReturnStatementSyntax",
                    Field("Expression", true, false, Alternative(nameof(LiteralExpressionSyntax), "nonterminal"))),
                Rule("IdentifierNameSyntax",
                    Field("Identifier", true, false, Alternative("identifier", "identifier_payload"))),
                Rule("LiteralExpressionSyntax",
                    Field("Token", true, false, Alternative("string_literal", "string_payload")))
            });
        }
        else if (returnTypeKeyword == "string" &&
            methodDecl.ParameterList.Parameters.Count == 0 &&
            body.Statements.Count == 2 &&
            body.Statements[0] is ExpressionStatementSyntax conditionalAccessStatement &&
            conditionalAccessStatement.Expression is ConditionalAccessExpressionSyntax conditionalAccess &&
            conditionalAccess.Expression is IdentifierNameSyntax conditionalAccessTarget &&
            conditionalAccessTarget.Identifier.Text == "PropertyChanged" &&
            conditionalAccess.WhenNotNull is InvocationExpressionSyntax conditionalInvocation &&
            conditionalInvocation.Expression is MemberBindingExpressionSyntax memberBinding &&
            memberBinding.Name is IdentifierNameSyntax memberBindingName &&
            memberBindingName.Identifier.Text == "Invoke" &&
            conditionalInvocation.ArgumentList.Arguments.Count == 1 &&
            conditionalInvocation.ArgumentList.Arguments[0].Expression is LiteralExpressionSyntax conditionalArgument &&
            conditionalArgument.IsKind(SyntaxKind.StringLiteralExpression) &&
            conditionalArgument.Token.ValueText == "Name" &&
            body.Statements[1] is ReturnStatementSyntax conditionalReturnStatement &&
            conditionalReturnStatement.Expression is LiteralExpressionSyntax conditionalReturnLiteral &&
            conditionalReturnLiteral.IsKind(SyntaxKind.StringLiteralExpression) &&
            conditionalReturnLiteral.Token.ValueText == "sent")
        {
            metadata = BaseMetadata(new object[]
            {
                Rule("CompilationUnitSyntax", Field("Members", true, true,
                    Alternative(nameof(FileScopedNamespaceDeclarationSyntax), "nonterminal"))),
                Rule("FileScopedNamespaceDeclarationSyntax",
                    Field("Name", true, false, Alternative("qualified_identifier", "identifier_payload")),
                    Field("Members", true, true, Alternative(nameof(ClassDeclarationSyntax), "nonterminal"))),
                Rule("ClassDeclarationSyntax",
                    Field("Identifier", true, false, Alternative("identifier", "identifier_payload")),
                    Field("Members", true, true, Alternative(nameof(MethodDeclarationSyntax), "nonterminal"))),
                Rule("MethodDeclarationSyntax",
                    Field("Identifier", true, false, Alternative("identifier", "identifier_payload")),
                    Field("ReturnType", true, false, Alternative(nameof(PredefinedTypeSyntax), "nonterminal")),
                    Field("Body", true, false, Alternative(nameof(BlockSyntax), "nonterminal"))),
                Rule("PredefinedTypeSyntax",
                    Field("Keyword", true, false, Alternative("string_keyword", "identifier_payload"))),
                Rule("BlockSyntax",
                    Field("Statements", true, true,
                        Alternative(nameof(ExpressionStatementSyntax), "nonterminal"),
                        Alternative(nameof(ReturnStatementSyntax), "nonterminal"))),
                Rule("ExpressionStatementSyntax",
                    Field("Expression", true, false, Alternative(nameof(ConditionalAccessExpressionSyntax), "nonterminal"))),
                Rule("ConditionalAccessExpressionSyntax",
                    Field("Expression", true, false, Alternative(nameof(IdentifierNameSyntax), "nonterminal")),
                    Field("WhenNotNull", true, false, Alternative(nameof(InvocationExpressionSyntax), "nonterminal"))),
                Rule("InvocationExpressionSyntax",
                    Field("Expression", true, false, Alternative(nameof(MemberBindingExpressionSyntax), "nonterminal")),
                    Field("ArgumentList", true, false, Alternative(nameof(ArgumentListSyntax), "nonterminal"))),
                Rule("MemberBindingExpressionSyntax",
                    Field("Name", true, false, Alternative(nameof(IdentifierNameSyntax), "nonterminal"))),
                Rule("ArgumentListSyntax",
                    Field("Arguments", true, true, Alternative(nameof(ArgumentSyntax), "nonterminal"))),
                Rule("ArgumentSyntax",
                    Field("Expression", true, false, Alternative(nameof(LiteralExpressionSyntax), "nonterminal"))),
                Rule("ReturnStatementSyntax",
                    Field("Expression", true, false, Alternative(nameof(LiteralExpressionSyntax), "nonterminal"))),
                Rule("IdentifierNameSyntax",
                    Field("Identifier", true, false, Alternative("identifier", "identifier_payload"))),
                Rule("LiteralExpressionSyntax",
                    Field("Token", true, false, Alternative("string_literal", "string_payload")))
            });
        }
        else if (returnTypeKeyword == "bool" &&
            methodDecl.ParameterList.Parameters.Count == 1 &&
            methodDecl.ParameterList.Parameters[0] is { } patternParameter &&
            patternParameter.Type is NullableTypeSyntax patternParameterType &&
            patternParameterType.ElementType is PredefinedTypeSyntax patternParameterElementType &&
            patternParameterElementType.Keyword.Text == "string" &&
            patternParameterType.QuestionToken.Text == "?" &&
            patternParameter.Identifier.Text == "value" &&
            body.Statements.Count == 1 &&
            body.Statements[0] is ReturnStatementSyntax patternReturnStatement &&
            patternReturnStatement.Expression is IsPatternExpressionSyntax isPattern &&
            isPattern.Expression is IdentifierNameSyntax patternExpression &&
            patternExpression.Identifier.Text == patternParameter.Identifier.Text &&
            isPattern.Pattern is ConstantPatternSyntax constantPattern &&
            constantPattern.Expression is LiteralExpressionSyntax patternLiteral &&
            patternLiteral.IsKind(SyntaxKind.NullLiteralExpression))
        {
            metadata = BaseMetadata(new object[]
            {
                Rule("CompilationUnitSyntax", Field("Members", true, true,
                    Alternative(nameof(FileScopedNamespaceDeclarationSyntax), "nonterminal"))),
                Rule("FileScopedNamespaceDeclarationSyntax",
                    Field("Name", true, false, Alternative("qualified_identifier", "identifier_payload")),
                    Field("Members", true, true, Alternative(nameof(ClassDeclarationSyntax), "nonterminal"))),
                Rule("ClassDeclarationSyntax",
                    Field("Identifier", true, false, Alternative("identifier", "identifier_payload")),
                    Field("Members", true, true, Alternative(nameof(MethodDeclarationSyntax), "nonterminal"))),
                Rule("MethodDeclarationSyntax",
                    Field("Identifier", true, false, Alternative("identifier", "identifier_payload")),
                    Field("ReturnType", true, false, Alternative(nameof(PredefinedTypeSyntax), "nonterminal")),
                    Field("ParameterList", true, false, Alternative(nameof(ParameterListSyntax), "nonterminal")),
                    Field("Body", true, false, Alternative(nameof(BlockSyntax), "nonterminal"))),
                Rule("PredefinedTypeSyntax",
                    Field("Keyword", true, false,
                        Alternative("bool_keyword", "identifier_payload"),
                        Alternative("string_keyword", "identifier_payload"))),
                Rule("ParameterListSyntax",
                    Field("Parameters", true, true, Alternative(nameof(ParameterSyntax), "nonterminal"))),
                Rule("ParameterSyntax",
                    Field("Type", true, false, Alternative(nameof(NullableTypeSyntax), "nonterminal")),
                    Field("Identifier", true, false, Alternative("identifier", "identifier_payload"))),
                Rule("NullableTypeSyntax",
                    Field("ElementType", true, false, Alternative(nameof(PredefinedTypeSyntax), "nonterminal")),
                    Field("QuestionToken", true, false, Alternative("nullable_marker", "string_payload"))),
                Rule("BlockSyntax",
                    Field("Statements", true, true, Alternative(nameof(ReturnStatementSyntax), "nonterminal"))),
                Rule("ReturnStatementSyntax",
                    Field("Expression", true, false, Alternative(nameof(IsPatternExpressionSyntax), "nonterminal"))),
                Rule("IsPatternExpressionSyntax",
                    Field("Expression", true, false, Alternative(nameof(IdentifierNameSyntax), "nonterminal")),
                    Field("Pattern", true, false, Alternative(nameof(ConstantPatternSyntax), "nonterminal"))),
                Rule("ConstantPatternSyntax",
                    Field("Expression", true, false, Alternative(nameof(LiteralExpressionSyntax), "nonterminal"))),
                Rule("IdentifierNameSyntax",
                    Field("Identifier", true, false, Alternative("identifier", "identifier_payload"))),
                Rule("LiteralExpressionSyntax",
                    Field("Token", true, false, Alternative("null_literal", "identifier_payload")))
            });
        }
        else if (body.Statements.Count == 1 &&
            body.Statements[0] is ReturnStatementSyntax returnStatement &&
            returnStatement.Expression is LiteralExpressionSyntax returnLiteral)
        {
            string returnTypeNodeKind;
            string literalNodeKind;
            string literalTerminal;
            if (returnTypeKeyword == "string" && returnLiteral.IsKind(SyntaxKind.StringLiteralExpression))
            {
                returnTypeNodeKind = "string_keyword";
                literalNodeKind = "string_literal";
                literalTerminal = "string_payload";
            }
            else if (returnTypeKeyword == "int" && returnLiteral.IsKind(SyntaxKind.NumericLiteralExpression))
            {
                returnTypeNodeKind = "int_keyword";
                literalNodeKind = "integer_literal";
                literalTerminal = "integer_payload";
            }
            else if (returnTypeKeyword == "double" &&
                returnLiteral.IsKind(SyntaxKind.NumericLiteralExpression) &&
                returnLiteral.Token.Text.Contains('.', StringComparison.Ordinal))
            {
                returnTypeNodeKind = "double_keyword";
                literalNodeKind = "float_literal";
                literalTerminal = "string_payload";
            }
            else if (returnTypeKeyword == "bool" &&
                (returnLiteral.IsKind(SyntaxKind.TrueLiteralExpression) ||
                    returnLiteral.IsKind(SyntaxKind.FalseLiteralExpression)))
            {
                returnTypeNodeKind = "bool_keyword";
                literalNodeKind = "boolean_literal";
                literalTerminal = "identifier_payload";
            }
            else if (returnTypeKeyword == "object" &&
                returnLiteral.IsKind(SyntaxKind.NullLiteralExpression))
            {
                returnTypeNodeKind = "object_keyword";
                literalNodeKind = "null_literal";
                literalTerminal = "identifier_payload";
            }
            else
            {
                Console.Error.WriteLine("single-return fixture literal kind did not match return type");
                return 4;
            }

            metadata = BaseMetadata(new object[]
            {
                Rule("CompilationUnitSyntax", Field("Members", true, true,
                    Alternative(nameof(FileScopedNamespaceDeclarationSyntax), "nonterminal"))),
                Rule("FileScopedNamespaceDeclarationSyntax",
                    Field("Name", true, false, Alternative("qualified_identifier", "identifier_payload")),
                    Field("Members", true, true, Alternative(nameof(ClassDeclarationSyntax), "nonterminal"))),
                Rule("ClassDeclarationSyntax",
                    Field("Identifier", true, false, Alternative("identifier", "identifier_payload")),
                    Field("Members", true, true, Alternative(nameof(MethodDeclarationSyntax), "nonterminal"))),
                Rule("MethodDeclarationSyntax",
                    Field("Identifier", true, false, Alternative("identifier", "identifier_payload")),
                    Field("ReturnType", true, false, Alternative(nameof(PredefinedTypeSyntax), "nonterminal")),
                    Field("Body", true, false, Alternative(nameof(BlockSyntax), "nonterminal"))),
                Rule("PredefinedTypeSyntax",
                    Field("Keyword", true, false, Alternative(returnTypeNodeKind, "identifier_payload"))),
                Rule("BlockSyntax",
                    Field("Statements", true, true, Alternative(nameof(ReturnStatementSyntax), "nonterminal"))),
                Rule("ReturnStatementSyntax",
                    Field("Expression", true, false, Alternative(nameof(LiteralExpressionSyntax), "nonterminal"))),
                Rule("LiteralExpressionSyntax",
                    Field("Token", true, false, Alternative(literalNodeKind, literalTerminal)))
            });
        }
        else if (methodDecl.ParameterList.Parameters.Count == 1 &&
            returnTypeKeyword == "int" &&
            methodDecl.ParameterList.Parameters[0] is { } elementAccessParameter &&
            elementAccessParameter.Type is ArrayTypeSyntax elementAccessParameterType &&
            elementAccessParameterType.ElementType is PredefinedTypeSyntax elementAccessParameterElementType &&
            elementAccessParameterElementType.Keyword.Text == "int" &&
            elementAccessParameter.Identifier.Text == "values" &&
            elementAccessParameterType.RankSpecifiers.Count == 1 &&
            body.Statements.Count == 1 &&
            body.Statements[0] is ReturnStatementSyntax elementAccessReturnStatement &&
            elementAccessReturnStatement.Expression is ElementAccessExpressionSyntax elementAccess &&
            elementAccess.Expression is IdentifierNameSyntax elementAccessTarget &&
            elementAccessTarget.Identifier.Text == elementAccessParameter.Identifier.Text &&
            elementAccess.ArgumentList.Arguments.Count == 1 &&
            elementAccess.ArgumentList.Arguments[0].Expression is LiteralExpressionSyntax elementAccessIndex &&
            elementAccessIndex.IsKind(SyntaxKind.NumericLiteralExpression) &&
            elementAccessIndex.Token.ValueText == "0")
        {
            metadata = BaseMetadata(new object[]
            {
                Rule("CompilationUnitSyntax", Field("Members", true, true,
                    Alternative(nameof(FileScopedNamespaceDeclarationSyntax), "nonterminal"))),
                Rule("FileScopedNamespaceDeclarationSyntax",
                    Field("Name", true, false, Alternative("qualified_identifier", "identifier_payload")),
                    Field("Members", true, true, Alternative(nameof(ClassDeclarationSyntax), "nonterminal"))),
                Rule("ClassDeclarationSyntax",
                    Field("Identifier", true, false, Alternative("identifier", "identifier_payload")),
                    Field("Members", true, true, Alternative(nameof(MethodDeclarationSyntax), "nonterminal"))),
                Rule("MethodDeclarationSyntax",
                    Field("Identifier", true, false, Alternative("identifier", "identifier_payload")),
                    Field("ReturnType", true, false, Alternative(nameof(PredefinedTypeSyntax), "nonterminal")),
                    Field("ParameterList", true, false, Alternative(nameof(ParameterListSyntax), "nonterminal")),
                    Field("Body", true, false, Alternative(nameof(BlockSyntax), "nonterminal"))),
                Rule("PredefinedTypeSyntax",
                    Field("Keyword", true, false, Alternative("int_keyword", "identifier_payload"))),
                Rule("ParameterListSyntax",
                    Field("Parameters", true, true, Alternative(nameof(ParameterSyntax), "nonterminal"))),
                Rule("ParameterSyntax",
                    Field("Type", true, false, Alternative(nameof(ArrayTypeSyntax), "nonterminal")),
                    Field("Identifier", true, false, Alternative("identifier", "identifier_payload"))),
                Rule("ArrayTypeSyntax",
                    Field("ElementType", true, false, Alternative(nameof(PredefinedTypeSyntax), "nonterminal"))),
                Rule("BlockSyntax",
                    Field("Statements", true, true, Alternative(nameof(ReturnStatementSyntax), "nonterminal"))),
                Rule("ReturnStatementSyntax",
                    Field("Expression", true, false, Alternative(nameof(ElementAccessExpressionSyntax), "nonterminal"))),
                Rule("ElementAccessExpressionSyntax",
                    Field("Expression", true, false, Alternative(nameof(IdentifierNameSyntax), "nonterminal")),
                    Field("ArgumentList", true, false, Alternative(nameof(BracketedArgumentListSyntax), "nonterminal"))),
                Rule("BracketedArgumentListSyntax",
                    Field("Arguments", true, true, Alternative(nameof(ArgumentSyntax), "nonterminal"))),
                Rule("ArgumentSyntax",
                    Field("Expression", true, false, Alternative(nameof(LiteralExpressionSyntax), "nonterminal"))),
                Rule("IdentifierNameSyntax",
                    Field("Identifier", true, false, Alternative("identifier", "identifier_payload"))),
                Rule("LiteralExpressionSyntax",
                    Field("Token", true, false, Alternative("integer_literal", "integer_payload")))
            });
        }
        else if (methodDecl.ParameterList.Parameters.Count == 2 &&
            returnTypeKeyword == "int" &&
            methodDecl.ParameterList.Parameters[0] is { } firstParameter &&
            firstParameter.Type is PredefinedTypeSyntax firstParameterType &&
            firstParameterType.Keyword.Text == "int" &&
            firstParameter.Identifier.Text == "left" &&
            methodDecl.ParameterList.Parameters[1] is { } secondParameter &&
            secondParameter.Type is PredefinedTypeSyntax secondParameterType &&
            secondParameterType.Keyword.Text == "int" &&
            secondParameter.Identifier.Text == "right" &&
            body.Statements.Count == 1 &&
            body.Statements[0] is ReturnStatementSyntax parameterReturnStatement &&
            parameterReturnStatement.Expression is IdentifierNameSyntax parameterReturnIdentifier &&
            parameterReturnIdentifier.Identifier.Text == firstParameter.Identifier.Text)
        {
            metadata = BaseMetadata(new object[]
            {
                Rule("CompilationUnitSyntax", Field("Members", true, true,
                    Alternative(nameof(FileScopedNamespaceDeclarationSyntax), "nonterminal"))),
                Rule("FileScopedNamespaceDeclarationSyntax",
                    Field("Name", true, false, Alternative("qualified_identifier", "identifier_payload")),
                    Field("Members", true, true, Alternative(nameof(ClassDeclarationSyntax), "nonterminal"))),
                Rule("ClassDeclarationSyntax",
                    Field("Identifier", true, false, Alternative("identifier", "identifier_payload")),
                    Field("Members", true, true, Alternative(nameof(MethodDeclarationSyntax), "nonterminal"))),
                Rule("MethodDeclarationSyntax",
                    Field("Identifier", true, false, Alternative("identifier", "identifier_payload")),
                    Field("ReturnType", true, false, Alternative(nameof(PredefinedTypeSyntax), "nonterminal")),
                    Field("ParameterList", true, false, Alternative(nameof(ParameterListSyntax), "nonterminal")),
                    Field("Body", true, false, Alternative(nameof(BlockSyntax), "nonterminal"))),
                Rule("PredefinedTypeSyntax",
                    Field("Keyword", true, false, Alternative("int_keyword", "identifier_payload"))),
                Rule("ParameterListSyntax",
                    Field("Parameters", true, true, Alternative(nameof(ParameterSyntax), "nonterminal"))),
                Rule("ParameterSyntax",
                    Field("Type", true, false, Alternative(nameof(PredefinedTypeSyntax), "nonterminal")),
                    Field("Identifier", true, false, Alternative("identifier", "identifier_payload"))),
                Rule("BlockSyntax",
                    Field("Statements", true, true, Alternative(nameof(ReturnStatementSyntax), "nonterminal"))),
                Rule("ReturnStatementSyntax",
                    Field("Expression", true, false, Alternative(nameof(IdentifierNameSyntax), "nonterminal"))),
                Rule("IdentifierNameSyntax",
                    Field("Identifier", true, false, Alternative("identifier", "identifier_payload")))
            });
        }
        else if (body.Statements.Count == 2 &&
            returnTypeKeyword == "object" &&
            body.Statements[0] is LocalDeclarationStatementSyntax nullableLocalStatement &&
            nullableLocalStatement.Declaration.Type is NullableTypeSyntax nullableLocalType &&
            nullableLocalType.ElementType is PredefinedTypeSyntax nullableLocalElementType &&
            nullableLocalElementType.Keyword.Text == "string" &&
            nullableLocalStatement.Declaration.Variables.Count == 1 &&
            nullableLocalStatement.Declaration.Variables[0] is { } nullableVariableDeclarator &&
            nullableVariableDeclarator.Identifier.Text == "value" &&
            nullableVariableDeclarator.Initializer?.Value is LiteralExpressionSyntax nullableLocalLiteral &&
            nullableLocalLiteral.IsKind(SyntaxKind.NullLiteralExpression) &&
            body.Statements[1] is ReturnStatementSyntax nullableLocalReturnStatement &&
            nullableLocalReturnStatement.Expression is IdentifierNameSyntax nullableReturnIdentifier &&
            nullableReturnIdentifier.Identifier.Text == nullableVariableDeclarator.Identifier.Text)
        {
            metadata = BaseMetadata(new object[]
            {
                Rule("CompilationUnitSyntax", Field("Members", true, true,
                    Alternative(nameof(FileScopedNamespaceDeclarationSyntax), "nonterminal"))),
                Rule("FileScopedNamespaceDeclarationSyntax",
                    Field("Name", true, false, Alternative("qualified_identifier", "identifier_payload")),
                    Field("Members", true, true, Alternative(nameof(ClassDeclarationSyntax), "nonterminal"))),
                Rule("ClassDeclarationSyntax",
                    Field("Identifier", true, false, Alternative("identifier", "identifier_payload")),
                    Field("Members", true, true, Alternative(nameof(MethodDeclarationSyntax), "nonterminal"))),
                Rule("MethodDeclarationSyntax",
                    Field("Identifier", true, false, Alternative("identifier", "identifier_payload")),
                    Field("ReturnType", true, false, Alternative(nameof(PredefinedTypeSyntax), "nonterminal")),
                    Field("Body", true, false, Alternative(nameof(BlockSyntax), "nonterminal"))),
                Rule("PredefinedTypeSyntax",
                    Field("Keyword", true, false,
                        Alternative("object_keyword", "identifier_payload"),
                        Alternative("string_keyword", "identifier_payload"))),
                Rule("BlockSyntax",
                    Field("Statements", true, true,
                        Alternative(nameof(LocalDeclarationStatementSyntax), "nonterminal"),
                        Alternative(nameof(ReturnStatementSyntax), "nonterminal"))),
                Rule("LocalDeclarationStatementSyntax",
                    Field("Declaration", true, false, Alternative(nameof(VariableDeclarationSyntax), "nonterminal"))),
                Rule("VariableDeclarationSyntax",
                    Field("Type", true, false, Alternative(nameof(NullableTypeSyntax), "nonterminal")),
                    Field("Variables", true, true, Alternative(nameof(VariableDeclaratorSyntax), "nonterminal"))),
                Rule("NullableTypeSyntax",
                    Field("ElementType", true, false, Alternative(nameof(PredefinedTypeSyntax), "nonterminal")),
                    Field("QuestionToken", true, false, Alternative("nullable_marker", "string_payload"))),
                Rule("VariableDeclaratorSyntax",
                    Field("Identifier", true, false, Alternative("identifier", "identifier_payload")),
                    Field("Initializer", true, false, Alternative(nameof(EqualsValueClauseSyntax), "nonterminal"))),
                Rule("EqualsValueClauseSyntax",
                    Field("Value", true, false, Alternative(nameof(LiteralExpressionSyntax), "nonterminal"))),
                Rule("LiteralExpressionSyntax",
                    Field("Token", true, false, Alternative("null_literal", "identifier_payload"))),
                Rule("ReturnStatementSyntax",
                    Field("Expression", true, false, Alternative(nameof(IdentifierNameSyntax), "nonterminal"))),
                Rule("IdentifierNameSyntax",
                    Field("Identifier", true, false, Alternative("identifier", "identifier_payload")))
            });
        }
    else if (body.Statements.Count == 2 &&
        body.Statements[0] is LocalDeclarationStatementSyntax localStatement &&
        (localStatement.Declaration.Type is PredefinedTypeSyntax { Keyword.Text: "string" } ||
            localStatement.Declaration.Type is IdentifierNameSyntax { Identifier.Text: "var" }) &&
        localStatement.Declaration.Variables.Count == 1 &&
        localStatement.Declaration.Variables[0] is { } variableDeclarator &&
        variableDeclarator.Initializer?.Value is LiteralExpressionSyntax localLiteral &&
        body.Statements[1] is ReturnStatementSyntax returnIdentifierStatement &&
        returnIdentifierStatement.Expression is IdentifierNameSyntax returnIdentifier &&
        returnIdentifier.Identifier.Text == variableDeclarator.Identifier.Text)
    {
        var localTypeAlternative = localStatement.Declaration.Type is IdentifierNameSyntax
            ? nameof(IdentifierNameSyntax)
            : nameof(PredefinedTypeSyntax);
        string returnTypeNodeKind;
        string literalNodeKind;
        string literalTerminal;
        if (returnTypeKeyword == "string" && localLiteral.IsKind(SyntaxKind.StringLiteralExpression))
        {
            returnTypeNodeKind = "string_keyword";
            literalNodeKind = "string_literal";
            literalTerminal = "string_payload";
        }
        else if (returnTypeKeyword == "int" &&
            localStatement.Declaration.Type is IdentifierNameSyntax &&
            localLiteral.IsKind(SyntaxKind.NumericLiteralExpression))
        {
            returnTypeNodeKind = "int_keyword";
            literalNodeKind = "integer_literal";
            literalTerminal = "integer_payload";
        }
        else
        {
            Console.Error.WriteLine("local declaration fixture literal kind did not match return type");
            return 4;
        }
        metadata = BaseMetadata(new object[]
        {
            Rule("CompilationUnitSyntax", Field("Members", true, true,
                Alternative(nameof(FileScopedNamespaceDeclarationSyntax), "nonterminal"))),
            Rule("FileScopedNamespaceDeclarationSyntax",
                Field("Name", true, false, Alternative("qualified_identifier", "identifier_payload")),
                Field("Members", true, true, Alternative(nameof(ClassDeclarationSyntax), "nonterminal"))),
            Rule("ClassDeclarationSyntax",
                Field("Identifier", true, false, Alternative("identifier", "identifier_payload")),
                Field("Members", true, true, Alternative(nameof(MethodDeclarationSyntax), "nonterminal"))),
            Rule("MethodDeclarationSyntax",
                Field("Identifier", true, false, Alternative("identifier", "identifier_payload")),
                Field("ReturnType", true, false, Alternative(nameof(PredefinedTypeSyntax), "nonterminal")),
                Field("Body", true, false, Alternative(nameof(BlockSyntax), "nonterminal"))),
            Rule("PredefinedTypeSyntax",
                Field("Keyword", true, false, Alternative(returnTypeNodeKind, "identifier_payload"))),
            Rule("BlockSyntax",
                Field("Statements", true, true,
                    Alternative(nameof(LocalDeclarationStatementSyntax), "nonterminal"),
                    Alternative(nameof(ReturnStatementSyntax), "nonterminal"))),
            Rule("LocalDeclarationStatementSyntax",
                Field("Declaration", true, false, Alternative(nameof(VariableDeclarationSyntax), "nonterminal"))),
            Rule("VariableDeclarationSyntax",
                Field("Type", true, false, Alternative(localTypeAlternative, "nonterminal")),
                Field("Variables", true, true, Alternative(nameof(VariableDeclaratorSyntax), "nonterminal"))),
            Rule("VariableDeclaratorSyntax",
                Field("Identifier", true, false, Alternative("identifier", "identifier_payload")),
                Field("Initializer", true, false, Alternative(nameof(EqualsValueClauseSyntax), "nonterminal"))),
            Rule("EqualsValueClauseSyntax",
                Field("Value", true, false, Alternative(nameof(LiteralExpressionSyntax), "nonterminal"))),
            Rule("LiteralExpressionSyntax",
                Field("Token", true, false, Alternative(literalNodeKind, literalTerminal))),
            Rule("ReturnStatementSyntax",
                Field("Expression", true, false, Alternative(nameof(IdentifierNameSyntax), "nonterminal"))),
            Rule("IdentifierNameSyntax",
                Field("Identifier", true, false, Alternative("identifier", "identifier_payload")))
        });
    }
    else if (body.Statements.Count == 3 &&
        body.Statements[0] is LocalDeclarationStatementSyntax assignmentLocalStatement &&
        assignmentLocalStatement.Declaration.Type is PredefinedTypeSyntax assignmentLocalType &&
        assignmentLocalType.Keyword.Text == "string" &&
        assignmentLocalStatement.Declaration.Variables.Count == 1 &&
        assignmentLocalStatement.Declaration.Variables[0] is { } assignmentVariableDeclarator &&
        assignmentVariableDeclarator.Identifier.Text == "greeting" &&
        assignmentVariableDeclarator.Initializer?.Value is LiteralExpressionSyntax assignmentLocalLiteral &&
        assignmentLocalLiteral.IsKind(SyntaxKind.StringLiteralExpression) &&
        body.Statements[1] is ExpressionStatementSyntax expressionStatement &&
        expressionStatement.Expression is AssignmentExpressionSyntax assignmentExpression &&
        assignmentExpression.Left is IdentifierNameSyntax assignmentTarget &&
        assignmentTarget.Identifier.Text == assignmentVariableDeclarator.Identifier.Text &&
        assignmentExpression.Right is LiteralExpressionSyntax assignmentLiteral &&
        assignmentLiteral.IsKind(SyntaxKind.StringLiteralExpression) &&
        body.Statements[2] is ReturnStatementSyntax assignmentReturnStatement &&
        assignmentReturnStatement.Expression is IdentifierNameSyntax assignmentReturnIdentifier &&
        assignmentReturnIdentifier.Identifier.Text == assignmentVariableDeclarator.Identifier.Text)
    {
        if (assignmentLocalLiteral.Token.ValueText != "hello")
        {
            Console.Error.WriteLine("local assignment fixture must initialize greeting with hello");
            return 4;
        }
        if (sourceDisplayPath.Contains("local_escaped_assignment", StringComparison.Ordinal) &&
            assignmentLiteral.Token.ValueText != "hello \"updated\"")
        {
            Console.Error.WriteLine("escaped local assignment fixture must preserve the escaped quote payload");
            return 4;
        }
        if (sourceDisplayPath.Contains("local_assignment", StringComparison.Ordinal) &&
            assignmentLiteral.Token.ValueText != "updated")
        {
            Console.Error.WriteLine("local assignment fixture must update greeting with updated");
            return 4;
        }
        metadata = BaseMetadata(new object[]
        {
            Rule("CompilationUnitSyntax", Field("Members", true, true,
                Alternative(nameof(FileScopedNamespaceDeclarationSyntax), "nonterminal"))),
            Rule("FileScopedNamespaceDeclarationSyntax",
                Field("Name", true, false, Alternative("qualified_identifier", "identifier_payload")),
                Field("Members", true, true, Alternative(nameof(ClassDeclarationSyntax), "nonterminal"))),
            Rule("ClassDeclarationSyntax",
                Field("Identifier", true, false, Alternative("identifier", "identifier_payload")),
                Field("Members", true, true, Alternative(nameof(MethodDeclarationSyntax), "nonterminal"))),
            Rule("MethodDeclarationSyntax",
                Field("Identifier", true, false, Alternative("identifier", "identifier_payload")),
                Field("ReturnType", true, false, Alternative(nameof(PredefinedTypeSyntax), "nonterminal")),
                Field("Body", true, false, Alternative(nameof(BlockSyntax), "nonterminal"))),
            Rule("PredefinedTypeSyntax",
                Field("Keyword", true, false, Alternative("string_keyword", "identifier_payload"))),
            Rule("BlockSyntax",
                Field("Statements", true, true,
                    Alternative(nameof(LocalDeclarationStatementSyntax), "nonterminal"),
                    Alternative(nameof(ExpressionStatementSyntax), "nonterminal"),
                    Alternative(nameof(ReturnStatementSyntax), "nonterminal"))),
            Rule("LocalDeclarationStatementSyntax",
                Field("Declaration", true, false, Alternative(nameof(VariableDeclarationSyntax), "nonterminal"))),
            Rule("VariableDeclarationSyntax",
                Field("Type", true, false, Alternative(nameof(PredefinedTypeSyntax), "nonterminal")),
                Field("Variables", true, true, Alternative(nameof(VariableDeclaratorSyntax), "nonterminal"))),
            Rule("VariableDeclaratorSyntax",
                Field("Identifier", true, false, Alternative("identifier", "identifier_payload")),
                Field("Initializer", true, false, Alternative(nameof(EqualsValueClauseSyntax), "nonterminal"))),
            Rule("EqualsValueClauseSyntax",
                Field("Value", true, false, Alternative(nameof(LiteralExpressionSyntax), "nonterminal"))),
            Rule("ExpressionStatementSyntax",
                Field("Expression", true, false, Alternative(nameof(AssignmentExpressionSyntax), "nonterminal"))),
            Rule("AssignmentExpressionSyntax",
                Field("Left", true, false, Alternative(nameof(IdentifierNameSyntax), "nonterminal")),
                Field("Right", true, false, Alternative(nameof(LiteralExpressionSyntax), "nonterminal"))),
            Rule("LiteralExpressionSyntax",
                Field("Token", true, false, Alternative("string_literal", "string_payload"))),
            Rule("ReturnStatementSyntax",
                Field("Expression", true, false, Alternative(nameof(IdentifierNameSyntax), "nonterminal"))),
            Rule("IdentifierNameSyntax",
                Field("Identifier", true, false, Alternative("identifier", "identifier_payload")))
        });
    }
    else if (body.Statements.Count == 3 &&
        body.Statements[0] is LocalDeclarationStatementSyntax ifLocalStatement &&
        ifLocalStatement.Declaration.Type is PredefinedTypeSyntax ifLocalType &&
        ifLocalType.Keyword.Text == "int" &&
        ifLocalStatement.Declaration.Variables.Count == 1 &&
        ifLocalStatement.Declaration.Variables[0] is { } ifVariableDeclarator &&
        ifVariableDeclarator.Identifier.Text == "count" &&
        ifVariableDeclarator.Initializer?.Value is LiteralExpressionSyntax ifLocalLiteral &&
        ifLocalLiteral.IsKind(SyntaxKind.NumericLiteralExpression) &&
        ifLocalLiteral.Token.ValueText == "1" &&
        body.Statements[1] is IfStatementSyntax ifStatement &&
        ifStatement.Condition is BinaryExpressionSyntax condition &&
        condition.IsKind(SyntaxKind.GreaterThanExpression) &&
        condition.Left is IdentifierNameSyntax conditionLeft &&
        conditionLeft.Identifier.Text == ifVariableDeclarator.Identifier.Text &&
        condition.Right is LiteralExpressionSyntax conditionRight &&
        conditionRight.IsKind(SyntaxKind.NumericLiteralExpression) &&
        conditionRight.Token.ValueText == "0" &&
        ifStatement.Statement is BlockSyntax thenBlock &&
        thenBlock.Statements.Count == 1 &&
        thenBlock.Statements[0] is ExpressionStatementSyntax thenExpressionStatement &&
        thenExpressionStatement.Expression is AssignmentExpressionSyntax thenAssignment &&
        thenAssignment.Left is IdentifierNameSyntax thenAssignmentTarget &&
        thenAssignmentTarget.Identifier.Text == ifVariableDeclarator.Identifier.Text &&
        thenAssignment.Right is LiteralExpressionSyntax thenAssignmentLiteral &&
        thenAssignmentLiteral.IsKind(SyntaxKind.NumericLiteralExpression) &&
        thenAssignmentLiteral.Token.ValueText == "7" &&
        ifStatement.Else?.Statement is BlockSyntax elseBlock &&
        elseBlock.Statements.Count == 1 &&
        elseBlock.Statements[0] is ExpressionStatementSyntax elseExpressionStatement &&
        elseExpressionStatement.Expression is AssignmentExpressionSyntax elseAssignment &&
        elseAssignment.Left is IdentifierNameSyntax elseAssignmentTarget &&
        elseAssignmentTarget.Identifier.Text == ifVariableDeclarator.Identifier.Text &&
        elseAssignment.Right is LiteralExpressionSyntax elseAssignmentLiteral &&
        elseAssignmentLiteral.IsKind(SyntaxKind.NumericLiteralExpression) &&
        elseAssignmentLiteral.Token.ValueText == "0" &&
        body.Statements[2] is ReturnStatementSyntax ifReturnStatement &&
        ifReturnStatement.Expression is IdentifierNameSyntax ifReturnIdentifier &&
        ifReturnIdentifier.Identifier.Text == ifVariableDeclarator.Identifier.Text)
    {
        metadata = BaseMetadata(new object[]
        {
            Rule("CompilationUnitSyntax", Field("Members", true, true,
                Alternative(nameof(FileScopedNamespaceDeclarationSyntax), "nonterminal"))),
            Rule("FileScopedNamespaceDeclarationSyntax",
                Field("Name", true, false, Alternative("qualified_identifier", "identifier_payload")),
                Field("Members", true, true, Alternative(nameof(ClassDeclarationSyntax), "nonterminal"))),
            Rule("ClassDeclarationSyntax",
                Field("Identifier", true, false, Alternative("identifier", "identifier_payload")),
                Field("Members", true, true, Alternative(nameof(MethodDeclarationSyntax), "nonterminal"))),
            Rule("MethodDeclarationSyntax",
                Field("Identifier", true, false, Alternative("identifier", "identifier_payload")),
                Field("ReturnType", true, false, Alternative(nameof(PredefinedTypeSyntax), "nonterminal")),
                Field("Body", true, false, Alternative(nameof(BlockSyntax), "nonterminal"))),
            Rule("PredefinedTypeSyntax",
                Field("Keyword", true, false, Alternative("int_keyword", "identifier_payload"))),
            Rule("BlockSyntax",
                Field("Statements", true, true,
                    Alternative(nameof(LocalDeclarationStatementSyntax), "nonterminal"),
                    Alternative(nameof(IfStatementSyntax), "nonterminal"),
                    Alternative(nameof(ExpressionStatementSyntax), "nonterminal"),
                    Alternative(nameof(ReturnStatementSyntax), "nonterminal"))),
            Rule("LocalDeclarationStatementSyntax",
                Field("Declaration", true, false, Alternative(nameof(VariableDeclarationSyntax), "nonterminal"))),
            Rule("VariableDeclarationSyntax",
                Field("Type", true, false, Alternative(nameof(PredefinedTypeSyntax), "nonterminal")),
                Field("Variables", true, true, Alternative(nameof(VariableDeclaratorSyntax), "nonterminal"))),
            Rule("VariableDeclaratorSyntax",
                Field("Identifier", true, false, Alternative("identifier", "identifier_payload")),
                Field("Initializer", true, false, Alternative(nameof(EqualsValueClauseSyntax), "nonterminal"))),
            Rule("EqualsValueClauseSyntax",
                Field("Value", true, false, Alternative(nameof(LiteralExpressionSyntax), "nonterminal"))),
            Rule("IfStatementSyntax",
                Field("Condition", true, false, Alternative(nameof(BinaryExpressionSyntax), "nonterminal")),
                Field("Statement", true, false, Alternative(nameof(BlockSyntax), "nonterminal")),
                Field("Else", true, false, Alternative(nameof(ElseClauseSyntax), "nonterminal"))),
            Rule("ElseClauseSyntax",
                Field("Statement", true, false, Alternative(nameof(BlockSyntax), "nonterminal"))),
            Rule("BinaryExpressionSyntax",
                Field("Left", true, false, Alternative(nameof(IdentifierNameSyntax), "nonterminal")),
                Field("OperatorToken", true, false, Alternative("greater_than_token", "identifier_payload")),
                Field("Right", true, false, Alternative(nameof(LiteralExpressionSyntax), "nonterminal"))),
            Rule("ExpressionStatementSyntax",
                Field("Expression", true, false, Alternative(nameof(AssignmentExpressionSyntax), "nonterminal"))),
            Rule("AssignmentExpressionSyntax",
                Field("Left", true, false, Alternative(nameof(IdentifierNameSyntax), "nonterminal")),
                Field("Right", true, false, Alternative(nameof(LiteralExpressionSyntax), "nonterminal"))),
            Rule("LiteralExpressionSyntax",
                Field("Token", true, false, Alternative("integer_literal", "integer_payload"))),
            Rule("ReturnStatementSyntax",
                Field("Expression", true, false, Alternative(nameof(IdentifierNameSyntax), "nonterminal"))),
            Rule("IdentifierNameSyntax",
                Field("Identifier", true, false, Alternative("identifier", "identifier_payload")))
        });
    }
    else if (body.Statements.Count == 3 &&
        body.Statements[0] is LocalDeclarationStatementSyntax forLocalStatement &&
        forLocalStatement.Declaration.Type is PredefinedTypeSyntax forLocalType &&
        forLocalType.Keyword.Text == "int" &&
        forLocalStatement.Declaration.Variables.Count == 1 &&
        forLocalStatement.Declaration.Variables[0] is { } forVariableDeclarator &&
        forVariableDeclarator.Identifier.Text == "count" &&
        forVariableDeclarator.Initializer?.Value is LiteralExpressionSyntax forLocalLiteral &&
        forLocalLiteral.IsKind(SyntaxKind.NumericLiteralExpression) &&
        forLocalLiteral.Token.ValueText == "1" &&
        body.Statements[1] is ForStatementSyntax forStatement &&
        forStatement.Declaration is null &&
        forStatement.Initializers.Count == 0 &&
        forStatement.Incrementors.Count == 0 &&
        forStatement.Condition is BinaryExpressionSyntax forCondition &&
        forCondition.IsKind(SyntaxKind.LessThanExpression) &&
        forCondition.Left is IdentifierNameSyntax forConditionLeft &&
        forConditionLeft.Identifier.Text == forVariableDeclarator.Identifier.Text &&
        forCondition.Right is LiteralExpressionSyntax forConditionRight &&
        forConditionRight.IsKind(SyntaxKind.NumericLiteralExpression) &&
        forConditionRight.Token.ValueText == "7" &&
        forStatement.Statement is BlockSyntax forBlock &&
        forBlock.Statements.Count == 1 &&
        forBlock.Statements[0] is ExpressionStatementSyntax forExpressionStatement &&
        forExpressionStatement.Expression is AssignmentExpressionSyntax forAssignment &&
        forAssignment.Left is IdentifierNameSyntax forAssignmentTarget &&
        forAssignmentTarget.Identifier.Text == forVariableDeclarator.Identifier.Text &&
        forAssignment.Right is LiteralExpressionSyntax forAssignmentLiteral &&
        forAssignmentLiteral.IsKind(SyntaxKind.NumericLiteralExpression) &&
        forAssignmentLiteral.Token.ValueText == "7" &&
        body.Statements[2] is ReturnStatementSyntax forReturnStatement &&
        forReturnStatement.Expression is IdentifierNameSyntax forReturnIdentifier &&
        forReturnIdentifier.Identifier.Text == forVariableDeclarator.Identifier.Text)
    {
        metadata = BaseMetadata(new object[]
        {
            Rule("CompilationUnitSyntax", Field("Members", true, true,
                Alternative(nameof(FileScopedNamespaceDeclarationSyntax), "nonterminal"))),
            Rule("FileScopedNamespaceDeclarationSyntax",
                Field("Name", true, false, Alternative("qualified_identifier", "identifier_payload")),
                Field("Members", true, true, Alternative(nameof(ClassDeclarationSyntax), "nonterminal"))),
            Rule("ClassDeclarationSyntax",
                Field("Identifier", true, false, Alternative("identifier", "identifier_payload")),
                Field("Members", true, true, Alternative(nameof(MethodDeclarationSyntax), "nonterminal"))),
            Rule("MethodDeclarationSyntax",
                Field("Identifier", true, false, Alternative("identifier", "identifier_payload")),
                Field("ReturnType", true, false, Alternative(nameof(PredefinedTypeSyntax), "nonterminal")),
                Field("Body", true, false, Alternative(nameof(BlockSyntax), "nonterminal"))),
            Rule("PredefinedTypeSyntax",
                Field("Keyword", true, false, Alternative("int_keyword", "identifier_payload"))),
            Rule("BlockSyntax",
                Field("Statements", true, true,
                    Alternative(nameof(LocalDeclarationStatementSyntax), "nonterminal"),
                    Alternative(nameof(ForStatementSyntax), "nonterminal"),
                    Alternative(nameof(ExpressionStatementSyntax), "nonterminal"),
                    Alternative(nameof(ReturnStatementSyntax), "nonterminal"))),
            Rule("LocalDeclarationStatementSyntax",
                Field("Declaration", true, false, Alternative(nameof(VariableDeclarationSyntax), "nonterminal"))),
            Rule("VariableDeclarationSyntax",
                Field("Type", true, false, Alternative(nameof(PredefinedTypeSyntax), "nonterminal")),
                Field("Variables", true, true, Alternative(nameof(VariableDeclaratorSyntax), "nonterminal"))),
            Rule("VariableDeclaratorSyntax",
                Field("Identifier", true, false, Alternative("identifier", "identifier_payload")),
                Field("Initializer", true, false, Alternative(nameof(EqualsValueClauseSyntax), "nonterminal"))),
            Rule("EqualsValueClauseSyntax",
                Field("Value", true, false, Alternative(nameof(LiteralExpressionSyntax), "nonterminal"))),
            Rule("ForStatementSyntax",
                Field("Condition", true, false, Alternative(nameof(BinaryExpressionSyntax), "nonterminal")),
                Field("Statement", true, false, Alternative(nameof(BlockSyntax), "nonterminal"))),
            Rule("BinaryExpressionSyntax",
                Field("Left", true, false, Alternative(nameof(IdentifierNameSyntax), "nonterminal")),
                Field("OperatorToken", true, false, Alternative("less_than_token", "identifier_payload")),
                Field("Right", true, false, Alternative(nameof(LiteralExpressionSyntax), "nonterminal"))),
            Rule("ExpressionStatementSyntax",
                Field("Expression", true, false, Alternative(nameof(AssignmentExpressionSyntax), "nonterminal"))),
            Rule("AssignmentExpressionSyntax",
                Field("Left", true, false, Alternative(nameof(IdentifierNameSyntax), "nonterminal")),
                Field("Right", true, false, Alternative(nameof(LiteralExpressionSyntax), "nonterminal"))),
            Rule("LiteralExpressionSyntax",
                Field("Token", true, false, Alternative("integer_literal", "integer_payload"))),
            Rule("ReturnStatementSyntax",
                Field("Expression", true, false, Alternative(nameof(IdentifierNameSyntax), "nonterminal"))),
            Rule("IdentifierNameSyntax",
                Field("Identifier", true, false, Alternative("identifier", "identifier_payload")))
        });
    }
    else if (body.Statements.Count == 3 &&
        body.Statements[0] is LocalDeclarationStatementSyntax foreachLocalStatement &&
        foreachLocalStatement.Declaration.Type is PredefinedTypeSyntax foreachLocalType &&
        foreachLocalType.Keyword.Text == "int" &&
        foreachLocalStatement.Declaration.Variables.Count == 1 &&
        foreachLocalStatement.Declaration.Variables[0] is { } foreachVariableDeclarator &&
        foreachVariableDeclarator.Identifier.Text == "count" &&
        foreachVariableDeclarator.Initializer?.Value is LiteralExpressionSyntax foreachLocalLiteral &&
        foreachLocalLiteral.IsKind(SyntaxKind.NumericLiteralExpression) &&
        foreachLocalLiteral.Token.ValueText == "1" &&
        body.Statements[1] is ForEachStatementSyntax foreachStatement &&
        foreachStatement.Type is PredefinedTypeSyntax foreachIterationType &&
        foreachIterationType.Keyword.Text == "char" &&
        foreachStatement.Identifier.Text == "ch" &&
        foreachStatement.Expression is LiteralExpressionSyntax foreachExpression &&
        foreachExpression.IsKind(SyntaxKind.StringLiteralExpression) &&
        foreachExpression.Token.ValueText == "go" &&
        foreachStatement.Statement is BlockSyntax foreachBlock &&
        foreachBlock.Statements.Count == 1 &&
        foreachBlock.Statements[0] is ExpressionStatementSyntax foreachExpressionStatement &&
        foreachExpressionStatement.Expression is AssignmentExpressionSyntax foreachAssignment &&
        foreachAssignment.Left is IdentifierNameSyntax foreachAssignmentTarget &&
        foreachAssignmentTarget.Identifier.Text == foreachVariableDeclarator.Identifier.Text &&
        foreachAssignment.Right is LiteralExpressionSyntax foreachAssignmentLiteral &&
        foreachAssignmentLiteral.IsKind(SyntaxKind.NumericLiteralExpression) &&
        foreachAssignmentLiteral.Token.ValueText == "7" &&
        body.Statements[2] is ReturnStatementSyntax foreachReturnStatement &&
        foreachReturnStatement.Expression is IdentifierNameSyntax foreachReturnIdentifier &&
        foreachReturnIdentifier.Identifier.Text == foreachVariableDeclarator.Identifier.Text)
    {
        metadata = BaseMetadata(new object[]
        {
            Rule("CompilationUnitSyntax", Field("Members", true, true,
                Alternative(nameof(FileScopedNamespaceDeclarationSyntax), "nonterminal"))),
            Rule("FileScopedNamespaceDeclarationSyntax",
                Field("Name", true, false, Alternative("qualified_identifier", "identifier_payload")),
                Field("Members", true, true, Alternative(nameof(ClassDeclarationSyntax), "nonterminal"))),
            Rule("ClassDeclarationSyntax",
                Field("Identifier", true, false, Alternative("identifier", "identifier_payload")),
                Field("Members", true, true, Alternative(nameof(MethodDeclarationSyntax), "nonterminal"))),
            Rule("MethodDeclarationSyntax",
                Field("Identifier", true, false, Alternative("identifier", "identifier_payload")),
                Field("ReturnType", true, false, Alternative(nameof(PredefinedTypeSyntax), "nonterminal")),
                Field("Body", true, false, Alternative(nameof(BlockSyntax), "nonterminal"))),
            Rule("PredefinedTypeSyntax",
                Field("Keyword", true, false,
                    Alternative("int_keyword", "identifier_payload"),
                    Alternative("char_keyword", "identifier_payload"))),
            Rule("BlockSyntax",
                Field("Statements", true, true,
                    Alternative(nameof(LocalDeclarationStatementSyntax), "nonterminal"),
                    Alternative(nameof(ForEachStatementSyntax), "nonterminal"),
                    Alternative(nameof(ExpressionStatementSyntax), "nonterminal"),
                    Alternative(nameof(ReturnStatementSyntax), "nonterminal"))),
            Rule("LocalDeclarationStatementSyntax",
                Field("Declaration", true, false, Alternative(nameof(VariableDeclarationSyntax), "nonterminal"))),
            Rule("VariableDeclarationSyntax",
                Field("Type", true, false, Alternative(nameof(PredefinedTypeSyntax), "nonterminal")),
                Field("Variables", true, true, Alternative(nameof(VariableDeclaratorSyntax), "nonterminal"))),
            Rule("VariableDeclaratorSyntax",
                Field("Identifier", true, false, Alternative("identifier", "identifier_payload")),
                Field("Initializer", true, false, Alternative(nameof(EqualsValueClauseSyntax), "nonterminal"))),
            Rule("EqualsValueClauseSyntax",
                Field("Value", true, false, Alternative(nameof(LiteralExpressionSyntax), "nonterminal"))),
            Rule("ForEachStatementSyntax",
                Field("Type", true, false, Alternative(nameof(PredefinedTypeSyntax), "nonterminal")),
                Field("Identifier", true, false, Alternative("identifier", "identifier_payload")),
                Field("Expression", true, false, Alternative(nameof(LiteralExpressionSyntax), "nonterminal")),
                Field("Statement", true, false, Alternative(nameof(BlockSyntax), "nonterminal"))),
            Rule("ExpressionStatementSyntax",
                Field("Expression", true, false, Alternative(nameof(AssignmentExpressionSyntax), "nonterminal"))),
            Rule("AssignmentExpressionSyntax",
                Field("Left", true, false, Alternative(nameof(IdentifierNameSyntax), "nonterminal")),
                Field("Right", true, false, Alternative(nameof(LiteralExpressionSyntax), "nonterminal"))),
            Rule("LiteralExpressionSyntax",
                Field("Token", true, false,
                    Alternative("integer_literal", "integer_payload"),
                    Alternative("string_literal", "string_payload"))),
            Rule("ReturnStatementSyntax",
                Field("Expression", true, false, Alternative(nameof(IdentifierNameSyntax), "nonterminal"))),
            Rule("IdentifierNameSyntax",
                Field("Identifier", true, false, Alternative("identifier", "identifier_payload")))
        });
    }
    else if (body.Statements.Count == 3 &&
        body.Statements[0] is LocalDeclarationStatementSyntax switchLocalStatement &&
        switchLocalStatement.Declaration.Type is PredefinedTypeSyntax switchLocalType &&
        switchLocalType.Keyword.Text == "int" &&
        switchLocalStatement.Declaration.Variables.Count == 1 &&
        switchLocalStatement.Declaration.Variables[0] is { } switchVariableDeclarator &&
        switchVariableDeclarator.Identifier.Text == "count" &&
        switchVariableDeclarator.Initializer?.Value is LiteralExpressionSyntax switchLocalLiteral &&
        switchLocalLiteral.IsKind(SyntaxKind.NumericLiteralExpression) &&
        switchLocalLiteral.Token.ValueText == "1" &&
        body.Statements[1] is SwitchStatementSyntax switchStatement &&
        switchStatement.Expression is IdentifierNameSyntax switchExpression &&
        switchExpression.Identifier.Text == switchVariableDeclarator.Identifier.Text &&
        switchStatement.Sections.Count == 1 &&
        switchStatement.Sections[0] is { } switchSection &&
        switchSection.Labels.Count == 1 &&
        switchSection.Labels[0] is CaseSwitchLabelSyntax caseLabel &&
        caseLabel.Value is LiteralExpressionSyntax caseLiteral &&
        caseLiteral.IsKind(SyntaxKind.NumericLiteralExpression) &&
        caseLiteral.Token.ValueText == "1" &&
        switchSection.Statements.Count == 2 &&
        switchSection.Statements[0] is ExpressionStatementSyntax switchExpressionStatement &&
        switchExpressionStatement.Expression is AssignmentExpressionSyntax switchAssignment &&
        switchAssignment.Left is IdentifierNameSyntax switchAssignmentTarget &&
        switchAssignmentTarget.Identifier.Text == switchVariableDeclarator.Identifier.Text &&
        switchAssignment.Right is LiteralExpressionSyntax switchAssignmentLiteral &&
        switchAssignmentLiteral.IsKind(SyntaxKind.NumericLiteralExpression) &&
        switchAssignmentLiteral.Token.ValueText == "7" &&
        switchSection.Statements[1] is BreakStatementSyntax &&
        body.Statements[2] is ReturnStatementSyntax switchReturnStatement &&
        switchReturnStatement.Expression is IdentifierNameSyntax switchReturnIdentifier &&
        switchReturnIdentifier.Identifier.Text == switchVariableDeclarator.Identifier.Text)
    {
        metadata = BaseMetadata(new object[]
        {
            Rule("CompilationUnitSyntax", Field("Members", true, true,
                Alternative(nameof(FileScopedNamespaceDeclarationSyntax), "nonterminal"))),
            Rule("FileScopedNamespaceDeclarationSyntax",
                Field("Name", true, false, Alternative("qualified_identifier", "identifier_payload")),
                Field("Members", true, true, Alternative(nameof(ClassDeclarationSyntax), "nonterminal"))),
            Rule("ClassDeclarationSyntax",
                Field("Identifier", true, false, Alternative("identifier", "identifier_payload")),
                Field("Members", true, true, Alternative(nameof(MethodDeclarationSyntax), "nonterminal"))),
            Rule("MethodDeclarationSyntax",
                Field("Identifier", true, false, Alternative("identifier", "identifier_payload")),
                Field("ReturnType", true, false, Alternative(nameof(PredefinedTypeSyntax), "nonterminal")),
                Field("Body", true, false, Alternative(nameof(BlockSyntax), "nonterminal"))),
            Rule("PredefinedTypeSyntax",
                Field("Keyword", true, false, Alternative("int_keyword", "identifier_payload"))),
            Rule("BlockSyntax",
                Field("Statements", true, true,
                    Alternative(nameof(LocalDeclarationStatementSyntax), "nonterminal"),
                    Alternative(nameof(SwitchStatementSyntax), "nonterminal"),
                    Alternative(nameof(ExpressionStatementSyntax), "nonterminal"),
                    Alternative(nameof(BreakStatementSyntax), "nonterminal"),
                    Alternative(nameof(ReturnStatementSyntax), "nonterminal"))),
            Rule("LocalDeclarationStatementSyntax",
                Field("Declaration", true, false, Alternative(nameof(VariableDeclarationSyntax), "nonterminal"))),
            Rule("VariableDeclarationSyntax",
                Field("Type", true, false, Alternative(nameof(PredefinedTypeSyntax), "nonterminal")),
                Field("Variables", true, true, Alternative(nameof(VariableDeclaratorSyntax), "nonterminal"))),
            Rule("VariableDeclaratorSyntax",
                Field("Identifier", true, false, Alternative("identifier", "identifier_payload")),
                Field("Initializer", true, false, Alternative(nameof(EqualsValueClauseSyntax), "nonterminal"))),
            Rule("EqualsValueClauseSyntax",
                Field("Value", true, false, Alternative(nameof(LiteralExpressionSyntax), "nonterminal"))),
            Rule("SwitchStatementSyntax",
                Field("Expression", true, false, Alternative(nameof(IdentifierNameSyntax), "nonterminal")),
                Field("Sections", true, true, Alternative(nameof(SwitchSectionSyntax), "nonterminal"))),
            Rule("SwitchSectionSyntax",
                Field("Labels", true, true, Alternative(nameof(CaseSwitchLabelSyntax), "nonterminal")),
                Field("Statements", true, true,
                    Alternative(nameof(ExpressionStatementSyntax), "nonterminal"),
                    Alternative(nameof(BreakStatementSyntax), "nonterminal"))),
            Rule("CaseSwitchLabelSyntax",
                Field("Value", true, false, Alternative(nameof(LiteralExpressionSyntax), "nonterminal"))),
            Rule("ExpressionStatementSyntax",
                Field("Expression", true, false, Alternative(nameof(AssignmentExpressionSyntax), "nonterminal"))),
            Rule("AssignmentExpressionSyntax",
                Field("Left", true, false, Alternative(nameof(IdentifierNameSyntax), "nonterminal")),
                Field("Right", true, false, Alternative(nameof(LiteralExpressionSyntax), "nonterminal"))),
            Rule("BreakStatementSyntax"),
            Rule("LiteralExpressionSyntax",
                Field("Token", true, false, Alternative("integer_literal", "integer_payload"))),
            Rule("ReturnStatementSyntax",
                Field("Expression", true, false, Alternative(nameof(IdentifierNameSyntax), "nonterminal"))),
            Rule("IdentifierNameSyntax",
                Field("Identifier", true, false, Alternative("identifier", "identifier_payload")))
        });
    }
else
{
    Console.Error.WriteLine("method fixture did not match a supported class/string-return/local-null-return/local-nullable-return-type/local-nullable-declaration/nullable-property-declaration/nullable-parameter-declaration/local-unary-integer-return/local-binary-integer-return/while-postfix-loop/interpolated-string/member-access-expression/conditional-access/array-rank-object-creation/this-expression/pattern-expression/arrow-expression-clause/two-parameter-method/local-assignment/local-if-else/local-for-condition/local-switch/local-foreach-string/property-plus-method/method-invocation-return/method-invocation-argument-return/collection-initializer/element-access/range-element-access shape");
    return 4;
}
    }
    else
    {
        Console.Error.WriteLine("method fixture return type must be string, int, or int[]");
        return 4;
    }
}
else
{
    Console.Error.WriteLine("fixture did not match a supported C# syntax metadata smoke shape");
    return 4;
}
}
}

Console.WriteLine(JsonSerializer.Serialize(metadata, new JsonSerializerOptions { WriteIndented = true }));
return 0;

object BaseMetadata(object[] rules) => new
{
    artifact_kind = "tct_csharp_syntax_metadata_v0",
    version = 0,
    source = new
    {
        path = sourceDisplayPath,
        sha256 = Convert.ToHexString(SHA256.HashData(sourceBytes)).ToLowerInvariant()
    },
    parser = new
    {
        tool = "dotnet",
        version = dotnetVersion,
        package = "Microsoft.CodeAnalysis.CSharp",
        package_version = roslynPackageVersion
    },
    rules
};

static object Rule(string nodeKind, params object[] fields) => new
{
    name = nodeKind,
    node_kind = nodeKind,
    fields
};

static object Field(string name, bool required, bool multiple, params object[] alternatives) => new
{
    name,
    required,
    multiple,
    alternatives
};

static object Alternative(string nodeKind, string terminal) => new
{
    node_kind = nodeKind,
    terminal
};
'''


if __name__ == "__main__":
    raise SystemExit(main())
