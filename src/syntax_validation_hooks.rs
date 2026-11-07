//! Sidecar-only syntax validation hook contract.
//!
//! This module validates the checked fixture that names syntax validation hook
//! surfaces. It keeps parser, renderer, formatter, compiler, and functional
//! validator checks as sidecar evidence until a later gate promotes one.

use crate::provenance::json_sha256;
use crate::syntax_artifact_envelope::{
    ARTIFACT_KIND as ARTIFACT_ENVELOPE_KIND, FIXTURE_PATH as ARTIFACT_ENVELOPE_FIXTURE_PATH,
    SyntaxGeneratedArtifactEnvelope, VERSION as ARTIFACT_ENVELOPE_VERSION,
};
use crate::syntax_backend_boundary::{
    API_VERSION, GATE_HELPER_PATH, SyntaxBackendBoundaryError, SyntaxBackendCompilerBoundary,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const TOOL: &str = "syntax_backend_validation_hook_contract_fixture";
pub const VERSION: u32 = 1;
pub const ARTIFACT_KIND: &str = "tct_syntax_backend_validation_hook_contract";
pub const FIXTURE_PATH: &str = "tests/fixtures/syntax_backend/validation-hook-contract-v1.json";
pub const SIDECAR_EVIDENCE_ARTIFACT_KIND: &str =
    "tct_syntax_backend_validation_hook_sidecar_evidence";
pub const SIDECAR_EVIDENCE_PATH: &str =
    "tests/fixtures/syntax_backend/validation-hook-sidecar-evidence-v1.json";
pub const COMPLETED_PROMOTION_CLASS: &str = "syntax backend validation hook contract fixture";
pub const COMPLETED_SIDECAR_EVIDENCE_PROMOTION_CLASS: &str =
    "syntax backend parser-backed validation hook sidecar evidence gate";
pub const NEXT_MISSING_STABLE_PROMOTION_CLASS: &str =
    "syntax backend JSON-backend parity routing gate";

const REQUIRED_TARGETS: &[&str] = &["toy", "csharp", "go"];
const REQUIRED_VALIDATION_SURFACES: &[&str] = &[
    "source_reparse_or_equivalent_check",
    "renderer_or_formatter_sidecar_status",
    "external_check_non_promotion_policy",
    "validation_hook_provenance",
];
const REQUIRED_EXTERNAL_CHECKS: &[&str] = &[
    "parser",
    "renderer",
    "formatter",
    "compiler",
    "functional_validator",
];
const REQUIRED_RENDERER_CHECKS: &[&str] = &[
    "renderer_metadata",
    "formatter_sidecar_status",
    "no_external_toolchain_in_generated_hot_path",
];
const REQUIRED_HOOK_IDS: &[&str] = &[
    "grammar_ir.required_fields",
    "toy_grammar_frontend.strict_decode",
    "grammar_structural_token_ranges",
];
const REQUIRED_HOOK_OWNER_MODULES: &[&str] = &[
    "src/grammar_ir.rs",
    "src/toy_grammar_frontend.rs",
    "src/grammar_ir_structural_model.rs",
];
const REQUIRED_PROVENANCE_FIELDS: &[&str] = &[
    "target_id",
    "source_or_metadata_sha256",
    "parser_or_grammar_identity",
    "renderer_metadata_id",
    "validation_hook_ids",
    "external_check_policy",
    "sidecar_boundary_sha256",
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxValidationHookContract {
    pub artifact_kind: String,
    pub version: u32,
    pub api_version: String,
    pub source_artifact_envelope: SyntaxValidationHookSourceArtifactEnvelope,
    pub target_ids: Vec<String>,
    pub validation_surfaces: Vec<String>,
    pub source_reparse_or_equivalent_checks: SyntaxValidationSourceChecks,
    pub renderer_formatter_status: SyntaxValidationRendererFormatterStatus,
    pub external_check_policy: SyntaxValidationExternalCheckPolicy,
    pub validation_hook_requirements: SyntaxValidationHookRequirements,
    pub provenance_fields: Vec<String>,
    pub compiler_boundary: SyntaxBackendCompilerBoundary,
    pub non_goals: Vec<String>,
    pub completed_promotion_class: String,
    pub next_missing_stable_promotion_class: String,
}

impl SyntaxValidationHookContract {
    pub fn validate_against_artifact_envelope(
        &self,
        envelope: &SyntaxGeneratedArtifactEnvelope,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        require_string("artifact_kind", &self.artifact_kind, ARTIFACT_KIND)?;
        require_u32("version", self.version, VERSION)?;
        require_string("api_version", &self.api_version, API_VERSION)?;
        self.source_artifact_envelope.validate(envelope)?;
        require_all("target_ids", &self.target_ids, REQUIRED_TARGETS)?;
        require_all(
            "target_ids",
            &self.target_ids,
            &envelope
                .artifact_metadata
                .target_ids
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>(),
        )?;
        require_all(
            "validation_surfaces",
            &self.validation_surfaces,
            REQUIRED_VALIDATION_SURFACES,
        )?;
        self.source_reparse_or_equivalent_checks.validate()?;
        self.renderer_formatter_status.validate()?;
        self.external_check_policy.validate()?;
        self.validation_hook_requirements.validate()?;
        require_all(
            "provenance_fields",
            &self.provenance_fields,
            REQUIRED_PROVENANCE_FIELDS,
        )?;
        require_sidecar_boundary(&self.compiler_boundary)?;
        require_all(
            "non_goals",
            &self.non_goals,
            &[
                "no TctCompiler route",
                "no external parser or formatter stable promotion",
                "no generated hot path external checks",
            ],
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
pub struct SyntaxValidationHookSidecarEvidence {
    pub artifact_kind: String,
    pub version: u32,
    pub api_version: String,
    pub source_validation_hook_contract: SyntaxValidationHookSidecarSourceContract,
    pub target_ids: Vec<String>,
    pub records: Vec<SyntaxValidationHookSidecarRecord>,
    pub sidecar_status: SyntaxValidationHookSidecarStatus,
    pub compiler_boundary: SyntaxBackendCompilerBoundary,
    pub completed_promotion_class: String,
    pub next_missing_stable_promotion_class: String,
}

impl SyntaxValidationHookSidecarEvidence {
    pub fn validate_against_contract(
        &self,
        contract: &SyntaxValidationHookContract,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        require_string(
            "artifact_kind",
            &self.artifact_kind,
            SIDECAR_EVIDENCE_ARTIFACT_KIND,
        )?;
        require_u32("version", self.version, VERSION)?;
        require_string("api_version", &self.api_version, API_VERSION)?;
        self.source_validation_hook_contract.validate(contract)?;
        require_all("target_ids", &self.target_ids, REQUIRED_TARGETS)?;
        require_all(
            "target_ids",
            &self.target_ids,
            &contract
                .target_ids
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>(),
        )?;

        if self.records.is_empty() {
            return Err(SyntaxBackendBoundaryError::new(
                "records",
                "expected at least one validation hook sidecar evidence record",
            ));
        }

        let allowed_targets = self
            .target_ids
            .iter()
            .map(String::as_str)
            .collect::<BTreeSet<_>>();
        let mut record_targets = BTreeSet::new();
        let validation_hook_contract_sha256 = json_sha256(contract).map_err(|error| {
            SyntaxBackendBoundaryError::new(
                "records.validation_hook_contract_sha256",
                format!("failed to hash validation hook contract: {error}"),
            )
        })?;
        let external_check_policy_sha256 =
            json_sha256(&contract.external_check_policy).map_err(|error| {
                SyntaxBackendBoundaryError::new(
                    "records.external_check_policy_sha256",
                    format!("failed to hash external check policy: {error}"),
                )
            })?;
        for record in &self.records {
            record.validate(
                contract,
                &allowed_targets,
                &validation_hook_contract_sha256,
                &external_check_policy_sha256,
            )?;
            record_targets.insert(record.target_id.as_str());
        }
        for target in REQUIRED_TARGETS {
            if !record_targets.contains(target) {
                return Err(SyntaxBackendBoundaryError::new(
                    "records.target_id",
                    format!("missing validation hook sidecar evidence target {target}"),
                ));
            }
        }

        self.sidecar_status.validate()?;
        require_sidecar_boundary(&self.compiler_boundary)?;
        require_string(
            "completed_promotion_class",
            &self.completed_promotion_class,
            COMPLETED_SIDECAR_EVIDENCE_PROMOTION_CLASS,
        )?;
        require_string(
            "next_missing_stable_promotion_class",
            &self.next_missing_stable_promotion_class,
            NEXT_MISSING_STABLE_PROMOTION_CLASS,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxValidationHookSidecarSourceContract {
    pub artifact_kind: String,
    pub version: u32,
    pub fixture_path: String,
    pub module_path: String,
    pub test_path: String,
    pub gate_helper: String,
}

impl SyntaxValidationHookSidecarSourceContract {
    fn validate(
        &self,
        contract: &SyntaxValidationHookContract,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        require_string(
            "source_validation_hook_contract.artifact_kind",
            &self.artifact_kind,
            ARTIFACT_KIND,
        )?;
        require_u32(
            "source_validation_hook_contract.version",
            self.version,
            VERSION,
        )?;
        require_u32(
            "source_validation_hook_contract.version",
            self.version,
            contract.version,
        )?;
        require_string(
            "source_validation_hook_contract.fixture_path",
            &self.fixture_path,
            FIXTURE_PATH,
        )?;
        require_string(
            "source_validation_hook_contract.module_path",
            &self.module_path,
            "src/syntax_validation_hooks.rs",
        )?;
        require_string(
            "source_validation_hook_contract.test_path",
            &self.test_path,
            "tests/syntax_backend_boundary_test.rs",
        )?;
        require_string(
            "source_validation_hook_contract.gate_helper",
            &self.gate_helper,
            GATE_HELPER_PATH,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxValidationHookSidecarRecord {
    pub target_id: String,
    pub source_or_metadata_sha256: String,
    pub parser_or_grammar_identity: SyntaxValidationParserOrGrammarIdentity,
    pub sidecar_command_identity: SyntaxValidationHookSidecarCommandIdentity,
    pub renderer_metadata_id: String,
    pub renderer_formatter_status: SyntaxValidationRendererFormatterStatus,
    pub validation_hook_ids: Vec<String>,
    pub validation_hook_ids_sha256: String,
    pub external_check_policy_sha256: String,
    pub validation_hook_contract_sha256: String,
    pub sidecar_boundary: SyntaxValidationHookSidecarBoundary,
    pub sidecar_boundary_sha256: String,
}

impl SyntaxValidationHookSidecarRecord {
    fn validate(
        &self,
        contract: &SyntaxValidationHookContract,
        allowed_targets: &BTreeSet<&str>,
        validation_hook_contract_sha256: &str,
        external_check_policy_sha256: &str,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        if !allowed_targets.contains(self.target_id.as_str()) {
            return Err(SyntaxBackendBoundaryError::new(
                "records.target_id",
                format!(
                    "unexpected validation hook sidecar target {}",
                    self.target_id
                ),
            ));
        }
        require_sha256(
            "records.source_or_metadata_sha256",
            &self.source_or_metadata_sha256,
        )?;
        self.parser_or_grammar_identity.validate()?;
        match self.target_id.as_str() {
            "toy" if self.parser_or_grammar_identity.kind != "grammar" => {
                return Err(SyntaxBackendBoundaryError::new(
                    "records.parser_or_grammar_identity.kind",
                    "toy sidecar evidence must record grammar identity",
                ));
            }
            "csharp" | "go" if self.parser_or_grammar_identity.kind != "parser" => {
                return Err(SyntaxBackendBoundaryError::new(
                    "records.parser_or_grammar_identity.kind",
                    format!(
                        "parser-backed target {} must record parser identity",
                        self.target_id
                    ),
                ));
            }
            _ => {}
        }

        let source_check = contract
            .source_reparse_or_equivalent_checks
            .checks
            .iter()
            .find(|check| check.target == self.target_id)
            .ok_or_else(|| {
                SyntaxBackendBoundaryError::new(
                    "records.sidecar_command_identity.target_id",
                    format!("missing validation source check for {}", self.target_id),
                )
            })?;
        self.sidecar_command_identity.validate(source_check)?;
        require_non_empty("records.renderer_metadata_id", &self.renderer_metadata_id)?;
        if self.renderer_formatter_status != contract.renderer_formatter_status {
            return Err(SyntaxBackendBoundaryError::new(
                "records.renderer_formatter_status",
                "renderer/formatter sidecar status must match validation hook contract",
            ));
        }
        self.renderer_formatter_status.validate()?;
        require_all(
            "records.validation_hook_ids",
            &self.validation_hook_ids,
            &contract
                .validation_hook_requirements
                .required_hook_ids
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>(),
        )?;
        let validation_hook_ids_sha256 =
            json_sha256(&self.validation_hook_ids).map_err(|error| {
                SyntaxBackendBoundaryError::new(
                    "records.validation_hook_ids_sha256",
                    format!("failed to hash validation hook ids: {error}"),
                )
            })?;
        require_string(
            "records.validation_hook_ids_sha256",
            &self.validation_hook_ids_sha256,
            &validation_hook_ids_sha256,
        )?;
        require_string(
            "records.external_check_policy_sha256",
            &self.external_check_policy_sha256,
            external_check_policy_sha256,
        )?;
        require_string(
            "records.validation_hook_contract_sha256",
            &self.validation_hook_contract_sha256,
            validation_hook_contract_sha256,
        )?;
        self.sidecar_boundary.validate()?;
        let sidecar_boundary_sha256 = json_sha256(&self.sidecar_boundary).map_err(|error| {
            SyntaxBackendBoundaryError::new(
                "records.sidecar_boundary_sha256",
                format!("failed to hash sidecar boundary: {error}"),
            )
        })?;
        require_string(
            "records.sidecar_boundary_sha256",
            &self.sidecar_boundary_sha256,
            &sidecar_boundary_sha256,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxValidationHookSidecarCommandIdentity {
    pub check_kind: String,
    pub command: String,
    pub sidecar_evidence: String,
    pub live_toolchain_required: bool,
    pub self_test_supported: bool,
}

impl SyntaxValidationHookSidecarCommandIdentity {
    fn validate(
        &self,
        source_check: &SyntaxValidationTargetCheck,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        require_string(
            "records.sidecar_command_identity.check_kind",
            &self.check_kind,
            &source_check.check_kind,
        )?;
        require_string(
            "records.sidecar_command_identity.sidecar_evidence",
            &self.sidecar_evidence,
            &source_check.sidecar_evidence,
        )?;
        require_non_empty("records.sidecar_command_identity.command", &self.command)?;
        require_bool(
            "records.sidecar_command_identity.live_toolchain_required",
            self.live_toolchain_required,
            false,
        )?;
        require_bool(
            "records.sidecar_command_identity.self_test_supported",
            self.self_test_supported,
            true,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxValidationParserOrGrammarIdentity {
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub grammar_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub grammar_version: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub package: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub package_version: Option<String>,
}

impl SyntaxValidationParserOrGrammarIdentity {
    fn validate(&self) -> Result<(), SyntaxBackendBoundaryError> {
        match self.kind.as_str() {
            "grammar" => {
                require_optional_non_empty(
                    "records.parser_or_grammar_identity.grammar_id",
                    self.grammar_id.as_deref(),
                )?;
                match self.grammar_version {
                    Some(version) if version > 0 => Ok(()),
                    _ => Err(SyntaxBackendBoundaryError::new(
                        "records.parser_or_grammar_identity.grammar_version",
                        "grammar version must be nonzero",
                    )),
                }
            }
            "parser" => {
                require_optional_non_empty(
                    "records.parser_or_grammar_identity.tool",
                    self.tool.as_deref(),
                )?;
                require_optional_non_empty(
                    "records.parser_or_grammar_identity.version",
                    self.version.as_deref(),
                )?;
                require_optional_non_empty(
                    "records.parser_or_grammar_identity.package",
                    self.package.as_deref(),
                )?;
                require_optional_non_empty(
                    "records.parser_or_grammar_identity.package_version",
                    self.package_version.as_deref(),
                )
            }
            _ => Err(SyntaxBackendBoundaryError::new(
                "records.parser_or_grammar_identity.kind",
                format!("unsupported identity kind {}", self.kind),
            )),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxValidationHookSidecarBoundary {
    pub role: String,
    pub generated_json_schema_output_changed: bool,
    pub core_compiler_path_changed: bool,
    pub production_grammar_compiler_path_changed: bool,
}

impl SyntaxValidationHookSidecarBoundary {
    fn validate(&self) -> Result<(), SyntaxBackendBoundaryError> {
        require_string(
            "records.sidecar_boundary.role",
            &self.role,
            "sidecar_evidence_only",
        )?;
        require_bool(
            "records.sidecar_boundary.generated_json_schema_output_changed",
            self.generated_json_schema_output_changed,
            false,
        )?;
        require_bool(
            "records.sidecar_boundary.core_compiler_path_changed",
            self.core_compiler_path_changed,
            false,
        )?;
        require_bool(
            "records.sidecar_boundary.production_grammar_compiler_path_changed",
            self.production_grammar_compiler_path_changed,
            false,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxValidationHookSidecarStatus {
    pub sidecar_only: bool,
    pub stable_gate_promoted: bool,
    pub live_toolchain_required: bool,
    pub generated_hot_path_external_checks_allowed: bool,
}

impl SyntaxValidationHookSidecarStatus {
    fn validate(&self) -> Result<(), SyntaxBackendBoundaryError> {
        require_bool("sidecar_status.sidecar_only", self.sidecar_only, true)?;
        require_bool(
            "sidecar_status.stable_gate_promoted",
            self.stable_gate_promoted,
            false,
        )?;
        require_bool(
            "sidecar_status.live_toolchain_required",
            self.live_toolchain_required,
            false,
        )?;
        require_bool(
            "sidecar_status.generated_hot_path_external_checks_allowed",
            self.generated_hot_path_external_checks_allowed,
            false,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxValidationHookSourceArtifactEnvelope {
    pub artifact_kind: String,
    pub version: u32,
    pub fixture_path: String,
    pub module_path: String,
    pub test_path: String,
    pub gate_helper: String,
}

impl SyntaxValidationHookSourceArtifactEnvelope {
    fn validate(
        &self,
        envelope: &SyntaxGeneratedArtifactEnvelope,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        require_string(
            "source_artifact_envelope.artifact_kind",
            &self.artifact_kind,
            ARTIFACT_ENVELOPE_KIND,
        )?;
        require_u32(
            "source_artifact_envelope.version",
            self.version,
            ARTIFACT_ENVELOPE_VERSION,
        )?;
        require_u32(
            "source_artifact_envelope.version",
            self.version,
            envelope.version,
        )?;
        require_string(
            "source_artifact_envelope.fixture_path",
            &self.fixture_path,
            ARTIFACT_ENVELOPE_FIXTURE_PATH,
        )?;
        require_string(
            "source_artifact_envelope.module_path",
            &self.module_path,
            "src/syntax_artifact_envelope.rs",
        )?;
        require_string(
            "source_artifact_envelope.test_path",
            &self.test_path,
            "tests/syntax_backend_boundary_test.rs",
        )?;
        require_string(
            "source_artifact_envelope.gate_helper",
            &self.gate_helper,
            GATE_HELPER_PATH,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxValidationSourceChecks {
    pub required: bool,
    pub checks: Vec<SyntaxValidationTargetCheck>,
}

impl SyntaxValidationSourceChecks {
    fn validate(&self) -> Result<(), SyntaxBackendBoundaryError> {
        require_bool(
            "source_reparse_or_equivalent_checks.required",
            self.required,
            true,
        )?;
        let targets = self
            .checks
            .iter()
            .map(|check| check.target.as_str())
            .collect::<BTreeSet<_>>();
        for target in REQUIRED_TARGETS {
            if !targets.contains(target) {
                return Err(SyntaxBackendBoundaryError::new(
                    "source_reparse_or_equivalent_checks.checks",
                    format!("missing target {target}"),
                ));
            }
        }
        for check in &self.checks {
            if check.check_kind.is_empty() || check.sidecar_evidence.is_empty() {
                return Err(SyntaxBackendBoundaryError::new(
                    "source_reparse_or_equivalent_checks.checks",
                    "each source check must name its kind and sidecar evidence",
                ));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxValidationTargetCheck {
    pub target: String,
    pub check_kind: String,
    pub sidecar_evidence: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxValidationRendererFormatterStatus {
    pub renderer_metadata_required: bool,
    pub formatter_status_sidecar_only: bool,
    pub external_formatter_stable_gate_promoted: bool,
    pub generated_hot_path_external_checks_allowed: bool,
    pub required_external_checks: Vec<String>,
}

impl SyntaxValidationRendererFormatterStatus {
    fn validate(&self) -> Result<(), SyntaxBackendBoundaryError> {
        require_bool(
            "renderer_formatter_status.renderer_metadata_required",
            self.renderer_metadata_required,
            true,
        )?;
        require_bool(
            "renderer_formatter_status.formatter_status_sidecar_only",
            self.formatter_status_sidecar_only,
            true,
        )?;
        require_bool(
            "renderer_formatter_status.external_formatter_stable_gate_promoted",
            self.external_formatter_stable_gate_promoted,
            false,
        )?;
        require_bool(
            "renderer_formatter_status.generated_hot_path_external_checks_allowed",
            self.generated_hot_path_external_checks_allowed,
            false,
        )?;
        require_all(
            "renderer_formatter_status.required_external_checks",
            &self.required_external_checks,
            REQUIRED_RENDERER_CHECKS,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxValidationExternalCheckPolicy {
    pub stable_gate_promoted: bool,
    pub sidecar_only_until_explicit_promotion: bool,
    pub disallowed_in_generated_hot_path: bool,
    pub checks: Vec<String>,
}

impl SyntaxValidationExternalCheckPolicy {
    fn validate(&self) -> Result<(), SyntaxBackendBoundaryError> {
        require_bool(
            "external_check_policy.stable_gate_promoted",
            self.stable_gate_promoted,
            false,
        )?;
        require_bool(
            "external_check_policy.sidecar_only_until_explicit_promotion",
            self.sidecar_only_until_explicit_promotion,
            true,
        )?;
        require_bool(
            "external_check_policy.disallowed_in_generated_hot_path",
            self.disallowed_in_generated_hot_path,
            true,
        )?;
        require_all(
            "external_check_policy.checks",
            &self.checks,
            REQUIRED_EXTERNAL_CHECKS,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxValidationHookRequirements {
    pub source_module: String,
    pub runs_in_generated_hot_path: bool,
    pub required_hook_ids: Vec<String>,
    pub required_owner_modules: Vec<String>,
}

impl SyntaxValidationHookRequirements {
    fn validate(&self) -> Result<(), SyntaxBackendBoundaryError> {
        require_string(
            "validation_hook_requirements.source_module",
            &self.source_module,
            "src/grammar_ir_structural_model.rs",
        )?;
        require_bool(
            "validation_hook_requirements.runs_in_generated_hot_path",
            self.runs_in_generated_hot_path,
            false,
        )?;
        require_all(
            "validation_hook_requirements.required_hook_ids",
            &self.required_hook_ids,
            REQUIRED_HOOK_IDS,
        )?;
        require_all(
            "validation_hook_requirements.required_owner_modules",
            &self.required_owner_modules,
            REQUIRED_HOOK_OWNER_MODULES,
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

fn require_optional_non_empty(
    field: &'static str,
    actual: Option<&str>,
) -> Result<(), SyntaxBackendBoundaryError> {
    match actual {
        Some(value) => require_non_empty(field, value),
        None => Err(SyntaxBackendBoundaryError::new(
            field,
            "expected non-empty value",
        )),
    }
}

fn require_sha256(field: &'static str, actual: &str) -> Result<(), SyntaxBackendBoundaryError> {
    if actual.len() == 64 && actual.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        Ok(())
    } else {
        Err(SyntaxBackendBoundaryError::new(
            field,
            "expected 64-character lowercase hex SHA-256",
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
