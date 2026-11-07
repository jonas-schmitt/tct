//! Smoke tests for the experimental frontend CLI binaries.
//!
//! Covers the Go tokenizer-generator and syntax source-corpus/program CLIs,
//! plus a `--help` smoke over every maintained binary. The core JSON Schema
//! CLI binaries are exercised in `tests/cli_binaries_test.rs`.

use serde_json::json;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tct::go_ast_schema_inventory::{
    GO_AST_RAW_TREE_RENDERER_CODEGEN_COVERAGE_NEXT_READY_TASK,
    GO_AST_RAW_TREE_RENDERER_CODEGEN_COVERAGE_STATUS_READY,
    GO_AST_SCHEMA_GRAMMAR_COVERAGE_NEXT_READY_TASK, GO_AST_SCHEMA_GRAMMAR_COVERAGE_STATUS_READY,
    GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH,
};
use tct::provenance::sha256_hex;
use tct::syntax_go_tokenizer_generator::{
    DEFAULT_GO_RAW_TREE_AST_PRINTER_ORACLE_REPORT_PATH,
    GO_RAW_TREE_AST_PRINTER_ORACLE_CUTOVER_NEXT_READY_TASK,
    GO_RAW_TREE_AST_PRINTER_ORACLE_CUTOVER_STATUS_ACCEPTED,
    MANIFEST_DEPTH_ARTIFACT_KIND as GO_TOKENIZER_GENERATOR_MANIFEST_DEPTH_ARTIFACT_KIND,
    NEXT_READY_TASK as GO_TOKENIZER_GENERATOR_NEXT_READY_TASK,
    RAW_DECODED_TREE_RENDER_READY_STATUS, RAW_TREE_RENDER_BROADENING_DECISION_SOURCE,
    RAW_TREE_RENDER_BROADENING_DIAGNOSTIC_ACTION,
    RAW_TREE_RENDER_BROADENING_DIAGNOSTIC_BLOCKED_STATUS,
    RAW_TREE_RENDER_BROADENING_GRAMMAR_MODE_ACTION,
    RAW_TREE_RENDER_BROADENING_GRAMMAR_MODE_READY_STATUS,
    RAW_TREE_RENDER_BROADENING_MANIFEST_ONLY_ACTION,
    RAW_TREE_RENDER_BROADENING_NEXT_CANDIDATE_STATUS, RAW_TREE_RENDER_BROADENING_NEXT_READY_TASK,
};
use tct::syntax_production_route::{SYNTAX_CODEC_REGISTRY, syntax_codec_registry_entry};
use tempfile::TempDir;

const GO_SOURCE_DRIVEN_TOKENIZER_GENERATOR_CORPUS_MANIFEST_PATH: &str =
    "tests/fixtures/go_syntax_corpus/source-driven-tokenizer-generator-corpus-v1.json";
const GO_CURRENT_PLUS_REPORT_INGESTION_MANIFEST_PATH: &str =
    "tests/fixtures/go_syntax_corpus/training-report-ingestion-current-plus-v1.json";
const GO_SOURCE_DRIVEN_TOKENIZER_GENERATOR_GRAMMAR_GAP_REPORT_PATH: &str =
    "target/tct-syntax-go-tokenizer-generator/source-driven-grammar-gap-report-v1.json";
const GO_REAL_SOURCE_CORPUS_MANIFEST_PATH: &str =
    "tests/fixtures/go_syntax_corpus/manifest-v1.json";
const GO_REAL_SOURCE_CORPUS_CHECK_REPORT_PATH: &str =
    "target/tct-analysis/go-syntax-corpus-check/report.json";
const GO_REAL_SOURCE_GENERATED_CODEC_ROUND_TRIP_BLOCKER: &str =
    "real_source_end_to_end_generated_codec_round_trip";
const GO_SOURCE_DRIVEN_GRAMMAR_GAP_REPORT_ARTIFACT_KIND: &str =
    "tct_go_source_driven_tokenizer_generator_grammar_gap_report_v1";
const GO_SOURCE_DRIVEN_GRAMMAR_GAP_REPORT_STATUS: &str =
    "ready_without_under_proven_registry_category";
const GO_SOURCE_DRIVEN_GRAMMAR_GAP_CLAIM_BOUNDARY: &str = "source-driven Go tokenizer-generator grammar-gap report only; parser-backed metadata, GrammarIR, StructuralModel, token layout, and raw syntax state-machine evidence remain the validity boundary. Scratch generation starts from a precompiled Go syntax tokenizer artifact and an empty GoSourceFile token-space prefix, not per-output parser metadata. This is not Arena IR, route-handoff provenance, exact-BPE source-prefix materialization, downstream artifact-ladder, paper, whole-language, type-correctness, or stable-backend evidence.";
const GO_SOURCE_DRIVEN_NO_STABLE_CLAIMS: &[&str] = &[
    "no_arena_ir_claim",
    "no_bpe_validity_authority",
    "no_downstream_artifact_ladder_claim",
    "no_go_file_specific_branch",
    "no_paper_facing_claim",
    "no_route_handoff_provenance_claim",
    "no_stable_backend_claim",
    "no_type_correctness_claim",
    "no_whole_language_claim",
];
const GO_GRAMMAR_CATEGORY_ORDER: &[&str] = &[
    "packages_imports_declarations",
    "functions_methods",
    "statements_control_flow",
    "expressions_and_precedence",
    "types_and_generics",
    "literals_and_composites",
    "comments_directives_build_tags",
    "uncategorized",
];

struct GoUnderProvenCategorySpec {
    category_id: &'static str,
    grammar_category: &'static str,
    description: &'static str,
    node_kinds: &'static [&'static str],
    grammar_fields: &'static [&'static str],
    payload_kinds: &'static [&'static str],
    smallest_corpus_broadening: &'static str,
}

const GO_UNDER_PROVEN_CATEGORY_SPECS: &[GoUnderProvenCategorySpec] = &[
    GoUnderProvenCategorySpec {
        category_id: "concurrency_channels",
        grammar_category: "statements_control_flow",
        description: "channel, goroutine, communication clause, and select statement syntax",
        node_kinds: &["ChanType", "CommClause", "GoStmt", "SelectStmt", "SendStmt"],
        grammar_fields: &[
            "ChanType.Dir",
            "ChanType.Value",
            "CommClause.Body",
            "CommClause.Comm",
            "GoStmt.Call",
            "SelectStmt.Body",
            "SendStmt.Chan",
            "SendStmt.Value",
        ],
        payload_kinds: &["string_payload"],
        smallest_corpus_broadening: "promote a registry-owned source-driven Go record that covers channel/select/send syntax as data before downstream artifact work",
    },
    GoUnderProvenCategorySpec {
        category_id: "type_switches",
        grammar_category: "statements_control_flow",
        description: "type-switch statement syntax at the raw source-driven tokenizer-generator boundary",
        node_kinds: &["TypeSwitchStmt"],
        grammar_fields: &["TypeSwitchStmt.Assign", "TypeSwitchStmt.Body"],
        payload_kinds: &[],
        smallest_corpus_broadening: "add a registry-owned type-switch source-driven Go tokenizer-generator corpus record as a manifest/metadata change, not a Rust branch",
    },
    GoUnderProvenCategorySpec {
        category_id: "switch_initializers",
        grammar_category: "statements_control_flow",
        description: "switch statements with init clauses at the raw source-driven tokenizer-generator boundary",
        node_kinds: &[],
        grammar_fields: &["SwitchStmt.Init"],
        payload_kinds: &[],
        smallest_corpus_broadening: "add the registry-owned local-switch-init source-driven Go tokenizer-generator corpus record as a manifest/metadata change",
    },
    GoUnderProvenCategorySpec {
        category_id: "variadic_and_three_index_sequences",
        grammar_category: "types_and_generics",
        description: "variadic ellipsis and three-index slice syntax",
        node_kinds: &["Ellipsis"],
        grammar_fields: &[
            "CallExpr.Ellipsis",
            "Ellipsis.Elt",
            "SliceExpr.Max",
            "SliceExpr.Slice3",
        ],
        payload_kinds: &[],
        smallest_corpus_broadening: "add a source-driven corpus record that covers variadic ellipsis or three-index slice metadata without source-specific generated-code branches",
    },
    GoUnderProvenCategorySpec {
        category_id: "declaration_comment_and_import_metadata",
        grammar_category: "comments_directives_build_tags",
        description: "import aliases, import comments, and declaration comment fields",
        node_kinds: &[],
        grammar_fields: &[
            "ImportSpec.Comment",
            "ImportSpec.Name",
            "TypeSpec.Assign",
            "TypeSpec.Comment",
            "TypeSpec.Doc",
            "ValueSpec.Comment",
        ],
        payload_kinds: &[],
        smallest_corpus_broadening: "add a source-driven corpus record that exercises declaration/comment metadata as registry-backed syntax coverage",
    },
    GoUnderProvenCategorySpec {
        category_id: "boolean_payloads",
        grammar_category: "literals_and_composites",
        description: "boolean terminal payload syntax",
        node_kinds: &[],
        grammar_fields: &[],
        payload_kinds: &["boolean_payload"],
        smallest_corpus_broadening: "add a source-driven corpus record that proves boolean payload syntax through the raw tokenizer-generator boundary",
    },
];

#[derive(Default, Debug, Clone)]
struct GoGrammarCoverage {
    node_kinds: BTreeSet<String>,
    grammar_fields: BTreeSet<String>,
    payload_kinds: BTreeSet<String>,
    category_counts: BTreeMap<String, usize>,
}

impl GoGrammarCoverage {
    fn merge(&mut self, other: &GoGrammarCoverage) {
        self.node_kinds.extend(other.node_kinds.iter().cloned());
        self.grammar_fields
            .extend(other.grammar_fields.iter().cloned());
        self.payload_kinds
            .extend(other.payload_kinds.iter().cloned());
        for (category, count) in &other.category_counts {
            *self.category_counts.entry(category.clone()).or_default() += count;
        }
    }
}

/// The source-record CLIs emit one record per registered route-manifest
/// construct, so the expected count is derived from the checked manifest
/// instead of a literal that breaks on every route promotion.
fn production_route_manifest_record_count() -> u64 {
    let manifest: serde_json::Value = serde_json::from_str(
        &fs::read_to_string("tests/fixtures/syntax_backend/production-route-manifest-v1.json")
            .expect("checked production route manifest should exist"),
    )
    .expect("checked production route manifest should parse");
    manifest["constructs"]
        .as_array()
        .expect("production route manifest should list constructs")
        .len() as u64
}

fn bin_path(name: &str) -> PathBuf {
    match name {
        "tct-analyze-usage" => PathBuf::from(env!("CARGO_BIN_EXE_tct-analyze-usage")),
        "tct-syntax-source-corpus-evidence" => {
            PathBuf::from(env!("CARGO_BIN_EXE_tct-syntax-source-corpus-evidence"))
        }
        "tct-syntax-source-program-handoff" => {
            PathBuf::from(env!("CARGO_BIN_EXE_tct-syntax-source-program-handoff"))
        }
        "tct-syntax-source-program-external-validation" => PathBuf::from(env!(
            "CARGO_BIN_EXE_tct-syntax-source-program-external-validation"
        )),
        "tct-syntax-go-tokenizer-generator" => {
            PathBuf::from(env!("CARGO_BIN_EXE_tct-syntax-go-tokenizer-generator"))
        }
        "tct-measure-tokens" => PathBuf::from(env!("CARGO_BIN_EXE_tct-measure-tokens")),
        "tct-bpe-lookup-benchmark" => PathBuf::from(env!("CARGO_BIN_EXE_tct-bpe-lookup-benchmark")),
        "tct-prepare-data" => PathBuf::from(env!("CARGO_BIN_EXE_tct-prepare-data")),
        "tct-build-bundle-wrapper" => PathBuf::from(env!("CARGO_BIN_EXE_tct-build-bundle-wrapper")),
        "tct-token-diff" => PathBuf::from(env!("CARGO_BIN_EXE_tct-token-diff")),
        "tct-train-bpe" => PathBuf::from(env!("CARGO_BIN_EXE_tct-train-bpe")),
        "tct-transliterate-workflows" => {
            PathBuf::from(env!("CARGO_BIN_EXE_tct-transliterate-workflows"))
        }
        "tct-validate-direct-hf-profile" => {
            PathBuf::from(env!("CARGO_BIN_EXE_tct-validate-direct-hf-profile"))
        }
        "tct-validate-server" => PathBuf::from(env!("CARGO_BIN_EXE_tct-validate-server")),
        _ => panic!("unknown binary: {name}"),
    }
}

