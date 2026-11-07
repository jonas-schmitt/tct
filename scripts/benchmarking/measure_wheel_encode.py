#!/usr/bin/env python3
"""Measure encode timing for an importable generated TCT Python wheel.

The script intentionally measures an already importable module. Building and
installing generated wheels remains an explicit external-tooling step because it
depends on Python, pip, and maturin availability.
"""

from __future__ import annotations

import argparse
import datetime as dt
import glob
import hashlib
import importlib
import json
import sys
import time
from pathlib import Path
from typing import Any


def main() -> int:
    args = parse_args()

    for path in reversed(args.pythonpath):
        sys.path.insert(0, path)

    try:
        module = importlib.import_module(args.module)
    except Exception as error:  # pragma: no cover - exercised through CLI tests
        print(f"error: failed to import module {args.module!r}: {error}", file=sys.stderr)
        return 2

    if not hasattr(module, "encode"):
        print(f"error: module {args.module!r} does not expose encode()", file=sys.stderr)
        return 2

    input_load_start = time.perf_counter()
    instance_paths = discover_instances(args.instances, args.instances_file)
    if not instance_paths:
        patterns = ", ".join(args.instances)
        print(f"error: no instance files matched: {patterns}", file=sys.stderr)
        return 2
    instances = [(path, path.read_text(encoding="utf-8")) for path in instance_paths]
    input_load_time_ms = elapsed_ms(input_load_start)

    warmup_start = time.perf_counter()
    for _ in range(args.warmup_iterations):
        for _, content in instances:
            try:
                module.encode(content)
            except Exception:
                pass
    warmup_time_ms = elapsed_ms(warmup_start)

    encode_start = time.perf_counter()
    # Keep first-pass raw return values for validation *after* the timed window
    # so normalize_tokens / list conversion is not counted as encode cost.
    first_pass_raw: list[tuple[Path, Any]] = []
    failure_samples: list[dict[str, str]] = []
    measured_encode_calls = 0

    for iteration in range(args.iterations):
        for path, content in instances:
            try:
                tokens = module.encode(content)
                measured_encode_calls += 1
            except Exception as error:
                if iteration == 0 and len(failure_samples) < args.failure_sample_limit:
                    failure_samples.append({"path": str(path), "error": str(error)})
                continue

            if iteration == 0:
                first_pass_raw.append((path, tokens))
    encode_time_ms = elapsed_ms(encode_start)

    first_pass_sequences: list[tuple[Path, list[int]]] = []
    for path, tokens in first_pass_raw:
        try:
            first_pass_sequences.append((path, normalize_tokens(tokens)))
        except (TypeError, ValueError) as error:
            if len(failure_samples) < args.failure_sample_limit:
                failure_samples.append({"path": str(path), "error": str(error)})

    if not first_pass_sequences:
        print("error: no instances were successfully encoded", file=sys.stderr)
        return 1

    decode_time_ms: int | None = None
    decode_failures: list[dict[str, str]] = []
    measured_decode_calls = 0
    if args.decode:
        if not hasattr(module, "decode"):
            print(f"error: module {args.module!r} does not expose decode()", file=sys.stderr)
            return 2
        decode_start = time.perf_counter()
        for _ in range(args.iterations):
            for path, tokens in first_pass_sequences:
                try:
                    module.decode(tokens)
                    measured_decode_calls += 1
                except Exception as error:
                    if len(decode_failures) < args.failure_sample_limit:
                        decode_failures.append({"path": str(path), "error": str(error)})
        decode_time_ms = elapsed_ms(decode_start)

    successful_paths = [path for path, _ in first_pass_sequences]
    report = {
        "scope": "python_wheel_encode",
        "module": args.module,
        "module_file": getattr(module, "__file__", None),
        "tokens": sum(len(tokens) for _, tokens in first_pass_sequences),
        "instances": len(instances),
        "successful_instances": len(first_pass_sequences),
        "failed_instances": len(instances) - len(first_pass_sequences),
        "characters": sum(path.stat().st_size for path in successful_paths),
        "iterations": args.iterations,
        "warmup_iterations": args.warmup_iterations,
        "measured_encode_calls": measured_encode_calls,
        "measured_decode_calls": measured_decode_calls,
        "decode_enabled": args.decode,
        "vocab_size": call_optional_u32(module, "vocab_size"),
        "pad_token": call_optional_u32(module, "pad_token"),
        "timings": {
            "input_load_time_ms": input_load_time_ms,
            "wheel_warmup_wall_time_ms": warmup_time_ms,
            "wheel_encode_wall_time_ms": encode_time_ms,
            "wheel_decode_wall_time_ms": decode_time_ms,
        },
        "failure_samples": failure_samples,
        "decode_failure_samples": decode_failures,
        "provenance": {
            "command": sys.argv,
            "module": args.module,
            "module_file": getattr(module, "__file__", None),
            "instances_patterns": args.instances,
            "instances_file": str(args.instances_file) if args.instances_file else None,
            "input_files": [file_provenance(path) for path in successful_paths],
            "created_at": dt.datetime.now(dt.timezone.utc).isoformat(),
            "python_version": sys.version,
        },
    }

    output = json.dumps(report, indent=2, sort_keys=True)
    print(output)
    if args.output:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(output + "\n", encoding="utf-8")

    return 0


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Measure encode timing for an importable generated TCT Python wheel."
    )
    parser.add_argument(
        "--module",
        required=True,
        help="Import name of the generated wheel module, for example tct_kubernetes.",
    )
    parser.add_argument(
        "--instances",
        action="append",
        default=[],
        help="Glob pattern for JSON instance files. May be passed more than once.",
    )
    parser.add_argument(
        "--instances-file",
        type=Path,
        help="File containing one JSON instance path per line.",
    )
    parser.add_argument(
        "--pythonpath",
        action="append",
        default=[],
        help="Path to prepend to sys.path before importing the module.",
    )
    parser.add_argument(
        "--iterations",
        type=positive_int,
        default=1,
        help="Measured encode passes over the matched instances.",
    )
    parser.add_argument(
        "--warmup-iterations",
        type=non_negative_int,
        default=0,
        help="Unmeasured warmup encode passes before timing.",
    )
    parser.add_argument(
        "--decode",
        action="store_true",
        help="Also time decode() on successfully encoded first-pass token sequences.",
    )
    parser.add_argument(
        "--failure-sample-limit",
        type=non_negative_int,
        default=20,
        help="Maximum encode/decode failures to include in the JSON report.",
    )
    parser.add_argument(
        "--output",
        type=Path,
        help="Optional JSON output path. The report is always printed to stdout.",
    )
    args = parser.parse_args()
    if not args.instances and args.instances_file is None:
        parser.error("at least one --instances pattern or --instances-file is required")
    return args


