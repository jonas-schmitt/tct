#!/usr/bin/env bash
# Run the checker-ready Go corpus generated-codec round-trip gate.
#
# A blocked gate is valid when it records a precise first generated-codec
# blocker for the selected source. Invalid selection/checker evidence or an
# unstable gate report still fails this script.

set -euo pipefail

script_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
repo_root=$(cd "$script_dir/../.." && pwd)

# Unified pinned Go export (TCT_GO_BIN / PATH) before any bare `go` use.
if [[ -f "${repo_root}/scripts/testing/resolve-pinned-go.sh" ]]; then
    # shellcheck disable=SC1091
    eval "$("${repo_root}/scripts/testing/resolve-pinned-go.sh" --export --repo-root "${repo_root}")"
fi


source_root=${TCT_GO_GENERATED_CODEC_GATE_SOURCE_ROOT:-"$(go env GOROOT)/src"}
existing_manifest=${TCT_GO_GENERATED_CODEC_GATE_EXISTING_MANIFEST:-tests/fixtures/go_syntax_corpus/training-manifest-fifteen-record-v1.json}
report_ingestion_manifest=${TCT_GO_GENERATED_CODEC_GATE_REPORT_INGESTION_MANIFEST:-tests/fixtures/go_syntax_corpus/training-report-ingestion-current-plus-v1.json}
exclude_report_ingestion_manifest=${TCT_GO_GENERATED_CODEC_GATE_EXCLUDE_REPORT_INGESTION_MANIFEST:-"$report_ingestion_manifest"}
selection_report=${TCT_GO_GENERATED_CODEC_GATE_SELECTION_REPORT:-target/tct-analysis/go-syntax-corpus-selection/current-plus-generated-codec-selection.json}
checker_report=${TCT_GO_GENERATED_CODEC_GATE_CHECKER_REPORT:-target/tct-analysis/go-syntax-corpus-check/current-plus-generated-codec-gap-report.json}
gate_report=${TCT_GO_GENERATED_CODEC_GATE_REPORT:-target/tct-analysis/go-generated-codec-round-trip-gate/current-plus-report.json}
max_files=${TCT_GO_GENERATED_CODEC_GATE_MAX_FILES:-120}
max_file_bytes=${TCT_GO_GENERATED_CODEC_GATE_MAX_FILE_BYTES:-12000}
max_total_bytes=${TCT_GO_GENERATED_CODEC_GATE_MAX_TOTAL_BYTES:-720000}

cd "$repo_root"

section() {
    printf '\n[%s]\n' "$1"
}

section "Go generated-codec round-trip gate: raw-tree renderer coverage audit"
# This audit proves inventory-backed raw-tree renderer rule readiness and
# skeleton dispatch coverage only. It is not GoIR, ArenaIR, route-handoff,
# BPE, source-program validity, or stable-backend promotion evidence.
cargo test --features experimental --test go_ast_schema_inventory_test \
    checked_go_ast_schema_inventory_classifies_raw_tree_renderer_codegen_coverage \
    -- --exact

section "Go generated-codec round-trip gate: selection"
go run scripts/analysis/go_syntax_corpus_select.go \
    --source-root "$source_root" \
    --existing-manifest "$existing_manifest" \
    --exclude-existing \
    --exclude-report-ingestion-manifest "$exclude_report_ingestion_manifest" \
    --include-test-files=false \
    --dedupe-shapes \
    --max-files "$max_files" \
    --max-file-bytes "$max_file_bytes" \
    --max-total-bytes "$max_total_bytes" \
    --output "$selection_report"

section "Go generated-codec round-trip gate: checker"
go run scripts/analysis/go_syntax_corpus_check.go \
    --selection-report "$selection_report" \
    --json-output "$checker_report"

section "Go generated-codec round-trip gate: source-instance reports"
cargo test --features experimental --test syntax_go_source_instance_projection_test \
    go_source_instance_projection_produces_generated_codec_reports_from_checked_data \
    -- --exact

section "Go generated-codec round-trip gate: report"
go run scripts/analysis/go_syntax_generated_codec_round_trip_gate.go \
    --selection-report "$selection_report" \
    --checker-report "$checker_report" \
    --report-ingestion-manifest "$report_ingestion_manifest" \
    --output "$gate_report"

go run scripts/analysis/go_syntax_generated_codec_round_trip_gate.go \
    --selection-report "$selection_report" \
    --checker-report "$checker_report" \
    --report-ingestion-manifest "$report_ingestion_manifest" \
    --validate-report "$gate_report"

python3 - "$gate_report" <<'PY'
import json
import sys

path = sys.argv[1]
with open(path, "r", encoding="utf-8") as handle:
    report = json.load(handle)
print(
    f"{path}: status={report['status']} "
    f"supported={report['supported_record_count']} "
    f"blocked={report['blocked_record_count']}"
)
blocker = report.get("first_generated_codec_blocker")
if blocker:
    print(f"first_blocker={blocker['kind']} path={blocker['path']}")
PY
