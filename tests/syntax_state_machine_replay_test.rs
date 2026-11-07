use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use tct::csharp_syntax_metadata::CSharpSyntaxMetadataAdapter;
use tct::go_ast_schema_inventory::{
    GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH, GoAstSchemaInventoryAdapter,
};
use tct::go_syntax_metadata::GoSyntaxMetadataAdapter;
use tct::grammar_ir_structural_model::{
    GrammarStructuralModelSidecar, lower_grammar_ir_to_structural_model,
};
use tct::syntax_csharp_raw_tree_renderer::{
    CSHARP_COMPILATION_UNIT_LOGGER_SOURCE_FILE_RAW_TREE_RENDERED_SOURCE,
    CSHARP_COMPILATION_UNIT_SOURCE_FILE_RAW_TREE_RENDERED_SOURCE,
    CSHARP_COMPILATION_UNIT_VERSION_ONE_SOURCE_FILE_RAW_TREE_RENDERED_SOURCE,
    CSHARP_LOCAL_INTEGER_RETURN_LITERAL_RAW_TREE_RENDERED_SOURCE,
    CSHARP_LOCAL_INTEGER_RETURN_LITERAL_RAW_TREE_RENDERER_SCOPE,
    CSHARP_LOCAL_UNARY_INTEGER_RETURN_RAW_TREE_RENDERED_SOURCE,
    CSHARP_LOCAL_UNARY_INTEGER_RETURN_RAW_TREE_RENDERER_SCOPE,
    CSHARP_MEMBER_ACCESS_EXPRESSION_RAW_TREE_RENDERED_SOURCE,
    CSHARP_MEMBER_ACCESS_EXPRESSION_RAW_TREE_RENDERER_SCOPE,
    CSHARP_MINIMAL_CLASS_RAW_TREE_RENDERED_SOURCE, CSHARP_RAW_TREE_RENDERER_ID,
    render_csharp_compilation_unit_logger_source_file_from_raw_tree,
    render_csharp_compilation_unit_source_file_from_raw_tree,
    render_csharp_compilation_unit_version_one_source_file_from_raw_tree,
    render_csharp_local_integer_return_literal_from_raw_tree,
    render_csharp_local_unary_integer_return_from_raw_tree,
    render_csharp_member_access_expression_from_raw_tree,
    render_csharp_minimal_class_source_from_raw_tree,
};
use tct::syntax_csharp_real_source_generated_codec_corpus::SyntaxCSharpRealSourceGeneratedCodecCorpusGate;
use tct::syntax_generated_codec::{
    CSHARP_COMPILATION_UNIT_LOGGER_SOURCE_FILE_METADATA_PATH,
    CSHARP_COMPILATION_UNIT_SOURCE_FILE_METADATA_PATH,
    CSHARP_COMPILATION_UNIT_VERSION_ONE_SOURCE_FILE_METADATA_PATH,
    CSHARP_LOCAL_IF_ELSE_METADATA_PATH, CSHARP_LOCAL_INTEGER_RETURN_LITERAL_METADATA_PATH,
    CSHARP_LOCAL_NULLABLE_DECLARATION_METADATA_PATH,
    CSHARP_LOCAL_NULLABLE_RETURN_TYPE_METADATA_PATH,
    CSHARP_LOCAL_UNARY_INTEGER_RETURN_METADATA_PATH, CSHARP_MEMBER_ACCESS_EXPRESSION_METADATA_PATH,
    CSHARP_METADATA_PATH,
};
use tct::syntax_generated_codec::{
    csharp_syntax_token_layout, go_syntax_source_file_token_layout, go_syntax_token_layout,
};
use tct::syntax_generated_codec_state_api_handoff::{
    ARTIFACT_KIND as SHARED_STATE_API_HANDOFF_ARTIFACT_KIND,
    COMPLETED_PROMOTION_CLASS as SHARED_STATE_API_HANDOFF_COMPLETED_CLASS,
    FIXTURE_PATH as SHARED_STATE_API_HANDOFF_FIXTURE_PATH,
    NEXT_MISSING_STABLE_PROMOTION_CLASS as SHARED_STATE_API_HANDOFF_NEXT_CLASS,
    SyntaxGeneratedCodecStateApiHandoffGate,
};
use tct::syntax_generated_codec_state_api_integration::{
    ARTIFACT_KIND as STATE_API_INTEGRATION_ARTIFACT_KIND,
    COMPLETED_PROMOTION_CLASS as STATE_API_INTEGRATION_COMPLETED_CLASS,
    CSHARP_LOCAL_NULLABLE_DECLARATION_MARKER_PRESENCE_RECORD_ID,
    CSHARP_NULLABLE_MARKER_PRESENCE_RECORD_ID, MATCHED_PRESENCE_MASK_SCOPE,
    NEXT_MISSING_STABLE_PROMOTION_CLASS as STATE_API_INTEGRATION_NEXT_CLASS,
    SyntaxGeneratedCodecStateApiIntegrationGate, TARGET_NEUTRAL_RAW_STATE_API_AUTHORITY,
    TargetNeutralGeneratedCodecStateApi, csharp_nullable_marker_presence_record_query,
    csharp_nullable_marker_presence_record_query_for_record_id,
    matched_go_csharp_presence_mask_contract,
};
use tct::syntax_go_scratch_seed::{
    go_ast_inventory_scratch_seeds, verify_go_scratch_seed_admissibility,
};
use tct::syntax_go_source_instance_projection::{
    RAW_TOKEN_ALTERNATIVE_NODE_START_OFFSET, SyntaxGoSourceInstanceProjectionReport,
    parse_checked_report, parse_projection_artifact,
};
use tct::syntax_go_tokenizer_generator::{
    GoTokenizerGeneratorReportIngestionSource, go_tokenizer_generator_report_ingestion_sources,
};
use tct::syntax_go_unified_artifact::{
    GO_UNIFIED_GRAMMAR_ARTIFACT_ID, GO_UNIFIED_GRAMMAR_ID, go_unified_grammar_artifact,
};
use tct::syntax_scratch_generation::{RawScratchClaimBoundaryFlags, RawScratchEmptyPrefixSummary};
use tct::syntax_state_machine::{
    CompiledSyntaxModel, RAW_SYNTAX_STATE_MACHINE_CORE_BOUNDARY,
    RAW_SYNTAX_STATE_MACHINE_CORE_BOUNDARY_ID, RawSyntaxAdmissibleTokens,
    RawSyntaxDecodedEventKind, RawSyntaxDecodedTree, RawSyntaxDecodedValue, RawSyntaxError,
    RawSyntaxSelectedCompletionTrace, RawSyntaxState, RawSyntaxStateMachineCore, RawSyntaxStatus,
    RawSyntaxTransition, SyntaxTokenRoles,
};
use tct::syntax_state_machine_replay::{
    CSHARP_COMPILATION_UNIT_LOGGER_SOURCE_FILE_GRAMMAR_ID,
    CSHARP_COMPILATION_UNIT_LOGGER_SOURCE_FILE_RAW_TREE_RENDERER_SCOPE,
    CSHARP_COMPILATION_UNIT_LOGGER_SOURCE_FILE_SCRATCH_GENERATION_SCOPE,
    CSHARP_COMPILATION_UNIT_SOURCE_FILE_GRAMMAR_ID,
    CSHARP_COMPILATION_UNIT_SOURCE_FILE_RAW_TREE_RENDERER_SCOPE,
    CSHARP_COMPILATION_UNIT_SOURCE_FILE_SCRATCH_GENERATION_SCOPE,
    CSHARP_COMPILATION_UNIT_VERSION_ONE_SOURCE_FILE_GRAMMAR_ID,
    CSHARP_COMPILATION_UNIT_VERSION_ONE_SOURCE_FILE_RAW_TREE_RENDERER_SCOPE,
    CSHARP_COMPILATION_UNIT_VERSION_ONE_SOURCE_FILE_SCRATCH_GENERATION_SCOPE,
    CSHARP_LOCAL_INTEGER_RETURN_LITERAL_GRAMMAR_ID,
    CSHARP_LOCAL_INTEGER_RETURN_LITERAL_SCRATCH_GENERATION_SCOPE,
    CSHARP_LOCAL_UNARY_INTEGER_RETURN_GRAMMAR_ID,
    CSHARP_LOCAL_UNARY_INTEGER_RETURN_SCRATCH_GENERATION_SCOPE,
    CSHARP_MEMBER_ACCESS_EXPRESSION_GRAMMAR_ID,
    CSHARP_MEMBER_ACCESS_EXPRESSION_SCRATCH_GENERATION_SCOPE,
    CSHARP_SCRATCH_GENERATION_NEXT_READY_TASK, CSHARP_SELECTED_COMPLETION_TRACE_SCOPE,
    CSHARP_SOURCE_RECORD_RAW_TOKEN_EMISSION_API_BOUNDARY,
    CSHARP_SOURCE_RECORD_RAW_TOKEN_EMISSION_API_ID, CSHARP_STATE_MACHINE_PARITY_CLAIM_SCOPE,
    CSHARP_STATE_MACHINE_PARITY_NEXT_READY_TASK, GENERATED_CODEC_STATE_API_SUCCESSOR_NEXT_TASK,
    GO_AST_DECLARATION_PRESENCE_REPLAY_ARTIFACT_KIND,
    GO_AST_DECLARATION_PRESENCE_REPLAY_NEXT_READY_TASK, REPLAY_PREFIX_INSPECTION_STATE_API_SOURCE,
    SELECTED_COMPLETION_TRACE_SOURCE, SyntaxCSharpStateMachineParityReport,
    SyntaxGeneratedCodecStateApiReport, SyntaxGeneratedCodecStrictRecoveryDecodeReport,
    build_csharp_compilation_unit_logger_source_file_raw_tree_render_token_stream_from_sidecar,
    build_csharp_compilation_unit_source_file_raw_tree_render_token_stream_from_sidecar,
    build_csharp_compilation_unit_version_one_source_file_raw_tree_render_token_stream_from_sidecar,
    build_csharp_local_integer_return_literal_raw_tree_render_token_stream_from_sidecar,
    build_csharp_local_unary_integer_return_raw_tree_render_token_stream_from_sidecar,
    build_csharp_member_access_expression_raw_tree_render_token_stream_from_sidecar,
    build_csharp_minimal_class_raw_tree_render_token_stream_from_sidecar,
    build_csharp_real_source_state_machine_parity_report, build_csharp_state_machine_parity_report,
    build_go_ast_declaration_presence_replay_report,
    build_go_source_instance_generated_codec_state_api_report,
    build_go_source_instance_replay_report, build_go_source_instance_strict_recovery_decode_report,
    csharp_real_source_state_machine_parity_report_path,
};
use tct::syntax_token_layout_diff::LIVE_RAW_SYNTAX_STREAM_CONTRACT;

const CHECKED_REPORT_PATH: &str =
    "tests/fixtures/go_syntax_corpus/source-instance-projection-report-httptest-server-v1.json";
const GO_SOURCE_INSTANCE_REPORT_DIR: &str = "tests/fixtures/go_syntax_corpus";
const GO_CURRENT_PLUS_REPORT_INGESTION_MANIFEST_PATH: &str =
    "tests/fixtures/go_syntax_corpus/training-report-ingestion-current-plus-v1.json";
/// Bounded current-plus sample for the mask/transition equivalence property.
/// Smallest-by-expected-token-count keeps the default gate linear; the full
/// 142-record corpus stays on ignored census generators, not this property.
const CURRENT_PLUS_EQUIVALENCE_SAMPLE_SIZE: usize = 4;
/// Replay-reference sample budget per real-source stream (incremental walker
/// still probes every prefix exhaustively).
const REAL_SOURCE_EQUIVALENCE_REPLAY_PREFIX_SAMPLES: usize = 64;
/// Cap on node-start alternative tokens probed on real-source streams.
const REAL_SOURCE_EQUIVALENCE_ALTERNATIVE_SAMPLE_LIMIT: usize = 8;
const STALE_SOURCE_PROGRAM_SUCCESSOR_TASK: &str =
    "syntax backend source-program claim handoff gate";
const GO_STATE_API_REPORT_DIR: &str = "target/tct-syntax-state-machine-replay";
const REPLAY_REPORT_PATH: &str =
    "target/tct-syntax-state-machine-replay/go-httptest-server-replay-report-v1.json";
const STATE_API_REPORT_PATH: &str =
    "target/tct-syntax-state-machine-replay/go-httptest-server-state-api-report-v1.json";
const STRICT_RECOVERY_REPORT_PATH: &str = "target/tct-syntax-state-machine-replay/go-httptest-server-strict-recovery-decode-report-v1.json";
const GO_AST_DECLARATION_PRESENCE_REPLAY_REPORT_PATH: &str =
    "target/tct-syntax-state-machine-replay/go-ast-declaration-presence-replay-report-v1.json";
const CSHARP_MINIMAL_CLASS_SCRATCH_SEED_PARITY_REPORT_PATH: &str =
    "target/tct-syntax-state-machine-replay/csharp-minimal-class-scratch-seed-parity-v1.json";
const CSHARP_LOCAL_INTEGER_RETURN_LITERAL_SCRATCH_SEED_PARITY_REPORT_PATH: &str = "target/tct-syntax-state-machine-replay/csharp-local-integer-return-literal-scratch-seed-parity-v1.json";
const CSHARP_LOCAL_UNARY_INTEGER_RETURN_SCRATCH_SEED_PARITY_REPORT_PATH: &str = "target/tct-syntax-state-machine-replay/csharp-local-unary-integer-return-scratch-seed-parity-v1.json";
const CSHARP_MEMBER_ACCESS_EXPRESSION_SCRATCH_SEED_PARITY_REPORT_PATH: &str = "target/tct-syntax-state-machine-replay/csharp-member-access-expression-scratch-seed-parity-v1.json";
const CSHARP_COMPILATION_UNIT_SOURCE_FILE_SCRATCH_SEED_PARITY_REPORT_PATH: &str = "target/tct-syntax-state-machine-replay/csharp-compilation-unit-source-file-scratch-seed-parity-v1.json";
const CSHARP_COMPILATION_UNIT_LOGGER_SOURCE_FILE_SCRATCH_SEED_PARITY_REPORT_PATH: &str = "target/tct-syntax-state-machine-replay/csharp-compilation-unit-logger-source-file-scratch-seed-parity-v1.json";
const CSHARP_COMPILATION_UNIT_VERSION_ONE_SOURCE_FILE_SCRATCH_SEED_PARITY_REPORT_PATH: &str = "target/tct-syntax-state-machine-replay/csharp-compilation-unit-version-one-source-file-scratch-seed-parity-v1.json";
const CSHARP_PARITY_REPORT_PATH: &str =
    "target/tct-syntax-state-machine-replay/csharp-local-if-else-state-machine-parity-v1.json";
const CSHARP_NULLABLE_MARKER_PARITY_REPORT_PATH: &str = "target/tct-syntax-state-machine-replay/csharp-local-nullable-return-type-marker-presence-parity-v1.json";
const CSHARP_LOCAL_NULLABLE_DECLARATION_MARKER_PARITY_REPORT_PATH: &str = "target/tct-syntax-state-machine-replay/csharp-local-nullable-declaration-marker-presence-parity-v1.json";
const CSHARP_REAL_SOURCE_PARITY_REPORT_PATH: &str =
    "target/tct-syntax-state-machine-replay/csharp-real-source-state-machine-parity-corpus-v1.json";
const CSHARP_REAL_SOURCE_GENERATED_CODEC_CORPUS_GATE: &str =
    include_str!("fixtures/syntax_backend/csharp-real-source-generated-codec-corpus-v1.json");
const SHARED_GENERATED_CODEC_STATE_API_HANDOFF_GATE: &str =
    include_str!("fixtures/syntax_backend/shared-generated-codec-state-api-handoff-v1.json");
const GENERATED_CODEC_STATE_API_INTEGRATION_GATE: &str =
    include_str!("fixtures/syntax_backend/generated-codec-state-api-integration-v1.json");

fn checked_report() -> SyntaxGoSourceInstanceProjectionReport {
    let contents =
        fs::read_to_string(CHECKED_REPORT_PATH).expect("checked projection report should exist");
    parse_checked_report(&contents).expect("checked projection report should validate")
}

/// Deterministic, cost-bounded sample of the current-plus report set for the
/// mask/transition equivalence property: the N smallest records by checked
/// source byte size, tie-broken by source path. Source bytes correlate with
/// stream length closely enough to keep the default gate linear without
/// re-projecting the full 142-record corpus first.
///
/// The dedicated httptest depth fixture is not in the current-plus manifest and
/// remains a separate depth check. This sample only wires the corpus breadth
/// that the RawSyntaxStream handle made cheap enough to keep on by default.
fn current_plus_equivalence_property_sample() -> Vec<CheckedSourceInstanceReport> {
    let mut sample = checked_source_instance_report_set();
    sample.sort_by(|left, right| {
        left.report
            .source
            .bytes
            .cmp(&right.report.source.bytes)
            .then_with(|| left.report.source.path.cmp(&right.report.source.path))
    });
    sample.truncate(CURRENT_PLUS_EQUIVALENCE_SAMPLE_SIZE);
    assert_eq!(
        sample.len(),
        CURRENT_PLUS_EQUIVALENCE_SAMPLE_SIZE,
        "current-plus equivalence sample requires at least {CURRENT_PLUS_EQUIVALENCE_SAMPLE_SIZE} checked records"
    );
    sample
}

/// Compile one source-instance projection report into a raw stream and run the
/// dual-path mask/transition equivalence property against it.
fn assert_transition_acceptance_matches_mask_on_source_instance_report(
    report: &SyntaxGoSourceInstanceProjectionReport,
) {
    let record_sidecar = checked_go_source_instance_sidecar(report);
    let artifact_contents = regenerated_projection_artifact_for(report);
    let artifact =
        parse_projection_artifact(&artifact_contents).expect("projection artifact should parse");
    let raw_tokens = artifact
        .raw_token_ids_against_report(report)
        .expect("projection artifact should emit raw token IDs");
    let record_model =
        CompiledSyntaxModel::from_sidecar_and_layout(&record_sidecar, &raw_tokens.token_layout)
            .expect("raw syntax state machine should compile from StructuralModel");
    let record_candidates = equivalence_probe_candidates(
        record_model.token_roles(),
        Some(REAL_SOURCE_EQUIVALENCE_ALTERNATIVE_SAMPLE_LIMIT),
    );
    let record_prefixes = sampled_prefix_lengths(
        &raw_tokens,
        record_model.token_roles(),
        REAL_SOURCE_EQUIVALENCE_REPLAY_PREFIX_SAMPLES,
    );
    assert_transition_acceptance_matches_mask(
        &record_model,
        &report.source.path,
        &raw_tokens.raw_token_ids,
        &record_candidates,
        &record_prefixes,
    );
}

fn checked_go_source_instance_sidecar(
    report: &SyntaxGoSourceInstanceProjectionReport,
) -> GrammarStructuralModelSidecar {
    if report.candidate_grammar_id == GO_UNIFIED_GRAMMAR_ID {
        return go_unified_grammar_artifact()
            .expect("unified Go grammar artifact should build")
            .sidecar
            .clone();
    }
    let grammar = GoSyntaxMetadataAdapter::new(&report.candidate_grammar_id)
        .lower_metadata_file(Path::new(&report.metadata.path))
        .expect("checked Go syntax metadata should lower to GrammarIR");
    lower_grammar_ir_to_structural_model(&grammar, go_syntax_source_file_token_layout())
        .expect("checked Go GrammarIR should lower to StructuralModel")
}

struct GoSourceInstanceStateApiReportSet {
    source_paths: Vec<String>,
    state_api_reports: Vec<SyntaxGeneratedCodecStateApiReport>,
    strict_recovery_reports: Vec<SyntaxGeneratedCodecStrictRecoveryDecodeReport>,
}

struct CheckedSourceInstanceReport {
    manifest_source: GoTokenizerGeneratorReportIngestionSource,
    report: SyntaxGoSourceInstanceProjectionReport,
}

fn current_plus_report_ingestion_sources() -> Vec<GoTokenizerGeneratorReportIngestionSource> {
    let ingestion_contents = fs::read_to_string(GO_CURRENT_PLUS_REPORT_INGESTION_MANIFEST_PATH)
        .expect("current-plus report-ingestion manifest should exist");
    go_tokenizer_generator_report_ingestion_sources(&ingestion_contents)
        .expect("current-plus report-ingestion manifest should parse")
}

fn checked_source_instance_report_set() -> Vec<CheckedSourceInstanceReport> {
    let manifest_sources = current_plus_report_ingestion_sources();
    let mut reports_by_source_path = BTreeMap::new();
    for entry in fs::read_dir(GO_SOURCE_INSTANCE_REPORT_DIR)
        .expect("Go source-instance report directory should be readable")
    {
        let entry = entry.expect("Go source-instance report entry should be readable");
        let path = entry.path();
        let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if !file_name.starts_with("source-instance-projection-report-")
            || !file_name.ends_with("-v1.json")
        {
            continue;
        }
        let contents =
            fs::read_to_string(&path).expect("checked source-instance report should be readable");
        let report =
            parse_checked_report(&contents).expect("checked source-instance report should parse");
        let key = (report.source.path.clone(), report.source.sha256.clone());
        if reports_by_source_path.insert(key, report).is_some() {
            panic!(
                "duplicate checked source-instance report for {}",
                path.display()
            );
        }
    }

    manifest_sources
        .into_iter()
        .map(|manifest_source| {
            let key = (
                manifest_source.source_path.clone(),
                manifest_source.source_sha256.clone(),
            );
            let report = reports_by_source_path.remove(&key).unwrap_or_else(|| {
                panic!(
                    "missing checked source-instance report for {}",
                    manifest_source.source_path
                )
            });
            assert_eq!(report.manifest_path, manifest_source.manifest_path);
            CheckedSourceInstanceReport {
                manifest_source,
                report,
            }
        })
        .collect()
}

fn go_source_instance_report_slug(source_path: &str) -> String {
    let relative = source_path
        .strip_prefix("tests/fixtures/go_syntax_corpus/src/")
        .expect("source-instance path should live under the Go corpus source root")
        .strip_suffix(".go")
        .expect("source-instance path should be a Go source file");
    let normalized = relative
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>();
    format!("go-source-instance-{normalized}")
}

fn go_state_api_report_path(source_path: &str) -> PathBuf {
    PathBuf::from(GO_STATE_API_REPORT_DIR).join(format!(
        "{}-state-api-report-v1.json",
        go_source_instance_report_slug(source_path)
    ))
}

fn go_strict_recovery_report_path(source_path: &str) -> PathBuf {
    PathBuf::from(GO_STATE_API_REPORT_DIR).join(format!(
        "{}-strict-recovery-decode-report-v1.json",
        go_source_instance_report_slug(source_path)
    ))
}

fn build_go_source_instance_state_api_report_set() -> GoSourceInstanceStateApiReportSet {
    let mut source_paths = Vec::new();
    let mut state_api_reports = Vec::new();
    let mut strict_recovery_reports = Vec::new();
    for checked_report in checked_source_instance_report_set() {
        let report = checked_report.report;
        let artifact_contents = regenerated_projection_artifact_for(&report);
        let artifact = parse_projection_artifact(&artifact_contents)
            .expect("projection artifact should parse");
        let raw_tokens = artifact
            .raw_token_ids_against_report(&report)
            .expect("projection artifact should emit raw token IDs");
        let generated_report = artifact
            .generated_codec_report_against_report(&report)
            .expect("projection artifact should emit generated-codec report");
        assert_eq!(
            generated_report.report_path, checked_report.manifest_source.report_path,
            "generated-codec report path should follow current-plus manifest order"
        );
        write_json(
            PathBuf::from(&generated_report.report_path),
            &generated_report,
        );

        let state_api_report = build_go_source_instance_generated_codec_state_api_report(
            &report,
            &raw_tokens,
            &generated_report,
        )
        .expect("Go generated-codec state API report should build");
        state_api_report
            .validate()
            .expect("Go generated-codec state API report should validate");

        let strict_recovery_report = build_go_source_instance_strict_recovery_decode_report(
            &report,
            &raw_tokens,
            &generated_report,
            &state_api_report,
        )
        .expect("Go strict/recovery decode report should build");
        strict_recovery_report
            .validate()
            .expect("Go strict/recovery decode report should validate");

        write_json(
            go_state_api_report_path(&report.source.path),
            &state_api_report,
        );
        write_json(
            go_strict_recovery_report_path(&report.source.path),
            &strict_recovery_report,
        );
        source_paths.push(checked_report.manifest_source.source_path);
        state_api_reports.push(state_api_report);
        strict_recovery_reports.push(strict_recovery_report);
    }

    GoSourceInstanceStateApiReportSet {
        source_paths,
        state_api_reports,
        strict_recovery_reports,
    }
}

/// Resolve the Go toolchain via the unified pin resolver.
fn go_bin() -> OsString {
    tct::go_toolchain::go_bin_os_string()
        .unwrap_or_else(|_| std::env::var_os("TCT_GO_BIN").unwrap_or_else(|| OsString::from("go")))
}

