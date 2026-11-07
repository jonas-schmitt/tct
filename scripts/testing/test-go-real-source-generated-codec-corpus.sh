#!/usr/bin/env bash
# Run the maintained Go real-source generated-codec corpus sidecar tests.
# This validates parser-backed generated-codec round trips only; it does not
# promote parser, formatter, type-checker, arena IR, or whole-file source
# validity claims.

set -euo pipefail

script_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
repo_root=$(cd "$script_dir/../.." && pwd)

# Unified pinned Go export before gates / bare go.
if [[ -f "${repo_root}/scripts/testing/resolve-pinned-go.sh" ]]; then
    # shellcheck disable=SC1091
    eval "$("${repo_root}/scripts/testing/resolve-pinned-go.sh" --export --repo-root "${repo_root}")"
fi

timeout_seconds=${TCT_GO_REAL_SOURCE_GENERATED_CODEC_CORPUS_TIMEOUT_SECONDS:-600}
report_dir="target/tct-syntax-real-source-demo"

cd "$repo_root"

tests=(
    "go_strings_compare_source_file_syntax_generated_codec_compiles_at_sidecar_boundary"
    "go_crypto_sysrand_rand_windows_source_file_syntax_generated_codec_compiles_at_sidecar_boundary"
    "go_strings_clone_source_file_syntax_generated_codec_compiles_at_sidecar_boundary"
    "go_sort_interface_source_file_syntax_generated_codec_compiles_at_sidecar_boundary"
    "go_sort_search_source_file_syntax_generated_codec_compiles_at_sidecar_boundary"
    "go_strings_search_source_file_syntax_generated_codec_compiles_at_sidecar_boundary"
    "go_path_match_source_file_syntax_generated_codec_compiles_at_sidecar_boundary"
)

labels=(
    "strings/compare.go"
    "crypto/internal/sysrand/rand_windows.go"
    "strings/clone.go"
    "sort/example_interface_test.go"
    "sort/example_search_test.go"
    "strings/search.go"
    "path/match.go"
)

reports=(
    "$report_dir/go-strings-compare-generated-codec-demo.json"
    "$report_dir/go-crypto-sysrand-rand-windows-generated-codec-demo.json"
    "$report_dir/go-strings-clone-generated-codec-demo.json"
    "$report_dir/go-sort-interface-generated-codec-demo.json"
    "$report_dir/go-sort-search-generated-codec-demo.json"
    "$report_dir/go-strings-search-generated-codec-demo.json"
    "$report_dir/go-path-match-generated-codec-demo.json"
)

section() {
    printf '\n[%s]\n' "$1"
}

mkdir -p "$report_dir"
rm -f "${reports[@]}"

for index in "${!tests[@]}"; do
    section "Go real-source generated-codec corpus: ${labels[$index]}"
    timeout "$timeout_seconds" cargo test --features experimental \
        --test syntax_generated_codec_gate_test \
        "${tests[$index]}" \
        -- \
        --exact

    report="${reports[$index]}"
    case "$report" in
        target/tct-syntax-real-source-demo/*.json) ;;
        *)
            echo "generated-codec corpus report path escaped target/: $report" >&2
            exit 1
            ;;
    esac
    if [[ ! -f "$report" ]]; then
        echo "expected generated-codec corpus report was not written: $report" >&2
        exit 1
    fi
done

section "Go real-source generated-codec corpus reports"
for report in "${reports[@]}"; do
    printf '%s\n' "$report"
done
