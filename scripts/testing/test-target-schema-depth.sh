#!/usr/bin/env bash
# Run selected target schemas through full/all release generated-crate round trips.
# Usage: ./scripts/testing/test-target-schema-depth.sh

set -euo pipefail

script_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
repo_root=$(cd "$script_dir/../.." && pwd)
manifest=${TARGET_SCHEMA_DEPTH_MANIFEST:-"$repo_root/schemas/target-schema-depth.txt"}
timeout_seconds=${TARGET_SCHEMA_DEPTH_TIMEOUT_SECONDS:-900}
generated_target_dir=${TCT_GENERATED_CARGO_TARGET_DIR:-"$repo_root/target/tct-target-schema-depth-generated-cargo"}
report_path=${TARGET_SCHEMA_DEPTH_REPORT:-"$repo_root/target/tct-target-schema-depth-report.json"}
report_dir=$(dirname "$report_path")
dry_run=${TARGET_SCHEMA_DEPTH_DRY_RUN:-0}
export TCT_GENERATED_CARGO_TARGET_DIR="$generated_target_dir"
run_report_tmp=$(mktemp "${TMPDIR:-/tmp}/tct-target-schema-depth-runs.XXXXXX")
trap 'rm -f "$run_report_tmp"' EXIT
gate_started_at=$(date -u +%Y-%m-%dT%H:%M:%SZ)

cd "$repo_root"
mkdir -p "$generated_target_dir"
mkdir -p "$report_dir"

if [ ! -f "$manifest" ]; then
    echo "Target schema depth manifest not found: $manifest"
    exit 1
fi