fn regenerated_projection_artifact_for(report: &SyntaxGoSourceInstanceProjectionReport) -> String {
    let tempdir = tempfile::tempdir().expect("projection tempdir should be created");
    let output_path = tempdir.path().join("projection.json");
    let output = Command::new(go_bin())
        .arg("run")
        .arg("scripts/analysis/go_syntax_source_instance_project.go")
        .arg("--source")
        .arg(&report.source.path)
        .arg("--metadata")
        .arg(&report.metadata.path)
        .arg("--manifest-path")
        .arg(&report.manifest_path)
        .arg("--candidate-grammar-id")
        .arg(&report.candidate_grammar_id)
        .arg("--projection-output")
        .arg(&output_path)
        .output()
        .expect("go source-instance projection helper should run");
    assert!(
        output.status.success(),
        "projection helper failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    fs::read_to_string(output_path).expect("projection artifact should be written")
}

fn write_json(path: impl AsRef<Path>, value: &impl serde::Serialize) {
    let path = path.as_ref();
    fs::create_dir_all(
        path.parent()
            .expect("report output path should have a parent directory"),
    )
    .expect("report output directory should be created");
    let serialized = serde_json::to_string_pretty(value).expect("report should serialize");
    fs::write(path, serialized).expect("report should be written");
}

#[allow(clippy::too_many_arguments)]
fn assert_csharp_whole_source_scratch_summary(
    parity_report: &SyntaxCSharpStateMachineParityReport,
    expected_scope: &str,
    expected_grammar_id: &str,
    expected_renderer_scope: &str,
    parser_command_fragment: &str,
    min_node_start_count: usize,
    min_payload_value_count: usize,
    raw_token_count_message: &str,
) {
    let selected_trace = &parity_report.selected_completion_trace;
    let scratch = parity_report
        .scratch_generation_summary
        .as_ref()
        .expect("C# whole-source parity should preserve scratch-generation summary evidence");
    assert_eq!(scratch.scope, expected_scope);
    assert_eq!(scratch.precompiled_artifact_grammar_id, expected_grammar_id);
    assert_eq!(scratch.empty_prefix_root, "CompilationUnitSyntax");
    assert_eq!(scratch.finite_admissible_raw_token_ids, vec![0]);
    assert!(scratch.selected_raw_token_admitted);
    assert!(scratch.selected_transition_accepted);
    assert_eq!(
        scratch.selected_completion_trace_scope,
        selected_trace.scope
    );
    assert_eq!(
        scratch.selected_completion_raw_token_count,
        parity_report.raw_token_count
    );
    assert_eq!(
        scratch.selected_completion_transition_count,
        parity_report.raw_token_count
    );
    assert!(scratch.selected_completion_every_token_admitted);
    assert_eq!(scratch.selected_completion_final_status, "complete");
    assert!(scratch.selected_completion_final_eof_valid);
    assert!(scratch.strict_incomplete_prefix_rejected);
    assert_eq!(scratch.strict_incomplete_prefix_error_kind, "premature_eof");
    assert!(scratch.token_only_decode_render_ready);
    assert!(
        scratch.token_only_raw_token_count > parity_report.raw_token_count,
        "{raw_token_count_message}"
    );
    assert_eq!(scratch.token_only_root_rule, "CompilationUnitSyntax");
    assert_eq!(scratch.token_only_root_node_kind, "CompilationUnitSyntax");
    assert!(scratch.token_only_node_start_count > min_node_start_count);
    assert!(scratch.token_only_payload_value_count > min_payload_value_count);
    assert_eq!(scratch.renderer_id, CSHARP_RAW_TREE_RENDERER_ID);
    assert_eq!(scratch.renderer_scope, expected_renderer_scope);
    assert_eq!(scratch.rendered_source_sha256, parity_report.source_sha256);
    assert!(scratch.rendered_source_matches_checked_source);
    assert_eq!(
        scratch.parser_sidecar_validation,
        "roslyn_self_test_required"
    );
    assert!(
        scratch
            .parser_sidecar_validation_command
            .contains(parser_command_fragment)
    );
    assert!(scratch.render_reparse_sidecar_blocker.is_none());
    assert!(!scratch.arena_ir_used);
    assert!(!scratch.route_handoff_provenance_used);
    assert!(!scratch.bpe_validity_authority_used);
    assert!(!scratch.stable_backend_claimed);
    assert_eq!(
        scratch.next_ready_task,
        CSHARP_SCRATCH_GENERATION_NEXT_READY_TASK
    );
}

fn csharp_real_source_generated_codec_corpus_gate() -> SyntaxCSharpRealSourceGeneratedCodecCorpusGate
{
    serde_json::from_str(CSHARP_REAL_SOURCE_GENERATED_CODEC_CORPUS_GATE)
        .expect("checked C# real-source generated-codec corpus gate should parse")
}

fn shared_generated_codec_state_api_handoff_gate() -> SyntaxGeneratedCodecStateApiHandoffGate {
    serde_json::from_str(SHARED_GENERATED_CODEC_STATE_API_HANDOFF_GATE)
        .expect("checked shared generated-codec state API handoff gate should parse")
}

fn generated_codec_state_api_integration_gate() -> SyntaxGeneratedCodecStateApiIntegrationGate {
    serde_json::from_str(GENERATED_CODEC_STATE_API_INTEGRATION_GATE)
        .expect("checked generated-codec state API integration gate should parse")
}

#[test]
#[ignore = "known failure: checked httptest-server projection report keeps per-file grammar id go_httptest_server_source_file_syntax_v0 without a per_file_grammar_blocker; see docs/frontends-development.md"]
fn raw_syntax_state_machine_accepts_checked_go_stream_without_segment_sidecars() {
    let report = checked_report();
    let sidecar = checked_go_source_instance_sidecar(&report);
    let artifact_contents = regenerated_projection_artifact_for(&report);
    let artifact =
        parse_projection_artifact(&artifact_contents).expect("projection artifact should parse");
    let raw_tokens = artifact
        .raw_token_ids_against_report(&report)
        .expect("projection artifact should emit raw token IDs");
    let model = CompiledSyntaxModel::from_sidecar_and_layout(&sidecar, &raw_tokens.token_layout)
        .expect("raw syntax state machine should compile from StructuralModel");
    let roles = model.token_roles();
    let core: &dyn RawSyntaxStateMachineCore = &model;
    assert_eq!(
        core.boundary_id(),
        RAW_SYNTAX_STATE_MACHINE_CORE_BOUNDARY_ID
    );
    assert_eq!(
        core.boundary_contract(),
        RAW_SYNTAX_STATE_MACHINE_CORE_BOUNDARY
    );

    let empty = core
        .state_after_raw_prefix(&[])
        .expect("empty raw prefix should produce the initial state");
    assert_eq!(empty.status, RawSyntaxStatus::Partial);
    assert_eq!(empty.consumed_raw_tokens, 0);
    assert_eq!(
        empty.admissible.finite_raw_token_ids,
        vec![roles.node_start]
    );

    let complete = core
        .strict_decode_complete(&raw_tokens.raw_token_ids)
        .expect("checked Go raw syntax stream should decode completely");
    assert_eq!(complete.status, RawSyntaxStatus::Complete);
    assert!(complete.eof_valid);
    assert_eq!(complete.consumed_raw_tokens, raw_tokens.raw_token_count);
    assert!(complete.admissible.finite_raw_token_ids.is_empty());

    let decoded_tree = core
        .decode_complete_tree(&raw_tokens.raw_token_ids)
        .expect("checked Go raw syntax stream should produce target-neutral tree events");
    assert_eq!(decoded_tree.state, complete);
    assert_eq!(
        decoded_tree.state.consumed_raw_tokens,
        raw_tokens.raw_token_count
    );
    assert_eq!(decoded_tree.root_rule(), Some("File"));
    assert_eq!(decoded_tree.root_node_kind(), Some("File"));
    assert!(decoded_tree.node_start_count() > 0);
    assert!(decoded_tree.field_start_count() > 0);
    assert!(decoded_tree.payload_event_count() > 0);
    assert!(decoded_tree.payload_token_byte_count() > 0);
    assert!(decoded_tree.payload_value_count() > 0);
    let first_event = decoded_tree
        .events
        .first()
        .expect("decoded tree should contain a root event");
    assert_eq!(first_event.event_kind, RawSyntaxDecodedEventKind::NodeStart);
    assert_eq!(first_event.raw_token_offset, 0);
    assert_eq!(first_event.rule.as_deref(), Some("File"));
    assert_eq!(first_event.node_kind.as_deref(), Some("File"));
    assert!(
        decoded_tree.events.iter().any(|event| {
            event.event_kind == RawSyntaxDecodedEventKind::FieldStart
                && event.rule.as_deref() == Some("File")
                && event.field.as_deref() == Some("Decls")
        }),
        "decoded tree events should preserve File.Decls field evidence"
    );
    assert!(
        decoded_tree.events.iter().any(|event| {
            matches!(
                event.event_kind,
                RawSyntaxDecodedEventKind::PayloadToken
                    | RawSyntaxDecodedEventKind::PayloadEnd
                    | RawSyntaxDecodedEventKind::BooleanPayload
            ) && event.payload_kind.is_some()
        }),
        "decoded tree events should preserve payload-kind evidence"
    );
    assert!(
        decoded_tree.events.iter().any(|event| {
            event.event_kind == RawSyntaxDecodedEventKind::PayloadToken
                && event.payload_token_byte.is_some()
        }),
        "decoded tree payload-token events should materialize raw payload bytes"
    );
    assert!(
        decoded_tree.events.iter().any(|event| {
            event.event_kind == RawSyntaxDecodedEventKind::PayloadEnd
                && event.payload_value.is_some()
        }),
        "decoded tree payload-end events should materialize completed payload values"
    );
    let decoded_node_tree = decoded_tree
        .decoded_node_tree()
        .expect("decoded tree events should materialize a nested node tree");
    assert_eq!(decoded_node_tree.rule_name, "File");
    assert_eq!(decoded_node_tree.node_kind, "File");
    let decls = decoded_node_tree
        .field("Decls")
        .expect("decoded node tree should preserve File.Decls");
    assert!(!decls.values.is_empty());
    assert!(decls.values.iter().all(|value| {
        matches!(
            value,
            RawSyntaxDecodedValue::Node(node)
                if matches!(node.node_kind.as_str(), "FuncDecl" | "GenDecl")
        )
    }));
    let comments = decoded_node_tree
        .field("Comments")
        .expect("decoded node tree should preserve File.Comments");
    let root_file_comment_group_count = comments
        .values
        .iter()
        .filter(|value| {
            matches!(
                value,
                RawSyntaxDecodedValue::Node(node) if node.node_kind == "CommentGroup"
            )
        })
        .count();
    assert!(root_file_comment_group_count > 0);
    assert!(
        decoded_node_tree.node_kind_count("CommentGroup") >= root_file_comment_group_count,
        "decoded raw tree should expose root comments plus any attached Doc/Comment groups"
    );
    let utf8_payload_segment = raw_tokens
        .raw_token_segments
        .iter()
        .find(|segment| {
            segment.event_kind == "terminal"
                && segment.primary_token_range_role == "utf8_byte_payload"
                && segment.raw_token_count >= 3
        })
        .expect("checked Go stream should contain a non-empty UTF-8 payload segment");
    let mut malformed_utf8 = raw_tokens.raw_token_ids.clone();
    malformed_utf8[utf8_payload_segment.raw_token_offset + 1] =
        roles.utf8_range.start + u32::from(0x80_u8);
    let malformed_error = core
        .decode_complete_tree(&malformed_utf8)
        .expect_err("decoded tree materialization should reject invalid UTF-8 payload bytes");
    assert_eq!(malformed_error.kind, "invalid_payload_continuation");
    let last_event = decoded_tree
        .events
        .last()
        .expect("decoded tree should contain a final event");
    assert_eq!(last_event.event_kind, RawSyntaxDecodedEventKind::NodeEnd);
    assert_eq!(last_event.rule.as_deref(), Some("File"));

    let decls_field_start = raw_tokens
        .raw_token_segments
        .iter()
        .find(|segment| {
            segment.event_kind == "field_start"
                && segment.owner_rule.as_deref() == Some("File")
                && segment.owner_field.as_deref() == Some("Decls")
        })
        .expect("checked Go stream should contain File.Decls field_start");
    let decls_prefix_len = decls_field_start.raw_token_offset + decls_field_start.raw_token_count;
    let decls_prefix = &raw_tokens.raw_token_ids[..decls_prefix_len];
    let decls_state = core
        .state_after_raw_prefix(decls_prefix)
        .expect("File.Decls raw prefix should be accepted without segment metadata");
    assert_eq!(decls_state.status, RawSyntaxStatus::Partial);
    assert_eq!(decls_state.active_rule.as_deref(), Some("File"));
    assert_eq!(decls_state.active_field.as_deref(), Some("Decls"));
    assert!(
        !decls_state
            .admissible
            .finite_raw_token_ids
            .contains(&roles.node_start),
        "multi-alternative File.Decls must not admit generic node_start"
    );
    assert!(
        decls_state
            .admissible
            .finite_raw_token_ids
            .iter()
            .any(|token| *token > roles.boolean_true),
        "multi-alternative File.Decls should advertise alternative-specific node_start tokens"
    );

    let actual_decls_child = raw_tokens.raw_token_ids[decls_prefix_len];
    let accepted_transition = core.candidate_raw_token_transition(decls_prefix, actual_decls_child);
    assert!(accepted_transition.accepted);
    assert_eq!(
        accepted_transition
            .state
            .as_ref()
            .expect("accepted transition should return a state")
            .consumed_raw_tokens,
        decls_prefix_len + 1
    );
    let generic_node_start_rejected =
        core.candidate_raw_token_transition(decls_prefix, roles.node_start);
    assert!(!generic_node_start_rejected.accepted);
    assert_eq!(
        generic_node_start_rejected
            .error
            .as_ref()
            .expect("rejected transition should return an error")
            .kind,
        "unexpected_structural_token"
    );

    let payload_segment = raw_tokens
        .raw_token_segments
        .iter()
        .find(|segment| {
            segment.event_kind == "terminal"
                && segment.raw_token_count > 2
                && !segment
                    .payload_kind
                    .as_deref()
                    .unwrap_or("")
                    .starts_with("boolean_payload:")
        })
        .expect("checked Go stream should contain a variable payload terminal");
    let partial_payload_len = payload_segment.raw_token_offset + 2;
    let payload_state = core
        .state_after_raw_prefix(&raw_tokens.raw_token_ids[..partial_payload_len])
        .expect("raw prefix inside a terminal payload should expose payload state");
    assert_eq!(payload_state.status, RawSyntaxStatus::Partial);
    assert_eq!(payload_state.active_payload_byte_count, 1);
    assert!(payload_state.admissible.payload_end_admissible);
    assert!(
        payload_state
            .admissible
            .finite_raw_token_ids
            .contains(&roles.payload_end)
    );
    assert!(!payload_state.admissible.raw_token_ranges.is_empty());

    let mut wrong_structural = raw_tokens.raw_token_ids.clone();
    wrong_structural[0] = roles.node_end;
    let wrong_structural_error = core
        .strict_decode_complete(&wrong_structural)
        .expect_err("wrong root token should be rejected from raw tokens alone");
    assert_eq!(wrong_structural_error.kind, "unexpected_structural_token");
    assert_eq!(wrong_structural_error.consumed_raw_tokens, 0);

    let missing_required = vec![roles.node_start, roles.node_end];
    let missing_required_error = core
        .strict_decode_complete(&missing_required)
        .expect_err("root closed before required fields should be rejected");
    assert_eq!(missing_required_error.kind, "missing_required_field");
    assert_eq!(missing_required_error.consumed_raw_tokens, 1);

    let mut invalid_payload = raw_tokens.raw_token_ids.clone();
    invalid_payload[payload_segment.raw_token_offset + 1] = roles.node_start;
    let invalid_payload_error = core
        .strict_decode_complete(&invalid_payload)
        .expect_err("payload byte outside the active raw range should be rejected");
    assert_eq!(invalid_payload_error.kind, "invalid_payload_token");
    assert_eq!(
        invalid_payload_error.consumed_raw_tokens,
        payload_segment.raw_token_offset + 1
    );

    let mut trailing = raw_tokens.raw_token_ids.clone();
    trailing.push(roles.node_start);
    let trailing_error = core
        .strict_decode_complete(&trailing)
        .expect_err("non-PAD surplus raw token should be rejected");
    assert_eq!(trailing_error.kind, "extra_trailing_token");
    assert_eq!(
        trailing_error.consumed_raw_tokens,
        raw_tokens.raw_token_count
    );

    let mut pad = raw_tokens.raw_token_ids.clone();
    pad[0] = roles.pad_token;
    let pad_error = core
        .strict_decode_complete(&pad)
        .expect_err("PAD_TOKEN must not appear in raw syntax streams");
    assert_eq!(pad_error.kind, "pad_token_in_raw_stream");

    let mut out_of_vocab = raw_tokens.raw_token_ids.clone();
    out_of_vocab[0] = roles.vocab_size;
    let out_of_vocab_error = core
        .strict_decode_complete(&out_of_vocab)
        .expect_err("raw token IDs must remain below VOCAB_SIZE");
    assert_eq!(out_of_vocab_error.kind, "token_id_out_of_vocab");
}

#[test]
#[ignore = "known failure: checked httptest-server projection report keeps per-file grammar id go_httptest_server_source_file_syntax_v0 without a per_file_grammar_blocker; see docs/frontends-development.md"]
fn go_source_instance_replay_accepts_stream_and_rejects_focused_mutations() {
    let report = checked_report();
    let artifact_contents = regenerated_projection_artifact_for(&report);
    let artifact =
        parse_projection_artifact(&artifact_contents).expect("projection artifact should parse");
    let raw_tokens = artifact
        .raw_token_ids_against_report(&report)
        .expect("projection artifact should emit raw token IDs");
    let generated_report = artifact
        .generated_codec_report_against_report(&report)
        .expect("projection artifact should emit generated-codec report");

    write_json(
        PathBuf::from(&generated_report.report_path),
        &generated_report,
    );
    let replay_report =
        build_go_source_instance_replay_report(&report, &raw_tokens, &generated_report)
            .expect("replay report should build from checked Go source-instance evidence");
    replay_report
        .validate()
        .expect("replay report should validate");
    write_json(REPLAY_REPORT_PATH, &replay_report);

    assert_eq!(replay_report.complete_stream.observed_status, "complete");
    assert_eq!(replay_report.complete_stream.consumed_raw_tokens, 21613);
    assert_eq!(replay_report.prefix_cases.len(), 2);
    assert!(replay_report.prefix_cases.iter().all(|case| case.accepted));
    assert!(replay_report.prefix_inspection_state_api_ready);
    assert_eq!(
        replay_report.prefix_inspection_state_api_source,
        REPLAY_PREFIX_INSPECTION_STATE_API_SOURCE
    );
    assert!(
        replay_report.prefix_cases.iter().all(|case| case
            .diagnostic
            .contains("generated-codec state API prefix inspection")),
        "replay prefix cases should consume the generated-codec state API"
    );
    assert!(
        replay_report
            .next_missing_workstream_blocker
            .contains("strict decode")
    );
    assert!(
        replay_report
            .no_stable_claims
            .iter()
            .any(|claim| claim == "no_exact_mask_state_api_wiring_claim")
    );
    assert!(replay_report.raw_token_self_sufficient_for_generated_codec);
    assert!(
        replay_report.generated_codec_promotion_blockers.is_empty(),
        "tokenized alternative identity should remove the generated-codec promotion blocker"
    );

    let rejected = replay_report
        .invalid_mutation_cases
        .iter()
        .filter_map(|case| case.error_kind.as_deref())
        .collect::<std::collections::BTreeSet<_>>();
    for expected in [
        "unexpected_structural_token",
        "missing_required_field",
        "repeated_field_cardinality",
        "invalid_payload_token",
        "premature_eof",
        "extra_trailing_token",
        "pad_token_in_raw_stream",
        "token_id_out_of_vocab",
    ] {
        assert!(
            rejected.contains(expected),
            "replay report should reject or block {expected}"
        );
    }

    let state_api_report = build_go_source_instance_generated_codec_state_api_report(
        &report,
        &raw_tokens,
        &generated_report,
    )
    .expect("generated-codec state API report should build from replay evidence");
    state_api_report
        .validate()
        .expect("generated-codec state API report should validate");
    write_json(STATE_API_REPORT_PATH, &state_api_report);

    let empty = state_api_report
        .state_cases
        .iter()
        .find(|case| case.case_id == "empty_prefix")
        .expect("state API should include empty-prefix state");
    assert_eq!(empty.admissible_next_raw_token_ids, vec![0]);
    assert!(!empty.eof_valid);

    let decls = state_api_report
        .state_cases
        .iter()
        .find(|case| case.case_id == "after_file_decls_field_start")
        .expect("state API should include File.Decls prefix state");
    assert_eq!(decls.active_rule.as_deref(), Some("File"));
    assert_eq!(decls.active_field.as_deref(), Some("Decls"));
    assert!(
        !decls.admissible_next_raw_token_ids.contains(&0),
        "multi-alternative File.Decls must not admit generic node_start"
    );
    assert!(
        decls
            .admissible_next_raw_token_ids
            .iter()
            .any(|token| *token > 9),
        "multi-alternative File.Decls should advertise alternative-specific node_start tokens"
    );
    assert!(state_api_report.payload_range_state_api_ready);
    let payload = state_api_report
        .state_cases
        .iter()
        .find(|case| case.case_id == "inside_first_terminal_payload_after_one_byte")
        .expect("state API should include a terminal payload-range prefix state");
    assert_eq!(
        payload.prefix_kind,
        "go_source_instance_partial_terminal_payload_prefix"
    );
    assert_eq!(payload.observed_status, "partial");
    assert_eq!(payload.active_payload_byte_count, 1);
    assert!(payload.payload_end_admissible);
    assert!(payload.admissible_next_raw_token_ids.contains(&7));
    assert_eq!(payload.admissible_next_raw_token_ranges.len(), 1);
    assert!(matches!(
        payload.admissible_next_raw_token_ranges[0].role.as_str(),
        "utf8_byte_payload" | "integer_lexeme_payload"
    ));
    assert!(
        payload
            .active_payload_kind
            .as_deref()
            .is_some_and(|kind| !kind.is_empty())
    );

    let accepted = state_api_report
        .transition_cases
        .iter()
        .find(|case| case.case_id == "file_decls_actual_alternative_node_start_is_accepted")
        .expect("state API should probe accepted File.Decls transition");
    assert!(accepted.accepted);
    assert_eq!(accepted.observed_status, "partial");

    let rejected = state_api_report
        .transition_cases
        .iter()
        .find(|case| case.case_id == "file_decls_generic_node_start_is_rejected")
        .expect("state API should probe rejected File.Decls transition");
    assert!(!rejected.accepted);
    assert_eq!(
        rejected.error_kind.as_deref(),
        Some("unexpected_structural_token")
    );

    let complete = state_api_report
        .state_cases
        .iter()
        .find(|case| case.case_id == "complete_stream")
        .expect("state API should include complete-stream state");
    assert!(complete.eof_valid);
    assert!(complete.admissible_next_raw_token_ids.is_empty());
    assert!(state_api_report.strict_recovery_decode_wiring_ready);
    assert_eq!(
        state_api_report.next_missing_workstream_blocker,
        GENERATED_CODEC_STATE_API_SUCCESSOR_NEXT_TASK
    );

    let strict_recovery_report = build_go_source_instance_strict_recovery_decode_report(
        &report,
        &raw_tokens,
        &generated_report,
        &state_api_report,
    )
    .expect("strict/recovery decode report should build from state API evidence");
    strict_recovery_report
        .validate()
        .expect("strict/recovery decode report should validate");
    write_json(STRICT_RECOVERY_REPORT_PATH, &strict_recovery_report);

    assert!(strict_recovery_report.state_api_consumed);
    assert!(strict_recovery_report.strict_recovery_decode_wiring_ready);
    assert!(strict_recovery_report.complete_stream_eof_valid);
    assert!(strict_recovery_report.trailing_pad_recovery_ready);
    assert!(
        strict_recovery_report
            .strict_cases
            .iter()
            .any(|case| case.case_id == "strict_partial_prefix_rejects_eof"
                && case.error_kind.as_deref() == Some("premature_eof"))
    );
    assert!(
        strict_recovery_report
            .strict_cases
            .iter()
            .any(
                |case| case.case_id == "strict_trailing_pad_rejects_raw_stream"
                    && case.error_kind.as_deref() == Some("pad_token_in_raw_stream")
            )
    );
    let trailing_pad_recovery = strict_recovery_report
        .recovery_cases
        .iter()
        .find(|case| case.case_id == "recovery_trailing_pad_normalizes_after_state_api_complete")
        .expect("strict/recovery report should include trailing PAD recovery case");
    assert_eq!(
        trailing_pad_recovery.observed_outcome,
        "normalized_trailing_pad"
    );
    assert_eq!(trailing_pad_recovery.ignored_trailing_tokens, 1);
    assert_eq!(
        trailing_pad_recovery.consumed_raw_tokens,
        raw_tokens.raw_token_count
    );
    assert_eq!(
        strict_recovery_report.next_missing_workstream_blocker,
        GENERATED_CODEC_STATE_API_SUCCESSOR_NEXT_TASK
    );
}

#[test]
fn go_ast_declaration_presence_replay_consumes_inventory_boolean_presence_decisions() {
    let report = build_go_ast_declaration_presence_replay_report()
        .expect("Go AST declaration presence replay report should build");
    report
        .validate()
        .expect("Go AST declaration presence replay report should validate");
    write_json(GO_AST_DECLARATION_PRESENCE_REPLAY_REPORT_PATH, &report);

    assert_eq!(
        report.artifact_kind,
        GO_AST_DECLARATION_PRESENCE_REPLAY_ARTIFACT_KIND
    );
    assert_eq!(
        report.next_ready_task,
        GO_AST_DECLARATION_PRESENCE_REPLAY_NEXT_READY_TASK
    );
    assert_eq!(report.complete_stream.observed_status, "complete");
    assert!(report.raw_state_machine_complete);
    assert!(report.declaration_presence_decisions_consumed);
    assert!(report.state_after_terminal_admissible_boolean_ready);
    assert!(report.inventory_backed_structural_model_used);
    assert!(!report.arena_ir_used);
    assert!(!report.bpe_layer_defines_validity);
    assert!(!report.stable_backend_promotion_claimed);
    assert_eq!(report.presence_decisions.len(), 5);

    let decisions = report
        .presence_decisions
        .iter()
        .map(|decision| {
            (
                format!("{}.{}", decision.owner_rule, decision.owner_field),
                decision,
            )
        })
        .collect::<BTreeMap<_, _>>();
    for (field, selected_node_kind) in [
        (
            "GenDecl.Lparen",
            "go_syntax_presence_gen_decl_group.PRESENT",
        ),
        (
            "GenDecl.Rparen",
            "go_syntax_presence_gen_decl_group.PRESENT",
        ),
        (
            "TypeSpec.Assign",
            "go_syntax_presence_type_spec_alias.PRESENT",
        ),
        ("FuncType.Func", "go_syntax_presence_func_type_func.PRESENT"),
        (
            "CallExpr.Ellipsis",
            "go_syntax_presence_call_expr_ellipsis.PRESENT",
        ),
    ] {
        let decision = decisions
            .get(field)
            .unwrap_or_else(|| panic!("missing declaration presence decision for {field}"));
        assert_eq!(decision.selected_node_kind, selected_node_kind);
        assert_eq!(decision.boolean_payload_kind, "boolean");
        assert_eq!(decision.boolean_payload_raw_token_id, 9);
        assert_eq!(
            decision.state_after_terminal_finite_admissible_raw_token_ids,
            vec![8, 9]
        );
        assert!(decision.terminal_event_decoded);
        assert!(decision.boolean_payload_decoded);
        assert!(decision.state_after_terminal_admits_false);
        assert!(decision.state_after_terminal_admits_true);
    }

    assert_eq!(
        report
            .invalid_boolean_payload_mutation
            .error_kind
            .as_deref(),
        Some("invalid_payload_token")
    );
    assert!(
        report
            .claim_boundary
            .contains("not a full Go source validity-by-construction claim")
    );
    assert!(report.claim_boundary.contains("not arena-IR"));
    assert!(report.claim_boundary.contains("not BPE"));
    assert!(
        report
            .claim_boundary
            .contains("not stable-backend promotion")
    );
}

#[test]
fn csharp_minimal_class_scratch_seed_uses_admissible_raw_state_machine_transitions() {
    let parity_report = build_csharp_state_machine_parity_report(
        "csharp_minimal_class_syntax_v0",
        Path::new(CSHARP_METADATA_PATH),
    )
    .expect("C# minimal-class scratch-seed parity report should build from checked metadata");
    parity_report
        .validate()
        .expect("C# minimal-class scratch-seed parity report should validate");
    write_json(
        CSHARP_MINIMAL_CLASS_SCRATCH_SEED_PARITY_REPORT_PATH,
        &parity_report,
    );

    assert_eq!(parity_report.target, "csharp");
    assert_eq!(
        parity_report.candidate_grammar_id,
        "csharp_minimal_class_syntax_v0"
    );
    assert_eq!(parity_report.metadata_path, CSHARP_METADATA_PATH);
    assert_eq!(
        parity_report.cross_target_parity_claim_scope,
        CSHARP_STATE_MACHINE_PARITY_CLAIM_SCOPE
    );
    assert!(parity_report.target_metadata_sidecar_only);
    assert!(!parity_report.arena_ir_used);
    assert!(!parity_report.bpe_layer_defines_validity);
    assert!(parity_report.state_after_prefix_ready);
    assert!(parity_report.admissible_next_raw_tokens_ready);
    assert!(parity_report.strict_recovery_decode_agreement_ready);
    assert!(parity_report.consumed_token_accounting_ok);
    let selected_trace = &parity_report.selected_completion_trace;
    let scratch = parity_report
        .scratch_generation_summary
        .as_ref()
        .expect("C# minimal-class parity should preserve scratch-generation summary evidence");
    assert_eq!(
        scratch.status,
        "csharp_empty_prefix_scratch_generation_parity_ready"
    );
    assert_eq!(
        scratch.precompiled_artifact_grammar_id,
        parity_report.candidate_grammar_id
    );
    assert_eq!(scratch.empty_prefix_root, "CompilationUnitSyntax");
    assert_eq!(scratch.finite_admissible_raw_token_ids, vec![0]);
    assert!(scratch.selected_raw_token_admitted);
    assert!(scratch.selected_transition_accepted);
    assert_eq!(
        scratch.selected_completion_trace_scope,
        selected_trace.scope
    );
    assert_eq!(
        scratch.selected_completion_raw_token_count,
        parity_report.raw_token_count
    );
    assert!(scratch.strict_incomplete_prefix_rejected);
    assert_eq!(scratch.strict_incomplete_prefix_error_kind, "premature_eof");
    assert!(scratch.token_only_decode_render_ready);
    assert_eq!(scratch.token_only_root_rule, "CompilationUnitSyntax");
    assert_eq!(scratch.token_only_root_node_kind, "CompilationUnitSyntax");
    assert_eq!(scratch.renderer_id, CSHARP_RAW_TREE_RENDERER_ID);
    assert_eq!(scratch.rendered_source_sha256, parity_report.source_sha256);
    assert!(scratch.rendered_source_matches_checked_source);
    assert_eq!(
        scratch.parser_sidecar_validation,
        "roslyn_self_test_required"
    );
    assert!(
        scratch
            .parser_sidecar_validation_command
            .contains("csharp_syntax_metadata_smoke.py --fixture minimal-class --self-test")
    );
    assert!(!scratch.arena_ir_used);
    assert!(!scratch.route_handoff_provenance_used);
    assert!(!scratch.bpe_validity_authority_used);
    assert!(!scratch.stable_backend_claimed);

    let root = parity_report
        .state_cases
        .iter()
        .find(|case| case.case_id == "csharp_empty_prefix")
        .expect("C# minimal-class parity should expose the empty-prefix state");
    assert_eq!(root.prefix_raw_token_count, 0);
    assert_eq!(root.admissible_next_raw_token_ids, vec![0]);
    assert!(!root.eof_valid);

    assert_eq!(selected_trace.scope, CSHARP_SELECTED_COMPLETION_TRACE_SCOPE);
    assert_eq!(
        selected_trace.selection_loop_source,
        SELECTED_COMPLETION_TRACE_SOURCE
    );
    assert_eq!(
        selected_trace.raw_token_count,
        parity_report.raw_token_count
    );
    assert_eq!(
        selected_trace.transition_count,
        parity_report.raw_token_count
    );
    assert_eq!(
        selected_trace.admissible_mask_query_count,
        parity_report.raw_token_count
    );
    assert!(selected_trace.finite_admissible_raw_token_mask_available);
    assert!(selected_trace.every_selected_raw_token_admitted);
    assert_eq!(
        selected_trace.selected_transition_accepted_count,
        parity_report.raw_token_count
    );
    assert_eq!(selected_trace.first_selected_raw_token_id, 0);
    assert_eq!(selected_trace.final_state_observed_status, "complete");
    assert_eq!(
        selected_trace.final_state_consumed_raw_tokens,
        parity_report.raw_token_count
    );
    assert!(selected_trace.final_state_eof_valid);
    assert_eq!(selected_trace.final_state_stack_depth, 0);
    assert_eq!(
        selected_trace.strict_decode_agreement_case_id,
        parity_report.complete_stream.case_id
    );
    assert_eq!(
        selected_trace.strict_decode_agreement_observed_status,
        "complete"
    );
    assert!(selected_trace.strict_decode_agreement_eof_valid);
    assert!(selected_trace.step_trace_sample.iter().all(
        |sample| sample.selected_raw_token_admitted
            && sample.selected_transition_accepted
            && sample.selected_raw_token_id < parity_report.token_layout.vocab_size
    ));
    assert!(selected_trace.step_trace_sample.iter().any(|sample| {
        sample.transition_observed_status == "complete" && sample.transition_eof_valid
    }));

    let complete = parity_report
        .state_cases
        .iter()
        .find(|case| case.case_id == "csharp_complete_stream")
        .expect("C# minimal-class parity should expose the complete state");
    assert!(complete.eof_valid);
    assert!(complete.admissible_next_raw_token_ids.is_empty());
    assert_eq!(parity_report.complete_stream.observed_status, "complete");
    assert_eq!(
        parity_report.complete_stream.consumed_raw_tokens,
        parity_report.raw_token_count
    );

    let grammar = CSharpSyntaxMetadataAdapter::new("csharp_minimal_class_syntax_v0")
        .lower_metadata_file(Path::new(CSHARP_METADATA_PATH))
        .expect("checked C# minimal-class metadata should lower to GrammarIR");
    let sidecar = lower_grammar_ir_to_structural_model(&grammar, csharp_syntax_token_layout())
        .expect("checked C# minimal-class GrammarIR should lower to StructuralModel");
    let raw_stream = build_csharp_minimal_class_raw_tree_render_token_stream_from_sidecar(&sidecar)
        .expect("C# minimal-class render replay should synthesize fixture source raw tokens");
    assert!(
        raw_stream.raw_token_count > parity_report.raw_token_count,
        "render replay should materialize fixture payload bytes while parity replay keeps short synthetic payloads"
    );
    let core = CompiledSyntaxModel::from_sidecar(&sidecar)
        .expect("checked C# minimal-class StructuralModel should compile to raw state machine");
    let shared_empty_prefix =
        RawScratchEmptyPrefixSummary::from_core(&core, "test.csharp_shared_empty_prefix")
            .expect("shared scratch helper should summarize the C# empty-prefix state");
    assert_eq!(
        shared_empty_prefix.finite_admissible_raw_token_ids,
        scratch.finite_admissible_raw_token_ids
    );
    assert_eq!(
        shared_empty_prefix
            .selected_transition_state
            .observed_status,
        scratch.selected_transition_observed_status
    );
    let shared_claim_flags = RawScratchClaimBoundaryFlags::raw_state_machine_only();
    assert!(!shared_claim_flags.source_prefix_table_used);
    assert!(!shared_claim_flags.arena_ir_used);
    assert!(!shared_claim_flags.route_handoff_provenance_used);
    assert!(!shared_claim_flags.bpe_validity_authority_used);
    assert!(!shared_claim_flags.stable_backend_claimed);
    let fixture_trace = core
        .selected_completion_trace(&raw_stream.raw_token_ids, &BTreeSet::new())
        .expect(
            "C# minimal-class fixture render stream should replay through selected transitions",
        );
    assert_eq!(fixture_trace.transition_count, raw_stream.raw_token_count);
    assert_eq!(
        fixture_trace.selected_transition_accepted_count,
        raw_stream.raw_token_count
    );
    assert!(fixture_trace.every_selected_raw_token_admitted);
    assert!(fixture_trace.final_state.eof_valid);
    let decoded_tree = core
        .decode_complete_tree(&raw_stream.raw_token_ids)
        .expect("C# minimal-class raw token stream should materialize a raw decoded tree");
    assert_eq!(decoded_tree.root_rule(), Some("CompilationUnitSyntax"));
    assert_eq!(decoded_tree.root_node_kind(), Some("CompilationUnitSyntax"));
    assert!(decoded_tree.node_start_count() >= 5);
    assert!(decoded_tree.payload_value_count() >= 5);
    let decoded_node_tree = decoded_tree
        .decoded_node_tree()
        .expect("C# minimal-class decoded events should materialize a nested node tree");
    assert_eq!(decoded_node_tree.node_kind, "CompilationUnitSyntax");
    for expected in [
        "FileScopedNamespaceDeclarationSyntax",
        "ClassDeclarationSyntax",
        "PropertyDeclarationSyntax",
        "PredefinedTypeSyntax",
    ] {
        assert_eq!(
            decoded_node_tree.node_kind_count(expected),
            1,
            "decoded C# raw tree should contain one {expected}"
        );
    }

    let rendered = render_csharp_minimal_class_source_from_raw_tree(&decoded_tree)
        .expect("C# minimal-class raw decoded tree should render source");
    assert_eq!(rendered.renderer_id, CSHARP_RAW_TREE_RENDERER_ID);
    assert_eq!(
        rendered.supported_construct,
        "minimal_file_scoped_namespace_class_string_property"
    );
    assert_eq!(
        rendered.source,
        CSHARP_MINIMAL_CLASS_RAW_TREE_RENDERED_SOURCE
    );
    assert_eq!(
        rendered.source,
        fs::read_to_string("tests/fixtures/csharp_syntax_sources/minimal_class/source.cs")
            .expect("checked C# minimal-class source fixture should be readable"),
        "rendered C# raw tree should match the Roslyn-checked minimal-class source bytes"
    );
}

#[test]
fn csharp_local_integer_return_scratch_seed_records_function_return_token_only_seed() {
    let parity_report = build_csharp_state_machine_parity_report(
        CSHARP_LOCAL_INTEGER_RETURN_LITERAL_GRAMMAR_ID,
        Path::new(CSHARP_LOCAL_INTEGER_RETURN_LITERAL_METADATA_PATH),
    )
    .expect("C# integer-return scratch-seed parity report should build from checked metadata");
    parity_report
        .validate()
        .expect("C# integer-return scratch-seed parity report should validate");
    write_json(
        CSHARP_LOCAL_INTEGER_RETURN_LITERAL_SCRATCH_SEED_PARITY_REPORT_PATH,
        &parity_report,
    );

    assert_eq!(parity_report.target, "csharp");
    assert_eq!(
        parity_report.candidate_grammar_id,
        CSHARP_LOCAL_INTEGER_RETURN_LITERAL_GRAMMAR_ID
    );
    assert_eq!(
        parity_report.metadata_path,
        CSHARP_LOCAL_INTEGER_RETURN_LITERAL_METADATA_PATH
    );
    assert_eq!(
        parity_report.next_ready_task,
        CSHARP_STATE_MACHINE_PARITY_NEXT_READY_TASK
    );
    assert!(parity_report.state_after_prefix_ready);
    assert!(parity_report.admissible_next_raw_tokens_ready);
    assert!(parity_report.strict_recovery_decode_agreement_ready);
    assert!(parity_report.consumed_token_accounting_ok);
    assert!(parity_report.target_metadata_sidecar_only);
    assert!(!parity_report.arena_ir_used);
    assert!(!parity_report.bpe_layer_defines_validity);
    assert_eq!(
        parity_report.next_ready_task,
        CSHARP_STATE_MACHINE_PARITY_NEXT_READY_TASK
    );
    assert_ne!(
        parity_report.next_ready_task,
        STALE_SOURCE_PROGRAM_SUCCESSOR_TASK
    );
    assert!(
        parity_report
            .next_ready_task
            .contains("Go Source-Record Raw-Token Generator Core")
    );

    let selected_trace = &parity_report.selected_completion_trace;
    assert_eq!(selected_trace.scope, CSHARP_SELECTED_COMPLETION_TRACE_SCOPE);
    assert_eq!(
        selected_trace.selection_loop_source,
        SELECTED_COMPLETION_TRACE_SOURCE
    );
    assert_eq!(
        selected_trace.transition_count,
        parity_report.raw_token_count
    );
    assert_eq!(
        selected_trace.selected_transition_accepted_count,
        parity_report.raw_token_count
    );
    assert!(selected_trace.every_selected_raw_token_admitted);
    assert_eq!(selected_trace.first_selected_raw_token_id, 0);
    assert_eq!(selected_trace.final_state_observed_status, "complete");
    assert!(selected_trace.final_state_eof_valid);
    assert_eq!(selected_trace.final_state_stack_depth, 0);
    assert!(selected_trace.step_trace_sample.iter().all(|sample| {
        sample.selected_raw_token_admitted
            && sample.selected_transition_accepted
            && sample.selected_raw_token_id < parity_report.token_layout.vocab_size
    }));

    let scratch = parity_report
        .scratch_generation_summary
        .as_ref()
        .expect("C# integer-return parity should preserve scratch-generation summary evidence");
    assert_eq!(
        scratch.scope,
        CSHARP_LOCAL_INTEGER_RETURN_LITERAL_SCRATCH_GENERATION_SCOPE
    );
    assert_eq!(
        scratch.precompiled_artifact_grammar_id,
        CSHARP_LOCAL_INTEGER_RETURN_LITERAL_GRAMMAR_ID
    );
    assert_eq!(scratch.empty_prefix_root, "CompilationUnitSyntax");
    assert_eq!(scratch.finite_admissible_raw_token_ids, vec![0]);
    assert!(scratch.selected_raw_token_admitted);
    assert!(scratch.selected_transition_accepted);
    assert_eq!(
        scratch.selected_completion_raw_token_count,
        parity_report.raw_token_count
    );
    assert_eq!(
        scratch.selected_completion_transition_count,
        parity_report.raw_token_count
    );
    assert_eq!(
        scratch.selected_completion_admissible_mask_query_count,
        parity_report.raw_token_count
    );
    assert!(scratch.selected_completion_every_token_admitted);
    assert_eq!(scratch.selected_completion_final_status, "complete");
    assert!(scratch.selected_completion_final_eof_valid);
    assert!(scratch.strict_incomplete_prefix_rejected);
    assert_eq!(scratch.strict_incomplete_prefix_error_kind, "premature_eof");
    assert!(scratch.token_only_decode_render_ready);
    assert_eq!(
        scratch.token_only_raw_token_count,
        parity_report.raw_token_count
    );
    assert_eq!(scratch.token_only_root_rule, "CompilationUnitSyntax");
    assert_eq!(scratch.token_only_root_node_kind, "CompilationUnitSyntax");
    assert!(scratch.token_only_node_start_count >= 8);
    assert_eq!(scratch.token_only_payload_value_count, 5);
    assert_eq!(scratch.renderer_id, CSHARP_RAW_TREE_RENDERER_ID);
    assert_eq!(
        scratch.renderer_scope,
        CSHARP_LOCAL_INTEGER_RETURN_LITERAL_RAW_TREE_RENDERER_SCOPE
    );
    assert_eq!(scratch.rendered_source_sha256, parity_report.source_sha256);
    assert!(scratch.rendered_source_matches_checked_source);
    assert_eq!(
        scratch.parser_sidecar_validation,
        "roslyn_self_test_required"
    );
    assert!(scratch.parser_sidecar_validation_command.contains(
        "csharp_syntax_metadata_smoke.py --fixture local-integer-return-literal --self-test"
    ));
    assert!(scratch.render_reparse_sidecar_blocker.is_none());
    assert!(!scratch.source_prefix_table_used);
    assert!(!scratch.arena_ir_used);
    assert!(!scratch.route_handoff_provenance_used);
    assert!(!scratch.bpe_validity_authority_used);
    assert!(!scratch.stable_backend_claimed);

    let grammar = CSharpSyntaxMetadataAdapter::new(CSHARP_LOCAL_INTEGER_RETURN_LITERAL_GRAMMAR_ID)
        .lower_metadata_file(Path::new(CSHARP_LOCAL_INTEGER_RETURN_LITERAL_METADATA_PATH))
        .expect("checked C# integer-return metadata should lower to GrammarIR");
    let sidecar = lower_grammar_ir_to_structural_model(&grammar, csharp_syntax_token_layout())
        .expect("checked C# integer-return GrammarIR should lower to StructuralModel");
    let raw_stream =
        build_csharp_local_integer_return_literal_raw_tree_render_token_stream_from_sidecar(
            &sidecar,
        )
        .expect("C# integer-return replay should synthesize fixture source raw tokens");
    assert_eq!(raw_stream.raw_token_count, parity_report.raw_token_count);
    let core = CompiledSyntaxModel::from_sidecar(&sidecar)
        .expect("checked C# integer-return StructuralModel should compile to raw state machine");
    let fixture_trace = core
        .selected_completion_trace(&raw_stream.raw_token_ids, &BTreeSet::new())
        .expect("C# integer-return fixture stream should replay through selected transitions");
    assert_eq!(fixture_trace.transition_count, raw_stream.raw_token_count);
    assert_eq!(
        fixture_trace.selected_transition_accepted_count,
        raw_stream.raw_token_count
    );
    assert!(fixture_trace.every_selected_raw_token_admitted);
    assert!(fixture_trace.final_state.eof_valid);
    let decoded_tree = core
        .decode_complete_tree(&raw_stream.raw_token_ids)
        .expect("C# integer-return raw token stream should materialize a raw decoded tree");
    assert_eq!(decoded_tree.root_rule(), Some("CompilationUnitSyntax"));
    assert_eq!(decoded_tree.root_node_kind(), Some("CompilationUnitSyntax"));
    assert_eq!(decoded_tree.payload_value_count(), 5);
    let decoded_node_tree = decoded_tree
        .decoded_node_tree()
        .expect("C# integer-return decoded events should materialize a nested node tree");
    for expected in [
        "FileScopedNamespaceDeclarationSyntax",
        "ClassDeclarationSyntax",
        "MethodDeclarationSyntax",
        "PredefinedTypeSyntax",
        "BlockSyntax",
        "ReturnStatementSyntax",
        "LiteralExpressionSyntax",
    ] {
        assert_eq!(
            decoded_node_tree.node_kind_count(expected),
            1,
            "decoded C# function-return raw tree should contain one {expected}"
        );
    }

    let rendered = render_csharp_local_integer_return_literal_from_raw_tree(&decoded_tree)
        .expect("C# integer-return raw decoded tree should render source");
    assert_eq!(rendered.renderer_id, CSHARP_RAW_TREE_RENDERER_ID);
    assert_eq!(rendered.supported_construct, "local_integer_return_literal");
    assert_eq!(
        rendered.source,
        CSHARP_LOCAL_INTEGER_RETURN_LITERAL_RAW_TREE_RENDERED_SOURCE
    );
    assert_eq!(
        rendered.source,
        fs::read_to_string(
            "tests/fixtures/csharp_syntax_sources/local_integer_return_literal/source.cs"
        )
        .expect("checked C# integer-return source fixture should be readable"),
        "rendered C# raw tree should match the Roslyn-checked integer-return source bytes"
    );
}

/// Bounded u8 integer payloads must not inherit UTF-8 payload_end gating:
/// a single high byte (value >= 128) is a complete integer value, so
/// `payload_end` must be mask-admissible and materialize the decimal string.
#[test]
fn u8_integer_payload_high_byte_round_trips_without_utf8_coupling() {
    let grammar = CSharpSyntaxMetadataAdapter::new(CSHARP_LOCAL_INTEGER_RETURN_LITERAL_GRAMMAR_ID)
        .lower_metadata_file(Path::new(CSHARP_LOCAL_INTEGER_RETURN_LITERAL_METADATA_PATH))
        .expect("checked C# integer-return metadata should lower to GrammarIR");
    let sidecar = lower_grammar_ir_to_structural_model(&grammar, csharp_syntax_token_layout())
        .expect("checked C# integer-return GrammarIR should lower to StructuralModel");
    let raw_stream =
        build_csharp_local_integer_return_literal_raw_tree_render_token_stream_from_sidecar(
            &sidecar,
        )
        .expect("C# integer-return replay should synthesize fixture source raw tokens");
    let core = CompiledSyntaxModel::from_sidecar(&sidecar)
        .expect("checked C# integer-return StructuralModel should compile to raw state machine");
    let roles = core.token_roles();
    assert_eq!(roles.integer_range.role, "u8_integer_payload");

    let fixture_value: u8 = 7;
    let high_value: u8 = 200;
    let fixture_integer_token = roles.integer_range.start + u32::from(fixture_value);
    let high_integer_token = roles.integer_range.start + u32::from(high_value);

    let integer_token_index = raw_stream
        .raw_token_ids
        .iter()
        .position(|&token| token == fixture_integer_token)
        .expect("fixture stream should encode the checked integer literal as a u8 payload byte");

    let mut high_stream = raw_stream.raw_token_ids.clone();
    high_stream[integer_token_index] = high_integer_token;

    let after_byte_prefix = &high_stream[..=integer_token_index];
    let after_byte_state = core
        .state_from_prefix(after_byte_prefix)
        .expect("prefix ending on a complete u8 integer byte should replay");
    assert!(
        after_byte_state.admissible.payload_end_admissible
            && after_byte_state
                .admissible
                .finite_raw_token_ids
                .contains(&roles.payload_end),
        "payload_end must be mask-admissible after a complete u8 integer byte, including values >= 128"
    );
    assert!(
        after_byte_state.admissible.raw_token_ranges.is_empty(),
        "u8 integer payloads are fixed-width: after the value byte, only payload_end is admissible"
    );
    let payload_end_step = core.transition(after_byte_prefix, roles.payload_end);
    assert!(
        payload_end_step.accepted,
        "transition must accept payload_end after a complete high-byte u8 integer payload"
    );

    let empty_integer_prefix = &high_stream[..integer_token_index];
    let empty_integer_state = core
        .state_from_prefix(empty_integer_prefix)
        .expect("prefix parked on an open u8 integer payload should replay");
    assert!(
        !empty_integer_state.admissible.payload_end_admissible
            && !empty_integer_state
                .admissible
                .finite_raw_token_ids
                .contains(&roles.payload_end),
        "empty u8 integer payloads must not admit payload_end"
    );

    let decoded_tree = core
        .decode_complete_tree(&high_stream)
        .expect("high-byte u8 integer payload stream should strict-decode without UTF-8 rejection");
    let high_payload_values: Vec<&str> = decoded_tree
        .events
        .iter()
        .filter(|event| event.event_kind == RawSyntaxDecodedEventKind::PayloadEnd)
        .filter_map(|event| event.payload_value.as_deref())
        .collect();
    assert!(
        high_payload_values.contains(&"200"),
        "u8 integer payload_end must materialize the decimal string of the value byte, got {high_payload_values:?}"
    );

    // Fixture-locked render validators expect the checked literal (7); the
    // raw decoded tree is the authority for high-byte materialization.
    let decoded_node_tree = decoded_tree
        .decoded_node_tree()
        .expect("high-byte stream should materialize a nested node tree");
    let integer_payloads = collect_integer_terminal_payloads(&decoded_node_tree);
    assert_eq!(
        integer_payloads,
        vec!["200".to_string()],
        "decoded integer terminal payload must be the decimal high-byte value"
    );
}

fn collect_integer_terminal_payloads(
    node: &tct::syntax_state_machine::RawSyntaxDecodedNode,
) -> Vec<String> {
    let mut payloads = Vec::new();
    for field in &node.fields {
        for value in &field.values {
            match value {
                RawSyntaxDecodedValue::Node(child) => {
                    payloads.extend(collect_integer_terminal_payloads(child));
                }
                RawSyntaxDecodedValue::Terminal(terminal)
                    if terminal.terminal.contains("integer") =>
                {
                    payloads.push(terminal.payload.clone());
                }
                RawSyntaxDecodedValue::Terminal(_) => {}
            }
        }
    }
    payloads
}

#[test]
fn csharp_local_unary_integer_return_scratch_seed_records_prefix_unary_token_only_seed() {
    let parity_report = build_csharp_state_machine_parity_report(
        CSHARP_LOCAL_UNARY_INTEGER_RETURN_GRAMMAR_ID,
        Path::new(CSHARP_LOCAL_UNARY_INTEGER_RETURN_METADATA_PATH),
    )
    .expect("C# prefix-unary scratch-seed parity report should build from checked metadata");
    parity_report
        .validate()
        .expect("C# prefix-unary scratch-seed parity report should validate");
    write_json(
        CSHARP_LOCAL_UNARY_INTEGER_RETURN_SCRATCH_SEED_PARITY_REPORT_PATH,
        &parity_report,
    );

    assert_eq!(parity_report.target, "csharp");
    assert_eq!(
        parity_report.candidate_grammar_id,
        CSHARP_LOCAL_UNARY_INTEGER_RETURN_GRAMMAR_ID
    );
    assert_eq!(
        parity_report.metadata_path,
        CSHARP_LOCAL_UNARY_INTEGER_RETURN_METADATA_PATH
    );
    assert_eq!(
        parity_report.next_ready_task,
        CSHARP_STATE_MACHINE_PARITY_NEXT_READY_TASK
    );
    assert!(parity_report.state_after_prefix_ready);
    assert!(parity_report.admissible_next_raw_tokens_ready);
    assert!(parity_report.strict_recovery_decode_agreement_ready);
    assert!(parity_report.consumed_token_accounting_ok);
    assert!(parity_report.target_metadata_sidecar_only);
    assert!(!parity_report.arena_ir_used);
    assert!(!parity_report.bpe_layer_defines_validity);

    let selected_trace = &parity_report.selected_completion_trace;
    assert_eq!(selected_trace.scope, CSHARP_SELECTED_COMPLETION_TRACE_SCOPE);
    assert_eq!(
        selected_trace.selection_loop_source,
        SELECTED_COMPLETION_TRACE_SOURCE
    );
    assert_eq!(
        selected_trace.transition_count,
        parity_report.raw_token_count
    );
    assert_eq!(
        selected_trace.selected_transition_accepted_count,
        parity_report.raw_token_count
    );
    assert!(selected_trace.every_selected_raw_token_admitted);
    assert_eq!(selected_trace.first_selected_raw_token_id, 0);
    assert_eq!(selected_trace.final_state_observed_status, "complete");
    assert!(selected_trace.final_state_eof_valid);
    assert_eq!(selected_trace.final_state_stack_depth, 0);
    assert!(selected_trace.step_trace_sample.iter().all(|sample| {
        sample.selected_raw_token_admitted
            && sample.selected_transition_accepted
            && sample.selected_raw_token_id < parity_report.token_layout.vocab_size
    }));

    let scratch = parity_report
        .scratch_generation_summary
        .as_ref()
        .expect("C# prefix-unary parity should preserve scratch-generation summary evidence");
    assert_eq!(
        scratch.scope,
        CSHARP_LOCAL_UNARY_INTEGER_RETURN_SCRATCH_GENERATION_SCOPE
    );
    assert_eq!(
        scratch.precompiled_artifact_grammar_id,
        CSHARP_LOCAL_UNARY_INTEGER_RETURN_GRAMMAR_ID
    );
    assert_eq!(scratch.empty_prefix_root, "CompilationUnitSyntax");
    assert_eq!(scratch.finite_admissible_raw_token_ids, vec![0]);
    assert!(scratch.selected_raw_token_admitted);
    assert!(scratch.selected_transition_accepted);
    assert_eq!(
        scratch.selected_completion_trace_scope,
        selected_trace.scope
    );
    assert_eq!(
        scratch.selected_completion_raw_token_count,
        parity_report.raw_token_count
    );
    assert_eq!(
        scratch.selected_completion_transition_count,
        parity_report.raw_token_count
    );
    assert_eq!(
        scratch.selected_completion_admissible_mask_query_count,
        parity_report.raw_token_count
    );
    assert!(scratch.selected_completion_every_token_admitted);
    assert_eq!(scratch.selected_completion_final_status, "complete");
    assert!(scratch.selected_completion_final_eof_valid);
    assert!(scratch.strict_incomplete_prefix_rejected);
    assert_eq!(scratch.strict_incomplete_prefix_error_kind, "premature_eof");
    assert!(scratch.token_only_decode_render_ready);
    assert!(
        scratch.token_only_raw_token_count > parity_report.raw_token_count,
        "prefix-unary render stream should materialize checked fixture payloads while parity replay keeps short synthetic payloads"
    );
    assert_eq!(scratch.token_only_root_rule, "CompilationUnitSyntax");
    assert_eq!(scratch.token_only_root_node_kind, "CompilationUnitSyntax");
    assert!(scratch.token_only_node_start_count >= 9);
    assert_eq!(scratch.token_only_payload_value_count, 6);
    assert_eq!(scratch.renderer_id, CSHARP_RAW_TREE_RENDERER_ID);
    assert_eq!(
        scratch.renderer_scope,
        CSHARP_LOCAL_UNARY_INTEGER_RETURN_RAW_TREE_RENDERER_SCOPE
    );
    assert_eq!(scratch.rendered_source_sha256, parity_report.source_sha256);
    assert!(scratch.rendered_source_matches_checked_source);
    assert_eq!(
        scratch.parser_sidecar_validation,
        "roslyn_self_test_required"
    );
    assert!(scratch.parser_sidecar_validation_command.contains(
        "csharp_syntax_metadata_smoke.py --fixture local-unary-integer-return --self-test"
    ));
    assert!(scratch.render_reparse_sidecar_blocker.is_none());
    assert!(!scratch.source_prefix_table_used);
    assert!(!scratch.arena_ir_used);
    assert!(!scratch.route_handoff_provenance_used);
    assert!(!scratch.bpe_validity_authority_used);
    assert!(!scratch.stable_backend_claimed);
    assert!(scratch.claim_boundary.contains("syntax-level C#"));
    assert!(
        scratch
            .claim_boundary
            .contains("empty CompilationUnitSyntax")
    );
    assert!(scratch.claim_boundary.contains("not arena-IR"));
    assert!(scratch.claim_boundary.contains("not BPE"));
    assert!(scratch.claim_boundary.contains("not stable-backend"));

    let grammar = CSharpSyntaxMetadataAdapter::new(CSHARP_LOCAL_UNARY_INTEGER_RETURN_GRAMMAR_ID)
        .lower_metadata_file(Path::new(CSHARP_LOCAL_UNARY_INTEGER_RETURN_METADATA_PATH))
        .expect("checked C# prefix-unary metadata should lower to GrammarIR");
    let sidecar = lower_grammar_ir_to_structural_model(&grammar, csharp_syntax_token_layout())
        .expect("checked C# prefix-unary GrammarIR should lower to StructuralModel");
    let raw_stream =
        build_csharp_local_unary_integer_return_raw_tree_render_token_stream_from_sidecar(&sidecar)
            .expect("C# prefix-unary replay should synthesize fixture source raw tokens");
    assert_eq!(
        raw_stream.raw_token_count,
        scratch.token_only_raw_token_count
    );
    let core = CompiledSyntaxModel::from_sidecar(&sidecar)
        .expect("checked C# prefix-unary StructuralModel should compile to raw state machine");
    let fixture_trace = core
        .selected_completion_trace(&raw_stream.raw_token_ids, &BTreeSet::new())
        .expect("C# prefix-unary fixture stream should replay through selected transitions");
    assert_eq!(fixture_trace.transition_count, raw_stream.raw_token_count);
    assert_eq!(
        fixture_trace.selected_transition_accepted_count,
        raw_stream.raw_token_count
    );
    assert!(fixture_trace.every_selected_raw_token_admitted);
    assert!(fixture_trace.final_state.eof_valid);
    let decoded_tree = core
        .decode_complete_tree(&raw_stream.raw_token_ids)
        .expect("C# prefix-unary raw token stream should materialize a raw decoded tree");
    assert_eq!(decoded_tree.root_rule(), Some("CompilationUnitSyntax"));
    assert_eq!(decoded_tree.root_node_kind(), Some("CompilationUnitSyntax"));
    assert_eq!(decoded_tree.payload_value_count(), 6);
    let decoded_node_tree = decoded_tree
        .decoded_node_tree()
        .expect("C# prefix-unary decoded events should materialize a nested node tree");
    for expected in [
        "FileScopedNamespaceDeclarationSyntax",
        "ClassDeclarationSyntax",
        "MethodDeclarationSyntax",
        "PredefinedTypeSyntax",
        "BlockSyntax",
        "ReturnStatementSyntax",
        "PrefixUnaryExpressionSyntax",
        "LiteralExpressionSyntax",
    ] {
        assert_eq!(
            decoded_node_tree.node_kind_count(expected),
            1,
            "decoded C# prefix-unary raw tree should contain one {expected}"
        );
    }

    let rendered = render_csharp_local_unary_integer_return_from_raw_tree(&decoded_tree)
        .expect("C# prefix-unary raw decoded tree should render source");
    assert_eq!(rendered.renderer_id, CSHARP_RAW_TREE_RENDERER_ID);
    assert_eq!(rendered.supported_construct, "local_unary_integer_return");
    assert_eq!(
        rendered.source,
        CSHARP_LOCAL_UNARY_INTEGER_RETURN_RAW_TREE_RENDERED_SOURCE
    );
    assert_eq!(
        rendered.source,
        fs::read_to_string(
            "tests/fixtures/csharp_syntax_sources/local_unary_integer_return/source.cs"
        )
        .expect("checked C# prefix-unary source fixture should be readable"),
        "rendered C# raw tree should match the Roslyn-checked prefix-unary source bytes"
    );
}

#[test]
fn csharp_member_access_scratch_seed_records_access_token_only_seed() {
    let parity_report = build_csharp_state_machine_parity_report(
        CSHARP_MEMBER_ACCESS_EXPRESSION_GRAMMAR_ID,
        Path::new(CSHARP_MEMBER_ACCESS_EXPRESSION_METADATA_PATH),
    )
    .expect("C# member-access scratch-seed parity report should build from checked metadata");
    parity_report
        .validate()
        .expect("C# member-access scratch-seed parity report should validate");
    write_json(
        CSHARP_MEMBER_ACCESS_EXPRESSION_SCRATCH_SEED_PARITY_REPORT_PATH,
        &parity_report,
    );

    assert_eq!(parity_report.target, "csharp");
    assert_eq!(
        parity_report.candidate_grammar_id,
        CSHARP_MEMBER_ACCESS_EXPRESSION_GRAMMAR_ID
    );
    assert_eq!(
        parity_report.metadata_path,
        CSHARP_MEMBER_ACCESS_EXPRESSION_METADATA_PATH
    );
    assert!(parity_report.state_after_prefix_ready);
    assert!(parity_report.admissible_next_raw_tokens_ready);
    assert!(parity_report.strict_recovery_decode_agreement_ready);
    assert!(parity_report.consumed_token_accounting_ok);
    assert!(parity_report.target_metadata_sidecar_only);
    assert!(!parity_report.arena_ir_used);
    assert!(!parity_report.bpe_layer_defines_validity);

    let selected_trace = &parity_report.selected_completion_trace;
    let scratch = parity_report
        .scratch_generation_summary
        .as_ref()
        .expect("C# member-access parity should preserve scratch-generation summary evidence");
    assert_eq!(
        scratch.scope,
        CSHARP_MEMBER_ACCESS_EXPRESSION_SCRATCH_GENERATION_SCOPE
    );
    assert_eq!(
        scratch.precompiled_artifact_grammar_id,
        CSHARP_MEMBER_ACCESS_EXPRESSION_GRAMMAR_ID
    );
    assert_eq!(scratch.empty_prefix_root, "CompilationUnitSyntax");
    assert_eq!(scratch.finite_admissible_raw_token_ids, vec![0]);
    assert!(scratch.selected_raw_token_admitted);
    assert!(scratch.selected_transition_accepted);
    assert_eq!(
        scratch.selected_completion_trace_scope,
        selected_trace.scope
    );
    assert_eq!(
        scratch.selected_completion_raw_token_count,
        parity_report.raw_token_count
    );
    assert_eq!(
        scratch.selected_completion_transition_count,
        parity_report.raw_token_count
    );
    assert!(scratch.selected_completion_every_token_admitted);
    assert_eq!(scratch.selected_completion_final_status, "complete");
    assert!(scratch.selected_completion_final_eof_valid);
    assert!(scratch.strict_incomplete_prefix_rejected);
    assert_eq!(scratch.strict_incomplete_prefix_error_kind, "premature_eof");
    assert!(scratch.token_only_decode_render_ready);
    assert!(
        scratch.token_only_raw_token_count > parity_report.raw_token_count,
        "member-access render stream should materialize checked fixture payloads while parity replay keeps short synthetic payloads"
    );
    assert_eq!(scratch.token_only_root_rule, "CompilationUnitSyntax");
    assert_eq!(scratch.token_only_root_node_kind, "CompilationUnitSyntax");
    assert!(scratch.token_only_node_start_count >= 9);
    assert!(scratch.token_only_payload_value_count >= 6);
    assert_eq!(scratch.renderer_id, CSHARP_RAW_TREE_RENDERER_ID);
    assert_eq!(
        scratch.renderer_scope,
        CSHARP_MEMBER_ACCESS_EXPRESSION_RAW_TREE_RENDERER_SCOPE
    );
    assert_eq!(scratch.rendered_source_sha256, parity_report.source_sha256);
    assert!(scratch.rendered_source_matches_checked_source);
    assert_eq!(
        scratch.parser_sidecar_validation,
        "roslyn_self_test_required"
    );
    assert!(scratch.parser_sidecar_validation_command.contains(
        "csharp_syntax_metadata_smoke.py --fixture member-access-expression --self-test"
    ));
    assert!(scratch.render_reparse_sidecar_blocker.is_none());
    assert!(!scratch.arena_ir_used);
    assert!(!scratch.route_handoff_provenance_used);
    assert!(!scratch.bpe_validity_authority_used);
    assert!(!scratch.stable_backend_claimed);

    let grammar = CSharpSyntaxMetadataAdapter::new(CSHARP_MEMBER_ACCESS_EXPRESSION_GRAMMAR_ID)
        .lower_metadata_file(Path::new(CSHARP_MEMBER_ACCESS_EXPRESSION_METADATA_PATH))
        .expect("checked C# member-access metadata should lower to GrammarIR");
    let sidecar = lower_grammar_ir_to_structural_model(&grammar, csharp_syntax_token_layout())
        .expect("checked C# member-access GrammarIR should lower to StructuralModel");
    let raw_stream =
        build_csharp_member_access_expression_raw_tree_render_token_stream_from_sidecar(&sidecar)
            .expect("C# member-access render replay should synthesize DateTime.Now raw tokens");
    let core = CompiledSyntaxModel::from_sidecar(&sidecar)
        .expect("checked C# member-access StructuralModel should compile to raw state machine");
    let fixture_trace = core
        .selected_completion_trace(&raw_stream.raw_token_ids, &BTreeSet::new())
        .expect(
            "C# member-access fixture render stream should replay through selected transitions",
        );
    assert_eq!(fixture_trace.transition_count, raw_stream.raw_token_count);
    assert_eq!(
        fixture_trace.selected_transition_accepted_count,
        raw_stream.raw_token_count
    );
    assert!(fixture_trace.every_selected_raw_token_admitted);
    assert!(fixture_trace.final_state.eof_valid);
    let decoded_tree = core
        .decode_complete_tree(&raw_stream.raw_token_ids)
        .expect("C# member-access raw token stream should materialize a raw decoded tree");
    assert_eq!(decoded_tree.root_rule(), Some("CompilationUnitSyntax"));
    assert_eq!(decoded_tree.root_node_kind(), Some("CompilationUnitSyntax"));
    let decoded_node_tree = decoded_tree
        .decoded_node_tree()
        .expect("C# member-access decoded events should materialize a nested node tree");
    assert_eq!(
        decoded_node_tree.node_kind_count("FileScopedNamespaceDeclarationSyntax"),
        1
    );
    assert_eq!(
        decoded_node_tree.node_kind_count("ClassDeclarationSyntax"),
        1
    );
    assert_eq!(
        decoded_node_tree.node_kind_count("MethodDeclarationSyntax"),
        1
    );
    assert_eq!(
        decoded_node_tree.node_kind_count("ReturnStatementSyntax"),
        1
    );
    assert_eq!(
        decoded_node_tree.node_kind_count("MemberAccessExpressionSyntax"),
        1
    );
    assert_eq!(decoded_node_tree.node_kind_count("IdentifierNameSyntax"), 2);

    let rendered = render_csharp_member_access_expression_from_raw_tree(&decoded_tree)
        .expect("C# member-access raw decoded tree should render source");
    assert_eq!(rendered.renderer_id, CSHARP_RAW_TREE_RENDERER_ID);
    assert_eq!(
        rendered.supported_construct,
        "member_access_expression_return"
    );
    assert_eq!(
        rendered.source,
        CSHARP_MEMBER_ACCESS_EXPRESSION_RAW_TREE_RENDERED_SOURCE
    );
    assert_eq!(
        rendered.source,
        fs::read_to_string(
            "tests/fixtures/csharp_syntax_sources/member_access_expression/source.cs"
        )
        .expect("checked C# member-access source fixture should be readable"),
        "rendered C# raw tree should match the Roslyn-checked member-access source bytes"
    );
}

#[test]
fn csharp_compilation_unit_source_file_scratch_seed_records_render_reparse_ready_sidecar() {
    let parity_report = build_csharp_state_machine_parity_report(
        CSHARP_COMPILATION_UNIT_SOURCE_FILE_GRAMMAR_ID,
        Path::new(CSHARP_COMPILATION_UNIT_SOURCE_FILE_METADATA_PATH),
    )
    .expect("C# compilation-unit scratch-seed parity report should build from checked metadata");
    parity_report
        .validate()
        .expect("C# compilation-unit scratch-seed parity report should validate");
    write_json(
        CSHARP_COMPILATION_UNIT_SOURCE_FILE_SCRATCH_SEED_PARITY_REPORT_PATH,
        &parity_report,
    );

    assert_eq!(parity_report.target, "csharp");
    assert_eq!(
        parity_report.candidate_grammar_id,
        CSHARP_COMPILATION_UNIT_SOURCE_FILE_GRAMMAR_ID
    );
    assert_eq!(
        parity_report.metadata_path,
        CSHARP_COMPILATION_UNIT_SOURCE_FILE_METADATA_PATH
    );
    assert_eq!(
        parity_report.cross_target_parity_claim_scope,
        CSHARP_STATE_MACHINE_PARITY_CLAIM_SCOPE
    );
    assert!(parity_report.state_after_prefix_ready);
    assert!(parity_report.admissible_next_raw_tokens_ready);
    assert!(parity_report.strict_recovery_decode_agreement_ready);
    assert!(parity_report.consumed_token_accounting_ok);

    assert_csharp_whole_source_scratch_summary(
        &parity_report,
        CSHARP_COMPILATION_UNIT_SOURCE_FILE_SCRATCH_GENERATION_SCOPE,
        CSHARP_COMPILATION_UNIT_SOURCE_FILE_GRAMMAR_ID,
        CSHARP_COMPILATION_UNIT_SOURCE_FILE_RAW_TREE_RENDERER_SCOPE,
        "csharp_syntax_metadata_smoke.py --fixture compilation-unit-source-file --self-test",
        0,
        0,
        "fixture render stream should materialize checked Shapes.cs payloads while parity replay keeps short synthetic payloads",
    );

    let root = parity_report
        .state_cases
        .iter()
        .find(|case| case.case_id == "csharp_empty_prefix")
        .expect("C# compilation-unit parity should expose the empty-prefix state");
    assert_eq!(root.prefix_raw_token_count, 0);
    assert_eq!(root.admissible_next_raw_token_ids, vec![0]);
    assert!(!root.eof_valid);

    let grammar = CSharpSyntaxMetadataAdapter::new(CSHARP_COMPILATION_UNIT_SOURCE_FILE_GRAMMAR_ID)
        .lower_metadata_file(Path::new(CSHARP_COMPILATION_UNIT_SOURCE_FILE_METADATA_PATH))
        .expect("checked C# compilation-unit metadata should lower to GrammarIR");
    let sidecar = lower_grammar_ir_to_structural_model(&grammar, csharp_syntax_token_layout())
        .expect("checked C# compilation-unit GrammarIR should lower to StructuralModel");
    let raw_stream =
        build_csharp_compilation_unit_source_file_raw_tree_render_token_stream_from_sidecar(
            &sidecar,
        )
        .expect("C# compilation-unit render replay should synthesize Shapes.cs raw tokens");
    let core = CompiledSyntaxModel::from_sidecar(&sidecar)
        .expect("checked C# compilation-unit StructuralModel should compile to raw state machine");
    let fixture_trace = core
        .selected_completion_trace(&raw_stream.raw_token_ids, &BTreeSet::new())
        .expect(
            "C# compilation-unit fixture render stream should replay through selected transitions",
        );
    assert_eq!(fixture_trace.transition_count, raw_stream.raw_token_count);
    assert_eq!(
        fixture_trace.selected_transition_accepted_count,
        raw_stream.raw_token_count
    );
    assert!(fixture_trace.every_selected_raw_token_admitted);
    assert!(fixture_trace.final_state.eof_valid);
    let decoded_tree = core
        .decode_complete_tree(&raw_stream.raw_token_ids)
        .expect("C# compilation-unit raw token stream should materialize a raw decoded tree");
    assert_eq!(decoded_tree.root_rule(), Some("CompilationUnitSyntax"));
    assert_eq!(decoded_tree.root_node_kind(), Some("CompilationUnitSyntax"));
    assert!(
        decoded_tree.node_start_count() > 50,
        "Shapes.cs decoded tree should include usings, nested names, namespace, type declarations, members, and constructor assignments"
    );
    assert_eq!(
        decoded_tree
            .decoded_node_tree()
            .expect("C# compilation-unit decoded events should materialize a nested node tree")
            .node_kind_count("UsingDirectiveSyntax"),
        5
    );
    let rendered = render_csharp_compilation_unit_source_file_from_raw_tree(&decoded_tree)
        .expect("C# compilation-unit raw decoded tree should render Shapes.cs source");
    assert_eq!(rendered.renderer_id, CSHARP_RAW_TREE_RENDERER_ID);
    assert_eq!(
        rendered.supported_construct,
        "compilation_unit_shapes_source_file"
    );
    assert_eq!(
        rendered.source,
        CSHARP_COMPILATION_UNIT_SOURCE_FILE_RAW_TREE_RENDERED_SOURCE
    );
    assert_eq!(
        rendered.source,
        fs::read_to_string("corpora/csharp/dotnet-samples-mini/src/pattern-matching/Shapes.cs")
            .expect("checked C# Shapes.cs source fixture should be readable"),
        "rendered C# raw tree should match the Roslyn-checked Shapes.cs source bytes"
    );
}

#[test]
fn csharp_compilation_unit_logger_source_file_scratch_seed_records_render_reparse_ready_sidecar() {
    let parity_report = build_csharp_state_machine_parity_report(
        CSHARP_COMPILATION_UNIT_LOGGER_SOURCE_FILE_GRAMMAR_ID,
        Path::new(CSHARP_COMPILATION_UNIT_LOGGER_SOURCE_FILE_METADATA_PATH),
    )
    .expect("C# Logger scratch-seed parity report should build from checked metadata");
    parity_report
        .validate()
        .expect("C# Logger scratch-seed parity report should validate");
    write_json(
        CSHARP_COMPILATION_UNIT_LOGGER_SOURCE_FILE_SCRATCH_SEED_PARITY_REPORT_PATH,
        &parity_report,
    );

    assert_eq!(parity_report.target, "csharp");
    assert_eq!(
        parity_report.candidate_grammar_id,
        CSHARP_COMPILATION_UNIT_LOGGER_SOURCE_FILE_GRAMMAR_ID
    );
    assert_eq!(
        parity_report.metadata_path,
        CSHARP_COMPILATION_UNIT_LOGGER_SOURCE_FILE_METADATA_PATH
    );
    assert!(parity_report.state_after_prefix_ready);
    assert!(parity_report.admissible_next_raw_tokens_ready);
    assert!(parity_report.strict_recovery_decode_agreement_ready);
    assert!(parity_report.consumed_token_accounting_ok);

    assert_csharp_whole_source_scratch_summary(
        &parity_report,
        CSHARP_COMPILATION_UNIT_LOGGER_SOURCE_FILE_SCRATCH_GENERATION_SCOPE,
        CSHARP_COMPILATION_UNIT_LOGGER_SOURCE_FILE_GRAMMAR_ID,
        CSHARP_COMPILATION_UNIT_LOGGER_SOURCE_FILE_RAW_TREE_RENDERER_SCOPE,
        "csharp_syntax_metadata_smoke.py --fixture compilation-unit-logger-source-file --self-test",
        25,
        20,
        "Logger render stream should materialize checked payloads while parity replay keeps short synthetic payloads",
    );

    let grammar =
        CSharpSyntaxMetadataAdapter::new(CSHARP_COMPILATION_UNIT_LOGGER_SOURCE_FILE_GRAMMAR_ID)
            .lower_metadata_file(Path::new(
                CSHARP_COMPILATION_UNIT_LOGGER_SOURCE_FILE_METADATA_PATH,
            ))
            .expect("checked C# Logger metadata should lower to GrammarIR");
    let sidecar = lower_grammar_ir_to_structural_model(&grammar, csharp_syntax_token_layout())
        .expect("checked C# Logger GrammarIR should lower to StructuralModel");
    let raw_stream =
        build_csharp_compilation_unit_logger_source_file_raw_tree_render_token_stream_from_sidecar(
            &sidecar,
        )
        .expect("C# Logger render replay should synthesize Logger.cs raw tokens");
    let core = CompiledSyntaxModel::from_sidecar(&sidecar)
        .expect("checked C# Logger StructuralModel should compile to raw state machine");
    let fixture_trace = core
        .selected_completion_trace(&raw_stream.raw_token_ids, &BTreeSet::new())
        .expect("C# Logger fixture render stream should replay through selected transitions");
    assert_eq!(fixture_trace.transition_count, raw_stream.raw_token_count);
    assert_eq!(
        fixture_trace.selected_transition_accepted_count,
        raw_stream.raw_token_count
    );
    assert!(fixture_trace.every_selected_raw_token_admitted);
    assert!(fixture_trace.final_state.eof_valid);
    let decoded_tree = core
        .decode_complete_tree(&raw_stream.raw_token_ids)
        .expect("C# Logger raw token stream should materialize a raw decoded tree");
    assert_eq!(decoded_tree.root_rule(), Some("CompilationUnitSyntax"));
    assert_eq!(decoded_tree.root_node_kind(), Some("CompilationUnitSyntax"));
    let decoded_node_tree = decoded_tree
        .decoded_node_tree()
        .expect("C# Logger decoded events should materialize a nested node tree");
    assert_eq!(
        decoded_node_tree.node_kind_count("FileScopedNamespaceDeclarationSyntax"),
        1
    );
    assert_eq!(
        decoded_node_tree.node_kind_count("EnumDeclarationSyntax"),
        1
    );
    assert_eq!(
        decoded_node_tree.node_kind_count("EnumMemberDeclarationSyntax"),
        6
    );
    assert_eq!(
        decoded_node_tree.node_kind_count("ClassDeclarationSyntax"),
        1
    );
    assert_eq!(
        decoded_node_tree.node_kind_count("FieldDeclarationSyntax"),
        1
    );
    assert_eq!(
        decoded_node_tree.node_kind_count("PropertyDeclarationSyntax"),
        1
    );
    assert_eq!(
        decoded_node_tree.node_kind_count("MethodDeclarationSyntax"),
        1
    );
    assert_eq!(
        decoded_node_tree.node_kind_count("InterpolatedStringExpressionSyntax"),
        1
    );

    let rendered = render_csharp_compilation_unit_logger_source_file_from_raw_tree(&decoded_tree)
        .expect("C# Logger raw decoded tree should render Logger.cs source");
    assert_eq!(rendered.renderer_id, CSHARP_RAW_TREE_RENDERER_ID);
    assert_eq!(
        rendered.supported_construct,
        "compilation_unit_logger_source_file"
    );
    assert_eq!(
        rendered.source,
        CSHARP_COMPILATION_UNIT_LOGGER_SOURCE_FILE_RAW_TREE_RENDERED_SOURCE
    );
    assert_eq!(
        rendered.source,
        fs::read_to_string("corpora/csharp/dotnet-samples-mini/src/delegates-and-events/Logger.cs")
            .expect("checked C# Logger.cs source fixture should be readable"),
        "rendered C# raw tree should match the Roslyn-checked Logger.cs source bytes"
    );
}

#[test]
fn csharp_compilation_unit_version_one_source_file_scratch_seed_records_render_reparse_ready_sidecar()
 {
    let parity_report = build_csharp_state_machine_parity_report(
        CSHARP_COMPILATION_UNIT_VERSION_ONE_SOURCE_FILE_GRAMMAR_ID,
        Path::new(CSHARP_COMPILATION_UNIT_VERSION_ONE_SOURCE_FILE_METADATA_PATH),
    )
    .expect("C# VersionOne scratch-seed parity report should build from checked metadata");
    parity_report
        .validate()
        .expect("C# VersionOne scratch-seed parity report should validate");
    write_json(
        CSHARP_COMPILATION_UNIT_VERSION_ONE_SOURCE_FILE_SCRATCH_SEED_PARITY_REPORT_PATH,
        &parity_report,
    );

    assert_eq!(parity_report.target, "csharp");
    assert_eq!(
        parity_report.candidate_grammar_id,
        CSHARP_COMPILATION_UNIT_VERSION_ONE_SOURCE_FILE_GRAMMAR_ID
    );
    assert_eq!(
        parity_report.metadata_path,
        CSHARP_COMPILATION_UNIT_VERSION_ONE_SOURCE_FILE_METADATA_PATH
    );
    assert!(parity_report.state_after_prefix_ready);
    assert!(parity_report.admissible_next_raw_tokens_ready);
    assert!(parity_report.strict_recovery_decode_agreement_ready);
    assert!(parity_report.consumed_token_accounting_ok);

    assert_csharp_whole_source_scratch_summary(
        &parity_report,
        CSHARP_COMPILATION_UNIT_VERSION_ONE_SOURCE_FILE_SCRATCH_GENERATION_SCOPE,
        CSHARP_COMPILATION_UNIT_VERSION_ONE_SOURCE_FILE_GRAMMAR_ID,
        CSHARP_COMPILATION_UNIT_VERSION_ONE_SOURCE_FILE_RAW_TREE_RENDERER_SCOPE,
        "csharp_syntax_metadata_smoke.py --fixture compilation-unit-version-one-source-file --self-test",
        30,
        20,
        "VersionOne render stream should materialize checked payloads while parity replay keeps short synthetic payloads",
    );

    let grammar = CSharpSyntaxMetadataAdapter::new(
        CSHARP_COMPILATION_UNIT_VERSION_ONE_SOURCE_FILE_GRAMMAR_ID,
    )
    .lower_metadata_file(Path::new(
        CSHARP_COMPILATION_UNIT_VERSION_ONE_SOURCE_FILE_METADATA_PATH,
    ))
    .expect("checked C# VersionOne metadata should lower to GrammarIR");
    let sidecar = lower_grammar_ir_to_structural_model(&grammar, csharp_syntax_token_layout())
        .expect("checked C# VersionOne GrammarIR should lower to StructuralModel");
    let raw_stream =
        build_csharp_compilation_unit_version_one_source_file_raw_tree_render_token_stream_from_sidecar(
            &sidecar,
        )
        .expect("C# VersionOne render replay should synthesize VersionOne.cs raw tokens");
    let core = CompiledSyntaxModel::from_sidecar(&sidecar)
        .expect("checked C# VersionOne StructuralModel should compile to raw state machine");
    let fixture_trace = core
        .selected_completion_trace(&raw_stream.raw_token_ids, &BTreeSet::new())
        .expect("C# VersionOne fixture render stream should replay through selected transitions");
    assert_eq!(fixture_trace.transition_count, raw_stream.raw_token_count);
    assert_eq!(
        fixture_trace.selected_transition_accepted_count,
        raw_stream.raw_token_count
    );
    assert!(fixture_trace.every_selected_raw_token_admitted);
    assert!(fixture_trace.final_state.eof_valid);
    let decoded_tree = core
        .decode_complete_tree(&raw_stream.raw_token_ids)
        .expect("C# VersionOne raw token stream should materialize a raw decoded tree");
    assert_eq!(decoded_tree.root_rule(), Some("CompilationUnitSyntax"));
    assert_eq!(decoded_tree.root_node_kind(), Some("CompilationUnitSyntax"));
    let decoded_node_tree = decoded_tree
        .decoded_node_tree()
        .expect("C# VersionOne decoded events should materialize a nested node tree");
    assert_eq!(
        decoded_node_tree.node_kind_count("FileScopedNamespaceDeclarationSyntax"),
        1
    );
    assert_eq!(
        decoded_node_tree.node_kind_count("ClassDeclarationSyntax"),
        2
    );
    assert_eq!(decoded_node_tree.node_kind_count("BaseListSyntax"), 1);
    assert_eq!(
        decoded_node_tree.node_kind_count("ArrowExpressionClauseSyntax"),
        1
    );
    assert_eq!(
        decoded_node_tree.node_kind_count("EventFieldDeclarationSyntax"),
        1
    );
    assert_eq!(
        decoded_node_tree.node_kind_count("ForEachStatementSyntax"),
        1
    );
    assert_eq!(
        decoded_node_tree.node_kind_count("ConditionalAccessExpressionSyntax"),
        1
    );
    assert_eq!(
        decoded_node_tree.node_kind_count("ObjectCreationExpressionSyntax"),
        1
    );
    assert_eq!(decoded_node_tree.node_kind_count("ThisExpressionSyntax"), 1);

    let rendered =
        render_csharp_compilation_unit_version_one_source_file_from_raw_tree(&decoded_tree)
            .expect("C# VersionOne raw decoded tree should render VersionOne.cs source");
    assert_eq!(rendered.renderer_id, CSHARP_RAW_TREE_RENDERER_ID);
    assert_eq!(
        rendered.supported_construct,
        "compilation_unit_version_one_source_file"
    );
    assert_eq!(
        rendered.source,
        CSHARP_COMPILATION_UNIT_VERSION_ONE_SOURCE_FILE_RAW_TREE_RENDERED_SOURCE
    );
    assert_eq!(
        rendered.source,
        fs::read_to_string("corpora/csharp/dotnet-samples-mini/src/events/VersionOne.cs")
            .expect("checked C# VersionOne.cs source fixture should be readable"),
        "rendered C# raw tree should match the Roslyn-checked VersionOne.cs source bytes"
    );
}

#[test]
fn csharp_state_machine_parity_consumes_shared_replay_api_contract() {
    let parity_report = build_csharp_state_machine_parity_report(
        "csharp_local_if_else_syntax_v0",
        Path::new(CSHARP_LOCAL_IF_ELSE_METADATA_PATH),
    )
    .expect("C# state-machine parity report should build from checked metadata");
    parity_report
        .validate()
        .expect("C# state-machine parity report should validate");
    write_json(CSHARP_PARITY_REPORT_PATH, &parity_report);

    assert_eq!(parity_report.target, "csharp");
    assert_eq!(
        parity_report.cross_target_parity_claim_scope,
        CSHARP_STATE_MACHINE_PARITY_CLAIM_SCOPE
    );
    assert_eq!(
        parity_report.next_ready_task,
        CSHARP_STATE_MACHINE_PARITY_NEXT_READY_TASK
    );
    assert!(parity_report.target_metadata_sidecar_only);
    assert!(!parity_report.arena_ir_used);
    assert!(parity_report.state_after_prefix_ready);
    assert!(parity_report.admissible_next_raw_tokens_ready);
    assert!(parity_report.invalid_mutation_rejection_ready);
    assert!(parity_report.strict_recovery_decode_agreement_ready);
    assert!(parity_report.consumed_token_accounting_ok);
    assert!(parity_report.token_layout_invariants_ok);

    let selected_trace = &parity_report.selected_completion_trace;
    assert_eq!(selected_trace.scope, CSHARP_SELECTED_COMPLETION_TRACE_SCOPE);
    assert_eq!(
        selected_trace.selection_loop_source,
        SELECTED_COMPLETION_TRACE_SOURCE
    );
    assert_eq!(
        selected_trace.raw_token_count,
        parity_report.raw_token_count
    );
    assert_eq!(
        selected_trace.transition_count,
        parity_report.raw_token_count
    );
    assert_eq!(
        selected_trace.admissible_mask_query_count,
        parity_report.raw_token_count
    );
    assert!(selected_trace.finite_admissible_raw_token_mask_available);
    assert!(selected_trace.every_selected_raw_token_admitted);
    assert_eq!(
        selected_trace.selected_transition_accepted_count,
        parity_report.raw_token_count
    );
    assert_eq!(selected_trace.final_state_observed_status, "complete");
    assert_eq!(
        selected_trace.final_state_consumed_raw_tokens,
        parity_report.raw_token_count
    );
    assert!(selected_trace.final_state_eof_valid);
    assert_eq!(selected_trace.final_state_stack_depth, 0);
    assert_eq!(
        selected_trace.strict_decode_agreement_case_id,
        parity_report.complete_stream.case_id
    );
    assert_eq!(
        selected_trace.strict_decode_agreement_observed_status,
        parity_report.complete_stream.observed_status
    );
    assert_eq!(
        selected_trace.strict_decode_agreement_eof_valid,
        parity_report.complete_stream.eof_valid
    );
    assert!(!selected_trace.step_trace_sample.is_empty());
    assert!(selected_trace.step_trace_sample.len() <= 6);
    assert!(selected_trace.step_trace_sample.iter().all(|sample| {
        sample.finite_admissible_raw_token_count > 0
            && sample.selected_raw_token_admitted
            && sample.selected_transition_accepted
            && sample.selected_raw_token_id < parity_report.token_layout.vocab_size
    }));
    assert!(selected_trace.step_trace_sample.iter().any(
        |sample| sample.transition_observed_status == "complete" && sample.transition_eof_valid
    ));

    let root = parity_report
        .state_cases
        .iter()
        .find(|case| case.case_id == "csharp_empty_prefix")
        .expect("C# parity report should include empty-prefix state");
    assert_eq!(root.admissible_next_raw_token_ids, vec![0]);
    assert!(!root.eof_valid);

    let multi = parity_report
        .state_cases
        .iter()
        .find(|case| case.case_id == "csharp_after_first_multi_alternative_field_start")
        .expect("C# parity report should include a multi-alternative field state");
    assert_eq!(multi.observed_status, "partial");
    assert!(
        !multi.admissible_next_raw_token_ids.contains(&0),
        "C# multi-alternative field must not admit generic node_start"
    );
    assert!(
        multi
            .admissible_next_raw_token_ids
            .iter()
            .any(|token| *token > 9),
        "C# multi-alternative field should advertise alternative-specific node_start tokens"
    );

    let payload = parity_report
        .state_cases
        .iter()
        .find(|case| case.case_id == "csharp_inside_first_terminal_payload_after_one_byte")
        .expect("C# parity report should include payload-range state");
    assert_eq!(payload.active_payload_byte_count, 1);
    assert!(payload.payload_end_admissible);
    assert_eq!(payload.admissible_next_raw_token_ranges.len(), 1);
    assert!(matches!(
        payload.admissible_next_raw_token_ranges[0].role.as_str(),
        "utf8_byte_payload" | "u8_integer_payload"
    ));

    assert!(
        parity_report
            .transition_cases
            .iter()
            .any(
                |case| case.case_id == "csharp_multi_alternative_actual_node_start_is_accepted"
                    && case.accepted
            )
    );
    assert!(
        parity_report
            .transition_cases
            .iter()
            .any(
                |case| case.case_id == "csharp_multi_alternative_generic_node_start_is_rejected"
                    && !case.accepted
                    && case.error_kind.as_deref() == Some("unexpected_structural_token")
            )
    );
    assert!(
        parity_report
            .invalid_mutation_cases
            .iter()
            .any(|case| case.error_kind.as_deref() == Some("invalid_payload_token"))
    );
    assert!(parity_report.strict_cases.iter().any(|case| case.case_id
        == "csharp_strict_partial_prefix_rejects_eof"
        && case.error_kind.as_deref() == Some("premature_eof")));
    assert!(parity_report.recovery_cases.iter().any(|case| case.case_id
        == "csharp_recovery_trailing_pad_normalizes_after_state_api_complete"
        && case.observed_outcome == "normalized_trailing_pad"
        && case.ignored_trailing_tokens == 1));
}

#[test]
fn csharp_nullable_marker_presence_parity_consumes_shared_replay_api_contract() {
    let parity_report = build_csharp_state_machine_parity_report(
        "csharp_local_nullable_return_type_syntax_v0",
        Path::new(CSHARP_LOCAL_NULLABLE_RETURN_TYPE_METADATA_PATH),
    )
    .expect("C# nullable marker state-machine parity report should build from checked metadata");
    parity_report
        .validate()
        .expect("C# nullable marker state-machine parity report should validate");
    write_json(CSHARP_NULLABLE_MARKER_PARITY_REPORT_PATH, &parity_report);

    assert_eq!(parity_report.complete_stream.observed_status, "complete");
    assert!(parity_report.admissible_next_raw_tokens_ready);
    assert!(parity_report.strict_recovery_decode_agreement_ready);
    assert!(!parity_report.arena_ir_used);
    assert!(!parity_report.bpe_layer_defines_validity);

    let marker_decision = parity_report
        .marker_presence_decisions
        .iter()
        .find(|decision| {
            decision.owner_rule == "NullableTypeSyntax" && decision.owner_field == "QuestionToken"
        })
        .expect("nullable marker presence decision should be preserved");
    assert_eq!(marker_decision.selected_node_kind, "nullable_marker");
    assert_eq!(marker_decision.payload_kind, "string");
    assert!(marker_decision.terminal_event_decoded);
    assert!(marker_decision.string_payload_byte_decoded);
    assert!(marker_decision.payload_end_decoded);
    assert!(marker_decision.state_after_terminal_admits_utf8_payload_range);
    assert!(marker_decision.state_after_payload_byte_admits_payload_end);
    let utf8_start = parity_report
        .token_layout
        .ranges
        .iter()
        .find(|range| range.role == "utf8_byte_payload")
        .expect("C# parity token layout should include UTF-8 payload range")
        .start;
    assert_eq!(
        marker_decision.first_payload_raw_token_id,
        utf8_start + u32::from(b'?')
    );
    assert_eq!(
        marker_decision.generated_marker_test_marker,
        "generated_csharp_local_nullable_return_type_rejects_invalid_nullable_marker"
    );
    assert!(marker_decision.generated_codec_marker_evidence_preserved);

    let invalid_marker_payload = parity_report
        .marker_presence_invalid_payload_mutations
        .iter()
        .find(|case| case.case_id == "csharp_nullable_marker_invalid_payload_is_rejected")
        .expect("nullable marker malformed-payload mutation should be checked");
    assert_eq!(
        invalid_marker_payload.error_kind.as_deref(),
        Some("invalid_payload_token")
    );
    assert!(!invalid_marker_payload.accepted);
    assert!(
        parity_report
            .claim_boundary
            .contains("not arena-IR evidence")
    );

    let local_parity_report = build_csharp_state_machine_parity_report(
        "csharp_local_nullable_declaration_syntax_v0",
        Path::new(CSHARP_LOCAL_NULLABLE_DECLARATION_METADATA_PATH),
    )
    .expect(
        "C# nullable local declaration marker parity report should build from checked metadata",
    );
    local_parity_report
        .validate()
        .expect("C# nullable local declaration marker parity report should validate");
    write_json(
        CSHARP_LOCAL_NULLABLE_DECLARATION_MARKER_PARITY_REPORT_PATH,
        &local_parity_report,
    );

    let local_marker_decision = local_parity_report
        .marker_presence_decisions
        .iter()
        .find(|decision| {
            decision.owner_rule == "NullableTypeSyntax" && decision.owner_field == "QuestionToken"
        })
        .expect("nullable local declaration marker presence decision should be preserved");
    assert_eq!(local_marker_decision.selected_node_kind, "nullable_marker");
    assert_eq!(local_marker_decision.payload_kind, "string");
    assert!(local_marker_decision.terminal_event_decoded);
    assert!(local_marker_decision.string_payload_byte_decoded);
    assert!(local_marker_decision.payload_end_decoded);
    assert!(local_marker_decision.state_after_terminal_admits_utf8_payload_range);
    assert!(local_marker_decision.state_after_payload_byte_admits_payload_end);
    assert_eq!(
        local_marker_decision.first_payload_raw_token_id,
        utf8_start + u32::from(b'?')
    );
    assert_eq!(
        local_marker_decision.generated_marker_test_marker,
        "generated_csharp_local_nullable_declaration_rejects_invalid_nullable_marker"
    );
    assert!(local_marker_decision.generated_codec_marker_evidence_preserved);

    let local_csharp_api = TargetNeutralGeneratedCodecStateApi::from_csharp_parity_report(
        &local_parity_report,
    )
    .expect(
        "C# nullable local declaration parity report should expose the target-neutral state API",
    );
    assert_eq!(
        local_csharp_api.record_id,
        CSHARP_LOCAL_NULLABLE_DECLARATION_MARKER_PRESENCE_RECORD_ID
    );
    let local_csharp_record_query = csharp_nullable_marker_presence_record_query_for_record_id(
        CSHARP_LOCAL_NULLABLE_DECLARATION_MARKER_PRESENCE_RECORD_ID,
    );
    let local_api_mask = local_csharp_api
        .admissible_token_mask_contract(&local_csharp_record_query)
        .expect("C# nullable local declaration admissible-token mask should consume the state API");
    assert_eq!(
        local_api_mask.record_id,
        CSHARP_LOCAL_NULLABLE_DECLARATION_MARKER_PRESENCE_RECORD_ID
    );
    assert_eq!(local_api_mask.finite_admissible_raw_token_count, 1);
    assert_eq!(local_api_mask.payload_range_count, 1);
    assert!(local_api_mask.payload_end_admissible);
    assert!(local_api_mask.eof_valid);
}

#[test]
fn matched_go_csharp_presence_masks_use_target_neutral_state_api() {
    let go_report = build_go_ast_declaration_presence_replay_report()
        .expect("Go AST declaration presence replay report should build");
    let csharp_report = build_csharp_state_machine_parity_report(
        "csharp_local_nullable_return_type_syntax_v0",
        Path::new(CSHARP_LOCAL_NULLABLE_RETURN_TYPE_METADATA_PATH),
    )
    .expect("C# nullable marker state-machine parity report should build");

    let contract = matched_go_csharp_presence_mask_contract(&go_report, &csharp_report)
        .expect("matched Go/C# presence mask contract should validate");

    assert_eq!(contract.scope, MATCHED_PRESENCE_MASK_SCOPE);
    assert!(contract.target_neutral_state_api_consumed);
    assert_eq!(contract.matched_record_count, 2);
    assert_eq!(contract.finite_mask_record_count, 2);
    assert_eq!(contract.payload_range_mask_record_count, 1);
    assert_eq!(contract.invalid_payload_rejection_count, 2);
    assert_eq!(
        contract.validity_authority,
        TARGET_NEUTRAL_RAW_STATE_API_AUTHORITY
    );
    assert!(!contract.arena_ir_used);
    assert!(!contract.bpe_layer_defines_validity);
    assert!(!contract.source_instance_segment_sidecars_used);
    assert!(!contract.downstream_artifact_ladder_used);

    let go_mask = contract
        .records
        .iter()
        .find(|record| record.target == "go" && record.owner_rule == "TypeSpec")
        .expect("Go TypeSpec.Assign presence mask should be present");
    assert_eq!(go_mask.owner_field, "Assign");
    assert_eq!(
        go_mask.selected_node_kind,
        "go_syntax_presence_type_spec_alias.PRESENT"
    );
    assert_eq!(go_mask.finite_admissible_raw_token_ids, vec![8, 9]);
    assert!(go_mask.payload_token_ranges.is_empty());
    assert!(!go_mask.payload_end_admissible);
    assert!(go_mask.invalid_payload_rejected);
    assert_eq!(go_mask.invalid_payload_error_kind, "invalid_payload_token");
    assert_eq!(
        go_mask.validity_authority,
        TARGET_NEUTRAL_RAW_STATE_API_AUTHORITY
    );

    let csharp_mask = contract
        .records
        .iter()
        .find(|record| record.target == "csharp" && record.owner_rule == "NullableTypeSyntax")
        .expect("C# nullable marker presence mask should be present");
    assert_eq!(csharp_mask.owner_field, "QuestionToken");
    assert_eq!(csharp_mask.selected_node_kind, "nullable_marker");
    assert_eq!(csharp_mask.finite_admissible_raw_token_ids, vec![7]);
    assert_eq!(csharp_mask.payload_token_ranges.len(), 1);
    assert_eq!(
        csharp_mask.payload_token_ranges[0].role,
        "utf8_byte_payload"
    );
    assert!(csharp_mask.payload_end_admissible);
    assert!(csharp_mask.invalid_payload_rejected);
    assert_eq!(
        csharp_mask.invalid_payload_error_kind,
        "invalid_payload_token"
    );
    assert_eq!(
        csharp_mask.validity_authority,
        TARGET_NEUTRAL_RAW_STATE_API_AUTHORITY
    );

    let csharp_api = TargetNeutralGeneratedCodecStateApi::from_csharp_parity_report(&csharp_report)
        .expect("C# nullable marker parity report should expose the target-neutral state API");
    assert_eq!(
        csharp_api.record_id,
        CSHARP_NULLABLE_MARKER_PRESENCE_RECORD_ID
    );
    let csharp_record_query = csharp_nullable_marker_presence_record_query();
    let marker_selection = csharp_api
        .candidate_raw_token_transition("csharp_nullable_marker_question_payload_byte_is_accepted")
        .expect("nullable marker payload-byte selection should resolve");
    assert!(marker_selection.accepted);
    assert_eq!(
        marker_selection.candidate_raw_token_id,
        csharp_mask.payload_token_ranges[0].start + u32::from(b'?')
    );
    let invalid_marker_selection = csharp_api
        .candidate_raw_token_transition(
            "csharp_nullable_marker_node_start_payload_byte_is_rejected",
        )
        .expect("nullable marker invalid payload-byte selection should resolve");
    assert!(!invalid_marker_selection.accepted);
    assert_eq!(
        invalid_marker_selection.error_kind.as_deref(),
        Some("invalid_payload_token")
    );
    let api_mask = csharp_api
        .admissible_token_mask_contract(&csharp_record_query)
        .expect("C# nullable marker admissible-token mask should consume the state API");
    assert_eq!(
        api_mask.record_id,
        CSHARP_NULLABLE_MARKER_PRESENCE_RECORD_ID
    );
    assert_eq!(api_mask.finite_admissible_raw_token_count, 1);
    assert_eq!(api_mask.payload_range_count, 1);
    assert!(api_mask.payload_end_admissible);
    assert!(api_mask.eof_valid);
    assert_eq!(api_mask.bpe_raw_expansion_replay_case_count, 2);
    assert_eq!(api_mask.bpe_raw_expansion_accepted_case_count, 1);
    assert_eq!(api_mask.bpe_raw_expansion_first_failure_case_count, 1);
    let continuation = csharp_api
        .prefix_state_continuation_contract(&csharp_record_query)
        .expect(
            "C# nullable marker continuation should consume prefix, strict, and recovery cases",
        );
    assert_eq!(continuation.request_count, 3);
    assert!(continuation.responses.iter().any(|response| {
        response.request_prefix_state_case_id == "csharp_nullable_marker_after_terminal"
            && !response.payload_token_ranges.is_empty()
    }));
    assert!(continuation.responses.iter().any(|response| {
        response.request_prefix_state_case_id == "csharp_nullable_marker_after_payload_byte"
            && response
                .finite_admissible_raw_token_ids
                .contains(&csharp_mask.finite_admissible_raw_token_ids[0])
    }));
    assert_eq!(continuation.bpe_raw_expansion_accepted_case_count, 1);
    assert_eq!(continuation.bpe_raw_expansion_first_failure_case_count, 1);

    let local_csharp_report = build_csharp_state_machine_parity_report(
        "csharp_local_nullable_declaration_syntax_v0",
        Path::new(CSHARP_LOCAL_NULLABLE_DECLARATION_METADATA_PATH),
    )
    .expect("C# nullable local declaration state-machine parity report should build");
    let local_contract = matched_go_csharp_presence_mask_contract(&go_report, &local_csharp_report)
        .expect("matched Go/C# local declaration presence mask contract should validate");
    assert_eq!(local_contract.matched_record_count, 2);
    assert_eq!(local_contract.finite_mask_record_count, 2);
    assert_eq!(local_contract.payload_range_mask_record_count, 1);
    assert_eq!(local_contract.invalid_payload_rejection_count, 2);
    let local_csharp_mask = local_contract
        .records
        .iter()
        .find(|record| {
            record.target == "csharp"
                && record.record_id == CSHARP_LOCAL_NULLABLE_DECLARATION_MARKER_PRESENCE_RECORD_ID
        })
        .expect("C# nullable local declaration marker presence mask should be present");
    assert_eq!(local_csharp_mask.owner_rule, "NullableTypeSyntax");
    assert_eq!(local_csharp_mask.owner_field, "QuestionToken");
    assert_eq!(local_csharp_mask.selected_node_kind, "nullable_marker");
    assert_eq!(local_csharp_mask.finite_admissible_raw_token_ids, vec![7]);
    assert_eq!(local_csharp_mask.payload_token_ranges.len(), 1);
    assert!(local_csharp_mask.payload_end_admissible);
    assert!(local_csharp_mask.invalid_payload_rejected);
    assert_eq!(
        local_csharp_mask.validity_authority,
        TARGET_NEUTRAL_RAW_STATE_API_AUTHORITY
    );
}

#[test]
fn csharp_real_source_state_machine_parity_consumes_generated_codec_structural_replay_sidecar() {
    let corpus_gate = csharp_real_source_generated_codec_corpus_gate();
    let parity_report = build_csharp_real_source_state_machine_parity_report(&corpus_gate)
        .expect("C# real-source state-machine parity should build from checked corpus evidence");
    parity_report
        .validate()
        .expect("C# real-source state-machine parity report should validate");
    write_json(CSHARP_REAL_SOURCE_PARITY_REPORT_PATH, &parity_report);

    assert!(parity_report.payload_sequence_control_preserved);
    assert!(
        parity_report
            .generated_codec_report_fields_preserved
            .iter()
            .any(|field| field == "structural_replay.raw_token_segments")
    );
    assert!(parity_report.missing_replay_report_fields.is_empty());
    assert_eq!(parity_report.corpus_replay_record_count, 5);
    assert_eq!(parity_report.source_report_paths.len(), 5);
    let corpus_manifest_paths = parity_report
        .corpus_replay_records
        .iter()
        .map(|record| record.manifest_path.as_str())
        .collect::<BTreeSet<_>>();
    assert_eq!(
        corpus_manifest_paths,
        BTreeSet::from([
            "pattern-matching/Shapes.cs",
            "delegates-and-events/Logger.cs",
            "events/VersionOne.cs",
            "iterators/IteratorMethods.cs",
            "serialization/Loan.cs"
        ])
    );
    assert_eq!(
        parity_report.supported_record_ids,
        vec![
            "pattern-matching/Shapes.cs".to_string(),
            "delegates-and-events/Logger.cs".to_string(),
            "events/VersionOne.cs".to_string(),
            "iterators/IteratorMethods.cs".to_string(),
            "serialization/Loan.cs".to_string(),
        ]
    );
    assert_eq!(
        parity_report.source_report_paths,
        parity_report
            .supported_record_ids
            .iter()
            .map(|record_id| csharp_real_source_state_machine_parity_report_path(record_id))
            .collect::<Vec<_>>()
    );
    assert_eq!(parity_report.generated_codec_report_paths.len(), 5);
    assert_eq!(parity_report.generated_codec_report_commands.len(), 5);

    let aggregate_payload_sequence_control_token_count = parity_report
        .corpus_replay_records
        .iter()
        .map(|record| record.payload_sequence_control_token_count)
        .sum::<usize>();
    let aggregate_raw_token_count = parity_report
        .corpus_replay_records
        .iter()
        .map(|record| record.structural_replay_token_stream.raw_token_count)
        .sum::<usize>();
    let aggregate_state_case_count = parity_report
        .corpus_replay_records
        .iter()
        .map(|record| record.state_cases.len())
        .sum::<usize>();
    let aggregate_transition_case_count = parity_report
        .corpus_replay_records
        .iter()
        .map(|record| record.transition_cases.len())
        .sum::<usize>();
    let aggregate_strict_case_count = parity_report
        .corpus_replay_records
        .iter()
        .map(|record| record.strict_cases.len())
        .sum::<usize>();
    let aggregate_recovery_case_count = parity_report
        .corpus_replay_records
        .iter()
        .map(|record| record.recovery_cases.len())
        .sum::<usize>();
    assert_eq!(
        parity_report.payload_sequence_control_token_count,
        aggregate_payload_sequence_control_token_count
    );
    assert_eq!(parity_report.raw_token_count, aggregate_raw_token_count);
    assert_eq!(parity_report.state_case_count, aggregate_state_case_count);
    assert_eq!(
        parity_report.transition_case_count,
        aggregate_transition_case_count
    );
    assert_eq!(
        parity_report.strict_decode_case_count,
        aggregate_strict_case_count
    );
    assert_eq!(
        parity_report.recovery_decode_case_count,
        aggregate_recovery_case_count
    );
    assert_eq!(parity_report.payload_range_state_count, 5);
    assert_eq!(parity_report.eof_valid_state_count, 5);
    assert!(parity_report.raw_tokens_below_vocab_size);
    assert!(parity_report.pad_token_absent);
    assert!(parity_report.raw_state_machine_complete);
    assert!(parity_report.payload_sequence_control_payload_end_collides_with_structural_terminal);
    assert!(parity_report.structural_replay_payload_end_matches_structural_payload_end);
    assert!(parity_report.raw_token_segment_bridge_ready);
    assert_eq!(
        parity_report.source_record_raw_token_emission_api_id,
        CSHARP_SOURCE_RECORD_RAW_TOKEN_EMISSION_API_ID
    );
    assert_eq!(
        parity_report.source_record_raw_token_emission_api_boundary,
        CSHARP_SOURCE_RECORD_RAW_TOKEN_EMISSION_API_BOUNDARY
    );

    for record in &parity_report.corpus_replay_records {
        assert!(
            record.missing_replay_report_fields.is_empty(),
            "{} should preserve every replay report field",
            record.manifest_path
        );
        assert_eq!(record.parser_tool, "dotnet");
        assert_eq!(
            record.parser_package.as_deref(),
            Some("Microsoft.CodeAnalysis.CSharp")
        );
        assert_eq!(record.parser_metadata_validation, "checked");
        assert!(record.parser_sidecar_validation_ready);
        assert_eq!(record.complete_stream.observed_status, "complete");
        assert_eq!(
            record.complete_stream.consumed_raw_tokens,
            record.structural_replay_token_stream.raw_token_count
        );
        assert!(record.strict_cases.iter().any(|case| case.accepted));
        assert!(
            record.strict_cases.iter().any(|case| {
                !case.accepted && case.error_kind.as_deref() == Some("premature_eof")
            })
        );
        assert!(record.recovery_cases.iter().any(|case| {
            case.observed_outcome == "normalized_trailing_pad" && case.ignored_trailing_tokens == 1
        }));
        assert!(record.consumed_token_accounting_ok);
        assert!(record.raw_token_segment_bridge_ready);
        assert_eq!(
            record.source_record_raw_token_emission_api_id,
            CSHARP_SOURCE_RECORD_RAW_TOKEN_EMISSION_API_ID
        );
        assert_eq!(
            record.source_record_raw_token_emission_api_boundary,
            CSHARP_SOURCE_RECORD_RAW_TOKEN_EMISSION_API_BOUNDARY
        );
        assert!(record.prefix_state_api_ready);
        assert!(record.admissible_next_raw_tokens_ready);
        assert!(record.eof_state_api_ready);
        assert!(record.strict_recovery_decode_agreement_ready);
        assert!(record.state_cases.iter().any(|case| {
            case.case_id.ends_with("_complete_stream")
                && case.eof_valid
                && case.admissible_next_raw_token_ids.is_empty()
        }));
        assert!(record.state_cases.iter().any(|case| {
            case.case_id
                .ends_with("_inside_first_terminal_payload_after_one_byte")
                && case.payload_end_admissible
                && !case.admissible_next_raw_token_ranges.is_empty()
        }));
        assert!(record.transition_cases.iter().any(|case| case.accepted));
        assert!(record.transition_cases.iter().any(|case| {
            !case.accepted && case.error_kind.as_deref() == Some("pad_token_in_raw_stream")
        }));
    }
    let iterator_record = parity_report
        .corpus_replay_records
        .iter()
        .find(|record| record.manifest_path == "iterators/IteratorMethods.cs")
        .expect("IteratorMethods should remain one checked C# corpus record");
    assert_eq!(
        iterator_record.source_path,
        "corpora/csharp/dotnet-samples-mini/src/iterators/IteratorMethods.cs"
    );
    assert_eq!(iterator_record.parser_tool, "dotnet");
    assert_eq!(
        iterator_record.parser_package.as_deref(),
        Some("Microsoft.CodeAnalysis.CSharp")
    );
    assert_eq!(iterator_record.parser_metadata_validation, "checked");
    assert!(iterator_record.parser_sidecar_validation_ready);
    assert_eq!(iterator_record.payload_sequence_control_token_count, 482);
    assert_eq!(
        parity_report.next_ready_task,
        CSHARP_STATE_MACHINE_PARITY_NEXT_READY_TASK
    );
    assert_eq!(
        iterator_record
            .structural_replay_token_stream
            .token_layout_kind,
        "structural_model_replay_roles"
    );
    assert_eq!(iterator_record.complete_stream.observed_status, "complete");
    assert_eq!(
        iterator_record.complete_stream.consumed_raw_tokens,
        iterator_record
            .structural_replay_token_stream
            .raw_token_count
    );
    assert!(parity_report.pipeline.parser_backed_metadata_loaded);
    assert!(parity_report.pipeline.grammar_ir_lowered);
    assert!(parity_report.pipeline.structural_model_lowered);
    assert!(parity_report.pipeline.token_layout_compiled);
    assert!(parity_report.pipeline.raw_state_machine_compiled);
    assert!(parity_report.pipeline.raw_token_encode_ready);
    assert!(parity_report.pipeline.raw_state_machine_decode_ready);
    assert!(parity_report.pipeline.parser_sidecar_validation_ready);
    assert!(!parity_report.pipeline.arena_ir_used);
    assert!(!parity_report.pipeline.route_handoff_provenance_used);
    assert!(!parity_report.pipeline.bpe_validity_authority_used);
    assert!(iterator_record.state_cases.iter().any(|case| {
        case.case_id
            == "csharp_iterators_iteratormethods_cs_inside_first_terminal_payload_after_one_byte"
            && case.payload_end_admissible
    }));
    assert!(iterator_record.transition_cases.iter().any(|case| {
        case.case_id == "csharp_iterators_iteratormethods_cs_first_field_next_segment_is_accepted"
            && case.accepted
    }));
    assert!(iterator_record.transition_cases.iter().any(|case| {
        case.case_id == "csharp_iterators_iteratormethods_cs_pad_token_candidate_is_rejected"
            && !case.accepted
            && case.error_kind.as_deref() == Some("pad_token_in_raw_stream")
    }));
    assert!(iterator_record.strict_cases.iter().any(|case| {
        case.case_id == "csharp_iterators_iteratormethods_cs_strict_partial_prefix_rejects_eof"
            && case.error_kind.as_deref() == Some("premature_eof")
    }));
    assert!(iterator_record.recovery_cases.iter().any(|case| {
        case.case_id
            == "csharp_iterators_iteratormethods_cs_recovery_trailing_pad_normalizes_after_state_api_complete"
            && case.observed_outcome == "normalized_trailing_pad"
            && case.ignored_trailing_tokens == 1
    }));
    assert_eq!(
        parity_report.next_ready_task,
        CSHARP_STATE_MACHINE_PARITY_NEXT_READY_TASK
    );
    assert!(
        parity_report.validation_commands.iter().any(
            |command| command.contains("syntax_structural_model_cross_target_equivalence_test")
        )
    );
    assert!(
        parity_report
            .validation_commands
            .iter()
            .any(|command| command.contains("csharp_route_contract_test"))
    );
    assert!(
        parity_report
            .validation_commands
            .iter()
            .any(|command| command.contains("test-quality-audit.py --changed-from origin/main"))
    );
    assert!(parity_report.claim_boundary.contains("not arena-IR"));
    assert!(parity_report.claim_boundary.contains("not BPE"));
}

#[test]
#[ignore = "known failure: current-plus report-ingestion manifest pins grammar_artifact_content_sha256 3f091674..., the live unified Go grammar artifact is 9a8f8f6c...; see docs/frontends-development.md"]
fn shared_generated_codec_state_api_handoff_consumes_go_and_csharp_state_api_reports() {
    let go_report_set = build_go_source_instance_state_api_report_set();
    let corpus_gate = csharp_real_source_generated_codec_corpus_gate();
    let csharp_parity_report = build_csharp_real_source_state_machine_parity_report(&corpus_gate)
        .expect("C# real-source state-machine parity should build");

    let handoff_gate = shared_generated_codec_state_api_handoff_gate();
    handoff_gate
        .validate_against_report_sets(
            &go_report_set.state_api_reports,
            &go_report_set.strict_recovery_reports,
            &csharp_parity_report,
        )
        .expect("shared Go/C# state API handoff should consume both report families");

    assert_eq!(
        handoff_gate.artifact_kind,
        SHARED_STATE_API_HANDOFF_ARTIFACT_KIND
    );
    assert_eq!(
        handoff_gate.completed_promotion_class,
        SHARED_STATE_API_HANDOFF_COMPLETED_CLASS
    );
    assert_eq!(
        handoff_gate.next_missing_stable_promotion_class,
        SHARED_STATE_API_HANDOFF_NEXT_CLASS
    );
    let csharp_source = handoff_gate
        .evidence_sources
        .iter()
        .find(|source| source.target == "csharp")
        .expect("C# handoff evidence source should be present");
    assert_eq!(csharp_source.source_record_count, 5);
    assert_eq!(
        csharp_source
            .source_record_raw_token_emission_api_id
            .as_deref(),
        Some(CSHARP_SOURCE_RECORD_RAW_TOKEN_EMISSION_API_ID)
    );
    assert_eq!(
        csharp_source
            .source_record_raw_token_emission_api_boundary
            .as_deref(),
        Some(CSHARP_SOURCE_RECORD_RAW_TOKEN_EMISSION_API_BOUNDARY)
    );
    let go_source = handoff_gate
        .evidence_sources
        .iter()
        .find(|source| source.target == "go")
        .expect("Go handoff evidence source should be present");
    assert_eq!(
        go_source.source_record_count,
        go_report_set.source_paths.len()
    );
    assert_eq!(
        go_source.state_case_count,
        go_report_set
            .state_api_reports
            .iter()
            .map(|report| report.state_cases.len())
            .sum::<usize>()
    );
    for source_path in &go_report_set.source_paths {
        assert!(
            go_source.supported_record_ids.contains(source_path),
            "Go handoff evidence should include manifest source {source_path}"
        );
    }
    assert!(
        handoff_gate
            .validation_commands
            .iter()
            .any(|command| command.contains(SHARED_STATE_API_HANDOFF_FIXTURE_PATH))
            || handoff_gate
                .validation_commands
                .iter()
                .any(|command| command
                    .contains("syntax_backend_shared_generated_codec_state_api_handoff"))
    );
}

#[test]
#[ignore = "known failure: current-plus report-ingestion manifest pins grammar_artifact_content_sha256 3f091674..., the live unified Go grammar artifact is 9a8f8f6c...; see docs/frontends-development.md"]
fn target_neutral_generated_codec_state_api_integration_consumes_shared_handoff_surface() {
    let go_report_set = build_go_source_instance_state_api_report_set();
    let state_api_report = go_report_set
        .state_api_reports
        .first()
        .expect("Go report set should include at least one manifest source");
    let strict_recovery_report = go_report_set
        .strict_recovery_reports
        .first()
        .expect("Go strict/recovery report set should include at least one manifest source");
    let corpus_gate = csharp_real_source_generated_codec_corpus_gate();
    let csharp_parity_report = build_csharp_real_source_state_machine_parity_report(&corpus_gate)
        .expect("C# real-source state-machine parity should build");
    let handoff_gate = shared_generated_codec_state_api_handoff_gate();
    let integration_gate = generated_codec_state_api_integration_gate();

    integration_gate
        .validate_against_report_sets(
            &handoff_gate,
            &go_report_set.state_api_reports,
            &go_report_set.strict_recovery_reports,
            &csharp_parity_report,
        )
        .expect("target-neutral integration should consume shared Go/C# state API reports");

    assert_eq!(
        integration_gate.artifact_kind,
        STATE_API_INTEGRATION_ARTIFACT_KIND
    );
    assert_eq!(
        integration_gate.completed_promotion_class,
        STATE_API_INTEGRATION_COMPLETED_CLASS
    );
    assert_eq!(
        integration_gate.next_missing_stable_promotion_class,
        STATE_API_INTEGRATION_NEXT_CLASS
    );
    assert_eq!(
        integration_gate.record_queries.len(),
        integration_gate
            .target_summaries
            .iter()
            .map(|summary| summary.source_record_count)
            .sum::<usize>()
    );
    assert!(
        integration_gate
            .target_summaries
            .iter()
            .any(|summary| summary.target == "go"
                && summary.source_record_count == go_report_set.source_paths.len())
    );
    assert!(
        integration_gate
            .integration_surface
            .capabilities
            .iter()
            .any(|capability| capability == "bpe_raw_expansion_query")
    );

    let go_api = TargetNeutralGeneratedCodecStateApi::from_go_reports(
        state_api_report,
        strict_recovery_report,
    )
    .expect("Go reports should expose target-neutral state API");
    let accepted = go_api
        .candidate_raw_token_transition("file_decls_actual_alternative_node_start_is_accepted")
        .expect("Go transition query should resolve");
    let bpe_result = go_api
        .bpe_raw_expansion_query(
            "file_decls_actual_alternative_node_start_is_accepted",
            &[accepted.candidate_raw_token_id],
        )
        .expect("Go BPE raw expansion should use the transition API");
    assert!(bpe_result.accepted);
    assert_eq!(
        bpe_result.validity_authority,
        "target-neutral raw transition API"
    );
    assert!(!bpe_result.bpe_layer_defines_validity);
    let go_multi = go_api
        .bpe_raw_expansion_replay_query(&[
            "file_decls_actual_alternative_node_start_is_accepted",
            "file_decls_after_actual_alternative_first_field_start_is_accepted",
        ])
        .expect("Go multi-token BPE raw expansion should replay through transitions");
    assert!(go_multi.accepted);
    assert_eq!(go_multi.expanded_raw_token_count, 2);
    assert_eq!(go_multi.accepted_raw_token_count, 2);
    assert_eq!(go_multi.final_observed_status, "partial");
    assert!(!go_multi.final_payload_partial);
    assert!(!go_multi.final_eof_valid);
    assert!(go_multi.first_failure_diagnostic.is_none());

    let go_rejected = go_api
        .bpe_raw_expansion_replay_query(&[
            "file_decls_actual_alternative_node_start_is_accepted",
            "file_decls_after_actual_alternative_pad_token_is_rejected",
        ])
        .expect("Go rejected multi-token BPE raw expansion should report first failure");
    assert!(!go_rejected.accepted);
    assert_eq!(go_rejected.expanded_raw_token_count, 2);
    assert_eq!(go_rejected.accepted_raw_token_count, 1);
    assert_eq!(
        go_rejected.first_failure_transition_case_id.as_deref(),
        Some("file_decls_after_actual_alternative_pad_token_is_rejected")
    );
    assert_eq!(
        go_rejected.first_failure_error_kind.as_deref(),
        Some("pad_token_in_raw_stream")
    );
    assert!(
        go_rejected
            .first_failure_diagnostic
            .as_deref()
            .is_some_and(|diagnostic| diagnostic.contains("PAD"))
    );
    let go_record_query = integration_gate
        .record_queries
        .iter()
        .find(|record| record.target == "go" && record.record_id == state_api_report.source_path)
        .expect("Go integration record query should be present");
    let go_mask = go_api
        .admissible_token_mask_contract(go_record_query)
        .expect("Go admissible-token mask contract should consume the state API");
    assert!(go_mask.finite_admissible_raw_token_count > 0);
    assert!(go_mask.payload_range_count > 0);
    assert!(go_mask.payload_range_token_count > 0);
    assert!(go_mask.payload_end_admissible);
    assert!(go_mask.eof_valid);
    assert_eq!(go_mask.eof_admissible_raw_token_count, 0);
    assert_eq!(go_mask.bpe_raw_expansion_replay_case_count, 2);
    assert_eq!(go_mask.bpe_raw_expansion_accepted_case_count, 1);
    assert_eq!(go_mask.bpe_raw_expansion_first_failure_case_count, 1);
    assert_eq!(go_mask.bpe_first_failure_diagnostic_count, 1);
    assert_eq!(
        go_mask.validity_authority,
        "target-neutral raw transition API"
    );
    assert!(!go_mask.bpe_layer_defines_validity);
    assert!(!go_mask.bpe_ids_used_as_syntax_validity_authority);
    let go_continuation = go_api
        .prefix_state_continuation_contract(go_record_query)
        .expect("Go prefix-state continuation contract should consume the route-owned mask");
    assert_eq!(
        go_continuation.request_count,
        go_record_query.prefix_state_case_ids.len()
    );
    assert_eq!(go_continuation.response_count, 4);
    assert!(
        go_continuation
            .responses
            .iter()
            .any(
                |response| response.request_prefix_state_case_id == "complete_stream"
                    && response.eof_valid
                    && response.finite_admissible_raw_token_ids.is_empty()
            )
    );
    assert!(go_continuation.responses.iter().any(
        |response| response.payload_end_admissible && !response.payload_token_ranges.is_empty()
    ));
    assert_eq!(go_continuation.bpe_raw_expansion_replay_case_count, 2);
    assert_eq!(go_continuation.bpe_raw_expansion_accepted_case_count, 1);
    assert_eq!(
        go_continuation.bpe_raw_expansion_first_failure_case_count,
        1
    );
    assert_eq!(go_continuation.bpe_first_failure_diagnostic_count, 1);
    assert_eq!(
        go_continuation.validity_authority,
        "target-neutral raw transition API"
    );
    assert!(!go_continuation.bpe_layer_defines_validity);
    assert!(!go_continuation.bpe_ids_used_as_syntax_validity_authority);

    let csharp_record = csharp_parity_report
        .corpus_replay_records
        .iter()
        .find(|record| record.manifest_path == "serialization/Loan.cs")
        .expect("C# Loan.cs corpus record should be present");
    let csharp_api = TargetNeutralGeneratedCodecStateApi::from_csharp_corpus_record(
        &csharp_parity_report,
        csharp_record,
    )
    .expect("C# corpus record should expose target-neutral state API");
    assert!(
        csharp_api
            .eof_validity("csharp_serialization_loan_cs_complete_stream")
            .expect("C# EOF query should resolve")
    );
    assert!(
        !csharp_api
            .candidate_raw_token_transition(
                "csharp_serialization_loan_cs_pad_token_candidate_is_rejected"
            )
            .expect("C# rejected transition query should resolve")
            .accepted
    );
    let csharp_multi = csharp_api
        .bpe_raw_expansion_replay_query(&[
            "csharp_serialization_loan_cs_first_field_next_segment_is_accepted",
            "csharp_serialization_loan_cs_after_first_segment_next_segment_is_accepted",
        ])
        .expect("C# multi-token BPE raw expansion should replay through transitions");
    assert!(csharp_multi.accepted);
    assert_eq!(csharp_multi.expanded_raw_token_count, 2);
    assert_eq!(csharp_multi.accepted_raw_token_count, 2);
    assert_eq!(csharp_multi.final_observed_status, "partial");
    assert!(!csharp_multi.final_payload_partial);
    assert!(!csharp_multi.final_eof_valid);

    let csharp_rejected = csharp_api
        .bpe_raw_expansion_replay_query(&[
            "csharp_serialization_loan_cs_first_field_next_segment_is_accepted",
            "csharp_serialization_loan_cs_after_first_segment_pad_token_is_rejected",
        ])
        .expect("C# rejected multi-token BPE raw expansion should report first failure");
    assert!(!csharp_rejected.accepted);
    assert_eq!(csharp_rejected.accepted_raw_token_count, 1);
    assert_eq!(
        csharp_rejected.first_failure_transition_case_id.as_deref(),
        Some("csharp_serialization_loan_cs_after_first_segment_pad_token_is_rejected")
    );
    assert_eq!(
        csharp_rejected.first_failure_error_kind.as_deref(),
        Some("pad_token_in_raw_stream")
    );
    let csharp_record_query = integration_gate
        .record_queries
        .iter()
        .find(|record| record.target == "csharp" && record.record_id == "serialization/Loan.cs")
        .expect("C# Loan.cs integration record query should be present");
    let csharp_mask = csharp_api
        .admissible_token_mask_contract(csharp_record_query)
        .expect("C# admissible-token mask contract should consume the state API");
    assert!(csharp_mask.finite_admissible_raw_token_count > 0);
    assert!(csharp_mask.payload_range_count > 0);
    assert!(csharp_mask.payload_range_token_count > 0);
    assert!(csharp_mask.payload_end_admissible);
    assert!(csharp_mask.eof_valid);
    assert_eq!(csharp_mask.eof_admissible_raw_token_count, 0);
    assert_eq!(csharp_mask.bpe_raw_expansion_replay_case_count, 2);
    assert_eq!(csharp_mask.bpe_raw_expansion_accepted_case_count, 1);
    assert_eq!(csharp_mask.bpe_raw_expansion_first_failure_case_count, 1);
    assert_eq!(csharp_mask.bpe_first_failure_diagnostic_count, 1);
    assert_eq!(
        csharp_mask.validity_authority,
        "target-neutral raw transition API"
    );
    assert!(!csharp_mask.bpe_layer_defines_validity);
    assert!(!csharp_mask.bpe_ids_used_as_syntax_validity_authority);
    let csharp_continuation = csharp_api
        .prefix_state_continuation_contract(csharp_record_query)
        .expect("C# prefix-state continuation contract should consume the route-owned mask");
    assert_eq!(
        csharp_continuation.request_count,
        csharp_record_query.prefix_state_case_ids.len()
    );
    assert_eq!(csharp_continuation.response_count, 4);
    assert!(csharp_continuation.responses.iter().any(|response| {
        response.request_prefix_state_case_id == "csharp_serialization_loan_cs_complete_stream"
            && response.eof_valid
            && response.finite_admissible_raw_token_ids.is_empty()
    }));
    assert!(csharp_continuation.responses.iter().any(
        |response| response.payload_end_admissible && !response.payload_token_ranges.is_empty()
    ));
    assert_eq!(csharp_continuation.bpe_raw_expansion_replay_case_count, 2);
    assert_eq!(csharp_continuation.bpe_raw_expansion_accepted_case_count, 1);
    assert_eq!(
        csharp_continuation.bpe_raw_expansion_first_failure_case_count,
        1
    );
    assert_eq!(csharp_continuation.bpe_first_failure_diagnostic_count, 1);
    assert_eq!(
        csharp_continuation.validity_authority,
        "target-neutral raw transition API"
    );
    assert!(!csharp_continuation.bpe_layer_defines_validity);
    assert!(!csharp_continuation.bpe_ids_used_as_syntax_validity_authority);
}

fn admissible_mask_contains(state: &RawSyntaxState, token: u32) -> bool {
    state.admissible.finite_raw_token_ids.contains(&token)
        || state
            .admissible
            .raw_token_ranges
            .iter()
            .any(|range| token >= range.start && token < range.end_exclusive)
}

fn equivalence_probe_candidates(
    roles: &SyntaxTokenRoles,
    alternative_sample_limit: Option<usize>,
) -> Vec<u32> {
    let mut candidates = vec![
        roles.pad_token,
        roles.node_start,
        roles.node_end,
        roles.field_start,
        roles.field_end,
        roles.field_missing,
        roles.repeated_empty,
        roles.terminal,
        roles.payload_end,
        roles.boolean_false,
        roles.boolean_true,
        roles.utf8_range.start,
        roles.utf8_range.end_exclusive - 1,
        roles.integer_range.start,
        roles.integer_range.end_exclusive - 1,
        roles.vocab_size,
    ];
    let alternatives: Vec<u32> = roles
        .node_start_alternative_tokens
        .values()
        .copied()
        .collect();
    match alternative_sample_limit {
        Some(limit) if alternatives.len() > limit.max(1) => {
            let stride = alternatives.len() / limit.max(1);
            candidates.extend(alternatives.into_iter().step_by(stride.max(1)).take(limit));
        }
        _ => candidates.extend(alternatives),
    }
    candidates.sort_unstable();
    candidates.dedup();
    candidates
}

fn exhaustive_prefix_lengths(raw_token_count: usize) -> BTreeSet<usize> {
    (0..=raw_token_count).collect()
}

/// The replay-shaped transition API is O(stream length) per probe, so
/// replay-probing every prefix of a long record stream is O(n^2). The
/// replay-reference comparisons sample stratified strides plus a bounded,
/// evenly spaced subset of marker-adjacent positions, where the mask/replay
/// divergence historically lived. The incremental walker probes every
/// prefix exhaustively regardless of this sample (see
/// `assert_transition_acceptance_matches_mask`); the witness corpus keeps
/// exhaustive replay coverage of every structural shape.
fn sampled_prefix_lengths(
    raw_tokens: &tct::syntax_go_source_instance_projection::SourceInstanceProjectionRawTokenIds,
    roles: &SyntaxTokenRoles,
    target_samples: usize,
) -> BTreeSet<usize> {
    let raw_token_count = raw_tokens.raw_token_ids.len();
    let mut lengths = BTreeSet::new();
    lengths.insert(0);
    lengths.insert(raw_token_count);
    let stride = (raw_token_count / target_samples.max(1)).max(1);
    let mut step = 0;
    while step <= raw_token_count {
        lengths.insert(step);
        step += stride;
    }
    let marker_indices: Vec<usize> = raw_tokens
        .raw_token_ids
        .iter()
        .enumerate()
        .filter(|&(_, &token)| token == roles.field_missing || token == roles.repeated_empty)
        .map(|(index, _)| index)
        .collect();
    let marker_sample_target = 16usize;
    let marker_stride = (marker_indices.len() / marker_sample_target.max(1)).max(1);
    for index in marker_indices.into_iter().step_by(marker_stride) {
        lengths.insert(index);
        lengths.insert(index + 1);
        lengths.insert(index + 2);
    }
    lengths
}

/// Dual-path equivalence: the replay API (`state_from_prefix`/`transition`)
/// stays the reference implementation, and the incremental
/// `RawSyntaxStream` walker is compared field-for-field against it at every
/// replay-probed prefix. Snapshot and error comparisons are exact
/// `assert_eq!` including diagnostic strings: both paths share the engine's
/// state/error builders, so any divergence is a contract break, not wording
/// drift.
///
/// Probing-design choice (recorded per the incremental-API tranche): the
/// incremental side probes EVERY candidate at EVERY prefix — `admits()`
/// versus live-mask membership, fork+advance for admitted candidates with
/// delta/accessor consistency, and in-place rejected advances with
/// post-`Err` snapshot equality (error-atomicity). Exhaustive incremental
/// probing of long real-source streams is cheap because mask access,
/// `admits()`, and fork-or-reuse advances are O(1)-amortized per token. The
/// replay reference stays at `replay_prefix_lengths` (sampled for real-source
/// streams) because each replay probe is O(prefix), so an exhaustive replay
/// leg would be O(n^2); at those prefixes the incremental and replay paths
/// are compared field-for-field, including per-candidate `transition`
/// outcomes. Default coverage is witness seeds + httptest depth + a bounded
/// current-plus sample; full-corpus census walks remain ignored generators.
///
/// Error-atomicity and erred-handle reuse: rejected candidates are advanced
/// on the primary walker itself (no fork). The post-`Err` snapshot must
/// equal the pre-advance snapshot, and the walker must then accept the next
/// canonical stream token, so a handle that returned `Err` from `advance`
/// is proven reusable at every prefix of the corpus, not discard-only.
fn assert_transition_acceptance_matches_mask(
    model: &CompiledSyntaxModel,
    stream_id: &str,
    raw_token_ids: &[u32],
    candidates: &[u32],
    replay_prefix_lengths: &BTreeSet<usize>,
) {
    let mut walker = model.begin_stream();
    for prefix_len in 0..=raw_token_ids.len() {
        let prefix = &raw_token_ids[..prefix_len];
        let live_mask = walker.admissible_next_raw_tokens();
        let pre_snapshot = walker.state_snapshot();
        let replay_state = replay_prefix_lengths.contains(&prefix_len).then(|| {
            model.state_from_prefix(prefix).unwrap_or_else(|error| {
                panic!(
                    "{stream_id}: canonical prefix of {prefix_len} tokens should replay: {error:?}"
                )
            })
        });
        if let Some(state) = replay_state.as_ref() {
            assert_eq!(
                &pre_snapshot, state,
                "{stream_id}: prefix length {prefix_len}: incremental state snapshot must equal the replay-reference state"
            );
            assert_eq!(
                live_mask, state.admissible,
                "{stream_id}: prefix length {prefix_len}: live admissible mask must equal the replay-reference mask"
            );
            assert_eq!(
                walker.consumed_raw_tokens(),
                state.consumed_raw_tokens,
                "{stream_id}: prefix length {prefix_len}: consumed_raw_tokens accessor must match the replay-reference state"
            );
            assert_eq!(
                walker.eof_valid(),
                state.eof_valid,
                "{stream_id}: prefix length {prefix_len}: eof_valid accessor must match the replay-reference state"
            );
            assert_eq!(
                walker.stack_depth(),
                state.stack_depth,
                "{stream_id}: prefix length {prefix_len}: stack_depth accessor must match the replay-reference state"
            );
            assert_eq!(
                walker.active_rule(),
                state.active_rule.as_deref(),
                "{stream_id}: prefix length {prefix_len}: active_rule accessor must match the replay-reference state"
            );
            assert_eq!(
                walker.active_field(),
                state.active_field.as_deref(),
                "{stream_id}: prefix length {prefix_len}: active_field accessor must match the replay-reference state"
            );
        }
        for &candidate in candidates {
            let mask_admits = admissible_mask_contains(&pre_snapshot, candidate);
            assert_eq!(
                walker.admits(candidate),
                mask_admits,
                "{stream_id}: prefix length {prefix_len}, candidate {candidate}: live admits() must equal admissible-mask membership"
            );
            let transition = replay_state
                .as_ref()
                .map(|_| model.transition(prefix, candidate));
            if let Some(transition) = transition.as_ref() {
                assert_eq!(
                    transition.accepted, mask_admits,
                    "{stream_id}: prefix length {prefix_len}, candidate {candidate}: transition acceptance ({}) must equal admissible-mask membership ({}); transition error: {:?}",
                    transition.accepted, mask_admits, transition.error
                );
            }
            if mask_admits {
                let mut fork = walker.clone();
                let delta = fork.advance(candidate).unwrap_or_else(|error| {
                    panic!(
                        "{stream_id}: prefix length {prefix_len}, candidate {candidate}: incremental advance rejected a mask-admitted candidate: {error:?}"
                    )
                });
                assert_eq!(
                    delta.applied_raw_token_id, candidate,
                    "{stream_id}: prefix length {prefix_len}, candidate {candidate}: advance delta must echo the applied raw token"
                );
                assert_eq!(
                    delta.consumed_raw_tokens,
                    prefix_len + 1,
                    "{stream_id}: prefix length {prefix_len}, candidate {candidate}: advance delta must consume exactly one token past the prefix"
                );
                assert_eq!(
                    delta.eof_valid,
                    fork.eof_valid(),
                    "{stream_id}: prefix length {prefix_len}, candidate {candidate}: advance delta eof_valid must match the post-advance handle"
                );
                assert_eq!(
                    delta.stack_depth,
                    fork.stack_depth(),
                    "{stream_id}: prefix length {prefix_len}, candidate {candidate}: advance delta stack_depth must match the post-advance handle"
                );
                if let Some(transition) = transition.as_ref() {
                    let transition_state = transition.state.as_ref().unwrap_or_else(|| {
                        panic!(
                            "{stream_id}: prefix length {prefix_len}, candidate {candidate}: accepted replay transition must carry a state"
                        )
                    });
                    assert_eq!(
                        &fork.state_snapshot(),
                        transition_state,
                        "{stream_id}: prefix length {prefix_len}, candidate {candidate}: post-advance snapshot must equal the replay transition state"
                    );
                    assert_eq!(
                        delta.status, transition_state.status,
                        "{stream_id}: prefix length {prefix_len}, candidate {candidate}: advance delta status must match the replay transition state"
                    );
                    assert_eq!(
                        delta.payload_active,
                        transition_state.active_payload_kind.is_some(),
                        "{stream_id}: prefix length {prefix_len}, candidate {candidate}: advance delta payload_active must match the replay transition state"
                    );
                    assert_eq!(
                        delta.payload_byte_count, transition_state.active_payload_byte_count,
                        "{stream_id}: prefix length {prefix_len}, candidate {candidate}: advance delta payload_byte_count must match the replay transition state"
                    );
                }
            } else {
                let error = match walker.advance(candidate) {
                    Ok(delta) => panic!(
                        "{stream_id}: prefix length {prefix_len}, candidate {candidate}: incremental advance accepted a mask-forbidden candidate: {delta:?}"
                    ),
                    Err(error) => error,
                };
                if let Some(transition) = transition.as_ref() {
                    assert_eq!(
                        Some(&error),
                        transition.error.as_ref(),
                        "{stream_id}: prefix length {prefix_len}, candidate {candidate}: incremental advance error must equal the replay transition error"
                    );
                }
                assert_eq!(
                    walker.state_snapshot(),
                    pre_snapshot,
                    "{stream_id}: prefix length {prefix_len}, candidate {candidate}: post-Err snapshot must equal the pre-advance state (error-atomic advance)"
                );
            }
        }
        if prefix_len < raw_token_ids.len() {
            let token = raw_token_ids[prefix_len];
            let delta = walker.advance(token).unwrap_or_else(|error| {
                panic!(
                    "{stream_id}: walker that absorbed rejected probes at prefix length {prefix_len} must remain usable and accept canonical token {token}: {error:?}"
                )
            });
            assert_eq!(
                delta.consumed_raw_tokens,
                prefix_len + 1,
                "{stream_id}: advance delta for canonical token at offset {prefix_len} must report the consumed-token count"
            );
        }
    }
}

#[test]
#[ignore = "known failure: checked httptest-server projection report keeps per-file grammar id go_httptest_server_source_file_syntax_v0 without a per_file_grammar_blocker; see docs/frontends-development.md"]
fn raw_replay_rejects_marker_streams_the_mask_forbids() {
    let report = checked_report();
    let sidecar = checked_go_source_instance_sidecar(&report);
    let artifact_contents = regenerated_projection_artifact_for(&report);
    let artifact =
        parse_projection_artifact(&artifact_contents).expect("projection artifact should parse");
    let raw_tokens = artifact
        .raw_token_ids_against_report(&report)
        .expect("projection artifact should emit raw token IDs");
    let model = CompiledSyntaxModel::from_sidecar_and_layout(&sidecar, &raw_tokens.token_layout)
        .expect("raw syntax state machine should compile from StructuralModel");
    let roles = model.token_roles().clone();

    let stream = &raw_tokens.raw_token_ids;
    let marker_segment = raw_tokens
        .raw_token_segments
        .iter()
        .find(|segment| segment.event_kind == "field_missing")
        .expect("checked stream should contain a field_missing marker");
    let marker_index = marker_segment.raw_token_offset;
    assert_eq!(
        stream[marker_index], roles.field_missing,
        "field_missing segment should point at the marker token"
    );
    assert_eq!(
        stream[marker_index + 1],
        roles.field_end,
        "field_missing marker should be followed by field_end in the canonical stream"
    );

    let error = model
        .decode_complete(stream)
        .map(|state| state.status)
        .expect("canonical checked stream must keep strict-decoding Complete");
    assert_eq!(error, RawSyntaxStatus::Complete);

    let mut duplicated_marker = stream.clone();
    duplicated_marker.insert(marker_index + 1, roles.field_missing);
    let duplicated_error = model.decode_complete(&duplicated_marker).expect_err(
        "a duplicated field_missing marker is mask-forbidden and must not strict-decode",
    );
    assert_eq!(duplicated_error.kind, "unexpected_structural_token");

    let marked_prefix = &stream[..=marker_index];
    let marker_state = model
        .state_from_prefix(marked_prefix)
        .expect("prefix ending at the marker should replay");
    assert_eq!(
        marker_state.admissible.finite_raw_token_ids,
        vec![roles.field_end],
        "after a missing marker the mask admits only field_end"
    );
    let second_marker = model.transition(marked_prefix, roles.field_missing);
    assert!(
        !second_marker.accepted,
        "transition must reject a second field_missing marker the mask forbids"
    );

    let mut unmarked_empty_field = stream.clone();
    unmarked_empty_field.remove(marker_index);
    let unmarked_error = model.decode_complete(&unmarked_empty_field).expect_err(
        "closing an optional field without a value or marker is mask-forbidden and must not strict-decode",
    );
    assert_eq!(unmarked_error.kind, "unexpected_structural_token");

    let terminal_index = (0..stream.len())
        .find(|&index| {
            stream[index] == roles.terminal
                && stream.get(index + 1).is_some_and(|&next| {
                    next >= roles.utf8_range.start && next < roles.utf8_range.end_exclusive
                })
        })
        .expect("checked stream should contain a terminal with a UTF-8 payload");
    let mut invalid_utf8_prefix = stream[..=terminal_index].to_vec();
    let continuation_byte_token = roles.utf8_range.start + 0x80;
    let byte_step = model.transition(&invalid_utf8_prefix, continuation_byte_token);
    assert!(
        byte_step.accepted,
        "a lone continuation byte is mask-admissible inside a UTF-8 payload"
    );
    invalid_utf8_prefix.push(continuation_byte_token);
    let invalid_state = model
        .state_from_prefix(&invalid_utf8_prefix)
        .expect("prefix with invalid UTF-8 payload bytes should still replay");
    assert!(
        !invalid_state.admissible.payload_end_admissible
            && !invalid_state
                .admissible
                .finite_raw_token_ids
                .contains(&roles.payload_end),
        "payload_end must not be mask-admissible while payload bytes are invalid UTF-8"
    );
    let invalid_end = model.transition(&invalid_utf8_prefix, roles.payload_end);
    assert!(
        !invalid_end.accepted,
        "transition must reject payload_end while payload bytes are invalid UTF-8"
    );
}

#[test]
#[ignore = "known failure: checked httptest-server projection report keeps per-file grammar id go_httptest_server_source_file_syntax_v0 without a per_file_grammar_blocker; see docs/frontends-development.md"]
fn transition_acceptance_matches_admissible_mask_over_checked_corpora() {
    let grammar = GoAstSchemaInventoryAdapter::lower_inventory_file(Path::new(
        GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH,
    ))
    .expect("checked Go AST schema inventory should lower to GrammarIR");
    let witness_sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout())
        .expect("inventory-derived GrammarIR should lower through StructuralModel");
    let witness_model = CompiledSyntaxModel::from_sidecar(&witness_sidecar)
        .expect("inventory-derived StructuralModel should compile to raw state machine");
    let witness_candidates = equivalence_probe_candidates(witness_model.token_roles(), None);
    let seeds = go_ast_inventory_scratch_seeds(&witness_model)
        .expect("scratch seed list should build all seeds");
    assert!(!seeds.is_empty());
    for seed in &seeds {
        assert_transition_acceptance_matches_mask(
            &witness_model,
            &seed.seed_id,
            &seed.raw_token_ids,
            &witness_candidates,
            &exhaustive_prefix_lengths(seed.raw_token_ids.len()),
        );
    }

    // Depth record: dedicated httptest fixture (outside the current-plus
    // ingestion set) still anchors the long-stream sampled-replay path.
    assert_transition_acceptance_matches_mask_on_source_instance_report(&checked_report());

    // Breadth sample: N smallest current-plus records, now cheap enough for
    // the default gate because the incremental RawSyntaxStream handle probes
    // every prefix while replay stays sampled.
    let current_plus_sample = current_plus_equivalence_property_sample();
    for checked in &current_plus_sample {
        assert_eq!(
            checked.report.candidate_grammar_id, GO_UNIFIED_GRAMMAR_ID,
            "{}: current-plus equivalence sample must use the unified inventory grammar",
            checked.report.source.path
        );
        assert_transition_acceptance_matches_mask_on_source_instance_report(&checked.report);
    }
}

