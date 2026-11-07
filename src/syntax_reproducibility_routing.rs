//! Sidecar-only generated-artifact reproducibility routing gate.
//!
//! This module pins the replay and provenance evidence a future syntax route
//! must preserve after token/vocabulary/BPE ownership is fixed. It does not
//! register a syntax route in `TctCompiler`.

use crate::syntax_backend_boundary::{
    GATE_HELPER_PATH, SyntaxBackendBoundaryError, SyntaxBackendCompilerBoundary,
};
use crate::syntax_reproducibility_replay::{
    ARTIFACT_KIND as REPLAY_ARTIFACT_KIND,
    COMPLETED_PROMOTION_CLASS as REPLAY_COMPLETED_PROMOTION_CLASS,
    FIXTURE_PATH as REPLAY_FIXTURE_PATH, PARSER_BACKED_REPORT_ARTIFACT_KIND,
    PARSER_BACKED_REPORT_PATH, SyntaxGeneratedArtifactReproducibilityReplay,
    TOY_REPORT_ARTIFACT_KIND, TOY_REPORT_PATH, VERSION as REPLAY_VERSION,
};
use crate::syntax_token_vocabulary_bpe_routing::{
    ARTIFACT_KIND as TOKEN_VOCAB_BPE_ARTIFACT_KIND,
    COMPLETED_PROMOTION_CLASS as TOKEN_VOCAB_BPE_COMPLETED_PROMOTION_CLASS,
    FIXTURE_PATH as TOKEN_VOCAB_BPE_FIXTURE_PATH,
    NEXT_MISSING_STABLE_PROMOTION_CLASS as TOKEN_VOCAB_BPE_NEXT_MISSING_STABLE_PROMOTION_CLASS,
    SyntaxTokenVocabularyBpeRoutingGate, VERSION as TOKEN_VOCAB_BPE_VERSION,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const TOOL: &str = "syntax_backend_reproducibility_routing_gate";
pub const VERSION: u32 = 1;
pub const ARTIFACT_KIND: &str = "tct_syntax_backend_reproducibility_routing_gate";
pub const FIXTURE_PATH: &str = "tests/fixtures/syntax_backend/reproducibility-routing-gate-v1.json";
pub const STATUS: &str = "sidecar_contract_only";
pub const COMPLETED_PROMOTION_CLASS: &str = "syntax backend reproducibility routing gate";
pub const NEXT_MISSING_STABLE_PROMOTION_CLASS: &str = "syntax backend audit coverage routing gate";

const REQUIRED_TARGETS: &[&str] = &["toy", "csharp", "go"];
const REQUIRED_DETERMINISTIC_HASH_FIELDS: &[&str] = &[
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
const REQUIRED_VOLATILE_REPORT_FIELDS: &[&str] = &[
    "volatile_report_dir",
    "toy_report_path",
    "parser_backed_report_path",
    "report_artifact_kind",
];
const REQUIRED_SOURCE_FIXTURE_HASH_FIELDS: &[&str] = &[
    "generated_artifact_envelope_sha256",
    "validation_hook_sidecar_evidence_sha256",
    "input_diagnostics_contract_sha256",
    "input_diagnostics_negative_fixture_sha256",
    "generated_artifact_replay_fixture_sha256",
];
const REQUIRED_GENERATED_CODEC_MARKERS: &[&str] = &[
    "toy_syntax_generated_codec_artifact_metadata_is_reproducible",
    "parser_backed_syntax_generated_codec_artifact_metadata_is_reproducible",
    "generated artifact replay identity",
    "volatile report",
    "target_manifest_sha256",
    "generation_options_sha256",
    "generated_crate_identity_sha256",
];
const REQUIRED_REPLAY_POLICY_MARKERS: &[&str] = &[
    "target/tct-syntax-generated-codec-reproducibility/toy-syntax-generated-codec.json",
    "target/tct-syntax-generated-codec-reproducibility/parser-backed-syntax-generated-codecs.json",
    "tct_syntax_generated_codec_reproducibility_report",
    "tct_parser_backed_syntax_generated_codec_reproducibility_report",
];
const REQUIRED_BLOCKERS: &[&str] = &["audit coverage"];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxReproducibilityRoutingGate {
    pub artifact_kind: String,
    pub version: u32,
    pub tool: String,
    pub status: String,
    pub source_token_vocabulary_bpe_routing_gate:
        SyntaxReproducibilitySourceTokenVocabularyBpeRoutingGate,
    pub source_generated_artifact_replay: SyntaxReproducibilitySourceGeneratedArtifactReplay,
    pub compiler_boundary: SyntaxBackendCompilerBoundary,
    pub route_state: SyntaxReproducibilityRouteState,
    pub reproducibility_contract: SyntaxReproducibilityRouteContract,
    pub generated_codec_evidence: SyntaxReproducibilityGeneratedCodecEvidence,
    pub blockers: Vec<String>,
    pub completed_promotion_class: String,
    pub next_missing_stable_promotion_class: String,
}

impl SyntaxReproducibilityRoutingGate {
    pub fn validate_against_token_gate_and_replay(
        &self,
        token_gate: &SyntaxTokenVocabularyBpeRoutingGate,
        replay: &SyntaxGeneratedArtifactReproducibilityReplay,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        require_string("artifact_kind", &self.artifact_kind, ARTIFACT_KIND)?;
        require_u32("version", self.version, VERSION)?;
        require_string("tool", &self.tool, TOOL)?;
        require_string("status", &self.status, STATUS)?;
        self.source_token_vocabulary_bpe_routing_gate
            .validate(token_gate)?;
        self.source_generated_artifact_replay.validate(replay)?;
        require_sidecar_boundary(&self.compiler_boundary)?;
        self.route_state.validate()?;
        self.reproducibility_contract
            .validate_against_replay(replay)?;
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
pub struct SyntaxReproducibilitySourceTokenVocabularyBpeRoutingGate {
    pub artifact_kind: String,
    pub version: u32,
    pub module_path: String,
    pub fixture_path: String,
    pub test_path: String,
    pub gate_helper: String,
    pub completed_promotion_class: String,
}

impl SyntaxReproducibilitySourceTokenVocabularyBpeRoutingGate {
    fn validate(
        &self,
        token_gate: &SyntaxTokenVocabularyBpeRoutingGate,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        require_string(
            "source_token_vocabulary_bpe_routing_gate.artifact_kind",
            &self.artifact_kind,
            TOKEN_VOCAB_BPE_ARTIFACT_KIND,
        )?;
        require_u32(
            "source_token_vocabulary_bpe_routing_gate.version",
            self.version,
            TOKEN_VOCAB_BPE_VERSION,
        )?;
        require_string(
            "source_token_vocabulary_bpe_routing_gate.module_path",
            &self.module_path,
            "src/syntax_token_vocabulary_bpe_routing.rs",
        )?;
        require_string(
            "source_token_vocabulary_bpe_routing_gate.fixture_path",
            &self.fixture_path,
            TOKEN_VOCAB_BPE_FIXTURE_PATH,
        )?;
        require_string(
            "source_token_vocabulary_bpe_routing_gate.test_path",
            &self.test_path,
            "tests/syntax_backend_boundary_test.rs",
        )?;
        require_string(
            "source_token_vocabulary_bpe_routing_gate.gate_helper",
            &self.gate_helper,
            GATE_HELPER_PATH,
        )?;
        require_string(
            "source_token_vocabulary_bpe_routing_gate.completed_promotion_class",
            &self.completed_promotion_class,
            TOKEN_VOCAB_BPE_COMPLETED_PROMOTION_CLASS,
        )?;
        require_string(
            "source_token_vocabulary_bpe_routing_gate.source_fixture_completed_class",
            &token_gate.completed_promotion_class,
            TOKEN_VOCAB_BPE_COMPLETED_PROMOTION_CLASS,
        )?;
        require_string(
            "source_token_vocabulary_bpe_routing_gate.source_fixture_next_missing_class",
            &token_gate.next_missing_stable_promotion_class,
            TOKEN_VOCAB_BPE_NEXT_MISSING_STABLE_PROMOTION_CLASS,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxReproducibilitySourceGeneratedArtifactReplay {
    pub artifact_kind: String,
    pub version: u32,
    pub module_path: String,
    pub fixture_path: String,
    pub test_path: String,
    pub gate_helper: String,
    pub completed_promotion_class: String,
}

impl SyntaxReproducibilitySourceGeneratedArtifactReplay {
    fn validate(
        &self,
        replay: &SyntaxGeneratedArtifactReproducibilityReplay,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        require_string(
            "source_generated_artifact_replay.artifact_kind",
            &self.artifact_kind,
            REPLAY_ARTIFACT_KIND,
        )?;
        require_u32(
            "source_generated_artifact_replay.version",
            self.version,
            REPLAY_VERSION,
        )?;
        require_string(
            "source_generated_artifact_replay.module_path",
            &self.module_path,
            "src/syntax_reproducibility_replay.rs",
        )?;
        require_string(
            "source_generated_artifact_replay.fixture_path",
            &self.fixture_path,
            REPLAY_FIXTURE_PATH,
        )?;
        require_string(
            "source_generated_artifact_replay.test_path",
            &self.test_path,
            "tests/syntax_backend_boundary_test.rs",
        )?;
        require_string(
            "source_generated_artifact_replay.gate_helper",
            &self.gate_helper,
            GATE_HELPER_PATH,
        )?;
        require_string(
            "source_generated_artifact_replay.completed_promotion_class",
            &self.completed_promotion_class,
            REPLAY_COMPLETED_PROMOTION_CLASS,
        )?;
        require_string(
            "source_generated_artifact_replay.source_fixture_completed_class",
            &replay.completed_promotion_class,
            REPLAY_COMPLETED_PROMOTION_CLASS,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxReproducibilityRouteState {
    pub route_registered: bool,
    pub route_enabled: bool,
    pub public_api_changed: bool,
    pub uses_json_schema_compiler: bool,
    pub uses_syntax_renderer_api: bool,
    pub persists_checked_in_generated_artifacts: bool,
}

impl SyntaxReproducibilityRouteState {
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
        )?;
        require_bool(
            "route_state.persists_checked_in_generated_artifacts",
            self.persists_checked_in_generated_artifacts,
            false,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxReproducibilityRouteContract {
    pub target_ids: Vec<String>,
    pub deterministic_hash_fields: Vec<String>,
    pub volatile_report_fields: Vec<String>,
    pub source_fixture_hash_fields: Vec<String>,
    pub route_owns_replay_before_renderer: bool,
    pub deterministic_replay_required: bool,
    pub volatile_reports_stay_under_target: bool,
    pub does_not_depend_on_generated_source_substrings: bool,
}

impl SyntaxReproducibilityRouteContract {
    fn validate_against_replay(
        &self,
        replay: &SyntaxGeneratedArtifactReproducibilityReplay,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        require_all(
            "reproducibility_contract.target_ids",
            &self.target_ids,
            REQUIRED_TARGETS,
        )?;
        require_all(
            "reproducibility_contract.deterministic_hash_fields",
            &self.deterministic_hash_fields,
            REQUIRED_DETERMINISTIC_HASH_FIELDS,
        )?;
        require_all(
            "reproducibility_contract.volatile_report_fields",
            &self.volatile_report_fields,
            REQUIRED_VOLATILE_REPORT_FIELDS,
        )?;
        require_all(
            "reproducibility_contract.source_fixture_hash_fields",
            &self.source_fixture_hash_fields,
            REQUIRED_SOURCE_FIXTURE_HASH_FIELDS,
        )?;
        require_bool(
            "reproducibility_contract.route_owns_replay_before_renderer",
            self.route_owns_replay_before_renderer,
            true,
        )?;
        require_bool(
            "reproducibility_contract.deterministic_replay_required",
            self.deterministic_replay_required,
            true,
        )?;
        require_bool(
            "reproducibility_contract.volatile_reports_stay_under_target",
            self.volatile_reports_stay_under_target,
            true,
        )?;
        require_bool(
            "reproducibility_contract.does_not_depend_on_generated_source_substrings",
            self.does_not_depend_on_generated_source_substrings,
            true,
        )?;
        require_all(
            "replay.deterministic_hash_fields",
            &replay.deterministic_hash_fields,
            REQUIRED_DETERMINISTIC_HASH_FIELDS,
        )?;
        require_all("replay.target_ids", &replay.target_ids, REQUIRED_TARGETS)?;
        require_string(
            "replay.volatile_report_policy.toy_report_path",
            &replay.volatile_report_policy.toy_report_path,
            TOY_REPORT_PATH,
        )?;
        require_string(
            "replay.volatile_report_policy.parser_backed_report_path",
            &replay.volatile_report_policy.parser_backed_report_path,
            PARSER_BACKED_REPORT_PATH,
        )?;
        require_all(
            "replay.volatile_report_policy.report_artifact_kinds",
            &replay.volatile_report_policy.report_artifact_kinds,
            &[TOY_REPORT_ARTIFACT_KIND, PARSER_BACKED_REPORT_ARTIFACT_KIND],
        )?;
        let replay_targets = replay
            .records
            .iter()
            .map(|record| record.target_id.as_str())
            .collect::<BTreeSet<_>>();
        for target in REQUIRED_TARGETS {
            if !replay_targets.contains(target) {
                return Err(SyntaxBackendBoundaryError::new(
                    "replay.records",
                    format!("missing replay record for target {target}"),
                ));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxReproducibilityGeneratedCodecEvidence {
    pub test_path: String,
    pub renderer_api_module: String,
    pub replay_module: String,
    pub target_ids: Vec<String>,
    pub required_test_markers: Vec<String>,
    pub required_replay_policy_markers: Vec<String>,
}

impl SyntaxReproducibilityGeneratedCodecEvidence {
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
            "generated_codec_evidence.replay_module",
            &self.replay_module,
            "src/syntax_reproducibility_replay.rs",
        )?;
        require_all(
            "generated_codec_evidence.target_ids",
            &self.target_ids,
            REQUIRED_TARGETS,
        )?;
        require_all(
            "generated_codec_evidence.required_test_markers",
            &self.required_test_markers,
            REQUIRED_GENERATED_CODEC_MARKERS,
        )?;
        require_all(
            "generated_codec_evidence.required_replay_policy_markers",
            &self.required_replay_policy_markers,
            REQUIRED_REPLAY_POLICY_MARKERS,
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
