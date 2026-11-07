//! Sidecar-only syntax `TctCompiler` routing gate contract.
//!
//! This module validates the routing handoff contract for a future syntax route
//! without registering that route in `TctCompiler`, changing the JSON Schema
//! compiler path, or changing generated JSON Schema output.

use crate::syntax_backend_boundary::{
    API_VERSION, ARTIFACT_KIND as GENERATOR_API_ARTIFACT_KIND, GATE_HELPER_PATH,
    SyntaxBackendBoundaryError, SyntaxBackendCompilerBoundary, SyntaxGeneratorApiBoundary,
};
use crate::syntax_generated_codec::{TOOL as RENDERER_API_TOOL, VERSION as RENDERER_API_VERSION};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const TOOL: &str = "syntax_backend_tct_compiler_routing_gate";
pub const VERSION: u32 = 1;
pub const ARTIFACT_KIND: &str = "tct_syntax_backend_tct_compiler_routing_gate";
pub const FIXTURE_PATH: &str = "tests/fixtures/syntax_backend/tct-compiler-routing-gate-v1.json";
pub const STATUS: &str = "sidecar_contract_only";
pub const COMPLETED_PROMOTION_CLASS: &str = "syntax backend production TctCompiler routing gate";
pub const NEXT_MISSING_STABLE_PROMOTION_CLASS: &str =
    "syntax backend JSON-backend parity routing gate";