/// Replay-only delegation wrapper over the compiled model. It keeps the
/// trait's default `begin_raw_stream` (no incremental handle), so the
/// scratch-seed admissibility proof exercises its replay-shaped fallback
/// path instead of the incremental walker.
struct ReplayOnlySyntaxCore<'a> {
    model: &'a CompiledSyntaxModel,
}

impl RawSyntaxStateMachineCore for ReplayOnlySyntaxCore<'_> {
    fn token_roles(&self) -> &SyntaxTokenRoles {
        self.model.token_roles()
    }

    fn state_after_raw_prefix(
        &self,
        raw_token_ids: &[u32],
    ) -> Result<RawSyntaxState, RawSyntaxError> {
        self.model.state_from_prefix(raw_token_ids)
    }

    fn candidate_raw_token_transition(
        &self,
        raw_prefix_token_ids: &[u32],
        candidate_raw_token_id: u32,
    ) -> RawSyntaxTransition {
        self.model
            .transition(raw_prefix_token_ids, candidate_raw_token_id)
    }

    fn strict_decode_complete(
        &self,
        raw_token_ids: &[u32],
    ) -> Result<RawSyntaxState, RawSyntaxError> {
        self.model.decode_complete(raw_token_ids)
    }

    fn decode_complete_tree(
        &self,
        raw_token_ids: &[u32],
    ) -> Result<RawSyntaxDecodedTree, RawSyntaxError> {
        self.model.decode_complete_tree(raw_token_ids)
    }

    fn selected_completion_trace(
        &self,
        raw_token_ids: &[u32],
        sample_steps: &BTreeSet<usize>,
    ) -> Result<RawSyntaxSelectedCompletionTrace, RawSyntaxError> {
        self.model
            .selected_completion_trace(raw_token_ids, sample_steps)
    }
}

