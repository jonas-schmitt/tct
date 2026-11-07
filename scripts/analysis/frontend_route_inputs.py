#!/usr/bin/env python3
"""Preflight route-decision input readiness for retained non-Go frontend candidates.

Java and TypeScript deferred frontend investigation helpers were retired with
the 2026-07 audit cleanup. C# remains the only target here because it is the
active cross-target check for the shared syntax path.
"""

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
from typing import Any


@dataclass(frozen=True)
class TargetConfig:
    target: str
    label: str
    extension: str
    marker: str
    manifest_env: str
    manifest_report_env: str
    source_root_env: str
    tool_env: str
    tool_command: str
    tool_version_args: tuple[str, ...]
    tool_missing_kind: str
    manifest_missing_kind: str
    manifest_validity_kind: str
    manifest_marker_kind: str
    manifest_entries_kind: str
    representative_corpus_kind: str
    manifest_report_missing_kind: str


TARGETS = {
    "csharp": TargetConfig(
        target="csharp",
        label="C#",
        extension=".cs",
        marker="tct-csharp-route-corpus-v1",
        manifest_env="TCT_CSHARP_SOURCE_MANIFEST",
        manifest_report_env="TCT_CSHARP_SOURCE_MANIFEST_REPORT",
        source_root_env="TCT_CSHARP_SOURCE_ROOT",
        tool_env="TCT_DOTNET_BIN",
        tool_command="dotnet",
        tool_version_args=("--version",),
        tool_missing_kind="dotnet_toolchain_available",
        manifest_missing_kind="pinned_csharp_source_manifest",
        manifest_validity_kind="csharp_source_manifest_validity",
        manifest_marker_kind="csharp_route_decision_manifest_marker",
        manifest_entries_kind="csharp_source_manifest_entries",
        representative_corpus_kind="representative_csharp_source_corpus",
        manifest_report_missing_kind="csharp_source_manifest_report",
    ),
}

MIN_REPRESENTATIVE_SOURCE_ENTRIES = 2


def trim_manifest_line(raw_line: str) -> str:
    return raw_line.split("#", maxsplit=1)[0].strip()


def resolve_optional_path(value: str | None, repo_root: Path) -> Path | None:
    if not value:
        return None
    path = Path(value)
    if not path.is_absolute():
        path = repo_root / path
    return path


def path_is_checked_in_fixture(value: Any) -> bool:
    if not isinstance(value, str) or not value:
        return False
    parts = Path(value).parts
    return "tests" in parts and "fixtures" in parts


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


def collect_toolchain(config: TargetConfig, explicit_tool: str | None) -> dict[str, Any]:
    if explicit_tool:
        path = explicit_tool
        source = "argument"
    elif os.environ.get(config.tool_env):
        path = os.environ[config.tool_env]
        source = config.tool_env
    else:
        path = shutil.which(config.tool_command)
        source = "PATH" if path else None

    if not path:
        return {
            "available": False,
            "path": None,
            "source": None,
            "version": None,
            "error": (
                f"{config.tool_command} not found on PATH and "
                f"{config.tool_env} is unset"
            ),
        }

    try:
        completed = subprocess.run(
            [path, *config.tool_version_args],
            check=False,
            capture_output=True,
            text=True,
        )
    except OSError as error:
        return {
            "available": False,
            "path": path,
            "source": source,
            "version": None,
            "error": str(error),
        }

    version = (completed.stdout or completed.stderr).strip() or None
    return {
        "available": completed.returncode == 0,
        "path": path,
        "source": source,
        "version": version,
        "error": None if completed.returncode == 0 else f"exit code {completed.returncode}",
    }


