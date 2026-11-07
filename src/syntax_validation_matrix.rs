//! Route-owned JSON-backend-grade syntax validation matrix.
//!
//! This gate ties each stable-promotion category to the target-agnostic syntax
//! compiler route metadata and to the sidecar control gate that still supplies
//! execution evidence. The route metadata now carries the feature-limited
//! stable scope; arena IR paths remain controls only.

use crate::syntax_backend_boundary::{SyntaxBackendBoundaryError, SyntaxBackendCompilerBoundary};
use crate::syntax_generated_codec::SyntaxGeneratedCodecArtifact;
use crate::syntax_production_route::{
    ARTIFACT_KIND as PRODUCTION_ROUTE_ARTIFACT_KIND,
    COMPLETED_PROMOTION_CLASS as PRODUCTION_ROUTE_COMPLETED_PROMOTION_CLASS,
    FIXTURE_PATH as PRODUCTION_ROUTE_FIXTURE_PATH,
    NEXT_MISSING_STABLE_PROMOTION_CLASS as PRODUCTION_ROUTE_NEXT_MISSING_STABLE_PROMOTION_CLASS,
    STABLE_BACKEND_SCOPE as PRODUCTION_ROUTE_STABLE_BACKEND_SCOPE,
    STABLE_METADATA_PROMOTION_COMPLETED_CLASS, SYNTAX_COMPILER_ROUTE_ENTRYPOINT,
    SYNTAX_COMPILER_VALIDATION_MATRIX_COMPLETED_PROMOTION_CLASS,
    SYNTAX_COMPILER_VALIDATION_MATRIX_NEXT_MISSING_STABLE_PROMOTION_CLASS,
    SyntaxProductionRouteImplementationGate, VERSION as PRODUCTION_ROUTE_VERSION,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeSet;

pub const TOOL: &str = "syntax_backend_validation_matrix_gate";
pub const VERSION: u32 = 1;
pub const ARTIFACT_KIND: &str = "tct_syntax_backend_validation_matrix_gate";
pub const FIXTURE_PATH: &str = "tests/fixtures/syntax_backend/validation-matrix-gate-v1.json";
pub const STATUS: &str = "experimental_route_validation_matrix";
pub const ROUTE_METADATA_FIELD: &str = "target_agnostic_compiler_route";
pub const COMPLETED_PROMOTION_CLASS: &str =
    SYNTAX_COMPILER_VALIDATION_MATRIX_COMPLETED_PROMOTION_CLASS;
pub const NEXT_MISSING_STABLE_PROMOTION_CLASS: &str =
    SYNTAX_COMPILER_VALIDATION_MATRIX_NEXT_MISSING_STABLE_PROMOTION_CLASS;
pub const EXECUTION_REPORT_ARTIFACT_KIND: &str =
    "tct_syntax_backend_validation_matrix_execution_report";
pub const EXECUTION_REPORT_TOOL: &str = "syntax_backend_validation_matrix_runner";
pub const EXECUTION_REPORT_STATUS: &str = "release_depth_execution_report";
pub const EXECUTION_REPORT_RUNNER_SCRIPT: &str = "scripts/testing/run-syntax-validation-matrix.py";
pub const EXECUTION_REPORT_PATH: &str =
    "target/tct-syntax-validation-matrix/execution-report-v1.json";
pub const EXECUTION_REPORT_LOG_DIR: &str = "target/tct-syntax-validation-matrix/logs";
pub const EXECUTION_REPORT_PRODUCER_COMMAND: &str = "python3 scripts/testing/run-syntax-validation-matrix.py --output target/tct-syntax-validation-matrix/execution-report-v1.json";
pub const EXECUTION_REPORT_VALIDATOR_COMMAND: &str = "python3 scripts/testing/run-syntax-validation-matrix.py --validate-report target/tct-syntax-validation-matrix/execution-report-v1.json";
pub const EXECUTION_REPORT_COMPLETED_PROMOTION_CLASS: &str =
    "syntax backend validation matrix execution report helper gate";

pub const REQUIRED_JSON_BACKEND_GRADE_CATEGORIES: &[&str] = &[
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
pub const REQUIRED_ROUTE_OWNED_EVIDENCE: &[&str] = &[
    "csharp_minimal_class_syntax_v0",
    "csharp_local_string_return_literal_syntax_v0",
    "go_minimal_function_syntax_v0",
    "go_local_declaration_syntax_v0",
    "generated_go_minimal_function_real_source_strings_compare_demo",
];
const REQUIRED_ROUTE_ARTIFACT_GRAMMARS: &[&str] = &[
    "csharp_minimal_class_syntax_v0",
    "csharp_local_string_return_literal_syntax_v0",
    "go_minimal_function_syntax_v0",
    "go_local_declaration_syntax_v0",
];
const REQUIRED_VALIDATION_COMMANDS: &[&str] = &[
    "./scripts/testing/quality-check.sh",
    "cargo test --test syntax_generated_codec_gate_test",
    "cargo test --test syntax_backend_boundary_test",
    "./scripts/testing/test-syntax-backend-gates.sh",
    "python3 scripts/testing/test-quality-audit.py --fail-on-gaps",
    "git diff --check",
];
const REQUIRED_STABLE_METADATA_CLAIMS: &[&str] = &[
    "feature_limited_parser_backed_syntax_backend",
    "stable_route_metadata_promoted",
    "no_arena_ir_promotion",
    "no_generated_json_schema_output_change",
    "no_core_compiler_path_change",
    "no_json_schema_compiler_pass_change",
];
const REQUIRED_SIDECAR_CONTROL_GATES: &[&str] = &[
    "tests/fixtures/syntax_backend/input-validation-negative-fixtures-v1.json",
    "tests/syntax_generated_codec_gate_test.rs",
    "tests/fixtures/syntax_backend/strict-recovery-decode-routing-gate-v1.json",
    "tests/fixtures/syntax_backend/consumed-token-accounting-routing-gate-v1.json",
    "tests/fixtures/syntax_backend/token-vocabulary-bpe-routing-gate-v1.json",
    "tests/fixtures/syntax_backend/reproducibility-routing-gate-v1.json",
    "docs/syntax-backend-gate-strategy.md",
    "tests/fixtures/syntax_backend/audit-coverage-routing-gate-v1.json",
    "tests/fixtures/syntax_backend/json-backend-parity-routing-gate-v1.json",
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxValidationMatrixGate {
    pub artifact_kind: String,
    pub version: u32,
    pub tool: String,
    pub status: String,
    pub source_production_route: SyntaxValidationMatrixSourceProductionRoute,
    pub compiler_boundary: SyntaxBackendCompilerBoundary,
    pub matrix_contract: SyntaxValidationMatrixContract,
    pub execution_report_policy: SyntaxValidationMatrixExecutionReportPolicy,
    pub stable_promotion_blocker: SyntaxValidationMatrixStablePromotionBlocker,
    pub completed_promotion_class: String,
    pub next_missing_stable_promotion_class: String,
}

impl SyntaxValidationMatrixGate {
    pub fn validate_against_production_route_gate(
        &self,
        route_gate: &SyntaxProductionRouteImplementationGate,
        route_artifacts: &[SyntaxGeneratedCodecArtifact],
    ) -> Result<(), SyntaxBackendBoundaryError> {
        require_string("artifact_kind", &self.artifact_kind, ARTIFACT_KIND)?;
        require_u32("version", self.version, VERSION)?;
        require_string("tool", &self.tool, TOOL)?;
        require_string("status", &self.status, STATUS)?;
        self.source_production_route.validate(route_gate)?;
        require_route_boundary(&self.compiler_boundary)?;
        self.matrix_contract.validate()?;
        self.execution_report_policy.validate()?;
        self.stable_promotion_blocker.validate()?;
        validate_route_artifacts(route_artifacts)?;
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
pub struct SyntaxValidationMatrixSourceProductionRoute {
    pub artifact_kind: String,
    pub version: u32,
    pub module_path: String,
    pub fixture_path: String,
    pub test_path: String,
    pub generated_codec_gate_test: String,
    pub route_entrypoint: String,
    pub route_metadata_field: String,
    pub completed_promotion_class: String,
    pub current_next_missing_stable_promotion_class: String,
}

impl SyntaxValidationMatrixSourceProductionRoute {
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
            "source_production_route.module_path",
            &self.module_path,
            "src/syntax_production_route.rs",
        )?;
        require_string(
            "source_production_route.fixture_path",
            &self.fixture_path,
            PRODUCTION_ROUTE_FIXTURE_PATH,
        )?;
        require_string(
            "source_production_route.test_path",
            &self.test_path,
            "tests/syntax_backend_boundary_test.rs",
        )?;
        require_string(
            "source_production_route.generated_codec_gate_test",
            &self.generated_codec_gate_test,
            "tests/syntax_generated_codec_gate_test.rs",
        )?;
        require_string(
            "source_production_route.route_entrypoint",
            &self.route_entrypoint,
            SYNTAX_COMPILER_ROUTE_ENTRYPOINT,
        )?;
        require_string(
            "source_production_route.route_metadata_field",
            &self.route_metadata_field,
            ROUTE_METADATA_FIELD,
        )?;
        require_string(
            "source_production_route.completed_promotion_class",
            &self.completed_promotion_class,
            PRODUCTION_ROUTE_COMPLETED_PROMOTION_CLASS,
        )?;
        require_string(
            "source_production_route.source_fixture_completed_class",
            &route_gate.completed_promotion_class,
            PRODUCTION_ROUTE_COMPLETED_PROMOTION_CLASS,
        )?;
        require_string(
            "source_production_route.current_next_missing_stable_promotion_class",
            &self.current_next_missing_stable_promotion_class,
            PRODUCTION_ROUTE_NEXT_MISSING_STABLE_PROMOTION_CLASS,
        )?;
        require_string(
            "source_production_route.source_fixture_next_missing_class",
            &route_gate.next_missing_stable_promotion_class,
            PRODUCTION_ROUTE_NEXT_MISSING_STABLE_PROMOTION_CLASS,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxValidationMatrixContract {
    pub matrix_id: String,
    pub route_entrypoint: String,
    pub route_metadata_field: String,
    pub route_owned_evidence: Vec<String>,
    pub categories: Vec<SyntaxValidationMatrixCategory>,
    pub required_sidecar_control_gates: Vec<String>,
    pub required_validation_commands: Vec<String>,
    pub stable_metadata_claims: Vec<String>,
}

impl SyntaxValidationMatrixContract {
    fn validate(&self) -> Result<(), SyntaxBackendBoundaryError> {
        require_string(
            "matrix_contract.matrix_id",
            &self.matrix_id,
            "syntax-json-backend-grade-validation-matrix-v1",
        )?;
        require_string(
            "matrix_contract.route_entrypoint",
            &self.route_entrypoint,
            SYNTAX_COMPILER_ROUTE_ENTRYPOINT,
        )?;
        require_string(
            "matrix_contract.route_metadata_field",
            &self.route_metadata_field,
            ROUTE_METADATA_FIELD,
        )?;
        require_all(
            "matrix_contract.route_owned_evidence",
            &self.route_owned_evidence,
            REQUIRED_ROUTE_OWNED_EVIDENCE,
        )?;
        require_all(
            "matrix_contract.required_sidecar_control_gates",
            &self.required_sidecar_control_gates,
            REQUIRED_SIDECAR_CONTROL_GATES,
        )?;
        require_all(
            "matrix_contract.required_validation_commands",
            &self.required_validation_commands,
            REQUIRED_VALIDATION_COMMANDS,
        )?;
        require_all(
            "matrix_contract.stable_metadata_claims",
            &self.stable_metadata_claims,
            REQUIRED_STABLE_METADATA_CLAIMS,
        )?;
        let category_ids: Vec<String> = self
            .categories
            .iter()
            .map(|category| category.category_id.clone())
            .collect();
        require_all(
            "matrix_contract.categories",
            &category_ids,
            REQUIRED_JSON_BACKEND_GRADE_CATEGORIES,
        )?;
        for category in &self.categories {
            category.validate()?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxValidationMatrixCategory {
    pub category_id: String,
    pub route_metadata_field: String,
    pub route_metadata_key: String,
    pub route_owned_evidence: Vec<String>,
    pub sidecar_control_gate: SyntaxValidationMatrixSidecarControlGate,
    pub validation_command: String,
    pub stable_promotion_required: bool,
}

impl SyntaxValidationMatrixCategory {
    fn validate(&self) -> Result<(), SyntaxBackendBoundaryError> {
        require_string(
            "matrix_contract.category.route_metadata_field",
            &self.route_metadata_field,
            ROUTE_METADATA_FIELD,
        )?;
        if !REQUIRED_JSON_BACKEND_GRADE_CATEGORIES.contains(&self.category_id.as_str()) {
            return Err(SyntaxBackendBoundaryError::new(
                "matrix_contract.category.category_id",
                format!("unknown category {}", self.category_id),
            ));
        }
        if !expected_route_metadata_keys(self.category_id.as_str())
            .contains(&self.route_metadata_key.as_str())
        {
            return Err(SyntaxBackendBoundaryError::new(
                "matrix_contract.category.route_metadata_key",
                format!(
                    "category {} cannot use route metadata key {}",
                    self.category_id, self.route_metadata_key
                ),
            ));
        }
        require_all(
            "matrix_contract.category.route_owned_evidence",
            &self.route_owned_evidence,
            REQUIRED_ROUTE_OWNED_EVIDENCE,
        )?;
        self.sidecar_control_gate.validate()?;
        if !REQUIRED_VALIDATION_COMMANDS.contains(&self.validation_command.as_str()) {
            return Err(SyntaxBackendBoundaryError::new(
                "matrix_contract.category.validation_command",
                format!(
                    "category {} uses unapproved validation command {}",
                    self.category_id, self.validation_command
                ),
            ));
        }
        require_bool(
            "matrix_contract.category.stable_promotion_required",
            self.stable_promotion_required,
            true,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxValidationMatrixSidecarControlGate {
    pub kind: String,
    pub module_path: String,
    pub fixture_or_doc_path: String,
    pub control_only: bool,
    pub arena_ir_not_proof: bool,
}

impl SyntaxValidationMatrixSidecarControlGate {
    fn validate(&self) -> Result<(), SyntaxBackendBoundaryError> {
        if self.kind != "fixture" && self.kind != "test" && self.kind != "doc" {
            return Err(SyntaxBackendBoundaryError::new(
                "matrix_contract.category.sidecar_control_gate.kind",
                format!("unexpected sidecar control kind {}", self.kind),
            ));
        }
        if !REQUIRED_SIDECAR_CONTROL_GATES.contains(&self.fixture_or_doc_path.as_str()) {
            return Err(SyntaxBackendBoundaryError::new(
                "matrix_contract.category.sidecar_control_gate.fixture_or_doc_path",
                format!(
                    "unexpected sidecar control gate {}",
                    self.fixture_or_doc_path
                ),
            ));
        }
        if self.module_path.contains("goir")
            || self.module_path.contains("arena")
            || self.fixture_or_doc_path.contains("goir")
            || self.fixture_or_doc_path.contains("arena")
        {
            return Err(SyntaxBackendBoundaryError::new(
                "matrix_contract.category.sidecar_control_gate",
                "arena IR controls cannot satisfy syntax-level proof",
            ));
        }
        require_bool(
            "matrix_contract.category.sidecar_control_gate.control_only",
            self.control_only,
            true,
        )?;
        require_bool(
            "matrix_contract.category.sidecar_control_gate.arena_ir_not_proof",
            self.arena_ir_not_proof,
            true,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxValidationMatrixExecutionReportPolicy {
    pub runner_script: String,
    pub report_artifact_kind: String,
    pub report_tool: String,
    pub report_status: String,
    pub generated_report_path: String,
    pub logs_dir: String,
    pub checked_fixture_path: String,
    pub source_report_producer_command: String,
    pub source_report_validator_command: String,
    pub report_generated_under_target: bool,
    pub report_checked_in: bool,
    pub executes_required_commands: bool,
    pub records_per_category_results: bool,
    pub arena_ir_proof_used: bool,
    pub json_schema_backend_unchanged: bool,
    pub completed_promotion_class: String,
}

impl SyntaxValidationMatrixExecutionReportPolicy {
    fn validate(&self) -> Result<(), SyntaxBackendBoundaryError> {
        require_string(
            "execution_report_policy.runner_script",
            &self.runner_script,
            EXECUTION_REPORT_RUNNER_SCRIPT,
        )?;
        require_string(
            "execution_report_policy.report_artifact_kind",
            &self.report_artifact_kind,
            EXECUTION_REPORT_ARTIFACT_KIND,
        )?;
        require_string(
            "execution_report_policy.report_tool",
            &self.report_tool,
            EXECUTION_REPORT_TOOL,
        )?;
        require_string(
            "execution_report_policy.report_status",
            &self.report_status,
            EXECUTION_REPORT_STATUS,
        )?;
        require_string(
            "execution_report_policy.generated_report_path",
            &self.generated_report_path,
            EXECUTION_REPORT_PATH,
        )?;
        require_string(
            "execution_report_policy.logs_dir",
            &self.logs_dir,
            EXECUTION_REPORT_LOG_DIR,
        )?;
        require_string(
            "execution_report_policy.checked_fixture_path",
            &self.checked_fixture_path,
            FIXTURE_PATH,
        )?;
        require_string(
            "execution_report_policy.source_report_producer_command",
            &self.source_report_producer_command,
            EXECUTION_REPORT_PRODUCER_COMMAND,
        )?;
        require_string(
            "execution_report_policy.source_report_validator_command",
            &self.source_report_validator_command,
            EXECUTION_REPORT_VALIDATOR_COMMAND,
        )?;
        if !self.generated_report_path.starts_with("target/")
            || !self.logs_dir.starts_with("target/")
        {
            return Err(SyntaxBackendBoundaryError::new(
                "execution_report_policy.generated_paths",
                "validation matrix execution reports must stay under target/",
            ));
        }
        require_bool(
            "execution_report_policy.report_generated_under_target",
            self.report_generated_under_target,
            true,
        )?;
        require_bool(
            "execution_report_policy.report_checked_in",
            self.report_checked_in,
            false,
        )?;
        require_bool(
            "execution_report_policy.executes_required_commands",
            self.executes_required_commands,
            true,
        )?;
        require_bool(
            "execution_report_policy.records_per_category_results",
            self.records_per_category_results,
            true,
        )?;
        require_bool(
            "execution_report_policy.arena_ir_proof_used",
            self.arena_ir_proof_used,
            false,
        )?;
        require_bool(
            "execution_report_policy.json_schema_backend_unchanged",
            self.json_schema_backend_unchanged,
            true,
        )?;
        require_string(
            "execution_report_policy.completed_promotion_class",
            &self.completed_promotion_class,
            EXECUTION_REPORT_COMPLETED_PROMOTION_CLASS,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxValidationMatrixStablePromotionBlocker {
    pub stable_backend_promoted: bool,
    pub stable_backend_scope: String,
    pub completed_stable_metadata_promotion_class: String,
    pub blocker: String,
    pub requires_matrix_execution: bool,
    pub requires_matrix_preservation: bool,
    pub json_schema_backend_unchanged: bool,
}

impl SyntaxValidationMatrixStablePromotionBlocker {
    fn validate(&self) -> Result<(), SyntaxBackendBoundaryError> {
        require_bool(
            "stable_promotion_blocker.stable_backend_promoted",
            self.stable_backend_promoted,
            true,
        )?;
        require_string(
            "stable_promotion_blocker.stable_backend_scope",
            &self.stable_backend_scope,
            PRODUCTION_ROUTE_STABLE_BACKEND_SCOPE,
        )?;
        require_string(
            "stable_promotion_blocker.completed_stable_metadata_promotion_class",
            &self.completed_stable_metadata_promotion_class,
            STABLE_METADATA_PROMOTION_COMPLETED_CLASS,
        )?;
        require_string(
            "stable_promotion_blocker.blocker",
            &self.blocker,
            PRODUCTION_ROUTE_NEXT_MISSING_STABLE_PROMOTION_CLASS,
        )?;
        require_bool(
            "stable_promotion_blocker.requires_matrix_execution",
            self.requires_matrix_execution,
            false,
        )?;
        require_bool(
            "stable_promotion_blocker.requires_matrix_preservation",
            self.requires_matrix_preservation,
            true,
        )?;
        require_bool(
            "stable_promotion_blocker.json_schema_backend_unchanged",
            self.json_schema_backend_unchanged,
            true,
        )
    }
}

fn validate_route_artifacts(
    route_artifacts: &[SyntaxGeneratedCodecArtifact],
) -> Result<(), SyntaxBackendBoundaryError> {
    if route_artifacts.is_empty() {
        return Err(SyntaxBackendBoundaryError::new(
            "route_artifacts",
            "validation matrix needs parser-backed compiler-route artifacts",
        ));
    }
    let mut grammars = BTreeSet::new();
    for artifact in route_artifacts {
        let route_metadata = artifact.metadata.get(ROUTE_METADATA_FIELD).ok_or_else(|| {
            SyntaxBackendBoundaryError::new(
                "route_artifact.metadata",
                "missing target_agnostic_compiler_route metadata",
            )
        })?;
        require_json_string(
            "route_artifact.metadata.entrypoint",
            route_metadata,
            "entrypoint",
            SYNTAX_COMPILER_ROUTE_ENTRYPOINT,
        )?;
        require_json_bool(
            "route_artifact.metadata.rejects_arena_ir_boundaries",
            route_metadata,
            "rejects_arena_ir_boundaries",
            true,
        )?;
        require_json_bool(
            "route_artifact.metadata.stable_backend_promoted",
            route_metadata,
            "stable_backend_promoted",
            true,
        )?;
        require_json_string(
            "route_artifact.metadata.stable_backend_scope",
            route_metadata,
            "stable_backend_scope",
            PRODUCTION_ROUTE_STABLE_BACKEND_SCOPE,
        )?;
        require_json_string(
            "route_artifact.metadata.stable_metadata_promotion_completed_class",
            route_metadata,
            "stable_metadata_promotion_completed_class",
            STABLE_METADATA_PROMOTION_COMPLETED_CLASS,
        )?;
        require_json_bool(
            "route_artifact.metadata.generated_json_schema_output_changed",
            route_metadata,
            "generated_json_schema_output_changed",
            false,
        )?;
        require_json_string(
            "route_artifact.metadata.validation_matrix_completed_promotion_class",
            route_metadata,
            "validation_matrix_completed_promotion_class",
            COMPLETED_PROMOTION_CLASS,
        )?;
        require_json_string(
            "route_artifact.metadata.validation_matrix_next_missing_stable_promotion_class",
            route_metadata,
            "validation_matrix_next_missing_stable_promotion_class",
            NEXT_MISSING_STABLE_PROMOTION_CLASS,
        )?;
        require_json_array_contains(
            "route_artifact.metadata.validation_matrix_categories",
            route_metadata,
            "validation_matrix_categories",
            REQUIRED_JSON_BACKEND_GRADE_CATEGORIES,
        )?;
        require_json_array_contains(
            "route_artifact.metadata.validation_matrix_route_evidence",
            route_metadata,
            "validation_matrix_route_evidence",
            REQUIRED_ROUTE_OWNED_EVIDENCE,
        )?;
        for (field, expected) in [
            (
                "generated_code_gate_representatives",
                REQUIRED_ROUTE_OWNED_EVIDENCE,
            ),
            (
                "strict_recovery_decode_route_evidence",
                REQUIRED_ROUTE_OWNED_EVIDENCE,
            ),
            (
                "consumed_token_accounting_route_evidence",
                REQUIRED_ROUTE_OWNED_EVIDENCE,
            ),
            (
                "token_vocabulary_bpe_route_evidence",
                REQUIRED_ROUTE_OWNED_EVIDENCE,
            ),
        ] {
            require_json_array_contains(
                "route_artifact.metadata.route_owned_evidence",
                route_metadata,
                field,
                expected,
            )?;
        }
        require_json_array_contains(
            "route_artifact.metadata.parser_backed_reproducibility_evidence",
            route_metadata,
            "parser_backed_reproducibility_evidence",
            REQUIRED_ROUTE_ARTIFACT_GRAMMARS,
        )?;
        let grammar_id = route_metadata
            .get("grammar_id")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                SyntaxBackendBoundaryError::new(
                    "route_artifact.metadata.grammar_id",
                    "missing grammar_id",
                )
            })?;
        grammars.insert(grammar_id.to_string());
    }
    require_all(
        "route_artifacts.grammar_ids",
        &grammars.into_iter().collect::<Vec<_>>(),
        REQUIRED_ROUTE_ARTIFACT_GRAMMARS,
    )
}

fn expected_route_metadata_keys(category_id: &str) -> &'static [&'static str] {
    match category_id {
        "input_validation" => &["accepted_input"],
        "generated_code_compilation" | "semantic_round_trips" => {
            &["generated_code_gate_representatives"]
        }
        "strict_recovery_decode" => &["strict_recovery_decode_route_evidence"],
        "consumed_token_accounting" => &["consumed_token_accounting_route_evidence"],
        "token_vocabulary_bpe" => &["token_vocabulary_bpe_route_evidence"],
        "generated_artifact_reproducibility" => &["parser_backed_reproducibility_evidence"],
        "maintained_documentation" | "audit_coverage" => &["validation_matrix_categories"],
        "json_backend_parity" => &["generated_json_schema_output_changed"],
        _ => &[],
    }
}

fn require_route_boundary(
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

fn require_json_string(
    field: &'static str,
    source: &Value,
    key: &str,
    expected: &str,
) -> Result<(), SyntaxBackendBoundaryError> {
    match source.get(key).and_then(Value::as_str) {
        Some(actual) => require_string(field, actual, expected),
        None => Err(SyntaxBackendBoundaryError::new(
            field,
            format!("missing string metadata key {key}"),
        )),
    }
}

fn require_json_bool(
    field: &'static str,
    source: &Value,
    key: &str,
    expected: bool,
) -> Result<(), SyntaxBackendBoundaryError> {
    match source.get(key).and_then(Value::as_bool) {
        Some(actual) => require_bool(field, actual, expected),
        None => Err(SyntaxBackendBoundaryError::new(
            field,
            format!("missing boolean metadata key {key}"),
        )),
    }
}

fn require_json_array_contains(
    field: &'static str,
    source: &Value,
    key: &str,
    expected: &[&str],
) -> Result<(), SyntaxBackendBoundaryError> {
    let values: Vec<String> = source
        .get(key)
        .and_then(Value::as_array)
        .ok_or_else(|| {
            SyntaxBackendBoundaryError::new(field, format!("missing array metadata key {key}"))
        })?
        .iter()
        .filter_map(Value::as_str)
        .map(str::to_string)
        .collect();
    require_all(field, &values, expected)
}

fn require_all(
    field: &'static str,
    actual: &[String],
    expected: &[&str],
) -> Result<(), SyntaxBackendBoundaryError> {
    let actual: BTreeSet<&str> = actual.iter().map(String::as_str).collect();
    let missing: Vec<&str> = expected
        .iter()
        .copied()
        .filter(|candidate| !actual.contains(candidate))
        .collect();
    if missing.is_empty() {
        Ok(())
    } else {
        Err(SyntaxBackendBoundaryError::new(
            field,
            format!("missing required values: {}", missing.join(", ")),
        ))
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
