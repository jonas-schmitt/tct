//! Sidecar-only generated syntax artifact envelope contract.
//!
//! This module validates the smallest checked fixture for generated syntax
//! codec artifact metadata. It keeps artifact persistence and production
//! compiler routing out of scope until a later handoff gate promotes them.

use crate::syntax_backend_boundary::{
    API_VERSION, ARTIFACT_KIND as API_BOUNDARY_ARTIFACT_KIND, BOUNDARY_MANIFEST_PATH,
    GATE_HELPER_PATH, SyntaxBackendBoundaryError, SyntaxBackendCompilerBoundary,
    SyntaxGeneratorApiBoundary,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const TOOL: &str = "syntax_backend_generated_artifact_envelope_fixture";
pub const VERSION: u32 = 1;
pub const ARTIFACT_KIND: &str = "tct_syntax_backend_generated_artifact_envelope";
pub const FIXTURE_PATH: &str = "tests/fixtures/syntax_backend/generated-artifact-envelope-v1.json";
pub const COMPLETED_PROMOTION_CLASS: &str = "syntax backend generated artifact envelope fixture";
pub const NEXT_MISSING_STABLE_PROMOTION_CLASS: &str =
    "syntax backend JSON-backend parity routing gate";
pub const VOLATILE_REPORT_DIR: &str = "target/tct-syntax-generated-codec-reproducibility";

const REQUIRED_TARGETS: &[&str] = &["toy", "csharp", "go"];
const REQUIRED_METADATA_FIELDS: &[&str] = &[
    "artifact_kind",
    "version",
    "artifact_version",
    "api_version",
    "source_contract",
    "compiler_boundary",
    "inputs",
    "outputs",
    "generated_codec_artifact_metadata",
    "rendered_source",
    "rendered_source_sha256",
    "generated_crate",
    "generated_crate_identity",
    "boundary_manifest",
    "target_manifest_sha256",
    "generation_options",
    "generation_options_sha256",
    "source_metadata_provenance",
    "structural_model",
    "token_layout",
    "sidecar_boundary",
    "validation_hook_contract",
    "validation_hook_contract_sha256",
    "validation_hook_sidecar_evidence",
    "validation_hook_sidecar_evidence_sha256",
    "generated_artifact_replay",
    "generated_artifact_replay_sha256",
    "input_diagnostics_contract",
    "input_diagnostics_contract_sha256",
    "input_diagnostics_negative_fixture",
    "input_diagnostics_negative_fixture_sha256",
];
const REQUIRED_PROVENANCE_HASHES: &[&str] = &[
    "rendered_source_sha256",
    "boundary_manifest_sha256",
    "target_manifest_sha256",
    "generation_options_sha256",
    "source_metadata_provenance_sha256",
    "structural_model_sha256",
    "token_layout_sha256",
];
const REQUIRED_TOKEN_LAYOUT_FIELDS: &[&str] = &[
    "pad_token",
    "vocab_size",
    "raw_base_vocab_size",
    "structural_ranges",
    "payload_ranges",
    "bpe_boundary",
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxGeneratedArtifactEnvelope {
    pub artifact_kind: String,
    pub version: u32,
    pub api_version: String,
    pub source_api_boundary: SyntaxGeneratedArtifactSourceApiBoundary,
    pub artifact_metadata: SyntaxGeneratedArtifactMetadataShape,
    pub provenance_hashes: SyntaxGeneratedArtifactProvenanceHashes,
    pub persistence_policy: SyntaxGeneratedArtifactPersistencePolicy,
    pub decode_policy: SyntaxGeneratedArtifactDecodePolicy,
    pub token_layout_metadata: SyntaxGeneratedArtifactTokenLayoutMetadata,
    pub compiler_boundary: SyntaxBackendCompilerBoundary,
    pub non_goals: Vec<String>,
    pub completed_promotion_class: String,
    pub next_missing_stable_promotion_class: String,
}

impl SyntaxGeneratedArtifactEnvelope {
    pub fn validate_against_api_boundary(
        &self,
        api_boundary: &SyntaxGeneratorApiBoundary,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        require_string("artifact_kind", &self.artifact_kind, ARTIFACT_KIND)?;
        require_u32("version", self.version, VERSION)?;
        require_string("api_version", &self.api_version, API_VERSION)?;
        self.source_api_boundary.validate(api_boundary)?;
        self.artifact_metadata.validate(api_boundary)?;
        self.provenance_hashes.validate()?;
        self.persistence_policy.validate()?;
        self.decode_policy.validate()?;
        self.token_layout_metadata.validate()?;
        require_sidecar_boundary(&self.compiler_boundary)?;
        require_all(
            "non_goals",
            &self.non_goals,
            &[
                "no TctCompiler route",
                "no generated JSON Schema output change",
                "no checked-in syntax artifact persistence",
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
pub struct SyntaxGeneratedArtifactSourceApiBoundary {
    pub artifact_kind: String,
    pub version: u32,
    pub api_version: String,
    pub module_path: String,
    pub test_path: String,
    pub contract_fixture: String,
    pub gate_helper: String,
}

impl SyntaxGeneratedArtifactSourceApiBoundary {
    fn validate(
        &self,
        api_boundary: &SyntaxGeneratorApiBoundary,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        require_string(
            "source_api_boundary.artifact_kind",
            &self.artifact_kind,
            API_BOUNDARY_ARTIFACT_KIND,
        )?;
        require_u32(
            "source_api_boundary.version",
            self.version,
            api_boundary.version,
        )?;
        require_string(
            "source_api_boundary.api_version",
            &self.api_version,
            &api_boundary.api_version,
        )?;
        require_string(
            "source_api_boundary.module_path",
            &self.module_path,
            "src/syntax_backend_boundary.rs",
        )?;
        require_string(
            "source_api_boundary.test_path",
            &self.test_path,
            "tests/syntax_backend_boundary_test.rs",
        )?;
        require_string(
            "source_api_boundary.contract_fixture",
            &self.contract_fixture,
            "tests/fixtures/syntax_backend/production-boundary-contract-v1.json",
        )?;
        require_string(
            "source_api_boundary.gate_helper",
            &self.gate_helper,
            GATE_HELPER_PATH,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxGeneratedArtifactMetadataShape {
    pub surface: String,
    pub target_ids: Vec<String>,
    pub required_fields: Vec<String>,
    pub volatile_report_dir: String,
}

impl SyntaxGeneratedArtifactMetadataShape {
    fn validate(
        &self,
        api_boundary: &SyntaxGeneratorApiBoundary,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        require_string(
            "artifact_metadata.surface",
            &self.surface,
            "generated_codec_artifact_metadata",
        )?;
        require_all(
            "artifact_metadata.target_ids",
            &self.target_ids,
            REQUIRED_TARGETS,
        )?;
        require_all(
            "artifact_metadata.required_fields",
            &self.required_fields,
            REQUIRED_METADATA_FIELDS,
        )?;
        require_all(
            "artifact_metadata.required_fields",
            &self.required_fields,
            &api_boundary
                .metadata_envelope
                .required_fields
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>(),
        )?;
        require_string(
            "artifact_metadata.volatile_report_dir",
            &self.volatile_report_dir,
            VOLATILE_REPORT_DIR,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxGeneratedArtifactProvenanceHashes {
    pub required: Vec<String>,
}

impl SyntaxGeneratedArtifactProvenanceHashes {
    fn validate(&self) -> Result<(), SyntaxBackendBoundaryError> {
        require_all(
            "provenance_hashes.required",
            &self.required,
            REQUIRED_PROVENANCE_HASHES,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxGeneratedArtifactPersistencePolicy {
    pub checked_in: bool,
    pub volatile_report_dir: String,
    pub promotion_required_for_checked_in_artifacts: bool,
}

impl SyntaxGeneratedArtifactPersistencePolicy {
    fn validate(&self) -> Result<(), SyntaxBackendBoundaryError> {
        require_bool("persistence_policy.checked_in", self.checked_in, false)?;
        require_string(
            "persistence_policy.volatile_report_dir",
            &self.volatile_report_dir,
            VOLATILE_REPORT_DIR,
        )?;
        require_bool(
            "persistence_policy.promotion_required_for_checked_in_artifacts",
            self.promotion_required_for_checked_in_artifacts,
            true,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxGeneratedArtifactDecodePolicy {
    pub strict_decode_authoritative: bool,
    pub recovery_normalizes_trailing_pad_only: bool,
    pub partial_prefix_failures_are_hard: bool,
    pub consumed_token_accounting_required: bool,
}

impl SyntaxGeneratedArtifactDecodePolicy {
    fn validate(&self) -> Result<(), SyntaxBackendBoundaryError> {
        require_bool(
            "decode_policy.strict_decode_authoritative",
            self.strict_decode_authoritative,
            true,
        )?;
        require_bool(
            "decode_policy.recovery_normalizes_trailing_pad_only",
            self.recovery_normalizes_trailing_pad_only,
            true,
        )?;
        require_bool(
            "decode_policy.partial_prefix_failures_are_hard",
            self.partial_prefix_failures_are_hard,
            true,
        )?;
        require_bool(
            "decode_policy.consumed_token_accounting_required",
            self.consumed_token_accounting_required,
            true,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxGeneratedArtifactTokenLayoutMetadata {
    pub required_fields: Vec<String>,
    pub boundary_manifest_path: String,
    pub requires_bpe_collision_evidence: bool,
}

impl SyntaxGeneratedArtifactTokenLayoutMetadata {
    fn validate(&self) -> Result<(), SyntaxBackendBoundaryError> {
        require_all(
            "token_layout_metadata.required_fields",
            &self.required_fields,
            REQUIRED_TOKEN_LAYOUT_FIELDS,
        )?;
        require_string(
            "token_layout_metadata.boundary_manifest_path",
            &self.boundary_manifest_path,
            BOUNDARY_MANIFEST_PATH,
        )?;
        require_bool(
            "token_layout_metadata.requires_bpe_collision_evidence",
            self.requires_bpe_collision_evidence,
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
