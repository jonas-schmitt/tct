//! Execution-report preservation for source-program external-validation evidence.
//!
//! The external-validation execution report stays volatile under `target/`.
//! This gate keeps the report identity, refresh commands, stable field
//! contract, and blocked-claim boundary checked in without turning parser,
//! formatter, type-checker, functional-validator, or arena IR controls into
//! syntax-level proof.

use crate::syntax_backend_boundary::SyntaxBackendBoundaryError;
use crate::syntax_source_program_external_validation::{
    ARTIFACT_KIND as EXTERNAL_VALIDATION_ARTIFACT_KIND,
    COMPLETED_PROMOTION_CLASS as EXTERNAL_VALIDATION_COMPLETED_PROMOTION_CLASS,
    EXECUTION_REPORT_ARTIFACT_KIND, EXECUTION_REPORT_COMPLETED_PROMOTION_CLASS,
    EXECUTION_REPORT_NEXT_MISSING_STABLE_PROMOTION_CLASS, EXECUTION_REPORT_PATH,
    EXECUTION_REPORT_PRODUCER_COMMAND, EXECUTION_REPORT_STATUS, EXECUTION_REPORT_TOOL,
    EXECUTION_REPORT_VALIDATOR_COMMAND, FIXTURE_PATH as EXTERNAL_VALIDATION_FIXTURE_PATH,
    NEXT_MISSING_STABLE_PROMOTION_CLASS as EXTERNAL_VALIDATION_NEXT_MISSING_STABLE_PROMOTION_CLASS,
    SyntaxSourceProgramExternalValidationBoundary,
    SyntaxSourceProgramExternalValidationControlsGate, TOOL as EXTERNAL_VALIDATION_TOOL,
    VERSION as EXTERNAL_VALIDATION_VERSION,
};
use crate::syntax_source_program_handoff::SyntaxSourceProgramCSharpGeneratedCodecCompletionHandoffInput;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const TOOL: &str =
    "syntax_backend_source_program_external_validation_execution_preservation_gate";
pub const VERSION: u32 = 1;
pub const ARTIFACT_KIND: &str =
    "tct_syntax_backend_source_program_external_validation_execution_preservation_gate";
pub const FIXTURE_PATH: &str = "tests/fixtures/syntax_backend/source-program-external-validation-execution-preservation-v1.json";
pub const STATUS: &str = "source_program_external_validation_execution_report_preservation_policy";
pub const COMPLETED_PROMOTION_CLASS: &str =
    "syntax backend source-program external validation execution preservation gate";
pub const NEXT_MISSING_STABLE_PROMOTION_CLASS: &str =
    "syntax backend source-program type-checker control feasibility gate";
pub const PRESERVATION_POLICY_ID: &str =
    "syntax-source-program-external-validation-execution-preservation-v1";
pub const REPORT_PRODUCER_BIN: &str = "src/bin/tct-syntax-source-program-external-validation.rs";

const REQUIRED_CONTROL_IDS: &[&str] = &[
    "parser_metadata_reproduction",
    "renderer_formatter_sidecar",
    "type_checker",
    "functional_validator",
];

