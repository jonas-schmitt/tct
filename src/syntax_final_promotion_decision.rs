//! Final syntax-backend stable-promotion decision gate.
//!
//! This gate consumes the route-owned validation matrix and its volatile
//! release-depth execution preservation policy. It records whether the syntax
//! backend is ready for a follow-on stable promotion implementation without
//! changing runtime route metadata or the JSON Schema backend in this step.

use crate::syntax_backend_boundary::{SyntaxBackendBoundaryError, SyntaxBackendCompilerBoundary};
use crate::syntax_json_backend_parity::{
    ARTIFACT_KIND as JSON_BACKEND_PARITY_ARTIFACT_KIND,
    COMPLETED_PROMOTION_CLASS as JSON_BACKEND_PARITY_COMPLETED_PROMOTION_CLASS,
    FIXTURE_PATH as JSON_BACKEND_PARITY_FIXTURE_PATH, SyntaxJsonBackendParityGate,
    VERSION as JSON_BACKEND_PARITY_VERSION,
};
use crate::syntax_production_route::{
    COMPLETED_PROMOTION_CLASS as PRODUCTION_ROUTE_COMPLETED_PROMOTION_CLASS,
    FIXTURE_PATH as PRODUCTION_ROUTE_FIXTURE_PATH, ROUTE_ENTRYPOINT as PRODUCTION_ROUTE_ENTRYPOINT,
    ROUTE_GATE, SYNTAX_COMPILER_ROUTE_ENTRYPOINT, SyntaxProductionRouteImplementationGate,
};
use crate::syntax_validation_matrix::{
    COMPLETED_PROMOTION_CLASS as VALIDATION_MATRIX_COMPLETED_PROMOTION_CLASS,
    EXECUTION_REPORT_COMPLETED_PROMOTION_CLASS, EXECUTION_REPORT_PATH,
    EXECUTION_REPORT_PRODUCER_COMMAND, EXECUTION_REPORT_STATUS, EXECUTION_REPORT_VALIDATOR_COMMAND,
    FIXTURE_PATH as VALIDATION_MATRIX_FIXTURE_PATH, REQUIRED_JSON_BACKEND_GRADE_CATEGORIES,
    REQUIRED_ROUTE_OWNED_EVIDENCE, ROUTE_METADATA_FIELD, SyntaxValidationMatrixGate,
};
use crate::syntax_validation_matrix_preservation::{
    ARTIFACT_KIND as MATRIX_PRESERVATION_ARTIFACT_KIND,
    COMPLETED_PROMOTION_CLASS as MATRIX_PRESERVATION_COMPLETED_PROMOTION_CLASS,
    FIXTURE_PATH as MATRIX_PRESERVATION_FIXTURE_PATH,
    NEXT_MISSING_STABLE_PROMOTION_CLASS as MATRIX_PRESERVATION_NEXT_MISSING_CLASS,
    PRESERVATION_POLICY_ID, SyntaxValidationMatrixExecutionPreservationGate,
    TOOL as MATRIX_PRESERVATION_TOOL, VERSION as MATRIX_PRESERVATION_VERSION,
};
use serde::{Deserialize, Serialize};

pub const TOOL: &str = "syntax_backend_final_stable_promotion_decision_gate";
pub const VERSION: u32 = 1;
pub const ARTIFACT_KIND: &str = "tct_syntax_backend_final_stable_promotion_decision_gate";
pub const FIXTURE_PATH: &str =
    "tests/fixtures/syntax_backend/final-stable-promotion-decision-v1.json";
pub const STATUS: &str = "stable_promotion_ready_decision";
pub const COMPLETED_PROMOTION_CLASS: &str = "syntax backend final stable-promotion review gate";
pub const NEXT_MISSING_STABLE_PROMOTION_CLASS: &str =
    "syntax backend stable metadata promotion implementation gate";
pub const DECISION: &str = "ready_for_stable_metadata_promotion_implementation";

const REQUIRED_COMPLETED_CLASSES: &[&str] = &[
    PRODUCTION_ROUTE_COMPLETED_PROMOTION_CLASS,
    JSON_BACKEND_PARITY_COMPLETED_PROMOTION_CLASS,
    VALIDATION_MATRIX_COMPLETED_PROMOTION_CLASS,
    EXECUTION_REPORT_COMPLETED_PROMOTION_CLASS,
    MATRIX_PRESERVATION_COMPLETED_PROMOTION_CLASS,
];

const REQUIRED_EVIDENCE_INPUTS: &[&str] = &[
    VALIDATION_MATRIX_FIXTURE_PATH,
    MATRIX_PRESERVATION_FIXTURE_PATH,
    JSON_BACKEND_PARITY_FIXTURE_PATH,
    PRODUCTION_ROUTE_FIXTURE_PATH,
    "scripts/testing/test-quality-audit.py",
    "reports/m6-cross-target-contract-closure-2026-07-21.md",
    EXECUTION_REPORT_PATH,
];

