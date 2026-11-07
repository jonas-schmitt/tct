//! Sidecar-only strict/recovery decode routing gate.
//!
//! This module pins the decode behavior a future syntax route must preserve at
//! the target-agnostic `GrammarIR -> StructuralModel` boundary. It does not
//! register a syntax route in `TctCompiler`.

use crate::syntax_artifact_envelope::{
    ARTIFACT_KIND as ENVELOPE_ARTIFACT_KIND, FIXTURE_PATH as ENVELOPE_FIXTURE_PATH,
    SyntaxGeneratedArtifactEnvelope, VERSION as ENVELOPE_VERSION,
};
use crate::syntax_backend_boundary::{SyntaxBackendBoundaryError, SyntaxBackendCompilerBoundary};
use crate::syntax_target_agnostic_routing_boundary::{
    ARTIFACT_KIND as TARGET_BOUNDARY_ARTIFACT_KIND,
    COMPLETED_PROMOTION_CLASS as TARGET_BOUNDARY_COMPLETED_PROMOTION_CLASS,
    FIXTURE_PATH as TARGET_BOUNDARY_FIXTURE_PATH,
    NEXT_MISSING_STABLE_PROMOTION_CLASS as TARGET_BOUNDARY_NEXT_MISSING_STABLE_PROMOTION_CLASS,
    SyntaxTargetAgnosticRoutingBoundaryGate, VERSION as TARGET_BOUNDARY_VERSION,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const TOOL: &str = "syntax_backend_strict_recovery_decode_routing_gate";
pub const VERSION: u32 = 1;
pub const ARTIFACT_KIND: &str = "tct_syntax_backend_strict_recovery_decode_routing_gate";
pub const FIXTURE_PATH: &str =
    "tests/fixtures/syntax_backend/strict-recovery-decode-routing-gate-v1.json";
pub const STATUS: &str = "sidecar_contract_only";
pub const COMPLETED_PROMOTION_CLASS: &str = "syntax backend strict/recovery decode routing gate";
pub const NEXT_MISSING_STABLE_PROMOTION_CLASS: &str =
    "syntax backend consumed-token accounting routing gate";

const REQUIRED_TARGETS: &[&str] = &["toy", "csharp", "go"];
const REQUIRED_PARSER_BACKED_TARGETS: &[&str] = &["csharp", "go"];
const REQUIRED_STRICT_FAILURES: &[&str] = &[
    "eof",
    "trailing_tokens",
    "malformed_structural_tokens",
    "malformed_payload_tokens",
    "invalid_parser_backed_payloads",
    "partial_prefix_failures",
    "non_admissible_marker_tokens",
];
const REQUIRED_RECOVERY_HARD_ERRORS: &[&str] = &[
    "eof",
    "non_pad_surplus",
    "malformed_structural_tokens",
    "malformed_payload_tokens",
    "invalid_parser_backed_payloads",
    "partial_prefix_failures",
];
const REQUIRED_GENERATED_CODEC_TEST_MARKERS: &[&str] = &[
    "strict_decode_rejects_eof",
    "strict_decode_rejects_trailing_tokens",
    "strict_decode_rejects_malformed_structural_tokens",
    "strict_decode_rejects_malformed_payload_tokens",
    "generated_csharp_strict_decode_rejects_eof_with_accounting",
    "generated_csharp_strict_decode_rejects_malformed_payload",
    "generated_go_strict_decode_rejects_eof_with_accounting",
    "generated_go_strict_decode_rejects_malformed_payload",
    "generated_go_decode_rejects_invalid_identifier_payloads_with_accounting",
    "recovery_policy_normalizes_trailing_padding",
    "recovery_policy_keeps_eof_and_malformed_payloads_hard_errors",
    "recovery_policy_rejects_partial_prefix_errors_without_result",
    "generated_csharp_recovery_normalizes_trailing_pad_only",
    "generated_csharp_recovery_keeps_partial_prefix_hard",
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
    "consumed-token accounting",
    "token/vocabulary/BPE invariants",
    "reproducibility",
    "audit coverage",
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxStrictRecoveryDecodeRoutingGate {
    pub artifact_kind: String,
    pub version: u32,
    pub tool: String,
    pub status: String,
    pub source_target_agnostic_routing_boundary_gate: SyntaxStrictRecoverySourceTargetBoundaryGate,
    pub source_generated_artifact_envelope: SyntaxStrictRecoverySourceGeneratedArtifactEnvelope,
    pub compiler_boundary: SyntaxBackendCompilerBoundary,
    pub route_state: SyntaxStrictRecoveryRouteState,
    pub strict_decode_contract: SyntaxStrictDecodeRouteContract,
    pub recovery_decode_contract: SyntaxRecoveryDecodeRouteContract,
    pub generated_codec_evidence: SyntaxStrictRecoveryGeneratedCodecEvidence,
    pub blockers: Vec<String>,
    pub completed_promotion_class: String,
    pub next_missing_stable_promotion_class: String,
}

impl SyntaxStrictRecoveryDecodeRoutingGate {
    pub fn validate_against_target_boundary_and_envelope(
        &self,
        target_boundary_gate: &SyntaxTargetAgnosticRoutingBoundaryGate,
        generated_artifact_envelope: &SyntaxGeneratedArtifactEnvelope,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        require_string("artifact_kind", &self.artifact_kind, ARTIFACT_KIND)?;
        require_u32("version", self.version, VERSION)?;
        require_string("tool", &self.tool, TOOL)?;
        require_string("status", &self.status, STATUS)?;
        self.source_target_agnostic_routing_boundary_gate
            .validate(target_boundary_gate)?;
        self.source_generated_artifact_envelope
            .validate(generated_artifact_envelope)?;
        require_sidecar_boundary(&self.compiler_boundary)?;
        self.route_state.validate()?;
        self.strict_decode_contract.validate()?;
        self.recovery_decode_contract.validate()?;
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
pub struct SyntaxStrictRecoverySourceTargetBoundaryGate {
    pub artifact_kind: String,
    pub version: u32,
    pub module_path: String,
    pub fixture_path: String,
    pub test_path: String,
    pub gate_helper: String,
    pub completed_promotion_class: String,
}

impl SyntaxStrictRecoverySourceTargetBoundaryGate {
    fn validate(
        &self,
        target_boundary_gate: &SyntaxTargetAgnosticRoutingBoundaryGate,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        require_string(
            "source_target_agnostic_routing_boundary_gate.artifact_kind",
            &self.artifact_kind,
            TARGET_BOUNDARY_ARTIFACT_KIND,
        )?;
        require_u32(
            "source_target_agnostic_routing_boundary_gate.version",
            self.version,
            TARGET_BOUNDARY_VERSION,
        )?;
        require_string(
            "source_target_agnostic_routing_boundary_gate.module_path",
            &self.module_path,
            "src/syntax_target_agnostic_routing_boundary.rs",
        )?;
        require_string(
            "source_target_agnostic_routing_boundary_gate.fixture_path",
            &self.fixture_path,
            TARGET_BOUNDARY_FIXTURE_PATH,
        )?;
        require_string(
            "source_target_agnostic_routing_boundary_gate.test_path",
            &self.test_path,
            "tests/syntax_backend_boundary_test.rs",
        )?;
        require_string(
            "source_target_agnostic_routing_boundary_gate.gate_helper",
            &self.gate_helper,
            "./scripts/testing/test-syntax-backend-gates.sh",
        )?;
        require_string(
            "source_target_agnostic_routing_boundary_gate.completed_promotion_class",
            &self.completed_promotion_class,
            TARGET_BOUNDARY_COMPLETED_PROMOTION_CLASS,
        )?;
        require_string(
            "source_target_agnostic_routing_boundary_gate.boundary_fixture_completed_class",
            &target_boundary_gate.completed_promotion_class,
            TARGET_BOUNDARY_COMPLETED_PROMOTION_CLASS,
        )?;
        require_string(
            "source_target_agnostic_routing_boundary_gate.boundary_fixture_next_missing_class",
            &target_boundary_gate.next_missing_stable_promotion_class,
            TARGET_BOUNDARY_NEXT_MISSING_STABLE_PROMOTION_CLASS,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxStrictRecoverySourceGeneratedArtifactEnvelope {
    pub artifact_kind: String,
    pub version: u32,
    pub module_path: String,
    pub fixture_path: String,
    pub test_path: String,
    pub gate_helper: String,
    pub decode_policy_source: String,
}

impl SyntaxStrictRecoverySourceGeneratedArtifactEnvelope {
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
            "source_generated_artifact_envelope.strict_decode_authoritative",
            generated_artifact_envelope
                .decode_policy
                .strict_decode_authoritative,
            true,
        )?;
        require_bool(
            "source_generated_artifact_envelope.recovery_normalizes_trailing_pad_only",
            generated_artifact_envelope
                .decode_policy
                .recovery_normalizes_trailing_pad_only,
            true,
        )?;
        require_bool(
            "source_generated_artifact_envelope.partial_prefix_failures_are_hard",
            generated_artifact_envelope
                .decode_policy
                .partial_prefix_failures_are_hard,
            true,
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
pub struct SyntaxStrictRecoveryRouteState {
    pub route_registered: bool,
    pub route_enabled: bool,
    pub public_api_changed: bool,
    pub uses_json_schema_compiler: bool,
    pub uses_syntax_renderer_api: bool,
}

impl SyntaxStrictRecoveryRouteState {
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
pub struct SyntaxStrictDecodeRouteContract {
    pub strict_decode_authoritative: bool,
    pub rejects_trailing_tokens: bool,
    pub transition_acceptance_equals_admissible_mask: bool,
    pub failure_kinds: Vec<String>,
    pub records_consumed_prefix_requirement: bool,
    pub records_partial_result_requirement: bool,
    pub does_not_default_partial_prefixes: bool,
}

impl SyntaxStrictDecodeRouteContract {
    fn validate(&self) -> Result<(), SyntaxBackendBoundaryError> {
        require_bool(
            "strict_decode_contract.strict_decode_authoritative",
            self.strict_decode_authoritative,
            true,
        )?;
        require_bool(
            "strict_decode_contract.rejects_trailing_tokens",
            self.rejects_trailing_tokens,
            true,
        )?;
        require_bool(
            "strict_decode_contract.transition_acceptance_equals_admissible_mask",
            self.transition_acceptance_equals_admissible_mask,
            true,
        )?;
        require_all(
            "strict_decode_contract.failure_kinds",
            &self.failure_kinds,
            REQUIRED_STRICT_FAILURES,
        )?;
        require_bool(
            "strict_decode_contract.records_consumed_prefix_requirement",
            self.records_consumed_prefix_requirement,
            true,
        )?;
        require_bool(
            "strict_decode_contract.records_partial_result_requirement",
            self.records_partial_result_requirement,
            true,
        )?;
        require_bool(
            "strict_decode_contract.does_not_default_partial_prefixes",
            self.does_not_default_partial_prefixes,
            true,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxRecoveryDecodeRouteContract {
    pub normalizes_trailing_pad_only: bool,
    pub normalization_requires_complete_syntax_prefix: bool,
    pub rejects_non_pad_surplus: bool,
    pub hard_error_kinds: Vec<String>,
    pub preserves_strict_error_surface: bool,
    pub does_not_promote_prefix_completion: bool,
}

impl SyntaxRecoveryDecodeRouteContract {
    fn validate(&self) -> Result<(), SyntaxBackendBoundaryError> {
        require_bool(
            "recovery_decode_contract.normalizes_trailing_pad_only",
            self.normalizes_trailing_pad_only,
            true,
        )?;
        require_bool(
            "recovery_decode_contract.normalization_requires_complete_syntax_prefix",
            self.normalization_requires_complete_syntax_prefix,
            true,
        )?;
        require_bool(
            "recovery_decode_contract.rejects_non_pad_surplus",
            self.rejects_non_pad_surplus,
            true,
        )?;
        require_all(
            "recovery_decode_contract.hard_error_kinds",
            &self.hard_error_kinds,
            REQUIRED_RECOVERY_HARD_ERRORS,
        )?;
        require_bool(
            "recovery_decode_contract.preserves_strict_error_surface",
            self.preserves_strict_error_surface,
            true,
        )?;
        require_bool(
            "recovery_decode_contract.does_not_promote_prefix_completion",
            self.does_not_promote_prefix_completion,
            true,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxStrictRecoveryGeneratedCodecEvidence {
    pub test_path: String,
    pub renderer_api_module: String,
    pub target_ids: Vec<String>,
    pub parser_backed_target_ids: Vec<String>,
    pub target_agnostic_compiler_route_metadata_field: String,
    pub target_agnostic_compiler_route_evidence: Vec<String>,
    pub policy_constants: Vec<String>,
    pub required_test_markers: Vec<String>,
    pub generated_artifact_envelope_decode_policy_confirmed: bool,
}

impl SyntaxStrictRecoveryGeneratedCodecEvidence {
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
            "generated_codec_evidence.policy_constants",
            &self.policy_constants,
            &[
                "RECOVERY_DECODE_POLICY",
                "PARSER_BACKED_RECOVERY_DECODE_POLICY",
            ],
        )?;
        require_all(
            "generated_codec_evidence.required_test_markers",
            &self.required_test_markers,
            REQUIRED_GENERATED_CODEC_TEST_MARKERS,
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
