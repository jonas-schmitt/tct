#!/usr/bin/env bash
# Local quality helper for TCT.
#
# Stable gates block the script by default. Advisory gates report drift that is
# worth fixing, but they do not fail ordinary local validation until the project
# has an explicit cleanup or ratchet policy. Clippy warnings are part of the
# stable gate. Set TCT_STRICT_QUALITY=1 to make advisory failures block as well.
#
# Concurrent safety: a repo-local flock serializes overlapping quality helpers.
# Generated-crate builds use a quality-owned CARGO target by default so they do
# not share state with direct `cargo test` runs or other
# scratch under target/.

set -uo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "${repo_root}" || exit 1


STRICT_ADVISORY="${TCT_STRICT_QUALITY:-0}"
DOC_THRESHOLD="${TCT_DOC_COVERAGE_THRESHOLD:-90}"
MODULE_LINE_LIMIT="${TCT_MODULE_LINE_LIMIT:-600}"
QUALITY_MIN_FREE_KIB="${TCT_QUALITY_MIN_FREE_KIB:-10485760}"
QUALITY_REPORT_DIR="${TCT_QUALITY_REPORT_DIR:-target/tct-quality}"
QUALITY_LOCK_FILE="${TCT_QUALITY_LOCK_FILE:-target/tct-quality.lock}"
QUALITY_LOCK_WAIT_SECONDS="${TCT_QUALITY_LOCK_WAIT_SECONDS:-0}"
QUALITY_USE_LOCK="${TCT_QUALITY_USE_LOCK:-1}"
# Preserve a caller-provided generated-Cargo target (for example local-ci.sh).
# Otherwise isolate quality runs from the direct-test default
# target/tct-generated-cargo and from other scratch.
if [[ -z "${TCT_GENERATED_CARGO_TARGET_DIR:-}" ]]; then
    export TCT_GENERATED_CARGO_TARGET_DIR="${repo_root}/target/tct-quality-generated-cargo"
fi
TMP_DIR="$(mktemp -d)"

stable_total=0
stable_failed=0
advisory_total=0
advisory_failed=0
disk_preflight_passed=0
quality_lock_fd=""

cleanup() {
    rm -rf "${TMP_DIR}"
    if [[ -n "${quality_lock_fd}" ]]; then
        # Drop the flock by closing the descriptor; leave the lock file path
        # as a durable handle for the next runer's diagnostics.
        eval "exec ${quality_lock_fd}>&-" 2>/dev/null || true
    fi
}
trap cleanup EXIT

acquire_quality_lock() {
    if [[ "${QUALITY_USE_LOCK}" != "1" ]]; then
        return 0
    fi
    if ! command -v flock >/dev/null 2>&1; then
        printf 'error: flock is required for the quality helper lock; set TCT_QUALITY_USE_LOCK=0 to disable it\n' >&2
        return 1
    fi
    if ! [[ "${QUALITY_LOCK_WAIT_SECONDS}" =~ ^[0-9]+$ ]]; then
        printf 'error: TCT_QUALITY_LOCK_WAIT_SECONDS must be a non-negative integer; got %s\n' \
            "${QUALITY_LOCK_WAIT_SECONDS}" >&2
        return 1
    fi

    mkdir -p "$(dirname -- "${QUALITY_LOCK_FILE}")"
    # Open read/write without truncating so a failed acquire can still print the
    # holder metadata written by the concurrent quality helper. FD 8 is reserved
    # for the quality helper lock.
    : >>"${QUALITY_LOCK_FILE}"
    exec 8<>"${QUALITY_LOCK_FILE}"
    quality_lock_fd=8

    local flock_rc=0
    if ((QUALITY_LOCK_WAIT_SECONDS > 0)); then
        flock -w "${QUALITY_LOCK_WAIT_SECONDS}" 8 || flock_rc=$?
    else
        flock -n 8 || flock_rc=$?
    fi

    if ((flock_rc != 0)); then
        printf 'error: another quality helper holds %s\n' "${QUALITY_LOCK_FILE}" >&2
        if [[ -s "${QUALITY_LOCK_FILE}" ]]; then
            printf 'lock holder metadata:\n' >&2
            sed 's/^/  /' "${QUALITY_LOCK_FILE}" >&2 || true
        else
            printf 'lock file exists but has no metadata yet (holder may still be starting)\n' >&2
        fi
        if ((QUALITY_LOCK_WAIT_SECONDS > 0)); then
            printf 'timed out after TCT_QUALITY_LOCK_WAIT_SECONDS=%s\n' \
                "${QUALITY_LOCK_WAIT_SECONDS}" >&2
        else
            printf 'pass TCT_QUALITY_LOCK_WAIT_SECONDS=N to wait up to N seconds, or wait for the other quality helper to finish\n' >&2
        fi
        quality_lock_fd=""
        exec 8>&- 2>/dev/null || true
        return 1
    fi

    # Holder rewrite for concurrent diagnostics (path rewrite keeps the flock
    # on this inode via FD 8).
    {
        printf 'pid=%s\nrepo_root=%s\nstarted_at=%s\ngenerated_cargo_target=%s\n' \
            "$$" \
            "${repo_root}" \
            "$(date -u +"%Y-%m-%dT%H:%M:%SZ")" \
            "${TCT_GENERATED_CARGO_TARGET_DIR}"
    } >"${QUALITY_LOCK_FILE}"
    return 0
}