const REQUIRED_BUILD_FUNCTIONS: &[&str] = &[
    "build_toy_syntax_generated_codec_artifact",
    "build_csharp_syntax_generated_codec_artifact",
    "build_go_syntax_generated_codec_artifact",
];
const REQUIRED_RENDERER_FUNCTIONS: &[&str] = &[
    "render_toy_syntax_codec_fixture",
    "render_csharp_syntax_codec_fixture",
    "render_go_syntax_codec_fixture",
];
const REQUIRED_HANDOFF_SURFACES: &[&str] = &[
    "checked_syntax_input",
    "structural_model_boundary",
    "syntax_generated_codec_renderer_api",
    "generated_codec_artifact_metadata",
    "strict_recovery_decode_policy",
    "consumed_token_accounting",
    "token_vocabulary_bpe_invariants",
    "validation_hooks",
    "audit_markers",
];
const REQUIRED_BLOCKERS: &[&str] = &[
    "JSON-backend parity gates",
    "target-agnostic syntax boundary selection",
    "strict/recovery decode behavior",
    "consumed-token accounting",
    "token/vocabulary/BPE invariants",
    "reproducibility",
    "audit coverage",
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxTctCompilerRoutingGate {
    pub artifact_kind: String,
    pub version: u32,
    pub tool: String,
    pub status: String,
    pub source_generator_api_boundary: SyntaxRoutingSourceGeneratorApiBoundary,
    pub source_renderer_api: SyntaxRoutingRendererApi,
    pub compiler_boundary: SyntaxBackendCompilerBoundary,
    pub tct_compiler_route: SyntaxTctCompilerRouteState,
    pub routing_handoff: SyntaxTctCompilerRoutingHandoff,
    pub json_backend_parity: SyntaxTctCompilerJsonBackendParity,
    pub blockers: Vec<String>,
    pub completed_promotion_class: String,
    pub next_missing_stable_promotion_class: String,
}

impl SyntaxTctCompilerRoutingGate {
    pub fn validate_against_generator_api(
        &self,
        generator_api: &SyntaxGeneratorApiBoundary,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        require_string("artifact_kind", &self.artifact_kind, ARTIFACT_KIND)?;
        require_u32("version", self.version, VERSION)?;
        require_string("tool", &self.tool, TOOL)?;
        require_string("status", &self.status, STATUS)?;
        self.source_generator_api_boundary.validate(generator_api)?;
        self.source_renderer_api.validate()?;
        require_sidecar_boundary(&self.compiler_boundary)?;
        self.tct_compiler_route.validate()?;
        self.routing_handoff.validate()?;
        self.json_backend_parity.validate()?;
        require_all("blockers", &self.blockers, REQUIRED_BLOCKERS)?;
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
pub struct SyntaxRoutingSourceGeneratorApiBoundary {
    pub artifact_kind: String,
    pub version: u32,
    pub api_version: String,
    pub module_path: String,
    pub contract_fixture: String,
    pub test_path: String,
    pub gate_helper: String,
}

impl SyntaxRoutingSourceGeneratorApiBoundary {
    fn validate(
        &self,
        generator_api: &SyntaxGeneratorApiBoundary,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        require_string(
            "source_generator_api_boundary.artifact_kind",
            &self.artifact_kind,
            GENERATOR_API_ARTIFACT_KIND,
        )?;
        require_u32(
            "source_generator_api_boundary.version",
            self.version,
            generator_api.version,
        )?;
        require_string(
            "source_generator_api_boundary.api_version",
            &self.api_version,
            API_VERSION,
        )?;
        require_string(
            "source_generator_api_boundary.module_path",
            &self.module_path,
            "src/syntax_backend_boundary.rs",
        )?;
        require_string(
            "source_generator_api_boundary.contract_fixture",
            &self.contract_fixture,
            "tests/fixtures/syntax_backend/production-boundary-contract-v1.json",
        )?;
        require_string(
            "source_generator_api_boundary.test_path",
            &self.test_path,
            "tests/syntax_backend_boundary_test.rs",
        )?;
        require_string(
            "source_generator_api_boundary.gate_helper",
            &self.gate_helper,
            GATE_HELPER_PATH,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxRoutingRendererApi {
    pub tool: String,
    pub version: u32,
    pub module_path: String,
    pub artifact_builder_type: String,
    pub build_functions: Vec<String>,
    pub renderer_functions: Vec<String>,
}

impl SyntaxRoutingRendererApi {
    fn validate(&self) -> Result<(), SyntaxBackendBoundaryError> {
        require_string("source_renderer_api.tool", &self.tool, RENDERER_API_TOOL)?;
        require_u32(
            "source_renderer_api.version",
            self.version,
            RENDERER_API_VERSION,
        )?;
        require_string(
            "source_renderer_api.module_path",
            &self.module_path,
            "src/syntax_generated_codec.rs",
        )?;
        require_string(
            "source_renderer_api.artifact_builder_type",
            &self.artifact_builder_type,
            "SyntaxGeneratedCodecArtifact",
        )?;
        require_all(
            "source_renderer_api.build_functions",
            &self.build_functions,
            REQUIRED_BUILD_FUNCTIONS,
        )?;
        require_all(
            "source_renderer_api.renderer_functions",
            &self.renderer_functions,
            REQUIRED_RENDERER_FUNCTIONS,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxTctCompilerRouteState {
    pub route_name: String,
    pub route_registered: bool,
    pub route_enabled: bool,
    pub public_api_changed: bool,
    pub uses_json_schema_compiler: bool,
    pub uses_syntax_renderer_api: bool,
    pub future_input: String,
}

impl SyntaxTctCompilerRouteState {
    fn validate(&self) -> Result<(), SyntaxBackendBoundaryError> {
        require_string("tct_compiler_route.route_name", &self.route_name, "syntax")?;
        require_bool(
            "tct_compiler_route.route_registered",
            self.route_registered,
            false,
        )?;
        require_bool(
            "tct_compiler_route.route_enabled",
            self.route_enabled,
            false,
        )?;
        require_bool(
            "tct_compiler_route.public_api_changed",
            self.public_api_changed,
            false,
        )?;
        require_bool(
            "tct_compiler_route.uses_json_schema_compiler",
            self.uses_json_schema_compiler,
            false,
        )?;
        require_bool(
            "tct_compiler_route.uses_syntax_renderer_api",
            self.uses_syntax_renderer_api,
            false,
        )?;
        require_string(
            "tct_compiler_route.future_input",
            &self.future_input,
            "checked GrammarIR -> StructuralModel",
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxTctCompilerRoutingHandoff {
    pub accepted_input: String,
    pub renderer_api_module: String,
    pub output_artifact: String,
    pub required_surfaces: Vec<String>,
}

impl SyntaxTctCompilerRoutingHandoff {
    fn validate(&self) -> Result<(), SyntaxBackendBoundaryError> {
        require_string(
            "routing_handoff.accepted_input",
            &self.accepted_input,
            "checked GrammarIR -> StructuralModel",
        )?;
        require_string(
            "routing_handoff.renderer_api_module",
            &self.renderer_api_module,
            "src/syntax_generated_codec.rs",
        )?;
        require_string(
            "routing_handoff.output_artifact",
            &self.output_artifact,
            "SyntaxGeneratedCodecArtifact",
        )?;
        require_all(
            "routing_handoff.required_surfaces",
            &self.required_surfaces,
            REQUIRED_HANDOFF_SURFACES,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxTctCompilerJsonBackendParity {
    pub stable_json_schema_backend_unchanged: bool,
    pub generated_json_schema_output_changed: bool,
    pub existing_generated_templates_changed: bool,
    pub requires_quality_check_before_route: bool,
    pub requires_round_trip_smoke_before_route: bool,
}

impl SyntaxTctCompilerJsonBackendParity {
    fn validate(&self) -> Result<(), SyntaxBackendBoundaryError> {
        require_bool(
            "json_backend_parity.stable_json_schema_backend_unchanged",
            self.stable_json_schema_backend_unchanged,
            true,
        )?;
        require_bool(
            "json_backend_parity.generated_json_schema_output_changed",
            self.generated_json_schema_output_changed,
            false,
        )?;
        require_bool(
            "json_backend_parity.existing_generated_templates_changed",
            self.existing_generated_templates_changed,
            false,
        )?;
        require_bool(
            "json_backend_parity.requires_quality_check_before_route",
            self.requires_quality_check_before_route,
            true,
        )?;
        require_bool(
            "json_backend_parity.requires_round_trip_smoke_before_route",
            self.requires_round_trip_smoke_before_route,
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
