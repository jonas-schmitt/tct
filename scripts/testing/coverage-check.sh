#!/usr/bin/env bash
# Report-only Rust coverage workflow for TCT using cargo llvm-cov.

set -euo pipefail

MODE="${1:-stable}"
OUT_ROOT="${TCT_COVERAGE_DIR:-target/tct-coverage}"
OUT_DIR="${OUT_ROOT}/${MODE}"
REPO_ROOT="$(pwd)"
script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

usage() {
    cat <<'EOF'
Usage: scripts/testing/coverage-check.sh [quick|stable|synthetic-ignored|round-trip-full]

Modes:
  quick              Run library coverage only. Useful while iterating.
  stable             Run non-ignored workspace/all-target coverage.
  synthetic-ignored  Run ignored synthetic generated-crate round trips.
  round-trip-full    Run full round_trip_test coverage with ROUND_TRIP_FULL=1.

Environment:
  TCT_COVERAGE_DIR       Output root. Default: target/tct-coverage
  TCT_COVERAGE_BRANCH=1  Enable nightly-only branch coverage.
  TCT_COVERAGE_NO_CLEAN=1
                         Reuse existing cargo-llvm-cov profile data.
  TCT_COVERAGE_FOCUS
                         Comma/space-separated repo paths to summarize from
                         summary.json into focused-summary.txt.
  TCT_COVERAGE_CHANGED_FROM
                         Git ref used to summarize changed Rust files from
                         summary.json into focused-summary.txt.
  TCT_SYNTHETIC_IGNORED_SCOPE=passing|all
                         For synthetic-ignored, default "passing" currently
                         runs the full ignored synthetic inventory. "all" is
                         kept as an explicit bypass for any future expected
                         failure skips.
  TCT_SYNTHETIC_IGNORED_TEST
                         Optional exact ignored synthetic test name for focused
                         coverage helper debugging.
  TCT_GENERATED_CARGO_TARGET_DIR
                         Shared Cargo target dir for generated temp crates.
                         Generated child Cargo commands append their lint cap
                         to inherited RUSTFLAGS or CARGO_ENCODED_RUSTFLAGS so
                         cargo llvm-cov instrumentation is preserved.

Reports:
  summary.txt
  summary.json
  lcov.info
  html/index.html
  generated-crates-summary.json and generated-crates-lcov.info for generated
  temp-crate modes when generated helper binaries are present.

Coverage is report-only. The maintained Rust coverage tool for this repository
is cargo llvm-cov, backed by Rust's LLVM source-based coverage instrumentation.
Line and function coverage run on the active stable toolchain. Branch coverage
requires nightly Rust and is opt-in with TCT_COVERAGE_BRANCH=1.
Stable coverage reports ignore generated temporary crate sources under /tmp and
target/tct-*; use synthetic-ignored or round-trip-full when intentionally
inspecting generated-crate coverage. Generated temporary crate coverage is
written as a sidecar report because cargo llvm-cov's workspace report does not
automatically include external helper binaries.
Do not add global thresholds until a trustworthy baseline and ratchet policy
exist.
EOF
}

if [[ "${MODE}" == "-h" || "${MODE}" == "--help" ]]; then
    usage
    exit 0
fi

if ! command -v cargo-llvm-cov >/dev/null 2>&1; then
    cat >&2 <<'EOF'
error: cargo-llvm-cov is required for coverage reports.

Install it with:
  cargo install cargo-llvm-cov
EOF
    exit 127
fi

ensure_llvm_tools() {
    if ! command -v rustup >/dev/null 2>&1; then
        return
    fi

    local toolchain
    toolchain="${1:-}"
    if [[ -z "${toolchain}" ]]; then
        toolchain="$(rustup show active-toolchain | awk '{print $1}')"
    fi
    if [[ -z "${toolchain}" ]]; then
        return
    fi

    if rustup component list --installed --toolchain "${toolchain}" | grep -Eq '^llvm-tools'; then
        return
    fi

    echo "Installing llvm-tools-preview for ${toolchain}..."
    rustup component add llvm-tools-preview --toolchain "${toolchain}"
}

branch_flags=()
cargo_llvm_cov=(cargo llvm-cov)
coverage_toolchain=""
report_flags=()

if [[ "${TCT_COVERAGE_BRANCH:-0}" == "1" ]]; then
    # Resolve the host triple so nightly checks work on aarch64 and x86_64
    # alike (never hardcode a single architecture triple).
    host_triple=""
    if command -v rustc >/dev/null 2>&1; then
        host_triple="$(rustc -vV 2>/dev/null | awk '/^host: / {print $2; exit}')"
    fi
    if [[ -z "${host_triple}" ]] && command -v rustup >/dev/null 2>&1; then
        host_triple="$(rustup show 2>/dev/null | awk -F': ' '/^Default host: / {print $2; exit}')"
    fi
    nightly_toolchain=""
    if [[ -n "${host_triple}" ]]; then
        nightly_toolchain="nightly-${host_triple}"
    fi
    if ! command -v rustup >/dev/null 2>&1 \
        || [[ -z "${nightly_toolchain}" ]] \
        || ! rustup toolchain list | awk '{print $1}' | grep -qx "${nightly_toolchain}"; then
        cat >&2 <<'EOF'
