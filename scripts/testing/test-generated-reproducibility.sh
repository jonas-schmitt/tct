#!/usr/bin/env bash
# Check generated Cargo/toolchain reproducibility surfaces.
#
# This is an explicit generated-artifact gate, not part of ordinary cargo test.
# It builds one generated temporary binary crate and one generated Python wheel,
# so it requires the external Python/maturin environment used by wheel tests.

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "${repo_root}"
script_dir="${repo_root}/scripts/testing"
gate_started_at="$(date -u +%Y-%m-%dT%H:%M:%SZ)"

record_gate_execution() {
    local exit_code=$1
    local wheel_skipped=false
    if [[ "${TCT_GENERATED_REPRODUCIBILITY_SKIP_WHEEL:-0}" == "1" ]]; then
        wheel_skipped=true
    fi
    "${script_dir}/record-gate-execution.sh" --gate-id generated-reproducibility --helper scripts/testing/test-generated-reproducibility.sh --command "./scripts/testing/test-generated-reproducibility.sh" --started-at "${gate_started_at}" --exit-code "${exit_code}" --result-json "{\"wheel_skipped\": ${wheel_skipped}}" || true
}
trap 'record_gate_execution "$?"' EXIT

section() {
    printf '\n[%s]\n' "$1"
}

section "Generated Cargo template drift"
cargo test generated_crate::tests

section "Generated binary crate build"
cargo test --test synthetic_schema_test test_simple_duplicate_round_trip -- --ignored --exact --nocapture

section "Generated Python wheel build"
if [[ "${TCT_GENERATED_REPRODUCIBILITY_SKIP_WHEEL:-0}" == "1" ]]; then
    echo "Skipped because TCT_GENERATED_REPRODUCIBILITY_SKIP_WHEEL=1."
else
    ./scripts/testing/test-wheel-builds.sh
fi