const REQUIRED_STABLE_REPORT_FIELDS: &[&str] = &[
    "artifact_kind",
    "version",
    "tool",
    "status",
    "controls_fixture.fixture_path",
    "controls_fixture.fixture_sha256",
    "source_handoff_report.report_path",
    "source_handoff_report.report_sha256",
    "source_handoff_report.stable_metadata_promotion.artifact_kind",
    "source_handoff_report.stable_metadata_promotion.version",
    "source_handoff_report.stable_metadata_promotion.tool",
    "source_handoff_report.stable_metadata_promotion.fixture_path",
    "source_handoff_report.stable_metadata_promotion.promotion_id",
    "source_handoff_report.stable_metadata_promotion.completed_promotion_class",
    "source_handoff_report.stable_metadata_promotion.next_missing_stable_promotion_class",
    "source_handoff_report.stable_metadata_promotion.source_completion_handoff_completed_class",
    "source_handoff_report.stable_metadata_promotion.source_completion_handoff_supported_record_count",
    "source_handoff_report.stable_metadata_promotion.source_completion_handoff_blocked_record_count",
    "source_handoff_report.stable_metadata_promotion.source_completion_handoff_supported_manifest_paths",
    "source_handoff_report.stable_metadata_promotion.source_completion_handoff_generated_codec_routes",
    "source_handoff_report.stable_metadata_promotion.source_completion_handoff_generated_codec_markers",
    "source_handoff_report.stable_metadata_promotion.source_completion_handoff_source_program_typecheck_execution_report_path",
    "source_handoff_report.stable_metadata_promotion.source_completion_handoff_source_program_typecheck_feasibility_report_path",
    "source_handoff_report.stable_metadata_promotion.source_completion_handoff_source_program_typecheck_execution_completed_promotion_class",
    "source_handoff_report.stable_metadata_promotion.source_completion_handoff_source_program_typecheck_execution_next_missing_stable_promotion_class",
    "source_handoff_report.stable_metadata_promotion.source_completion_handoff_source_program_preserved_typecheck_stable_report_fields",
    "source_handoff_report.stable_metadata_promotion.source_completion_handoff_source_program_preserved_typecheck_report_contract_stable_report_fields",
    "source_handoff_report.stable_metadata_promotion.source_completion_handoff_source_program_preserved_typecheck_blocked_claims",
    "source_handoff_report.stable_metadata_promotion.source_completion_handoff_source_program_preserved_typecheck_no_stable_claims",
    "source_handoff_report.stable_metadata_promotion.source_completion_handoff_source_program_execution_pass_envelope_required_record_count",
    "source_handoff_report.stable_metadata_promotion.source_completion_handoff_source_program_execution_pass_envelope_required_record_status",
    "source_handoff_report.stable_metadata_promotion.source_completion_handoff_source_program_execution_pass_envelope_required_records_all_passed",
    "source_handoff_report.stable_metadata_promotion.source_completion_handoff_source_program_execution_pass_envelope_required_record_ids",
    "source_handoff_report.stable_metadata_promotion.source_completion_handoff_source_program_typechecker_results_sidecar_only",
    "source_handoff_report.stable_metadata_promotion.source_completion_handoff_source_program_type_correctness_claim_allowed",
    "source_handoff_report.stable_metadata_promotion.source_completion_handoff_source_program_typechecker_arena_ir_proof_used",
    "source_handoff_report.stable_metadata_promotion.source_completion_handoff_source_program_typechecker_json_schema_backend_unchanged",
    "source_handoff_report.stable_metadata_promotion.source_completion_handoff_source_program_corpus_check_report_path",
    "source_handoff_report.stable_metadata_promotion.source_completion_handoff_source_program_corpus_eligible_record_count",
    "source_handoff_report.stable_metadata_promotion.source_completion_handoff_source_program_corpus_unsupported_record_count",
    "source_handoff_report.stable_metadata_promotion.source_completion_handoff_source_program_sidecar_validity_claim_allowed",
    "source_handoff_report.stable_metadata_promotion.source_completion_handoff_source_program_sidecar_arena_ir_proof_used",
    "source_handoff_report.stable_metadata_promotion.source_completion_handoff_source_program_sidecar_json_schema_backend_unchanged",
    "source_handoff_report.stable_metadata_promotion.source_completion_handoff_source_program_no_stable_claims",
    "source_handoff_report.stable_metadata_promotion.source_completion_handoff_generated_codec_report_paths",
    "source_handoff_report.stable_metadata_promotion.source_completion_handoff_generated_codec_report_commands",
    "source_handoff_report.stable_metadata_promotion.source_completion_handoff_generated_codec_round_trip_records",
    "source_handoff_report.stable_metadata_promotion.source_completion_handoff_generated_codec_allowed_claims",
    "source_handoff_report.stable_metadata_promotion.source_completion_handoff_generated_codec_whole_file_claim_scope",
    "source_handoff_report.stable_metadata_promotion.source_completion_handoff_generated_codec_no_stable_claims",
    "source_handoff_report.stable_metadata_promotion.completion_handoff_consumed",
    "source_handoff_report.stable_metadata_promotion.final_decision_consumed",
    "source_handoff_report.stable_metadata_promotion.stable_route_metadata_promoted",
    "source_handoff_report.stable_metadata_promotion.preserves_json_schema_backend",
    "source_handoff_report.stable_metadata_promotion.generated_json_schema_output_changed",
    "source_handoff_report.stable_metadata_promotion.source_program_claim_allowed",
    "source_handoff_report.stable_metadata_promotion.arena_ir_proof_used",
    "source_handoff_report.go_generated_codec_completion_handoff.artifact_kind",
    "source_handoff_report.go_generated_codec_completion_handoff.version",
    "source_handoff_report.go_generated_codec_completion_handoff.tool",
    "source_handoff_report.go_generated_codec_completion_handoff.fixture_path",
    "source_handoff_report.go_generated_codec_completion_handoff.completed_promotion_class",
    "source_handoff_report.go_generated_codec_completion_handoff.next_missing_stable_promotion_class",
    "source_handoff_report.go_generated_codec_completion_handoff.supported_record_count",
    "source_handoff_report.go_generated_codec_completion_handoff.blocked_record_count",
    "source_handoff_report.go_generated_codec_completion_handoff.source_program_claim_allowed",
    "source_handoff_report.go_generated_codec_completion_handoff.whole_file_generated_codec_claim_allowed",
    "source_handoff_report.go_generated_codec_completion_handoff.arena_ir_proof_used",
    "source_handoff_report.go_generated_codec_completion_handoff.generated_json_schema_output_changed",
    "source_handoff_report.go_generated_codec_completion_handoff.json_schema_backend_preserved",
    "source_handoff_report.csharp_generated_codec_completion_handoff.artifact_kind",
    "source_handoff_report.csharp_generated_codec_completion_handoff.version",
    "source_handoff_report.csharp_generated_codec_completion_handoff.tool",
    "source_handoff_report.csharp_generated_codec_completion_handoff.fixture_path",
    "source_handoff_report.csharp_generated_codec_completion_handoff.completed_promotion_class",
    "source_handoff_report.csharp_generated_codec_completion_handoff.next_missing_stable_promotion_class",
    "source_handoff_report.csharp_generated_codec_completion_handoff.supported_record_count",
    "source_handoff_report.csharp_generated_codec_completion_handoff.blocked_record_count",
    "source_handoff_report.csharp_generated_codec_completion_handoff.supported_manifest_paths",
    "source_handoff_report.csharp_generated_codec_completion_handoff.generated_codec_routes",
    "source_handoff_report.csharp_generated_codec_completion_handoff.generated_codec_markers",
    "source_handoff_report.csharp_generated_codec_completion_handoff.generated_codec_report_paths",
    "source_handoff_report.csharp_generated_codec_completion_handoff.generated_codec_report_commands",
    "source_handoff_report.csharp_generated_codec_completion_handoff.source_program_claim_allowed",
    "source_handoff_report.csharp_generated_codec_completion_handoff.whole_file_generated_codec_claim_allowed",
    "source_handoff_report.csharp_generated_codec_completion_handoff.type_correctness_claim_allowed",
    "source_handoff_report.csharp_generated_codec_completion_handoff.arena_ir_proof_used",
    "source_handoff_report.csharp_generated_codec_completion_handoff.generated_json_schema_output_changed",
    "source_handoff_report.csharp_generated_codec_completion_handoff.json_schema_backend_preserved",
    "source_handoff_report.record_count",
    "source_handoff_report.records_sha256",
    "control_boundary.syntax_level_boundary",
    "control_boundary.control_evidence_only",
    "control_boundary.generated_codec_validity_claim_only",
    "control_boundary.source_program_claim_allowed",
    "control_boundary.paper_claim_allowed",
    "control_boundary.arena_ir_proof_allowed",
    "control_boundary.external_sidecar_stable_promotion_allowed",
    "control_boundary.generated_hot_path_external_checks_allowed",
    "control_boundary.json_schema_backend_preserved",
    "audit_coverage.quality_audit_command",
    "audit_coverage.execution_report_path",
    "audit_coverage.generated_reports_checked_in",
    "tool_envelope.tct_version",
    "tool_envelope.observed_tools.tool_id",
    "tool_envelope.observed_tools.command",
    "tool_envelope.observed_tools.available",
    "control_summary.control_id",
    "control_summary.required_status",
    "control_summary.execution_status",
    "control_summary.ran",
    "control_summary.record_count",
    "control_summary.passed_records",
    "control_summary.unavailable_records",
    "control_summary.supported_claims",
    "control_summary.unsupported_claims",
    "control_summary.stable_promotion_allowed",
    "control_summary.generated_hot_path_allowed",
    "control_summary.blocker",
    "record_count",
    "records_sha256",
    "records.target_id",
    "records.source_sha256",
    "records.metadata_sha256",
    "records.parser_tool",
    "records.parser_version",
    "records.structural_model_sha256",
    "records.token_layout_sha256",
    "records.route_metadata",
    "records.validation_hook_status",
    "records.sidecar_controls",
    "records.control_results.control_id",
    "records.control_results.execution_status",
    "records.control_results.ran",
    "records.control_results.supported_claims",
    "records.control_results.unsupported_claims",
    "records.control_results.stable_promotion_allowed",
    "records.control_results.generated_hot_path_allowed",
    "records.control_results.blocker",
    "allowed_claims",
    "blocked_claims",
    "validation_commands",
    "completed_promotion_class",
    "next_missing_stable_promotion_class",
];