new_tmp() {
    mktemp "${TMP_DIR}/quality-check.XXXXXX"
}

report_path() {
    printf '%s/%s\n' "${QUALITY_REPORT_DIR}" "$1"
}

section() {
    printf '\n[%s]\n' "$1"
}

stable_pass() {
    printf 'PASS stable: %s\n' "$1"
}

stable_fail() {
    printf 'FAIL stable: %s\n' "$1"
    stable_failed=$((stable_failed + 1))
}

advisory_pass() {
    printf 'PASS advisory: %s\n' "$1"
}

advisory_warn() {
    printf 'WARN advisory: %s\n' "$1"
    advisory_failed=$((advisory_failed + 1))
}

run_stable() {
    local name="$1"
    shift
    stable_total=$((stable_total + 1))
    section "${name}"
    if "$@"; then
        stable_pass "${name}"
        return 0
    else
        stable_fail "${name}"
        return 1
    fi
}

skip_stable() {
    local name="$1"
    local reason="$2"
    stable_total=$((stable_total + 1))
    section "${name}"
    printf '%s\n' "${reason}"
    stable_fail "${name}"
}

run_advisory() {
    local name="$1"
    shift
    advisory_total=$((advisory_total + 1))
    section "${name}"
    if "$@"; then
        advisory_pass "${name}"
    else
        advisory_warn "${name}"
    fi
}

check_clippy() {
    local log
    log="$(new_tmp)"

    stable_total=$((stable_total + 1))
    section "Clippy build"
    if cargo clippy --all-targets --quiet >"${log}" 2>&1; then
        stable_pass "clippy completed without compiler errors"
    else
        tail -80 "${log}"
        stable_fail "clippy failed to complete"
        return
    fi

    stable_total=$((stable_total + 1))
    section "Clippy warnings"
    local warning_count
    warning_count="$(grep -c "warning:" "${log}" || true)"
    if [[ "${warning_count}" == "0" ]]; then
        stable_pass "zero clippy warnings"
    else
        grep "warning:" "${log}" | head -40
        printf 'Found %s clippy warnings. Run `cargo clippy --all-targets -- -D warnings` locally for a hard-fail view.\n' "${warning_count}"
        stable_fail "clippy warnings are present"
    fi
}

format_kib_as_gib() {
    awk -v kib="$1" 'BEGIN { printf "%.1f", kib / 1024 / 1024 }'
}

check_disk_space() {
    if ! [[ "${QUALITY_MIN_FREE_KIB}" =~ ^[0-9]+$ ]]; then
        printf 'TCT_QUALITY_MIN_FREE_KIB must be an integer KiB value; got %s.\n' \
            "${QUALITY_MIN_FREE_KIB}"
        return 1
    fi

    local entries=(
        "repo:${PWD}"
        "temp:${TMPDIR:-/tmp}"
    )
    local seen_filesystems=" "
    local failed=0

    for entry in "${entries[@]}"; do
        local label="${entry%%:*}"
        local path="${entry#*:}"
        local filesystem available_kib

        if [[ ! -e "${path}" ]]; then
            printf '%s path does not exist: %s\n' "${label}" "${path}"
            failed=1
            continue
        fi

        read -r filesystem available_kib < <(df -Pk "${path}" | awk 'NR == 2 { print $1, $4 }')
        if [[ -z "${filesystem:-}" || -z "${available_kib:-}" ]]; then
            printf 'Could not determine free disk space for %s (%s).\n' "${label}" "${path}"
            failed=1
            continue
        fi

        if [[ "${seen_filesystems}" == *" ${filesystem} "* ]]; then
            continue
        fi
        seen_filesystems+="${filesystem} "

        printf '%s filesystem %s has %s GiB free; minimum is %s GiB.\n' \
            "${label}" \
            "${filesystem}" \
            "$(format_kib_as_gib "${available_kib}")" \
            "$(format_kib_as_gib "${QUALITY_MIN_FREE_KIB}")"

        if ((available_kib < QUALITY_MIN_FREE_KIB)); then
            failed=1
        fi
    done

    if ((failed)); then
        printf 'Run `cargo clean` for repo-local build artifacts, free stale TCT temp crates under %s, or lower TCT_QUALITY_MIN_FREE_KIB only for constrained environments.\n' \
            "${TMPDIR:-/tmp}"
        return 1
    fi
}

