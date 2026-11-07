#!/usr/bin/env bash
# Run maintained round-trip smoke coverage both without BPE and with BPE presets.

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "${repo_root}"

timeout_seconds=${ROUND_TRIP_BPE_SMOKE_TIMEOUT_SECONDS:-600}
bpe_preset=${ROUND_TRIP_BPE_SMOKE_PRESET:-legacy-tct-500}
generated_target_dir=${TCT_GENERATED_CARGO_TARGET_DIR:-"$repo_root/target/tct-round-trip-bpe-smoke-generated-cargo"}
export TCT_GENERATED_CARGO_TARGET_DIR="$generated_target_dir"

mkdir -p "$generated_target_dir"

section() {
    printf '\n[%s]\n' "$1"
}

section "Default round-trip smoke without BPE"
env -u ROUND_TRIP_BPE_PRESET -u BPE_MERGES \
    timeout "$timeout_seconds" \
    cargo test --test round_trip_test test_round_trip -- --exact

section "Default round-trip smoke with ${bpe_preset} BPE preset"
env ROUND_TRIP_BPE_PRESET="$bpe_preset" \
    timeout "$timeout_seconds" \
    cargo test --test round_trip_test test_round_trip -- --exact

focused_schema_specs=(
    "github-workflow:popular"
    "package:popular"
)

for schema_spec in "${focused_schema_specs[@]}"; do
    IFS=':' read -r schema folder <<< "$schema_spec"

    section "Focused ${schema} round-trip smoke without BPE"
    env -u ROUND_TRIP_BPE_PRESET -u BPE_MERGES \
        SCHEMA_NAME="$schema" \
        SCHEMA_FOLDER="$folder" \
        timeout "$timeout_seconds" \
        cargo test --test round_trip_test test_round_trip -- --exact

    section "Focused ${schema} round-trip smoke with ${bpe_preset} BPE preset"
    env SCHEMA_NAME="$schema" \
        SCHEMA_FOLDER="$folder" \
        ROUND_TRIP_BPE_PRESET="$bpe_preset" \
        timeout "$timeout_seconds" \
        cargo test --test round_trip_test test_round_trip -- --exact
done
