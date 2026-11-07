#!/usr/bin/env bash
# Run the manifest-backed syntax backend sidecar gate bundle.
# This is sidecar validation only; it does not promote GrammarIR or
# StructuralModel into the production JSON Schema compiler path.

set -euo pipefail

script_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
repo_root=$(cd "$script_dir/../.." && pwd)
timeout_seconds=${TCT_SYNTAX_BACKEND_GATE_TIMEOUT_SECONDS:-600}

cd "$repo_root"

# Unified pinned Go: export TCT_GO_BIN / TCT_GOFMT_BIN / PATH pin before gates.
if [[ -f "${repo_root}/scripts/testing/resolve-pinned-go.sh" ]]; then
    # shellcheck disable=SC1091
    eval "$("${repo_root}/scripts/testing/resolve-pinned-go.sh" --export --repo-root "${repo_root}")"
fi


export TCT_REQUIRE_GO_SIDECARS="${TCT_REQUIRE_GO_SIDECARS:-1}"

printf 'TCT_REQUIRE_GO_SIDECARS=%s\n' "$TCT_REQUIRE_GO_SIDECARS"

section() {
    printf '\n[%s]\n' "$1"
}

csharp_checked_dotnet_version() {
    local fixture=$1
    python3 - "$fixture" <<'PY'
import importlib.util
import json
import sys
from pathlib import Path

fixture = sys.argv[1]
module_path = Path("scripts/analysis/csharp_syntax_metadata_smoke.py")
spec = importlib.util.spec_from_file_location("csharp_syntax_metadata_smoke", module_path)
module = importlib.util.module_from_spec(spec)
assert spec.loader is not None
spec.loader.exec_module(module)
checked_metadata = module.FIXTURES[fixture][1]
with checked_metadata.open(encoding="utf-8") as handle:
    print(json.load(handle)["parser"]["version"])
PY
}

dotnet_bin_for_version() {
    local version=$1
    local candidate

    candidate="$repo_root/target/dotnet-sdk-$version/dotnet"
    if [[ -x "$candidate" ]]; then
        printf '%s\n' "$candidate"
        return 0
    fi

    candidate="/usr/local/dotnet/dotnet"
    if [[ -x "$candidate" ]] && [[ "$("$candidate" --version)" == "$version" ]]; then
        printf '%s\n' "$candidate"
        return 0
    fi

    candidate=$(command -v dotnet || true)
    if [[ -n "$candidate" ]] && [[ "$(dotnet --version)" == "$version" ]]; then
        printf '%s\n' "$candidate"
        return 0
    fi

    return 1
}

run_csharp_metadata_smoke() {
    if [[ "${1:-}" == "--fixture" ]]; then
        shift
    fi
    local fixture=$1
    local checked_version
    local dotnet_bin
    local dotnet_root

    checked_version=$(csharp_checked_dotnet_version "$fixture")
    if dotnet_bin=$(dotnet_bin_for_version "$checked_version"); then
        dotnet_root=$(dirname "$dotnet_bin")
        timeout "$timeout_seconds" env \
            DOTNET_ROOT="$dotnet_root" \
            PATH="$dotnet_root:$PATH" \
            python3 scripts/analysis/csharp_syntax_metadata_smoke.py \
                --fixture "$fixture" \
                --dotnet-bin "$dotnet_bin" \
                --self-test
    else
        timeout "$timeout_seconds" python3 scripts/analysis/csharp_syntax_metadata_smoke.py \
            --fixture "$fixture" \
            --self-test
    fi
}

section "Go token layout pin gate"
timeout "$timeout_seconds" cargo test --features experimental --test syntax_backend_boundary_test \
    go_token_layout_pin_fixture_matches_live_layouts_and_breaking_transition -- --exact

section "Syntax generated codec gate"
timeout "$timeout_seconds" cargo test --features experimental --test syntax_generated_codec_gate_test

