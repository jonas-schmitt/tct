//! Replay contract for syntax generated-artifact reproducibility evidence.

use crate::provenance::json_sha256;
use crate::syntax_artifact_envelope::{
    ARTIFACT_KIND as ENVELOPE_ARTIFACT_KIND, FIXTURE_PATH as ENVELOPE_FIXTURE_PATH,
    SyntaxGeneratedArtifactEnvelope, VERSION as ENVELOPE_VERSION, VOLATILE_REPORT_DIR,
};
use crate::syntax_backend_boundary::{
    API_VERSION, GATE_HELPER_PATH, SyntaxBackendBoundaryError, SyntaxBackendCompilerBoundary,
};
use crate::syntax_input_diagnostics::{
    ARTIFACT_KIND as INPUT_DIAGNOSTICS_ARTIFACT_KIND,
    FIXTURE_PATH as INPUT_DIAGNOSTICS_FIXTURE_PATH,
    NEGATIVE_FIXTURE_ARTIFACT_KIND as INPUT_DIAGNOSTICS_NEGATIVE_FIXTURE_ARTIFACT_KIND,
    NEGATIVE_FIXTURE_PATH as INPUT_DIAGNOSTICS_NEGATIVE_FIXTURE_PATH,
    SyntaxInputDiagnosticsContract, SyntaxInputDiagnosticsNegativeFixtureGate,
    VERSION as INPUT_DIAGNOSTICS_VERSION,
};
use crate::syntax_validation_hooks::{
    SIDECAR_EVIDENCE_ARTIFACT_KIND as VALIDATION_HOOK_SIDECAR_EVIDENCE_ARTIFACT_KIND,
    SIDECAR_EVIDENCE_PATH as VALIDATION_HOOK_SIDECAR_EVIDENCE_PATH,
    SyntaxValidationHookSidecarEvidence, VERSION as VALIDATION_HOOK_VERSION,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const TOOL: &str = "syntax_backend_generated_artifact_reproducibility_replay_fixture";
pub const VERSION: u32 = 1;
pub const ARTIFACT_KIND: &str = "tct_syntax_backend_generated_artifact_reproducibility_replay";
pub const FIXTURE_PATH: &str = "tests/fixtures/syntax_backend/generated-artifact-replay-v1.json";
pub const COMPLETED_PROMOTION_CLASS: &str =
    "syntax backend generated artifact reproducibility replay gate";
pub const NEXT_MISSING_STABLE_PROMOTION_CLASS: &str =
    "syntax backend JSON-backend parity routing gate";
pub const TOY_REPORT_PATH: &str =
    "target/tct-syntax-generated-codec-reproducibility/toy-syntax-generated-codec.json";
pub const PARSER_BACKED_REPORT_PATH: &str =
    "target/tct-syntax-generated-codec-reproducibility/parser-backed-syntax-generated-codecs.json";
pub const TOY_REPORT_ARTIFACT_KIND: &str = "tct_syntax_generated_codec_reproducibility_report";
pub const PARSER_BACKED_REPORT_ARTIFACT_KIND: &str =
    "tct_parser_backed_syntax_generated_codec_reproducibility_report";

const EXPECTED_TARGET_IDS: [&str; 3] = ["toy", "csharp", "go"];
const REQUIRED_HASH_FIELDS: [&str; 14] = [
    "rendered_source_sha256",
    "source_or_metadata_sha256",
    "grammar_input_sha256",
    "source_metadata_provenance_sha256",
    "structural_model_sha256",
    "token_layout_sha256",
    "generated_crate_identity_sha256",
    "generation_options_sha256",
    "generated_artifact_envelope_sha256",
    "validation_hook_sidecar_evidence_sha256",
    "input_diagnostics_contract_sha256",
    "input_diagnostics_negative_fixture_sha256",
    "boundary_manifest_sha256",
    "target_manifest_sha256",
];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SyntaxGeneratedArtifactReproducibilityReplay {
    pub artifact_kind: String,
    pub version: u32,
    pub tool: String,
    pub api_version: String,
    pub source_generated_artifact_envelope: SyntaxGeneratedArtifactReplaySourceFixture,
    pub source_validation_hook_sidecar_evidence: SyntaxGeneratedArtifactReplaySourceFixture,
    pub source_input_diagnostics_contract: SyntaxGeneratedArtifactReplaySourceFixture,
    pub source_input_diagnostics_negative_fixture: SyntaxGeneratedArtifactReplaySourceFixture,
    pub target_ids: Vec<String>,
    pub volatile_report_policy: SyntaxGeneratedArtifactReplayVolatileReportPolicy,
    pub deterministic_hash_fields: Vec<String>,
    pub records: Vec<SyntaxGeneratedArtifactReplayRecord>,
    pub compiler_boundary: SyntaxBackendCompilerBoundary,
    pub completed_promotion_class: String,
    pub next_missing_stable_promotion_class: String,
}

impl SyntaxGeneratedArtifactReproducibilityReplay {
    pub fn validate_against_sources(
        &self,
        envelope: &SyntaxGeneratedArtifactEnvelope,
        sidecar_evidence: &SyntaxValidationHookSidecarEvidence,
        input_diagnostics: &SyntaxInputDiagnosticsContract,
        negative_fixture: &SyntaxInputDiagnosticsNegativeFixtureGate,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        if self.artifact_kind != ARTIFACT_KIND {
            return Err(invalid(format!(
                "expected artifact_kind {ARTIFACT_KIND}, got {}",
                self.artifact_kind
            )));
        }
        if self.version != VERSION {
            return Err(invalid(format!(
                "expected version {VERSION}, got {}",
                self.version
            )));
        }
        if self.tool != TOOL {
            return Err(invalid(format!("expected tool {TOOL}, got {}", self.tool)));
        }
        if self.api_version != API_VERSION {
            return Err(invalid(format!(
                "expected api_version {API_VERSION}, got {}",
                self.api_version
            )));
        }

        self.source_generated_artifact_envelope.validate(
            "source_generated_artifact_envelope",
            &ExpectedReplaySourceFixture {
                artifact_kind: ENVELOPE_ARTIFACT_KIND,
                version: ENVELOPE_VERSION,
                fixture_path: ENVELOPE_FIXTURE_PATH,
                module_path: "src/syntax_artifact_envelope.rs",
                test_path: "tests/syntax_backend_boundary_test.rs",
                gate_helper: GATE_HELPER_PATH,
            },
        )?;
        self.source_validation_hook_sidecar_evidence.validate(
            "source_validation_hook_sidecar_evidence",
            &ExpectedReplaySourceFixture {
                artifact_kind: VALIDATION_HOOK_SIDECAR_EVIDENCE_ARTIFACT_KIND,
                version: VALIDATION_HOOK_VERSION,
                fixture_path: VALIDATION_HOOK_SIDECAR_EVIDENCE_PATH,
                module_path: "src/syntax_validation_hooks.rs",
                test_path: "tests/syntax_backend_boundary_test.rs",
                gate_helper: GATE_HELPER_PATH,
            },
        )?;
        self.source_input_diagnostics_contract.validate(
            "source_input_diagnostics_contract",
            &ExpectedReplaySourceFixture {
                artifact_kind: INPUT_DIAGNOSTICS_ARTIFACT_KIND,
                version: INPUT_DIAGNOSTICS_VERSION,
                fixture_path: INPUT_DIAGNOSTICS_FIXTURE_PATH,
                module_path: "src/syntax_input_diagnostics.rs",
                test_path: "tests/syntax_backend_boundary_test.rs",
                gate_helper: GATE_HELPER_PATH,
            },
        )?;
        self.source_input_diagnostics_negative_fixture.validate(
            "source_input_diagnostics_negative_fixture",
            &ExpectedReplaySourceFixture {
                artifact_kind: INPUT_DIAGNOSTICS_NEGATIVE_FIXTURE_ARTIFACT_KIND,
                version: INPUT_DIAGNOSTICS_VERSION,
                fixture_path: INPUT_DIAGNOSTICS_NEGATIVE_FIXTURE_PATH,
                module_path: "src/syntax_input_diagnostics.rs",
                test_path: "tests/syntax_backend_boundary_test.rs",
                gate_helper: GATE_HELPER_PATH,
            },
        )?;

        require_target_ids(&self.target_ids)?;
        self.volatile_report_policy.validate()?;
        require_all(
            "deterministic_hash_fields",
            &self.deterministic_hash_fields,
            &REQUIRED_HASH_FIELDS,
        )?;
        require_compiler_boundary(&self.compiler_boundary)?;
        if self.completed_promotion_class != COMPLETED_PROMOTION_CLASS {
            return Err(invalid(format!(
                "completed promotion class must be {COMPLETED_PROMOTION_CLASS}"
            )));
        }
        if self.next_missing_stable_promotion_class != NEXT_MISSING_STABLE_PROMOTION_CLASS {
            return Err(invalid(format!(
                "next missing stable promotion class must be {NEXT_MISSING_STABLE_PROMOTION_CLASS}"
            )));
        }

        let envelope_sha256 = json_sha256(envelope).map_err(|error| {
            invalid(format!(
                "failed to hash generated artifact envelope fixture: {error}"
            ))
        })?;
        let sidecar_evidence_sha256 = json_sha256(sidecar_evidence).map_err(|error| {
            invalid(format!(
                "failed to hash validation hook sidecar evidence fixture: {error}"
            ))
        })?;
        let input_diagnostics_sha256 = json_sha256(input_diagnostics).map_err(|error| {
            invalid(format!(
                "failed to hash input diagnostics contract fixture: {error}"
            ))
        })?;
        let negative_fixture_sha256 = json_sha256(negative_fixture).map_err(|error| {
            invalid(format!(
                "failed to hash input diagnostics negative fixture: {error}"
            ))
        })?;
        let sidecar_targets: BTreeSet<&str> = sidecar_evidence
            .records
            .iter()
            .map(|record| record.target_id.as_str())
            .collect();

        if self.records.len() != EXPECTED_TARGET_IDS.len() {
            return Err(invalid(format!(
                "expected {} replay records, got {}",
                EXPECTED_TARGET_IDS.len(),
                self.records.len()
            )));
        }

        let mut seen = BTreeSet::new();
        for record in &self.records {
            if !self
                .target_ids
                .iter()
                .any(|target_id| target_id == &record.target_id)
            {
                return Err(invalid(format!(
                    "replay record target_id {} is not listed in target_ids",
                    record.target_id
                )));
            }
            if !sidecar_targets.contains(record.target_id.as_str()) {
                return Err(invalid(format!(
                    "replay record target_id {} is not present in validation hook sidecar evidence",
                    record.target_id
                )));
            }
            if !seen.insert(record.target_id.as_str()) {
                return Err(invalid(format!(
                    "duplicate replay record target_id {}",
                    record.target_id
                )));
            }
            record.validate(
                &envelope_sha256,
                &sidecar_evidence_sha256,
                &input_diagnostics_sha256,
                &negative_fixture_sha256,
            )?;
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SyntaxGeneratedArtifactReplaySourceFixture {
    pub artifact_kind: String,
    pub version: u32,
    pub fixture_path: String,
    pub module_path: String,
    pub test_path: String,
    pub gate_helper: String,
}

impl SyntaxGeneratedArtifactReplaySourceFixture {
    fn validate(
        &self,
        field_name: &str,
        expected: &ExpectedReplaySourceFixture<'_>,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        if self.artifact_kind != expected.artifact_kind {
            return Err(invalid(format!(
                "{field_name}.artifact_kind must be {}, got {}",
                expected.artifact_kind, self.artifact_kind
            )));
        }
        if self.version != expected.version {
            return Err(invalid(format!(
                "{field_name}.version must be {}, got {}",
                expected.version, self.version
            )));
        }
        if self.fixture_path != expected.fixture_path {
            return Err(invalid(format!(
                "{field_name}.fixture_path must be {}, got {}",
                expected.fixture_path, self.fixture_path
            )));
        }
        if self.module_path != expected.module_path {
            return Err(invalid(format!(
                "{field_name}.module_path must be {}, got {}",
                expected.module_path, self.module_path
            )));
        }
        if self.test_path != expected.test_path {
            return Err(invalid(format!(
                "{field_name}.test_path must be {}, got {}",
                expected.test_path, self.test_path
            )));
        }
        if self.gate_helper != expected.gate_helper {
            return Err(invalid(format!(
                "{field_name}.gate_helper must be {}, got {}",
                expected.gate_helper, self.gate_helper
            )));
        }
        Ok(())
    }
}

struct ExpectedReplaySourceFixture<'a> {
    artifact_kind: &'a str,
    version: u32,
    fixture_path: &'a str,
    module_path: &'a str,
    test_path: &'a str,
    gate_helper: &'a str,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SyntaxGeneratedArtifactReplayVolatileReportPolicy {
    pub checked_in_reports: bool,
    pub volatile_report_dir: String,
    pub toy_report_path: String,
    pub parser_backed_report_path: String,
    pub report_artifact_kinds: Vec<String>,
}

impl SyntaxGeneratedArtifactReplayVolatileReportPolicy {
    fn validate(&self) -> Result<(), SyntaxBackendBoundaryError> {
        if self.checked_in_reports {
            return Err(invalid(
                "syntax generated-artifact replay reports must remain volatile".to_string(),
            ));
        }
        if self.volatile_report_dir != VOLATILE_REPORT_DIR {
            return Err(invalid(format!(
                "volatile_report_dir must be {VOLATILE_REPORT_DIR}, got {}",
                self.volatile_report_dir
            )));
        }
        if self.toy_report_path != TOY_REPORT_PATH {
            return Err(invalid(format!(
                "toy_report_path must be {TOY_REPORT_PATH}, got {}",
                self.toy_report_path
            )));
        }
        if self.parser_backed_report_path != PARSER_BACKED_REPORT_PATH {
            return Err(invalid(format!(
                "parser_backed_report_path must be {PARSER_BACKED_REPORT_PATH}, got {}",
                self.parser_backed_report_path
            )));
        }
        require_all(
            "report_artifact_kinds",
            &self.report_artifact_kinds,
            &[TOY_REPORT_ARTIFACT_KIND, PARSER_BACKED_REPORT_ARTIFACT_KIND],
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SyntaxGeneratedArtifactReplayRecord {
    pub target_id: String,
    pub source_kind: String,
    pub report_path: String,
    pub report_artifact_kind: String,
    pub rendered_source_sha256: String,
    pub source_or_metadata_sha256: String,
    pub parser_or_grammar_identity: SyntaxGeneratedArtifactReplayParserOrGrammarIdentity,
    pub grammar_input_sha256: String,
    pub source_metadata_provenance_sha256: String,
    pub structural_model_sha256: String,
    pub token_layout_sha256: String,
    pub generated_crate_identity_sha256: String,
    pub generation_options_sha256: String,
    pub generated_artifact_envelope_sha256: String,
    pub validation_hook_sidecar_evidence_sha256: String,
    pub input_diagnostics_contract_sha256: String,
    pub input_diagnostics_negative_fixture_sha256: String,
    pub boundary_manifest_sha256: String,
    pub target_manifest_sha256: String,
    pub compiler_boundary_status: SyntaxGeneratedArtifactReplayCompilerBoundaryStatus,
}

impl SyntaxGeneratedArtifactReplayRecord {
    fn validate(
        &self,
        envelope_sha256: &str,
        sidecar_evidence_sha256: &str,
        input_diagnostics_sha256: &str,
        negative_fixture_sha256: &str,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        match self.target_id.as_str() {
            "toy" => {
                require_eq("toy source_kind", &self.source_kind, "toy_grammar")?;
                require_eq("toy report_path", &self.report_path, TOY_REPORT_PATH)?;
                require_eq(
                    "toy report_artifact_kind",
                    &self.report_artifact_kind,
                    TOY_REPORT_ARTIFACT_KIND,
                )?;
            }
            "csharp" | "go" => {
                require_eq(
                    "parser-backed source_kind",
                    &self.source_kind,
                    "tree_sitter",
                )?;
                require_eq(
                    "parser-backed report_path",
                    &self.report_path,
                    PARSER_BACKED_REPORT_PATH,
                )?;
                require_eq(
                    "parser-backed report_artifact_kind",
                    &self.report_artifact_kind,
                    PARSER_BACKED_REPORT_ARTIFACT_KIND,
                )?;
            }
            target => {
                return Err(invalid(format!("unexpected replay target_id {target}")));
            }
        }

        self.parser_or_grammar_identity
            .validate(self.target_id.as_str())?;
        require_sha256("rendered_source_sha256", &self.rendered_source_sha256)?;
        require_sha256("source_or_metadata_sha256", &self.source_or_metadata_sha256)?;
        require_sha256("grammar_input_sha256", &self.grammar_input_sha256)?;
        require_sha256(
            "source_metadata_provenance_sha256",
            &self.source_metadata_provenance_sha256,
        )?;
        require_sha256("structural_model_sha256", &self.structural_model_sha256)?;
        require_sha256("token_layout_sha256", &self.token_layout_sha256)?;
        require_sha256(
            "generated_crate_identity_sha256",
            &self.generated_crate_identity_sha256,
        )?;
        require_sha256("generation_options_sha256", &self.generation_options_sha256)?;
        require_eq(
            "generated_artifact_envelope_sha256",
            &self.generated_artifact_envelope_sha256,
            envelope_sha256,
        )?;
        require_eq(
            "validation_hook_sidecar_evidence_sha256",
            &self.validation_hook_sidecar_evidence_sha256,
            sidecar_evidence_sha256,
        )?;
        require_eq(
            "input_diagnostics_contract_sha256",
            &self.input_diagnostics_contract_sha256,
            input_diagnostics_sha256,
        )?;
        require_eq(
            "input_diagnostics_negative_fixture_sha256",
            &self.input_diagnostics_negative_fixture_sha256,
            negative_fixture_sha256,
        )?;
        require_sha256("boundary_manifest_sha256", &self.boundary_manifest_sha256)?;
        require_sha256("target_manifest_sha256", &self.target_manifest_sha256)?;
        self.compiler_boundary_status
            .validate(self.target_id.as_str())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SyntaxGeneratedArtifactReplayParserOrGrammarIdentity {
    pub kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grammar_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grammar_version: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub package: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub package_version: Option<String>,
}

impl SyntaxGeneratedArtifactReplayParserOrGrammarIdentity {
    fn validate(&self, target_id: &str) -> Result<(), SyntaxBackendBoundaryError> {
        match target_id {
            "toy" => {
                require_eq("toy parser_or_grammar_identity.kind", &self.kind, "grammar")?;
                require_some("toy grammar_id", self.grammar_id.as_deref())?;
                if self.grammar_version.is_none() {
                    return Err(invalid("toy grammar_version must be present".to_string()));
                }
                require_none("toy parser tool", self.tool.as_deref())?;
                require_none("toy parser package", self.package.as_deref())?;
                require_none("toy parser version", self.version.as_deref())?;
                require_none(
                    "toy parser package_version",
                    self.package_version.as_deref(),
                )?;
            }
            "csharp" | "go" => {
                require_eq(
                    "parser-backed parser_or_grammar_identity.kind",
                    &self.kind,
                    "parser",
                )?;
                require_none("parser-backed grammar_id", self.grammar_id.as_deref())?;
                if self.grammar_version.is_some() {
                    return Err(invalid(
                        "parser-backed grammar_version must be absent".to_string(),
                    ));
                }
                require_some("parser-backed parser tool", self.tool.as_deref())?;
                require_some("parser-backed parser package", self.package.as_deref())?;
                require_some("parser-backed parser version", self.version.as_deref())?;
                require_some(
                    "parser-backed parser package_version",
                    self.package_version.as_deref(),
                )?;
            }
            target => {
                return Err(invalid(format!("unexpected replay target_id {target}")));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SyntaxGeneratedArtifactReplayCompilerBoundaryStatus {
    pub syntax_frontend_only: bool,
    pub uses_json_schema_compiler: bool,
    pub uses_tct_codegen: bool,
    pub stable_tct_backend_route: bool,
    pub sidecar_artifact_only: bool,
}

impl SyntaxGeneratedArtifactReplayCompilerBoundaryStatus {
    fn validate(&self, target_id: &str) -> Result<(), SyntaxBackendBoundaryError> {
        if !self.syntax_frontend_only {
            return Err(invalid(format!(
                "{target_id} replay must remain syntax_frontend_only"
            )));
        }
        if self.uses_json_schema_compiler {
            return Err(invalid(format!(
                "{target_id} replay must not use the JSON Schema compiler"
            )));
        }
        if self.uses_tct_codegen {
            return Err(invalid(format!(
                "{target_id} replay must not use TCT codegen yet"
            )));
        }
        if self.stable_tct_backend_route {
            return Err(invalid(format!(
                "{target_id} replay must not claim a stable TCT backend route"
            )));
        }
        if !self.sidecar_artifact_only {
            return Err(invalid(format!(
                "{target_id} replay must be recorded as sidecar_artifact_only"
            )));
        }
        Ok(())
    }
}

fn require_target_ids(target_ids: &[String]) -> Result<(), SyntaxBackendBoundaryError> {
    if target_ids.len() != EXPECTED_TARGET_IDS.len() {
        return Err(invalid(format!(
            "expected target_ids {:?}, got {:?}",
            EXPECTED_TARGET_IDS, target_ids
        )));
    }
    for expected in EXPECTED_TARGET_IDS {
        if !target_ids.iter().any(|target_id| target_id == expected) {
            return Err(invalid(format!("target_ids must include {expected}")));
        }
    }
    Ok(())
}

fn invalid(message: impl Into<String>) -> SyntaxBackendBoundaryError {
    SyntaxBackendBoundaryError::new("syntax_reproducibility_replay", message)
}

fn require_compiler_boundary(
    boundary: &SyntaxBackendCompilerBoundary,
) -> Result<(), SyntaxBackendBoundaryError> {
    if boundary.core_compiler_path_changed {
        return Err(invalid(
            "compiler boundary must not change core compiler path",
        ));
    }
    if boundary.generated_json_schema_output_changed {
        return Err(invalid(
            "compiler boundary must not change generated JSON Schema output",
        ));
    }
    if !boundary.preserves_json_schema_backend_contract {
        return Err(invalid(
            "compiler boundary must preserve the JSON Schema backend contract",
        ));
    }
    if boundary.production_grammar_compiler_path_changed {
        return Err(invalid(
            "compiler boundary must not change the production grammar compiler path",
        ));
    }
    if !boundary.sidecar_only {
        return Err(invalid("compiler boundary must remain sidecar-only"));
    }
    if boundary.tct_compiler_route_allowed {
        return Err(invalid(
            "compiler boundary must not allow TctCompiler routing",
        ));
    }
    Ok(())
}

fn require_all(
    field_name: &str,
    actual: &[String],
    expected: &[&str],
) -> Result<(), SyntaxBackendBoundaryError> {
    for required in expected {
        if !actual.iter().any(|item| item == required) {
            return Err(invalid(format!("{field_name} must include {required}")));
        }
    }
    Ok(())
}

fn require_eq(
    field_name: &str,
    actual: &str,
    expected: &str,
) -> Result<(), SyntaxBackendBoundaryError> {
    if actual != expected {
        return Err(invalid(format!(
            "{field_name} must be {expected}, got {actual}"
        )));
    }
    Ok(())
}

fn require_sha256(field_name: &str, value: &str) -> Result<(), SyntaxBackendBoundaryError> {
    if value.len() != 64 || !value.chars().all(|ch| matches!(ch, '0'..='9' | 'a'..='f')) {
        return Err(invalid(format!(
            "{field_name} must be a lowercase sha256 hex digest, got {value}"
        )));
    }
    Ok(())
}

fn require_some(field_name: &str, value: Option<&str>) -> Result<(), SyntaxBackendBoundaryError> {
    if value.is_none_or(str::is_empty) {
        return Err(invalid(format!("{field_name} must be present")));
    }
    Ok(())
}

fn require_none(field_name: &str, value: Option<&str>) -> Result<(), SyntaxBackendBoundaryError> {
    if value.is_some() {
        return Err(invalid(format!("{field_name} must be absent")));
    }
    Ok(())
}
