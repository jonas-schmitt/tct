//! Source-program external validation control evidence gate.
//!
//! This gate consumes the generated source-program handoff report and classifies
//! external parser, renderer, formatter, compiler, and functional-validator
//! evidence as controls only. It does not promote those controls into the
//! syntax-level validity-by-construction boundary.
//!
//! Audit markers: `tct_syntax_backend_csharp_real_source_generated_codec_completion_handoff_gate`,
//! `serialization/Loan.cs`,
//! `target/tct-syntax-real-source-demo/csharp-loan-generated-codec-demo.json`.

use crate::provenance::json_sha256;
use crate::syntax_backend_boundary::SyntaxBackendBoundaryError;
use crate::syntax_source_program_handoff::{
    self, SYNTAX_LEVEL_BOUNDARY, SyntaxSourceProgramCSharpGeneratedCodecCompletionHandoffInput,
    SyntaxSourceProgramClaimBoundary, SyntaxSourceProgramClaimHandoffGate,
    SyntaxSourceProgramGoGeneratedCodecCompletionHandoffInput, SyntaxSourceProgramHandoffRecord,
    SyntaxSourceProgramHandoffReport, SyntaxSourceProgramStableMetadataPromotionInput,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeSet;

pub const TOOL: &str = "syntax_backend_source_program_external_validation_controls";
pub const VERSION: u32 = 1;
pub const ARTIFACT_KIND: &str = "tct_syntax_backend_source_program_external_validation_controls";
pub const FIXTURE_PATH: &str =
    "tests/fixtures/syntax_backend/source-program-external-validation-controls-v1.json";
pub const STATUS: &str = "checked_external_validation_control_gate";
pub const COMPLETED_PROMOTION_CLASS: &str =
    "syntax backend source-program external validation control evidence gate";
pub const NEXT_MISSING_STABLE_PROMOTION_CLASS: &str =
    "syntax backend source-program external validation execution report gate";
pub const EXECUTION_REPORT_TOOL: &str =
    "syntax_backend_source_program_external_validation_execution_report";
pub const EXECUTION_REPORT_ARTIFACT_KIND: &str =
    "tct_syntax_backend_source_program_external_validation_execution_report";
pub const EXECUTION_REPORT_STATUS: &str = "generated_external_validation_execution_report";
pub const EXECUTION_REPORT_PATH: &str =
    "target/tct-syntax-source-program-external-validation/execution-report-v1.json";
pub const EXECUTION_REPORT_PRODUCER_COMMAND: &str = "cargo run --bin tct-syntax-source-program-external-validation -- --source-program-handoff-report target/tct-syntax-source-program-handoff/source-program-claim-handoff-v1.json --output target/tct-syntax-source-program-external-validation/execution-report-v1.json";
pub const EXECUTION_REPORT_VALIDATOR_COMMAND: &str = "cargo run --bin tct-syntax-source-program-external-validation -- --validate-report target/tct-syntax-source-program-external-validation/execution-report-v1.json";
pub const EXECUTION_REPORT_COMPLETED_PROMOTION_CLASS: &str =
    "syntax backend source-program external validation execution report gate";
pub const EXECUTION_REPORT_NEXT_MISSING_STABLE_PROMOTION_CLASS: &str =
    "syntax backend source-program external validation execution preservation gate";

const REQUIRED_TARGETS: &[&str] = &["csharp", "go"];
const REQUIRED_CONTROL_IDS: &[&str] = &[
    "parser_metadata_reproduction",
    "renderer_formatter_sidecar",
    "type_checker",
    "functional_validator",
];
const REQUIRED_PRESENT_CONTROL_IDS: &[&str] =
    &["parser_metadata_reproduction", "renderer_formatter_sidecar"];
const REQUIRED_MISSING_CONTROL_IDS: &[&str] = &["type_checker", "functional_validator"];
const REQUIRED_RECORD_CONTROL_FIELDS: &[&str] = &[
    "parser_tool",
    "parser_version",
    "source_reparse_or_equivalent_check",
    "renderer_formatter_status",
    "external_check_policy",
    "validation_hook_sidecar_status",
    "input_diagnostics_contract_sidecar_status",
    "generated_artifact_replay_compiler_boundary",
];
const REQUIRED_EXTERNAL_CHECKS: &[&str] = &[
    "parser",
    "renderer",
    "formatter",
    "compiler",
    "functional_validator",
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
const REQUIRED_VALIDATION_COMMANDS: &[&str] = &[
    "cargo test --test syntax_backend_boundary_test",
    "python3 scripts/testing/test-quality-audit.py --fail-on-gaps",
    "git diff --check",
];
const REQUIRED_EXECUTION_VALIDATION_COMMANDS: &[&str] = &[
    "cargo test --test syntax_backend_boundary_test",
    "cargo test --test maintained_cli_binaries_test syntax_source_program_external_validation_cli_writes_report -- --exact",
    "python3 scripts/testing/test-quality-audit.py --fail-on-gaps",
    "git diff --check",
];
const SOURCE_PROGRAM_MODULE_PATH: &str = "src/syntax_source_program_handoff.rs";
const EXTERNAL_VALIDATION_MODULE_PATH: &str = "src/syntax_source_program_external_validation.rs";
const EXTERNAL_VALIDATION_FIXTURE_PATH: &str =
    "tests/fixtures/syntax_backend/source-program-external-validation-controls-v1.json";
const BOUNDARY_UNSUPPORTED_CLAIMS: &[&str] = &[
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxSourceProgramExternalValidationControlsGate {
    pub artifact_kind: String,
    pub version: u32,
    pub tool: String,
    pub status: String,
    pub source_handoff: SyntaxSourceProgramExternalValidationSourceHandoff,
    pub control_boundary: SyntaxSourceProgramExternalValidationBoundary,
    pub required_controls: Vec<SyntaxSourceProgramExternalValidationControl>,
    pub required_record_control_fields: Vec<String>,
    pub blocked_claims: Vec<String>,
    pub validation_commands: Vec<String>,
    pub completed_promotion_class: String,
    pub next_missing_stable_promotion_class: String,
}

impl SyntaxSourceProgramExternalValidationControlsGate {
    pub fn validate_against_handoff(
        &self,
        handoff_gate: &SyntaxSourceProgramClaimHandoffGate,
        handoff_report: &SyntaxSourceProgramHandoffReport,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        handoff_gate.validate_report(handoff_report)?;
        require_string("artifact_kind", &self.artifact_kind, ARTIFACT_KIND)?;
        require_u32("version", self.version, VERSION)?;
        require_string("tool", &self.tool, TOOL)?;
        require_string("status", &self.status, STATUS)?;
        self.source_handoff.validate(handoff_gate, handoff_report)?;
        self.control_boundary
            .validate(&handoff_report.claim_boundary)?;
        validate_controls(&self.required_controls)?;
        require_all(
            "required_record_control_fields",
            &self.required_record_control_fields,
            REQUIRED_RECORD_CONTROL_FIELDS,
        )?;
        require_all(
            "blocked_claims",
            &self.blocked_claims,
            REQUIRED_BLOCKED_CLAIMS,
        )?;
        require_all(
            "validation_commands",
            &self.validation_commands,
            REQUIRED_VALIDATION_COMMANDS,
        )?;
        require_string(
            "handoff_report.next_missing_stable_promotion_class",
            &handoff_report.next_missing_stable_promotion_class,
            COMPLETED_PROMOTION_CLASS,
        )?;
        require_all(
            "handoff_report.blocked_claims",
            &handoff_report.blocked_claims,
            REQUIRED_BLOCKED_CLAIMS,
        )?;
        validate_handoff_records(&handoff_report.records)?;
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

    pub fn build_execution_report(
        &self,
        handoff_gate: &SyntaxSourceProgramClaimHandoffGate,
        controls_fixture_path: &str,
        handoff_report_path: &str,
        handoff_report: &SyntaxSourceProgramHandoffReport,
        tool_envelope: SyntaxSourceProgramExternalValidationToolEnvelope,
    ) -> Result<SyntaxSourceProgramExternalValidationExecutionReport, SyntaxBackendBoundaryError>
    {
        self.validate_against_handoff(handoff_gate, handoff_report)?;
        let controls_fixture_sha256 = json_sha256(self).map_err(|error| {
            SyntaxBackendBoundaryError::new(
                "controls_fixture",
                format!("failed to hash external-validation controls fixture: {error}"),
            )
        })?;
        let handoff_report_sha256 = json_sha256(handoff_report).map_err(|error| {
            SyntaxBackendBoundaryError::new(
                "source_handoff_report",
                format!("failed to hash source-program handoff report: {error}"),
            )
        })?;
        let records = handoff_report
            .records
            .iter()
            .map(|record| self.execution_record(record, &tool_envelope))
            .collect::<Result<Vec<_>, _>>()?;
        let records_sha256 = json_sha256(&records).map_err(|error| {
            SyntaxBackendBoundaryError::new(
                "execution_records",
                format!("failed to hash external-validation execution records: {error}"),
            )
        })?;
        let control_summary = self.execution_control_summary(
            handoff_report.records.len(),
            &handoff_report.records,
            &tool_envelope,
        )?;
        Ok(SyntaxSourceProgramExternalValidationExecutionReport {
            artifact_kind: EXECUTION_REPORT_ARTIFACT_KIND.to_string(),
            version: VERSION,
            tool: EXECUTION_REPORT_TOOL.to_string(),
            status: EXECUTION_REPORT_STATUS.to_string(),
            controls_fixture: SyntaxSourceProgramExternalValidationControlsFixtureReport {
                fixture_path: controls_fixture_path.to_string(),
                fixture_sha256: controls_fixture_sha256,
                artifact_kind: self.artifact_kind.clone(),
                tool: self.tool.clone(),
                status: self.status.clone(),
                completed_promotion_class: self.completed_promotion_class.clone(),
                next_missing_stable_promotion_class: self
                    .next_missing_stable_promotion_class
                    .clone(),
            },
            source_handoff_report: SyntaxSourceProgramExternalValidationSourceHandoffReport {
                report_path: handoff_report_path.to_string(),
                report_sha256: handoff_report_sha256,
                artifact_kind: handoff_report.artifact_kind.clone(),
                tool: handoff_report.tool.clone(),
                status: handoff_report.status.clone(),
                profile_id: handoff_report.profile_id.clone(),
                stable_metadata_promotion: handoff_report.stable_metadata_promotion.clone(),
                go_generated_codec_completion_handoff: handoff_report
                    .go_generated_codec_completion_handoff
                    .clone(),
                csharp_generated_codec_completion_handoff: handoff_report
                    .csharp_generated_codec_completion_handoff
                    .clone(),
                record_count: handoff_report.record_count,
                records_sha256: handoff_report.records_sha256.clone(),
                completed_promotion_class: handoff_report.completed_promotion_class.clone(),
                next_missing_stable_promotion_class: handoff_report
                    .next_missing_stable_promotion_class
                    .clone(),
            },
            control_boundary: self.control_boundary.clone(),
            audit_coverage: SyntaxSourceProgramExternalValidationAuditCoverage {
                quality_audit_command:
                    "python3 scripts/testing/test-quality-audit.py --fail-on-gaps".to_string(),
                module_path: EXTERNAL_VALIDATION_MODULE_PATH.to_string(),
                fixture_path: EXTERNAL_VALIDATION_FIXTURE_PATH.to_string(),
                source_program_handoff_module_path: SOURCE_PROGRAM_MODULE_PATH.to_string(),
                source_program_handoff_fixture_path: syntax_source_program_handoff::FIXTURE_PATH
                    .to_string(),
                execution_report_path: EXECUTION_REPORT_PATH.to_string(),
                generated_reports_checked_in: false,
            },
            tool_envelope,
            control_summary,
            record_count: records.len(),
            records_sha256,
            records,
            allowed_claims: handoff_report.allowed_claims.clone(),
            blocked_claims: merged_blocked_claims(
                &handoff_report.blocked_claims,
                &self.blocked_claims,
            ),
            validation_commands: REQUIRED_EXECUTION_VALIDATION_COMMANDS
                .iter()
                .map(|command| (*command).to_string())
                .chain([
                    EXECUTION_REPORT_PRODUCER_COMMAND.to_string(),
                    EXECUTION_REPORT_VALIDATOR_COMMAND.to_string(),
                ])
                .collect(),
            completed_promotion_class: EXECUTION_REPORT_COMPLETED_PROMOTION_CLASS.to_string(),
            next_missing_stable_promotion_class:
                EXECUTION_REPORT_NEXT_MISSING_STABLE_PROMOTION_CLASS.to_string(),
        })
    }

    pub fn validate_execution_report(
        &self,
        handoff_gate: &SyntaxSourceProgramClaimHandoffGate,
        handoff_report: &SyntaxSourceProgramHandoffReport,
        report: &SyntaxSourceProgramExternalValidationExecutionReport,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        self.validate_against_handoff(handoff_gate, handoff_report)?;
        require_string(
            "execution_report.artifact_kind",
            &report.artifact_kind,
            EXECUTION_REPORT_ARTIFACT_KIND,
        )?;
        require_u32("execution_report.version", report.version, VERSION)?;
        require_string("execution_report.tool", &report.tool, EXECUTION_REPORT_TOOL)?;
        require_string(
            "execution_report.status",
            &report.status,
            EXECUTION_REPORT_STATUS,
        )?;
        report.controls_fixture.validate(self)?;
        report.source_handoff_report.validate(handoff_report)?;
        require_control_boundary_equal(
            "execution_report.control_boundary",
            &report.control_boundary,
            &self.control_boundary,
        )?;
        report.audit_coverage.validate()?;
        report.tool_envelope.validate()?;
        validate_execution_control_summary(&report.control_summary, report.record_count)?;
        require_usize(
            "execution_report.record_count",
            report.record_count,
            report.records.len(),
        )?;
        require_usize(
            "execution_report.record_count",
            report.record_count,
            handoff_report.records.len(),
        )?;
        let records_sha256 = json_sha256(&report.records).map_err(|error| {
            SyntaxBackendBoundaryError::new(
                "execution_report.records",
                format!("failed to hash external-validation execution records: {error}"),
            )
        })?;
        require_string(
            "execution_report.records_sha256",
            &report.records_sha256,
            &records_sha256,
        )?;
        validate_execution_records(&report.records, &handoff_report.records)?;
        require_vec_equal(
            "execution_report.allowed_claims",
            &report.allowed_claims,
            &handoff_report.allowed_claims,
        )?;
        require_all(
            "execution_report.blocked_claims",
            &report.blocked_claims,
            BOUNDARY_UNSUPPORTED_CLAIMS,
        )?;
        require_all(
            "execution_report.validation_commands",
            &report.validation_commands,
            REQUIRED_EXECUTION_VALIDATION_COMMANDS,
        )?;
        require_all(
            "execution_report.validation_commands",
            &report.validation_commands,
            &[
                EXECUTION_REPORT_PRODUCER_COMMAND,
                EXECUTION_REPORT_VALIDATOR_COMMAND,
            ],
        )?;
        require_string(
            "execution_report.completed_promotion_class",
            &report.completed_promotion_class,
            EXECUTION_REPORT_COMPLETED_PROMOTION_CLASS,
        )?;
        require_string(
            "execution_report.next_missing_stable_promotion_class",
            &report.next_missing_stable_promotion_class,
            EXECUTION_REPORT_NEXT_MISSING_STABLE_PROMOTION_CLASS,
        )
    }

    fn execution_control_summary(
        &self,
        record_count: usize,
        records: &[SyntaxSourceProgramHandoffRecord],
        tool_envelope: &SyntaxSourceProgramExternalValidationToolEnvelope,
    ) -> Result<
        Vec<SyntaxSourceProgramExternalValidationControlExecution>,
        SyntaxBackendBoundaryError,
    > {
        let parser_versions = records
            .iter()
            .map(|record| {
                format!(
                    "{}:{}",
                    record.parser_tool,
                    parser_version_with_package(record)
                )
            })
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        let observed_tool_versions = tool_envelope
            .observed_tools
            .iter()
            .map(|tool| {
                format!(
                    "{}:{}",
                    tool.tool_id,
                    tool.version.as_deref().unwrap_or(if tool.available {
                        "unknown"
                    } else {
                        "unavailable"
                    })
                )
            })
            .collect::<Vec<_>>();
        self.required_controls
            .iter()
            .map(|control| {
                let (
                    execution_status,
                    ran,
                    passed_records,
                    unavailable_records,
                    observed_versions,
                    supported_claims,
                ) = match control.control_id.as_str() {
                    "parser_metadata_reproduction" => (
                        "ran_from_handoff_parser_metadata_sidecar",
                        true,
                        record_count,
                        0,
                        parser_versions.clone(),
                        vec![
                            "parser_backed_source_corpus_identity_preserved".to_string(),
                            "parser_metadata_reproduction_sidecar_observed".to_string(),
                        ],
                    ),
                    "renderer_formatter_sidecar" => (
                        "ran_from_handoff_renderer_formatter_sidecar_policy",
                        true,
                        record_count,
                        0,
                        vec![format!("tct:{}", tool_envelope.tct_version)],
                        vec![
                            "renderer_formatter_sidecar_policy_observed".to_string(),
                            "no_external_toolchain_in_generated_hot_path".to_string(),
                        ],
                    ),
                    "type_checker" => (
                        "unavailable_missing_source_program_typecheck_harness",
                        false,
                        0,
                        record_count,
                        observed_tool_versions.clone(),
                        Vec::new(),
                    ),
                    "functional_validator" => (
                        "unavailable_missing_source_program_functional_harness",
                        false,
                        0,
                        record_count,
                        observed_tool_versions.clone(),
                        Vec::new(),
                    ),
                    other => {
                        return Err(SyntaxBackendBoundaryError::new(
                            "required_controls.control_id",
                            format!("unexpected external validation control {other}"),
                        ));
                    }
                };
                Ok(SyntaxSourceProgramExternalValidationControlExecution {
                    control_id: control.control_id.clone(),
                    control_kind: control.control_kind.clone(),
                    required_status: control.status.clone(),
                    execution_status: execution_status.to_string(),
                    ran,
                    record_count,
                    passed_records,
                    unavailable_records,
                    observed_tool_versions: observed_versions,
                    supported_claims,
                    unsupported_claims: BOUNDARY_UNSUPPORTED_CLAIMS
                        .iter()
                        .map(|claim| (*claim).to_string())
                        .collect(),
                    stable_promotion_allowed: false,
                    generated_hot_path_allowed: false,
                    blocker: control.blocker.clone(),
                })
            })
            .collect()
    }

    fn execution_record(
        &self,
        record: &SyntaxSourceProgramHandoffRecord,
        tool_envelope: &SyntaxSourceProgramExternalValidationToolEnvelope,
    ) -> Result<SyntaxSourceProgramExternalValidationExecutionRecord, SyntaxBackendBoundaryError>
    {
        validate_handoff_record_controls(record)?;
        Ok(SyntaxSourceProgramExternalValidationExecutionRecord {
            target_id: record.target_id.clone(),
            grammar_id: record.grammar_id.clone(),
            source_path: record.source_path.clone(),
            source_sha256: record.source_sha256.clone(),
            metadata_path: record.metadata_path.clone(),
            metadata_sha256: record.metadata_sha256.clone(),
            parser_tool: record.parser_tool.clone(),
            parser_version: record.parser_version.clone(),
            parser_package: record.parser_package.clone(),
            parser_package_version: record.parser_package_version.clone(),
            structural_model_sha256: record.structural_model_sha256.clone(),
            token_layout_sha256: record.token_layout_sha256.clone(),
            rendered_source_sha256: record.rendered_source_sha256.clone(),
            generated_codec_metadata_sha256: record.generated_codec_metadata_sha256.clone(),
            generated_crate_identity_sha256: record.generated_crate_identity_sha256.clone(),
            generation_options_sha256: record.generation_options_sha256.clone(),
            route_metadata: record.route_metadata.clone(),
            validation_hook_status: record.validation_hook_status.clone(),
            sidecar_controls: record.sidecar_controls.clone(),
            control_results: self.record_control_results(record, tool_envelope)?,
        })
    }

    fn record_control_results(
        &self,
        record: &SyntaxSourceProgramHandoffRecord,
        tool_envelope: &SyntaxSourceProgramExternalValidationToolEnvelope,
    ) -> Result<
        Vec<SyntaxSourceProgramExternalValidationRecordControlResult>,
        SyntaxBackendBoundaryError,
    > {
        self.required_controls
            .iter()
            .map(|control| match control.control_id.as_str() {
                "parser_metadata_reproduction" => Ok(record_control_result(
                    control,
                    "ran_from_handoff_parser_metadata_sidecar",
                    true,
                    &record.parser_tool,
                    &parser_version_with_package(record),
                    vec![
                        "parser_backed_source_corpus_identity_preserved".to_string(),
                        "parser_metadata_reproduction_sidecar_observed".to_string(),
                    ],
                )),
                "renderer_formatter_sidecar" => Ok(record_control_result(
                    control,
                    "ran_from_handoff_renderer_formatter_sidecar_policy",
                    true,
                    "tct_renderer_metadata",
                    &tool_envelope.tct_version,
                    vec![
                        "renderer_formatter_sidecar_policy_observed".to_string(),
                        "no_external_toolchain_in_generated_hot_path".to_string(),
                    ],
                )),
                "type_checker" => Ok(record_control_result(
                    control,
                    "unavailable_missing_source_program_typecheck_harness",
                    false,
                    &record.parser_tool,
                    &parser_version_with_package(record),
                    Vec::new(),
                )),
                "functional_validator" => Ok(record_control_result(
                    control,
                    "unavailable_missing_source_program_functional_harness",
                    false,
                    "target_functional_validator",
                    tool_envelope.tct_version.as_str(),
                    Vec::new(),
                )),
                other => Err(SyntaxBackendBoundaryError::new(
                    "required_controls.control_id",
                    format!("unexpected external validation control {other}"),
                )),
            })
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SyntaxSourceProgramExternalValidationExecutionReport {
    pub artifact_kind: String,
    pub version: u32,
    pub tool: String,
    pub status: String,
    pub controls_fixture: SyntaxSourceProgramExternalValidationControlsFixtureReport,
    pub source_handoff_report: SyntaxSourceProgramExternalValidationSourceHandoffReport,
    pub control_boundary: SyntaxSourceProgramExternalValidationBoundary,
    pub audit_coverage: SyntaxSourceProgramExternalValidationAuditCoverage,
    pub tool_envelope: SyntaxSourceProgramExternalValidationToolEnvelope,
    pub control_summary: Vec<SyntaxSourceProgramExternalValidationControlExecution>,
    pub record_count: usize,
    pub records_sha256: String,
    pub records: Vec<SyntaxSourceProgramExternalValidationExecutionRecord>,
    pub allowed_claims: Vec<String>,
    pub blocked_claims: Vec<String>,
    pub validation_commands: Vec<String>,
    pub completed_promotion_class: String,
    pub next_missing_stable_promotion_class: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxSourceProgramExternalValidationControlsFixtureReport {
    pub fixture_path: String,
    pub fixture_sha256: String,
    pub artifact_kind: String,
    pub tool: String,
    pub status: String,
    pub completed_promotion_class: String,
    pub next_missing_stable_promotion_class: String,
}

impl SyntaxSourceProgramExternalValidationControlsFixtureReport {
    fn validate(
        &self,
        gate: &SyntaxSourceProgramExternalValidationControlsGate,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        if !self
            .fixture_path
            .ends_with(EXTERNAL_VALIDATION_FIXTURE_PATH)
        {
            return Err(SyntaxBackendBoundaryError::new(
                "execution_report.controls_fixture.fixture_path",
                "external-validation controls fixture path must identify the checked fixture",
            ));
        }
        require_sha256(
            "execution_report.controls_fixture.fixture_sha256",
            &self.fixture_sha256,
        )?;
        let fixture_sha256 = json_sha256(gate).map_err(|error| {
            SyntaxBackendBoundaryError::new(
                "execution_report.controls_fixture.fixture_sha256",
                format!("failed to hash external-validation controls fixture: {error}"),
            )
        })?;
        require_string(
            "execution_report.controls_fixture.fixture_sha256",
            &self.fixture_sha256,
            &fixture_sha256,
        )?;
        require_string(
            "execution_report.controls_fixture.artifact_kind",
            &self.artifact_kind,
            ARTIFACT_KIND,
        )?;
        require_string("execution_report.controls_fixture.tool", &self.tool, TOOL)?;
        require_string(
            "execution_report.controls_fixture.status",
            &self.status,
            STATUS,
        )?;
        require_string(
            "execution_report.controls_fixture.completed_promotion_class",
            &self.completed_promotion_class,
            COMPLETED_PROMOTION_CLASS,
        )?;
        require_string(
            "execution_report.controls_fixture.next_missing_stable_promotion_class",
            &self.next_missing_stable_promotion_class,
            NEXT_MISSING_STABLE_PROMOTION_CLASS,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxSourceProgramExternalValidationSourceHandoffReport {
    pub report_path: String,
    pub report_sha256: String,
    pub artifact_kind: String,
    pub tool: String,
    pub status: String,
    pub profile_id: String,
    pub stable_metadata_promotion: SyntaxSourceProgramStableMetadataPromotionInput,
    pub go_generated_codec_completion_handoff:
        SyntaxSourceProgramGoGeneratedCodecCompletionHandoffInput,
    pub csharp_generated_codec_completion_handoff:
        SyntaxSourceProgramCSharpGeneratedCodecCompletionHandoffInput,
    pub record_count: usize,
    pub records_sha256: String,
    pub completed_promotion_class: String,
    pub next_missing_stable_promotion_class: String,
}

impl SyntaxSourceProgramExternalValidationSourceHandoffReport {
    fn validate(
        &self,
        handoff_report: &SyntaxSourceProgramHandoffReport,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        require_non_empty(
            "execution_report.source_handoff_report.report_path",
            &self.report_path,
        )?;
        require_sha256(
            "execution_report.source_handoff_report.report_sha256",
            &self.report_sha256,
        )?;
        let report_sha256 = json_sha256(handoff_report).map_err(|error| {
            SyntaxBackendBoundaryError::new(
                "execution_report.source_handoff_report.report_sha256",
                format!("failed to hash source-program handoff report: {error}"),
            )
        })?;
        require_string(
            "execution_report.source_handoff_report.report_sha256",
            &self.report_sha256,
            &report_sha256,
        )?;
        require_string(
            "execution_report.source_handoff_report.artifact_kind",
            &self.artifact_kind,
            syntax_source_program_handoff::REPORT_ARTIFACT_KIND,
        )?;
        require_string(
            "execution_report.source_handoff_report.tool",
            &self.tool,
            syntax_source_program_handoff::TOOL,
        )?;
        require_string(
            "execution_report.source_handoff_report.status",
            &self.status,
            syntax_source_program_handoff::REPORT_STATUS,
        )?;
        require_string(
            "execution_report.source_handoff_report.profile_id",
            &self.profile_id,
            syntax_source_program_handoff::PROFILE_ID,
        )?;
        require_stable_metadata_promotion_equal(
            "execution_report.source_handoff_report.stable_metadata_promotion",
            &self.stable_metadata_promotion,
            &handoff_report.stable_metadata_promotion,
        )?;
        require_go_completion_handoff_equal(
            "execution_report.source_handoff_report.go_generated_codec_completion_handoff",
            &self.go_generated_codec_completion_handoff,
            &handoff_report.go_generated_codec_completion_handoff,
        )?;
        require_csharp_completion_handoff_equal(
            "execution_report.source_handoff_report.csharp_generated_codec_completion_handoff",
            &self.csharp_generated_codec_completion_handoff,
            &handoff_report.csharp_generated_codec_completion_handoff,
        )?;
        require_usize(
            "execution_report.source_handoff_report.record_count",
            self.record_count,
            handoff_report.record_count,
        )?;
        require_string(
            "execution_report.source_handoff_report.records_sha256",
            &self.records_sha256,
            &handoff_report.records_sha256,
        )?;
        require_string(
            "execution_report.source_handoff_report.completed_promotion_class",
            &self.completed_promotion_class,
            syntax_source_program_handoff::REPORT_PRODUCER_COMPLETED_PROMOTION_CLASS,
        )?;
        require_string(
            "execution_report.source_handoff_report.next_missing_stable_promotion_class",
            &self.next_missing_stable_promotion_class,
            COMPLETED_PROMOTION_CLASS,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxSourceProgramExternalValidationAuditCoverage {
    pub quality_audit_command: String,
    pub module_path: String,
    pub fixture_path: String,
    pub source_program_handoff_module_path: String,
    pub source_program_handoff_fixture_path: String,
    pub execution_report_path: String,
    pub generated_reports_checked_in: bool,
}

impl SyntaxSourceProgramExternalValidationAuditCoverage {
    fn validate(&self) -> Result<(), SyntaxBackendBoundaryError> {
        require_string(
            "execution_report.audit_coverage.quality_audit_command",
            &self.quality_audit_command,
            "python3 scripts/testing/test-quality-audit.py --fail-on-gaps",
        )?;
        require_string(
            "execution_report.audit_coverage.module_path",
            &self.module_path,
            EXTERNAL_VALIDATION_MODULE_PATH,
        )?;
        require_string(
            "execution_report.audit_coverage.fixture_path",
            &self.fixture_path,
            EXTERNAL_VALIDATION_FIXTURE_PATH,
        )?;
        require_string(
            "execution_report.audit_coverage.source_program_handoff_module_path",
            &self.source_program_handoff_module_path,
            SOURCE_PROGRAM_MODULE_PATH,
        )?;
        require_string(
            "execution_report.audit_coverage.source_program_handoff_fixture_path",
            &self.source_program_handoff_fixture_path,
            syntax_source_program_handoff::FIXTURE_PATH,
        )?;
        require_string(
            "execution_report.audit_coverage.execution_report_path",
            &self.execution_report_path,
            EXECUTION_REPORT_PATH,
        )?;
        require_bool(
            "execution_report.audit_coverage.generated_reports_checked_in",
            self.generated_reports_checked_in,
            false,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxSourceProgramExternalValidationToolEnvelope {
    pub tct_version: String,
    pub observed_tools: Vec<SyntaxSourceProgramExternalValidationToolObservation>,
}

impl SyntaxSourceProgramExternalValidationToolEnvelope {
    pub fn new(
        tct_version: impl Into<String>,
        observed_tools: Vec<SyntaxSourceProgramExternalValidationToolObservation>,
    ) -> Self {
        Self {
            tct_version: tct_version.into(),
            observed_tools,
        }
    }

    fn validate(&self) -> Result<(), SyntaxBackendBoundaryError> {
        require_non_empty(
            "execution_report.tool_envelope.tct_version",
            &self.tct_version,
        )?;
        if self.observed_tools.is_empty() {
            return Err(SyntaxBackendBoundaryError::new(
                "execution_report.tool_envelope.observed_tools",
                "expected at least one observed external tool",
            ));
        }
        let mut ids = BTreeSet::new();
        for tool in &self.observed_tools {
            tool.validate()?;
            if !ids.insert(tool.tool_id.as_str()) {
                return Err(SyntaxBackendBoundaryError::new(
                    "execution_report.tool_envelope.observed_tools.tool_id",
                    format!("duplicate observed tool {}", tool.tool_id),
                ));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxSourceProgramExternalValidationToolObservation {
    pub tool_id: String,
    pub command: String,
    pub available: bool,
    pub version: Option<String>,
    pub unavailable_reason: Option<String>,
}

impl SyntaxSourceProgramExternalValidationToolObservation {
    pub fn available(
        tool_id: impl Into<String>,
        command: impl Into<String>,
        version: impl Into<String>,
    ) -> Self {
        Self {
            tool_id: tool_id.into(),
            command: command.into(),
            available: true,
            version: Some(version.into()),
            unavailable_reason: None,
        }
    }

    pub fn unavailable(
        tool_id: impl Into<String>,
        command: impl Into<String>,
        unavailable_reason: impl Into<String>,
    ) -> Self {
        Self {
            tool_id: tool_id.into(),
            command: command.into(),
            available: false,
            version: None,
            unavailable_reason: Some(unavailable_reason.into()),
        }
    }

    fn validate(&self) -> Result<(), SyntaxBackendBoundaryError> {
        require_non_empty("execution_report.tool_envelope.tool_id", &self.tool_id)?;
        require_non_empty("execution_report.tool_envelope.command", &self.command)?;
        if self.available {
            require_non_empty(
                "execution_report.tool_envelope.version",
                self.version.as_deref().unwrap_or(""),
            )
        } else {
            require_non_empty(
                "execution_report.tool_envelope.unavailable_reason",
                self.unavailable_reason.as_deref().unwrap_or(""),
            )
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxSourceProgramExternalValidationControlExecution {
    pub control_id: String,
    pub control_kind: String,
    pub required_status: String,
    pub execution_status: String,
    pub ran: bool,
    pub record_count: usize,
    pub passed_records: usize,
    pub unavailable_records: usize,
    pub observed_tool_versions: Vec<String>,
    pub supported_claims: Vec<String>,
    pub unsupported_claims: Vec<String>,
    pub stable_promotion_allowed: bool,
    pub generated_hot_path_allowed: bool,
    pub blocker: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SyntaxSourceProgramExternalValidationExecutionRecord {
    pub target_id: String,
    pub grammar_id: String,
    pub source_path: String,
    pub source_sha256: String,
    pub metadata_path: String,
    pub metadata_sha256: String,
    pub parser_tool: String,
    pub parser_version: String,
    pub parser_package: Option<String>,
    pub parser_package_version: Option<String>,
    pub structural_model_sha256: String,
    pub token_layout_sha256: String,
    pub rendered_source_sha256: String,
    pub generated_codec_metadata_sha256: String,
    pub generated_crate_identity_sha256: String,
    pub generation_options_sha256: String,
    pub route_metadata: crate::syntax_source_corpus_evidence::SyntaxSourceCorpusRouteEvidence,
    pub validation_hook_status: Value,
    pub sidecar_controls: Value,
    pub control_results: Vec<SyntaxSourceProgramExternalValidationRecordControlResult>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxSourceProgramExternalValidationRecordControlResult {
    pub control_id: String,
    pub execution_status: String,
    pub ran: bool,
    pub tool: String,
    pub observed_tool_version: String,
    pub supported_claims: Vec<String>,
    pub unsupported_claims: Vec<String>,
    pub stable_promotion_allowed: bool,
    pub generated_hot_path_allowed: bool,
    pub blocker: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxSourceProgramExternalValidationSourceHandoff {
    pub artifact_kind: String,
    pub version: u32,
    pub fixture_path: String,
    pub module_path: String,
    pub report_artifact_kind: String,
    pub report_status: String,
    pub report_path: String,
    pub profile_id: String,
    pub stable_metadata_promotion: SyntaxSourceProgramStableMetadataPromotionInput,
    pub go_generated_codec_completion_handoff:
        SyntaxSourceProgramGoGeneratedCodecCompletionHandoffInput,
    pub csharp_generated_codec_completion_handoff:
        SyntaxSourceProgramCSharpGeneratedCodecCompletionHandoffInput,
    pub required_next_missing_class: String,
}

impl SyntaxSourceProgramExternalValidationSourceHandoff {
    fn validate(
        &self,
        handoff_gate: &SyntaxSourceProgramClaimHandoffGate,
        handoff_report: &SyntaxSourceProgramHandoffReport,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        require_string(
            "source_handoff.artifact_kind",
            &self.artifact_kind,
            syntax_source_program_handoff::ARTIFACT_KIND,
        )?;
        require_u32(
            "source_handoff.version",
            self.version,
            syntax_source_program_handoff::VERSION,
        )?;
        require_u32("source_handoff.version", self.version, handoff_gate.version)?;
        require_string(
            "source_handoff.fixture_path",
            &self.fixture_path,
            syntax_source_program_handoff::FIXTURE_PATH,
        )?;
        require_string(
            "source_handoff.module_path",
            &self.module_path,
            "src/syntax_source_program_handoff.rs",
        )?;
        require_string(
            "source_handoff.report_artifact_kind",
            &self.report_artifact_kind,
            syntax_source_program_handoff::REPORT_ARTIFACT_KIND,
        )?;
        require_string(
            "source_handoff.report_status",
            &self.report_status,
            syntax_source_program_handoff::REPORT_STATUS,
        )?;
        require_string(
            "source_handoff.report_path",
            &self.report_path,
            syntax_source_program_handoff::REPORT_PATH,
        )?;
        require_string(
            "source_handoff.profile_id",
            &self.profile_id,
            syntax_source_program_handoff::PROFILE_ID,
        )?;
        require_source_handoff_stable_metadata_source_inputs(
            &self.stable_metadata_promotion,
            &handoff_gate.stable_metadata_promotion,
        )?;
        require_stable_metadata_promotion_equal(
            "source_handoff.stable_metadata_promotion",
            &self.stable_metadata_promotion,
            &handoff_gate.stable_metadata_promotion,
        )?;
        require_stable_metadata_promotion_equal(
            "source_handoff.report_stable_metadata_promotion",
            &self.stable_metadata_promotion,
            &handoff_report.stable_metadata_promotion,
        )?;
        require_go_completion_handoff_equal(
            "source_handoff.go_generated_codec_completion_handoff",
            &self.go_generated_codec_completion_handoff,
            &handoff_gate.go_generated_codec_completion_handoff,
        )?;
        require_go_completion_handoff_equal(
            "source_handoff.report_go_generated_codec_completion_handoff",
            &self.go_generated_codec_completion_handoff,
            &handoff_report.go_generated_codec_completion_handoff,
        )?;
        require_csharp_completion_handoff_equal(
            "source_handoff.csharp_generated_codec_completion_handoff",
            &self.csharp_generated_codec_completion_handoff,
            &handoff_gate.csharp_generated_codec_completion_handoff,
        )?;
        require_csharp_completion_handoff_equal(
            "source_handoff.report_csharp_generated_codec_completion_handoff",
            &self.csharp_generated_codec_completion_handoff,
            &handoff_report.csharp_generated_codec_completion_handoff,
        )?;
        require_string(
            "source_handoff.required_next_missing_class",
            &self.required_next_missing_class,
            syntax_source_program_handoff::NEXT_MISSING_STABLE_PROMOTION_CLASS,
        )?;
        require_string(
            "handoff_report.artifact_kind",
            &handoff_report.artifact_kind,
            &self.report_artifact_kind,
        )?;
        require_string(
            "handoff_report.status",
            &handoff_report.status,
            &self.report_status,
        )?;
        require_string(
            "handoff_report.profile_id",
            &handoff_report.profile_id,
            &self.profile_id,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxSourceProgramExternalValidationBoundary {
    pub syntax_level_boundary: String,
    pub control_evidence_only: bool,
    pub generated_codec_validity_claim_only: bool,
    pub source_program_claim_allowed: bool,
    pub paper_claim_allowed: bool,
    pub arena_ir_proof_allowed: bool,
    pub external_sidecar_stable_promotion_allowed: bool,
    pub generated_hot_path_external_checks_allowed: bool,
    pub json_schema_backend_preserved: bool,
}

impl SyntaxSourceProgramExternalValidationBoundary {
    fn validate(
        &self,
        claim_boundary: &SyntaxSourceProgramClaimBoundary,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        require_string(
            "control_boundary.syntax_level_boundary",
            &self.syntax_level_boundary,
            SYNTAX_LEVEL_BOUNDARY,
        )?;
        require_string(
            "handoff_report.claim_boundary.syntax_level_boundary",
            &claim_boundary.syntax_level_boundary,
            &self.syntax_level_boundary,
        )?;
        require_bool(
            "control_boundary.control_evidence_only",
            self.control_evidence_only,
            true,
        )?;
        require_bool(
            "control_boundary.generated_codec_validity_claim_only",
            self.generated_codec_validity_claim_only,
            true,
        )?;
        require_bool(
            "handoff_report.claim_boundary.generated_codec_validity_claim_allowed",
            claim_boundary.generated_codec_validity_claim_allowed,
            true,
        )?;
        require_bool(
            "control_boundary.source_program_claim_allowed",
            self.source_program_claim_allowed,
            false,
        )?;
        require_bool(
            "handoff_report.claim_boundary.source_program_claim_allowed",
            claim_boundary.source_program_claim_allowed,
            false,
        )?;
        require_bool(
            "control_boundary.paper_claim_allowed",
            self.paper_claim_allowed,
            false,
        )?;
        require_bool(
            "handoff_report.claim_boundary.paper_claim_allowed",
            claim_boundary.paper_claim_allowed,
            false,
        )?;
        require_bool(
            "control_boundary.arena_ir_proof_allowed",
            self.arena_ir_proof_allowed,
            false,
        )?;
        require_bool(
            "handoff_report.claim_boundary.arena_ir_proof_used",
            claim_boundary.arena_ir_proof_used,
            false,
        )?;
        require_bool(
            "control_boundary.external_sidecar_stable_promotion_allowed",
            self.external_sidecar_stable_promotion_allowed,
            false,
        )?;
        require_bool(
            "handoff_report.claim_boundary.external_sidecars_promoted",
            claim_boundary.external_sidecars_promoted,
            false,
        )?;
        require_bool(
            "control_boundary.generated_hot_path_external_checks_allowed",
            self.generated_hot_path_external_checks_allowed,
            false,
        )?;
        require_bool(
            "control_boundary.json_schema_backend_preserved",
            self.json_schema_backend_preserved,
            true,
        )?;
        require_bool(
            "handoff_report.claim_boundary.json_schema_backend_preserved",
            claim_boundary.json_schema_backend_preserved,
            true,
        )?;
        require_bool(
            "handoff_report.claim_boundary.json_schema_backend_changed",
            claim_boundary.json_schema_backend_changed,
            false,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxSourceProgramExternalValidationControl {
    pub control_id: String,
    pub control_kind: String,
    pub status: String,
    pub evidence_fields: Vec<String>,
    pub blocking_claims: Vec<String>,
    pub stable_promotion_allowed: bool,
    pub generated_hot_path_allowed: bool,
    pub blocker: String,
}

impl SyntaxSourceProgramExternalValidationControl {
    fn validate(&self) -> Result<(), SyntaxBackendBoundaryError> {
        if !REQUIRED_CONTROL_IDS.contains(&self.control_id.as_str()) {
            return Err(SyntaxBackendBoundaryError::new(
                "required_controls.control_id",
                format!("unexpected external validation control {}", self.control_id),
            ));
        }
        require_bool(
            "required_controls.stable_promotion_allowed",
            self.stable_promotion_allowed,
            false,
        )?;
        require_bool(
            "required_controls.generated_hot_path_allowed",
            self.generated_hot_path_allowed,
            false,
        )?;
        require_non_empty("required_controls.control_kind", &self.control_kind)?;
        match self.status.as_str() {
            "present_sidecar_only" => {
                if !REQUIRED_PRESENT_CONTROL_IDS.contains(&self.control_id.as_str()) {
                    return Err(SyntaxBackendBoundaryError::new(
                        "required_controls.status",
                        format!(
                            "{} cannot be marked present_sidecar_only at this gate",
                            self.control_id
                        ),
                    ));
                }
                require_non_empty_vec("required_controls.evidence_fields", &self.evidence_fields)?;
                if !self.blocker.is_empty() {
                    return Err(SyntaxBackendBoundaryError::new(
                        "required_controls.blocker",
                        format!(
                            "{} present sidecar controls must not carry a missing-control blocker",
                            self.control_id
                        ),
                    ));
                }
                Ok(())
            }
            "missing_downstream_control" => {
                if !REQUIRED_MISSING_CONTROL_IDS.contains(&self.control_id.as_str()) {
                    return Err(SyntaxBackendBoundaryError::new(
                        "required_controls.status",
                        format!(
                            "{} cannot be marked missing_downstream_control at this gate",
                            self.control_id
                        ),
                    ));
                }
                require_non_empty("required_controls.blocker", &self.blocker)?;
                require_all(
                    "required_controls.blocking_claims",
                    &self.blocking_claims,
                    &[
                        "whole_source_language_validity",
                        "cross_repo_paper_claim",
                        "functional_correctness_claim",
                    ],
                )
            }
            other => Err(SyntaxBackendBoundaryError::new(
                "required_controls.status",
                format!("unexpected external validation control status {other}"),
            )),
        }
    }
}

fn validate_controls(
    controls: &[SyntaxSourceProgramExternalValidationControl],
) -> Result<(), SyntaxBackendBoundaryError> {
    if controls.len() != REQUIRED_CONTROL_IDS.len() {
        return Err(SyntaxBackendBoundaryError::new(
            "required_controls",
            format!(
                "expected {} controls, got {}",
                REQUIRED_CONTROL_IDS.len(),
                controls.len()
            ),
        ));
    }
    let mut ids = BTreeSet::new();
    for control in controls {
        control.validate()?;
        if !ids.insert(control.control_id.as_str()) {
            return Err(SyntaxBackendBoundaryError::new(
                "required_controls.control_id",
                format!(
                    "duplicate external validation control {}",
                    control.control_id
                ),
            ));
        }
    }
    for required in REQUIRED_CONTROL_IDS {
        if !ids.contains(required) {
            return Err(SyntaxBackendBoundaryError::new(
                "required_controls.control_id",
                format!("missing external validation control {required}"),
            ));
        }
    }
    Ok(())
}

fn validate_handoff_records(
    records: &[SyntaxSourceProgramHandoffRecord],
) -> Result<(), SyntaxBackendBoundaryError> {
    if records.is_empty() {
        return Err(SyntaxBackendBoundaryError::new(
            "handoff_report.records",
            "expected at least one source-program handoff record",
        ));
    }
    let mut targets = BTreeSet::new();
    for record in records {
        validate_handoff_record_controls(record)?;
        targets.insert(record.target_id.as_str());
    }
    for target in REQUIRED_TARGETS {
        if !targets.contains(target) {
            return Err(SyntaxBackendBoundaryError::new(
                "handoff_report.records.target_id",
                format!("missing external validation control target {target}"),
            ));
        }
    }
    Ok(())
}

fn validate_handoff_record_controls(
    record: &SyntaxSourceProgramHandoffRecord,
) -> Result<(), SyntaxBackendBoundaryError> {
    require_non_empty("record.parser_tool", &record.parser_tool)?;
    require_non_empty("record.parser_version", &record.parser_version)?;
    require_sha256("record.source_sha256", &record.source_sha256)?;
    require_sha256("record.metadata_sha256", &record.metadata_sha256)?;
    require_sha256(
        "record.structural_model_sha256",
        &record.structural_model_sha256,
    )?;
    require_sha256("record.token_layout_sha256", &record.token_layout_sha256)?;

    let source_check = json_field(
        "record.validation_hook_status",
        &record.validation_hook_status,
        "source_reparse_or_equivalent_check",
    )?;
    require_json_bool(
        "record.validation_hook_status.source_reparse_or_equivalent_check.required",
        json_field(
            "record.validation_hook_status.source_reparse_or_equivalent_check",
            source_check,
            "required",
        )?,
        true,
    )?;
    require_json_string(
        "record.validation_hook_status.source_reparse_or_equivalent_check.check_kind",
        json_field(
            "record.validation_hook_status.source_reparse_or_equivalent_check",
            source_check,
            "check_kind",
        )?,
        "parser_metadata_reproduction",
    )?;
    require_json_non_empty_string(
        "record.validation_hook_status.source_reparse_or_equivalent_check.sidecar_evidence",
        json_field(
            "record.validation_hook_status.source_reparse_or_equivalent_check",
            source_check,
            "sidecar_evidence",
        )?,
    )?;

    let renderer_status = json_field(
        "record.validation_hook_status",
        &record.validation_hook_status,
        "renderer_formatter_status",
    )?;
    let renderer_contract = json_field(
        "record.validation_hook_status.renderer_formatter_status",
        renderer_status,
        "contract",
    )?;
    require_json_bool(
        "record.validation_hook_status.renderer_formatter_status.contract.formatter_status_sidecar_only",
        json_field(
            "record.validation_hook_status.renderer_formatter_status.contract",
            renderer_contract,
            "formatter_status_sidecar_only",
        )?,
        true,
    )?;
    require_json_bool(
        "record.validation_hook_status.renderer_formatter_status.contract.external_formatter_stable_gate_promoted",
        json_field(
            "record.validation_hook_status.renderer_formatter_status.contract",
            renderer_contract,
            "external_formatter_stable_gate_promoted",
        )?,
        false,
    )?;
    require_json_bool(
        "record.validation_hook_status.renderer_formatter_status.contract.generated_hot_path_external_checks_allowed",
        json_field(
            "record.validation_hook_status.renderer_formatter_status.contract",
            renderer_contract,
            "generated_hot_path_external_checks_allowed",
        )?,
        false,
    )?;
    require_json_array_contains(
        "record.validation_hook_status.renderer_formatter_status.contract.required_external_checks",
        json_field(
            "record.validation_hook_status.renderer_formatter_status.contract",
            renderer_contract,
            "required_external_checks",
        )?,
        &[
            "renderer_metadata",
            "formatter_sidecar_status",
            "no_external_toolchain_in_generated_hot_path",
        ],
    )?;

    let external_policy = json_field(
        "record.validation_hook_status",
        &record.validation_hook_status,
        "external_check_policy",
    )?;
    let policy = json_field(
        "record.validation_hook_status.external_check_policy",
        external_policy,
        "policy",
    )?;
    require_json_bool(
        "record.validation_hook_status.external_check_policy.policy.stable_gate_promoted",
        json_field(
            "record.validation_hook_status.external_check_policy.policy",
            policy,
            "stable_gate_promoted",
        )?,
        false,
    )?;
    require_json_bool(
        "record.validation_hook_status.external_check_policy.policy.sidecar_only_until_explicit_promotion",
        json_field(
            "record.validation_hook_status.external_check_policy.policy",
            policy,
            "sidecar_only_until_explicit_promotion",
        )?,
        true,
    )?;
    require_json_bool(
        "record.validation_hook_status.external_check_policy.policy.disallowed_in_generated_hot_path",
        json_field(
            "record.validation_hook_status.external_check_policy.policy",
            policy,
            "disallowed_in_generated_hot_path",
        )?,
        true,
    )?;
    require_json_array_contains(
        "record.validation_hook_status.external_check_policy.policy.checks",
        json_field(
            "record.validation_hook_status.external_check_policy.policy",
            policy,
            "checks",
        )?,
        REQUIRED_EXTERNAL_CHECKS,
    )?;

    require_json_bool(
        "record.sidecar_controls.arena_ir_proof_used",
        json_field(
            "record.sidecar_controls",
            &record.sidecar_controls,
            "arena_ir_proof_used",
        )?,
        false,
    )?;
    require_json_bool(
        "record.sidecar_controls.external_sidecars_promoted",
        json_field(
            "record.sidecar_controls",
            &record.sidecar_controls,
            "external_sidecars_promoted",
        )?,
        false,
    )?;
    require_sidecar_status(
        "record.sidecar_controls.validation_hook_sidecar_status",
        json_field(
            "record.sidecar_controls",
            &record.sidecar_controls,
            "validation_hook_sidecar_status",
        )?,
    )?;
    require_sidecar_status(
        "record.sidecar_controls.input_diagnostics_contract_sidecar_status",
        json_field(
            "record.sidecar_controls",
            &record.sidecar_controls,
            "input_diagnostics_contract_sidecar_status",
        )?,
    )?;
    require_sidecar_status(
        "record.sidecar_controls.input_diagnostics_negative_fixture_sidecar_status",
        json_field(
            "record.sidecar_controls",
            &record.sidecar_controls,
            "input_diagnostics_negative_fixture_sidecar_status",
        )?,
    )?;
    require_replay_boundary(json_field(
        "record.sidecar_controls",
        &record.sidecar_controls,
        "generated_artifact_replay_compiler_boundary",
    )?)
}

fn require_sidecar_status(
    path: &'static str,
    value: &Value,
) -> Result<(), SyntaxBackendBoundaryError> {
    require_json_bool(path, json_field(path, value, "sidecar_only")?, true)?;
    require_json_bool(
        path,
        json_field(path, value, "stable_gate_promoted")?,
        false,
    )?;
    require_json_bool(
        path,
        json_field(path, value, "generated_hot_path_external_checks_allowed")?,
        false,
    )
}

fn require_replay_boundary(value: &Value) -> Result<(), SyntaxBackendBoundaryError> {
    require_json_bool(
        "record.sidecar_controls.generated_artifact_replay_compiler_boundary.core_compiler_path_changed",
        json_field(
            "record.sidecar_controls.generated_artifact_replay_compiler_boundary",
            value,
            "core_compiler_path_changed",
        )?,
        false,
    )?;
    require_json_bool(
        "record.sidecar_controls.generated_artifact_replay_compiler_boundary.generated_json_schema_output_changed",
        json_field(
            "record.sidecar_controls.generated_artifact_replay_compiler_boundary",
            value,
            "generated_json_schema_output_changed",
        )?,
        false,
    )?;
    require_json_bool(
        "record.sidecar_controls.generated_artifact_replay_compiler_boundary.preserves_json_schema_backend_contract",
        json_field(
            "record.sidecar_controls.generated_artifact_replay_compiler_boundary",
            value,
            "preserves_json_schema_backend_contract",
        )?,
        true,
    )?;
    require_json_bool(
        "record.sidecar_controls.generated_artifact_replay_compiler_boundary.production_grammar_compiler_path_changed",
        json_field(
            "record.sidecar_controls.generated_artifact_replay_compiler_boundary",
            value,
            "production_grammar_compiler_path_changed",
        )?,
        false,
    )?;
    require_json_bool(
        "record.sidecar_controls.generated_artifact_replay_compiler_boundary.sidecar_only",
        json_field(
            "record.sidecar_controls.generated_artifact_replay_compiler_boundary",
            value,
            "sidecar_only",
        )?,
        true,
    )?;
    require_json_bool(
        "record.sidecar_controls.generated_artifact_replay_compiler_boundary.tct_compiler_route_allowed",
        json_field(
            "record.sidecar_controls.generated_artifact_replay_compiler_boundary",
            value,
            "tct_compiler_route_allowed",
        )?,
        false,
    )
}

fn merged_blocked_claims(left: &[String], right: &[String]) -> Vec<String> {
    left.iter()
        .chain(right.iter())
        .cloned()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn parser_version_with_package(record: &SyntaxSourceProgramHandoffRecord) -> String {
    match (&record.parser_package, &record.parser_package_version) {
        (Some(package), Some(package_version)) => {
            format!("{} ({package} {package_version})", record.parser_version)
        }
        _ => record.parser_version.clone(),
    }
}

fn record_control_result(
    control: &SyntaxSourceProgramExternalValidationControl,
    execution_status: &str,
    ran: bool,
    tool: &str,
    observed_tool_version: &str,
    supported_claims: Vec<String>,
) -> SyntaxSourceProgramExternalValidationRecordControlResult {
    SyntaxSourceProgramExternalValidationRecordControlResult {
        control_id: control.control_id.clone(),
        execution_status: execution_status.to_string(),
        ran,
        tool: tool.to_string(),
        observed_tool_version: observed_tool_version.to_string(),
        supported_claims,
        unsupported_claims: BOUNDARY_UNSUPPORTED_CLAIMS
            .iter()
            .map(|claim| (*claim).to_string())
            .collect(),
        stable_promotion_allowed: false,
        generated_hot_path_allowed: false,
        blocker: control.blocker.clone(),
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
            "stable metadata promotion input does not match the source-program handoff",
        ))
    }
}

fn require_source_handoff_stable_metadata_source_inputs(
    actual: &SyntaxSourceProgramStableMetadataPromotionInput,
    expected: &SyntaxSourceProgramStableMetadataPromotionInput,
) -> Result<(), SyntaxBackendBoundaryError> {
    require_usize(
        "source_handoff.stable_metadata_promotion.source_completion_handoff_supported_record_count",
        actual.source_completion_handoff_supported_record_count,
        expected.source_completion_handoff_supported_record_count,
    )?;
    require_usize(
        "source_handoff.stable_metadata_promotion.source_completion_handoff_blocked_record_count",
        actual.source_completion_handoff_blocked_record_count,
        expected.source_completion_handoff_blocked_record_count,
    )?;
    require_vec_equal(
        "source_handoff.stable_metadata_promotion.source_completion_handoff_supported_manifest_paths",
        &actual.source_completion_handoff_supported_manifest_paths,
        &expected.source_completion_handoff_supported_manifest_paths,
    )?;
    require_vec_equal(
        "source_handoff.stable_metadata_promotion.source_completion_handoff_generated_codec_routes",
        &actual.source_completion_handoff_generated_codec_routes,
        &expected.source_completion_handoff_generated_codec_routes,
    )?;
    require_vec_equal(
        "source_handoff.stable_metadata_promotion.source_completion_handoff_generated_codec_markers",
        &actual.source_completion_handoff_generated_codec_markers,
        &expected.source_completion_handoff_generated_codec_markers,
    )?;
    require_string(
        "source_handoff.stable_metadata_promotion.source_completion_handoff_source_program_typecheck_execution_report_path",
        &actual.source_completion_handoff_source_program_typecheck_execution_report_path,
        &expected.source_completion_handoff_source_program_typecheck_execution_report_path,
    )?;
    require_string(
        "source_handoff.stable_metadata_promotion.source_completion_handoff_source_program_typecheck_feasibility_report_path",
        &actual.source_completion_handoff_source_program_typecheck_feasibility_report_path,
        &expected.source_completion_handoff_source_program_typecheck_feasibility_report_path,
    )?;
    require_string(
        "source_handoff.stable_metadata_promotion.source_completion_handoff_source_program_typecheck_execution_completed_promotion_class",
        &actual
            .source_completion_handoff_source_program_typecheck_execution_completed_promotion_class,
        &expected
            .source_completion_handoff_source_program_typecheck_execution_completed_promotion_class,
    )?;
    require_string(
        "source_handoff.stable_metadata_promotion.source_completion_handoff_source_program_typecheck_execution_next_missing_stable_promotion_class",
        &actual
            .source_completion_handoff_source_program_typecheck_execution_next_missing_stable_promotion_class,
        &expected
            .source_completion_handoff_source_program_typecheck_execution_next_missing_stable_promotion_class,
    )?;
    require_vec_equal(
        "source_handoff.stable_metadata_promotion.source_completion_handoff_source_program_preserved_typecheck_stable_report_fields",
        &actual.source_completion_handoff_source_program_preserved_typecheck_stable_report_fields,
        &expected.source_completion_handoff_source_program_preserved_typecheck_stable_report_fields,
    )?;
    require_vec_equal(
        "source_handoff.stable_metadata_promotion.source_completion_handoff_source_program_preserved_typecheck_report_contract_stable_report_fields",
        &actual
            .source_completion_handoff_source_program_preserved_typecheck_report_contract_stable_report_fields,
        &expected
            .source_completion_handoff_source_program_preserved_typecheck_report_contract_stable_report_fields,
    )?;
    require_vec_equal(
        "source_handoff.stable_metadata_promotion.source_completion_handoff_source_program_preserved_typecheck_blocked_claims",
        &actual.source_completion_handoff_source_program_preserved_typecheck_blocked_claims,
        &expected.source_completion_handoff_source_program_preserved_typecheck_blocked_claims,
    )?;
    require_vec_equal(
        "source_handoff.stable_metadata_promotion.source_completion_handoff_source_program_preserved_typecheck_no_stable_claims",
        &actual.source_completion_handoff_source_program_preserved_typecheck_no_stable_claims,
        &expected.source_completion_handoff_source_program_preserved_typecheck_no_stable_claims,
    )?;
    require_usize(
        "source_handoff.stable_metadata_promotion.source_completion_handoff_source_program_execution_pass_envelope_required_record_count",
        actual
            .source_completion_handoff_source_program_execution_pass_envelope_required_record_count,
        expected
            .source_completion_handoff_source_program_execution_pass_envelope_required_record_count,
    )?;
    require_string(
        "source_handoff.stable_metadata_promotion.source_completion_handoff_source_program_execution_pass_envelope_required_record_status",
        &actual.source_completion_handoff_source_program_execution_pass_envelope_required_record_status,
        &expected.source_completion_handoff_source_program_execution_pass_envelope_required_record_status,
    )?;
    require_bool(
        "source_handoff.stable_metadata_promotion.source_completion_handoff_source_program_execution_pass_envelope_required_records_all_passed",
        actual.source_completion_handoff_source_program_execution_pass_envelope_required_records_all_passed,
        expected
            .source_completion_handoff_source_program_execution_pass_envelope_required_records_all_passed,
    )?;
    require_vec_equal(
        "source_handoff.stable_metadata_promotion.source_completion_handoff_source_program_execution_pass_envelope_required_record_ids",
        &actual
            .source_completion_handoff_source_program_execution_pass_envelope_required_record_ids,
        &expected
            .source_completion_handoff_source_program_execution_pass_envelope_required_record_ids,
    )?;
    require_bool(
        "source_handoff.stable_metadata_promotion.source_completion_handoff_source_program_typechecker_results_sidecar_only",
        actual.source_completion_handoff_source_program_typechecker_results_sidecar_only,
        expected.source_completion_handoff_source_program_typechecker_results_sidecar_only,
    )?;
    require_bool(
        "source_handoff.stable_metadata_promotion.source_completion_handoff_source_program_type_correctness_claim_allowed",
        actual.source_completion_handoff_source_program_type_correctness_claim_allowed,
        expected.source_completion_handoff_source_program_type_correctness_claim_allowed,
    )?;
    require_bool(
        "source_handoff.stable_metadata_promotion.source_completion_handoff_source_program_typechecker_arena_ir_proof_used",
        actual.source_completion_handoff_source_program_typechecker_arena_ir_proof_used,
        expected.source_completion_handoff_source_program_typechecker_arena_ir_proof_used,
    )?;
    require_bool(
        "source_handoff.stable_metadata_promotion.source_completion_handoff_source_program_typechecker_json_schema_backend_unchanged",
        actual.source_completion_handoff_source_program_typechecker_json_schema_backend_unchanged,
        expected.source_completion_handoff_source_program_typechecker_json_schema_backend_unchanged,
    )?;
    require_string(
        "source_handoff.stable_metadata_promotion.source_completion_handoff_source_program_corpus_check_report_path",
        &actual.source_completion_handoff_source_program_corpus_check_report_path,
        &expected.source_completion_handoff_source_program_corpus_check_report_path,
    )?;
    require_usize(
        "source_handoff.stable_metadata_promotion.source_completion_handoff_source_program_corpus_eligible_record_count",
        actual.source_completion_handoff_source_program_corpus_eligible_record_count,
        expected.source_completion_handoff_source_program_corpus_eligible_record_count,
    )?;
    require_usize(
        "source_handoff.stable_metadata_promotion.source_completion_handoff_source_program_corpus_unsupported_record_count",
        actual.source_completion_handoff_source_program_corpus_unsupported_record_count,
        expected.source_completion_handoff_source_program_corpus_unsupported_record_count,
    )?;
    require_bool(
        "source_handoff.stable_metadata_promotion.source_completion_handoff_source_program_sidecar_validity_claim_allowed",
        actual.source_completion_handoff_source_program_sidecar_validity_claim_allowed,
        expected.source_completion_handoff_source_program_sidecar_validity_claim_allowed,
    )?;
    require_bool(
        "source_handoff.stable_metadata_promotion.source_completion_handoff_source_program_sidecar_arena_ir_proof_used",
        actual.source_completion_handoff_source_program_sidecar_arena_ir_proof_used,
        expected.source_completion_handoff_source_program_sidecar_arena_ir_proof_used,
    )?;
    require_bool(
        "source_handoff.stable_metadata_promotion.source_completion_handoff_source_program_sidecar_json_schema_backend_unchanged",
        actual.source_completion_handoff_source_program_sidecar_json_schema_backend_unchanged,
        expected.source_completion_handoff_source_program_sidecar_json_schema_backend_unchanged,
    )?;
    require_vec_equal(
        "source_handoff.stable_metadata_promotion.source_completion_handoff_source_program_no_stable_claims",
        &actual.source_completion_handoff_source_program_no_stable_claims,
        &expected.source_completion_handoff_source_program_no_stable_claims,
    )?;
    require_vec_equal(
        "source_handoff.stable_metadata_promotion.source_completion_handoff_generated_codec_report_paths",
        &actual.source_completion_handoff_generated_codec_report_paths,
        &expected.source_completion_handoff_generated_codec_report_paths,
    )?;
    require_vec_equal(
        "source_handoff.stable_metadata_promotion.source_completion_handoff_generated_codec_report_commands",
        &actual.source_completion_handoff_generated_codec_report_commands,
        &expected.source_completion_handoff_generated_codec_report_commands,
    )?;
    if actual.source_completion_handoff_generated_codec_round_trip_records
        != expected.source_completion_handoff_generated_codec_round_trip_records
    {
        return Err(SyntaxBackendBoundaryError::new(
            "source_handoff.stable_metadata_promotion.source_completion_handoff_generated_codec_round_trip_records",
            "generated-codec round-trip records do not match the source-program handoff",
        ));
    }
    require_vec_equal(
        "source_handoff.stable_metadata_promotion.source_completion_handoff_generated_codec_allowed_claims",
        &actual.source_completion_handoff_generated_codec_allowed_claims,
        &expected.source_completion_handoff_generated_codec_allowed_claims,
    )?;
    require_string(
        "source_handoff.stable_metadata_promotion.source_completion_handoff_generated_codec_whole_file_claim_scope",
        &actual.source_completion_handoff_generated_codec_whole_file_claim_scope,
        &expected.source_completion_handoff_generated_codec_whole_file_claim_scope,
    )?;
    require_vec_equal(
        "source_handoff.stable_metadata_promotion.source_completion_handoff_generated_codec_no_stable_claims",
        &actual.source_completion_handoff_generated_codec_no_stable_claims,
        &expected.source_completion_handoff_generated_codec_no_stable_claims,
    )
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
            "Go generated-codec completion handoff input does not match the source-program handoff",
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
            "C# generated-codec completion handoff input does not match the source-program handoff",
        ))
    }
}

fn validate_execution_control_summary(
    controls: &[SyntaxSourceProgramExternalValidationControlExecution],
    report_record_count: usize,
) -> Result<(), SyntaxBackendBoundaryError> {
    if controls.len() != REQUIRED_CONTROL_IDS.len() {
        return Err(SyntaxBackendBoundaryError::new(
            "execution_report.control_summary",
            format!(
                "expected {} controls, got {}",
                REQUIRED_CONTROL_IDS.len(),
                controls.len()
            ),
        ));
    }
    let mut ids = BTreeSet::new();
    for control in controls {
        require_non_empty(
            "execution_report.control_summary.control_kind",
            &control.control_kind,
        )?;
        require_non_empty(
            "execution_report.control_summary.required_status",
            &control.required_status,
        )?;
        require_non_empty(
            "execution_report.control_summary.execution_status",
            &control.execution_status,
        )?;
        require_usize(
            "execution_report.control_summary.record_count",
            control.record_count,
            report_record_count,
        )?;
        require_bool(
            "execution_report.control_summary.stable_promotion_allowed",
            control.stable_promotion_allowed,
            false,
        )?;
        require_bool(
            "execution_report.control_summary.generated_hot_path_allowed",
            control.generated_hot_path_allowed,
            false,
        )?;
        require_all(
            "execution_report.control_summary.unsupported_claims",
            &control.unsupported_claims,
            BOUNDARY_UNSUPPORTED_CLAIMS,
        )?;
        if !ids.insert(control.control_id.as_str()) {
            return Err(SyntaxBackendBoundaryError::new(
                "execution_report.control_summary.control_id",
                format!("duplicate control {}", control.control_id),
            ));
        }
        match control.control_id.as_str() {
            "parser_metadata_reproduction" | "renderer_formatter_sidecar" => {
                require_bool("execution_report.control_summary.ran", control.ran, true)?;
                require_usize(
                    "execution_report.control_summary.passed_records",
                    control.passed_records,
                    report_record_count,
                )?;
                require_usize(
                    "execution_report.control_summary.unavailable_records",
                    control.unavailable_records,
                    0,
                )?;
                require_non_empty_vec(
                    "execution_report.control_summary.observed_tool_versions",
                    &control.observed_tool_versions,
                )?;
                require_non_empty_vec(
                    "execution_report.control_summary.supported_claims",
                    &control.supported_claims,
                )?;
                if !control.blocker.is_empty() {
                    return Err(SyntaxBackendBoundaryError::new(
                        "execution_report.control_summary.blocker",
                        "present controls must not carry missing-control blockers",
                    ));
                }
            }
            "type_checker" | "functional_validator" => {
                require_bool("execution_report.control_summary.ran", control.ran, false)?;
                require_usize(
                    "execution_report.control_summary.passed_records",
                    control.passed_records,
                    0,
                )?;
                require_usize(
                    "execution_report.control_summary.unavailable_records",
                    control.unavailable_records,
                    report_record_count,
                )?;
                require_non_empty("execution_report.control_summary.blocker", &control.blocker)?;
            }
            other => {
                return Err(SyntaxBackendBoundaryError::new(
                    "execution_report.control_summary.control_id",
                    format!("unexpected execution control {other}"),
                ));
            }
        }
    }
    for required in REQUIRED_CONTROL_IDS {
        if !ids.contains(required) {
            return Err(SyntaxBackendBoundaryError::new(
                "execution_report.control_summary.control_id",
                format!("missing execution control {required}"),
            ));
        }
    }
    Ok(())
}

fn validate_execution_records(
    records: &[SyntaxSourceProgramExternalValidationExecutionRecord],
    handoff_records: &[SyntaxSourceProgramHandoffRecord],
) -> Result<(), SyntaxBackendBoundaryError> {
    require_usize(
        "execution_report.records.len",
        records.len(),
        handoff_records.len(),
    )?;
    for (record, expected) in records.iter().zip(handoff_records) {
        require_string(
            "execution_report.records.target_id",
            &record.target_id,
            &expected.target_id,
        )?;
        require_string(
            "execution_report.records.grammar_id",
            &record.grammar_id,
            &expected.grammar_id,
        )?;
        require_string(
            "execution_report.records.source_path",
            &record.source_path,
            &expected.source_path,
        )?;
        require_string(
            "execution_report.records.source_sha256",
            &record.source_sha256,
            &expected.source_sha256,
        )?;
        require_string(
            "execution_report.records.metadata_path",
            &record.metadata_path,
            &expected.metadata_path,
        )?;
        require_string(
            "execution_report.records.metadata_sha256",
            &record.metadata_sha256,
            &expected.metadata_sha256,
        )?;
        require_string(
            "execution_report.records.parser_tool",
            &record.parser_tool,
            &expected.parser_tool,
        )?;
        require_string(
            "execution_report.records.parser_version",
            &record.parser_version,
            &expected.parser_version,
        )?;
        require_option_equal(
            "execution_report.records.parser_package",
            &record.parser_package,
            &expected.parser_package,
        )?;
        require_option_equal(
            "execution_report.records.parser_package_version",
            &record.parser_package_version,
            &expected.parser_package_version,
        )?;
        require_string(
            "execution_report.records.structural_model_sha256",
            &record.structural_model_sha256,
            &expected.structural_model_sha256,
        )?;
        require_string(
            "execution_report.records.token_layout_sha256",
            &record.token_layout_sha256,
            &expected.token_layout_sha256,
        )?;
        require_string(
            "execution_report.records.rendered_source_sha256",
            &record.rendered_source_sha256,
            &expected.rendered_source_sha256,
        )?;
        require_string(
            "execution_report.records.generated_codec_metadata_sha256",
            &record.generated_codec_metadata_sha256,
            &expected.generated_codec_metadata_sha256,
        )?;
        require_string(
            "execution_report.records.generated_crate_identity_sha256",
            &record.generated_crate_identity_sha256,
            &expected.generated_crate_identity_sha256,
        )?;
        require_string(
            "execution_report.records.generation_options_sha256",
            &record.generation_options_sha256,
            &expected.generation_options_sha256,
        )?;
        if record.route_metadata != expected.route_metadata {
            return Err(SyntaxBackendBoundaryError::new(
                "execution_report.records.route_metadata",
                "execution report route metadata must match the source-program handoff record",
            ));
        }
        if record.validation_hook_status != expected.validation_hook_status {
            return Err(SyntaxBackendBoundaryError::new(
                "execution_report.records.validation_hook_status",
                "execution report validation-hook status must match the source-program handoff record",
            ));
        }
        if record.sidecar_controls != expected.sidecar_controls {
            return Err(SyntaxBackendBoundaryError::new(
                "execution_report.records.sidecar_controls",
                "execution report sidecar-control status must match the source-program handoff record",
            ));
        }
        validate_record_control_results(&record.control_results)?;
    }
    Ok(())
}

fn validate_record_control_results(
    results: &[SyntaxSourceProgramExternalValidationRecordControlResult],
) -> Result<(), SyntaxBackendBoundaryError> {
    if results.len() != REQUIRED_CONTROL_IDS.len() {
        return Err(SyntaxBackendBoundaryError::new(
            "execution_report.records.control_results",
            format!(
                "expected {} record control results, got {}",
                REQUIRED_CONTROL_IDS.len(),
                results.len()
            ),
        ));
    }
    let mut ids = BTreeSet::new();
    for result in results {
        if !ids.insert(result.control_id.as_str()) {
            return Err(SyntaxBackendBoundaryError::new(
                "execution_report.records.control_results.control_id",
                format!("duplicate record control result {}", result.control_id),
            ));
        }
        require_non_empty(
            "execution_report.records.control_results.execution_status",
            &result.execution_status,
        )?;
        require_non_empty(
            "execution_report.records.control_results.tool",
            &result.tool,
        )?;
        require_non_empty(
            "execution_report.records.control_results.observed_tool_version",
            &result.observed_tool_version,
        )?;
        require_bool(
            "execution_report.records.control_results.stable_promotion_allowed",
            result.stable_promotion_allowed,
            false,
        )?;
        require_bool(
            "execution_report.records.control_results.generated_hot_path_allowed",
            result.generated_hot_path_allowed,
            false,
        )?;
        require_all(
            "execution_report.records.control_results.unsupported_claims",
            &result.unsupported_claims,
            BOUNDARY_UNSUPPORTED_CLAIMS,
        )?;
        match result.control_id.as_str() {
            "parser_metadata_reproduction" | "renderer_formatter_sidecar" => {
                require_bool(
                    "execution_report.records.control_results.ran",
                    result.ran,
                    true,
                )?;
                require_non_empty_vec(
                    "execution_report.records.control_results.supported_claims",
                    &result.supported_claims,
                )?;
                if !result.blocker.is_empty() {
                    return Err(SyntaxBackendBoundaryError::new(
                        "execution_report.records.control_results.blocker",
                        "present record controls must not carry missing-control blockers",
                    ));
                }
            }
            "type_checker" | "functional_validator" => {
                require_bool(
                    "execution_report.records.control_results.ran",
                    result.ran,
                    false,
                )?;
                require_non_empty(
                    "execution_report.records.control_results.blocker",
                    &result.blocker,
                )?;
            }
            other => {
                return Err(SyntaxBackendBoundaryError::new(
                    "execution_report.records.control_results.control_id",
                    format!("unexpected record control result {other}"),
                ));
            }
        }
    }
    for required in REQUIRED_CONTROL_IDS {
        if !ids.contains(required) {
            return Err(SyntaxBackendBoundaryError::new(
                "execution_report.records.control_results.control_id",
                format!("missing record control result {required}"),
            ));
        }
    }
    Ok(())
}

fn json_field<'a>(
    path: &'static str,
    value: &'a Value,
    field: &str,
) -> Result<&'a Value, SyntaxBackendBoundaryError> {
    value
        .get(field)
        .ok_or_else(|| SyntaxBackendBoundaryError::new(path, format!("missing JSON field {field}")))
}

fn require_json_bool(
    field: &'static str,
    value: &Value,
    expected: bool,
) -> Result<(), SyntaxBackendBoundaryError> {
    match value.as_bool() {
        Some(actual) if actual == expected => Ok(()),
        Some(actual) => Err(SyntaxBackendBoundaryError::new(
            field,
            format!("expected {expected}, got {actual}"),
        )),
        None => Err(SyntaxBackendBoundaryError::new(
            field,
            "expected boolean JSON value",
        )),
    }
}

fn require_json_string(
    field: &'static str,
    value: &Value,
    expected: &str,
) -> Result<(), SyntaxBackendBoundaryError> {
    match value.as_str() {
        Some(actual) if actual == expected => Ok(()),
        Some(actual) => Err(SyntaxBackendBoundaryError::new(
            field,
            format!("expected {expected}, got {actual}"),
        )),
        None => Err(SyntaxBackendBoundaryError::new(
            field,
            "expected string JSON value",
        )),
    }
}

fn require_json_non_empty_string(
    field: &'static str,
    value: &Value,
) -> Result<(), SyntaxBackendBoundaryError> {
    match value.as_str() {
        Some(actual) if !actual.is_empty() => Ok(()),
        Some(_) => Err(SyntaxBackendBoundaryError::new(
            field,
            "expected non-empty string JSON value",
        )),
        None => Err(SyntaxBackendBoundaryError::new(
            field,
            "expected string JSON value",
        )),
    }
}

fn require_json_array_contains(
    field: &'static str,
    value: &Value,
    expected: &[&str],
) -> Result<(), SyntaxBackendBoundaryError> {
    let Some(values) = value.as_array() else {
        return Err(SyntaxBackendBoundaryError::new(
            field,
            "expected array JSON value",
        ));
    };
    let actual = values
        .iter()
        .filter_map(Value::as_str)
        .collect::<BTreeSet<_>>();
    let missing = expected
        .iter()
        .copied()
        .filter(|expected_value| !actual.contains(expected_value))
        .collect::<Vec<_>>();
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

fn require_vec_equal(
    field: &'static str,
    actual: &[String],
    expected: &[String],
) -> Result<(), SyntaxBackendBoundaryError> {
    if actual == expected {
        Ok(())
    } else {
        Err(SyntaxBackendBoundaryError::new(
            field,
            "execution report value does not match source-program handoff report",
        ))
    }
}

fn require_option_equal(
    field: &'static str,
    actual: &Option<String>,
    expected: &Option<String>,
) -> Result<(), SyntaxBackendBoundaryError> {
    if actual == expected {
        Ok(())
    } else {
        Err(SyntaxBackendBoundaryError::new(
            field,
            "execution report optional value does not match source-program handoff record",
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
            "execution report control boundary does not match checked external-validation gate",
        ))
    }
}

fn require_all(
    field: &'static str,
    actual: &[String],
    expected: &[&str],
) -> Result<(), SyntaxBackendBoundaryError> {
    let actual_set = actual.iter().map(String::as_str).collect::<BTreeSet<_>>();
    let missing = expected
        .iter()
        .copied()
        .filter(|expected_value| !actual_set.contains(expected_value))
        .collect::<Vec<_>>();
    if missing.is_empty() {
        Ok(())
    } else {
        Err(SyntaxBackendBoundaryError::new(
            field,
            format!("missing required values: {}", missing.join(", ")),
        ))
    }
}

fn require_non_empty(field: &'static str, actual: &str) -> Result<(), SyntaxBackendBoundaryError> {
    if actual.is_empty() {
        Err(SyntaxBackendBoundaryError::new(
            field,
            "expected non-empty value",
        ))
    } else {
        Ok(())
    }
}

fn require_non_empty_vec(
    field: &'static str,
    actual: &[String],
) -> Result<(), SyntaxBackendBoundaryError> {
    if actual.is_empty() {
        Err(SyntaxBackendBoundaryError::new(
            field,
            "expected at least one value",
        ))
    } else {
        Ok(())
    }
}

fn require_sha256(field: &'static str, actual: &str) -> Result<(), SyntaxBackendBoundaryError> {
    if actual.len() == 64 && actual.chars().all(|ch| ch.is_ascii_hexdigit()) {
        Ok(())
    } else {
        Err(SyntaxBackendBoundaryError::new(
            field,
            format!("expected 64-character sha256 hex, got {actual}"),
        ))
    }
}