/// Pin Go syntax CLI tests via the unified resolver (`tct::go_toolchain`).
fn command_with_pinned_go(bin: &str) -> Command {
    let mut command = Command::new(bin_path(bin));
    if let Ok(tc) = tct::go_toolchain::resolve() {
        command.env("TCT_GO_BIN", &tc.go_bin);
        command.env("TCT_GOFMT_BIN", &tc.gofmt_bin);
        if let Some(parent) = tc.go_bin.parent() {
            let mut path = parent.as_os_str().to_os_string();
            path.push(":");
            if let Some(existing) = std::env::var_os("PATH") {
                path.push(existing);
            }
            command.env("PATH", path);
        }
    }
    command
}

fn collect_go_metadata_coverage(metadata_path: &str) -> GoGrammarCoverage {
    let metadata: serde_json::Value = serde_json::from_slice(
        &fs::read(metadata_path)
            .unwrap_or_else(|error| panic!("read Go syntax metadata {metadata_path}: {error}")),
    )
    .unwrap_or_else(|error| panic!("parse Go syntax metadata {metadata_path}: {error}"));
    assert_eq!(
        metadata["artifact_kind"], "tct_go_syntax_metadata_v0",
        "gap runner should read checked Go syntax metadata"
    );

    let mut coverage = GoGrammarCoverage::default();
    for rule in metadata["rules"]
        .as_array()
        .expect("Go syntax metadata rules should be an array")
    {
        let node_kind = rule["node_kind"]
            .as_str()
            .or_else(|| rule["name"].as_str())
            .expect("Go syntax metadata rule should carry a node kind");
        coverage.node_kinds.insert(node_kind.to_string());
        *coverage
            .category_counts
            .entry(go_grammar_category_for_node_kind(node_kind).to_string())
            .or_default() += 1;

        for field in rule["fields"]
            .as_array()
            .expect("Go syntax metadata fields should be an array")
        {
            let field_name = field["name"]
                .as_str()
                .expect("Go syntax metadata field should carry a name");
            coverage
                .grammar_fields
                .insert(format!("{node_kind}.{field_name}"));
            *coverage
                .category_counts
                .entry(go_grammar_category_for_field(node_kind, field_name).to_string())
                .or_default() += 1;

            for alternative in field["alternatives"]
                .as_array()
                .expect("Go syntax metadata alternatives should be an array")
            {
                let terminal = alternative["terminal"]
                    .as_str()
                    .expect("Go syntax metadata alternative should carry a terminal");
                if terminal.ends_with("_payload") {
                    coverage.payload_kinds.insert(terminal.to_string());
                }
            }
        }
    }
    coverage
}

fn go_grammar_category_for_field(node_kind: &str, field_name: &str) -> &'static str {
    if field_name == "TypeParams" || node_kind == "IndexListExpr" {
        return "types_and_generics";
    }
    if node_kind == "Comment"
        || node_kind == "CommentGroup"
        || field_name == "Comments"
        || field_name == "HeaderComments"
        || field_name == "OutputComments"
        || field_name == "Doc"
        || field_name == "Comment"
        || field_name == "Text"
    {
        return "comments_directives_build_tags";
    }
    go_grammar_category_for_node_kind(node_kind)
}

fn go_grammar_category_for_node_kind(node_kind: &str) -> &'static str {
    match node_kind {
        "File" | "GenDecl" | "ImportSpec" | "TypeSpec" | "ValueSpec" => {
            "packages_imports_declarations"
        }
        "FuncDecl" | "FuncType" | "FieldList" | "Field" => "functions_methods",
        "BlockStmt" | "DeclStmt" | "ReturnStmt" | "AssignStmt" | "IfStmt" | "ForStmt"
        | "RangeStmt" | "SwitchStmt" | "TypeSwitchStmt" | "CaseClause" | "CommClause"
        | "SelectStmt" | "BranchStmt" | "GoStmt" | "DeferStmt" | "SendStmt" | "LabeledStmt"
        | "ExprStmt" | "IncDecStmt" => "statements_control_flow",
        "CallExpr" | "SelectorExpr" | "Ident" | "ParenExpr" | "UnaryExpr" | "BinaryExpr"
        | "IndexExpr" | "SliceExpr" | "StarExpr" | "TypeAssertExpr" => "expressions_and_precedence",
        "IndexListExpr" | "ArrayType" | "StructType" | "InterfaceType" | "MapType" | "ChanType"
        | "Ellipsis" => "types_and_generics",
        "BasicLit" | "FuncLit" | "CompositeLit" | "KeyValueExpr" => "literals_and_composites",
        "Comment" | "CommentGroup" => "comments_directives_build_tags",
        _ => "uncategorized",
    }
}

fn go_coverage_category_report(coverage: &GoGrammarCoverage) -> Vec<serde_json::Value> {
    GO_GRAMMAR_CATEGORY_ORDER
        .iter()
        .filter_map(|category| {
            coverage.category_counts.get(*category).map(|count| {
                json!({
                    "grammar_category": category,
                    "observed_item_count": count,
                })
            })
        })
        .collect()
}

fn go_sorted_difference(expected: &BTreeSet<String>, observed: &BTreeSet<String>) -> Vec<String> {
    expected.difference(observed).cloned().collect()
}

/// Resolve the Go toolchain through the unified pin resolver (`tct::go_toolchain`:
/// `TCT_GO_BIN`, then `PATH` when it reports the pin, then `$HOME/sdk/<pin>/bin/go`).
/// Falls back to a bare `go` so a missing pin surfaces as the usual version error.
fn go_bin() -> std::ffi::OsString {
    tct::go_toolchain::go_bin_os_string().unwrap_or_else(|_| {
        std::env::var_os("TCT_GO_BIN").unwrap_or_else(|| std::ffi::OsString::from("go"))
    })
}

fn run_go_real_source_corpus_check_report(output_path: &str) -> serde_json::Value {
    let output_path = Path::new(output_path);
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent).expect("real-source corpus check report dir should be created");
    }
    let output = Command::new(go_bin())
        .args([
            "run",
            "scripts/analysis/go_syntax_corpus_check.go",
            "--manifest",
            GO_REAL_SOURCE_CORPUS_MANIFEST_PATH,
            "--json-output",
        ])
        .arg(output_path)
        .output()
        .expect("real-source Go corpus check should run");
    assert!(
        output.status.success(),
        "real-source Go corpus check failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(
        &fs::read(output_path).expect("real-source Go corpus check report should be written"),
    )
    .expect("real-source Go corpus check report should parse");
    assert_eq!(report["artifact_kind"], "tct_go_syntax_corpus_check_v1");
    assert_eq!(report["status"], "ready");
    assert_eq!(report["entry_count"], 7);
    assert!(
        report["unsupported_constructs"]
            .as_array()
            .expect("real-source unsupported constructs should be an array")
            .is_empty(),
        "real-source checker should expose no unsupported constructs before source-driven comparison"
    );
    assert!(
        report["unsupported_grammar_gaps"]
            .as_array()
            .expect("real-source unsupported grammar gaps should be an array")
            .is_empty(),
        "real-source checker should expose no unsupported grammar gaps before source-driven comparison"
    );
    assert!(report["next_missing_construct"].is_null());
    assert!(report["next_missing_grammar_gap"].is_null());
    assert!(
        report["stable_promotion_blockers"]
            .as_array()
            .expect("real-source stable blockers should be an array")
            .iter()
            .any(|blocker| blocker == GO_REAL_SOURCE_GENERATED_CODEC_ROUND_TRIP_BLOCKER),
        "real-source checker should keep the generated-codec round-trip blocker explicit"
    );
    report
}

fn go_real_source_corpus_check_summary(
    report: &serde_json::Value,
    source_driven_records: &[serde_json::Value],
    registry_coverage: &GoGrammarCoverage,
) -> serde_json::Value {
    let mut source_driven_paths = BTreeMap::new();
    for record in source_driven_records {
        let source_path = record["source_path"]
            .as_str()
            .expect("source-driven record should carry a source path");
        let manifest_path = record["manifest_path"]
            .as_str()
            .expect("source-driven record should carry a manifest path");
        source_driven_paths.insert(source_path.to_string(), manifest_path.to_string());
    }

    let mut overlap_records = Vec::new();
    let mut checker_ready_without_source_driven_records = Vec::new();
    for file in report["files"]
        .as_array()
        .expect("real-source corpus report should carry files")
    {
        let path = file["path"]
            .as_str()
            .expect("real-source corpus file should carry a path");
        if let Some(manifest_path) = source_driven_paths.get(path) {
            overlap_records.push(json!({
                "path": path,
                "manifest_path": manifest_path,
                "sha256": file["sha256"],
            }));
        } else {
            checker_ready_without_source_driven_records.push(json!({
                "path": path,
                "sha256": file["sha256"],
                "bytes": file["bytes"],
                "package": file["package"],
                "constructs": file["constructs"],
            }));
        }
    }

    let grammar_coverage = &report["grammar_coverage"];
    let metadata_node_kind_count = grammar_coverage["node_kind_count"]
        .as_i64()
        .expect("real-source grammar coverage should carry node kind count");
    let metadata_grammar_field_count = grammar_coverage["grammar_field_count"]
        .as_i64()
        .expect("real-source grammar coverage should carry grammar field count");
    let metadata_payload_kind_count = grammar_coverage["payload_kind_count"]
        .as_i64()
        .expect("real-source grammar coverage should carry payload kind count");

    json!({
        "report_path": GO_REAL_SOURCE_CORPUS_CHECK_REPORT_PATH,
        "report_command": format!(
            "go run scripts/analysis/go_syntax_corpus_check.go --manifest {GO_REAL_SOURCE_CORPUS_MANIFEST_PATH} --json-output {GO_REAL_SOURCE_CORPUS_CHECK_REPORT_PATH}"
        ),
        "manifest_path": GO_REAL_SOURCE_CORPUS_MANIFEST_PATH,
        "artifact_kind": report["artifact_kind"],
        "status": report["status"],
        "corpus_id": report["corpus_id"],
        "source_root": report["source_root"],
        "source_manifest": report["source_manifest"],
        "entry_count": report["entry_count"],
        "observed_construct_count": report["observed_constructs"]
            .as_array()
            .expect("real-source observed constructs should be an array")
            .len(),
        "unsupported_construct_count": report["unsupported_constructs"]
            .as_array()
            .expect("real-source unsupported constructs should be an array")
            .len(),
        "unsupported_grammar_gap_count": report["unsupported_grammar_gaps"]
            .as_array()
            .expect("real-source unsupported grammar gaps should be an array")
            .len(),
        "next_missing_construct": report["next_missing_construct"],
        "next_missing_grammar_gap": report["next_missing_grammar_gap"],
        "grammar_coverage": grammar_coverage,
        "source_driven_registry_coverage": {
            "node_kind_count": registry_coverage.node_kinds.len(),
            "grammar_field_count": registry_coverage.grammar_fields.len(),
            "payload_kind_count": registry_coverage.payload_kinds.len(),
        },
        "metadata_root_to_registry_delta": {
            "node_kind_count": metadata_node_kind_count - registry_coverage.node_kinds.len() as i64,
            "grammar_field_count": metadata_grammar_field_count - registry_coverage.grammar_fields.len() as i64,
            "payload_kind_count": metadata_payload_kind_count - registry_coverage.payload_kinds.len() as i64,
        },
        "source_driven_overlap_count": overlap_records.len(),
        "source_driven_overlap_records": overlap_records,
        "checker_ready_without_source_driven_record_count": checker_ready_without_source_driven_records.len(),
        "checker_ready_without_source_driven_records": checker_ready_without_source_driven_records,
        "stable_promotion_blockers": report["stable_promotion_blockers"],
        "depth_interpretation": "the maintained real-source Go corpus check is parser-ready with no unsupported constructs or grammar gaps; the source-driven tokenizer-generator corpus covers every registry-owned Go grammar category, so the next depth blocker is generated-codec round-trip evidence rather than another fixture-shaped breadth record",
        "next_concrete_task": GO_TOKENIZER_GENERATOR_NEXT_READY_TASK,
    })
}