error: branch coverage requires a nightly Rust toolchain.

Install it with:
  rustup toolchain install nightly
EOF
        exit 127
    fi

    cargo_llvm_cov=(cargo +nightly llvm-cov)
    coverage_toolchain="nightly"
    branch_flags+=(--branch)
fi

ensure_llvm_tools "${coverage_toolchain}"

report_flags=("${branch_flags[@]}")
if [[ "${MODE}" == "stable" ]]; then
    # Stable coverage is the workspace signal. Tests may compile generated
    # temporary crates as subprocesses; keep their profiles out of stable
    # reports so generated-crate coverage can be inspected in the dedicated
    # slow modes without polluting the workspace baseline.
    report_flags+=(--ignore-filename-regex '(^/tmp/|/target/tct-)')
fi

cleanup_generated_temp_dirs() {
    case "${MODE}" in
        synthetic-ignored)
            rm -rf /tmp/tct_synthetic_batch_*
            ;;
        round-trip-full)
            rm -rf /tmp/tct_efficient_test_*
            ;;
    esac
}

generated_target_dir_for_mode() {
    case "${MODE}" in
        synthetic-ignored)
            printf '%s\n' "${TCT_GENERATED_CARGO_TARGET_DIR:-${REPO_ROOT}/target/tct-generated-cargo/synthetic-ignored}"
            ;;
        round-trip-full)
            printf '%s\n' "${TCT_GENERATED_CARGO_TARGET_DIR:-${REPO_ROOT}/target/tct-generated-cargo/round-trip-full}"
            ;;
    esac
}

record_gate_execution() {
    local exit_code=$1
    local result="{\"mode\": \"${MODE}\", \"out_dir\": \"${OUT_DIR}\"}"
    if [[ "${MODE}" == "synthetic-ignored" ]]; then
        local focused=false
        if [[ -n "${TCT_SYNTHETIC_IGNORED_TEST:-}" ]]; then
            focused=true
        fi
        result="{\"mode\": \"${MODE}\", \"scope\": \"${TCT_SYNTHETIC_IGNORED_SCOPE:-passing}\", \"focused_test\": ${focused}, \"out_dir\": \"${OUT_DIR}\"}"
    fi
    "${script_dir}/record-gate-execution.sh" --gate-id "coverage-${MODE}" --helper scripts/testing/coverage-check.sh --command "./scripts/testing/coverage-check.sh ${MODE}" --started-at "${gate_started_at}" --exit-code "${exit_code}" --result-json "${result}" || true
}

case "${MODE}" in
    synthetic-ignored|round-trip-full)
        gate_started_at=$(date -u +%Y-%m-%dT%H:%M:%SZ)
        # shellcheck disable=SC2154 # gate_exit is assigned inside the trap string
        trap 'gate_exit=$?; cleanup_generated_temp_dirs; record_gate_execution "${gate_exit}"' EXIT
        cleanup_generated_temp_dirs
        ;;
esac

run_coverage() {
    case "${MODE}" in
        quick)
            "${cargo_llvm_cov[@]}" --workspace --lib --no-report "${branch_flags[@]}"
            ;;
        stable)
            "${cargo_llvm_cov[@]}" --workspace --all-targets --no-report "${branch_flags[@]}"
            ;;
        synthetic-ignored)
            local synthetic_args=(--ignored --nocapture --test-threads=1)
            case "${TCT_SYNTHETIC_IGNORED_SCOPE:-passing}" in
                passing|all)
                    ;;
                *)
                    cat >&2 <<'EOF'
error: TCT_SYNTHETIC_IGNORED_SCOPE must be "passing" or "all".
EOF
                    exit 2
                    ;;
            esac
            if [[ -n "${TCT_SYNTHETIC_IGNORED_TEST:-}" ]]; then
                synthetic_args=("${TCT_SYNTHETIC_IGNORED_TEST}" --exact "${synthetic_args[@]}")
            fi
            TCT_GENERATED_CARGO_TARGET_DIR="$(generated_target_dir_for_mode)" \
                "${cargo_llvm_cov[@]}" --workspace --test synthetic_schema_test --no-report "${branch_flags[@]}" -- "${synthetic_args[@]}"
            ;;
        round-trip-full)
            ROUND_TRIP_FULL=1 ROUND_TRIP_SCENARIOS=all ROUND_TRIP_BUILD_PROFILE=release \
                TCT_GENERATED_CARGO_TARGET_DIR="$(generated_target_dir_for_mode)" \
                "${cargo_llvm_cov[@]}" --workspace --test round_trip_test --no-report "${branch_flags[@]}" -- test_round_trip --nocapture
            ;;
        *)
            usage >&2
            exit 2
            ;;
    esac
}

llvm_tool() {
    local name="$1"
    local sysroot
    if [[ -n "${coverage_toolchain}" ]]; then
        sysroot="$(rustc "+${coverage_toolchain}" --print sysroot)"
    else
        sysroot="$(rustc --print sysroot)"
    fi
    find "${sysroot}" -type f -name "${name}" | head -n 1
}

