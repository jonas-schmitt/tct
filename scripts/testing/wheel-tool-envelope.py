#!/usr/bin/env python3
"""Record the Python packaging tool envelope used by wheel smoke tests."""

from __future__ import annotations

import argparse
import importlib.metadata
import json
import platform
import re
import shutil
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path
from typing import Any


MANIFEST_VERSION = 1
ARTIFACT_KIND = "tct_python_wheel_tool_envelope"
TOOL_NAME = "wheel-tool-envelope"
BUILD_INPUT_MARKERS = {
    "generated_schema_wheel": {
        "function": "generate_pyproject_toml",
        "requires": ["maturin>=1.0,<2.0"],
        "build_backend": "maturin",
    },
    "bundle_wrapper_wheel": {
        "function": "generate_bundle_pyproject_toml",
        "requires": ["setuptools>=61"],
        "build_backend": "setuptools.build_meta",
    },
}
REQUIRED_WHEEL_BUILD_TOOLS = {
    "maturin": "generated schema wheel builds",
    "setuptools": "bundle wrapper wheel builds",
    "wheel": "bundle wrapper wheel builds",
}


def repo_relative(repo_root: Path, path: Path) -> str:
    try:
        return path.resolve().relative_to(repo_root.resolve()).as_posix()
    except ValueError:
        return path.as_posix()


def git_value(repo_root: Path, *args: str) -> str | None:
    try:
        completed = subprocess.run(
            ["git", "-C", str(repo_root), *args],
            check=True,
            capture_output=True,
            text=True,
        )
    except (OSError, subprocess.CalledProcessError):
        return None
    return completed.stdout.strip()


def command_probe(command: list[str]) -> dict[str, Any]:
    executable = shutil.which(command[0])
    if executable is None:
        return {
            "command": command,
            "available": False,
            "executable": None,
            "returncode": None,
            "stdout": "",
            "stderr": "",
        }

    completed = subprocess.run(command, capture_output=True, text=True)
    return {
        "command": command,
        "available": completed.returncode == 0,
        "executable": executable,
        "returncode": completed.returncode,
        "stdout": completed.stdout.strip(),
        "stderr": completed.stderr.strip(),
    }


def package_version(name: str) -> dict[str, Any]:
    try:
        version = importlib.metadata.version(name)
    except importlib.metadata.PackageNotFoundError:
        return {"name": name, "available": False, "version": None}
    return {"name": name, "available": True, "version": version}


def function_source(source: str, function_name: str) -> str:
    start = source.find(f"fn {function_name}(")
    if start == -1:
        return ""
    end = source.find("\nfn ", start + 1)
    return source[start:] if end == -1 else source[start:end]


def extract_pyproject_templates(repo_root: Path) -> dict[str, Any]:
    source_path = repo_root / "src/wheel_builder.rs"
    if not source_path.is_file():
        return {
            "source": repo_relative(repo_root, source_path),
            "readable": False,
            "templates": {},
        }

    source = source_path.read_text(encoding="utf-8")
    templates = {}
    for name, expected in BUILD_INPUT_MARKERS.items():
        template_source = function_source(source, expected["function"])
        requires_literal = ", ".join(json.dumps(item) for item in expected["requires"])
        requires_pattern = rf"requires\s*=\s*\[\s*{re.escape(requires_literal)}\s*\]"
        backend_pattern = (
            r"build-backend\s*=\s*"
            + re.escape(json.dumps(expected["build_backend"]))
        )
        requires_present = re.search(requires_pattern, template_source) is not None
        backend_present = re.search(backend_pattern, template_source) is not None
        templates[name] = {
            "function": expected["function"],
            "requires": expected["requires"] if requires_present else [],
            "build_backend": expected["build_backend"] if backend_present else None,
            "expected_requires": expected["requires"],
            "expected_build_backend": expected["build_backend"],
            "requires_marker_present": requires_present,
            "build_backend_marker_present": backend_present,
        }
    return {
        "source": repo_relative(repo_root, source_path),
        "readable": True,
        "templates": templates,
    }


def build_report(repo_root: Path) -> dict[str, Any]:
    repo_status = git_value(repo_root, "status", "--porcelain")
    return {
        "manifest_version": MANIFEST_VERSION,
        "artifact_kind": ARTIFACT_KIND,
        "tool": TOOL_NAME,
        "created_at": datetime.now(timezone.utc).isoformat(),
        "repo_root": str(repo_root),
        "repo_commit": git_value(repo_root, "rev-parse", "HEAD"),
        "repo_dirty": None if repo_status is None else bool(repo_status),
        "python": {
            "executable": sys.executable,
            "version": sys.version,
            "implementation": platform.python_implementation(),
            "platform": platform.platform(),
            "prefix": sys.prefix,
            "base_prefix": sys.base_prefix,
        },
        "tools": {
            "pip": command_probe([sys.executable, "-m", "pip", "--version"]),
            "maturin": command_probe(["maturin", "--version"]),
            "setuptools": package_version("setuptools"),
            "wheel": package_version("wheel"),
        },
        "build_inputs": extract_pyproject_templates(repo_root),
    }


def missing_required_wheel_build_tools(report: dict[str, Any]) -> list[dict[str, str]]:
    tools = report.get("tools", {})
    missing = []
    for tool, purpose in REQUIRED_WHEEL_BUILD_TOOLS.items():
        if not tools.get(tool, {}).get("available"):
            missing.append({"tool": tool, "purpose": purpose})
    return missing


def format_missing_required_wheel_build_tools(
    missing_tools: list[dict[str, str]], output_path: str | None
) -> str:
    tool_list = ", ".join(
        f"{entry['tool']} ({entry['purpose']})" for entry in missing_tools
    )
    envelope_note = (
        f" Wrote the detected tool envelope to {output_path}."
        if output_path
        else " The detected tool envelope was printed to stdout."
    )
    return (
        "Missing required Python wheel build tool(s): "
        f"{tool_list}.{envelope_note} Install the missing tool(s) in the active "
        "Python environment before running scripts/testing/test-wheel-builds.sh."
    )


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Record the Python packaging tool envelope for TCT wheel smoke tests."
    )
    parser.add_argument(
        "--repo-root",
        default=".",
        help="Repository root used for git metadata and build-template markers.",
    )
    parser.add_argument(
        "--output",
        help="Optional JSON output path. The report is always printed to stdout.",
    )
    parser.add_argument(
        "--require-wheel-build-tools",
        action="store_true",
        help=(
            "Exit nonzero after writing the report when maturin, setuptools, or "
            "wheel are unavailable for the maintained wheel smoke."
        ),
    )
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    repo_root = Path(args.repo_root).resolve()
    report = build_report(repo_root)
    rendered = json.dumps(report, indent=2, sort_keys=True)
    if args.output:
        output_path = Path(args.output)
        output_path.parent.mkdir(parents=True, exist_ok=True)
        output_path.write_text(rendered + "\n", encoding="utf-8")
    exit_code = 0
    if args.require_wheel_build_tools:
        missing_tools = missing_required_wheel_build_tools(report)
        if missing_tools:
            sys.stderr.write(
                format_missing_required_wheel_build_tools(missing_tools, args.output)
                + "\n"
            )
            exit_code = 2
    try:
        sys.stdout.write(rendered + "\n")
    except BrokenPipeError:
        return 0
    return exit_code


if __name__ == "__main__":
    sys.exit(main())
