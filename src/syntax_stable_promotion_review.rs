//! Sidecar-only stable-promotion review gate.
//!
//! This module pins the review decision after the syntax route-boundary stack
//! reached audit coverage. It does not register a syntax route in
//! `TctCompiler`; it only allows the next tranche to implement the first
//! production route slice behind the existing JSON-backend-grade contracts.

use crate::syntax_audit_coverage_routing::{
    ARTIFACT_KIND as AUDIT_COVERAGE_ROUTING_ARTIFACT_KIND,
    COMPLETED_PROMOTION_CLASS as AUDIT_COVERAGE_ROUTING_COMPLETED_PROMOTION_CLASS,
    FIXTURE_PATH as AUDIT_COVERAGE_ROUTING_FIXTURE_PATH,
    NEXT_MISSING_STABLE_PROMOTION_CLASS as AUDIT_COVERAGE_ROUTING_NEXT_MISSING_STABLE_PROMOTION_CLASS,
    SyntaxAuditCoverageRoutingGate, VERSION as AUDIT_COVERAGE_ROUTING_VERSION,
};
use crate::syntax_backend_boundary::{
    GATE_HELPER_PATH, SyntaxBackendBoundaryError, SyntaxBackendCompilerBoundary,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const TOOL: &str = "syntax_backend_stable_promotion_review_gate";
pub const VERSION: u32 = 1;
pub const ARTIFACT_KIND: &str = "tct_syntax_backend_stable_promotion_review_gate";
pub const FIXTURE_PATH: &str = "tests/fixtures/syntax_backend/stable-promotion-review-gate-v1.json";
pub const STATUS: &str = "sidecar_contract_only";
pub const COMPLETED_PROMOTION_CLASS: &str = "syntax backend stable promotion review gate";
pub const NEXT_MISSING_STABLE_PROMOTION_CLASS: &str =
    "syntax backend production route implementation gate";

const REQUIRED_COMPLETED_ROUTE_CLASSES: &[&str] = &[
    "syntax backend production TctCompiler routing gate",
    "syntax backend JSON-backend parity routing gate",
    "syntax backend target-agnostic routing boundary gate",
    "syntax backend strict/recovery decode routing gate",
    "syntax backend consumed-token accounting routing gate",
    "syntax backend token/vocabulary/BPE routing gate",
    "syntax backend reproducibility routing gate",
    "syntax backend audit coverage routing gate",
];
const REQUIRED_JSON_BACKEND_GRADE_CATEGORIES: &[&str] = &[
    "input_validation",
    "generated_code_compilation",
    "semantic_round_trips",
    "strict_recovery_decode",
    "consumed_token_accounting",
    "token_vocabulary_bpe",
    "generated_artifact_reproducibility",
    "maintained_documentation",
    "audit_coverage",
    "json_backend_parity",
];
const REQUIRED_DOCUMENTS: &[&str] = &[
    "docs/syntax-backend-gate-strategy.md",
    "docs/syntax-structural-model-stable-gate-plan.md",
    "docs/structural-model-boundary-selection.md",
    "reports/m6-cross-target-contract-closure-2026-07-21.md",
    "TODO.md",
];
const REQUIRED_HELPERS: &[&str] = &[
    "scripts/testing/test-quality-audit.py",
    "scripts/testing/test-syntax-backend-gates.sh",
    "tests/syntax_backend_boundary_test.rs",
];
const REQUIRED_EVIDENCE_GATES: &[&str] = &[
    "src/syntax_backend_boundary.rs",
    "src/syntax_generated_codec.rs",
    "src/syntax_tct_compiler_routing.rs",
    "src/syntax_json_backend_parity.rs",
    "src/syntax_target_agnostic_routing_boundary.rs",
    "src/syntax_strict_recovery_decode_routing.rs",
    "src/syntax_consumed_token_accounting_routing.rs",
    "src/syntax_token_vocabulary_bpe_routing.rs",
    "src/syntax_reproducibility_routing.rs",
    "src/syntax_audit_coverage_routing.rs",
];
const REQUIRED_NO_ROUTE_CLAIMS: &[&str] = &[
    "no_stable_tct_compiler_route",
    "no_generated_json_schema_output_change",
    "no_core_compiler_path_change",
    "no_generated_source_substring_dependency",
    "no_stable_backend_promotion_in_this_gate",
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxStablePromotionReviewGate {
    pub artifact_kind: String,
    pub version: u32,
    pub tool: String,
    pub status: String,
    pub source_audit_coverage_routing_gate: SyntaxStablePromotionSourceAuditCoverageGate,
    pub compiler_boundary: SyntaxBackendCompilerBoundary,
    pub route_state: SyntaxStablePromotionRouteState,
    pub review_contract: SyntaxStablePromotionReviewContract,
    pub promotion_decision: SyntaxStablePromotionDecision,
    pub completed_promotion_class: String,
    pub next_missing_stable_promotion_class: String,
}

impl SyntaxStablePromotionReviewGate {
    pub fn validate_against_audit_coverage_gate(
        &self,
        audit_gate: &SyntaxAuditCoverageRoutingGate,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        require_string("artifact_kind", &self.artifact_kind, ARTIFACT_KIND)?;
        require_u32("version", self.version, VERSION)?;
        require_string("tool", &self.tool, TOOL)?;
        require_string("status", &self.status, STATUS)?;
        self.source_audit_coverage_routing_gate
            .validate(audit_gate)?;
        require_sidecar_boundary(&self.compiler_boundary)?;
        self.route_state.validate()?;
        self.review_contract.validate()?;
        self.promotion_decision.validate()?;
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
pub struct SyntaxStablePromotionSourceAuditCoverageGate {
    pub artifact_kind: String,
    pub version: u32,
    pub module_path: String,
    pub fixture_path: String,
    pub test_path: String,
    pub gate_helper: String,
    pub completed_promotion_class: String,
}

impl SyntaxStablePromotionSourceAuditCoverageGate {
    fn validate(
        &self,
        audit_gate: &SyntaxAuditCoverageRoutingGate,
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
            "source_audit_coverage_routing_gate.test_path",
            &self.test_path,
            "tests/syntax_backend_boundary_test.rs",
        )?;
        require_string(
            "source_audit_coverage_routing_gate.gate_helper",
            &self.gate_helper,
            GATE_HELPER_PATH,
        )?;
        require_string(
            "source_audit_coverage_routing_gate.completed_promotion_class",
            &self.completed_promotion_class,
            AUDIT_COVERAGE_ROUTING_COMPLETED_PROMOTION_CLASS,
        )?;
        require_string(
            "source_audit_coverage_routing_gate.source_fixture_completed_class",
            &audit_gate.completed_promotion_class,
            AUDIT_COVERAGE_ROUTING_COMPLETED_PROMOTION_CLASS,
        )?;
        require_string(
            "source_audit_coverage_routing_gate.source_fixture_next_missing_class",
            &audit_gate.next_missing_stable_promotion_class,
            AUDIT_COVERAGE_ROUTING_NEXT_MISSING_STABLE_PROMOTION_CLASS,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxStablePromotionRouteState {
    pub route_registered: bool,
    pub route_enabled: bool,
    pub public_api_changed: bool,
    pub uses_json_schema_compiler: bool,
    pub uses_syntax_renderer_api: bool,
    pub stable_backend_promoted: bool,
    pub first_production_route_slice_allowed_next: bool,
}

impl SyntaxStablePromotionRouteState {
    fn validate(&self) -> Result<(), SyntaxBackendBoundaryError> {
        require_bool("route_state.route_registered", self.route_registered, false)?;
        require_bool("route_state.route_enabled", self.route_enabled, false)?;
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
            false,
        )?;
        require_bool(
            "route_state.stable_backend_promoted",
            self.stable_backend_promoted,
            false,
        )?;
        require_bool(
            "route_state.first_production_route_slice_allowed_next",
            self.first_production_route_slice_allowed_next,
            true,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxStablePromotionReviewContract {
    pub completed_route_boundary_classes: Vec<String>,
    pub json_backend_grade_categories: Vec<String>,
    pub required_documents: Vec<String>,
    pub required_helpers: Vec<String>,
    pub required_evidence_gates: Vec<String>,
    pub no_route_claims: Vec<String>,
    pub current_missing_stable_promotion_class: String,
    pub sidecar_review_only: bool,
    pub fail_on_gaps_required: bool,
    pub does_not_depend_on_generated_source_substrings: bool,
}

impl SyntaxStablePromotionReviewContract {
    fn validate(&self) -> Result<(), SyntaxBackendBoundaryError> {
        require_all(
            "review_contract.completed_route_boundary_classes",
            &self.completed_route_boundary_classes,
            REQUIRED_COMPLETED_ROUTE_CLASSES,
        )?;
        require_all(
            "review_contract.json_backend_grade_categories",
            &self.json_backend_grade_categories,
            REQUIRED_JSON_BACKEND_GRADE_CATEGORIES,
        )?;
        require_all(
            "review_contract.required_documents",
            &self.required_documents,
            REQUIRED_DOCUMENTS,
        )?;
        require_all(
            "review_contract.required_helpers",
            &self.required_helpers,
            REQUIRED_HELPERS,
        )?;
        require_all(
            "review_contract.required_evidence_gates",
            &self.required_evidence_gates,
            REQUIRED_EVIDENCE_GATES,
        )?;
        require_all(
            "review_contract.no_route_claims",
            &self.no_route_claims,
            REQUIRED_NO_ROUTE_CLAIMS,
        )?;
        require_string(
            "review_contract.current_missing_stable_promotion_class",
            &self.current_missing_stable_promotion_class,
            COMPLETED_PROMOTION_CLASS,
        )?;
        require_bool(
            "review_contract.sidecar_review_only",
            self.sidecar_review_only,
            true,
        )?;
        require_bool(
            "review_contract.fail_on_gaps_required",
            self.fail_on_gaps_required,
            true,
        )?;
        require_bool(
            "review_contract.does_not_depend_on_generated_source_substrings",
            self.does_not_depend_on_generated_source_substrings,
            true,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxStablePromotionDecision {
    pub decision: String,
    pub next_task: String,
    pub stable_backend_promoted: bool,
    pub enable_route_in_this_gate: bool,
    pub requires_json_backend_contract_preservation: bool,
    pub requires_validation_before_stable_enablement: bool,
}

impl SyntaxStablePromotionDecision {
    fn validate(&self) -> Result<(), SyntaxBackendBoundaryError> {
        require_string(
            "promotion_decision.decision",
            &self.decision,
            "ready_for_first_production_route_implementation_slice",
        )?;
        require_string(
            "promotion_decision.next_task",
            &self.next_task,
            NEXT_MISSING_STABLE_PROMOTION_CLASS,
        )?;
        require_bool(
            "promotion_decision.stable_backend_promoted",
            self.stable_backend_promoted,
            false,
        )?;
        require_bool(
            "promotion_decision.enable_route_in_this_gate",
            self.enable_route_in_this_gate,
            false,
        )?;
        require_bool(
            "promotion_decision.requires_json_backend_contract_preservation",
            self.requires_json_backend_contract_preservation,
            true,
        )?;
        require_bool(
            "promotion_decision.requires_validation_before_stable_enablement",
            self.requires_validation_before_stable_enablement,
            true,
        )
    }
}

fn require_sidecar_boundary(
    boundary: &SyntaxBackendCompilerBoundary,
) -> Result<(), SyntaxBackendBoundaryError> {
    require_bool(
        "compiler_boundary.sidecar_only",
        boundary.sidecar_only,
        true,
    )?;
    require_bool(
        "compiler_boundary.tct_compiler_route_allowed",
        boundary.tct_compiler_route_allowed,
        false,
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