fn go_source_driven_under_proven_categories(
    registry_coverage: &GoGrammarCoverage,
    observed_coverage: &GoGrammarCoverage,
) -> Vec<serde_json::Value> {
    GO_UNDER_PROVEN_CATEGORY_SPECS
        .iter()
        .filter_map(|spec| {
            let missing_node_kinds = spec
                .node_kinds
                .iter()
                .filter(|node_kind| {
                    let node_kind = (*node_kind).to_string();
                    registry_coverage.node_kinds.contains(&node_kind)
                        && !observed_coverage.node_kinds.contains(&node_kind)
                })
                .copied()
                .collect::<Vec<_>>();
            let missing_grammar_fields = spec
                .grammar_fields
                .iter()
                .filter(|field| {
                    let field = (*field).to_string();
                    registry_coverage.grammar_fields.contains(&field)
                        && !observed_coverage.grammar_fields.contains(&field)
                })
                .copied()
                .collect::<Vec<_>>();
            let missing_payload_kinds = spec
                .payload_kinds
                .iter()
                .filter(|payload_kind| {
                    let payload_kind = (*payload_kind).to_string();
                    registry_coverage.payload_kinds.contains(&payload_kind)
                        && !observed_coverage.payload_kinds.contains(&payload_kind)
                })
                .copied()
                .collect::<Vec<_>>();
            if missing_node_kinds.is_empty()
                && missing_grammar_fields.is_empty()
                && missing_payload_kinds.is_empty()
            {
                return None;
            }
            Some(json!({
                "category_id": spec.category_id,
                "grammar_category": spec.grammar_category,
                "description": spec.description,
                "missing_node_kinds": missing_node_kinds,
                "missing_grammar_fields": missing_grammar_fields,
                "missing_payload_kinds": missing_payload_kinds,
                "smallest_corpus_broadening": spec.smallest_corpus_broadening,
            }))
        })
        .collect()
}

fn write_go_source_driven_grammar_gap_report(
    manifest: &serde_json::Value,
    source_driven_records: Vec<serde_json::Value>,
    observed_coverage: &GoGrammarCoverage,
    real_source_corpus_report: &serde_json::Value,
    output_path: &str,
) -> serde_json::Value {
    let mut registry_coverage = GoGrammarCoverage::default();
    let mut registry_go_grammar_ids = Vec::new();
    for entry in SYNTAX_CODEC_REGISTRY
        .iter()
        .filter(|entry| entry.target_id == "go")
    {
        registry_go_grammar_ids.push(entry.grammar_id);
        registry_coverage.merge(&collect_go_metadata_coverage(entry.metadata_path));
    }
    registry_go_grammar_ids.sort_unstable();

    let missing_node_kinds =
        go_sorted_difference(&registry_coverage.node_kinds, &observed_coverage.node_kinds);
    let missing_grammar_fields = go_sorted_difference(
        &registry_coverage.grammar_fields,
        &observed_coverage.grammar_fields,
    );
    let missing_payload_kinds = go_sorted_difference(
        &registry_coverage.payload_kinds,
        &observed_coverage.payload_kinds,
    );
    let under_proven_categories =
        go_source_driven_under_proven_categories(&registry_coverage, observed_coverage);
    let next_under_proven = under_proven_categories
        .first()
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let real_source_corpus_check = go_real_source_corpus_check_summary(
        real_source_corpus_report,
        &source_driven_records,
        &registry_coverage,
    );

    let report = json!({
        "artifact_kind": GO_SOURCE_DRIVEN_GRAMMAR_GAP_REPORT_ARTIFACT_KIND,
        "version": 1,
        "tool": "maintained_cli_binaries_test::generated_go_source_driven_tokenizer_generator_corpus_smoke_validates",
        "status": GO_SOURCE_DRIVEN_GRAMMAR_GAP_REPORT_STATUS,
        "source_manifest": GO_SOURCE_DRIVEN_TOKENIZER_GENERATOR_CORPUS_MANIFEST_PATH,
        "source_manifest_artifact_kind": manifest["artifact_kind"],
        "source_of_truth": "src/syntax_production_route.rs::SYNTAX_CODEC_REGISTRY",
        "route_manifest": manifest["route_manifest"],
        "record_count": source_driven_records.len(),
        "registry_go_grammar_id_count": registry_go_grammar_ids.len(),
        "registry_go_grammar_ids": registry_go_grammar_ids,
        "records": source_driven_records,
        "observed_coverage": {
            "node_kind_count": observed_coverage.node_kinds.len(),
            "grammar_field_count": observed_coverage.grammar_fields.len(),
            "payload_kind_count": observed_coverage.payload_kinds.len(),
            "grammar_categories": go_coverage_category_report(observed_coverage),
            "node_kinds": observed_coverage.node_kinds.clone(),
            "grammar_fields": observed_coverage.grammar_fields.clone(),
            "payload_kinds": observed_coverage.payload_kinds.clone(),
        },
        "registry_coverage": {
            "node_kind_count": registry_coverage.node_kinds.len(),
            "grammar_field_count": registry_coverage.grammar_fields.len(),
            "payload_kind_count": registry_coverage.payload_kinds.len(),
        },
        "unsupported_grammar_gaps": [],
        "under_proven_registry_gaps": {
            "missing_node_kinds": missing_node_kinds,
            "missing_grammar_fields": missing_grammar_fields,
            "missing_payload_kinds": missing_payload_kinds,
            "categories": under_proven_categories,
        },
        "real_source_corpus_check": real_source_corpus_check,
        "next_under_proven_grammar_category": next_under_proven,
        "next_ready_task": GO_TOKENIZER_GENERATOR_NEXT_READY_TASK,
        "validation_commands": [
            "go run scripts/analysis/go_syntax_corpus_check.go --self-test",
            "go run scripts/analysis/go_syntax_corpus_check.go --manifest tests/fixtures/go_syntax_corpus/manifest-v1.json --json-output target/tct-analysis/go-syntax-corpus-check/report.json",
            "PATH=\"$PWD/target/tct-go-bin:$PATH\" cargo test --test maintained_cli_binaries_test generated_go_source_driven_tokenizer_generator_corpus_smoke_validates -- --exact",
            "cargo test --test syntax_generated_codec_gate_test go_local_type_switch_syntax_generated_codec_compiles_through_production_route -- --exact",
            "cargo test --test syntax_generated_codec_gate_test go_local_switch_init_syntax_generated_codec_compiles_through_production_route -- --exact",
            "cargo test --test syntax_generated_codec_gate_test go_concurrency_map_interface_type_assertion_syntax_generated_codec_compiles_through_production_route -- --exact",
            "cargo test --test syntax_generated_codec_gate_test go_local_generic_function_syntax_generated_codec_compiles_through_production_route -- --exact",
            "cargo test --test syntax_generated_codec_gate_test go_local_variadic_ellipsis_syntax_generated_codec_compiles_through_production_route -- --exact",
            "cargo test --test syntax_generated_codec_gate_test go_slice_expression_three_index_syntax_generated_codec_compiles_through_production_route -- --exact",
            "cargo test --test syntax_generated_codec_gate_test go_import_spec_comment_syntax_generated_codec_compiles_through_production_route -- --exact",
            "cargo test --test syntax_generated_codec_gate_test go_import_spec_name_syntax_generated_codec_compiles_through_production_route -- --exact",
            "cargo test --test syntax_generated_codec_gate_test go_type_spec_assign_syntax_generated_codec_compiles_through_production_route -- --exact",
            "cargo test --test syntax_generated_codec_gate_test go_type_spec_comment_syntax_generated_codec_compiles_through_production_route -- --exact",
            "cargo test --test syntax_generated_codec_gate_test go_type_spec_doc_syntax_generated_codec_compiles_through_production_route -- --exact",
            "cargo test --test syntax_generated_codec_gate_test go_value_spec_comment_syntax_generated_codec_compiles_through_production_route -- --exact",
            "cargo test --test syntax_generated_codec_gate_test parser_backed_syntax_codec_registry_builds_every_registered_entry -- --exact",
            "python3 scripts/testing/test-quality-audit.py --changed-from origin/main",
            "git diff --check"
        ],
        "claim_boundary": GO_SOURCE_DRIVEN_GRAMMAR_GAP_CLAIM_BOUNDARY,
        "no_stable_claims": GO_SOURCE_DRIVEN_NO_STABLE_CLAIMS,
    });

    let output_path = Path::new(output_path);
    fs::create_dir_all(
        output_path
            .parent()
            .expect("grammar gap report should have a parent directory"),
    )
    .expect("grammar gap report directory should be created");
    let mut rendered =
        serde_json::to_vec_pretty(&report).expect("grammar gap report should serialize");
    rendered.push(b'\n');
    fs::write(output_path, rendered).expect("grammar gap report should be written");
    report
}

fn assert_sha256_field(value: &serde_json::Value) {
    let digest = value.as_str().expect("sha256 field should be a string");
    assert_eq!(digest.len(), 64);
    assert!(digest.chars().all(|c| c.is_ascii_hexdigit()));
}

fn assert_go_renderer_position_policy_summary(summary: &serde_json::Value) {
    assert_eq!(
        summary["inventory_artifact_path"],
        GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH
    );
    assert_eq!(summary["total_field_count"], json!(64));

    let category_counts = summary["category_counts"]
        .as_array()
        .expect("renderer-position category counts should be an array")
        .iter()
        .map(|category| {
            (
                category["category"]
                    .as_str()
                    .expect("renderer-position category should be a string"),
                category["field_count"]
                    .as_u64()
                    .expect("renderer-position field count should be an integer"),
            )
        })
        .collect::<BTreeMap<_, _>>();
    assert_eq!(category_counts.len(), 6);
    for category in [
        "bad_node_source_range_unsupported",
        "comment_trivia_position_sidecar",
        "parser_source_position_sidecar",
        "fixed_syntax_token_renderer_metadata",
        "finite_domain_token_renderer_metadata",
        "syntax_presence_renderer_metadata",
    ] {
        assert!(
            category_counts.get(category).copied().unwrap_or(0) > 0,
            "missing renderer-position policy category {category}"
        );
    }

    let representative_policies = summary["representative_field_policies"]
        .as_array()
        .expect("representative renderer-position policies should be an array")
        .iter()
        .map(|field| {
            let node_name = field["node_name"]
                .as_str()
                .expect("representative renderer-position node should be a string");
            let field_name = field["field_name"]
                .as_str()
                .expect("representative renderer-position field should be a string");
            let policy = field["policy"]
                .as_str()
                .expect("representative renderer-position policy should be a string");
            (format!("{node_name}.{field_name}"), policy)
        })
        .collect::<BTreeMap<_, _>>();
    for (path, expected_policy) in [
        ("ArrayType.Lbrack", "fixed_syntax_token_renderer_metadata"),
        ("BranchStmt.TokPos", "finite_domain_token_renderer_metadata"),
        ("CaseClause.Case", "syntax_presence_renderer_metadata"),
        ("FieldList.Opening", "syntax_presence_renderer_metadata"),
        ("Ident.NamePos", "parser_source_position_sidecar"),
        ("ImportSpec.EndPos", "parser_source_position_sidecar"),
        ("ParenExpr.Lparen", "fixed_syntax_token_renderer_metadata"),
        ("UnaryExpr.OpPos", "finite_domain_token_renderer_metadata"),
    ] {
        assert_eq!(
            representative_policies.get(path).copied(),
            Some(expected_policy),
            "unexpected renderer-position policy for {path}"
        );
    }
}

