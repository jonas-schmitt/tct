//! Sidecar-only syntax input validation diagnostics contract.
//!
//! This module validates the checked fixture that names diagnostics required
//! for checked syntax input. It keeps diagnostics as sidecar evidence until a
//! later generated-code handoff gate promotes their identity.

use crate::provenance::{json_sha256, sha256_hex};
use crate::syntax_backend_boundary::{
    API_VERSION, GATE_HELPER_PATH, SyntaxBackendBoundaryError, SyntaxBackendCompilerBoundary,
};
use crate::syntax_validation_hooks::{
    ARTIFACT_KIND as VALIDATION_HOOK_ARTIFACT_KIND, FIXTURE_PATH as VALIDATION_HOOK_FIXTURE_PATH,
    SyntaxValidationHookContract, VERSION as VALIDATION_HOOK_VERSION,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const TOOL: &str = "syntax_backend_input_validation_diagnostics_contract_fixture";
pub const VERSION: u32 = 1;
pub const ARTIFACT_KIND: &str = "tct_syntax_backend_input_validation_diagnostics_contract";
pub const FIXTURE_PATH: &str = "tests/fixtures/syntax_backend/input-validation-diagnostics-v1.json";
pub const NEGATIVE_FIXTURE_ARTIFACT_KIND: &str =
    "tct_syntax_backend_input_validation_negative_fixtures";
pub const NEGATIVE_FIXTURE_PATH: &str =
    "tests/fixtures/syntax_backend/input-validation-negative-fixtures-v1.json";
pub const COMPLETED_PROMOTION_CLASS: &str =
    "syntax backend input validation diagnostics contract fixture";
pub const COMPLETED_NEGATIVE_FIXTURE_PROMOTION_CLASS: &str =
    "syntax backend input validation negative fixture gate";
pub const NEXT_MISSING_STABLE_PROMOTION_CLASS: &str =
    "syntax backend JSON-backend parity routing gate";

const REQUIRED_TARGETS: &[&str] = &["toy", "csharp", "go"];
const REQUIRED_NEGATIVE_FIXTURE_TARGETS: &[&str] = &["toy", "csharp", "go"];
const REQUIRED_INPUT_FIELDS: &[&str] = &[
    "target_id",
    "source_or_metadata_sha256",
    "parser_or_grammar_identity",
    "diagnostics",
    "unsupported_syntax_errors",
    "sidecar_boundary",
];
const REQUIRED_DIAGNOSTIC_FIELDS: &[&str] = &[
    "severity",
    "code",
    "message",
    "rule_path",
    "source_path",
    "span",
];
const REQUIRED_SEVERITIES: &[&str] = &["error", "warning"];
const REQUIRED_UNSUPPORTED_SYNTAX_FIELDS: &[&str] = &[
    "target_id",
    "unsupported_kind",
    "rule_path",
    "message",
    "source_or_metadata_sha256",
    "parser_or_grammar_identity",
];
const REQUIRED_PROVENANCE_FIELDS: &[&str] = &[
    "target_id",
    "source_or_metadata_sha256",
    "parser_or_grammar_identity",
    "sidecar_boundary_sha256",
    "diagnostic_contract_sha256",
];
const REQUIRED_NEGATIVE_FIXTURE_KINDS: &[&str] = &["malformed_syntax", "unsupported_syntax"];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxInputDiagnosticsContract {
    pub artifact_kind: String,
    pub version: u32,
    pub api_version: String,
    pub source_validation_hook_contract: SyntaxInputDiagnosticsSourceValidationHookContract,
    pub target_ids: Vec<String>,
    pub input_surface: SyntaxInputDiagnosticsInputSurface,
    pub diagnostic_shape: SyntaxInputDiagnosticShape,
    pub unsupported_syntax_error_shape: SyntaxUnsupportedSyntaxErrorShape,
    pub provenance_fields: Vec<String>,
    pub sidecar_status: SyntaxInputDiagnosticsSidecarStatus,
    pub compiler_boundary: SyntaxBackendCompilerBoundary,
    pub non_goals: Vec<String>,
    pub completed_promotion_class: String,
    pub next_missing_stable_promotion_class: String,
}

impl SyntaxInputDiagnosticsContract {
    pub fn validate_against_validation_hook_contract(
        &self,
        contract: &SyntaxValidationHookContract,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        require_string("artifact_kind", &self.artifact_kind, ARTIFACT_KIND)?;
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
        self.input_surface.validate()?;
        self.diagnostic_shape.validate()?;
        self.unsupported_syntax_error_shape.validate()?;
        require_all(
            "provenance_fields",
            &self.provenance_fields,
            REQUIRED_PROVENANCE_FIELDS,
        )?;
        self.sidecar_status.validate()?;
        require_sidecar_boundary(&self.compiler_boundary)?;
        require_all(
            "non_goals",
            &self.non_goals,
            &[
                "no TctCompiler route",
                "no generated JSON Schema output change",
                "no external parser or formatter stable promotion",
                "no live toolchain requirement for fixture validation",
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
pub struct SyntaxInputDiagnosticsSourceValidationHookContract {
    pub artifact_kind: String,
    pub version: u32,
    pub fixture_path: String,
    pub module_path: String,
    pub test_path: String,
    pub gate_helper: String,
}

impl SyntaxInputDiagnosticsSourceValidationHookContract {
    fn validate(
        &self,
        contract: &SyntaxValidationHookContract,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        require_string(
            "source_validation_hook_contract.artifact_kind",
            &self.artifact_kind,
            VALIDATION_HOOK_ARTIFACT_KIND,
        )?;
        require_u32(
            "source_validation_hook_contract.version",
            self.version,
            VALIDATION_HOOK_VERSION,
        )?;
        require_u32(
            "source_validation_hook_contract.version",
            self.version,
            contract.version,
        )?;
        require_string(
            "source_validation_hook_contract.fixture_path",
            &self.fixture_path,
            VALIDATION_HOOK_FIXTURE_PATH,
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
pub struct SyntaxInputDiagnosticsInputSurface {
    pub surface: String,
    pub required_fields: Vec<String>,
    pub rule_path_required: bool,
    pub source_or_metadata_sha256_required: bool,
    pub parser_or_grammar_identity_required: bool,
}

impl SyntaxInputDiagnosticsInputSurface {
    fn validate(&self) -> Result<(), SyntaxBackendBoundaryError> {
        require_string(
            "input_surface.surface",
            &self.surface,
            "checked_syntax_input",
        )?;
        require_all(
            "input_surface.required_fields",
            &self.required_fields,
            REQUIRED_INPUT_FIELDS,
        )?;
        require_bool(
            "input_surface.rule_path_required",
            self.rule_path_required,
            true,
        )?;
        require_bool(
            "input_surface.source_or_metadata_sha256_required",
            self.source_or_metadata_sha256_required,
            true,
        )?;
        require_bool(
            "input_surface.parser_or_grammar_identity_required",
            self.parser_or_grammar_identity_required,
            true,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxInputDiagnosticShape {
    pub fields: Vec<String>,
    pub required_severities: Vec<String>,
    pub rule_path_format: String,
    pub span_policy: String,
}

impl SyntaxInputDiagnosticShape {
    fn validate(&self) -> Result<(), SyntaxBackendBoundaryError> {
        require_all(
            "diagnostic_shape.fields",
            &self.fields,
            REQUIRED_DIAGNOSTIC_FIELDS,
        )?;
        require_all(
            "diagnostic_shape.required_severities",
            &self.required_severities,
            REQUIRED_SEVERITIES,
        )?;
        require_string(
            "diagnostic_shape.rule_path_format",
            &self.rule_path_format,
            "grammar_rule_or_parser_node_path",
        )?;
        require_string(
            "diagnostic_shape.span_policy",
            &self.span_policy,
            "optional_when_source_offsets_unavailable",
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxUnsupportedSyntaxErrorShape {
    pub required: bool,
    pub fields: Vec<String>,
    pub requires_rule_path: bool,
    pub stable_gate_promoted: bool,
}

impl SyntaxUnsupportedSyntaxErrorShape {
    fn validate(&self) -> Result<(), SyntaxBackendBoundaryError> {
        require_bool(
            "unsupported_syntax_error_shape.required",
            self.required,
            true,
        )?;
        require_all(
            "unsupported_syntax_error_shape.fields",
            &self.fields,
            REQUIRED_UNSUPPORTED_SYNTAX_FIELDS,
        )?;
        require_bool(
            "unsupported_syntax_error_shape.requires_rule_path",
            self.requires_rule_path,
            true,
        )?;
        require_bool(
            "unsupported_syntax_error_shape.stable_gate_promoted",
            self.stable_gate_promoted,
            false,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxInputDiagnosticsSidecarStatus {
    pub sidecar_only: bool,
    pub stable_gate_promoted: bool,
    pub live_toolchain_required: bool,
    pub generated_hot_path_external_checks_allowed: bool,
}

impl SyntaxInputDiagnosticsSidecarStatus {
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
pub struct SyntaxInputDiagnosticsNegativeFixtureGate {
    pub artifact_kind: String,
    pub version: u32,
    pub api_version: String,
    pub source_input_diagnostics_contract: SyntaxInputDiagnosticsNegativeSourceContract,
    pub target_ids: Vec<String>,
    pub fixtures: Vec<SyntaxInputDiagnosticsNegativeFixture>,
    pub sidecar_status: SyntaxInputDiagnosticsSidecarStatus,
    pub compiler_boundary: SyntaxBackendCompilerBoundary,
    pub completed_promotion_class: String,
    pub next_missing_stable_promotion_class: String,
}

impl SyntaxInputDiagnosticsNegativeFixtureGate {
    pub fn validate_against_contract(
        &self,
        contract: &SyntaxInputDiagnosticsContract,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        require_string(
            "artifact_kind",
            &self.artifact_kind,
            NEGATIVE_FIXTURE_ARTIFACT_KIND,
        )?;
        require_u32("version", self.version, VERSION)?;
        require_string("api_version", &self.api_version, API_VERSION)?;
        self.source_input_diagnostics_contract.validate(contract)?;
        require_all(
            "target_ids",
            &self.target_ids,
            REQUIRED_NEGATIVE_FIXTURE_TARGETS,
        )?;

        let contract_targets = contract
            .target_ids
            .iter()
            .map(String::as_str)
            .collect::<BTreeSet<_>>();
        for target in &self.target_ids {
            if !contract_targets.contains(target.as_str()) {
                return Err(SyntaxBackendBoundaryError::new(
                    "target_ids",
                    format!("target {target} is not named by diagnostics contract"),
                ));
            }
        }

        if self.fixtures.is_empty() {
            return Err(SyntaxBackendBoundaryError::new(
                "fixtures",
                "expected at least one negative checked-input fixture",
            ));
        }

        let fixture_targets = self
            .target_ids
            .iter()
            .map(String::as_str)
            .collect::<BTreeSet<_>>();
        let diagnostic_contract_sha256 = json_sha256(contract).map_err(|error| {
            SyntaxBackendBoundaryError::new(
                "diagnostic_contract_sha256",
                format!("failed to hash diagnostics contract: {error}"),
            )
        })?;
        let mut fixture_kinds = BTreeSet::new();
        let mut fixture_target_kinds = BTreeSet::new();
        let mut severities = BTreeSet::new();
        for fixture in &self.fixtures {
            fixture.validate(
                contract,
                &fixture_targets,
                &diagnostic_contract_sha256,
                &mut severities,
            )?;
            fixture_kinds.insert(fixture.input_kind.as_str());
            fixture_target_kinds.insert((fixture.target_id.clone(), fixture.input_kind.clone()));
        }
        for kind in REQUIRED_NEGATIVE_FIXTURE_KINDS {
            if !fixture_kinds.contains(kind) {
                return Err(SyntaxBackendBoundaryError::new(
                    "fixtures.input_kind",
                    format!("missing negative fixture kind {kind}"),
                ));
            }
        }
        for target in &self.target_ids {
            for kind in REQUIRED_NEGATIVE_FIXTURE_KINDS {
                if !fixture_target_kinds.contains(&(target.clone(), (*kind).to_string())) {
                    return Err(SyntaxBackendBoundaryError::new(
                        "fixtures.target_id",
                        format!("missing {kind} negative fixture for target {target}"),
                    ));
                }
            }
        }
        for severity in &contract.diagnostic_shape.required_severities {
            if !severities.contains(severity.as_str()) {
                return Err(SyntaxBackendBoundaryError::new(
                    "fixtures.diagnostics.severity",
                    format!("missing diagnostic severity {severity}"),
                ));
            }
        }

        self.sidecar_status.validate()?;
        require_sidecar_boundary(&self.compiler_boundary)?;
        require_string(
            "completed_promotion_class",
            &self.completed_promotion_class,
            COMPLETED_NEGATIVE_FIXTURE_PROMOTION_CLASS,
        )?;
        require_string(
            "next_missing_stable_promotion_class",
            &self.next_missing_stable_promotion_class,
            NEXT_MISSING_STABLE_PROMOTION_CLASS,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxInputDiagnosticsNegativeSourceContract {
    pub artifact_kind: String,
    pub version: u32,
    pub fixture_path: String,
    pub module_path: String,
    pub test_path: String,
    pub gate_helper: String,
}

impl SyntaxInputDiagnosticsNegativeSourceContract {
    fn validate(
        &self,
        contract: &SyntaxInputDiagnosticsContract,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        require_string(
            "source_input_diagnostics_contract.artifact_kind",
            &self.artifact_kind,
            ARTIFACT_KIND,
        )?;
        require_u32(
            "source_input_diagnostics_contract.version",
            self.version,
            VERSION,
        )?;
        require_u32(
            "source_input_diagnostics_contract.version",
            self.version,
            contract.version,
        )?;
        require_string(
            "source_input_diagnostics_contract.fixture_path",
            &self.fixture_path,
            FIXTURE_PATH,
        )?;
        require_string(
            "source_input_diagnostics_contract.module_path",
            &self.module_path,
            "src/syntax_input_diagnostics.rs",
        )?;
        require_string(
            "source_input_diagnostics_contract.test_path",
            &self.test_path,
            "tests/syntax_backend_boundary_test.rs",
        )?;
        require_string(
            "source_input_diagnostics_contract.gate_helper",
            &self.gate_helper,
            GATE_HELPER_PATH,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxInputDiagnosticsNegativeFixture {
    pub id: String,
    pub target_id: String,
    pub input_kind: String,
    pub synthetic_input: SyntaxInputDiagnosticsSyntheticInput,
    pub parser_or_grammar_identity: SyntaxParserOrGrammarIdentity,
    pub diagnostics: Vec<SyntaxInputDiagnosticRecord>,
    pub unsupported_syntax_errors: Vec<SyntaxUnsupportedSyntaxErrorRecord>,
    pub sidecar_boundary: SyntaxInputDiagnosticsSidecarBoundary,
    pub sidecar_boundary_sha256: String,
    pub diagnostic_contract_sha256: String,
}

impl SyntaxInputDiagnosticsNegativeFixture {
    fn validate(
        &self,
        contract: &SyntaxInputDiagnosticsContract,
        allowed_targets: &BTreeSet<&str>,
        diagnostic_contract_sha256: &str,
        severities: &mut BTreeSet<String>,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        require_non_empty("fixtures.id", &self.id)?;
        if !allowed_targets.contains(self.target_id.as_str()) {
            return Err(SyntaxBackendBoundaryError::new(
                "fixtures.target_id",
                format!("unexpected negative fixture target {}", self.target_id),
            ));
        }
        if !REQUIRED_NEGATIVE_FIXTURE_KINDS
            .iter()
            .any(|kind| *kind == self.input_kind)
        {
            return Err(SyntaxBackendBoundaryError::new(
                "fixtures.input_kind",
                format!("unsupported negative fixture kind {}", self.input_kind),
            ));
        }
        self.synthetic_input.validate()?;
        self.parser_or_grammar_identity.validate()?;
        if matches!(self.target_id.as_str(), "csharp" | "go")
            && self.parser_or_grammar_identity.kind != "parser"
        {
            return Err(SyntaxBackendBoundaryError::new(
                "fixtures.parser_or_grammar_identity.kind",
                format!(
                    "parser-backed target {} must record parser identity",
                    self.target_id
                ),
            ));
        }
        self.sidecar_boundary.validate()?;
        let sidecar_boundary_sha256 = json_sha256(&self.sidecar_boundary).map_err(|error| {
            SyntaxBackendBoundaryError::new(
                "fixtures.sidecar_boundary_sha256",
                format!("failed to hash sidecar boundary: {error}"),
            )
        })?;
        require_string(
            "fixtures.sidecar_boundary_sha256",
            &self.sidecar_boundary_sha256,
            &sidecar_boundary_sha256,
        )?;
        require_string(
            "fixtures.diagnostic_contract_sha256",
            &self.diagnostic_contract_sha256,
            diagnostic_contract_sha256,
        )?;
        if self.diagnostics.is_empty() {
            return Err(SyntaxBackendBoundaryError::new(
                "fixtures.diagnostics",
                "expected at least one diagnostic record",
            ));
        }
        for diagnostic in &self.diagnostics {
            diagnostic.validate(
                contract,
                &self.synthetic_input,
                &self.parser_or_grammar_identity,
            )?;
            severities.insert(diagnostic.severity.clone());
        }
        if self.input_kind == "unsupported_syntax" && self.unsupported_syntax_errors.is_empty() {
            return Err(SyntaxBackendBoundaryError::new(
                "fixtures.unsupported_syntax_errors",
                "unsupported syntax fixture must include unsupported syntax errors",
            ));
        }
        for error in &self.unsupported_syntax_errors {
            error.validate(
                &self.target_id,
                &self.synthetic_input,
                &self.parser_or_grammar_identity,
            )?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxInputDiagnosticsSyntheticInput {
    pub source_path: String,
    pub text: String,
    pub source_or_metadata_sha256: String,
}

impl SyntaxInputDiagnosticsSyntheticInput {
    fn validate(&self) -> Result<(), SyntaxBackendBoundaryError> {
        require_non_empty("fixtures.synthetic_input.source_path", &self.source_path)?;
        require_non_empty("fixtures.synthetic_input.text", &self.text)?;
        require_string(
            "fixtures.synthetic_input.source_or_metadata_sha256",
            &self.source_or_metadata_sha256,
            &sha256_hex(self.text.as_bytes()),
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxParserOrGrammarIdentity {
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

impl SyntaxParserOrGrammarIdentity {
    fn validate(&self) -> Result<(), SyntaxBackendBoundaryError> {
        match self.kind.as_str() {
            "grammar" => {
                require_optional_non_empty(
                    "parser_or_grammar_identity.grammar_id",
                    self.grammar_id.as_deref(),
                )?;
                match self.grammar_version {
                    Some(version) if version > 0 => Ok(()),
                    _ => Err(SyntaxBackendBoundaryError::new(
                        "parser_or_grammar_identity.grammar_version",
                        "grammar version must be nonzero",
                    )),
                }
            }
            "parser" => {
                require_optional_non_empty(
                    "parser_or_grammar_identity.tool",
                    self.tool.as_deref(),
                )?;
                require_optional_non_empty(
                    "parser_or_grammar_identity.version",
                    self.version.as_deref(),
                )?;
                if let Some(package) = self.package.as_deref() {
                    require_non_empty("parser_or_grammar_identity.package", package)?;
                }
                if let Some(package_version) = self.package_version.as_deref() {
                    require_non_empty(
                        "parser_or_grammar_identity.package_version",
                        package_version,
                    )?;
                }
                Ok(())
            }
            _ => Err(SyntaxBackendBoundaryError::new(
                "parser_or_grammar_identity.kind",
                format!("unsupported identity kind {}", self.kind),
            )),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxInputDiagnosticRecord {
    pub severity: String,
    pub code: String,
    pub message: String,
    pub rule_path: Vec<String>,
    pub source_path: String,
    pub span: Option<SyntaxInputDiagnosticSpan>,
}

impl SyntaxInputDiagnosticRecord {
    fn validate(
        &self,
        contract: &SyntaxInputDiagnosticsContract,
        input: &SyntaxInputDiagnosticsSyntheticInput,
        _identity: &SyntaxParserOrGrammarIdentity,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        if !contract
            .diagnostic_shape
            .required_severities
            .iter()
            .any(|severity| severity == &self.severity)
        {
            return Err(SyntaxBackendBoundaryError::new(
                "fixtures.diagnostics.severity",
                format!("unsupported diagnostic severity {}", self.severity),
            ));
        }
        require_non_empty("fixtures.diagnostics.code", &self.code)?;
        require_non_empty("fixtures.diagnostics.message", &self.message)?;
        require_non_empty_list("fixtures.diagnostics.rule_path", &self.rule_path)?;
        require_string(
            "fixtures.diagnostics.source_path",
            &self.source_path,
            &input.source_path,
        )?;
        if let Some(span) = &self.span {
            span.validate()?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxInputDiagnosticSpan {
    pub start: u32,
    pub end: u32,
}

impl SyntaxInputDiagnosticSpan {
    fn validate(&self) -> Result<(), SyntaxBackendBoundaryError> {
        if self.start <= self.end {
            Ok(())
        } else {
            Err(SyntaxBackendBoundaryError::new(
                "fixtures.diagnostics.span",
                "span start must be at or before span end",
            ))
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxUnsupportedSyntaxErrorRecord {
    pub target_id: String,
    pub unsupported_kind: String,
    pub rule_path: Vec<String>,
    pub message: String,
    pub source_or_metadata_sha256: String,
    pub parser_or_grammar_identity: SyntaxParserOrGrammarIdentity,
}

impl SyntaxUnsupportedSyntaxErrorRecord {
    fn validate(
        &self,
        target_id: &str,
        input: &SyntaxInputDiagnosticsSyntheticInput,
        identity: &SyntaxParserOrGrammarIdentity,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        require_string(
            "fixtures.unsupported_syntax_errors.target_id",
            &self.target_id,
            target_id,
        )?;
        require_non_empty(
            "fixtures.unsupported_syntax_errors.unsupported_kind",
            &self.unsupported_kind,
        )?;
        require_non_empty_list(
            "fixtures.unsupported_syntax_errors.rule_path",
            &self.rule_path,
        )?;
        require_non_empty("fixtures.unsupported_syntax_errors.message", &self.message)?;
        require_string(
            "fixtures.unsupported_syntax_errors.source_or_metadata_sha256",
            &self.source_or_metadata_sha256,
            &input.source_or_metadata_sha256,
        )?;
        if &self.parser_or_grammar_identity != identity {
            return Err(SyntaxBackendBoundaryError::new(
                "fixtures.unsupported_syntax_errors.parser_or_grammar_identity",
                "unsupported syntax provenance must match fixture input provenance",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxInputDiagnosticsSidecarBoundary {
    pub boundary: String,
    pub sidecar_only: bool,
    pub tct_compiler_route_allowed: bool,
}

impl SyntaxInputDiagnosticsSidecarBoundary {
    fn validate(&self) -> Result<(), SyntaxBackendBoundaryError> {
        require_string(
            "fixtures.sidecar_boundary.boundary",
            &self.boundary,
            "GrammarIR -> StructuralModel",
        )?;
        require_bool(
            "fixtures.sidecar_boundary.sidecar_only",
            self.sidecar_only,
            true,
        )?;
        require_bool(
            "fixtures.sidecar_boundary.tct_compiler_route_allowed",
            self.tct_compiler_route_allowed,
            false,
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

fn require_non_empty_list(
    field: &'static str,
    actual: &[String],
) -> Result<(), SyntaxBackendBoundaryError> {
    if actual.is_empty() {
        return Err(SyntaxBackendBoundaryError::new(
            field,
            "expected at least one value",
        ));
    }
    if actual.iter().any(String::is_empty) {
        return Err(SyntaxBackendBoundaryError::new(
            field,
            "expected only non-empty values",
        ));
    }
    Ok(())
}

fn require_all(
    field: &'static str,
    actual: &[String],
    required: &[&str],
) -> Result<(), SyntaxBackendBoundaryError> {
    let actual = actual.iter().map(String::as_str).collect::<BTreeSet<_>>();
    for required in required {
        if !actual.contains(required) {
            return Err(SyntaxBackendBoundaryError::new(
                field,
                format!("missing required value {required}"),
            ));
        }
    }
    Ok(())
}
