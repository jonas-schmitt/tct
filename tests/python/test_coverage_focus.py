#!/usr/bin/env python3
"""Tests for the report-only Rust coverage focus helper."""

import json
import subprocess
import sys
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[2]
COVERAGE_FOCUS = REPO_ROOT / "scripts/testing/coverage-focus.py"


def file_entry(path: Path, lines=(4, 5), functions=(1, 2), regions=(6, 8)):
    return {
        "filename": str(path),
        "summary": {
            "lines": {"covered": lines[0], "count": lines[1]},
            "functions": {"covered": functions[0], "count": functions[1]},
            "regions": {"covered": regions[0], "count": regions[1]},
        },
    }


def write_summary(tmp_path: Path) -> Path:
    summary = {
        "data": [
            {
                "files": [
                    file_entry(REPO_ROOT / "src/base256.rs"),
                    file_entry(REPO_ROOT / "src/bpe/encoder.rs", lines=(7, 10)),
                ]
            }
        ]
    }
    path = tmp_path / "summary.json"
    path.write_text(json.dumps(summary))
    return path


def test_coverage_focus_reports_files_directories_and_missing_paths(tmp_path):
    summary = write_summary(tmp_path)
    output = tmp_path / "focused-summary.txt"

    completed = subprocess.run(
        [
            sys.executable,
            str(COVERAGE_FOCUS),
            str(summary),
            "--repo-root",
            str(REPO_ROOT),
            "--output",
            str(output),
            "src/base256.rs",
            "src/bpe",
            "tests/not-in-coverage.rs",
        ],
        check=True,
        capture_output=True,
        text=True,
    )

    assert completed.stdout == output.read_text()
    assert "src/base256.rs" in completed.stdout
    assert "src/bpe/encoder.rs" in completed.stdout
    assert "TOTAL" in completed.stdout
    assert "tests/not-in-coverage.rs" in completed.stdout
    assert "Paths without coverage rows" in completed.stdout
