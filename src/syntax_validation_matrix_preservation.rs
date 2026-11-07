//! Release-depth execution evidence preservation for the syntax validation matrix.
//!
//! The execution report itself stays volatile under `target/`. This gate keeps
//! the refresh and validation contract checked in so the final promotion review
//! can depend on reproducible release-depth evidence without treating logs or
//! wall-clock timings as durable artifacts.

use crate::syntax_backend_boundary::{SyntaxBackendBoundaryError, SyntaxBackendCompilerBoundary};
use crate::syntax_validation_matrix::{
    ARTIFACT_KIND as VALIDATION_MATRIX_ARTIFACT_KIND,
    COMPLETED_PROMOTION_CLASS as VALIDATION_MATRIX_COMPLETED_PROMOTION_CLASS,
    EXECUTION_REPORT_ARTIFACT_KIND, EXECUTION_REPORT_LOG_DIR, EXECUTION_REPORT_PATH,
    EXECUTION_REPORT_PRODUCER_COMMAND, EXECUTION_REPORT_RUNNER_SCRIPT, EXECUTION_REPORT_STATUS,
    EXECUTION_REPORT_TOOL, EXECUTION_REPORT_VALIDATOR_COMMAND,
    FIXTURE_PATH as VALIDATION_MATRIX_FIXTURE_PATH,
    NEXT_MISSING_STABLE_PROMOTION_CLASS as VALIDATION_MATRIX_NEXT_MISSING_STABLE_PROMOTION_CLASS,
    REQUIRED_JSON_BACKEND_GRADE_CATEGORIES, REQUIRED_ROUTE_OWNED_EVIDENCE, ROUTE_METADATA_FIELD,
    SyntaxValidationMatrixGate, TOOL as VALIDATION_MATRIX_TOOL,
    VERSION as VALIDATION_MATRIX_VERSION,
};
use serde::{Deserialize, Serialize};

pub const TOOL: &str = "syntax_backend_validation_matrix_execution_preservation_gate";
pub const VERSION: u32 = 1;
pub const ARTIFACT_KIND: &str = "tct_syntax_backend_validation_matrix_execution_preservation_gate";
pub const FIXTURE_PATH: &str =
    "tests/fixtures/syntax_backend/validation-matrix-execution-preservation-v1.json";
pub const STATUS: &str = "release_depth_execution_evidence_preservation_policy";
pub const COMPLETED_PROMOTION_CLASS: &str =
    "syntax backend validation matrix execution evidence preservation gate";
pub const NEXT_MISSING_STABLE_PROMOTION_CLASS: &str =
    "syntax backend final stable-promotion review gate";
pub const PRESERVATION_POLICY_ID: &str = "syntax-validation-matrix-execution-preservation-v1";
pub const DRY_RUN_STATUS: &str = "planned_release_depth_execution_report";
pub const COMMAND_TIMEOUT_ENV: &str = "TCT_SYNTAX_VALIDATION_MATRIX_TIMEOUT_SECONDS";

const REQUIRED_STABLE_REPORT_FIELDS: &[&str] = &[
    "artifact_kind",
    "version",
    "tool",
    "status",
    "fixture_path",
    "output_path",
    "log_dir",
    "matrix_artifact_kind",
    "matrix_tool",
    "matrix_id",
    "matrix_status",
    "source_production_route",
    "compiler_boundary",
    "json_schema_backend_unchanged",
    "arena_ir_proof_used",
    "all_passed",
    "timeout_seconds",
    "command_results.command",
    "command_results.status",
    "command_results.exit_code",
    "command_results.category_ids",
    "category_results.category_id",
    "category_results.validation_command",
    "category_results.command_status",
    "category_results.route_metadata_field",
    "category_results.route_metadata_key",
    "category_results.route_owned_evidence",
    "category_results.sidecar_control_gate",
    "category_results.stable_promotion_required",
    "stable_metadata_claims",
    "completed_promotion_class",
    "next_missing_stable_promotion_class",
];

