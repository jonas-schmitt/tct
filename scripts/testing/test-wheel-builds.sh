#!/usr/bin/env bash
# Run maintained Python wheel build and import smoke tests.

set -euo pipefail

script_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
repo_root=$(cd "$script_dir/../.." && pwd)
envelope_output=${TCT_WHEEL_TOOL_ENVELOPE_OUTPUT:-"$repo_root/target/tct-wheel-tool-envelope.json"}

cd "$repo_root"
python3 scripts/testing/wheel-tool-envelope.py --output "$envelope_output" --require-wheel-build-tools >/dev/null
echo "Wrote Python packaging tool envelope to ${envelope_output#$repo_root/}"

cargo test --test streaming_wheel_build_test test_streaming_wheel_build -- --ignored --exact --nocapture
cargo test --test bundle_wrapper_build_test test_bundle_wrapper_wheel_build -- --ignored --exact --nocapture