#[test]
fn maintained_cli_binaries_print_help() {
    let binaries = [
        ("tct-analyze-usage", "Analyze vocabulary usage"),
        (
            "tct-syntax-source-corpus-evidence",
            "Write syntax source-corpus evidence reports",
        ),
        (
            "tct-syntax-source-program-handoff",
            "Write syntax source-program handoff reports",
        ),
        (
            "tct-syntax-source-program-external-validation",
            "Write syntax source-program external-validation execution reports",
        ),
        (
            "tct-syntax-go-tokenizer-generator",
            "Generate Go raw syntax-token tokenizer reports",
        ),
        ("tct-measure-tokens", "Measure actual token counts"),
        (
            "tct-bpe-lookup-benchmark",
            "Compare BPE pair lookup strategies",
        ),
        ("tct-prepare-data", "Prepare encoded token sequences"),
        ("tct-build-bundle-wrapper", "Assemble a pure-Python package"),
        ("tct-token-diff", "Produce token-ID layout diff manifests"),
        ("tct-train-bpe", "Train BPE merge tables"),
        (
            "tct-transliterate-workflows",
            "Transliterate GitHub workflows",
        ),
        (
            "tct-validate-direct-hf-profile",
            "Validate serialized direct HF/TCT compatibility profiles",
        ),
        ("tct-validate-server", "Fast batch workflow validator"),
    ];

    for (name, expected_text) in binaries {
        let output = Command::new(bin_path(name))
            .arg("--help")
            .output()
            .unwrap_or_else(|e| panic!("failed to run {name} --help: {e}"));

        assert!(
            output.status.success(),
            "{name} --help failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains(expected_text),
            "{name} help output did not contain {expected_text:?}:\n{stdout}"
        );
        if matches!(
            name,
            "tct-measure-tokens" | "tct-prepare-data" | "tct-token-diff" | "tct-train-bpe"
        ) {
            assert!(
                stdout.contains("OVocab"),
                "{name} help output should document named optimization profiles:\n{stdout}"
            );
        }
        if name == "tct-train-bpe" {
            assert!(
                stdout.contains("512,1024,2048,4096"),
                "{name} help output should use the maintained BPE checkpoint grid:\n{stdout}"
            );
            assert!(
                !stdout.contains("8192"),
                "{name} help output should not advertise target vocabularies above the maintained policy ceiling:\n{stdout}"
            );
        }
        if name == "tct-measure-tokens" {
            assert!(
                stdout.contains("--cleanup-generated-helper"),
                "{name} help output should document successful helper cleanup:\n{stdout}"
            );
        }
        if name == "tct-token-diff" {
            assert!(
                stdout.contains("--string-encoding"),
                "{name} help output should document string encoding config:\n{stdout}"
            );
            assert!(
                stdout.contains("--max-base-vocab"),
                "{name} help output should document base-vocabulary config:\n{stdout}"
            );
            assert!(
                stdout.contains("--new-bit-packing"),
                "{name} help output should document side-specific pass overrides:\n{stdout}"
            );
        }
        if matches!(
            name,
            "tct-syntax-source-corpus-evidence"
                | "tct-syntax-source-program-handoff"
                | "tct-syntax-source-program-external-validation"
                | "tct-syntax-go-tokenizer-generator"
        ) {
            assert!(
                stdout.contains("--validate-report"),
                "{name} help output should document report validation:\n{stdout}"
            );
        }
    }
}

#[test]
#[ignore = "known failure: current-plus report-ingestion manifest pins grammar_artifact_content_sha256 3f091674..., the live unified Go grammar artifact is 9a8f8f6c...; see docs/frontends-development.md"]
fn go_tokenizer_generator_cli_writes_and_validates_report() {
    let dir = TempDir::new().unwrap();
    let projection_path = dir
        .path()
        .join("os-dir-unix-source-instance-projection-v1.json");
    let report_path = dir.path().join("go-tokenizer-generator-report.json");

    let output = command_with_pinned_go("tct-syntax-go-tokenizer-generator")
        .arg("--projection-output")
        .arg(&projection_path)
        .arg("--output")
        .arg(&report_path)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "Go tokenizer generator CLI failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("wrote Go tokenizer-generator report"),
        "producer should report success:\n{}",
        String::from_utf8_lossy(&output.stdout)
    );

    let report: serde_json::Value = serde_json::from_slice(&fs::read(&report_path).unwrap())
        .expect("Go tokenizer generator report should parse");
    assert_eq!(
        report["artifact_kind"],
        "tct_syntax_go_tokenizer_generator_report_v1"
    );
    assert_eq!(
        report["status"],
        "go_raw_syntax_tokenizer_generator_fixture_ready"
    );
    assert_eq!(
        report["route"],
        "go_parser_metadata_raw_syntax_state_machine"
    );
    assert_eq!(
        report["source_path"],
        "tests/fixtures/go_syntax_corpus/src/os/dir_unix.go"
    );
    assert_eq!(
        report["candidate_grammar_id"],
        "go_ast_schema_inventory_go1_24_3"
    );
    assert_eq!(report["projection_report_mode"], "checked_fixture");
    assert_eq!(report["raw_token_count"], json!(10553));
    assert_eq!(report["canonical_token_count"], json!(6903));
    assert_eq!(report["forced_token_count"], json!(3650));
    assert_eq!(report["stream_contract"], "raw-syntax-stream-elided-v1");
    assert_eq!(report["raw_state_machine_complete"], true);
    assert_eq!(
        report["raw_state_machine_decode"]["observed_status"],
        "complete"
    );
    assert_eq!(report["raw_state_machine_decode"]["eof_valid"], true);
    assert_eq!(
        report["token_only_decode_render"]["status"],
        "go_tokenizer_generator_token_only_decode_render_ready"
    );
    assert_eq!(
        report["token_only_decode_render"]["raw_token_stream_source"],
        "raw_token_ids_only"
    );
    assert_eq!(
        report["token_only_decode_render"]["decoded_tree"]["root_rule"],
        "File"
    );
    assert_eq!(
        report["token_only_decode_render"]["canonical_render_ready"],
        true
    );
    assert_eq!(
        report["token_only_decode_render"]["render_reparse_sidecar_validation"],
        true
    );
    assert_eq!(report["gofmt_validation"], "passed");
    assert_eq!(report["go_parser_reparse_validation"], "passed");
    assert_eq!(report["pipeline"]["arena_ir_used"], false);
    assert_eq!(report["pipeline"]["route_handoff_provenance_used"], false);
    assert_eq!(report["pipeline"]["bpe_validity_authority_used"], false);
    assert_go_renderer_position_policy_summary(&report["renderer_position_policy_summary"]);

    let validation = command_with_pinned_go("tct-syntax-go-tokenizer-generator")
        .arg("--validate-report")
        .arg(&report_path)
        .output()
        .unwrap();
    assert!(
        validation.status.success(),
        "Go tokenizer generator validation failed: {}",
        String::from_utf8_lossy(&validation.stderr)
    );
    assert!(
        String::from_utf8_lossy(&validation.stdout)
            .contains("validated Go tokenizer-generator report"),
        "validator should report success:\n{}",
        String::from_utf8_lossy(&validation.stdout)
    );

    let mut missing_summary_report = report.clone();
    missing_summary_report
        .as_object_mut()
        .expect("Go tokenizer-generator report should be an object")
        .remove("renderer_position_policy_summary");
    let missing_summary_path = dir
        .path()
        .join("go-tokenizer-generator-report-missing-renderer-summary.json");
    fs::write(
        &missing_summary_path,
        serde_json::to_vec_pretty(&missing_summary_report).unwrap(),
    )
    .unwrap();
    let missing_summary_validation = command_with_pinned_go("tct-syntax-go-tokenizer-generator")
        .arg("--validate-report")
        .arg(&missing_summary_path)
        .output()
        .unwrap();
    assert!(
        !missing_summary_validation.status.success(),
        "validator should reject reports missing renderer-position policy summary"
    );
    assert!(
        String::from_utf8_lossy(&missing_summary_validation.stderr)
            .contains("renderer_position_policy_summary"),
        "missing-summary error should name renderer-position policy summary:\n{}",
        String::from_utf8_lossy(&missing_summary_validation.stderr)
    );

    let mut stale_summary_report = report.clone();
    stale_summary_report["renderer_position_policy_summary"]["total_field_count"] = json!(63);
    let stale_summary_path = dir
        .path()
        .join("go-tokenizer-generator-report-stale-renderer-summary.json");
    fs::write(
        &stale_summary_path,
        serde_json::to_vec_pretty(&stale_summary_report).unwrap(),
    )
    .unwrap();
    let stale_summary_validation = command_with_pinned_go("tct-syntax-go-tokenizer-generator")
        .arg("--validate-report")
        .arg(&stale_summary_path)
        .output()
        .unwrap();
    assert!(
        !stale_summary_validation.status.success(),
        "validator should reject stale renderer-position policy summary"
    );
    assert!(
        String::from_utf8_lossy(&stale_summary_validation.stderr)
            .contains("renderer_position_policy_summary.total_field_count"),
        "stale-summary error should name renderer-position policy summary drift:\n{}",
        String::from_utf8_lossy(&stale_summary_validation.stderr)
    );
}