/// The scratch-seed admissibility proof walks one incremental
/// `RawSyntaxStream` when the core exposes one (the compiled model does)
/// and keeps the replay-shaped probe loop as the fallback for trait
/// implementations without incremental access. The two execution paths must
/// be observably identical: equal proofs on every checked witness seed and
/// equal typed errors on rejected streams.
#[test]
fn scratch_seed_admissibility_incremental_path_matches_replay_fallback() {
    let grammar = GoAstSchemaInventoryAdapter::lower_inventory_file(Path::new(
        GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH,
    ))
    .expect("checked Go AST schema inventory should lower to GrammarIR");
    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout())
        .expect("inventory-derived GrammarIR should lower through StructuralModel");
    let model = CompiledSyntaxModel::from_sidecar(&sidecar)
        .expect("inventory-derived StructuralModel should compile to raw state machine");
    let replay_only = ReplayOnlySyntaxCore { model: &model };
    assert!(
        replay_only.begin_raw_stream().is_none(),
        "replay-only wrapper must keep the default begin_raw_stream and exercise the fallback"
    );
    assert!(
        RawSyntaxStateMachineCore::begin_raw_stream(&model).is_some(),
        "compiled model must expose the incremental stream handle through the core trait"
    );

    let seeds =
        go_ast_inventory_scratch_seeds(&model).expect("scratch seed list should build all seeds");
    assert!(!seeds.is_empty());
    for seed in &seeds {
        let incremental = verify_go_scratch_seed_admissibility(&model, &seed.raw_token_ids)
            .unwrap_or_else(|error| {
                panic!(
                    "{}: incremental admissibility proof should pass: {error}",
                    seed.seed_id
                )
            });
        let replay = verify_go_scratch_seed_admissibility(&replay_only, &seed.raw_token_ids)
            .unwrap_or_else(|error| {
                panic!(
                    "{}: replay-fallback admissibility proof should pass: {error}",
                    seed.seed_id
                )
            });
        assert_eq!(
            incremental, replay,
            "{}: incremental and replay-fallback admissibility proofs must be identical",
            seed.seed_id
        );
    }

    let seed = &seeds[0];
    let roles = model.token_roles();
    let mut forbidden = seed.raw_token_ids.clone();
    forbidden.insert(1, roles.vocab_size);
    let incremental_error = verify_go_scratch_seed_admissibility(&model, &forbidden)
        .expect_err("an out-of-vocab token is never admissible on the incremental path");
    let replay_error = verify_go_scratch_seed_admissibility(&replay_only, &forbidden)
        .expect_err("an out-of-vocab token is never admissible on the replay fallback");
    assert_eq!(
        incremental_error, replay_error,
        "rejected-token errors must be identical on both admissibility paths"
    );

    let truncated = &seed.raw_token_ids[..seed.raw_token_ids.len() - 1];
    let incremental_error = verify_go_scratch_seed_admissibility(&model, truncated)
        .expect_err("a truncated seed stream must fail the strict-decode anchor (incremental)");
    let replay_error = verify_go_scratch_seed_admissibility(&replay_only, truncated)
        .expect_err("a truncated seed stream must fail the strict-decode anchor (fallback)");
    assert_eq!(
        incremental_error, replay_error,
        "truncated-stream errors must be identical on both admissibility paths"
    );
}