const REQUIRED_VALIDATION_COMMANDS: &[&str] = &[
    EXECUTION_REPORT_PRODUCER_COMMAND,
    EXECUTION_REPORT_VALIDATOR_COMMAND,
    "./scripts/testing/quality-check.sh",
    "cargo test --test syntax_generated_codec_gate_test",
    "cargo test --test syntax_backend_boundary_test",
    "./scripts/testing/test-syntax-backend-gates.sh",
    "python3 scripts/testing/test-quality-audit.py --fail-on-gaps",
    "git diff --check",
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxFinalStablePromotionDecisionGate {
    pub artifact_kind: String,
    pub version: u32,
    pub tool: String,
    pub status: String,
    pub source_validation_matrix_preservation_gate:
        SyntaxFinalPromotionSourceMatrixPreservationGate,
    pub compiler_boundary: SyntaxBackendCompilerBoundary,
    pub evidence_snapshot: SyntaxFinalPromotionEvidenceSnapshot,
    pub promotion_decision: SyntaxFinalPromotionDecision,
    pub completed_promotion_class: String,
    pub next_missing_stable_promotion_class: String,
}

impl SyntaxFinalStablePromotionDecisionGate {
    pub fn validate_against_preserved_matrix(
        &self,
        preservation_gate: &SyntaxValidationMatrixExecutionPreservationGate,
        matrix_gate: &SyntaxValidationMatrixGate,
        route_gate: &SyntaxProductionRouteImplementationGate,
        json_parity_gate: &SyntaxJsonBackendParityGate,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        require_string("artifact_kind", &self.artifact_kind, ARTIFACT_KIND)?;
        require_u32("version", self.version, VERSION)?;
        require_string("tool", &self.tool, TOOL)?;
        require_string("status", &self.status, STATUS)?;
        self.source_validation_matrix_preservation_gate
            .validate(preservation_gate)?;
        require_decision_boundary(&self.compiler_boundary)?;
        self.evidence_snapshot.validate(
            preservation_gate,
            matrix_gate,
            route_gate,
            json_parity_gate,
        )?;
        self.promotion_decision.validate(route_gate)?;
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
pub struct SyntaxFinalPromotionSourceMatrixPreservationGate {
    pub artifact_kind: String,
    pub version: u32,
    pub tool: String,
    pub module_path: String,
    pub fixture_path: String,
    pub source_validation_matrix_fixture: String,
    pub execution_report_path: String,
    pub report_validator_command: String,
    pub completed_promotion_class: String,
    pub next_missing_stable_promotion_class: String,
}

impl SyntaxFinalPromotionSourceMatrixPreservationGate {
    fn validate(
        &self,
        preservation_gate: &SyntaxValidationMatrixExecutionPreservationGate,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        require_string(
            "source_validation_matrix_preservation_gate.artifact_kind",
            &self.artifact_kind,
            MATRIX_PRESERVATION_ARTIFACT_KIND,
        )?;
        require_u32(
            "source_validation_matrix_preservation_gate.version",
            self.version,
            MATRIX_PRESERVATION_VERSION,
        )?;
        require_string(
            "source_validation_matrix_preservation_gate.tool",
            &self.tool,
            MATRIX_PRESERVATION_TOOL,
        )?;
        require_string(
            "source_validation_matrix_preservation_gate.module_path",
            &self.module_path,
            "src/syntax_validation_matrix_preservation.rs",
        )?;
        require_string(
            "source_validation_matrix_preservation_gate.fixture_path",
            &self.fixture_path,
            MATRIX_PRESERVATION_FIXTURE_PATH,
        )?;
        require_string(
            "source_validation_matrix_preservation_gate.source_validation_matrix_fixture",
            &self.source_validation_matrix_fixture,
            VALIDATION_MATRIX_FIXTURE_PATH,
        )?;
        require_string(
            "source_validation_matrix_preservation_gate.execution_report_path",
            &self.execution_report_path,
            EXECUTION_REPORT_PATH,
        )?;
        require_string(
            "source_validation_matrix_preservation_gate.report_validator_command",
            &self.report_validator_command,
            EXECUTION_REPORT_VALIDATOR_COMMAND,
        )?;
        require_string(
            "source_validation_matrix_preservation_gate.completed_promotion_class",
            &self.completed_promotion_class,
            MATRIX_PRESERVATION_COMPLETED_PROMOTION_CLASS,
        )?;
        require_string(
            "source_validation_matrix_preservation_gate.next_missing_stable_promotion_class",
            &self.next_missing_stable_promotion_class,
            MATRIX_PRESERVATION_NEXT_MISSING_CLASS,
        )?;
        require_string(
            "source_validation_matrix_preservation_gate.source_fixture_completed_class",
            &preservation_gate.completed_promotion_class,
            MATRIX_PRESERVATION_COMPLETED_PROMOTION_CLASS,
        )?;
        require_string(
            "source_validation_matrix_preservation_gate.source_fixture_next_missing_class",
            &preservation_gate.next_missing_stable_promotion_class,
            MATRIX_PRESERVATION_NEXT_MISSING_CLASS,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxFinalPromotionEvidenceSnapshot {
    pub matrix_id: String,
    pub route_entrypoint: String,
    pub production_route_entrypoint: String,
    pub route_gate: String,
    pub route_metadata_field: String,
    pub preservation_policy_id: String,
    pub execution_report_path: String,
    pub execution_report_status: String,
    pub audit_inventory_helper: String,
    pub promotion_readiness_report: String,
    pub json_backend_parity_fixture: String,
    pub completed_classes: Vec<String>,
    pub required_categories: Vec<String>,
    pub route_owned_evidence: Vec<String>,
    pub required_evidence_inputs: Vec<String>,
    pub required_validation_commands: Vec<String>,
    pub report_refresh_validated_for_decision: bool,
    pub audit_inventory_current: bool,
    pub json_schema_backend_unchanged: bool,
    pub arena_ir_proof_used: bool,
}

impl SyntaxFinalPromotionEvidenceSnapshot {
    fn validate(
        &self,
        preservation_gate: &SyntaxValidationMatrixExecutionPreservationGate,
        matrix_gate: &SyntaxValidationMatrixGate,
        route_gate: &SyntaxProductionRouteImplementationGate,
        json_parity_gate: &SyntaxJsonBackendParityGate,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        require_string(
            "evidence_snapshot.matrix_id",
            &self.matrix_id,
            "syntax-json-backend-grade-validation-matrix-v1",
        )?;
        require_string(
            "evidence_snapshot.route_entrypoint",
            &self.route_entrypoint,
            SYNTAX_COMPILER_ROUTE_ENTRYPOINT,
        )?;
        require_string(
            "evidence_snapshot.production_route_entrypoint",
            &self.production_route_entrypoint,
            PRODUCTION_ROUTE_ENTRYPOINT,
        )?;
        require_string("evidence_snapshot.route_gate", &self.route_gate, ROUTE_GATE)?;
        require_string(
            "evidence_snapshot.route_metadata_field",
            &self.route_metadata_field,
            ROUTE_METADATA_FIELD,
        )?;
        require_string(
            "evidence_snapshot.preservation_policy_id",
            &self.preservation_policy_id,
            PRESERVATION_POLICY_ID,
        )?;
        require_string(
            "evidence_snapshot.execution_report_path",
            &self.execution_report_path,
            EXECUTION_REPORT_PATH,
        )?;
        require_string(
            "evidence_snapshot.execution_report_status",
            &self.execution_report_status,
            EXECUTION_REPORT_STATUS,
        )?;
        require_string(
            "evidence_snapshot.audit_inventory_helper",
            &self.audit_inventory_helper,
            "scripts/testing/test-quality-audit.py",
        )?;
        require_string(
            "evidence_snapshot.promotion_readiness_report",
            &self.promotion_readiness_report,
            "reports/m6-cross-target-contract-closure-2026-07-21.md",
        )?;
        require_string(
            "evidence_snapshot.json_backend_parity_fixture",
            &self.json_backend_parity_fixture,
            JSON_BACKEND_PARITY_FIXTURE_PATH,
        )?;
        require_all(
            "evidence_snapshot.completed_classes",
            &self.completed_classes,
            REQUIRED_COMPLETED_CLASSES,
        )?;
        require_all(
            "evidence_snapshot.required_categories",
            &self.required_categories,
            REQUIRED_JSON_BACKEND_GRADE_CATEGORIES,
        )?;
        require_all(
            "evidence_snapshot.route_owned_evidence",
            &self.route_owned_evidence,
            REQUIRED_ROUTE_OWNED_EVIDENCE,
        )?;
        require_all(
            "evidence_snapshot.required_evidence_inputs",
            &self.required_evidence_inputs,
            REQUIRED_EVIDENCE_INPUTS,
        )?;
        require_all(
            "evidence_snapshot.required_validation_commands",
            &self.required_validation_commands,
            REQUIRED_VALIDATION_COMMANDS,
        )?;
        require_bool(
            "evidence_snapshot.report_refresh_validated_for_decision",
            self.report_refresh_validated_for_decision,
            true,
        )?;
        require_bool(
            "evidence_snapshot.audit_inventory_current",
            self.audit_inventory_current,
            true,
        )?;
        require_bool(
            "evidence_snapshot.json_schema_backend_unchanged",
            self.json_schema_backend_unchanged,
            true,
        )?;
        require_bool(
            "evidence_snapshot.arena_ir_proof_used",
            self.arena_ir_proof_used,
            false,
        )?;
        require_string(
            "evidence_snapshot.source_preservation_policy_id",
            &preservation_gate.execution_evidence_policy.policy_id,
            PRESERVATION_POLICY_ID,
        )?;
        require_bool(
            "evidence_snapshot.source_report_refresh_required",
            preservation_gate
                .execution_evidence_policy
                .report_refresh_required_before_stable_promotion,
            true,
        )?;
        require_string(
            "evidence_snapshot.source_matrix_id",
            &matrix_gate.matrix_contract.matrix_id,
            "syntax-json-backend-grade-validation-matrix-v1",
        )?;
        require_string(
            "evidence_snapshot.source_route_entrypoint",
            &matrix_gate.matrix_contract.route_entrypoint,
            SYNTAX_COMPILER_ROUTE_ENTRYPOINT,
        )?;
        require_string(
            "evidence_snapshot.source_production_route_entrypoint",
            &route_gate.route_state.route_entrypoint,
            PRODUCTION_ROUTE_ENTRYPOINT,
        )?;
        require_bool(
            "evidence_snapshot.source_route_enabled",
            route_gate.route_state.route_enabled,
            true,
        )?;
        require_bool(
            "evidence_snapshot.source_route_stable_backend_promoted",
            route_gate.route_state.stable_backend_promoted,
            true,
        )?;
        require_string(
            "evidence_snapshot.source_json_backend_parity_artifact_kind",
            &json_parity_gate.artifact_kind,
            JSON_BACKEND_PARITY_ARTIFACT_KIND,
        )?;
        require_u32(
            "evidence_snapshot.source_json_backend_parity_version",
            json_parity_gate.version,
            JSON_BACKEND_PARITY_VERSION,
        )?;
        require_string(
            "evidence_snapshot.source_json_backend_parity_completed_class",
            &json_parity_gate.completed_promotion_class,
            JSON_BACKEND_PARITY_COMPLETED_PROMOTION_CLASS,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxFinalPromotionDecision {
    pub decision: String,
    pub stable_ready: bool,
    pub stable_backend_promoted_in_this_gate: bool,
    pub route_enabled: bool,
    pub next_task: String,
    pub stable_backend_scope: String,
    pub report_refresh_validated: bool,
    pub preserves_json_schema_backend: bool,
    pub arena_ir_proof_used: bool,
    pub external_sidecars_promoted: bool,
    pub remaining_blockers: Vec<String>,
}

impl SyntaxFinalPromotionDecision {
    fn validate(
        &self,
        route_gate: &SyntaxProductionRouteImplementationGate,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        require_string("promotion_decision.decision", &self.decision, DECISION)?;
        require_bool("promotion_decision.stable_ready", self.stable_ready, true)?;
        require_bool(
            "promotion_decision.stable_backend_promoted_in_this_gate",
            self.stable_backend_promoted_in_this_gate,
            false,
        )?;
        require_bool("promotion_decision.route_enabled", self.route_enabled, true)?;
        require_string(
            "promotion_decision.next_task",
            &self.next_task,
            NEXT_MISSING_STABLE_PROMOTION_CLASS,
        )?;
        require_string(
            "promotion_decision.stable_backend_scope",
            &self.stable_backend_scope,
            "feature_limited_parser_backed_syntax_backend",
        )?;
        require_bool(
            "promotion_decision.report_refresh_validated",
            self.report_refresh_validated,
            true,
        )?;
        require_bool(
            "promotion_decision.preserves_json_schema_backend",
            self.preserves_json_schema_backend,
            true,
        )?;
        require_bool(
            "promotion_decision.arena_ir_proof_used",
            self.arena_ir_proof_used,
            false,
        )?;
        require_bool(
            "promotion_decision.external_sidecars_promoted",
            self.external_sidecars_promoted,
            false,
        )?;
        if !self.remaining_blockers.is_empty() {
            return Err(SyntaxBackendBoundaryError::new(
                "promotion_decision.remaining_blockers",
                "stable-ready decision must not retain blockers",
            ));
        }
        require_bool(
            "promotion_decision.source_route_enabled",
            route_gate.route_state.route_enabled,
            true,
        )?;
        require_bool(
            "promotion_decision.source_route_stable_backend_promoted",
            route_gate.route_state.stable_backend_promoted,
            true,
        )
    }
}

fn require_decision_boundary(
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
    required: &[&str],
) -> Result<(), SyntaxBackendBoundaryError> {
    for expected in required {
        if !actual.iter().any(|candidate| candidate == expected) {
            return Err(SyntaxBackendBoundaryError::new(
                field,
                format!("missing {expected}"),
            ));
        }
    }
    Ok(())
}