#[test]
#[ignore = "known failure: current-plus report-ingestion manifest pins grammar_artifact_content_sha256 3f091674..., the live unified Go grammar artifact is 9a8f8f6c...; see docs/frontends-development.md"]
fn go_tokenizer_generator_cli_writes_and_validates_manifest_depth_report() {
    let dir = TempDir::new().unwrap();
    let report_path = dir.path().join("go-tokenizer-generator-depth-report.json");
    let report_ingestion_manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(GO_CURRENT_PLUS_REPORT_INGESTION_MANIFEST_PATH).unwrap())
            .expect("current-plus report-ingestion manifest should parse");
    let oracle_report_contents =
        fs::read(DEFAULT_GO_RAW_TREE_AST_PRINTER_ORACLE_REPORT_PATH).unwrap();
    let manifest_records = report_ingestion_manifest["records"]
        .as_array()
        .expect("current-plus report-ingestion manifest records should be an array");
    let expected_record_count = manifest_records.len();
    let expected_raw_tree_render_attempts = manifest_records
        .iter()
        .filter(|record| record["raw_tree_render_attempt"].as_bool().unwrap_or(false))
        .map(|record| {
            record["manifest_path"]
                .as_str()
                .expect("raw-tree render attempt should carry manifest_path")
                .to_string()
        })
        .collect::<Vec<_>>();
    let expected_next_raw_tree_render_candidate = manifest_records
        .iter()
        .find(|record| !record["raw_tree_render_attempt"].as_bool().unwrap_or(false))
        .and_then(|record| record["manifest_path"].as_str())
        .map(str::to_string);

    let output = command_with_pinned_go("tct-syntax-go-tokenizer-generator")
        .arg("--report-ingestion-manifest")
        .arg(GO_CURRENT_PLUS_REPORT_INGESTION_MANIFEST_PATH)
        .arg("--output")
        .arg(&report_path)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "Go tokenizer generator manifest-depth CLI failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stdout)
            .contains("wrote Go tokenizer-generator manifest-depth report"),
        "producer should report success:\n{}",
        String::from_utf8_lossy(&output.stdout)
    );

    let report: serde_json::Value = serde_json::from_slice(&fs::read(&report_path).unwrap())
        .expect("Go tokenizer generator manifest-depth report should parse");
    let report_records = report["records"]
        .as_array()
        .expect("manifest-depth records should be an array");
    let expected_ready_raw_tree_render_attempts = report_records
        .iter()
        .filter(|record| {
            record["raw_tree_render_attempt_selected_by_manifest"] == true
                && record["raw_decoded_tree_render_blocker"]["status"]
                    == RAW_DECODED_TREE_RENDER_READY_STATUS
        })
        .count();
    let expected_grammar_mode_ready_raw_tree_render_attempts = report_records
        .iter()
        .filter(|record| {
            record["raw_tree_render_attempt_selected_by_manifest"] == true
                && record["raw_decoded_tree_render_blocker"]["raw_tree_grammar_mode_render_ready"]
                    == true
        })
        .count();
    let expected_canonical_fidelity_ready_raw_tree_render_attempts = report_records
        .iter()
        .filter(|record| {
            record["raw_tree_render_attempt_selected_by_manifest"] == true
                && record["raw_decoded_tree_render_blocker"]["raw_tree_canonical_fidelity_ready"]
                    == true
        })
        .count();
    let grammar_mode_only_record = report_records.iter().find(|record| {
        record["raw_tree_render_attempt_selected_by_manifest"] == true
            && record["raw_decoded_tree_render_blocker"]["raw_tree_grammar_mode_render_ready"]
                == true
            && record["raw_decoded_tree_render_blocker"]["raw_tree_canonical_fidelity_ready"]
                == false
    });
    let expected_raw_tree_render_diagnostics = report_records
        .iter()
        .filter(|record| {
            record["raw_tree_render_attempt_selected_by_manifest"] == true
                && !record["raw_decoded_tree_render_blocker"]["renderer_diagnostic"].is_null()
        })
        .count();
    let first_raw_tree_render_diagnostic = report_records.iter().find(|record| {
        record["raw_tree_render_attempt_selected_by_manifest"] == true
            && !record["raw_decoded_tree_render_blocker"]["renderer_diagnostic"].is_null()
    });
    assert_eq!(
        report["artifact_kind"],
        GO_TOKENIZER_GENERATOR_MANIFEST_DEPTH_ARTIFACT_KIND
    );
    assert_eq!(
        report["report_ingestion_manifest_path"],
        GO_CURRENT_PLUS_REPORT_INGESTION_MANIFEST_PATH
    );
    assert_eq!(report["record_count"], json!(expected_record_count));
    assert_eq!(report["supported_record_count"], report["record_count"]);
    assert_eq!(report["raw_state_machine_complete"], true);
    assert_eq!(report["raw_tokens_below_vocab_size"], true);
    assert_eq!(report["pad_token_absent"], true);
    assert_eq!(report["canonical_render_ready"], true);
    assert_eq!(report["token_only_decode_render_ready"], true);
    assert_eq!(
        report["raw_tree_render_attempt_summary"]["selection_source"],
        "report_ingestion_manifest.raw_tree_render_attempt"
    );
    assert_eq!(
        report["raw_tree_render_attempt_summary"]["selecting_report_ingestion_manifest_path"],
        GO_CURRENT_PLUS_REPORT_INGESTION_MANIFEST_PATH
    );
    assert_eq!(
        report["raw_tree_render_attempt_summary"]["selecting_report_ingestion_manifest_sha256"],
        report["report_ingestion_manifest_sha256"]
    );
    assert_eq!(
        report["raw_tree_render_attempt_summary"]["manifest_selected_attempt_count"],
        json!(expected_raw_tree_render_attempts.len())
    );
    assert_eq!(
        report["raw_tree_render_attempt_summary"]["ready_count"],
        json!(expected_ready_raw_tree_render_attempts)
    );
    assert_eq!(
        report["raw_tree_render_attempt_summary"]["grammar_mode_ready_count"],
        json!(expected_grammar_mode_ready_raw_tree_render_attempts)
    );
    assert_eq!(
        report["raw_tree_render_attempt_summary"]["canonical_fidelity_ready_count"],
        json!(expected_canonical_fidelity_ready_raw_tree_render_attempts)
    );
    assert!(
        grammar_mode_only_record.is_some(),
        "manifest-depth JSON should show grammar-mode readiness can advance before canonical comment/layout fidelity"
    );
    assert_eq!(
        report["raw_tree_render_attempt_summary"]["diagnostic_count"],
        json!(expected_raw_tree_render_diagnostics)
    );
    assert_eq!(
        report["raw_tree_render_attempt_summary"]["attempted_manifest_paths"],
        json!(expected_raw_tree_render_attempts)
    );
    assert_eq!(
        report["go_raw_tree_ast_printer_oracle_cutover"]["report_path"],
        DEFAULT_GO_RAW_TREE_AST_PRINTER_ORACLE_REPORT_PATH
    );
    assert_eq!(
        report["go_raw_tree_ast_printer_oracle_cutover"]["report_sha256"],
        sha256_hex(&oracle_report_contents)
    );
    assert_eq!(
        report["go_raw_tree_ast_printer_oracle_cutover"]["status"],
        GO_RAW_TREE_AST_PRINTER_ORACLE_CUTOVER_STATUS_ACCEPTED
    );
    assert_eq!(
        report["go_raw_tree_ast_printer_oracle_cutover"]["accepted"],
        true
    );
    assert_eq!(
        report["go_raw_tree_ast_printer_oracle_cutover"]["record_count"],
        json!(expected_raw_tree_render_attempts.len())
    );
    assert!(
        report["go_raw_tree_ast_printer_oracle_cutover"]["canonical_fidelity_only_gap_count"]
            .as_u64()
            .expect("oracle evidence should carry canonical gap count")
            > 0,
        "manifest-depth JSON should keep canonical-fidelity gaps diagnostic-only"
    );
    assert!(
        report["go_raw_tree_ast_printer_oracle_cutover"]["claim_boundary"]
            .as_str()
            .expect("oracle evidence should carry claim boundary")
            .contains("canonical source fidelity remains diagnostic")
    );
    assert_eq!(
        report["raw_tree_render_broadening_decision"]["decision_source"],
        RAW_TREE_RENDER_BROADENING_DECISION_SOURCE
    );
    assert_eq!(
        report["raw_tree_render_broadening_decision"]["selected_attempt_count"],
        json!(expected_raw_tree_render_attempts.len())
    );
    assert_eq!(
        report["raw_tree_render_broadening_decision"]["ready_attempt_count"],
        json!(expected_ready_raw_tree_render_attempts)
    );
    assert_eq!(
        report["raw_tree_render_broadening_decision"]["grammar_mode_ready_attempt_count"],
        json!(expected_grammar_mode_ready_raw_tree_render_attempts)
    );
    assert_eq!(
        report["raw_tree_render_broadening_decision"]["canonical_fidelity_ready_attempt_count"],
        json!(expected_canonical_fidelity_ready_raw_tree_render_attempts)
    );
    assert_eq!(
        report["raw_tree_render_broadening_decision"]["diagnostic_count"],
        json!(expected_raw_tree_render_diagnostics)
    );
    assert_eq!(
        report["raw_tree_render_broadening_decision"]["attempted_manifest_paths"],
        report["raw_tree_render_attempt_summary"]["attempted_manifest_paths"]
    );
    if let Some(diagnostic_record) = first_raw_tree_render_diagnostic {
        let diagnostic =
            &diagnostic_record["raw_decoded_tree_render_blocker"]["renderer_diagnostic"];
        let first_blocker = &report["raw_tree_render_broadening_decision"]["first_blocker"];
        assert_eq!(
            report["raw_tree_render_broadening_decision"]["status"],
            RAW_TREE_RENDER_BROADENING_DIAGNOSTIC_BLOCKED_STATUS
        );
        assert_eq!(
            report["raw_tree_render_broadening_decision"]["next_action"],
            RAW_TREE_RENDER_BROADENING_DIAGNOSTIC_ACTION
        );
        assert!(report["raw_tree_render_broadening_decision"]["next_candidate"].is_null());
        assert_eq!(
            first_blocker["manifest_path"],
            diagnostic_record["manifest_path"]
        );
        assert_eq!(
            first_blocker["source_path"],
            diagnostic_record["source_path"]
        );
        assert_eq!(
            first_blocker["missing_capability"],
            diagnostic["missing_capability"]
        );
        assert_eq!(
            first_blocker["renderer_category"],
            diagnostic["unsupported_category"]
        );
        assert_eq!(
            report["raw_tree_render_broadening_decision"]["manifest_only_attempt_ready"],
            false
        );
        assert_eq!(
            report["raw_tree_render_broadening_decision"]["rust_side_setup_required"],
            true
        );
    } else if expected_grammar_mode_ready_raw_tree_render_attempts
        == expected_raw_tree_render_attempts.len()
    {
        assert_eq!(
            report["raw_tree_render_broadening_decision"]["status"],
            RAW_TREE_RENDER_BROADENING_GRAMMAR_MODE_READY_STATUS
        );
        assert_eq!(
            report["raw_tree_render_broadening_decision"]["next_action"],
            RAW_TREE_RENDER_BROADENING_GRAMMAR_MODE_ACTION
        );
        assert!(report["raw_tree_render_broadening_decision"]["next_candidate"].is_null());
        assert!(report["raw_tree_render_broadening_decision"]["first_blocker"].is_null());
        assert_eq!(
            report["raw_tree_render_broadening_decision"]["manifest_only_attempt_ready"],
            false
        );
        assert_eq!(
            report["raw_tree_render_broadening_decision"]["rust_side_setup_required"],
            false
        );
    } else {
        assert_eq!(
            report["raw_tree_render_broadening_decision"]["status"],
            RAW_TREE_RENDER_BROADENING_NEXT_CANDIDATE_STATUS
        );
        assert_eq!(
            report["raw_tree_render_broadening_decision"]["next_action"],
            RAW_TREE_RENDER_BROADENING_MANIFEST_ONLY_ACTION
        );
        assert_eq!(
            report["raw_tree_render_broadening_decision"]["next_candidate"]["manifest_path"],
            expected_next_raw_tree_render_candidate.expect(
                "current-plus manifest should have an unattempted raw-tree render candidate"
            )
        );
        assert_eq!(
            report["raw_tree_render_broadening_decision"]["manifest_only_attempt_ready"],
            true
        );
        assert_eq!(
            report["raw_tree_render_broadening_decision"]["rust_side_setup_required"],
            false
        );
        assert!(
            report["raw_tree_render_broadening_decision"]["first_blocker"].is_null(),
            "clean selected attempts should not produce a broadening blocker"
        );
    }
    let expected_broadening_next_ready_task = if report["raw_tree_render_broadening_decision"]["next_action"]
        == RAW_TREE_RENDER_BROADENING_GRAMMAR_MODE_ACTION
    {
        GO_RAW_TREE_AST_PRINTER_ORACLE_CUTOVER_NEXT_READY_TASK
    } else {
        RAW_TREE_RENDER_BROADENING_NEXT_READY_TASK
    };
    assert_eq!(
        report["raw_tree_render_broadening_decision"]["next_ready_task"],
        expected_broadening_next_ready_task
    );
    assert_eq!(
        report["next_ready_task"],
        GO_TOKENIZER_GENERATOR_NEXT_READY_TASK
    );
    assert!(
        report["validation_commands"]
            .as_array()
            .expect("manifest-depth report should carry validation commands")
            .iter()
            .any(|command| command == "cargo test --test go_ast_schema_inventory_test"),
        "manifest-depth CLI report should keep the Go AST grammar coverage gate first-class"
    );
    assert_eq!(report["pipeline"]["arena_ir_used"], false);
    assert_eq!(report["pipeline"]["route_handoff_provenance_used"], false);
    assert_eq!(report["pipeline"]["bpe_validity_authority_used"], false);
    assert_go_renderer_position_policy_summary(&report["renderer_position_policy_summary"]);
    assert_eq!(
        report["go_ast_grammar_coverage"]["status"],
        GO_AST_SCHEMA_GRAMMAR_COVERAGE_STATUS_READY
    );
    assert_eq!(
        report["go_ast_grammar_coverage"]["inventory_surface_checked"],
        true
    );
    assert_eq!(
        report["go_ast_grammar_coverage"]["tokenizable_node_count"],
        report["go_ast_grammar_coverage"]["grammar_ir_rule_count"]
    );
    assert_eq!(
        report["go_ast_grammar_coverage"]["grammar_ir_rule_count"],
        report["go_ast_grammar_coverage"]["structural_model_record_count"]
    );
    assert_eq!(
        report["go_ast_grammar_coverage"]["tokenizable_field_count"],
        report["go_ast_grammar_coverage"]["structural_model_field_count"]
    );
    assert!(
        report["go_ast_grammar_coverage"]["token_layout"]
            ["multi_alternative_nonterminal_node_kind_count"]
            .as_u64()
            .expect("coverage should carry multi-alternative node count")
            <= report["go_ast_grammar_coverage"]["token_layout"]
                ["alternative_node_start_capacity"]
                .as_u64()
                .expect("coverage should carry alternative-node capacity")
    );
    assert_eq!(
        report["go_ast_grammar_coverage"]["raw_state_machine"]["raw_state_machine_compile_ready"],
        true
    );
    assert_eq!(
        report["go_ast_grammar_coverage"]["raw_state_machine"]["initial_state_ready"],
        true
    );
    assert!(report["go_ast_grammar_coverage"]["first_blocker"].is_null());
    assert_eq!(
        report["go_ast_grammar_coverage"]["next_ready_task"],
        GO_AST_SCHEMA_GRAMMAR_COVERAGE_NEXT_READY_TASK
    );
    assert_eq!(
        report["go_ast_raw_tree_renderer_codegen_coverage"]["status"],
        GO_AST_RAW_TREE_RENDERER_CODEGEN_COVERAGE_STATUS_READY
    );
    assert_eq!(
        report["go_ast_raw_tree_renderer_codegen_coverage"]["tokenizable_node_count"],
        report["go_ast_grammar_coverage"]["tokenizable_node_count"]
    );
    assert_eq!(
        report["go_ast_raw_tree_renderer_codegen_coverage"]["missing_policy_count"],
        0
    );
    assert!(
        report["go_ast_raw_tree_renderer_codegen_coverage"]["node_coverages"]
            .as_array()
            .expect("renderer codegen coverage should carry node coverages")
            .iter()
            .any(|coverage| coverage["node_kind"] == "EmptyStmt"
                && coverage["manual_policy_hooks"]
                    .as_array()
                    .expect("node coverage should carry manual hooks")
                    .iter()
                    .any(|hook| hook == "fixed_semicolon_grammar_mode"))
    );
    assert_eq!(
        report["go_ast_raw_tree_renderer_codegen_coverage"]["next_ready_task"],
        GO_AST_RAW_TREE_RENDERER_CODEGEN_COVERAGE_NEXT_READY_TASK
    );
    assert!(
        report_records
            .iter()
            .all(|record| record["raw_state_machine_complete"] == true
                && record["token_only_decode_render"]["decoded_from_token_only_stream"] == true
                && record["token_only_decode_render"]["canonical_render_ready"] == true
                && record["gofmt_validation"] == "passed"
                && record["go_parser_reparse_validation"] == "passed"),
        "every manifest-depth record should pass raw state-machine and sidecar validation"
    );

    let validation = command_with_pinned_go("tct-syntax-go-tokenizer-generator")
        .arg("--validate-report")
        .arg(&report_path)
        .output()
        .unwrap();
    assert!(
        validation.status.success(),
        "Go tokenizer generator manifest-depth validation failed: {}",
        String::from_utf8_lossy(&validation.stderr)
    );
    assert!(
        String::from_utf8_lossy(&validation.stdout)
            .contains("validated Go tokenizer-generator manifest-depth report"),
        "validator should report success:\n{}",
        String::from_utf8_lossy(&validation.stdout)
    );
}