/// TDD guards for the layout-validation hardening: a pad token inside a
/// payload range, a range beyond vocab_size, an inverted range, or an
/// alternative token colliding with the fixed role offsets must all be
/// rejected when roles are built from a layout summary.
#[test]
#[ignore = "known failure: checked httptest-server projection report keeps per-file grammar id go_httptest_server_source_file_syntax_v0 without a per_file_grammar_blocker; see docs/frontends-development.md"]
fn token_roles_validation_rejects_malformed_layouts() {
    let report = checked_report();
    let sidecar = checked_go_source_instance_sidecar(&report);
    let valid_layout =
        tct::syntax_go_source_instance_projection::ProjectionTokenLayoutSummary::from(
            &sidecar.structural_model.token_ranges,
        );
    assert!(CompiledSyntaxModel::from_sidecar_and_layout(&sidecar, &valid_layout).is_ok());

    let mut pad_in_payload = valid_layout.clone();
    pad_in_payload.pad_token = pad_in_payload
        .ranges
        .iter()
        .find(|range| range.role == "utf8_byte_payload")
        .expect("layout should expose the utf8 payload range")
        .start;
    let error = CompiledSyntaxModel::from_sidecar_and_layout(&sidecar, &pad_in_payload)
        .expect_err("a pad token inside a payload range must be rejected");
    assert_eq!(error.field(), "syntax_state_machine.token_layout.pad_token");

    let mut pad_beyond_vocab = valid_layout.clone();
    pad_beyond_vocab.pad_token = pad_beyond_vocab.vocab_size;
    let error = CompiledSyntaxModel::from_sidecar_and_layout(&sidecar, &pad_beyond_vocab)
        .expect_err("a pad token at or beyond vocab size must be rejected");
    assert_eq!(error.field(), "syntax_state_machine.token_layout.pad_token");

    let mut range_beyond_vocab = valid_layout.clone();
    range_beyond_vocab.vocab_size = range_beyond_vocab
        .ranges
        .iter()
        .map(|range| range.end_exclusive)
        .max()
        .expect("layout should have ranges")
        - 1;
    let error = CompiledSyntaxModel::from_sidecar_and_layout(&sidecar, &range_beyond_vocab)
        .expect_err("a range ending beyond vocab size must be rejected");
    assert_eq!(error.field(), "syntax_state_machine.token_layout.range");

    let mut inverted_range = valid_layout.clone();
    for range in &mut inverted_range.ranges {
        if range.role == "integer_lexeme_payload" || range.role == "u8_integer_payload" {
            range.end_exclusive = range.start;
        }
    }
    let error = CompiledSyntaxModel::from_sidecar_and_layout(&sidecar, &inverted_range)
        .expect_err("an empty or inverted range must be rejected");
    assert_eq!(error.field(), "syntax_state_machine.token_layout.range");
}