section "C# syntax parser metadata smoke"
run_csharp_metadata_smoke --fixture minimal-class
run_csharp_metadata_smoke --fixture local-string-return-literal
run_csharp_metadata_smoke --fixture local-integer-return-literal
run_csharp_metadata_smoke --fixture local-boolean-return-literal
run_csharp_metadata_smoke --fixture local-typed-declaration
run_csharp_metadata_smoke --fixture local-implicit-declaration
run_csharp_metadata_smoke --fixture local-integer-implicit-declaration
run_csharp_metadata_smoke --fixture local-assignment
run_csharp_metadata_smoke --fixture local-escaped-assignment

section "Go syntax parser metadata smokes"
timeout "$timeout_seconds" go run scripts/analysis/go_syntax_metadata_smoke.go --self-test
timeout "$timeout_seconds" go run scripts/analysis/go_syntax_metadata_smoke.go \
    --fixture local-declaration \
    --self-test
timeout "$timeout_seconds" go run scripts/analysis/go_syntax_metadata_smoke.go \
    --fixture local-assignment \
    --self-test
timeout "$timeout_seconds" go run scripts/analysis/go_syntax_metadata_smoke.go \
    --fixture local-escaped-assignment \
    --self-test
timeout "$timeout_seconds" go run scripts/analysis/go_syntax_metadata_smoke.go \
    --fixture local-short-var-declaration \
    --self-test
timeout "$timeout_seconds" go run scripts/analysis/go_syntax_metadata_smoke.go \
    --fixture local-integer-short-var-declaration \
    --self-test
timeout "$timeout_seconds" go run scripts/analysis/go_syntax_metadata_smoke.go \
    --fixture local-integer-return-literal \
    --self-test
timeout "$timeout_seconds" go run scripts/analysis/go_syntax_metadata_smoke.go \
    --fixture local-parenthesized-integer-return-expression \
    --self-test
timeout "$timeout_seconds" go run scripts/analysis/go_syntax_metadata_smoke.go \
    --fixture local-unary-integer-return-expression \
    --self-test
timeout "$timeout_seconds" go run scripts/analysis/go_syntax_metadata_smoke.go \
    --fixture local-binary-integer-return-expression \
    --self-test
timeout "$timeout_seconds" go run scripts/analysis/go_syntax_metadata_smoke.go \
    --fixture local-call-return-expression \
    --self-test
timeout "$timeout_seconds" go run scripts/analysis/go_syntax_metadata_smoke.go \
    --fixture local-argument-call-return-expression \
    --self-test
timeout "$timeout_seconds" go run scripts/analysis/go_syntax_metadata_smoke.go \
    --fixture local-string-return-literal \
    --self-test
timeout "$timeout_seconds" go run scripts/analysis/go_syntax_metadata_smoke.go \
    --fixture local-float-return-literal \
    --self-test
timeout "$timeout_seconds" go run scripts/analysis/go_syntax_metadata_smoke.go \
    --fixture local-boolean-return-identifier \
    --self-test
timeout "$timeout_seconds" go run scripts/analysis/go_syntax_metadata_smoke.go \
    --fixture local-nil-return-identifier \
    --self-test
timeout "$timeout_seconds" go run scripts/analysis/go_syntax_metadata_smoke.go \
    --fixture local-typed-declaration \
    --self-test
timeout "$timeout_seconds" go run scripts/analysis/go_syntax_metadata_smoke.go \
    --fixture local-zero-value-typed-declaration \
    --self-test
timeout "$timeout_seconds" go run scripts/analysis/go_syntax_metadata_smoke.go \
    --fixture local-integer-typed-declaration \
    --self-test
timeout "$timeout_seconds" go run scripts/analysis/go_syntax_metadata_smoke.go \
    --fixture local-untyped-integer-declaration \
    --self-test
timeout "$timeout_seconds" go run scripts/analysis/go_syntax_metadata_smoke.go \
    --fixture local-integer-assignment \
    --self-test
timeout "$timeout_seconds" go run scripts/analysis/go_syntax_metadata_smoke.go \
    --fixture local-compound-assignment \
    --self-test