#[test]
fn go_tokenizer_generator_cli_writes_and_validates_stdlib_coverage_report() {
    let repo_root = std::env::current_dir().unwrap();
    let source_dir = TempDir::new().unwrap();
    let target_dir = TempDir::new_in("target").unwrap();
    let target_dir_path = target_dir
        .path()
        .strip_prefix(&repo_root)
        .unwrap_or(target_dir.path());
    let goroot = source_dir.path().join("goroot");
    let alpha_path = goroot.join("src/alpha/alpha.go");
    let broken_path = goroot.join("src/beta/broken.go");
    fs::create_dir_all(alpha_path.parent().unwrap()).unwrap();
    fs::create_dir_all(broken_path.parent().unwrap()).unwrap();
    let alpha_source = b"package alpha\n\nfunc Add(x int) int {\n\treturn x + 1\n}\n";
    let broken_source = b"package beta\n\nfunc Broken(\n";
    fs::write(&alpha_path, alpha_source).unwrap();
    fs::write(&broken_path, broken_source).unwrap();

    let records_path = source_dir.path().join("stdlib.files.jsonl");
    let records = format!(
        "{}\n{}\n",
        json!({
            "path": "src/alpha/alpha.go",
            "sha256": sha256_hex(alpha_source),
            "bytes": alpha_source.len(),
            "is_test": false,
            "is_build_constrained": false,
            "is_cgo": false,
            "is_generated": false,
            "parse_failed": false
        }),
        json!({
            "path": "src/beta/broken.go",
            "sha256": sha256_hex(broken_source),
            "bytes": broken_source.len(),
            "is_test": false,
            "is_build_constrained": false,
            "is_cgo": false,
            "is_generated": false,
            "parse_failed": true,
            "parse_error": "fixture parse failure"
        })
    );
    fs::write(&records_path, records).unwrap();

    let report_path = source_dir.path().join("stdlib-coverage-report.json");
    let output = command_with_pinned_go("tct-syntax-go-tokenizer-generator")
        .arg("--stdlib-coverage-records")
        .arg(&records_path)
        .arg("--stdlib-coverage-goroot")
        .arg(&goroot)
        .arg("--stdlib-coverage-output-dir")
        .arg(target_dir_path)
        .arg("--stdlib-coverage-jobs")
        .arg("2")
        .arg("--output")
        .arg(&report_path)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "Go stdlib coverage CLI failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stdout)
            .contains("wrote Go stdlib syntax tokenizer coverage report"),
        "producer should report success:\n{}",
        String::from_utf8_lossy(&output.stdout)
    );

    let report: serde_json::Value = serde_json::from_slice(&fs::read(&report_path).unwrap())
        .expect("Go stdlib coverage report should parse");
    assert_eq!(
        report["artifact_kind"],
        "tct_go_stdlib_syntax_tokenizer_coverage_report_v1"
    );
    assert_eq!(
        report["status"],
        "go_stdlib_syntax_tokenizer_coverage_passed"
    );
    assert_eq!(report["coverage_scope"], "full_denominator");
    assert_eq!(report["parallel_jobs"], json!(2));
    assert_eq!(
        report["candidate_grammar_id"],
        "go_ast_schema_inventory_go1_24_3"
    );
    assert_eq!(report["input_file_count"], json!(2));
    assert_eq!(report["parse_failed_count"], json!(1));
    assert_eq!(report["input_denominator"], json!(1));
    assert_eq!(report["attempted_record_count"], json!(1));
    assert_eq!(report["supported_record_count"], json!(1));
    assert_eq!(report["token_only_render_round_trip_equal_count"], json!(1));
    assert_eq!(report["not_attempted_count"], json!(0));
    assert_eq!(report["target_met"], true);
    assert_eq!(report["records"][0]["path"], "src/alpha/alpha.go");
    assert_eq!(report["records"][0]["status"], "passed");
    assert_eq!(report["records"][0]["raw_state_machine_complete"], true);
    assert_eq!(
        report["records"][0]["token_only_render_round_trip_equal"],
        true
    );
    assert_eq!(report["records"][0]["source_gofmt_reparse_ok"], true);
    assert_eq!(report["records"][1]["status"], "parse_failed_excluded");
    assert_eq!(
        report["gap_buckets"][0]["bucket"],
        "parse_failed_excluded_from_denominator"
    );
    assert!(
        report["claim_boundary"]
            .as_str()
            .expect("claim boundary should be a string")
            .contains("not Arena IR")
    );

    let validation = command_with_pinned_go("tct-syntax-go-tokenizer-generator")
        .arg("--validate-report")
        .arg(&report_path)
        .output()
        .unwrap();
    assert!(
        validation.status.success(),
        "Go stdlib coverage validation failed: {}",
        String::from_utf8_lossy(&validation.stderr)
    );
    assert!(
        String::from_utf8_lossy(&validation.stdout)
            .contains("validated Go stdlib syntax tokenizer coverage report"),
        "validator should report success:\n{}",
        String::from_utf8_lossy(&validation.stdout)
    );
}

