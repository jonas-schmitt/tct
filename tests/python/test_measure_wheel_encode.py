#!/usr/bin/env python3
"""Tests for the generated-wheel encode measurement helper."""

import json
import subprocess
import sys
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[2]
SCRIPT = REPO_ROOT / "scripts/benchmarking/measure_wheel_encode.py"


def write_fake_module(tmp_path: Path, *, encode_returns: str = "list") -> Path:
    if encode_returns == "list":
        encode_body = """
def encode(json_str):
    value = json.loads(json_str)
    if value.get("fail"):
        raise ValueError("intentional schema failure")
    return [len(json_str), len(value)]
"""
    elif encode_returns == "tuple":
        # Non-list returns must fail outside the timed encode window.
        encode_body = """
def encode(json_str):
    value = json.loads(json_str)
    if value.get("fail"):
        raise ValueError("intentional schema failure")
    return (len(json_str), len(value))
"""
    else:
        raise ValueError(f"unknown encode_returns={encode_returns!r}")

    module = tmp_path / "fake_tct_wheel.py"
    module.write_text(
        f"""
import json

{encode_body}

def decode(tokens):
    return ("{{}}", len(tokens), len(tokens))


def vocab_size():
    return 1234


def pad_token():
    return 1233
""",
        encoding="utf-8",
    )
    return module


def test_measure_wheel_encode_reports_provenance_and_timing(tmp_path):
    write_fake_module(tmp_path)
    instances = tmp_path / "instances"
    instances.mkdir()
    (instances / "valid-one.json").write_text('{"name": "one"}', encoding="utf-8")
    (instances / "valid-two.json").write_text('{"name": "two"}', encoding="utf-8")
    (instances / "invalid.json").write_text('{"fail": true}', encoding="utf-8")
    output = tmp_path / "report.json"

    completed = subprocess.run(
        [
            sys.executable,
            str(SCRIPT),
            "--module",
            "fake_tct_wheel",
            "--pythonpath",
            str(tmp_path),
            "--instances",
            str(instances / "*.json"),
            "--iterations",
            "2",
            "--warmup-iterations",
            "1",
            "--decode",
            "--output",
            str(output),
        ],
        check=True,
        capture_output=True,
        text=True,
    )

    report = json.loads(completed.stdout)
    assert json.loads(output.read_text(encoding="utf-8")) == report
    assert report["scope"] == "python_wheel_encode"
    assert report["module"] == "fake_tct_wheel"
    assert report["instances"] == 3
    assert report["successful_instances"] == 2
    assert report["failed_instances"] == 1
    assert report["iterations"] == 2
    assert report["warmup_iterations"] == 1
    assert report["measured_encode_calls"] == 4
    assert report["measured_decode_calls"] == 4
    assert report["decode_enabled"] is True
    assert report["vocab_size"] == 1234
    assert report["pad_token"] == 1233
    assert len(report["provenance"]["input_files"]) == 2
    assert report["failure_samples"][0]["path"].endswith("invalid.json")
    assert report["timings"]["wheel_encode_wall_time_ms"] >= 0


def test_measure_wheel_encode_rejects_empty_instance_glob(tmp_path):
    write_fake_module(tmp_path)

    completed = subprocess.run(
        [
            sys.executable,
            str(SCRIPT),
            "--module",
            "fake_tct_wheel",
            "--pythonpath",
            str(tmp_path),
            "--instances",
            str(tmp_path / "missing" / "*.json"),
        ],
        capture_output=True,
        text=True,
    )

    assert completed.returncode == 2
    assert "no instance files matched" in completed.stderr


def test_measure_wheel_encode_accepts_generated_measured_file_manifest(tmp_path):
    write_fake_module(tmp_path)
    instances = tmp_path / "instances"
    instances.mkdir()
    valid_one = instances / "valid-one.json"
    valid_two = instances / "valid-two.json"
    invalid = instances / "invalid.json"
    valid_one.write_text('{"name": "one"}', encoding="utf-8")
    valid_two.write_text('{"name": "two"}', encoding="utf-8")
    invalid.write_text('{"fail": true}', encoding="utf-8")
    manifest = tmp_path / "measured-files.txt"
    manifest.write_text(f"{valid_two}\n{valid_one}\n", encoding="utf-8")

    completed = subprocess.run(
        [
            sys.executable,
            str(SCRIPT),
            "--module",
            "fake_tct_wheel",
            "--pythonpath",
            str(tmp_path),
            "--instances-file",
            str(manifest),
        ],
        check=True,
        capture_output=True,
        text=True,
    )

    report = json.loads(completed.stdout)
    assert report["instances"] == 2
    assert report["successful_instances"] == 2
    assert report["failed_instances"] == 0
    assert report["provenance"]["instances_file"] == str(manifest)
    paths = [entry["path"] for entry in report["provenance"]["input_files"]]
    assert paths == sorted([str(valid_one), str(valid_two)])


def test_measure_wheel_encode_normalizes_after_timed_window(tmp_path):
    """Token-list validation must not abort the timed encode path mid-run."""
    write_fake_module(tmp_path, encode_returns="tuple")
    instances = tmp_path / "instances"
    instances.mkdir()
    (instances / "valid.json").write_text('{"name": "one"}', encoding="utf-8")

    completed = subprocess.run(
        [
            sys.executable,
            str(SCRIPT),
            "--module",
            "fake_tct_wheel",
            "--pythonpath",
            str(tmp_path),
            "--instances",
            str(instances / "*.json"),
            "--iterations",
            "1",
        ],
        capture_output=True,
        text=True,
    )

    # Encode succeeded (return value captured) but post-window normalize rejects
    # non-list tokens, so the run reports no successful sequences.
    assert completed.returncode == 1
    assert "no instances were successfully encoded" in completed.stderr