timeout "$timeout_seconds" go run scripts/analysis/go_syntax_metadata_smoke.go \
    --fixture local-increment-decrement \
    --self-test
timeout "$timeout_seconds" go run scripts/analysis/go_syntax_metadata_smoke.go \
    --fixture local-branch-statement \
    --self-test
timeout "$timeout_seconds" go run scripts/analysis/go_syntax_metadata_smoke.go \
    --fixture local-if-else \
    --self-test
timeout "$timeout_seconds" go run scripts/analysis/go_syntax_metadata_smoke.go \
    --fixture local-for-condition \
    --self-test
timeout "$timeout_seconds" go run scripts/analysis/go_syntax_metadata_smoke.go \
    --fixture local-switch \
    --self-test
timeout "$timeout_seconds" go run scripts/analysis/go_syntax_metadata_smoke.go \
    --fixture local-range-string \
    --self-test
timeout "$timeout_seconds" go run scripts/analysis/go_syntax_metadata_smoke.go \
    --fixture package-type-declaration \
    --self-test
timeout "$timeout_seconds" go run scripts/analysis/go_syntax_metadata_smoke.go \
    --fixture method-declaration \
    --self-test
timeout "$timeout_seconds" go run scripts/analysis/go_syntax_metadata_smoke.go \
    --fixture struct-type \
    --self-test
timeout "$timeout_seconds" go run scripts/analysis/go_syntax_metadata_smoke.go \
    --fixture composite-literal \
    --self-test
timeout "$timeout_seconds" go run scripts/analysis/go_syntax_metadata_smoke.go \
    --fixture key-value-composite-literal \
    --self-test
timeout "$timeout_seconds" go run scripts/analysis/go_syntax_metadata_smoke.go \
    --fixture array-composite-literal \
    --self-test
timeout "$timeout_seconds" go run scripts/analysis/go_syntax_metadata_smoke.go \
    --fixture index-expression \
    --self-test
timeout "$timeout_seconds" go run scripts/analysis/go_syntax_metadata_smoke.go \
    --fixture slice-expression \
    --self-test
timeout "$timeout_seconds" go run scripts/analysis/go_syntax_metadata_smoke.go \
    --fixture star-expression \
    --self-test
timeout "$timeout_seconds" go run scripts/analysis/go_syntax_metadata_smoke.go \
    --fixture package-var-declaration \
    --self-test
timeout "$timeout_seconds" go run scripts/analysis/go_syntax_metadata_smoke.go \
    --fixture function-literal \
    --self-test

section "Go syntax real-source corpus sidecar check"
timeout "$timeout_seconds" python3 scripts/analysis/route_source_manifest.py --self-test
timeout "$timeout_seconds" go run scripts/analysis/go_syntax_corpus_check.go --self-test
timeout "$timeout_seconds" go run scripts/analysis/go_syntax_corpus_check.go \
    --manifest tests/fixtures/go_syntax_corpus/manifest-v1.json \
    --json-output target/tct-analysis/go-syntax-corpus-check/report.json

section "Syntax StructuralModel conformance gate"
timeout "$timeout_seconds" cargo test --features experimental --test syntax_structural_model_conformance_test

section "Syntax StructuralModel cross-target equivalence gate"
timeout "$timeout_seconds" cargo test --features experimental --test syntax_structural_model_cross_target_equivalence_test

section "Syntax state-machine replay and parity gate"
timeout "$timeout_seconds" cargo test --features experimental --test syntax_state_machine_replay_test

# Mixed-payload contract unit tests live under --lib (model_facts + raw_replay
# modules), not under a dedicated integration test binary; keep them inside
# the primary syntax gate named for syntax work.
section "Syntax state-machine mixed-payload unit gate"
timeout "$timeout_seconds" cargo test --features experimental --lib syntax_state_machine

section "Syntax production route table and generator API boundary gate"
timeout "$timeout_seconds" cargo test --features experimental --test syntax_backend_boundary_test
