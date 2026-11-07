#!/usr/bin/env bash
# Test the maintained target schema round-trip set.
# Usage: ./scripts/testing/test-target-schemas.sh

set -euo pipefail

script_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
repo_root=$(cd "$script_dir/../.." && pwd)
manifest=${TARGET_SCHEMA_MANIFEST:-"$repo_root/schemas/target-schemas.txt"}
timeout_seconds=${TARGET_SCHEMA_TIMEOUT_SECONDS:-240}
generated_target_dir=${TCT_GENERATED_CARGO_TARGET_DIR:-"$repo_root/target/tct-target-schema-generated-cargo"}
export TCT_GENERATED_CARGO_TARGET_DIR="$generated_target_dir"
gate_started_at=$(date -u +%Y-%m-%dT%H:%M:%SZ)

cd "$repo_root"
mkdir -p "$generated_target_dir"

if [ ! -f "$manifest" ]; then
    echo "Target schema manifest not found: $manifest"
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
    echo "Target schema manifest is empty: $manifest"
    exit 1
fi

schema_names=()
for schema_spec in "${target_schemas[@]}"; do
    IFS=':' read -r schema folder extra <<< "$schema_spec"
    if [ -z "$schema" ] || [ -z "$folder" ] || [ -n "$extra" ]; then
        echo "Invalid target schema manifest entry: $schema_spec"
        echo "Expected format: schema-name:schema-folder"
        exit 1
    fi
    if [ ! -f "schemas/$folder/$schema.json" ]; then
        echo "Target schema file does not exist for manifest entry: $schema_spec"
        echo "Missing: schemas/$folder/$schema.json"
        exit 1
    fi
    schema_names+=("$schema")
done

echo "Testing ${#target_schemas[@]} maintained target schemas..."
echo "Manifest: ${manifest#$repo_root/}"
echo "Generated Cargo target dir: ${generated_target_dir#$repo_root/}"
echo "Per-schema timeout: ${timeout_seconds}s"
printf 'Schemas: %s\n' "${schema_names[*]}"
echo "========================================"

passed=0
failed=0
failed_schemas=()
mkdir -p target

for schema_spec in "${target_schemas[@]}"; do
    IFS=':' read -r schema folder <<< "$schema_spec"
    echo -n "Testing $schema... "

    env_args=(
        -u BPE_MERGES
        -u ROUND_TRIP_BPE_PRESET
        -u MAX_INSTANCES
        -u MAX_BASE_VOCAB
        ROUND_TRIP_FULL=0
        ROUND_TRIP_SCENARIOS=o3
        ROUND_TRIP_BUILD_PROFILE=debug
        SCHEMA_NAME="$schema"
        SCHEMA_FOLDER="$folder"
    )
    if result=$(env "${env_args[@]}" timeout "$timeout_seconds" cargo test --quiet --test round_trip_test test_round_trip -- --exact 2>&1); then
        echo "✅ PASS"
        passed=$((passed + 1))
    else
        echo "❌ FAIL"
        failed=$((failed + 1))
        failed_schemas+=("$schema")
        printf '%s\n' "$result" > "target/tct-target-schema-${schema}.log"
    fi
done

echo ""
echo "========================================"
echo "Results: $passed passed, $failed failed"

gate_exit=0
if [ $failed -gt 0 ]; then
    gate_exit=1
fi
"$script_dir/record-gate-execution.sh" --gate-id target-schemas --helper scripts/testing/test-target-schemas.sh --command "./scripts/testing/test-target-schemas.sh" --started-at "$gate_started_at" --exit-code "$gate_exit" --result-json "{\"passed\": $passed, \"failed\": $failed, \"schema_count\": ${#target_schemas[@]}}" || true

if [ $failed -gt 0 ]; then
    echo "Failed schemas: ${failed_schemas[*]}"
    echo "Failure logs were written to target/tct-target-schema-<schema>.log"
    echo ""
    echo "Run with SCHEMA_NAME=<schema> to see detailed errors:"
    echo "  env -u BPE_MERGES -u ROUND_TRIP_BPE_PRESET -u MAX_INSTANCES -u MAX_BASE_VOCAB ROUND_TRIP_FULL=0 ROUND_TRIP_SCENARIOS=o3 ROUND_TRIP_BUILD_PROFILE=debug SCHEMA_NAME=<schema> cargo test --test round_trip_test test_round_trip -- --exact"
    exit 1
else
    echo "✅ All target schemas passing!"
fi
