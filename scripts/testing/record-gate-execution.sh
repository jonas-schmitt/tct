#!/usr/bin/env bash
# Best-effort opt-in gate execution ledger recorder.
#
# Appends one timestamped JSON run record per invocation under
# target/gate-execution-ledger/<gate-id>/ and refreshes the per-gate
# latest.json pointer. Ledger writes are advisory local evidence of which
# gates ran; this helper must never fail the calling gate, so every failure
# path warns on stderr and exits 0.
#
# Usage:
#   record-gate-execution.sh --gate-id <id> [--helper <repo-path>] \
#       [--command <text>] [--started-at <UTC ISO-8601 Z>] \
#       [--exit-code <n>] [--result-json <json-object>]
#
# Environment:
#   TCT_GATE_EXECUTION_LEDGER_DIR   Ledger root override.
#                                   Default: <repo>/target/gate-execution-ledger
#   TCT_GATE_EXECUTION_LEDGER_DISABLE=1   Skip recording entirely.

set -u

script_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
repo_root=$(cd "$script_dir/../.." && pwd)

if [ "${TCT_GATE_EXECUTION_LEDGER_DISABLE:-0}" = "1" ]; then
    exit 0
fi

gate_id=""
helper=""
command_text=""
started_at=""
exit_code=""
result_json=""

while [ $# -gt 0 ]; do
    case "$1" in
        --gate-id) gate_id=${2:-}; shift 2 ;;
        --helper) helper=${2:-}; shift 2 ;;
        --command) command_text=${2:-}; shift 2 ;;
        --started-at) started_at=${2:-}; shift 2 ;;
        --exit-code) exit_code=${2:-}; shift 2 ;;
        --result-json) result_json=${2:-}; shift 2 ;;
        *)
            echo "warning: record-gate-execution.sh: unknown argument '$1'; skipping ledger record" >&2
            exit 0
            ;;
    esac
done

if [ -z "$gate_id" ]; then
    echo "warning: record-gate-execution.sh: --gate-id is required; skipping ledger record" >&2
    exit 0
fi

case "$gate_id" in
    *[!a-zA-Z0-9._-]*)
        echo "warning: record-gate-execution.sh: invalid gate id '$gate_id'; skipping ledger record" >&2
        exit 0
        ;;
esac

RECORD_GATE_ID="$gate_id" \
RECORD_HELPER="$helper" \
RECORD_COMMAND="$command_text" \
RECORD_STARTED_AT="$started_at" \
RECORD_EXIT_CODE="$exit_code" \
RECORD_RESULT_JSON="$result_json" \
RECORD_LEDGER_DIR="${TCT_GATE_EXECUTION_LEDGER_DIR:-$repo_root/target/gate-execution-ledger}" \
REPO_ROOT="$repo_root" \
python3 - <<'PY' || echo "warning: record-gate-execution.sh: failed to write gate-execution ledger record for '$gate_id'" >&2
import json
import os
import subprocess
from datetime import datetime, timezone
from pathlib import Path

repo_root = Path(os.environ["REPO_ROOT"])
ledger_dir = Path(os.environ["RECORD_LEDGER_DIR"])
gate_id = os.environ["RECORD_GATE_ID"]
TIMESTAMP_FORMAT = "%Y-%m-%dT%H:%M:%SZ"


def git_output(args):
    try:
        completed = subprocess.run(
            ["git", *args], cwd=repo_root, text=True, capture_output=True, check=False
        )
    except OSError:
        return None
    if completed.returncode != 0:
        return None
    return completed.stdout.strip()


now = datetime.now(timezone.utc).replace(microsecond=0)
finished_at = now.strftime(TIMESTAMP_FORMAT)

started_at = os.environ["RECORD_STARTED_AT"] or None
wall_seconds = None
if started_at:
    try:
        started = datetime.strptime(started_at, TIMESTAMP_FORMAT).replace(
            tzinfo=timezone.utc
        )
    except ValueError:
        wall_seconds = None
    else:
        wall_seconds = max(0, int((now - started).total_seconds()))

exit_code_text = os.environ["RECORD_EXIT_CODE"]
try:
    exit_code = int(exit_code_text)
except ValueError:
    exit_code = None

result_json = os.environ["RECORD_RESULT_JSON"]
result = None
if result_json:
    try:
        result = json.loads(result_json)
    except ValueError:
        result = {"unparseable_result_json": result_json}

dirty_output = git_output(["status", "--porcelain"])
record = {
    "format_version": "tct_gate_execution_record_v1",
    "schema_version": 1,
    "gate_id": gate_id,
    "helper": os.environ["RECORD_HELPER"] or None,
    "command": os.environ["RECORD_COMMAND"] or None,
    "started_at": started_at,
    "finished_at": finished_at,
    "wall_seconds": wall_seconds,
    "exit_code": exit_code,
    "git_head": git_output(["rev-parse", "HEAD"]),
    "git_dirty": None if dirty_output is None else bool(dirty_output),
    "result": result,
}

gate_dir = ledger_dir / gate_id
gate_dir.mkdir(parents=True, exist_ok=True)
payload = json.dumps(record, indent=2, sort_keys=True) + "\n"
run_name = f"run-{now.strftime('%Y%m%dT%H%M%SZ')}-{os.getpid()}.json"
(gate_dir / run_name).write_text(payload, encoding="utf-8")
(gate_dir / "latest.json").write_text(payload, encoding="utf-8")
PY
exit 0
