//! Sidecar-only syntax backend production generator API boundary.
//!
//! This module turns the checked production-boundary contract fixture into a
//! small Rust API description for the future syntax generator. It does not
//! route `GrammarIR` or `StructuralModel` through `TctCompiler`, and it does not
//! generate production syntax codecs.

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::error::Error;
use std::fmt;

pub const TOOL: &str = "syntax_backend_production_generator_api_boundary";
pub const VERSION: u32 = 1;
pub const ARTIFACT_KIND: &str = "tct_syntax_backend_production_generator_api_boundary";
pub const API_VERSION: &str = "syntax-generator-api-v0";
pub const CONTRACT_ARTIFACT_KIND: &str = "tct_syntax_backend_production_boundary_contract";
pub const CONTRACT_VERSION: u32 = 1;
pub const CONTRACT_STATUS: &str = "sidecar_contract_only";
pub const BOUNDARY_MANIFEST_PATH: &str =
    "tests/fixtures/syntax_structural_model/boundary-manifest-v1.json";
pub const BOUNDARY_MANIFEST_ARTIFACT_KIND: &str = "tct_syntax_structural_model_boundary_manifest";
pub const READINESS_REPORT_PATH: &str = "reports/m6-cross-target-contract-closure-2026-07-21.md";
pub const GATE_HELPER_PATH: &str = "./scripts/testing/test-syntax-backend-gates.sh";
pub const COMPLETED_PROMOTION_CLASS: &str = "syntax backend production generator API boundary";
pub const NEXT_MISSING_STABLE_PROMOTION_CLASS: &str =
    "syntax backend JSON-backend parity routing gate";

