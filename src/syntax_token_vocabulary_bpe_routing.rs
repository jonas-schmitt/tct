//! Sidecar-only token/vocabulary/BPE routing gate.
//!
//! This module pins the token layout and optional BPE ownership a future syntax
//! route must preserve after consumed-token accounting is fixed. It does not
//! register a syntax route in `TctCompiler`.

use crate::syntax_artifact_envelope::{
    ARTIFACT_KIND as ENVELOPE_ARTIFACT_KIND, FIXTURE_PATH as ENVELOPE_FIXTURE_PATH,
    SyntaxGeneratedArtifactEnvelope, VERSION as ENVELOPE_VERSION,
};
use crate::syntax_backend_boundary::{SyntaxBackendBoundaryError, SyntaxBackendCompilerBoundary};
use crate::syntax_consumed_token_accounting_routing::{
    ARTIFACT_KIND as CONSUMED_ACCOUNTING_ARTIFACT_KIND,
    COMPLETED_PROMOTION_CLASS as CONSUMED_ACCOUNTING_COMPLETED_PROMOTION_CLASS,
    FIXTURE_PATH as CONSUMED_ACCOUNTING_FIXTURE_PATH,
    NEXT_MISSING_STABLE_PROMOTION_CLASS as CONSUMED_ACCOUNTING_NEXT_MISSING_STABLE_PROMOTION_CLASS,
    SyntaxConsumedTokenAccountingRoutingGate, VERSION as CONSUMED_ACCOUNTING_VERSION,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const TOOL: &str = "syntax_backend_token_vocabulary_bpe_routing_gate";
pub const VERSION: u32 = 1;
pub const ARTIFACT_KIND: &str = "tct_syntax_backend_token_vocabulary_bpe_routing_gate";
pub const FIXTURE_PATH: &str =
    "tests/fixtures/syntax_backend/token-vocabulary-bpe-routing-gate-v1.json";
pub const STATUS: &str = "sidecar_contract_only";
pub const COMPLETED_PROMOTION_CLASS: &str = "syntax backend token/vocabulary/BPE routing gate";
pub const NEXT_MISSING_STABLE_PROMOTION_CLASS: &str = "syntax backend reproducibility routing gate";

const REQUIRED_TARGETS: &[&str] = &["toy", "csharp", "go"];
const REQUIRED_PARSER_BACKED_TARGETS: &[&str] = &["csharp", "go"];
const REQUIRED_TOKEN_LAYOUT_FIELDS: &[&str] = &[
    "PAD_TOKEN",
    "VOCAB_SIZE",
    "raw_base_vocab_size",
    "structural_ranges",
    "payload_ranges",
    "optional_bpe_ranges",
];
const REQUIRED_COLLISION_CHECKS: &[&str] = &[
    "pad_token_below_vocab_size",
    "pad_token_outside_raw_base_ranges",
    "structural_ranges_do_not_overlap_payload_ranges",
    "bpe_range_starts_at_raw_base_vocab_size",
    "bpe_range_below_vocab_size_or_relocates_pad",
    "bpe_ids_do_not_collide_with_pad_token",
    "emitted_tokens_below_vocab_size",
    "emitted_tokens_exclude_pad_token",
];
const REQUIRED_BPE_BOUNDARY_FIELDS: &[&str] = &[
    "bpe_start",
    "bpe_end_exclusive",
    "bpe_merges",
    "current_layout_hosts_bpe",
    "merge_table_sha256",
];
const REQUIRED_GENERATED_CODEC_TEST_MARKERS: &[&str] = &[
    "toy_syntax_generated_codec_token_layout_reserves_bpe_boundary",
    "toy_syntax_bpe_merge_artifact_round_trips_token_corpus",
    "toy_syntax_generated_codec_artifact_metadata_is_reproducible",
    "parser_backed_syntax_generated_codec_artifact_metadata_is_reproducible",
    "raw_base_vocab_size",
    "structural_ranges",
    "payload_ranges",
    "bpe_boundary",
    "bpe_ids_fit_reserved_boundary",
    "tokens_below_vocab_size",
    "*token != boundary.pad_token && *token < boundary.vocab_size",
];
const REQUIRED_RENDERER_API_MARKERS: &[&str] = &[
    "GrammarStructuralTokenLayout",
    "reserve_bpe_boundary",
    "token_layout_envelope_metadata",
    "csharp_syntax_token_layout",
    "go_syntax_token_layout",
];
const REQUIRED_TARGET_AGNOSTIC_COMPILER_ROUTE_EVIDENCE: &[&str] = &[
    "csharp_minimal_class_syntax_v0",
    "csharp_local_string_return_literal_syntax_v0",
    "go_minimal_function_syntax_v0",
    "go_local_declaration_syntax_v0",
    "generated_go_minimal_function_real_source_strings_compare_demo",
];
const REQUIRED_BLOCKERS: &[&str] = &["reproducibility", "audit coverage"];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxTokenVocabularyBpeRoutingGate {
    pub artifact_kind: String,
    pub version: u32,
    pub tool: String,
    pub status: String,
    pub source_consumed_token_accounting_routing_gate:
        SyntaxTokenVocabularySourceConsumedAccountingRoutingGate,
    pub source_generated_artifact_envelope: SyntaxTokenVocabularySourceGeneratedArtifactEnvelope,
    pub compiler_boundary: SyntaxBackendCompilerBoundary,
    pub route_state: SyntaxTokenVocabularyRouteState,
    pub token_layout_contract: SyntaxTokenVocabularyRouteContract,
    pub bpe_boundary_contract: SyntaxBpeBoundaryRouteContract,
    pub generated_codec_evidence: SyntaxTokenVocabularyGeneratedCodecEvidence,
    pub blockers: Vec<String>,
    pub completed_promotion_class: String,
    pub next_missing_stable_promotion_class: String,
}

impl SyntaxTokenVocabularyBpeRoutingGate {
    pub fn validate_against_consumed_accounting_and_envelope(
        &self,
        consumed_accounting_gate: &SyntaxConsumedTokenAccountingRoutingGate,
        generated_artifact_envelope: &SyntaxGeneratedArtifactEnvelope,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        require_string("artifact_kind", &self.artifact_kind, ARTIFACT_KIND)?;
        require_u32("version", self.version, VERSION)?;
        require_string("tool", &self.tool, TOOL)?;
        require_string("status", &self.status, STATUS)?;
        self.source_consumed_token_accounting_routing_gate
            .validate(consumed_accounting_gate)?;
        self.source_generated_artifact_envelope
            .validate(generated_artifact_envelope)?;
        require_sidecar_boundary(&self.compiler_boundary)?;
        self.route_state.validate()?;
        self.token_layout_contract.validate()?;
        self.bpe_boundary_contract.validate()?;
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
pub struct SyntaxTokenVocabularySourceConsumedAccountingRoutingGate {
    pub artifact_kind: String,
    pub version: u32,
    pub module_path: String,
    pub fixture_path: String,
    pub test_path: String,
    pub gate_helper: String,
    pub completed_promotion_class: String,
}

impl SyntaxTokenVocabularySourceConsumedAccountingRoutingGate {
    fn validate(
        &self,
        consumed_accounting_gate: &SyntaxConsumedTokenAccountingRoutingGate,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        require_string(
            "source_consumed_token_accounting_routing_gate.artifact_kind",
            &self.artifact_kind,
            CONSUMED_ACCOUNTING_ARTIFACT_KIND,
        )?;
        require_u32(
            "source_consumed_token_accounting_routing_gate.version",
            self.version,
            CONSUMED_ACCOUNTING_VERSION,
        )?;
        require_string(
            "source_consumed_token_accounting_routing_gate.module_path",
            &self.module_path,
            "src/syntax_consumed_token_accounting_routing.rs",
        )?;
        require_string(
            "source_consumed_token_accounting_routing_gate.fixture_path",
            &self.fixture_path,
            CONSUMED_ACCOUNTING_FIXTURE_PATH,
        )?;
        require_string(
            "source_consumed_token_accounting_routing_gate.test_path",
            &self.test_path,
            "tests/syntax_backend_boundary_test.rs",
        )?;
        require_string(
            "source_consumed_token_accounting_routing_gate.gate_helper",
            &self.gate_helper,
            "./scripts/testing/test-syntax-backend-gates.sh",
        )?;
        require_string(
            "source_consumed_token_accounting_routing_gate.completed_promotion_class",
            &self.completed_promotion_class,
            CONSUMED_ACCOUNTING_COMPLETED_PROMOTION_CLASS,
        )?;
        require_string(
            "source_consumed_token_accounting_routing_gate.source_fixture_completed_class",
            &consumed_accounting_gate.completed_promotion_class,
            CONSUMED_ACCOUNTING_COMPLETED_PROMOTION_CLASS,
        )?;
        require_string(
            "source_consumed_token_accounting_routing_gate.source_fixture_next_missing_class",
            &consumed_accounting_gate.next_missing_stable_promotion_class,
            CONSUMED_ACCOUNTING_NEXT_MISSING_STABLE_PROMOTION_CLASS,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxTokenVocabularySourceGeneratedArtifactEnvelope {
    pub artifact_kind: String,
    pub version: u32,
    pub module_path: String,
    pub fixture_path: String,
    pub test_path: String,
    pub gate_helper: String,
    pub token_layout_metadata_source: String,
}

impl SyntaxTokenVocabularySourceGeneratedArtifactEnvelope {
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
            "source_generated_artifact_envelope.token_layout_metadata_source",
            &self.token_layout_metadata_source,
            "generated artifact envelope token_layout_metadata",
        )?;
        require_all(
            "source_generated_artifact_envelope.required_fields",
            &generated_artifact_envelope
                .token_layout_metadata
                .required_fields,
            &[
                "pad_token",
                "vocab_size",
                "raw_base_vocab_size",
                "structural_ranges",
                "payload_ranges",
                "bpe_boundary",
            ],
        )?;
        require_bool(
            "source_generated_artifact_envelope.requires_bpe_collision_evidence",
            generated_artifact_envelope
                .token_layout_metadata
                .requires_bpe_collision_evidence,
            true,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxTokenVocabularyRouteState {
    pub route_registered: bool,
    pub route_enabled: bool,
    pub public_api_changed: bool,
    pub uses_json_schema_compiler: bool,
    pub uses_syntax_renderer_api: bool,
}

impl SyntaxTokenVocabularyRouteState {
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
pub struct SyntaxTokenVocabularyRouteContract {
    pub target_ids: Vec<String>,
    pub token_layout_fields: Vec<String>,
    pub collision_checks: Vec<String>,
    pub route_owns_token_layout_before_renderer: bool,
    pub emitted_token_upper_bound_required: bool,
    pub does_not_allocate_json_schema_tokens: bool,
}

impl SyntaxTokenVocabularyRouteContract {
    fn validate(&self) -> Result<(), SyntaxBackendBoundaryError> {
        require_all(
            "token_layout_contract.target_ids",
            &self.target_ids,
            REQUIRED_TARGETS,
        )?;
        require_all(
            "token_layout_contract.token_layout_fields",
            &self.token_layout_fields,
            REQUIRED_TOKEN_LAYOUT_FIELDS,
        )?;
        require_all(
            "token_layout_contract.collision_checks",
            &self.collision_checks,
            REQUIRED_COLLISION_CHECKS,
        )?;
        require_bool(
            "token_layout_contract.route_owns_token_layout_before_renderer",
            self.route_owns_token_layout_before_renderer,
            true,
        )?;
        require_bool(
            "token_layout_contract.emitted_token_upper_bound_required",
            self.emitted_token_upper_bound_required,
            true,
        )?;
        require_bool(
            "token_layout_contract.does_not_allocate_json_schema_tokens",
            self.does_not_allocate_json_schema_tokens,
            true,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxBpeBoundaryRouteContract {
    pub bpe_boundary_fields: Vec<String>,
    pub optional_bpe_range_supported: bool,
    pub bpe_absence_is_explicit: bool,
    pub bpe_artifact_identity_required_when_present: bool,
    pub no_bpe_claim_without_merge_artifact: bool,
}

impl SyntaxBpeBoundaryRouteContract {
    fn validate(&self) -> Result<(), SyntaxBackendBoundaryError> {
        require_all(
            "bpe_boundary_contract.bpe_boundary_fields",
            &self.bpe_boundary_fields,
            REQUIRED_BPE_BOUNDARY_FIELDS,
        )?;
        require_bool(
            "bpe_boundary_contract.optional_bpe_range_supported",
            self.optional_bpe_range_supported,
            true,
        )?;
        require_bool(
            "bpe_boundary_contract.bpe_absence_is_explicit",
            self.bpe_absence_is_explicit,
            true,
        )?;
        require_bool(
            "bpe_boundary_contract.bpe_artifact_identity_required_when_present",
            self.bpe_artifact_identity_required_when_present,
            true,
        )?;
        require_bool(
            "bpe_boundary_contract.no_bpe_claim_without_merge_artifact",
            self.no_bpe_claim_without_merge_artifact,
            true,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxTokenVocabularyGeneratedCodecEvidence {
    pub test_path: String,
    pub renderer_api_module: String,
    pub token_layout_source_module: String,
    pub target_ids: Vec<String>,
    pub parser_backed_target_ids: Vec<String>,
    pub required_test_markers: Vec<String>,
    pub required_renderer_api_markers: Vec<String>,
    pub target_agnostic_compiler_route_metadata_field: String,
    pub target_agnostic_compiler_route_evidence: Vec<String>,
    pub generated_artifact_envelope_token_layout_confirmed: bool,
}

impl SyntaxTokenVocabularyGeneratedCodecEvidence {
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
        require_string(
            "generated_codec_evidence.token_layout_source_module",
            &self.token_layout_source_module,
            "src/grammar_ir_structural_model.rs",
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
        require_all(
            "generated_codec_evidence.required_test_markers",
            &self.required_test_markers,
            REQUIRED_GENERATED_CODEC_TEST_MARKERS,
        )?;
        require_all(
            "generated_codec_evidence.required_renderer_api_markers",
            &self.required_renderer_api_markers,
            REQUIRED_RENDERER_API_MARKERS,
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
        require_bool(
            "generated_codec_evidence.generated_artifact_envelope_token_layout_confirmed",
            self.generated_artifact_envelope_token_layout_confirmed,
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
