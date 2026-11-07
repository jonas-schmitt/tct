//! Sidecar-only consumed-token accounting routing gate.
//!
//! This module pins the consumed-token and partial-result accounting a future
//! syntax route must preserve after strict/recovery decode behavior is fixed.
//! It does not register a syntax route in `TctCompiler`.

use crate::syntax_artifact_envelope::{
    ARTIFACT_KIND as ENVELOPE_ARTIFACT_KIND, FIXTURE_PATH as ENVELOPE_FIXTURE_PATH,
    SyntaxGeneratedArtifactEnvelope, VERSION as ENVELOPE_VERSION,
};
use crate::syntax_backend_boundary::{SyntaxBackendBoundaryError, SyntaxBackendCompilerBoundary};
use crate::syntax_strict_recovery_decode_routing::{
    ARTIFACT_KIND as STRICT_RECOVERY_ARTIFACT_KIND,
    COMPLETED_PROMOTION_CLASS as STRICT_RECOVERY_COMPLETED_PROMOTION_CLASS,
    FIXTURE_PATH as STRICT_RECOVERY_FIXTURE_PATH,
    NEXT_MISSING_STABLE_PROMOTION_CLASS as STRICT_RECOVERY_NEXT_MISSING_STABLE_PROMOTION_CLASS,
    SyntaxStrictRecoveryDecodeRoutingGate, VERSION as STRICT_RECOVERY_VERSION,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const TOOL: &str = "syntax_backend_consumed_token_accounting_routing_gate";
pub const VERSION: u32 = 1;
pub const ARTIFACT_KIND: &str = "tct_syntax_backend_consumed_token_accounting_routing_gate";
pub const FIXTURE_PATH: &str =
    "tests/fixtures/syntax_backend/consumed-token-accounting-routing-gate-v1.json";
pub const STATUS: &str = "sidecar_contract_only";
pub const COMPLETED_PROMOTION_CLASS: &str = "syntax backend consumed-token accounting routing gate";
pub const NEXT_MISSING_STABLE_PROMOTION_CLASS: &str =
    "syntax backend token/vocabulary/BPE routing gate";

const REQUIRED_TARGETS: &[&str] = &["toy", "csharp", "go"];
const REQUIRED_PARSER_BACKED_TARGETS: &[&str] = &["csharp", "go"];
const REQUIRED_ACCOUNTING_CASES: &[&str] = &[
    "complete_stream",
    "trailing_pad_recovery",
    "eof",
    "non_pad_surplus",
    "malformed_structural_tokens",
    "malformed_payload_tokens",
    "invalid_parser_backed_payloads",
    "partial_prefix_failures",
];
const REQUIRED_ACCOUNTING_FIELDS: &[&str] = &[
    "consumed",
    "ignored_trailing_tokens",
    "partial_fields",
    "partial_statements",
];
const REQUIRED_GENERATED_CODEC_TEST_MARKERS: &[&str] = &[
    "strict_decode_accepts_complete_assignment",
    "strict_decode_reports_consumed_prefix_for_partial_result_errors",
    "recovery_policy_accepts_strict_success",
    "recovery_policy_normalizes_trailing_padding",
    "recovery_policy_rejects_non_padding_surplus",
    "recovery_policy_keeps_eof_and_malformed_payloads_hard_errors",
    "recovery_policy_rejects_partial_prefix_errors_without_result",
    "generated_csharp_minimal_class_round_trips",
    "generated_csharp_strict_decode_rejects_eof_with_accounting",
    "generated_csharp_strict_decode_rejects_trailing_tokens",
    "generated_csharp_recovery_normalizes_trailing_pad_only",
    "generated_csharp_recovery_keeps_partial_prefix_hard",
    "generated_go_minimal_function_round_trips",
    "generated_go_strict_decode_rejects_eof_with_accounting",
    "generated_go_strict_decode_rejects_trailing_tokens",
    "generated_go_decode_rejects_invalid_identifier_payloads_with_accounting",
    "generated_go_recovery_normalizes_trailing_pad_only",
    "generated_go_recovery_keeps_partial_prefix_hard",
];
const REQUIRED_TARGET_AGNOSTIC_COMPILER_ROUTE_EVIDENCE: &[&str] = &[
    "csharp_minimal_class_syntax_v0",
    "csharp_local_string_return_literal_syntax_v0",
    "go_minimal_function_syntax_v0",
    "go_local_declaration_syntax_v0",
    "generated_go_minimal_function_real_source_strings_compare_demo",
];
const REQUIRED_BLOCKERS: &[&str] = &[
    "token/vocabulary/BPE invariants",
    "reproducibility",
    "audit coverage",
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxConsumedTokenAccountingRoutingGate {
    pub artifact_kind: String,
    pub version: u32,
    pub tool: String,
    pub status: String,
    pub source_strict_recovery_decode_routing_gate:
        SyntaxConsumedTokenSourceStrictRecoveryDecodeRoutingGate,
    pub source_generated_artifact_envelope: SyntaxConsumedTokenSourceGeneratedArtifactEnvelope,
    pub compiler_boundary: SyntaxBackendCompilerBoundary,
    pub route_state: SyntaxConsumedTokenRouteState,
    pub accounting_contract: SyntaxConsumedTokenAccountingRouteContract,
    pub generated_codec_evidence: SyntaxConsumedTokenGeneratedCodecEvidence,
    pub blockers: Vec<String>,
    pub completed_promotion_class: String,
    pub next_missing_stable_promotion_class: String,
}

impl SyntaxConsumedTokenAccountingRoutingGate {
    pub fn validate_against_strict_recovery_and_envelope(
        &self,
        strict_recovery_gate: &SyntaxStrictRecoveryDecodeRoutingGate,
        generated_artifact_envelope: &SyntaxGeneratedArtifactEnvelope,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        require_string("artifact_kind", &self.artifact_kind, ARTIFACT_KIND)?;
        require_u32("version", self.version, VERSION)?;
        require_string("tool", &self.tool, TOOL)?;
        require_string("status", &self.status, STATUS)?;
        self.source_strict_recovery_decode_routing_gate
            .validate(strict_recovery_gate)?;
        self.source_generated_artifact_envelope
            .validate(generated_artifact_envelope)?;
        require_sidecar_boundary(&self.compiler_boundary)?;
        self.route_state.validate()?;
        self.accounting_contract.validate()?;
        self.generated_codec_evidence.validate()?;
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
pub struct SyntaxConsumedTokenSourceStrictRecoveryDecodeRoutingGate {
    pub artifact_kind: String,
    pub version: u32,
    pub module_path: String,
    pub fixture_path: String,
    pub test_path: String,
    pub gate_helper: String,
    pub completed_promotion_class: String,
}

impl SyntaxConsumedTokenSourceStrictRecoveryDecodeRoutingGate {
    fn validate(
        &self,
        strict_recovery_gate: &SyntaxStrictRecoveryDecodeRoutingGate,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        require_string(
            "source_strict_recovery_decode_routing_gate.artifact_kind",
            &self.artifact_kind,
            STRICT_RECOVERY_ARTIFACT_KIND,
        )?;
        require_u32(
            "source_strict_recovery_decode_routing_gate.version",
            self.version,
            STRICT_RECOVERY_VERSION,
        )?;
        require_string(
            "source_strict_recovery_decode_routing_gate.module_path",
            &self.module_path,
            "src/syntax_strict_recovery_decode_routing.rs",
        )?;
        require_string(
            "source_strict_recovery_decode_routing_gate.fixture_path",
            &self.fixture_path,
            STRICT_RECOVERY_FIXTURE_PATH,
        )?;
        require_string(
            "source_strict_recovery_decode_routing_gate.test_path",
            &self.test_path,
            "tests/syntax_backend_boundary_test.rs",
        )?;
        require_string(
            "source_strict_recovery_decode_routing_gate.gate_helper",
            &self.gate_helper,
            "./scripts/testing/test-syntax-backend-gates.sh",
        )?;
        require_string(
            "source_strict_recovery_decode_routing_gate.completed_promotion_class",
            &self.completed_promotion_class,
            STRICT_RECOVERY_COMPLETED_PROMOTION_CLASS,
        )?;
        require_string(
            "source_strict_recovery_decode_routing_gate.source_fixture_completed_class",
            &strict_recovery_gate.completed_promotion_class,
            STRICT_RECOVERY_COMPLETED_PROMOTION_CLASS,
        )?;
        require_string(
            "source_strict_recovery_decode_routing_gate.source_fixture_next_missing_class",
            &strict_recovery_gate.next_missing_stable_promotion_class,
            STRICT_RECOVERY_NEXT_MISSING_STABLE_PROMOTION_CLASS,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxConsumedTokenSourceGeneratedArtifactEnvelope {
    pub artifact_kind: String,
    pub version: u32,
    pub module_path: String,
    pub fixture_path: String,
    pub test_path: String,
    pub gate_helper: String,
    pub decode_policy_source: String,
}

impl SyntaxConsumedTokenSourceGeneratedArtifactEnvelope {
    fn validate(
        &self,
        generated_artifact_envelope: &SyntaxGeneratedArtifactEnvelope,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        require_string(
            "source_generated_artifact_envelope.artifact_kind",
            &self.artifact_kind,
            ENVELOPE_ARTIFACT_KIND,
        )?;
        require_u32(
            "source_generated_artifact_envelope.version",
            self.version,
            ENVELOPE_VERSION,
        )?;
        require_string(
            "source_generated_artifact_envelope.module_path",
            &self.module_path,
            "src/syntax_artifact_envelope.rs",
        )?;
        require_string(
            "source_generated_artifact_envelope.fixture_path",
            &self.fixture_path,
            ENVELOPE_FIXTURE_PATH,
        )?;
        require_string(
            "source_generated_artifact_envelope.test_path",
            &self.test_path,
            "tests/syntax_backend_boundary_test.rs",
        )?;
        require_string(
            "source_generated_artifact_envelope.gate_helper",
            &self.gate_helper,
            "./scripts/testing/test-syntax-backend-gates.sh",
        )?;
        require_string(
            "source_generated_artifact_envelope.decode_policy_source",
            &self.decode_policy_source,
            "generated artifact envelope decode_policy",
        )?;
        require_bool(
            "source_generated_artifact_envelope.consumed_token_accounting_required",
            generated_artifact_envelope
                .decode_policy
                .consumed_token_accounting_required,
            true,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxConsumedTokenRouteState {
    pub route_registered: bool,
    pub route_enabled: bool,
    pub public_api_changed: bool,
    pub uses_json_schema_compiler: bool,
    pub uses_syntax_renderer_api: bool,
}

impl SyntaxConsumedTokenRouteState {
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
pub struct SyntaxConsumedTokenAccountingRouteContract {
    pub accounting_cases: Vec<String>,
    pub result_fields: Vec<String>,
    pub strict_success_consumes_entire_stream: bool,
    pub recovery_trailing_pad_reports_ignored_tokens: bool,
    pub hard_errors_report_failure_position: bool,
    pub partial_prefix_errors_report_partial_count: bool,
    pub does_not_default_partial_prefixes: bool,
    pub does_not_promote_prefix_completion: bool,
}

impl SyntaxConsumedTokenAccountingRouteContract {
    fn validate(&self) -> Result<(), SyntaxBackendBoundaryError> {
        require_all(
            "accounting_contract.accounting_cases",
            &self.accounting_cases,
            REQUIRED_ACCOUNTING_CASES,
        )?;
        require_all(
            "accounting_contract.result_fields",
            &self.result_fields,
            REQUIRED_ACCOUNTING_FIELDS,
        )?;
        require_bool(
            "accounting_contract.strict_success_consumes_entire_stream",
            self.strict_success_consumes_entire_stream,
            true,
        )?;
        require_bool(
            "accounting_contract.recovery_trailing_pad_reports_ignored_tokens",
            self.recovery_trailing_pad_reports_ignored_tokens,
            true,
        )?;
        require_bool(
            "accounting_contract.hard_errors_report_failure_position",
            self.hard_errors_report_failure_position,
            true,
        )?;
        require_bool(
            "accounting_contract.partial_prefix_errors_report_partial_count",
            self.partial_prefix_errors_report_partial_count,
            true,
        )?;
        require_bool(
            "accounting_contract.does_not_default_partial_prefixes",
            self.does_not_default_partial_prefixes,
            true,
        )?;
        require_bool(
            "accounting_contract.does_not_promote_prefix_completion",
            self.does_not_promote_prefix_completion,
            true,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxConsumedTokenGeneratedCodecEvidence {
    pub test_path: String,
    pub renderer_api_module: String,
    pub target_ids: Vec<String>,
    pub parser_backed_target_ids: Vec<String>,
    pub target_agnostic_compiler_route_metadata_field: String,
    pub target_agnostic_compiler_route_evidence: Vec<String>,
    pub required_test_markers: Vec<String>,
    pub strict_recovery_gate_fixture_confirmed: bool,
    pub generated_artifact_envelope_decode_policy_confirmed: bool,
}

impl SyntaxConsumedTokenGeneratedCodecEvidence {
    fn validate(&self) -> Result<(), SyntaxBackendBoundaryError> {
        require_string(
            "generated_codec_evidence.test_path",
            &self.test_path,
            "tests/syntax_generated_codec_gate_test.rs",
        )?;
        require_string(
            "generated_codec_evidence.renderer_api_module",
            &self.renderer_api_module,
            "src/syntax_generated_codec.rs",
        )?;
        require_all(
            "generated_codec_evidence.target_ids",
            &self.target_ids,
            REQUIRED_TARGETS,
        )?;
        require_all(
            "generated_codec_evidence.parser_backed_target_ids",
            &self.parser_backed_target_ids,
            REQUIRED_PARSER_BACKED_TARGETS,
        )?;
        require_string(
            "generated_codec_evidence.target_agnostic_compiler_route_metadata_field",
            &self.target_agnostic_compiler_route_metadata_field,
            "target_agnostic_compiler_route",
        )?;
        require_all(
            "generated_codec_evidence.target_agnostic_compiler_route_evidence",
            &self.target_agnostic_compiler_route_evidence,
            REQUIRED_TARGET_AGNOSTIC_COMPILER_ROUTE_EVIDENCE,
        )?;
        require_all(
            "generated_codec_evidence.required_test_markers",
            &self.required_test_markers,
            REQUIRED_GENERATED_CODEC_TEST_MARKERS,
        )?;
        require_bool(
            "generated_codec_evidence.strict_recovery_gate_fixture_confirmed",
            self.strict_recovery_gate_fixture_confirmed,
            true,
        )?;
        require_bool(
            "generated_codec_evidence.generated_artifact_envelope_decode_policy_confirmed",
            self.generated_artifact_envelope_decode_policy_confirmed,
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