#[test]
fn generated_go_source_driven_tokenizer_generator_corpus_smoke_validates() {
    let repo_root = std::env::current_dir().unwrap();
    let manifest: serde_json::Value = serde_json::from_slice(
        &fs::read(GO_SOURCE_DRIVEN_TOKENIZER_GENERATOR_CORPUS_MANIFEST_PATH).unwrap(),
    )
    .expect("source-driven Go tokenizer generator corpus manifest should parse");
    assert_eq!(
        manifest["artifact_kind"],
        "tct_go_source_driven_tokenizer_generator_corpus_manifest_v1"
    );
    assert_eq!(
        manifest["source_of_truth"],
        "src/syntax_production_route.rs::SYNTAX_CODEC_REGISTRY"
    );
    assert_eq!(
        manifest["route_manifest"],
        "tests/fixtures/syntax_backend/production-route-manifest-v1.json"
    );
    assert!(
        manifest["claim_boundary"]
            .as_str()
            .expect("claim boundary should be a string")
            .contains("not Arena IR"),
        "corpus manifest should keep wrong-boundary claims explicit"
    );
    assert!(
        manifest["validation_commands"]
            .as_array()
            .expect("validation commands should be an array")
            .iter()
            .any(|command| command
                .as_str()
                .expect("validation command should be a string")
                .contains("generated_go_source_driven_tokenizer_generator_corpus_smoke_validates")),
        "manifest should name the focused corpus smoke"
    );
    assert_eq!(
        manifest["next_ready_task"],
        GO_TOKENIZER_GENERATOR_NEXT_READY_TASK
    );
    let real_source_corpus_report =
        run_go_real_source_corpus_check_report(GO_REAL_SOURCE_CORPUS_CHECK_REPORT_PATH);

    let records = manifest["records"]
        .as_array()
        .expect("corpus manifest records should be an array");
    assert_eq!(records.len(), 14);
    let dir = TempDir::new_in("target").unwrap();
    let dir_path = dir.path().strip_prefix(&repo_root).unwrap_or(dir.path());

    let mut grammar_ids = BTreeSet::new();
    let mut source_driven_records = Vec::new();
    let mut observed_coverage = GoGrammarCoverage::default();
    for (index, record) in records.iter().enumerate() {
        let source_path = record["source_path"]
            .as_str()
            .expect("corpus source path should be a string");
        let grammar_id = record["grammar_id"]
            .as_str()
            .expect("corpus grammar id should be a string");
        let manifest_path = record["manifest_path"]
            .as_str()
            .expect("corpus manifest path should be a string");
        let registry_entry = syntax_codec_registry_entry(grammar_id)
            .expect("corpus grammar id should be registered in SYNTAX_CODEC_REGISTRY");
        assert_eq!(registry_entry.target_id, "go");
        assert!(
            grammar_ids.insert(grammar_id),
            "corpus manifest should not duplicate grammar id {grammar_id}"
        );
        assert!(Path::new(source_path).exists());
        assert!(Path::new(registry_entry.metadata_path).exists());

        let output_stem = manifest_path.replace(['/', '.'], "-");
        let projection_path = dir_path.join(format!(
            "{index}-{output_stem}-source-instance-projection-v1.json"
        ));
        let projection_report_path = dir_path.join(format!(
            "{index}-{output_stem}-source-instance-projection-report-v1.json"
        ));
        let report_path = dir_path.join(format!(
            "{index}-{output_stem}-tokenizer-generator-report-v1.json"
        ));

        let output = command_with_pinned_go("tct-syntax-go-tokenizer-generator")
            .arg("--source")
            .arg(source_path)
            .arg("--grammar-id")
            .arg(grammar_id)
            .arg("--manifest-path")
            .arg(manifest_path)
            .arg("--projection-output")
            .arg(&projection_path)
            .arg("--projection-report-output")
            .arg(&projection_report_path)
            .arg("--output")
            .arg(&report_path)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "source-driven Go tokenizer generator corpus record {grammar_id} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        let report: serde_json::Value = serde_json::from_slice(&fs::read(&report_path).unwrap())
            .expect("source-driven Go tokenizer generator report should parse");
        assert_eq!(
            report["artifact_kind"],
            "tct_syntax_go_tokenizer_generator_report_v1"
        );
        assert_eq!(report["projection_report_mode"], "source_driven_generated");
        assert_eq!(report["source_path"], source_path);
        assert_eq!(report["candidate_grammar_id"], grammar_id);
        assert_eq!(report["metadata_path"], registry_entry.metadata_path);
        assert_eq!(
            report["checked_projection_report_path"],
            projection_report_path.display().to_string()
        );
        assert_eq!(
            report["projection_artifact_path"],
            projection_path.display().to_string()
        );
        assert!(report["raw_token_count"].as_u64().unwrap() > 0);
        assert_eq!(report["raw_state_machine_complete"], true);
        assert_eq!(
            report["raw_state_machine_decode"]["observed_status"],
            "complete"
        );
        assert_eq!(report["raw_state_machine_decode"]["eof_valid"], true);
        assert_eq!(report["raw_state_machine_tree"]["root_rule"], "File");
        assert_eq!(report["raw_state_machine_tree"]["root_node_kind"], "File");
        assert_eq!(report["raw_state_machine_tree"]["eof_valid"], true);
        assert_eq!(
            report["token_only_decode_render"]["decoded_from_token_only_stream"],
            true
        );
        assert_eq!(
            report["token_only_decode_render"]["render_reparse_sidecar_validation"],
            true
        );
        assert_eq!(report["canonical_render_ready"], true);
        assert_eq!(report["gofmt_validation"], "passed");
        assert_eq!(report["go_parser_reparse_validation"], "passed");
        assert_eq!(report["pipeline"]["arena_ir_used"], false);
        assert_eq!(report["pipeline"]["route_handoff_provenance_used"], false);
        assert_eq!(report["pipeline"]["bpe_validity_authority_used"], false);
        assert_eq!(
            report["next_ready_task"],
            GO_TOKENIZER_GENERATOR_NEXT_READY_TASK
        );

        let projection_report: serde_json::Value =
            serde_json::from_slice(&fs::read(&projection_report_path).unwrap())
                .expect("generated source projection report should parse");
        assert_eq!(projection_report["candidate_grammar_id"], grammar_id);
        assert_eq!(projection_report["manifest_path"], manifest_path);
        assert_eq!(
            projection_report["metadata"]["path"],
            registry_entry.metadata_path
        );
        assert!(
            projection_report["validation_commands"]
                .as_array()
                .expect("projection validation commands should be an array")
                .iter()
                .any(|command| command
                    .as_str()
                    .expect("projection validation command should be a string")
                    .contains("--route-registered")),
            "generated projection report should record registry-backed route validation"
        );
        assert!(
            !projection_report["current_blockers"]
                .as_array()
                .expect("projection blockers should be an array")
                .iter()
                .any(|blocker| blocker["kind"] == "generated_codec_route_unregistered"),
            "registry-backed source-driven projection report must not keep the unregistered-route blocker"
        );

        let validation = command_with_pinned_go("tct-syntax-go-tokenizer-generator")
            .arg("--validate-report")
            .arg(&report_path)
            .output()
            .unwrap();
        assert!(
            validation.status.success(),
            "source-driven Go tokenizer generator validation failed for {grammar_id}: {}",
            String::from_utf8_lossy(&validation.stderr)
        );

        let record_coverage = collect_go_metadata_coverage(registry_entry.metadata_path);
        observed_coverage.merge(&record_coverage);
        source_driven_records.push(json!({
            "manifest_path": manifest_path,
            "source_path": source_path,
            "grammar_id": grammar_id,
            "metadata_path": registry_entry.metadata_path,
            "raw_token_count": report["raw_token_count"],
            "raw_state_machine_complete": report["raw_state_machine_complete"],
            "gofmt_validation": report["gofmt_validation"],
            "go_parser_reparse_validation": report["go_parser_reparse_validation"],
            "node_kind_count": record_coverage.node_kinds.len(),
            "grammar_field_count": record_coverage.grammar_fields.len(),
            "payload_kind_count": record_coverage.payload_kinds.len(),
            "grammar_categories": go_coverage_category_report(&record_coverage),
            "node_kinds": record_coverage.node_kinds,
            "grammar_fields": record_coverage.grammar_fields,
            "payload_kinds": record_coverage.payload_kinds,
        }));
    }
    assert!(grammar_ids.contains("go_strings_compare_source_file_syntax_v0"));
    assert!(grammar_ids.contains("go_os_dir_unix_source_file_syntax_v0"));
    assert!(grammar_ids.contains("go_local_type_switch_syntax_v0"));
    assert!(grammar_ids.contains("go_local_switch_init_syntax_v0"));
    assert!(grammar_ids.contains("go_local_generic_function_syntax_v0"));
    assert!(grammar_ids.contains("go_local_variadic_ellipsis_syntax_v0"));
    assert!(grammar_ids.contains("go_slice_expression_three_index_syntax_v0"));
    assert!(grammar_ids.contains("go_concurrency_map_interface_type_assertion_syntax_v0"));
    assert!(grammar_ids.contains("go_import_spec_comment_syntax_v0"));
    assert!(grammar_ids.contains("go_import_spec_name_syntax_v0"));
    assert!(grammar_ids.contains("go_type_spec_assign_syntax_v0"));
    assert!(grammar_ids.contains("go_type_spec_comment_syntax_v0"));
    assert!(grammar_ids.contains("go_type_spec_doc_syntax_v0"));
    assert!(grammar_ids.contains("go_value_spec_comment_syntax_v0"));

    let gap_report = write_go_source_driven_grammar_gap_report(
        &manifest,
        source_driven_records,
        &observed_coverage,
        &real_source_corpus_report,
        GO_SOURCE_DRIVEN_TOKENIZER_GENERATOR_GRAMMAR_GAP_REPORT_PATH,
    );
    assert_eq!(
        gap_report["artifact_kind"],
        GO_SOURCE_DRIVEN_GRAMMAR_GAP_REPORT_ARTIFACT_KIND
    );
    assert_eq!(
        gap_report["status"],
        GO_SOURCE_DRIVEN_GRAMMAR_GAP_REPORT_STATUS
    );
    assert_eq!(
        gap_report["record_count"],
        json!(records.len()),
        "gap report should cover the whole source-driven manifest"
    );
    assert_eq!(
        gap_report["unsupported_grammar_gaps"]
            .as_array()
            .expect("unsupported grammar gaps should be an array")
            .len(),
        0,
        "registered source-driven records should have no unsupported local grammar gaps"
    );
    assert!(
        gap_report["next_under_proven_grammar_category"].is_null(),
        "source-driven corpus should cover every currently registry-owned Go grammar category"
    );
    assert_eq!(
        gap_report["under_proven_registry_gaps"]["missing_node_kinds"]
            .as_array()
            .expect("missing node kinds should be an array")
            .len(),
        0,
        "source-driven corpus should cover all current registry-owned Go node kinds"
    );
    assert_eq!(
        gap_report["under_proven_registry_gaps"]["missing_payload_kinds"]
            .as_array()
            .expect("missing payload kinds should be an array")
            .len(),
        0,
        "source-driven corpus should cover all current registry-owned Go payload kinds"
    );
    assert!(
        !gap_report["under_proven_registry_gaps"]["missing_grammar_fields"]
            .as_array()
            .expect("missing grammar fields should be an array")
            .iter()
            .any(|field| field == "SwitchStmt.Init"),
        "SwitchStmt.Init should be covered by the source-driven corpus"
    );
    assert!(
        !gap_report["under_proven_registry_gaps"]["missing_grammar_fields"]
            .as_array()
            .expect("missing grammar fields should be an array")
            .iter()
            .any(|field| field == "FuncType.TypeParams"),
        "FuncType.TypeParams should be covered by the source-driven corpus"
    );
    assert!(
        !gap_report["under_proven_registry_gaps"]["missing_grammar_fields"]
            .as_array()
            .expect("missing grammar fields should be an array")
            .iter()
            .any(|field| field == "TypeSpec.Assign"),
        "TypeSpec.Assign should be covered by the source-driven corpus"
    );
    assert!(
        !gap_report["under_proven_registry_gaps"]["missing_grammar_fields"]
            .as_array()
            .expect("missing grammar fields should be an array")
            .iter()
            .any(|field| field == "TypeSpec.Comment"),
        "TypeSpec.Comment should be covered by the source-driven corpus"
    );
    assert!(
        !gap_report["under_proven_registry_gaps"]["missing_grammar_fields"]
            .as_array()
            .expect("missing grammar fields should be an array")
            .iter()
            .any(|field| field == "TypeSpec.Doc"),
        "TypeSpec.Doc should be covered by the source-driven corpus"
    );
    assert!(
        !gap_report["under_proven_registry_gaps"]["missing_grammar_fields"]
            .as_array()
            .expect("missing grammar fields should be an array")
            .iter()
            .any(|field| field == "ValueSpec.Comment"),
        "ValueSpec.Comment should be covered by the source-driven corpus"
    );
    assert!(
        !gap_report["under_proven_registry_gaps"]["missing_grammar_fields"]
            .as_array()
            .expect("missing grammar fields should be an array")
            .iter()
            .any(|field| field == "ImportSpec.Comment"),
        "ImportSpec.Comment should be covered by the source-driven corpus"
    );
    assert!(
        !gap_report["under_proven_registry_gaps"]["missing_grammar_fields"]
            .as_array()
            .expect("missing grammar fields should be an array")
            .iter()
            .any(|field| field == "ImportSpec.Name"),
        "ImportSpec.Name should be covered by the source-driven corpus"
    );
    assert!(
        gap_report["claim_boundary"]
            .as_str()
            .expect("claim boundary should be a string")
            .contains("not Arena IR")
    );
    let real_source_corpus_check = &gap_report["real_source_corpus_check"];
    assert_eq!(
        real_source_corpus_check["report_path"],
        GO_REAL_SOURCE_CORPUS_CHECK_REPORT_PATH
    );
    assert_eq!(real_source_corpus_check["status"], "ready");
    assert_eq!(real_source_corpus_check["entry_count"], 7);
    assert_eq!(real_source_corpus_check["unsupported_construct_count"], 0);
    assert_eq!(real_source_corpus_check["unsupported_grammar_gap_count"], 0);
    assert!(
        real_source_corpus_check["next_missing_construct"].is_null(),
        "real-source checker should not report a next missing construct"
    );
    assert!(
        real_source_corpus_check["next_missing_grammar_gap"].is_null(),
        "real-source checker should not report a next missing grammar gap"
    );
    assert_eq!(
        real_source_corpus_check["source_driven_overlap_count"], 1,
        "source-driven depth comparison should record the checked real-source overlap"
    );
    assert_eq!(
        real_source_corpus_check["checker_ready_without_source_driven_record_count"], 6,
        "source-driven depth comparison should name checker-ready real-source files outside the source-driven manifest"
    );
    assert!(
        real_source_corpus_check["stable_promotion_blockers"]
            .as_array()
            .expect("real-source stable blockers should be an array")
            .iter()
            .any(|blocker| blocker == GO_REAL_SOURCE_GENERATED_CODEC_ROUND_TRIP_BLOCKER),
        "real-source comparison should retain the generated-codec round-trip blocker"
    );
    assert_eq!(
        real_source_corpus_check["next_concrete_task"],
        GO_TOKENIZER_GENERATOR_NEXT_READY_TASK
    );
    assert_eq!(
        gap_report["next_ready_task"],
        GO_TOKENIZER_GENERATOR_NEXT_READY_TASK
    );
    let persisted_gap_report: serde_json::Value = serde_json::from_slice(
        &fs::read(GO_SOURCE_DRIVEN_TOKENIZER_GENERATOR_GRAMMAR_GAP_REPORT_PATH).unwrap(),
    )
    .expect("persisted source-driven grammar gap report should parse");
    assert_eq!(persisted_gap_report, gap_report);
}

#[test]
fn syntax_source_corpus_evidence_cli_writes_report() {
    let expected_record_count = production_route_manifest_record_count();

    let dir = TempDir::new().unwrap();
    let output_path = dir.path().join("source-corpus-report.json");

    let output = Command::new(bin_path("tct-syntax-source-corpus-evidence"))
        .arg("--output")
        .arg(&output_path)
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "tct-syntax-source-corpus-evidence failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stdout)
            .contains(&format!("wrote {expected_record_count} records")),
        "producer should report the written record count: {}",
        String::from_utf8_lossy(&output.stdout)
    );

    let report: serde_json::Value =
        serde_json::from_slice(&fs::read(&output_path).unwrap()).unwrap();
    assert_eq!(
        report["artifact_kind"],
        "tct_syntax_backend_source_corpus_evidence"
    );
    assert_eq!(report["version"], 1);
    assert_eq!(report["tool"], "syntax_backend_source_corpus_evidence");
    assert_eq!(report["status"], "generated_sidecar_report");
    assert_eq!(report["record_count"], expected_record_count);
    // The next promotion class rotates with roadmap progress; pin only that
    // the producer names one.
    assert!(
        report["next_missing_stable_promotion_class"]
            .as_str()
            .is_some_and(|class| !class.is_empty()),
        "producer should name the next missing stable promotion class"
    );
    assert_eq!(
        report["source_production_route"]["route_manifest_loader"],
        "src/syntax_route_manifest.rs::checked_syntax_production_route_manifest_constructs"
    );
    assert_sha256_field(&report["records_sha256"]);

    let records = report["records"]
        .as_array()
        .expect("source-corpus records should be an array");
    assert!(records.iter().any(|record| record["target_id"] == "csharp"));
    assert!(records.iter().any(|record| record["target_id"] == "go"));
    assert_sha256_field(&records[0]["source_sha256"]);
    assert_sha256_field(&records[0]["metadata_sha256"]);
    assert_eq!(
        records[0]["projector_metadata_setup"]["checked_metadata_path"],
        records[0]["metadata_path"]
    );
    assert!(
        records[0]["projector_metadata_setup"]["generated_output_path"]
            .as_str()
            .expect("projector output path should be a string")
            .starts_with("target/")
    );
    assert_sha256_field(&records[0]["generated_codec"]["rendered_source_sha256"]);
    assert_eq!(
        report["report_policy"]["generated_report_path"],
        "target/tct-syntax-source-corpus-evidence/parser-backed-source-corpus-evidence-v1.json"
    );
    assert_eq!(
        report["report_policy"]["source_report_validator_command"],
        "cargo run --bin tct-syntax-source-corpus-evidence -- --route-manifest tests/fixtures/syntax_backend/production-route-manifest-v1.json --validate-report target/tct-syntax-source-corpus-evidence/parser-backed-source-corpus-evidence-v1.json"
    );

    let validation = Command::new(bin_path("tct-syntax-source-corpus-evidence"))
        .arg("--validate-report")
        .arg(&output_path)
        .output()
        .unwrap();
    assert!(
        validation.status.success(),
        "tct-syntax-source-corpus-evidence validation failed: {}",
        String::from_utf8_lossy(&validation.stderr)
    );
    assert!(
        String::from_utf8_lossy(&validation.stdout)
            .contains(&format!("validated {expected_record_count} records")),
        "validator should report the checked record count: {}",
        String::from_utf8_lossy(&validation.stdout)
    );

    let stale_path = dir.path().join("stale-source-corpus-report.json");
    let mut stale_report = report.clone();
    stale_report["record_count"] = json!(expected_record_count - 2);
    fs::write(
        &stale_path,
        serde_json::to_string_pretty(&stale_report).unwrap(),
    )
    .unwrap();
    let stale_validation = Command::new(bin_path("tct-syntax-source-corpus-evidence"))
        .arg("--validate-report")
        .arg(&stale_path)
        .output()
        .unwrap();
    assert!(
        !stale_validation.status.success(),
        "stale source-corpus report validation should fail"
    );
    assert!(
        String::from_utf8_lossy(&stale_validation.stderr).contains("report.record_count"),
        "stale validation should identify the drifting field: {}",
        String::from_utf8_lossy(&stale_validation.stderr)
    );
}

