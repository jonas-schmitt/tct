//! Go real-source generated-codec corpus completion handoff gate.
//!
//! This gate consumes the completed checked Go real-source generated-codec
//! corpus, including its source-program corpus handoff inputs, and records the
//! next stable-promotion prerequisite. It keeps the completed corpus as
//! sidecar-only syntax evidence and does not promote source-program,
//! whole-file, paper, model-quality, type-correctness, arena IR, generated
//! hot-path, or JSON Schema output claims.

use crate::syntax_backend_boundary::SyntaxBackendBoundaryError;
use crate::syntax_final_promotion_decision::{
    ARTIFACT_KIND as FINAL_PROMOTION_ARTIFACT_KIND,
    COMPLETED_PROMOTION_CLASS as FINAL_PROMOTION_COMPLETED_CLASS, DECISION,
    FIXTURE_PATH as FINAL_PROMOTION_FIXTURE_PATH,
    NEXT_MISSING_STABLE_PROMOTION_CLASS as STABLE_METADATA_PROMOTION_IMPLEMENTATION_CLASS,
    SyntaxFinalStablePromotionDecisionGate, TOOL as FINAL_PROMOTION_TOOL,
    VERSION as FINAL_PROMOTION_VERSION,
};
use crate::syntax_go_real_source_generated_codec_corpus::{
    ARTIFACT_KIND as CORPUS_ARTIFACT_KIND, COMPLETED_PROMOTION_CLASS as CORPUS_COMPLETED_CLASS,
    CRYPTO_SYSRAND_RAND_WINDOWS_GENERATED_CODEC_MARKER,
    CRYPTO_SYSRAND_RAND_WINDOWS_GENERATED_CODEC_REPORT_COMMAND,
    CRYPTO_SYSRAND_RAND_WINDOWS_GENERATED_CODEC_REPORT_PATH,
    CRYPTO_SYSRAND_RAND_WINDOWS_GENERATED_CODEC_ROUTE_ID,
    CRYPTO_SYSRAND_RAND_WINDOWS_MANIFEST_PATH, FIXTURE_PATH as CORPUS_FIXTURE_PATH,
    GENERATED_CODEC_MARKER, GENERATED_CODEC_REPORT_COMMAND, GENERATED_CODEC_REPORT_PATH,
    GENERATED_CODEC_ROUTE_ID, GeneratedCodecCorpusSupportedRecord,
    NEXT_MISSING_STABLE_PROMOTION_CLASS as CORPUS_NEXT_CLASS, PATH_MATCH_GENERATED_CODEC_MARKER,
    PATH_MATCH_GENERATED_CODEC_REPORT_COMMAND, PATH_MATCH_GENERATED_CODEC_REPORT_PATH,
    PATH_MATCH_GENERATED_CODEC_ROUTE_ID, PATH_MATCH_MANIFEST_PATH,
    SORT_INTERFACE_GENERATED_CODEC_MARKER, SORT_INTERFACE_GENERATED_CODEC_REPORT_COMMAND,
    SORT_INTERFACE_GENERATED_CODEC_REPORT_PATH, SORT_INTERFACE_GENERATED_CODEC_ROUTE_ID,
    SORT_INTERFACE_MANIFEST_PATH, SORT_SEARCH_GENERATED_CODEC_MARKER,
    SORT_SEARCH_GENERATED_CODEC_REPORT_COMMAND, SORT_SEARCH_GENERATED_CODEC_REPORT_PATH,
    SORT_SEARCH_GENERATED_CODEC_ROUTE_ID, SORT_SEARCH_MANIFEST_PATH,
    STRINGS_CLONE_GENERATED_CODEC_MARKER, STRINGS_CLONE_GENERATED_CODEC_REPORT_COMMAND,
    STRINGS_CLONE_GENERATED_CODEC_REPORT_PATH, STRINGS_CLONE_GENERATED_CODEC_ROUTE_ID,
    STRINGS_CLONE_MANIFEST_PATH, STRINGS_SEARCH_GENERATED_CODEC_MARKER,
    STRINGS_SEARCH_GENERATED_CODEC_REPORT_COMMAND, STRINGS_SEARCH_GENERATED_CODEC_REPORT_PATH,
    STRINGS_SEARCH_GENERATED_CODEC_ROUTE_ID, STRINGS_SEARCH_MANIFEST_PATH, SUPPORTED_MANIFEST_PATH,
    SyntaxGoRealSourceGeneratedCodecCorpusGate, TOOL as CORPUS_TOOL, VERSION as CORPUS_VERSION,
    WHOLE_FILE_GENERATED_CODEC_CLAIM, WHOLE_FILE_GENERATED_CODEC_CLAIM_SCOPE,
};
use crate::syntax_source_program_handoff::{
    SYNTAX_LEVEL_BOUNDARY, SyntaxSourceProgramGoGeneratedCodecCompletionHandoffInput,
    SyntaxSourceProgramStableMetadataPromotionInput, TYPECHECK_PRESERVATION_REPORT_STABLE_FIELDS,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const TOOL: &str = "syntax_backend_go_real_source_generated_codec_completion_handoff_gate";
pub const VERSION: u32 = 1;
pub const ARTIFACT_KIND: &str =
    "tct_syntax_backend_go_real_source_generated_codec_completion_handoff_gate";
pub const FIXTURE_PATH: &str =
    "tests/fixtures/syntax_backend/go-real-source-generated-codec-completion-handoff-v1.json";
pub const STATUS: &str = "go_real_source_generated_codec_completion_handoff";
pub const HANDOFF_ID: &str = "go-real-source-generated-codec-completion-handoff-v1";
pub const COMPLETED_PROMOTION_CLASS: &str =
    "syntax backend Go real-source generated-codec corpus completion handoff gate";
pub const NEXT_MISSING_STABLE_PROMOTION_CLASS: &str =
    STABLE_METADATA_PROMOTION_IMPLEMENTATION_CLASS;

const REQUIRED_RECORD_COUNT: usize = 7;
const REQUIRED_BLOCKED_RECORD_COUNT: usize = 0;

const REQUIRED_SUPPORTED_MANIFEST_PATHS: &[&str] = &[
    SUPPORTED_MANIFEST_PATH,
    CRYPTO_SYSRAND_RAND_WINDOWS_MANIFEST_PATH,
    STRINGS_CLONE_MANIFEST_PATH,
    SORT_INTERFACE_MANIFEST_PATH,
    SORT_SEARCH_MANIFEST_PATH,
    STRINGS_SEARCH_MANIFEST_PATH,
    PATH_MATCH_MANIFEST_PATH,
];

const REQUIRED_GENERATED_CODEC_ROUTES: &[&str] = &[
    GENERATED_CODEC_ROUTE_ID,
    CRYPTO_SYSRAND_RAND_WINDOWS_GENERATED_CODEC_ROUTE_ID,
    STRINGS_CLONE_GENERATED_CODEC_ROUTE_ID,
    SORT_INTERFACE_GENERATED_CODEC_ROUTE_ID,
    SORT_SEARCH_GENERATED_CODEC_ROUTE_ID,
    STRINGS_SEARCH_GENERATED_CODEC_ROUTE_ID,
    PATH_MATCH_GENERATED_CODEC_ROUTE_ID,
];

const REQUIRED_GENERATED_CODEC_MARKERS: &[&str] = &[
    GENERATED_CODEC_MARKER,
    CRYPTO_SYSRAND_RAND_WINDOWS_GENERATED_CODEC_MARKER,
    STRINGS_CLONE_GENERATED_CODEC_MARKER,
    SORT_INTERFACE_GENERATED_CODEC_MARKER,
    SORT_SEARCH_GENERATED_CODEC_MARKER,
    STRINGS_SEARCH_GENERATED_CODEC_MARKER,
    PATH_MATCH_GENERATED_CODEC_MARKER,
];

const REQUIRED_SOURCE_PROGRAM_TYPECHECK_STABLE_REPORT_FIELDS: &[&str] = &[
    "source_feasibility_report.stable_metadata_promotion",
    "source_feasibility_report.go_generated_codec_completion_handoff",
    "source_feasibility_report.csharp_generated_codec_completion_handoff",
];

const REQUIRED_SOURCE_PROGRAM_TYPECHECK_BLOCKED_CLAIMS: &[&str] = &[
    "whole_source_language_validity",
    "cross_repo_paper_claim",
    "model_quality_claim",
    "functional_correctness_claim",
    "source_completion_or_infill_claim",
    "type_correctness_claim",
    "arena_ir_promotion",
    "external_sidecar_stable_promotion",
    "generated_hot_path_claim",
    "generated_json_schema_output_change",
];

const REQUIRED_SOURCE_PROGRAM_NO_STABLE_CLAIMS: &[&str] = &[
    "no_source_program_validity_claim",
    "no_whole_source_language_validity_claim",
    "no_cross_repo_paper_claim",
    "no_model_quality_claim",
    "no_functional_correctness_claim",
    "no_type_correctness_claim",
    "no_completion_or_infill_claim",
    "no_arena_ir_promotion",
    "no_external_sidecar_stable_promotion",
    "no_generated_hot_path_claim",
    "no_generated_json_schema_output_change",
];

const REQUIRED_SOURCE_PROGRAM_TYPECHECK_NO_STABLE_CLAIMS: &[&str] = &[
    "no_source_program_validity_claim",
    "no_whole_source_language_validity_claim",
    "no_cross_repo_paper_claim",
    "no_model_quality_claim",
    "no_functional_correctness_claim",
    "no_type_correctness_claim",
    "no_completion_or_infill_claim",
    "no_arena_ir_promotion",
    "no_external_sidecar_stable_promotion",
    "no_generated_hot_path_typechecker",
    "no_generated_json_schema_output_change",
];

const REQUIRED_GENERATED_CODEC_REPORT_PATHS: &[&str] = &[
    GENERATED_CODEC_REPORT_PATH,
    CRYPTO_SYSRAND_RAND_WINDOWS_GENERATED_CODEC_REPORT_PATH,
    STRINGS_CLONE_GENERATED_CODEC_REPORT_PATH,
    SORT_INTERFACE_GENERATED_CODEC_REPORT_PATH,
    SORT_SEARCH_GENERATED_CODEC_REPORT_PATH,
    STRINGS_SEARCH_GENERATED_CODEC_REPORT_PATH,
    PATH_MATCH_GENERATED_CODEC_REPORT_PATH,
];

const REQUIRED_GENERATED_CODEC_REPORT_COMMANDS: &[&str] = &[
    GENERATED_CODEC_REPORT_COMMAND,
    CRYPTO_SYSRAND_RAND_WINDOWS_GENERATED_CODEC_REPORT_COMMAND,
    STRINGS_CLONE_GENERATED_CODEC_REPORT_COMMAND,
    SORT_INTERFACE_GENERATED_CODEC_REPORT_COMMAND,
    SORT_SEARCH_GENERATED_CODEC_REPORT_COMMAND,
    STRINGS_SEARCH_GENERATED_CODEC_REPORT_COMMAND,
    PATH_MATCH_GENERATED_CODEC_REPORT_COMMAND,
];

const REQUIRED_ALLOWED_CLAIMS: &[&str] = &[
    "generated_codec_validity_for_checked_go_real_source_corpus_records",
    WHOLE_FILE_GENERATED_CODEC_CLAIM,
    "parser_backed_source_corpus_identity_preserved",
    "stable_metadata_promotion_prerequisite_identified",
];

const REQUIRED_BLOCKED_CLAIMS: &[&str] = &[
    "source_program_validity_claim",
    "whole_source_language_validity_claim",
    "cross_repo_paper_claim",
    "model_quality_claim",
    "functional_correctness_claim",
    "completion_or_infill_claim",
    "type_correctness_claim",
    "arena_ir_promotion",
    "external_sidecar_stable_promotion",
    "generated_hot_path_claim",
    "generated_json_schema_output_change",
];

const REQUIRED_NO_STABLE_CLAIMS: &[&str] = &[
    "no_source_program_validity_claim",
    "no_whole_source_language_validity_claim",
    "no_cross_repo_paper_claim",
    "no_model_quality_claim",
    "no_functional_correctness_claim",
    "no_type_correctness_claim",
    "no_completion_or_infill_claim",
    "no_arena_ir_promotion",
    "no_external_sidecar_stable_promotion",
    "no_generated_hot_path_claim",
    "no_generated_json_schema_output_change",
];

const REQUIRED_VALIDATION_COMMANDS: &[&str] = &[
    "cargo test --test syntax_backend_boundary_test syntax_backend_go_real_source_generated_codec_completion_handoff_consumes_generated_codec_corpus_source_handoff_source_handoff_source_handoff_source_handoff_source_inputs -- --exact",
    "python3 -m unittest tests.python.test_syntax_backend_gate_strategy tests.python.test_structural_model_boundary_selection",
    "python3 scripts/testing/test-quality-audit.py --fail-on-gaps",
    "git diff --check",
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxGoRealSourceGeneratedCodecCompletionHandoffGate {
    pub artifact_kind: String,
    pub version: u32,
    pub tool: String,
    pub status: String,
    pub handoff_id: String,
    pub source_generated_codec_corpus: GeneratedCodecCompletionSourceCorpus,
    pub stable_promotion_decision: GeneratedCodecCompletionStablePromotionDecision,
    pub completion_summary: GeneratedCodecCompletionSummary,
    pub claim_boundary: GeneratedCodecCompletionClaimBoundary,
    pub validation_commands: Vec<String>,
    pub completed_promotion_class: String,
    pub next_missing_stable_promotion_class: String,
}

impl SyntaxGoRealSourceGeneratedCodecCompletionHandoffGate {
    pub fn validate_against_corpus(
        &self,
        corpus_gate: &SyntaxGoRealSourceGeneratedCodecCorpusGate,
        final_decision_gate: &SyntaxFinalStablePromotionDecisionGate,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        require_string("artifact_kind", &self.artifact_kind, ARTIFACT_KIND)?;
        require_u32("version", self.version, VERSION)?;
        require_string("tool", &self.tool, TOOL)?;
        require_string("status", &self.status, STATUS)?;
        require_string("handoff_id", &self.handoff_id, HANDOFF_ID)?;
        self.source_generated_codec_corpus.validate(corpus_gate)?;
        self.stable_promotion_decision
            .validate(final_decision_gate)?;
        self.completion_summary
            .validate(corpus_gate, final_decision_gate)?;
        self.claim_boundary.validate(corpus_gate)?;
        require_all(
            "validation_commands",
            &self.validation_commands,
            REQUIRED_VALIDATION_COMMANDS,
        )?;
        require_string(
            "completed_promotion_class",
            &self.completed_promotion_class,
            COMPLETED_PROMOTION_CLASS,
        )?;
        require_string(
            "next_missing_stable_promotion_class",
            &self.next_missing_stable_promotion_class,
            NEXT_MISSING_STABLE_PROMOTION_CLASS,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeneratedCodecCompletionSourceCorpus {
    pub artifact_kind: String,
    pub version: u32,
    pub tool: String,
    pub module_path: String,
    pub fixture_path: String,
    pub completed_promotion_class: String,
    pub next_missing_stable_promotion_class: String,
    pub corpus_id: String,
    pub corpus_manifest_path: String,
    pub stable_metadata_promotion: SyntaxSourceProgramStableMetadataPromotionInput,
    pub go_generated_codec_completion_handoff:
        SyntaxSourceProgramGoGeneratedCodecCompletionHandoffInput,
    pub source_program_typecheck_execution_report_path: String,
    pub source_program_typecheck_feasibility_report_path: String,
    pub source_program_typecheck_execution_completed_promotion_class: String,
    pub source_program_typecheck_execution_next_missing_stable_promotion_class: String,
    pub total_corpus_record_count: usize,
    pub supported_record_count: usize,
    pub blocked_record_count: usize,
    pub supported_manifest_paths: Vec<String>,
    pub generated_codec_routes: Vec<String>,
    pub generated_codec_markers: Vec<String>,
    pub source_program_preserved_typecheck_stable_report_fields: Vec<String>,
    pub source_program_preserved_typecheck_report_contract_stable_report_fields: Vec<String>,
    pub source_program_preserved_typecheck_blocked_claims: Vec<String>,
    pub source_program_preserved_typecheck_no_stable_claims: Vec<String>,
    pub source_program_execution_pass_envelope_required_record_count: usize,
    pub source_program_execution_pass_envelope_required_record_status: String,
    pub source_program_execution_pass_envelope_required_records_all_passed: bool,
    pub source_program_execution_pass_envelope_required_record_ids: Vec<String>,
    pub source_program_typechecker_results_sidecar_only: bool,
    pub source_program_type_correctness_claim_allowed: bool,
    pub source_program_typechecker_arena_ir_proof_used: bool,
    pub source_program_typechecker_json_schema_backend_unchanged: bool,
    pub source_program_corpus_check_report_path: String,
    pub source_program_corpus_eligible_record_count: usize,
    pub source_program_corpus_unsupported_record_count: usize,
    pub source_program_sidecar_validity_claim_allowed: bool,
    pub source_program_sidecar_arena_ir_proof_used: bool,
    pub source_program_sidecar_json_schema_backend_unchanged: bool,
    pub source_program_no_stable_claims: Vec<String>,
    pub generated_codec_report_paths: Vec<String>,
    pub generated_codec_report_commands: Vec<String>,
    pub generated_codec_round_trip_records: Vec<GeneratedCodecCorpusSupportedRecord>,
    pub whole_file_generated_codec_claim_allowed: bool,
    pub whole_file_generated_codec_claim_scope: String,
    pub allowed_claims: Vec<String>,
    pub no_stable_claims: Vec<String>,
    pub sidecar_only_completed: bool,
}

impl GeneratedCodecCompletionSourceCorpus {
    fn validate(
        &self,
        corpus_gate: &SyntaxGoRealSourceGeneratedCodecCorpusGate,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        require_string(
            "source_generated_codec_corpus.artifact_kind",
            &self.artifact_kind,
            CORPUS_ARTIFACT_KIND,
        )?;
        require_u32(
            "source_generated_codec_corpus.version",
            self.version,
            CORPUS_VERSION,
        )?;
        require_string(
            "source_generated_codec_corpus.tool",
            &self.tool,
            CORPUS_TOOL,
        )?;
        require_string(
            "source_generated_codec_corpus.module_path",
            &self.module_path,
            "src/syntax_go_real_source_generated_codec_corpus.rs",
        )?;
        require_string(
            "source_generated_codec_corpus.fixture_path",
            &self.fixture_path,
            CORPUS_FIXTURE_PATH,
        )?;
        require_string(
            "source_generated_codec_corpus.completed_promotion_class",
            &self.completed_promotion_class,
            CORPUS_COMPLETED_CLASS,
        )?;
        require_string(
            "source_generated_codec_corpus.next_missing_stable_promotion_class",
            &self.next_missing_stable_promotion_class,
            CORPUS_NEXT_CLASS,
        )?;
        require_string(
            "source_generated_codec_corpus.source_fixture_artifact_kind",
            &corpus_gate.artifact_kind,
            CORPUS_ARTIFACT_KIND,
        )?;
        require_string(
            "source_generated_codec_corpus.source_fixture_completed_class",
            &corpus_gate.completed_promotion_class,
            CORPUS_COMPLETED_CLASS,
        )?;
        require_string(
            "source_generated_codec_corpus.source_fixture_next_missing_class",
            &corpus_gate.next_missing_stable_promotion_class,
            CORPUS_NEXT_CLASS,
        )?;
        require_same_set(
            "source_generated_codec_corpus.stable_metadata_promotion.source_completion_handoff_source_program_preserved_typecheck_report_contract_stable_report_fields",
            &self
                .stable_metadata_promotion
                .source_completion_handoff_source_program_preserved_typecheck_report_contract_stable_report_fields,
            &corpus_gate
                .source_program_corpus_gate
                .stable_metadata_promotion
                .source_completion_handoff_source_program_preserved_typecheck_report_contract_stable_report_fields,
        )?;
        require_stable_metadata_promotion_equal(
            "source_generated_codec_corpus.stable_metadata_promotion",
            &self.stable_metadata_promotion,
            &corpus_gate
                .source_program_corpus_gate
                .stable_metadata_promotion,
        )?;
        require_go_completion_handoff_equal(
            "source_generated_codec_corpus.go_generated_codec_completion_handoff",
            &self.go_generated_codec_completion_handoff,
            &corpus_gate
                .source_program_corpus_gate
                .go_generated_codec_completion_handoff,
        )?;
        require_string(
            "source_generated_codec_corpus.source_program_typecheck_execution_report_path",
            &self.source_program_typecheck_execution_report_path,
            &corpus_gate
                .source_program_corpus_gate
                .source_typecheck_execution_report_path,
        )?;
        require_string(
            "source_generated_codec_corpus.source_program_typecheck_feasibility_report_path",
            &self.source_program_typecheck_feasibility_report_path,
            &corpus_gate
                .source_program_corpus_gate
                .source_typecheck_feasibility_report_path,
        )?;
        require_string(
            "source_generated_codec_corpus.source_program_typecheck_execution_completed_promotion_class",
            &self.source_program_typecheck_execution_completed_promotion_class,
            &corpus_gate
                .source_program_corpus_gate
                .source_typecheck_execution_completed_promotion_class,
        )?;
        require_string(
            "source_generated_codec_corpus.source_program_typecheck_execution_next_missing_stable_promotion_class",
            &self.source_program_typecheck_execution_next_missing_stable_promotion_class,
            &corpus_gate
                .source_program_corpus_gate
                .source_typecheck_execution_next_missing_stable_promotion_class,
        )?;
        require_usize(
            "source_generated_codec_corpus.total_corpus_record_count",
            self.total_corpus_record_count,
            corpus_gate.corpus_round_trip.total_corpus_record_count,
        )?;
        require_usize(
            "source_generated_codec_corpus.supported_record_count",
            self.supported_record_count,
            corpus_gate.corpus_round_trip.supported_records.len(),
        )?;
        require_usize(
            "source_generated_codec_corpus.supported_record_count.required",
            self.supported_record_count,
            REQUIRED_RECORD_COUNT,
        )?;
        require_usize(
            "source_generated_codec_corpus.blocked_record_count",
            self.blocked_record_count,
            corpus_gate.corpus_round_trip.blocked_records.len(),
        )?;
        require_usize(
            "source_generated_codec_corpus.blocked_record_count.required",
            self.blocked_record_count,
            REQUIRED_BLOCKED_RECORD_COUNT,
        )?;
        require_set(
            "source_generated_codec_corpus.supported_manifest_paths",
            &self.supported_manifest_paths,
            REQUIRED_SUPPORTED_MANIFEST_PATHS,
        )?;
        require_set(
            "source_generated_codec_corpus.generated_codec_routes",
            &self.generated_codec_routes,
            REQUIRED_GENERATED_CODEC_ROUTES,
        )?;
        require_set(
            "source_generated_codec_corpus.generated_codec_markers",
            &self.generated_codec_markers,
            REQUIRED_GENERATED_CODEC_MARKERS,
        )?;
        let corpus_manifest_paths = corpus_gate
            .corpus_round_trip
            .supported_records
            .iter()
            .map(|record| record.manifest_path.clone())
            .collect::<Vec<_>>();
        require_same_set(
            "source_generated_codec_corpus.source_fixture_supported_manifest_paths",
            &self.supported_manifest_paths,
            &corpus_manifest_paths,
        )?;
        let corpus_routes = corpus_gate
            .corpus_round_trip
            .supported_records
            .iter()
            .map(|record| record.generated_codec_route.clone())
            .collect::<Vec<_>>();
        require_same_set(
            "source_generated_codec_corpus.source_fixture_generated_codec_routes",
            &self.generated_codec_routes,
            &corpus_routes,
        )?;
        let corpus_markers = corpus_gate
            .corpus_round_trip
            .supported_records
            .iter()
            .map(|record| record.generated_codec_marker.clone())
            .collect::<Vec<_>>();
        require_same_set(
            "source_generated_codec_corpus.source_fixture_generated_codec_markers",
            &self.generated_codec_markers,
            &corpus_markers,
        )?;
        require_set(
            "source_generated_codec_corpus.source_program_preserved_typecheck_stable_report_fields",
            &self.source_program_preserved_typecheck_stable_report_fields,
            REQUIRED_SOURCE_PROGRAM_TYPECHECK_STABLE_REPORT_FIELDS,
        )?;
        require_same_set(
            "source_generated_codec_corpus.source_fixture_source_program_preserved_typecheck_stable_report_fields",
            &self.source_program_preserved_typecheck_stable_report_fields,
            &corpus_gate
                .source_program_corpus_gate
                .preserved_typecheck_stable_report_fields,
        )?;
        require_set(
            "source_generated_codec_corpus.source_program_preserved_typecheck_report_contract_stable_report_fields",
            &self.source_program_preserved_typecheck_report_contract_stable_report_fields,
            TYPECHECK_PRESERVATION_REPORT_STABLE_FIELDS,
        )?;
        require_same_set(
            "source_generated_codec_corpus.source_fixture_source_program_preserved_typecheck_report_contract_stable_report_fields",
            &self.source_program_preserved_typecheck_report_contract_stable_report_fields,
            &corpus_gate
                .source_program_corpus_gate
                .preserved_typecheck_report_contract_stable_report_fields,
        )?;
        require_set(
            "source_generated_codec_corpus.source_program_preserved_typecheck_blocked_claims",
            &self.source_program_preserved_typecheck_blocked_claims,
            REQUIRED_SOURCE_PROGRAM_TYPECHECK_BLOCKED_CLAIMS,
        )?;
        require_same_set(
            "source_generated_codec_corpus.source_fixture_source_program_preserved_typecheck_blocked_claims",
            &self.source_program_preserved_typecheck_blocked_claims,
            &corpus_gate
                .source_program_corpus_gate
                .preserved_typecheck_blocked_claims,
        )?;
        require_set(
            "source_generated_codec_corpus.source_program_preserved_typecheck_no_stable_claims",
            &self.source_program_preserved_typecheck_no_stable_claims,
            REQUIRED_SOURCE_PROGRAM_TYPECHECK_NO_STABLE_CLAIMS,
        )?;
        require_same_set(
            "source_generated_codec_corpus.source_fixture_source_program_preserved_typecheck_no_stable_claims",
            &self.source_program_preserved_typecheck_no_stable_claims,
            &corpus_gate
                .source_program_corpus_gate
                .preserved_typecheck_no_stable_claims,
        )?;
        require_usize(
            "source_generated_codec_corpus.source_program_execution_pass_envelope_required_record_count",
            self.source_program_execution_pass_envelope_required_record_count,
            corpus_gate
                .source_program_corpus_gate
                .execution_pass_envelope_required_record_count,
        )?;
        require_string(
            "source_generated_codec_corpus.source_program_execution_pass_envelope_required_record_status",
            &self.source_program_execution_pass_envelope_required_record_status,
            &corpus_gate
                .source_program_corpus_gate
                .execution_pass_envelope_required_record_status,
        )?;
        require_bool(
            "source_generated_codec_corpus.source_program_execution_pass_envelope_required_records_all_passed",
            self.source_program_execution_pass_envelope_required_records_all_passed,
            corpus_gate
                .source_program_corpus_gate
                .execution_pass_envelope_required_records_all_passed,
        )?;
        require_same_set(
            "source_generated_codec_corpus.source_program_execution_pass_envelope_required_record_ids",
            &self.source_program_execution_pass_envelope_required_record_ids,
            &corpus_gate
                .source_program_corpus_gate
                .execution_pass_envelope_required_record_ids,
        )?;
        require_bool(
            "source_generated_codec_corpus.source_program_typechecker_results_sidecar_only",
            self.source_program_typechecker_results_sidecar_only,
            corpus_gate
                .source_program_corpus_gate
                .execution_evidence_typechecker_results_sidecar_only,
        )?;
        require_bool(
            "source_generated_codec_corpus.source_program_type_correctness_claim_allowed",
            self.source_program_type_correctness_claim_allowed,
            corpus_gate
                .source_program_corpus_gate
                .execution_evidence_type_correctness_claim_allowed,
        )?;
        require_bool(
            "source_generated_codec_corpus.source_program_typechecker_arena_ir_proof_used",
            self.source_program_typechecker_arena_ir_proof_used,
            corpus_gate
                .source_program_corpus_gate
                .execution_evidence_arena_ir_proof_used,
        )?;
        require_bool(
            "source_generated_codec_corpus.source_program_typechecker_json_schema_backend_unchanged",
            self.source_program_typechecker_json_schema_backend_unchanged,
            corpus_gate
                .source_program_corpus_gate
                .execution_evidence_json_schema_backend_unchanged,
        )?;
        require_string(
            "source_generated_codec_corpus.source_program_corpus_check_report_path",
            &self.source_program_corpus_check_report_path,
            &corpus_gate
                .source_program_corpus_gate
                .corpus_check_report_path,
        )?;
        require_usize(
            "source_generated_codec_corpus.source_program_corpus_eligible_record_count",
            self.source_program_corpus_eligible_record_count,
            corpus_gate
                .source_program_corpus_gate
                .corpus_eligible_record_count,
        )?;
        require_usize(
            "source_generated_codec_corpus.source_program_corpus_unsupported_record_count",
            self.source_program_corpus_unsupported_record_count,
            corpus_gate
                .source_program_corpus_gate
                .corpus_unsupported_record_count,
        )?;
        require_bool(
            "source_generated_codec_corpus.source_program_sidecar_validity_claim_allowed",
            self.source_program_sidecar_validity_claim_allowed,
            corpus_gate
                .source_program_corpus_gate
                .sidecar_source_program_validity_claim_allowed,
        )?;
        require_bool(
            "source_generated_codec_corpus.source_program_sidecar_arena_ir_proof_used",
            self.source_program_sidecar_arena_ir_proof_used,
            corpus_gate
                .source_program_corpus_gate
                .sidecar_arena_ir_proof_used,
        )?;
        require_bool(
            "source_generated_codec_corpus.source_program_sidecar_json_schema_backend_unchanged",
            self.source_program_sidecar_json_schema_backend_unchanged,
            corpus_gate
                .source_program_corpus_gate
                .sidecar_json_schema_backend_unchanged,
        )?;
        require_set(
            "source_generated_codec_corpus.source_program_no_stable_claims",
            &self.source_program_no_stable_claims,
            REQUIRED_SOURCE_PROGRAM_NO_STABLE_CLAIMS,
        )?;
        require_same_set(
            "source_generated_codec_corpus.source_fixture_source_program_no_stable_claims",
            &self.source_program_no_stable_claims,
            &corpus_gate
                .source_program_corpus_gate
                .source_program_no_stable_claims,
        )?;
        require_set(
            "source_generated_codec_corpus.generated_codec_report_paths",
            &self.generated_codec_report_paths,
            REQUIRED_GENERATED_CODEC_REPORT_PATHS,
        )?;
        let mut corpus_report_paths = vec![
            corpus_gate
                .generated_codec_report_contract
                .report_path
                .clone(),
        ];
        corpus_report_paths.extend(
            corpus_gate
                .generated_codec_report_contract
                .additional_report_paths
                .iter()
                .cloned(),
        );
        require_same_set(
            "source_generated_codec_corpus.source_fixture_generated_codec_report_paths",
            &self.generated_codec_report_paths,
            &corpus_report_paths,
        )?;
        require_set(
            "source_generated_codec_corpus.generated_codec_report_commands",
            &self.generated_codec_report_commands,
            REQUIRED_GENERATED_CODEC_REPORT_COMMANDS,
        )?;
        let mut corpus_report_commands = vec![
            corpus_gate
                .generated_codec_report_contract
                .report_command
                .clone(),
        ];
        corpus_report_commands.extend(
            corpus_gate
                .generated_codec_report_contract
                .additional_report_commands
                .iter()
                .cloned(),
        );
        require_same_set(
            "source_generated_codec_corpus.source_fixture_generated_codec_report_commands",
            &self.generated_codec_report_commands,
            &corpus_report_commands,
        )?;
        require_usize(
            "source_generated_codec_corpus.generated_codec_round_trip_records",
            self.generated_codec_round_trip_records.len(),
            corpus_gate.corpus_round_trip.supported_records.len(),
        )?;
        require_same_supported_records(
            "source_generated_codec_corpus.generated_codec_round_trip_records",
            &self.generated_codec_round_trip_records,
            &corpus_gate.corpus_round_trip.supported_records,
        )?;
        require_bool(
            "source_generated_codec_corpus.whole_file_generated_codec_claim_allowed",
            self.whole_file_generated_codec_claim_allowed,
            true,
        )?;
        require_bool(
            "source_generated_codec_corpus.source_fixture_whole_file_generated_codec_claim_allowed",
            corpus_gate.whole_file_generated_codec_claim_allowed,
            true,
        )?;
        require_string(
            "source_generated_codec_corpus.whole_file_generated_codec_claim_scope",
            &self.whole_file_generated_codec_claim_scope,
            WHOLE_FILE_GENERATED_CODEC_CLAIM_SCOPE,
        )?;
        require_string(
            "source_generated_codec_corpus.source_fixture_whole_file_generated_codec_claim_scope",
            &corpus_gate.whole_file_generated_codec_claim_scope,
            WHOLE_FILE_GENERATED_CODEC_CLAIM_SCOPE,
        )?;
        require_all(
            "source_generated_codec_corpus.allowed_claims",
            &self.allowed_claims,
            REQUIRED_ALLOWED_CLAIMS,
        )?;
        require_all(
            "source_generated_codec_corpus.source_fixture_allowed_claims",
            &corpus_gate.allowed_claims,
            REQUIRED_ALLOWED_CLAIMS,
        )?;
        require_all(
            "source_generated_codec_corpus.no_stable_claims",
            &self.no_stable_claims,
            REQUIRED_NO_STABLE_CLAIMS,
        )?;
        require_all(
            "source_generated_codec_corpus.source_fixture_no_stable_claims",
            &corpus_gate.no_stable_claims,
            REQUIRED_NO_STABLE_CLAIMS,
        )?;
        require_bool(
            "source_generated_codec_corpus.sidecar_only_completed",
            self.sidecar_only_completed,
            true,
        )?;
        for record in &corpus_gate.corpus_round_trip.supported_records {
            require_bool(
                "source_generated_codec_corpus.source_fixture_record_stable_promotion_allowed",
                record.stable_promotion_allowed,
                false,
            )?;
            require_bool(
                "source_generated_codec_corpus.source_fixture_record_source_program_validity_claim_allowed",
                record.source_program_validity_claim_allowed,
                false,
            )?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeneratedCodecCompletionStablePromotionDecision {
    pub artifact_kind: String,
    pub version: u32,
    pub tool: String,
    pub fixture_path: String,
    pub completed_promotion_class: String,
    pub next_missing_stable_promotion_class: String,
    pub decision: String,
    pub stable_ready: bool,
    pub stable_backend_promoted_in_this_gate: bool,
    pub stable_metadata_promotion_prerequisite: String,
    pub remaining_blockers: Vec<String>,
}

impl GeneratedCodecCompletionStablePromotionDecision {
    fn validate(
        &self,
        final_decision_gate: &SyntaxFinalStablePromotionDecisionGate,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        require_string(
            "stable_promotion_decision.artifact_kind",
            &self.artifact_kind,
            FINAL_PROMOTION_ARTIFACT_KIND,
        )?;
        require_u32(
            "stable_promotion_decision.version",
            self.version,
            FINAL_PROMOTION_VERSION,
        )?;
        require_string(
            "stable_promotion_decision.tool",
            &self.tool,
            FINAL_PROMOTION_TOOL,
        )?;
        require_string(
            "stable_promotion_decision.fixture_path",
            &self.fixture_path,
            FINAL_PROMOTION_FIXTURE_PATH,
        )?;
        require_string(
            "stable_promotion_decision.completed_promotion_class",
            &self.completed_promotion_class,
            FINAL_PROMOTION_COMPLETED_CLASS,
        )?;
        require_string(
            "stable_promotion_decision.next_missing_stable_promotion_class",
            &self.next_missing_stable_promotion_class,
            STABLE_METADATA_PROMOTION_IMPLEMENTATION_CLASS,
        )?;
        require_string(
            "stable_promotion_decision.decision",
            &self.decision,
            DECISION,
        )?;
        require_bool(
            "stable_promotion_decision.stable_ready",
            self.stable_ready,
            true,
        )?;
        require_bool(
            "stable_promotion_decision.stable_backend_promoted_in_this_gate",
            self.stable_backend_promoted_in_this_gate,
            false,
        )?;
        require_string(
            "stable_promotion_decision.stable_metadata_promotion_prerequisite",
            &self.stable_metadata_promotion_prerequisite,
            STABLE_METADATA_PROMOTION_IMPLEMENTATION_CLASS,
        )?;
        if !self.remaining_blockers.is_empty() {
            return Err(SyntaxBackendBoundaryError::new(
                "stable_promotion_decision.remaining_blockers",
                "completion handoff cannot carry stable-promotion blockers",
            ));
        }
        require_string(
            "stable_promotion_decision.source_fixture_artifact_kind",
            &final_decision_gate.artifact_kind,
            FINAL_PROMOTION_ARTIFACT_KIND,
        )?;
        require_string(
            "stable_promotion_decision.source_fixture_next_missing_class",
            &final_decision_gate.next_missing_stable_promotion_class,
            STABLE_METADATA_PROMOTION_IMPLEMENTATION_CLASS,
        )?;
        require_bool(
            "stable_promotion_decision.source_fixture_stable_ready",
            final_decision_gate.promotion_decision.stable_ready,
            true,
        )?;
        require_bool(
            "stable_promotion_decision.source_fixture_stable_backend_promoted_in_this_gate",
            final_decision_gate
                .promotion_decision
                .stable_backend_promoted_in_this_gate,
            false,
        )?;
        require_bool(
            "stable_promotion_decision.source_fixture_arena_ir_proof_used",
            final_decision_gate.promotion_decision.arena_ir_proof_used,
            false,
        )?;
        require_bool(
            "stable_promotion_decision.source_fixture_external_sidecars_promoted",
            final_decision_gate
                .promotion_decision
                .external_sidecars_promoted,
            false,
        )?;
        if !final_decision_gate
            .promotion_decision
            .remaining_blockers
            .is_empty()
        {
            return Err(SyntaxBackendBoundaryError::new(
                "stable_promotion_decision.source_fixture_remaining_blockers",
                "final stable-promotion decision must remain blocker-free",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeneratedCodecCompletionSummary {
    pub source_language: String,
    pub corpus_id: String,
    pub corpus_manifest_path: String,
    pub checked_record_count: usize,
    pub supported_generated_codec_record_count: usize,
    pub blocked_generated_codec_record_count: usize,
    pub all_checked_records_supported: bool,
    pub route_scope: String,
    pub stable_promotion_prerequisite: String,
}

impl GeneratedCodecCompletionSummary {
    fn validate(
        &self,
        corpus_gate: &SyntaxGoRealSourceGeneratedCodecCorpusGate,
        final_decision_gate: &SyntaxFinalStablePromotionDecisionGate,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        require_string(
            "completion_summary.source_language",
            &self.source_language,
            "go",
        )?;
        require_string(
            "completion_summary.corpus_id",
            &self.corpus_id,
            &corpus_gate.corpus_round_trip.corpus_id,
        )?;
        require_string(
            "completion_summary.corpus_manifest_path",
            &self.corpus_manifest_path,
            &corpus_gate.corpus_round_trip.corpus_manifest_path,
        )?;
        require_usize(
            "completion_summary.checked_record_count",
            self.checked_record_count,
            corpus_gate.corpus_round_trip.total_corpus_record_count,
        )?;
        require_usize(
            "completion_summary.supported_generated_codec_record_count",
            self.supported_generated_codec_record_count,
            corpus_gate
                .corpus_round_trip
                .generated_codec_round_trip_record_count,
        )?;
        require_usize(
            "completion_summary.blocked_generated_codec_record_count",
            self.blocked_generated_codec_record_count,
            corpus_gate.corpus_round_trip.blocked_record_count,
        )?;
        require_bool(
            "completion_summary.all_checked_records_supported",
            self.all_checked_records_supported,
            true,
        )?;
        require_string(
            "completion_summary.route_scope",
            &self.route_scope,
            "checked_go_real_source_generated_codec_sidecars",
        )?;
        require_string(
            "completion_summary.stable_promotion_prerequisite",
            &self.stable_promotion_prerequisite,
            &final_decision_gate.next_missing_stable_promotion_class,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeneratedCodecCompletionClaimBoundary {
    pub syntax_level_boundary: String,
    pub allowed_claims: Vec<String>,
    pub blocked_claims: Vec<String>,
    pub parser_formatter_sidecars_promoted: bool,
    pub type_checker_sidecars_promoted: bool,
    pub source_program_validity_claim_allowed: bool,
    pub whole_file_generated_codec_claim_allowed: bool,
    pub arena_ir_proof_used: bool,
    pub generated_hot_path_claim_allowed: bool,
    pub generated_json_schema_output_changed: bool,
    pub json_schema_backend_preserved: bool,
}

impl GeneratedCodecCompletionClaimBoundary {
    fn validate(
        &self,
        corpus_gate: &SyntaxGoRealSourceGeneratedCodecCorpusGate,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        require_string(
            "claim_boundary.syntax_level_boundary",
            &self.syntax_level_boundary,
            SYNTAX_LEVEL_BOUNDARY,
        )?;
        require_all(
            "claim_boundary.allowed_claims",
            &self.allowed_claims,
            REQUIRED_ALLOWED_CLAIMS,
        )?;
        require_all(
            "claim_boundary.blocked_claims",
            &self.blocked_claims,
            REQUIRED_BLOCKED_CLAIMS,
        )?;
        require_bool(
            "claim_boundary.parser_formatter_sidecars_promoted",
            self.parser_formatter_sidecars_promoted,
            false,
        )?;
        require_bool(
            "claim_boundary.type_checker_sidecars_promoted",
            self.type_checker_sidecars_promoted,
            false,
        )?;
        require_bool(
            "claim_boundary.source_program_validity_claim_allowed",
            self.source_program_validity_claim_allowed,
            false,
        )?;
        require_bool(
            "claim_boundary.whole_file_generated_codec_claim_allowed",
            self.whole_file_generated_codec_claim_allowed,
            true,
        )?;
        require_bool(
            "claim_boundary.arena_ir_proof_used",
            self.arena_ir_proof_used,
            false,
        )?;
        require_bool(
            "claim_boundary.generated_hot_path_claim_allowed",
            self.generated_hot_path_claim_allowed,
            false,
        )?;
        require_bool(
            "claim_boundary.generated_json_schema_output_changed",
            self.generated_json_schema_output_changed,
            false,
        )?;
        require_bool(
            "claim_boundary.json_schema_backend_preserved",
            self.json_schema_backend_preserved,
            true,
        )?;
        require_all(
            "claim_boundary.source_fixture_no_stable_claims",
            &corpus_gate.no_stable_claims,
            REQUIRED_NO_STABLE_CLAIMS,
        )
    }
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

fn require_all(
    field: &'static str,
    actual: &[String],
    required: &[&str],
) -> Result<(), SyntaxBackendBoundaryError> {
    let actual = actual.iter().map(String::as_str).collect::<BTreeSet<_>>();
    let missing = required
        .iter()
        .copied()
        .filter(|required| !actual.contains(required))
        .collect::<Vec<_>>();
    if missing.is_empty() {
        Ok(())
    } else {
        Err(SyntaxBackendBoundaryError::new(
            field,
            format!("missing required entries: {}", missing.join(", ")),
        ))
    }
}

fn require_set(
    field: &'static str,
    actual: &[String],
    expected: &[&str],
) -> Result<(), SyntaxBackendBoundaryError> {
    let actual = actual.iter().map(String::as_str).collect::<BTreeSet<_>>();
    let expected = expected.iter().copied().collect::<BTreeSet<_>>();
    if actual == expected {
        Ok(())
    } else {
        Err(SyntaxBackendBoundaryError::new(
            field,
            format!("expected {expected:?}, got {actual:?}"),
        ))
    }
}

fn require_same_set(
    field: &'static str,
    left: &[String],
    right: &[String],
) -> Result<(), SyntaxBackendBoundaryError> {
    let left = left.iter().map(String::as_str).collect::<BTreeSet<_>>();
    let right = right.iter().map(String::as_str).collect::<BTreeSet<_>>();
    if left == right {
        Ok(())
    } else {
        Err(SyntaxBackendBoundaryError::new(
            field,
            format!("expected {right:?}, got {left:?}"),
        ))
    }
}

fn require_same_supported_records(
    field: &'static str,
    left: &[GeneratedCodecCorpusSupportedRecord],
    right: &[GeneratedCodecCorpusSupportedRecord],
) -> Result<(), SyntaxBackendBoundaryError> {
    if left == right {
        Ok(())
    } else {
        Err(SyntaxBackendBoundaryError::new(
            field,
            "generated-codec round-trip records do not match generated-codec corpus gate",
        ))
    }
}

fn require_stable_metadata_promotion_equal(
    field: &'static str,
    actual: &SyntaxSourceProgramStableMetadataPromotionInput,
    expected: &SyntaxSourceProgramStableMetadataPromotionInput,
) -> Result<(), SyntaxBackendBoundaryError> {
    if actual == expected {
        Ok(())
    } else {
        Err(SyntaxBackendBoundaryError::new(
            field,
            "stable metadata promotion source handoff input does not match generated-codec corpus gate",
        ))
    }
}

fn require_go_completion_handoff_equal(
    field: &'static str,
    actual: &SyntaxSourceProgramGoGeneratedCodecCompletionHandoffInput,
    expected: &SyntaxSourceProgramGoGeneratedCodecCompletionHandoffInput,
) -> Result<(), SyntaxBackendBoundaryError> {
    if actual == expected {
        Ok(())
    } else {
        Err(SyntaxBackendBoundaryError::new(
            field,
            "Go generated-codec completion source handoff input does not match generated-codec corpus gate",
        ))
    }
}
