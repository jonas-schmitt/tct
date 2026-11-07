#!/usr/bin/env bash
# Run reproducible generated-crate and optional Python-wheel tokenizer measurements.

set -euo pipefail

usage() {
    cat <<'EOF'
Usage:
  scripts/benchmarking/run-reproducible-tokenizer-benchmark.sh \
    --schema SCHEMA_JSON \
    --instances INSTANCE_GLOB \
    --output-dir OUTPUT_DIR \
    [--bpe-merges MERGES_JSON] \
    [--optimization O3] \
    [--warmup-iterations N] \
    [--track-allocations] \
    [--generated-bpe-metrics] \
    [--generated-encode-phase-metrics] \
    [--generated-encode-hot-path-metrics] \
    [--generated-encode-validation-phase-metrics] \
    [--generated-decode-prefix-metrics] \
    [--cleanup-generated-helper] \
    [--wheel-module MODULE] \
    [--wheel-pythonpath PATH] \
    [--wheel-iterations N] \
    [--wheel-warmup-iterations N] \
    [--wheel-decode]

The generated-crate path always runs tct-measure-tokens and writes:
  generated-crate-report.json
  generated-crate.log

If --wheel-module is provided, the script also measures an already importable
generated Python wheel module through measure_wheel_encode.py and writes:
  wheel-encode-report.json
  wheel-encode.log

Build or install the generated wheel explicitly before using --wheel-module.
Wheel measurement is opt-in because it depends on external Python/maturin/pip
tooling that is not part of the ordinary cargo test gate.
EOF
}

schema=""
instances=""
output_dir=""
bpe_merges=""
optimization="O3"
warmup_iterations="1"
track_allocations=0
generated_bpe_metrics=0
generated_encode_phase_metrics=0
generated_encode_hot_path_metrics=0
generated_encode_validation_phase_metrics=0
generated_decode_prefix_metrics=0
cleanup_generated_helper=0
wheel_module=""
wheel_pythonpath=()
wheel_iterations="1"
wheel_warmup_iterations="1"
wheel_decode=0

while [[ $# -gt 0 ]]; do
    case "$1" in
        --schema)
            schema="${2:?missing value for --schema}"
            shift 2
            ;;
        --instances)
            instances="${2:?missing value for --instances}"
            shift 2
            ;;
        --output-dir)
            output_dir="${2:?missing value for --output-dir}"
            shift 2
            ;;
        --bpe-merges)
            bpe_merges="${2:?missing value for --bpe-merges}"
            shift 2
            ;;
        --optimization)
            optimization="${2:?missing value for --optimization}"
            shift 2
            ;;
        --warmup-iterations)
            warmup_iterations="${2:?missing value for --warmup-iterations}"
            shift 2
            ;;
        --track-allocations)
            track_allocations=1
            shift
            ;;
        --generated-bpe-metrics)
            generated_bpe_metrics=1
            shift
            ;;
        --generated-encode-phase-metrics)
            generated_encode_phase_metrics=1
            shift
            ;;
        --generated-encode-hot-path-metrics)
            generated_encode_hot_path_metrics=1
            shift
            ;;
        --generated-encode-validation-phase-metrics)
            generated_encode_validation_phase_metrics=1
            shift
            ;;
        --generated-decode-prefix-metrics)
            generated_decode_prefix_metrics=1
            shift
            ;;
        --cleanup-generated-helper)
            cleanup_generated_helper=1
            shift
            ;;
        --wheel-module)
            wheel_module="${2:?missing value for --wheel-module}"
            shift 2
            ;;
        --wheel-pythonpath)
            wheel_pythonpath+=("${2:?missing value for --wheel-pythonpath}")
            shift 2
            ;;
        --wheel-iterations)
            wheel_iterations="${2:?missing value for --wheel-iterations}"
            shift 2
            ;;
        --wheel-warmup-iterations)
            wheel_warmup_iterations="${2:?missing value for --wheel-warmup-iterations}"
            shift 2
            ;;
        --wheel-decode)
            wheel_decode=1
            shift
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        *)
            echo "error: unknown argument: $1" >&2
            usage >&2
            exit 2
            ;;
    esac
done

if [[ -z "${schema}" || -z "${instances}" || -z "${output_dir}" ]]; then
    echo "error: --schema, --instances, and --output-dir are required" >&2
    usage >&2
    exit 2
fi
if [[ "${generated_bpe_metrics}" == "1" && -z "${bpe_merges}" ]]; then
    echo "error: --generated-bpe-metrics requires --bpe-merges" >&2
    usage >&2
    exit 2
fi

mkdir -p "${output_dir}"

generated_report="${output_dir}/generated-crate-report.json"
generated_log="${output_dir}/generated-crate.log"
measured_files="${output_dir}/measured-files.txt"

generated_args=(
    cargo run --release --bin tct-measure-tokens --
    --schema "${schema}"
    --instances "${instances}"
    --optimization "${optimization}"
    --warmup-iterations "${warmup_iterations}"
    --output-files "${measured_files}"
)

if [[ -n "${bpe_merges}" ]]; then
    generated_args+=(--bpe-merges "${bpe_merges}")
fi
if [[ "${track_allocations}" == "1" ]]; then
    generated_args+=(--track-allocations)
fi
if [[ "${generated_bpe_metrics}" == "1" ]]; then
    generated_args+=(--generated-bpe-metrics)
fi
if [[ "${generated_encode_phase_metrics}" == "1" ]]; then
    generated_args+=(--generated-encode-phase-metrics)
fi
if [[ "${generated_encode_hot_path_metrics}" == "1" ]]; then
    generated_args+=(--generated-encode-hot-path-metrics)
fi
if [[ "${generated_encode_validation_phase_metrics}" == "1" ]]; then
    generated_args+=(--generated-encode-validation-phase-metrics)
fi
if [[ "${generated_decode_prefix_metrics}" == "1" ]]; then
    generated_args+=(--generated-decode-prefix-metrics)
fi
if [[ "${cleanup_generated_helper}" == "1" ]]; then
    generated_args+=(--cleanup-generated-helper)
fi

echo "Running generated-crate measurement..."
"${generated_args[@]}" >"${generated_report}" 2>"${generated_log}"
echo "Generated-crate report: ${generated_report}"
echo "Generated-crate log:    ${generated_log}"
echo "Measured file list:     ${measured_files}"

if [[ -n "${wheel_module}" ]]; then
    wheel_report="${output_dir}/wheel-encode-report.json"
    wheel_log="${output_dir}/wheel-encode.log"
    wheel_args=(
        python3 scripts/benchmarking/measure_wheel_encode.py
        --module "${wheel_module}"
        --instances-file "${measured_files}"
        --iterations "${wheel_iterations}"
        --warmup-iterations "${wheel_warmup_iterations}"
    )
    for path in "${wheel_pythonpath[@]}"; do
        wheel_args+=(--pythonpath "${path}")
    done
    if [[ "${wheel_decode}" == "1" ]]; then
        wheel_args+=(--decode)
    fi

    echo "Running Python wheel encode measurement..."
    "${wheel_args[@]}" >"${wheel_report}" 2>"${wheel_log}"
    echo "Wheel encode report:   ${wheel_report}"
    echo "Wheel encode log:      ${wheel_log}"
fi