const REQUIRED_VOLATILE_REPORT_FIELDS: &[&str] = &[
    "tool_envelope.observed_tools.version",
    "tool_envelope.observed_tools.unavailable_reason",
    "controls_fixture.fixture_sha256",
    "source_handoff_report.report_sha256",
    "records_sha256",
];

const REQUIRED_BLOCKED_CLAIMS: &[&str] = &[
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
    "cargo test --test syntax_backend_boundary_test",
    "python3 scripts/testing/test-quality-audit.py --fail-on-gaps",
    "git diff --check",
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxSourceProgramExternalValidationExecutionPreservationGate {
    pub artifact_kind: String,
    pub version: u32,
    pub tool: String,
    pub status: String,
    pub source_external_validation_gate:
        SyntaxSourceProgramExternalValidationPreservationSourceGate,
    pub control_boundary: SyntaxSourceProgramExternalValidationBoundary,
    pub execution_evidence_policy: SyntaxSourceProgramExternalValidationExecutionEvidencePolicy,
    pub preserved_report_contract: SyntaxSourceProgramExternalValidationPreservedReportContract,
    pub no_stable_claims: Vec<String>,
    pub validation_commands: Vec<String>,
    pub completed_promotion_class: String,
    pub next_missing_stable_promotion_class: String,
}

impl SyntaxSourceProgramExternalValidationExecutionPreservationGate {
    pub fn validate_against_external_validation_gate(
        &self,
        external_validation_gate: &SyntaxSourceProgramExternalValidationControlsGate,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        require_string("artifact_kind", &self.artifact_kind, ARTIFACT_KIND)?;
        require_u32("version", self.version, VERSION)?;
        require_string("tool", &self.tool, TOOL)?;
        require_string("status", &self.status, STATUS)?;
        self.source_external_validation_gate
            .validate(external_validation_gate)?;
        require_control_boundary_equal(
            "control_boundary",
            &self.control_boundary,
            &external_validation_gate.control_boundary,
        )?;
        require_preserved_control_boundary(&self.control_boundary)?;
        self.execution_evidence_policy.validate()?;
        self.preserved_report_contract
            .validate(external_validation_gate)?;
        require_all(
            "no_stable_claims",
            &self.no_stable_claims,
            REQUIRED_NO_STABLE_CLAIMS,
        )?;
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
pub struct SyntaxSourceProgramExternalValidationPreservationSourceGate {
    pub artifact_kind: String,
    pub version: u32,
    pub tool: String,
    pub module_path: String,
    pub fixture_path: String,
    pub report_producer_bin: String,
    pub report_artifact_kind: String,
    pub report_tool: String,
    pub report_status: String,
    pub report_path: String,
    pub report_producer_command: String,
    pub report_validator_command: String,
    pub completed_promotion_class: String,
    pub next_missing_stable_promotion_class: String,
}

impl SyntaxSourceProgramExternalValidationPreservationSourceGate {
    fn validate(
        &self,
        external_validation_gate: &SyntaxSourceProgramExternalValidationControlsGate,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        require_string(
            "source_external_validation_gate.artifact_kind",
            &self.artifact_kind,
            EXTERNAL_VALIDATION_ARTIFACT_KIND,
        )?;
        require_u32(
            "source_external_validation_gate.version",
            self.version,
            EXTERNAL_VALIDATION_VERSION,
        )?;
        require_string(
            "source_external_validation_gate.tool",
            &self.tool,
            EXTERNAL_VALIDATION_TOOL,
        )?;
        require_string(
            "source_external_validation_gate.module_path",
            &self.module_path,
            "src/syntax_source_program_external_validation.rs",
        )?;
        require_string(
            "source_external_validation_gate.fixture_path",
            &self.fixture_path,
            EXTERNAL_VALIDATION_FIXTURE_PATH,
        )?;
        require_string(
            "source_external_validation_gate.report_producer_bin",
            &self.report_producer_bin,
            REPORT_PRODUCER_BIN,
        )?;
        require_string(
            "source_external_validation_gate.report_artifact_kind",
            &self.report_artifact_kind,
            EXECUTION_REPORT_ARTIFACT_KIND,
        )?;
        require_string(
            "source_external_validation_gate.report_tool",
            &self.report_tool,
            EXECUTION_REPORT_TOOL,
        )?;
        require_string(
            "source_external_validation_gate.report_status",
            &self.report_status,
            EXECUTION_REPORT_STATUS,
        )?;
        require_string(
            "source_external_validation_gate.report_path",
            &self.report_path,
            EXECUTION_REPORT_PATH,
        )?;
        require_string(
            "source_external_validation_gate.report_producer_command",
            &self.report_producer_command,
            EXECUTION_REPORT_PRODUCER_COMMAND,
        )?;
        require_string(
            "source_external_validation_gate.report_validator_command",
            &self.report_validator_command,
            EXECUTION_REPORT_VALIDATOR_COMMAND,
        )?;
        require_string(
            "source_external_validation_gate.completed_promotion_class",
            &self.completed_promotion_class,
            EXTERNAL_VALIDATION_COMPLETED_PROMOTION_CLASS,
        )?;
        require_string(
            "source_external_validation_gate.next_missing_stable_promotion_class",
            &self.next_missing_stable_promotion_class,
            EXTERNAL_VALIDATION_NEXT_MISSING_STABLE_PROMOTION_CLASS,
        )?;
        require_string(
            "source_external_validation_gate.source_fixture_completed_class",
            &external_validation_gate.completed_promotion_class,
            EXTERNAL_VALIDATION_COMPLETED_PROMOTION_CLASS,
        )?;
        require_string(
            "source_external_validation_gate.source_fixture_next_missing_class",
            &external_validation_gate.next_missing_stable_promotion_class,
            EXTERNAL_VALIDATION_NEXT_MISSING_STABLE_PROMOTION_CLASS,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxSourceProgramExternalValidationExecutionEvidencePolicy {
    pub policy_id: String,
    pub generated_report_path: String,
    pub report_generated_under_target: bool,
    pub report_checked_in: bool,
    pub stable_report_hash_pinned: bool,
    pub refresh_command: String,
    pub validate_command: String,
    pub required_report_status: String,
    pub validates_report_identity_back_to_checked_controls: bool,
    pub validates_report_identity_back_to_source_handoff_report: bool,
    pub preserves_source_handoff_promotion_inputs: bool,
    pub report_refresh_required_before_source_program_claim_promotion: bool,
    pub parser_formatter_controls_sidecar_only: bool,
    pub type_checker_control_available: bool,
    pub functional_validator_control_available: bool,
    pub external_sidecar_stable_promotion_allowed: bool,
    pub arena_ir_proof_used: bool,
    pub json_schema_backend_unchanged: bool,
}

impl SyntaxSourceProgramExternalValidationExecutionEvidencePolicy {
    fn validate(&self) -> Result<(), SyntaxBackendBoundaryError> {
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
            "execution_evidence_policy.required_report_status",
            &self.required_report_status,
            EXECUTION_REPORT_STATUS,
        )?;
        if !self.generated_report_path.starts_with("target/") {
            return Err(SyntaxBackendBoundaryError::new(
                "execution_evidence_policy.generated_report_path",
                "external-validation execution reports must stay under target/",
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
                "execution_evidence_policy.stable_report_hash_pinned",
                self.stable_report_hash_pinned,
                false,
            ),
            (
                "execution_evidence_policy.validates_report_identity_back_to_checked_controls",
                self.validates_report_identity_back_to_checked_controls,
                true,
            ),
            (
                "execution_evidence_policy.validates_report_identity_back_to_source_handoff_report",
                self.validates_report_identity_back_to_source_handoff_report,
                true,
            ),
            (
                "execution_evidence_policy.preserves_source_handoff_promotion_inputs",
                self.preserves_source_handoff_promotion_inputs,
                true,
            ),
            (
                "execution_evidence_policy.report_refresh_required_before_source_program_claim_promotion",
                self.report_refresh_required_before_source_program_claim_promotion,
                true,
            ),
            (
                "execution_evidence_policy.parser_formatter_controls_sidecar_only",
                self.parser_formatter_controls_sidecar_only,
                true,
            ),
            (
                "execution_evidence_policy.type_checker_control_available",
                self.type_checker_control_available,
                false,
            ),
            (
                "execution_evidence_policy.functional_validator_control_available",
                self.functional_validator_control_available,
                false,
            ),
            (
                "execution_evidence_policy.external_sidecar_stable_promotion_allowed",
                self.external_sidecar_stable_promotion_allowed,
                false,
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
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxSourceProgramExternalValidationPreservedReportContract {
    pub stable_report_fields: Vec<String>,
    pub volatile_report_fields: Vec<String>,
    pub required_control_ids: Vec<String>,
    pub required_blocked_claims: Vec<String>,
    pub required_validation_commands: Vec<String>,
    pub csharp_generated_codec_completion_handoff:
        SyntaxSourceProgramCSharpGeneratedCodecCompletionHandoffInput,
    pub completed_promotion_class: String,
    pub next_missing_stable_promotion_class: String,
}

impl SyntaxSourceProgramExternalValidationPreservedReportContract {
    fn validate(
        &self,
        external_validation_gate: &SyntaxSourceProgramExternalValidationControlsGate,
    ) -> Result<(), SyntaxBackendBoundaryError> {
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
            "preserved_report_contract.required_control_ids",
            &self.required_control_ids,
            REQUIRED_CONTROL_IDS,
        )?;
        let source_control_ids = external_validation_gate
            .required_controls
            .iter()
            .map(|control| control.control_id.as_str())
            .collect::<BTreeSet<_>>();
        for control_id in REQUIRED_CONTROL_IDS {
            if !source_control_ids.contains(control_id) {
                return Err(SyntaxBackendBoundaryError::new(
                    "preserved_report_contract.required_control_ids",
                    format!("source external-validation gate is missing {control_id}"),
                ));
            }
        }
        require_all(
            "preserved_report_contract.required_blocked_claims",
            &self.required_blocked_claims,
            REQUIRED_BLOCKED_CLAIMS,
        )?;
        require_all(
            "preserved_report_contract.required_blocked_claims",
            &external_validation_gate.blocked_claims,
            REQUIRED_BLOCKED_CLAIMS,
        )?;
        require_all(
            "preserved_report_contract.required_validation_commands",
            &self.required_validation_commands,
            REQUIRED_VALIDATION_COMMANDS,
        )?;
        require_all(
            "preserved_report_contract.required_validation_commands",
            &self.required_validation_commands,
            &[
                EXECUTION_REPORT_PRODUCER_COMMAND,
                EXECUTION_REPORT_VALIDATOR_COMMAND,
            ],
        )?;
        require_csharp_completion_handoff_equal(
            "preserved_report_contract.csharp_generated_codec_completion_handoff",
            &self.csharp_generated_codec_completion_handoff,
            &external_validation_gate
                .source_handoff
                .csharp_generated_codec_completion_handoff,
        )?;
        require_string(
            "preserved_report_contract.completed_promotion_class",
            &self.completed_promotion_class,
            EXECUTION_REPORT_COMPLETED_PROMOTION_CLASS,
        )?;
        require_string(
            "preserved_report_contract.next_missing_stable_promotion_class",
            &self.next_missing_stable_promotion_class,
            EXECUTION_REPORT_NEXT_MISSING_STABLE_PROMOTION_CLASS,
        )
    }
}

fn require_preserved_control_boundary(
    boundary: &SyntaxSourceProgramExternalValidationBoundary,
) -> Result<(), SyntaxBackendBoundaryError> {
    for (field, actual, expected) in [
        (
            "control_boundary.control_evidence_only",
            boundary.control_evidence_only,
            true,
        ),
        (
            "control_boundary.generated_codec_validity_claim_only",
            boundary.generated_codec_validity_claim_only,
            true,
        ),
        (
            "control_boundary.source_program_claim_allowed",
            boundary.source_program_claim_allowed,
            false,
        ),
        (
            "control_boundary.paper_claim_allowed",
            boundary.paper_claim_allowed,
            false,
        ),
        (
            "control_boundary.arena_ir_proof_allowed",
            boundary.arena_ir_proof_allowed,
            false,
        ),
        (
            "control_boundary.external_sidecar_stable_promotion_allowed",
            boundary.external_sidecar_stable_promotion_allowed,
            false,
        ),
        (
            "control_boundary.generated_hot_path_external_checks_allowed",
            boundary.generated_hot_path_external_checks_allowed,
            false,
        ),
        (
            "control_boundary.json_schema_backend_preserved",
            boundary.json_schema_backend_preserved,
            true,
        ),
    ] {
        require_bool(field, actual, expected)?;
    }
    Ok(())
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

fn require_control_boundary_equal(
    field: &'static str,
    actual: &SyntaxSourceProgramExternalValidationBoundary,
    expected: &SyntaxSourceProgramExternalValidationBoundary,
) -> Result<(), SyntaxBackendBoundaryError> {
    if actual == expected {
        Ok(())
    } else {
        Err(SyntaxBackendBoundaryError::new(
            field,
            "preservation gate control boundary must match the checked external-validation gate",
        ))
    }
}

fn require_csharp_completion_handoff_equal(
    field: &'static str,
    actual: &SyntaxSourceProgramCSharpGeneratedCodecCompletionHandoffInput,
    expected: &SyntaxSourceProgramCSharpGeneratedCodecCompletionHandoffInput,
) -> Result<(), SyntaxBackendBoundaryError> {
    if actual == expected {
        Ok(())
    } else {
        Err(SyntaxBackendBoundaryError::new(
            field,
            "C# generated-codec completion handoff input does not match the source external-validation gate",
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
