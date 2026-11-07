//! Sidecar-only target-agnostic syntax routing boundary gate.
//!
//! This module pins the route input shape a future syntax backend may accept:
//! checked `GrammarIR -> StructuralModel` evidence shared across targets. It
//! deliberately rejects target-specific arena IR shapes and does not register a
//! route in `TctCompiler`. The source manifest is
//! `tests/fixtures/syntax_structural_model/boundary-manifest-v1.json`.

use crate::grammar_ir_structural_model::MODEL_VERSION;
use crate::syntax_backend_boundary::{
    BOUNDARY_MANIFEST_ARTIFACT_KIND, BOUNDARY_MANIFEST_PATH, SyntaxBackendBoundaryError,
    SyntaxBackendCompilerBoundary,
};
use crate::syntax_json_backend_parity::{
    ARTIFACT_KIND as JSON_BACKEND_PARITY_ARTIFACT_KIND,
    COMPLETED_PROMOTION_CLASS as JSON_BACKEND_PARITY_COMPLETED_PROMOTION_CLASS,
    FIXTURE_PATH as JSON_BACKEND_PARITY_FIXTURE_PATH,
    NEXT_MISSING_STABLE_PROMOTION_CLASS as JSON_BACKEND_PARITY_NEXT_MISSING_STABLE_PROMOTION_CLASS,
    SyntaxJsonBackendParityGate, VERSION as JSON_BACKEND_PARITY_VERSION,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const TOOL: &str = "syntax_backend_target_agnostic_routing_boundary_gate";
pub const VERSION: u32 = 1;
pub const ARTIFACT_KIND: &str = "tct_syntax_backend_target_agnostic_routing_boundary_gate";
pub const FIXTURE_PATH: &str =
    "tests/fixtures/syntax_backend/target-agnostic-routing-boundary-gate-v1.json";
pub const STATUS: &str = "sidecar_contract_only";
pub const COMPLETED_PROMOTION_CLASS: &str = "syntax backend target-agnostic routing boundary gate";
pub const NEXT_MISSING_STABLE_PROMOTION_CLASS: &str =
    "syntax backend strict/recovery decode routing gate";

const REQUIRED_TARGETS: &[&str] = &["toy", "csharp", "go"];
const REQUIRED_STRUCTURAL_CATEGORIES: &[&str] = &[
    "records",
    "finite_domains",
    "variants",
    "lists",
    "optionals",
    "references",
    "payload_leaves",
    "token_ranges",
    "renderer_metadata",
    "validation_hooks",
    "decision_graph_nodes",
    "decision_graph_edges",
    "source_provenance",
    "parser_provenance",
];
const REQUIRED_PRE_ROUTE_EVIDENCE: &[&str] = &[
    "stable_json_backend_parity",
    "target_agnostic_structural_model_manifest",
    "strict_recovery_decode_policy",
    "consumed_token_accounting",
    "token_vocabulary_bpe_invariants",
    "generated_artifact_reproducibility",
    "audit_markers",
];
const REQUIRED_BLOCKERS: &[&str] = &[
    "strict/recovery decode behavior",
    "consumed-token accounting",
    "token/vocabulary/BPE invariants",
    "reproducibility",
    "audit coverage",
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxTargetAgnosticRoutingBoundaryGate {
    pub artifact_kind: String,
    pub version: u32,
    pub tool: String,
    pub status: String,
    pub source_json_backend_parity_gate: SyntaxTargetRouteSourceJsonBackendParityGate,
    pub source_structural_model_manifest: SyntaxTargetRouteStructuralModelManifest,
    pub compiler_boundary: SyntaxBackendCompilerBoundary,
    pub route_state: SyntaxTargetRouteState,
    pub accepted_route_input: SyntaxTargetRouteInput,
    pub routing_handoff: SyntaxTargetRoutingHandoff,
    pub blockers: Vec<String>,
    pub completed_promotion_class: String,
    pub next_missing_stable_promotion_class: String,
}

impl SyntaxTargetAgnosticRoutingBoundaryGate {
    pub fn validate_against_json_backend_parity_gate(
        &self,
        json_backend_parity_gate: &SyntaxJsonBackendParityGate,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        require_string("artifact_kind", &self.artifact_kind, ARTIFACT_KIND)?;
        require_u32("version", self.version, VERSION)?;
        require_string("tool", &self.tool, TOOL)?;
        require_string("status", &self.status, STATUS)?;
        self.source_json_backend_parity_gate
            .validate(json_backend_parity_gate)?;
        self.source_structural_model_manifest.validate()?;
        require_sidecar_boundary(&self.compiler_boundary)?;
        self.route_state.validate()?;
        self.accepted_route_input.validate()?;
        self.routing_handoff.validate()?;
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
pub struct SyntaxTargetRouteSourceJsonBackendParityGate {
    pub artifact_kind: String,
    pub version: u32,
    pub module_path: String,
    pub fixture_path: String,
    pub test_path: String,
    pub gate_helper: String,
    pub completed_promotion_class: String,
}

impl SyntaxTargetRouteSourceJsonBackendParityGate {
    fn validate(
        &self,
        json_backend_parity_gate: &SyntaxJsonBackendParityGate,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        require_string(
            "source_json_backend_parity_gate.artifact_kind",
            &self.artifact_kind,
            JSON_BACKEND_PARITY_ARTIFACT_KIND,
        )?;
        require_u32(
            "source_json_backend_parity_gate.version",
            self.version,
            JSON_BACKEND_PARITY_VERSION,
        )?;
        require_string(
            "source_json_backend_parity_gate.module_path",
            &self.module_path,
            "src/syntax_json_backend_parity.rs",
        )?;
        require_string(
            "source_json_backend_parity_gate.fixture_path",
            &self.fixture_path,
            JSON_BACKEND_PARITY_FIXTURE_PATH,
        )?;
        require_string(
            "source_json_backend_parity_gate.test_path",
            &self.test_path,
            "tests/syntax_backend_boundary_test.rs",
        )?;
        require_string(
            "source_json_backend_parity_gate.gate_helper",
            &self.gate_helper,
            "./scripts/testing/test-syntax-backend-gates.sh",
        )?;
        require_string(
            "source_json_backend_parity_gate.completed_promotion_class",
            &self.completed_promotion_class,
            JSON_BACKEND_PARITY_COMPLETED_PROMOTION_CLASS,
        )?;
        require_string(
            "source_json_backend_parity_gate.parity_fixture_completed_class",
            &json_backend_parity_gate.completed_promotion_class,
            JSON_BACKEND_PARITY_COMPLETED_PROMOTION_CLASS,
        )?;
        require_string(
            "source_json_backend_parity_gate.parity_fixture_next_missing_class",
            &json_backend_parity_gate.next_missing_stable_promotion_class,
            JSON_BACKEND_PARITY_NEXT_MISSING_STABLE_PROMOTION_CLASS,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxTargetRouteStructuralModelManifest {
    pub path: String,
    pub artifact_kind: String,
    pub version: u32,
    pub model_version: String,
    pub boundary: String,
    pub target_ids: Vec<String>,
    pub target_agnostic: bool,
    pub shared_structural_categories: Vec<String>,
}

impl SyntaxTargetRouteStructuralModelManifest {
    fn validate(&self) -> Result<(), SyntaxBackendBoundaryError> {
        require_string(
            "source_structural_model_manifest.path",
            &self.path,
            BOUNDARY_MANIFEST_PATH,
        )?;
        require_string(
            "source_structural_model_manifest.artifact_kind",
            &self.artifact_kind,
            BOUNDARY_MANIFEST_ARTIFACT_KIND,
        )?;
        require_u32("source_structural_model_manifest.version", self.version, 1)?;
        require_string(
            "source_structural_model_manifest.model_version",
            &self.model_version,
            MODEL_VERSION,
        )?;
        require_string(
            "source_structural_model_manifest.boundary",
            &self.boundary,
            "GrammarIR -> StructuralModel",
        )?;
        require_all(
            "source_structural_model_manifest.target_ids",
            &self.target_ids,
            REQUIRED_TARGETS,
        )?;
        require_bool(
            "source_structural_model_manifest.target_agnostic",
            self.target_agnostic,
            true,
        )?;
        require_all(
            "source_structural_model_manifest.shared_structural_categories",
            &self.shared_structural_categories,
            REQUIRED_STRUCTURAL_CATEGORIES,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxTargetRouteState {
    pub route_registered: bool,
    pub route_enabled: bool,
    pub public_api_changed: bool,
    pub uses_json_schema_compiler: bool,
    pub uses_syntax_renderer_api: bool,
}

impl SyntaxTargetRouteState {
    fn validate(&self) -> Result<(), SyntaxBackendBoundaryError> {
        require_bool("route_state.route_registered", self.route_registered, false)?;
        require_bool("route_state.route_enabled", self.route_enabled, false)?;
        require_bool(
            "route_state.public_api_changed",
            self.public_api_changed,
            false,
        )?;
        require_bool(
            "route_state.uses_json_schema_compiler",
            self.uses_json_schema_compiler,
            false,
        )?;
        require_bool(
            "route_state.uses_syntax_renderer_api",
            self.uses_syntax_renderer_api,
            false,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxTargetRouteInput {
    pub input_boundary: String,
    pub requires_checked_grammar_ir: bool,
    pub requires_structural_model: bool,
    pub target_agnostic: bool,
    pub rejects_goir_arena_ir: bool,
    pub rejects_csharp_arena_ir: bool,
    pub rejects_target_specific_schema_shape: bool,
    pub target_ids: Vec<String>,
    pub shared_structural_categories: Vec<String>,
}

impl SyntaxTargetRouteInput {
    fn validate(&self) -> Result<(), SyntaxBackendBoundaryError> {
        require_string(
            "accepted_route_input.input_boundary",
            &self.input_boundary,
            "checked GrammarIR -> StructuralModel",
        )?;
        require_bool(
            "accepted_route_input.requires_checked_grammar_ir",
            self.requires_checked_grammar_ir,
            true,
        )?;
        require_bool(
            "accepted_route_input.requires_structural_model",
            self.requires_structural_model,
            true,
        )?;
        require_bool(
            "accepted_route_input.target_agnostic",
            self.target_agnostic,
            true,
        )?;
        require_bool(
            "accepted_route_input.rejects_goir_arena_ir",
            self.rejects_goir_arena_ir,
            true,
        )?;
        require_bool(
            "accepted_route_input.rejects_csharp_arena_ir",
            self.rejects_csharp_arena_ir,
            true,
        )?;
        require_bool(
            "accepted_route_input.rejects_target_specific_schema_shape",
            self.rejects_target_specific_schema_shape,
            true,
        )?;
        require_all(
            "accepted_route_input.target_ids",
            &self.target_ids,
            REQUIRED_TARGETS,
        )?;
        require_all(
            "accepted_route_input.shared_structural_categories",
            &self.shared_structural_categories,
            REQUIRED_STRUCTURAL_CATEGORIES,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxTargetRoutingHandoff {
    pub accepted_input: String,
    pub renderer_api_module: String,
    pub output_artifact: String,
    pub required_pre_route_evidence: Vec<String>,
}

impl SyntaxTargetRoutingHandoff {
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
            "routing_handoff.required_pre_route_evidence",
            &self.required_pre_route_evidence,
            REQUIRED_PRE_ROUTE_EVIDENCE,
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
