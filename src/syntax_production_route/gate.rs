//! Production-route gate checks for syntax metadata promotion.

use crate::syntax_audit_coverage_routing::{
    ARTIFACT_KIND as AUDIT_COVERAGE_ROUTING_ARTIFACT_KIND,
    COMPLETED_PROMOTION_CLASS as AUDIT_COVERAGE_ROUTING_COMPLETED_PROMOTION_CLASS,
    FIXTURE_PATH as AUDIT_COVERAGE_ROUTING_FIXTURE_PATH, VERSION as AUDIT_COVERAGE_ROUTING_VERSION,
};
use crate::syntax_backend_boundary::{
    GATE_HELPER_PATH, SyntaxBackendBoundaryError, SyntaxBackendCompilerBoundary,
};
use crate::syntax_stable_promotion_review::{
    ARTIFACT_KIND as STABLE_PROMOTION_REVIEW_ARTIFACT_KIND,
    COMPLETED_PROMOTION_CLASS as STABLE_PROMOTION_REVIEW_COMPLETED_PROMOTION_CLASS,
    FIXTURE_PATH as STABLE_PROMOTION_REVIEW_FIXTURE_PATH,
    NEXT_MISSING_STABLE_PROMOTION_CLASS as STABLE_PROMOTION_REVIEW_NEXT_MISSING_CLASS,
    SyntaxStablePromotionReviewGate, VERSION as STABLE_PROMOTION_REVIEW_VERSION,
};
use serde::{Deserialize, Serialize};