#[test]
fn syntax_source_program_handoff_cli_writes_report() {
    let expected_record_count = production_route_manifest_record_count();

    let dir = TempDir::new().unwrap();
    let source_corpus_path = dir.path().join("source-corpus-report.json");
    let handoff_path = dir.path().join("source-program-handoff-report.json");

    let source_corpus_output = Command::new(bin_path("tct-syntax-source-corpus-evidence"))
        .arg("--output")
        .arg(&source_corpus_path)
        .output()
        .unwrap();
    assert!(
        source_corpus_output.status.success(),
        "tct-syntax-source-corpus-evidence failed before handoff: {}",
        String::from_utf8_lossy(&source_corpus_output.stderr)
    );

    let output = Command::new(bin_path("tct-syntax-source-program-handoff"))
        .arg("--source-corpus-report")
        .arg(&source_corpus_path)
        .arg("--output")
        .arg(&handoff_path)
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "tct-syntax-source-program-handoff failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stdout)
            .contains(&format!("wrote {expected_record_count} records")),
        "producer should report the written record count: {}",
        String::from_utf8_lossy(&output.stdout)
    );

    let report: serde_json::Value =
        serde_json::from_slice(&fs::read(&handoff_path).unwrap()).unwrap();
    assert_eq!(
        report["artifact_kind"],
        "tct_syntax_backend_source_program_claim_handoff_report"
    );
    assert_eq!(report["version"], 1);
    assert_eq!(
        report["tool"],
        "syntax_backend_source_program_claim_handoff"
    );
    assert_eq!(report["status"], "generated_handoff_report");
    assert_eq!(
        report["profile_id"],
        "syntax-source-program-claim-handoff-v1"
    );
    assert_eq!(report["record_count"], expected_record_count);
    assert_eq!(
        report["stable_metadata_promotion"]["artifact_kind"],
        "tct_syntax_backend_stable_metadata_promotion_implementation_gate"
    );
    assert_eq!(
        report["go_generated_codec_completion_handoff"]["artifact_kind"],
        "tct_syntax_backend_go_real_source_generated_codec_completion_handoff_gate"
    );
    assert_eq!(
        report["completed_promotion_class"],
        "syntax backend source-program handoff report producer gate"
    );
    assert_eq!(
        report["next_missing_stable_promotion_class"],
        "syntax backend source-program external validation control evidence gate"
    );
    assert_eq!(
        report["source_corpus"]["artifact_kind"],
        "tct_syntax_backend_source_corpus_evidence"
    );
    assert_sha256_field(&report["source_corpus"]["report_sha256"]);
    assert_sha256_field(&report["records_sha256"]);

    let records = report["records"]
        .as_array()
        .expect("source-program handoff records should be an array");
    assert!(records.iter().any(|record| record["target_id"] == "csharp"));
    assert!(records.iter().any(|record| record["target_id"] == "go"));
    assert_sha256_field(&records[0]["source_sha256"]);
    assert_sha256_field(&records[0]["metadata_sha256"]);
    assert_sha256_field(&records[0]["structural_model_sha256"]);
    assert_sha256_field(&records[0]["token_layout_sha256"]);
    assert_eq!(
        records[0]["route_metadata"]["generated_json_schema_output_changed"],
        false
    );
    assert_eq!(records[0]["sidecar_controls"]["arena_ir_proof_used"], false);
    assert_eq!(
        records[0]["sidecar_controls"]["external_sidecars_promoted"],
        false
    );

    let validation = Command::new(bin_path("tct-syntax-source-program-handoff"))
        .arg("--validate-report")
        .arg(&handoff_path)
        .output()
        .unwrap();
    assert!(
        validation.status.success(),
        "tct-syntax-source-program-handoff validation failed: {}",
        String::from_utf8_lossy(&validation.stderr)
    );
    assert!(
        String::from_utf8_lossy(&validation.stdout)
            .contains(&format!("validated {expected_record_count} records")),
        "validator should report the checked record count: {}",
        String::from_utf8_lossy(&validation.stdout)
    );

    let stale_path = dir.path().join("stale-source-program-handoff-report.json");
    let mut stale_report = report.clone();
    stale_report["record_count"] = json!(expected_record_count - 2);
    fs::write(
        &stale_path,
        serde_json::to_string_pretty(&stale_report).unwrap(),
    )
    .unwrap();
    let stale_validation = Command::new(bin_path("tct-syntax-source-program-handoff"))
        .arg("--validate-report")
        .arg(&stale_path)
        .output()
        .unwrap();
    assert!(
        !stale_validation.status.success(),
        "stale source-program handoff report validation should fail"
    );
    assert!(
        String::from_utf8_lossy(&stale_validation.stderr).contains("report.record_count"),
        "stale validation should identify the drifting field: {}",
        String::from_utf8_lossy(&stale_validation.stderr)
    );
}

#[test]
fn syntax_source_program_external_validation_cli_writes_report() {
    let expected_record_count = production_route_manifest_record_count();

    let dir = TempDir::new().unwrap();
    let source_corpus_path = dir.path().join("source-corpus-report.json");
    let handoff_path = dir.path().join("source-program-handoff-report.json");
    let external_validation_path = dir.path().join("external-validation-report.json");

    let source_corpus_output = Command::new(bin_path("tct-syntax-source-corpus-evidence"))
        .arg("--output")
        .arg(&source_corpus_path)
        .output()
        .unwrap();
    assert!(
        source_corpus_output.status.success(),
        "tct-syntax-source-corpus-evidence failed before external validation: {}",
        String::from_utf8_lossy(&source_corpus_output.stderr)
    );

    let handoff_output = Command::new(bin_path("tct-syntax-source-program-handoff"))
        .arg("--source-corpus-report")
        .arg(&source_corpus_path)
        .arg("--output")
        .arg(&handoff_path)
        .output()
        .unwrap();
    assert!(
        handoff_output.status.success(),
        "tct-syntax-source-program-handoff failed before external validation: {}",
        String::from_utf8_lossy(&handoff_output.stderr)
    );

    let output = Command::new(bin_path("tct-syntax-source-program-external-validation"))
        .arg("--source-program-handoff-report")
        .arg(&handoff_path)
        .arg("--output")
        .arg(&external_validation_path)
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "tct-syntax-source-program-external-validation failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stdout)
            .contains(&format!("wrote {expected_record_count} records")),
        "producer should report the written record count: {}",
        String::from_utf8_lossy(&output.stdout)
    );

    let report: serde_json::Value =
        serde_json::from_slice(&fs::read(&external_validation_path).unwrap()).unwrap();
    assert_eq!(
        report["artifact_kind"],
        "tct_syntax_backend_source_program_external_validation_execution_report"
    );
    assert_eq!(report["version"], 1);
    assert_eq!(
        report["tool"],
        "syntax_backend_source_program_external_validation_execution_report"
    );
    assert_eq!(
        report["status"],
        "generated_external_validation_execution_report"
    );
    assert_eq!(report["record_count"], expected_record_count);
    assert_eq!(
        report["controls_fixture"]["artifact_kind"],
        "tct_syntax_backend_source_program_external_validation_controls"
    );
    assert_eq!(
        report["source_handoff_report"]["artifact_kind"],
        "tct_syntax_backend_source_program_claim_handoff_report"
    );
    assert_sha256_field(&report["controls_fixture"]["fixture_sha256"]);
    assert_sha256_field(&report["source_handoff_report"]["report_sha256"]);
    assert_sha256_field(&report["records_sha256"]);
    assert_eq!(
        report["completed_promotion_class"],
        "syntax backend source-program external validation execution report gate"
    );
    assert_eq!(
        report["next_missing_stable_promotion_class"],
        "syntax backend source-program external validation execution preservation gate"
    );
    assert_eq!(
        report["audit_coverage"]["generated_reports_checked_in"],
        false
    );
    assert!(
        report["tool_envelope"]["observed_tools"]
            .as_array()
            .expect("tool observations should be an array")
            .iter()
            .any(|tool| tool["tool_id"] == "go")
    );
    assert!(
        report["control_summary"]
            .as_array()
            .expect("control summary should be an array")
            .iter()
            .any(|control| control["control_id"] == "type_checker"
                && control["ran"] == false
                && control["unsupported_claims"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|claim| claim == "type_correctness_claim"))
    );

    let records = report["records"]
        .as_array()
        .expect("external-validation records should be an array");
    assert!(records.iter().any(|record| record["target_id"] == "csharp"));
    assert!(records.iter().any(|record| record["target_id"] == "go"));
    assert_sha256_field(&records[0]["source_sha256"]);
    assert_sha256_field(&records[0]["metadata_sha256"]);
    assert!(
        records[0]["control_results"]
            .as_array()
            .unwrap()
            .iter()
            .any(|control| control["control_id"] == "functional_validator"
                && control["ran"] == false
                && control["stable_promotion_allowed"] == false)
    );

    let validation = Command::new(bin_path("tct-syntax-source-program-external-validation"))
        .arg("--source-program-handoff-report")
        .arg(&handoff_path)
        .arg("--validate-report")
        .arg(&external_validation_path)
        .output()
        .unwrap();
    assert!(
        validation.status.success(),
        "tct-syntax-source-program-external-validation validation failed: {}",
        String::from_utf8_lossy(&validation.stderr)
    );
    assert!(
        String::from_utf8_lossy(&validation.stdout)
            .contains(&format!("validated {expected_record_count} records")),
        "validator should report the checked record count: {}",
        String::from_utf8_lossy(&validation.stdout)
    );

    let stale_path = dir.path().join("stale-external-validation-report.json");
    let mut stale_report = report.clone();
    stale_report["record_count"] = json!(expected_record_count - 2);
    fs::write(
        &stale_path,
        serde_json::to_string_pretty(&stale_report).unwrap(),
    )
    .unwrap();
    let stale_validation = Command::new(bin_path("tct-syntax-source-program-external-validation"))
        .arg("--source-program-handoff-report")
        .arg(&handoff_path)
        .arg("--validate-report")
        .arg(&stale_path)
        .output()
        .unwrap();
    assert!(
        !stale_validation.status.success(),
        "stale external-validation report validation should fail"
    );
    assert!(
        String::from_utf8_lossy(&stale_validation.stderr)
            .contains("execution_report.control_summary.record_count"),
        "stale validation should identify the drifting field: {}",
        String::from_utf8_lossy(&stale_validation.stderr)
    );
}