const FORCED_SHARE_CENSUS_REPORT_PATH: &str = "target/forced-share-census-post-elision.json";
const FORCED_SHARE_CENSUS_ARTIFACT_KIND: &str = "tct_go_mask_forced_token_share_census_v2";
const FORCED_SHARE_CENSUS_CLAIM_BOUNDARY: &str = "Post-elision mask-forced token census over the \
    142-record current-plus Go stdlib corpus under the unified pinned inventory grammar artifact \
    and the elided canonical stream contract (raw-syntax-stream-elided-v1). The canonical \
    (elided) layer is walked on the elided handle and must contain zero mask-forced positions; \
    the expanded layer is reproduced via expand_elided_raw_tokens and class-censused against the \
    same model. Census evidence only: no model, BPE, embedding, source-program, or paper-grade \
    claim.";

const FORCED_CENSUS_CLASS_COUNT: usize = 5;
const FORCED_CENSUS_CLASS_NAMES: [&str; FORCED_CENSUS_CLASS_COUNT] = [
    "fixed_structural_role",
    "payload_end",
    "alternative_choice",
    "utf8_payload_byte",
    "integer_payload_byte",
];

/// Census token class for one consumed raw token, against the per-record
/// roles. `payload_end` is split out of the fixed structural roles so the
/// census can report payload-terminator positions separately.
fn forced_census_class_index(roles: &SyntaxTokenRoles, token: u32) -> usize {
    if token == roles.payload_end {
        return 1;
    }
    let structural = &roles.structural_range;
    if token >= structural.start
        && token < structural.start + RAW_TOKEN_ALTERNATIVE_NODE_START_OFFSET
    {
        return 0;
    }
    if token >= structural.start && token < structural.end_exclusive {
        return 2;
    }
    if token >= roles.utf8_range.start && token < roles.utf8_range.end_exclusive {
        return 3;
    }
    if token >= roles.integer_range.start && token < roles.integer_range.end_exclusive {
        return 4;
    }
    panic!("raw token {token} falls outside every census token class");
}

