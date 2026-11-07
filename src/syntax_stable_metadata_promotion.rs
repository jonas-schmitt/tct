//! Stable metadata promotion implementation gate for the syntax backend.
//!
//! This gate consumes the final stable-promotion decision, the feature-limited
//! production route metadata, and the Go real-source generated-codec completion
//! handoff. It promotes only route metadata for the parser-backed syntax
//! boundary; source-program, paper, model-quality, type-correctness, arena IR,
//! generated hot-path, and generated JSON Schema output claims stay blocked.
//! Completed promotion class: syntax backend stable metadata promotion
//! implementation gate. Next prerequisite: Build Go Source-Record Raw-Token
//! Generator Core.

use crate::syntax_backend_boundary::SyntaxBackendBoundaryError;
use crate::syntax_final_promotion_decision::{
    ARTIFACT_KIND as FINAL_DECISION_ARTIFACT_KIND,
    COMPLETED_PROMOTION_CLASS as FINAL_DECISION_COMPLETED_CLASS, DECISION as FINAL_DECISION,
    FIXTURE_PATH as FINAL_DECISION_FIXTURE_PATH,
    NEXT_MISSING_STABLE_PROMOTION_CLASS as FINAL_DECISION_NEXT_CLASS,
    SyntaxFinalStablePromotionDecisionGate, TOOL as FINAL_DECISION_TOOL,
    VERSION as FINAL_DECISION_VERSION,
};
use crate::syntax_go_real_source_generated_codec_completion_handoff::{
    ARTIFACT_KIND as COMPLETION_HANDOFF_ARTIFACT_KIND,
    COMPLETED_PROMOTION_CLASS as COMPLETION_HANDOFF_COMPLETED_CLASS,
    FIXTURE_PATH as COMPLETION_HANDOFF_FIXTURE_PATH,
    NEXT_MISSING_STABLE_PROMOTION_CLASS as COMPLETION_HANDOFF_NEXT_CLASS,
    SyntaxGoRealSourceGeneratedCodecCompletionHandoffGate, TOOL as COMPLETION_HANDOFF_TOOL,
    VERSION as COMPLETION_HANDOFF_VERSION,
};
use crate::syntax_go_real_source_generated_codec_corpus::{
    GeneratedCodecCorpusSupportedRecord, WHOLE_FILE_GENERATED_CODEC_CLAIM,
    WHOLE_FILE_GENERATED_CODEC_CLAIM_SCOPE,
};
use crate::syntax_production_route::{
    ARTIFACT_KIND as PRODUCTION_ROUTE_ARTIFACT_KIND,
    COMPLETED_PROMOTION_CLASS as PRODUCTION_ROUTE_COMPLETED_CLASS,
    FIXTURE_PATH as PRODUCTION_ROUTE_FIXTURE_PATH,
    NEXT_MISSING_STABLE_PROMOTION_CLASS as PRODUCTION_ROUTE_NEXT_MISSING_CLASS, ROUTE_ENTRYPOINT,
    ROUTE_GATE, STABLE_BACKEND_SCOPE, STABLE_METADATA_PROMOTION_DECISION,
    STABLE_METADATA_PROMOTION_DECISION_FIXTURE_PATH, SyntaxProductionRouteImplementationGate,
    TOOL as PRODUCTION_ROUTE_TOOL, VERSION as PRODUCTION_ROUTE_VERSION,
};
use crate::syntax_source_program_handoff::{
    SYNTAX_LEVEL_BOUNDARY, SyntaxSourceProgramGoGeneratedCodecCompletionHandoffInput,
    SyntaxSourceProgramStableMetadataPromotionInput, TYPECHECK_PRESERVATION_REPORT_STABLE_FIELDS,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const TOOL: &str = "syntax_backend_stable_metadata_promotion_implementation_gate";
pub const VERSION: u32 = 1;
pub const ARTIFACT_KIND: &str = "tct_syntax_backend_stable_metadata_promotion_implementation_gate";
pub const FIXTURE_PATH: &str =
    "tests/fixtures/syntax_backend/stable-metadata-promotion-implementation-v1.json";
pub const STATUS: &str = "stable_metadata_promotion_implemented_from_go_corpus_handoff";
pub const PROMOTION_ID: &str = "stable-metadata-promotion-implementation-v1";
pub const COMPLETED_PROMOTION_CLASS: &str =
    "syntax backend stable metadata promotion implementation gate";
pub const NEXT_MISSING_STABLE_PROMOTION_CLASS: &str =
    crate::syntax_roadmap_successors::GO_SOURCE_RECORD_RAW_TOKEN_GENERATOR_CORE_SUCCESSOR_TASK;

const REQUIRED_SUPPORTED_RECORD_COUNT: usize = 7;
const REQUIRED_BLOCKED_RECORD_COUNT: usize = 0;

const REQUIRED_SUPPORTED_MANIFEST_PATHS: &[&str] = &[
    "strings/compare.go",
    "crypto/internal/sysrand/rand_windows.go",
    "strings/clone.go",
    "sort/example_interface_test.go",
    "sort/example_search_test.go",
    "strings/search.go",
    "path/match.go",
];

const REQUIRED_GENERATED_CODEC_ROUTES: &[&str] = &[
    "go_strings_compare_source_file_syntax_v0",
    "go_crypto_sysrand_rand_windows_source_file_syntax_v0",
    "go_strings_clone_source_file_syntax_v0",
    "go_sort_interface_source_file_syntax_v0",
    "go_sort_search_source_file_syntax_v0",
    "go_strings_search_source_file_syntax_v0",
    "go_path_match_source_file_syntax_v0",
];

const REQUIRED_GENERATED_CODEC_MARKERS: &[&str] = &[
    "generated_go_strings_compare_source_file_round_trips",
    "generated_go_crypto_sysrand_rand_windows_source_file_round_trips",
    "generated_go_strings_clone_source_file_round_trips",
    "generated_go_sort_interface_source_file_round_trips",
    "generated_go_sort_search_source_file_round_trips",
    "generated_go_strings_search_source_file_round_trips",
    "generated_go_path_match_source_file_round_trips",
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

const REQUIRED_GENERATED_CODEC_NO_STABLE_CLAIMS: &[&str] = &[
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

const REQUIRED_GENERATED_CODEC_ALLOWED_CLAIMS: &[&str] = &[
    "generated_codec_validity_for_checked_go_real_source_corpus_records",
    WHOLE_FILE_GENERATED_CODEC_CLAIM,
    "parser_backed_source_corpus_identity_preserved",
    "stable_metadata_promotion_prerequisite_identified",
];

const REQUIRED_STABLE_METADATA_CLAIMS: &[&str] = &[
    "feature_limited_parser_backed_syntax_backend",
    "stable_route_metadata_promoted",
    "no_arena_ir_promotion",
    "no_generated_json_schema_output_change",
    "no_core_compiler_path_change",
    "no_json_schema_compiler_pass_change",
    "no_generated_source_substring_dependency",
];

const REQUIRED_ALLOWED_CLAIMS: &[&str] = &[
    "stable_route_metadata_promoted",
    "generated_codec_validity_for_checked_go_real_source_corpus_records",
    WHOLE_FILE_GENERATED_CODEC_CLAIM,
    "parser_backed_source_corpus_identity_preserved",
    "json_schema_backend_preserved",
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

const REQUIRED_VALIDATION_COMMANDS: &[&str] = &[
    "cargo test --test syntax_backend_boundary_test syntax_backend_stable_metadata_promotion_consumes_completion_handoff_source_handoff_source_handoff_source_handoff_source_inputs -- --exact",
    "cargo test --test syntax_backend_boundary_test syntax_backend_go_real_source_generated_codec_completion_handoff_consumes_generated_codec_corpus_source_handoff_source_handoff_source_handoff_source_handoff_source_inputs -- --exact",
    "python3 -m unittest tests.python.test_syntax_backend_gate_strategy tests.python.test_structural_model_boundary_selection",
    "python3 scripts/testing/test-quality-audit.py --fail-on-gaps",
    "git diff --check",
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxStableMetadataPromotionImplementationGate {
    pub artifact_kind: String,
    pub version: u32,
    pub tool: String,
    pub status: String,
    pub promotion_id: String,
    pub source_completion_handoff: StableMetadataPromotionSourceCompletionHandoff,
    pub source_final_promotion_decision: StableMetadataPromotionSourceFinalDecision,
    pub source_production_route: StableMetadataPromotionSourceProductionRoute,
    pub promotion_contract: StableMetadataPromotionContract,
    pub claim_boundary: StableMetadataPromotionClaimBoundary,
    pub validation_commands: Vec<String>,
    pub completed_promotion_class: String,
    pub next_missing_stable_promotion_class: String,
}

impl SyntaxStableMetadataPromotionImplementationGate {
    pub fn validate_against_handoff_and_route(
        &self,
        completion_handoff: &SyntaxGoRealSourceGeneratedCodecCompletionHandoffGate,
        final_decision: &SyntaxFinalStablePromotionDecisionGate,
        route_gate: &SyntaxProductionRouteImplementationGate,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        require_string("artifact_kind", &self.artifact_kind, ARTIFACT_KIND)?;
        require_u32("version", self.version, VERSION)?;
        require_string("tool", &self.tool, TOOL)?;
        require_string("status", &self.status, STATUS)?;
        require_string("promotion_id", &self.promotion_id, PROMOTION_ID)?;
        self.source_completion_handoff
            .validate(completion_handoff)?;
        self.source_final_promotion_decision
            .validate(final_decision, completion_handoff)?;
        self.source_production_route.validate(route_gate)?;
        self.promotion_contract
            .validate(completion_handoff, route_gate)?;
        self.claim_boundary
            .validate(completion_handoff, route_gate)?;
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
pub struct StableMetadataPromotionSourceCompletionHandoff {
    pub artifact_kind: String,
    pub version: u32,
    pub tool: String,
    pub fixture_path: String,
    pub completed_promotion_class: String,
    pub next_missing_stable_promotion_class: String,
    pub stable_metadata_promotion: SyntaxSourceProgramStableMetadataPromotionInput,
    pub go_generated_codec_completion_handoff:
        SyntaxSourceProgramGoGeneratedCodecCompletionHandoffInput,
    pub source_program_typecheck_execution_report_path: String,
    pub source_program_typecheck_feasibility_report_path: String,
    pub source_program_typecheck_execution_completed_promotion_class: String,
    pub source_program_typecheck_execution_next_missing_stable_promotion_class: String,
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
    pub generated_codec_allowed_claims: Vec<String>,
    pub generated_codec_whole_file_claim_scope: String,
    pub generated_codec_no_stable_claims: Vec<String>,
    pub stable_metadata_promotion_prerequisite: String,
    pub source_program_claim_allowed: bool,
    pub whole_file_generated_codec_claim_allowed: bool,
    pub arena_ir_proof_used: bool,
}

impl StableMetadataPromotionSourceCompletionHandoff {
    fn validate(
        &self,
        completion_handoff: &SyntaxGoRealSourceGeneratedCodecCompletionHandoffGate,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        require_string(
            "source_completion_handoff.artifact_kind",
            &self.artifact_kind,
            COMPLETION_HANDOFF_ARTIFACT_KIND,
        )?;
        require_u32(
            "source_completion_handoff.version",
            self.version,
            COMPLETION_HANDOFF_VERSION,
        )?;
        require_string(
            "source_completion_handoff.tool",
            &self.tool,
            COMPLETION_HANDOFF_TOOL,
        )?;
        require_string(
            "source_completion_handoff.fixture_path",
            &self.fixture_path,
            COMPLETION_HANDOFF_FIXTURE_PATH,
        )?;
        require_string(
            "source_completion_handoff.completed_promotion_class",
            &self.completed_promotion_class,
            COMPLETION_HANDOFF_COMPLETED_CLASS,
        )?;
        require_string(
            "source_completion_handoff.next_missing_stable_promotion_class",
            &self.next_missing_stable_promotion_class,
            COMPLETION_HANDOFF_NEXT_CLASS,
        )?;
        require_same_set(
            "source_completion_handoff.stable_metadata_promotion.source_completion_handoff_source_program_preserved_typecheck_report_contract_stable_report_fields",
            &self
                .stable_metadata_promotion
                .source_completion_handoff_source_program_preserved_typecheck_report_contract_stable_report_fields,
            &completion_handoff
                .source_generated_codec_corpus
                .stable_metadata_promotion
                .source_completion_handoff_source_program_preserved_typecheck_report_contract_stable_report_fields,
        )?;
        require_stable_metadata_promotion_equal(
            "source_completion_handoff.stable_metadata_promotion",
            &self.stable_metadata_promotion,
            &completion_handoff
                .source_generated_codec_corpus
                .stable_metadata_promotion,
        )?;
        require_go_completion_handoff_equal(
            "source_completion_handoff.go_generated_codec_completion_handoff",
            &self.go_generated_codec_completion_handoff,
            &completion_handoff
                .source_generated_codec_corpus
                .go_generated_codec_completion_handoff,
        )?;
        require_string(
            "source_completion_handoff.source_program_typecheck_execution_report_path",
            &self.source_program_typecheck_execution_report_path,
            &completion_handoff
                .source_generated_codec_corpus
                .source_program_typecheck_execution_report_path,
        )?;
        require_string(
            "source_completion_handoff.source_program_typecheck_feasibility_report_path",
            &self.source_program_typecheck_feasibility_report_path,
            &completion_handoff
                .source_generated_codec_corpus
                .source_program_typecheck_feasibility_report_path,
        )?;
        require_string(
            "source_completion_handoff.source_program_typecheck_execution_completed_promotion_class",
            &self.source_program_typecheck_execution_completed_promotion_class,
            &completion_handoff
                .source_generated_codec_corpus
                .source_program_typecheck_execution_completed_promotion_class,
        )?;
        require_string(
            "source_completion_handoff.source_program_typecheck_execution_next_missing_stable_promotion_class",
            &self.source_program_typecheck_execution_next_missing_stable_promotion_class,
            &completion_handoff
                .source_generated_codec_corpus
                .source_program_typecheck_execution_next_missing_stable_promotion_class,
        )?;
        require_usize(
            "source_completion_handoff.supported_record_count",
            self.supported_record_count,
            completion_handoff
                .source_generated_codec_corpus
                .supported_record_count,
        )?;
        require_usize(
            "source_completion_handoff.supported_record_count.required",
            self.supported_record_count,
            REQUIRED_SUPPORTED_RECORD_COUNT,
        )?;
        require_usize(
            "source_completion_handoff.blocked_record_count",
            self.blocked_record_count,
            completion_handoff
                .source_generated_codec_corpus
                .blocked_record_count,
        )?;
        require_usize(
            "source_completion_handoff.blocked_record_count.required",
            self.blocked_record_count,
            REQUIRED_BLOCKED_RECORD_COUNT,
        )?;
        require_set(
            "source_completion_handoff.supported_manifest_paths",
            &self.supported_manifest_paths,
            REQUIRED_SUPPORTED_MANIFEST_PATHS,
        )?;
        require_same_set(
            "source_completion_handoff.source_fixture_supported_manifest_paths",
            &self.supported_manifest_paths,
            &completion_handoff
                .source_generated_codec_corpus
                .supported_manifest_paths,
        )?;
        require_set(
            "source_completion_handoff.generated_codec_routes",
            &self.generated_codec_routes,
            REQUIRED_GENERATED_CODEC_ROUTES,
        )?;
        require_same_set(
            "source_completion_handoff.source_fixture_generated_codec_routes",
            &self.generated_codec_routes,
            &completion_handoff
                .source_generated_codec_corpus
                .generated_codec_routes,
        )?;
        require_set(
            "source_completion_handoff.generated_codec_markers",
            &self.generated_codec_markers,
            REQUIRED_GENERATED_CODEC_MARKERS,
        )?;
        require_same_set(
            "source_completion_handoff.source_fixture_generated_codec_markers",
            &self.generated_codec_markers,
            &completion_handoff
                .source_generated_codec_corpus
                .generated_codec_markers,
        )?;
        require_set(
            "source_completion_handoff.source_program_preserved_typecheck_stable_report_fields",
            &self.source_program_preserved_typecheck_stable_report_fields,
            REQUIRED_SOURCE_PROGRAM_TYPECHECK_STABLE_REPORT_FIELDS,
        )?;
        require_same_set(
            "source_completion_handoff.source_fixture_source_program_preserved_typecheck_stable_report_fields",
            &self.source_program_preserved_typecheck_stable_report_fields,
            &completion_handoff
                .source_generated_codec_corpus
                .source_program_preserved_typecheck_stable_report_fields,
        )?;
        require_set(
            "source_completion_handoff.source_program_preserved_typecheck_report_contract_stable_report_fields",
            &self.source_program_preserved_typecheck_report_contract_stable_report_fields,
            TYPECHECK_PRESERVATION_REPORT_STABLE_FIELDS,
        )?;
        require_same_set(
            "source_completion_handoff.source_fixture_source_program_preserved_typecheck_report_contract_stable_report_fields",
            &self.source_program_preserved_typecheck_report_contract_stable_report_fields,
            &completion_handoff
                .source_generated_codec_corpus
                .source_program_preserved_typecheck_report_contract_stable_report_fields,
        )?;
        require_set(
            "source_completion_handoff.source_program_preserved_typecheck_blocked_claims",
            &self.source_program_preserved_typecheck_blocked_claims,
            REQUIRED_SOURCE_PROGRAM_TYPECHECK_BLOCKED_CLAIMS,
        )?;
        require_same_set(
            "source_completion_handoff.source_fixture_source_program_preserved_typecheck_blocked_claims",
            &self.source_program_preserved_typecheck_blocked_claims,
            &completion_handoff
                .source_generated_codec_corpus
                .source_program_preserved_typecheck_blocked_claims,
        )?;
        require_set(
            "source_completion_handoff.source_program_preserved_typecheck_no_stable_claims",
            &self.source_program_preserved_typecheck_no_stable_claims,
            REQUIRED_SOURCE_PROGRAM_TYPECHECK_NO_STABLE_CLAIMS,
        )?;
        require_same_set(
            "source_completion_handoff.source_fixture_source_program_preserved_typecheck_no_stable_claims",
            &self.source_program_preserved_typecheck_no_stable_claims,
            &completion_handoff
                .source_generated_codec_corpus
                .source_program_preserved_typecheck_no_stable_claims,
        )?;
        require_usize(
            "source_completion_handoff.source_program_execution_pass_envelope_required_record_count",
            self.source_program_execution_pass_envelope_required_record_count,
            completion_handoff
                .source_generated_codec_corpus
                .source_program_execution_pass_envelope_required_record_count,
        )?;
        require_string(
            "source_completion_handoff.source_program_execution_pass_envelope_required_record_status",
            &self.source_program_execution_pass_envelope_required_record_status,
            &completion_handoff
                .source_generated_codec_corpus
                .source_program_execution_pass_envelope_required_record_status,
        )?;
        require_bool(
            "source_completion_handoff.source_program_execution_pass_envelope_required_records_all_passed",
            self.source_program_execution_pass_envelope_required_records_all_passed,
            completion_handoff
                .source_generated_codec_corpus
                .source_program_execution_pass_envelope_required_records_all_passed,
        )?;
        require_same_set(
            "source_completion_handoff.source_program_execution_pass_envelope_required_record_ids",
            &self.source_program_execution_pass_envelope_required_record_ids,
            &completion_handoff
                .source_generated_codec_corpus
                .source_program_execution_pass_envelope_required_record_ids,
        )?;
        require_bool(
            "source_completion_handoff.source_program_typechecker_results_sidecar_only",
            self.source_program_typechecker_results_sidecar_only,
            completion_handoff
                .source_generated_codec_corpus
                .source_program_typechecker_results_sidecar_only,
        )?;
        require_bool(
            "source_completion_handoff.source_program_type_correctness_claim_allowed",
            self.source_program_type_correctness_claim_allowed,
            completion_handoff
                .source_generated_codec_corpus
                .source_program_type_correctness_claim_allowed,
        )?;
        require_bool(
            "source_completion_handoff.source_program_typechecker_arena_ir_proof_used",
            self.source_program_typechecker_arena_ir_proof_used,
            completion_handoff
                .source_generated_codec_corpus
                .source_program_typechecker_arena_ir_proof_used,
        )?;
        require_bool(
            "source_completion_handoff.source_program_typechecker_json_schema_backend_unchanged",
            self.source_program_typechecker_json_schema_backend_unchanged,
            completion_handoff
                .source_generated_codec_corpus
                .source_program_typechecker_json_schema_backend_unchanged,
        )?;
        require_string(
            "source_completion_handoff.source_program_corpus_check_report_path",
            &self.source_program_corpus_check_report_path,
            &completion_handoff
                .source_generated_codec_corpus
                .source_program_corpus_check_report_path,
        )?;
        require_usize(
            "source_completion_handoff.source_program_corpus_eligible_record_count",
            self.source_program_corpus_eligible_record_count,
            completion_handoff
                .source_generated_codec_corpus
                .source_program_corpus_eligible_record_count,
        )?;
        require_usize(
            "source_completion_handoff.source_program_corpus_unsupported_record_count",
            self.source_program_corpus_unsupported_record_count,
            completion_handoff
                .source_generated_codec_corpus
                .source_program_corpus_unsupported_record_count,
        )?;
        require_bool(
            "source_completion_handoff.source_program_sidecar_validity_claim_allowed",
            self.source_program_sidecar_validity_claim_allowed,
            completion_handoff
                .source_generated_codec_corpus
                .source_program_sidecar_validity_claim_allowed,
        )?;
        require_bool(
            "source_completion_handoff.source_program_sidecar_arena_ir_proof_used",
            self.source_program_sidecar_arena_ir_proof_used,
            completion_handoff
                .source_generated_codec_corpus
                .source_program_sidecar_arena_ir_proof_used,
        )?;
        require_bool(
            "source_completion_handoff.source_program_sidecar_json_schema_backend_unchanged",
            self.source_program_sidecar_json_schema_backend_unchanged,
            completion_handoff
                .source_generated_codec_corpus
                .source_program_sidecar_json_schema_backend_unchanged,
        )?;
        require_set(
            "source_completion_handoff.source_program_no_stable_claims",
            &self.source_program_no_stable_claims,
            REQUIRED_SOURCE_PROGRAM_NO_STABLE_CLAIMS,
        )?;
        require_same_set(
            "source_completion_handoff.source_fixture_source_program_no_stable_claims",
            &self.source_program_no_stable_claims,
            &completion_handoff
                .source_generated_codec_corpus
                .source_program_no_stable_claims,
        )?;
        require_same_set(
            "source_completion_handoff.source_fixture_generated_codec_report_paths",
            &self.generated_codec_report_paths,
            &completion_handoff
                .source_generated_codec_corpus
                .generated_codec_report_paths,
        )?;
        require_same_set(
            "source_completion_handoff.source_fixture_generated_codec_report_commands",
            &self.generated_codec_report_commands,
            &completion_handoff
                .source_generated_codec_corpus
                .generated_codec_report_commands,
        )?;
        require_usize(
            "source_completion_handoff.generated_codec_round_trip_records",
            self.generated_codec_round_trip_records.len(),
            completion_handoff
                .source_generated_codec_corpus
                .generated_codec_round_trip_records
                .len(),
        )?;
        require_same_supported_records(
            "source_completion_handoff.generated_codec_round_trip_records",
            &self.generated_codec_round_trip_records,
            &completion_handoff
                .source_generated_codec_corpus
                .generated_codec_round_trip_records,
        )?;
        require_set(
            "source_completion_handoff.generated_codec_allowed_claims",
            &self.generated_codec_allowed_claims,
            REQUIRED_GENERATED_CODEC_ALLOWED_CLAIMS,
        )?;
        require_same_set(
            "source_completion_handoff.source_fixture_generated_codec_allowed_claims",
            &self.generated_codec_allowed_claims,
            &completion_handoff
                .source_generated_codec_corpus
                .allowed_claims,
        )?;
        require_string(
            "source_completion_handoff.generated_codec_whole_file_claim_scope",
            &self.generated_codec_whole_file_claim_scope,
            WHOLE_FILE_GENERATED_CODEC_CLAIM_SCOPE,
        )?;
        require_string(
            "source_completion_handoff.source_fixture_generated_codec_whole_file_claim_scope",
            &completion_handoff
                .source_generated_codec_corpus
                .whole_file_generated_codec_claim_scope,
            WHOLE_FILE_GENERATED_CODEC_CLAIM_SCOPE,
        )?;
        require_set(
            "source_completion_handoff.generated_codec_no_stable_claims",
            &self.generated_codec_no_stable_claims,
            REQUIRED_GENERATED_CODEC_NO_STABLE_CLAIMS,
        )?;
        require_same_set(
            "source_completion_handoff.source_fixture_generated_codec_no_stable_claims",
            &self.generated_codec_no_stable_claims,
            &completion_handoff
                .source_generated_codec_corpus
                .no_stable_claims,
        )?;
        require_string(
            "source_completion_handoff.stable_metadata_promotion_prerequisite",
            &self.stable_metadata_promotion_prerequisite,
            COMPLETED_PROMOTION_CLASS,
        )?;
        require_bool(
            "source_completion_handoff.source_program_claim_allowed",
            self.source_program_claim_allowed,
            false,
        )?;
        require_bool(
            "source_completion_handoff.whole_file_generated_codec_claim_allowed",
            self.whole_file_generated_codec_claim_allowed,
            true,
        )?;
        require_bool(
            "source_completion_handoff.arena_ir_proof_used",
            self.arena_ir_proof_used,
            false,
        )?;
        require_bool(
            "source_completion_handoff.source_fixture_source_program_claim_allowed",
            completion_handoff
                .claim_boundary
                .source_program_validity_claim_allowed,
            false,
        )?;
        require_bool(
            "source_completion_handoff.source_fixture_whole_file_claim_allowed",
            completion_handoff
                .claim_boundary
                .whole_file_generated_codec_claim_allowed,
            true,
        )?;
        require_bool(
            "source_completion_handoff.source_fixture_arena_ir_proof_used",
            completion_handoff.claim_boundary.arena_ir_proof_used,
            false,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StableMetadataPromotionSourceFinalDecision {
    pub artifact_kind: String,
    pub version: u32,
    pub tool: String,
    pub fixture_path: String,
    pub completed_promotion_class: String,
    pub next_missing_stable_promotion_class: String,
    pub decision: String,
    pub stable_ready: bool,
    pub stable_backend_promoted_in_this_gate: bool,
    pub remaining_blockers: Vec<String>,
}

impl StableMetadataPromotionSourceFinalDecision {
    fn validate(
        &self,
        final_decision: &SyntaxFinalStablePromotionDecisionGate,
        completion_handoff: &SyntaxGoRealSourceGeneratedCodecCompletionHandoffGate,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        require_string(
            "source_final_promotion_decision.artifact_kind",
            &self.artifact_kind,
            FINAL_DECISION_ARTIFACT_KIND,
        )?;
        require_u32(
            "source_final_promotion_decision.version",
            self.version,
            FINAL_DECISION_VERSION,
        )?;
        require_string(
            "source_final_promotion_decision.tool",
            &self.tool,
            FINAL_DECISION_TOOL,
        )?;
        require_string(
            "source_final_promotion_decision.fixture_path",
            &self.fixture_path,
            FINAL_DECISION_FIXTURE_PATH,
        )?;
        require_string(
            "source_final_promotion_decision.completed_promotion_class",
            &self.completed_promotion_class,
            FINAL_DECISION_COMPLETED_CLASS,
        )?;
        require_string(
            "source_final_promotion_decision.next_missing_stable_promotion_class",
            &self.next_missing_stable_promotion_class,
            FINAL_DECISION_NEXT_CLASS,
        )?;
        require_string(
            "source_final_promotion_decision.decision",
            &self.decision,
            FINAL_DECISION,
        )?;
        require_bool(
            "source_final_promotion_decision.stable_ready",
            self.stable_ready,
            true,
        )?;
        require_bool(
            "source_final_promotion_decision.stable_backend_promoted_in_this_gate",
            self.stable_backend_promoted_in_this_gate,
            false,
        )?;
        require_empty(
            "source_final_promotion_decision.remaining_blockers",
            &self.remaining_blockers,
        )?;
        require_string(
            "source_final_promotion_decision.source_fixture_decision",
            &final_decision.promotion_decision.decision,
            FINAL_DECISION,
        )?;
        require_bool(
            "source_final_promotion_decision.source_fixture_stable_ready",
            final_decision.promotion_decision.stable_ready,
            true,
        )?;
        require_bool(
            "source_final_promotion_decision.source_fixture_backend_promoted_in_this_gate",
            final_decision
                .promotion_decision
                .stable_backend_promoted_in_this_gate,
            false,
        )?;
        require_string(
            "source_final_promotion_decision.handoff_decision",
            &completion_handoff.stable_promotion_decision.decision,
            FINAL_DECISION,
        )?;
        require_bool(
            "source_final_promotion_decision.handoff_stable_ready",
            completion_handoff.stable_promotion_decision.stable_ready,
            true,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StableMetadataPromotionSourceProductionRoute {
    pub artifact_kind: String,
    pub version: u32,
    pub tool: String,
    pub fixture_path: String,
    pub completed_promotion_class: String,
    pub next_missing_stable_promotion_class: String,
    pub route_entrypoint: String,
    pub route_gate: String,
    pub stable_backend_scope: String,
    pub stable_backend_promoted: bool,
    pub stable_metadata_claims: Vec<String>,
}

impl StableMetadataPromotionSourceProductionRoute {
    fn validate(
        &self,
        route_gate: &SyntaxProductionRouteImplementationGate,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        require_string(
            "source_production_route.artifact_kind",
            &self.artifact_kind,
            PRODUCTION_ROUTE_ARTIFACT_KIND,
        )?;
        require_u32(
            "source_production_route.version",
            self.version,
            PRODUCTION_ROUTE_VERSION,
        )?;
        require_string(
            "source_production_route.tool",
            &self.tool,
            PRODUCTION_ROUTE_TOOL,
        )?;
        require_string(
            "source_production_route.fixture_path",
            &self.fixture_path,
            PRODUCTION_ROUTE_FIXTURE_PATH,
        )?;
        require_string(
            "source_production_route.completed_promotion_class",
            &self.completed_promotion_class,
            PRODUCTION_ROUTE_COMPLETED_CLASS,
        )?;
        require_string(
            "source_production_route.next_missing_stable_promotion_class",
            &self.next_missing_stable_promotion_class,
            PRODUCTION_ROUTE_NEXT_MISSING_CLASS,
        )?;
        require_string(
            "source_production_route.route_entrypoint",
            &self.route_entrypoint,
            ROUTE_ENTRYPOINT,
        )?;
        require_string(
            "source_production_route.route_gate",
            &self.route_gate,
            ROUTE_GATE,
        )?;
        require_string(
            "source_production_route.stable_backend_scope",
            &self.stable_backend_scope,
            STABLE_BACKEND_SCOPE,
        )?;
        require_bool(
            "source_production_route.stable_backend_promoted",
            self.stable_backend_promoted,
            true,
        )?;
        require_all(
            "source_production_route.stable_metadata_claims",
            &self.stable_metadata_claims,
            REQUIRED_STABLE_METADATA_CLAIMS,
        )?;
        require_bool(
            "source_production_route.source_fixture_stable_backend_promoted",
            route_gate.route_state.stable_backend_promoted,
            true,
        )?;
        require_string(
            "source_production_route.source_fixture_stable_metadata_completed_class",
            &route_gate.stable_metadata.completed_promotion_class,
            COMPLETED_PROMOTION_CLASS,
        )?;
        require_string(
            "source_production_route.source_fixture_next_missing_class",
            &route_gate.next_missing_stable_promotion_class,
            PRODUCTION_ROUTE_NEXT_MISSING_CLASS,
        )?;
        require_all(
            "source_production_route.source_fixture_stable_metadata_claims",
            &route_gate.route_contract.stable_metadata_claims,
            REQUIRED_STABLE_METADATA_CLAIMS,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StableMetadataPromotionContract {
    pub stable_backend_scope: String,
    pub promotion_decision: String,
    pub promotion_decision_fixture: String,
    pub accepted_input: String,
    pub route_entrypoint: String,
    pub route_gate: String,
    pub completion_handoff_consumed: bool,
    pub final_decision_consumed: bool,
    pub stable_route_metadata_promoted: bool,
    pub preserves_json_schema_backend: bool,
    pub generated_json_schema_output_changed: bool,
    pub core_compiler_path_changed: bool,
    pub arena_ir_proof_used: bool,
    pub external_sidecars_promoted: bool,
}

impl StableMetadataPromotionContract {
    fn validate(
        &self,
        completion_handoff: &SyntaxGoRealSourceGeneratedCodecCompletionHandoffGate,
        route_gate: &SyntaxProductionRouteImplementationGate,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        require_string(
            "promotion_contract.stable_backend_scope",
            &self.stable_backend_scope,
            STABLE_BACKEND_SCOPE,
        )?;
        require_string(
            "promotion_contract.promotion_decision",
            &self.promotion_decision,
            STABLE_METADATA_PROMOTION_DECISION,
        )?;
        require_string(
            "promotion_contract.promotion_decision_fixture",
            &self.promotion_decision_fixture,
            STABLE_METADATA_PROMOTION_DECISION_FIXTURE_PATH,
        )?;
        require_string(
            "promotion_contract.accepted_input",
            &self.accepted_input,
            "checked GrammarIR -> StructuralModel",
        )?;
        require_string(
            "promotion_contract.route_entrypoint",
            &self.route_entrypoint,
            ROUTE_ENTRYPOINT,
        )?;
        require_string(
            "promotion_contract.route_gate",
            &self.route_gate,
            ROUTE_GATE,
        )?;
        require_bool(
            "promotion_contract.completion_handoff_consumed",
            self.completion_handoff_consumed,
            true,
        )?;
        require_bool(
            "promotion_contract.final_decision_consumed",
            self.final_decision_consumed,
            true,
        )?;
        require_bool(
            "promotion_contract.stable_route_metadata_promoted",
            self.stable_route_metadata_promoted,
            true,
        )?;
        require_bool(
            "promotion_contract.preserves_json_schema_backend",
            self.preserves_json_schema_backend,
            true,
        )?;
        require_bool(
            "promotion_contract.generated_json_schema_output_changed",
            self.generated_json_schema_output_changed,
            false,
        )?;
        require_bool(
            "promotion_contract.core_compiler_path_changed",
            self.core_compiler_path_changed,
            false,
        )?;
        require_bool(
            "promotion_contract.arena_ir_proof_used",
            self.arena_ir_proof_used,
            false,
        )?;
        require_bool(
            "promotion_contract.external_sidecars_promoted",
            self.external_sidecars_promoted,
            false,
        )?;
        require_bool(
            "promotion_contract.handoff_json_schema_backend_preserved",
            completion_handoff
                .claim_boundary
                .json_schema_backend_preserved,
            true,
        )?;
        require_bool(
            "promotion_contract.route_json_schema_backend_preserved",
            route_gate
                .compiler_boundary
                .preserves_json_schema_backend_contract,
            true,
        )?;
        require_bool(
            "promotion_contract.route_generated_json_schema_output_changed",
            route_gate
                .compiler_boundary
                .generated_json_schema_output_changed,
            false,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StableMetadataPromotionClaimBoundary {
    pub syntax_level_boundary: String,
    pub allowed_claims: Vec<String>,
    pub blocked_claims: Vec<String>,
    pub source_program_claim_allowed: bool,
    pub whole_file_generated_codec_claim_allowed: bool,
    pub paper_claim_allowed: bool,
    pub model_quality_claim_allowed: bool,
    pub type_correctness_claim_allowed: bool,
    pub completion_or_infill_claim_allowed: bool,
    pub generated_hot_path_claim_allowed: bool,
    pub arena_ir_proof_used: bool,
    pub external_sidecars_promoted: bool,
    pub generated_json_schema_output_changed: bool,
    pub json_schema_backend_preserved: bool,
}

impl StableMetadataPromotionClaimBoundary {
    fn validate(
        &self,
        completion_handoff: &SyntaxGoRealSourceGeneratedCodecCompletionHandoffGate,
        route_gate: &SyntaxProductionRouteImplementationGate,
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
            "claim_boundary.source_program_claim_allowed",
            self.source_program_claim_allowed,
            false,
        )?;
        require_bool(
            "claim_boundary.whole_file_generated_codec_claim_allowed",
            self.whole_file_generated_codec_claim_allowed,
            true,
        )?;
        require_bool(
            "claim_boundary.paper_claim_allowed",
            self.paper_claim_allowed,
            false,
        )?;
        require_bool(
            "claim_boundary.model_quality_claim_allowed",
            self.model_quality_claim_allowed,
            false,
        )?;
        require_bool(
            "claim_boundary.type_correctness_claim_allowed",
            self.type_correctness_claim_allowed,
            false,
        )?;
        require_bool(
            "claim_boundary.completion_or_infill_claim_allowed",
            self.completion_or_infill_claim_allowed,
            false,
        )?;
        require_bool(
            "claim_boundary.generated_hot_path_claim_allowed",
            self.generated_hot_path_claim_allowed,
            false,
        )?;
        require_bool(
            "claim_boundary.arena_ir_proof_used",
            self.arena_ir_proof_used,
            false,
        )?;
        require_bool(
            "claim_boundary.external_sidecars_promoted",
            self.external_sidecars_promoted,
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
        require_bool(
            "claim_boundary.handoff_source_program_claim_allowed",
            completion_handoff
                .claim_boundary
                .source_program_validity_claim_allowed,
            false,
        )?;
        require_bool(
            "claim_boundary.route_external_sidecars_promoted",
            route_gate.stable_metadata.external_sidecars_promoted,
            false,
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

fn require_empty(field: &'static str, actual: &[String]) -> Result<(), SyntaxBackendBoundaryError> {
    if actual.is_empty() {
        Ok(())
    } else {
        Err(SyntaxBackendBoundaryError::new(
            field,
            format!("expected empty list, got {actual:?}"),
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
            "generated-codec round-trip records do not match completion handoff",
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
            "stable metadata promotion source handoff input does not match completion handoff",
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
            "Go generated-codec completion source handoff input does not match completion handoff",
        ))
    }
}