check_module_size() {
    python3 scripts/testing/module-size-report.py \
        --limit "${MODULE_LINE_LIMIT}" \
        --json-output "$(report_path module-size-report.json)"
}

check_generated_source_inventory() {
    python3 scripts/testing/generated-source-inventory.py \
        --mode all \
        --json-format artifact \
        --json-output "$(report_path generated-source-inventory.json)" \
        --fail-on-production \
        --fail-on-test-assertions
}

check_documentation_coverage() {
    python3 scripts/testing/documentation-coverage-report.py \
        --threshold "${DOC_THRESHOLD}" \
        --json-output "$(report_path documentation-coverage-report.json)"
}

check_unwrap_expect() {
    python3 scripts/testing/unwrap-expect-report.py \
        --root src \
        --limit 40 \
        --json-output "$(report_path unwrap-expect-report.json)"
}

printf 'TCT local quality check\n'
printf 'Stable gates block by default; advisory gates block only with TCT_STRICT_QUALITY=1.\n'
printf 'TCT_GENERATED_CARGO_TARGET_DIR=%s\n' "${TCT_GENERATED_CARGO_TARGET_DIR}"
if ! mkdir -p "${QUALITY_REPORT_DIR}"; then
    printf 'Could not create quality report directory: %s\n' "${QUALITY_REPORT_DIR}"
    exit 1
fi
printf 'Advisory JSON reports: %s\n' "${QUALITY_REPORT_DIR}"

if ! acquire_quality_lock; then
    exit 1
fi
if [[ "${QUALITY_USE_LOCK}" == "1" ]]; then
    printf 'Quality lock: %s (wait_seconds=%s)\n' \
        "${QUALITY_LOCK_FILE}" "${QUALITY_LOCK_WAIT_SECONDS}"
fi

if run_stable "Disk space preflight" check_disk_space; then
    disk_preflight_passed=1
fi
check_clippy
run_stable "Code formatting" cargo fmt --all -- --check
run_stable "Test suite" cargo test --quiet
# Recursion guard: the tests/python suite itself spawns this helper
# (tests/python/test_quality_check_script.py), so a stubbed interpreter that
# fails to intercept would let the gate below re-enter the real suite, which
# would spawn the helper again without bound. Every suite runner exports
# TCT_PYTHON_SUITE_ACTIVE=1 (tests/python/conftest.py and the gate below), so a
# set sentinel means this helper is running INSIDE a tests/python run and the
# gate must refuse instead of recursing.
run_python_suite() {
    TCT_PYTHON_SUITE_ACTIVE=1 python3 -m unittest discover tests/python
}
if [[ -n "${TCT_PYTHON_SUITE_ACTIVE:-}" ]]; then
    skip_stable \
        "Python maintenance tests" \
        "Refusing nested tests/python suite: TCT_PYTHON_SUITE_ACTIVE=${TCT_PYTHON_SUITE_ACTIVE} means an enclosing tests/python run is active (pytest gate, unittest discover, or another quality helper), and re-running the suite from inside it recurses without bound. Run this helper from a top-level shell instead."
else
    run_stable "Python maintenance tests" run_python_suite
fi
run_advisory "Module size report" check_module_size
run_advisory "Documentation coverage report" check_documentation_coverage
run_advisory "Generated-source substring inventory" check_generated_source_inventory
if ((disk_preflight_passed)); then
    run_stable "Release build" cargo build --release --quiet
else
    skip_stable \
        "Release build" \
        "Skipping release build because disk space preflight failed."
fi
run_advisory "Unwrap/expect policy report" check_unwrap_expect

printf '\nSummary\n'
printf 'Stable gates:   %s passed, %s failed\n' "$((stable_total - stable_failed))" "${stable_failed}"
printf 'Advisory gates: %s passed, %s reported issues\n' "$((advisory_total - advisory_failed))" "${advisory_failed}"

if ((stable_failed > 0)); then
    printf 'Stable quality gates failed.\n'
    exit 1
fi

if [[ "${STRICT_ADVISORY}" == "1" && "${advisory_failed}" != "0" ]]; then
    printf 'Advisory gates failed under TCT_STRICT_QUALITY=1.\n'
    exit 1
fi

printf 'Stable quality gates passed.\n'
if ((advisory_failed > 0)); then
    printf 'Advisory issues remain; see the report above.\n'
fi