fn admissible_mask_cardinality(mask: &RawSyntaxAdmissibleTokens) -> usize {
    mask.finite_raw_token_ids.len()
        + mask
            .raw_token_ranges
            .iter()
            .map(|range| (range.end_exclusive - range.start) as usize)
            .sum::<usize>()
}

fn admissible_mask_admits(mask: &RawSyntaxAdmissibleTokens, token: u32) -> bool {
    mask.finite_raw_token_ids.contains(&token)
        || mask
            .raw_token_ranges
            .iter()
            .any(|range| token >= range.start && token < range.end_exclusive)
}

fn current_commit_id() -> String {
    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .expect("git rev-parse HEAD should run");
    assert!(output.status.success(), "git rev-parse HEAD should succeed");
    String::from_utf8(output.stdout)
        .expect("git rev-parse HEAD should emit UTF-8")
        .trim()
        .to_string()
}

/// Census-only view of the current-plus manifest rows: the fields the
/// forced-share census needs beyond `go_tokenizer_generator_report_ingestion_sources`.
#[derive(Debug, serde::Deserialize)]
struct ForcedCensusManifestRecord {
    source_path: String,
    source_sha256: String,
    source_bytes: usize,
    expected_tct_token_count: usize,
    expected_tct_tokens_sha256: String,
}

#[derive(Debug, serde::Deserialize)]
struct ForcedCensusManifest {
    records: Vec<ForcedCensusManifestRecord>,
}

#[derive(Debug, serde::Serialize)]
struct ForcedCensusClassSummary {
    class: &'static str,
    token_count: usize,
    forced_token_count: usize,
    share_of_stream: f64,
    forced_share_within_class: f64,
    share_of_all_forced: f64,
}

#[derive(Debug, serde::Serialize)]
struct ForcedCensusRecordRow {
    manifest_path: String,
    source_path: String,
    candidate_grammar_id: String,
    source_bytes: usize,
    canonical_token_count: usize,
    canonical_forced_position_count: usize,
    expanded_token_count: usize,
    forced_token_count: usize,
    forced_share_of_expanded: f64,
    canonical_tokens_per_byte: f64,
    expanded_tokens_per_byte: f64,
}

#[derive(Debug, serde::Serialize)]
struct ForcedShareCensusReport {
    artifact_kind: &'static str,
    version: u32,
    commit: String,
    manifest_path: &'static str,
    grammar_artifact_id: &'static str,
    stream_contract: &'static str,
    record_count: usize,
    total_source_bytes: usize,
    canonical_token_count: usize,
    canonical_forced_position_count: usize,
    canonical_tokens_per_byte: f64,
    expanded_token_count: usize,
    expanded_forced_token_count: usize,
    expanded_forced_share: f64,
    expanded_tokens_per_byte: f64,
    mask_empty_or_token_not_admissible_error_count: usize,
    classes: Vec<ForcedCensusClassSummary>,
    claim_boundary: &'static str,
    records: Vec<ForcedCensusRecordRow>,
}

/// Census evidence generator for the landed elision contract: walks every
/// checked current-plus record on both stream layers against the unified
/// pinned inventory grammar artifact.
///
/// Canonical (elided) layer: the regenerated canonical stream is
/// manifest-pinned (length and `json_sha256`), walked token-by-token on the
/// incremental `ElidedRawSyntaxStream` handle, and must contain zero
/// mask-forced positions (the no-forced invariant of
/// `raw-syntax-stream-elided-v1`).
///
/// Expanded layer: `expand_elided_raw_tokens` reproduces the expanded
/// stream, which is class-censused with the mask-cardinality detector; the
/// detector's forced count must equal the elide/expand length difference
/// (the per-record forced counts the elision break recorded).
///
/// Ignored by default: this is a census evidence generator, not a
/// regression gate. It regenerates all 142 projection artifacts through the
/// Go projection helper before walking ~3.9M stream positions, and its
/// output (`target/forced-share-census-post-elision.json`) feeds the
/// checked census report under `reports/`. Run with:
///
/// ```text
/// cargo test --test syntax_state_machine_replay_test \
///   go_mask_forced_token_share_census_over_current_plus_records -- --ignored --exact
/// ```
#[test]
#[ignore = "evidence generator: regenerates the mask-forced token census report from the 142-record corpus; run manually when refreshing reports/go-mask-forced-token-census evidence"]
fn go_mask_forced_token_share_census_over_current_plus_records() {
    let manifest: ForcedCensusManifest = serde_json::from_str(
        &fs::read_to_string(GO_CURRENT_PLUS_REPORT_INGESTION_MANIFEST_PATH)
            .expect("current-plus report-ingestion manifest should exist"),
    )
    .expect("current-plus report-ingestion manifest should parse the census fields");
    let checked_reports = checked_source_instance_report_set();
    assert_eq!(
        manifest.records.len(),
        checked_reports.len(),
        "census manifest rows must match the checked report set in manifest order"
    );

    let mut class_totals = [0usize; FORCED_CENSUS_CLASS_COUNT];
    let mut class_forced = [0usize; FORCED_CENSUS_CLASS_COUNT];
    let mut canonical_token_count = 0usize;
    let mut canonical_forced_position_count = 0usize;
    let mut expanded_token_count = 0usize;
    let mut expanded_forced_token_count = 0usize;
    let mut total_source_bytes = 0usize;
    let mut error_contexts: Vec<String> = Vec::new();
    let mut record_rows = Vec::with_capacity(checked_reports.len());

    for (manifest_record, checked) in manifest.records.iter().zip(&checked_reports) {
        let report = &checked.report;
        assert_eq!(
            manifest_record.source_path, checked.manifest_source.source_path,
            "census manifest row and checked report set must stay in manifest order"
        );
        assert_eq!(
            manifest_record.source_sha256, checked.manifest_source.source_sha256,
            "{}: census manifest row must pin the same source",
            manifest_record.source_path
        );
        assert_eq!(
            report.candidate_grammar_id, GO_UNIFIED_GRAMMAR_ID,
            "{}: every current-plus record must carry the unified inventory grammar id",
            manifest_record.source_path
        );

        let sidecar = checked_go_source_instance_sidecar(report);
        let artifact_contents = regenerated_projection_artifact_for(report);
        let artifact = parse_projection_artifact(&artifact_contents)
            .expect("projection artifact should parse");
        let raw_tokens = artifact
            .raw_token_ids_against_report(report)
            .expect("projection artifact should emit raw token IDs");
        let model =
            CompiledSyntaxModel::from_sidecar_and_layout(&sidecar, &raw_tokens.token_layout)
                .expect("unified StructuralModel should compile to a raw state machine");
        let expanded = &raw_tokens.raw_token_ids;

        // Canonical layer: the elided stream is the manifest-pinned
        // encoded stream; its re-expansion must reproduce the expanded
        // projection stream token-for-token.
        let elided = model
            .elide_forced_raw_tokens(expanded)
            .unwrap_or_else(|error| {
                panic!(
                    "{}: regenerated expanded stream must elide cleanly: {error:?}",
                    manifest_record.source_path
                )
            });
        assert_eq!(
            elided.len(),
            manifest_record.expected_tct_token_count,
            "{}: canonical elided stream length must match the manifest expectation",
            manifest_record.source_path
        );
        let elided_sha256 =
            tct::provenance::json_sha256(&elided).expect("canonical elided stream should hash");
        assert_eq!(
            elided_sha256, manifest_record.expected_tct_tokens_sha256,
            "{}: canonical elided stream hash must match the manifest expectation",
            manifest_record.source_path
        );
        let expanded_round_trip = model
            .expand_elided_raw_tokens(&elided)
            .unwrap_or_else(|error| {
                panic!(
                    "{}: canonical elided stream must re-expand: {error:?}",
                    manifest_record.source_path
                )
            });
        assert_eq!(
            &expanded_round_trip, expanded,
            "{}: expand(elide(s)) must reproduce the expanded projection stream",
            manifest_record.source_path
        );

        // Canonical layer walk: the elided handle must never sit on a
        // singleton (or empty) mask at a consumed-token position.
        let mut elided_walker = model.begin_elided_stream().unwrap_or_else(|error| {
            panic!(
                "{}: elided walker construction must drain the initial run: {error:?}",
                manifest_record.source_path
            )
        });
        let mut record_canonical_forced = 0usize;
        for (elided_index, &token) in elided.iter().enumerate() {
            let mask = elided_walker.admissible_next_raw_tokens();
            let cardinality = admissible_mask_cardinality(&mask);
            assert!(
                admissible_mask_admits(&mask, token),
                "{}: canonical token {token} at elided index {elided_index} must be \
                 mask-admissible (mask cardinality {cardinality})",
                manifest_record.source_path
            );
            if cardinality <= 1 {
                record_canonical_forced += 1;
            }
            elided_walker.advance(token).unwrap_or_else(|error| {
                panic!(
                    "{}: canonical token {token} at elided index {elided_index} must \
                     advance: {error:?}",
                    manifest_record.source_path
                )
            });
        }
        assert!(
            elided_walker.eof_valid(),
            "{}: canonical elided stream must end in a complete state",
            manifest_record.source_path
        );
        assert_eq!(
            record_canonical_forced, 0,
            "{}: the canonical elided stream must contain zero mask-forced positions",
            manifest_record.source_path
        );

        // Expanded layer census: the mask-cardinality detector over the
        // re-expanded stream, split by token class.
        let roles = model.token_roles().clone();
        let mut walker = model.begin_stream();
        let mut record_forced = 0usize;
        let mut record_error = None;
        for (offset, &token) in expanded.iter().enumerate() {
            let mask = walker.admissible_next_raw_tokens();
            let cardinality = admissible_mask_cardinality(&mask);
            if cardinality == 0 || !admissible_mask_admits(&mask, token) {
                record_error = Some(format!(
                    "{}: raw token {token} at offset {offset} is not admissible \
                     (mask cardinality {cardinality}); every checked record stream replays green",
                    manifest_record.source_path
                ));
                break;
            }
            let class_index = forced_census_class_index(&roles, token);
            class_totals[class_index] += 1;
            if cardinality == 1 {
                class_forced[class_index] += 1;
                record_forced += 1;
            }
            walker.advance(token).unwrap_or_else(|error| {
                panic!(
                    "{}: mask-admitted raw token {token} at offset {offset} must advance: {error:?}",
                    manifest_record.source_path
                )
            });
        }
        if let Some(error) = record_error {
            error_contexts.push(error);
            continue;
        }
        assert!(
            walker.eof_valid(),
            "{}: expanded record stream must end in a complete state",
            manifest_record.source_path
        );
        assert_eq!(
            record_forced,
            expanded.len() - elided.len(),
            "{}: the census detector's forced count must equal the elide/expand length \
             difference recorded by the elision break",
            manifest_record.source_path
        );

        record_rows.push(ForcedCensusRecordRow {
            manifest_path: checked.manifest_source.manifest_path.clone(),
            source_path: manifest_record.source_path.clone(),
            candidate_grammar_id: report.candidate_grammar_id.clone(),
            source_bytes: manifest_record.source_bytes,
            canonical_token_count: elided.len(),
            canonical_forced_position_count: record_canonical_forced,
            expanded_token_count: expanded.len(),
            forced_token_count: record_forced,
            forced_share_of_expanded: record_forced as f64 / expanded.len() as f64,
            canonical_tokens_per_byte: elided.len() as f64 / manifest_record.source_bytes as f64,
            expanded_tokens_per_byte: expanded.len() as f64 / manifest_record.source_bytes as f64,
        });
        canonical_token_count += elided.len();
        canonical_forced_position_count += record_canonical_forced;
        expanded_token_count += expanded.len();
        expanded_forced_token_count += record_forced;
        total_source_bytes += manifest_record.source_bytes;
    }

    assert!(
        error_contexts.is_empty(),
        "no census position may have an empty mask or an inadmissible consumed token:\n{}",
        error_contexts.join("\n")
    );
    assert_eq!(
        canonical_forced_position_count, 0,
        "the canonical elided layer must report zero mask-forced positions corpus-wide"
    );
    assert_eq!(
        class_totals.iter().sum::<usize>(),
        expanded_token_count,
        "every consumed expanded token must land in exactly one census class"
    );
    let expected_canonical_tokens: usize = manifest
        .records
        .iter()
        .map(|record| record.expected_tct_token_count)
        .sum();
    assert_eq!(
        canonical_token_count, expected_canonical_tokens,
        "canonical census token total must match the manifest expectation"
    );
    assert_eq!(
        expanded_token_count - expanded_forced_token_count,
        canonical_token_count,
        "the expanded layer minus its forced positions must reproduce the canonical layer"
    );

    let classes = (0..FORCED_CENSUS_CLASS_COUNT)
        .map(|index| ForcedCensusClassSummary {
            class: FORCED_CENSUS_CLASS_NAMES[index],
            token_count: class_totals[index],
            forced_token_count: class_forced[index],
            share_of_stream: class_totals[index] as f64 / expanded_token_count as f64,
            forced_share_within_class: if class_totals[index] == 0 {
                0.0
            } else {
                class_forced[index] as f64 / class_totals[index] as f64
            },
            share_of_all_forced: if expanded_forced_token_count == 0 {
                0.0
            } else {
                class_forced[index] as f64 / expanded_forced_token_count as f64
            },
        })
        .collect();
    let census = ForcedShareCensusReport {
        artifact_kind: FORCED_SHARE_CENSUS_ARTIFACT_KIND,
        version: 2,
        commit: current_commit_id(),
        manifest_path: GO_CURRENT_PLUS_REPORT_INGESTION_MANIFEST_PATH,
        grammar_artifact_id: GO_UNIFIED_GRAMMAR_ARTIFACT_ID,
        stream_contract: LIVE_RAW_SYNTAX_STREAM_CONTRACT,
        record_count: record_rows.len(),
        total_source_bytes,
        canonical_token_count,
        canonical_forced_position_count,
        canonical_tokens_per_byte: canonical_token_count as f64 / total_source_bytes as f64,
        expanded_token_count,
        expanded_forced_token_count,
        expanded_forced_share: expanded_forced_token_count as f64 / expanded_token_count as f64,
        expanded_tokens_per_byte: expanded_token_count as f64 / total_source_bytes as f64,
        mask_empty_or_token_not_admissible_error_count: error_contexts.len(),
        classes,
        claim_boundary: FORCED_SHARE_CENSUS_CLAIM_BOUNDARY,
        records: record_rows,
    };
    write_json(FORCED_SHARE_CENSUS_REPORT_PATH, &census);
}

