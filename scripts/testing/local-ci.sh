#!/usr/bin/env bash
# Mirror the ordinary repository CI gate locally.
#
# This intentionally covers the stable job in .github/workflows/ci.yml without
# requiring GitHub-hosted Actions. The quality helper's `cargo test` already
# runs the default round-trip smoke (`round_trip_test::test_round_trip` is not
# ignored), so no separate round-trip step is needed. Extended, release-depth,
# wheel, and evidence gates remain opt-in and should be run separately when
# touched behavior needs them.

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
dry_run=0

usage() {
    cat <<'EOF'
Usage: scripts/testing/local-ci.sh [--dry-run]

Run the ordinary local CI mirror:
  1. scripts/testing/quality-check.sh
     (its cargo test run includes the default round-trip smoke,
     round_trip_test::test_round_trip)

Options:
  --dry-run  Print the commands without executing them.
  -h, --help Show this help text.
EOF
}

while (($#)); do
    case "$1" in
        --dry-run)
            dry_run=1
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        *)
            printf 'Unknown argument: %s\n\n' "$1" >&2
            usage >&2
            exit 2
            ;;
    esac
    shift
done

section() {
    printf '\n[%s]\n' "$1"
}

format_command() {
    printf '%q ' "$@"
    printf '\n'
}

run_step() {
    local name="$1"
    shift
    section "$name"
    if ((dry_run)); then
        format_command "$@"
    else
        "$@"
    fi
}

cd "$repo_root"



export CARGO_TERM_COLOR="${CARGO_TERM_COLOR:-always}"
# Preserve a caller override; otherwise use a local-ci-owned cache under the
# shared tct-generated-cargo tree.
export TCT_GENERATED_CARGO_TARGET_DIR="${TCT_GENERATED_CARGO_TARGET_DIR:-${repo_root}/target/tct-generated-cargo/local-ci-default-round-trip}"

printf 'TCT local CI mirror\n'
printf 'Repository: %s\n' "$repo_root"
printf 'TCT_GENERATED_CARGO_TARGET_DIR=%s\n' "$TCT_GENERATED_CARGO_TARGET_DIR"
if command -v rustup >/dev/null 2>&1; then
    printf 'Active Rust toolchain: '
    rustup show active-toolchain || true
fi

run_step "Stable quality helper" ./scripts/testing/quality-check.sh