const REQUIRED_VOLATILE_REPORT_FIELDS: &[&str] = &[
    "command_results.duration_ms",
    "command_results.log_path",
    "log_dir_contents",
    "wall_clock_duration",
];

const REQUIRED_NO_STABLE_CLAIMS: &[&str] = &[
    "no_stable_backend_promotion_in_this_gate",
    "no_arena_ir_promotion",
    "no_generated_json_schema_output_change",
    "no_core_compiler_path_change",
    "no_json_schema_compiler_pass_change",
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxValidationMatrixExecutionPreservationGate {
    pub artifact_kind: String,
    pub version: u32,
    pub tool: String,
    pub status: String,
    pub source_validation_matrix_gate: SyntaxValidationMatrixPreservationSourceGate,
    pub compiler_boundary: SyntaxBackendCompilerBoundary,
    pub execution_evidence_policy: SyntaxValidationMatrixExecutionEvidencePolicy,
    pub preserved_report_contract: SyntaxValidationMatrixPreservedReportContract,
    pub no_stable_claims: Vec<String>,
    pub completed_promotion_class: String,
    pub next_missing_stable_promotion_class: String,
}

impl SyntaxValidationMatrixExecutionPreservationGate {
    pub fn validate_against_validation_matrix_gate(
        &self,
        matrix_gate: &SyntaxValidationMatrixGate,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        require_string("artifact_kind", &self.artifact_kind, ARTIFACT_KIND)?;
        require_u32("version", self.version, VERSION)?;
        require_string("tool", &self.tool, TOOL)?;
        require_string("status", &self.status, STATUS)?;
        self.source_validation_matrix_gate.validate(matrix_gate)?;
        require_route_owned_boundary(&self.compiler_boundary)?;
        self.execution_evidence_policy.validate(matrix_gate)?;
        self.preserved_report_contract.validate(matrix_gate)?;
        require_all(
            "no_stable_claims",
            &self.no_stable_claims,
            REQUIRED_NO_STABLE_CLAIMS,
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
pub struct SyntaxValidationMatrixPreservationSourceGate {
    pub artifact_kind: String,
    pub version: u32,
    pub tool: String,
    pub module_path: String,
    pub fixture_path: String,
    pub runner_script: String,
    pub report_artifact_kind: String,
    pub report_tool: String,
    pub report_status: String,
    pub report_path: String,
    pub completed_promotion_class: String,
    pub next_missing_stable_promotion_class: String,
}

impl SyntaxValidationMatrixPreservationSourceGate {
    fn validate(
        &self,
        matrix_gate: &SyntaxValidationMatrixGate,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        require_string(
            "source_validation_matrix_gate.artifact_kind",
            &self.artifact_kind,
            VALIDATION_MATRIX_ARTIFACT_KIND,
        )?;
        require_u32(
            "source_validation_matrix_gate.version",
            self.version,
            VALIDATION_MATRIX_VERSION,
        )?;
        require_string(
            "source_validation_matrix_gate.tool",
            &self.tool,
            VALIDATION_MATRIX_TOOL,
        )?;
        require_string(
            "source_validation_matrix_gate.module_path",
            &self.module_path,
            "src/syntax_validation_matrix.rs",
        )?;
        require_string(
            "source_validation_matrix_gate.fixture_path",
            &self.fixture_path,
            VALIDATION_MATRIX_FIXTURE_PATH,
        )?;
        require_string(
            "source_validation_matrix_gate.runner_script",
            &self.runner_script,
            EXECUTION_REPORT_RUNNER_SCRIPT,
        )?;
        require_string(
            "source_validation_matrix_gate.report_artifact_kind",
            &self.report_artifact_kind,
            EXECUTION_REPORT_ARTIFACT_KIND,
        )?;
        require_string(
            "source_validation_matrix_gate.report_tool",
            &self.report_tool,
            EXECUTION_REPORT_TOOL,
        )?;
        require_string(
            "source_validation_matrix_gate.report_status",
            &self.report_status,
            EXECUTION_REPORT_STATUS,
        )?;
        require_string(
            "source_validation_matrix_gate.report_path",
            &self.report_path,
            EXECUTION_REPORT_PATH,
        )?;
        require_string(
            "source_validation_matrix_gate.completed_promotion_class",
            &self.completed_promotion_class,
            VALIDATION_MATRIX_COMPLETED_PROMOTION_CLASS,
        )?;
        require_string(
            "source_validation_matrix_gate.next_missing_stable_promotion_class",
            &self.next_missing_stable_promotion_class,
            VALIDATION_MATRIX_NEXT_MISSING_STABLE_PROMOTION_CLASS,
        )?;
        require_string(
            "source_validation_matrix_gate.source_fixture_completed_class",
            &matrix_gate.completed_promotion_class,
            VALIDATION_MATRIX_COMPLETED_PROMOTION_CLASS,
        )?;
        require_string(
            "source_validation_matrix_gate.source_fixture_next_missing_class",
            &matrix_gate.next_missing_stable_promotion_class,
            VALIDATION_MATRIX_NEXT_MISSING_STABLE_PROMOTION_CLASS,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxValidationMatrixExecutionEvidencePolicy {
    pub policy_id: String,
    pub generated_report_path: String,
    pub logs_dir: String,
    pub report_generated_under_target: bool,
    pub report_checked_in: bool,
    pub logs_checked_in: bool,
    pub stable_report_hash_pinned: bool,
    pub refresh_command: String,
    pub validate_command: String,
    pub command_timeout_env: String,
    pub required_report_status: String,
    pub dry_run_status_allowed_for_planning: String,
    pub requires_passed_report: bool,
    pub report_refresh_required_before_stable_promotion: bool,
    pub validates_report_identity_back_to_checked_matrix: bool,
    pub arena_ir_proof_used: bool,
    pub json_schema_backend_unchanged: bool,
}

impl SyntaxValidationMatrixExecutionEvidencePolicy {
    fn validate(
        &self,
        matrix_gate: &SyntaxValidationMatrixGate,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        require_string(
            "execution_evidence_policy.policy_id",
            &self.policy_id,
            PRESERVATION_POLICY_ID,
        )?;
        require_string(
            "execution_evidence_policy.generated_report_path",
            &self.generated_report_path,
            EXECUTION_REPORT_PATH,
        )?;
        require_string(
            "execution_evidence_policy.logs_dir",
            &self.logs_dir,
            EXECUTION_REPORT_LOG_DIR,
        )?;
        require_string(
            "execution_evidence_policy.refresh_command",
            &self.refresh_command,
            EXECUTION_REPORT_PRODUCER_COMMAND,
        )?;
        require_string(
            "execution_evidence_policy.validate_command",
            &self.validate_command,
            EXECUTION_REPORT_VALIDATOR_COMMAND,
        )?;
        require_string(
            "execution_evidence_policy.command_timeout_env",
            &self.command_timeout_env,
            COMMAND_TIMEOUT_ENV,
        )?;
        require_string(
            "execution_evidence_policy.required_report_status",
            &self.required_report_status,
            EXECUTION_REPORT_STATUS,
        )?;
        require_string(
            "execution_evidence_policy.dry_run_status_allowed_for_planning",
            &self.dry_run_status_allowed_for_planning,
            DRY_RUN_STATUS,
        )?;
        if !self.generated_report_path.starts_with("target/")
            || !self.logs_dir.starts_with("target/")
        {
            return Err(SyntaxBackendBoundaryError::new(
                "execution_evidence_policy.generated_paths",
                "validation matrix execution evidence must stay under target/",
            ));
        }
        for (field, actual, expected) in [
            (
                "execution_evidence_policy.report_generated_under_target",
                self.report_generated_under_target,
                true,
            ),
            (
                "execution_evidence_policy.report_checked_in",
                self.report_checked_in,
                false,
            ),
            (
                "execution_evidence_policy.logs_checked_in",
                self.logs_checked_in,
                false,
            ),
            (
                "execution_evidence_policy.stable_report_hash_pinned",
                self.stable_report_hash_pinned,
                false,
            ),
            (
                "execution_evidence_policy.requires_passed_report",
                self.requires_passed_report,
                true,
            ),
            (
                "execution_evidence_policy.report_refresh_required_before_stable_promotion",
                self.report_refresh_required_before_stable_promotion,
                true,
            ),
            (
                "execution_evidence_policy.validates_report_identity_back_to_checked_matrix",
                self.validates_report_identity_back_to_checked_matrix,
                true,
            ),
            (
                "execution_evidence_policy.arena_ir_proof_used",
                self.arena_ir_proof_used,
                false,
            ),
            (
                "execution_evidence_policy.json_schema_backend_unchanged",
                self.json_schema_backend_unchanged,
                true,
            ),
        ] {
            require_bool(field, actual, expected)?;
        }
        require_string(
            "execution_evidence_policy.source_generated_report_path",
            &matrix_gate.execution_report_policy.generated_report_path,
            EXECUTION_REPORT_PATH,
        )?;
        require_string(
            "execution_evidence_policy.source_report_producer_command",
            &matrix_gate
                .execution_report_policy
                .source_report_producer_command,
            EXECUTION_REPORT_PRODUCER_COMMAND,
        )?;
        require_string(
            "execution_evidence_policy.source_report_validator_command",
            &matrix_gate
                .execution_report_policy
                .source_report_validator_command,
            EXECUTION_REPORT_VALIDATOR_COMMAND,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxValidationMatrixPreservedReportContract {
    pub matrix_id: String,
    pub route_entrypoint: String,
    pub route_metadata_field: String,
    pub stable_report_fields: Vec<String>,
    pub volatile_report_fields: Vec<String>,
    pub required_commands: Vec<String>,
    pub required_categories: Vec<String>,
    pub required_route_owned_evidence: Vec<String>,
    pub required_no_stable_claims: Vec<String>,
    pub required_sidecar_controls: Vec<String>,
}

impl SyntaxValidationMatrixPreservedReportContract {
    fn validate(
        &self,
        matrix_gate: &SyntaxValidationMatrixGate,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        require_string(
            "preserved_report_contract.matrix_id",
            &self.matrix_id,
            "syntax-json-backend-grade-validation-matrix-v1",
        )?;
        require_string(
            "preserved_report_contract.route_entrypoint",
            &self.route_entrypoint,
            "compile_checked_syntax_compiler_route",
        )?;
        require_string(
            "preserved_report_contract.route_metadata_field",
            &self.route_metadata_field,
            ROUTE_METADATA_FIELD,
        )?;
        require_all(
            "preserved_report_contract.stable_report_fields",
            &self.stable_report_fields,
            REQUIRED_STABLE_REPORT_FIELDS,
        )?;
        require_all(
            "preserved_report_contract.volatile_report_fields",
            &self.volatile_report_fields,
            REQUIRED_VOLATILE_REPORT_FIELDS,
        )?;
        require_all(
            "preserved_report_contract.required_commands",
            &self.required_commands,
            string_slice(&matrix_gate.matrix_contract.required_validation_commands).as_slice(),
        )?;
        require_all(
            "preserved_report_contract.required_categories",
            &self.required_categories,
            REQUIRED_JSON_BACKEND_GRADE_CATEGORIES,
        )?;
        require_all(
            "preserved_report_contract.required_route_owned_evidence",
            &self.required_route_owned_evidence,
            REQUIRED_ROUTE_OWNED_EVIDENCE,
        )?;
        require_all(
            "preserved_report_contract.required_no_stable_claims",
            &self.required_no_stable_claims,
            REQUIRED_NO_STABLE_CLAIMS,
        )?;
        require_all(
            "preserved_report_contract.required_sidecar_controls",
            &self.required_sidecar_controls,
            string_slice(&matrix_gate.matrix_contract.required_sidecar_control_gates).as_slice(),
        )
    }
}

fn require_route_owned_boundary(
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

fn string_slice(values: &[String]) -> Vec<&str> {
    values.iter().map(String::as_str).collect()
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