def collect_source_manifest(
    config: TargetConfig,
    manifest_path: Path | None,
    source_root: Path,
) -> dict[str, Any]:
    report: dict[str, Any] = {
        "path": manifest_path.as_posix() if manifest_path else None,
        "source_root": source_root.as_posix(),
        "present": False,
        "route_decision_marker": config.marker,
        "route_decision_marker_present": False,
        "entry_count": 0,
        "min_representative_entries": MIN_REPRESENTATIVE_SOURCE_ENTRIES,
        "representative_entry_count_ready": False,
        "corpus_fingerprint": None,
        "fixture_scope": False,
        "evidence_role": "unknown",
        "entries": [],
        "errors": [],
    }
    if manifest_path is None:
        report["errors"].append("source manifest not provided")
        return report
    if not manifest_path.is_file():
        report["errors"].append(f"source manifest not found: {manifest_path}")
        return report

    report["present"] = True
    report["manifest"] = file_artifact(manifest_path)
    report["fixture_scope"] = path_is_checked_in_fixture(
        report["manifest"]["path"]
    ) or path_is_checked_in_fixture(report["source_root"])
    report["evidence_role"] = (
        "checked_in_fixture_smoke"
        if report["fixture_scope"]
        else "route_intended_corpus"
    )
    seen_entries: set[str] = set()
    for line_number, raw_line in enumerate(
        manifest_path.read_text(encoding="utf-8").splitlines(),
        start=1,
    ):
        stripped = raw_line.strip()
        if stripped.startswith("#") and config.marker in stripped:
            report["route_decision_marker_present"] = True
        source_spec = trim_manifest_line(raw_line)
        if not source_spec:
            continue
        source_path = Path(source_spec)
        if source_path.is_absolute():
            report["errors"].append(
                f"{manifest_path}:{line_number}: source path must be relative: "
                f"{source_spec}"
            )
            continue
        if source_spec in seen_entries:
            report["errors"].append(
                f"{manifest_path}:{line_number}: duplicate source entry: "
                f"{source_spec}"
            )
            continue
        seen_entries.add(source_spec)
        if source_path.suffix != config.extension:
            report["errors"].append(
                f"{manifest_path}:{line_number}: expected {config.extension} file: "
                f"{source_spec}"
            )
            continue
        absolute_source = source_root / source_path
        if not absolute_source.is_file():
            report["errors"].append(
                f"{manifest_path}:{line_number}: source file not found: "
                f"{source_spec}"
            )
            continue
        report["entries"].append(
            {
                "manifest_path": source_spec,
                "line": line_number,
                "file": file_artifact(absolute_source, source_spec),
            }
        )

    report["entry_count"] = len(report["entries"])
    report["representative_entry_count_ready"] = (
        report["entry_count"] >= MIN_REPRESENTATIVE_SOURCE_ENTRIES
    )
    report["corpus_fingerprint"] = corpus_fingerprint(report["entries"])
    return report