// === Elided canonical raw-stream contract (state-machine elide/expand core) ===
//
// Owner contract: tokens whose admissible mask at their position is a
// singleton carry zero information and must not be represented in the
// canonical encoded stream. These properties pin the state-machine-level
// `elide_forced_raw_tokens` / `expand_elided_raw_tokens` / elided strict
// decode core against the replay reference and the checked census evidence
// before any producer switches to the elided contract. The expanded stream
// remains the state machine's native semantics; every elided-path result is
// asserted against the expanded-path reference.

const CHECKED_FORCED_TOKEN_CENSUS_REPORT_PATH: &str =
    "reports/go-mask-forced-token-census-2026-06-11.json";
/// The smallest current-plus record in the checked census report
/// (`fe_arm64_noasm.go`): the per-record census row pins `token_count`,
/// `forced_token_count`, and `elided_token_count` for census-agreement
/// checks against the live eliding function.
const CENSUS_PINNED_RECORD_REPORT_PATH: &str = "tests/fixtures/go_syntax_corpus/source-instance-projection-report-crypto-internal-fips140-edwards25519-field-fe-arm64-noasm-v1.json";

/// Census-agreement view of the checked census report rows: only the
/// per-record fields the eliding function must reproduce.
#[derive(Debug, serde::Deserialize)]
struct CheckedForcedCensusRecordRow {
    source_path: String,
    token_count: usize,
    forced_token_count: usize,
    elided_token_count: usize,
}

#[derive(Debug, serde::Deserialize)]
struct CheckedForcedCensusReport {
    records: Vec<CheckedForcedCensusRecordRow>,
}

/// Checked record streams are regenerated through the Go projection helper,
/// so the elided-stream record properties require the documented toolchain
/// pin. Fail loudly with the pin instead of silently passing on a Go-less
/// or differently versioned machine.
fn require_pinned_go_toolchain() {
    let pin = tct::go_toolchain::PINNED_GO_VERSION;
    let tc = tct::go_toolchain::resolve().unwrap_or_else(|error| {
        panic!(
            "{pin} is required to regenerate checked record streams for the              elided-stream properties; pin resolve failed: {error}"
        )
    });
    assert_eq!(
        tc.version, pin,
        "elided-stream record properties require the pinned {pin} toolchain, found: {}",
        tc.version
    );
}

/// Reference forced-position detector (the census walk): the expanded
/// offsets whose pre-consumption admissible mask is a singleton. The
/// eliding function must agree with this detector exactly; the census
/// evidence in `reports/` was produced by the same walk.
fn census_forced_token_offsets(
    model: &CompiledSyntaxModel,
    stream_id: &str,
    expanded: &[u32],
) -> Vec<usize> {
    let mut walker = model.begin_stream();
    let mut forced_offsets = Vec::new();
    for (offset, &token) in expanded.iter().enumerate() {
        let mask = walker.admissible_next_raw_tokens();
        assert!(
            admissible_mask_admits(&mask, token),
            "{stream_id}: census reference walk requires every consumed token to be \
             mask-admissible; token {token} at expanded offset {offset} is not"
        );
        if admissible_mask_cardinality(&mask) == 1 {
            forced_offsets.push(offset);
        }
        walker.advance(token).unwrap_or_else(|error| {
            panic!(
                "{stream_id}: census reference walk must replay the checked stream; \
                 token {token} at expanded offset {offset} failed: {error:?}"
            )
        });
    }
    assert!(
        walker.eof_valid(),
        "{stream_id}: census reference walk must end in a complete EOF-valid state"
    );
    forced_offsets
}

/// The inventory-derived witness model shared by the elided-stream
/// property tests (same construction as the transition-acceptance
/// equivalence property).
fn elision_witness_inventory_model() -> CompiledSyntaxModel {
    let grammar = GoAstSchemaInventoryAdapter::lower_inventory_file(Path::new(
        GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH,
    ))
    .expect("checked Go AST schema inventory should lower to GrammarIR");
    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout())
        .expect("inventory-derived GrammarIR should lower through StructuralModel");
    CompiledSyntaxModel::from_sidecar(&sidecar)
        .expect("inventory-derived StructuralModel should compile to raw state machine")
}

/// Core elided-stream contract properties for one checked expanded stream:
///
/// 1. census agreement: `len(s) - len(elide(s))` equals the reference
///    detector's forced count, and elide removes exactly the forced offsets;
/// 2. exact re-expansion: `expand(elide(s)) == s` token-for-token;
/// 3. no-forced-positions invariant: the elided walker never sits on a
///    singleton (or empty) mask at a consumed-token position;
/// 4. dual-path equivalence: the incremental `ElidedRawSyntaxStream`
///    reconstruction equals the batch expansion;
/// 5. decode equivalence: elided strict decode and decoded tree equal the
///    expanded-path reference results.
///
/// Returns the forced-token count so callers can assert census numbers.
fn assert_elided_stream_contract_properties(
    model: &CompiledSyntaxModel,
    stream_id: &str,
    expanded: &[u32],
) -> usize {
    let forced_offsets = census_forced_token_offsets(model, stream_id, expanded);
    let forced_offset_set: BTreeSet<usize> = forced_offsets.iter().copied().collect();

    let elided = model
        .elide_forced_raw_tokens(expanded)
        .unwrap_or_else(|error| {
            panic!("{stream_id}: checked expanded stream must elide cleanly: {error:?}")
        });
    assert_eq!(
        expanded.len() - elided.len(),
        forced_offsets.len(),
        "{stream_id}: elide must remove exactly the census reference's forced-token count"
    );
    let expected_elided: Vec<u32> = expanded
        .iter()
        .enumerate()
        .filter(|(offset, _)| !forced_offset_set.contains(offset))
        .map(|(_, &token)| token)
        .collect();
    assert_eq!(
        elided, expected_elided,
        "{stream_id}: elide must remove exactly the census-forced positions and keep order"
    );

    let expanded_round_trip = model
        .expand_elided_raw_tokens(&elided)
        .unwrap_or_else(|error| {
            panic!("{stream_id}: canonical elided stream must re-expand: {error:?}")
        });
    assert_eq!(
        expanded_round_trip, expanded,
        "{stream_id}: expand(elide(s)) must reproduce the expanded stream token-for-token"
    );

    let mut elided_walker = model.begin_elided_stream().unwrap_or_else(|error| {
        panic!("{stream_id}: elided walker construction must drain the initial run: {error:?}")
    });
    let mut incremental_expansion: Vec<u32> = elided_walker
        .initial_forced_run()
        .iter()
        .map(|forced| forced.raw_token_id)
        .collect();
    for (elided_index, &token) in elided.iter().enumerate() {
        let mask = elided_walker.admissible_next_raw_tokens();
        let cardinality = admissible_mask_cardinality(&mask);
        assert!(
            cardinality >= 2,
            "{stream_id}: elided index {elided_index}: the elided walker must never sit on \
             a singleton or empty mask at a consumed-token position (cardinality {cardinality})"
        );
        let advance = elided_walker.advance(token).unwrap_or_else(|error| {
            panic!(
                "{stream_id}: canonical elided token {token} at elided index {elided_index} \
                 must advance: {error:?}"
            )
        });
        incremental_expansion.push(token);
        incremental_expansion.extend(
            advance
                .forced_tokens_applied
                .iter()
                .map(|forced| forced.raw_token_id),
        );
    }
    assert!(
        elided_walker.eof_valid(),
        "{stream_id}: the elided walker must drain the trailing forced run to EOF validity"
    );
    assert_eq!(
        elided_walker.consumed_elided_tokens(),
        elided.len(),
        "{stream_id}: the elided walker must account every consumed elided token"
    );
    assert_eq!(
        incremental_expansion, expanded,
        "{stream_id}: the incremental elided walker must reconstruct the batch expansion"
    );

    let reference_tree = model
        .decode_complete_tree(expanded)
        .unwrap_or_else(|error| {
            panic!("{stream_id}: expanded reference stream must strict-decode: {error:?}")
        });
    let elided_tree = model
        .decode_complete_tree_elided(&elided)
        .unwrap_or_else(|error| {
            panic!("{stream_id}: canonical elided stream must strict-decode: {error:?}")
        });
    assert_eq!(
        elided_tree, reference_tree,
        "{stream_id}: elided decoded tree must equal the expanded-path reference tree"
    );
    let reference_state = model.decode_complete(expanded).unwrap_or_else(|error| {
        panic!("{stream_id}: expanded reference stream must strict-decode: {error:?}")
    });
    let elided_state = model
        .strict_decode_complete_elided(&elided)
        .unwrap_or_else(|error| {
            panic!("{stream_id}: canonical elided stream must strict-decode: {error:?}")
        });
    assert_eq!(
        elided_state, reference_state,
        "{stream_id}: elided strict-decode state must equal the expanded-path reference state"
    );

    forced_offsets.len()
}

/// TDD guard for the total singleton extractor: the helper is defined over
/// total mask cardinality (finite ids plus range widths), not token classes,
/// so a width-1 payload range, a lone payload_end, or a lone boolean is
/// extracted exactly like a lone structural role even though no current
/// grammar produces the payload shapes.
#[test]
fn singleton_admissible_raw_token_is_total_over_mask_shapes() {
    use tct::syntax_state_machine::singleton_admissible_raw_token;

    fn range(
        role: &str,
        start: u32,
        end_exclusive: u32,
    ) -> tct::syntax_go_source_instance_projection::ProjectionTokenLayoutRangeSummary {
        tct::syntax_go_source_instance_projection::ProjectionTokenLayoutRangeSummary {
            id: role.to_string(),
            role: role.to_string(),
            start,
            end_exclusive,
        }
    }
    fn mask(
        finite: Vec<u32>,
        ranges: Vec<tct::syntax_go_source_instance_projection::ProjectionTokenLayoutRangeSummary>,
        payload_end_admissible: bool,
    ) -> RawSyntaxAdmissibleTokens {
        RawSyntaxAdmissibleTokens {
            finite_raw_token_ids: finite,
            raw_token_ranges: ranges,
            payload_end_admissible,
        }
    }

    // Lone finite ids: structural role, payload_end, boolean.
    assert_eq!(
        singleton_admissible_raw_token(&mask(vec![4], vec![], false)),
        Some(4)
    );
    assert_eq!(
        singleton_admissible_raw_token(&mask(vec![7], vec![], true)),
        Some(7),
        "a lone payload_end is a singleton like any other lone finite id"
    );
    // Width-1 payload range with no finite ids: structurally impossible
    // today, but the extractor must not rely on that.
    assert_eq!(
        singleton_admissible_raw_token(&mask(
            vec![],
            vec![range("utf8_byte_payload", 33, 34)],
            false
        )),
        Some(33),
        "a width-1 payload range is a singleton even though no current grammar produces it"
    );
    // Non-singletons of every shape.
    assert_eq!(
        singleton_admissible_raw_token(&mask(vec![], vec![], false)),
        None
    );
    assert_eq!(
        singleton_admissible_raw_token(&mask(vec![8, 9], vec![], false)),
        None
    );
    assert_eq!(
        singleton_admissible_raw_token(&mask(
            vec![7],
            vec![range("utf8_byte_payload", 33, 34)],
            true
        )),
        None,
        "a lone finite id plus a width-1 range is cardinality 2, not a singleton"
    );
    assert_eq!(
        singleton_admissible_raw_token(&mask(
            vec![],
            vec![range("utf8_byte_payload", 100, 356)],
            false
        )),
        None
    );
    assert_eq!(
        singleton_admissible_raw_token(&mask(
            vec![7],
            vec![range("utf8_byte_payload", 100, 356)],
            true
        )),
        None,
        "the typical payload mask (payload_end plus a byte range) is never a singleton"
    );
}

/// Elision round-trip core over the witness corpora: every inventory-derived
/// Go scratch seed plus the C# minimal-class stream (the transform is
/// target-neutral, so the second grammar inherits it unchanged).
#[test]
fn elided_stream_round_trip_matches_reference_over_witness_corpora() {
    let model = elision_witness_inventory_model();
    let seeds =
        go_ast_inventory_scratch_seeds(&model).expect("scratch seed list should build all seeds");
    assert!(!seeds.is_empty());
    let mut witness_forced_total = 0usize;
    for seed in &seeds {
        witness_forced_total +=
            assert_elided_stream_contract_properties(&model, &seed.seed_id, &seed.raw_token_ids);
    }
    assert!(
        witness_forced_total > 0,
        "the witness corpus must contain mask-forced positions or the elision properties are vacuous"
    );

    let grammar = CSharpSyntaxMetadataAdapter::new("csharp_minimal_class_syntax_v0")
        .lower_metadata_file(Path::new(CSHARP_METADATA_PATH))
        .expect("checked C# minimal-class metadata should lower to GrammarIR");
    let sidecar = lower_grammar_ir_to_structural_model(&grammar, csharp_syntax_token_layout())
        .expect("checked C# minimal-class GrammarIR should lower to StructuralModel");
    let csharp_model = CompiledSyntaxModel::from_sidecar(&sidecar)
        .expect("checked C# minimal-class StructuralModel should compile to raw state machine");
    let csharp_stream =
        build_csharp_minimal_class_raw_tree_render_token_stream_from_sidecar(&sidecar)
            .expect("C# minimal-class render replay should synthesize fixture source raw tokens");
    let csharp_forced = assert_elided_stream_contract_properties(
        &csharp_model,
        "csharp_minimal_class",
        &csharp_stream.raw_token_ids,
    );
    assert!(
        csharp_forced > 0,
        "the C# minimal-class stream must contain mask-forced positions"
    );
}

/// Elision round-trip core over the checked httptest-server record stream,
/// plus the record-level rejection spot checks (the canonical elided stream
/// with one forced token spelled back in must fail with the dedicated typed
/// error; truncation must follow the reference EOF semantics).
#[test]
#[ignore = "known failure: checked httptest-server projection report keeps per-file grammar id go_httptest_server_source_file_syntax_v0 without a per_file_grammar_blocker; see docs/frontends-development.md"]
fn elided_stream_round_trip_matches_reference_on_checked_httptest_record() {
    require_pinned_go_toolchain();
    let report = checked_report();
    let sidecar = checked_go_source_instance_sidecar(&report);
    let artifact_contents = regenerated_projection_artifact_for(&report);
    let artifact =
        parse_projection_artifact(&artifact_contents).expect("projection artifact should parse");
    let raw_tokens = artifact
        .raw_token_ids_against_report(&report)
        .expect("projection artifact should emit raw token IDs");
    let model = CompiledSyntaxModel::from_sidecar_and_layout(&sidecar, &raw_tokens.token_layout)
        .expect("raw syntax state machine should compile from StructuralModel");
    let expanded = &raw_tokens.raw_token_ids;

    let forced_count =
        assert_elided_stream_contract_properties(&model, &report.source.path, expanded);
    assert!(forced_count > 0);
    eprintln!(
        "httptest-server elision: expanded {} tokens, elided {} tokens, removed {} forced tokens",
        expanded.len(),
        expanded.len() - forced_count,
        forced_count
    );

    let forced_offsets = census_forced_token_offsets(&model, &report.source.path, expanded);
    let forced_offset_set: BTreeSet<usize> = forced_offsets.iter().copied().collect();
    let elided = model
        .elide_forced_raw_tokens(expanded)
        .expect("checked expanded stream must elide cleanly");

    // Spell the first forced token back into the canonical elided stream:
    // strict elided decode must reject it with the dedicated typed error
    // even though it matches the forced value.
    let first_forced_offset = forced_offsets[0];
    let forced_token = expanded[first_forced_offset];
    let elided_index = (0..first_forced_offset)
        .filter(|offset| !forced_offset_set.contains(offset))
        .count();
    let mut spelled_out = elided.clone();
    spelled_out.insert(elided_index, forced_token);
    let error = model
        .strict_decode_complete_elided(&spelled_out)
        .expect_err("a spelled-out forced token must not strict-decode as an elided stream");
    assert_eq!(error.kind, "forced_token_in_elided_stream");
    assert_eq!(error.failing_token, Some(forced_token));
    assert!(
        error.message.contains(&forced_token.to_string())
            && error.message.contains("expanded offset"),
        "the typed error must name the forced token and the expanded position: {}",
        error.message
    );

    // Truncation behaves per the reference path's EOF semantics.
    let truncated_error = model
        .strict_decode_complete_elided(&elided[..elided.len() - 1])
        .expect_err("a truncated elided stream must not strict-decode");
    let reference_error = model
        .decode_complete(&expanded[..expanded.len() - 1])
        .expect_err("a truncated expanded stream must not strict-decode");
    assert_eq!(truncated_error.kind, "premature_eof");
    assert_eq!(truncated_error.kind, reference_error.kind);
}

/// Census agreement against the checked census report: the eliding function
/// must reproduce the pinned per-record forced/elided counts for the pinned
/// census record. The checked census is dated pre-unification evidence over
/// the historical per-file grammars, so the agreement leg rebuilds the
/// per-file-shaped stream from the historical metadata fixture; the live
/// unified canonical stream is covered by the unified-artifact witness and
/// the current-plus depth gate instead.
#[test]
fn elide_agrees_with_checked_census_forced_count_for_pinned_record() {
    require_pinned_go_toolchain();
    let census: CheckedForcedCensusReport = serde_json::from_str(
        &fs::read_to_string(CHECKED_FORCED_TOKEN_CENSUS_REPORT_PATH)
            .expect("checked forced-token census report should exist"),
    )
    .expect("checked forced-token census report should parse the census-agreement fields");
    let contents = fs::read_to_string(CENSUS_PINNED_RECORD_REPORT_PATH)
        .expect("census-pinned projection report should exist");
    let report = parse_checked_report(&contents).expect("census-pinned report should validate");
    let census_row = census
        .records
        .iter()
        .find(|row| row.source_path == report.source.path)
        .expect("census report should contain the pinned record row");

    // Historical per-file leg: the census predates the unified-artifact
    // corpus switch, so its rows pin per-file-grammar streams.
    let per_file_grammar = GoSyntaxMetadataAdapter::new(CENSUS_PINNED_RECORD_PER_FILE_GRAMMAR_ID)
        .lower_metadata_file(Path::new(CENSUS_PINNED_RECORD_PER_FILE_METADATA_PATH))
        .expect("historical per-file Go syntax metadata should lower to GrammarIR");
    let sidecar = lower_grammar_ir_to_structural_model(
        &per_file_grammar,
        go_syntax_source_file_token_layout(),
    )
    .expect("historical per-file GrammarIR should lower to StructuralModel");
    let artifact_contents = per_file_projection_artifact_for(
        &report.source.path,
        CENSUS_PINNED_RECORD_PER_FILE_METADATA_PATH,
        &report.manifest_path,
        CENSUS_PINNED_RECORD_PER_FILE_GRAMMAR_ID,
    );
    let artifact =
        parse_projection_artifact(&artifact_contents).expect("projection artifact should parse");
    let raw_token_ids = artifact
        .contextual_raw_token_ids_for_sidecar(&sidecar)
        .expect("historical per-file projection should emit raw token IDs");
    assert_eq!(
        raw_token_ids.len(),
        census_row.token_count,
        "regenerated stream length must match the checked census row"
    );
    let model = CompiledSyntaxModel::from_sidecar(&sidecar)
        .expect("raw syntax state machine should compile from StructuralModel");

    let elided = model
        .elide_forced_raw_tokens(&raw_token_ids)
        .expect("checked record stream must elide cleanly");
    assert_eq!(
        raw_token_ids.len() - elided.len(),
        census_row.forced_token_count,
        "elide must remove exactly the checked census forced count for the pinned record"
    );
    assert_eq!(
        elided.len(),
        census_row.elided_token_count,
        "the elided length must match the checked census row"
    );
    let forced_count =
        assert_elided_stream_contract_properties(&model, &report.source.path, &raw_token_ids);
    assert_eq!(forced_count, census_row.forced_token_count);
}

const CENSUS_PINNED_RECORD_PER_FILE_METADATA_PATH: &str = "tests/fixtures/go_syntax_metadata/crypto_internal_fips140_edwards25519_field_fe_arm64_noasm_source_file/syntax-metadata-v0.json";
const CENSUS_PINNED_RECORD_PER_FILE_GRAMMAR_ID: &str =
    "go_crypto_internal_fips140_edwards25519_field_fe_arm64_noasm_source_file_syntax_v0";

fn per_file_projection_artifact_for(
    source_path: &str,
    metadata_path: &str,
    manifest_path: &str,
    candidate_grammar_id: &str,
) -> String {
    let tempdir = tempfile::tempdir().expect("projection tempdir should be created");
    let output_path = tempdir.path().join("projection.json");
    let output = Command::new(go_bin())
        .arg("run")
        .arg("scripts/analysis/go_syntax_source_instance_project.go")
        .arg("--source")
        .arg(source_path)
        .arg("--metadata")
        .arg(metadata_path)
        .arg("--manifest-path")
        .arg(manifest_path)
        .arg("--candidate-grammar-id")
        .arg(candidate_grammar_id)
        .arg("--projection-output")
        .arg(&output_path)
        .output()
        .expect("go source-instance projection helper should run");
    assert!(
        output.status.success(),
        "projection helper failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    fs::read_to_string(output_path).expect("projection artifact should be written")
}

/// Rejection table for elided strict decode over the witness corpus:
///
/// (a) every forced token spelled back into the canonical elided stream is
///     either rejected (at least once with the dedicated typed error) or
///     decodes to a *different* expanded stream whose canonical elision is
///     the mutated input (encoding uniqueness either way);
/// (b) pad and out-of-vocab tokens inherit the engine's typed rejections;
/// (c) a mask-inadmissible in-vocab token is rejected, and the rejected
///     elided walker stays usable (inherited error atomicity);
/// (d) truncated and empty streams fail with the reference EOF semantics;
/// (e) trailing tokens after root close inherit `extra_trailing_token`.
#[test]
fn elided_strict_decode_rejects_non_canonical_streams() {
    let model = elision_witness_inventory_model();
    let roles = model.token_roles().clone();
    let seeds =
        go_ast_inventory_scratch_seeds(&model).expect("scratch seed list should build all seeds");
    assert!(!seeds.is_empty());

    let mut dedicated_rejection_count = 0usize;
    let mut spelled_out_probe_count = 0usize;
    for seed in &seeds {
        let expanded = &seed.raw_token_ids;
        let stream_id = &seed.seed_id;
        let forced_offsets = census_forced_token_offsets(&model, stream_id, expanded);
        let forced_offset_set: BTreeSet<usize> = forced_offsets.iter().copied().collect();
        let elided = model
            .elide_forced_raw_tokens(expanded)
            .unwrap_or_else(|error| {
                panic!("{stream_id}: witness stream must elide cleanly: {error:?}")
            });

        // (a) spell each forced token back in at its canonical position.
        for &forced_offset in &forced_offsets {
            spelled_out_probe_count += 1;
            let forced_token = expanded[forced_offset];
            let elided_index = (0..forced_offset)
                .filter(|offset| !forced_offset_set.contains(offset))
                .count();
            let mut spelled_out = elided.clone();
            spelled_out.insert(elided_index, forced_token);
            match model.strict_decode_complete_elided(&spelled_out) {
                Err(error) => {
                    if error.kind == "forced_token_in_elided_stream" {
                        dedicated_rejection_count += 1;
                        assert_eq!(
                            error.failing_token,
                            Some(forced_token),
                            "{stream_id}: dedicated rejection must name the offending input token"
                        );
                        assert!(
                            error.message.contains(&forced_token.to_string())
                                && error.message.contains("expanded offset")
                                && error.message.contains("elided"),
                            "{stream_id}: dedicated rejection must name the position, the \
                             forced token, and the offending input token: {}",
                            error.message
                        );
                    }
                }
                Ok(_) => {
                    // The mutated input was admissible as a different
                    // program: encoding uniqueness still requires it to be
                    // the canonical elision of its own expansion.
                    let reinterpreted = model
                        .expand_elided_raw_tokens(&spelled_out)
                        .unwrap_or_else(|error| {
                            panic!(
                                "{stream_id}: accepted mutated elided stream must expand: {error:?}"
                            )
                        });
                    assert_ne!(
                        &reinterpreted, expanded,
                        "{stream_id}: a mutated elided stream must never silently decode to \
                         the original expanded stream"
                    );
                    assert_eq!(
                        model
                            .elide_forced_raw_tokens(&reinterpreted)
                            .expect("re-expanded stream must elide"),
                        spelled_out,
                        "{stream_id}: an accepted mutated elided stream must be the canonical \
                         elision of its own expansion (encoding uniqueness)"
                    );
                }
            }
        }
    }
    assert!(spelled_out_probe_count > 0);
    assert!(
        dedicated_rejection_count > 0,
        "at least one spelled-out forced token must hit the dedicated \
         forced_token_in_elided_stream rejection"
    );

    // (b)-(e) on the first witness seed.
    let seed = &seeds[0];
    let elided = model
        .elide_forced_raw_tokens(&seed.raw_token_ids)
        .expect("witness stream must elide cleanly");

    let mut pad_input = elided.clone();
    pad_input.insert(0, roles.pad_token);
    let pad_error = model
        .strict_decode_complete_elided(&pad_input)
        .expect_err("a pad token in an elided stream must be rejected");
    assert_eq!(pad_error.kind, "pad_token_in_raw_stream");

    let mut out_of_vocab_input = elided.clone();
    out_of_vocab_input.insert(0, roles.vocab_size);
    let vocab_error = model
        .strict_decode_complete_elided(&out_of_vocab_input)
        .expect_err("an out-of-vocab token in an elided stream must be rejected");
    assert_eq!(vocab_error.kind, "token_id_out_of_vocab");

    // (c) a mask-inadmissible in-vocab token, derived from the live mask so
    // the probe stays deterministic; the rejected walker stays usable.
    let mut walker = model
        .begin_elided_stream()
        .expect("elided walker construction must drain the initial run");
    let mask = walker.admissible_next_raw_tokens();
    let initial_run_tokens: BTreeSet<u32> = walker
        .initial_forced_run()
        .iter()
        .map(|forced| forced.raw_token_id)
        .collect();
    let inadmissible_probe = (0..roles.vocab_size)
        .find(|&token| {
            token != roles.pad_token
                && !admissible_mask_admits(&mask, token)
                && !initial_run_tokens.contains(&token)
        })
        .expect("the witness grammar mask must leave at least one in-vocab token inadmissible");
    let probe_error = walker
        .advance(inadmissible_probe)
        .expect_err("a mask-inadmissible token must be rejected by the elided walker");
    assert_ne!(probe_error.kind, "forced_token_in_elided_stream");
    let recovered = walker
        .advance(elided[0])
        .expect("a rejected elided walker must stay usable (inherited error atomicity)");
    assert_eq!(recovered.consumed.applied_raw_token_id, elided[0]);

    // (d) truncated and empty inputs follow the reference EOF semantics.
    let truncated_error = model
        .strict_decode_complete_elided(&elided[..elided.len() - 1])
        .expect_err("a truncated elided stream must not strict-decode");
    assert_eq!(truncated_error.kind, "premature_eof");
    let empty_error = model
        .strict_decode_complete_elided(&[])
        .expect_err("an empty elided stream must not strict-decode for the witness grammar");
    assert_eq!(empty_error.kind, "premature_eof");

    // (e) trailing tokens after root close inherit the engine rejection.
    let mut trailing = elided.clone();
    trailing.push(roles.node_end);
    let trailing_error = model
        .strict_decode_complete_elided(&trailing)
        .expect_err("a token after root close must not strict-decode");
    assert_eq!(trailing_error.kind, "extra_trailing_token");
}
