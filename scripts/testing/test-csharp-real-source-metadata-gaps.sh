#!/usr/bin/env bash
# Validate parser-backed C# real-source syntax-metadata gap evidence. This gate
# records unsupported syntax-level gaps only; it does not promote source-program
# validity, generated-codec real-source coverage, arena IR, or stable backend
# status.

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "${repo_root}"

output_dir="${TCT_CSHARP_REAL_SOURCE_METADATA_GAPS_OUTPUT_DIR:-target/tct-csharp-real-source-metadata-gaps}"
report_json="${output_dir}/metadata-gaps.json"
report_md="${output_dir}/metadata-gaps.md"

./scripts/testing/test-csharp-real-source-corpus.sh

python3 scripts/analysis/csharp_real_source_metadata_gaps.py \
    --source-manifest corpora/csharp/dotnet-samples-mini/sources.txt \
    --source-root corpora/csharp/dotnet-samples-mini/src \
    --json-output "${report_json}" \
    --markdown-output "${report_md}" \
    --require-parser-ready

python3 - "${report_json}" <<'PY'
import json
import pathlib
import sys

report = json.loads(pathlib.Path(sys.argv[1]).read_text(encoding="utf-8"))

def require(condition: bool, message: str) -> None:
    if not condition:
        raise SystemExit(message)

require(report["tool"] == "csharp_real_source_metadata_gaps", "unexpected tool")
require(report["version"] == 1, "unexpected report version")
require(
    report["route_readiness_boundary"]["parser_backed_gap_report_ready"] is True,
    "parser-backed gap report is not ready",
)
require(report["summary"]["files"] == 5, "unexpected source file count")
require(report["summary"]["parsed_files"] == 5, "not all source files parsed")
require(report["summary"]["parse_diagnostics"] == 0, "parse diagnostics present")
require(report["summary"]["unsupported_gap_count"] == 0, "unexpected unsupported route gaps")
require(report["summary"]["unsupported_node_kind_count"] == 0, "unexpected unsupported node kinds")
require(report["next_smallest_tranche"] is None, "expected no remaining C# gap tranche")
require(
    "no_arena_ir_promotion"
    in report["route_readiness_boundary"]["no_stable_claims"],
    "missing no-arena-IR claim",
)
print(
    "C# real-source metadata gaps: "
    f"files={report['summary']['files']}, "
    f"unsupported_gaps={report['summary']['unsupported_gap_count']}, "
    "next=none"
)
PY