target_schemas=()
while IFS= read -r line || [ -n "$line" ]; do
    line=${line%%#*}
    line=$(printf '%s' "$line" | tr -d '[:space:]')
    if [ -n "$line" ]; then
        target_schemas+=("$line")
    fi
done < "$manifest"

if [ ${#target_schemas[@]} -eq 0 ]; then
    echo "Target schema depth manifest is empty: $manifest"
    exit 1
fi

for schema_spec in "${target_schemas[@]}"; do
    IFS=':' read -r schema folder bpe_preset extra <<< "$schema_spec"
    if [ -z "$schema" ] || [ -z "$folder" ] || [ -n "$extra" ]; then
        echo "Invalid target schema depth manifest entry: $schema_spec"
        echo "Expected format: schema-name:schema-folder[:bpe-preset]"
        exit 1
    fi
    if [ ! -f "schemas/$folder/$schema.json" ]; then
        echo "Target schema file does not exist for manifest entry: $schema_spec"
        echo "Missing: schemas/$folder/$schema.json"
        exit 1
    fi
    if [ -n "${bpe_preset:-}" ]; then
        case "$bpe_preset" in
            legacy-tct-500)
                bpe_path="schemas/bpe/$schema-tct-bpe-500.json"
                ;;
            *)
                echo "Unsupported BPE preset '$bpe_preset' in manifest entry: $schema_spec"
                echo "Supported presets: legacy-tct-500"
                exit 1
                ;;
        esac
        if [ ! -f "$bpe_path" ]; then
            echo "BPE preset artifact does not exist for manifest entry: $schema_spec"
            echo "Missing: $bpe_path"
            exit 1
        fi
        bpe_provenance_path="${bpe_path%.json}.provenance.json"
        if [ ! -f "$bpe_provenance_path" ]; then
            echo "BPE preset provenance sidecar does not exist for manifest entry: $schema_spec"
            echo "Missing: $bpe_provenance_path"
            exit 1
        fi
        python3 -m json.tool "$bpe_provenance_path" >/dev/null
    fi
done

echo "Testing ${#target_schemas[@]} selected target schema depth entries..."
echo "Manifest: ${manifest#$repo_root/}"
echo "Generated Cargo target dir: ${generated_target_dir#$repo_root/}"
echo "Report: ${report_path#$repo_root/}"
echo "Per-run timeout: ${timeout_seconds}s"
echo "Round-trip mode: ROUND_TRIP_FULL=1 ROUND_TRIP_SCENARIOS=all ROUND_TRIP_BUILD_PROFILE=release"
if [ "$dry_run" = "1" ]; then
    echo "Dry run: commands will be printed, not executed"
fi
echo "========================================"

passed=0
failed=0
dry_runs=0
timed_out=0
failed_runs=()
mkdir -p target

shell_join() {
    printf '%q ' "$@"
}

append_run_report() {
    local schema=$1
    local folder=$2
    local manifest_entry=$3
    local mode=$4
    local bpe_preset=$5
    local status=$6
    local exit_code=$7
    local log_path=$8
    local round_trip_report_path=$9
    local wall_seconds=${10}
    local command_text=${11}

    RUN_SCHEMA="$schema" \
    RUN_FOLDER="$folder" \
    RUN_MANIFEST_ENTRY="$manifest_entry" \
    RUN_MODE="$mode" \
    RUN_BPE_PRESET="$bpe_preset" \
    RUN_STATUS="$status" \
    RUN_EXIT_CODE="$exit_code" \
    RUN_LOG_PATH="$log_path" \
    RUN_ROUND_TRIP_REPORT_PATH="$round_trip_report_path" \
    RUN_WALL_SECONDS="$wall_seconds" \
    RUN_COMMAND="$command_text" \
    REPO_ROOT="$repo_root" \
    python3 - "$run_report_tmp" <<'PY'
import json
import os
import sys
import hashlib
from pathlib import Path

repo_root = Path(os.environ["REPO_ROOT"]).resolve()

def sha256(path: Path) -> str | None:
    if not path.exists():
        return None
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()

def relative(path: str) -> str | None:
    if not path:
        return None
    resolved = Path(path)
    if not resolved.is_absolute():
        resolved = (repo_root / resolved).resolve()
    try:
        return resolved.relative_to(repo_root).as_posix()
    except ValueError:
        return str(resolved)

def path_record(path: Path | None) -> dict | None:
    if path is None:
        return None
    return {
        "path": relative(str(path)),
        "sha256": sha256(path),
    }

schema = os.environ["RUN_SCHEMA"]
folder = os.environ["RUN_FOLDER"]
bpe_preset = os.environ["RUN_BPE_PRESET"] or None
schema_path = repo_root / "schemas" / folder / f"{schema}.json"
bpe_path = None
bpe_provenance_path = None
if bpe_preset == "legacy-tct-500":
    bpe_path = repo_root / "schemas" / "bpe" / f"{schema}-tct-bpe-500.json"
    bpe_provenance_path = bpe_path.with_name(f"{schema}-tct-bpe-500.provenance.json")

exit_code = os.environ["RUN_EXIT_CODE"]
record = {
    "schema": schema,
    "folder": folder,
    "manifest_entry": os.environ["RUN_MANIFEST_ENTRY"],
    "mode": os.environ["RUN_MODE"],
    "bpe_preset": bpe_preset,
    "status": os.environ["RUN_STATUS"],
    "executed": os.environ["RUN_STATUS"] != "dry_run",
    "exit_code": int(exit_code) if exit_code else None,
    "schema_file": path_record(schema_path),
    "bpe_file": path_record(bpe_path),
    "bpe_provenance_file": path_record(bpe_provenance_path),
    "log_path": relative(os.environ["RUN_LOG_PATH"]),
    "round_trip_report": path_record(Path(os.environ["RUN_ROUND_TRIP_REPORT_PATH"])) if os.environ["RUN_ROUND_TRIP_REPORT_PATH"] else None,
    "wall_seconds": int(os.environ["RUN_WALL_SECONDS"]),
    "command": os.environ["RUN_COMMAND"].strip(),
}
with open(sys.argv[1], "a", encoding="utf-8") as handle:
    handle.write(json.dumps(record, sort_keys=True))
    handle.write("\n")
PY
}

write_report() {
    python3 - "$report_path" "$manifest" "$generated_target_dir" "$run_report_tmp" <<'PY'
import json
import os
import sys
import hashlib
import subprocess
from pathlib import Path

report_path = Path(sys.argv[1])
manifest = Path(sys.argv[2])
generated_target_dir = Path(sys.argv[3])
run_report_tmp = Path(sys.argv[4])
repo_root = Path(os.environ["PWD"]).resolve()

def sha256(path: Path) -> str | None:
    if not path.exists():
        return None
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()

def relative(path: Path) -> str:
    resolved = path.resolve()
    try:
        return resolved.relative_to(repo_root).as_posix()
    except ValueError:
        return str(resolved)

def command_output(args: list[str]) -> str | None:
    try:
        completed = subprocess.run(
            args,
            cwd=repo_root,
            text=True,
            capture_output=True,
            check=False,
        )
    except OSError:
        return None
    output = completed.stdout.strip() or completed.stderr.strip()
    return output or None

def git_dirty() -> bool | None:
    try:
        completed = subprocess.run(
            ["git", "status", "--porcelain"],
            cwd=repo_root,
            text=True,
            capture_output=True,
            check=False,
        )
    except OSError:
        return None
    if completed.returncode != 0:
        return None
    return bool(completed.stdout.strip())

runs = []
if run_report_tmp.exists():
    with run_report_tmp.open(encoding="utf-8") as handle:
        runs = [json.loads(line) for line in handle if line.strip()]

passed_round_trip_report_hash_gaps = [
    f"{run['schema']}:{run['mode']}"
    for run in runs
    if run["status"] == "passed"
    and run["executed"]
    and (
        not isinstance(run.get("round_trip_report"), dict)
        or not run["round_trip_report"].get("sha256")
    )
]

summary = {
    "manifest_entry_count": len({run["manifest_entry"] for run in runs}),
    "planned_run_count": len(runs),
    "passed": int(os.environ["TARGET_SCHEMA_DEPTH_PASSED"]),
    "failed": int(os.environ["TARGET_SCHEMA_DEPTH_FAILED"]),
    "timed_out": int(os.environ["TARGET_SCHEMA_DEPTH_TIMED_OUT"]),
    "dry_run": int(os.environ["TARGET_SCHEMA_DEPTH_DRY_RUNS"]),
    "passed_round_trip_report_hash_gaps": passed_round_trip_report_hash_gaps,
}
report = {
    "format_version": "tct_target_schema_depth_v1",
    "schema_version": 1,
    "helper": "scripts/testing/test-target-schema-depth.sh",
    "manifest": relative(manifest),
    "manifest_sha256": sha256(manifest),
    "generated_target_dir": relative(generated_target_dir),
    "timeout_seconds": int(os.environ["TARGET_SCHEMA_DEPTH_TIMEOUT_SECONDS_EFFECTIVE"]),
    "dry_run": os.environ["TARGET_SCHEMA_DEPTH_DRY_RUN_EFFECTIVE"] == "1",
    "round_trip_policy": {
        "ROUND_TRIP_FULL": "1",
        "ROUND_TRIP_SCENARIOS": "all",
        "ROUND_TRIP_BUILD_PROFILE": "release",
    },
    "toolchain": {
        "rustc": command_output(["rustc", "-Vv"]),
        "cargo": command_output(["cargo", "-V"]),
    },
    "git": {
        "head": command_output(["git", "rev-parse", "HEAD"]),
        "dirty": git_dirty(),
    },
    "summary": summary,
    "passed": summary["passed"],
    "failed": summary["failed"],
    "timed_out": summary["timed_out"],
    "dry_run_count": summary["dry_run"],
    "passed_round_trip_report_hash_gaps": passed_round_trip_report_hash_gaps,
    "failed_runs": [
        f"{run['schema']}:{run['mode']}"
        for run in runs
        if run["status"] in {"failed", "timed_out"}
    ],
    "runs": runs,
}
report_path.parent.mkdir(parents=True, exist_ok=True)
with report_path.open("w", encoding="utf-8") as handle:
    json.dump(report, handle, indent=2, sort_keys=True)
    handle.write("\n")
PY
}

run_round_trip() {
    local schema=$1
    local folder=$2
    local mode=$3
    local bpe_preset=${4:-}
    local manifest_entry=${5:-"$schema:$folder"}
    local log_path="target/tct-target-schema-depth-${schema}-${mode}.log"
    local round_trip_copy="$report_dir/tct-target-schema-depth-${schema}-${mode}-round-trip-report.json"
    local env_args=(
        -u BPE_MERGES
        -u MAX_INSTANCES
        -u MAX_BASE_VOCAB
        ROUND_TRIP_FULL=1
        ROUND_TRIP_SCENARIOS=all
        ROUND_TRIP_BUILD_PROFILE=release
        SCHEMA_NAME="$schema"
        SCHEMA_FOLDER="$folder"
    )
    if [ -n "$bpe_preset" ]; then
        env_args+=(ROUND_TRIP_BPE_PRESET="$bpe_preset")
    else
        env_args=(-u ROUND_TRIP_BPE_PRESET "${env_args[@]}")
    fi
    local command=(timeout "$timeout_seconds" cargo test --quiet --test round_trip_test test_round_trip -- --exact)
    local command_text
    command_text=$(shell_join env "${env_args[@]}" "${command[@]}")
    local start_seconds=$SECONDS

    printf 'Testing %s [%s]... ' "$schema" "$mode"
    if [ "$dry_run" = "1" ]; then
        echo "DRY RUN"
        printf '  env'
        printf ' %q' "${env_args[@]}" "${command[@]}"
        printf '\n'
        dry_runs=$((dry_runs + 1))
        append_run_report "$schema" "$folder" "$manifest_entry" "$mode" "${bpe_preset:-}" "dry_run" "" "" "" "$((SECONDS - start_seconds))" "$command_text"
        return
    fi

    rm -f target/tct-round-trip-report.json "$round_trip_copy"
    if result=$(env "${env_args[@]}" "${command[@]}" 2>&1); then
        local round_trip_report=""
        if [ -f target/tct-round-trip-report.json ]; then
            cp target/tct-round-trip-report.json "$round_trip_copy"
            round_trip_report="$round_trip_copy"
            echo "PASS"
            passed=$((passed + 1))
            append_run_report "$schema" "$folder" "$manifest_entry" "$mode" "${bpe_preset:-}" "passed" "0" "" "$round_trip_report" "$((SECONDS - start_seconds))" "$command_text"
        else
            echo "FAIL (missing round-trip report)"
            failed=$((failed + 1))
            failed_runs+=("$schema:$mode")
            {
                printf '%s\n' "$result"
                printf '\nSuccessful run did not write target/tct-round-trip-report.json\n'
            } > "$log_path"
            append_run_report "$schema" "$folder" "$manifest_entry" "$mode" "${bpe_preset:-}" "failed" "0" "$log_path" "" "$((SECONDS - start_seconds))" "$command_text"
        fi
    else
        local exit_code=$?
        local status="failed"
        if [ "$exit_code" -eq 124 ]; then
            status="timed_out"
            timed_out=$((timed_out + 1))
        fi
        echo "FAIL"
        failed=$((failed + 1))
        failed_runs+=("$schema:$mode")
        printf '%s\n' "$result" > "$log_path"
        local round_trip_report=""
        if [ -f target/tct-round-trip-report.json ]; then
            cp target/tct-round-trip-report.json "$round_trip_copy"
            round_trip_report="$round_trip_copy"
        fi
        append_run_report "$schema" "$folder" "$manifest_entry" "$mode" "${bpe_preset:-}" "$status" "$exit_code" "$log_path" "$round_trip_report" "$((SECONDS - start_seconds))" "$command_text"
    fi
}

for schema_spec in "${target_schemas[@]}"; do
    IFS=':' read -r schema folder bpe_preset <<< "$schema_spec"
    run_round_trip "$schema" "$folder" "no-bpe" "" "$schema_spec"
    if [ -n "${bpe_preset:-}" ]; then
        run_round_trip "$schema" "$folder" "bpe-${bpe_preset}" "$bpe_preset" "$schema_spec"
    fi
done

echo ""
echo "========================================"
echo "Results: $passed passed, $failed failed, $timed_out timed out, $dry_runs dry runs"
TARGET_SCHEMA_DEPTH_TIMEOUT_SECONDS_EFFECTIVE="$timeout_seconds" \
TARGET_SCHEMA_DEPTH_DRY_RUN_EFFECTIVE="$dry_run" \
TARGET_SCHEMA_DEPTH_PASSED="$passed" \
TARGET_SCHEMA_DEPTH_FAILED="$failed" \
TARGET_SCHEMA_DEPTH_TIMED_OUT="$timed_out" \
TARGET_SCHEMA_DEPTH_DRY_RUNS="$dry_runs" \
write_report
echo "Report written to ${report_path#$repo_root/}"

gate_exit=0
if [ $failed -gt 0 ]; then
    gate_exit=1
fi
if [ "$dry_run" != "1" ]; then
    "$script_dir/record-gate-execution.sh" --gate-id target-schema-depth --helper scripts/testing/test-target-schema-depth.sh --command "./scripts/testing/test-target-schema-depth.sh" --started-at "$gate_started_at" --exit-code "$gate_exit" --result-json "{\"passed\": $passed, \"failed\": $failed, \"timed_out\": $timed_out, \"round_trip_full\": true}" || true
fi

if [ $failed -gt 0 ]; then
    echo "Failed runs: ${failed_runs[*]}"
    echo "Failure logs were written to target/tct-target-schema-depth-<schema>-<mode>.log"
    exit 1
else
    echo "All selected target schema depth runs passed."
fi
