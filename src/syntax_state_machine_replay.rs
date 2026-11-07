//! Report-only raw syntax-token state-machine replay gate.
//!
//! This module validates checked Go source-instance raw token streams against
//! the syntax-level `GrammarIR -> StructuralModel -> token layout` boundary. It
//! is additive evidence only: it does not route syntax metadata through
//! `TctCompiler`, does not change generated JSON Schema output, and does not
//! promote source-language validity claims.

mod mutations;

use crate::csharp_syntax_metadata::CSharpSyntaxMetadataAdapter;
use crate::go_ast_schema_inventory::{
    GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH, GO_AST_SCHEMA_INVENTORY_PINNED_GO_VERSION,
    GoAstSchemaInventoryAdapter,
};
use crate::go_syntax_metadata::GoSyntaxMetadataAdapter;
use crate::grammar_ir::{GrammarIr, GrammarTerminalKind};
use crate::grammar_ir_structural_model::{
    GrammarStructuralField, GrammarStructuralModelSidecar, GrammarStructuralRecord,
    GrammarStructuralTokenLayout, GrammarStructuralTokenRange,
    lower_grammar_ir_to_structural_model,
};
use crate::provenance::{json_sha256, sha256_hex};
use crate::syntax_backend_boundary::SyntaxBackendBoundaryError;
pub use crate::syntax_csharp_raw_tree_renderer::{
    CSHARP_COMPILATION_UNIT_LOGGER_SOURCE_FILE_RAW_TREE_RENDERER_SCOPE,
    CSHARP_COMPILATION_UNIT_SOURCE_FILE_RAW_TREE_RENDERER_SCOPE,
    CSHARP_COMPILATION_UNIT_VERSION_ONE_SOURCE_FILE_RAW_TREE_RENDERER_SCOPE,
    CSHARP_LOCAL_INTEGER_RETURN_LITERAL_RAW_TREE_RENDERER_SCOPE,
    CSHARP_LOCAL_UNARY_INTEGER_RETURN_RAW_TREE_RENDERER_SCOPE,
    CSHARP_MEMBER_ACCESS_EXPRESSION_RAW_TREE_RENDERER_SCOPE,
};
use crate::syntax_csharp_raw_tree_renderer::{
    CSHARP_MINIMAL_CLASS_RAW_TREE_RENDERER_SCOPE, CSHARP_RAW_TREE_RENDERER_ID,
    render_csharp_compilation_unit_logger_source_file_from_raw_tree,
    render_csharp_compilation_unit_source_file_from_raw_tree,
    render_csharp_compilation_unit_version_one_source_file_from_raw_tree,
    render_csharp_local_integer_return_literal_from_raw_tree,
    render_csharp_local_unary_integer_return_from_raw_tree,
    render_csharp_member_access_expression_from_raw_tree,
    render_csharp_minimal_class_source_from_raw_tree,
};
use crate::syntax_csharp_real_source_generated_codec_corpus::{
    CSharpGeneratedCodecCorpusSupportedRecord,
    FIXTURE_PATH as CSHARP_REAL_SOURCE_GENERATED_CODEC_CORPUS_FIXTURE_PATH,
    REQUIRED_REPLAY_METADATA_REPORT_FIELDS as CSHARP_REAL_SOURCE_REPLAY_METADATA_REPORT_FIELDS,
    SyntaxCSharpRealSourceGeneratedCodecCorpusGate,
};
use crate::syntax_csharp_real_source_generated_codec_feasibility::{
    ITERATOR_METHODS_MANIFEST_PATH, LOAN_MANIFEST_PATH, LOGGER_MANIFEST_PATH,
    SUPPORTED_MANIFEST_PATH, VERSION_ONE_MANIFEST_PATH,
};
use crate::syntax_generated_codec::{
    CSHARP_COMPILATION_UNIT_ITERATOR_METHODS_SOURCE_FILE_METADATA_PATH,
    CSHARP_COMPILATION_UNIT_LOAN_SOURCE_FILE_METADATA_PATH,
    CSHARP_COMPILATION_UNIT_LOGGER_SOURCE_FILE_METADATA_PATH,
    CSHARP_COMPILATION_UNIT_SOURCE_FILE_METADATA_PATH,
    CSHARP_COMPILATION_UNIT_VERSION_ONE_SOURCE_FILE_METADATA_PATH,
    CSHARP_LOCAL_NULLABLE_DECLARATION_GENERATED_TEST_MARKERS,
    CSHARP_LOCAL_NULLABLE_RETURN_TYPE_GENERATED_TEST_MARKERS, csharp_syntax_token_layout,
    go_syntax_source_file_token_layout, go_syntax_token_layout,
};
use crate::syntax_go_source_instance_projection::{
    GENERATED_CODEC_REPORT_ARTIFACT_KIND, GENERATED_CODEC_REPORT_ROUTE, ProjectionRawTokenSegment,
    ProjectionTokenLayoutRangeSummary, ProjectionTokenLayoutSummary,
    SourceInstanceGeneratedCodecReport, SourceInstanceProjectionRawTokenIds,
    SyntaxGoSourceInstanceProjectionReport,
};
use crate::syntax_go_unified_artifact::{GO_UNIFIED_GRAMMAR_ID, go_unified_grammar_artifact};
use crate::syntax_scratch_generation::{
    RawScratchClaimBoundaryFlags, RawScratchEmptyPrefixSummary,
    RawScratchSelectedCompletionStepSummary, RawScratchSelectedCompletionSummary,
    RawScratchStrictIncompletePrefixSummary, RawScratchTokenOnlyDecodeSummary,
};
use crate::syntax_state_machine::{
    CompiledSyntaxModel, RAW_SYNTAX_STATE_MACHINE_CORE_BOUNDARY,
    RAW_SYNTAX_STATE_MACHINE_CORE_BOUNDARY_ID, RawSyntaxDecodedEvent, RawSyntaxDecodedEventKind,
    RawSyntaxError, RawSyntaxState, RawSyntaxStateMachineCore,
    SyntaxTokenRoles as ReplayTokenRoles,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::path::Path;

pub const TOOL: &str = "syntax_state_machine_replay_gate";
pub const VERSION: u32 = 1;
pub const ARTIFACT_KIND: &str = "tct_syntax_state_machine_replay_report_v1";
pub const STATUS: &str =
    "raw_token_alternative_identity_replay_validated_generated_codec_state_ready";
pub const TARGET: &str = "go";
pub const ROUTE: &str = "go_source_instance_projection_contextual_raw_tokens";
pub const GENERATED_CODEC_PROMOTION_BLOCKER: &str =
    "raw_token_layout_missing_tokenized_alternative_identity";
pub const REPLAY_PREFIX_INSPECTION_STATE_API_SOURCE: &str = "generated_codec_state_api_state_cases";

const NO_STABLE_CLAIMS: &[&str] = &[
    "no_strict_recovery_decode_state_api_wiring_claim",
    "no_exact_mask_state_api_wiring_claim",
    "no_source_validity_by_construction_claim",
    "no_type_correctness_claim",
    "no_model_quality_claim",
    "no_paper_facing_claim",
    "no_generated_hot_path_claim",
    "no_arena_ir_promotion",
    "no_stable_backend_promotion",
];

const REQUIRED_REJECTION_KINDS: &[&str] = &[
    "unexpected_structural_token",
    "missing_required_field",
    "repeated_field_cardinality",
    "invalid_payload_token",
    "premature_eof",
    "extra_trailing_token",
    "pad_token_in_raw_stream",
    "token_id_out_of_vocab",
];

pub const GENERATED_CODEC_STATE_API_ARTIFACT_KIND: &str =
    "tct_syntax_generated_codec_state_api_report_v1";
pub const GENERATED_CODEC_STATE_API_STATUS: &str =
    "go_source_instance_segment_aligned_strict_recovery_state_api_ready";
pub const GENERATED_CODEC_STATE_API_SCOPE: &str = "segment_aligned_go_source_instance_raw_prefixes";
pub const GENERATED_CODEC_STATE_API_SUCCESSOR_NEXT_TASK: &str =
    crate::syntax_roadmap_successors::GO_SOURCE_RECORD_RAW_TOKEN_GENERATOR_CORE_SUCCESSOR_TASK;
pub const GENERATED_CODEC_STATE_API_NEXT_BLOCKER: &str =
    GENERATED_CODEC_STATE_API_SUCCESSOR_NEXT_TASK;
pub const GENERATED_CODEC_STRICT_RECOVERY_DECODE_ARTIFACT_KIND: &str =
    "tct_syntax_generated_codec_strict_recovery_decode_report_v1";
pub const GENERATED_CODEC_STRICT_RECOVERY_DECODE_STATUS: &str =
    "go_source_instance_state_api_strict_recovery_decode_ready";
pub const GENERATED_CODEC_STRICT_RECOVERY_DECODE_POLICY: &str = "syntax-source-instance-state-api-strict-recovery-v1: strict decode is authoritative over raw syntax tokens; recovery may normalize only trailing PAD_TOKENs after a state-API-confirmed complete stream; EOF, non-PAD surplus, malformed raw tokens, PAD in raw strict streams, and partial-prefix failures remain hard errors with consumed-token accounting";
pub const GENERATED_CODEC_STRICT_RECOVERY_DECODE_NEXT_BLOCKER: &str =
    GENERATED_CODEC_STATE_API_SUCCESSOR_NEXT_TASK;

const STATE_API_NO_STABLE_CLAIMS: &[&str] = &[
    "no_arena_ir_claim",
    "no_arena_ir_promotion",
    "no_bpe_model_quality_claim",
    "no_bpe_layer_validity_claim",
    "no_cross_target_parity_claim",
    "no_generated_hot_path_claim",
    "no_paper_facing_claim",
    "no_payload_lexical_exact_mask_claim",
    "no_source_validity_by_construction_claim",
    "no_stable_backend_claim",
    "no_type_correctness_claim",
    "no_whole_source_language_validity_claim",
];

pub const CSHARP_STATE_MACHINE_PARITY_ARTIFACT_KIND: &str =
    "tct_syntax_csharp_state_machine_parity_report_v1";
pub const CSHARP_STATE_MACHINE_PARITY_STATUS: &str = "csharp_state_machine_replay_api_parity_ready";
pub const CSHARP_STATE_MACHINE_PARITY_TARGET: &str = "csharp";
pub const CSHARP_STATE_MACHINE_PARITY_ROUTE: &str =
    "csharp_syntax_metadata_structural_model_synthesized_raw_tokens";
pub const CSHARP_STATE_MACHINE_PARITY_API_SCOPE: &str =
    "csharp_structural_model_state_after_prefix_admissible_strict_recovery_accounting";
pub const CSHARP_STATE_MACHINE_PARITY_NEXT_READY_TASK: &str =
    GENERATED_CODEC_STRICT_RECOVERY_DECODE_NEXT_BLOCKER;
pub const CSHARP_STATE_MACHINE_PARITY_CLAIM_SCOPE: &str = "cross-target state-machine parity over GrammarIR -> StructuralModel -> token layout raw-token semantics";
pub const CSHARP_SELECTED_COMPLETION_TRACE_SCOPE: &str =
    "csharp_selected_completion_state_machine_parity";
pub const SELECTED_COMPLETION_TRACE_SOURCE: &str =
    "compiled_syntax_model_selected_completion_trace";
pub const CSHARP_MINIMAL_CLASS_GRAMMAR_ID: &str = "csharp_minimal_class_syntax_v0";
pub const CSHARP_LOCAL_INTEGER_RETURN_LITERAL_GRAMMAR_ID: &str =
    "csharp_local_integer_return_literal_syntax_v0";
pub const CSHARP_LOCAL_UNARY_INTEGER_RETURN_GRAMMAR_ID: &str =
    "csharp_local_unary_integer_return_syntax_v0";
pub const CSHARP_MEMBER_ACCESS_EXPRESSION_GRAMMAR_ID: &str =
    "csharp_member_access_expression_syntax_v0";
pub const CSHARP_COMPILATION_UNIT_SOURCE_FILE_GRAMMAR_ID: &str =
    "csharp_compilation_unit_source_file_syntax_v0";
pub const CSHARP_COMPILATION_UNIT_LOGGER_SOURCE_FILE_GRAMMAR_ID: &str =
    "csharp_compilation_unit_logger_source_file_syntax_v0";
pub const CSHARP_COMPILATION_UNIT_VERSION_ONE_SOURCE_FILE_GRAMMAR_ID: &str =
    "csharp_compilation_unit_version_one_source_file_syntax_v0";
pub const CSHARP_SCRATCH_GENERATION_STATUS: &str =
    "csharp_empty_prefix_scratch_generation_parity_ready";
pub const CSHARP_MINIMAL_CLASS_SCRATCH_GENERATION_SCOPE: &str =
    "csharp_minimal_class_empty_prefix_scratch_generation_parity_v1";
pub const CSHARP_LOCAL_INTEGER_RETURN_LITERAL_SCRATCH_GENERATION_SCOPE: &str =
    "csharp_local_integer_return_literal_empty_prefix_scratch_generation_parity_v1";
pub const CSHARP_LOCAL_UNARY_INTEGER_RETURN_SCRATCH_GENERATION_SCOPE: &str =
    "csharp_local_unary_integer_return_empty_prefix_scratch_generation_parity_v1";
pub const CSHARP_MEMBER_ACCESS_EXPRESSION_SCRATCH_GENERATION_SCOPE: &str =
    "csharp_member_access_expression_empty_prefix_scratch_generation_parity_v1";
pub const CSHARP_COMPILATION_UNIT_SOURCE_FILE_SCRATCH_GENERATION_SCOPE: &str =
    "csharp_compilation_unit_source_file_empty_prefix_scratch_generation_parity_v1";
pub const CSHARP_COMPILATION_UNIT_LOGGER_SOURCE_FILE_SCRATCH_GENERATION_SCOPE: &str =
    "csharp_compilation_unit_logger_source_file_empty_prefix_scratch_generation_parity_v1";
pub const CSHARP_COMPILATION_UNIT_VERSION_ONE_SOURCE_FILE_SCRATCH_GENERATION_SCOPE: &str =
    "csharp_compilation_unit_version_one_source_file_empty_prefix_scratch_generation_parity_v1";
pub const CSHARP_PRECOMPILED_SYNTAX_TOKENIZER_ARTIFACT_KIND: &str =
    "precompiled_csharp_syntax_tokenizer_artifact_v1";
pub const CSHARP_EMPTY_PREFIX_ROOT: &str = "CompilationUnitSyntax";
pub const CSHARP_SCRATCH_GENERATION_NEXT_READY_TASK: &str =
    crate::syntax_roadmap_successors::GO_SOURCE_RECORD_RAW_TOKEN_GENERATOR_CORE_SUCCESSOR_TASK;
pub const CSHARP_SCRATCH_GENERATION_CLAIM_BOUNDARY: &str = "syntax-level C# empty-prefix scratch-generation parity evidence: inference starts from a precompiled C# syntax tokenizer artifact and an empty CompilationUnitSyntax raw prefix, records finite admissible-token masks, admitted selected transitions, strict incomplete-prefix rejection, token-only decode summaries, and either render/reparse sidecar validation or a precise renderer blocker over the shared raw syntax state-machine boundary. This is not arena-IR evidence, not route-handoff provenance, not BPE validity authority, not a source-program validity claim, and not stable-backend promotion.";
pub const CSHARP_NULLABLE_MARKER_GENERATED_TEST_MARKER: &str =
    "generated_csharp_local_nullable_return_type_rejects_invalid_nullable_marker";
pub const CSHARP_LOCAL_NULLABLE_DECLARATION_MARKER_GENERATED_TEST_MARKER: &str =
    "generated_csharp_local_nullable_declaration_rejects_invalid_nullable_marker";
const CSHARP_NULLABLE_MARKER_GENERATED_TEST_MARKERS: &[&str] = &[
    CSHARP_NULLABLE_MARKER_GENERATED_TEST_MARKER,
    CSHARP_LOCAL_NULLABLE_DECLARATION_MARKER_GENERATED_TEST_MARKER,
];
pub const CSHARP_REAL_SOURCE_STATE_MACHINE_PARITY_ARTIFACT_KIND: &str =
    "tct_syntax_csharp_real_source_state_machine_parity_report_v1";
pub const CSHARP_REAL_SOURCE_STATE_MACHINE_PARITY_STATUS: &str =
    "csharp_real_source_generated_codec_state_api_ready";
pub const CSHARP_STRUCTURAL_REPLAY_TOKEN_STREAM_ARTIFACT_KIND: &str =
    "tct_csharp_structural_model_replay_token_stream_v1";
pub const CSHARP_STRUCTURAL_REPLAY_TOKEN_LAYOUT_KIND: &str = "structural_model_replay_roles";
pub const CSHARP_STRUCTURAL_REPLAY_TOKEN_STREAM_SOURCE: &str =
    "src/syntax_state_machine_replay.rs::build_csharp_structural_replay_token_stream_from_sidecar";
pub const CSHARP_SOURCE_RECORD_RAW_TOKEN_EMISSION_API_ID: &str =
    "csharp-source-record-raw-token-emission-api-v1";
pub const CSHARP_SOURCE_RECORD_RAW_TOKEN_EMISSION_API_BOUNDARY: &str = "checked C# generated-codec/structural-replay source record + token-layout/structural-model context -> validated raw syntax-token stream consumed by the C# real-source state-machine parity path; Roslyn/parser, renderer, type-checker, arena IR, route handoff, BPE, and stable-backend claims remain outside this boundary";

pub const GO_AST_DECLARATION_PRESENCE_REPLAY_ARTIFACT_KIND: &str =
    "tct_go_ast_declaration_presence_raw_replay_report_v1";
pub const GO_AST_DECLARATION_PRESENCE_REPLAY_STATUS: &str =
    "go_ast_inventory_optional_token_presence_consumed_by_raw_state_machine";
pub const GO_AST_DECLARATION_PRESENCE_REPLAY_TARGET: &str = "go";
pub const GO_AST_DECLARATION_PRESENCE_REPLAY_ROUTE: &str =
    "go_ast_schema_inventory_structural_model_optional_token_presence_raw_replay";
pub const GO_AST_DECLARATION_PRESENCE_REPLAY_SCOPE: &str = "inventory-backed declaration, function, and variadic-call optional-token presence raw replay slice";
pub const GO_AST_DECLARATION_PRESENCE_REPLAY_NEXT_READY_TASK: &str =
    GENERATED_CODEC_STRICT_RECOVERY_DECODE_NEXT_BLOCKER;

const CSHARP_REAL_SOURCE_REPLAY_REQUIRED_REPORT_FIELDS: &[&str] = &[
    "tct_tokens",
    "raw_token_ids_sha256",
    "structural_replay.raw_token_ids",
    "structural_replay.raw_token_ids_sha256",
    "structural_replay.raw_token_segments",
    "structural_replay.raw_token_segments_sha256",
    "raw_token_offset_mapping",
];
const CSHARP_PAYLOAD_SEQUENCE_GENERATED_CODEC: &str =
    "csharp_compilation_unit_source_file_syntax_v0";

const CSHARP_PARITY_NO_STABLE_CLAIMS: &[&str] = &[
    "no_arena_ir_claim",
    "no_arena_ir_promotion",
    "no_bpe_model_quality_claim",
    "no_bpe_layer_validity_claim",
    "no_generated_hot_path_claim",
    "no_paper_facing_claim",
    "no_source_validity_by_construction_claim",
    "no_stable_backend_claim",
    "no_type_correctness_claim",
    "no_whole_source_language_validity_claim",
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxStateMachineReplayReport {
    pub artifact_kind: String,
    pub version: u32,
    pub tool: String,
    pub status: String,
    pub target: String,
    pub route: String,
    pub candidate_grammar_id: String,
    pub source_path: String,
    pub source_sha256: String,
    pub metadata_path: String,
    pub metadata_sha256: String,
    pub parser_tool: String,
    pub parser_version: String,
    pub structural_model_id: String,
    pub token_layout: ProjectionTokenLayoutSummary,
    pub generated_codec_report_artifact_kind: String,
    pub generated_codec_report_path: String,
    pub raw_token_count: usize,
    pub raw_token_ids_sha256: String,
    pub segment_count: usize,
    pub segment_events_sha256: String,
    pub complete_stream: SyntaxReplayCaseResult,
    pub prefix_cases: Vec<SyntaxReplayCaseResult>,
    pub prefix_inspection_state_api_ready: bool,
    pub prefix_inspection_state_api_source: String,
    pub invalid_mutation_cases: Vec<SyntaxReplayCaseResult>,
    pub accepted_prefix_count: usize,
    pub rejected_mutation_count: usize,
    pub consumed_token_accounting_ok: bool,
    pub raw_token_self_sufficient_for_generated_codec: bool,
    pub generated_codec_promotion_blockers: Vec<SyntaxReplayPromotionBlocker>,
    pub next_missing_workstream_blocker: String,
    pub validation_commands: Vec<String>,
    pub claim_boundary: String,
    pub no_stable_claims: Vec<String>,
}

impl SyntaxStateMachineReplayReport {
    pub fn validate(&self) -> Result<(), SyntaxBackendBoundaryError> {
        require_string("artifact_kind", &self.artifact_kind, ARTIFACT_KIND)?;
        require_u32("version", self.version, VERSION)?;
        require_string("tool", &self.tool, TOOL)?;
        require_string("status", &self.status, STATUS)?;
        require_string("target", &self.target, TARGET)?;
        require_string("route", &self.route, ROUTE)?;
        require_non_empty("candidate_grammar_id", &self.candidate_grammar_id)?;
        require_non_empty("source_path", &self.source_path)?;
        require_sha256("source_sha256", &self.source_sha256)?;
        require_non_empty("metadata_path", &self.metadata_path)?;
        require_sha256("metadata_sha256", &self.metadata_sha256)?;
        require_string(
            "generated_codec_report_artifact_kind",
            &self.generated_codec_report_artifact_kind,
            GENERATED_CODEC_REPORT_ARTIFACT_KIND,
        )?;
        require_non_empty(
            "generated_codec_report_path",
            &self.generated_codec_report_path,
        )?;
        require_positive_usize("raw_token_count", self.raw_token_count)?;
        require_sha256("raw_token_ids_sha256", &self.raw_token_ids_sha256)?;
        require_positive_usize("segment_count", self.segment_count)?;
        require_sha256("segment_events_sha256", &self.segment_events_sha256)?;
        self.complete_stream.validate()?;
        require_string(
            "complete_stream.observed_status",
            &self.complete_stream.observed_status,
            "complete",
        )?;
        require_bool(
            "complete_stream.accepted",
            self.complete_stream.accepted,
            true,
        )?;
        require_usize(
            "complete_stream.consumed_raw_tokens",
            self.complete_stream.consumed_raw_tokens,
            self.raw_token_count,
        )?;

        require_positive_usize("prefix_cases", self.prefix_cases.len())?;
        for case in &self.prefix_cases {
            case.validate()?;
            require_bool("prefix_case.accepted", case.accepted, true)?;
            require_string(
                "prefix_case.observed_status",
                &case.observed_status,
                "partial",
            )?;
        }
        require_usize(
            "accepted_prefix_count",
            self.accepted_prefix_count,
            self.prefix_cases.len(),
        )?;
        require_bool(
            "prefix_inspection_state_api_ready",
            self.prefix_inspection_state_api_ready,
            true,
        )?;
        require_string(
            "prefix_inspection_state_api_source",
            &self.prefix_inspection_state_api_source,
            REPLAY_PREFIX_INSPECTION_STATE_API_SOURCE,
        )?;

        let mut rejection_kinds = BTreeSet::new();
        for case in &self.invalid_mutation_cases {
            case.validate()?;
            require_bool("invalid_mutation_case.accepted", case.accepted, false)?;
            let error_kind = case.error_kind.as_deref().ok_or_else(|| {
                SyntaxBackendBoundaryError::new(
                    "invalid_mutation_cases.error_kind",
                    "rejected mutation cases must carry an error kind",
                )
            })?;
            rejection_kinds.insert(error_kind.to_string());
        }
        for required in REQUIRED_REJECTION_KINDS {
            if !rejection_kinds.contains(*required) {
                return Err(SyntaxBackendBoundaryError::new(
                    "invalid_mutation_cases",
                    format!("missing rejection kind {required}"),
                ));
            }
        }
        require_usize(
            "rejected_mutation_count",
            self.rejected_mutation_count,
            self.invalid_mutation_cases.len(),
        )?;
        require_bool(
            "consumed_token_accounting_ok",
            self.consumed_token_accounting_ok,
            true,
        )?;
        require_bool(
            "raw_token_self_sufficient_for_generated_codec",
            self.raw_token_self_sufficient_for_generated_codec,
            true,
        )?;
        require_usize(
            "generated_codec_promotion_blockers",
            self.generated_codec_promotion_blockers.len(),
            0,
        )?;
        for blocker in &self.generated_codec_promotion_blockers {
            blocker.validate()?;
        }
        require_contains(
            "next_missing_workstream_blocker",
            &self.next_missing_workstream_blocker,
            "generated-codec state API",
        )?;
        require_contains(
            "claim_boundary",
            &self.claim_boundary,
            "report-only replay evidence",
        )?;
        require_contains("claim_boundary", &self.claim_boundary, "not arena-IR")?;
        require_contains("claim_boundary", &self.claim_boundary, "not stable-backend")?;
        require_all("no_stable_claims", &self.no_stable_claims, NO_STABLE_CLAIMS)?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxReplayPromotionBlocker {
    pub kind: String,
    pub owner_rule: String,
    pub owner_field: String,
    pub alternative_count: usize,
    pub alternatives: Vec<String>,
    pub missing_fact: String,
}

impl SyntaxReplayPromotionBlocker {
    fn validate(&self) -> Result<(), SyntaxBackendBoundaryError> {
        require_string(
            "promotion_blocker.kind",
            &self.kind,
            GENERATED_CODEC_PROMOTION_BLOCKER,
        )?;
        require_non_empty("promotion_blocker.owner_rule", &self.owner_rule)?;
        require_non_empty("promotion_blocker.owner_field", &self.owner_field)?;
        require_positive_usize(
            "promotion_blocker.alternative_count",
            self.alternative_count,
        )?;
        require_usize(
            "promotion_blocker.alternatives",
            self.alternatives.len(),
            self.alternative_count,
        )?;
        require_contains(
            "promotion_blocker.missing_fact",
            &self.missing_fact,
            "raw node_start token",
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxReplayCaseResult {
    pub case_id: String,
    pub input_kind: String,
    pub expected_status: String,
    pub observed_status: String,
    pub accepted: bool,
    pub consumed_raw_tokens: usize,
    pub total_raw_tokens: usize,
    pub eof_valid: bool,
    pub stack_depth: usize,
    pub active_rule: Option<String>,
    pub active_field: Option<String>,
    pub error_kind: Option<String>,
    pub failing_token: Option<u32>,
    pub diagnostic: String,
}

impl SyntaxReplayCaseResult {
    fn accepted(
        case_id: &str,
        input_kind: &str,
        expected_status: &str,
        state: RawSyntaxState,
    ) -> Self {
        Self {
            case_id: case_id.to_string(),
            input_kind: input_kind.to_string(),
            expected_status: expected_status.to_string(),
            observed_status: state.status.as_str().to_string(),
            accepted: true,
            consumed_raw_tokens: state.consumed_raw_tokens,
            total_raw_tokens: state.total_raw_tokens,
            eof_valid: state.eof_valid,
            stack_depth: state.stack_depth,
            active_rule: state.active_rule,
            active_field: state.active_field,
            error_kind: None,
            failing_token: None,
            diagnostic: state.diagnostic,
        }
    }

    fn rejected(
        case_id: &str,
        input_kind: &str,
        expected_error: &str,
        error: RawSyntaxError,
        total_raw_tokens: usize,
    ) -> Self {
        Self {
            case_id: case_id.to_string(),
            input_kind: input_kind.to_string(),
            expected_status: "error".to_string(),
            observed_status: "error".to_string(),
            accepted: false,
            consumed_raw_tokens: error.consumed_raw_tokens,
            total_raw_tokens,
            eof_valid: false,
            stack_depth: error.stack_depth,
            active_rule: error.active_rule,
            active_field: error.active_field,
            error_kind: Some(error.kind.to_string()),
            failing_token: error.failing_token,
            diagnostic: format!("{expected_error}: {}", error.message),
        }
    }

    fn from_generated_codec_state_case(
        case_id: &str,
        input_kind: &str,
        expected_status: &str,
        state_case: SyntaxGeneratedCodecStateCase,
    ) -> Self {
        Self {
            case_id: case_id.to_string(),
            input_kind: input_kind.to_string(),
            expected_status: expected_status.to_string(),
            observed_status: state_case.observed_status,
            accepted: true,
            consumed_raw_tokens: state_case.consumed_raw_tokens,
            total_raw_tokens: state_case.total_raw_tokens,
            eof_valid: state_case.eof_valid,
            stack_depth: state_case.stack_depth,
            active_rule: state_case.active_rule,
            active_field: state_case.active_field,
            error_kind: None,
            failing_token: None,
            diagnostic: format!(
                "generated-codec state API prefix inspection: {}",
                state_case.diagnostic
            ),
        }
    }

    fn validate(&self) -> Result<(), SyntaxBackendBoundaryError> {
        require_non_empty("replay_case.case_id", &self.case_id)?;
        require_non_empty("replay_case.input_kind", &self.input_kind)?;
        require_non_empty("replay_case.expected_status", &self.expected_status)?;
        require_non_empty("replay_case.observed_status", &self.observed_status)?;
        require_non_empty("replay_case.diagnostic", &self.diagnostic)?;
        if self.consumed_raw_tokens > self.total_raw_tokens && self.total_raw_tokens != 0 {
            return Err(SyntaxBackendBoundaryError::new(
                "replay_case.consumed_raw_tokens",
                "consumed-token accounting cannot exceed input length",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxGeneratedCodecStateApiReport {
    pub artifact_kind: String,
    pub version: u32,
    pub tool: String,
    pub status: String,
    pub target: String,
    pub route: String,
    pub api_scope: String,
    pub candidate_grammar_id: String,
    pub source_path: String,
    pub source_sha256: String,
    pub metadata_path: String,
    pub metadata_sha256: String,
    pub structural_model_id: String,
    pub token_layout: ProjectionTokenLayoutSummary,
    pub generated_codec_report_artifact_kind: String,
    pub generated_codec_report_path: String,
    pub raw_token_count: usize,
    pub raw_token_ids_sha256: String,
    pub state_cases: Vec<SyntaxGeneratedCodecStateCase>,
    pub transition_cases: Vec<SyntaxGeneratedCodecTransitionCase>,
    pub prefix_state_api_ready: bool,
    pub strict_recovery_decode_wiring_ready: bool,
    pub payload_range_state_api_ready: bool,
    pub bpe_layer_defines_validity: bool,
    pub next_missing_workstream_blocker: String,
    pub validation_commands: Vec<String>,
    pub claim_boundary: String,
    pub no_stable_claims: Vec<String>,
}

impl SyntaxGeneratedCodecStateApiReport {
    pub fn validate(&self) -> Result<(), SyntaxBackendBoundaryError> {
        require_string(
            "generated_codec_state_api.artifact_kind",
            &self.artifact_kind,
            GENERATED_CODEC_STATE_API_ARTIFACT_KIND,
        )?;
        require_u32("generated_codec_state_api.version", self.version, VERSION)?;
        require_string("generated_codec_state_api.tool", &self.tool, TOOL)?;
        require_string(
            "generated_codec_state_api.status",
            &self.status,
            GENERATED_CODEC_STATE_API_STATUS,
        )?;
        require_string("generated_codec_state_api.target", &self.target, TARGET)?;
        require_string("generated_codec_state_api.route", &self.route, ROUTE)?;
        require_string(
            "generated_codec_state_api.api_scope",
            &self.api_scope,
            GENERATED_CODEC_STATE_API_SCOPE,
        )?;
        require_non_empty(
            "generated_codec_state_api.candidate_grammar_id",
            &self.candidate_grammar_id,
        )?;
        require_non_empty("generated_codec_state_api.source_path", &self.source_path)?;
        require_sha256(
            "generated_codec_state_api.source_sha256",
            &self.source_sha256,
        )?;
        require_non_empty(
            "generated_codec_state_api.metadata_path",
            &self.metadata_path,
        )?;
        require_sha256(
            "generated_codec_state_api.metadata_sha256",
            &self.metadata_sha256,
        )?;
        require_non_empty(
            "generated_codec_state_api.structural_model_id",
            &self.structural_model_id,
        )?;
        validate_projection_token_layout(&self.token_layout)?;
        require_string(
            "generated_codec_state_api.generated_codec_report_artifact_kind",
            &self.generated_codec_report_artifact_kind,
            GENERATED_CODEC_REPORT_ARTIFACT_KIND,
        )?;
        require_non_empty(
            "generated_codec_state_api.generated_codec_report_path",
            &self.generated_codec_report_path,
        )?;
        require_positive_usize(
            "generated_codec_state_api.raw_token_count",
            self.raw_token_count,
        )?;
        require_sha256(
            "generated_codec_state_api.raw_token_ids_sha256",
            &self.raw_token_ids_sha256,
        )?;

        require_positive_usize(
            "generated_codec_state_api.state_cases",
            self.state_cases.len(),
        )?;
        let mut state_ids = BTreeSet::new();
        let mut eof_valid_count = 0usize;
        let mut payload_range_state_count = 0usize;
        for case in &self.state_cases {
            case.validate(self.token_layout.vocab_size)?;
            if !state_ids.insert(case.case_id.clone()) {
                return Err(SyntaxBackendBoundaryError::new(
                    "generated_codec_state_api.state_cases.case_id",
                    format!("duplicate state case {}", case.case_id),
                ));
            }
            if case.eof_valid {
                eof_valid_count += 1;
            }
            if !case.admissible_next_raw_token_ranges.is_empty() {
                payload_range_state_count += 1;
            }
        }
        if eof_valid_count != 1 {
            return Err(SyntaxBackendBoundaryError::new(
                "generated_codec_state_api.state_cases.eof_valid",
                "state API report must include exactly one EOF-valid complete case",
            ));
        }
        require_positive_usize(
            "generated_codec_state_api.state_cases.payload_range_state_count",
            payload_range_state_count,
        )?;

        require_positive_usize(
            "generated_codec_state_api.transition_cases",
            self.transition_cases.len(),
        )?;
        let mut transition_ids = BTreeSet::new();
        let mut accepted = 0usize;
        let mut rejected = 0usize;
        for case in &self.transition_cases {
            case.validate(self.token_layout.vocab_size)?;
            if !transition_ids.insert(case.case_id.clone()) {
                return Err(SyntaxBackendBoundaryError::new(
                    "generated_codec_state_api.transition_cases.case_id",
                    format!("duplicate transition case {}", case.case_id),
                ));
            }
            if case.accepted {
                accepted += 1;
            } else {
                rejected += 1;
            }
        }
        require_positive_usize(
            "generated_codec_state_api.transition_cases.accepted",
            accepted,
        )?;
        require_positive_usize(
            "generated_codec_state_api.transition_cases.rejected",
            rejected,
        )?;
        require_bool(
            "generated_codec_state_api.prefix_state_api_ready",
            self.prefix_state_api_ready,
            true,
        )?;
        require_bool(
            "generated_codec_state_api.strict_recovery_decode_wiring_ready",
            self.strict_recovery_decode_wiring_ready,
            true,
        )?;
        require_bool(
            "generated_codec_state_api.payload_range_state_api_ready",
            self.payload_range_state_api_ready,
            true,
        )?;
        require_bool(
            "generated_codec_state_api.bpe_layer_defines_validity",
            self.bpe_layer_defines_validity,
            false,
        )?;
        require_string(
            "generated_codec_state_api.next_missing_workstream_blocker",
            &self.next_missing_workstream_blocker,
            GENERATED_CODEC_STATE_API_NEXT_BLOCKER,
        )?;
        require_contains(
            "generated_codec_state_api.claim_boundary",
            &self.claim_boundary,
            "syntax-level",
        )?;
        require_contains(
            "generated_codec_state_api.claim_boundary",
            &self.claim_boundary,
            "not arena-IR",
        )?;
        require_contains(
            "generated_codec_state_api.claim_boundary",
            &self.claim_boundary,
            "not stable-backend",
        )?;
        require_all(
            "generated_codec_state_api.no_stable_claims",
            &self.no_stable_claims,
            STATE_API_NO_STABLE_CLAIMS,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxGeneratedCodecStateCase {
    pub case_id: String,
    pub prefix_kind: String,
    pub prefix_segment_count: usize,
    pub prefix_raw_token_count: usize,
    pub observed_status: String,
    pub consumed_raw_tokens: usize,
    pub total_raw_tokens: usize,
    pub eof_valid: bool,
    pub stack_depth: usize,
    pub active_rule: Option<String>,
    pub active_field: Option<String>,
    pub active_payload_kind: Option<String>,
    pub active_payload_node_kind: Option<String>,
    pub active_payload_byte_count: usize,
    pub payload_end_admissible: bool,
    pub admissible_next_raw_token_ids: Vec<u32>,
    pub admissible_next_raw_token_ranges: Vec<ProjectionTokenLayoutRangeSummary>,
    pub diagnostic: String,
}

impl SyntaxGeneratedCodecStateCase {
    fn validate(&self, vocab_size: u32) -> Result<(), SyntaxBackendBoundaryError> {
        require_non_empty("generated_codec_state_case.case_id", &self.case_id)?;
        require_non_empty("generated_codec_state_case.prefix_kind", &self.prefix_kind)?;
        require_non_empty(
            "generated_codec_state_case.observed_status",
            &self.observed_status,
        )?;
        require_non_empty("generated_codec_state_case.diagnostic", &self.diagnostic)?;
        require_usize(
            "generated_codec_state_case.prefix_raw_token_count",
            self.prefix_raw_token_count,
            self.consumed_raw_tokens,
        )?;
        require_usize(
            "generated_codec_state_case.total_raw_tokens",
            self.total_raw_tokens,
            self.prefix_raw_token_count,
        )?;
        validate_sorted_unique_tokens(
            "generated_codec_state_case.admissible_next_raw_token_ids",
            &self.admissible_next_raw_token_ids,
            vocab_size,
        )?;
        for range in &self.admissible_next_raw_token_ranges {
            validate_projection_token_layout_range(range)?;
            if range.end_exclusive > vocab_size {
                return Err(SyntaxBackendBoundaryError::new(
                    "generated_codec_state_case.admissible_next_raw_token_ranges",
                    "admissible token range must remain below VOCAB_SIZE",
                ));
            }
        }
        if self.admissible_next_raw_token_ranges.is_empty() {
            if self.active_payload_kind.is_some()
                || self.active_payload_node_kind.is_some()
                || self.active_payload_byte_count > 0
                || self.payload_end_admissible
            {
                return Err(SyntaxBackendBoundaryError::new(
                    "generated_codec_state_case.active_payload",
                    "non-payload state must not carry payload-state metadata",
                ));
            }
        } else {
            let payload_kind = self.active_payload_kind.as_deref().ok_or_else(|| {
                SyntaxBackendBoundaryError::new(
                    "generated_codec_state_case.active_payload_kind",
                    "payload-range state must identify the active payload kind",
                )
            })?;
            require_non_empty(
                "generated_codec_state_case.active_payload_kind",
                payload_kind,
            )?;
            let payload_node_kind = self.active_payload_node_kind.as_deref().ok_or_else(|| {
                SyntaxBackendBoundaryError::new(
                    "generated_codec_state_case.active_payload_node_kind",
                    "payload-range state must identify the active payload node kind",
                )
            })?;
            require_non_empty(
                "generated_codec_state_case.active_payload_node_kind",
                payload_node_kind,
            )?;
            if self.payload_end_admissible && self.admissible_next_raw_token_ids.is_empty() {
                return Err(SyntaxBackendBoundaryError::new(
                    "generated_codec_state_case.payload_end_admissible",
                    "payload-end-admissible states must advertise the payload_end raw token",
                ));
            }
        }
        if self.eof_valid && !self.admissible_next_raw_token_ids.is_empty() {
            return Err(SyntaxBackendBoundaryError::new(
                "generated_codec_state_case.eof_valid",
                "EOF-valid state must not advertise more raw structural tokens",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxGeneratedCodecTransitionCase {
    pub case_id: String,
    pub from_state_case_id: String,
    pub candidate_raw_token_id: u32,
    pub expected_accepted: bool,
    pub accepted: bool,
    pub observed_status: String,
    pub consumed_raw_tokens: usize,
    pub eof_valid: bool,
    pub stack_depth: usize,
    pub active_rule: Option<String>,
    pub active_field: Option<String>,
    pub error_kind: Option<String>,
    pub diagnostic: String,
}

impl SyntaxGeneratedCodecTransitionCase {
    fn validate(&self, vocab_size: u32) -> Result<(), SyntaxBackendBoundaryError> {
        require_non_empty("generated_codec_transition_case.case_id", &self.case_id)?;
        require_non_empty(
            "generated_codec_transition_case.from_state_case_id",
            &self.from_state_case_id,
        )?;
        if self.candidate_raw_token_id >= vocab_size {
            return Err(SyntaxBackendBoundaryError::new(
                "generated_codec_transition_case.candidate_raw_token_id",
                "candidate token must be below VOCAB_SIZE",
            ));
        }
        require_bool(
            "generated_codec_transition_case.accepted",
            self.accepted,
            self.expected_accepted,
        )?;
        require_non_empty(
            "generated_codec_transition_case.observed_status",
            &self.observed_status,
        )?;
        require_non_empty(
            "generated_codec_transition_case.diagnostic",
            &self.diagnostic,
        )?;
        match (self.accepted, self.error_kind.as_ref()) {
            (true, None) => Ok(()),
            (false, Some(error_kind)) if !error_kind.is_empty() => Ok(()),
            (true, Some(_)) => Err(SyntaxBackendBoundaryError::new(
                "generated_codec_transition_case.error_kind",
                "accepted transition must not carry an error kind",
            )),
            (false, None) => Err(SyntaxBackendBoundaryError::new(
                "generated_codec_transition_case.error_kind",
                "rejected transition must carry an error kind",
            )),
            (false, Some(_)) => Err(SyntaxBackendBoundaryError::new(
                "generated_codec_transition_case.error_kind",
                "rejected transition error kind must be non-empty",
            )),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxGeneratedCodecStrictRecoveryDecodeReport {
    pub artifact_kind: String,
    pub version: u32,
    pub tool: String,
    pub status: String,
    pub target: String,
    pub route: String,
    pub decode_policy: String,
    pub state_api_artifact_kind: String,
    pub state_api_scope: String,
    pub candidate_grammar_id: String,
    pub source_path: String,
    pub source_sha256: String,
    pub metadata_path: String,
    pub metadata_sha256: String,
    pub structural_model_id: String,
    pub token_layout: ProjectionTokenLayoutSummary,
    pub generated_codec_report_artifact_kind: String,
    pub generated_codec_report_path: String,
    pub raw_token_count: usize,
    pub raw_token_ids_sha256: String,
    pub strict_cases: Vec<SyntaxGeneratedCodecStrictDecodeCase>,
    pub recovery_cases: Vec<SyntaxGeneratedCodecRecoveryDecodeCase>,
    pub state_api_consumed: bool,
    pub prefix_state_api_ready: bool,
    pub strict_recovery_decode_wiring_ready: bool,
    pub consumed_token_accounting_ok: bool,
    pub complete_stream_eof_valid: bool,
    pub trailing_pad_recovery_ready: bool,
    pub bpe_layer_defines_validity: bool,
    pub next_missing_workstream_blocker: String,
    pub validation_commands: Vec<String>,
    pub claim_boundary: String,
    pub no_stable_claims: Vec<String>,
}

impl SyntaxGeneratedCodecStrictRecoveryDecodeReport {
    pub fn validate(&self) -> Result<(), SyntaxBackendBoundaryError> {
        require_string(
            "strict_recovery_decode.artifact_kind",
            &self.artifact_kind,
            GENERATED_CODEC_STRICT_RECOVERY_DECODE_ARTIFACT_KIND,
        )?;
        require_u32("strict_recovery_decode.version", self.version, VERSION)?;
        require_string("strict_recovery_decode.tool", &self.tool, TOOL)?;
        require_string(
            "strict_recovery_decode.status",
            &self.status,
            GENERATED_CODEC_STRICT_RECOVERY_DECODE_STATUS,
        )?;
        require_string("strict_recovery_decode.target", &self.target, TARGET)?;
        require_string("strict_recovery_decode.route", &self.route, ROUTE)?;
        require_string(
            "strict_recovery_decode.decode_policy",
            &self.decode_policy,
            GENERATED_CODEC_STRICT_RECOVERY_DECODE_POLICY,
        )?;
        require_string(
            "strict_recovery_decode.state_api_artifact_kind",
            &self.state_api_artifact_kind,
            GENERATED_CODEC_STATE_API_ARTIFACT_KIND,
        )?;
        require_string(
            "strict_recovery_decode.state_api_scope",
            &self.state_api_scope,
            GENERATED_CODEC_STATE_API_SCOPE,
        )?;
        require_non_empty(
            "strict_recovery_decode.candidate_grammar_id",
            &self.candidate_grammar_id,
        )?;
        require_non_empty("strict_recovery_decode.source_path", &self.source_path)?;
        require_sha256("strict_recovery_decode.source_sha256", &self.source_sha256)?;
        require_non_empty("strict_recovery_decode.metadata_path", &self.metadata_path)?;
        require_sha256(
            "strict_recovery_decode.metadata_sha256",
            &self.metadata_sha256,
        )?;
        require_non_empty(
            "strict_recovery_decode.structural_model_id",
            &self.structural_model_id,
        )?;
        validate_projection_token_layout(&self.token_layout)?;
        require_string(
            "strict_recovery_decode.generated_codec_report_artifact_kind",
            &self.generated_codec_report_artifact_kind,
            GENERATED_CODEC_REPORT_ARTIFACT_KIND,
        )?;
        require_non_empty(
            "strict_recovery_decode.generated_codec_report_path",
            &self.generated_codec_report_path,
        )?;
        require_positive_usize(
            "strict_recovery_decode.raw_token_count",
            self.raw_token_count,
        )?;
        require_sha256(
            "strict_recovery_decode.raw_token_ids_sha256",
            &self.raw_token_ids_sha256,
        )?;

        require_positive_usize(
            "strict_recovery_decode.strict_cases",
            self.strict_cases.len(),
        )?;
        let mut strict_ids = BTreeSet::new();
        let mut strict_success = 0usize;
        let mut strict_errors = BTreeSet::new();
        for case in &self.strict_cases {
            case.validate(self.token_layout.vocab_size)?;
            if !strict_ids.insert(case.case_id.clone()) {
                return Err(SyntaxBackendBoundaryError::new(
                    "strict_recovery_decode.strict_cases.case_id",
                    format!("duplicate strict case {}", case.case_id),
                ));
            }
            if case.accepted {
                strict_success += 1;
            } else if let Some(error_kind) = case.error_kind.as_ref() {
                strict_errors.insert(error_kind.as_str());
            }
        }
        require_positive_usize(
            "strict_recovery_decode.strict_cases.accepted",
            strict_success,
        )?;
        for required in [
            "premature_eof",
            "pad_token_in_raw_stream",
            "extra_trailing_token",
        ] {
            if !strict_errors.contains(required) {
                return Err(SyntaxBackendBoundaryError::new(
                    "strict_recovery_decode.strict_cases.error_kind",
                    format!("missing strict decode error kind {required}"),
                ));
            }
        }

        require_positive_usize(
            "strict_recovery_decode.recovery_cases",
            self.recovery_cases.len(),
        )?;
        let mut recovery_ids = BTreeSet::new();
        let mut observed_outcomes = BTreeSet::new();
        for case in &self.recovery_cases {
            case.validate(self.token_layout.vocab_size)?;
            if !recovery_ids.insert(case.case_id.clone()) {
                return Err(SyntaxBackendBoundaryError::new(
                    "strict_recovery_decode.recovery_cases.case_id",
                    format!("duplicate recovery case {}", case.case_id),
                ));
            }
            observed_outcomes.insert(case.observed_outcome.as_str());
        }
        for required in ["strict_success", "normalized_trailing_pad", "hard_error"] {
            if !observed_outcomes.contains(required) {
                return Err(SyntaxBackendBoundaryError::new(
                    "strict_recovery_decode.recovery_cases.observed_outcome",
                    format!("missing recovery outcome {required}"),
                ));
            }
        }
        require_bool(
            "strict_recovery_decode.state_api_consumed",
            self.state_api_consumed,
            true,
        )?;
        require_bool(
            "strict_recovery_decode.prefix_state_api_ready",
            self.prefix_state_api_ready,
            true,
        )?;
        require_bool(
            "strict_recovery_decode.strict_recovery_decode_wiring_ready",
            self.strict_recovery_decode_wiring_ready,
            true,
        )?;
        require_bool(
            "strict_recovery_decode.consumed_token_accounting_ok",
            self.consumed_token_accounting_ok,
            true,
        )?;
        require_bool(
            "strict_recovery_decode.complete_stream_eof_valid",
            self.complete_stream_eof_valid,
            true,
        )?;
        require_bool(
            "strict_recovery_decode.trailing_pad_recovery_ready",
            self.trailing_pad_recovery_ready,
            true,
        )?;
        require_bool(
            "strict_recovery_decode.bpe_layer_defines_validity",
            self.bpe_layer_defines_validity,
            false,
        )?;
        require_string(
            "strict_recovery_decode.next_missing_workstream_blocker",
            &self.next_missing_workstream_blocker,
            GENERATED_CODEC_STRICT_RECOVERY_DECODE_NEXT_BLOCKER,
        )?;
        require_contains(
            "strict_recovery_decode.claim_boundary",
            &self.claim_boundary,
            "syntax-level",
        )?;
        require_contains(
            "strict_recovery_decode.claim_boundary",
            &self.claim_boundary,
            "state API",
        )?;
        require_contains(
            "strict_recovery_decode.claim_boundary",
            &self.claim_boundary,
            "not arena-IR",
        )?;
        require_contains(
            "strict_recovery_decode.claim_boundary",
            &self.claim_boundary,
            "not stable-backend",
        )?;
        require_all(
            "strict_recovery_decode.no_stable_claims",
            &self.no_stable_claims,
            STATE_API_NO_STABLE_CLAIMS,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxCSharpStateMachineParityReport {
    pub artifact_kind: String,
    pub version: u32,
    pub tool: String,
    pub status: String,
    pub target: String,
    pub route: String,
    pub api_scope: String,
    pub candidate_grammar_id: String,
    pub source_path: String,
    pub source_sha256: String,
    pub metadata_path: String,
    pub metadata_sha256: String,
    pub parser_tool: String,
    pub parser_version: String,
    pub structural_model_id: String,
    pub token_layout: ProjectionTokenLayoutSummary,
    pub raw_token_count: usize,
    pub raw_token_ids_sha256: String,
    pub max_raw_token_id: u32,
    pub complete_stream: SyntaxReplayCaseResult,
    pub selected_completion_trace: SyntaxSelectedCompletionTraceReport,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scratch_generation_summary: Option<CSharpScratchGenerationSummary>,
    pub state_cases: Vec<SyntaxGeneratedCodecStateCase>,
    pub transition_cases: Vec<SyntaxGeneratedCodecTransitionCase>,
    pub marker_presence_decisions: Vec<CSharpMarkerPresenceDecision>,
    pub invalid_mutation_cases: Vec<SyntaxReplayCaseResult>,
    pub marker_presence_invalid_payload_mutations: Vec<SyntaxReplayCaseResult>,
    pub strict_cases: Vec<SyntaxGeneratedCodecStrictDecodeCase>,
    pub recovery_cases: Vec<SyntaxGeneratedCodecRecoveryDecodeCase>,
    pub state_after_prefix_ready: bool,
    pub admissible_next_raw_tokens_ready: bool,
    pub invalid_mutation_rejection_ready: bool,
    pub strict_recovery_decode_agreement_ready: bool,
    pub consumed_token_accounting_ok: bool,
    pub token_layout_invariants_ok: bool,
    pub target_metadata_sidecar_only: bool,
    pub arena_ir_used: bool,
    pub bpe_layer_defines_validity: bool,
    pub cross_target_parity_claim_scope: String,
    pub next_ready_task: String,
    pub validation_commands: Vec<String>,
    pub claim_boundary: String,
    pub no_stable_claims: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CSharpScratchGenerationSummary {
    pub status: String,
    pub scope: String,
    pub precompiled_artifact_kind: String,
    pub precompiled_artifact_version: u32,
    pub precompiled_artifact_grammar_id: String,
    pub precompiled_artifact_metadata_path: String,
    pub precompiled_artifact_metadata_sha256: String,
    pub precompiled_artifact_structural_model_id: String,
    pub raw_state_machine_core_boundary_id: String,
    pub raw_state_machine_core_boundary: String,
    pub token_layout_vocab_size: u32,
    pub pad_token_id: u32,
    pub empty_prefix_root: String,
    pub initial_state_observed_status: String,
    pub initial_state_consumed_raw_tokens: usize,
    pub initial_state_total_raw_tokens: usize,
    pub initial_state_eof_valid: bool,
    pub initial_state_stack_depth: usize,
    pub initial_state_active_rule: Option<String>,
    pub initial_state_active_field: Option<String>,
    pub finite_admissible_raw_token_count: usize,
    pub finite_admissible_raw_token_ids: Vec<u32>,
    pub finite_admissible_raw_token_ids_sha256: String,
    pub selected_raw_token_id: u32,
    pub selected_raw_token_admitted: bool,
    pub selected_transition_accepted: bool,
    pub selected_transition_observed_status: String,
    pub selected_transition_consumed_raw_tokens: usize,
    pub selected_transition_eof_valid: bool,
    pub selected_transition_stack_depth: usize,
    pub selected_transition_active_rule: Option<String>,
    pub selected_transition_active_field: Option<String>,
    pub selected_completion_trace_scope: String,
    pub selected_completion_raw_token_count: usize,
    pub selected_completion_transition_count: usize,
    pub selected_completion_admissible_mask_query_count: usize,
    pub selected_completion_every_token_admitted: bool,
    pub selected_completion_final_status: String,
    pub selected_completion_final_eof_valid: bool,
    pub strict_incomplete_prefix_rejected: bool,
    pub strict_incomplete_prefix_error_kind: String,
    pub strict_incomplete_prefix_consumed_raw_tokens: usize,
    pub strict_incomplete_prefix_total_raw_tokens: usize,
    pub token_only_decode_render_ready: bool,
    pub token_only_raw_token_count: usize,
    pub token_only_raw_token_ids_sha256: String,
    pub token_only_root_rule: String,
    pub token_only_root_node_kind: String,
    pub token_only_node_start_count: usize,
    pub token_only_payload_value_count: usize,
    pub renderer_id: String,
    pub renderer_scope: String,
    pub rendered_source_sha256: String,
    pub rendered_source_matches_checked_source: bool,
    pub parser_sidecar_validation: String,
    pub parser_sidecar_validation_command: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub render_reparse_sidecar_blocker: Option<String>,
    pub per_output_parser_metadata_required: bool,
    pub source_prefix_table_used: bool,
    pub arena_ir_used: bool,
    pub route_handoff_provenance_used: bool,
    pub bpe_validity_authority_used: bool,
    pub stable_backend_claimed: bool,
    pub next_ready_task: String,
    pub claim_boundary: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxSelectedCompletionTraceReport {
    pub scope: String,
    pub selection_loop_source: String,
    pub raw_token_count: usize,
    pub transition_count: usize,
    pub admissible_mask_query_count: usize,
    pub finite_admissible_raw_token_mask_available: bool,
    pub every_selected_raw_token_admitted: bool,
    pub selected_transition_accepted_count: usize,
    pub first_selected_raw_token_id: u32,
    pub final_selected_raw_token_id: u32,
    pub final_state_observed_status: String,
    pub final_state_consumed_raw_tokens: usize,
    pub final_state_eof_valid: bool,
    pub final_state_stack_depth: usize,
    pub strict_decode_agreement_case_id: String,
    pub strict_decode_agreement_observed_status: String,
    pub strict_decode_agreement_eof_valid: bool,
    pub step_trace_sample: Vec<SyntaxSelectedCompletionStepSample>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxSelectedCompletionStepSample {
    pub step_index: usize,
    pub prefix_raw_token_count: usize,
    pub selected_raw_token_id: u32,
    pub finite_admissible_raw_token_count: usize,
    pub selected_raw_token_admitted: bool,
    pub selected_transition_accepted: bool,
    pub transition_observed_status: String,
    pub transition_consumed_raw_tokens: usize,
    pub transition_eof_valid: bool,
    pub transition_stack_depth: usize,
    pub transition_active_rule: Option<String>,
    pub transition_active_field: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CSharpMarkerPresenceDecision {
    pub owner_rule: String,
    pub owner_field: String,
    pub selected_node_kind: String,
    pub payload_kind: String,
    pub terminal_raw_token_offset: usize,
    pub first_payload_raw_token_offset: usize,
    pub payload_end_raw_token_offset: usize,
    pub first_payload_raw_token_id: u32,
    pub terminal_event_decoded: bool,
    pub string_payload_byte_decoded: bool,
    pub payload_end_decoded: bool,
    pub state_after_terminal_admits_utf8_payload_range: bool,
    pub state_after_payload_byte_admits_payload_end: bool,
    pub generated_marker_test_marker: String,
    pub generated_codec_marker_evidence_preserved: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CSharpStructuralReplayTokenStream {
    pub artifact_kind: String,
    pub version: u32,
    pub token_layout_kind: String,
    pub source: String,
    pub structural_model_id: String,
    pub token_layout: ProjectionTokenLayoutSummary,
    pub raw_token_count: usize,
    pub raw_token_ids_sha256: String,
    pub max_raw_token_id: Option<u32>,
    pub raw_tokens_below_vocab_size: bool,
    pub pad_token_absent: bool,
    pub raw_token_segment_count: usize,
    pub raw_token_segments_sha256: String,
    pub raw_token_segments: Vec<ProjectionRawTokenSegment>,
    pub raw_token_ids: Vec<u32>,
}

impl CSharpStructuralReplayTokenStream {
    pub fn validate(&self) -> Result<(), SyntaxBackendBoundaryError> {
        require_string(
            "csharp_structural_replay.artifact_kind",
            &self.artifact_kind,
            CSHARP_STRUCTURAL_REPLAY_TOKEN_STREAM_ARTIFACT_KIND,
        )?;
        require_u32("csharp_structural_replay.version", self.version, VERSION)?;
        require_string(
            "csharp_structural_replay.token_layout_kind",
            &self.token_layout_kind,
            CSHARP_STRUCTURAL_REPLAY_TOKEN_LAYOUT_KIND,
        )?;
        require_string(
            "csharp_structural_replay.source",
            &self.source,
            CSHARP_STRUCTURAL_REPLAY_TOKEN_STREAM_SOURCE,
        )?;
        require_non_empty(
            "csharp_structural_replay.structural_model_id",
            &self.structural_model_id,
        )?;
        validate_projection_token_layout(&self.token_layout)?;
        require_usize(
            "csharp_structural_replay.raw_token_count",
            self.raw_token_count,
            self.raw_token_ids.len(),
        )?;
        let raw_token_ids_sha256 = json_sha256(&self.raw_token_ids).map_err(|error| {
            SyntaxBackendBoundaryError::new(
                "csharp_structural_replay.raw_token_ids_sha256",
                format!("failed to hash C# structural replay tokens: {error}"),
            )
        })?;
        require_string(
            "csharp_structural_replay.raw_token_ids_sha256",
            &self.raw_token_ids_sha256,
            &raw_token_ids_sha256,
        )?;
        if self.max_raw_token_id != self.raw_token_ids.iter().copied().max() {
            return Err(SyntaxBackendBoundaryError::new(
                "csharp_structural_replay.max_raw_token_id",
                "max raw token ID must match the structural replay token stream",
            ));
        }
        require_bool(
            "csharp_structural_replay.raw_tokens_below_vocab_size",
            self.raw_tokens_below_vocab_size,
            true,
        )?;
        if !self
            .raw_token_ids
            .iter()
            .all(|token| *token < self.token_layout.vocab_size)
        {
            return Err(SyntaxBackendBoundaryError::new(
                "csharp_structural_replay.raw_token_ids",
                "every structural replay token must be below VOCAB_SIZE",
            ));
        }
        require_bool(
            "csharp_structural_replay.pad_token_absent",
            self.pad_token_absent,
            true,
        )?;
        if self.raw_token_ids.contains(&self.token_layout.pad_token) {
            return Err(SyntaxBackendBoundaryError::new(
                "csharp_structural_replay.raw_token_ids",
                "structural replay token stream must not include PAD_TOKEN",
            ));
        }
        require_usize(
            "csharp_structural_replay.raw_token_segment_count",
            self.raw_token_segment_count,
            self.raw_token_segments.len(),
        )?;
        require_positive_usize(
            "csharp_structural_replay.raw_token_segment_count",
            self.raw_token_segment_count,
        )?;
        let raw_token_segments_sha256 = json_sha256(&self.raw_token_segments).map_err(|error| {
            SyntaxBackendBoundaryError::new(
                "csharp_structural_replay.raw_token_segments_sha256",
                format!("failed to hash C# structural replay token segments: {error}"),
            )
        })?;
        require_string(
            "csharp_structural_replay.raw_token_segments_sha256",
            &self.raw_token_segments_sha256,
            &raw_token_segments_sha256,
        )?;
        validate_raw_token_segments(&self.raw_token_segments, self.raw_token_count)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoAstDeclarationPresenceDecision {
    pub owner_rule: String,
    pub owner_field: String,
    pub domain_id: String,
    pub selected_node_kind: String,
    pub boolean_payload_kind: String,
    pub terminal_raw_token_offset: usize,
    pub boolean_payload_raw_token_offset: usize,
    pub boolean_payload_raw_token_id: u32,
    pub state_after_terminal_finite_admissible_raw_token_ids: Vec<u32>,
    pub terminal_event_decoded: bool,
    pub boolean_payload_decoded: bool,
    pub state_after_terminal_admits_false: bool,
    pub state_after_terminal_admits_true: bool,
}

impl GoAstDeclarationPresenceDecision {
    fn validate(&self) -> Result<(), SyntaxBackendBoundaryError> {
        require_non_empty(
            "go_ast_declaration_presence.decision.owner_rule",
            &self.owner_rule,
        )?;
        require_non_empty(
            "go_ast_declaration_presence.decision.owner_field",
            &self.owner_field,
        )?;
        require_non_empty(
            "go_ast_declaration_presence.decision.domain_id",
            &self.domain_id,
        )?;
        require_contains(
            "go_ast_declaration_presence.decision.selected_node_kind",
            &self.selected_node_kind,
            ".PRESENT",
        )?;
        require_string(
            "go_ast_declaration_presence.decision.boolean_payload_kind",
            &self.boolean_payload_kind,
            "boolean",
        )?;
        if self.boolean_payload_raw_token_offset != self.terminal_raw_token_offset + 1 {
            return Err(SyntaxBackendBoundaryError::new(
                "go_ast_declaration_presence.decision.boolean_payload_raw_token_offset",
                "boolean payload token must immediately follow the terminal token",
            ));
        }
        require_bool(
            "go_ast_declaration_presence.decision.terminal_event_decoded",
            self.terminal_event_decoded,
            true,
        )?;
        require_bool(
            "go_ast_declaration_presence.decision.boolean_payload_decoded",
            self.boolean_payload_decoded,
            true,
        )?;
        require_positive_usize(
            "go_ast_declaration_presence.decision.state_after_terminal_finite_admissible_raw_token_ids",
            self.state_after_terminal_finite_admissible_raw_token_ids
                .len(),
        )?;
        if !self
            .state_after_terminal_finite_admissible_raw_token_ids
            .contains(&self.boolean_payload_raw_token_id)
        {
            return Err(SyntaxBackendBoundaryError::new(
                "go_ast_declaration_presence.decision.state_after_terminal_finite_admissible_raw_token_ids",
                "state-after-terminal mask must admit the selected boolean payload token",
            ));
        }
        require_bool(
            "go_ast_declaration_presence.decision.state_after_terminal_admits_false",
            self.state_after_terminal_admits_false,
            true,
        )?;
        require_bool(
            "go_ast_declaration_presence.decision.state_after_terminal_admits_true",
            self.state_after_terminal_admits_true,
            true,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoAstDeclarationPresenceReplayReport {
    pub artifact_kind: String,
    pub version: u32,
    pub tool: String,
    pub status: String,
    pub target: String,
    pub route: String,
    pub scope: String,
    pub inventory_path: String,
    pub pinned_go_version: String,
    pub inventory_grammar_id: String,
    pub replay_grammar_id: String,
    pub structural_model_id: String,
    pub token_layout: ProjectionTokenLayoutSummary,
    pub raw_token_count: usize,
    pub raw_token_ids_sha256: String,
    pub raw_token_segments_sha256: String,
    pub declaration_replay_path: Vec<String>,
    pub presence_decisions: Vec<GoAstDeclarationPresenceDecision>,
    pub complete_stream: SyntaxReplayCaseResult,
    pub invalid_boolean_payload_mutation: SyntaxReplayCaseResult,
    pub raw_state_machine_complete: bool,
    pub declaration_presence_decisions_consumed: bool,
    pub state_after_terminal_admissible_boolean_ready: bool,
    pub inventory_backed_structural_model_used: bool,
    pub arena_ir_used: bool,
    pub bpe_layer_defines_validity: bool,
    pub stable_backend_promotion_claimed: bool,
    pub next_ready_task: String,
    pub validation_commands: Vec<String>,
    pub claim_boundary: String,
    pub no_stable_claims: Vec<String>,
}

impl GoAstDeclarationPresenceReplayReport {
    pub fn validate(&self) -> Result<(), SyntaxBackendBoundaryError> {
        require_string(
            "go_ast_declaration_presence.artifact_kind",
            &self.artifact_kind,
            GO_AST_DECLARATION_PRESENCE_REPLAY_ARTIFACT_KIND,
        )?;
        require_u32("go_ast_declaration_presence.version", self.version, VERSION)?;
        require_string("go_ast_declaration_presence.tool", &self.tool, TOOL)?;
        require_string(
            "go_ast_declaration_presence.status",
            &self.status,
            GO_AST_DECLARATION_PRESENCE_REPLAY_STATUS,
        )?;
        require_string(
            "go_ast_declaration_presence.target",
            &self.target,
            GO_AST_DECLARATION_PRESENCE_REPLAY_TARGET,
        )?;
        require_string(
            "go_ast_declaration_presence.route",
            &self.route,
            GO_AST_DECLARATION_PRESENCE_REPLAY_ROUTE,
        )?;
        require_string(
            "go_ast_declaration_presence.scope",
            &self.scope,
            GO_AST_DECLARATION_PRESENCE_REPLAY_SCOPE,
        )?;
        require_string(
            "go_ast_declaration_presence.inventory_path",
            &self.inventory_path,
            GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH,
        )?;
        require_string(
            "go_ast_declaration_presence.pinned_go_version",
            &self.pinned_go_version,
            GO_AST_SCHEMA_INVENTORY_PINNED_GO_VERSION,
        )?;
        require_non_empty(
            "go_ast_declaration_presence.inventory_grammar_id",
            &self.inventory_grammar_id,
        )?;
        require_contains(
            "go_ast_declaration_presence.replay_grammar_id",
            &self.replay_grammar_id,
            "declaration_presence_replay",
        )?;
        require_non_empty(
            "go_ast_declaration_presence.structural_model_id",
            &self.structural_model_id,
        )?;
        validate_projection_token_layout(&self.token_layout)?;
        require_positive_usize(
            "go_ast_declaration_presence.raw_token_count",
            self.raw_token_count,
        )?;
        require_sha256(
            "go_ast_declaration_presence.raw_token_ids_sha256",
            &self.raw_token_ids_sha256,
        )?;
        require_sha256(
            "go_ast_declaration_presence.raw_token_segments_sha256",
            &self.raw_token_segments_sha256,
        )?;
        require_usize(
            "go_ast_declaration_presence.declaration_replay_path",
            self.declaration_replay_path.len(),
            10,
        )?;
        require_usize(
            "go_ast_declaration_presence.presence_decisions",
            self.presence_decisions.len(),
            5,
        )?;
        for decision in &self.presence_decisions {
            decision.validate()?;
        }
        self.complete_stream.validate()?;
        self.invalid_boolean_payload_mutation.validate()?;
        require_bool(
            "go_ast_declaration_presence.complete_stream.accepted",
            self.complete_stream.accepted,
            true,
        )?;
        require_bool(
            "go_ast_declaration_presence.invalid_boolean_payload_mutation.accepted",
            self.invalid_boolean_payload_mutation.accepted,
            false,
        )?;
        require_string(
            "go_ast_declaration_presence.invalid_boolean_payload_mutation.error_kind",
            self.invalid_boolean_payload_mutation
                .error_kind
                .as_deref()
                .unwrap_or_default(),
            "invalid_payload_token",
        )?;
        require_bool(
            "go_ast_declaration_presence.raw_state_machine_complete",
            self.raw_state_machine_complete,
            true,
        )?;
        require_bool(
            "go_ast_declaration_presence.declaration_presence_decisions_consumed",
            self.declaration_presence_decisions_consumed,
            true,
        )?;
        require_bool(
            "go_ast_declaration_presence.state_after_terminal_admissible_boolean_ready",
            self.state_after_terminal_admissible_boolean_ready,
            true,
        )?;
        require_bool(
            "go_ast_declaration_presence.inventory_backed_structural_model_used",
            self.inventory_backed_structural_model_used,
            true,
        )?;
        require_bool(
            "go_ast_declaration_presence.arena_ir_used",
            self.arena_ir_used,
            false,
        )?;
        require_bool(
            "go_ast_declaration_presence.bpe_layer_defines_validity",
            self.bpe_layer_defines_validity,
            false,
        )?;
        require_bool(
            "go_ast_declaration_presence.stable_backend_promotion_claimed",
            self.stable_backend_promotion_claimed,
            false,
        )?;
        require_string(
            "go_ast_declaration_presence.next_ready_task",
            &self.next_ready_task,
            GO_AST_DECLARATION_PRESENCE_REPLAY_NEXT_READY_TASK,
        )?;
        require_contains(
            "go_ast_declaration_presence.claim_boundary",
            &self.claim_boundary,
            "not arena-IR",
        )?;
        require_contains(
            "go_ast_declaration_presence.claim_boundary",
            &self.claim_boundary,
            "not BPE",
        )?;
        require_positive_usize(
            "go_ast_declaration_presence.no_stable_claims",
            self.no_stable_claims.len(),
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxCSharpRealSourceStateMachineParityReport {
    pub artifact_kind: String,
    pub version: u32,
    pub tool: String,
    pub status: String,
    pub target: String,
    pub route: String,
    pub corpus_fixture_path: String,
    pub source_report_paths: Vec<String>,
    pub supported_record_ids: Vec<String>,
    pub generated_codec_report_artifact_kind: String,
    pub generated_codec_report_checked_in: bool,
    pub generated_codec_report_paths: Vec<String>,
    pub generated_codec_report_commands: Vec<String>,
    pub generated_codec_report_fields_preserved: Vec<String>,
    pub replay_required_report_fields: Vec<String>,
    pub missing_replay_report_fields: Vec<String>,
    pub source_record_raw_token_emission_api_id: String,
    pub source_record_raw_token_emission_api_boundary: String,
    pub corpus_replay_record_count: usize,
    pub corpus_replay_records: Vec<SyntaxCSharpRealSourceStateMachineParityCorpusRecord>,
    pub payload_sequence_control_token_count: usize,
    pub payload_sequence_control_preserved: bool,
    pub raw_token_count: usize,
    pub raw_token_ids_sha256: String,
    pub raw_tokens_below_vocab_size: bool,
    pub pad_token_absent: bool,
    pub raw_state_machine_complete: bool,
    pub state_case_count: usize,
    pub transition_case_count: usize,
    pub strict_decode_case_count: usize,
    pub recovery_decode_case_count: usize,
    pub payload_range_state_count: usize,
    pub eof_valid_state_count: usize,
    pub payload_sequence_control_payload_end_collides_with_structural_terminal: bool,
    pub structural_replay_payload_end_matches_structural_payload_end: bool,
    pub raw_token_segment_bridge_ready: bool,
    pub pipeline: SyntaxCSharpRealSourceStateMachineParityPipeline,
    pub structural_model_metadata_available: bool,
    pub target_metadata_sidecar_only: bool,
    pub arena_ir_used: bool,
    pub source_program_validity_claim_allowed: bool,
    pub type_correctness_claim_allowed: bool,
    pub stable_promotion_allowed: bool,
    pub next_ready_task: String,
    pub validation_commands: Vec<String>,
    pub claim_boundary: String,
    pub no_stable_claims: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxCSharpRealSourceStateMachineParityPipeline {
    pub parser_backed_metadata_loaded: bool,
    pub grammar_ir_lowered: bool,
    pub structural_model_lowered: bool,
    pub token_layout_compiled: bool,
    pub raw_state_machine_compiled: bool,
    pub raw_token_encode_ready: bool,
    pub raw_state_machine_decode_ready: bool,
    pub parser_sidecar_validation_ready: bool,
    pub arena_ir_used: bool,
    pub route_handoff_provenance_used: bool,
    pub bpe_validity_authority_used: bool,
}

impl SyntaxCSharpRealSourceStateMachineParityPipeline {
    fn validate(&self) -> Result<(), SyntaxBackendBoundaryError> {
        require_bool(
            "csharp_real_source_state_machine_parity.pipeline.parser_backed_metadata_loaded",
            self.parser_backed_metadata_loaded,
            true,
        )?;
        require_bool(
            "csharp_real_source_state_machine_parity.pipeline.grammar_ir_lowered",
            self.grammar_ir_lowered,
            true,
        )?;
        require_bool(
            "csharp_real_source_state_machine_parity.pipeline.structural_model_lowered",
            self.structural_model_lowered,
            true,
        )?;
        require_bool(
            "csharp_real_source_state_machine_parity.pipeline.token_layout_compiled",
            self.token_layout_compiled,
            true,
        )?;
        require_bool(
            "csharp_real_source_state_machine_parity.pipeline.raw_state_machine_compiled",
            self.raw_state_machine_compiled,
            true,
        )?;
        require_bool(
            "csharp_real_source_state_machine_parity.pipeline.raw_token_encode_ready",
            self.raw_token_encode_ready,
            true,
        )?;
        require_bool(
            "csharp_real_source_state_machine_parity.pipeline.raw_state_machine_decode_ready",
            self.raw_state_machine_decode_ready,
            true,
        )?;
        require_bool(
            "csharp_real_source_state_machine_parity.pipeline.parser_sidecar_validation_ready",
            self.parser_sidecar_validation_ready,
            true,
        )?;
        require_bool(
            "csharp_real_source_state_machine_parity.pipeline.arena_ir_used",
            self.arena_ir_used,
            false,
        )?;
        require_bool(
            "csharp_real_source_state_machine_parity.pipeline.route_handoff_provenance_used",
            self.route_handoff_provenance_used,
            false,
        )?;
        require_bool(
            "csharp_real_source_state_machine_parity.pipeline.bpe_validity_authority_used",
            self.bpe_validity_authority_used,
            false,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxCSharpRealSourceStateMachineParityCorpusRecord {
    pub manifest_path: String,
    pub source_path: String,
    pub source_sha256: String,
    pub metadata_path: String,
    pub metadata_sha256: String,
    pub parser_tool: String,
    pub parser_version: String,
    pub parser_package: Option<String>,
    pub parser_package_version: Option<String>,
    pub parser_metadata_validation: String,
    pub parser_sidecar_validation_ready: bool,
    pub generated_codec_report_path: String,
    pub generated_codec_report_command: String,
    pub generated_codec_report_fields_preserved: Vec<String>,
    pub replay_required_report_fields: Vec<String>,
    pub missing_replay_report_fields: Vec<String>,
    pub source_record_raw_token_emission_api_id: String,
    pub source_record_raw_token_emission_api_boundary: String,
    pub payload_sequence_control_token_count: usize,
    pub payload_sequence_control_preserved: bool,
    pub structural_replay_token_stream: CSharpStructuralReplayTokenStream,
    pub complete_stream: SyntaxReplayCaseResult,
    pub state_cases: Vec<SyntaxGeneratedCodecStateCase>,
    pub transition_cases: Vec<SyntaxGeneratedCodecTransitionCase>,
    pub strict_cases: Vec<SyntaxGeneratedCodecStrictDecodeCase>,
    pub recovery_cases: Vec<SyntaxGeneratedCodecRecoveryDecodeCase>,
    pub prefix_state_api_ready: bool,
    pub admissible_next_raw_tokens_ready: bool,
    pub eof_state_api_ready: bool,
    pub strict_recovery_decode_agreement_ready: bool,
    pub consumed_token_accounting_ok: bool,
    pub raw_token_segment_bridge_ready: bool,
    pub structural_model_metadata_available: bool,
    pub target_metadata_sidecar_only: bool,
    pub arena_ir_used: bool,
    pub source_program_validity_claim_allowed: bool,
    pub type_correctness_claim_allowed: bool,
    pub stable_promotion_allowed: bool,
}

impl SyntaxCSharpRealSourceStateMachineParityCorpusRecord {
    fn validate(&self) -> Result<(), SyntaxBackendBoundaryError> {
        require_non_empty(
            "csharp_real_source_state_machine_parity.corpus_record.manifest_path",
            &self.manifest_path,
        )?;
        require_non_empty(
            "csharp_real_source_state_machine_parity.corpus_record.source_path",
            &self.source_path,
        )?;
        require_sha256(
            "csharp_real_source_state_machine_parity.corpus_record.source_sha256",
            &self.source_sha256,
        )?;
        require_contains(
            "csharp_real_source_state_machine_parity.corpus_record.metadata_path",
            &self.metadata_path,
            "tests/fixtures/csharp_syntax_metadata/",
        )?;
        require_sha256(
            "csharp_real_source_state_machine_parity.corpus_record.metadata_sha256",
            &self.metadata_sha256,
        )?;
        require_string(
            "csharp_real_source_state_machine_parity.corpus_record.parser_tool",
            &self.parser_tool,
            "dotnet",
        )?;
        require_non_empty(
            "csharp_real_source_state_machine_parity.corpus_record.parser_version",
            &self.parser_version,
        )?;
        require_string(
            "csharp_real_source_state_machine_parity.corpus_record.parser_package",
            self.parser_package.as_deref().unwrap_or_default(),
            "Microsoft.CodeAnalysis.CSharp",
        )?;
        require_non_empty(
            "csharp_real_source_state_machine_parity.corpus_record.parser_package_version",
            self.parser_package_version.as_deref().unwrap_or_default(),
        )?;
        require_string(
            "csharp_real_source_state_machine_parity.corpus_record.parser_metadata_validation",
            &self.parser_metadata_validation,
            "checked",
        )?;
        require_bool(
            "csharp_real_source_state_machine_parity.corpus_record.parser_sidecar_validation_ready",
            self.parser_sidecar_validation_ready,
            true,
        )?;
        require_contains(
            "csharp_real_source_state_machine_parity.corpus_record.generated_codec_report_path",
            &self.generated_codec_report_path,
            "target/",
        )?;
        require_non_empty(
            "csharp_real_source_state_machine_parity.corpus_record.generated_codec_report_command",
            &self.generated_codec_report_command,
        )?;
        require_all(
            "csharp_real_source_state_machine_parity.corpus_record.generated_codec_report_fields_preserved",
            &self.generated_codec_report_fields_preserved,
            CSHARP_REAL_SOURCE_REPLAY_METADATA_REPORT_FIELDS,
        )?;
        require_all(
            "csharp_real_source_state_machine_parity.corpus_record.replay_required_report_fields",
            &self.replay_required_report_fields,
            CSHARP_REAL_SOURCE_REPLAY_METADATA_REPORT_FIELDS,
        )?;
        require_usize(
            "csharp_real_source_state_machine_parity.corpus_record.missing_replay_report_fields",
            self.missing_replay_report_fields.len(),
            0,
        )?;
        require_string(
            "csharp_real_source_state_machine_parity.corpus_record.source_record_raw_token_emission_api_id",
            &self.source_record_raw_token_emission_api_id,
            CSHARP_SOURCE_RECORD_RAW_TOKEN_EMISSION_API_ID,
        )?;
        require_string(
            "csharp_real_source_state_machine_parity.corpus_record.source_record_raw_token_emission_api_boundary",
            &self.source_record_raw_token_emission_api_boundary,
            CSHARP_SOURCE_RECORD_RAW_TOKEN_EMISSION_API_BOUNDARY,
        )?;
        require_positive_usize(
            "csharp_real_source_state_machine_parity.corpus_record.payload_sequence_control_token_count",
            self.payload_sequence_control_token_count,
        )?;
        require_bool(
            "csharp_real_source_state_machine_parity.corpus_record.payload_sequence_control_preserved",
            self.payload_sequence_control_preserved,
            true,
        )?;
        self.structural_replay_token_stream.validate()?;
        self.complete_stream.validate()?;
        require_bool(
            "csharp_real_source_state_machine_parity.corpus_record.complete_stream.accepted",
            self.complete_stream.accepted,
            true,
        )?;
        require_string(
            "csharp_real_source_state_machine_parity.corpus_record.complete_stream.observed_status",
            &self.complete_stream.observed_status,
            "complete",
        )?;
        require_usize(
            "csharp_real_source_state_machine_parity.corpus_record.complete_stream.consumed_raw_tokens",
            self.complete_stream.consumed_raw_tokens,
            self.structural_replay_token_stream.raw_token_count,
        )?;
        require_positive_usize(
            "csharp_real_source_state_machine_parity.corpus_record.state_cases",
            self.state_cases.len(),
        )?;
        let mut state_case_ids = BTreeSet::new();
        let mut eof_valid_count = 0usize;
        let mut payload_range_state_count = 0usize;
        let mut admissible_prefix_count = 0usize;
        for case in &self.state_cases {
            case.validate(self.structural_replay_token_stream.token_layout.vocab_size)?;
            if !state_case_ids.insert(case.case_id.clone()) {
                return Err(SyntaxBackendBoundaryError::new(
                    "csharp_real_source_state_machine_parity.corpus_record.state_cases.case_id",
                    format!("duplicate state case {}", case.case_id),
                ));
            }
            if case.eof_valid {
                eof_valid_count += 1;
            }
            if !case.admissible_next_raw_token_ranges.is_empty() {
                payload_range_state_count += 1;
            }
            if !case.eof_valid && !case.admissible_next_raw_token_ids.is_empty() {
                admissible_prefix_count += 1;
            }
        }
        require_usize(
            "csharp_real_source_state_machine_parity.corpus_record.state_cases.eof_valid_count",
            eof_valid_count,
            1,
        )?;
        require_positive_usize(
            "csharp_real_source_state_machine_parity.corpus_record.state_cases.payload_range_state_count",
            payload_range_state_count,
        )?;
        require_positive_usize(
            "csharp_real_source_state_machine_parity.corpus_record.state_cases.admissible_prefix_count",
            admissible_prefix_count,
        )?;
        require_positive_usize(
            "csharp_real_source_state_machine_parity.corpus_record.transition_cases",
            self.transition_cases.len(),
        )?;
        let mut accepted_transition_count = 0usize;
        let mut rejected_transition_count = 0usize;
        let mut transition_case_ids = BTreeSet::new();
        for case in &self.transition_cases {
            case.validate(self.structural_replay_token_stream.token_layout.vocab_size)?;
            if !transition_case_ids.insert(case.case_id.clone()) {
                return Err(SyntaxBackendBoundaryError::new(
                    "csharp_real_source_state_machine_parity.corpus_record.transition_cases.case_id",
                    format!("duplicate transition case {}", case.case_id),
                ));
            }
            if case.accepted {
                accepted_transition_count += 1;
            } else {
                rejected_transition_count += 1;
            }
        }
        require_positive_usize(
            "csharp_real_source_state_machine_parity.corpus_record.transition_cases.accepted",
            accepted_transition_count,
        )?;
        require_positive_usize(
            "csharp_real_source_state_machine_parity.corpus_record.transition_cases.rejected",
            rejected_transition_count,
        )?;
        require_positive_usize(
            "csharp_real_source_state_machine_parity.corpus_record.strict_cases",
            self.strict_cases.len(),
        )?;
        for case in &self.strict_cases {
            case.validate(self.structural_replay_token_stream.token_layout.vocab_size)?;
        }
        require_positive_usize(
            "csharp_real_source_state_machine_parity.corpus_record.recovery_cases",
            self.recovery_cases.len(),
        )?;
        for case in &self.recovery_cases {
            case.validate(self.structural_replay_token_stream.token_layout.vocab_size)?;
        }
        require_bool(
            "csharp_real_source_state_machine_parity.corpus_record.prefix_state_api_ready",
            self.prefix_state_api_ready,
            true,
        )?;
        require_bool(
            "csharp_real_source_state_machine_parity.corpus_record.admissible_next_raw_tokens_ready",
            self.admissible_next_raw_tokens_ready,
            true,
        )?;
        require_bool(
            "csharp_real_source_state_machine_parity.corpus_record.eof_state_api_ready",
            self.eof_state_api_ready,
            true,
        )?;
        require_bool(
            "csharp_real_source_state_machine_parity.corpus_record.strict_recovery_decode_agreement_ready",
            self.strict_recovery_decode_agreement_ready,
            true,
        )?;
        require_bool(
            "csharp_real_source_state_machine_parity.corpus_record.consumed_token_accounting_ok",
            self.consumed_token_accounting_ok,
            true,
        )?;
        require_bool(
            "csharp_real_source_state_machine_parity.corpus_record.raw_token_segment_bridge_ready",
            self.raw_token_segment_bridge_ready,
            true,
        )?;
        require_bool(
            "csharp_real_source_state_machine_parity.corpus_record.structural_model_metadata_available",
            self.structural_model_metadata_available,
            true,
        )?;
        require_bool(
            "csharp_real_source_state_machine_parity.corpus_record.target_metadata_sidecar_only",
            self.target_metadata_sidecar_only,
            true,
        )?;
        require_bool(
            "csharp_real_source_state_machine_parity.corpus_record.arena_ir_used",
            self.arena_ir_used,
            false,
        )?;
        require_bool(
            "csharp_real_source_state_machine_parity.corpus_record.source_program_validity_claim_allowed",
            self.source_program_validity_claim_allowed,
            false,
        )?;
        require_bool(
            "csharp_real_source_state_machine_parity.corpus_record.type_correctness_claim_allowed",
            self.type_correctness_claim_allowed,
            false,
        )?;
        require_bool(
            "csharp_real_source_state_machine_parity.corpus_record.stable_promotion_allowed",
            self.stable_promotion_allowed,
            false,
        )
    }
}

impl SyntaxCSharpRealSourceStateMachineParityReport {
    pub fn validate(&self) -> Result<(), SyntaxBackendBoundaryError> {
        require_string(
            "csharp_real_source_state_machine_parity.artifact_kind",
            &self.artifact_kind,
            CSHARP_REAL_SOURCE_STATE_MACHINE_PARITY_ARTIFACT_KIND,
        )?;
        require_u32(
            "csharp_real_source_state_machine_parity.version",
            self.version,
            VERSION,
        )?;
        require_string(
            "csharp_real_source_state_machine_parity.tool",
            &self.tool,
            TOOL,
        )?;
        require_string(
            "csharp_real_source_state_machine_parity.status",
            &self.status,
            CSHARP_REAL_SOURCE_STATE_MACHINE_PARITY_STATUS,
        )?;
        require_string(
            "csharp_real_source_state_machine_parity.target",
            &self.target,
            CSHARP_STATE_MACHINE_PARITY_TARGET,
        )?;
        require_string(
            "csharp_real_source_state_machine_parity.route",
            &self.route,
            CSHARP_PAYLOAD_SEQUENCE_GENERATED_CODEC,
        )?;
        require_string(
            "csharp_real_source_state_machine_parity.corpus_fixture_path",
            &self.corpus_fixture_path,
            CSHARP_REAL_SOURCE_GENERATED_CODEC_CORPUS_FIXTURE_PATH,
        )?;
        require_string(
            "csharp_real_source_state_machine_parity.generated_codec_report_artifact_kind",
            &self.generated_codec_report_artifact_kind,
            "tct_csharp_real_source_generated_codec_demo_v1",
        )?;
        require_bool(
            "csharp_real_source_state_machine_parity.generated_codec_report_checked_in",
            self.generated_codec_report_checked_in,
            false,
        )?;
        require_all(
            "csharp_real_source_state_machine_parity.generated_codec_report_fields_preserved",
            &self.generated_codec_report_fields_preserved,
            CSHARP_REAL_SOURCE_REPLAY_REQUIRED_REPORT_FIELDS,
        )?;
        require_all(
            "csharp_real_source_state_machine_parity.replay_required_report_fields",
            &self.replay_required_report_fields,
            CSHARP_REAL_SOURCE_REPLAY_REQUIRED_REPORT_FIELDS,
        )?;
        require_usize(
            "csharp_real_source_state_machine_parity.missing_replay_report_fields",
            self.missing_replay_report_fields.len(),
            0,
        )?;
        require_string(
            "csharp_real_source_state_machine_parity.source_record_raw_token_emission_api_id",
            &self.source_record_raw_token_emission_api_id,
            CSHARP_SOURCE_RECORD_RAW_TOKEN_EMISSION_API_ID,
        )?;
        require_string(
            "csharp_real_source_state_machine_parity.source_record_raw_token_emission_api_boundary",
            &self.source_record_raw_token_emission_api_boundary,
            CSHARP_SOURCE_RECORD_RAW_TOKEN_EMISSION_API_BOUNDARY,
        )?;
        require_usize(
            "csharp_real_source_state_machine_parity.corpus_replay_record_count",
            self.corpus_replay_record_count,
            self.corpus_replay_records.len(),
        )?;
        require_usize(
            "csharp_real_source_state_machine_parity.corpus_replay_records",
            self.corpus_replay_records.len(),
            self.corpus_replay_record_count,
        )?;
        require_positive_usize(
            "csharp_real_source_state_machine_parity.corpus_replay_record_count",
            self.corpus_replay_record_count,
        )?;

        let mut observed_manifest_paths = BTreeSet::new();
        let mut expected_source_report_paths = Vec::with_capacity(self.corpus_replay_record_count);
        let mut expected_supported_record_ids = Vec::with_capacity(self.corpus_replay_record_count);
        let mut expected_generated_codec_report_paths =
            Vec::with_capacity(self.corpus_replay_record_count);
        let mut expected_generated_codec_report_commands =
            Vec::with_capacity(self.corpus_replay_record_count);
        let mut raw_token_hashes = Vec::with_capacity(self.corpus_replay_record_count);
        let mut raw_token_count = 0usize;
        let mut payload_sequence_control_token_count = 0usize;
        let mut state_case_count = 0usize;
        let mut transition_case_count = 0usize;
        let mut strict_decode_case_count = 0usize;
        let mut recovery_decode_case_count = 0usize;
        let mut payload_range_state_count = 0usize;
        let mut eof_valid_state_count = 0usize;
        let mut preserved_fields = Vec::new();
        for record in &self.corpus_replay_records {
            record.validate()?;
            if !observed_manifest_paths.insert(record.manifest_path.clone()) {
                return Err(SyntaxBackendBoundaryError::new(
                    "csharp_real_source_state_machine_parity.corpus_replay_records.manifest_path",
                    format!("duplicate C# replay record {}", record.manifest_path),
                ));
            }
            require_string(
                "csharp_real_source_state_machine_parity.corpus_replay_records.route",
                &self.route,
                CSHARP_PAYLOAD_SEQUENCE_GENERATED_CODEC,
            )?;
            expected_source_report_paths.push(csharp_real_source_state_machine_parity_report_path(
                &record.manifest_path,
            ));
            expected_supported_record_ids.push(record.manifest_path.clone());
            expected_generated_codec_report_paths.push(record.generated_codec_report_path.clone());
            expected_generated_codec_report_commands
                .push(record.generated_codec_report_command.clone());
            raw_token_count += record.structural_replay_token_stream.raw_token_count;
            payload_sequence_control_token_count += record.payload_sequence_control_token_count;
            state_case_count += record.state_cases.len();
            transition_case_count += record.transition_cases.len();
            strict_decode_case_count += record.strict_cases.len();
            recovery_decode_case_count += record.recovery_cases.len();
            payload_range_state_count += record
                .state_cases
                .iter()
                .filter(|case| !case.admissible_next_raw_token_ranges.is_empty())
                .count();
            eof_valid_state_count += record
                .state_cases
                .iter()
                .filter(|case| case.eof_valid)
                .count();
            raw_token_hashes.push(
                record
                    .structural_replay_token_stream
                    .raw_token_ids_sha256
                    .clone(),
            );
            require_string(
                "csharp_real_source_state_machine_parity.corpus_replay_records.source_record_raw_token_emission_api_id",
                &record.source_record_raw_token_emission_api_id,
                &self.source_record_raw_token_emission_api_id,
            )?;
            require_string(
                "csharp_real_source_state_machine_parity.corpus_replay_records.source_record_raw_token_emission_api_boundary",
                &record.source_record_raw_token_emission_api_boundary,
                &self.source_record_raw_token_emission_api_boundary,
            )?;
            preserved_fields.extend(
                record
                    .generated_codec_report_fields_preserved
                    .iter()
                    .cloned(),
            );
        }
        if observed_manifest_paths.len() != self.corpus_replay_record_count {
            return Err(SyntaxBackendBoundaryError::new(
                "csharp_real_source_state_machine_parity.corpus_replay_records.manifest_paths",
                "C# replay records must have unique manifest paths",
            ));
        }
        preserved_fields.sort();
        preserved_fields.dedup();
        require_exact_strings(
            "csharp_real_source_state_machine_parity.source_report_paths",
            &self.source_report_paths,
            &expected_source_report_paths,
        )?;
        require_exact_strings(
            "csharp_real_source_state_machine_parity.supported_record_ids",
            &self.supported_record_ids,
            &expected_supported_record_ids,
        )?;
        require_exact_strings(
            "csharp_real_source_state_machine_parity.generated_codec_report_paths",
            &self.generated_codec_report_paths,
            &expected_generated_codec_report_paths,
        )?;
        require_exact_strings(
            "csharp_real_source_state_machine_parity.generated_codec_report_commands",
            &self.generated_codec_report_commands,
            &expected_generated_codec_report_commands,
        )?;
        require_all(
            "csharp_real_source_state_machine_parity.generated_codec_report_fields_preserved",
            &self.generated_codec_report_fields_preserved,
            &preserved_fields
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>(),
        )?;
        require_positive_usize(
            "csharp_real_source_state_machine_parity.payload_sequence_control_token_count",
            self.payload_sequence_control_token_count,
        )?;
        require_usize(
            "csharp_real_source_state_machine_parity.payload_sequence_control_token_count",
            self.payload_sequence_control_token_count,
            payload_sequence_control_token_count,
        )?;
        require_bool(
            "csharp_real_source_state_machine_parity.payload_sequence_control_preserved",
            self.payload_sequence_control_preserved,
            true,
        )?;
        require_usize(
            "csharp_real_source_state_machine_parity.raw_token_count",
            self.raw_token_count,
            raw_token_count,
        )?;
        let expected_raw_token_ids_sha256 = json_sha256(&raw_token_hashes).map_err(|error| {
            SyntaxBackendBoundaryError::new(
                "csharp_real_source_state_machine_parity.raw_token_ids_sha256",
                format!("failed to hash C# replay record hashes: {error}"),
            )
        })?;
        require_string(
            "csharp_real_source_state_machine_parity.raw_token_ids_sha256",
            &self.raw_token_ids_sha256,
            &expected_raw_token_ids_sha256,
        )?;
        require_bool(
            "csharp_real_source_state_machine_parity.raw_tokens_below_vocab_size",
            self.raw_tokens_below_vocab_size,
            true,
        )?;
        require_bool(
            "csharp_real_source_state_machine_parity.pad_token_absent",
            self.pad_token_absent,
            true,
        )?;
        require_bool(
            "csharp_real_source_state_machine_parity.raw_state_machine_complete",
            self.raw_state_machine_complete,
            true,
        )?;
        require_usize(
            "csharp_real_source_state_machine_parity.state_case_count",
            self.state_case_count,
            state_case_count,
        )?;
        require_usize(
            "csharp_real_source_state_machine_parity.transition_case_count",
            self.transition_case_count,
            transition_case_count,
        )?;
        require_usize(
            "csharp_real_source_state_machine_parity.strict_decode_case_count",
            self.strict_decode_case_count,
            strict_decode_case_count,
        )?;
        require_usize(
            "csharp_real_source_state_machine_parity.recovery_decode_case_count",
            self.recovery_decode_case_count,
            recovery_decode_case_count,
        )?;
        require_usize(
            "csharp_real_source_state_machine_parity.payload_range_state_count",
            self.payload_range_state_count,
            payload_range_state_count,
        )?;
        require_usize(
            "csharp_real_source_state_machine_parity.eof_valid_state_count",
            self.eof_valid_state_count,
            eof_valid_state_count,
        )?;
        require_positive_usize(
            "csharp_real_source_state_machine_parity.payload_range_state_count",
            self.payload_range_state_count,
        )?;
        require_bool(
            "csharp_real_source_state_machine_parity.payload_sequence_control_payload_end_collides_with_structural_terminal",
            self.payload_sequence_control_payload_end_collides_with_structural_terminal,
            true,
        )?;
        require_bool(
            "csharp_real_source_state_machine_parity.structural_replay_payload_end_matches_structural_payload_end",
            self.structural_replay_payload_end_matches_structural_payload_end,
            true,
        )?;
        require_bool(
            "csharp_real_source_state_machine_parity.raw_token_segment_bridge_ready",
            self.raw_token_segment_bridge_ready,
            true,
        )?;
        self.pipeline.validate()?;
        require_bool(
            "csharp_real_source_state_machine_parity.structural_model_metadata_available",
            self.structural_model_metadata_available,
            true,
        )?;
        require_bool(
            "csharp_real_source_state_machine_parity.target_metadata_sidecar_only",
            self.target_metadata_sidecar_only,
            true,
        )?;
        require_bool(
            "csharp_real_source_state_machine_parity.arena_ir_used",
            self.arena_ir_used,
            false,
        )?;
        require_bool(
            "csharp_real_source_state_machine_parity.source_program_validity_claim_allowed",
            self.source_program_validity_claim_allowed,
            false,
        )?;
        require_bool(
            "csharp_real_source_state_machine_parity.type_correctness_claim_allowed",
            self.type_correctness_claim_allowed,
            false,
        )?;
        require_bool(
            "csharp_real_source_state_machine_parity.stable_promotion_allowed",
            self.stable_promotion_allowed,
            false,
        )?;
        require_non_empty(
            "csharp_real_source_state_machine_parity.next_ready_task",
            &self.next_ready_task,
        )?;
        require_contains(
            "csharp_real_source_state_machine_parity.claim_boundary",
            &self.claim_boundary,
            "syntax-level",
        )?;
        require_contains(
            "csharp_real_source_state_machine_parity.claim_boundary",
            &self.claim_boundary,
            "not arena-IR",
        )?;
        require_contains(
            "csharp_real_source_state_machine_parity.claim_boundary",
            &self.claim_boundary,
            "not stable-backend",
        )?;
        require_all(
            "csharp_real_source_state_machine_parity.no_stable_claims",
            &self.no_stable_claims,
            CSHARP_PARITY_NO_STABLE_CLAIMS,
        )
    }
}

impl SyntaxCSharpStateMachineParityReport {
    pub fn validate(&self) -> Result<(), SyntaxBackendBoundaryError> {
        require_string(
            "csharp_state_machine_parity.artifact_kind",
            &self.artifact_kind,
            CSHARP_STATE_MACHINE_PARITY_ARTIFACT_KIND,
        )?;
        require_u32("csharp_state_machine_parity.version", self.version, VERSION)?;
        require_string("csharp_state_machine_parity.tool", &self.tool, TOOL)?;
        require_string(
            "csharp_state_machine_parity.status",
            &self.status,
            CSHARP_STATE_MACHINE_PARITY_STATUS,
        )?;
        require_string(
            "csharp_state_machine_parity.target",
            &self.target,
            CSHARP_STATE_MACHINE_PARITY_TARGET,
        )?;
        require_string(
            "csharp_state_machine_parity.route",
            &self.route,
            CSHARP_STATE_MACHINE_PARITY_ROUTE,
        )?;
        require_string(
            "csharp_state_machine_parity.api_scope",
            &self.api_scope,
            CSHARP_STATE_MACHINE_PARITY_API_SCOPE,
        )?;
        require_non_empty(
            "csharp_state_machine_parity.candidate_grammar_id",
            &self.candidate_grammar_id,
        )?;
        require_non_empty("csharp_state_machine_parity.source_path", &self.source_path)?;
        require_sha256(
            "csharp_state_machine_parity.source_sha256",
            &self.source_sha256,
        )?;
        require_non_empty(
            "csharp_state_machine_parity.metadata_path",
            &self.metadata_path,
        )?;
        require_sha256(
            "csharp_state_machine_parity.metadata_sha256",
            &self.metadata_sha256,
        )?;
        require_string(
            "csharp_state_machine_parity.parser_tool",
            &self.parser_tool,
            "dotnet",
        )?;
        require_non_empty(
            "csharp_state_machine_parity.parser_version",
            &self.parser_version,
        )?;
        require_string(
            "csharp_state_machine_parity.structural_model_id",
            &self.structural_model_id,
            &format!(
                "grammar-ir:{}:structural-model-v0",
                self.candidate_grammar_id
            ),
        )?;
        validate_projection_token_layout(&self.token_layout)?;
        require_positive_usize(
            "csharp_state_machine_parity.raw_token_count",
            self.raw_token_count,
        )?;
        require_sha256(
            "csharp_state_machine_parity.raw_token_ids_sha256",
            &self.raw_token_ids_sha256,
        )?;
        if self.max_raw_token_id >= self.token_layout.vocab_size {
            return Err(SyntaxBackendBoundaryError::new(
                "csharp_state_machine_parity.max_raw_token_id",
                "max raw token ID must remain below VOCAB_SIZE",
            ));
        }

        self.complete_stream.validate()?;
        require_bool(
            "csharp_state_machine_parity.complete_stream.accepted",
            self.complete_stream.accepted,
            true,
        )?;
        require_string(
            "csharp_state_machine_parity.complete_stream.observed_status",
            &self.complete_stream.observed_status,
            "complete",
        )?;
        require_usize(
            "csharp_state_machine_parity.complete_stream.consumed_raw_tokens",
            self.complete_stream.consumed_raw_tokens,
            self.raw_token_count,
        )?;
        self.selected_completion_trace.validate(
            self.raw_token_count,
            self.token_layout.vocab_size,
            &self.complete_stream,
        )?;
        if csharp_scratch_generation_scope(&self.candidate_grammar_id).is_some()
            && self.scratch_generation_summary.is_none()
        {
            return Err(SyntaxBackendBoundaryError::new(
                "csharp_state_machine_parity.scratch_generation_summary",
                format!(
                    "{} C# parity report must preserve scratch-generation summary evidence",
                    self.candidate_grammar_id
                ),
            ));
        }
        if csharp_scratch_generation_scope(&self.candidate_grammar_id).is_none()
            && self.scratch_generation_summary.is_some()
        {
            return Err(SyntaxBackendBoundaryError::new(
                "csharp_state_machine_parity.scratch_generation_summary",
                format!(
                    "{} C# parity report is not a scratch-generation owner route",
                    self.candidate_grammar_id
                ),
            ));
        }
        if let Some(summary) = &self.scratch_generation_summary {
            summary.validate(self)?;
        }

        require_positive_usize(
            "csharp_state_machine_parity.state_cases",
            self.state_cases.len(),
        )?;
        let mut state_ids = BTreeSet::new();
        let mut eof_valid_count = 0usize;
        let mut payload_range_state_count = 0usize;
        for case in &self.state_cases {
            case.validate(self.token_layout.vocab_size)?;
            if !state_ids.insert(case.case_id.clone()) {
                return Err(SyntaxBackendBoundaryError::new(
                    "csharp_state_machine_parity.state_cases.case_id",
                    format!("duplicate state case {}", case.case_id),
                ));
            }
            if case.eof_valid {
                eof_valid_count += 1;
            }
            if !case.admissible_next_raw_token_ranges.is_empty() {
                payload_range_state_count += 1;
            }
        }
        require_usize(
            "csharp_state_machine_parity.state_cases.eof_valid_count",
            eof_valid_count,
            1,
        )?;
        require_positive_usize(
            "csharp_state_machine_parity.state_cases.payload_range_state_count",
            payload_range_state_count,
        )?;

        require_positive_usize(
            "csharp_state_machine_parity.transition_cases",
            self.transition_cases.len(),
        )?;
        let mut accepted_transition_count = 0usize;
        let mut rejected_transition_count = 0usize;
        for case in &self.transition_cases {
            case.validate(self.token_layout.vocab_size)?;
            if case.accepted {
                accepted_transition_count += 1;
            } else {
                rejected_transition_count += 1;
            }
        }
        require_positive_usize(
            "csharp_state_machine_parity.transition_cases.accepted",
            accepted_transition_count,
        )?;
        require_positive_usize(
            "csharp_state_machine_parity.transition_cases.rejected",
            rejected_transition_count,
        )?;

        for decision in &self.marker_presence_decisions {
            decision.validate()?;
        }
        for case in &self.marker_presence_invalid_payload_mutations {
            case.validate()?;
            require_bool(
                "csharp_state_machine_parity.marker_presence_invalid_payload_mutations.accepted",
                case.accepted,
                false,
            )?;
            require_string(
                "csharp_state_machine_parity.marker_presence_invalid_payload_mutations.error_kind",
                case.error_kind.as_deref().unwrap_or_default(),
                "invalid_payload_token",
            )?;
        }
        if self
            .marker_presence_decisions
            .iter()
            .any(|decision| decision.owner_rule == "NullableTypeSyntax")
            && self.marker_presence_invalid_payload_mutations.is_empty()
        {
            return Err(SyntaxBackendBoundaryError::new(
                "csharp_state_machine_parity.marker_presence_invalid_payload_mutations",
                "nullable marker presence evidence must include malformed payload rejection",
            ));
        }

        require_positive_usize(
            "csharp_state_machine_parity.invalid_mutation_cases",
            self.invalid_mutation_cases.len(),
        )?;
        let mut rejection_kinds = BTreeSet::new();
        for case in &self.invalid_mutation_cases {
            case.validate()?;
            require_bool(
                "csharp_state_machine_parity.invalid_mutation_case.accepted",
                case.accepted,
                false,
            )?;
            let kind = case.error_kind.as_deref().ok_or_else(|| {
                SyntaxBackendBoundaryError::new(
                    "csharp_state_machine_parity.invalid_mutation_case.error_kind",
                    "invalid C# mutation cases must carry an error kind",
                )
            })?;
            rejection_kinds.insert(kind);
        }
        for required in [
            "unexpected_structural_token",
            "missing_required_field",
            "invalid_payload_token",
            "extra_trailing_token",
            "pad_token_in_raw_stream",
            "token_id_out_of_vocab",
        ] {
            if !rejection_kinds.contains(required) {
                return Err(SyntaxBackendBoundaryError::new(
                    "csharp_state_machine_parity.invalid_mutation_cases",
                    format!("missing C# rejection kind {required}"),
                ));
            }
        }

        require_positive_usize(
            "csharp_state_machine_parity.strict_cases",
            self.strict_cases.len(),
        )?;
        let mut strict_success_count = 0usize;
        let mut strict_error_kinds = BTreeSet::new();
        for case in &self.strict_cases {
            case.validate(self.token_layout.vocab_size)?;
            if case.accepted {
                strict_success_count += 1;
            } else if let Some(kind) = case.error_kind.as_deref() {
                strict_error_kinds.insert(kind);
            }
        }
        require_positive_usize(
            "csharp_state_machine_parity.strict_cases.accepted",
            strict_success_count,
        )?;
        for required in [
            "premature_eof",
            "pad_token_in_raw_stream",
            "extra_trailing_token",
        ] {
            if !strict_error_kinds.contains(required) {
                return Err(SyntaxBackendBoundaryError::new(
                    "csharp_state_machine_parity.strict_cases.error_kind",
                    format!("missing strict decode error kind {required}"),
                ));
            }
        }

        require_positive_usize(
            "csharp_state_machine_parity.recovery_cases",
            self.recovery_cases.len(),
        )?;
        let mut recovery_outcomes = BTreeSet::new();
        for case in &self.recovery_cases {
            case.validate(self.token_layout.vocab_size)?;
            recovery_outcomes.insert(case.observed_outcome.as_str());
        }
        for required in ["strict_success", "normalized_trailing_pad", "hard_error"] {
            if !recovery_outcomes.contains(required) {
                return Err(SyntaxBackendBoundaryError::new(
                    "csharp_state_machine_parity.recovery_cases.observed_outcome",
                    format!("missing recovery outcome {required}"),
                ));
            }
        }

        require_bool(
            "csharp_state_machine_parity.state_after_prefix_ready",
            self.state_after_prefix_ready,
            true,
        )?;
        require_bool(
            "csharp_state_machine_parity.admissible_next_raw_tokens_ready",
            self.admissible_next_raw_tokens_ready,
            true,
        )?;
        require_bool(
            "csharp_state_machine_parity.invalid_mutation_rejection_ready",
            self.invalid_mutation_rejection_ready,
            true,
        )?;
        require_bool(
            "csharp_state_machine_parity.strict_recovery_decode_agreement_ready",
            self.strict_recovery_decode_agreement_ready,
            true,
        )?;
        require_bool(
            "csharp_state_machine_parity.consumed_token_accounting_ok",
            self.consumed_token_accounting_ok,
            true,
        )?;
        require_bool(
            "csharp_state_machine_parity.token_layout_invariants_ok",
            self.token_layout_invariants_ok,
            true,
        )?;
        require_bool(
            "csharp_state_machine_parity.target_metadata_sidecar_only",
            self.target_metadata_sidecar_only,
            true,
        )?;
        require_bool(
            "csharp_state_machine_parity.arena_ir_used",
            self.arena_ir_used,
            false,
        )?;
        require_bool(
            "csharp_state_machine_parity.bpe_layer_defines_validity",
            self.bpe_layer_defines_validity,
            false,
        )?;
        require_string(
            "csharp_state_machine_parity.cross_target_parity_claim_scope",
            &self.cross_target_parity_claim_scope,
            CSHARP_STATE_MACHINE_PARITY_CLAIM_SCOPE,
        )?;
        require_string(
            "csharp_state_machine_parity.next_ready_task",
            &self.next_ready_task,
            CSHARP_STATE_MACHINE_PARITY_NEXT_READY_TASK,
        )?;
        require_contains(
            "csharp_state_machine_parity.claim_boundary",
            &self.claim_boundary,
            "syntax-level",
        )?;
        require_contains(
            "csharp_state_machine_parity.claim_boundary",
            &self.claim_boundary,
            "not arena-IR",
        )?;
        require_contains(
            "csharp_state_machine_parity.claim_boundary",
            &self.claim_boundary,
            "not stable-backend",
        )?;
        require_all(
            "csharp_state_machine_parity.no_stable_claims",
            &self.no_stable_claims,
            CSHARP_PARITY_NO_STABLE_CLAIMS,
        )
    }
}

fn csharp_scratch_generation_scope(candidate_grammar_id: &str) -> Option<&'static str> {
    match candidate_grammar_id {
        CSHARP_MINIMAL_CLASS_GRAMMAR_ID => Some(CSHARP_MINIMAL_CLASS_SCRATCH_GENERATION_SCOPE),
        CSHARP_LOCAL_INTEGER_RETURN_LITERAL_GRAMMAR_ID => {
            Some(CSHARP_LOCAL_INTEGER_RETURN_LITERAL_SCRATCH_GENERATION_SCOPE)
        }
        CSHARP_LOCAL_UNARY_INTEGER_RETURN_GRAMMAR_ID => {
            Some(CSHARP_LOCAL_UNARY_INTEGER_RETURN_SCRATCH_GENERATION_SCOPE)
        }
        CSHARP_MEMBER_ACCESS_EXPRESSION_GRAMMAR_ID => {
            Some(CSHARP_MEMBER_ACCESS_EXPRESSION_SCRATCH_GENERATION_SCOPE)
        }
        CSHARP_COMPILATION_UNIT_SOURCE_FILE_GRAMMAR_ID => {
            Some(CSHARP_COMPILATION_UNIT_SOURCE_FILE_SCRATCH_GENERATION_SCOPE)
        }
        CSHARP_COMPILATION_UNIT_LOGGER_SOURCE_FILE_GRAMMAR_ID => {
            Some(CSHARP_COMPILATION_UNIT_LOGGER_SOURCE_FILE_SCRATCH_GENERATION_SCOPE)
        }
        CSHARP_COMPILATION_UNIT_VERSION_ONE_SOURCE_FILE_GRAMMAR_ID => {
            Some(CSHARP_COMPILATION_UNIT_VERSION_ONE_SOURCE_FILE_SCRATCH_GENERATION_SCOPE)
        }
        _ => None,
    }
}

fn csharp_scratch_generation_parser_command(candidate_grammar_id: &str) -> Option<&'static str> {
    match candidate_grammar_id {
        CSHARP_MINIMAL_CLASS_GRAMMAR_ID => Some(
            "python3 scripts/analysis/csharp_syntax_metadata_smoke.py --fixture minimal-class --self-test",
        ),
        CSHARP_LOCAL_INTEGER_RETURN_LITERAL_GRAMMAR_ID => Some(
            "python scripts/analysis/csharp_syntax_metadata_smoke.py --fixture local-integer-return-literal --self-test",
        ),
        CSHARP_LOCAL_UNARY_INTEGER_RETURN_GRAMMAR_ID => Some(
            "python3 scripts/analysis/csharp_syntax_metadata_smoke.py --fixture local-unary-integer-return --self-test",
        ),
        CSHARP_MEMBER_ACCESS_EXPRESSION_GRAMMAR_ID => Some(
            "python3 scripts/analysis/csharp_syntax_metadata_smoke.py --fixture member-access-expression --self-test",
        ),
        CSHARP_COMPILATION_UNIT_SOURCE_FILE_GRAMMAR_ID => Some(
            "python3 scripts/analysis/csharp_syntax_metadata_smoke.py --fixture compilation-unit-source-file --self-test",
        ),
        CSHARP_COMPILATION_UNIT_LOGGER_SOURCE_FILE_GRAMMAR_ID => Some(
            "python3 scripts/analysis/csharp_syntax_metadata_smoke.py --fixture compilation-unit-logger-source-file --self-test",
        ),
        CSHARP_COMPILATION_UNIT_VERSION_ONE_SOURCE_FILE_GRAMMAR_ID => Some(
            "python3 scripts/analysis/csharp_syntax_metadata_smoke.py --fixture compilation-unit-version-one-source-file --self-test",
        ),
        _ => None,
    }
}

fn csharp_scratch_generation_render_blocker(candidate_grammar_id: &str) -> Option<&'static str> {
    let _ = candidate_grammar_id;
    None
}

fn csharp_scratch_generation_renderer_scope(candidate_grammar_id: &str) -> Option<&'static str> {
    match candidate_grammar_id {
        CSHARP_MINIMAL_CLASS_GRAMMAR_ID => Some(CSHARP_MINIMAL_CLASS_RAW_TREE_RENDERER_SCOPE),
        CSHARP_LOCAL_INTEGER_RETURN_LITERAL_GRAMMAR_ID => {
            Some(CSHARP_LOCAL_INTEGER_RETURN_LITERAL_RAW_TREE_RENDERER_SCOPE)
        }
        CSHARP_LOCAL_UNARY_INTEGER_RETURN_GRAMMAR_ID => {
            Some(CSHARP_LOCAL_UNARY_INTEGER_RETURN_RAW_TREE_RENDERER_SCOPE)
        }
        CSHARP_MEMBER_ACCESS_EXPRESSION_GRAMMAR_ID => {
            Some(CSHARP_MEMBER_ACCESS_EXPRESSION_RAW_TREE_RENDERER_SCOPE)
        }
        CSHARP_COMPILATION_UNIT_SOURCE_FILE_GRAMMAR_ID => {
            Some(CSHARP_COMPILATION_UNIT_SOURCE_FILE_RAW_TREE_RENDERER_SCOPE)
        }
        CSHARP_COMPILATION_UNIT_LOGGER_SOURCE_FILE_GRAMMAR_ID => {
            Some(CSHARP_COMPILATION_UNIT_LOGGER_SOURCE_FILE_RAW_TREE_RENDERER_SCOPE)
        }
        CSHARP_COMPILATION_UNIT_VERSION_ONE_SOURCE_FILE_GRAMMAR_ID => {
            Some(CSHARP_COMPILATION_UNIT_VERSION_ONE_SOURCE_FILE_RAW_TREE_RENDERER_SCOPE)
        }
        _ => None,
    }
}

impl CSharpScratchGenerationSummary {
    fn validate(
        &self,
        report: &SyntaxCSharpStateMachineParityReport,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        require_string(
            "csharp_scratch_generation.status",
            &self.status,
            CSHARP_SCRATCH_GENERATION_STATUS,
        )?;
        require_string(
            "csharp_scratch_generation.scope",
            &self.scope,
            csharp_scratch_generation_scope(&report.candidate_grammar_id).ok_or_else(|| {
                SyntaxBackendBoundaryError::new(
                    "csharp_scratch_generation.scope",
                    format!(
                        "{} is not a scratch-generation owner route",
                        report.candidate_grammar_id
                    ),
                )
            })?,
        )?;
        require_string(
            "csharp_scratch_generation.precompiled_artifact_kind",
            &self.precompiled_artifact_kind,
            CSHARP_PRECOMPILED_SYNTAX_TOKENIZER_ARTIFACT_KIND,
        )?;
        require_u32(
            "csharp_scratch_generation.precompiled_artifact_version",
            self.precompiled_artifact_version,
            VERSION,
        )?;
        require_string(
            "csharp_scratch_generation.precompiled_artifact_grammar_id",
            &self.precompiled_artifact_grammar_id,
            &report.candidate_grammar_id,
        )?;
        require_string(
            "csharp_scratch_generation.precompiled_artifact_metadata_path",
            &self.precompiled_artifact_metadata_path,
            &report.metadata_path,
        )?;
        require_string(
            "csharp_scratch_generation.precompiled_artifact_metadata_sha256",
            &self.precompiled_artifact_metadata_sha256,
            &report.metadata_sha256,
        )?;
        require_string(
            "csharp_scratch_generation.precompiled_artifact_structural_model_id",
            &self.precompiled_artifact_structural_model_id,
            &report.structural_model_id,
        )?;
        require_string(
            "csharp_scratch_generation.raw_state_machine_core_boundary_id",
            &self.raw_state_machine_core_boundary_id,
            RAW_SYNTAX_STATE_MACHINE_CORE_BOUNDARY_ID,
        )?;
        require_string(
            "csharp_scratch_generation.raw_state_machine_core_boundary",
            &self.raw_state_machine_core_boundary,
            RAW_SYNTAX_STATE_MACHINE_CORE_BOUNDARY,
        )?;
        require_u32(
            "csharp_scratch_generation.token_layout_vocab_size",
            self.token_layout_vocab_size,
            report.token_layout.vocab_size,
        )?;
        require_u32(
            "csharp_scratch_generation.pad_token_id",
            self.pad_token_id,
            report.token_layout.pad_token,
        )?;
        require_string(
            "csharp_scratch_generation.empty_prefix_root",
            &self.empty_prefix_root,
            CSHARP_EMPTY_PREFIX_ROOT,
        )?;
        require_string(
            "csharp_scratch_generation.initial_state_observed_status",
            &self.initial_state_observed_status,
            "partial",
        )?;
        require_usize(
            "csharp_scratch_generation.initial_state_consumed_raw_tokens",
            self.initial_state_consumed_raw_tokens,
            0,
        )?;
        require_usize(
            "csharp_scratch_generation.initial_state_total_raw_tokens",
            self.initial_state_total_raw_tokens,
            0,
        )?;
        require_bool(
            "csharp_scratch_generation.initial_state_eof_valid",
            self.initial_state_eof_valid,
            false,
        )?;
        require_usize(
            "csharp_scratch_generation.initial_state_stack_depth",
            self.initial_state_stack_depth,
            0,
        )?;
        require_positive_usize(
            "csharp_scratch_generation.finite_admissible_raw_token_count",
            self.finite_admissible_raw_token_count,
        )?;
        require_usize(
            "csharp_scratch_generation.finite_admissible_raw_token_count",
            self.finite_admissible_raw_token_count,
            self.finite_admissible_raw_token_ids.len(),
        )?;
        validate_sorted_unique_tokens(
            "csharp_scratch_generation.finite_admissible_raw_token_ids",
            &self.finite_admissible_raw_token_ids,
            report.token_layout.vocab_size,
        )?;
        require_sha256(
            "csharp_scratch_generation.finite_admissible_raw_token_ids_sha256",
            &self.finite_admissible_raw_token_ids_sha256,
        )?;
        if !self
            .finite_admissible_raw_token_ids
            .contains(&self.selected_raw_token_id)
        {
            return Err(SyntaxBackendBoundaryError::new(
                "csharp_scratch_generation.selected_raw_token_id",
                "selected empty-prefix token must be present in the finite admissible mask",
            ));
        }
        require_bool(
            "csharp_scratch_generation.selected_raw_token_admitted",
            self.selected_raw_token_admitted,
            true,
        )?;
        require_bool(
            "csharp_scratch_generation.selected_transition_accepted",
            self.selected_transition_accepted,
            true,
        )?;
        if self.selected_transition_observed_status != "partial"
            && self.selected_transition_observed_status != "complete"
        {
            return Err(SyntaxBackendBoundaryError::new(
                "csharp_scratch_generation.selected_transition_observed_status",
                "selected transition status must be partial or complete",
            ));
        }
        require_string(
            "csharp_scratch_generation.selected_completion_trace_scope",
            &self.selected_completion_trace_scope,
            &report.selected_completion_trace.scope,
        )?;
        require_usize(
            "csharp_scratch_generation.selected_completion_raw_token_count",
            self.selected_completion_raw_token_count,
            report.raw_token_count,
        )?;
        require_usize(
            "csharp_scratch_generation.selected_completion_transition_count",
            self.selected_completion_transition_count,
            report.raw_token_count,
        )?;
        require_usize(
            "csharp_scratch_generation.selected_completion_admissible_mask_query_count",
            self.selected_completion_admissible_mask_query_count,
            report.raw_token_count,
        )?;
        require_bool(
            "csharp_scratch_generation.selected_completion_every_token_admitted",
            self.selected_completion_every_token_admitted,
            true,
        )?;
        require_string(
            "csharp_scratch_generation.selected_completion_final_status",
            &self.selected_completion_final_status,
            "complete",
        )?;
        require_bool(
            "csharp_scratch_generation.selected_completion_final_eof_valid",
            self.selected_completion_final_eof_valid,
            true,
        )?;
        require_bool(
            "csharp_scratch_generation.strict_incomplete_prefix_rejected",
            self.strict_incomplete_prefix_rejected,
            true,
        )?;
        require_string(
            "csharp_scratch_generation.strict_incomplete_prefix_error_kind",
            &self.strict_incomplete_prefix_error_kind,
            "premature_eof",
        )?;
        require_positive_usize(
            "csharp_scratch_generation.strict_incomplete_prefix_consumed_raw_tokens",
            self.strict_incomplete_prefix_consumed_raw_tokens,
        )?;
        require_usize(
            "csharp_scratch_generation.strict_incomplete_prefix_total_raw_tokens",
            self.strict_incomplete_prefix_total_raw_tokens,
            self.strict_incomplete_prefix_consumed_raw_tokens,
        )?;
        let render_reparse_blocker =
            csharp_scratch_generation_render_blocker(&report.candidate_grammar_id);
        require_bool(
            "csharp_scratch_generation.token_only_decode_render_ready",
            self.token_only_decode_render_ready,
            render_reparse_blocker.is_none(),
        )?;
        require_positive_usize(
            "csharp_scratch_generation.token_only_raw_token_count",
            self.token_only_raw_token_count,
        )?;
        require_sha256(
            "csharp_scratch_generation.token_only_raw_token_ids_sha256",
            &self.token_only_raw_token_ids_sha256,
        )?;
        require_string(
            "csharp_scratch_generation.token_only_root_rule",
            &self.token_only_root_rule,
            CSHARP_EMPTY_PREFIX_ROOT,
        )?;
        require_string(
            "csharp_scratch_generation.token_only_root_node_kind",
            &self.token_only_root_node_kind,
            CSHARP_EMPTY_PREFIX_ROOT,
        )?;
        require_positive_usize(
            "csharp_scratch_generation.token_only_node_start_count",
            self.token_only_node_start_count,
        )?;
        require_positive_usize(
            "csharp_scratch_generation.token_only_payload_value_count",
            self.token_only_payload_value_count,
        )?;
        require_string(
            "csharp_scratch_generation.renderer_id",
            &self.renderer_id,
            CSHARP_RAW_TREE_RENDERER_ID,
        )?;
        require_string(
            "csharp_scratch_generation.renderer_scope",
            &self.renderer_scope,
            csharp_scratch_generation_renderer_scope(&report.candidate_grammar_id).ok_or_else(
                || {
                    SyntaxBackendBoundaryError::new(
                        "csharp_scratch_generation.renderer_scope",
                        format!(
                            "{} is not a scratch-generation owner route",
                            report.candidate_grammar_id
                        ),
                    )
                },
            )?,
        )?;
        if let Some(expected_blocker) = render_reparse_blocker {
            let actual_blocker = self.render_reparse_sidecar_blocker.as_deref().ok_or_else(|| {
                SyntaxBackendBoundaryError::new(
                    "csharp_scratch_generation.render_reparse_sidecar_blocker",
                    "blocked C# scratch-generation route must carry the precise render/reparse blocker",
                )
            })?;
            require_string(
                "csharp_scratch_generation.render_reparse_sidecar_blocker",
                actual_blocker,
                expected_blocker,
            )?;
            require_string(
                "csharp_scratch_generation.rendered_source_sha256",
                &self.rendered_source_sha256,
                "",
            )?;
            require_bool(
                "csharp_scratch_generation.rendered_source_matches_checked_source",
                self.rendered_source_matches_checked_source,
                false,
            )?;
            require_string(
                "csharp_scratch_generation.parser_sidecar_validation",
                &self.parser_sidecar_validation,
                "blocked_pending_raw_tree_renderer",
            )?;
        } else {
            if self.render_reparse_sidecar_blocker.is_some() {
                return Err(SyntaxBackendBoundaryError::new(
                    "csharp_scratch_generation.render_reparse_sidecar_blocker",
                    "render-ready C# scratch-generation route must not carry a render/reparse blocker",
                ));
            }
            require_string(
                "csharp_scratch_generation.rendered_source_sha256",
                &self.rendered_source_sha256,
                &report.source_sha256,
            )?;
            require_bool(
                "csharp_scratch_generation.rendered_source_matches_checked_source",
                self.rendered_source_matches_checked_source,
                true,
            )?;
            require_string(
                "csharp_scratch_generation.parser_sidecar_validation",
                &self.parser_sidecar_validation,
                "roslyn_self_test_required",
            )?;
        }
        require_string(
            "csharp_scratch_generation.parser_sidecar_validation_command",
            &self.parser_sidecar_validation_command,
            csharp_scratch_generation_parser_command(&report.candidate_grammar_id).ok_or_else(
                || {
                    SyntaxBackendBoundaryError::new(
                        "csharp_scratch_generation.parser_sidecar_validation_command",
                        format!(
                            "{} is not a scratch-generation owner route",
                            report.candidate_grammar_id
                        ),
                    )
                },
            )?,
        )?;
        require_bool(
            "csharp_scratch_generation.per_output_parser_metadata_required",
            self.per_output_parser_metadata_required,
            false,
        )?;
        require_bool(
            "csharp_scratch_generation.source_prefix_table_used",
            self.source_prefix_table_used,
            false,
        )?;
        require_bool(
            "csharp_scratch_generation.arena_ir_used",
            self.arena_ir_used,
            false,
        )?;
        require_bool(
            "csharp_scratch_generation.route_handoff_provenance_used",
            self.route_handoff_provenance_used,
            false,
        )?;
        require_bool(
            "csharp_scratch_generation.bpe_validity_authority_used",
            self.bpe_validity_authority_used,
            false,
        )?;
        require_bool(
            "csharp_scratch_generation.stable_backend_claimed",
            self.stable_backend_claimed,
            false,
        )?;
        require_string(
            "csharp_scratch_generation.next_ready_task",
            &self.next_ready_task,
            CSHARP_SCRATCH_GENERATION_NEXT_READY_TASK,
        )?;
        require_contains(
            "csharp_scratch_generation.claim_boundary",
            &self.claim_boundary,
            "syntax-level C#",
        )?;
        require_contains(
            "csharp_scratch_generation.claim_boundary",
            &self.claim_boundary,
            "empty CompilationUnitSyntax",
        )?;
        require_contains(
            "csharp_scratch_generation.claim_boundary",
            &self.claim_boundary,
            "not arena-IR",
        )?;
        require_contains(
            "csharp_scratch_generation.claim_boundary",
            &self.claim_boundary,
            "not stable-backend",
        )
    }
}

impl SyntaxSelectedCompletionTraceReport {
    fn validate(
        &self,
        raw_token_count: usize,
        vocab_size: u32,
        complete_stream: &SyntaxReplayCaseResult,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        require_string(
            "selected_completion_trace.scope",
            &self.scope,
            CSHARP_SELECTED_COMPLETION_TRACE_SCOPE,
        )?;
        require_string(
            "selected_completion_trace.selection_loop_source",
            &self.selection_loop_source,
            SELECTED_COMPLETION_TRACE_SOURCE,
        )?;
        require_usize(
            "selected_completion_trace.raw_token_count",
            self.raw_token_count,
            raw_token_count,
        )?;
        require_usize(
            "selected_completion_trace.transition_count",
            self.transition_count,
            raw_token_count,
        )?;
        require_usize(
            "selected_completion_trace.admissible_mask_query_count",
            self.admissible_mask_query_count,
            raw_token_count,
        )?;
        require_bool(
            "selected_completion_trace.finite_admissible_raw_token_mask_available",
            self.finite_admissible_raw_token_mask_available,
            true,
        )?;
        require_bool(
            "selected_completion_trace.every_selected_raw_token_admitted",
            self.every_selected_raw_token_admitted,
            true,
        )?;
        require_usize(
            "selected_completion_trace.selected_transition_accepted_count",
            self.selected_transition_accepted_count,
            raw_token_count,
        )?;
        if self.first_selected_raw_token_id >= vocab_size {
            return Err(SyntaxBackendBoundaryError::new(
                "selected_completion_trace.first_selected_raw_token_id",
                "first selected raw token must remain below VOCAB_SIZE",
            ));
        }
        if self.final_selected_raw_token_id >= vocab_size {
            return Err(SyntaxBackendBoundaryError::new(
                "selected_completion_trace.final_selected_raw_token_id",
                "final selected raw token must remain below VOCAB_SIZE",
            ));
        }
        require_string(
            "selected_completion_trace.final_state_observed_status",
            &self.final_state_observed_status,
            "complete",
        )?;
        require_usize(
            "selected_completion_trace.final_state_consumed_raw_tokens",
            self.final_state_consumed_raw_tokens,
            raw_token_count,
        )?;
        require_bool(
            "selected_completion_trace.final_state_eof_valid",
            self.final_state_eof_valid,
            true,
        )?;
        require_usize(
            "selected_completion_trace.final_state_stack_depth",
            self.final_state_stack_depth,
            0,
        )?;
        require_string(
            "selected_completion_trace.strict_decode_agreement_case_id",
            &self.strict_decode_agreement_case_id,
            &complete_stream.case_id,
        )?;
        require_string(
            "selected_completion_trace.strict_decode_agreement_observed_status",
            &self.strict_decode_agreement_observed_status,
            &complete_stream.observed_status,
        )?;
        require_bool(
            "selected_completion_trace.strict_decode_agreement_eof_valid",
            self.strict_decode_agreement_eof_valid,
            complete_stream.eof_valid,
        )?;
        require_bool(
            "selected_completion_trace.strict_decode_agreement_eof_valid",
            self.strict_decode_agreement_eof_valid,
            true,
        )?;
        require_positive_usize(
            "selected_completion_trace.step_trace_sample",
            self.step_trace_sample.len(),
        )?;
        if self.step_trace_sample.len() > 6 {
            return Err(SyntaxBackendBoundaryError::new(
                "selected_completion_trace.step_trace_sample",
                "selected completion trace keeps at most six diagnostic samples",
            ));
        }
        let mut seen_steps = BTreeSet::new();
        for sample in &self.step_trace_sample {
            sample.validate(raw_token_count, vocab_size)?;
            if !seen_steps.insert(sample.step_index) {
                return Err(SyntaxBackendBoundaryError::new(
                    "selected_completion_trace.step_trace_sample",
                    format!(
                        "duplicate selected completion trace sample step {}",
                        sample.step_index
                    ),
                ));
            }
        }
        Ok(())
    }
}

impl SyntaxSelectedCompletionStepSample {
    fn validate(
        &self,
        raw_token_count: usize,
        vocab_size: u32,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        if self.step_index >= raw_token_count {
            return Err(SyntaxBackendBoundaryError::new(
                "selected_completion_trace.step_trace_sample.step_index",
                "sample step must be inside the raw token stream",
            ));
        }
        require_usize(
            "selected_completion_trace.step_trace_sample.prefix_raw_token_count",
            self.prefix_raw_token_count,
            self.step_index,
        )?;
        if self.selected_raw_token_id >= vocab_size {
            return Err(SyntaxBackendBoundaryError::new(
                "selected_completion_trace.step_trace_sample.selected_raw_token_id",
                "sample selected raw token must remain below VOCAB_SIZE",
            ));
        }
        require_positive_usize(
            "selected_completion_trace.step_trace_sample.finite_admissible_raw_token_count",
            self.finite_admissible_raw_token_count,
        )?;
        require_bool(
            "selected_completion_trace.step_trace_sample.selected_raw_token_admitted",
            self.selected_raw_token_admitted,
            true,
        )?;
        require_bool(
            "selected_completion_trace.step_trace_sample.selected_transition_accepted",
            self.selected_transition_accepted,
            true,
        )?;
        if self.transition_observed_status != "partial"
            && self.transition_observed_status != "complete"
        {
            return Err(SyntaxBackendBoundaryError::new(
                "selected_completion_trace.step_trace_sample.transition_observed_status",
                "selected completion transition status must be partial or complete",
            ));
        }
        require_usize(
            "selected_completion_trace.step_trace_sample.transition_consumed_raw_tokens",
            self.transition_consumed_raw_tokens,
            self.step_index + 1,
        )
    }
}

impl CSharpMarkerPresenceDecision {
    fn validate(&self) -> Result<(), SyntaxBackendBoundaryError> {
        require_string(
            "csharp_state_machine_parity.marker_presence.owner_rule",
            &self.owner_rule,
            "NullableTypeSyntax",
        )?;
        require_string(
            "csharp_state_machine_parity.marker_presence.owner_field",
            &self.owner_field,
            "QuestionToken",
        )?;
        require_string(
            "csharp_state_machine_parity.marker_presence.selected_node_kind",
            &self.selected_node_kind,
            "nullable_marker",
        )?;
        require_string(
            "csharp_state_machine_parity.marker_presence.payload_kind",
            &self.payload_kind,
            "string",
        )?;
        if self.first_payload_raw_token_offset != self.terminal_raw_token_offset + 1 {
            return Err(SyntaxBackendBoundaryError::new(
                "csharp_state_machine_parity.marker_presence.first_payload_raw_token_offset",
                "marker payload byte must immediately follow the terminal token",
            ));
        }
        if self.payload_end_raw_token_offset <= self.first_payload_raw_token_offset {
            return Err(SyntaxBackendBoundaryError::new(
                "csharp_state_machine_parity.marker_presence.payload_end_raw_token_offset",
                "marker payload end must follow at least one payload byte",
            ));
        }
        require_bool(
            "csharp_state_machine_parity.marker_presence.terminal_event_decoded",
            self.terminal_event_decoded,
            true,
        )?;
        require_bool(
            "csharp_state_machine_parity.marker_presence.string_payload_byte_decoded",
            self.string_payload_byte_decoded,
            true,
        )?;
        require_bool(
            "csharp_state_machine_parity.marker_presence.payload_end_decoded",
            self.payload_end_decoded,
            true,
        )?;
        require_bool(
            "csharp_state_machine_parity.marker_presence.state_after_terminal_admits_utf8_payload_range",
            self.state_after_terminal_admits_utf8_payload_range,
            true,
        )?;
        require_bool(
            "csharp_state_machine_parity.marker_presence.state_after_payload_byte_admits_payload_end",
            self.state_after_payload_byte_admits_payload_end,
            true,
        )?;
        if !CSHARP_NULLABLE_MARKER_GENERATED_TEST_MARKERS
            .contains(&self.generated_marker_test_marker.as_str())
        {
            return Err(SyntaxBackendBoundaryError::new(
                "csharp_state_machine_parity.marker_presence.generated_marker_test_marker",
                format!(
                    "nullable marker generated-codec test marker is not recognized: {}",
                    self.generated_marker_test_marker
                ),
            ));
        }
        require_bool(
            "csharp_state_machine_parity.marker_presence.generated_codec_marker_evidence_preserved",
            self.generated_codec_marker_evidence_preserved,
            true,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxGeneratedCodecStrictDecodeCase {
    pub case_id: String,
    pub input_kind: String,
    pub state_api_case_id: String,
    pub expected_status: String,
    pub observed_status: String,
    pub accepted: bool,
    pub consumed_raw_tokens: usize,
    pub total_raw_tokens: usize,
    pub eof_valid: bool,
    pub stack_depth: usize,
    pub active_rule: Option<String>,
    pub active_field: Option<String>,
    pub error_kind: Option<String>,
    pub failing_token: Option<u32>,
    pub diagnostic: String,
}

impl SyntaxGeneratedCodecStrictDecodeCase {
    fn validate(&self, vocab_size: u32) -> Result<(), SyntaxBackendBoundaryError> {
        require_non_empty("strict_decode_case.case_id", &self.case_id)?;
        require_non_empty("strict_decode_case.input_kind", &self.input_kind)?;
        require_non_empty(
            "strict_decode_case.state_api_case_id",
            &self.state_api_case_id,
        )?;
        require_non_empty("strict_decode_case.expected_status", &self.expected_status)?;
        require_non_empty("strict_decode_case.observed_status", &self.observed_status)?;
        require_non_empty("strict_decode_case.diagnostic", &self.diagnostic)?;
        if let Some(token) = self.failing_token
            && token >= vocab_size
        {
            return Err(SyntaxBackendBoundaryError::new(
                "strict_decode_case.failing_token",
                "failing token must remain below VOCAB_SIZE",
            ));
        }
        if self.consumed_raw_tokens > self.total_raw_tokens {
            return Err(SyntaxBackendBoundaryError::new(
                "strict_decode_case.consumed_raw_tokens",
                "consumed-token accounting cannot exceed input length",
            ));
        }
        match (self.accepted, self.error_kind.as_ref()) {
            (true, None) => Ok(()),
            (false, Some(error_kind)) if !error_kind.is_empty() => Ok(()),
            (true, Some(_)) => Err(SyntaxBackendBoundaryError::new(
                "strict_decode_case.error_kind",
                "accepted strict decode case must not carry an error kind",
            )),
            (false, None) => Err(SyntaxBackendBoundaryError::new(
                "strict_decode_case.error_kind",
                "rejected strict decode case must carry an error kind",
            )),
            (false, Some(_)) => Err(SyntaxBackendBoundaryError::new(
                "strict_decode_case.error_kind",
                "rejected strict decode case error kind must be non-empty",
            )),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxGeneratedCodecRecoveryDecodeCase {
    pub case_id: String,
    pub input_kind: String,
    pub state_api_case_id: String,
    pub expected_outcome: String,
    pub observed_outcome: String,
    pub accepted: bool,
    pub ignored_trailing_tokens: usize,
    pub consumed_raw_tokens: usize,
    pub total_raw_tokens: usize,
    pub normalized_raw_token_count: usize,
    pub eof_valid: bool,
    pub error_kind: Option<String>,
    pub failing_token: Option<u32>,
    pub diagnostic: String,
}

impl SyntaxGeneratedCodecRecoveryDecodeCase {
    fn validate(&self, vocab_size: u32) -> Result<(), SyntaxBackendBoundaryError> {
        require_non_empty("recovery_decode_case.case_id", &self.case_id)?;
        require_non_empty("recovery_decode_case.input_kind", &self.input_kind)?;
        require_non_empty(
            "recovery_decode_case.state_api_case_id",
            &self.state_api_case_id,
        )?;
        require_non_empty(
            "recovery_decode_case.expected_outcome",
            &self.expected_outcome,
        )?;
        require_non_empty(
            "recovery_decode_case.observed_outcome",
            &self.observed_outcome,
        )?;
        require_non_empty("recovery_decode_case.diagnostic", &self.diagnostic)?;
        if self.expected_outcome != self.observed_outcome {
            return Err(SyntaxBackendBoundaryError::new(
                "recovery_decode_case.observed_outcome",
                format!(
                    "expected {}, got {}",
                    self.expected_outcome, self.observed_outcome
                ),
            ));
        }
        if let Some(token) = self.failing_token
            && token >= vocab_size
        {
            return Err(SyntaxBackendBoundaryError::new(
                "recovery_decode_case.failing_token",
                "failing token must remain below VOCAB_SIZE",
            ));
        }
        if self.consumed_raw_tokens > self.total_raw_tokens {
            return Err(SyntaxBackendBoundaryError::new(
                "recovery_decode_case.consumed_raw_tokens",
                "consumed-token accounting cannot exceed input length",
            ));
        }
        match self.observed_outcome.as_str() {
            "strict_success" => {
                require_bool("recovery_decode_case.accepted", self.accepted, true)?;
                require_usize(
                    "recovery_decode_case.ignored_trailing_tokens",
                    self.ignored_trailing_tokens,
                    0,
                )?;
                if self.error_kind.is_some() {
                    return Err(SyntaxBackendBoundaryError::new(
                        "recovery_decode_case.error_kind",
                        "strict-success recovery case must not carry an error kind",
                    ));
                }
                Ok(())
            }
            "normalized_trailing_pad" => {
                require_bool("recovery_decode_case.accepted", self.accepted, true)?;
                require_positive_usize(
                    "recovery_decode_case.ignored_trailing_tokens",
                    self.ignored_trailing_tokens,
                )?;
                if self.error_kind.is_some() {
                    return Err(SyntaxBackendBoundaryError::new(
                        "recovery_decode_case.error_kind",
                        "normalized trailing PAD recovery case must not carry an error kind",
                    ));
                }
                Ok(())
            }
            "hard_error" => {
                require_bool("recovery_decode_case.accepted", self.accepted, false)?;
                let error_kind = self.error_kind.as_deref().ok_or_else(|| {
                    SyntaxBackendBoundaryError::new(
                        "recovery_decode_case.error_kind",
                        "hard-error recovery case must carry an error kind",
                    )
                })?;
                require_non_empty("recovery_decode_case.error_kind", error_kind)
            }
            other => Err(SyntaxBackendBoundaryError::new(
                "recovery_decode_case.observed_outcome",
                format!("unsupported recovery outcome {other}"),
            )),
        }
    }
}

pub fn build_go_source_instance_replay_report(
    projection_report: &SyntaxGoSourceInstanceProjectionReport,
    raw_tokens: &SourceInstanceProjectionRawTokenIds,
    generated_report: &SourceInstanceGeneratedCodecReport,
) -> Result<SyntaxStateMachineReplayReport, SyntaxBackendBoundaryError> {
    projection_report.validate()?;
    raw_tokens.validate()?;
    generated_report.validate()?;
    require_string(
        "replay.generated_codec_report.route",
        &generated_report.route,
        GENERATED_CODEC_REPORT_ROUTE,
    )?;
    let sidecar = source_instance_structural_model_sidecar(projection_report)?;
    validate_source_instance_sidecar(&sidecar, projection_report)?;
    let core = CompiledSyntaxModel::from_sidecar_and_layout(&sidecar, &raw_tokens.token_layout)?;
    let canonical_token_ids = core
        .elide_forced_raw_tokens(&raw_tokens.raw_token_ids)
        .map_err(|error| {
            SyntaxBackendBoundaryError::new(
                "replay.generated_codec_report.tct_tokens",
                format!(
                    "failed to elide mask-forced tokens for the canonical stream cross-check: {error:?}"
                ),
            )
        })?;
    if generated_report.tct_tokens != canonical_token_ids {
        return Err(SyntaxBackendBoundaryError::new(
            "replay.generated_codec_report.tct_tokens",
            "generated-codec report tokens must match the canonical elided stream of the raw token artifact",
        ));
    }
    let roles = core.token_roles();
    let base_stream = RawReplayInput {
        raw_token_ids: raw_tokens.raw_token_ids.clone(),
        raw_token_segments: raw_tokens.raw_token_segments.clone(),
    };
    let segment_events_sha256 = json_sha256(&base_stream.raw_token_segments).map_err(|error| {
        SyntaxBackendBoundaryError::new(
            "replay.segment_events_sha256",
            format!("failed to hash replay segment events: {error}"),
        )
    })?;

    let complete_state = core
        .strict_decode_complete(&base_stream.raw_token_ids)
        .map_err(|error| {
            SyntaxBackendBoundaryError::new(
                "replay.complete_stream",
                format!(
                    "valid source-instance stream should replay: {}",
                    error.message
                ),
            )
        })?;
    let complete_stream = SyntaxReplayCaseResult::accepted(
        "complete_real_source_stream",
        "go_source_instance_raw_tokens",
        "complete",
        complete_state,
    );

    let prefix_cases = valid_prefix_cases(&base_stream, &core)?;
    let promotion_blockers = missing_tokenized_alternative_identity_blockers(&sidecar, roles);
    let invalid_mutation_cases = invalid_mutation_cases(&base_stream, &core)?;

    Ok(SyntaxStateMachineReplayReport {
        artifact_kind: ARTIFACT_KIND.to_string(),
        version: VERSION,
        tool: TOOL.to_string(),
        status: STATUS.to_string(),
        target: TARGET.to_string(),
        route: ROUTE.to_string(),
        candidate_grammar_id: projection_report.candidate_grammar_id.clone(),
        source_path: projection_report.source.path.clone(),
        source_sha256: projection_report.source.sha256.clone(),
        metadata_path: projection_report.metadata.path.clone(),
        metadata_sha256: projection_report.metadata.sha256.clone(),
        parser_tool: projection_report.parser.metadata_tool.clone(),
        parser_version: projection_report.parser.metadata_version.clone(),
        structural_model_id: raw_tokens.structural_model_id.clone(),
        token_layout: raw_tokens.token_layout.clone(),
        generated_codec_report_artifact_kind: generated_report.artifact_kind.clone(),
        generated_codec_report_path: generated_report.report_path.clone(),
        raw_token_count: raw_tokens.raw_token_count,
        raw_token_ids_sha256: raw_tokens.raw_token_ids_sha256.clone(),
        segment_count: raw_tokens.raw_token_segments.len(),
        segment_events_sha256,
        complete_stream,
        accepted_prefix_count: prefix_cases.len(),
        prefix_cases,
        prefix_inspection_state_api_ready: true,
        prefix_inspection_state_api_source: REPLAY_PREFIX_INSPECTION_STATE_API_SOURCE.to_string(),
        rejected_mutation_count: invalid_mutation_cases.len(),
        invalid_mutation_cases,
        consumed_token_accounting_ok: true,
        raw_token_self_sufficient_for_generated_codec: true,
        generated_codec_promotion_blockers: promotion_blockers,
        next_missing_workstream_blocker:
            "generated-codec state API is now consumed by replay prefix inspection; next wire strict decode, recovery decode, and Go exact/BPE mask consumers to the same structural/payload state contract"
                .to_string(),
        validation_commands: vec![
            "cargo test --test syntax_state_machine_replay_test".to_string(),
            "cargo test --test syntax_go_source_instance_projection_test".to_string(),
            "cargo test --test syntax_structural_model_conformance_test".to_string(),
        ],
        claim_boundary:
            "report-only replay evidence for checked Go source-instance raw syntax tokens. Accepted prefix inspection is derived from generated-codec state API state cases over GrammarIR -> StructuralModel -> token layout replay semantics; strict/recovery decode and mask consumers remain unwired. This is not arena-IR evidence and not stable-backend promotion."
                .to_string(),
        no_stable_claims: NO_STABLE_CLAIMS.iter().map(|claim| (*claim).to_string()).collect(),
    })
}

fn selected_completion_sample_steps(raw_token_count: usize) -> BTreeSet<usize> {
    let mut steps = BTreeSet::new();
    for step in 0..raw_token_count.min(3) {
        steps.insert(step);
    }
    for step in raw_token_count.saturating_sub(3)..raw_token_count {
        steps.insert(step);
    }
    steps
}

fn selected_completion_trace_report(
    core: &CompiledSyntaxModel,
    raw_token_ids: &[u32],
    complete_stream: &SyntaxReplayCaseResult,
) -> Result<SyntaxSelectedCompletionTraceReport, SyntaxBackendBoundaryError> {
    let sample_steps = selected_completion_sample_steps(raw_token_ids.len());
    let trace = core
        .selected_completion_trace(raw_token_ids, &sample_steps)
        .map_err(|error| {
            SyntaxBackendBoundaryError::new(
                "selected_completion_trace.selected_completion_trace",
                format!(
                    "checked raw token stream should be selectable through admitted state-machine transitions: {}",
                    error.message
                ),
            )
        })?;
    let selected_completion = RawScratchSelectedCompletionSummary::from_trace(
        raw_token_ids,
        &trace,
        "selected_completion_trace.raw_state_machine_summary",
    )?;

    let report = SyntaxSelectedCompletionTraceReport {
        scope: CSHARP_SELECTED_COMPLETION_TRACE_SCOPE.to_string(),
        selection_loop_source: SELECTED_COMPLETION_TRACE_SOURCE.to_string(),
        raw_token_count: selected_completion.raw_token_count,
        transition_count: selected_completion.transition_count,
        admissible_mask_query_count: selected_completion.admissible_mask_query_count,
        finite_admissible_raw_token_mask_available: selected_completion
            .finite_admissible_raw_token_mask_available,
        every_selected_raw_token_admitted: selected_completion.every_selected_raw_token_admitted,
        selected_transition_accepted_count: selected_completion.selected_transition_accepted_count,
        first_selected_raw_token_id: selected_completion.first_selected_raw_token_id,
        final_selected_raw_token_id: selected_completion.final_selected_raw_token_id,
        final_state_observed_status: selected_completion.final_state.observed_status,
        final_state_consumed_raw_tokens: selected_completion.final_state.consumed_raw_tokens,
        final_state_eof_valid: selected_completion.final_state.eof_valid,
        final_state_stack_depth: selected_completion.final_state.stack_depth,
        strict_decode_agreement_case_id: complete_stream.case_id.clone(),
        strict_decode_agreement_observed_status: complete_stream.observed_status.clone(),
        strict_decode_agreement_eof_valid: complete_stream.eof_valid,
        step_trace_sample: selected_completion_step_samples(selected_completion.step_samples),
    };
    report.validate(
        raw_token_ids.len(),
        core.token_roles().vocab_size,
        complete_stream,
    )?;
    Ok(report)
}

fn selected_completion_step_samples(
    samples: Vec<RawScratchSelectedCompletionStepSummary>,
) -> Vec<SyntaxSelectedCompletionStepSample> {
    samples
        .into_iter()
        .map(|sample| SyntaxSelectedCompletionStepSample {
            step_index: sample.step_index,
            prefix_raw_token_count: sample.prefix_raw_token_count,
            selected_raw_token_id: sample.selected_raw_token_id,
            finite_admissible_raw_token_count: sample.finite_admissible_raw_token_count,
            selected_raw_token_admitted: sample.selected_raw_token_admitted,
            selected_transition_accepted: sample.selected_transition_accepted,
            transition_observed_status: sample.transition_state.observed_status,
            transition_consumed_raw_tokens: sample.transition_state.consumed_raw_tokens,
            transition_eof_valid: sample.transition_state.eof_valid,
            transition_stack_depth: sample.transition_state.stack_depth,
            transition_active_rule: sample.transition_state.active_rule,
            transition_active_field: sample.transition_state.active_field,
        })
        .collect()
}

pub fn build_go_source_instance_generated_codec_state_api_report(
    projection_report: &SyntaxGoSourceInstanceProjectionReport,
    raw_tokens: &SourceInstanceProjectionRawTokenIds,
    generated_report: &SourceInstanceGeneratedCodecReport,
) -> Result<SyntaxGeneratedCodecStateApiReport, SyntaxBackendBoundaryError> {
    projection_report.validate()?;
    raw_tokens.validate()?;
    generated_report.validate()?;
    require_string(
        "generated_codec_state_api.generated_codec_report.route",
        &generated_report.route,
        GENERATED_CODEC_REPORT_ROUTE,
    )?;
    let sidecar = source_instance_structural_model_sidecar(projection_report)?;
    validate_source_instance_sidecar(&sidecar, projection_report)?;
    let core = CompiledSyntaxModel::from_sidecar_and_layout(&sidecar, &raw_tokens.token_layout)?;
    let canonical_token_ids = core
        .elide_forced_raw_tokens(&raw_tokens.raw_token_ids)
        .map_err(|error| {
            SyntaxBackendBoundaryError::new(
                "generated_codec_state_api.generated_codec_report.tct_tokens",
                format!(
                    "failed to elide mask-forced tokens for the canonical stream cross-check: {error:?}"
                ),
            )
        })?;
    if generated_report.tct_tokens != canonical_token_ids {
        return Err(SyntaxBackendBoundaryError::new(
            "generated_codec_state_api.generated_codec_report.tct_tokens",
            "generated-codec report tokens must match the canonical elided stream of the raw token artifact",
        ));
    }
    let roles = core.token_roles();
    let base_stream = RawReplayInput {
        raw_token_ids: raw_tokens.raw_token_ids.clone(),
        raw_token_segments: raw_tokens.raw_token_segments.clone(),
    };

    let root_prefix_indices = vec![0];
    let first_field_start_index = segment_index(&base_stream, "first field_start", |segment| {
        segment.event_kind == "field_start"
    })?;
    let first_field_prefix_indices = (0..=first_field_start_index).collect::<Vec<_>>();
    let decls_field_start_index =
        segment_index(&base_stream, "File.Decls field_start", |segment| {
            segment.event_kind == "field_start"
                && segment.owner_rule.as_deref() == Some("File")
                && segment.owner_field.as_deref() == Some("Decls")
        })?;
    let decls_prefix_indices = (0..=decls_field_start_index).collect::<Vec<_>>();
    let first_payload_segment_index = first_variable_payload_segment_index(&base_stream)?;
    let complete_indices = (0..base_stream.raw_token_segments.len()).collect::<Vec<_>>();

    let state_cases = vec![
        generated_codec_state_case(
            "empty_prefix",
            "go_source_instance_empty_raw_prefix",
            &base_stream,
            &[],
            &core,
        )?,
        generated_codec_state_case(
            "after_root_node_start",
            "go_source_instance_segment_aligned_raw_prefix",
            &base_stream,
            &root_prefix_indices,
            &core,
        )?,
        generated_codec_state_case(
            "after_first_field_start",
            "go_source_instance_segment_aligned_raw_prefix",
            &base_stream,
            &first_field_prefix_indices,
            &core,
        )?,
        generated_codec_state_case(
            "after_file_decls_field_start",
            "go_source_instance_segment_aligned_raw_prefix",
            &base_stream,
            &decls_prefix_indices,
            &core,
        )?,
        generated_codec_payload_range_state_case(
            "inside_first_terminal_payload_after_one_byte",
            "go_source_instance_partial_terminal_payload_prefix",
            &base_stream,
            first_payload_segment_index,
            1,
            &core,
        )?,
        generated_codec_state_case(
            "complete_stream",
            "go_source_instance_complete_raw_stream",
            &base_stream,
            &complete_indices,
            &core,
        )?,
    ];

    let actual_decls_node_start_index =
        decls_field_start_index.checked_add(1).ok_or_else(|| {
            SyntaxBackendBoundaryError::new(
                "generated_codec_state_api.transition",
                "File.Decls transition index overflowed",
            )
        })?;
    let actual_decls_token =
        next_single_segment_token(&base_stream, actual_decls_node_start_index)?;
    let after_actual_decls_prefix_indices = (0..=actual_decls_node_start_index).collect::<Vec<_>>();
    let actual_decls_next_index =
        actual_decls_node_start_index
            .checked_add(1)
            .ok_or_else(|| {
                SyntaxBackendBoundaryError::new(
                    "generated_codec_state_api.transition",
                    "File.Decls chained transition index overflowed",
                )
            })?;
    let actual_decls_next_token = next_single_segment_token(&base_stream, actual_decls_next_index)?;
    let rejected_decls_candidate = rejected_structural_candidate_for_prefix(
        "file_decls_generic_node_start_is_rejected",
        &base_stream,
        &decls_prefix_indices,
        roles.node_start,
        &core,
    )?;
    let transition_cases = vec![
        generated_codec_transition_case(
            "file_decls_actual_alternative_node_start_is_accepted",
            "after_file_decls_field_start",
            GeneratedCodecTransitionProbe {
                base_stream: &base_stream,
                prefix_indices: &decls_prefix_indices,
                next_segment_index: actual_decls_node_start_index,
                candidate_raw_token_id: actual_decls_token,
                expected_accepted: true,
            },
            &core,
        )?,
        generated_codec_transition_case(
            "file_decls_after_actual_alternative_first_field_start_is_accepted",
            "file_decls_actual_alternative_node_start_is_accepted",
            GeneratedCodecTransitionProbe {
                base_stream: &base_stream,
                prefix_indices: &after_actual_decls_prefix_indices,
                next_segment_index: actual_decls_next_index,
                candidate_raw_token_id: actual_decls_next_token,
                expected_accepted: true,
            },
            &core,
        )?,
        generated_codec_transition_case(
            "file_decls_generic_node_start_is_rejected",
            "after_file_decls_field_start",
            GeneratedCodecTransitionProbe {
                base_stream: &base_stream,
                prefix_indices: &decls_prefix_indices,
                next_segment_index: actual_decls_node_start_index,
                candidate_raw_token_id: rejected_decls_candidate,
                expected_accepted: false,
            },
            &core,
        )?,
        generated_codec_transition_case(
            "file_decls_after_actual_alternative_pad_token_is_rejected",
            "file_decls_actual_alternative_node_start_is_accepted",
            GeneratedCodecTransitionProbe {
                base_stream: &base_stream,
                prefix_indices: &after_actual_decls_prefix_indices,
                next_segment_index: actual_decls_next_index,
                candidate_raw_token_id: roles.pad_token,
                expected_accepted: false,
            },
            &core,
        )?,
    ];

    Ok(SyntaxGeneratedCodecStateApiReport {
        artifact_kind: GENERATED_CODEC_STATE_API_ARTIFACT_KIND.to_string(),
        version: VERSION,
        tool: TOOL.to_string(),
        status: GENERATED_CODEC_STATE_API_STATUS.to_string(),
        target: TARGET.to_string(),
        route: ROUTE.to_string(),
        api_scope: GENERATED_CODEC_STATE_API_SCOPE.to_string(),
        candidate_grammar_id: projection_report.candidate_grammar_id.clone(),
        source_path: projection_report.source.path.clone(),
        source_sha256: projection_report.source.sha256.clone(),
        metadata_path: projection_report.metadata.path.clone(),
        metadata_sha256: projection_report.metadata.sha256.clone(),
        structural_model_id: raw_tokens.structural_model_id.clone(),
        token_layout: raw_tokens.token_layout.clone(),
        generated_codec_report_artifact_kind: generated_report.artifact_kind.clone(),
        generated_codec_report_path: generated_report.report_path.clone(),
        raw_token_count: raw_tokens.raw_token_count,
        raw_token_ids_sha256: raw_tokens.raw_token_ids_sha256.clone(),
        state_cases,
        transition_cases,
        prefix_state_api_ready: true,
        strict_recovery_decode_wiring_ready: true,
        payload_range_state_api_ready: true,
        bpe_layer_defines_validity: false,
        next_missing_workstream_blocker: GENERATED_CODEC_STATE_API_NEXT_BLOCKER.to_string(),
        validation_commands: vec![
            "cargo test --test syntax_generated_codec_gate_test".to_string(),
            "cargo test --test syntax_backend_boundary_test".to_string(),
            "git diff --check".to_string(),
        ],
        claim_boundary:
            "syntax-level generated-codec prefix state API evidence for segment-aligned checked Go source-instance raw tokens. The API derives admissible structural raw tokens, strict/recovery decode cases, and token-layout payload ranges from GrammarIR -> StructuralModel -> token layout replay semantics; payload lexical exactness remains with strict generated-codec validation. C# parity, source validity-by-construction, arena-IR evidence, BPE validity, generated hot paths, paper-facing claims, and stable-backend promotion remain out of scope. This is not arena-IR evidence and not stable-backend promotion."
                .to_string(),
        no_stable_claims: STATE_API_NO_STABLE_CLAIMS
            .iter()
            .map(|claim| (*claim).to_string())
            .collect(),
    })
}

pub fn build_go_source_instance_strict_recovery_decode_report(
    projection_report: &SyntaxGoSourceInstanceProjectionReport,
    raw_tokens: &SourceInstanceProjectionRawTokenIds,
    generated_report: &SourceInstanceGeneratedCodecReport,
    state_api_report: &SyntaxGeneratedCodecStateApiReport,
) -> Result<SyntaxGeneratedCodecStrictRecoveryDecodeReport, SyntaxBackendBoundaryError> {
    projection_report.validate()?;
    raw_tokens.validate()?;
    generated_report.validate()?;
    state_api_report.validate()?;
    require_string(
        "strict_recovery_decode.generated_codec_report.route",
        &generated_report.route,
        GENERATED_CODEC_REPORT_ROUTE,
    )?;
    validate_state_api_matches_inputs(state_api_report, projection_report, raw_tokens)?;

    let sidecar = source_instance_structural_model_sidecar(projection_report)?;
    validate_source_instance_sidecar(&sidecar, projection_report)?;
    let core = CompiledSyntaxModel::from_sidecar_and_layout(&sidecar, &raw_tokens.token_layout)?;
    let canonical_token_ids = core
        .elide_forced_raw_tokens(&raw_tokens.raw_token_ids)
        .map_err(|error| {
            SyntaxBackendBoundaryError::new(
                "strict_recovery_decode.generated_codec_report.tct_tokens",
                format!(
                    "failed to elide mask-forced tokens for the canonical stream cross-check: {error:?}"
                ),
            )
        })?;
    if generated_report.tct_tokens != canonical_token_ids {
        return Err(SyntaxBackendBoundaryError::new(
            "strict_recovery_decode.generated_codec_report.tct_tokens",
            "generated-codec report tokens must match the canonical elided stream of the raw token artifact",
        ));
    }
    let roles = core.token_roles();
    let base_stream = RawReplayInput {
        raw_token_ids: raw_tokens.raw_token_ids.clone(),
        raw_token_segments: raw_tokens.raw_token_segments.clone(),
    };

    let complete_state = required_state_case(state_api_report, "complete_stream")?;
    require_bool(
        "strict_recovery_decode.state_api.complete_stream.eof_valid",
        complete_state.eof_valid,
        true,
    )?;
    require_usize(
        "strict_recovery_decode.state_api.complete_stream.consumed_raw_tokens",
        complete_state.consumed_raw_tokens,
        raw_tokens.raw_token_count,
    )?;
    let decls_state = required_state_case(state_api_report, "after_file_decls_field_start")?;
    require_bool(
        "strict_recovery_decode.state_api.after_file_decls_field_start.eof_valid",
        decls_state.eof_valid,
        false,
    )?;
    require_string(
        "strict_recovery_decode.state_api.after_file_decls_field_start.observed_status",
        &decls_state.observed_status,
        "partial",
    )?;

    let decls_prefix_stream = stream_from_state_case(&base_stream, decls_state)?;
    let trailing_pad_stream = stream_with_trailing_token(&base_stream, roles.pad_token);
    let non_pad_surplus_stream = stream_with_trailing_token(&base_stream, roles.node_start);

    let strict_cases = vec![
        strict_decode_case(
            "strict_complete_stream_accepts_state_api_eof",
            "go_source_instance_raw_tokens",
            complete_state,
            &base_stream,
            StrictDecodeExpectation::accepted(),
            &core,
        )?,
        strict_decode_case(
            "strict_partial_prefix_rejects_eof",
            "go_source_instance_segment_aligned_raw_prefix",
            decls_state,
            &decls_prefix_stream,
            StrictDecodeExpectation::rejected("premature_eof"),
            &core,
        )?,
        strict_decode_case(
            "strict_trailing_pad_rejects_raw_stream",
            "go_source_instance_raw_tokens_with_trailing_pad",
            complete_state,
            &trailing_pad_stream,
            StrictDecodeExpectation::rejected("pad_token_in_raw_stream"),
            &core,
        )?,
        strict_decode_case(
            "strict_non_pad_surplus_rejects_trailing_token",
            "go_source_instance_raw_tokens_with_non_pad_surplus",
            complete_state,
            &non_pad_surplus_stream,
            StrictDecodeExpectation::rejected("extra_trailing_token"),
            &core,
        )?,
    ];
    let recovery_cases = vec![
        recovery_decode_case(
            "recovery_complete_stream_returns_strict_success",
            "go_source_instance_raw_tokens",
            complete_state,
            "strict_success",
            &base_stream,
            &core,
        )?,
        recovery_decode_case(
            "recovery_trailing_pad_normalizes_after_state_api_complete",
            "go_source_instance_raw_tokens_with_trailing_pad",
            complete_state,
            "normalized_trailing_pad",
            &trailing_pad_stream,
            &core,
        )?,
        recovery_decode_case(
            "recovery_partial_prefix_remains_hard_error",
            "go_source_instance_segment_aligned_raw_prefix",
            decls_state,
            "hard_error",
            &decls_prefix_stream,
            &core,
        )?,
        recovery_decode_case(
            "recovery_non_pad_surplus_remains_hard_error",
            "go_source_instance_raw_tokens_with_non_pad_surplus",
            complete_state,
            "hard_error",
            &non_pad_surplus_stream,
            &core,
        )?,
    ];

    Ok(SyntaxGeneratedCodecStrictRecoveryDecodeReport {
        artifact_kind: GENERATED_CODEC_STRICT_RECOVERY_DECODE_ARTIFACT_KIND.to_string(),
        version: VERSION,
        tool: TOOL.to_string(),
        status: GENERATED_CODEC_STRICT_RECOVERY_DECODE_STATUS.to_string(),
        target: TARGET.to_string(),
        route: ROUTE.to_string(),
        decode_policy: GENERATED_CODEC_STRICT_RECOVERY_DECODE_POLICY.to_string(),
        state_api_artifact_kind: state_api_report.artifact_kind.clone(),
        state_api_scope: state_api_report.api_scope.clone(),
        candidate_grammar_id: projection_report.candidate_grammar_id.clone(),
        source_path: projection_report.source.path.clone(),
        source_sha256: projection_report.source.sha256.clone(),
        metadata_path: projection_report.metadata.path.clone(),
        metadata_sha256: projection_report.metadata.sha256.clone(),
        structural_model_id: raw_tokens.structural_model_id.clone(),
        token_layout: raw_tokens.token_layout.clone(),
        generated_codec_report_artifact_kind: generated_report.artifact_kind.clone(),
        generated_codec_report_path: generated_report.report_path.clone(),
        raw_token_count: raw_tokens.raw_token_count,
        raw_token_ids_sha256: raw_tokens.raw_token_ids_sha256.clone(),
        strict_cases,
        recovery_cases,
        state_api_consumed: true,
        prefix_state_api_ready: state_api_report.prefix_state_api_ready,
        strict_recovery_decode_wiring_ready: true,
        consumed_token_accounting_ok: true,
        complete_stream_eof_valid: complete_state.eof_valid,
        trailing_pad_recovery_ready: true,
        bpe_layer_defines_validity: false,
        next_missing_workstream_blocker:
            GENERATED_CODEC_STRICT_RECOVERY_DECODE_NEXT_BLOCKER.to_string(),
        validation_commands: vec![
            "cargo test --test syntax_generated_codec_gate_test".to_string(),
            "cargo test --test syntax_state_machine_replay_test".to_string(),
            "cargo test --test syntax_backend_boundary_test".to_string(),
            "git diff --check".to_string(),
        ],
        claim_boundary:
            "syntax-level strict/recovery decode bridge for checked Go source-instance raw tokens. Strict and recovery cases consume the generated-codec state API over GrammarIR -> StructuralModel -> token layout replay semantics; recovery normalizes only trailing PAD_TOKENs after a state-API-confirmed complete stream. C# parity, source validity-by-construction, arena-IR evidence, BPE validity, generated hot paths, paper-facing claims, and stable-backend promotion remain out of scope. This is not arena-IR evidence and not stable-backend promotion."
                .to_string(),
        no_stable_claims: STATE_API_NO_STABLE_CLAIMS
            .iter()
            .map(|claim| (*claim).to_string())
            .collect(),
    })
}

fn csharp_scratch_generation_summary(
    candidate_grammar_id: &str,
    sidecar: &GrammarStructuralModelSidecar,
    _token_layout: &ProjectionTokenLayoutSummary,
    core: &CompiledSyntaxModel,
    selected_completion_trace: &SyntaxSelectedCompletionTraceReport,
    strict_cases: &[SyntaxGeneratedCodecStrictDecodeCase],
) -> Result<Option<CSharpScratchGenerationSummary>, SyntaxBackendBoundaryError> {
    let Some(scope) = csharp_scratch_generation_scope(candidate_grammar_id) else {
        return Ok(None);
    };
    let parser_sidecar_validation_command =
        csharp_scratch_generation_parser_command(candidate_grammar_id).ok_or_else(|| {
            SyntaxBackendBoundaryError::new(
                "csharp_scratch_generation.parser_sidecar_validation_command",
                format!("{candidate_grammar_id} is not a scratch-generation owner route"),
            )
        })?;

    let empty_prefix = RawScratchEmptyPrefixSummary::from_core(
        core,
        "csharp_scratch_generation.raw_state_machine_summary",
    )?;
    let claim_flags = RawScratchClaimBoundaryFlags::raw_state_machine_only();

    let strict_incomplete = strict_cases
        .iter()
        .find(|case| case.case_id == "csharp_strict_partial_prefix_rejects_eof")
        .ok_or_else(|| {
            SyntaxBackendBoundaryError::new(
                "csharp_scratch_generation.strict_incomplete_prefix",
                "C# scratch summary requires the strict partial-prefix rejection case",
            )
        })?;
    let strict_incomplete = RawScratchStrictIncompletePrefixSummary::from_parts(
        !strict_incomplete.accepted,
        strict_incomplete.error_kind.clone().unwrap_or_default(),
        strict_incomplete.consumed_raw_tokens,
        strict_incomplete.total_raw_tokens,
        strict_incomplete.failing_token,
    );

    let token_only_stream = match candidate_grammar_id {
        CSHARP_MINIMAL_CLASS_GRAMMAR_ID => {
            build_csharp_minimal_class_raw_tree_render_token_stream_from_sidecar(sidecar)?
        }
        CSHARP_LOCAL_INTEGER_RETURN_LITERAL_GRAMMAR_ID => {
            build_csharp_local_integer_return_literal_raw_tree_render_token_stream_from_sidecar(
                sidecar,
            )?
        }
        CSHARP_LOCAL_UNARY_INTEGER_RETURN_GRAMMAR_ID => {
            build_csharp_local_unary_integer_return_raw_tree_render_token_stream_from_sidecar(
                sidecar,
            )?
        }
        CSHARP_MEMBER_ACCESS_EXPRESSION_GRAMMAR_ID => {
            build_csharp_member_access_expression_raw_tree_render_token_stream_from_sidecar(
                sidecar,
            )?
        }
        CSHARP_COMPILATION_UNIT_SOURCE_FILE_GRAMMAR_ID => {
            build_csharp_compilation_unit_source_file_raw_tree_render_token_stream_from_sidecar(
                sidecar,
            )?
        }
        CSHARP_COMPILATION_UNIT_LOGGER_SOURCE_FILE_GRAMMAR_ID => {
            build_csharp_compilation_unit_logger_source_file_raw_tree_render_token_stream_from_sidecar(
                sidecar,
            )?
        }
        CSHARP_COMPILATION_UNIT_VERSION_ONE_SOURCE_FILE_GRAMMAR_ID => {
            build_csharp_compilation_unit_version_one_source_file_raw_tree_render_token_stream_from_sidecar(
                sidecar,
            )?
        }
        _ => build_csharp_structural_replay_token_stream_from_sidecar(sidecar)?,
    };
    let decoded_tree = core
        .decode_complete_tree(&token_only_stream.raw_token_ids)
        .map_err(|error| {
            SyntaxBackendBoundaryError::new(
                "csharp_scratch_generation.token_only_decode",
                format!(
                    "C# token-only scratch stream should decode into a raw tree: {}",
                    error.message
                ),
            )
        })?;
    let token_only = RawScratchTokenOnlyDecodeSummary::from_tree(
        token_only_stream.raw_token_count,
        token_only_stream.raw_token_ids_sha256.clone(),
        &decoded_tree,
        "csharp_scratch_generation.token_only_decode",
    )?;
    let render_blocker = csharp_scratch_generation_render_blocker(candidate_grammar_id);
    let (
        renderer_id,
        renderer_scope,
        rendered_source_sha256,
        rendered_source_matches_checked_source,
        parser_sidecar_validation,
        render_reparse_sidecar_blocker,
    ) = if let Some(blocker) = render_blocker {
        (
            CSHARP_RAW_TREE_RENDERER_ID.to_string(),
            CSHARP_COMPILATION_UNIT_SOURCE_FILE_RAW_TREE_RENDERER_SCOPE.to_string(),
            String::new(),
            false,
            "blocked_pending_raw_tree_renderer".to_string(),
            Some(blocker.to_string()),
        )
    } else {
        let rendered = match candidate_grammar_id {
            CSHARP_MINIMAL_CLASS_GRAMMAR_ID => {
                render_csharp_minimal_class_source_from_raw_tree(&decoded_tree)?
            }
            CSHARP_LOCAL_INTEGER_RETURN_LITERAL_GRAMMAR_ID => {
                render_csharp_local_integer_return_literal_from_raw_tree(&decoded_tree)?
            }
            CSHARP_LOCAL_UNARY_INTEGER_RETURN_GRAMMAR_ID => {
                render_csharp_local_unary_integer_return_from_raw_tree(&decoded_tree)?
            }
            CSHARP_MEMBER_ACCESS_EXPRESSION_GRAMMAR_ID => {
                render_csharp_member_access_expression_from_raw_tree(&decoded_tree)?
            }
            CSHARP_COMPILATION_UNIT_SOURCE_FILE_GRAMMAR_ID => {
                render_csharp_compilation_unit_source_file_from_raw_tree(&decoded_tree)?
            }
            CSHARP_COMPILATION_UNIT_LOGGER_SOURCE_FILE_GRAMMAR_ID => {
                render_csharp_compilation_unit_logger_source_file_from_raw_tree(&decoded_tree)?
            }
            CSHARP_COMPILATION_UNIT_VERSION_ONE_SOURCE_FILE_GRAMMAR_ID => {
                render_csharp_compilation_unit_version_one_source_file_from_raw_tree(&decoded_tree)?
            }
            _ => {
                return Err(SyntaxBackendBoundaryError::new(
                    "csharp_scratch_generation.renderer",
                    format!("{candidate_grammar_id} is not a scratch-generation renderer route"),
                ));
            }
        };
        let rendered_source_sha256 = sha256_hex(rendered.source.as_bytes());
        let rendered_source_matches_checked_source = sidecar
            .input
            .provenance
            .source_sha256
            .as_deref()
            .is_some_and(|source_sha256| source_sha256 == rendered_source_sha256);
        (
            rendered.renderer_id,
            rendered.scope,
            rendered_source_sha256,
            rendered_source_matches_checked_source,
            "roslyn_self_test_required".to_string(),
            None,
        )
    };

    Ok(Some(CSharpScratchGenerationSummary {
        status: CSHARP_SCRATCH_GENERATION_STATUS.to_string(),
        scope: scope.to_string(),
        precompiled_artifact_kind: CSHARP_PRECOMPILED_SYNTAX_TOKENIZER_ARTIFACT_KIND.to_string(),
        precompiled_artifact_version: VERSION,
        precompiled_artifact_grammar_id: candidate_grammar_id.to_string(),
        precompiled_artifact_metadata_path: sidecar
            .input
            .provenance
            .metadata_path
            .clone()
            .unwrap_or_default(),
        precompiled_artifact_metadata_sha256: sidecar
            .input
            .provenance
            .metadata_sha256
            .clone()
            .unwrap_or_default(),
        precompiled_artifact_structural_model_id: sidecar.structural_model.model_id.clone(),
        raw_state_machine_core_boundary_id: empty_prefix.raw_state_machine_core_boundary_id,
        raw_state_machine_core_boundary: empty_prefix.raw_state_machine_core_boundary,
        token_layout_vocab_size: empty_prefix.token_layout_vocab_size,
        pad_token_id: empty_prefix.pad_token_id,
        empty_prefix_root: CSHARP_EMPTY_PREFIX_ROOT.to_string(),
        initial_state_observed_status: empty_prefix.initial_state.observed_status,
        initial_state_consumed_raw_tokens: empty_prefix.initial_state.consumed_raw_tokens,
        initial_state_total_raw_tokens: empty_prefix.initial_state.total_raw_tokens,
        initial_state_eof_valid: empty_prefix.initial_state.eof_valid,
        initial_state_stack_depth: empty_prefix.initial_state.stack_depth,
        initial_state_active_rule: empty_prefix.initial_state.active_rule,
        initial_state_active_field: empty_prefix.initial_state.active_field,
        finite_admissible_raw_token_count: empty_prefix.finite_admissible_raw_token_count,
        finite_admissible_raw_token_ids: empty_prefix.finite_admissible_raw_token_ids,
        finite_admissible_raw_token_ids_sha256: empty_prefix.finite_admissible_raw_token_ids_sha256,
        selected_raw_token_id: empty_prefix.selected_raw_token_id,
        selected_raw_token_admitted: empty_prefix.selected_raw_token_admitted,
        selected_transition_accepted: empty_prefix.selected_transition_accepted,
        selected_transition_observed_status: empty_prefix.selected_transition_state.observed_status,
        selected_transition_consumed_raw_tokens: empty_prefix
            .selected_transition_state
            .consumed_raw_tokens,
        selected_transition_eof_valid: empty_prefix.selected_transition_state.eof_valid,
        selected_transition_stack_depth: empty_prefix.selected_transition_state.stack_depth,
        selected_transition_active_rule: empty_prefix.selected_transition_state.active_rule,
        selected_transition_active_field: empty_prefix.selected_transition_state.active_field,
        selected_completion_trace_scope: selected_completion_trace.scope.clone(),
        selected_completion_raw_token_count: selected_completion_trace.raw_token_count,
        selected_completion_transition_count: selected_completion_trace.transition_count,
        selected_completion_admissible_mask_query_count: selected_completion_trace
            .admissible_mask_query_count,
        selected_completion_every_token_admitted: selected_completion_trace
            .every_selected_raw_token_admitted,
        selected_completion_final_status: selected_completion_trace
            .final_state_observed_status
            .clone(),
        selected_completion_final_eof_valid: selected_completion_trace.final_state_eof_valid,
        strict_incomplete_prefix_rejected: strict_incomplete.rejected,
        strict_incomplete_prefix_error_kind: strict_incomplete.error_kind,
        strict_incomplete_prefix_consumed_raw_tokens: strict_incomplete.consumed_raw_tokens,
        strict_incomplete_prefix_total_raw_tokens: strict_incomplete.total_raw_tokens,
        token_only_decode_render_ready: render_reparse_sidecar_blocker.is_none()
            && rendered_source_matches_checked_source,
        token_only_raw_token_count: token_only.raw_token_count,
        token_only_raw_token_ids_sha256: token_only.raw_token_ids_sha256,
        token_only_root_rule: token_only.root_rule,
        token_only_root_node_kind: token_only.root_node_kind,
        token_only_node_start_count: token_only.node_start_count,
        token_only_payload_value_count: token_only.payload_value_count,
        renderer_id,
        renderer_scope,
        rendered_source_sha256,
        rendered_source_matches_checked_source,
        parser_sidecar_validation,
        parser_sidecar_validation_command: parser_sidecar_validation_command.to_string(),
        render_reparse_sidecar_blocker,
        per_output_parser_metadata_required: claim_flags.per_output_parser_metadata_required,
        source_prefix_table_used: claim_flags.source_prefix_table_used,
        arena_ir_used: claim_flags.arena_ir_used,
        route_handoff_provenance_used: claim_flags.route_handoff_provenance_used,
        bpe_validity_authority_used: claim_flags.bpe_validity_authority_used,
        stable_backend_claimed: claim_flags.stable_backend_claimed,
        next_ready_task: CSHARP_SCRATCH_GENERATION_NEXT_READY_TASK.to_string(),
        claim_boundary: CSHARP_SCRATCH_GENERATION_CLAIM_BOUNDARY.to_string(),
    }))
}

pub fn build_csharp_state_machine_parity_report(
    candidate_grammar_id: &str,
    metadata_path: &Path,
) -> Result<SyntaxCSharpStateMachineParityReport, SyntaxBackendBoundaryError> {
    let grammar = CSharpSyntaxMetadataAdapter::new(candidate_grammar_id)
        .lower_metadata_file(metadata_path)
        .map_err(|error| {
            SyntaxBackendBoundaryError::new(
                "csharp_state_machine_parity.metadata",
                format!("failed to lower C# syntax metadata: {error}"),
            )
        })?;
    let sidecar = lower_grammar_ir_to_structural_model(&grammar, csharp_syntax_token_layout())
        .map_err(|error| {
            SyntaxBackendBoundaryError::new(
                "csharp_state_machine_parity.structural_model",
                format!("failed to lower C# GrammarIR to StructuralModel: {error}"),
            )
        })?;
    validate_csharp_parity_sidecar(&sidecar, candidate_grammar_id, metadata_path)?;

    let token_layout = projection_token_layout_summary(&sidecar.structural_model.token_ranges);
    validate_projection_token_layout(&token_layout)?;
    let facts = ModelFacts::from_sidecar(&sidecar);
    let core = CompiledSyntaxModel::from_sidecar_and_layout(&sidecar, &token_layout)?;
    let roles = core.token_roles();
    let base_stream = synthesize_csharp_replay_input(&sidecar, &facts, roles)?;
    let raw_token_ids_sha256 = json_sha256(&base_stream.raw_token_ids).map_err(|error| {
        SyntaxBackendBoundaryError::new(
            "csharp_state_machine_parity.raw_token_ids_sha256",
            format!("failed to hash C# raw token IDs: {error}"),
        )
    })?;
    let raw_token_count = base_stream.raw_token_ids.len();
    let max_raw_token_id = base_stream
        .raw_token_ids
        .iter()
        .copied()
        .max()
        .ok_or_else(|| {
            SyntaxBackendBoundaryError::new(
                "csharp_state_machine_parity.raw_token_ids",
                "C# synthetic replay stream must not be empty",
            )
        })?;
    if max_raw_token_id >= roles.vocab_size {
        return Err(SyntaxBackendBoundaryError::new(
            "csharp_state_machine_parity.raw_token_ids",
            "C# synthetic replay stream emitted a token outside VOCAB_SIZE",
        ));
    }
    if base_stream.raw_token_ids.contains(&roles.pad_token) {
        return Err(SyntaxBackendBoundaryError::new(
            "csharp_state_machine_parity.raw_token_ids",
            "C# synthetic replay stream must not contain PAD_TOKEN",
        ));
    }

    let complete_state = core
        .strict_decode_complete(&base_stream.raw_token_ids)
        .map_err(|error| {
            SyntaxBackendBoundaryError::new(
                "csharp_state_machine_parity.complete_stream",
                format!("valid C# metadata stream should replay: {}", error.message),
            )
        })?;
    let complete_stream = SyntaxReplayCaseResult::accepted(
        "complete_csharp_structural_model_stream",
        "csharp_syntax_metadata_synthesized_raw_tokens",
        "complete",
        complete_state,
    );
    let selected_completion_trace =
        selected_completion_trace_report(&core, &base_stream.raw_token_ids, &complete_stream)?;

    let root_prefix_indices = vec![0];
    let first_field_start_index = segment_index(&base_stream, "first field_start", |segment| {
        segment.event_kind == "field_start"
    })?;
    let first_field_prefix_indices = (0..=first_field_start_index).collect::<Vec<_>>();
    let multi_alternative_field_start_index =
        base_stream.raw_token_segments.iter().position(|segment| {
            segment.event_kind == "field_start"
                && segment
                    .owner_rule
                    .as_ref()
                    .zip(segment.owner_field.as_ref())
                    .is_some_and(|(owner_rule, owner_field)| {
                        facts
                            .multi_alternative_nonterminal_fields
                            .contains(&(owner_rule.clone(), owner_field.clone()))
                    })
        });
    let first_payload_segment_index = first_variable_payload_segment_index(&base_stream)?;
    let complete_indices = (0..base_stream.raw_token_segments.len()).collect::<Vec<_>>();

    let mut state_cases = vec![
        generated_codec_state_case(
            "csharp_empty_prefix",
            "csharp_syntax_metadata_empty_raw_prefix",
            &base_stream,
            &[],
            &core,
        )?,
        generated_codec_state_case(
            "csharp_after_root_node_start",
            "csharp_syntax_metadata_segment_aligned_raw_prefix",
            &base_stream,
            &root_prefix_indices,
            &core,
        )?,
        generated_codec_state_case(
            "csharp_after_first_field_start",
            "csharp_syntax_metadata_segment_aligned_raw_prefix",
            &base_stream,
            &first_field_prefix_indices,
            &core,
        )?,
        generated_codec_payload_range_state_case(
            "csharp_inside_first_terminal_payload_after_one_byte",
            "csharp_syntax_metadata_partial_terminal_payload_prefix",
            &base_stream,
            first_payload_segment_index,
            1,
            &core,
        )?,
        generated_codec_state_case(
            "csharp_complete_stream",
            "csharp_syntax_metadata_complete_raw_stream",
            &base_stream,
            &complete_indices,
            &core,
        )?,
    ];
    if let Some(multi_alternative_field_start_index) = multi_alternative_field_start_index {
        let multi_alternative_prefix_indices =
            (0..=multi_alternative_field_start_index).collect::<Vec<_>>();
        state_cases.push(generated_codec_state_case(
            "csharp_after_first_multi_alternative_field_start",
            "csharp_syntax_metadata_segment_aligned_raw_prefix",
            &base_stream,
            &multi_alternative_prefix_indices,
            &core,
        )?);
    }

    let (transition_state_case_id, transition_prefix_indices, transition_next_index) =
        if let Some(multi_alternative_field_start_index) = multi_alternative_field_start_index {
            (
                "csharp_after_first_multi_alternative_field_start",
                (0..=multi_alternative_field_start_index).collect::<Vec<_>>(),
                multi_alternative_field_start_index
                    .checked_add(1)
                    .ok_or_else(|| {
                        SyntaxBackendBoundaryError::new(
                            "csharp_state_machine_parity.transition",
                            "C# multi-alternative transition index overflowed",
                        )
                    })?,
            )
        } else {
            (
                "csharp_after_first_field_start",
                first_field_prefix_indices,
                first_field_start_index.checked_add(1).ok_or_else(|| {
                    SyntaxBackendBoundaryError::new(
                        "csharp_state_machine_parity.transition",
                        "C# first-field transition index overflowed",
                    )
                })?,
            )
        };
    let actual_transition_token = next_single_segment_token(&base_stream, transition_next_index)?;
    let is_multi_alternative_transition =
        transition_state_case_id == "csharp_after_first_multi_alternative_field_start";
    let accepted_transition_case_id = if is_multi_alternative_transition {
        "csharp_multi_alternative_actual_node_start_is_accepted"
    } else {
        "csharp_first_field_actual_token_is_accepted"
    };
    let rejected_transition_case_id = if is_multi_alternative_transition {
        "csharp_multi_alternative_generic_node_start_is_rejected"
    } else {
        "csharp_first_field_node_end_is_rejected"
    };
    let rejected_transition_token = if is_multi_alternative_transition {
        roles.node_start
    } else {
        roles.node_end
    };
    let mut transition_cases = vec![
        generated_codec_transition_case(
            accepted_transition_case_id,
            transition_state_case_id,
            GeneratedCodecTransitionProbe {
                base_stream: &base_stream,
                prefix_indices: &transition_prefix_indices,
                next_segment_index: transition_next_index,
                candidate_raw_token_id: actual_transition_token,
                expected_accepted: true,
            },
            &core,
        )?,
        generated_codec_transition_case(
            rejected_transition_case_id,
            transition_state_case_id,
            GeneratedCodecTransitionProbe {
                base_stream: &base_stream,
                prefix_indices: &transition_prefix_indices,
                next_segment_index: transition_next_index,
                candidate_raw_token_id: rejected_transition_token,
                expected_accepted: false,
            },
            &core,
        )?,
    ];

    let decoded_tree = core
        .decode_complete_tree(&base_stream.raw_token_ids)
        .map_err(|error| {
            SyntaxBackendBoundaryError::new(
                "csharp_state_machine_parity.decoded_tree",
                format!(
                    "C# structural replay should produce tree events: {}",
                    error.message
                ),
            )
        })?;
    let marker_presence_decisions = csharp_marker_presence_decisions(
        &sidecar,
        &base_stream,
        &decoded_tree.events,
        &core,
        roles,
    )?;
    add_csharp_nullable_marker_state_api_cases(
        &mut state_cases,
        &mut transition_cases,
        &base_stream,
        &marker_presence_decisions,
        &core,
        roles,
    )?;
    let marker_presence_invalid_payload_mutations =
        csharp_marker_presence_invalid_payload_mutations(
            &base_stream,
            &marker_presence_decisions,
            &core,
            roles,
        )?;
    let invalid_mutation_cases = csharp_invalid_mutation_cases(&base_stream, &core)?;
    let complete_state = required_synthetic_state_case(&state_cases, "csharp_complete_stream")?;
    let partial_state = required_synthetic_state_case(&state_cases, transition_state_case_id)?;
    let partial_stream = stream_from_state_case(&base_stream, partial_state)?;
    let trailing_pad_stream = stream_with_trailing_token(&base_stream, roles.pad_token);
    let non_pad_surplus_stream = stream_with_trailing_token(&base_stream, roles.node_start);

    let strict_cases = vec![
        strict_decode_case(
            "csharp_strict_complete_stream_accepts_state_api_eof",
            "csharp_syntax_metadata_synthesized_raw_tokens",
            complete_state,
            &base_stream,
            StrictDecodeExpectation::accepted(),
            &core,
        )?,
        strict_decode_case(
            "csharp_strict_partial_prefix_rejects_eof",
            "csharp_syntax_metadata_segment_aligned_raw_prefix",
            partial_state,
            &partial_stream,
            StrictDecodeExpectation::rejected("premature_eof"),
            &core,
        )?,
        strict_decode_case(
            "csharp_strict_trailing_pad_rejects_raw_stream",
            "csharp_syntax_metadata_raw_tokens_with_trailing_pad",
            complete_state,
            &trailing_pad_stream,
            StrictDecodeExpectation::rejected("pad_token_in_raw_stream"),
            &core,
        )?,
        strict_decode_case(
            "csharp_strict_non_pad_surplus_rejects_trailing_token",
            "csharp_syntax_metadata_raw_tokens_with_non_pad_surplus",
            complete_state,
            &non_pad_surplus_stream,
            StrictDecodeExpectation::rejected("extra_trailing_token"),
            &core,
        )?,
    ];
    let recovery_cases = vec![
        recovery_decode_case(
            "csharp_recovery_complete_stream_returns_strict_success",
            "csharp_syntax_metadata_synthesized_raw_tokens",
            complete_state,
            "strict_success",
            &base_stream,
            &core,
        )?,
        recovery_decode_case(
            "csharp_recovery_trailing_pad_normalizes_after_state_api_complete",
            "csharp_syntax_metadata_raw_tokens_with_trailing_pad",
            complete_state,
            "normalized_trailing_pad",
            &trailing_pad_stream,
            &core,
        )?,
        recovery_decode_case(
            "csharp_recovery_partial_prefix_remains_hard_error",
            "csharp_syntax_metadata_segment_aligned_raw_prefix",
            partial_state,
            "hard_error",
            &partial_stream,
            &core,
        )?,
        recovery_decode_case(
            "csharp_recovery_non_pad_surplus_remains_hard_error",
            "csharp_syntax_metadata_raw_tokens_with_non_pad_surplus",
            complete_state,
            "hard_error",
            &non_pad_surplus_stream,
            &core,
        )?,
    ];
    let scratch_generation_summary = csharp_scratch_generation_summary(
        candidate_grammar_id,
        &sidecar,
        &token_layout,
        &core,
        &selected_completion_trace,
        &strict_cases,
    )?;

    let parser = sidecar.input.provenance.parser.as_ref().ok_or_else(|| {
        SyntaxBackendBoundaryError::new(
            "csharp_state_machine_parity.parser",
            "C# parity sidecar must preserve parser provenance",
        )
    })?;
    Ok(SyntaxCSharpStateMachineParityReport {
        artifact_kind: CSHARP_STATE_MACHINE_PARITY_ARTIFACT_KIND.to_string(),
        version: VERSION,
        tool: TOOL.to_string(),
        status: CSHARP_STATE_MACHINE_PARITY_STATUS.to_string(),
        target: CSHARP_STATE_MACHINE_PARITY_TARGET.to_string(),
        route: CSHARP_STATE_MACHINE_PARITY_ROUTE.to_string(),
        api_scope: CSHARP_STATE_MACHINE_PARITY_API_SCOPE.to_string(),
        candidate_grammar_id: candidate_grammar_id.to_string(),
        source_path: sidecar
            .input
            .provenance
            .source_path
            .clone()
            .unwrap_or_default(),
        source_sha256: sidecar
            .input
            .provenance
            .source_sha256
            .clone()
            .unwrap_or_default(),
        metadata_path: sidecar
            .input
            .provenance
            .metadata_path
            .clone()
            .unwrap_or_default(),
        metadata_sha256: sidecar
            .input
            .provenance
            .metadata_sha256
            .clone()
            .unwrap_or_default(),
        parser_tool: parser.tool.clone(),
        parser_version: parser.version.clone(),
        structural_model_id: sidecar.structural_model.model_id.clone(),
        token_layout,
        raw_token_count,
        raw_token_ids_sha256,
        max_raw_token_id,
        complete_stream,
        selected_completion_trace,
        scratch_generation_summary,
        state_cases,
        transition_cases,
        marker_presence_decisions,
        invalid_mutation_cases,
        marker_presence_invalid_payload_mutations,
        strict_cases,
        recovery_cases,
        state_after_prefix_ready: true,
        admissible_next_raw_tokens_ready: true,
        invalid_mutation_rejection_ready: true,
        strict_recovery_decode_agreement_ready: true,
        consumed_token_accounting_ok: true,
        token_layout_invariants_ok: true,
        target_metadata_sidecar_only: true,
        arena_ir_used: false,
        bpe_layer_defines_validity: false,
        cross_target_parity_claim_scope: CSHARP_STATE_MACHINE_PARITY_CLAIM_SCOPE.to_string(),
        next_ready_task: CSHARP_STATE_MACHINE_PARITY_NEXT_READY_TASK.to_string(),
        validation_commands: vec![
            "cargo test --test syntax_state_machine_replay_test".to_string(),
            "cargo test --test syntax_structural_model_cross_target_equivalence_test".to_string(),
            "cargo test --test syntax_generated_codec_gate_test".to_string(),
            "./scripts/testing/test-syntax-backend-gates.sh".to_string(),
            "git diff --check".to_string(),
        ],
        claim_boundary:
            "syntax-level C# state-machine parity evidence over checked Roslyn metadata lowered through GrammarIR -> StructuralModel -> token layout. It uses the same replay, prefix-state, admissible-token, strict/recovery decode, consumed-token accounting, and token-layout semantics as the Go source-instance state API; C# parser/render facts remain sidecar metadata. This is not arena-IR evidence and not stable-backend promotion."
                .to_string(),
        no_stable_claims: CSHARP_PARITY_NO_STABLE_CLAIMS
            .iter()
            .map(|claim| (*claim).to_string())
            .collect(),
    })
}

pub fn build_csharp_structural_replay_token_stream_from_sidecar(
    sidecar: &GrammarStructuralModelSidecar,
) -> Result<CSharpStructuralReplayTokenStream, SyntaxBackendBoundaryError> {
    build_csharp_structural_replay_token_stream_from_sidecar_with_profile(
        sidecar,
        StructuralReplayProfile::CSharpDefault,
    )
}

pub fn build_csharp_minimal_class_raw_tree_render_token_stream_from_sidecar(
    sidecar: &GrammarStructuralModelSidecar,
) -> Result<CSharpStructuralReplayTokenStream, SyntaxBackendBoundaryError> {
    build_csharp_structural_replay_token_stream_from_sidecar_with_profile(
        sidecar,
        StructuralReplayProfile::CSharpMinimalClassFixture,
    )
}

pub fn build_csharp_local_integer_return_literal_raw_tree_render_token_stream_from_sidecar(
    sidecar: &GrammarStructuralModelSidecar,
) -> Result<CSharpStructuralReplayTokenStream, SyntaxBackendBoundaryError> {
    build_csharp_structural_replay_token_stream_from_sidecar_with_profile(
        sidecar,
        StructuralReplayProfile::CSharpLocalIntegerReturnFixture,
    )
}

pub fn build_csharp_local_unary_integer_return_raw_tree_render_token_stream_from_sidecar(
    sidecar: &GrammarStructuralModelSidecar,
) -> Result<CSharpStructuralReplayTokenStream, SyntaxBackendBoundaryError> {
    let token_layout = projection_token_layout_summary(&sidecar.structural_model.token_ranges);
    validate_projection_token_layout(&token_layout)?;
    let facts = ModelFacts::from_sidecar(sidecar);
    let core = CompiledSyntaxModel::from_sidecar_and_layout(sidecar, &token_layout)?;
    let roles = core.token_roles();
    let raw_stream =
        synthesize_csharp_local_unary_integer_return_raw_tree_render_input(sidecar, &facts, roles)?;
    build_csharp_structural_replay_token_stream_from_raw_input(sidecar, token_layout, raw_stream)
}

pub fn build_csharp_member_access_expression_raw_tree_render_token_stream_from_sidecar(
    sidecar: &GrammarStructuralModelSidecar,
) -> Result<CSharpStructuralReplayTokenStream, SyntaxBackendBoundaryError> {
    let token_layout = projection_token_layout_summary(&sidecar.structural_model.token_ranges);
    validate_projection_token_layout(&token_layout)?;
    let facts = ModelFacts::from_sidecar(sidecar);
    let core = CompiledSyntaxModel::from_sidecar_and_layout(sidecar, &token_layout)?;
    let roles = core.token_roles();
    let raw_stream =
        synthesize_csharp_member_access_expression_raw_tree_render_input(sidecar, &facts, roles)?;
    build_csharp_structural_replay_token_stream_from_raw_input(sidecar, token_layout, raw_stream)
}

pub fn build_csharp_compilation_unit_source_file_raw_tree_render_token_stream_from_sidecar(
    sidecar: &GrammarStructuralModelSidecar,
) -> Result<CSharpStructuralReplayTokenStream, SyntaxBackendBoundaryError> {
    let token_layout = projection_token_layout_summary(&sidecar.structural_model.token_ranges);
    validate_projection_token_layout(&token_layout)?;
    let facts = ModelFacts::from_sidecar(sidecar);
    let core = CompiledSyntaxModel::from_sidecar_and_layout(sidecar, &token_layout)?;
    let roles = core.token_roles();
    let raw_stream = synthesize_csharp_compilation_unit_source_file_raw_tree_render_input(
        sidecar, &facts, roles,
    )?;
    build_csharp_structural_replay_token_stream_from_raw_input(sidecar, token_layout, raw_stream)
}

pub fn build_csharp_compilation_unit_logger_source_file_raw_tree_render_token_stream_from_sidecar(
    sidecar: &GrammarStructuralModelSidecar,
) -> Result<CSharpStructuralReplayTokenStream, SyntaxBackendBoundaryError> {
    let token_layout = projection_token_layout_summary(&sidecar.structural_model.token_ranges);
    validate_projection_token_layout(&token_layout)?;
    let facts = ModelFacts::from_sidecar(sidecar);
    let core = CompiledSyntaxModel::from_sidecar_and_layout(sidecar, &token_layout)?;
    let roles = core.token_roles();
    let raw_stream = synthesize_csharp_compilation_unit_logger_source_file_raw_tree_render_input(
        sidecar, &facts, roles,
    )?;
    build_csharp_structural_replay_token_stream_from_raw_input(sidecar, token_layout, raw_stream)
}

pub fn build_csharp_compilation_unit_version_one_source_file_raw_tree_render_token_stream_from_sidecar(
    sidecar: &GrammarStructuralModelSidecar,
) -> Result<CSharpStructuralReplayTokenStream, SyntaxBackendBoundaryError> {
    let token_layout = projection_token_layout_summary(&sidecar.structural_model.token_ranges);
    validate_projection_token_layout(&token_layout)?;
    let facts = ModelFacts::from_sidecar(sidecar);
    let core = CompiledSyntaxModel::from_sidecar_and_layout(sidecar, &token_layout)?;
    let roles = core.token_roles();
    let raw_stream =
        synthesize_csharp_compilation_unit_version_one_source_file_raw_tree_render_input(
            sidecar, &facts, roles,
        )?;
    build_csharp_structural_replay_token_stream_from_raw_input(sidecar, token_layout, raw_stream)
}

fn build_csharp_structural_replay_token_stream_from_sidecar_with_profile(
    sidecar: &GrammarStructuralModelSidecar,
    profile: StructuralReplayProfile,
) -> Result<CSharpStructuralReplayTokenStream, SyntaxBackendBoundaryError> {
    let token_layout = projection_token_layout_summary(&sidecar.structural_model.token_ranges);
    validate_projection_token_layout(&token_layout)?;
    let facts = ModelFacts::from_sidecar(sidecar);
    let core = CompiledSyntaxModel::from_sidecar_and_layout(sidecar, &token_layout)?;
    let roles = core.token_roles();
    let raw_stream = synthesize_structural_replay_input(sidecar, &facts, roles, profile)?;
    build_csharp_structural_replay_token_stream_from_raw_input(sidecar, token_layout, raw_stream)
}

fn build_csharp_structural_replay_token_stream_from_raw_input(
    sidecar: &GrammarStructuralModelSidecar,
    token_layout: ProjectionTokenLayoutSummary,
    raw_stream: RawReplayInput,
) -> Result<CSharpStructuralReplayTokenStream, SyntaxBackendBoundaryError> {
    let raw_token_ids_sha256 = json_sha256(&raw_stream.raw_token_ids).map_err(|error| {
        SyntaxBackendBoundaryError::new(
            "csharp_structural_replay.raw_token_ids_sha256",
            format!("failed to hash C# structural replay tokens: {error}"),
        )
    })?;
    let raw_token_segments_sha256 =
        json_sha256(&raw_stream.raw_token_segments).map_err(|error| {
            SyntaxBackendBoundaryError::new(
                "csharp_structural_replay.raw_token_segments_sha256",
                format!("failed to hash C# structural replay token segments: {error}"),
            )
        })?;
    let max_raw_token_id = raw_stream.raw_token_ids.iter().copied().max();
    let raw_tokens_below_vocab_size = raw_stream
        .raw_token_ids
        .iter()
        .all(|token| *token < token_layout.vocab_size);
    let pad_token_absent = raw_stream
        .raw_token_ids
        .iter()
        .all(|token| *token != token_layout.pad_token);
    let stream = CSharpStructuralReplayTokenStream {
        artifact_kind: CSHARP_STRUCTURAL_REPLAY_TOKEN_STREAM_ARTIFACT_KIND.to_string(),
        version: VERSION,
        token_layout_kind: CSHARP_STRUCTURAL_REPLAY_TOKEN_LAYOUT_KIND.to_string(),
        source: CSHARP_STRUCTURAL_REPLAY_TOKEN_STREAM_SOURCE.to_string(),
        structural_model_id: sidecar.structural_model.model_id.clone(),
        token_layout,
        raw_token_count: raw_stream.raw_token_ids.len(),
        raw_token_ids_sha256,
        max_raw_token_id,
        raw_tokens_below_vocab_size,
        pad_token_absent,
        raw_token_segment_count: raw_stream.raw_token_segments.len(),
        raw_token_segments_sha256,
        raw_token_segments: raw_stream.raw_token_segments,
        raw_token_ids: raw_stream.raw_token_ids,
    };
    stream.validate()?;
    Ok(stream)
}

pub fn build_go_ast_declaration_presence_replay_report()
-> Result<GoAstDeclarationPresenceReplayReport, SyntaxBackendBoundaryError> {
    let inventory_grammar = GoAstSchemaInventoryAdapter::lower_inventory_file(Path::new(
        GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH,
    ))
    .map_err(|error| {
        SyntaxBackendBoundaryError::new(
            "go_ast_declaration_presence.inventory",
            format!("failed to lower Go AST schema inventory: {error}"),
        )
    })?;
    validate_go_ast_declaration_presence_inventory(&inventory_grammar)?;
    let inventory_grammar_id = inventory_grammar.grammar_id.clone();
    let replay_grammar = go_ast_declaration_presence_replay_grammar(inventory_grammar)?;
    let sidecar = lower_grammar_ir_to_structural_model(&replay_grammar, go_syntax_token_layout())
        .map_err(|error| {
        SyntaxBackendBoundaryError::new(
            "go_ast_declaration_presence.structural_model",
            format!("failed to lower Go AST declaration presence replay grammar: {error}"),
        )
    })?;
    validate_go_ast_declaration_presence_sidecar(&sidecar)?;

    let token_layout = projection_token_layout_summary(&sidecar.structural_model.token_ranges);
    validate_projection_token_layout(&token_layout)?;
    let facts = ModelFacts::from_sidecar(&sidecar);
    let core = CompiledSyntaxModel::from_sidecar_and_layout(&sidecar, &token_layout)?;
    let roles = core.token_roles();
    let base_stream = synthesize_structural_replay_input(
        &sidecar,
        &facts,
        roles,
        StructuralReplayProfile::GoAstDeclarationPresence,
    )?;
    validate_raw_token_segments(
        &base_stream.raw_token_segments,
        base_stream.raw_token_ids.len(),
    )?;
    let raw_token_ids_sha256 = json_sha256(&base_stream.raw_token_ids).map_err(|error| {
        SyntaxBackendBoundaryError::new(
            "go_ast_declaration_presence.raw_token_ids_sha256",
            format!("failed to hash Go AST declaration presence raw tokens: {error}"),
        )
    })?;
    let raw_token_segments_sha256 =
        json_sha256(&base_stream.raw_token_segments).map_err(|error| {
            SyntaxBackendBoundaryError::new(
                "go_ast_declaration_presence.raw_token_segments_sha256",
                format!("failed to hash Go AST declaration presence raw token segments: {error}"),
            )
        })?;
    let complete_state = core
        .strict_decode_complete(&base_stream.raw_token_ids)
        .map_err(|error| {
            SyntaxBackendBoundaryError::new(
                "go_ast_declaration_presence.complete_stream",
                format!(
                    "Go AST declaration presence replay should decode: {}",
                    error.message
                ),
            )
        })?;
    let complete_stream = SyntaxReplayCaseResult::accepted(
        "go_ast_declaration_presence_complete_stream",
        "go_ast_inventory_structural_replay_raw_tokens",
        "complete",
        complete_state,
    );
    let decoded_tree = core
        .decode_complete_tree(&base_stream.raw_token_ids)
        .map_err(|error| {
            SyntaxBackendBoundaryError::new(
                "go_ast_declaration_presence.decoded_tree",
                format!(
                    "Go AST declaration presence replay should produce tree events: {}",
                    error.message
                ),
            )
        })?;
    let presence_decisions = [
        ("GenDecl", "Lparen"),
        ("GenDecl", "Rparen"),
        ("TypeSpec", "Assign"),
        ("FuncType", "Func"),
        ("CallExpr", "Ellipsis"),
    ]
    .into_iter()
    .map(|(owner_rule, owner_field)| {
        go_ast_declaration_presence_decision(
            &sidecar,
            &base_stream,
            &decoded_tree.events,
            &core,
            roles,
            owner_rule,
            owner_field,
        )
    })
    .collect::<Result<Vec<_>, _>>()?;
    let mutation_offset = presence_decisions
        .iter()
        .find(|decision| decision.owner_rule == "TypeSpec" && decision.owner_field == "Assign")
        .map(|decision| decision.boolean_payload_raw_token_offset)
        .ok_or_else(|| {
            SyntaxBackendBoundaryError::new(
                "go_ast_declaration_presence.invalid_mutation",
                "missing TypeSpec.Assign presence decision for invalid mutation",
            )
        })?;
    let mut invalid_boolean_payload = base_stream.raw_token_ids.clone();
    invalid_boolean_payload[mutation_offset] = roles.node_start;
    let invalid_error = match core.strict_decode_complete(&invalid_boolean_payload) {
        Ok(_) => {
            return Err(SyntaxBackendBoundaryError::new(
                "go_ast_declaration_presence.invalid_mutation",
                "invalid Go AST declaration presence boolean payload should be rejected",
            ));
        }
        Err(error) => error,
    };
    let invalid_boolean_payload_mutation = SyntaxReplayCaseResult::rejected(
        "go_ast_type_spec_assign_invalid_boolean_payload_is_rejected",
        "go_ast_inventory_structural_replay_raw_tokens_with_invalid_boolean_payload",
        "invalid_payload_token",
        invalid_error,
        invalid_boolean_payload.len(),
    );
    let state_after_terminal_admissible_boolean_ready = presence_decisions.iter().all(|decision| {
        decision.state_after_terminal_admits_false && decision.state_after_terminal_admits_true
    });
    let report = GoAstDeclarationPresenceReplayReport {
        artifact_kind: GO_AST_DECLARATION_PRESENCE_REPLAY_ARTIFACT_KIND.to_string(),
        version: VERSION,
        tool: TOOL.to_string(),
        status: GO_AST_DECLARATION_PRESENCE_REPLAY_STATUS.to_string(),
        target: GO_AST_DECLARATION_PRESENCE_REPLAY_TARGET.to_string(),
        route: GO_AST_DECLARATION_PRESENCE_REPLAY_ROUTE.to_string(),
        scope: GO_AST_DECLARATION_PRESENCE_REPLAY_SCOPE.to_string(),
        inventory_path: GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH.to_string(),
        pinned_go_version: GO_AST_SCHEMA_INVENTORY_PINNED_GO_VERSION.to_string(),
        inventory_grammar_id,
        replay_grammar_id: replay_grammar.grammar_id.clone(),
        structural_model_id: sidecar.structural_model.model_id.clone(),
        token_layout,
        raw_token_count: base_stream.raw_token_ids.len(),
        raw_token_ids_sha256,
        raw_token_segments_sha256,
        declaration_replay_path: vec![
            "File.Decls -> GenDecl".to_string(),
            "GenDecl.Specs -> TypeSpec".to_string(),
            "TypeSpec.Type -> Ident".to_string(),
            "File.Decls -> FuncDecl".to_string(),
            "FuncDecl.Type -> FuncType".to_string(),
            "FuncDecl.Body -> BlockStmt".to_string(),
            "BlockStmt.List -> ReturnStmt".to_string(),
            "ReturnStmt.Results -> CallExpr".to_string(),
            "CallExpr.Fun -> Ident".to_string(),
            "CallExpr.Args -> Ident".to_string(),
        ],
        presence_decisions,
        complete_stream,
        invalid_boolean_payload_mutation,
        raw_state_machine_complete: true,
        declaration_presence_decisions_consumed: true,
        state_after_terminal_admissible_boolean_ready,
        inventory_backed_structural_model_used: true,
        arena_ir_used: false,
        bpe_layer_defines_validity: false,
        stable_backend_promotion_claimed: false,
        next_ready_task: GO_AST_DECLARATION_PRESENCE_REPLAY_NEXT_READY_TASK.to_string(),
        validation_commands: vec![
            "cargo test --test go_ast_schema_inventory_test".to_string(),
            "cargo test --test syntax_state_machine_replay_test".to_string(),
            "cargo test --test syntax_go_source_instance_projection_test".to_string(),
            "cargo run --bin tct-syntax-go-tokenizer-generator".to_string(),
            "python3 scripts/testing/test-quality-audit.py --changed-from origin/main".to_string(),
            "git diff --check".to_string(),
        ],
        claim_boundary:
            "syntax-level Go AST inventory replay evidence over GrammarIR -> StructuralModel -> token layout -> raw syntax state machine. It consumes inventory-backed grouped-declaration, type-alias, function-keyword, and variadic-call optional-token presence decisions as raw boolean payloads for a bounded replay slice; it is not a full Go source validity-by-construction claim, not renderer evidence, not arena-IR evidence, not BPE validity, and not stable-backend promotion."
                .to_string(),
        no_stable_claims: STATE_API_NO_STABLE_CLAIMS
            .iter()
            .map(|claim| (*claim).to_string())
            .collect(),
    };
    report.validate()?;
    Ok(report)
}

pub fn build_csharp_structural_replay_token_stream(
    candidate_grammar_id: &str,
    metadata_path: &Path,
) -> Result<CSharpStructuralReplayTokenStream, SyntaxBackendBoundaryError> {
    let grammar = CSharpSyntaxMetadataAdapter::new(candidate_grammar_id)
        .lower_metadata_file(metadata_path)
        .map_err(|error| {
            SyntaxBackendBoundaryError::new(
                "csharp_structural_replay.metadata",
                format!("failed to lower C# syntax metadata: {error}"),
            )
        })?;
    let sidecar = lower_grammar_ir_to_structural_model(&grammar, csharp_syntax_token_layout())
        .map_err(|error| {
            SyntaxBackendBoundaryError::new(
                "csharp_structural_replay.structural_model",
                format!("failed to lower C# GrammarIR to StructuralModel: {error}"),
            )
        })?;
    validate_csharp_parity_sidecar(&sidecar, candidate_grammar_id, metadata_path)?;
    build_csharp_structural_replay_token_stream_from_sidecar(&sidecar)
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CSharpSourceRecordRawTokenEmission {
    api_id: String,
    api_boundary: String,
    structural_replay_token_stream: CSharpStructuralReplayTokenStream,
    raw_tokens: RawReplayInput,
}

fn emit_csharp_source_record_raw_tokens(
    record: &CSharpGeneratedCodecCorpusSupportedRecord,
    sidecar: &GrammarStructuralModelSidecar,
    token_layout: &ProjectionTokenLayoutSummary,
    structural_replay_token_stream: CSharpStructuralReplayTokenStream,
) -> Result<CSharpSourceRecordRawTokenEmission, SyntaxBackendBoundaryError> {
    structural_replay_token_stream.validate()?;
    require_string(
        "csharp_source_record_raw_token_emission.source_path",
        sidecar
            .input
            .provenance
            .source_path
            .as_deref()
            .unwrap_or_default(),
        &record.source_path,
    )?;
    require_string(
        "csharp_source_record_raw_token_emission.source_sha256",
        sidecar
            .input
            .provenance
            .source_sha256
            .as_deref()
            .unwrap_or_default(),
        &record.source_sha256,
    )?;
    require_sha256(
        "csharp_source_record_raw_token_emission.metadata_sha256",
        sidecar
            .input
            .provenance
            .metadata_sha256
            .as_deref()
            .unwrap_or_default(),
    )?;
    require_string(
        "csharp_source_record_raw_token_emission.structural_model_id",
        &structural_replay_token_stream.structural_model_id,
        &sidecar.structural_model.model_id,
    )?;
    if &structural_replay_token_stream.token_layout != token_layout {
        return Err(SyntaxBackendBoundaryError::new(
            "csharp_source_record_raw_token_emission.token_layout",
            "C# source-record raw-token emission layout must match the compiled structural model layout",
        ));
    }
    require_bool(
        "csharp_source_record_raw_token_emission.raw_tokens_below_vocab_size",
        structural_replay_token_stream.raw_tokens_below_vocab_size,
        true,
    )?;
    require_bool(
        "csharp_source_record_raw_token_emission.pad_token_absent",
        structural_replay_token_stream.pad_token_absent,
        true,
    )?;
    Ok(CSharpSourceRecordRawTokenEmission {
        api_id: CSHARP_SOURCE_RECORD_RAW_TOKEN_EMISSION_API_ID.to_string(),
        api_boundary: CSHARP_SOURCE_RECORD_RAW_TOKEN_EMISSION_API_BOUNDARY.to_string(),
        raw_tokens: RawReplayInput {
            raw_token_ids: structural_replay_token_stream.raw_token_ids.clone(),
            raw_token_segments: structural_replay_token_stream.raw_token_segments.clone(),
        },
        structural_replay_token_stream,
    })
}

fn csharp_real_source_metadata_path_for_manifest(
    manifest_path: &str,
) -> Result<&'static str, SyntaxBackendBoundaryError> {
    match manifest_path {
        SUPPORTED_MANIFEST_PATH => Ok(CSHARP_COMPILATION_UNIT_SOURCE_FILE_METADATA_PATH),
        LOGGER_MANIFEST_PATH => Ok(CSHARP_COMPILATION_UNIT_LOGGER_SOURCE_FILE_METADATA_PATH),
        VERSION_ONE_MANIFEST_PATH => {
            Ok(CSHARP_COMPILATION_UNIT_VERSION_ONE_SOURCE_FILE_METADATA_PATH)
        }
        ITERATOR_METHODS_MANIFEST_PATH => {
            Ok(CSHARP_COMPILATION_UNIT_ITERATOR_METHODS_SOURCE_FILE_METADATA_PATH)
        }
        LOAN_MANIFEST_PATH => Ok(CSHARP_COMPILATION_UNIT_LOAN_SOURCE_FILE_METADATA_PATH),
        other => Err(SyntaxBackendBoundaryError::new(
            "csharp_real_source_state_machine_parity.corpus_record.manifest_path",
            format!("unsupported C# real-source replay manifest path {other}"),
        )),
    }
}

fn csharp_real_source_case_slug(manifest_path: &str) -> String {
    manifest_path
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect()
}

pub fn csharp_real_source_state_machine_parity_report_path(manifest_path: &str) -> String {
    format!(
        "target/tct-syntax-state-machine-replay/csharp-{}-real-source-state-machine-parity-v1.json",
        csharp_real_source_case_slug(manifest_path)
    )
}

fn build_csharp_real_source_state_machine_parity_corpus_record(
    record: &CSharpGeneratedCodecCorpusSupportedRecord,
    corpus_gate: &SyntaxCSharpRealSourceGeneratedCodecCorpusGate,
) -> Result<SyntaxCSharpRealSourceStateMachineParityCorpusRecord, SyntaxBackendBoundaryError> {
    require_bool(
        "csharp_real_source_state_machine_parity.corpus_record.arena_ir_proof_used",
        record.arena_ir_proof_used,
        false,
    )?;
    require_bool(
        "csharp_real_source_state_machine_parity.corpus_record.stable_promotion_allowed",
        record.stable_promotion_allowed,
        false,
    )?;
    require_string(
        "csharp_real_source_state_machine_parity.corpus_record.parser_metadata_validation",
        &record.parser_metadata_validation,
        "checked",
    )?;

    let metadata_path = csharp_real_source_metadata_path_for_manifest(&record.manifest_path)?;
    let grammar = CSharpSyntaxMetadataAdapter::new(CSHARP_PAYLOAD_SEQUENCE_GENERATED_CODEC)
        .lower_metadata_file(Path::new(metadata_path))
        .map_err(|error| {
            SyntaxBackendBoundaryError::new(
                "csharp_real_source_state_machine_parity.corpus_record.metadata",
                format!(
                    "failed to lower C# syntax metadata for {}: {error}",
                    record.manifest_path
                ),
            )
        })?;
    let sidecar = lower_grammar_ir_to_structural_model(&grammar, csharp_syntax_token_layout())
        .map_err(|error| {
            SyntaxBackendBoundaryError::new(
                "csharp_real_source_state_machine_parity.corpus_record.structural_model",
                format!(
                    "failed to lower C# GrammarIR to StructuralModel for {}: {error}",
                    record.manifest_path
                ),
            )
        })?;
    validate_csharp_parity_sidecar(
        &sidecar,
        CSHARP_PAYLOAD_SEQUENCE_GENERATED_CODEC,
        Path::new(metadata_path),
    )?;
    require_string(
        "csharp_real_source_state_machine_parity.corpus_record.source_path",
        sidecar
            .input
            .provenance
            .source_path
            .as_deref()
            .unwrap_or_default(),
        &record.source_path,
    )?;
    require_string(
        "csharp_real_source_state_machine_parity.corpus_record.source_sha256",
        sidecar
            .input
            .provenance
            .source_sha256
            .as_deref()
            .unwrap_or_default(),
        &record.source_sha256,
    )?;

    let token_layout = projection_token_layout_summary(&sidecar.structural_model.token_ranges);
    validate_projection_token_layout(&token_layout)?;
    let core = CompiledSyntaxModel::from_sidecar_and_layout(&sidecar, &token_layout)?;
    let roles = core.token_roles();
    let raw_token_emission = emit_csharp_source_record_raw_tokens(
        record,
        &sidecar,
        &token_layout,
        build_csharp_structural_replay_token_stream_from_sidecar(&sidecar)?,
    )?;
    let structural_replay_token_stream = raw_token_emission.structural_replay_token_stream.clone();
    let parser = sidecar.input.provenance.parser.as_ref().ok_or_else(|| {
        SyntaxBackendBoundaryError::new(
            "csharp_real_source_state_machine_parity.corpus_record.parser",
            format!(
                "{} metadata must preserve parser provenance",
                record.manifest_path
            ),
        )
    })?;
    if record.payload_sequence_control_token_count.is_some() {
        require_usize(
            "csharp_real_source_state_machine_parity.corpus_record.retargeted_tct_token_count",
            record.tct_token_count,
            structural_replay_token_stream.raw_token_count,
        )?;
    }
    let payload_sequence_control_token_count = record
        .payload_sequence_control_token_count
        .unwrap_or(record.tct_token_count);
    let base_stream = raw_token_emission.raw_tokens.clone();
    let complete_state = core
        .strict_decode_complete(&base_stream.raw_token_ids)
        .map_err(|error| {
            SyntaxBackendBoundaryError::new(
                "csharp_real_source_state_machine_parity.corpus_record.complete_stream",
                format!(
                    "{} structural replay stream should replay: {}",
                    record.manifest_path, error.message
                ),
            )
        })?;
    let slug = csharp_real_source_case_slug(&record.manifest_path);
    let complete_stream = SyntaxReplayCaseResult::accepted(
        &format!("csharp_{slug}_structural_replay_complete_stream"),
        "csharp_generated_codec_structural_replay_sidecar_tokens",
        "complete",
        complete_state,
    );
    let root_prefix_indices = vec![0];
    let first_field_start_index = segment_index(&base_stream, "first field_start", |segment| {
        segment.event_kind == "field_start"
    })?;
    let first_field_prefix_indices = (0..=first_field_start_index).collect::<Vec<_>>();
    let first_payload_segment_index = first_variable_payload_segment_index(&base_stream)?;
    let complete_indices = (0..base_stream.raw_token_segments.len()).collect::<Vec<_>>();
    let state_cases = vec![
        generated_codec_state_case(
            &format!("csharp_{slug}_empty_prefix"),
            "csharp_generated_codec_structural_replay_empty_raw_prefix",
            &base_stream,
            &[],
            &core,
        )?,
        generated_codec_state_case(
            &format!("csharp_{slug}_after_root_node_start"),
            "csharp_generated_codec_structural_replay_segment_aligned_raw_prefix",
            &base_stream,
            &root_prefix_indices,
            &core,
        )?,
        generated_codec_state_case(
            &format!("csharp_{slug}_after_first_field_start"),
            "csharp_generated_codec_structural_replay_segment_aligned_raw_prefix",
            &base_stream,
            &first_field_prefix_indices,
            &core,
        )?,
        generated_codec_payload_range_state_case(
            &format!("csharp_{slug}_inside_first_terminal_payload_after_one_byte"),
            "csharp_generated_codec_structural_replay_partial_terminal_payload_prefix",
            &base_stream,
            first_payload_segment_index,
            1,
            &core,
        )?,
        generated_codec_state_case(
            &format!("csharp_{slug}_complete_stream"),
            "csharp_generated_codec_structural_replay_complete_raw_stream",
            &base_stream,
            &complete_indices,
            &core,
        )?,
    ];
    let partial_state = required_synthetic_state_case(
        &state_cases,
        &format!("csharp_{slug}_after_first_field_start"),
    )?;
    let complete_state =
        required_synthetic_state_case(&state_cases, &format!("csharp_{slug}_complete_stream"))?;
    let partial_stream = stream_from_state_case(&base_stream, partial_state)?;
    let trailing_pad_stream = stream_with_trailing_token(&base_stream, roles.pad_token);
    let non_pad_surplus_stream = stream_with_trailing_token(&base_stream, roles.node_start);
    let transition_next_index = first_field_start_index.checked_add(1).ok_or_else(|| {
        SyntaxBackendBoundaryError::new(
            "csharp_real_source_state_machine_parity.corpus_record.transition",
            "C# real-source corpus transition index overflowed",
        )
    })?;
    let transition_token = next_single_segment_token(&base_stream, transition_next_index)?;
    let after_first_segment_prefix_indices = (0..=transition_next_index).collect::<Vec<_>>();
    let chained_transition_next_index = transition_next_index.checked_add(1).ok_or_else(|| {
        SyntaxBackendBoundaryError::new(
            "csharp_real_source_state_machine_parity.corpus_record.transition",
            "C# real-source corpus chained transition index overflowed",
        )
    })?;
    let chained_transition_token =
        next_single_segment_token(&base_stream, chained_transition_next_index)?;
    let transition_cases = vec![
        generated_codec_transition_case(
            &format!("csharp_{slug}_first_field_next_segment_is_accepted"),
            &format!("csharp_{slug}_after_first_field_start"),
            GeneratedCodecTransitionProbe {
                base_stream: &base_stream,
                prefix_indices: &first_field_prefix_indices,
                next_segment_index: transition_next_index,
                candidate_raw_token_id: transition_token,
                expected_accepted: true,
            },
            &core,
        )?,
        generated_codec_transition_case(
            &format!("csharp_{slug}_after_first_segment_next_segment_is_accepted"),
            &format!("csharp_{slug}_first_field_next_segment_is_accepted"),
            GeneratedCodecTransitionProbe {
                base_stream: &base_stream,
                prefix_indices: &after_first_segment_prefix_indices,
                next_segment_index: chained_transition_next_index,
                candidate_raw_token_id: chained_transition_token,
                expected_accepted: true,
            },
            &core,
        )?,
        generated_codec_transition_case(
            &format!("csharp_{slug}_pad_token_candidate_is_rejected"),
            &format!("csharp_{slug}_after_first_field_start"),
            GeneratedCodecTransitionProbe {
                base_stream: &base_stream,
                prefix_indices: &first_field_prefix_indices,
                next_segment_index: transition_next_index,
                candidate_raw_token_id: roles.pad_token,
                expected_accepted: false,
            },
            &core,
        )?,
        generated_codec_transition_case(
            &format!("csharp_{slug}_after_first_segment_pad_token_is_rejected"),
            &format!("csharp_{slug}_first_field_next_segment_is_accepted"),
            GeneratedCodecTransitionProbe {
                base_stream: &base_stream,
                prefix_indices: &after_first_segment_prefix_indices,
                next_segment_index: chained_transition_next_index,
                candidate_raw_token_id: roles.pad_token,
                expected_accepted: false,
            },
            &core,
        )?,
    ];
    let strict_cases = vec![
        strict_decode_case(
            &format!("csharp_{slug}_strict_complete_stream_accepts_state_api_eof"),
            "csharp_generated_codec_structural_replay_sidecar_tokens",
            complete_state,
            &base_stream,
            StrictDecodeExpectation::accepted(),
            &core,
        )?,
        strict_decode_case(
            &format!("csharp_{slug}_strict_partial_prefix_rejects_eof"),
            "csharp_generated_codec_structural_replay_segment_aligned_raw_prefix",
            partial_state,
            &partial_stream,
            StrictDecodeExpectation::rejected("premature_eof"),
            &core,
        )?,
        strict_decode_case(
            &format!("csharp_{slug}_strict_trailing_pad_rejects_raw_stream"),
            "csharp_generated_codec_structural_replay_raw_tokens_with_trailing_pad",
            complete_state,
            &trailing_pad_stream,
            StrictDecodeExpectation::rejected("pad_token_in_raw_stream"),
            &core,
        )?,
    ];
    let recovery_cases = vec![
        recovery_decode_case(
            &format!("csharp_{slug}_recovery_complete_stream_returns_strict_success"),
            "csharp_generated_codec_structural_replay_sidecar_tokens",
            complete_state,
            "strict_success",
            &base_stream,
            &core,
        )?,
        recovery_decode_case(
            &format!("csharp_{slug}_recovery_trailing_pad_normalizes_after_state_api_complete"),
            "csharp_generated_codec_structural_replay_raw_tokens_with_trailing_pad",
            complete_state,
            "normalized_trailing_pad",
            &trailing_pad_stream,
            &core,
        )?,
        recovery_decode_case(
            &format!("csharp_{slug}_recovery_non_pad_surplus_remains_hard_error"),
            "csharp_generated_codec_structural_replay_raw_tokens_with_non_pad_surplus",
            complete_state,
            "hard_error",
            &non_pad_surplus_stream,
            &core,
        )?,
    ];

    let mut preserved_fields = corpus_gate
        .generated_codec_report_contract
        .stable_report_fields
        .clone();
    preserved_fields.extend(
        corpus_gate
            .generated_codec_report_contract
            .volatile_report_fields
            .iter()
            .cloned(),
    );
    preserved_fields.extend(record.replay_metadata_report_fields.iter().cloned());
    preserved_fields.sort();
    preserved_fields.dedup();
    let missing_replay_report_fields = CSHARP_REAL_SOURCE_REPLAY_METADATA_REPORT_FIELDS
        .iter()
        .filter(|field| {
            !preserved_fields
                .iter()
                .any(|preserved| preserved == **field)
        })
        .map(|field| (*field).to_string())
        .collect::<Vec<_>>();

    Ok(SyntaxCSharpRealSourceStateMachineParityCorpusRecord {
        manifest_path: record.manifest_path.clone(),
        source_path: record.source_path.clone(),
        source_sha256: record.source_sha256.clone(),
        metadata_path: metadata_path.to_string(),
        metadata_sha256: sidecar
            .input
            .provenance
            .metadata_sha256
            .clone()
            .unwrap_or_default(),
        parser_tool: parser.tool.clone(),
        parser_version: parser.version.clone(),
        parser_package: parser.package.clone(),
        parser_package_version: parser.package_version.clone(),
        parser_metadata_validation: record.parser_metadata_validation.clone(),
        parser_sidecar_validation_ready: record.parser_metadata_validation == "checked",
        generated_codec_report_path: record.report_path.clone(),
        generated_codec_report_command: record.report_command.clone(),
        generated_codec_report_fields_preserved: preserved_fields,
        replay_required_report_fields: CSHARP_REAL_SOURCE_REPLAY_METADATA_REPORT_FIELDS
            .iter()
            .map(|field| (*field).to_string())
            .collect(),
        missing_replay_report_fields,
        source_record_raw_token_emission_api_id: raw_token_emission.api_id,
        source_record_raw_token_emission_api_boundary: raw_token_emission.api_boundary,
        payload_sequence_control_token_count,
        payload_sequence_control_preserved: true,
        structural_replay_token_stream,
        complete_stream,
        state_cases,
        transition_cases,
        strict_cases,
        recovery_cases,
        prefix_state_api_ready: true,
        admissible_next_raw_tokens_ready: true,
        eof_state_api_ready: true,
        strict_recovery_decode_agreement_ready: true,
        consumed_token_accounting_ok: true,
        raw_token_segment_bridge_ready: true,
        structural_model_metadata_available: record.parser_metadata_validation == "checked",
        target_metadata_sidecar_only: true,
        arena_ir_used: false,
        source_program_validity_claim_allowed: record.source_program_validity_claim_allowed,
        type_correctness_claim_allowed: record.type_correctness_claim_allowed,
        stable_promotion_allowed: record.stable_promotion_allowed,
    })
}

pub fn build_csharp_real_source_state_machine_parity_report(
    corpus_gate: &SyntaxCSharpRealSourceGeneratedCodecCorpusGate,
) -> Result<SyntaxCSharpRealSourceStateMachineParityReport, SyntaxBackendBoundaryError> {
    let corpus_replay_records = corpus_gate
        .corpus_round_trip
        .supported_records
        .iter()
        .map(|record| {
            build_csharp_real_source_state_machine_parity_corpus_record(record, corpus_gate)
        })
        .collect::<Result<Vec<_>, _>>()?;
    require_positive_usize(
        "csharp_real_source_state_machine_parity.corpus_replay_record_count",
        corpus_replay_records.len(),
    )?;

    let first_route = corpus_replay_records
        .first()
        .map(|record| record.generated_codec_report_path.as_str())
        .ok_or_else(|| {
            SyntaxBackendBoundaryError::new(
                "csharp_real_source_state_machine_parity.corpus_replay_records",
                "C# real-source state-machine parity requires at least one checked corpus record",
            )
        })?;
    require_contains(
        "csharp_real_source_state_machine_parity.first_generated_codec_report_path",
        first_route,
        "target/",
    )?;

    let mut source_report_paths = Vec::with_capacity(corpus_replay_records.len());
    let mut supported_record_ids = Vec::with_capacity(corpus_replay_records.len());
    let mut generated_codec_report_paths = Vec::with_capacity(corpus_replay_records.len());
    let mut generated_codec_report_commands = Vec::with_capacity(corpus_replay_records.len());
    let mut generated_codec_report_fields_preserved = Vec::new();
    let mut raw_token_hashes = Vec::with_capacity(corpus_replay_records.len());
    let mut raw_token_count = 0usize;
    let mut payload_sequence_control_token_count = 0usize;
    let mut state_case_count = 0usize;
    let mut transition_case_count = 0usize;
    let mut strict_decode_case_count = 0usize;
    let mut recovery_decode_case_count = 0usize;
    let mut payload_range_state_count = 0usize;
    let mut eof_valid_state_count = 0usize;

    for record in &corpus_replay_records {
        source_report_paths.push(csharp_real_source_state_machine_parity_report_path(
            &record.manifest_path,
        ));
        supported_record_ids.push(record.manifest_path.clone());
        generated_codec_report_paths.push(record.generated_codec_report_path.clone());
        generated_codec_report_commands.push(record.generated_codec_report_command.clone());
        generated_codec_report_fields_preserved.extend(
            record
                .generated_codec_report_fields_preserved
                .iter()
                .cloned(),
        );
        raw_token_hashes.push(
            record
                .structural_replay_token_stream
                .raw_token_ids_sha256
                .clone(),
        );
        raw_token_count += record.structural_replay_token_stream.raw_token_count;
        payload_sequence_control_token_count += record.payload_sequence_control_token_count;
        state_case_count += record.state_cases.len();
        transition_case_count += record.transition_cases.len();
        strict_decode_case_count += record.strict_cases.len();
        recovery_decode_case_count += record.recovery_cases.len();
        payload_range_state_count += record
            .state_cases
            .iter()
            .filter(|case| !case.admissible_next_raw_token_ranges.is_empty())
            .count();
        eof_valid_state_count += record
            .state_cases
            .iter()
            .filter(|case| case.eof_valid)
            .count();
    }
    generated_codec_report_fields_preserved.sort();
    generated_codec_report_fields_preserved.dedup();

    let raw_token_ids_sha256 = json_sha256(&raw_token_hashes).map_err(|error| {
        SyntaxBackendBoundaryError::new(
            "csharp_real_source_state_machine_parity.raw_token_ids_sha256",
            format!("failed to hash C# replay record hashes: {error}"),
        )
    })?;

    Ok(SyntaxCSharpRealSourceStateMachineParityReport {
        artifact_kind: CSHARP_REAL_SOURCE_STATE_MACHINE_PARITY_ARTIFACT_KIND.to_string(),
        version: VERSION,
        tool: TOOL.to_string(),
        status: CSHARP_REAL_SOURCE_STATE_MACHINE_PARITY_STATUS.to_string(),
        target: CSHARP_STATE_MACHINE_PARITY_TARGET.to_string(),
        route: CSHARP_PAYLOAD_SEQUENCE_GENERATED_CODEC.to_string(),
        corpus_fixture_path: CSHARP_REAL_SOURCE_GENERATED_CODEC_CORPUS_FIXTURE_PATH.to_string(),
        source_report_paths,
        supported_record_ids,
        generated_codec_report_artifact_kind: corpus_gate
            .generated_codec_report_contract
            .report_artifact_kind
            .clone(),
        generated_codec_report_checked_in: corpus_gate
            .generated_codec_report_contract
            .report_checked_in,
        generated_codec_report_paths,
        generated_codec_report_commands,
        generated_codec_report_fields_preserved,
        replay_required_report_fields: CSHARP_REAL_SOURCE_REPLAY_REQUIRED_REPORT_FIELDS
            .iter()
            .map(|field| (*field).to_string())
            .collect(),
        missing_replay_report_fields: Vec::new(),
        source_record_raw_token_emission_api_id:
            CSHARP_SOURCE_RECORD_RAW_TOKEN_EMISSION_API_ID.to_string(),
        source_record_raw_token_emission_api_boundary:
            CSHARP_SOURCE_RECORD_RAW_TOKEN_EMISSION_API_BOUNDARY.to_string(),
        corpus_replay_record_count: corpus_replay_records.len(),
        corpus_replay_records,
        payload_sequence_control_token_count,
        payload_sequence_control_preserved: true,
        raw_token_count,
        raw_token_ids_sha256,
        raw_tokens_below_vocab_size: true,
        pad_token_absent: true,
        raw_state_machine_complete: true,
        state_case_count,
        transition_case_count,
        strict_decode_case_count,
        recovery_decode_case_count,
        payload_range_state_count,
        eof_valid_state_count,
        payload_sequence_control_payload_end_collides_with_structural_terminal: true,
        structural_replay_payload_end_matches_structural_payload_end: true,
        raw_token_segment_bridge_ready: true,
        pipeline: SyntaxCSharpRealSourceStateMachineParityPipeline {
            parser_backed_metadata_loaded: true,
            grammar_ir_lowered: true,
            structural_model_lowered: true,
            token_layout_compiled: true,
            raw_state_machine_compiled: true,
            raw_token_encode_ready: true,
            raw_state_machine_decode_ready: true,
            parser_sidecar_validation_ready: true,
            arena_ir_used: false,
            route_handoff_provenance_used: false,
            bpe_validity_authority_used: false,
        },
        structural_model_metadata_available: true,
        target_metadata_sidecar_only: true,
        arena_ir_used: false,
        source_program_validity_claim_allowed: false,
        type_correctness_claim_allowed: false,
        stable_promotion_allowed: false,
        next_ready_task: CSHARP_STATE_MACHINE_PARITY_NEXT_READY_TASK.to_string(),
        validation_commands: vec![
            "cargo test --test syntax_state_machine_replay_test".to_string(),
            "cargo test --test syntax_generated_codec_gate_test csharp_compilation_unit_source_file_syntax_generated_codec_compiles_through_compiler_route -- --exact".to_string(),
            "cargo test --test syntax_generated_codec_gate_test csharp_compilation_unit_logger_source_file_syntax_generated_codec_compiles_through_compiler_route -- --exact".to_string(),
            "cargo test --test syntax_generated_codec_gate_test csharp_compilation_unit_version_one_source_file_syntax_generated_codec_compiles_through_compiler_route -- --exact".to_string(),
            "cargo test --test syntax_generated_codec_gate_test csharp_compilation_unit_iterator_methods_source_file_syntax_generated_codec_compiles_through_compiler_route -- --exact".to_string(),
            "cargo test --test syntax_generated_codec_gate_test csharp_compilation_unit_loan_source_file_syntax_generated_codec_compiles_through_compiler_route -- --exact".to_string(),
            "cargo test --test syntax_structural_model_cross_target_equivalence_test".to_string(),
            "cargo test --test csharp_route_contract_test".to_string(),
            "cargo test --test syntax_backend_boundary_test syntax_backend_csharp_real_source_generated_codec_completion_handoff_consumes_completed_corpus -- --exact".to_string(),
            "python3 scripts/testing/test-quality-audit.py --changed-from origin/main".to_string(),
            "git diff --check".to_string(),
        ],
        claim_boundary:
            "syntax-level C# real-source generated-codec state and admissible-token API evidence over every checked Roslyn parser metadata record lowered through GrammarIR -> StructuralModel -> token layout. Each checked C# corpus record retargets authoritative tct_tokens to StructuralModel replay roles while preserving the payload-sequence stream as an explicit compatibility control. Parser/render facts remain sidecar checks; this is not arena-IR evidence, not BPE validity authority, and not stable-backend promotion."
                .to_string(),
        no_stable_claims: CSHARP_PARITY_NO_STABLE_CLAIMS
            .iter()
            .map(|claim| (*claim).to_string())
            .collect(),
    })
}

fn validate_state_api_matches_inputs(
    state_api_report: &SyntaxGeneratedCodecStateApiReport,
    projection_report: &SyntaxGoSourceInstanceProjectionReport,
    raw_tokens: &SourceInstanceProjectionRawTokenIds,
) -> Result<(), SyntaxBackendBoundaryError> {
    require_string(
        "strict_recovery_decode.state_api.candidate_grammar_id",
        &state_api_report.candidate_grammar_id,
        &projection_report.candidate_grammar_id,
    )?;
    require_string(
        "strict_recovery_decode.state_api.source_path",
        &state_api_report.source_path,
        &projection_report.source.path,
    )?;
    require_string(
        "strict_recovery_decode.state_api.source_sha256",
        &state_api_report.source_sha256,
        &projection_report.source.sha256,
    )?;
    require_string(
        "strict_recovery_decode.state_api.metadata_path",
        &state_api_report.metadata_path,
        &projection_report.metadata.path,
    )?;
    require_string(
        "strict_recovery_decode.state_api.metadata_sha256",
        &state_api_report.metadata_sha256,
        &projection_report.metadata.sha256,
    )?;
    require_string(
        "strict_recovery_decode.state_api.structural_model_id",
        &state_api_report.structural_model_id,
        &raw_tokens.structural_model_id,
    )?;
    if state_api_report.token_layout != raw_tokens.token_layout {
        return Err(SyntaxBackendBoundaryError::new(
            "strict_recovery_decode.state_api.token_layout",
            "state API token layout must match raw-token source-instance layout",
        ));
    }
    require_usize(
        "strict_recovery_decode.state_api.raw_token_count",
        state_api_report.raw_token_count,
        raw_tokens.raw_token_count,
    )?;
    require_string(
        "strict_recovery_decode.state_api.raw_token_ids_sha256",
        &state_api_report.raw_token_ids_sha256,
        &raw_tokens.raw_token_ids_sha256,
    )
}

fn required_state_case<'a>(
    state_api_report: &'a SyntaxGeneratedCodecStateApiReport,
    case_id: &str,
) -> Result<&'a SyntaxGeneratedCodecStateCase, SyntaxBackendBoundaryError> {
    state_api_report
        .state_cases
        .iter()
        .find(|case| case.case_id == case_id)
        .ok_or_else(|| {
            SyntaxBackendBoundaryError::new(
                "strict_recovery_decode.state_api.state_cases",
                format!("missing state API case {case_id}"),
            )
        })
}

fn required_synthetic_state_case<'a>(
    state_cases: &'a [SyntaxGeneratedCodecStateCase],
    case_id: &str,
) -> Result<&'a SyntaxGeneratedCodecStateCase, SyntaxBackendBoundaryError> {
    state_cases
        .iter()
        .find(|case| case.case_id == case_id)
        .ok_or_else(|| {
            SyntaxBackendBoundaryError::new(
                "csharp_state_machine_parity.state_cases",
                format!("missing C# state API case {case_id}"),
            )
        })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct StrictDecodeExpectation {
    accepted: bool,
    error_kind: Option<&'static str>,
}

impl StrictDecodeExpectation {
    fn accepted() -> Self {
        Self {
            accepted: true,
            error_kind: None,
        }
    }

    fn rejected(error_kind: &'static str) -> Self {
        Self {
            accepted: false,
            error_kind: Some(error_kind),
        }
    }
}

fn strict_decode_case(
    case_id: &str,
    input_kind: &str,
    state_case: &SyntaxGeneratedCodecStateCase,
    stream: &RawReplayInput,
    expectation: StrictDecodeExpectation,
    core: &impl RawSyntaxStateMachineCore,
) -> Result<SyntaxGeneratedCodecStrictDecodeCase, SyntaxBackendBoundaryError> {
    match core.strict_decode_complete(&stream.raw_token_ids) {
        Ok(state) => {
            if !expectation.accepted {
                return Err(SyntaxBackendBoundaryError::new(
                    "strict_recovery_decode.strict_case",
                    format!("{case_id} unexpectedly accepted"),
                ));
            }
            Ok(SyntaxGeneratedCodecStrictDecodeCase {
                case_id: case_id.to_string(),
                input_kind: input_kind.to_string(),
                state_api_case_id: state_case.case_id.clone(),
                expected_status: "complete".to_string(),
                observed_status: state.status.as_str().to_string(),
                accepted: true,
                consumed_raw_tokens: state.consumed_raw_tokens,
                total_raw_tokens: state.total_raw_tokens,
                eof_valid: state.eof_valid,
                stack_depth: state.stack_depth,
                active_rule: state.active_rule,
                active_field: state.active_field,
                error_kind: None,
                failing_token: None,
                diagnostic: format!(
                    "strict decode consumed raw syntax state-machine core case {}: {}",
                    state_case.case_id, state.diagnostic
                ),
            })
        }
        Err(error) => {
            if expectation.accepted {
                return Err(SyntaxBackendBoundaryError::new(
                    "strict_recovery_decode.strict_case",
                    format!("{case_id} unexpectedly failed: {}", error.message),
                ));
            }
            let expected_error_kind = expectation.error_kind.ok_or_else(|| {
                SyntaxBackendBoundaryError::new(
                    "strict_recovery_decode.strict_case.expected_error_kind",
                    format!("{case_id} must name the expected strict decode error"),
                )
            })?;
            if error.kind != expected_error_kind {
                return Err(SyntaxBackendBoundaryError::new(
                    "strict_recovery_decode.strict_case.error_kind",
                    format!(
                        "{case_id} expected {expected_error_kind}, got {}",
                        error.kind
                    ),
                ));
            }
            Ok(SyntaxGeneratedCodecStrictDecodeCase {
                case_id: case_id.to_string(),
                input_kind: input_kind.to_string(),
                state_api_case_id: state_case.case_id.clone(),
                expected_status: "error".to_string(),
                observed_status: "error".to_string(),
                accepted: false,
                consumed_raw_tokens: error.consumed_raw_tokens,
                total_raw_tokens: stream.raw_token_ids.len(),
                eof_valid: false,
                stack_depth: error.stack_depth,
                active_rule: error.active_rule,
                active_field: error.active_field,
                error_kind: Some(error.kind.to_string()),
                failing_token: error.failing_token,
                diagnostic: format!(
                    "strict decode consumed raw syntax state-machine core case {} and rejected {expected_error_kind}: {}",
                    state_case.case_id, error.message
                ),
            })
        }
    }
}

fn recovery_decode_case(
    case_id: &str,
    input_kind: &str,
    state_case: &SyntaxGeneratedCodecStateCase,
    expected_outcome: &str,
    stream: &RawReplayInput,
    core: &impl RawSyntaxStateMachineCore,
) -> Result<SyntaxGeneratedCodecRecoveryDecodeCase, SyntaxBackendBoundaryError> {
    let case = match core.strict_decode_complete(&stream.raw_token_ids) {
        Ok(state) => SyntaxGeneratedCodecRecoveryDecodeCase {
            case_id: case_id.to_string(),
            input_kind: input_kind.to_string(),
            state_api_case_id: state_case.case_id.clone(),
            expected_outcome: expected_outcome.to_string(),
            observed_outcome: "strict_success".to_string(),
            accepted: true,
            ignored_trailing_tokens: 0,
            consumed_raw_tokens: state.consumed_raw_tokens,
            total_raw_tokens: state.total_raw_tokens,
            normalized_raw_token_count: state.total_raw_tokens,
            eof_valid: state.eof_valid,
            error_kind: None,
            failing_token: None,
            diagnostic: format!(
                "recovery decode consumed raw syntax state-machine core case {} as strict success: {}",
                state_case.case_id, state.diagnostic
            ),
        },
        Err(error) => {
            let trailing_pad_count =
                trailing_pad_count(&stream.raw_token_ids, core.token_roles().pad_token);
            if trailing_pad_count > 0 && state_case.eof_valid {
                let normalized = stream_without_trailing_pad(stream, trailing_pad_count)?;
                if state_case.consumed_raw_tokens == normalized.raw_token_ids.len() {
                    match core.strict_decode_complete(&normalized.raw_token_ids) {
                        Ok(state) => SyntaxGeneratedCodecRecoveryDecodeCase {
                            case_id: case_id.to_string(),
                            input_kind: input_kind.to_string(),
                            state_api_case_id: state_case.case_id.clone(),
                            expected_outcome: expected_outcome.to_string(),
                            observed_outcome: "normalized_trailing_pad".to_string(),
                            accepted: true,
                            ignored_trailing_tokens: trailing_pad_count,
                            consumed_raw_tokens: state.consumed_raw_tokens,
                            total_raw_tokens: stream.raw_token_ids.len(),
                            normalized_raw_token_count: normalized.raw_token_ids.len(),
                            eof_valid: state.eof_valid,
                            error_kind: None,
                            failing_token: None,
                            diagnostic: format!(
                                "recovery decode normalized {trailing_pad_count} trailing PAD_TOKEN(s) after raw syntax state-machine core case {} confirmed complete EOF-valid stream",
                                state_case.case_id
                            ),
                        },
                        Err(normalized_error) => hard_recovery_decode_case(
                            case_id,
                            input_kind,
                            state_case,
                            expected_outcome,
                            stream,
                            normalized_error,
                            "normalized trailing PAD candidate remained invalid",
                        ),
                    }
                } else {
                    hard_recovery_decode_case(
                        case_id,
                        input_kind,
                        state_case,
                        expected_outcome,
                        stream,
                        error,
                        "trailing PAD appeared after a prefix not matched by the state API complete stream",
                    )
                }
            } else {
                hard_recovery_decode_case(
                    case_id,
                    input_kind,
                    state_case,
                    expected_outcome,
                    stream,
                    error,
                    "recovery keeps this strict decode failure hard",
                )
            }
        }
    };
    if case.observed_outcome != expected_outcome {
        return Err(SyntaxBackendBoundaryError::new(
            "strict_recovery_decode.recovery_case.observed_outcome",
            format!(
                "{case_id} expected {expected_outcome}, got {}",
                case.observed_outcome
            ),
        ));
    }
    Ok(case)
}

fn hard_recovery_decode_case(
    case_id: &str,
    input_kind: &str,
    state_case: &SyntaxGeneratedCodecStateCase,
    expected_outcome: &str,
    stream: &RawReplayInput,
    error: RawSyntaxError,
    reason: &str,
) -> SyntaxGeneratedCodecRecoveryDecodeCase {
    SyntaxGeneratedCodecRecoveryDecodeCase {
        case_id: case_id.to_string(),
        input_kind: input_kind.to_string(),
        state_api_case_id: state_case.case_id.clone(),
        expected_outcome: expected_outcome.to_string(),
        observed_outcome: "hard_error".to_string(),
        accepted: false,
        ignored_trailing_tokens: 0,
        consumed_raw_tokens: error.consumed_raw_tokens,
        total_raw_tokens: stream.raw_token_ids.len(),
        normalized_raw_token_count: stream.raw_token_ids.len(),
        eof_valid: false,
        error_kind: Some(error.kind.to_string()),
        failing_token: error.failing_token,
        diagnostic: format!(
            "{reason} after consuming state API case {}: {}",
            state_case.case_id, error.message
        ),
    }
}

fn stream_from_state_case(
    base_stream: &RawReplayInput,
    state_case: &SyntaxGeneratedCodecStateCase,
) -> Result<RawReplayInput, SyntaxBackendBoundaryError> {
    let indices = (0..state_case.prefix_segment_count).collect::<Vec<_>>();
    let stream = stream_from_segment_indices(base_stream, &indices)?;
    require_usize(
        "strict_recovery_decode.state_case.prefix_raw_token_count",
        stream.raw_token_ids.len(),
        state_case.prefix_raw_token_count,
    )?;
    Ok(stream)
}

fn stream_with_trailing_token(base_stream: &RawReplayInput, token: u32) -> RawReplayInput {
    let mut stream = base_stream.clone();
    stream.raw_token_ids.push(token);
    stream
}

fn stream_without_trailing_pad(
    stream: &RawReplayInput,
    trailing_pad_count: usize,
) -> Result<RawReplayInput, SyntaxBackendBoundaryError> {
    let normalized_len = stream
        .raw_token_ids
        .len()
        .checked_sub(trailing_pad_count)
        .ok_or_else(|| {
            SyntaxBackendBoundaryError::new(
                "strict_recovery_decode.recovery.trailing_pad",
                "trailing PAD count exceeded token stream length",
            )
        })?;
    let mut raw_token_segments = Vec::new();
    for segment in &stream.raw_token_segments {
        let end = segment
            .raw_token_offset
            .checked_add(segment.raw_token_count)
            .ok_or_else(|| {
                SyntaxBackendBoundaryError::new(
                    "strict_recovery_decode.recovery.trailing_pad",
                    "segment raw token range overflowed",
                )
            })?;
        if end <= normalized_len {
            raw_token_segments.push(segment.clone());
        } else {
            return Err(SyntaxBackendBoundaryError::new(
                "strict_recovery_decode.recovery.trailing_pad",
                "trailing PAD normalization cannot cut through a raw token segment",
            ));
        }
    }
    Ok(RawReplayInput {
        raw_token_ids: stream.raw_token_ids[..normalized_len].to_vec(),
        raw_token_segments,
    })
}

fn trailing_pad_count(raw_token_ids: &[u32], pad_token: u32) -> usize {
    raw_token_ids
        .iter()
        .rev()
        .take_while(|token| **token == pad_token)
        .count()
}

fn generated_codec_state_case(
    case_id: &str,
    prefix_kind: &str,
    base_stream: &RawReplayInput,
    prefix_indices: &[usize],
    core: &impl RawSyntaxStateMachineCore,
) -> Result<SyntaxGeneratedCodecStateCase, SyntaxBackendBoundaryError> {
    let prefix = stream_from_segment_indices(base_stream, prefix_indices)?;
    let state = core
        .state_after_raw_prefix(&prefix.raw_token_ids)
        .map_err(|error| {
            SyntaxBackendBoundaryError::new(
                "generated_codec_state_api.state_case",
                format!("{case_id} prefix should replay: {}", error.message),
            )
        })?;
    Ok(SyntaxGeneratedCodecStateCase {
        case_id: case_id.to_string(),
        prefix_kind: prefix_kind.to_string(),
        prefix_segment_count: prefix.raw_token_segments.len(),
        prefix_raw_token_count: prefix.raw_token_ids.len(),
        observed_status: state.status.as_str().to_string(),
        consumed_raw_tokens: state.consumed_raw_tokens,
        total_raw_tokens: state.total_raw_tokens,
        eof_valid: state.eof_valid,
        stack_depth: state.stack_depth,
        active_rule: state.active_rule,
        active_field: state.active_field,
        active_payload_kind: None,
        active_payload_node_kind: None,
        active_payload_byte_count: 0,
        payload_end_admissible: false,
        admissible_next_raw_token_ids: state.admissible.finite_raw_token_ids,
        admissible_next_raw_token_ranges: Vec::new(),
        diagnostic: state.diagnostic,
    })
}

fn generated_codec_payload_range_state_case(
    case_id: &str,
    prefix_kind: &str,
    base_stream: &RawReplayInput,
    terminal_segment_index: usize,
    payload_byte_count: usize,
    core: &impl RawSyntaxStateMachineCore,
) -> Result<SyntaxGeneratedCodecStateCase, SyntaxBackendBoundaryError> {
    let prefix_indices = (0..terminal_segment_index).collect::<Vec<_>>();
    let prefix_before_terminal = stream_from_segment_indices(base_stream, &prefix_indices)?;
    let pre_terminal_state = core
        .state_after_raw_prefix(&prefix_before_terminal.raw_token_ids)
        .map_err(|error| {
            SyntaxBackendBoundaryError::new(
                "generated_codec_state_api.payload_range_case",
                format!(
                    "{case_id} pre-terminal prefix should replay: {}",
                    error.message
                ),
            )
        })?;
    let roles = core.token_roles();
    if !pre_terminal_state
        .admissible
        .finite_raw_token_ids
        .contains(&roles.terminal)
    {
        return Err(SyntaxBackendBoundaryError::new(
            "generated_codec_state_api.payload_range_case",
            "pre-terminal state must admit a terminal payload marker",
        ));
    }
    let segment = base_stream
        .raw_token_segments
        .get(terminal_segment_index)
        .ok_or_else(|| {
            SyntaxBackendBoundaryError::new(
                "generated_codec_state_api.payload_range_case",
                format!("terminal segment index {terminal_segment_index} is out of range"),
            )
        })?;
    if segment.event_kind != "terminal" {
        return Err(SyntaxBackendBoundaryError::new(
            "generated_codec_state_api.payload_range_case",
            "payload-range case must reference a terminal segment",
        ));
    }
    let payload_kind =
        required_payload_segment_value(segment.payload_kind.as_deref(), "terminal.payload_kind")?;
    if payload_kind.starts_with("boolean_payload:") {
        return Err(SyntaxBackendBoundaryError::new(
            "generated_codec_state_api.payload_range_case",
            "boolean payloads use finite tokens and do not expose byte-range state",
        ));
    }
    let node_kind =
        required_payload_segment_value(segment.node_kind.as_deref(), "terminal.node_kind")?;
    let partial_raw_token_count = 1usize.checked_add(payload_byte_count).ok_or_else(|| {
        SyntaxBackendBoundaryError::new(
            "generated_codec_state_api.payload_range_case",
            "partial payload token count overflowed",
        )
    })?;
    if segment.raw_token_count <= partial_raw_token_count {
        return Err(SyntaxBackendBoundaryError::new(
            "generated_codec_state_api.payload_range_case",
            "payload-range case requires a terminal marker plus at least one payload byte before payload_end",
        ));
    }
    let terminal_token = base_stream
        .raw_token_ids
        .get(segment.raw_token_offset)
        .copied()
        .ok_or_else(|| {
            SyntaxBackendBoundaryError::new(
                "generated_codec_state_api.payload_range_case",
                "terminal segment raw token offset is out of range",
            )
        })?;
    if terminal_token != roles.terminal {
        return Err(SyntaxBackendBoundaryError::new(
            "generated_codec_state_api.payload_range_case",
            format!(
                "terminal segment expected token {}, got {terminal_token}",
                roles.terminal
            ),
        ));
    }
    let payload_range = payload_token_range_for_kind(payload_kind, roles)?;
    for offset in
        (segment.raw_token_offset + 1)..(segment.raw_token_offset + 1 + payload_byte_count)
    {
        let token = base_stream
            .raw_token_ids
            .get(offset)
            .copied()
            .ok_or_else(|| {
                SyntaxBackendBoundaryError::new(
                    "generated_codec_state_api.payload_range_case",
                    "partial payload byte offset is out of range",
                )
            })?;
        if token < payload_range.start || token >= payload_range.end_exclusive {
            return Err(SyntaxBackendBoundaryError::new(
                "generated_codec_state_api.payload_range_case",
                format!(
                    "payload byte token {token} is outside range {}..{}",
                    payload_range.start, payload_range.end_exclusive
                ),
            ));
        }
    }

    let prefix_raw_token_count =
        prefix_before_terminal.raw_token_ids.len() + partial_raw_token_count;
    let payload_state = core
        .state_after_raw_prefix(&base_stream.raw_token_ids[..prefix_raw_token_count])
        .map_err(|error| {
            SyntaxBackendBoundaryError::new(
                "generated_codec_state_api.payload_range_case",
                format!(
                    "{case_id} partial payload prefix should replay: {}",
                    error.message
                ),
            )
        })?;
    Ok(SyntaxGeneratedCodecStateCase {
        case_id: case_id.to_string(),
        prefix_kind: prefix_kind.to_string(),
        prefix_segment_count: prefix_before_terminal.raw_token_segments.len(),
        prefix_raw_token_count,
        observed_status: payload_state.status.as_str().to_string(),
        consumed_raw_tokens: payload_state.consumed_raw_tokens,
        total_raw_tokens: payload_state.total_raw_tokens,
        eof_valid: payload_state.eof_valid,
        stack_depth: payload_state.stack_depth,
        active_rule: payload_state.active_rule,
        active_field: payload_state.active_field,
        active_payload_kind: payload_state
            .active_payload_kind
            .or_else(|| Some(payload_kind.to_string())),
        active_payload_node_kind: payload_state
            .active_payload_node_kind
            .or_else(|| Some(node_kind.to_string())),
        active_payload_byte_count: payload_state.active_payload_byte_count,
        payload_end_admissible: payload_state.admissible.payload_end_admissible,
        admissible_next_raw_token_ids: payload_state.admissible.finite_raw_token_ids,
        admissible_next_raw_token_ranges: payload_state.admissible.raw_token_ranges,
        diagnostic: format!(
            "raw syntax state-machine core accepted partial terminal payload prefix for {payload_kind} after {payload_byte_count} payload byte(s): {}",
            payload_state.diagnostic
        ),
    })
}

#[derive(Debug, Clone, Copy)]
struct GeneratedCodecTransitionProbe<'a> {
    base_stream: &'a RawReplayInput,
    prefix_indices: &'a [usize],
    next_segment_index: usize,
    candidate_raw_token_id: u32,
    expected_accepted: bool,
}

fn generated_codec_transition_case(
    case_id: &str,
    from_state_case_id: &str,
    probe: GeneratedCodecTransitionProbe<'_>,
    core: &impl RawSyntaxStateMachineCore,
) -> Result<SyntaxGeneratedCodecTransitionCase, SyntaxBackendBoundaryError> {
    let prefix = stream_from_segment_indices(probe.base_stream, probe.prefix_indices)?;
    let _ = next_single_segment_token(probe.base_stream, probe.next_segment_index)?;
    let transition =
        core.candidate_raw_token_transition(&prefix.raw_token_ids, probe.candidate_raw_token_id);
    match transition.state {
        Some(state) => Ok(SyntaxGeneratedCodecTransitionCase {
            case_id: case_id.to_string(),
            from_state_case_id: from_state_case_id.to_string(),
            candidate_raw_token_id: probe.candidate_raw_token_id,
            expected_accepted: probe.expected_accepted,
            accepted: true,
            observed_status: state.status.as_str().to_string(),
            consumed_raw_tokens: state.consumed_raw_tokens,
            eof_valid: state.eof_valid,
            stack_depth: state.stack_depth,
            active_rule: state.active_rule,
            active_field: state.active_field,
            error_kind: None,
            diagnostic: state.diagnostic,
        }),
        None => {
            let error = transition.error.ok_or_else(|| {
                SyntaxBackendBoundaryError::new(
                    "generated_codec_state_api.transition",
                    "rejected transition must carry a raw syntax state-machine core error",
                )
            })?;
            Ok(SyntaxGeneratedCodecTransitionCase {
                case_id: case_id.to_string(),
                from_state_case_id: from_state_case_id.to_string(),
                candidate_raw_token_id: probe.candidate_raw_token_id,
                expected_accepted: probe.expected_accepted,
                accepted: false,
                observed_status: "error".to_string(),
                consumed_raw_tokens: error.consumed_raw_tokens,
                eof_valid: false,
                stack_depth: error.stack_depth,
                active_rule: error.active_rule,
                active_field: error.active_field,
                error_kind: Some(error.kind.to_string()),
                diagnostic: error.message,
            })
        }
    }
}

fn generated_codec_raw_prefix_transition_case(
    case_id: &str,
    from_state_case_id: &str,
    base_stream: &RawReplayInput,
    prefix_raw_token_count: usize,
    candidate_raw_token_id: u32,
    expected_accepted: bool,
    core: &impl RawSyntaxStateMachineCore,
) -> Result<SyntaxGeneratedCodecTransitionCase, SyntaxBackendBoundaryError> {
    if prefix_raw_token_count > base_stream.raw_token_ids.len() {
        return Err(SyntaxBackendBoundaryError::new(
            "generated_codec_state_api.raw_prefix_transition",
            "raw prefix transition length exceeds the source token stream",
        ));
    }
    let transition = core.candidate_raw_token_transition(
        &base_stream.raw_token_ids[..prefix_raw_token_count],
        candidate_raw_token_id,
    );
    match transition.state {
        Some(state) => Ok(SyntaxGeneratedCodecTransitionCase {
            case_id: case_id.to_string(),
            from_state_case_id: from_state_case_id.to_string(),
            candidate_raw_token_id,
            expected_accepted,
            accepted: true,
            observed_status: state.status.as_str().to_string(),
            consumed_raw_tokens: state.consumed_raw_tokens,
            eof_valid: state.eof_valid,
            stack_depth: state.stack_depth,
            active_rule: state.active_rule,
            active_field: state.active_field,
            error_kind: None,
            diagnostic: state.diagnostic,
        }),
        None => {
            let error = transition.error.ok_or_else(|| {
                SyntaxBackendBoundaryError::new(
                    "generated_codec_state_api.raw_prefix_transition",
                    "rejected transition must carry a raw syntax state-machine core error",
                )
            })?;
            Ok(SyntaxGeneratedCodecTransitionCase {
                case_id: case_id.to_string(),
                from_state_case_id: from_state_case_id.to_string(),
                candidate_raw_token_id,
                expected_accepted,
                accepted: false,
                observed_status: "error".to_string(),
                consumed_raw_tokens: error.consumed_raw_tokens,
                eof_valid: false,
                stack_depth: error.stack_depth,
                active_rule: error.active_rule,
                active_field: error.active_field,
                error_kind: Some(error.kind.to_string()),
                diagnostic: error.message,
            })
        }
    }
}

fn segment_index(
    base_stream: &RawReplayInput,
    label: &str,
    predicate: impl Fn(&ProjectionRawTokenSegment) -> bool,
) -> Result<usize, SyntaxBackendBoundaryError> {
    base_stream
        .raw_token_segments
        .iter()
        .position(predicate)
        .ok_or_else(|| {
            SyntaxBackendBoundaryError::new(
                "generated_codec_state_api.segment_index",
                format!("source-instance stream must contain {label}"),
            )
        })
}

fn first_variable_payload_segment_index(
    base_stream: &RawReplayInput,
) -> Result<usize, SyntaxBackendBoundaryError> {
    base_stream
        .raw_token_segments
        .iter()
        .position(|segment| {
            segment.event_kind == "terminal"
                && segment.raw_token_count > 2
                && !segment
                    .payload_kind
                    .as_deref()
                    .unwrap_or("")
                    .starts_with("boolean_payload:")
        })
        .ok_or_else(|| {
            SyntaxBackendBoundaryError::new(
                "generated_codec_state_api.payload_range_segment",
                "source-instance stream must contain a variable-length terminal payload segment",
            )
        })
}

fn first_variable_payload_segment(
    base_stream: &RawReplayInput,
) -> Result<&ProjectionRawTokenSegment, SyntaxBackendBoundaryError> {
    let index = first_variable_payload_segment_index(base_stream)?;
    base_stream.raw_token_segments.get(index).ok_or_else(|| {
        SyntaxBackendBoundaryError::new(
            "generated_codec_state_api.payload_range_segment",
            "variable-length terminal payload segment index is out of range",
        )
    })
}

fn payload_token_range_for_kind(
    payload_kind: &str,
    roles: &ReplayTokenRoles,
) -> Result<ProjectionTokenLayoutRangeSummary, SyntaxBackendBoundaryError> {
    if payload_kind.starts_with("integer_payload:") {
        Ok(roles.integer_range.clone())
    } else if payload_kind.starts_with("string_payload:")
        || payload_kind.starts_with("identifier_payload:")
    {
        Ok(roles.utf8_range.clone())
    } else {
        Err(SyntaxBackendBoundaryError::new(
            "generated_codec_state_api.payload_kind",
            format!("unsupported variable payload kind {payload_kind}"),
        ))
    }
}

fn next_single_segment_token(
    base_stream: &RawReplayInput,
    segment_index: usize,
) -> Result<u32, SyntaxBackendBoundaryError> {
    let segment = base_stream
        .raw_token_segments
        .get(segment_index)
        .ok_or_else(|| {
            SyntaxBackendBoundaryError::new(
                "generated_codec_state_api.transition.next_segment",
                format!("segment index {segment_index} is out of range"),
            )
        })?;
    if segment.raw_token_count != 1 {
        return Err(SyntaxBackendBoundaryError::new(
            "generated_codec_state_api.transition.next_segment",
            "single-token transition probe requires a single-token segment",
        ));
    }
    base_stream
        .raw_token_ids
        .get(segment.raw_token_offset)
        .copied()
        .ok_or_else(|| {
            SyntaxBackendBoundaryError::new(
                "generated_codec_state_api.transition.next_segment",
                "next segment raw token offset is out of range",
            )
        })
}

fn valid_prefix_cases(
    base_stream: &RawReplayInput,
    core: &impl RawSyntaxStateMachineCore,
) -> Result<Vec<SyntaxReplayCaseResult>, SyntaxBackendBoundaryError> {
    let root_prefix_indices = vec![0];
    let root_state_case = generated_codec_state_case(
        "after_root_node_start",
        "go_source_instance_segment_aligned_raw_prefix",
        base_stream,
        &root_prefix_indices,
        core,
    )?;

    let field_start_index = segment_index(base_stream, "first field_start", |segment| {
        segment.event_kind == "field_start"
    })?;
    let field_prefix_indices = (0..=field_start_index).collect::<Vec<_>>();
    let field_state_case = generated_codec_state_case(
        "after_first_field_start",
        "go_source_instance_segment_aligned_raw_prefix",
        base_stream,
        &field_prefix_indices,
        core,
    )?;

    Ok(vec![
        SyntaxReplayCaseResult::from_generated_codec_state_case(
            "valid_prefix_after_root_node_start",
            "go_source_instance_raw_token_prefix",
            "partial",
            root_state_case,
        ),
        SyntaxReplayCaseResult::from_generated_codec_state_case(
            "valid_prefix_after_first_field_start",
            "go_source_instance_raw_token_prefix",
            "partial",
            field_state_case,
        ),
    ])
}

fn invalid_mutation_cases(
    base_stream: &RawReplayInput,
    core: &impl RawSyntaxStateMachineCore,
) -> Result<Vec<SyntaxReplayCaseResult>, SyntaxBackendBoundaryError> {
    mutations::invalid_mutation_inputs(base_stream, core.token_roles())?
        .into_iter()
        .map(|case| {
            expect_replay_error(
                case.case_id,
                case.input_kind,
                case.expected_error,
                case.stream,
                core,
            )
        })
        .collect()
}

fn csharp_invalid_mutation_cases(
    base_stream: &RawReplayInput,
    core: &impl RawSyntaxStateMachineCore,
) -> Result<Vec<SyntaxReplayCaseResult>, SyntaxBackendBoundaryError> {
    let roles = core.token_roles();
    let mut wrong_structural = base_stream.clone();
    wrong_structural.raw_token_ids[0] = roles.node_end;

    let root_rule = base_stream
        .raw_token_segments
        .first()
        .and_then(|segment| segment.owner_rule.as_deref())
        .ok_or_else(|| {
            SyntaxBackendBoundaryError::new(
                "csharp_state_machine_parity.invalid_mutation.root_rule",
                "C# replay stream must start with an annotated root node_start",
            )
        })?;
    let root_end_index = base_stream
        .raw_token_segments
        .iter()
        .rposition(|segment| {
            segment.event_kind == "node_end" && segment.owner_rule.as_deref() == Some(root_rule)
        })
        .ok_or_else(|| {
            SyntaxBackendBoundaryError::new(
                "csharp_state_machine_parity.invalid_mutation.root_end",
                "C# replay stream must contain a root node_end",
            )
        })?;
    let missing_required = stream_from_segment_indices(base_stream, &[0, root_end_index])?;

    let mut invalid_payload = base_stream.clone();
    let payload_segment = first_variable_payload_segment(base_stream)?;
    invalid_payload.raw_token_ids[payload_segment.raw_token_offset + 1] = roles.node_start;

    let mut extra_trailing = base_stream.clone();
    extra_trailing.raw_token_ids.push(roles.node_start);

    let mut pad = base_stream.clone();
    pad.raw_token_ids[0] = roles.pad_token;

    let mut out_of_vocab = base_stream.clone();
    out_of_vocab.raw_token_ids[0] = roles.vocab_size;

    [
        (
            "csharp_wrong_structural_token_for_active_state",
            "mutated_csharp_raw_tokens",
            "unexpected_structural_token",
            wrong_structural,
        ),
        (
            "csharp_missing_required_root_field_before_node_end",
            "mutated_csharp_raw_segments",
            "missing_required_field",
            missing_required,
        ),
        (
            "csharp_invalid_payload_byte",
            "mutated_csharp_raw_tokens",
            "invalid_payload_token",
            invalid_payload,
        ),
        (
            "csharp_extra_trailing_token_after_root_close",
            "mutated_csharp_raw_tokens",
            "extra_trailing_token",
            extra_trailing,
        ),
        (
            "csharp_pad_token_inside_raw_content",
            "mutated_csharp_raw_tokens",
            "pad_token_in_raw_stream",
            pad,
        ),
        (
            "csharp_token_id_out_of_vocab",
            "mutated_csharp_raw_tokens",
            "token_id_out_of_vocab",
            out_of_vocab,
        ),
    ]
    .into_iter()
    .map(|(case_id, input_kind, expected_error, stream)| {
        expect_replay_error(case_id, input_kind, expected_error, stream, core)
    })
    .collect()
}

fn expect_replay_error(
    case_id: &str,
    input_kind: &str,
    expected_error: &str,
    stream: RawReplayInput,
    core: &impl RawSyntaxStateMachineCore,
) -> Result<SyntaxReplayCaseResult, SyntaxBackendBoundaryError> {
    match core.strict_decode_complete(&stream.raw_token_ids) {
        Ok(state) => Err(SyntaxBackendBoundaryError::new(
            "replay.invalid_mutation",
            format!(
                "{case_id} unexpectedly replayed with status {}",
                state.status.as_str()
            ),
        )),
        Err(error) => {
            if error.kind != expected_error {
                return Err(SyntaxBackendBoundaryError::new(
                    "replay.invalid_mutation.error_kind",
                    format!("{case_id} expected {expected_error}, got {}", error.kind),
                ));
            }
            Ok(SyntaxReplayCaseResult::rejected(
                case_id,
                input_kind,
                expected_error,
                error,
                stream.raw_token_ids.len(),
            ))
        }
    }
}

fn rejected_structural_candidate_for_prefix(
    case_id: &str,
    base_stream: &RawReplayInput,
    prefix_indices: &[usize],
    preferred_candidate: u32,
    core: &impl RawSyntaxStateMachineCore,
) -> Result<u32, SyntaxBackendBoundaryError> {
    let prefix = stream_from_segment_indices(base_stream, prefix_indices)?;
    let state = core
        .state_after_raw_prefix(&prefix.raw_token_ids)
        .map_err(|error| {
            SyntaxBackendBoundaryError::new(
                "generated_codec_state_api.transition.rejected_candidate",
                format!(
                    "{case_id} prefix should be a valid state: {}",
                    error.message
                ),
            )
        })?;
    let roles = core.token_roles();
    let mut candidates = vec![
        preferred_candidate,
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
    ];
    candidates.extend(roles.node_start_alternative_tokens.values().copied());
    candidates.extend(roles.structural_range.start..roles.structural_range.end_exclusive);

    let mut seen = BTreeSet::new();
    for candidate in candidates {
        if !seen.insert(candidate) || candidate == roles.pad_token || candidate >= roles.vocab_size
        {
            continue;
        }
        let finite_admissible = state.admissible.finite_raw_token_ids.contains(&candidate);
        let range_admissible = state
            .admissible
            .raw_token_ranges
            .iter()
            .any(|range| candidate >= range.start && candidate < range.end_exclusive);
        if !finite_admissible && !range_admissible {
            return Ok(candidate);
        }
    }

    Err(SyntaxBackendBoundaryError::new(
        "generated_codec_state_api.transition.rejected_candidate",
        format!("{case_id} could not find a non-admissible structural token candidate"),
    ))
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RawReplayInput {
    raw_token_ids: Vec<u32>,
    raw_token_segments: Vec<ProjectionRawTokenSegment>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ModelFacts {
    multi_alternative_nonterminal_fields: BTreeSet<(String, String)>,
}

impl ModelFacts {
    fn from_sidecar(sidecar: &GrammarStructuralModelSidecar) -> Self {
        let multi_alternative_nonterminal_fields = sidecar
            .structural_model
            .finite_domains
            .iter()
            .filter(|domain| domain.alternatives.len() > 1)
            .filter(|domain| {
                domain.alternatives.iter().any(|alternative| {
                    alternative.terminal == crate::grammar_ir::GrammarTerminalKind::Nonterminal
                })
            })
            .map(|domain| (domain.owner_rule.clone(), domain.owner_field.clone()))
            .collect();
        Self {
            multi_alternative_nonterminal_fields,
        }
    }
}

fn go_ast_declaration_presence_replay_grammar(
    mut grammar: GrammarIr,
) -> Result<GrammarIr, SyntaxBackendBoundaryError> {
    let file_rule_index = grammar
        .rules
        .iter()
        .position(|rule| rule.name == "File")
        .ok_or_else(|| {
            SyntaxBackendBoundaryError::new(
                "go_ast_declaration_presence.inventory.rule",
                "checked Go AST inventory grammar must contain File",
            )
        })?;
    let file_rule = grammar.rules.remove(file_rule_index);
    grammar.rules.insert(0, file_rule);
    grammar.grammar_id = format!("{}_declaration_presence_replay_v1", grammar.grammar_id);
    Ok(grammar)
}

fn validate_go_ast_declaration_presence_inventory(
    grammar: &GrammarIr,
) -> Result<(), SyntaxBackendBoundaryError> {
    for (rule, field, node_kind, terminal) in [
        ("File", "Decls", "GenDecl", GrammarTerminalKind::Nonterminal),
        (
            "GenDecl",
            "Specs",
            "TypeSpec",
            GrammarTerminalKind::Nonterminal,
        ),
        (
            "TypeSpec",
            "Type",
            "Ident",
            GrammarTerminalKind::Nonterminal,
        ),
        (
            "GenDecl",
            "Lparen",
            "go_syntax_presence_gen_decl_group.PRESENT",
            GrammarTerminalKind::BooleanPayload,
        ),
        (
            "GenDecl",
            "Rparen",
            "go_syntax_presence_gen_decl_group.PRESENT",
            GrammarTerminalKind::BooleanPayload,
        ),
        (
            "TypeSpec",
            "Assign",
            "go_syntax_presence_type_spec_alias.PRESENT",
            GrammarTerminalKind::BooleanPayload,
        ),
        (
            "FuncType",
            "Func",
            "go_syntax_presence_func_type_func.PRESENT",
            GrammarTerminalKind::BooleanPayload,
        ),
        (
            "CallExpr",
            "Ellipsis",
            "go_syntax_presence_call_expr_ellipsis.PRESENT",
            GrammarTerminalKind::BooleanPayload,
        ),
    ] {
        require_go_ast_inventory_alternative(grammar, rule, field, node_kind, terminal)?;
    }
    for (rule, field) in [
        ("GenDecl", "Lparen"),
        ("GenDecl", "Rparen"),
        ("TypeSpec", "Assign"),
        ("FuncType", "Func"),
        ("CallExpr", "Ellipsis"),
    ] {
        let grammar_rule = grammar.rule(rule).ok_or_else(|| {
            SyntaxBackendBoundaryError::new(
                "go_ast_declaration_presence.inventory.rule",
                format!("checked Go AST inventory grammar must contain {rule}"),
            )
        })?;
        let grammar_field = grammar_rule
            .fields
            .iter()
            .find(|candidate| candidate.name == field)
            .ok_or_else(|| {
                SyntaxBackendBoundaryError::new(
                    "go_ast_declaration_presence.inventory.field",
                    format!("checked Go AST inventory grammar must contain {rule}.{field}"),
                )
            })?;
        if grammar_field.required || grammar_field.alternatives.len() != 2 {
            return Err(SyntaxBackendBoundaryError::new(
                "go_ast_declaration_presence.inventory.field",
                format!(
                    "checked Go AST inventory field {rule}.{field} must remain an optional two-alternative syntax-presence decision"
                ),
            ));
        }
    }
    Ok(())
}

fn require_go_ast_inventory_alternative(
    grammar: &GrammarIr,
    rule: &str,
    field: &str,
    node_kind: &str,
    terminal: GrammarTerminalKind,
) -> Result<(), SyntaxBackendBoundaryError> {
    let grammar_rule = grammar.rule(rule).ok_or_else(|| {
        SyntaxBackendBoundaryError::new(
            "go_ast_declaration_presence.inventory.rule",
            format!("checked Go AST inventory grammar must contain {rule}"),
        )
    })?;
    let grammar_field = grammar_rule
        .fields
        .iter()
        .find(|candidate| candidate.name == field)
        .ok_or_else(|| {
            SyntaxBackendBoundaryError::new(
                "go_ast_declaration_presence.inventory.field",
                format!("checked Go AST inventory grammar must contain {rule}.{field}"),
            )
        })?;
    if grammar_field
        .alternatives
        .iter()
        .any(|alternative| alternative.node_kind == node_kind && alternative.terminal == terminal)
    {
        Ok(())
    } else {
        Err(SyntaxBackendBoundaryError::new(
            "go_ast_declaration_presence.inventory.alternative",
            format!("checked Go AST inventory grammar must contain {rule}.{field} -> {node_kind}"),
        ))
    }
}

fn validate_go_ast_declaration_presence_sidecar(
    sidecar: &GrammarStructuralModelSidecar,
) -> Result<(), SyntaxBackendBoundaryError> {
    require_string(
        "go_ast_declaration_presence.sidecar.root_rule",
        sidecar
            .structural_model
            .records
            .first()
            .map(|record| record.grammar_rule.as_str())
            .unwrap_or_default(),
        "File",
    )?;
    for (rule, field) in [
        ("GenDecl", "Lparen"),
        ("GenDecl", "Rparen"),
        ("TypeSpec", "Assign"),
        ("FuncType", "Func"),
        ("CallExpr", "Ellipsis"),
    ] {
        let domain = finite_domain_by_field(sidecar, rule, field)?;
        require_usize(
            "go_ast_declaration_presence.sidecar.presence_domain.alternatives",
            domain.alternatives.len(),
            2,
        )?;
        if !domain
            .alternatives
            .iter()
            .all(|alternative| alternative.terminal == GrammarTerminalKind::BooleanPayload)
        {
            return Err(SyntaxBackendBoundaryError::new(
                "go_ast_declaration_presence.sidecar.presence_domain",
                format!("{rule}.{field} must lower as boolean payload alternatives"),
            ));
        }
        if !sidecar
            .structural_model
            .optionals
            .iter()
            .any(|optional| optional.owner_rule == rule && optional.field == field)
        {
            return Err(SyntaxBackendBoundaryError::new(
                "go_ast_declaration_presence.sidecar.optional",
                format!("{rule}.{field} must remain optional in the StructuralModel"),
            ));
        }
    }
    Ok(())
}

fn finite_domain_by_field<'a>(
    sidecar: &'a GrammarStructuralModelSidecar,
    owner_rule: &str,
    owner_field: &str,
) -> Result<&'a crate::grammar_ir_structural_model::GrammarFiniteDomain, SyntaxBackendBoundaryError>
{
    sidecar
        .structural_model
        .finite_domains
        .iter()
        .find(|domain| domain.owner_rule == owner_rule && domain.owner_field == owner_field)
        .ok_or_else(|| {
            SyntaxBackendBoundaryError::new(
                "go_ast_declaration_presence.sidecar.presence_domain",
                format!("missing finite domain for {owner_rule}.{owner_field}"),
            )
        })
}

fn csharp_marker_presence_decisions(
    sidecar: &GrammarStructuralModelSidecar,
    base_stream: &RawReplayInput,
    decoded_events: &[RawSyntaxDecodedEvent],
    core: &CompiledSyntaxModel,
    roles: &ReplayTokenRoles,
) -> Result<Vec<CSharpMarkerPresenceDecision>, SyntaxBackendBoundaryError> {
    if !sidecar
        .structural_model
        .records
        .iter()
        .any(|record| record.grammar_rule == "NullableTypeSyntax")
    {
        return Ok(Vec::new());
    }
    if csharp_nullable_marker_generated_test_markers(&sidecar.input.grammar_id).is_err() {
        return Ok(Vec::new());
    }
    Ok(vec![csharp_nullable_marker_presence_decision(
        sidecar,
        base_stream,
        decoded_events,
        core,
        roles,
    )?])
}

fn csharp_nullable_marker_presence_decision(
    sidecar: &GrammarStructuralModelSidecar,
    base_stream: &RawReplayInput,
    decoded_events: &[RawSyntaxDecodedEvent],
    core: &CompiledSyntaxModel,
    roles: &ReplayTokenRoles,
) -> Result<CSharpMarkerPresenceDecision, SyntaxBackendBoundaryError> {
    let owner_rule = "NullableTypeSyntax";
    let owner_field = "QuestionToken";
    let selected_node_kind = "nullable_marker";
    let generated_marker_test_marker =
        csharp_nullable_marker_generated_test_marker(&sidecar.input.grammar_id)?;
    let generated_test_markers =
        csharp_nullable_marker_generated_test_markers(&sidecar.input.grammar_id)?;
    let domain = finite_domain_by_field(sidecar, owner_rule, owner_field)?;
    if !domain.alternatives.iter().any(|alternative| {
        alternative.node_kind == selected_node_kind
            && alternative.terminal == GrammarTerminalKind::StringPayload
    }) {
        return Err(SyntaxBackendBoundaryError::new(
            "csharp_state_machine_parity.marker_presence.domain",
            "NullableTypeSyntax.QuestionToken must lower nullable_marker as a string payload",
        ));
    }
    let payload_kind = sidecar
        .structural_model
        .payload_leaves
        .iter()
        .find(|payload| {
            payload.owner_rule == owner_rule
                && payload.field == owner_field
                && payload.node_kind == selected_node_kind
        })
        .map(|payload| payload.payload_kind.clone())
        .ok_or_else(|| {
            SyntaxBackendBoundaryError::new(
                "csharp_state_machine_parity.marker_presence.payload",
                "missing payload leaf for NullableTypeSyntax.QuestionToken nullable_marker",
            )
        })?;
    let segment = base_stream
        .raw_token_segments
        .iter()
        .find(|segment| {
            segment.event_kind == "terminal"
                && segment.owner_rule.as_deref() == Some(owner_rule)
                && segment.owner_field.as_deref() == Some(owner_field)
                && segment.node_kind.as_deref() == Some(selected_node_kind)
        })
        .ok_or_else(|| {
            SyntaxBackendBoundaryError::new(
                "csharp_state_machine_parity.marker_presence.segment",
                "missing terminal segment for NullableTypeSyntax.QuestionToken",
            )
        })?;
    if segment.raw_token_count < 3 {
        return Err(SyntaxBackendBoundaryError::new(
            "csharp_state_machine_parity.marker_presence.segment",
            "nullable marker terminal segment must include terminal, payload byte, and payload_end",
        ));
    }
    let terminal_raw_token_offset = segment.raw_token_offset;
    let first_payload_raw_token_offset = terminal_raw_token_offset + 1;
    let payload_end_raw_token_offset = terminal_raw_token_offset + segment.raw_token_count - 1;
    require_usize(
        "csharp_state_machine_parity.marker_presence.terminal_raw_token",
        *base_stream
            .raw_token_ids
            .get(terminal_raw_token_offset)
            .ok_or_else(|| {
                SyntaxBackendBoundaryError::new(
                    "csharp_state_machine_parity.marker_presence.token",
                    "terminal offset is outside the raw token stream",
                )
            })? as usize,
        roles.terminal as usize,
    )?;
    let first_payload_raw_token_id = *base_stream
        .raw_token_ids
        .get(first_payload_raw_token_offset)
        .ok_or_else(|| {
            SyntaxBackendBoundaryError::new(
                "csharp_state_machine_parity.marker_presence.token",
                "first payload offset is outside the raw token stream",
            )
        })?;
    if first_payload_raw_token_id != roles.utf8_range.start + u32::from(b'?') {
        return Err(SyntaxBackendBoundaryError::new(
            "csharp_state_machine_parity.marker_presence.token",
            "nullable marker replay should materialize '?' as the first UTF-8 payload byte",
        ));
    }
    require_usize(
        "csharp_state_machine_parity.marker_presence.payload_end_raw_token",
        *base_stream
            .raw_token_ids
            .get(payload_end_raw_token_offset)
            .ok_or_else(|| {
                SyntaxBackendBoundaryError::new(
                    "csharp_state_machine_parity.marker_presence.token",
                    "payload_end offset is outside the raw token stream",
                )
            })? as usize,
        roles.payload_end as usize,
    )?;
    let state_after_terminal = core
        .state_after_raw_prefix(&base_stream.raw_token_ids[..=terminal_raw_token_offset])
        .map_err(|error| {
            SyntaxBackendBoundaryError::new(
                "csharp_state_machine_parity.marker_presence.state_after_terminal",
                format!(
                    "state after nullable marker terminal should be valid: {}",
                    error.message
                ),
            )
        })?;
    let state_after_payload_byte = core
        .state_after_raw_prefix(&base_stream.raw_token_ids[..=first_payload_raw_token_offset])
        .map_err(|error| {
            SyntaxBackendBoundaryError::new(
                "csharp_state_machine_parity.marker_presence.state_after_payload_byte",
                format!(
                    "state after nullable marker payload byte should be valid: {}",
                    error.message
                ),
            )
        })?;
    let generated_codec_marker_evidence_preserved =
        generated_test_markers.contains(&generated_marker_test_marker);
    Ok(CSharpMarkerPresenceDecision {
        owner_rule: owner_rule.to_string(),
        owner_field: owner_field.to_string(),
        selected_node_kind: selected_node_kind.to_string(),
        payload_kind,
        terminal_raw_token_offset,
        first_payload_raw_token_offset,
        payload_end_raw_token_offset,
        first_payload_raw_token_id,
        terminal_event_decoded: decoded_events.iter().any(|event| {
            event.raw_token_offset == terminal_raw_token_offset
                && event.event_kind == RawSyntaxDecodedEventKind::TerminalStart
                && event.rule.as_deref() == Some(owner_rule)
                && event.field.as_deref() == Some(owner_field)
        }),
        string_payload_byte_decoded: decoded_events.iter().any(|event| {
            event.raw_token_offset == first_payload_raw_token_offset
                && event.event_kind == RawSyntaxDecodedEventKind::PayloadToken
                && event.rule.as_deref() == Some(owner_rule)
                && event.field.as_deref() == Some(owner_field)
                && event.raw_token_id == first_payload_raw_token_id
        }),
        payload_end_decoded: decoded_events.iter().any(|event| {
            event.raw_token_offset == payload_end_raw_token_offset
                && event.event_kind == RawSyntaxDecodedEventKind::PayloadEnd
                && event.rule.as_deref() == Some(owner_rule)
                && event.field.as_deref() == Some(owner_field)
        }),
        state_after_terminal_admits_utf8_payload_range: state_after_terminal
            .admissible
            .raw_token_ranges
            .iter()
            .any(|range| range.role == roles.utf8_range.role),
        state_after_payload_byte_admits_payload_end: state_after_payload_byte
            .admissible
            .payload_end_admissible
            && state_after_payload_byte
                .admissible
                .finite_raw_token_ids
                .contains(&roles.payload_end),
        generated_marker_test_marker: generated_marker_test_marker.to_string(),
        generated_codec_marker_evidence_preserved,
    })
}

fn csharp_nullable_marker_generated_test_marker(
    candidate_grammar_id: &str,
) -> Result<&'static str, SyntaxBackendBoundaryError> {
    match candidate_grammar_id {
        "csharp_local_nullable_return_type_syntax_v0" => {
            Ok(CSHARP_NULLABLE_MARKER_GENERATED_TEST_MARKER)
        }
        "csharp_local_nullable_declaration_syntax_v0" => {
            Ok(CSHARP_LOCAL_NULLABLE_DECLARATION_MARKER_GENERATED_TEST_MARKER)
        }
        other => Err(SyntaxBackendBoundaryError::new(
            "csharp_state_machine_parity.marker_presence.generated_marker_test_marker",
            format!("missing nullable-marker generated test marker mapping for {other}"),
        )),
    }
}

fn csharp_nullable_marker_generated_test_markers(
    candidate_grammar_id: &str,
) -> Result<&'static [&'static str], SyntaxBackendBoundaryError> {
    match candidate_grammar_id {
        "csharp_local_nullable_return_type_syntax_v0" => {
            Ok(CSHARP_LOCAL_NULLABLE_RETURN_TYPE_GENERATED_TEST_MARKERS)
        }
        "csharp_local_nullable_declaration_syntax_v0" => {
            Ok(CSHARP_LOCAL_NULLABLE_DECLARATION_GENERATED_TEST_MARKERS)
        }
        other => Err(SyntaxBackendBoundaryError::new(
            "csharp_state_machine_parity.marker_presence.generated_marker_test_markers",
            format!("missing nullable-marker generated test marker list for {other}"),
        )),
    }
}

fn add_csharp_nullable_marker_state_api_cases(
    state_cases: &mut Vec<SyntaxGeneratedCodecStateCase>,
    transition_cases: &mut Vec<SyntaxGeneratedCodecTransitionCase>,
    base_stream: &RawReplayInput,
    decisions: &[CSharpMarkerPresenceDecision],
    core: &impl RawSyntaxStateMachineCore,
    roles: &ReplayTokenRoles,
) -> Result<(), SyntaxBackendBoundaryError> {
    let Some(decision) = decisions.iter().find(|decision| {
        decision.owner_rule == "NullableTypeSyntax" && decision.owner_field == "QuestionToken"
    }) else {
        return Ok(());
    };
    let marker_segment_index = segment_index(
        base_stream,
        "NullableTypeSyntax.QuestionToken terminal segment",
        |segment| {
            segment.event_kind == "terminal"
                && segment.owner_rule.as_deref() == Some("NullableTypeSyntax")
                && segment.owner_field.as_deref() == Some("QuestionToken")
                && segment.node_kind.as_deref() == Some("nullable_marker")
        },
    )?;

    state_cases.push(generated_codec_payload_range_state_case(
        "csharp_nullable_marker_after_terminal",
        "csharp_nullable_marker_terminal_raw_prefix",
        base_stream,
        marker_segment_index,
        0,
        core,
    )?);
    state_cases.push(generated_codec_payload_range_state_case(
        "csharp_nullable_marker_after_payload_byte",
        "csharp_nullable_marker_payload_byte_raw_prefix",
        base_stream,
        marker_segment_index,
        1,
        core,
    )?);

    transition_cases.extend(csharp_nullable_marker_transition_cases(
        base_stream,
        decision,
        core,
        roles,
    )?);
    Ok(())
}

fn csharp_nullable_marker_transition_cases(
    base_stream: &RawReplayInput,
    decision: &CSharpMarkerPresenceDecision,
    core: &impl RawSyntaxStateMachineCore,
    roles: &ReplayTokenRoles,
) -> Result<Vec<SyntaxGeneratedCodecTransitionCase>, SyntaxBackendBoundaryError> {
    let after_terminal_prefix_raw_token_count = decision.first_payload_raw_token_offset;
    let after_payload_byte_prefix_raw_token_count = decision.payload_end_raw_token_offset;
    Ok(vec![
        generated_codec_raw_prefix_transition_case(
            "csharp_nullable_marker_question_payload_byte_is_accepted",
            "csharp_nullable_marker_after_terminal",
            base_stream,
            after_terminal_prefix_raw_token_count,
            decision.first_payload_raw_token_id,
            true,
            core,
        )?,
        generated_codec_raw_prefix_transition_case(
            "csharp_nullable_marker_node_start_payload_byte_is_rejected",
            "csharp_nullable_marker_after_terminal",
            base_stream,
            after_terminal_prefix_raw_token_count,
            roles.node_start,
            false,
            core,
        )?,
        generated_codec_raw_prefix_transition_case(
            "csharp_nullable_marker_payload_end_is_accepted",
            "csharp_nullable_marker_after_payload_byte",
            base_stream,
            after_payload_byte_prefix_raw_token_count,
            roles.payload_end,
            true,
            core,
        )?,
        generated_codec_raw_prefix_transition_case(
            "csharp_nullable_marker_after_payload_byte_pad_token_is_rejected",
            "csharp_nullable_marker_after_payload_byte",
            base_stream,
            after_payload_byte_prefix_raw_token_count,
            roles.pad_token,
            false,
            core,
        )?,
    ])
}

fn csharp_marker_presence_invalid_payload_mutations(
    base_stream: &RawReplayInput,
    decisions: &[CSharpMarkerPresenceDecision],
    core: &impl RawSyntaxStateMachineCore,
    roles: &ReplayTokenRoles,
) -> Result<Vec<SyntaxReplayCaseResult>, SyntaxBackendBoundaryError> {
    decisions
        .iter()
        .map(|decision| {
            let mut invalid_payload = base_stream.clone();
            invalid_payload.raw_token_ids[decision.first_payload_raw_token_offset] =
                roles.node_start;
            expect_replay_error(
                "csharp_nullable_marker_invalid_payload_is_rejected",
                "csharp_nullable_marker_raw_tokens_with_invalid_payload",
                "invalid_payload_token",
                invalid_payload,
                core,
            )
        })
        .collect()
}

fn go_ast_presence_present_node_kind(
    owner_rule: &str,
    owner_field: &str,
) -> Result<&'static str, SyntaxBackendBoundaryError> {
    match (owner_rule, owner_field) {
        ("GenDecl", "Lparen") | ("GenDecl", "Rparen") => {
            Ok("go_syntax_presence_gen_decl_group.PRESENT")
        }
        ("TypeSpec", "Assign") => Ok("go_syntax_presence_type_spec_alias.PRESENT"),
        ("FuncType", "Func") => Ok("go_syntax_presence_func_type_func.PRESENT"),
        ("CallExpr", "Ellipsis") => Ok("go_syntax_presence_call_expr_ellipsis.PRESENT"),
        _ => Err(SyntaxBackendBoundaryError::new(
            "go_ast_declaration_presence.decision",
            format!("unsupported Go AST declaration presence field {owner_rule}.{owner_field}"),
        )),
    }
}

fn go_ast_declaration_presence_decision(
    sidecar: &GrammarStructuralModelSidecar,
    base_stream: &RawReplayInput,
    decoded_events: &[RawSyntaxDecodedEvent],
    core: &CompiledSyntaxModel,
    roles: &ReplayTokenRoles,
    owner_rule: &str,
    owner_field: &str,
) -> Result<GoAstDeclarationPresenceDecision, SyntaxBackendBoundaryError> {
    let selected_node_kind = go_ast_presence_present_node_kind(owner_rule, owner_field)?;
    let domain = finite_domain_by_field(sidecar, owner_rule, owner_field)?;
    if !domain
        .alternatives
        .iter()
        .any(|alternative| alternative.node_kind == selected_node_kind)
    {
        return Err(SyntaxBackendBoundaryError::new(
            "go_ast_declaration_presence.decision.domain",
            format!("missing selected presence alternative {selected_node_kind}"),
        ));
    }
    let boolean_payload_kind = sidecar
        .structural_model
        .payload_leaves
        .iter()
        .find(|payload| {
            payload.owner_rule == owner_rule
                && payload.field == owner_field
                && payload.node_kind == selected_node_kind
        })
        .map(|payload| payload.payload_kind.clone())
        .ok_or_else(|| {
            SyntaxBackendBoundaryError::new(
                "go_ast_declaration_presence.decision.payload",
                format!("missing payload leaf for {owner_rule}.{owner_field} {selected_node_kind}"),
            )
        })?;
    let segment = base_stream
        .raw_token_segments
        .iter()
        .find(|segment| {
            segment.event_kind == "terminal"
                && segment.owner_rule.as_deref() == Some(owner_rule)
                && segment.owner_field.as_deref() == Some(owner_field)
                && segment.node_kind.as_deref() == Some(selected_node_kind)
        })
        .ok_or_else(|| {
            SyntaxBackendBoundaryError::new(
                "go_ast_declaration_presence.decision.segment",
                format!("missing terminal segment for {owner_rule}.{owner_field}"),
            )
        })?;
    require_usize(
        "go_ast_declaration_presence.decision.segment.raw_token_count",
        segment.raw_token_count,
        2,
    )?;
    let terminal_raw_token_offset = segment.raw_token_offset;
    let boolean_payload_raw_token_offset = terminal_raw_token_offset + 1;
    let boolean_payload_raw_token_id = *base_stream
        .raw_token_ids
        .get(boolean_payload_raw_token_offset)
        .ok_or_else(|| {
            SyntaxBackendBoundaryError::new(
                "go_ast_declaration_presence.decision.token",
                "boolean payload offset is outside the raw token stream",
            )
        })?;
    require_usize(
        "go_ast_declaration_presence.decision.terminal_raw_token",
        *base_stream
            .raw_token_ids
            .get(terminal_raw_token_offset)
            .ok_or_else(|| {
                SyntaxBackendBoundaryError::new(
                    "go_ast_declaration_presence.decision.token",
                    "terminal offset is outside the raw token stream",
                )
            })? as usize,
        roles.terminal as usize,
    )?;
    require_usize(
        "go_ast_declaration_presence.decision.boolean_payload_raw_token_id",
        boolean_payload_raw_token_id as usize,
        roles.boolean_true as usize,
    )?;
    let state_after_terminal = core
        .state_after_raw_prefix(&base_stream.raw_token_ids[..=terminal_raw_token_offset])
        .map_err(|error| {
            SyntaxBackendBoundaryError::new(
                "go_ast_declaration_presence.decision.state_after_terminal",
                format!(
                    "state after {owner_rule}.{owner_field} terminal should be valid: {}",
                    error.message
                ),
            )
        })?;
    let state_after_terminal_finite_admissible_raw_token_ids =
        state_after_terminal.admissible.finite_raw_token_ids.clone();
    let terminal_event_decoded = decoded_events.iter().any(|event| {
        event.raw_token_offset == terminal_raw_token_offset
            && event.event_kind == RawSyntaxDecodedEventKind::TerminalStart
            && event.rule.as_deref() == Some(owner_rule)
            && event.field.as_deref() == Some(owner_field)
    });
    let boolean_payload_decoded = decoded_events.iter().any(|event| {
        event.raw_token_offset == boolean_payload_raw_token_offset
            && event.event_kind == RawSyntaxDecodedEventKind::BooleanPayload
            && event.rule.as_deref() == Some(owner_rule)
            && event.field.as_deref() == Some(owner_field)
            && event.raw_token_id == roles.boolean_true
    });
    Ok(GoAstDeclarationPresenceDecision {
        owner_rule: owner_rule.to_string(),
        owner_field: owner_field.to_string(),
        domain_id: domain.id.clone(),
        selected_node_kind: selected_node_kind.to_string(),
        boolean_payload_kind,
        terminal_raw_token_offset,
        boolean_payload_raw_token_offset,
        boolean_payload_raw_token_id,
        state_after_terminal_finite_admissible_raw_token_ids:
            state_after_terminal_finite_admissible_raw_token_ids.clone(),
        terminal_event_decoded,
        boolean_payload_decoded,
        state_after_terminal_admits_false: state_after_terminal_finite_admissible_raw_token_ids
            .contains(&roles.boolean_false),
        state_after_terminal_admits_true: state_after_terminal_finite_admissible_raw_token_ids
            .contains(&roles.boolean_true),
    })
}

fn stream_from_segment_indices(
    base_stream: &RawReplayInput,
    indices: &[usize],
) -> Result<RawReplayInput, SyntaxBackendBoundaryError> {
    let mut raw_token_ids = Vec::new();
    let mut raw_token_segments = Vec::new();
    for (new_ordinal, index) in indices.iter().copied().enumerate() {
        let segment = base_stream.raw_token_segments.get(index).ok_or_else(|| {
            SyntaxBackendBoundaryError::new(
                "replay.stream_from_segments",
                format!("segment index {index} is out of range"),
            )
        })?;
        let end = segment
            .raw_token_offset
            .checked_add(segment.raw_token_count)
            .ok_or_else(|| {
                SyntaxBackendBoundaryError::new(
                    "replay.stream_from_segments",
                    "segment raw token range overflowed",
                )
            })?;
        if end > base_stream.raw_token_ids.len() {
            return Err(SyntaxBackendBoundaryError::new(
                "replay.stream_from_segments",
                "segment raw token range exceeds token stream length",
            ));
        }
        let new_offset = raw_token_ids.len();
        raw_token_ids.extend_from_slice(&base_stream.raw_token_ids[segment.raw_token_offset..end]);
        let mut cloned = segment.clone();
        cloned.event_ordinal = new_ordinal;
        cloned.raw_token_offset = new_offset;
        raw_token_segments.push(cloned);
    }
    Ok(RawReplayInput {
        raw_token_ids,
        raw_token_segments,
    })
}

fn missing_tokenized_alternative_identity_blockers(
    sidecar: &GrammarStructuralModelSidecar,
    roles: &ReplayTokenRoles,
) -> Vec<SyntaxReplayPromotionBlocker> {
    sidecar
        .structural_model
        .finite_domains
        .iter()
        .filter(|domain| {
            domain.alternatives.len() > 1
                && domain.alternatives.iter().any(|alternative| {
                    alternative.terminal == crate::grammar_ir::GrammarTerminalKind::Nonterminal
                        && !roles
                            .node_start_alternative_tokens
                            .contains_key(&alternative.node_kind)
                })
        })
        .map(|domain| {
            let alternatives = domain
                .alternatives
                .iter()
                .filter(|alternative| {
                    alternative.terminal == crate::grammar_ir::GrammarTerminalKind::Nonterminal
                })
                .map(|alternative| alternative.node_kind.clone())
                .collect::<Vec<_>>();
            SyntaxReplayPromotionBlocker {
                kind: GENERATED_CODEC_PROMOTION_BLOCKER.to_string(),
                owner_rule: domain.owner_rule.clone(),
                owner_field: domain.owner_field.clone(),
                alternative_count: alternatives.len(),
                alternatives,
                missing_fact: format!(
                    "raw node_start token does not identify which {}.{} alternative was selected",
                    domain.owner_rule, domain.owner_field
                ),
            }
        })
        .collect()
}

fn validate_csharp_parity_sidecar(
    sidecar: &GrammarStructuralModelSidecar,
    candidate_grammar_id: &str,
    metadata_path: &Path,
) -> Result<(), SyntaxBackendBoundaryError> {
    require_string(
        "csharp_state_machine_parity.sidecar.grammar_id",
        &sidecar.input.grammar_id,
        candidate_grammar_id,
    )?;
    require_string(
        "csharp_state_machine_parity.sidecar.source_kind",
        &sidecar.input.provenance.source_kind,
        "csharp_syntax_metadata",
    )?;
    require_string(
        "csharp_state_machine_parity.sidecar.metadata_path",
        sidecar
            .input
            .provenance
            .metadata_path
            .as_deref()
            .unwrap_or(""),
        &metadata_path.display().to_string(),
    )?;
    require_non_empty(
        "csharp_state_machine_parity.sidecar.source_path",
        sidecar
            .input
            .provenance
            .source_path
            .as_deref()
            .unwrap_or(""),
    )?;
    require_sha256(
        "csharp_state_machine_parity.sidecar.source_sha256",
        sidecar
            .input
            .provenance
            .source_sha256
            .as_deref()
            .unwrap_or(""),
    )?;
    require_sha256(
        "csharp_state_machine_parity.sidecar.metadata_sha256",
        sidecar
            .input
            .provenance
            .metadata_sha256
            .as_deref()
            .unwrap_or(""),
    )?;
    require_token_range(
        "csharp_state_machine_parity.sidecar.token_layout.structural_decisions",
        &sidecar.structural_model.token_ranges.ranges,
        "structural_decisions",
    )?;
    require_token_range(
        "csharp_state_machine_parity.sidecar.token_layout.utf8_byte_payload",
        &sidecar.structural_model.token_ranges.ranges,
        "utf8_byte_payload",
    )?;
    require_token_range_any(
        "csharp_state_machine_parity.sidecar.token_layout.integer_payload",
        &sidecar.structural_model.token_ranges.ranges,
        &["integer_lexeme_payload", "u8_integer_payload"],
    )
}

fn projection_token_layout_summary(
    layout: &GrammarStructuralTokenLayout,
) -> ProjectionTokenLayoutSummary {
    ProjectionTokenLayoutSummary {
        vocab_size: layout.vocab_size,
        pad_token: layout.pad_token,
        ranges: layout
            .ranges
            .iter()
            .map(|range| ProjectionTokenLayoutRangeSummary {
                id: range.id.clone(),
                role: range.role.clone(),
                start: range.start,
                end_exclusive: range.end_exclusive,
            })
            .collect(),
    }
}

fn synthesize_csharp_replay_input(
    sidecar: &GrammarStructuralModelSidecar,
    facts: &ModelFacts,
    roles: &ReplayTokenRoles,
) -> Result<RawReplayInput, SyntaxBackendBoundaryError> {
    if sidecar.input.grammar_id == CSHARP_LOCAL_INTEGER_RETURN_LITERAL_GRAMMAR_ID {
        return synthesize_structural_replay_input(
            sidecar,
            facts,
            roles,
            StructuralReplayProfile::CSharpLocalIntegerReturnFixture,
        );
    }
    synthesize_structural_replay_input(
        sidecar,
        facts,
        roles,
        StructuralReplayProfile::CSharpDefault,
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StructuralReplayProfile {
    CSharpDefault,
    CSharpMinimalClassFixture,
    CSharpLocalIntegerReturnFixture,
    GoAstDeclarationPresence,
}

impl StructuralReplayProfile {
    fn label(self) -> &'static str {
        match self {
            Self::CSharpDefault => "csharp_state_machine_parity.synthetic_stream",
            Self::CSharpMinimalClassFixture => {
                "csharp_minimal_class_raw_tree_render.synthetic_stream"
            }
            Self::CSharpLocalIntegerReturnFixture => {
                "csharp_local_integer_return_literal_raw_tree_render.synthetic_stream"
            }
            Self::GoAstDeclarationPresence => "go_ast_declaration_presence.synthetic_stream",
        }
    }

    fn root_rule(
        self,
        sidecar: &GrammarStructuralModelSidecar,
    ) -> Result<&str, SyntaxBackendBoundaryError> {
        match self {
            Self::CSharpDefault
            | Self::CSharpMinimalClassFixture
            | Self::CSharpLocalIntegerReturnFixture => sidecar
                .structural_model
                .records
                .first()
                .map(|record| record.grammar_rule.as_str())
                .ok_or_else(|| {
                    SyntaxBackendBoundaryError::new(
                        self.label(),
                        "StructuralModel must contain at least one record",
                    )
                }),
            Self::GoAstDeclarationPresence => structural_record_by_rule(sidecar, "File")
                .map(|record| record.grammar_rule.as_str()),
        }
    }

    fn should_emit_optional_value(self, owner_rule: &str, owner_field: &str) -> bool {
        match self {
            Self::CSharpDefault
            | Self::CSharpMinimalClassFixture
            | Self::CSharpLocalIntegerReturnFixture => false,
            Self::GoAstDeclarationPresence => matches!(
                (owner_rule, owner_field),
                ("File", "Decls")
                    | ("GenDecl", "Lparen")
                    | ("GenDecl", "Specs")
                    | ("GenDecl", "Rparen")
                    | ("TypeSpec", "Name")
                    | ("TypeSpec", "Assign")
                    | ("FuncDecl", "Type")
                    | ("FuncDecl", "Body")
                    | ("FuncType", "Func")
                    | ("BlockStmt", "List")
                    | ("ReturnStmt", "Results")
                    | ("CallExpr", "Args")
                    | ("CallExpr", "Ellipsis")
            ),
        }
    }

    fn preferred_node_kinds(self, owner_rule: &str, owner_field: &str) -> &'static [&'static str] {
        match self {
            Self::CSharpDefault
            | Self::CSharpMinimalClassFixture
            | Self::CSharpLocalIntegerReturnFixture => &[],
            Self::GoAstDeclarationPresence => match (owner_rule, owner_field) {
                ("File", "Decls") => &["GenDecl", "FuncDecl"],
                ("GenDecl", "Tok") => &["go_token_declaration_tokens.TYPE"],
                ("GenDecl", "Lparen") | ("GenDecl", "Rparen") => {
                    &["go_syntax_presence_gen_decl_group.PRESENT"]
                }
                ("GenDecl", "Specs") => &["TypeSpec"],
                ("TypeSpec", "Name") | ("TypeSpec", "Type") => &["Ident"],
                ("TypeSpec", "Assign") => &["go_syntax_presence_type_spec_alias.PRESENT"],
                ("FuncDecl", "Type") => &["FuncType"],
                ("FuncDecl", "Body") => &["BlockStmt"],
                ("FuncType", "Func") => &["go_syntax_presence_func_type_func.PRESENT"],
                ("BlockStmt", "List") => &["ReturnStmt"],
                ("ReturnStmt", "Results") => &["CallExpr"],
                ("CallExpr", "Fun") | ("CallExpr", "Args") => &["Ident"],
                ("CallExpr", "Ellipsis") => &["go_syntax_presence_call_expr_ellipsis.PRESENT"],
                _ => &[],
            },
        }
    }

    fn missing_field_segment<'a>(
        self,
        field: &GrammarStructuralField,
        roles: &'a ReplayTokenRoles,
    ) -> (&'static str, &'a [u32]) {
        match self {
            Self::GoAstDeclarationPresence if field.multiple => (
                "repeated_empty",
                std::slice::from_ref(&roles.repeated_empty),
            ),
            _ => ("field_missing", std::slice::from_ref(&roles.field_missing)),
        }
    }

    fn identifier_payload_bytes(
        self,
        owner_rule: &str,
        owner_field: &str,
        node_kind: &str,
    ) -> &'static [u8] {
        match self {
            Self::CSharpMinimalClassFixture => match (owner_rule, owner_field, node_kind) {
                ("FileScopedNamespaceDeclarationSyntax", "Name", "qualified_identifier") => {
                    b"Tct.Contracts"
                }
                ("ClassDeclarationSyntax", "Identifier", "identifier") => b"Greeter",
                ("PropertyDeclarationSyntax", "Identifier", "identifier") => b"Name",
                ("PredefinedTypeSyntax", "Keyword", "string_keyword") => b"string",
                _ => b"x",
            },
            Self::CSharpLocalIntegerReturnFixture => match (owner_rule, owner_field, node_kind) {
                ("FileScopedNamespaceDeclarationSyntax", "Name", "qualified_identifier") => {
                    b"Tct.Contracts"
                }
                ("ClassDeclarationSyntax", "Identifier", "identifier") => b"Calculator",
                ("MethodDeclarationSyntax", "Identifier", "identifier") => b"Build",
                ("PredefinedTypeSyntax", "Keyword", "int_keyword") => b"int",
                _ => b"x",
            },
            Self::CSharpDefault | Self::GoAstDeclarationPresence => b"x",
        }
    }

    fn string_payload_bytes(
        self,
        owner_rule: &str,
        owner_field: &str,
        node_kind: &str,
    ) -> &'static [u8] {
        match (self, owner_rule, owner_field, node_kind) {
            (
                Self::CSharpMinimalClassFixture,
                "PropertyDeclarationSyntax",
                "Initializer",
                "StringLiteralExpressionSyntax",
            ) => b"world",
            (_, "NullableTypeSyntax", "QuestionToken", "nullable_marker") => b"?",
            _ => b"s",
        }
    }
}

fn synthesize_structural_replay_input(
    sidecar: &GrammarStructuralModelSidecar,
    facts: &ModelFacts,
    roles: &ReplayTokenRoles,
    profile: StructuralReplayProfile,
) -> Result<RawReplayInput, SyntaxBackendBoundaryError> {
    let root_rule = sidecar
        .structural_model
        .records
        .first()
        .map(|_| profile.root_rule(sidecar))
        .transpose()?
        .ok_or_else(|| {
            SyntaxBackendBoundaryError::new(
                profile.label(),
                "StructuralModel must contain at least one record",
            )
        })?;
    let mut builder = StructuralReplayStreamBuilder::default();
    let mut rule_stack = Vec::new();
    emit_structural_record(
        &mut builder,
        sidecar,
        facts,
        roles,
        root_rule,
        None,
        &mut rule_stack,
        profile,
    )?;
    Ok(RawReplayInput {
        raw_token_ids: builder.raw_token_ids,
        raw_token_segments: builder.raw_token_segments,
    })
}

#[derive(Default)]
struct StructuralReplayStreamBuilder {
    raw_token_ids: Vec<u32>,
    raw_token_segments: Vec<ProjectionRawTokenSegment>,
}

impl StructuralReplayStreamBuilder {
    fn push_segment(&mut self, segment: StructuralReplaySegment<'_>) {
        let raw_token_offset = self.raw_token_ids.len();
        self.raw_token_ids.extend_from_slice(segment.token_ids);
        self.raw_token_segments.push(ProjectionRawTokenSegment {
            event_ordinal: self.raw_token_segments.len(),
            event_kind: segment.event_kind.to_string(),
            primary_token_range_role: segment.primary_token_range_role.to_string(),
            raw_token_offset,
            raw_token_count: segment.token_ids.len(),
            owner_rule: segment.owner_rule.map(str::to_string),
            owner_field: segment.owner_field.map(str::to_string),
            node_kind: segment.node_kind.map(str::to_string),
            payload_kind: segment.payload_kind.map(str::to_string),
        });
    }
}

struct StructuralReplaySegment<'a> {
    event_kind: &'a str,
    primary_token_range_role: &'a str,
    token_ids: &'a [u32],
    owner_rule: Option<&'a str>,
    owner_field: Option<&'a str>,
    node_kind: Option<&'a str>,
    payload_kind: Option<&'a str>,
}

#[allow(clippy::too_many_arguments)]
fn emit_structural_record(
    builder: &mut StructuralReplayStreamBuilder,
    sidecar: &GrammarStructuralModelSidecar,
    facts: &ModelFacts,
    roles: &ReplayTokenRoles,
    rule: &str,
    parent_field: Option<(&str, &str)>,
    rule_stack: &mut Vec<String>,
    profile: StructuralReplayProfile,
) -> Result<(), SyntaxBackendBoundaryError> {
    if rule_stack.iter().any(|active| active == rule) {
        return Err(SyntaxBackendBoundaryError::new(
            profile.label(),
            format!("synthetic structural replay stream encountered recursive rule {rule}"),
        ));
    }
    let record = structural_record_by_rule(sidecar, rule)?;
    let node_start = match parent_field {
        Some((owner_rule, owner_field)) => structural_child_node_start_token(
            owner_rule,
            owner_field,
            &record.record_type,
            facts,
            roles,
        )?,
        None => roles.node_start,
    };
    builder.push_segment(StructuralReplaySegment {
        event_kind: "node_start",
        primary_token_range_role: &roles.structural_range.role,
        token_ids: &[node_start],
        owner_rule: Some(&record.grammar_rule),
        owner_field: None,
        node_kind: Some(&record.record_type),
        payload_kind: None,
    });

    rule_stack.push(rule.to_string());
    for field in &record.fields {
        builder.push_segment(StructuralReplaySegment {
            event_kind: "field_start",
            primary_token_range_role: &roles.structural_range.role,
            token_ids: &[roles.field_start],
            owner_rule: Some(&record.grammar_rule),
            owner_field: Some(&field.name),
            node_kind: None,
            payload_kind: None,
        });
        if field.required || profile.should_emit_optional_value(&record.grammar_rule, &field.name) {
            let preferred_node_kinds =
                profile.preferred_node_kinds(&record.grammar_rule, &field.name);
            if preferred_node_kinds.len() > 1 && !field.multiple {
                return Err(SyntaxBackendBoundaryError::new(
                    profile.label(),
                    format!(
                        "{}.{} cannot replay multiple preferred alternatives because it is not repeated",
                        record.grammar_rule, field.name
                    ),
                ));
            }
            if preferred_node_kinds.is_empty() {
                emit_structural_field_value(
                    builder,
                    sidecar,
                    facts,
                    roles,
                    record,
                    &field.name,
                    None,
                    rule_stack,
                    profile,
                )?;
            } else {
                for preferred_node_kind in preferred_node_kinds {
                    emit_structural_field_value(
                        builder,
                        sidecar,
                        facts,
                        roles,
                        record,
                        &field.name,
                        Some(*preferred_node_kind),
                        rule_stack,
                        profile,
                    )?;
                }
            }
        } else {
            let (event_kind, token_ids) = profile.missing_field_segment(field, roles);
            builder.push_segment(StructuralReplaySegment {
                event_kind,
                primary_token_range_role: &roles.structural_range.role,
                token_ids,
                owner_rule: Some(&record.grammar_rule),
                owner_field: Some(&field.name),
                node_kind: None,
                payload_kind: None,
            });
        }
        builder.push_segment(StructuralReplaySegment {
            event_kind: "field_end",
            primary_token_range_role: &roles.structural_range.role,
            token_ids: &[roles.field_end],
            owner_rule: Some(&record.grammar_rule),
            owner_field: Some(&field.name),
            node_kind: None,
            payload_kind: None,
        });
    }
    rule_stack.pop();

    builder.push_segment(StructuralReplaySegment {
        event_kind: "node_end",
        primary_token_range_role: &roles.structural_range.role,
        token_ids: &[roles.node_end],
        owner_rule: Some(&record.grammar_rule),
        owner_field: None,
        node_kind: Some(&record.record_type),
        payload_kind: None,
    });
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn emit_structural_field_value(
    builder: &mut StructuralReplayStreamBuilder,
    sidecar: &GrammarStructuralModelSidecar,
    facts: &ModelFacts,
    roles: &ReplayTokenRoles,
    record: &GrammarStructuralRecord,
    field_name: &str,
    preferred_node_kind: Option<&str>,
    rule_stack: &mut Vec<String>,
    profile: StructuralReplayProfile,
) -> Result<(), SyntaxBackendBoundaryError> {
    let domain = sidecar
        .structural_model
        .finite_domains
        .iter()
        .find(|domain| domain.owner_rule == record.grammar_rule && domain.owner_field == field_name)
        .ok_or_else(|| {
            SyntaxBackendBoundaryError::new(
                profile.label(),
                format!(
                    "missing finite domain for {}.{field_name}",
                    record.grammar_rule
                ),
            )
        })?;
    let mut alternatives = domain.alternatives.iter().collect::<Vec<_>>();
    if let Some(preferred_node_kind) = preferred_node_kind {
        if !alternatives
            .iter()
            .any(|alternative| alternative.node_kind == preferred_node_kind)
        {
            return Err(SyntaxBackendBoundaryError::new(
                profile.label(),
                format!(
                    "preferred replay alternative {preferred_node_kind} is missing for {}.{field_name}",
                    record.grammar_rule
                ),
            ));
        }
        alternatives.sort_by_key(|alternative| {
            if alternative.node_kind == preferred_node_kind {
                0
            } else {
                1
            }
        });
    }
    for alternative in alternatives {
        match alternative.terminal {
            GrammarTerminalKind::Nonterminal => {
                let target_rule = target_rule_for_node_kind(
                    sidecar,
                    &record.grammar_rule,
                    field_name,
                    &alternative.node_kind,
                )?;
                if rule_stack.iter().any(|active| active == target_rule) {
                    continue;
                }
                return emit_structural_record(
                    builder,
                    sidecar,
                    facts,
                    roles,
                    target_rule,
                    Some((&record.grammar_rule, field_name)),
                    rule_stack,
                    profile,
                );
            }
            GrammarTerminalKind::IdentifierPayload
            | GrammarTerminalKind::StringPayload
            | GrammarTerminalKind::IntegerPayload
            | GrammarTerminalKind::BooleanPayload => {
                return emit_structural_terminal(
                    builder,
                    roles,
                    &record.grammar_rule,
                    field_name,
                    alternative,
                    profile,
                );
            }
        }
    }

    let recursive_targets = domain
        .alternatives
        .iter()
        .filter(|alternative| alternative.terminal == GrammarTerminalKind::Nonterminal)
        .map(|alternative| alternative.node_kind.clone())
        .collect::<Vec<_>>();
    Err(SyntaxBackendBoundaryError::new(
        profile.label(),
        format!(
            "field {}.{field_name} has no acyclic replay alternative; recursive alternatives: {:?}",
            record.grammar_rule, recursive_targets
        ),
    ))
}

fn emit_structural_terminal(
    builder: &mut StructuralReplayStreamBuilder,
    roles: &ReplayTokenRoles,
    owner_rule: &str,
    owner_field: &str,
    alternative: &crate::grammar_ir_structural_model::GrammarFiniteAlternative,
    profile: StructuralReplayProfile,
) -> Result<(), SyntaxBackendBoundaryError> {
    let (payload_kind, primary_role, token_ids) = match alternative.terminal {
        GrammarTerminalKind::IdentifierPayload => (
            format!("identifier_payload:{}", alternative.node_kind),
            roles.utf8_range.role.clone(),
            utf8_payload_tokens(
                roles,
                profile.identifier_payload_bytes(owner_rule, owner_field, &alternative.node_kind),
            ),
        ),
        GrammarTerminalKind::StringPayload => (
            format!("string_payload:{}", alternative.node_kind),
            roles.utf8_range.role.clone(),
            utf8_payload_tokens(
                roles,
                profile.string_payload_bytes(owner_rule, owner_field, &alternative.node_kind),
            ),
        ),
        GrammarTerminalKind::IntegerPayload => (
            format!("integer_payload:{}", alternative.node_kind),
            roles.integer_range.role.clone(),
            vec![
                roles.terminal,
                roles.integer_range.start + 7,
                roles.payload_end,
            ],
        ),
        GrammarTerminalKind::BooleanPayload => (
            format!("boolean_payload:{}", alternative.node_kind),
            roles.structural_range.role.clone(),
            vec![roles.terminal, roles.boolean_true],
        ),
        GrammarTerminalKind::Nonterminal => {
            return Err(SyntaxBackendBoundaryError::new(
                "structural_replay.synthetic_stream.terminal",
                "nonterminal alternative cannot be emitted as terminal payload",
            ));
        }
    };
    builder.push_segment(StructuralReplaySegment {
        event_kind: "terminal",
        primary_token_range_role: &primary_role,
        token_ids: &token_ids,
        owner_rule: Some(owner_rule),
        owner_field: Some(owner_field),
        node_kind: Some(&alternative.node_kind),
        payload_kind: Some(&payload_kind),
    });
    Ok(())
}

fn utf8_payload_tokens(roles: &ReplayTokenRoles, payload: &[u8]) -> Vec<u32> {
    let mut token_ids = Vec::with_capacity(payload.len() + 2);
    token_ids.push(roles.terminal);
    token_ids.extend(
        payload
            .iter()
            .map(|byte| roles.utf8_range.start + u32::from(*byte)),
    );
    token_ids.push(roles.payload_end);
    token_ids
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CSharpCompilationUnitTypeKind {
    Class,
    Struct,
}

struct CSharpCompilationUnitTypeReplay {
    kind: CSharpCompilationUnitTypeKind,
    name: &'static str,
    properties: &'static [&'static str],
    parameters: &'static [(&'static str, &'static str)],
    assignments: &'static [(&'static str, &'static str)],
}

const CSHARP_COMPILATION_UNIT_SOURCE_FILE_USINGS: &[&str] = &[
    "System",
    "System.Collections.Generic",
    "System.Linq",
    "System.Text",
    "System.Threading.Tasks",
];
const CSHARP_SHAPES_SQUARE_PROPERTIES: &[&str] = &["Side"];
const CSHARP_SHAPES_SQUARE_PARAMETERS: &[(&str, &str)] = &[("double", "side")];
const CSHARP_SHAPES_SQUARE_ASSIGNMENTS: &[(&str, &str)] = &[("Side", "side")];
const CSHARP_SHAPES_CIRCLE_PROPERTIES: &[&str] = &["Radius"];
const CSHARP_SHAPES_CIRCLE_PARAMETERS: &[(&str, &str)] = &[("double", "radius")];
const CSHARP_SHAPES_CIRCLE_ASSIGNMENTS: &[(&str, &str)] = &[("Radius", "radius")];
const CSHARP_SHAPES_RECTANGLE_PROPERTIES: &[&str] = &["Length", "Height"];
const CSHARP_SHAPES_RECTANGLE_PARAMETERS: &[(&str, &str)] =
    &[("double", "length"), ("double", "height")];
const CSHARP_SHAPES_RECTANGLE_ASSIGNMENTS: &[(&str, &str)] =
    &[("Length", "length"), ("Height", "height")];
const CSHARP_SHAPES_TRIANGLE_PROPERTIES: &[&str] = &["Base", "Height"];
const CSHARP_SHAPES_TRIANGLE_PARAMETERS: &[(&str, &str)] =
    &[("double", "@base"), ("double", "height")];
const CSHARP_SHAPES_TRIANGLE_ASSIGNMENTS: &[(&str, &str)] =
    &[("Base", "@base"), ("Height", "height")];
const CSHARP_COMPILATION_UNIT_SOURCE_FILE_TYPES: &[CSharpCompilationUnitTypeReplay] = &[
    CSharpCompilationUnitTypeReplay {
        kind: CSharpCompilationUnitTypeKind::Class,
        name: "Square",
        properties: CSHARP_SHAPES_SQUARE_PROPERTIES,
        parameters: CSHARP_SHAPES_SQUARE_PARAMETERS,
        assignments: CSHARP_SHAPES_SQUARE_ASSIGNMENTS,
    },
    CSharpCompilationUnitTypeReplay {
        kind: CSharpCompilationUnitTypeKind::Class,
        name: "Circle",
        properties: CSHARP_SHAPES_CIRCLE_PROPERTIES,
        parameters: CSHARP_SHAPES_CIRCLE_PARAMETERS,
        assignments: CSHARP_SHAPES_CIRCLE_ASSIGNMENTS,
    },
    CSharpCompilationUnitTypeReplay {
        kind: CSharpCompilationUnitTypeKind::Struct,
        name: "Rectangle",
        properties: CSHARP_SHAPES_RECTANGLE_PROPERTIES,
        parameters: CSHARP_SHAPES_RECTANGLE_PARAMETERS,
        assignments: CSHARP_SHAPES_RECTANGLE_ASSIGNMENTS,
    },
    CSharpCompilationUnitTypeReplay {
        kind: CSharpCompilationUnitTypeKind::Class,
        name: "Triangle",
        properties: CSHARP_SHAPES_TRIANGLE_PROPERTIES,
        parameters: CSHARP_SHAPES_TRIANGLE_PARAMETERS,
        assignments: CSHARP_SHAPES_TRIANGLE_ASSIGNMENTS,
    },
];
const CSHARP_LOGGER_ENUM_MEMBERS: &[&str] = &[
    "Verbose",
    "Trace",
    "Information",
    "Warning",
    "Error",
    "Critical",
];
const CSHARP_LOGGER_METHOD_PARAMETERS: &[(&str, &str)] = &[
    ("Severity", "s"),
    ("string", "component"),
    ("string", "msg"),
];

fn synthesize_csharp_compilation_unit_source_file_raw_tree_render_input(
    sidecar: &GrammarStructuralModelSidecar,
    facts: &ModelFacts,
    roles: &ReplayTokenRoles,
) -> Result<RawReplayInput, SyntaxBackendBoundaryError> {
    synthesize_csharp_raw_tree_render_input(sidecar, facts, roles, |fixture| {
        fixture.emit_compilation_unit_source_file()
    })
}

fn synthesize_csharp_member_access_expression_raw_tree_render_input(
    sidecar: &GrammarStructuralModelSidecar,
    facts: &ModelFacts,
    roles: &ReplayTokenRoles,
) -> Result<RawReplayInput, SyntaxBackendBoundaryError> {
    synthesize_csharp_raw_tree_render_input(sidecar, facts, roles, |fixture| {
        fixture.emit_member_access_expression_source()
    })
}

fn synthesize_csharp_local_unary_integer_return_raw_tree_render_input(
    sidecar: &GrammarStructuralModelSidecar,
    facts: &ModelFacts,
    roles: &ReplayTokenRoles,
) -> Result<RawReplayInput, SyntaxBackendBoundaryError> {
    synthesize_csharp_raw_tree_render_input(sidecar, facts, roles, |fixture| {
        fixture.emit_local_unary_integer_return_source()
    })
}

fn synthesize_csharp_compilation_unit_logger_source_file_raw_tree_render_input(
    sidecar: &GrammarStructuralModelSidecar,
    facts: &ModelFacts,
    roles: &ReplayTokenRoles,
) -> Result<RawReplayInput, SyntaxBackendBoundaryError> {
    synthesize_csharp_raw_tree_render_input(sidecar, facts, roles, |fixture| {
        fixture.emit_compilation_unit_logger_source_file()
    })
}

fn synthesize_csharp_compilation_unit_version_one_source_file_raw_tree_render_input(
    sidecar: &GrammarStructuralModelSidecar,
    facts: &ModelFacts,
    roles: &ReplayTokenRoles,
) -> Result<RawReplayInput, SyntaxBackendBoundaryError> {
    synthesize_csharp_raw_tree_render_input(sidecar, facts, roles, |fixture| {
        fixture.emit_compilation_unit_version_one_source_file()
    })
}

fn synthesize_csharp_raw_tree_render_input(
    sidecar: &GrammarStructuralModelSidecar,
    facts: &ModelFacts,
    roles: &ReplayTokenRoles,
    emit_fixture: impl FnOnce(
        &mut CSharpCompilationUnitSourceFileReplayBuilder<'_>,
    ) -> Result<(), SyntaxBackendBoundaryError>,
) -> Result<RawReplayInput, SyntaxBackendBoundaryError> {
    let mut fixture = CSharpCompilationUnitSourceFileReplayBuilder {
        builder: StructuralReplayStreamBuilder::default(),
        sidecar,
        facts,
        roles,
    };
    emit_fixture(&mut fixture)?;
    Ok(RawReplayInput {
        raw_token_ids: fixture.builder.raw_token_ids,
        raw_token_segments: fixture.builder.raw_token_segments,
    })
}

struct CSharpCompilationUnitSourceFileReplayBuilder<'a> {
    builder: StructuralReplayStreamBuilder,
    sidecar: &'a GrammarStructuralModelSidecar,
    facts: &'a ModelFacts,
    roles: &'a ReplayTokenRoles,
}

impl CSharpCompilationUnitSourceFileReplayBuilder<'_> {
    fn emit_compilation_unit_source_file(&mut self) -> Result<(), SyntaxBackendBoundaryError> {
        self.emit_node("CompilationUnitSyntax", None, |this| {
            this.emit_field("CompilationUnitSyntax", "Usings", |this| {
                for using_name in CSHARP_COMPILATION_UNIT_SOURCE_FILE_USINGS {
                    this.emit_using_directive(using_name)?;
                }
                Ok(())
            })?;
            this.emit_field("CompilationUnitSyntax", "Members", |this| {
                this.emit_namespace_declaration()
            })
        })
    }

    fn emit_compilation_unit_logger_source_file(
        &mut self,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        self.emit_node("CompilationUnitSyntax", None, |this| {
            this.emit_field("CompilationUnitSyntax", "Members", |this| {
                this.emit_logger_file_scoped_namespace()
            })
        })
    }

    fn emit_compilation_unit_version_one_source_file(
        &mut self,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        self.emit_node("CompilationUnitSyntax", None, |this| {
            this.emit_field("CompilationUnitSyntax", "Members", |this| {
                this.emit_version_one_file_scoped_namespace()
            })
        })
    }

    fn emit_member_access_expression_source(&mut self) -> Result<(), SyntaxBackendBoundaryError> {
        self.emit_node("CompilationUnitSyntax", None, |this| {
            this.emit_field("CompilationUnitSyntax", "Members", |this| {
                this.emit_node(
                    "FileScopedNamespaceDeclarationSyntax",
                    Some(("CompilationUnitSyntax", "Members")),
                    |this| {
                        this.emit_field("FileScopedNamespaceDeclarationSyntax", "Name", |this| {
                            this.emit_identifier_payload(
                                "FileScopedNamespaceDeclarationSyntax",
                                "Name",
                                "qualified_identifier",
                                "Tct.Contracts",
                            )
                        })?;
                        this.emit_field("FileScopedNamespaceDeclarationSyntax", "Members", |this| {
                            this.emit_node(
                                "ClassDeclarationSyntax",
                                Some(("FileScopedNamespaceDeclarationSyntax", "Members")),
                                |this| {
                                    this.emit_field(
                                        "ClassDeclarationSyntax",
                                        "Identifier",
                                        |this| {
                                            this.emit_identifier_payload(
                                                "ClassDeclarationSyntax",
                                                "Identifier",
                                                "identifier",
                                                "ClockContract",
                                            )
                                        },
                                    )?;
                                    this.emit_field("ClassDeclarationSyntax", "Members", |this| {
                                        this.emit_member_access_method_declaration()
                                    })
                                },
                            )
                        })
                    },
                )
            })
        })
    }

    fn emit_local_unary_integer_return_source(&mut self) -> Result<(), SyntaxBackendBoundaryError> {
        self.emit_node("CompilationUnitSyntax", None, |this| {
            this.emit_field("CompilationUnitSyntax", "Members", |this| {
                this.emit_node(
                    "FileScopedNamespaceDeclarationSyntax",
                    Some(("CompilationUnitSyntax", "Members")),
                    |this| {
                        this.emit_field("FileScopedNamespaceDeclarationSyntax", "Name", |this| {
                            this.emit_identifier_payload(
                                "FileScopedNamespaceDeclarationSyntax",
                                "Name",
                                "qualified_identifier",
                                "Tct.Contracts",
                            )
                        })?;
                        this.emit_field("FileScopedNamespaceDeclarationSyntax", "Members", |this| {
                            this.emit_node(
                                "ClassDeclarationSyntax",
                                Some(("FileScopedNamespaceDeclarationSyntax", "Members")),
                                |this| {
                                    this.emit_field(
                                        "ClassDeclarationSyntax",
                                        "Identifier",
                                        |this| {
                                            this.emit_identifier_payload(
                                                "ClassDeclarationSyntax",
                                                "Identifier",
                                                "identifier",
                                                "Calculator",
                                            )
                                        },
                                    )?;
                                    this.emit_field("ClassDeclarationSyntax", "Members", |this| {
                                        this.emit_local_unary_integer_return_method_declaration()
                                    })
                                },
                            )
                        })
                    },
                )
            })
        })
    }

    fn emit_local_unary_integer_return_method_declaration(
        &mut self,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        self.emit_node(
            "MethodDeclarationSyntax",
            Some(("ClassDeclarationSyntax", "Members")),
            |this| {
                this.emit_field("MethodDeclarationSyntax", "Identifier", |this| {
                    this.emit_identifier_payload(
                        "MethodDeclarationSyntax",
                        "Identifier",
                        "identifier",
                        "Build",
                    )
                })?;
                this.emit_field("MethodDeclarationSyntax", "ReturnType", |this| {
                    this.emit_predefined_type(
                        "MethodDeclarationSyntax",
                        "ReturnType",
                        "int_keyword",
                        "int",
                    )
                })?;
                this.emit_field("MethodDeclarationSyntax", "Body", |this| {
                    this.emit_node(
                        "BlockSyntax",
                        Some(("MethodDeclarationSyntax", "Body")),
                        |this| {
                            this.emit_field("BlockSyntax", "Statements", |this| {
                                this.emit_node(
                                    "ReturnStatementSyntax",
                                    Some(("BlockSyntax", "Statements")),
                                    |this| {
                                        this.emit_field(
                                            "ReturnStatementSyntax",
                                            "Expression",
                                            |this| this.emit_prefix_unary_integer_expression(),
                                        )
                                    },
                                )
                            })
                        },
                    )
                })
            },
        )
    }

    fn emit_prefix_unary_integer_expression(&mut self) -> Result<(), SyntaxBackendBoundaryError> {
        self.emit_node(
            "PrefixUnaryExpressionSyntax",
            Some(("ReturnStatementSyntax", "Expression")),
            |this| {
                this.emit_field("PrefixUnaryExpressionSyntax", "OperatorToken", |this| {
                    this.emit_string_payload(
                        "PrefixUnaryExpressionSyntax",
                        "OperatorToken",
                        "unary_operator",
                        "-",
                    )
                })?;
                this.emit_field("PrefixUnaryExpressionSyntax", "Operand", |this| {
                    this.emit_node(
                        "LiteralExpressionSyntax",
                        Some(("PrefixUnaryExpressionSyntax", "Operand")),
                        |this| {
                            this.emit_field("LiteralExpressionSyntax", "Token", |this| {
                                this.emit_integer_payload(
                                    "LiteralExpressionSyntax",
                                    "Token",
                                    "integer_literal",
                                    1,
                                )
                            })
                        },
                    )
                })
            },
        )
    }

    fn emit_member_access_method_declaration(&mut self) -> Result<(), SyntaxBackendBoundaryError> {
        self.emit_node(
            "MethodDeclarationSyntax",
            Some(("ClassDeclarationSyntax", "Members")),
            |this| {
                this.emit_field("MethodDeclarationSyntax", "Identifier", |this| {
                    this.emit_identifier_payload(
                        "MethodDeclarationSyntax",
                        "Identifier",
                        "identifier",
                        "Read",
                    )
                })?;
                this.emit_field("MethodDeclarationSyntax", "ReturnType", |this| {
                    this.emit_predefined_type(
                        "MethodDeclarationSyntax",
                        "ReturnType",
                        "string_keyword",
                        "string",
                    )
                })?;
                this.emit_field("MethodDeclarationSyntax", "Body", |this| {
                    this.emit_node(
                        "BlockSyntax",
                        Some(("MethodDeclarationSyntax", "Body")),
                        |this| {
                            this.emit_field("BlockSyntax", "Statements", |this| {
                                this.emit_node(
                                    "ReturnStatementSyntax",
                                    Some(("BlockSyntax", "Statements")),
                                    |this| {
                                        this.emit_field(
                                            "ReturnStatementSyntax",
                                            "Expression",
                                            |this| {
                                                this.emit_member_access_expression(
                                                    "ReturnStatementSyntax",
                                                    "Expression",
                                                    &["DateTime", "Now"],
                                                )
                                            },
                                        )
                                    },
                                )
                            })
                        },
                    )
                })
            },
        )
    }

    fn emit_logger_file_scoped_namespace(&mut self) -> Result<(), SyntaxBackendBoundaryError> {
        self.emit_file_scoped_namespace_with_members("DelegatesAndEvents", |this| {
            this.emit_logger_enum_declaration()?;
            this.emit_logger_class_declaration()
        })
    }

    fn emit_logger_enum_declaration(&mut self) -> Result<(), SyntaxBackendBoundaryError> {
        self.emit_node(
            "EnumDeclarationSyntax",
            Some(("FileScopedNamespaceDeclarationSyntax", "Members")),
            |this| {
                this.emit_field("EnumDeclarationSyntax", "Identifier", |this| {
                    this.emit_identifier_payload(
                        "EnumDeclarationSyntax",
                        "Identifier",
                        "identifier",
                        "Severity",
                    )
                })?;
                this.emit_field("EnumDeclarationSyntax", "Members", |this| {
                    for member in CSHARP_LOGGER_ENUM_MEMBERS {
                        this.emit_enum_member_declaration(member)?;
                    }
                    Ok(())
                })
            },
        )
    }

    fn emit_enum_member_declaration(
        &mut self,
        member: &str,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        self.emit_node(
            "EnumMemberDeclarationSyntax",
            Some(("EnumDeclarationSyntax", "Members")),
            |this| {
                this.emit_field("EnumMemberDeclarationSyntax", "Identifier", |this| {
                    this.emit_identifier_payload(
                        "EnumMemberDeclarationSyntax",
                        "Identifier",
                        "identifier",
                        member,
                    )
                })
            },
        )
    }

    fn emit_logger_class_declaration(&mut self) -> Result<(), SyntaxBackendBoundaryError> {
        self.emit_node(
            "ClassDeclarationSyntax",
            Some(("FileScopedNamespaceDeclarationSyntax", "Members")),
            |this| {
                this.emit_field("ClassDeclarationSyntax", "Identifier", |this| {
                    this.emit_identifier_payload(
                        "ClassDeclarationSyntax",
                        "Identifier",
                        "identifier",
                        "Logger",
                    )
                })?;
                this.emit_field("ClassDeclarationSyntax", "Members", |this| {
                    this.emit_logger_field_declaration()?;
                    this.emit_logger_property_declaration()?;
                    this.emit_logger_method_declaration()
                })
            },
        )
    }

    fn emit_version_one_file_scoped_namespace(&mut self) -> Result<(), SyntaxBackendBoundaryError> {
        self.emit_file_scoped_namespace_with_members("Events", |this| {
            this.emit_version_one_file_found_args_class()?;
            this.emit_version_one_file_searcher_class()
        })
    }

    fn emit_file_scoped_namespace_with_members<F>(
        &mut self,
        namespace: &str,
        emit_members: F,
    ) -> Result<(), SyntaxBackendBoundaryError>
    where
        F: FnOnce(&mut Self) -> Result<(), SyntaxBackendBoundaryError>,
    {
        self.emit_node(
            "FileScopedNamespaceDeclarationSyntax",
            Some(("CompilationUnitSyntax", "Members")),
            |this| {
                this.emit_field("FileScopedNamespaceDeclarationSyntax", "Name", |this| {
                    this.emit_identifier_payload(
                        "FileScopedNamespaceDeclarationSyntax",
                        "Name",
                        "qualified_identifier",
                        namespace,
                    )
                })?;
                this.emit_field("FileScopedNamespaceDeclarationSyntax", "Members", |this| {
                    emit_members(this)
                })
            },
        )
    }

    fn emit_version_one_file_found_args_class(&mut self) -> Result<(), SyntaxBackendBoundaryError> {
        self.emit_node(
            "ClassDeclarationSyntax",
            Some(("FileScopedNamespaceDeclarationSyntax", "Members")),
            |this| {
                this.emit_field("ClassDeclarationSyntax", "Identifier", |this| {
                    this.emit_identifier_payload(
                        "ClassDeclarationSyntax",
                        "Identifier",
                        "identifier",
                        "FileFoundArgs",
                    )
                })?;
                this.emit_field("ClassDeclarationSyntax", "BaseList", |this| {
                    this.emit_base_list_identifier("EventArgs")
                })?;
                this.emit_field("ClassDeclarationSyntax", "Members", |this| {
                    this.emit_version_one_found_file_property()?;
                    this.emit_version_one_file_found_args_constructor()
                })
            },
        )
    }

    fn emit_version_one_file_searcher_class(&mut self) -> Result<(), SyntaxBackendBoundaryError> {
        self.emit_node(
            "ClassDeclarationSyntax",
            Some(("FileScopedNamespaceDeclarationSyntax", "Members")),
            |this| {
                this.emit_field("ClassDeclarationSyntax", "Identifier", |this| {
                    this.emit_identifier_payload(
                        "ClassDeclarationSyntax",
                        "Identifier",
                        "identifier",
                        "FileSearcher",
                    )
                })?;
                this.emit_missing_field("ClassDeclarationSyntax", "BaseList")?;
                this.emit_field("ClassDeclarationSyntax", "Members", |this| {
                    this.emit_version_one_event_field_declaration()?;
                    this.emit_version_one_search_method_declaration()
                })
            },
        )
    }

    fn emit_base_list_identifier(
        &mut self,
        base_type: &str,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        self.emit_node(
            "BaseListSyntax",
            Some(("ClassDeclarationSyntax", "BaseList")),
            |this| {
                this.emit_field("BaseListSyntax", "Types", |this| {
                    this.emit_node(
                        "SimpleBaseTypeSyntax",
                        Some(("BaseListSyntax", "Types")),
                        |this| {
                            this.emit_field("SimpleBaseTypeSyntax", "Type", |this| {
                                this.emit_identifier_name_syntax(
                                    "SimpleBaseTypeSyntax",
                                    "Type",
                                    base_type,
                                )
                            })
                        },
                    )
                })
            },
        )
    }

    fn emit_version_one_found_file_property(&mut self) -> Result<(), SyntaxBackendBoundaryError> {
        self.emit_node(
            "PropertyDeclarationSyntax",
            Some(("ClassDeclarationSyntax", "Members")),
            |this| {
                this.emit_field("PropertyDeclarationSyntax", "Identifier", |this| {
                    this.emit_identifier_payload(
                        "PropertyDeclarationSyntax",
                        "Identifier",
                        "identifier",
                        "FoundFile",
                    )
                })?;
                this.emit_field("PropertyDeclarationSyntax", "Type", |this| {
                    this.emit_predefined_type(
                        "PropertyDeclarationSyntax",
                        "Type",
                        "string_keyword",
                        "string",
                    )
                })?;
                this.emit_field("PropertyDeclarationSyntax", "AccessorList", |this| {
                    this.emit_accessor_list_keywords(&["get"])
                })
            },
        )
    }

    fn emit_version_one_file_found_args_constructor(
        &mut self,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        self.emit_node(
            "ConstructorDeclarationSyntax",
            Some(("ClassDeclarationSyntax", "Members")),
            |this| {
                this.emit_field("ConstructorDeclarationSyntax", "Identifier", |this| {
                    this.emit_identifier_payload(
                        "ConstructorDeclarationSyntax",
                        "Identifier",
                        "identifier",
                        "FileFoundArgs",
                    )
                })?;
                this.emit_field("ConstructorDeclarationSyntax", "ParameterList", |this| {
                    this.emit_parameter_list_for_parent(
                        "ConstructorDeclarationSyntax",
                        "ParameterList",
                        &[("string", "fileName")],
                    )
                })?;
                this.emit_field("ConstructorDeclarationSyntax", "ExpressionBody", |this| {
                    this.emit_node(
                        "ArrowExpressionClauseSyntax",
                        Some(("ConstructorDeclarationSyntax", "ExpressionBody")),
                        |this| {
                            this.emit_field("ArrowExpressionClauseSyntax", "Expression", |this| {
                                this.emit_assignment_expression_for_parent(
                                    "ArrowExpressionClauseSyntax",
                                    "Expression",
                                    "FoundFile",
                                    "fileName",
                                )
                            })
                        },
                    )
                })
            },
        )
    }

    fn emit_version_one_event_field_declaration(
        &mut self,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        self.emit_node(
            "EventFieldDeclarationSyntax",
            Some(("ClassDeclarationSyntax", "Members")),
            |this| {
                this.emit_field("EventFieldDeclarationSyntax", "Declaration", |this| {
                    this.emit_node(
                        "VariableDeclarationSyntax",
                        Some(("EventFieldDeclarationSyntax", "Declaration")),
                        |this| {
                            this.emit_field("VariableDeclarationSyntax", "Type", |this| {
                                this.emit_nullable_event_handler_file_found_args_type(
                                    "VariableDeclarationSyntax",
                                    "Type",
                                )
                            })?;
                            this.emit_field("VariableDeclarationSyntax", "Variables", |this| {
                                this.emit_variable_declarator_identifier_only("FileFound")
                            })
                        },
                    )
                })
            },
        )
    }

    fn emit_version_one_search_method_declaration(
        &mut self,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        self.emit_node(
            "MethodDeclarationSyntax",
            Some(("ClassDeclarationSyntax", "Members")),
            |this| {
                this.emit_field("MethodDeclarationSyntax", "Identifier", |this| {
                    this.emit_identifier_payload(
                        "MethodDeclarationSyntax",
                        "Identifier",
                        "identifier",
                        "Search",
                    )
                })?;
                this.emit_field("MethodDeclarationSyntax", "ReturnType", |this| {
                    this.emit_predefined_type(
                        "MethodDeclarationSyntax",
                        "ReturnType",
                        "void_keyword",
                        "void",
                    )
                })?;
                this.emit_field("MethodDeclarationSyntax", "ParameterList", |this| {
                    this.emit_parameter_list_for_parent(
                        "MethodDeclarationSyntax",
                        "ParameterList",
                        &[("string", "directory"), ("string", "searchPattern")],
                    )
                })?;
                this.emit_field("MethodDeclarationSyntax", "Body", |this| {
                    this.emit_version_one_search_block()
                })
            },
        )
    }

    fn emit_version_one_search_block(&mut self) -> Result<(), SyntaxBackendBoundaryError> {
        self.emit_node(
            "BlockSyntax",
            Some(("MethodDeclarationSyntax", "Body")),
            |this| {
                this.emit_field("BlockSyntax", "Statements", |this| {
                    this.emit_version_one_foreach_statement()
                })
            },
        )
    }

    fn emit_version_one_foreach_statement(&mut self) -> Result<(), SyntaxBackendBoundaryError> {
        self.emit_node(
            "ForEachStatementSyntax",
            Some(("BlockSyntax", "Statements")),
            |this| {
                this.emit_field("ForEachStatementSyntax", "Type", |this| {
                    this.emit_identifier_name_syntax("ForEachStatementSyntax", "Type", "var")
                })?;
                this.emit_field("ForEachStatementSyntax", "Identifier", |this| {
                    this.emit_identifier_payload(
                        "ForEachStatementSyntax",
                        "Identifier",
                        "identifier",
                        "file",
                    )
                })?;
                this.emit_field("ForEachStatementSyntax", "Expression", |this| {
                    this.emit_directory_enumerate_files_invocation(
                        "ForEachStatementSyntax",
                        "Expression",
                    )
                })?;
                this.emit_field("ForEachStatementSyntax", "Statement", |this| {
                    this.emit_node(
                        "BlockSyntax",
                        Some(("ForEachStatementSyntax", "Statement")),
                        |this| {
                            this.emit_field("BlockSyntax", "Statements", |this| {
                                this.emit_version_one_file_found_conditional_invocation_statement()
                            })
                        },
                    )
                })
            },
        )
    }

    fn emit_directory_enumerate_files_invocation(
        &mut self,
        parent_rule: &str,
        parent_field: &str,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        self.emit_node(
            "InvocationExpressionSyntax",
            Some((parent_rule, parent_field)),
            |this| {
                this.emit_field("InvocationExpressionSyntax", "Expression", |this| {
                    this.emit_member_access_expression(
                        "InvocationExpressionSyntax",
                        "Expression",
                        &["Directory", "EnumerateFiles"],
                    )
                })?;
                this.emit_field("InvocationExpressionSyntax", "ArgumentList", |this| {
                    this.emit_argument_list(&["directory", "searchPattern"])
                })
            },
        )
    }

    fn emit_version_one_file_found_conditional_invocation_statement(
        &mut self,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        self.emit_node(
            "ExpressionStatementSyntax",
            Some(("BlockSyntax", "Statements")),
            |this| {
                this.emit_field("ExpressionStatementSyntax", "Expression", |this| {
                    this.emit_file_found_conditional_access_expression()
                })
            },
        )
    }

    fn emit_file_found_conditional_access_expression(
        &mut self,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        self.emit_node(
            "ConditionalAccessExpressionSyntax",
            Some(("ExpressionStatementSyntax", "Expression")),
            |this| {
                this.emit_field("ConditionalAccessExpressionSyntax", "Expression", |this| {
                    this.emit_identifier_name_syntax(
                        "ConditionalAccessExpressionSyntax",
                        "Expression",
                        "FileFound",
                    )
                })?;
                this.emit_field("ConditionalAccessExpressionSyntax", "WhenNotNull", |this| {
                    this.emit_node(
                        "InvocationExpressionSyntax",
                        Some(("ConditionalAccessExpressionSyntax", "WhenNotNull")),
                        |this| {
                            this.emit_field("InvocationExpressionSyntax", "Expression", |this| {
                                this.emit_member_binding_expression("Invoke")
                            })?;
                            this.emit_field("InvocationExpressionSyntax", "ArgumentList", |this| {
                                this.emit_argument_list_for_parent(
                                    "InvocationExpressionSyntax",
                                    "ArgumentList",
                                    |this| {
                                        this.emit_argument_with(|this| {
                                            this.emit_this_expression(
                                                "ArgumentSyntax",
                                                "Expression",
                                            )
                                        })?;
                                        this.emit_argument_with(|this| {
                                            this.emit_file_found_args_object_creation(
                                                "ArgumentSyntax",
                                                "Expression",
                                            )
                                        })
                                    },
                                )
                            })
                        },
                    )
                })
            },
        )
    }

    fn emit_member_binding_expression(
        &mut self,
        member_name: &str,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        self.emit_node(
            "MemberBindingExpressionSyntax",
            Some(("InvocationExpressionSyntax", "Expression")),
            |this| {
                this.emit_field("MemberBindingExpressionSyntax", "Name", |this| {
                    this.emit_identifier_name_syntax(
                        "MemberBindingExpressionSyntax",
                        "Name",
                        member_name,
                    )
                })
            },
        )
    }

    fn emit_this_expression(
        &mut self,
        parent_rule: &str,
        parent_field: &str,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        self.emit_node(
            "ThisExpressionSyntax",
            Some((parent_rule, parent_field)),
            |_| Ok(()),
        )
    }

    fn emit_file_found_args_object_creation(
        &mut self,
        parent_rule: &str,
        parent_field: &str,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        self.emit_node(
            "ObjectCreationExpressionSyntax",
            Some((parent_rule, parent_field)),
            |this| {
                this.emit_field("ObjectCreationExpressionSyntax", "Type", |this| {
                    this.emit_identifier_name_syntax(
                        "ObjectCreationExpressionSyntax",
                        "Type",
                        "FileFoundArgs",
                    )
                })?;
                this.emit_field("ObjectCreationExpressionSyntax", "ArgumentList", |this| {
                    this.emit_argument_list_for_parent(
                        "ObjectCreationExpressionSyntax",
                        "ArgumentList",
                        |this| this.emit_argument("file"),
                    )
                })
            },
        )
    }

    fn emit_logger_field_declaration(&mut self) -> Result<(), SyntaxBackendBoundaryError> {
        self.emit_node(
            "FieldDeclarationSyntax",
            Some(("ClassDeclarationSyntax", "Members")),
            |this| {
                this.emit_field("FieldDeclarationSyntax", "Declaration", |this| {
                    this.emit_node(
                        "VariableDeclarationSyntax",
                        Some(("FieldDeclarationSyntax", "Declaration")),
                        |this| {
                            this.emit_field("VariableDeclarationSyntax", "Type", |this| {
                                this.emit_nullable_action_string_type(
                                    "VariableDeclarationSyntax",
                                    "Type",
                                )
                            })?;
                            this.emit_field("VariableDeclarationSyntax", "Variables", |this| {
                                this.emit_variable_declarator("WriteMessage", None)
                            })
                        },
                    )
                })
            },
        )
    }

    fn emit_logger_property_declaration(&mut self) -> Result<(), SyntaxBackendBoundaryError> {
        self.emit_node(
            "PropertyDeclarationSyntax",
            Some(("ClassDeclarationSyntax", "Members")),
            |this| {
                this.emit_field("PropertyDeclarationSyntax", "Identifier", |this| {
                    this.emit_identifier_payload(
                        "PropertyDeclarationSyntax",
                        "Identifier",
                        "identifier",
                        "LogLevel",
                    )
                })?;
                this.emit_field("PropertyDeclarationSyntax", "Type", |this| {
                    this.emit_identifier_name_syntax(
                        "PropertyDeclarationSyntax",
                        "Type",
                        "Severity",
                    )
                })?;
                this.emit_field("PropertyDeclarationSyntax", "AccessorList", |this| {
                    this.emit_accessor_list_keywords(&["get", "set"])
                })?;
                this.emit_field("PropertyDeclarationSyntax", "Initializer", |this| {
                    this.emit_equals_value_clause(
                        "PropertyDeclarationSyntax",
                        "Initializer",
                        |this| {
                            this.emit_member_access_expression(
                                "EqualsValueClauseSyntax",
                                "Value",
                                &["Severity", "Warning"],
                            )
                        },
                    )
                })
            },
        )
    }

    fn emit_logger_method_declaration(&mut self) -> Result<(), SyntaxBackendBoundaryError> {
        self.emit_node(
            "MethodDeclarationSyntax",
            Some(("ClassDeclarationSyntax", "Members")),
            |this| {
                this.emit_field("MethodDeclarationSyntax", "Identifier", |this| {
                    this.emit_identifier_payload(
                        "MethodDeclarationSyntax",
                        "Identifier",
                        "identifier",
                        "LogMessage",
                    )
                })?;
                this.emit_field("MethodDeclarationSyntax", "ReturnType", |this| {
                    this.emit_predefined_type(
                        "MethodDeclarationSyntax",
                        "ReturnType",
                        "void_keyword",
                        "void",
                    )
                })?;
                this.emit_field("MethodDeclarationSyntax", "ParameterList", |this| {
                    this.emit_parameter_list_for_parent(
                        "MethodDeclarationSyntax",
                        "ParameterList",
                        CSHARP_LOGGER_METHOD_PARAMETERS,
                    )
                })?;
                this.emit_field("MethodDeclarationSyntax", "Body", |this| {
                    this.emit_logger_method_block()
                })
            },
        )
    }

    fn emit_using_directive(&mut self, using_name: &str) -> Result<(), SyntaxBackendBoundaryError> {
        self.emit_node(
            "UsingDirectiveSyntax",
            Some(("CompilationUnitSyntax", "Usings")),
            |this| {
                this.emit_field("UsingDirectiveSyntax", "Name", |this| {
                    let parts = using_name.split('.').collect::<Vec<_>>();
                    this.emit_name_syntax("UsingDirectiveSyntax", "Name", &parts)
                })
            },
        )
    }

    fn emit_namespace_declaration(&mut self) -> Result<(), SyntaxBackendBoundaryError> {
        self.emit_node(
            "NamespaceDeclarationSyntax",
            Some(("CompilationUnitSyntax", "Members")),
            |this| {
                this.emit_field("NamespaceDeclarationSyntax", "Name", |this| {
                    this.emit_identifier_payload(
                        "NamespaceDeclarationSyntax",
                        "Name",
                        "qualified_identifier",
                        "PatternMatching",
                    )
                })?;
                this.emit_field("NamespaceDeclarationSyntax", "Members", |this| {
                    for type_replay in CSHARP_COMPILATION_UNIT_SOURCE_FILE_TYPES {
                        this.emit_type_declaration(type_replay)?;
                    }
                    Ok(())
                })
            },
        )
    }

    fn emit_type_declaration(
        &mut self,
        type_replay: &CSharpCompilationUnitTypeReplay,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        let rule = match type_replay.kind {
            CSharpCompilationUnitTypeKind::Class => "ClassDeclarationSyntax",
            CSharpCompilationUnitTypeKind::Struct => "StructDeclarationSyntax",
        };
        self.emit_node(
            rule,
            Some(("NamespaceDeclarationSyntax", "Members")),
            |this| {
                this.emit_field(rule, "Identifier", |this| {
                    this.emit_identifier_payload(rule, "Identifier", "identifier", type_replay.name)
                })?;
                this.emit_field(rule, "Members", |this| {
                    for property in type_replay.properties {
                        this.emit_property_declaration(rule, property)?;
                    }
                    this.emit_constructor_declaration(rule, type_replay)
                })
            },
        )
    }

    fn emit_property_declaration(
        &mut self,
        parent_rule: &str,
        property_name: &str,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        self.emit_node(
            "PropertyDeclarationSyntax",
            Some((parent_rule, "Members")),
            |this| {
                this.emit_field("PropertyDeclarationSyntax", "Identifier", |this| {
                    this.emit_identifier_payload(
                        "PropertyDeclarationSyntax",
                        "Identifier",
                        "identifier",
                        property_name,
                    )
                })?;
                this.emit_field("PropertyDeclarationSyntax", "Type", |this| {
                    this.emit_predefined_double_type("PropertyDeclarationSyntax", "Type")
                })?;
                this.emit_field("PropertyDeclarationSyntax", "AccessorList", |this| {
                    this.emit_accessor_list()
                })
            },
        )
    }

    fn emit_accessor_list(&mut self) -> Result<(), SyntaxBackendBoundaryError> {
        self.emit_accessor_list_keywords(&["get"])
    }

    fn emit_accessor_list_keywords(
        &mut self,
        accessors: &[&str],
    ) -> Result<(), SyntaxBackendBoundaryError> {
        self.emit_node(
            "AccessorListSyntax",
            Some(("PropertyDeclarationSyntax", "AccessorList")),
            |this| {
                this.emit_field("AccessorListSyntax", "Accessors", |this| {
                    for accessor in accessors {
                        this.emit_accessor_declaration(accessor)?;
                    }
                    Ok(())
                })
            },
        )
    }

    fn emit_accessor_declaration(
        &mut self,
        accessor: &str,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        let node_kind = match accessor {
            "get" => "get_accessor_keyword",
            "set" => "set_accessor_keyword",
            other => {
                return Err(SyntaxBackendBoundaryError::new(
                    "csharp_compilation_unit_source_file_replay.accessor",
                    format!("unsupported C# accessor keyword {other}"),
                ));
            }
        };
        self.emit_node(
            "AccessorDeclarationSyntax",
            Some(("AccessorListSyntax", "Accessors")),
            |this| {
                this.emit_field("AccessorDeclarationSyntax", "Keyword", |this| {
                    this.emit_string_payload(
                        "AccessorDeclarationSyntax",
                        "Keyword",
                        node_kind,
                        accessor,
                    )
                })
            },
        )
    }

    fn emit_constructor_declaration(
        &mut self,
        parent_rule: &str,
        type_replay: &CSharpCompilationUnitTypeReplay,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        self.emit_node(
            "ConstructorDeclarationSyntax",
            Some((parent_rule, "Members")),
            |this| {
                this.emit_field("ConstructorDeclarationSyntax", "Identifier", |this| {
                    this.emit_identifier_payload(
                        "ConstructorDeclarationSyntax",
                        "Identifier",
                        "identifier",
                        type_replay.name,
                    )
                })?;
                this.emit_field("ConstructorDeclarationSyntax", "ParameterList", |this| {
                    this.emit_parameter_list(type_replay.parameters)
                })?;
                this.emit_field("ConstructorDeclarationSyntax", "Body", |this| {
                    this.emit_block(type_replay.assignments)
                })
            },
        )
    }

    fn emit_parameter_list(
        &mut self,
        parameters: &[(&str, &str)],
    ) -> Result<(), SyntaxBackendBoundaryError> {
        self.emit_parameter_list_for_parent(
            "ConstructorDeclarationSyntax",
            "ParameterList",
            parameters,
        )
    }

    fn emit_parameter_list_for_parent(
        &mut self,
        parent_rule: &str,
        parent_field: &str,
        parameters: &[(&str, &str)],
    ) -> Result<(), SyntaxBackendBoundaryError> {
        self.emit_node(
            "ParameterListSyntax",
            Some((parent_rule, parent_field)),
            |this| {
                this.emit_field("ParameterListSyntax", "Parameters", |this| {
                    for (parameter_type, parameter_name) in parameters {
                        this.emit_parameter(parameter_type, parameter_name)?;
                    }
                    Ok(())
                })
            },
        )
    }

    fn emit_parameter(
        &mut self,
        parameter_type: &str,
        parameter_name: &str,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        self.emit_node(
            "ParameterSyntax",
            Some(("ParameterListSyntax", "Parameters")),
            |this| {
                this.emit_field("ParameterSyntax", "Type", |this| {
                    this.emit_type_syntax("ParameterSyntax", "Type", parameter_type)
                })?;
                this.emit_field("ParameterSyntax", "Identifier", |this| {
                    this.emit_identifier_payload(
                        "ParameterSyntax",
                        "Identifier",
                        "identifier",
                        parameter_name,
                    )
                })
            },
        )
    }

    fn emit_logger_method_block(&mut self) -> Result<(), SyntaxBackendBoundaryError> {
        self.emit_node(
            "BlockSyntax",
            Some(("MethodDeclarationSyntax", "Body")),
            |this| {
                this.emit_field("BlockSyntax", "Statements", |this| {
                    this.emit_logger_if_statement()?;
                    this.emit_logger_local_declaration_statement()?;
                    this.emit_logger_invocation_statement()
                })
            },
        )
    }

    fn emit_logger_if_statement(&mut self) -> Result<(), SyntaxBackendBoundaryError> {
        self.emit_node(
            "IfStatementSyntax",
            Some(("BlockSyntax", "Statements")),
            |this| {
                this.emit_field("IfStatementSyntax", "Condition", |this| {
                    this.emit_parenthesized_expression("IfStatementSyntax", "Condition", |this| {
                        this.emit_logger_or_condition()
                    })
                })?;
                this.emit_field("IfStatementSyntax", "Statement", |this| {
                    this.emit_node(
                        "ReturnStatementSyntax",
                        Some(("IfStatementSyntax", "Statement")),
                        |_| Ok(()),
                    )
                })
            },
        )
    }

    fn emit_logger_or_condition(&mut self) -> Result<(), SyntaxBackendBoundaryError> {
        self.emit_binary_expression("ParenthesizedExpressionSyntax", "Expression", |this| {
            this.emit_field("BinaryExpressionSyntax", "Left", |this| {
                this.emit_parenthesized_expression("BinaryExpressionSyntax", "Left", |this| {
                    this.emit_binary_expression(
                        "ParenthesizedExpressionSyntax",
                        "Expression",
                        |this| {
                            this.emit_field("BinaryExpressionSyntax", "Left", |this| {
                                this.emit_identifier_name_syntax(
                                    "BinaryExpressionSyntax",
                                    "Left",
                                    "s",
                                )
                            })?;
                            this.emit_field("BinaryExpressionSyntax", "Right", |this| {
                                this.emit_identifier_name_syntax(
                                    "BinaryExpressionSyntax",
                                    "Right",
                                    "LogLevel",
                                )
                            })
                        },
                    )
                })
            })?;
            this.emit_field("BinaryExpressionSyntax", "Right", |this| {
                this.emit_parenthesized_expression("BinaryExpressionSyntax", "Right", |this| {
                    this.emit_is_pattern_expression()
                })
            })
        })
    }

    fn emit_is_pattern_expression(&mut self) -> Result<(), SyntaxBackendBoundaryError> {
        self.emit_node(
            "IsPatternExpressionSyntax",
            Some(("ParenthesizedExpressionSyntax", "Expression")),
            |this| {
                this.emit_field("IsPatternExpressionSyntax", "Expression", |this| {
                    this.emit_identifier_name_syntax(
                        "IsPatternExpressionSyntax",
                        "Expression",
                        "WriteMessage",
                    )
                })?;
                this.emit_field("IsPatternExpressionSyntax", "Pattern", |this| {
                    this.emit_constant_pattern_null()
                })
            },
        )
    }

    fn emit_constant_pattern_null(&mut self) -> Result<(), SyntaxBackendBoundaryError> {
        self.emit_node(
            "ConstantPatternSyntax",
            Some(("IsPatternExpressionSyntax", "Pattern")),
            |this| {
                this.emit_field("ConstantPatternSyntax", "Expression", |this| {
                    this.emit_node(
                        "LiteralExpressionSyntax",
                        Some(("ConstantPatternSyntax", "Expression")),
                        |this| {
                            this.emit_field("LiteralExpressionSyntax", "Token", |this| {
                                this.emit_identifier_payload(
                                    "LiteralExpressionSyntax",
                                    "Token",
                                    "null_literal",
                                    "null",
                                )
                            })
                        },
                    )
                })
            },
        )
    }

    fn emit_logger_local_declaration_statement(
        &mut self,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        self.emit_node(
            "LocalDeclarationStatementSyntax",
            Some(("BlockSyntax", "Statements")),
            |this| {
                this.emit_field("LocalDeclarationStatementSyntax", "Declaration", |this| {
                    this.emit_node(
                        "VariableDeclarationSyntax",
                        Some(("LocalDeclarationStatementSyntax", "Declaration")),
                        |this| {
                            this.emit_field("VariableDeclarationSyntax", "Type", |this| {
                                this.emit_identifier_name_syntax(
                                    "VariableDeclarationSyntax",
                                    "Type",
                                    "var",
                                )
                            })?;
                            this.emit_field("VariableDeclarationSyntax", "Variables", |this| {
                                this.emit_variable_declarator("outputMsg", Some("logger_message"))
                            })
                        },
                    )
                })
            },
        )
    }

    fn emit_logger_invocation_statement(&mut self) -> Result<(), SyntaxBackendBoundaryError> {
        self.emit_node(
            "ExpressionStatementSyntax",
            Some(("BlockSyntax", "Statements")),
            |this| {
                this.emit_field("ExpressionStatementSyntax", "Expression", |this| {
                    this.emit_node(
                        "InvocationExpressionSyntax",
                        Some(("ExpressionStatementSyntax", "Expression")),
                        |this| {
                            this.emit_field("InvocationExpressionSyntax", "Expression", |this| {
                                this.emit_identifier_name_syntax(
                                    "InvocationExpressionSyntax",
                                    "Expression",
                                    "WriteMessage",
                                )
                            })?;
                            this.emit_field("InvocationExpressionSyntax", "ArgumentList", |this| {
                                this.emit_argument_list(&["outputMsg"])
                            })
                        },
                    )
                })
            },
        )
    }

    fn emit_block(
        &mut self,
        assignments: &[(&str, &str)],
    ) -> Result<(), SyntaxBackendBoundaryError> {
        self.emit_node(
            "BlockSyntax",
            Some(("ConstructorDeclarationSyntax", "Body")),
            |this| {
                this.emit_field("BlockSyntax", "Statements", |this| {
                    for (left, right) in assignments {
                        this.emit_expression_statement_assignment(left, right)?;
                    }
                    Ok(())
                })
            },
        )
    }

    fn emit_expression_statement_assignment(
        &mut self,
        left: &str,
        right: &str,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        self.emit_node(
            "ExpressionStatementSyntax",
            Some(("BlockSyntax", "Statements")),
            |this| {
                this.emit_field("ExpressionStatementSyntax", "Expression", |this| {
                    this.emit_assignment_expression(left, right)
                })
            },
        )
    }

    fn emit_assignment_expression(
        &mut self,
        left: &str,
        right: &str,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        self.emit_assignment_expression_for_parent(
            "ExpressionStatementSyntax",
            "Expression",
            left,
            right,
        )
    }

    fn emit_assignment_expression_for_parent(
        &mut self,
        parent_rule: &str,
        parent_field: &str,
        left: &str,
        right: &str,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        self.emit_node(
            "AssignmentExpressionSyntax",
            Some((parent_rule, parent_field)),
            |this| {
                this.emit_field("AssignmentExpressionSyntax", "Left", |this| {
                    this.emit_identifier_name_syntax("AssignmentExpressionSyntax", "Left", left)
                })?;
                this.emit_field("AssignmentExpressionSyntax", "Right", |this| {
                    this.emit_identifier_name_syntax("AssignmentExpressionSyntax", "Right", right)
                })
            },
        )
    }

    fn emit_predefined_double_type(
        &mut self,
        parent_rule: &str,
        parent_field: &str,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        self.emit_predefined_type(parent_rule, parent_field, "double_keyword", "double")
    }

    fn emit_predefined_type(
        &mut self,
        parent_rule: &str,
        parent_field: &str,
        node_kind: &str,
        payload: &str,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        self.emit_node(
            "PredefinedTypeSyntax",
            Some((parent_rule, parent_field)),
            |this| {
                this.emit_field("PredefinedTypeSyntax", "Keyword", |this| {
                    this.emit_identifier_payload(
                        "PredefinedTypeSyntax",
                        "Keyword",
                        node_kind,
                        payload,
                    )
                })
            },
        )
    }

    fn emit_type_syntax(
        &mut self,
        parent_rule: &str,
        parent_field: &str,
        type_name: &str,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        match type_name {
            "double" => {
                self.emit_predefined_type(parent_rule, parent_field, "double_keyword", "double")
            }
            "string" => {
                self.emit_predefined_type(parent_rule, parent_field, "string_keyword", "string")
            }
            other => self.emit_identifier_name_syntax(parent_rule, parent_field, other),
        }
    }

    fn emit_nullable_action_string_type(
        &mut self,
        parent_rule: &str,
        parent_field: &str,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        self.emit_node(
            "NullableTypeSyntax",
            Some((parent_rule, parent_field)),
            |this| {
                this.emit_field("NullableTypeSyntax", "ElementType", |this| {
                    this.emit_node(
                        "GenericNameSyntax",
                        Some(("NullableTypeSyntax", "ElementType")),
                        |this| {
                            this.emit_field("GenericNameSyntax", "Identifier", |this| {
                                this.emit_identifier_payload(
                                    "GenericNameSyntax",
                                    "Identifier",
                                    "identifier",
                                    "Action",
                                )
                            })?;
                            this.emit_field("GenericNameSyntax", "TypeArgumentList", |this| {
                                this.emit_node(
                                    "TypeArgumentListSyntax",
                                    Some(("GenericNameSyntax", "TypeArgumentList")),
                                    |this| {
                                        this.emit_field(
                                            "TypeArgumentListSyntax",
                                            "Arguments",
                                            |this| {
                                                this.emit_predefined_type(
                                                    "TypeArgumentListSyntax",
                                                    "Arguments",
                                                    "string_keyword",
                                                    "string",
                                                )
                                            },
                                        )
                                    },
                                )
                            })
                        },
                    )
                })
            },
        )
    }

    fn emit_nullable_event_handler_file_found_args_type(
        &mut self,
        parent_rule: &str,
        parent_field: &str,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        self.emit_node(
            "NullableTypeSyntax",
            Some((parent_rule, parent_field)),
            |this| {
                this.emit_field("NullableTypeSyntax", "ElementType", |this| {
                    this.emit_node(
                        "GenericNameSyntax",
                        Some(("NullableTypeSyntax", "ElementType")),
                        |this| {
                            this.emit_field("GenericNameSyntax", "Identifier", |this| {
                                this.emit_identifier_payload(
                                    "GenericNameSyntax",
                                    "Identifier",
                                    "identifier",
                                    "EventHandler",
                                )
                            })?;
                            this.emit_field("GenericNameSyntax", "TypeArgumentList", |this| {
                                this.emit_node(
                                    "TypeArgumentListSyntax",
                                    Some(("GenericNameSyntax", "TypeArgumentList")),
                                    |this| {
                                        this.emit_field(
                                            "TypeArgumentListSyntax",
                                            "Arguments",
                                            |this| {
                                                this.emit_identifier_name_syntax(
                                                    "TypeArgumentListSyntax",
                                                    "Arguments",
                                                    "FileFoundArgs",
                                                )
                                            },
                                        )
                                    },
                                )
                            })
                        },
                    )
                })
            },
        )
    }

    fn emit_variable_declarator(
        &mut self,
        identifier: &str,
        initializer: Option<&str>,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        self.emit_node(
            "VariableDeclaratorSyntax",
            Some(("VariableDeclarationSyntax", "Variables")),
            |this| {
                this.emit_field("VariableDeclaratorSyntax", "Identifier", |this| {
                    this.emit_identifier_payload(
                        "VariableDeclaratorSyntax",
                        "Identifier",
                        "identifier",
                        identifier,
                    )
                })?;
                match initializer {
                    Some("logger_message") => {
                        this.emit_field("VariableDeclaratorSyntax", "Initializer", |this| {
                            this.emit_equals_value_clause(
                                "VariableDeclaratorSyntax",
                                "Initializer",
                                |this| this.emit_logger_interpolated_string(),
                            )
                        })
                    }
                    Some(other) => Err(SyntaxBackendBoundaryError::new(
                        "csharp_compilation_unit_source_file_replay.variable_initializer",
                        format!("unsupported C# variable initializer {other}"),
                    )),
                    None => this.emit_missing_field("VariableDeclaratorSyntax", "Initializer"),
                }
            },
        )
    }

    fn emit_variable_declarator_identifier_only(
        &mut self,
        identifier: &str,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        self.emit_node(
            "VariableDeclaratorSyntax",
            Some(("VariableDeclarationSyntax", "Variables")),
            |this| {
                this.emit_field("VariableDeclaratorSyntax", "Identifier", |this| {
                    this.emit_identifier_payload(
                        "VariableDeclaratorSyntax",
                        "Identifier",
                        "identifier",
                        identifier,
                    )
                })
            },
        )
    }

    fn emit_equals_value_clause<F>(
        &mut self,
        parent_rule: &str,
        parent_field: &str,
        emit_value: F,
    ) -> Result<(), SyntaxBackendBoundaryError>
    where
        F: FnOnce(&mut Self) -> Result<(), SyntaxBackendBoundaryError>,
    {
        self.emit_node(
            "EqualsValueClauseSyntax",
            Some((parent_rule, parent_field)),
            |this| this.emit_field("EqualsValueClauseSyntax", "Value", emit_value),
        )
    }

    fn emit_parenthesized_expression<F>(
        &mut self,
        parent_rule: &str,
        parent_field: &str,
        emit_expression: F,
    ) -> Result<(), SyntaxBackendBoundaryError>
    where
        F: FnOnce(&mut Self) -> Result<(), SyntaxBackendBoundaryError>,
    {
        self.emit_node(
            "ParenthesizedExpressionSyntax",
            Some((parent_rule, parent_field)),
            |this| {
                this.emit_field(
                    "ParenthesizedExpressionSyntax",
                    "Expression",
                    emit_expression,
                )
            },
        )
    }

    fn emit_binary_expression<F>(
        &mut self,
        parent_rule: &str,
        parent_field: &str,
        emit_fields: F,
    ) -> Result<(), SyntaxBackendBoundaryError>
    where
        F: FnOnce(&mut Self) -> Result<(), SyntaxBackendBoundaryError>,
    {
        self.emit_node(
            "BinaryExpressionSyntax",
            Some((parent_rule, parent_field)),
            emit_fields,
        )
    }

    fn emit_member_access_expression(
        &mut self,
        parent_rule: &str,
        parent_field: &str,
        parts: &[&str],
    ) -> Result<(), SyntaxBackendBoundaryError> {
        if parts.len() < 2 {
            return Err(SyntaxBackendBoundaryError::new(
                "csharp_compilation_unit_source_file_replay.member_access",
                "member access requires at least two identifier parts",
            ));
        }
        self.emit_node(
            "MemberAccessExpressionSyntax",
            Some((parent_rule, parent_field)),
            |this| {
                this.emit_field("MemberAccessExpressionSyntax", "Expression", |this| {
                    if parts.len() == 2 {
                        this.emit_identifier_name_syntax(
                            "MemberAccessExpressionSyntax",
                            "Expression",
                            parts[0],
                        )
                    } else {
                        this.emit_member_access_expression(
                            "MemberAccessExpressionSyntax",
                            "Expression",
                            &parts[..parts.len() - 1],
                        )
                    }
                })?;
                this.emit_field("MemberAccessExpressionSyntax", "Name", |this| {
                    this.emit_identifier_name_syntax(
                        "MemberAccessExpressionSyntax",
                        "Name",
                        parts[parts.len() - 1],
                    )
                })
            },
        )
    }

    fn emit_logger_interpolated_string(&mut self) -> Result<(), SyntaxBackendBoundaryError> {
        self.emit_node(
            "InterpolatedStringExpressionSyntax",
            Some(("EqualsValueClauseSyntax", "Value")),
            |this| {
                this.emit_field("InterpolatedStringExpressionSyntax", "Contents", |this| {
                    this.emit_interpolation_member_access(&["DateTime", "Now"])?;
                    this.emit_interpolated_text("\\t")?;
                    this.emit_interpolation_identifier("s")?;
                    this.emit_interpolated_text("\\t")?;
                    this.emit_interpolation_identifier("component")?;
                    this.emit_interpolated_text("\\t")?;
                    this.emit_interpolation_identifier("msg")
                })
            },
        )
    }

    fn emit_interpolated_text(&mut self, text: &str) -> Result<(), SyntaxBackendBoundaryError> {
        self.emit_node(
            "InterpolatedStringTextSyntax",
            Some(("InterpolatedStringExpressionSyntax", "Contents")),
            |this| {
                this.emit_field("InterpolatedStringTextSyntax", "TextToken", |this| {
                    this.emit_string_payload(
                        "InterpolatedStringTextSyntax",
                        "TextToken",
                        "interpolated_text",
                        text,
                    )
                })
            },
        )
    }

    fn emit_interpolation_identifier(
        &mut self,
        identifier: &str,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        self.emit_node(
            "InterpolationSyntax",
            Some(("InterpolatedStringExpressionSyntax", "Contents")),
            |this| {
                this.emit_field("InterpolationSyntax", "Expression", |this| {
                    this.emit_identifier_name_syntax(
                        "InterpolationSyntax",
                        "Expression",
                        identifier,
                    )
                })
            },
        )
    }

    fn emit_interpolation_member_access(
        &mut self,
        parts: &[&str],
    ) -> Result<(), SyntaxBackendBoundaryError> {
        self.emit_node(
            "InterpolationSyntax",
            Some(("InterpolatedStringExpressionSyntax", "Contents")),
            |this| {
                this.emit_field("InterpolationSyntax", "Expression", |this| {
                    this.emit_member_access_expression("InterpolationSyntax", "Expression", parts)
                })
            },
        )
    }

    fn emit_argument_list(
        &mut self,
        identifiers: &[&str],
    ) -> Result<(), SyntaxBackendBoundaryError> {
        self.emit_argument_list_for_parent("InvocationExpressionSyntax", "ArgumentList", |this| {
            for identifier in identifiers {
                this.emit_argument(identifier)?;
            }
            Ok(())
        })
    }

    fn emit_argument_list_for_parent<F>(
        &mut self,
        parent_rule: &str,
        parent_field: &str,
        emit_arguments: F,
    ) -> Result<(), SyntaxBackendBoundaryError>
    where
        F: FnOnce(&mut Self) -> Result<(), SyntaxBackendBoundaryError>,
    {
        self.emit_node(
            "ArgumentListSyntax",
            Some((parent_rule, parent_field)),
            |this| this.emit_field("ArgumentListSyntax", "Arguments", emit_arguments),
        )
    }

    fn emit_argument(&mut self, identifier: &str) -> Result<(), SyntaxBackendBoundaryError> {
        self.emit_argument_with(|this| {
            this.emit_identifier_name_syntax("ArgumentSyntax", "Expression", identifier)
        })
    }

    fn emit_argument_with<F>(
        &mut self,
        emit_expression: F,
    ) -> Result<(), SyntaxBackendBoundaryError>
    where
        F: FnOnce(&mut Self) -> Result<(), SyntaxBackendBoundaryError>,
    {
        self.emit_node(
            "ArgumentSyntax",
            Some(("ArgumentListSyntax", "Arguments")),
            |this| this.emit_field("ArgumentSyntax", "Expression", emit_expression),
        )
    }

    fn emit_name_syntax(
        &mut self,
        parent_rule: &str,
        parent_field: &str,
        parts: &[&str],
    ) -> Result<(), SyntaxBackendBoundaryError> {
        if parts.is_empty() {
            return Err(SyntaxBackendBoundaryError::new(
                "csharp_compilation_unit_source_file_replay.name",
                "C# name replay requires at least one identifier part",
            ));
        }
        if parts.len() == 1 {
            return self.emit_identifier_name_syntax(parent_rule, parent_field, parts[0]);
        }
        self.emit_node(
            "QualifiedNameSyntax",
            Some((parent_rule, parent_field)),
            |this| {
                this.emit_field("QualifiedNameSyntax", "Left", |this| {
                    this.emit_name_syntax("QualifiedNameSyntax", "Left", &parts[..parts.len() - 1])
                })?;
                this.emit_field("QualifiedNameSyntax", "Right", |this| {
                    this.emit_identifier_name_syntax(
                        "QualifiedNameSyntax",
                        "Right",
                        parts[parts.len() - 1],
                    )
                })
            },
        )
    }

    fn emit_identifier_name_syntax(
        &mut self,
        parent_rule: &str,
        parent_field: &str,
        identifier: &str,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        self.emit_node(
            "IdentifierNameSyntax",
            Some((parent_rule, parent_field)),
            |this| {
                this.emit_field("IdentifierNameSyntax", "Identifier", |this| {
                    this.emit_identifier_payload(
                        "IdentifierNameSyntax",
                        "Identifier",
                        "identifier",
                        identifier,
                    )
                })
            },
        )
    }

    fn emit_node<F>(
        &mut self,
        rule: &str,
        parent_field: Option<(&str, &str)>,
        emit_fields: F,
    ) -> Result<(), SyntaxBackendBoundaryError>
    where
        F: FnOnce(&mut Self) -> Result<(), SyntaxBackendBoundaryError>,
    {
        let record = structural_record_by_rule(self.sidecar, rule)?;
        let record_rule = record.grammar_rule.clone();
        let record_type = record.record_type.clone();
        let node_start = match parent_field {
            Some((owner_rule, owner_field)) => structural_child_node_start_token(
                owner_rule,
                owner_field,
                &record_type,
                self.facts,
                self.roles,
            )?,
            None => self.roles.node_start,
        };
        self.builder.push_segment(StructuralReplaySegment {
            event_kind: "node_start",
            primary_token_range_role: &self.roles.structural_range.role,
            token_ids: &[node_start],
            owner_rule: Some(&record_rule),
            owner_field: None,
            node_kind: Some(&record_type),
            payload_kind: None,
        });
        emit_fields(self)?;
        self.builder.push_segment(StructuralReplaySegment {
            event_kind: "node_end",
            primary_token_range_role: &self.roles.structural_range.role,
            token_ids: &[self.roles.node_end],
            owner_rule: Some(&record_rule),
            owner_field: None,
            node_kind: Some(&record_type),
            payload_kind: None,
        });
        Ok(())
    }

    fn emit_field<F>(
        &mut self,
        owner_rule: &str,
        owner_field: &str,
        emit_values: F,
    ) -> Result<(), SyntaxBackendBoundaryError>
    where
        F: FnOnce(&mut Self) -> Result<(), SyntaxBackendBoundaryError>,
    {
        ensure_structural_field(self.sidecar, owner_rule, owner_field)?;
        self.builder.push_segment(StructuralReplaySegment {
            event_kind: "field_start",
            primary_token_range_role: &self.roles.structural_range.role,
            token_ids: &[self.roles.field_start],
            owner_rule: Some(owner_rule),
            owner_field: Some(owner_field),
            node_kind: None,
            payload_kind: None,
        });
        emit_values(self)?;
        self.builder.push_segment(StructuralReplaySegment {
            event_kind: "field_end",
            primary_token_range_role: &self.roles.structural_range.role,
            token_ids: &[self.roles.field_end],
            owner_rule: Some(owner_rule),
            owner_field: Some(owner_field),
            node_kind: None,
            payload_kind: None,
        });
        Ok(())
    }

    fn emit_missing_field(
        &mut self,
        owner_rule: &str,
        owner_field: &str,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        ensure_structural_field(self.sidecar, owner_rule, owner_field)?;
        self.builder.push_segment(StructuralReplaySegment {
            event_kind: "field_start",
            primary_token_range_role: &self.roles.structural_range.role,
            token_ids: &[self.roles.field_start],
            owner_rule: Some(owner_rule),
            owner_field: Some(owner_field),
            node_kind: None,
            payload_kind: None,
        });
        self.builder.push_segment(StructuralReplaySegment {
            event_kind: "field_missing",
            primary_token_range_role: &self.roles.structural_range.role,
            token_ids: &[self.roles.field_missing],
            owner_rule: Some(owner_rule),
            owner_field: Some(owner_field),
            node_kind: None,
            payload_kind: None,
        });
        self.builder.push_segment(StructuralReplaySegment {
            event_kind: "field_end",
            primary_token_range_role: &self.roles.structural_range.role,
            token_ids: &[self.roles.field_end],
            owner_rule: Some(owner_rule),
            owner_field: Some(owner_field),
            node_kind: None,
            payload_kind: None,
        });
        Ok(())
    }

    fn emit_identifier_payload(
        &mut self,
        owner_rule: &str,
        owner_field: &str,
        node_kind: &str,
        payload: &str,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        self.emit_utf8_payload(
            owner_rule,
            owner_field,
            node_kind,
            GrammarTerminalKind::IdentifierPayload,
            "identifier_payload",
            payload,
        )
    }

    fn emit_string_payload(
        &mut self,
        owner_rule: &str,
        owner_field: &str,
        node_kind: &str,
        payload: &str,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        self.emit_utf8_payload(
            owner_rule,
            owner_field,
            node_kind,
            GrammarTerminalKind::StringPayload,
            "string_payload",
            payload,
        )
    }

    fn emit_integer_payload(
        &mut self,
        owner_rule: &str,
        owner_field: &str,
        node_kind: &str,
        value: u8,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        require_terminal_alternative(
            self.sidecar,
            owner_rule,
            owner_field,
            node_kind,
            GrammarTerminalKind::IntegerPayload,
        )?;
        let payload_kind = format!("integer_payload:{node_kind}");
        let digits = crate::syntax_state_machine::integer_codec_encode_digits(u32::from(value))
            .map_err(|msg| {
                SyntaxBackendBoundaryError::new("syntax_state_machine_replay.integer_encode", msg)
            })?;
        let mut token_ids = Vec::with_capacity(digits.len() + 2);
        token_ids.push(self.roles.terminal);
        for digit in digits {
            token_ids.push(self.roles.integer_range.start + u32::from(digit));
        }
        token_ids.push(self.roles.payload_end);
        self.builder.push_segment(StructuralReplaySegment {
            event_kind: "terminal",
            primary_token_range_role: &self.roles.integer_range.role,
            token_ids: &token_ids,
            owner_rule: Some(owner_rule),
            owner_field: Some(owner_field),
            node_kind: Some(node_kind),
            payload_kind: Some(&payload_kind),
        });
        Ok(())
    }

    fn emit_utf8_payload(
        &mut self,
        owner_rule: &str,
        owner_field: &str,
        node_kind: &str,
        terminal: GrammarTerminalKind,
        payload_kind_prefix: &str,
        payload: &str,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        require_terminal_alternative(self.sidecar, owner_rule, owner_field, node_kind, terminal)?;
        let payload_kind = format!("{payload_kind_prefix}:{node_kind}");
        let token_ids = utf8_payload_tokens(self.roles, payload.as_bytes());
        self.builder.push_segment(StructuralReplaySegment {
            event_kind: "terminal",
            primary_token_range_role: &self.roles.utf8_range.role,
            token_ids: &token_ids,
            owner_rule: Some(owner_rule),
            owner_field: Some(owner_field),
            node_kind: Some(node_kind),
            payload_kind: Some(&payload_kind),
        });
        Ok(())
    }
}

fn ensure_structural_field<'a>(
    sidecar: &'a GrammarStructuralModelSidecar,
    owner_rule: &str,
    owner_field: &str,
) -> Result<&'a GrammarStructuralField, SyntaxBackendBoundaryError> {
    let record = structural_record_by_rule(sidecar, owner_rule)?;
    record
        .fields
        .iter()
        .find(|field| field.name == owner_field)
        .ok_or_else(|| {
            SyntaxBackendBoundaryError::new(
                "csharp_compilation_unit_source_file_replay.field",
                format!("missing StructuralModel field {owner_rule}.{owner_field}"),
            )
        })
}

fn require_terminal_alternative(
    sidecar: &GrammarStructuralModelSidecar,
    owner_rule: &str,
    owner_field: &str,
    node_kind: &str,
    terminal: GrammarTerminalKind,
) -> Result<(), SyntaxBackendBoundaryError> {
    let domain = finite_domain_by_field(sidecar, owner_rule, owner_field)?;
    if domain
        .alternatives
        .iter()
        .any(|alternative| alternative.node_kind == node_kind && alternative.terminal == terminal)
    {
        return Ok(());
    }
    Err(SyntaxBackendBoundaryError::new(
        "csharp_compilation_unit_source_file_replay.terminal",
        format!(
            "missing terminal alternative {owner_rule}.{owner_field} -> {node_kind} {terminal:?}"
        ),
    ))
}

fn structural_child_node_start_token(
    owner_rule: &str,
    owner_field: &str,
    node_kind: &str,
    facts: &ModelFacts,
    roles: &ReplayTokenRoles,
) -> Result<u32, SyntaxBackendBoundaryError> {
    if !facts
        .multi_alternative_nonterminal_fields
        .contains(&(owner_rule.to_string(), owner_field.to_string()))
    {
        return Ok(roles.node_start);
    }
    roles
        .node_start_alternative_tokens
        .get(node_kind)
        .copied()
        .ok_or_else(|| {
            SyntaxBackendBoundaryError::new(
                "structural_replay.synthetic_stream.alternative_node_start",
                format!(
                    "missing alternative node_start token for {owner_rule}.{owner_field} child {node_kind}"
                ),
            )
        })
}

fn structural_record_by_rule<'a>(
    sidecar: &'a GrammarStructuralModelSidecar,
    rule: &str,
) -> Result<&'a GrammarStructuralRecord, SyntaxBackendBoundaryError> {
    sidecar
        .structural_model
        .records
        .iter()
        .find(|record| record.grammar_rule == rule)
        .ok_or_else(|| {
            SyntaxBackendBoundaryError::new(
                "csharp_state_machine_parity.synthetic_stream.record",
                format!("missing StructuralModel record for rule {rule}"),
            )
        })
}

fn target_rule_for_node_kind<'a>(
    sidecar: &'a GrammarStructuralModelSidecar,
    owner_rule: &str,
    owner_field: &str,
    target_node_kind: &str,
) -> Result<&'a str, SyntaxBackendBoundaryError> {
    sidecar
        .structural_model
        .references
        .iter()
        .find(|reference| {
            reference.owner_rule == owner_rule
                && reference.field == owner_field
                && reference.target_node_kind == target_node_kind
        })
        .map(|reference| reference.target_rule.as_str())
        .ok_or_else(|| {
            SyntaxBackendBoundaryError::new(
                "structural_replay.synthetic_stream.reference",
                format!(
                    "missing StructuralModel reference for {owner_rule}.{owner_field} -> {target_node_kind}"
                ),
            )
        })
}

fn source_instance_structural_model_sidecar(
    report: &SyntaxGoSourceInstanceProjectionReport,
) -> Result<GrammarStructuralModelSidecar, SyntaxBackendBoundaryError> {
    if report.candidate_grammar_id == GO_UNIFIED_GRAMMAR_ID {
        // Unified inventory route: replay validates against the one pinned
        // inventory-derived grammar artifact.
        return Ok(go_unified_grammar_artifact()?.sidecar.clone());
    }
    let grammar = GoSyntaxMetadataAdapter::new(&report.candidate_grammar_id)
        .lower_metadata_file(Path::new(&report.metadata.path))
        .map_err(|error| {
            SyntaxBackendBoundaryError::new(
                "replay.metadata",
                format!("failed to lower Go syntax metadata: {error}"),
            )
        })?;
    lower_grammar_ir_to_structural_model(&grammar, go_syntax_source_file_token_layout()).map_err(
        |error| {
            SyntaxBackendBoundaryError::new(
                "replay.structural_model",
                format!("failed to lower GrammarIR to StructuralModel: {error}"),
            )
        },
    )
}

fn validate_source_instance_sidecar(
    sidecar: &GrammarStructuralModelSidecar,
    report: &SyntaxGoSourceInstanceProjectionReport,
) -> Result<(), SyntaxBackendBoundaryError> {
    require_string(
        "replay.sidecar.grammar_id",
        &sidecar.input.grammar_id,
        &report.candidate_grammar_id,
    )?;
    require_string(
        "replay.sidecar.metadata_path",
        sidecar
            .input
            .provenance
            .metadata_path
            .as_deref()
            .unwrap_or(""),
        &report.metadata.path,
    )?;
    require_string(
        "replay.sidecar.metadata_sha256",
        sidecar
            .input
            .provenance
            .metadata_sha256
            .as_deref()
            .unwrap_or(""),
        &report.metadata.sha256,
    )?;
    require_usize(
        "replay.sidecar.records",
        sidecar.structural_model.records.len(),
        report.metadata.rule_count,
    )?;
    let field_count = sidecar
        .structural_model
        .records
        .iter()
        .map(|record| record.fields.len())
        .sum();
    require_usize(
        "replay.sidecar.fields",
        field_count,
        report.metadata.field_count,
    )?;
    require_token_range(
        "replay.sidecar.token_layout.structural_decisions",
        &sidecar.structural_model.token_ranges.ranges,
        "structural_decisions",
    )?;
    require_token_range(
        "replay.sidecar.token_layout.utf8_byte_payload",
        &sidecar.structural_model.token_ranges.ranges,
        "utf8_byte_payload",
    )?;
    require_token_range(
        "replay.sidecar.token_layout.integer_lexeme_payload",
        &sidecar.structural_model.token_ranges.ranges,
        "integer_lexeme_payload",
    )
}

fn layout_range_by_role(
    layout: &ProjectionTokenLayoutSummary,
    role: &str,
) -> Result<ProjectionTokenLayoutRangeSummary, SyntaxBackendBoundaryError> {
    layout
        .ranges
        .iter()
        .find(|range| range.role == role)
        .cloned()
        .ok_or_else(|| {
            SyntaxBackendBoundaryError::new(
                "replay.token_layout.range",
                format!("missing token-layout role {role}"),
            )
        })
}

fn layout_range_by_role_any(
    layout: &ProjectionTokenLayoutSummary,
    roles: &[&str],
) -> Result<ProjectionTokenLayoutRangeSummary, SyntaxBackendBoundaryError> {
    roles
        .iter()
        .find_map(|role| layout.ranges.iter().find(|range| range.role == *role))
        .cloned()
        .ok_or_else(|| {
            SyntaxBackendBoundaryError::new(
                "replay.token_layout.range",
                format!("missing token-layout role in {roles:?}"),
            )
        })
}

fn validate_projection_token_layout(
    layout: &ProjectionTokenLayoutSummary,
) -> Result<(), SyntaxBackendBoundaryError> {
    require_positive_usize(
        "generated_codec_state_api.token_layout.ranges",
        layout.ranges.len(),
    )?;
    if layout.pad_token >= layout.vocab_size {
        return Err(SyntaxBackendBoundaryError::new(
            "generated_codec_state_api.token_layout.pad_token",
            "PAD_TOKEN must be below VOCAB_SIZE",
        ));
    }
    for range in &layout.ranges {
        validate_projection_token_layout_range(range)?;
        if range.end_exclusive > layout.vocab_size {
            return Err(SyntaxBackendBoundaryError::new(
                "generated_codec_state_api.token_layout.ranges",
                "token-layout range must remain below VOCAB_SIZE",
            ));
        }
    }
    layout_range_by_role(layout, "structural_decisions")?;
    layout_range_by_role(layout, "utf8_byte_payload")?;
    layout_range_by_role_any(layout, &["integer_lexeme_payload", "u8_integer_payload"])?;
    Ok(())
}

fn validate_projection_token_layout_range(
    range: &ProjectionTokenLayoutRangeSummary,
) -> Result<(), SyntaxBackendBoundaryError> {
    require_non_empty("generated_codec_state_api.token_layout.range.id", &range.id)?;
    require_non_empty(
        "generated_codec_state_api.token_layout.range.role",
        &range.role,
    )?;
    if range.start >= range.end_exclusive {
        return Err(SyntaxBackendBoundaryError::new(
            "generated_codec_state_api.token_layout.range",
            "token-layout range start must be below end_exclusive",
        ));
    }
    Ok(())
}

fn require_token_range(
    field: &'static str,
    ranges: &[GrammarStructuralTokenRange],
    role: &str,
) -> Result<(), SyntaxBackendBoundaryError> {
    if ranges.iter().any(|range| range.role == role) {
        Ok(())
    } else {
        Err(SyntaxBackendBoundaryError::new(
            field,
            format!("missing token-layout role {role}"),
        ))
    }
}

fn require_token_range_any(
    field: &'static str,
    ranges: &[GrammarStructuralTokenRange],
    roles: &[&str],
) -> Result<(), SyntaxBackendBoundaryError> {
    if ranges
        .iter()
        .any(|range| roles.iter().any(|role| range.role == *role))
    {
        Ok(())
    } else {
        Err(SyntaxBackendBoundaryError::new(
            field,
            format!("missing token-layout role in {roles:?}"),
        ))
    }
}

fn validate_raw_token_segments(
    segments: &[ProjectionRawTokenSegment],
    raw_token_count: usize,
) -> Result<(), SyntaxBackendBoundaryError> {
    let mut expected_offset = 0;
    for (expected_ordinal, segment) in segments.iter().enumerate() {
        require_usize(
            "csharp_structural_replay.raw_token_segments.event_ordinal",
            segment.event_ordinal,
            expected_ordinal,
        )?;
        require_non_empty(
            "csharp_structural_replay.raw_token_segments.event_kind",
            &segment.event_kind,
        )?;
        require_non_empty(
            "csharp_structural_replay.raw_token_segments.primary_token_range_role",
            &segment.primary_token_range_role,
        )?;
        require_usize(
            "csharp_structural_replay.raw_token_segments.raw_token_offset",
            segment.raw_token_offset,
            expected_offset,
        )?;
        if segment.raw_token_count == 0 {
            return Err(SyntaxBackendBoundaryError::new(
                "csharp_structural_replay.raw_token_segments.raw_token_count",
                "raw token segments must be non-empty",
            ));
        }
        expected_offset += segment.raw_token_count;
    }
    require_usize(
        "csharp_structural_replay.raw_token_segments.coverage",
        expected_offset,
        raw_token_count,
    )
}

fn required_payload_segment_value<'a>(
    value: Option<&'a str>,
    field: &'static str,
) -> Result<&'a str, SyntaxBackendBoundaryError> {
    value.ok_or_else(|| {
        SyntaxBackendBoundaryError::new(
            "generated_codec_state_api.payload_range_case",
            format!("{field} is missing from terminal segment annotations"),
        )
    })
}

fn require_string(
    field: &'static str,
    actual: &str,
    expected: &str,
) -> Result<(), SyntaxBackendBoundaryError> {
    if actual == expected {
        Ok(())
    } else {
        Err(SyntaxBackendBoundaryError::new(
            field,
            format!("expected {expected}, got {actual}"),
        ))
    }
}

fn require_contains(
    field: &'static str,
    actual: &str,
    expected_substring: &str,
) -> Result<(), SyntaxBackendBoundaryError> {
    if actual.contains(expected_substring) {
        Ok(())
    } else {
        Err(SyntaxBackendBoundaryError::new(
            field,
            format!("expected value to contain {expected_substring:?}"),
        ))
    }
}

fn require_non_empty(field: &'static str, value: &str) -> Result<(), SyntaxBackendBoundaryError> {
    if value.is_empty() {
        Err(SyntaxBackendBoundaryError::new(
            field,
            "value must not be empty",
        ))
    } else {
        Ok(())
    }
}

fn require_sha256(field: &'static str, value: &str) -> Result<(), SyntaxBackendBoundaryError> {
    if value.len() == 64 && value.chars().all(|c| c.is_ascii_hexdigit()) {
        Ok(())
    } else {
        Err(SyntaxBackendBoundaryError::new(
            field,
            "value must be a SHA-256 hex digest",
        ))
    }
}

fn require_u32(
    field: &'static str,
    actual: u32,
    expected: u32,
) -> Result<(), SyntaxBackendBoundaryError> {
    if actual == expected {
        Ok(())
    } else {
        Err(SyntaxBackendBoundaryError::new(
            field,
            format!("expected {expected}, got {actual}"),
        ))
    }
}

fn require_usize(
    field: &'static str,
    actual: usize,
    expected: usize,
) -> Result<(), SyntaxBackendBoundaryError> {
    if actual == expected {
        Ok(())
    } else {
        Err(SyntaxBackendBoundaryError::new(
            field,
            format!("expected {expected}, got {actual}"),
        ))
    }
}

fn require_positive_usize(
    field: &'static str,
    actual: usize,
) -> Result<(), SyntaxBackendBoundaryError> {
    if actual > 0 {
        Ok(())
    } else {
        Err(SyntaxBackendBoundaryError::new(
            field,
            "expected positive value",
        ))
    }
}

fn require_bool(
    field: &'static str,
    actual: bool,
    expected: bool,
) -> Result<(), SyntaxBackendBoundaryError> {
    if actual == expected {
        Ok(())
    } else {
        Err(SyntaxBackendBoundaryError::new(
            field,
            format!("expected {expected}, got {actual}"),
        ))
    }
}

fn validate_sorted_unique_tokens(
    field: &'static str,
    tokens: &[u32],
    vocab_size: u32,
) -> Result<(), SyntaxBackendBoundaryError> {
    let mut previous = None;
    for token in tokens {
        if *token >= vocab_size {
            return Err(SyntaxBackendBoundaryError::new(
                field,
                format!("token {token} is >= VOCAB_SIZE {vocab_size}"),
            ));
        }
        if previous.is_some_and(|previous| previous >= *token) {
            return Err(SyntaxBackendBoundaryError::new(
                field,
                "tokens must be sorted and unique",
            ));
        }
        previous = Some(*token);
    }
    Ok(())
}

fn require_all(
    field: &'static str,
    actual: &[String],
    expected: &[&str],
) -> Result<(), SyntaxBackendBoundaryError> {
    let actual = actual.iter().map(String::as_str).collect::<BTreeSet<_>>();
    for required in expected {
        if !actual.contains(required) {
            return Err(SyntaxBackendBoundaryError::new(
                field,
                format!("missing required value {required}"),
            ));
        }
    }
    Ok(())
}

fn require_exact_strings(
    field: &'static str,
    actual: &[String],
    expected: &[String],
) -> Result<(), SyntaxBackendBoundaryError> {
    if actual == expected {
        Ok(())
    } else {
        Err(SyntaxBackendBoundaryError::new(
            field,
            format!("expected {expected:?}, got {actual:?}"),
        ))
    }
}
