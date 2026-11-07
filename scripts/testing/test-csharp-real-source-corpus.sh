#!/usr/bin/env bash
# Validate the checked C# real-source corpus selection. This is route-planning
# evidence only; it does not promote arena IR, source-program validity,
# generated-codec real-source coverage, or stable backend status.
#
# The retired C# arena frontend evidence helper
# (`test-csharp-frontend-evidence.sh` / `csharp_evidence_summary.py`) is not
# invoked. This selection gate only validates `route_source_manifest` output
# against the checked selection manifest. Retained C# toolchain/metadata smokes
# stay separate helpers.

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "${repo_root}"

corpus_root="corpora/csharp/dotnet-samples-mini"
source_root="${corpus_root}/src"
source_manifest="${corpus_root}/sources.txt"
selection_manifest="${corpus_root}/manifest-v1.json"
output_dir="${TCT_CSHARP_REAL_SOURCE_OUTPUT_DIR:-target/tct-csharp-real-source-corpus}"
generated_manifest="${output_dir}/sources.txt"
route_report="${output_dir}/route-source-manifest.json"
route_markdown="${output_dir}/route-source-manifest.md"

mkdir -p "${output_dir}"

python3 scripts/analysis/route_source_manifest.py \
    --target csharp \
    --source-root "${source_root}" \
    --output "${generated_manifest}" \
    --json-output "${route_report}" \
    --markdown-output "${route_markdown}" \
    --min-files 5 \
    --corpus-id csharp-dotnet-samples-mini-v1 \
    --corpus-source https://github.com/dotnet/samples/tree/c6eb3268ba912447e820ba78e3d35ee2c5d7e0b0 \
    --corpus-license MIT \
    --selection-note 'Small standalone C# files from dotnet/samples chosen for parser/toolchain route planning; sidecar evidence only.'

if ! cmp -s "${source_manifest}" "${generated_manifest}"; then
    echo "checked C# real-source manifest is stale: ${source_manifest}" >&2
    diff -u "${source_manifest}" "${generated_manifest}" >&2 || true
    exit 1
fi

python3 - "${selection_manifest}" "${route_report}" <<'PY'
import json
import pathlib
import sys

manifest_path = pathlib.Path(sys.argv[1])
report_path = pathlib.Path(sys.argv[2])
selection = json.loads(manifest_path.read_text(encoding="utf-8"))
report = json.loads(report_path.read_text(encoding="utf-8"))

def require(condition: bool, message: str) -> None:
    if not condition:
        raise SystemExit(message)

require(selection.get("artifact_kind") == "tct_csharp_real_source_corpus_selection", "unexpected artifact kind")
require(selection.get("version") == 1, "unexpected manifest version")
require(report.get("tool") == "route_source_manifest", "unexpected route manifest tool")
require(report.get("target") == "csharp", "unexpected route manifest target")
require(report.get("fixture_scope") is False, "C# real-source corpus must not be fixture-scoped")
require(report.get("evidence_role") == "route_intended_corpus", "unexpected evidence role")

source_manifest = selection["source_manifest"]
route_fingerprint = selection["route_source_manifest_report"]
require(report["entry_count"] == source_manifest["entry_count"], "entry count mismatch")
require(report["manifest"]["sha256"] == source_manifest["sha256"], "source manifest hash mismatch")
require(
    report["corpus_fingerprint"]["sha256"]
    == route_fingerprint["corpus_fingerprint_sha256"],
    "route-source corpus fingerprint mismatch",
)
require(
    report["corpus_fingerprint"]["fingerprint_kind"]
    == route_fingerprint["corpus_fingerprint_kind"],
    "route-source corpus fingerprint kind mismatch",
)

expected_entries = source_manifest["entries"]
reported_entries = report["entries"]
require(len(reported_entries) == len(expected_entries), "reported entries length mismatch")
for expected, actual in zip(expected_entries, reported_entries):
    require(
        actual["manifest_path"] == expected["manifest_path"],
        f"entry path mismatch: {actual['manifest_path']} != {expected['manifest_path']}",
    )
    require(
        actual["file"]["sha256"] == expected["sha256"],
        f"entry hash mismatch: {expected['manifest_path']}",
    )
    require(
        actual["file"]["bytes"] == expected["bytes"],
        f"entry byte length mismatch: {expected['manifest_path']}",
    )

provenance = report["corpus_provenance"]
corpus = selection["corpus"]
require(provenance["corpus_id"] == corpus["corpus_id"], "corpus id mismatch")
require(provenance["source"] == corpus["source"], "corpus source mismatch")
require(provenance["license"] == corpus["license"], "corpus license mismatch")

print(
    "C# real-source corpus selection: "
    f"files={source_manifest['entry_count']}, "
    f"route_role={report.get('evidence_role')}"
)
PY