def positive_int(value: str) -> int:
    parsed = int(value)
    if parsed < 1:
        raise argparse.ArgumentTypeError("must be >= 1")
    return parsed


def non_negative_int(value: str) -> int:
    parsed = int(value)
    if parsed < 0:
        raise argparse.ArgumentTypeError("must be >= 0")
    return parsed


def discover_instances(patterns: list[str], instances_file: Path | None) -> list[Path]:
    paths: list[Path] = []
    seen: set[Path] = set()
    if instances_file is not None:
        for line in instances_file.read_text(encoding="utf-8").splitlines():
            stripped = line.strip()
            if not stripped:
                continue
            path = Path(stripped)
            if not path.is_file():
                continue
            resolved = path.resolve()
            if resolved in seen:
                continue
            seen.add(resolved)
            paths.append(path)
    for pattern in patterns:
        for match in glob.glob(pattern, recursive=True):
            path = Path(match)
            if not path.is_file():
                continue
            resolved = path.resolve()
            if resolved in seen:
                continue
            seen.add(resolved)
            paths.append(path)
    return sorted(paths)


def normalize_tokens(tokens: Any) -> list[int]:
    if not isinstance(tokens, list):
        raise TypeError(f"encode() returned {type(tokens).__name__}, expected list[int]")
    normalized: list[int] = []
    for token in tokens:
        if not isinstance(token, int):
            raise TypeError(f"encode() returned non-integer token {token!r}")
        if token < 0 or token > 2**32 - 1:
            raise ValueError(f"encode() returned out-of-range u32 token {token}")
        normalized.append(token)
    return normalized


def call_optional_u32(module: Any, name: str) -> int | None:
    function = getattr(module, name, None)
    if function is None:
        return None
    value = function()
    if isinstance(value, int) and 0 <= value <= 2**32 - 1:
        return value
    return None


def file_provenance(path: Path) -> dict[str, Any]:
    content = path.read_bytes()
    return {
        "path": str(path),
        "bytes": len(content),
        "sha256": hashlib.sha256(content).hexdigest(),
    }


def elapsed_ms(start: float) -> int:
    return round((time.perf_counter() - start) * 1000)


if __name__ == "__main__":
    raise SystemExit(main())