def collect_source_manifest_report(
    config: TargetConfig,
    report_path: Path | None,
    source_manifest: dict[str, Any],
) -> dict[str, Any]:
    report: dict[str, Any] = {
        "path": report_path.as_posix() if report_path else None,
        "present": False,
        "valid": False,
        "artifact": None,
        "target": None,
        "output": None,
        "corpus_fingerprint": None,
        "corpus_provenance": None,
        "errors": [],
    }
    if report_path is None:
        return report
    if not report_path.is_file():
        report["errors"].append(f"source manifest report not found: {report_path}")
        return report

    report["present"] = True
    report["artifact"] = file_artifact(report_path)
    try:
        data = json.loads(report_path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        report["errors"].append(f"invalid source manifest report JSON: {error}")
        return report
    if not isinstance(data, dict):
        report["errors"].append("source manifest report must be a JSON object")
        return report

    report["target"] = data.get("target")
    report["output"] = data.get("output")
    report["corpus_fingerprint"] = data.get("corpus_fingerprint")
    report["corpus_provenance"] = data.get("corpus_provenance")

    if data.get("tool") != "route_source_manifest":
        report["errors"].append("source manifest report tool is not route_source_manifest")
    if data.get("version") != 1:
        report["errors"].append("source manifest report version is not 1")
    if data.get("target") != config.target:
        report["errors"].append(
            f"source manifest report target is not {config.target}"
        )

    manifest_artifact = data.get("manifest")
    if not isinstance(manifest_artifact, dict):
        report["errors"].append("source manifest report manifest artifact is missing")
    elif source_manifest.get("manifest") and (
        manifest_artifact.get("sha256")
        != source_manifest["manifest"].get("sha256")
    ):
        report["errors"].append("source manifest report manifest hash mismatch")

    if data.get("corpus_fingerprint") != source_manifest.get("corpus_fingerprint"):
        report["errors"].append("source manifest report corpus fingerprint mismatch")
    provenance = data.get("corpus_provenance")
    if not isinstance(provenance, dict):
        report["errors"].append("source manifest report corpus provenance is missing")
    else:
        missing_provenance = [
            key
            for key in ("corpus_id", "source", "license")
            if not str(provenance.get(key) or "").strip()
        ]
        if provenance.get("provided") is not True or missing_provenance:
            missing_text = ", ".join(missing_provenance) or "provided"
            report["errors"].append(
                "source manifest report corpus provenance is incomplete: "
                f"{missing_text}"
            )

    report["valid"] = not report["errors"]
    return report


def missing_inputs(
    config: TargetConfig,
    toolchain: dict[str, Any],
    source_manifest: dict[str, Any],
    source_manifest_report: dict[str, Any],
) -> list[dict[str, str]]:
    missing: list[dict[str, str]] = []
    if not toolchain["available"]:
        missing.append(
            {
                "kind": config.tool_missing_kind,
                "status": "missing_or_unusable",
                "required_before": "frontend_route_decision_evidence",
            }
        )
    if not source_manifest["present"]:
        missing.append(
            {
                "kind": config.manifest_missing_kind,
                "status": "missing",
                "required_before": "frontend_route_decision_evidence",
            }
        )
    elif source_manifest["errors"]:
        missing.append(
            {
                "kind": config.manifest_validity_kind,
                "status": "invalid",
                "required_before": "frontend_route_decision_evidence",
            }
        )
    if source_manifest["present"] and not source_manifest["route_decision_marker_present"]:
        missing.append(
            {
                "kind": config.manifest_marker_kind,
                "status": "missing",
                "required_before": "frontend_route_decision_evidence",
            }
        )
    if source_manifest["present"] and int(source_manifest["entry_count"]) == 0:
        missing.append(
            {
                "kind": config.manifest_entries_kind,
                "status": "missing",
                "required_before": "frontend_route_decision_evidence",
            }
        )
    if source_manifest["present"] and source_manifest.get("fixture_scope"):
        missing.append(
            {
                "kind": config.representative_corpus_kind,
                "status": "missing",
                "required_before": "frontend_route_decision_evidence",
            }
        )
    elif (
        source_manifest["present"]
        and int(source_manifest["entry_count"]) > 0
        and not source_manifest.get("representative_entry_count_ready")
    ):
        missing.append(
            {
                "kind": config.representative_corpus_kind,
                "status": "insufficient_entries",
                "required_before": "frontend_route_decision_evidence",
            }
        )
    route_intended_manifest_ready = (
        source_manifest["present"]
        and not source_manifest["errors"]
        and not source_manifest.get("fixture_scope")
        and source_manifest.get("representative_entry_count_ready")
    )
    if route_intended_manifest_ready and not source_manifest_report["present"]:
        missing.append(
            {
                "kind": config.manifest_report_missing_kind,
                "status": "missing",
                "required_before": "frontend_route_decision_evidence",
            }
        )
    if source_manifest_report["present"] and source_manifest_report["errors"]:
        missing.append(
            {
                "kind": f"{config.target}_source_manifest_report_validity",
                "status": "invalid",
                "required_before": "frontend_route_decision_evidence",
            }
        )
    return missing


def build_report(
    config: TargetConfig,
    source_manifest_path: Path | None,
    source_manifest_report_path: Path | None,
    source_root: Path,
    tool_bin: str | None,
) -> dict[str, Any]:
    toolchain = collect_toolchain(config, tool_bin)
    source_manifest = collect_source_manifest(config, source_manifest_path, source_root)
    source_manifest_report = collect_source_manifest_report(
        config,
        source_manifest_report_path,
        source_manifest,
    )
    missing = missing_inputs(config, toolchain, source_manifest, source_manifest_report)
    return {
        "tool": "frontend_route_inputs",
        "version": 1,
        "target": config.target,
        "target_label": config.label,
        "inputs_ready": not missing,
        "missing_inputs": missing,
        "toolchain": toolchain,
        "source_manifest": source_manifest,
        "source_manifest_report": source_manifest_report,
        "evidence_boundary": (
            "input preflight only; does not run lexical analysis, parser or "
            "compiler smoke, generated TCT round trip, route selection, or "
            "paper evidence"
        ),
    }


def render_markdown(report: dict[str, Any]) -> str:
    toolchain = report["toolchain"]
    source_manifest = report["source_manifest"]
    source_manifest_report = report["source_manifest_report"]
    missing = report["missing_inputs"]
    manifest_ready = (
        source_manifest["present"]
        and not source_manifest["errors"]
        and int(source_manifest["entry_count"]) > 0
        and bool(source_manifest.get("representative_entry_count_ready"))
        and not source_manifest.get("fixture_scope")
    )
    lines = [
        f"# {report['target_label']} Route Input Preflight",
        "",
        "This report checks inputs only. It is not frontend route evidence.",
        "",
        f"Inputs ready: `{report['inputs_ready']}`",
        "",
        "| Input | Ready | Detail |",
        "| --- | ---: | --- |",
        "| `toolchain` | "
        f"{toolchain['available']} | "
        f"{toolchain.get('version') or toolchain.get('error') or 'ok'} |",
        "| `source_manifest` | "
        f"{manifest_ready} | "
        f"{source_manifest['entry_count']} entries, "
        f"min {source_manifest.get('min_representative_entries')}, "
        f"{source_manifest.get('evidence_role')} |",
        "| `route_marker` | "
        f"{source_manifest['route_decision_marker_present']} | "
        f"{source_manifest['route_decision_marker']} |",
        "| `source_manifest_report` | "
        f"{source_manifest_report['valid']} | "
        f"{source_manifest_report.get('path') or 'not provided'} |",
        "",
    ]
    if missing:
        lines.append("Missing inputs:")
        lines.append("")
        for item in missing:
            lines.append(f"- `{item['kind']}`: {item['status']}")
        lines.append("")
    return "\n".join(lines)


def self_test() -> None:
    with tempfile.TemporaryDirectory() as tmpdir:
        tmp = Path(tmpdir)
        source = tmp / "Sample.cs"
        second_source = tmp / "Second.cs"
        manifest = tmp / "sources.txt"
        fake_dotnet = tmp / "dotnet"
        source.write_text("class Sample {}\n", encoding="utf-8")
        second_source.write_text("class Second {}\n", encoding="utf-8")
        manifest.write_text(
            "# tct-csharp-route-corpus-v1\nSample.cs\nSecond.cs\n",
            encoding="utf-8",
        )
        source_manifest = collect_source_manifest(TARGETS["csharp"], manifest, tmp)
        manifest_report = tmp / "sources.manifest.json"
        manifest_report.write_text(
            json.dumps(
                {
                    "tool": "route_source_manifest",
                    "version": 1,
                    "target": "csharp",
                    "manifest": source_manifest["manifest"],
                    "corpus_fingerprint": source_manifest["corpus_fingerprint"],
                    "corpus_provenance": {
                        "provided": True,
                        "corpus_id": "self-test",
                        "source": "temporary self-test corpus",
                        "license": "test-only",
                    },
                }
            ),
            encoding="utf-8",
        )
        fake_dotnet.write_text(
            "#!/usr/bin/env sh\necho '8.0.100'\n", encoding="utf-8"
        )
        fake_dotnet.chmod(fake_dotnet.stat().st_mode | 0o100)
        report = build_report(
            TARGETS["csharp"],
            manifest,
            manifest_report,
            tmp,
            fake_dotnet.as_posix(),
        )
    assert report["inputs_ready"] is True
    assert report["source_manifest"]["entry_count"] == 2


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--target", choices=sorted(TARGETS), help="Frontend target")
    parser.add_argument(
        "--source-manifest",
        type=Path,
        help="Pinned source manifest; defaults to target-specific env var",
    )
    parser.add_argument(
        "--source-root",
        type=Path,
        help="Base directory for relative source manifest entries",
    )
    parser.add_argument(
        "--source-manifest-report",
        type=Path,
        help=(
            "route_source_manifest JSON report to validate against the source "
            "manifest fingerprint and corpus provenance; required before "
            "--require-inputs-ready can pass for route-intended non-fixture "
            "manifests"
        ),
    )
    parser.add_argument(
        "--tool-bin",
        help="Toolchain binary path; defaults to target-specific env var or PATH",
    )
    parser.add_argument("--json-output", type=Path, help="Write JSON report")
    parser.add_argument("--markdown-output", type=Path, help="Write Markdown report")
    parser.add_argument(
        "--require-inputs-ready",
        action="store_true",
        help="Exit nonzero unless all route inputs are ready",
    )
    parser.add_argument("--self-test", action="store_true", help="Run built-in checks")
    return parser.parse_args(argv)


def main(argv: list[str]) -> int:
    args = parse_args(argv)
    if args.self_test:
        self_test()
        print("frontend_route_inputs self-test passed")
        return 0
    if not args.target:
        raise SystemExit("error: --target is required unless --self-test is used")

    config = TARGETS[args.target]
    repo_root = Path(".")
    manifest = args.source_manifest or resolve_optional_path(
        os.environ.get(config.manifest_env),
        repo_root,
    )
    manifest_report = args.source_manifest_report or resolve_optional_path(
        os.environ.get(config.manifest_report_env),
        repo_root,
    )
    source_root = args.source_root or resolve_optional_path(
        os.environ.get(config.source_root_env),
        repo_root,
    )
    if source_root is None:
        source_root = repo_root

    report = build_report(config, manifest, manifest_report, source_root, args.tool_bin)
    json_text = json.dumps(report, indent=2, sort_keys=True) + "\n"
    if args.json_output:
        args.json_output.parent.mkdir(parents=True, exist_ok=True)
        args.json_output.write_text(json_text, encoding="utf-8")
    if args.markdown_output:
        args.markdown_output.parent.mkdir(parents=True, exist_ok=True)
        args.markdown_output.write_text(render_markdown(report), encoding="utf-8")
    if not args.json_output and not args.markdown_output:
        print(json_text, end="")
    if args.require_inputs_ready and not report["inputs_ready"]:
        missing = ", ".join(item["kind"] for item in report["missing_inputs"])
        print(
            f"{config.label} route inputs are incomplete: {missing}",
            file=sys.stderr,
        )
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