const REQUIRED_TARGETS: &[&str] = &["toy", "csharp", "go"];
const REQUIRED_SURFACES: &[&str] = &[
    "checked_syntax_input",
    "structural_model_boundary",
    "token_layout",
    "generated_codec_artifact_metadata",
    "strict_recovery_decode_policy",
    "consumed_token_accounting",
    "token_vocabulary_bpe_invariants",
    "validation_hooks",
    "audit_markers",
    "maintained_documentation",
    "external_sidecar_evidence",
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxBackendProductionBoundaryContract {
    pub artifact_kind: String,
    pub version: u32,
    pub status: String,
    pub source_evidence: SyntaxBackendSourceEvidence,
    pub compiler_boundary: SyntaxBackendCompilerBoundary,
    pub production_handoff: SyntaxBackendProductionHandoff,
    pub required_surfaces: Vec<String>,
    pub next_missing_stable_promotion_class: String,
}

impl SyntaxBackendProductionBoundaryContract {
    pub fn validate(&self) -> Result<(), SyntaxBackendBoundaryError> {
        require_string("artifact_kind", &self.artifact_kind, CONTRACT_ARTIFACT_KIND)?;
        require_u32("version", self.version, CONTRACT_VERSION)?;
        require_string("status", &self.status, CONTRACT_STATUS)?;
        require_string(
            "source_evidence.readiness_report",
            &self.source_evidence.readiness_report,
            READINESS_REPORT_PATH,
        )?;
        require_string(
            "source_evidence.gate_helper",
            &self.source_evidence.gate_helper,
            GATE_HELPER_PATH,
        )?;
        let manifest = &self.source_evidence.boundary_manifest;
        require_string(
            "boundary_manifest.path",
            &manifest.path,
            BOUNDARY_MANIFEST_PATH,
        )?;
        require_string(
            "boundary_manifest.artifact_kind",
            &manifest.artifact_kind,
            BOUNDARY_MANIFEST_ARTIFACT_KIND,
        )?;
        require_u32("boundary_manifest.version", manifest.version, 1)?;
        require_all(
            "boundary_manifest.target_ids",
            &manifest.target_ids,
            REQUIRED_TARGETS,
        )?;

        self.compiler_boundary.require_sidecar_only()?;
        require_all(
            "required_surfaces",
            &self.required_surfaces,
            REQUIRED_SURFACES,
        )?;
        self.production_handoff.validate()?;
        require_string(
            "next_missing_stable_promotion_class",
            &self.next_missing_stable_promotion_class,
            NEXT_MISSING_STABLE_PROMOTION_CLASS,
        )?;
        Ok(())
    }

    pub fn production_generator_api_boundary(
        &self,
    ) -> Result<SyntaxGeneratorApiBoundary, SyntaxBackendBoundaryError> {
        self.validate()?;
        let handoff = &self.production_handoff;
        Ok(SyntaxGeneratorApiBoundary {
            tool: TOOL.to_string(),
            version: VERSION,
            artifact_kind: ARTIFACT_KIND.to_string(),
            api_version: API_VERSION.to_string(),
            source_contract: SyntaxGeneratorSourceContract {
                artifact_kind: self.artifact_kind.clone(),
                version: self.version,
                status: self.status.clone(),
                readiness_report: self.source_evidence.readiness_report.clone(),
                boundary_manifest: self.source_evidence.boundary_manifest.clone(),
                gate_helper: self.source_evidence.gate_helper.clone(),
                completed_promotion_class: COMPLETED_PROMOTION_CLASS.to_string(),
            },
            compiler_boundary: self.compiler_boundary.clone(),
            inputs: vec![
                api_surface_from_one(&handoff.input),
                api_surface_from_one(&handoff.normalization),
                api_surface_from_one(&handoff.token_layout),
            ],
            outputs: vec![
                api_surface_from_one(&handoff.generated_codec),
                api_surface_from_many(&handoff.decode),
                api_surface_from_one(&handoff.validation),
                api_surface_from_many(&handoff.audit),
            ],
            metadata_envelope: SyntaxGeneratorMetadataEnvelope {
                required_fields: vec![
                    "artifact_kind".to_string(),
                    "version".to_string(),
                    "api_version".to_string(),
                    "source_contract".to_string(),
                    "compiler_boundary".to_string(),
                    "inputs".to_string(),
                    "outputs".to_string(),
                    "generated_codec_artifact_metadata".to_string(),
                    "generated_crate_identity".to_string(),
                    "rendered_source_sha256".to_string(),
                    "target_manifest_sha256".to_string(),
                    "generation_options_sha256".to_string(),
                ],
                volatile_artifact_policy: "generated syntax artifacts remain target/ sidecar evidence until a persistence gate promotes them".to_string(),
                sidecar_evidence_paths: vec![
                    self.source_evidence.boundary_manifest.path.clone(),
                    self.source_evidence.readiness_report.clone(),
                    self.source_evidence.gate_helper.clone(),
                ],
            },
            error_surface: SyntaxGeneratorErrorSurface {
                unsupported_syntax_errors_require_rule_path: true,
                strict_decode_authoritative: true,
                recovery_normalizes_trailing_pad_only: true,
                partial_prefix_failures_are_hard: true,
                consumed_token_accounting_required: true,
            },
            non_goals: vec![
                "do not route GrammarIR or StructuralModel through TctCompiler".to_string(),
                "do not change generated JSON Schema output".to_string(),
                "do not change the core compiler path".to_string(),
                "do not promote external parser or formatter checks to stable gates".to_string(),
                "do not persist syntax artifacts outside target/ without a later artifact envelope gate".to_string(),
            ],
            next_missing_stable_promotion_class: NEXT_MISSING_STABLE_PROMOTION_CLASS.to_string(),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxBackendSourceEvidence {
    pub boundary_manifest: SyntaxBackendBoundaryManifestEvidence,
    pub gate_helper: String,
    pub readiness_report: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxBackendBoundaryManifestEvidence {
    pub artifact_kind: String,
    pub path: String,
    pub target_ids: Vec<String>,
    pub version: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxBackendCompilerBoundary {
    pub core_compiler_path_changed: bool,
    pub generated_json_schema_output_changed: bool,
    pub preserves_json_schema_backend_contract: bool,
    pub production_grammar_compiler_path_changed: bool,
    pub sidecar_only: bool,
    pub tct_compiler_route_allowed: bool,
}

impl SyntaxBackendCompilerBoundary {
    fn require_sidecar_only(&self) -> Result<(), SyntaxBackendBoundaryError> {
        require_bool("compiler_boundary.sidecar_only", self.sidecar_only, true)?;
        require_bool(
            "compiler_boundary.tct_compiler_route_allowed",
            self.tct_compiler_route_allowed,
            false,
        )?;
        require_bool(
            "compiler_boundary.generated_json_schema_output_changed",
            self.generated_json_schema_output_changed,
            false,
        )?;
        require_bool(
            "compiler_boundary.core_compiler_path_changed",
            self.core_compiler_path_changed,
            false,
        )?;
        require_bool(
            "compiler_boundary.production_grammar_compiler_path_changed",
            self.production_grammar_compiler_path_changed,
            false,
        )?;
        require_bool(
            "compiler_boundary.preserves_json_schema_backend_contract",
            self.preserves_json_schema_backend_contract,
            true,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxBackendProductionHandoff {
    pub input: SyntaxBackendSurfaceRequirement,
    pub normalization: SyntaxBackendSurfaceRequirement,
    pub token_layout: SyntaxBackendSurfaceRequirement,
    pub generated_codec: SyntaxBackendSurfaceRequirement,
    pub decode: SyntaxBackendMultiSurfaceRequirement,
    pub validation: SyntaxBackendSurfaceRequirement,
    pub audit: SyntaxBackendMultiSurfaceRequirement,
}

impl SyntaxBackendProductionHandoff {
    fn validate(&self) -> Result<(), SyntaxBackendBoundaryError> {
        require_string(
            "production_handoff.input.surface",
            &self.input.surface,
            "checked_syntax_input",
        )?;
        require_string(
            "production_handoff.normalization.surface",
            &self.normalization.surface,
            "structural_model_boundary",
        )?;
        require_string(
            "production_handoff.token_layout.surface",
            &self.token_layout.surface,
            "token_layout",
        )?;
        require_string(
            "production_handoff.generated_codec.surface",
            &self.generated_codec.surface,
            "generated_codec_artifact_metadata",
        )?;
        require_all(
            "production_handoff.decode.surfaces",
            &self.decode.surfaces,
            &["strict_recovery_decode_policy", "consumed_token_accounting"],
        )?;
        require_string(
            "production_handoff.validation.surface",
            &self.validation.surface,
            "validation_hooks",
        )?;
        require_all(
            "production_handoff.audit.surfaces",
            &self.audit.surfaces,
            &["audit_markers", "maintained_documentation"],
        )?;
        for (field, requires) in [
            ("production_handoff.input.requires", &self.input.requires),
            (
                "production_handoff.normalization.requires",
                &self.normalization.requires,
            ),
            (
                "production_handoff.token_layout.requires",
                &self.token_layout.requires,
            ),
            (
                "production_handoff.generated_codec.requires",
                &self.generated_codec.requires,
            ),
            ("production_handoff.decode.requires", &self.decode.requires),
            (
                "production_handoff.validation.requires",
                &self.validation.requires,
            ),
            ("production_handoff.audit.requires", &self.audit.requires),
        ] {
            if requires.is_empty() {
                return Err(SyntaxBackendBoundaryError::new(
                    field,
                    "must name at least one required input or invariant",
                ));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxBackendSurfaceRequirement {
    pub surface: String,
    pub requires: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxBackendMultiSurfaceRequirement {
    pub surfaces: Vec<String>,
    pub requires: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxGeneratorApiBoundary {
    pub tool: String,
    pub version: u32,
    pub artifact_kind: String,
    pub api_version: String,
    pub source_contract: SyntaxGeneratorSourceContract,
    pub compiler_boundary: SyntaxBackendCompilerBoundary,
    pub inputs: Vec<SyntaxGeneratorApiSurface>,
    pub outputs: Vec<SyntaxGeneratorApiSurface>,
    pub metadata_envelope: SyntaxGeneratorMetadataEnvelope,
    pub error_surface: SyntaxGeneratorErrorSurface,
    pub non_goals: Vec<String>,
    pub next_missing_stable_promotion_class: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxGeneratorSourceContract {
    pub artifact_kind: String,
    pub version: u32,
    pub status: String,
    pub readiness_report: String,
    pub boundary_manifest: SyntaxBackendBoundaryManifestEvidence,
    pub gate_helper: String,
    pub completed_promotion_class: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxGeneratorApiSurface {
    pub surfaces: Vec<String>,
    pub requires: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxGeneratorMetadataEnvelope {
    pub required_fields: Vec<String>,
    pub volatile_artifact_policy: String,
    pub sidecar_evidence_paths: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxGeneratorErrorSurface {
    pub unsupported_syntax_errors_require_rule_path: bool,
    pub strict_decode_authoritative: bool,
    pub recovery_normalizes_trailing_pad_only: bool,
    pub partial_prefix_failures_are_hard: bool,
    pub consumed_token_accounting_required: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyntaxBackendBoundaryError {
    field: &'static str,
    message: String,
}

impl SyntaxBackendBoundaryError {
    pub(crate) fn new(field: &'static str, message: impl Into<String>) -> Self {
        Self {
            field,
            message: message.into(),
        }
    }

    pub fn field(&self) -> &'static str {
        self.field
    }
}

impl fmt::Display for SyntaxBackendBoundaryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.field, self.message)
    }
}

impl Error for SyntaxBackendBoundaryError {}

fn api_surface_from_one(
    requirement: &SyntaxBackendSurfaceRequirement,
) -> SyntaxGeneratorApiSurface {
    SyntaxGeneratorApiSurface {
        surfaces: vec![requirement.surface.clone()],
        requires: requirement.requires.clone(),
    }
}

fn api_surface_from_many(
    requirement: &SyntaxBackendMultiSurfaceRequirement,
) -> SyntaxGeneratorApiSurface {
    SyntaxGeneratorApiSurface {
        surfaces: requirement.surfaces.clone(),
        requires: requirement.requires.clone(),
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
