//! Sidecar-only syntax JSON-backend parity routing gate.
//!
//! This module pins the stable JSON Schema backend checks that must stay green
//! before a future syntax route can call the sidecar generated-code renderer.
//! It does not register a syntax route in `TctCompiler`.

use crate::syntax_backend_boundary::{SyntaxBackendBoundaryError, SyntaxBackendCompilerBoundary};
use crate::syntax_tct_compiler_routing::{
    ARTIFACT_KIND as ROUTING_ARTIFACT_KIND,
    COMPLETED_PROMOTION_CLASS as ROUTING_COMPLETED_PROMOTION_CLASS,
    FIXTURE_PATH as ROUTING_FIXTURE_PATH, SyntaxTctCompilerRoutingGate, VERSION as ROUTING_VERSION,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const TOOL: &str = "syntax_backend_json_backend_parity_routing_gate";
pub const VERSION: u32 = 1;
pub const ARTIFACT_KIND: &str = "tct_syntax_backend_json_backend_parity_routing_gate";
pub const FIXTURE_PATH: &str =
    "tests/fixtures/syntax_backend/json-backend-parity-routing-gate-v1.json";
pub const STATUS: &str = "sidecar_contract_only";
pub const COMPLETED_PROMOTION_CLASS: &str = "syntax backend JSON-backend parity routing gate";
pub const NEXT_MISSING_STABLE_PROMOTION_CLASS: &str =
    "syntax backend target-agnostic routing boundary gate";

const REQUIRED_BACKEND_SURFACES: &[&str] = &[
    "schema_compilation",
    "generated_code_compilation",
    "token_encoding",
    "token_decoding",
    "data_validation",
    "vocabulary_accounting",
    "bpe_integration",
    "maintained_docs",
    "stable_gate_commands",
];
const REQUIRED_COMMANDS: &[&str] = &[
    "./scripts/testing/quality-check.sh",
    "cargo test --test round_trip_test test_round_trip",
    "python3 scripts/testing/test-quality-audit.py --fail-on-gaps",
];
const REQUIRED_BLOCKERS: &[&str] = &[
    "target-agnostic syntax boundary selection",
    "strict/recovery decode behavior",
    "consumed-token accounting",
    "token/vocabulary/BPE invariants",
    "reproducibility",
    "audit coverage",
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxJsonBackendParityGate {
    pub artifact_kind: String,
    pub version: u32,
    pub tool: String,
    pub status: String,
    pub source_routing_gate: SyntaxJsonBackendParitySourceRoutingGate,
    pub compiler_boundary: SyntaxBackendCompilerBoundary,
    pub route_state: SyntaxJsonBackendParityRouteState,
    pub stable_json_backend: SyntaxJsonBackendStableContract,
    pub stable_gate_commands: Vec<SyntaxJsonBackendStableGateCommand>,
    pub blockers: Vec<String>,
    pub completed_promotion_class: String,
    pub next_missing_stable_promotion_class: String,
}

impl SyntaxJsonBackendParityGate {
    pub fn validate_against_routing_gate(
        &self,
        routing_gate: &SyntaxTctCompilerRoutingGate,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        require_string("artifact_kind", &self.artifact_kind, ARTIFACT_KIND)?;
        require_u32("version", self.version, VERSION)?;
        require_string("tool", &self.tool, TOOL)?;
        require_string("status", &self.status, STATUS)?;
        self.source_routing_gate.validate(routing_gate)?;
        require_sidecar_boundary(&self.compiler_boundary)?;
        self.route_state.validate()?;
        self.stable_json_backend.validate()?;
        validate_stable_gate_commands(&self.stable_gate_commands)?;
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
pub struct SyntaxJsonBackendParitySourceRoutingGate {
    pub artifact_kind: String,
    pub version: u32,
    pub module_path: String,
    pub fixture_path: String,
    pub test_path: String,
    pub gate_helper: String,
    pub completed_promotion_class: String,
}

impl SyntaxJsonBackendParitySourceRoutingGate {
    fn validate(
        &self,
        routing_gate: &SyntaxTctCompilerRoutingGate,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        require_string(
            "source_routing_gate.artifact_kind",
            &self.artifact_kind,
            ROUTING_ARTIFACT_KIND,
        )?;
        require_u32("source_routing_gate.version", self.version, ROUTING_VERSION)?;
        require_string(
            "source_routing_gate.module_path",
            &self.module_path,
            "src/syntax_tct_compiler_routing.rs",
        )?;
        require_string(
            "source_routing_gate.fixture_path",
            &self.fixture_path,
            ROUTING_FIXTURE_PATH,
        )?;
        require_string(
            "source_routing_gate.test_path",
            &self.test_path,
            "tests/syntax_backend_boundary_test.rs",
        )?;
        require_string(
            "source_routing_gate.gate_helper",
            &self.gate_helper,
            "./scripts/testing/test-syntax-backend-gates.sh",
        )?;
        require_string(
            "source_routing_gate.completed_promotion_class",
            &self.completed_promotion_class,
            ROUTING_COMPLETED_PROMOTION_CLASS,
        )?;
        require_string(
            "source_routing_gate.routing_fixture_class",
            &routing_gate.completed_promotion_class,
            ROUTING_COMPLETED_PROMOTION_CLASS,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxJsonBackendParityRouteState {
    pub route_registered: bool,
    pub route_enabled: bool,
    pub public_api_changed: bool,
    pub uses_json_schema_compiler: bool,
    pub uses_syntax_renderer_api: bool,
}

impl SyntaxJsonBackendParityRouteState {
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
pub struct SyntaxJsonBackendStableContract {
    pub backend_surfaces: Vec<String>,
    pub schema_compilation_unchanged: bool,
    pub generated_code_compilation_unchanged: bool,
    pub token_encoding_unchanged: bool,
    pub token_decoding_unchanged: bool,
    pub data_validation_unchanged: bool,
    pub vocabulary_accounting_unchanged: bool,
    pub bpe_integration_unchanged: bool,
    pub maintained_docs_unchanged: bool,
    pub generated_json_schema_output_changed: bool,
    pub existing_generated_templates_changed: bool,
}

impl SyntaxJsonBackendStableContract {
    fn validate(&self) -> Result<(), SyntaxBackendBoundaryError> {
        require_all(
            "stable_json_backend.backend_surfaces",
            &self.backend_surfaces,
            REQUIRED_BACKEND_SURFACES,
        )?;
        require_bool(
            "stable_json_backend.schema_compilation_unchanged",
            self.schema_compilation_unchanged,
            true,
        )?;
        require_bool(
            "stable_json_backend.generated_code_compilation_unchanged",
            self.generated_code_compilation_unchanged,
            true,
        )?;
        require_bool(
            "stable_json_backend.token_encoding_unchanged",
            self.token_encoding_unchanged,
            true,
        )?;
        require_bool(
            "stable_json_backend.token_decoding_unchanged",
            self.token_decoding_unchanged,
            true,
        )?;
        require_bool(
            "stable_json_backend.data_validation_unchanged",
            self.data_validation_unchanged,
            true,
        )?;
        require_bool(
            "stable_json_backend.vocabulary_accounting_unchanged",
            self.vocabulary_accounting_unchanged,
            true,
        )?;
        require_bool(
            "stable_json_backend.bpe_integration_unchanged",
            self.bpe_integration_unchanged,
            true,
        )?;
        require_bool(
            "stable_json_backend.maintained_docs_unchanged",
            self.maintained_docs_unchanged,
            true,
        )?;
        require_bool(
            "stable_json_backend.generated_json_schema_output_changed",
            self.generated_json_schema_output_changed,
            false,
        )?;
        require_bool(
            "stable_json_backend.existing_generated_templates_changed",
            self.existing_generated_templates_changed,
            false,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxJsonBackendStableGateCommand {
    pub command: String,
    pub proves: Vec<String>,
}

fn validate_stable_gate_commands(
    commands: &[SyntaxJsonBackendStableGateCommand],
) -> Result<(), SyntaxBackendBoundaryError> {
    let actual = commands
        .iter()
        .map(|command| command.command.as_str())
        .collect::<BTreeSet<_>>();
    for required in REQUIRED_COMMANDS {
        if !actual.contains(required) {
            return Err(SyntaxBackendBoundaryError::new(
                "stable_gate_commands",
                format!("missing required command {required}"),
            ));
        }
    }
    for command in commands {
        if command.proves.is_empty() {
            return Err(SyntaxBackendBoundaryError::new(
                "stable_gate_commands.proves",
                format!("command {} has no proof surface", command.command),
            ));
        }
    }
    Ok(())
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