use super::{
    ARTIFACT_KIND, COMPLETED_PROMOTION_CLASS, CSHARP_CHECKED_METADATA_PROJECTOR_COMMAND,
    GO_CHECKED_METADATA_PROJECTOR_COMMAND, NEXT_MISSING_STABLE_PROMOTION_CLASS,
    PROJECTOR_METADATA_SETUP_COMPLETED_PROMOTION_CLASS, PROJECTOR_METADATA_SETUP_SOURCE_OF_TRUTH,
    REQUIRED_JSON_BACKEND_CHECKS, REQUIRED_JSON_BACKEND_GRADE_CATEGORIES,
    REQUIRED_RENDERER_FUNCTIONS, REQUIRED_ROUTE_SETUP_FIELDS, REQUIRED_ROUTE_SURFACES,
    REQUIRED_STABLE_METADATA_CLAIMS, REQUIRED_TARGETS, REQUIRED_VALIDATION_MATRIX_ROUTE_EVIDENCE,
    ROUTE_ENTRYPOINT, ROUTE_GATE, ROUTE_SETUP_COMPLETED_PROMOTION_CLASS,
    ROUTE_SETUP_NEXT_MISSING_CLASS_SOURCE, ROUTE_SETUP_SOURCE_OF_TRUTH, STABLE_BACKEND_SCOPE,
    STABLE_METADATA_PROMOTION_COMPLETED_CLASS, STABLE_METADATA_PROMOTION_DECISION,
    STABLE_METADATA_PROMOTION_DECISION_FIXTURE_PATH, STATUS, SYNTAX_COMPILER_ROUTE_ENTRYPOINT,
    SYNTAX_COMPILER_VALIDATION_MATRIX_COMPLETED_PROMOTION_CLASS, TOOL, VERSION,
    projector_metadata_setup_fields, require_all, require_bool, require_string, require_u32,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxProductionRouteImplementationGate {
    pub artifact_kind: String,
    pub version: u32,
    pub tool: String,
    pub status: String,
    pub source_stable_promotion_review_gate: SyntaxProductionRouteSourceStablePromotionReview,
    pub route_state: SyntaxProductionRouteState,
    pub stable_metadata: SyntaxProductionRouteStableMetadata,
    pub compiler_boundary: SyntaxBackendCompilerBoundary,
    pub route_contract: SyntaxProductionRouteContract,
    pub route_setup: SyntaxProductionRouteSetup,
    pub json_backend_parity: SyntaxProductionRouteJsonBackendParity,
    pub completed_promotion_class: String,
    pub next_missing_stable_promotion_class: String,
}

impl SyntaxProductionRouteImplementationGate {
    pub fn validate_against_stable_review_gate(
        &self,
        review_gate: &SyntaxStablePromotionReviewGate,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        require_string("artifact_kind", &self.artifact_kind, ARTIFACT_KIND)?;
        require_u32("version", self.version, VERSION)?;
        require_string("tool", &self.tool, TOOL)?;
        require_string("status", &self.status, STATUS)?;
        self.source_stable_promotion_review_gate
            .validate(review_gate)?;
        self.route_state.validate()?;
        self.stable_metadata.validate()?;
        require_production_route_boundary(&self.compiler_boundary)?;
        self.route_contract.validate()?;
        self.route_setup.validate()?;
        self.json_backend_parity.validate()?;
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
pub struct SyntaxProductionRouteSourceStablePromotionReview {
    pub artifact_kind: String,
    pub version: u32,
    pub module_path: String,
    pub fixture_path: String,
    pub test_path: String,
    pub gate_helper: String,
    pub completed_promotion_class: String,
    pub source_audit_coverage_routing_gate: SyntaxProductionRouteSourceAuditCoverage,
}

impl SyntaxProductionRouteSourceStablePromotionReview {
    fn validate(
        &self,
        review_gate: &SyntaxStablePromotionReviewGate,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        require_string(
            "source_stable_promotion_review_gate.artifact_kind",
            &self.artifact_kind,
            STABLE_PROMOTION_REVIEW_ARTIFACT_KIND,
        )?;
        require_u32(
            "source_stable_promotion_review_gate.version",
            self.version,
            STABLE_PROMOTION_REVIEW_VERSION,
        )?;
        require_string(
            "source_stable_promotion_review_gate.module_path",
            &self.module_path,
            "src/syntax_stable_promotion_review.rs",
        )?;
        require_string(
            "source_stable_promotion_review_gate.fixture_path",
            &self.fixture_path,
            STABLE_PROMOTION_REVIEW_FIXTURE_PATH,
        )?;
        require_string(
            "source_stable_promotion_review_gate.test_path",
            &self.test_path,
            "tests/syntax_backend_boundary_test.rs",
        )?;
        require_string(
            "source_stable_promotion_review_gate.gate_helper",
            &self.gate_helper,
            GATE_HELPER_PATH,
        )?;
        require_string(
            "source_stable_promotion_review_gate.completed_promotion_class",
            &self.completed_promotion_class,
            STABLE_PROMOTION_REVIEW_COMPLETED_PROMOTION_CLASS,
        )?;
        require_string(
            "source_stable_promotion_review_gate.source_fixture_completed_class",
            &review_gate.completed_promotion_class,
            STABLE_PROMOTION_REVIEW_COMPLETED_PROMOTION_CLASS,
        )?;
        require_string(
            "source_stable_promotion_review_gate.source_fixture_next_missing_class",
            &review_gate.next_missing_stable_promotion_class,
            STABLE_PROMOTION_REVIEW_NEXT_MISSING_CLASS,
        )?;
        self.source_audit_coverage_routing_gate
            .validate(review_gate)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxProductionRouteSourceAuditCoverage {
    pub artifact_kind: String,
    pub version: u32,
    pub module_path: String,
    pub fixture_path: String,
    pub completed_promotion_class: String,
}

impl SyntaxProductionRouteSourceAuditCoverage {
    fn validate(
        &self,
        review_gate: &SyntaxStablePromotionReviewGate,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        require_string(
            "source_audit_coverage_routing_gate.artifact_kind",
            &self.artifact_kind,
            AUDIT_COVERAGE_ROUTING_ARTIFACT_KIND,
        )?;
        require_u32(
            "source_audit_coverage_routing_gate.version",
            self.version,
            AUDIT_COVERAGE_ROUTING_VERSION,
        )?;
        require_string(
            "source_audit_coverage_routing_gate.module_path",
            &self.module_path,
            "src/syntax_audit_coverage_routing.rs",
        )?;
        require_string(
            "source_audit_coverage_routing_gate.fixture_path",
            &self.fixture_path,
            AUDIT_COVERAGE_ROUTING_FIXTURE_PATH,
        )?;
        require_string(
            "source_audit_coverage_routing_gate.completed_promotion_class",
            &self.completed_promotion_class,
            AUDIT_COVERAGE_ROUTING_COMPLETED_PROMOTION_CLASS,
        )?;
        require_string(
            "source_audit_coverage_routing_gate.source_fixture_completed_class",
            &review_gate
                .source_audit_coverage_routing_gate
                .completed_promotion_class,
            AUDIT_COVERAGE_ROUTING_COMPLETED_PROMOTION_CLASS,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxProductionRouteState {
    pub route_name: String,
    pub route_registered: bool,
    pub route_enabled: bool,
    pub public_api_changed: bool,
    pub uses_json_schema_compiler: bool,
    pub uses_syntax_renderer_api: bool,
    pub stable_backend_promoted: bool,
    pub route_entrypoint: String,
    pub route_gate: String,
}

impl SyntaxProductionRouteState {
    fn validate(&self) -> Result<(), SyntaxBackendBoundaryError> {
        require_string("route_state.route_name", &self.route_name, "syntax")?;
        require_bool("route_state.route_registered", self.route_registered, true)?;
        require_bool("route_state.route_enabled", self.route_enabled, true)?;
        require_bool(
            "route_state.public_api_changed",
            self.public_api_changed,
            false,
        )?;
        require_bool(
            "route_state.uses_json_schema_compiler",
            self.uses_json_schema_compiler,
            false,
        )?;
        require_bool(
            "route_state.uses_syntax_renderer_api",
            self.uses_syntax_renderer_api,
            true,
        )?;
        require_bool(
            "route_state.stable_backend_promoted",
            self.stable_backend_promoted,
            true,
        )?;
        require_string(
            "route_state.route_entrypoint",
            &self.route_entrypoint,
            ROUTE_ENTRYPOINT,
        )?;
        require_string("route_state.route_gate", &self.route_gate, ROUTE_GATE)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxProductionRouteStableMetadata {
    pub stable_backend_scope: String,
    pub promotion_decision: String,
    pub promotion_decision_fixture: String,
    pub completed_promotion_class: String,
    pub next_missing_stable_promotion_class: String,
    pub generated_json_schema_output_changed: bool,
    pub core_compiler_path_changed: bool,
    pub arena_ir_proof_used: bool,
    pub external_sidecars_promoted: bool,
}

impl SyntaxProductionRouteStableMetadata {
    fn validate(&self) -> Result<(), SyntaxBackendBoundaryError> {
        require_string(
            "stable_metadata.stable_backend_scope",
            &self.stable_backend_scope,
            STABLE_BACKEND_SCOPE,
        )?;
        require_string(
            "stable_metadata.promotion_decision",
            &self.promotion_decision,
            STABLE_METADATA_PROMOTION_DECISION,
        )?;
        require_string(
            "stable_metadata.promotion_decision_fixture",
            &self.promotion_decision_fixture,
            STABLE_METADATA_PROMOTION_DECISION_FIXTURE_PATH,
        )?;
        require_string(
            "stable_metadata.completed_promotion_class",
            &self.completed_promotion_class,
            STABLE_METADATA_PROMOTION_COMPLETED_CLASS,
        )?;
        require_string(
            "stable_metadata.next_missing_stable_promotion_class",
            &self.next_missing_stable_promotion_class,
            NEXT_MISSING_STABLE_PROMOTION_CLASS,
        )?;
        require_bool(
            "stable_metadata.generated_json_schema_output_changed",
            self.generated_json_schema_output_changed,
            false,
        )?;
        require_bool(
            "stable_metadata.core_compiler_path_changed",
            self.core_compiler_path_changed,
            false,
        )?;
        require_bool(
            "stable_metadata.arena_ir_proof_used",
            self.arena_ir_proof_used,
            false,
        )?;
        require_bool(
            "stable_metadata.external_sidecars_promoted",
            self.external_sidecars_promoted,
            false,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxProductionRouteContract {
    pub accepted_input: String,
    pub target_ids: Vec<String>,
    pub target_agnostic_compiler_entrypoint: String,
    pub target_inference_source: String,
    pub paired_route_evidence: Vec<String>,
    pub parser_backed_reproducibility_evidence: Vec<String>,
    pub strict_recovery_decode_route_evidence: Vec<String>,
    pub consumed_token_accounting_route_evidence: Vec<String>,
    pub token_vocabulary_bpe_route_evidence: Vec<String>,
    pub validation_matrix_categories: Vec<String>,
    pub validation_matrix_route_evidence: Vec<String>,
    pub validation_matrix_completed_promotion_class: String,
    pub renderer_api_module: String,
    pub renderer_functions: Vec<String>,
    pub output_artifact: String,
    pub required_surfaces: Vec<String>,
    pub stable_metadata_claims: Vec<String>,
}

impl SyntaxProductionRouteContract {
    fn validate(&self) -> Result<(), SyntaxBackendBoundaryError> {
        require_string(
            "route_contract.accepted_input",
            &self.accepted_input,
            "checked GrammarIR -> StructuralModel",
        )?;
        require_all(
            "route_contract.target_ids",
            &self.target_ids,
            REQUIRED_TARGETS,
        )?;
        require_string(
            "route_contract.target_agnostic_compiler_entrypoint",
            &self.target_agnostic_compiler_entrypoint,
            SYNTAX_COMPILER_ROUTE_ENTRYPOINT,
        )?;
        require_string(
            "route_contract.target_inference_source",
            &self.target_inference_source,
            ROUTE_SETUP_SOURCE_OF_TRUTH,
        )?;
        require_all(
            "route_contract.paired_route_evidence",
            &self.paired_route_evidence,
            &[
                "csharp_minimal_class_syntax_v0",
                "csharp_local_string_return_literal_syntax_v0",
                "go_minimal_function_syntax_v0",
                "go_local_declaration_syntax_v0",
                "generated_go_minimal_function_real_source_strings_compare_demo",
            ],
        )?;
        require_all(
            "route_contract.parser_backed_reproducibility_evidence",
            &self.parser_backed_reproducibility_evidence,
            &[
                "csharp_minimal_class_syntax_v0",
                "csharp_local_string_return_literal_syntax_v0",
                "go_minimal_function_syntax_v0",
                "go_local_declaration_syntax_v0",
            ],
        )?;
        require_all(
            "route_contract.strict_recovery_decode_route_evidence",
            &self.strict_recovery_decode_route_evidence,
            &[
                "csharp_minimal_class_syntax_v0",
                "csharp_local_string_return_literal_syntax_v0",
                "go_minimal_function_syntax_v0",
                "go_local_declaration_syntax_v0",
                "generated_go_minimal_function_real_source_strings_compare_demo",
            ],
        )?;
        require_all(
            "route_contract.consumed_token_accounting_route_evidence",
            &self.consumed_token_accounting_route_evidence,
            &[
                "csharp_minimal_class_syntax_v0",
                "csharp_local_string_return_literal_syntax_v0",
                "go_minimal_function_syntax_v0",
                "go_local_declaration_syntax_v0",
                "generated_go_minimal_function_real_source_strings_compare_demo",
            ],
        )?;
        require_all(
            "route_contract.token_vocabulary_bpe_route_evidence",
            &self.token_vocabulary_bpe_route_evidence,
            &[
                "csharp_minimal_class_syntax_v0",
                "csharp_local_string_return_literal_syntax_v0",
                "go_minimal_function_syntax_v0",
                "go_local_declaration_syntax_v0",
                "generated_go_minimal_function_real_source_strings_compare_demo",
            ],
        )?;
        require_all(
            "route_contract.validation_matrix_categories",
            &self.validation_matrix_categories,
            REQUIRED_JSON_BACKEND_GRADE_CATEGORIES,
        )?;
        require_all(
            "route_contract.validation_matrix_route_evidence",
            &self.validation_matrix_route_evidence,
            REQUIRED_VALIDATION_MATRIX_ROUTE_EVIDENCE,
        )?;
        require_string(
            "route_contract.validation_matrix_completed_promotion_class",
            &self.validation_matrix_completed_promotion_class,
            SYNTAX_COMPILER_VALIDATION_MATRIX_COMPLETED_PROMOTION_CLASS,
        )?;
        require_string(
            "route_contract.renderer_api_module",
            &self.renderer_api_module,
            "src/syntax_generated_codec.rs",
        )?;
        require_all(
            "route_contract.renderer_functions",
            &self.renderer_functions,
            REQUIRED_RENDERER_FUNCTIONS,
        )?;
        require_string(
            "route_contract.output_artifact",
            &self.output_artifact,
            "SyntaxGeneratedCodecArtifact",
        )?;
        require_all(
            "route_contract.required_surfaces",
            &self.required_surfaces,
            REQUIRED_ROUTE_SURFACES,
        )?;
        require_all(
            "route_contract.stable_metadata_claims",
            &self.stable_metadata_claims,
            REQUIRED_STABLE_METADATA_CLAIMS,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxProductionRouteSetup {
    pub source_of_truth: String,
    pub completed_promotion_class: String,
    pub construct_identity_fields: Vec<String>,
    pub next_missing_stable_promotion_class_source: String,
    pub projector_metadata_setup_source: String,
    pub projector_metadata_setup_fields: Vec<String>,
    pub completed_projector_metadata_setup_class: String,
    pub checked_csharp_metadata_projector: String,
    pub checked_go_metadata_projector: String,
    pub projector_driven_checked_metadata: bool,
    pub build_script_required: bool,
}

impl SyntaxProductionRouteSetup {
    fn validate(&self) -> Result<(), SyntaxBackendBoundaryError> {
        require_string(
            "route_setup.source_of_truth",
            &self.source_of_truth,
            ROUTE_SETUP_SOURCE_OF_TRUTH,
        )?;
        require_string(
            "route_setup.completed_promotion_class",
            &self.completed_promotion_class,
            ROUTE_SETUP_COMPLETED_PROMOTION_CLASS,
        )?;
        require_all(
            "route_setup.construct_identity_fields",
            &self.construct_identity_fields,
            REQUIRED_ROUTE_SETUP_FIELDS,
        )?;
        require_string(
            "route_setup.next_missing_stable_promotion_class_source",
            &self.next_missing_stable_promotion_class_source,
            ROUTE_SETUP_NEXT_MISSING_CLASS_SOURCE,
        )?;
        require_string(
            "route_setup.projector_metadata_setup_source",
            &self.projector_metadata_setup_source,
            PROJECTOR_METADATA_SETUP_SOURCE_OF_TRUTH,
        )?;
        require_vec_exact(
            "route_setup.projector_metadata_setup_fields",
            &self.projector_metadata_setup_fields,
            &projector_metadata_setup_fields(),
        )?;
        require_string(
            "route_setup.completed_projector_metadata_setup_class",
            &self.completed_projector_metadata_setup_class,
            PROJECTOR_METADATA_SETUP_COMPLETED_PROMOTION_CLASS,
        )?;
        require_string(
            "route_setup.checked_csharp_metadata_projector",
            &self.checked_csharp_metadata_projector,
            CSHARP_CHECKED_METADATA_PROJECTOR_COMMAND,
        )?;
        require_string(
            "route_setup.checked_go_metadata_projector",
            &self.checked_go_metadata_projector,
            GO_CHECKED_METADATA_PROJECTOR_COMMAND,
        )?;
        require_bool(
            "route_setup.projector_driven_checked_metadata",
            self.projector_driven_checked_metadata,
            true,
        )?;
        require_bool(
            "route_setup.build_script_required",
            self.build_script_required,
            false,
        )
    }
}

fn require_vec_exact(
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxProductionRouteJsonBackendParity {
    pub stable_json_schema_backend_unchanged: bool,
    pub generated_json_schema_output_changed: bool,
    pub existing_generated_templates_changed: bool,
    pub json_schema_compiler_passes_changed: bool,
    pub required_validation: Vec<String>,
}

impl SyntaxProductionRouteJsonBackendParity {
    fn validate(&self) -> Result<(), SyntaxBackendBoundaryError> {
        require_bool(
            "json_backend_parity.stable_json_schema_backend_unchanged",
            self.stable_json_schema_backend_unchanged,
            true,
        )?;
        require_bool(
            "json_backend_parity.generated_json_schema_output_changed",
            self.generated_json_schema_output_changed,
            false,
        )?;
        require_bool(
            "json_backend_parity.existing_generated_templates_changed",
            self.existing_generated_templates_changed,
            false,
        )?;
        require_bool(
            "json_backend_parity.json_schema_compiler_passes_changed",
            self.json_schema_compiler_passes_changed,
            false,
        )?;
        require_all(
            "json_backend_parity.required_validation",
            &self.required_validation,
            REQUIRED_JSON_BACKEND_CHECKS,
        )
    }
}

fn require_production_route_boundary(
    boundary: &SyntaxBackendCompilerBoundary,
) -> Result<(), SyntaxBackendBoundaryError> {
    require_bool(
        "compiler_boundary.sidecar_only",
        boundary.sidecar_only,
        false,
    )?;
    require_bool(
        "compiler_boundary.tct_compiler_route_allowed",
        boundary.tct_compiler_route_allowed,
        true,
    )?;
    require_bool(
        "compiler_boundary.generated_json_schema_output_changed",
        boundary.generated_json_schema_output_changed,
        false,
    )?;
    require_bool(
        "compiler_boundary.core_compiler_path_changed",
        boundary.core_compiler_path_changed,
        false,
    )?;
    require_bool(
        "compiler_boundary.production_grammar_compiler_path_changed",
        boundary.production_grammar_compiler_path_changed,
        false,
    )?;
    require_bool(
        "compiler_boundary.preserves_json_schema_backend_contract",
        boundary.preserves_json_schema_backend_contract,
        true,
    )
}
