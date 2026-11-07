//! Sidecar-only audit coverage routing gate.
//!
//! This module pins the maintained audit inventory a future syntax route must
//! preserve after reproducibility routing is fixed. It does not register a
//! syntax route in `TctCompiler`.

use crate::syntax_backend_boundary::{
    GATE_HELPER_PATH, SyntaxBackendBoundaryError, SyntaxBackendCompilerBoundary,
};
use crate::syntax_reproducibility_routing::{
    ARTIFACT_KIND as REPRODUCIBILITY_ROUTING_ARTIFACT_KIND,
    COMPLETED_PROMOTION_CLASS as REPRODUCIBILITY_ROUTING_COMPLETED_PROMOTION_CLASS,
    FIXTURE_PATH as REPRODUCIBILITY_ROUTING_FIXTURE_PATH,
    NEXT_MISSING_STABLE_PROMOTION_CLASS as REPRODUCIBILITY_ROUTING_NEXT_MISSING_STABLE_PROMOTION_CLASS,
    SyntaxReproducibilityRoutingGate, VERSION as REPRODUCIBILITY_ROUTING_VERSION,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const TOOL: &str = "syntax_backend_audit_coverage_routing_gate";
pub const VERSION: u32 = 1;
pub const ARTIFACT_KIND: &str = "tct_syntax_backend_audit_coverage_routing_gate";
pub const FIXTURE_PATH: &str = "tests/fixtures/syntax_backend/audit-coverage-routing-gate-v1.json";
pub const STATUS: &str = "sidecar_contract_only";
pub const COMPLETED_PROMOTION_CLASS: &str = "syntax backend audit coverage routing gate";
pub const NEXT_MISSING_STABLE_PROMOTION_CLASS: &str = "syntax backend stable promotion review gate";

const REQUIRED_DOCUMENTS: &[&str] = &[
    "docs/syntax-backend-gate-strategy.md",
    "docs/syntax-structural-model-stable-gate-plan.md",
    "docs/structural-model-boundary-selection.md",
    "reports/m6-cross-target-contract-closure-2026-07-21.md",
    "TODO.md",
];
const REQUIRED_HELPERS: &[&str] = &[
    "scripts/testing/test-quality-audit.py",
    "scripts/testing/test-syntax-backend-gates.sh",
    "tests/syntax_backend_boundary_test.rs",
];
const REQUIRED_ROUTE_MODULES: &[&str] = &[
    "src/syntax_tct_compiler_routing.rs",
    "src/syntax_json_backend_parity.rs",
    "src/syntax_target_agnostic_routing_boundary.rs",
    "src/syntax_strict_recovery_decode_routing.rs",
    "src/syntax_consumed_token_accounting_routing.rs",
    "src/syntax_token_vocabulary_bpe_routing.rs",
    "src/syntax_reproducibility_routing.rs",
];
const REQUIRED_ROUTE_FIXTURES: &[&str] = &[
    "tests/fixtures/syntax_backend/tct-compiler-routing-gate-v1.json",
    "tests/fixtures/syntax_backend/json-backend-parity-routing-gate-v1.json",
    "tests/fixtures/syntax_backend/target-agnostic-routing-boundary-gate-v1.json",
    "tests/fixtures/syntax_backend/strict-recovery-decode-routing-gate-v1.json",
    "tests/fixtures/syntax_backend/consumed-token-accounting-routing-gate-v1.json",
    "tests/fixtures/syntax_backend/token-vocabulary-bpe-routing-gate-v1.json",
    "tests/fixtures/syntax_backend/reproducibility-routing-gate-v1.json",
];
const REQUIRED_AUDIT_FIELDS: &[&str] = &[
    "strategy_doc",
    "stable_gate_plan_doc",
    "promotion_readiness_report",
    "current_missing_stable_promotion_classes",
    "validation_gaps",
    "generated_codec_gate_command",
    "tct_compiler_routing_module",
    "json_backend_parity_module",
    "target_agnostic_routing_boundary_module",
    "strict_recovery_decode_routing_module",
    "consumed_token_accounting_routing_module",
    "token_vocabulary_bpe_routing_module",
    "reproducibility_routing_module",
];
const REQUIRED_NO_ROUTE_CLAIMS: &[&str] = &[
    "no_stable_tct_compiler_route",
    "no_generated_json_schema_output_change",
    "no_core_compiler_path_change",
    "no_generated_source_substring_dependency",
];
const REQUIRED_BLOCKERS: &[&str] = &["stable promotion review"];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxAuditCoverageRoutingGate {
    pub artifact_kind: String,
    pub version: u32,
    pub tool: String,
    pub status: String,
    pub source_reproducibility_routing_gate: SyntaxAuditSourceReproducibilityRoutingGate,
    pub compiler_boundary: SyntaxBackendCompilerBoundary,
    pub route_state: SyntaxAuditCoverageRouteState,
    pub audit_contract: SyntaxAuditCoverageRouteContract,
    pub blockers: Vec<String>,
    pub completed_promotion_class: String,
    pub next_missing_stable_promotion_class: String,
}

impl SyntaxAuditCoverageRoutingGate {
    pub fn validate_against_reproducibility_gate(
        &self,
        reproducibility_gate: &SyntaxReproducibilityRoutingGate,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        require_string("artifact_kind", &self.artifact_kind, ARTIFACT_KIND)?;
        require_u32("version", self.version, VERSION)?;
        require_string("tool", &self.tool, TOOL)?;
        require_string("status", &self.status, STATUS)?;
        self.source_reproducibility_routing_gate
            .validate(reproducibility_gate)?;
        require_sidecar_boundary(&self.compiler_boundary)?;
        self.route_state.validate()?;
        self.audit_contract.validate()?;
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
pub struct SyntaxAuditSourceReproducibilityRoutingGate {
    pub artifact_kind: String,
    pub version: u32,
    pub module_path: String,
    pub fixture_path: String,
    pub test_path: String,
    pub gate_helper: String,
    pub completed_promotion_class: String,
}

impl SyntaxAuditSourceReproducibilityRoutingGate {
    fn validate(
        &self,
        reproducibility_gate: &SyntaxReproducibilityRoutingGate,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        require_string(
            "source_reproducibility_routing_gate.artifact_kind",
            &self.artifact_kind,
            REPRODUCIBILITY_ROUTING_ARTIFACT_KIND,
        )?;
        require_u32(
            "source_reproducibility_routing_gate.version",
            self.version,
            REPRODUCIBILITY_ROUTING_VERSION,
        )?;
        require_string(
            "source_reproducibility_routing_gate.module_path",
            &self.module_path,
            "src/syntax_reproducibility_routing.rs",
        )?;
        require_string(
            "source_reproducibility_routing_gate.fixture_path",
            &self.fixture_path,
            REPRODUCIBILITY_ROUTING_FIXTURE_PATH,
        )?;
        require_string(
            "source_reproducibility_routing_gate.test_path",
            &self.test_path,
            "tests/syntax_backend_boundary_test.rs",
        )?;
        require_string(
            "source_reproducibility_routing_gate.gate_helper",
            &self.gate_helper,
            GATE_HELPER_PATH,
        )?;
        require_string(
            "source_reproducibility_routing_gate.completed_promotion_class",
            &self.completed_promotion_class,
            REPRODUCIBILITY_ROUTING_COMPLETED_PROMOTION_CLASS,
        )?;
        require_string(
            "source_reproducibility_routing_gate.source_fixture_completed_class",
            &reproducibility_gate.completed_promotion_class,
            REPRODUCIBILITY_ROUTING_COMPLETED_PROMOTION_CLASS,
        )?;
        require_string(
            "source_reproducibility_routing_gate.source_fixture_next_missing_class",
            &reproducibility_gate.next_missing_stable_promotion_class,
            REPRODUCIBILITY_ROUTING_NEXT_MISSING_STABLE_PROMOTION_CLASS,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxAuditCoverageRouteState {
    pub route_registered: bool,
    pub route_enabled: bool,
    pub public_api_changed: bool,
    pub uses_json_schema_compiler: bool,
    pub uses_syntax_renderer_api: bool,
}

impl SyntaxAuditCoverageRouteState {
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
pub struct SyntaxAuditCoverageRouteContract {
    pub required_documents: Vec<String>,
    pub required_helpers: Vec<String>,
    pub required_route_modules: Vec<String>,
    pub required_route_fixtures: Vec<String>,
    pub required_audit_fields: Vec<String>,
    pub no_route_claims: Vec<String>,
    pub current_missing_stable_promotion_class: String,
    pub fail_on_gaps_required: bool,
    pub route_inventory_is_report_only: bool,
    pub does_not_depend_on_generated_source_substrings: bool,
}

impl SyntaxAuditCoverageRouteContract {
    fn validate(&self) -> Result<(), SyntaxBackendBoundaryError> {
        require_all(
            "audit_contract.required_documents",
            &self.required_documents,
            REQUIRED_DOCUMENTS,
        )?;
        require_all(
            "audit_contract.required_helpers",
            &self.required_helpers,
            REQUIRED_HELPERS,
        )?;
        require_all(
            "audit_contract.required_route_modules",
            &self.required_route_modules,
            REQUIRED_ROUTE_MODULES,
        )?;
        require_all(
            "audit_contract.required_route_fixtures",
            &self.required_route_fixtures,
            REQUIRED_ROUTE_FIXTURES,
        )?;
        require_all(
            "audit_contract.required_audit_fields",
            &self.required_audit_fields,
            REQUIRED_AUDIT_FIELDS,
        )?;
        require_all(
            "audit_contract.no_route_claims",
            &self.no_route_claims,
            REQUIRED_NO_ROUTE_CLAIMS,
        )?;
        require_string(
            "audit_contract.current_missing_stable_promotion_class",
            &self.current_missing_stable_promotion_class,
            COMPLETED_PROMOTION_CLASS,
        )?;
        require_bool(
            "audit_contract.fail_on_gaps_required",
            self.fail_on_gaps_required,
            true,
        )?;
        require_bool(
            "audit_contract.route_inventory_is_report_only",
            self.route_inventory_is_report_only,
            true,
        )?;
        require_bool(
            "audit_contract.does_not_depend_on_generated_source_substrings",
            self.does_not_depend_on_generated_source_substrings,
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