write_generated_crate_coverage_reports() {
    case "${MODE}" in
        synthetic-ignored|round-trip-full)
            ;;
        *)
            return
            ;;
    esac

    local generated_target_dir profile llvm_cov
    generated_target_dir="$(generated_target_dir_for_mode)"
    if [[ ! -d "${generated_target_dir}" ]]; then
        echo "Generated crate coverage sidecar skipped: ${generated_target_dir} not found"
        return
    fi

    profile="${REPO_ROOT}/target/llvm-cov-target/tct.profdata"
    if [[ ! -f "${profile}" ]]; then
        profile="$(find "${REPO_ROOT}/target/llvm-cov-target" -maxdepth 1 -name '*.profdata' | sort | head -n 1)"
    fi
    if [[ -z "${profile}" || ! -f "${profile}" ]]; then
        echo "Generated crate coverage sidecar skipped: merged llvm-cov profile not found"
        return
    fi

    llvm_cov="$(llvm_tool llvm-cov)"
    if [[ -z "${llvm_cov}" ]]; then
        echo "Generated crate coverage sidecar skipped: llvm-cov binary not found"
        return
    fi

    local objects=()
    while IFS= read -r -d '' object; do
        objects+=("${object}")
    done < <(
        find "${generated_target_dir}" \
            \( -path '*/debug/*' -o -path '*/release/*' \) \
            -type f -perm -111 \
            ! -path '*/deps/*' \
            ! -path '*/build/*' \
            ! -path '*/.fingerprint/*' \
            ! -name '*.d' \
            -print0
    )

    if (( ${#objects[@]} == 0 )); then
        echo "Generated crate coverage sidecar skipped: no helper binaries found in ${generated_target_dir}"
        return
    fi

    local extra_objects=()
    local object
    for object in "${objects[@]:1}"; do
        extra_objects+=(--object "${object}")
    done

    "${llvm_cov}" export \
        --format=text \
        --summary-only \
        --instr-profile="${profile}" \
        --ignore-filename-regex '(/\.cargo/registry|/rustc/)' \
        "${objects[0]}" \
        "${extra_objects[@]}" \
        > "${OUT_DIR}/generated-crates-summary.json"
    "${llvm_cov}" export \
        --format=lcov \
        --instr-profile="${profile}" \
        --ignore-filename-regex '(/\.cargo/registry|/rustc/)' \
        "${objects[0]}" \
        "${extra_objects[@]}" \
        > "${OUT_DIR}/generated-crates-lcov.info"

    echo "Generated crate coverage sidecar written for ${#objects[@]} helper binary/binaries"
}

if [[ "${TCT_COVERAGE_NO_CLEAN:-0}" != "1" ]]; then
    "${cargo_llvm_cov[@]}" clean --workspace
fi

rm -rf "${OUT_DIR}"
mkdir -p "${OUT_DIR}"

run_coverage

"${cargo_llvm_cov[@]}" report "${report_flags[@]}" --text --output-path "${OUT_DIR}/summary.txt"
"${cargo_llvm_cov[@]}" report "${report_flags[@]}" --summary-only --json --output-path "${OUT_DIR}/summary.json"
"${cargo_llvm_cov[@]}" report "${report_flags[@]}" --lcov --output-path "${OUT_DIR}/lcov.info"
"${cargo_llvm_cov[@]}" report "${report_flags[@]}" --html --output-dir "${OUT_DIR}"
write_generated_crate_coverage_reports

focus_args=()
if [[ -n "${TCT_COVERAGE_CHANGED_FROM:-}" ]]; then
    focus_args+=(--changed-from "${TCT_COVERAGE_CHANGED_FROM}")
fi
if [[ -n "${TCT_COVERAGE_FOCUS:-}" ]]; then
    # shellcheck disable=SC2206
    focus_paths=(${TCT_COVERAGE_FOCUS//,/ })
    focus_args+=("${focus_paths[@]}")
fi
if (( ${#focus_args[@]} > 0 )); then
    python3 "${REPO_ROOT}/scripts/testing/coverage-focus.py" \
        "${OUT_DIR}/summary.json" \
        --output "${OUT_DIR}/focused-summary.txt" \
        "${focus_args[@]}"
fi

cat <<EOF
Coverage reports written to ${OUT_DIR}
  Text summary: ${OUT_DIR}/summary.txt
  JSON summary: ${OUT_DIR}/summary.json
  LCOV:         ${OUT_DIR}/lcov.info
  HTML:         ${OUT_DIR}/html/index.html
EOF
if [[ -f "${OUT_DIR}/generated-crates-summary.json" ]]; then
    echo "  Generated:    ${OUT_DIR}/generated-crates-summary.json"
    echo "  Generated LCOV: ${OUT_DIR}/generated-crates-lcov.info"
fi
if (( ${#focus_args[@]} > 0 )); then
    echo "  Focus:       ${OUT_DIR}/focused-summary.txt"
fi
