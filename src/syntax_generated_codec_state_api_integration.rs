//! Target-neutral generated-codec state API integration gate.
//!
//! This module consumes the checked shared Go/C# generated-codec state API
//! handoff and presents one generated-codec-facing query contract. It remains
//! a sidecar integration slice: it does not register a syntax route in
//! `TctCompiler`, and it does not route `GrammarIR` or `StructuralModel`
//! through the stable compiler path.

use crate::syntax_backend_boundary::SyntaxBackendBoundaryError;
use crate::syntax_generated_codec_state_api_handoff::{
    ARTIFACT_KIND as HANDOFF_ARTIFACT_KIND, BPE_RAW_EXPANSION_POLICY,
    COMPLETED_PROMOTION_CLASS as HANDOFF_COMPLETED_PROMOTION_CLASS,
    FIXTURE_PATH as HANDOFF_FIXTURE_PATH, HANDOFF_ID,
    NEXT_MISSING_STABLE_PROMOTION_CLASS as HANDOFF_NEXT_MISSING_STABLE_PROMOTION_CLASS,
    SyntaxGeneratedCodecStateApiHandoffGate,
};
use crate::syntax_go_source_instance_projection::ProjectionTokenLayoutRangeSummary;
use crate::syntax_state_machine::{
    RAW_BOOLEAN_FALSE_OFFSET, RAW_BOOLEAN_TRUE_OFFSET, RAW_PAYLOAD_END_OFFSET,
};
use crate::syntax_state_machine_replay::{
    CSHARP_STATE_MACHINE_PARITY_TARGET, CSharpMarkerPresenceDecision,
    GoAstDeclarationPresenceDecision, GoAstDeclarationPresenceReplayReport,
    SyntaxCSharpRealSourceStateMachineParityCorpusRecord,
    SyntaxCSharpRealSourceStateMachineParityReport, SyntaxCSharpStateMachineParityReport,
    SyntaxGeneratedCodecRecoveryDecodeCase, SyntaxGeneratedCodecStateApiReport,
    SyntaxGeneratedCodecStateCase, SyntaxGeneratedCodecStrictDecodeCase,
    SyntaxGeneratedCodecStrictRecoveryDecodeReport, SyntaxGeneratedCodecTransitionCase,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const TOOL: &str = "syntax_generated_codec_state_api_integration_gate";
pub const VERSION: u32 = 1;
pub const ARTIFACT_KIND: &str = "tct_syntax_generated_codec_state_api_integration_gate_v1";
pub const FIXTURE_PATH: &str =
    "tests/fixtures/syntax_backend/generated-codec-state-api-integration-v1.json";
pub const STATUS: &str = "target_neutral_generated_codec_state_api_integration_ready";
pub const INTEGRATION_ID: &str = "target-neutral-generated-codec-state-api-integration-v1";
pub const COMPLETED_PROMOTION_CLASS: &str =
    "syntax backend target-neutral generated-codec state API integration gate";
pub const NEXT_MISSING_STABLE_PROMOTION_CLASS: &str =
    crate::syntax_roadmap_successors::GO_SOURCE_RECORD_RAW_TOKEN_GENERATOR_CORE_SUCCESSOR_TASK;
pub const MATCHED_PRESENCE_MASK_SCOPE: &str =
    "matched-go-csharp-presence-generated-codec-admissible-mask-v1";
pub const TARGET_NEUTRAL_RAW_STATE_API_AUTHORITY: &str = "target-neutral raw state API";
pub const CSHARP_NULLABLE_MARKER_PRESENCE_RECORD_ID: &str =
    "csharp-local-nullable-return-type.NullableTypeSyntax.QuestionToken";
pub const CSHARP_LOCAL_NULLABLE_DECLARATION_MARKER_PRESENCE_RECORD_ID: &str =
    "csharp-local-nullable-declaration.NullableTypeSyntax.QuestionToken";

const REQUIRED_TARGETS: &[&str] = &["csharp", "go"];
const REQUIRED_CAPABILITIES: &[&str] = &[
    "state_after_raw_prefix_query",
    "candidate_raw_token_transition_query",
    "eof_validity_query",
    "strict_decode_query",
    "recovery_decode_query",
    "consumed_raw_token_accounting_query",
    "admissible_next_raw_token_ids_query",
    "admissible_next_raw_token_ranges_query",
    "bpe_raw_expansion_query",
    "multi_token_bpe_raw_expansion_replay_query",
];
const REQUIRED_NO_STABLE_CLAIMS: &[&str] = &[
    "no_arena_ir_claim",
    "no_arena_ir_promotion",
    "no_bpe_layer_validity_claim",
    "no_generated_hot_path_claim",
    "no_paper_facing_claim",
    "no_source_program_validity_claim",
    "no_stable_backend_claim",
    "no_tct_compiler_routing_claim",
    "no_type_correctness_claim",
    "no_whole_source_language_validity_claim",
];
const REQUIRED_VALIDATION_COMMANDS: &[&str] = &[
    "cargo test --test syntax_state_machine_replay_test target_neutral_generated_codec_state_api_integration_consumes_shared_handoff_surface -- --exact",
    "cargo test --test syntax_backend_boundary_test syntax_backend_generated_codec_state_api_integration_consumes_shared_handoff_contract -- --exact",
    "cargo test --test syntax_generated_codec_gate_test csharp_compilation_unit_source_file_syntax_generated_codec_compiles_through_compiler_route -- --exact",
    "cargo test --test syntax_generated_codec_gate_test csharp_compilation_unit_logger_source_file_syntax_generated_codec_compiles_through_compiler_route -- --exact",
    "cargo test --test syntax_generated_codec_gate_test csharp_compilation_unit_version_one_source_file_syntax_generated_codec_compiles_through_compiler_route -- --exact",
    "cargo test --test syntax_generated_codec_gate_test csharp_compilation_unit_iterator_methods_source_file_syntax_generated_codec_compiles_through_compiler_route -- --exact",
    "cargo test --test syntax_generated_codec_gate_test csharp_compilation_unit_loan_source_file_syntax_generated_codec_compiles_through_compiler_route -- --exact",
    "git diff --check",
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxGeneratedCodecStateApiIntegrationGate {
    pub artifact_kind: String,
    pub version: u32,
    pub tool: String,
    pub status: String,
    pub integration_id: String,
    pub source_handoff: GeneratedCodecStateApiIntegrationSourceHandoff,
    pub integration_surface: GeneratedCodecStateApiIntegrationSurface,
    pub target_summaries: Vec<GeneratedCodecStateApiIntegrationTargetSummary>,
    pub record_queries: Vec<GeneratedCodecStateApiIntegrationRecordQuery>,
    pub bpe_raw_expansion_contract: GeneratedCodecStateApiIntegrationBpeContract,
    pub claim_boundary: GeneratedCodecStateApiIntegrationClaimBoundary,
    pub validation_commands: Vec<String>,
    pub completed_promotion_class: String,
    pub next_missing_stable_promotion_class: String,
}

impl SyntaxGeneratedCodecStateApiIntegrationGate {
    pub fn validate_against_handoff(
        &self,
        handoff_gate: &SyntaxGeneratedCodecStateApiHandoffGate,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        self.validate_shape()?;
        handoff_gate.validate()?;
        self.source_handoff.validate(handoff_gate)?;
        validate_target_summaries(&self.target_summaries, handoff_gate)?;
        self.integration_surface.validate(handoff_gate)?;
        self.bpe_raw_expansion_contract.validate(handoff_gate)?;
        self.claim_boundary.validate()?;
        validate_record_query_targets(&self.record_queries, handoff_gate)?;
        validate_bpe_replay_query_shape(&self.record_queries, &self.bpe_raw_expansion_contract)?;
        Ok(())
    }

    pub fn validate_against_reports(
        &self,
        handoff_gate: &SyntaxGeneratedCodecStateApiHandoffGate,
        go_state_api: &SyntaxGeneratedCodecStateApiReport,
        go_strict_recovery: &SyntaxGeneratedCodecStrictRecoveryDecodeReport,
        csharp_real_source: &SyntaxCSharpRealSourceStateMachineParityReport,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        self.validate_against_report_sets(
            handoff_gate,
            std::slice::from_ref(go_state_api),
            std::slice::from_ref(go_strict_recovery),
            csharp_real_source,
        )
    }

    pub fn validate_against_report_sets<'a>(
        &self,
        handoff_gate: &SyntaxGeneratedCodecStateApiHandoffGate,
        go_state_api_reports: &'a [SyntaxGeneratedCodecStateApiReport],
        go_strict_recovery_reports: &'a [SyntaxGeneratedCodecStrictRecoveryDecodeReport],
        csharp_real_source: &'a SyntaxCSharpRealSourceStateMachineParityReport,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        self.validate_against_handoff(handoff_gate)?;
        handoff_gate.validate_against_report_sets(
            go_state_api_reports,
            go_strict_recovery_reports,
            csharp_real_source,
        )?;

        let mut apis = go_state_api_reports
            .iter()
            .zip(go_strict_recovery_reports.iter())
            .map(|(state_api, strict_recovery)| {
                TargetNeutralGeneratedCodecStateApi::from_go_reports(state_api, strict_recovery)
            })
            .collect::<Result<Vec<_>, _>>()?;
        apis.extend(
            csharp_real_source
                .corpus_replay_records
                .iter()
                .map(|record| {
                    TargetNeutralGeneratedCodecStateApi::from_csharp_corpus_record(
                        csharp_real_source,
                        record,
                    )
                })
                .collect::<Result<Vec<_>, _>>()?,
        );

        for record_query in &self.record_queries {
            let api = apis
                .iter()
                .find(|api| {
                    api.target == record_query.target && api.record_id == record_query.record_id
                })
                .ok_or_else(|| {
                    SyntaxBackendBoundaryError::new(
                        "record_queries.record_id",
                        format!(
                            "missing target-neutral state API record {}:{}",
                            record_query.target, record_query.record_id
                        ),
                    )
                })?;
            api.admissible_token_mask_contract(record_query)?;
        }

        Ok(())
    }

    fn validate_shape(&self) -> Result<(), SyntaxBackendBoundaryError> {
        require_string("artifact_kind", &self.artifact_kind, ARTIFACT_KIND)?;
        require_u32("version", self.version, VERSION)?;
        require_string("tool", &self.tool, TOOL)?;
        require_string("status", &self.status, STATUS)?;
        require_string("integration_id", &self.integration_id, INTEGRATION_ID)?;
        require_positive_usize("target_summaries", self.target_summaries.len())?;
        require_positive_usize("record_queries", self.record_queries.len())?;
        require_all(
            "validation_commands",
            &self.validation_commands,
            REQUIRED_VALIDATION_COMMANDS,
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
pub struct GeneratedCodecStateApiIntegrationSourceHandoff {
    pub artifact_kind: String,
    pub fixture_path: String,
    pub module_path: String,
    pub handoff_id: String,
    pub completed_promotion_class: String,
    pub next_missing_stable_promotion_class: String,
    pub target_count: usize,
    pub total_source_record_count: usize,
    pub target_neutral_surface_id: String,
}

impl GeneratedCodecStateApiIntegrationSourceHandoff {
    fn validate(
        &self,
        handoff_gate: &SyntaxGeneratedCodecStateApiHandoffGate,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        require_string(
            "source_handoff.artifact_kind",
            &self.artifact_kind,
            HANDOFF_ARTIFACT_KIND,
        )?;
        require_string(
            "source_handoff.fixture_path",
            &self.fixture_path,
            HANDOFF_FIXTURE_PATH,
        )?;
        require_string(
            "source_handoff.module_path",
            &self.module_path,
            "src/syntax_generated_codec_state_api_handoff.rs",
        )?;
        require_string("source_handoff.handoff_id", &self.handoff_id, HANDOFF_ID)?;
        require_string(
            "source_handoff.completed_promotion_class",
            &self.completed_promotion_class,
            HANDOFF_COMPLETED_PROMOTION_CLASS,
        )?;
        require_string(
            "source_handoff.next_missing_stable_promotion_class",
            &self.next_missing_stable_promotion_class,
            HANDOFF_NEXT_MISSING_STABLE_PROMOTION_CLASS,
        )?;
        require_usize(
            "source_handoff.target_count",
            self.target_count,
            handoff_gate.evidence_sources.len(),
        )?;
        let total_source_record_count = handoff_gate
            .evidence_sources
            .iter()
            .map(|source| source.source_record_count)
            .sum();
        require_usize(
            "source_handoff.total_source_record_count",
            self.total_source_record_count,
            total_source_record_count,
        )?;
        require_string(
            "source_handoff.target_neutral_surface_id",
            &self.target_neutral_surface_id,
            &handoff_gate.target_neutral_surface.surface_id,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeneratedCodecStateApiIntegrationSurface {
    pub surface_id: String,
    pub source_surface_id: String,
    pub capabilities: Vec<String>,
    pub target_neutral: bool,
    pub query_api_ready: bool,
    pub prefix_state_queries_ready: bool,
    pub transition_queries_ready: bool,
    pub eof_queries_ready: bool,
    pub strict_recovery_decode_queries_ready: bool,
    pub consumed_token_accounting_queries_ready: bool,
    pub admissible_raw_token_set_queries_ready: bool,
    pub admissible_raw_token_range_queries_ready: bool,
    pub bpe_raw_expansion_queries_ready: bool,
    pub tct_compiler_route_registered: bool,
    pub grammar_ir_or_structural_model_routed_through_tct_compiler: bool,
}

impl GeneratedCodecStateApiIntegrationSurface {
    fn validate(
        &self,
        handoff_gate: &SyntaxGeneratedCodecStateApiHandoffGate,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        require_string(
            "integration_surface.surface_id",
            &self.surface_id,
            "target-neutral-generated-codec-state-api-integration-surface-v1",
        )?;
        require_string(
            "integration_surface.source_surface_id",
            &self.source_surface_id,
            &handoff_gate.target_neutral_surface.surface_id,
        )?;
        require_all(
            "integration_surface.capabilities",
            &self.capabilities,
            REQUIRED_CAPABILITIES,
        )?;
        require_bool(
            "integration_surface.target_neutral",
            self.target_neutral,
            true,
        )?;
        require_bool(
            "integration_surface.query_api_ready",
            self.query_api_ready,
            true,
        )?;
        require_bool(
            "integration_surface.prefix_state_queries_ready",
            self.prefix_state_queries_ready,
            true,
        )?;
        require_bool(
            "integration_surface.transition_queries_ready",
            self.transition_queries_ready,
            true,
        )?;
        require_bool(
            "integration_surface.eof_queries_ready",
            self.eof_queries_ready,
            true,
        )?;
        require_bool(
            "integration_surface.strict_recovery_decode_queries_ready",
            self.strict_recovery_decode_queries_ready,
            true,
        )?;
        require_bool(
            "integration_surface.consumed_token_accounting_queries_ready",
            self.consumed_token_accounting_queries_ready,
            true,
        )?;
        require_bool(
            "integration_surface.admissible_raw_token_set_queries_ready",
            self.admissible_raw_token_set_queries_ready,
            true,
        )?;
        require_bool(
            "integration_surface.admissible_raw_token_range_queries_ready",
            self.admissible_raw_token_range_queries_ready,
            true,
        )?;
        require_bool(
            "integration_surface.bpe_raw_expansion_queries_ready",
            self.bpe_raw_expansion_queries_ready,
            true,
        )?;
        require_bool(
            "integration_surface.tct_compiler_route_registered",
            self.tct_compiler_route_registered,
            false,
        )?;
        require_bool(
            "integration_surface.grammar_ir_or_structural_model_routed_through_tct_compiler",
            self.grammar_ir_or_structural_model_routed_through_tct_compiler,
            false,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeneratedCodecStateApiIntegrationTargetSummary {
    pub target: String,
    pub source_record_count: usize,
    pub state_case_count: usize,
    pub transition_case_count: usize,
    pub strict_decode_case_count: usize,
    pub recovery_decode_case_count: usize,
    pub prefix_state_api_ready: bool,
    pub transition_api_ready: bool,
    pub eof_state_api_ready: bool,
    pub strict_recovery_decode_ready: bool,
    pub consumed_token_accounting_ok: bool,
    pub admissible_next_raw_token_sets_ready: bool,
    pub admissible_next_raw_token_ranges_ready: bool,
    pub bpe_layer_defines_validity: bool,
    pub target_metadata_sidecar_only: bool,
    pub arena_ir_used: bool,
    pub stable_promotion_allowed: bool,
}

impl GeneratedCodecStateApiIntegrationTargetSummary {
    fn validate_against_handoff_source(
        &self,
        source: &crate::syntax_generated_codec_state_api_handoff::GeneratedCodecStateApiHandoffEvidenceSource,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        require_string("target_summary.target", &self.target, &source.target)?;
        require_usize(
            "target_summary.source_record_count",
            self.source_record_count,
            source.source_record_count,
        )?;
        require_usize(
            "target_summary.state_case_count",
            self.state_case_count,
            source.state_case_count,
        )?;
        require_usize(
            "target_summary.transition_case_count",
            self.transition_case_count,
            source.transition_case_count,
        )?;
        require_usize(
            "target_summary.strict_decode_case_count",
            self.strict_decode_case_count,
            source.strict_decode_case_count,
        )?;
        require_usize(
            "target_summary.recovery_decode_case_count",
            self.recovery_decode_case_count,
            source.recovery_decode_case_count,
        )?;
        require_bool(
            "target_summary.prefix_state_api_ready",
            self.prefix_state_api_ready,
            source.prefix_state_api_ready,
        )?;
        require_bool(
            "target_summary.transition_api_ready",
            self.transition_api_ready,
            source.transition_api_ready,
        )?;
        require_bool(
            "target_summary.eof_state_api_ready",
            self.eof_state_api_ready,
            source.eof_state_api_ready,
        )?;
        require_bool(
            "target_summary.strict_recovery_decode_ready",
            self.strict_recovery_decode_ready,
            source.strict_recovery_decode_ready,
        )?;
        require_bool(
            "target_summary.consumed_token_accounting_ok",
            self.consumed_token_accounting_ok,
            source.consumed_token_accounting_ok,
        )?;
        require_bool(
            "target_summary.admissible_next_raw_token_sets_ready",
            self.admissible_next_raw_token_sets_ready,
            source.admissible_next_raw_token_sets_ready,
        )?;
        require_bool(
            "target_summary.admissible_next_raw_token_ranges_ready",
            self.admissible_next_raw_token_ranges_ready,
            source.admissible_next_raw_token_ranges_ready,
        )?;
        require_bool(
            "target_summary.bpe_layer_defines_validity",
            self.bpe_layer_defines_validity,
            source.bpe_layer_defines_validity,
        )?;
        require_bool(
            "target_summary.target_metadata_sidecar_only",
            self.target_metadata_sidecar_only,
            source.target_metadata_sidecar_only,
        )?;
        require_bool(
            "target_summary.arena_ir_used",
            self.arena_ir_used,
            source.arena_ir_used,
        )?;
        require_bool(
            "target_summary.stable_promotion_allowed",
            self.stable_promotion_allowed,
            source.stable_promotion_allowed,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeneratedCodecStateApiIntegrationRecordQuery {
    pub target: String,
    pub record_id: String,
    pub prefix_state_case_ids: Vec<String>,
    pub eof_state_case_id: String,
    pub payload_range_state_case_id: String,
    pub accepted_transition_case_id: String,
    pub rejected_transition_case_id: String,
    pub strict_accept_case_id: String,
    pub strict_reject_case_id: String,
    pub recovery_success_case_id: String,
    pub recovery_normalized_case_id: String,
    pub bpe_raw_expansion_transition_case_ids: Vec<String>,
    pub bpe_raw_expansion_replay_cases: Vec<GeneratedCodecStateApiIntegrationBpeReplayCase>,
}

impl GeneratedCodecStateApiIntegrationRecordQuery {
    fn validate_against_api(
        &self,
        api: &TargetNeutralGeneratedCodecStateApi<'_>,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        require_string("record_query.target", &self.target, api.target)?;
        require_string("record_query.record_id", &self.record_id, api.record_id)?;
        require_positive_usize(
            "record_query.prefix_state_case_ids",
            self.prefix_state_case_ids.len(),
        )?;
        for case_id in &self.prefix_state_case_ids {
            let state = api.state_after_raw_prefix(case_id)?;
            if state.consumed_raw_tokens > api.raw_token_count {
                return Err(SyntaxBackendBoundaryError::new(
                    "record_query.prefix_state_case_ids",
                    "prefix query consumed-token count exceeds the source record length",
                ));
            }
        }

        let eof_state = api.state_after_raw_prefix(&self.eof_state_case_id)?;
        require_bool(
            "record_query.eof_state_case_id.eof_valid",
            eof_state.eof_valid,
            true,
        )?;
        require_usize(
            "record_query.eof_state_case_id.admissible_next_raw_token_ids",
            eof_state.admissible_next_raw_token_ids.len(),
            0,
        )?;
        require_bool(
            "record_query.eof_validity",
            api.eof_validity(&self.eof_state_case_id)?,
            true,
        )?;

        let payload_state = api.state_after_raw_prefix(&self.payload_range_state_case_id)?;
        require_positive_usize(
            "record_query.payload_range_state_case_id.admissible_next_raw_token_ranges",
            payload_state.admissible_next_raw_token_ranges.len(),
        )?;
        require_positive_usize(
            "record_query.payload_range_state_case_id.admissible_next_raw_token_ids",
            payload_state.admissible_next_raw_token_ids.len(),
        )?;
        require_positive_usize(
            "record_query.payload_range_state_case_id.admissible_ranges_via_query",
            api.admissible_next_raw_token_ranges(&self.payload_range_state_case_id)?
                .len(),
        )?;
        require_positive_usize(
            "record_query.payload_range_state_case_id.admissible_ids_via_query",
            api.admissible_next_raw_token_ids(&self.payload_range_state_case_id)?
                .len(),
        )?;

        let accepted_transition =
            api.candidate_raw_token_transition(&self.accepted_transition_case_id)?;
        require_bool(
            "record_query.accepted_transition_case_id.accepted",
            accepted_transition.accepted,
            true,
        )?;
        let rejected_transition =
            api.candidate_raw_token_transition(&self.rejected_transition_case_id)?;
        require_bool(
            "record_query.rejected_transition_case_id.accepted",
            rejected_transition.accepted,
            false,
        )?;

        let strict_accept = api.strict_decode(&self.strict_accept_case_id)?;
        require_bool(
            "record_query.strict_accept_case_id.accepted",
            strict_accept.accepted,
            true,
        )?;
        let strict_reject = api.strict_decode(&self.strict_reject_case_id)?;
        require_bool(
            "record_query.strict_reject_case_id.accepted",
            strict_reject.accepted,
            false,
        )?;
        require_non_empty(
            "record_query.strict_reject_case_id.error_kind",
            strict_reject.error_kind.as_deref().unwrap_or_default(),
        )?;
        require_usize(
            "record_query.strict_accept_case_id.consumed_raw_tokens",
            api.consumed_raw_token_count_for_strict(&self.strict_accept_case_id)?,
            api.raw_token_count,
        )?;

        let recovery_success = api.recovery_decode(&self.recovery_success_case_id)?;
        require_string(
            "record_query.recovery_success_case_id.observed_outcome",
            &recovery_success.observed_outcome,
            "strict_success",
        )?;
        let recovery_normalized = api.recovery_decode(&self.recovery_normalized_case_id)?;
        require_string(
            "record_query.recovery_normalized_case_id.observed_outcome",
            &recovery_normalized.observed_outcome,
            "normalized_trailing_pad",
        )?;
        require_positive_usize(
            "record_query.recovery_normalized_case_id.ignored_trailing_tokens",
            recovery_normalized.ignored_trailing_tokens,
        )?;

        require_all(
            "record_query.bpe_raw_expansion_transition_case_ids",
            &self.bpe_raw_expansion_transition_case_ids,
            &[
                self.accepted_transition_case_id.as_str(),
                self.rejected_transition_case_id.as_str(),
            ],
        )?;
        for transition_id in &self.bpe_raw_expansion_transition_case_ids {
            let transition = api.candidate_raw_token_transition(transition_id)?;
            let result =
                api.bpe_raw_expansion_query(transition_id, &[transition.candidate_raw_token_id])?;
            require_string(
                "record_query.bpe_raw_expansion.validity_authority",
                &result.validity_authority,
                "target-neutral raw transition API",
            )?;
            require_bool(
                "record_query.bpe_raw_expansion.bpe_layer_defines_validity",
                result.bpe_layer_defines_validity,
                false,
            )?;
            require_bool(
                "record_query.bpe_raw_expansion.accepted",
                result.accepted,
                transition.accepted,
            )?;
        }
        for replay_case in &self.bpe_raw_expansion_replay_cases {
            replay_case.validate_against_api(api)?;
        }

        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeneratedCodecStateApiIntegrationBpeReplayCase {
    pub case_id: String,
    pub transition_case_ids: Vec<String>,
    pub expanded_raw_token_count: usize,
    pub expected_accepted_raw_token_count: usize,
    pub expected_accepted: bool,
    pub expected_final_observed_status: String,
    pub expected_final_payload_partial: bool,
    pub expected_final_eof_valid: bool,
    pub expected_first_failure_transition_case_id: Option<String>,
    pub expected_first_failure_error_kind: Option<String>,
}

impl GeneratedCodecStateApiIntegrationBpeReplayCase {
    fn validate_shape(&self) -> Result<(), SyntaxBackendBoundaryError> {
        require_non_empty("bpe_raw_expansion_replay.case_id", &self.case_id)?;
        require_positive_usize(
            "bpe_raw_expansion_replay.transition_case_ids",
            self.transition_case_ids.len(),
        )?;
        if self.transition_case_ids.len() < 2 {
            return Err(SyntaxBackendBoundaryError::new(
                "bpe_raw_expansion_replay.transition_case_ids",
                "multi-token BPE replay case must contain more than one raw transition",
            ));
        }
        require_usize(
            "bpe_raw_expansion_replay.expanded_raw_token_count",
            self.expanded_raw_token_count,
            self.transition_case_ids.len(),
        )?;
        if self.expected_accepted_raw_token_count > self.expanded_raw_token_count {
            return Err(SyntaxBackendBoundaryError::new(
                "bpe_raw_expansion_replay.expected_accepted_raw_token_count",
                "accepted raw-token count must not exceed expansion length",
            ));
        }
        require_non_empty(
            "bpe_raw_expansion_replay.expected_final_observed_status",
            &self.expected_final_observed_status,
        )?;
        match (
            self.expected_accepted,
            self.expected_first_failure_transition_case_id.as_ref(),
            self.expected_first_failure_error_kind.as_ref(),
        ) {
            (true, None, None) => Ok(()),
            (false, Some(transition), Some(error_kind))
                if !transition.is_empty() && !error_kind.is_empty() =>
            {
                Ok(())
            }
            (true, _, _) => Err(SyntaxBackendBoundaryError::new(
                "bpe_raw_expansion_replay.expected_first_failure",
                "accepted replay case must not name a first failure",
            )),
            (false, _, _) => Err(SyntaxBackendBoundaryError::new(
                "bpe_raw_expansion_replay.expected_first_failure",
                "rejected replay case must name the first failing transition and error kind",
            )),
        }
    }

    fn validate_against_api(
        &self,
        api: &TargetNeutralGeneratedCodecStateApi<'_>,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        self.validate_shape()?;
        let transition_case_ids = self
            .transition_case_ids
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>();
        let result = api.bpe_raw_expansion_replay_query(&transition_case_ids)?;
        require_usize(
            "bpe_raw_expansion_replay.result.expanded_raw_token_count",
            result.expanded_raw_token_count,
            self.expanded_raw_token_count,
        )?;
        require_usize(
            "bpe_raw_expansion_replay.result.accepted_raw_token_count",
            result.accepted_raw_token_count,
            self.expected_accepted_raw_token_count,
        )?;
        require_bool(
            "bpe_raw_expansion_replay.result.accepted",
            result.accepted,
            self.expected_accepted,
        )?;
        require_string(
            "bpe_raw_expansion_replay.result.final_observed_status",
            &result.final_observed_status,
            &self.expected_final_observed_status,
        )?;
        require_bool(
            "bpe_raw_expansion_replay.result.final_payload_partial",
            result.final_payload_partial,
            self.expected_final_payload_partial,
        )?;
        require_bool(
            "bpe_raw_expansion_replay.result.final_eof_valid",
            result.final_eof_valid,
            self.expected_final_eof_valid,
        )?;
        require_non_empty(
            "bpe_raw_expansion_replay.result.final_prefix_state",
            &result.final_prefix_state,
        )?;
        require_optional_string(
            "bpe_raw_expansion_replay.result.first_failure_transition_case_id",
            result.first_failure_transition_case_id.as_deref(),
            self.expected_first_failure_transition_case_id.as_deref(),
        )?;
        require_optional_string(
            "bpe_raw_expansion_replay.result.first_failure_error_kind",
            result.first_failure_error_kind.as_deref(),
            self.expected_first_failure_error_kind.as_deref(),
        )?;
        if !self.expected_accepted {
            require_non_empty(
                "bpe_raw_expansion_replay.result.first_failure_diagnostic",
                result
                    .first_failure_diagnostic
                    .as_deref()
                    .unwrap_or_default(),
            )?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeneratedCodecStateApiIntegrationBpeContract {
    pub policy: String,
    pub expansion_uses_transition_api: bool,
    pub bpe_layer_defines_validity: bool,
    pub bpe_ids_used_as_syntax_validity_authority: bool,
    pub one_token_raw_expansion_probe_ready: bool,
    pub multi_token_raw_expansion_runtime_replay_ready: bool,
    pub next_generated_codec_state_api_work: String,
}

impl GeneratedCodecStateApiIntegrationBpeContract {
    fn validate(
        &self,
        handoff_gate: &SyntaxGeneratedCodecStateApiHandoffGate,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        require_string(
            "bpe_raw_expansion_contract.policy",
            &self.policy,
            BPE_RAW_EXPANSION_POLICY,
        )?;
        require_string(
            "bpe_raw_expansion_contract.handoff_policy",
            &self.policy,
            &handoff_gate.target_neutral_surface.bpe_raw_expansion_policy,
        )?;
        require_bool(
            "bpe_raw_expansion_contract.expansion_uses_transition_api",
            self.expansion_uses_transition_api,
            true,
        )?;
        require_bool(
            "bpe_raw_expansion_contract.bpe_layer_defines_validity",
            self.bpe_layer_defines_validity,
            false,
        )?;
        require_bool(
            "bpe_raw_expansion_contract.bpe_ids_used_as_syntax_validity_authority",
            self.bpe_ids_used_as_syntax_validity_authority,
            false,
        )?;
        require_bool(
            "bpe_raw_expansion_contract.one_token_raw_expansion_probe_ready",
            self.one_token_raw_expansion_probe_ready,
            true,
        )?;
        require_bool(
            "bpe_raw_expansion_contract.multi_token_raw_expansion_runtime_replay_ready",
            self.multi_token_raw_expansion_runtime_replay_ready,
            true,
        )?;
        require_string(
            "bpe_raw_expansion_contract.next_generated_codec_state_api_work",
            &self.next_generated_codec_state_api_work,
            NEXT_MISSING_STABLE_PROMOTION_CLASS,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeneratedCodecStateApiIntegrationClaimBoundary {
    pub syntax_level_boundary: String,
    pub target_neutral_generated_codec_integration_slice: bool,
    pub tct_compiler_routing_used: bool,
    pub grammar_ir_or_structural_model_routed_through_tct_compiler: bool,
    pub source_program_validity_claim_allowed: bool,
    pub type_correctness_claim_allowed: bool,
    pub generated_hot_path_claim_allowed: bool,
    pub paper_facing_claim_allowed: bool,
    pub stable_backend_promotion_allowed: bool,
    pub arena_ir_proof_used: bool,
    pub bpe_layer_validity_claim_allowed: bool,
    pub json_schema_backend_changed: bool,
    pub no_stable_claims: Vec<String>,
}

impl GeneratedCodecStateApiIntegrationClaimBoundary {
    fn validate(&self) -> Result<(), SyntaxBackendBoundaryError> {
        require_string(
            "claim_boundary.syntax_level_boundary",
            &self.syntax_level_boundary,
            "parser-backed metadata -> GrammarIR -> StructuralModel -> token layout -> generated codec",
        )?;
        require_bool(
            "claim_boundary.target_neutral_generated_codec_integration_slice",
            self.target_neutral_generated_codec_integration_slice,
            true,
        )?;
        require_bool(
            "claim_boundary.tct_compiler_routing_used",
            self.tct_compiler_routing_used,
            false,
        )?;
        require_bool(
            "claim_boundary.grammar_ir_or_structural_model_routed_through_tct_compiler",
            self.grammar_ir_or_structural_model_routed_through_tct_compiler,
            false,
        )?;
        require_bool(
            "claim_boundary.source_program_validity_claim_allowed",
            self.source_program_validity_claim_allowed,
            false,
        )?;
        require_bool(
            "claim_boundary.type_correctness_claim_allowed",
            self.type_correctness_claim_allowed,
            false,
        )?;
        require_bool(
            "claim_boundary.generated_hot_path_claim_allowed",
            self.generated_hot_path_claim_allowed,
            false,
        )?;
        require_bool(
            "claim_boundary.paper_facing_claim_allowed",
            self.paper_facing_claim_allowed,
            false,
        )?;
        require_bool(
            "claim_boundary.stable_backend_promotion_allowed",
            self.stable_backend_promotion_allowed,
            false,
        )?;
        require_bool(
            "claim_boundary.arena_ir_proof_used",
            self.arena_ir_proof_used,
            false,
        )?;
        require_bool(
            "claim_boundary.bpe_layer_validity_claim_allowed",
            self.bpe_layer_validity_claim_allowed,
            false,
        )?;
        require_bool(
            "claim_boundary.json_schema_backend_changed",
            self.json_schema_backend_changed,
            false,
        )?;
        require_all(
            "claim_boundary.no_stable_claims",
            &self.no_stable_claims,
            REQUIRED_NO_STABLE_CLAIMS,
        )
    }
}

#[derive(Debug)]
pub struct TargetNeutralGeneratedCodecStateApi<'a> {
    pub target: &'static str,
    pub route: &'a str,
    pub record_id: &'a str,
    pub raw_token_count: usize,
    pub raw_token_ids_sha256: &'a str,
    pub vocab_size: u32,
    pub pad_token: u32,
    state_cases: &'a [SyntaxGeneratedCodecStateCase],
    transition_cases: &'a [SyntaxGeneratedCodecTransitionCase],
    strict_cases: &'a [SyntaxGeneratedCodecStrictDecodeCase],
    recovery_cases: &'a [SyntaxGeneratedCodecRecoveryDecodeCase],
    bpe_layer_defines_validity: bool,
}

impl<'a> TargetNeutralGeneratedCodecStateApi<'a> {
    pub fn from_go_reports(
        state_api: &'a SyntaxGeneratedCodecStateApiReport,
        strict_recovery: &'a SyntaxGeneratedCodecStrictRecoveryDecodeReport,
    ) -> Result<Self, SyntaxBackendBoundaryError> {
        state_api.validate()?;
        strict_recovery.validate()?;
        require_string(
            "go_state_api.strict_recovery.route",
            &strict_recovery.route,
            &state_api.route,
        )?;
        require_string(
            "go_state_api.strict_recovery.source_path",
            &strict_recovery.source_path,
            &state_api.source_path,
        )?;
        require_string(
            "go_state_api.strict_recovery.raw_token_ids_sha256",
            &strict_recovery.raw_token_ids_sha256,
            &state_api.raw_token_ids_sha256,
        )?;
        require_bool(
            "go_state_api.strict_recovery.state_api_consumed",
            strict_recovery.state_api_consumed,
            true,
        )?;
        require_bool(
            "go_state_api.bpe_layer_defines_validity",
            state_api.bpe_layer_defines_validity || strict_recovery.bpe_layer_defines_validity,
            false,
        )?;
        Ok(Self {
            target: "go",
            route: &state_api.route,
            record_id: &state_api.source_path,
            raw_token_count: state_api.raw_token_count,
            raw_token_ids_sha256: &state_api.raw_token_ids_sha256,
            vocab_size: state_api.token_layout.vocab_size,
            pad_token: state_api.token_layout.pad_token,
            state_cases: &state_api.state_cases,
            transition_cases: &state_api.transition_cases,
            strict_cases: &strict_recovery.strict_cases,
            recovery_cases: &strict_recovery.recovery_cases,
            bpe_layer_defines_validity: false,
        })
    }

    pub fn from_csharp_corpus_record(
        report: &'a SyntaxCSharpRealSourceStateMachineParityReport,
        record: &'a SyntaxCSharpRealSourceStateMachineParityCorpusRecord,
    ) -> Result<Self, SyntaxBackendBoundaryError> {
        report.validate()?;
        require_bool(
            "csharp_state_api.record.prefix_state_api_ready",
            record.prefix_state_api_ready,
            true,
        )?;
        require_bool(
            "csharp_state_api.record.admissible_next_raw_tokens_ready",
            record.admissible_next_raw_tokens_ready,
            true,
        )?;
        require_bool(
            "csharp_state_api.record.strict_recovery_decode_agreement_ready",
            record.strict_recovery_decode_agreement_ready,
            true,
        )?;
        require_bool(
            "csharp_state_api.record.bpe_layer_defines_validity",
            report.arena_ir_used || record.arena_ir_used,
            false,
        )?;
        Ok(Self {
            target: CSHARP_STATE_MACHINE_PARITY_TARGET,
            route: &report.route,
            record_id: &record.manifest_path,
            raw_token_count: record.structural_replay_token_stream.raw_token_count,
            raw_token_ids_sha256: &record.structural_replay_token_stream.raw_token_ids_sha256,
            vocab_size: record
                .structural_replay_token_stream
                .token_layout
                .vocab_size,
            pad_token: record.structural_replay_token_stream.token_layout.pad_token,
            state_cases: &record.state_cases,
            transition_cases: &record.transition_cases,
            strict_cases: &record.strict_cases,
            recovery_cases: &record.recovery_cases,
            bpe_layer_defines_validity: false,
        })
    }

    pub fn from_csharp_parity_report(
        report: &'a SyntaxCSharpStateMachineParityReport,
    ) -> Result<Self, SyntaxBackendBoundaryError> {
        report.validate()?;
        let record_id = csharp_nullable_marker_presence_record_id(&report.candidate_grammar_id)?;
        require_bool(
            "csharp_nullable_marker_state_api.prefix_state_api_ready",
            report.state_after_prefix_ready,
            true,
        )?;
        require_bool(
            "csharp_nullable_marker_state_api.admissible_next_raw_tokens_ready",
            report.admissible_next_raw_tokens_ready,
            true,
        )?;
        require_bool(
            "csharp_nullable_marker_state_api.strict_recovery_decode_agreement_ready",
            report.strict_recovery_decode_agreement_ready,
            true,
        )?;
        require_bool(
            "csharp_nullable_marker_state_api.arena_ir_used",
            report.arena_ir_used,
            false,
        )?;
        require_bool(
            "csharp_nullable_marker_state_api.bpe_layer_defines_validity",
            report.bpe_layer_defines_validity,
            false,
        )?;
        Ok(Self {
            target: CSHARP_STATE_MACHINE_PARITY_TARGET,
            route: &report.route,
            record_id,
            raw_token_count: report.raw_token_count,
            raw_token_ids_sha256: &report.raw_token_ids_sha256,
            vocab_size: report.token_layout.vocab_size,
            pad_token: report.token_layout.pad_token,
            state_cases: &report.state_cases,
            transition_cases: &report.transition_cases,
            strict_cases: &report.strict_cases,
            recovery_cases: &report.recovery_cases,
            bpe_layer_defines_validity: false,
        })
    }

    pub fn state_after_raw_prefix(
        &self,
        case_id: &str,
    ) -> Result<&'a SyntaxGeneratedCodecStateCase, SyntaxBackendBoundaryError> {
        self.state_cases
            .iter()
            .find(|case| case.case_id == case_id)
            .ok_or_else(|| {
                SyntaxBackendBoundaryError::new(
                    "target_neutral_state_api.state_case",
                    format!("missing prefix-state query case {case_id}"),
                )
            })
    }

    pub fn candidate_raw_token_transition(
        &self,
        case_id: &str,
    ) -> Result<&'a SyntaxGeneratedCodecTransitionCase, SyntaxBackendBoundaryError> {
        self.transition_cases
            .iter()
            .find(|case| case.case_id == case_id)
            .ok_or_else(|| {
                SyntaxBackendBoundaryError::new(
                    "target_neutral_state_api.transition_case",
                    format!("missing transition query case {case_id}"),
                )
            })
    }

    pub fn eof_validity(&self, state_case_id: &str) -> Result<bool, SyntaxBackendBoundaryError> {
        Ok(self.state_after_raw_prefix(state_case_id)?.eof_valid)
    }

    pub fn strict_decode(
        &self,
        case_id: &str,
    ) -> Result<&'a SyntaxGeneratedCodecStrictDecodeCase, SyntaxBackendBoundaryError> {
        self.strict_cases
            .iter()
            .find(|case| case.case_id == case_id)
            .ok_or_else(|| {
                SyntaxBackendBoundaryError::new(
                    "target_neutral_state_api.strict_decode_case",
                    format!("missing strict decode query case {case_id}"),
                )
            })
    }

    pub fn recovery_decode(
        &self,
        case_id: &str,
    ) -> Result<&'a SyntaxGeneratedCodecRecoveryDecodeCase, SyntaxBackendBoundaryError> {
        self.recovery_cases
            .iter()
            .find(|case| case.case_id == case_id)
            .ok_or_else(|| {
                SyntaxBackendBoundaryError::new(
                    "target_neutral_state_api.recovery_decode_case",
                    format!("missing recovery decode query case {case_id}"),
                )
            })
    }

    pub fn consumed_raw_token_count_for_strict(
        &self,
        case_id: &str,
    ) -> Result<usize, SyntaxBackendBoundaryError> {
        Ok(self.strict_decode(case_id)?.consumed_raw_tokens)
    }

    pub fn admissible_next_raw_token_ids(
        &self,
        state_case_id: &str,
    ) -> Result<&'a [u32], SyntaxBackendBoundaryError> {
        Ok(&self
            .state_after_raw_prefix(state_case_id)?
            .admissible_next_raw_token_ids)
    }

    pub fn admissible_next_raw_token_ranges(
        &self,
        state_case_id: &str,
    ) -> Result<&'a [ProjectionTokenLayoutRangeSummary], SyntaxBackendBoundaryError> {
        Ok(&self
            .state_after_raw_prefix(state_case_id)?
            .admissible_next_raw_token_ranges)
    }

    pub fn admissible_token_mask_contract(
        &self,
        record_query: &GeneratedCodecStateApiIntegrationRecordQuery,
    ) -> Result<TargetNeutralAdmissibleTokenMaskResult, SyntaxBackendBoundaryError> {
        record_query.validate_against_api(self)?;

        let mut finite_admissible_raw_token_ids = BTreeSet::new();
        let mut finite_mask_state_case_count = 0usize;
        for state_case_id in &record_query.prefix_state_case_ids {
            let state = self.state_after_raw_prefix(state_case_id)?;
            if state.admissible_next_raw_token_ids.is_empty() {
                continue;
            }
            finite_mask_state_case_count += 1;
            for token_id in &state.admissible_next_raw_token_ids {
                if *token_id == self.pad_token {
                    return Err(SyntaxBackendBoundaryError::new(
                        "admissible_token_mask.finite_raw_token_ids",
                        "raw admissible-token masks must not admit PAD_TOKEN",
                    ));
                }
                if *token_id >= self.vocab_size {
                    return Err(SyntaxBackendBoundaryError::new(
                        "admissible_token_mask.finite_raw_token_ids",
                        "raw admissible-token masks must remain below VOCAB_SIZE",
                    ));
                }
                finite_admissible_raw_token_ids.insert(*token_id);
            }
        }
        require_positive_usize(
            "admissible_token_mask.finite_mask_state_case_count",
            finite_mask_state_case_count,
        )?;
        require_positive_usize(
            "admissible_token_mask.finite_admissible_raw_token_count",
            finite_admissible_raw_token_ids.len(),
        )?;

        let payload_state =
            self.state_after_raw_prefix(&record_query.payload_range_state_case_id)?;
        require_positive_usize(
            "admissible_token_mask.payload_range_count",
            payload_state.admissible_next_raw_token_ranges.len(),
        )?;
        let mut payload_range_token_count = 0usize;
        for range in &payload_state.admissible_next_raw_token_ranges {
            if range.start <= self.pad_token && self.pad_token < range.end_exclusive {
                return Err(SyntaxBackendBoundaryError::new(
                    "admissible_token_mask.payload_ranges",
                    "payload admissible-token ranges must not contain PAD_TOKEN",
                ));
            }
            if range.end_exclusive > self.vocab_size {
                return Err(SyntaxBackendBoundaryError::new(
                    "admissible_token_mask.payload_ranges",
                    "payload admissible-token ranges must remain below VOCAB_SIZE",
                ));
            }
            payload_range_token_count += (range.end_exclusive - range.start) as usize;
        }
        require_bool(
            "admissible_token_mask.payload_end_admissible",
            payload_state.payload_end_admissible,
            true,
        )?;

        let eof_state = self.state_after_raw_prefix(&record_query.eof_state_case_id)?;
        require_bool("admissible_token_mask.eof_valid", eof_state.eof_valid, true)?;
        require_usize(
            "admissible_token_mask.eof_admissible_raw_token_count",
            eof_state.admissible_next_raw_token_ids.len(),
            0,
        )?;

        let mut bpe_raw_expansion_accepted_case_count = 0usize;
        let mut bpe_raw_expansion_first_failure_case_count = 0usize;
        let mut bpe_first_failure_diagnostic_count = 0usize;
        for replay_case in &record_query.bpe_raw_expansion_replay_cases {
            let transition_case_ids = replay_case
                .transition_case_ids
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>();
            let result = self.bpe_raw_expansion_replay_query(&transition_case_ids)?;
            require_string(
                "admissible_token_mask.bpe_raw_expansion.validity_authority",
                &result.validity_authority,
                "target-neutral raw transition API",
            )?;
            require_bool(
                "admissible_token_mask.bpe_layer_defines_validity",
                result.bpe_layer_defines_validity,
                false,
            )?;
            require_usize(
                "admissible_token_mask.bpe_raw_expansion.accepted_raw_token_count",
                result.accepted_raw_token_count,
                replay_case.expected_accepted_raw_token_count,
            )?;
            if result.accepted {
                bpe_raw_expansion_accepted_case_count += 1;
            } else {
                bpe_raw_expansion_first_failure_case_count += 1;
                require_non_empty(
                    "admissible_token_mask.bpe_first_failure_diagnostic",
                    result
                        .first_failure_diagnostic
                        .as_deref()
                        .unwrap_or_default(),
                )?;
                bpe_first_failure_diagnostic_count += 1;
            }
        }

        Ok(TargetNeutralAdmissibleTokenMaskResult {
            target: self.target.to_string(),
            record_id: self.record_id.to_string(),
            finite_mask_state_case_count,
            finite_admissible_raw_token_count: finite_admissible_raw_token_ids.len(),
            payload_range_state_case_id: record_query.payload_range_state_case_id.clone(),
            payload_range_count: payload_state.admissible_next_raw_token_ranges.len(),
            payload_range_token_count,
            payload_end_admissible: payload_state.payload_end_admissible,
            eof_state_case_id: record_query.eof_state_case_id.clone(),
            eof_valid: eof_state.eof_valid,
            eof_admissible_raw_token_count: eof_state.admissible_next_raw_token_ids.len(),
            bpe_raw_expansion_replay_case_count: record_query.bpe_raw_expansion_replay_cases.len(),
            bpe_raw_expansion_accepted_case_count,
            bpe_raw_expansion_first_failure_case_count,
            bpe_first_failure_diagnostic_count,
            validity_authority: "target-neutral raw transition API".to_string(),
            bpe_layer_defines_validity: false,
            bpe_ids_used_as_syntax_validity_authority: false,
        })
    }

    pub fn prefix_state_continuation_contract(
        &self,
        record_query: &GeneratedCodecStateApiIntegrationRecordQuery,
    ) -> Result<TargetNeutralPrefixStateContinuationResult, SyntaxBackendBoundaryError> {
        let admissible_mask = self.admissible_token_mask_contract(record_query)?;

        let mut responses = Vec::with_capacity(record_query.prefix_state_case_ids.len());
        let mut finite_admissible_response_count = 0usize;
        let mut payload_range_response_count = 0usize;
        let mut eof_valid_response_count = 0usize;
        for state_case_id in &record_query.prefix_state_case_ids {
            let state = self.state_after_raw_prefix(state_case_id)?;
            if state.consumed_raw_tokens > self.raw_token_count {
                return Err(SyntaxBackendBoundaryError::new(
                    "prefix_state_continuation.consumed_raw_tokens",
                    "prefix continuation response consumed more raw tokens than the source record",
                ));
            }

            let finite_admissible_raw_token_ids =
                self.admissible_next_raw_token_ids(state_case_id)?.to_vec();
            for token_id in &finite_admissible_raw_token_ids {
                if *token_id == self.pad_token {
                    return Err(SyntaxBackendBoundaryError::new(
                        "prefix_state_continuation.finite_admissible_raw_token_ids",
                        "prefix continuation response must not admit PAD_TOKEN",
                    ));
                }
                if *token_id >= self.vocab_size {
                    return Err(SyntaxBackendBoundaryError::new(
                        "prefix_state_continuation.finite_admissible_raw_token_ids",
                        "prefix continuation response must remain below VOCAB_SIZE",
                    ));
                }
            }
            if !finite_admissible_raw_token_ids.is_empty() {
                finite_admissible_response_count += 1;
            }

            let payload_token_ranges = self
                .admissible_next_raw_token_ranges(state_case_id)?
                .to_vec();
            for range in &payload_token_ranges {
                if range.start <= self.pad_token && self.pad_token < range.end_exclusive {
                    return Err(SyntaxBackendBoundaryError::new(
                        "prefix_state_continuation.payload_token_ranges",
                        "prefix continuation payload ranges must not contain PAD_TOKEN",
                    ));
                }
                if range.end_exclusive > self.vocab_size {
                    return Err(SyntaxBackendBoundaryError::new(
                        "prefix_state_continuation.payload_token_ranges",
                        "prefix continuation payload ranges must remain below VOCAB_SIZE",
                    ));
                }
            }
            if !payload_token_ranges.is_empty() {
                payload_range_response_count += 1;
            }
            if state.eof_valid {
                eof_valid_response_count += 1;
            }

            responses.push(TargetNeutralPrefixStateContinuationResponse {
                request_prefix_state_case_id: state_case_id.clone(),
                consumed_raw_tokens: state.consumed_raw_tokens,
                prefix_status: state.observed_status.clone(),
                eof_valid: state.eof_valid,
                active_rule: state.active_rule.clone(),
                active_field: state.active_field.clone(),
                finite_admissible_raw_token_ids,
                payload_token_ranges,
                payload_end_admissible: state.payload_end_admissible,
            });
        }

        require_positive_usize("prefix_state_continuation.response_count", responses.len())?;
        require_positive_usize(
            "prefix_state_continuation.finite_admissible_response_count",
            finite_admissible_response_count,
        )?;
        require_positive_usize(
            "prefix_state_continuation.payload_range_response_count",
            payload_range_response_count,
        )?;
        require_positive_usize(
            "prefix_state_continuation.eof_valid_response_count",
            eof_valid_response_count,
        )?;

        let mut bpe_replay_responses =
            Vec::with_capacity(record_query.bpe_raw_expansion_replay_cases.len());
        let mut bpe_raw_expansion_accepted_case_count = 0usize;
        let mut bpe_raw_expansion_first_failure_case_count = 0usize;
        let mut bpe_first_failure_diagnostic_count = 0usize;
        for replay_case in &record_query.bpe_raw_expansion_replay_cases {
            let transition_case_ids = replay_case
                .transition_case_ids
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>();
            let result = self.bpe_raw_expansion_replay_query(&transition_case_ids)?;
            if result.accepted {
                bpe_raw_expansion_accepted_case_count += 1;
            } else {
                bpe_raw_expansion_first_failure_case_count += 1;
                require_non_empty(
                    "prefix_state_continuation.bpe_first_failure_diagnostic",
                    result
                        .first_failure_diagnostic
                        .as_deref()
                        .unwrap_or_default(),
                )?;
                bpe_first_failure_diagnostic_count += 1;
            }

            bpe_replay_responses.push(TargetNeutralPrefixStateContinuationBpeReplayResponse {
                case_id: replay_case.case_id.clone(),
                expanded_raw_token_count: result.expanded_raw_token_count,
                accepted_raw_token_count: result.accepted_raw_token_count,
                accepted: result.accepted,
                final_prefix_status: result.final_observed_status.clone(),
                final_eof_valid: result.final_eof_valid,
                first_failure_transition_case_id: result.first_failure_transition_case_id,
                first_failure_error_kind: result.first_failure_error_kind,
                first_failure_diagnostic: result.first_failure_diagnostic,
            });
        }
        require_usize(
            "prefix_state_continuation.bpe_raw_expansion_replay_case_count",
            bpe_replay_responses.len(),
            admissible_mask.bpe_raw_expansion_replay_case_count,
        )?;
        require_usize(
            "prefix_state_continuation.bpe_raw_expansion_accepted_case_count",
            bpe_raw_expansion_accepted_case_count,
            admissible_mask.bpe_raw_expansion_accepted_case_count,
        )?;
        require_usize(
            "prefix_state_continuation.bpe_raw_expansion_first_failure_case_count",
            bpe_raw_expansion_first_failure_case_count,
            admissible_mask.bpe_raw_expansion_first_failure_case_count,
        )?;
        require_usize(
            "prefix_state_continuation.bpe_first_failure_diagnostic_count",
            bpe_first_failure_diagnostic_count,
            admissible_mask.bpe_first_failure_diagnostic_count,
        )?;

        Ok(TargetNeutralPrefixStateContinuationResult {
            target: self.target.to_string(),
            record_id: self.record_id.to_string(),
            raw_token_count: self.raw_token_count,
            request_count: record_query.prefix_state_case_ids.len(),
            response_count: responses.len(),
            responses,
            finite_admissible_response_count,
            payload_range_response_count,
            eof_valid_response_count,
            bpe_replay_responses,
            bpe_raw_expansion_replay_case_count: admissible_mask
                .bpe_raw_expansion_replay_case_count,
            bpe_raw_expansion_accepted_case_count,
            bpe_raw_expansion_first_failure_case_count,
            bpe_first_failure_diagnostic_count,
            admissible_mask,
            validity_authority: "target-neutral raw transition API".to_string(),
            bpe_layer_defines_validity: false,
            bpe_ids_used_as_syntax_validity_authority: false,
        })
    }

    pub fn bpe_raw_expansion_query(
        &self,
        transition_case_id: &str,
        expanded_raw_token_ids: &[u32],
    ) -> Result<TargetNeutralBpeRawExpansionResult, SyntaxBackendBoundaryError> {
        require_usize(
            "target_neutral_state_api.bpe_raw_expansion.expanded_raw_token_ids",
            expanded_raw_token_ids.len(),
            1,
        )?;
        let transition = self.candidate_raw_token_transition(transition_case_id)?;
        require_u32(
            "target_neutral_state_api.bpe_raw_expansion.candidate_raw_token_id",
            expanded_raw_token_ids[0],
            transition.candidate_raw_token_id,
        )?;
        self.bpe_raw_expansion_replay_query(&[transition_case_id])
    }

    pub fn bpe_raw_expansion_replay_query(
        &self,
        transition_case_ids: &[&str],
    ) -> Result<TargetNeutralBpeRawExpansionResult, SyntaxBackendBoundaryError> {
        require_positive_usize(
            "target_neutral_state_api.bpe_raw_expansion.transition_case_ids",
            transition_case_ids.len(),
        )?;
        let transitions = transition_case_ids
            .iter()
            .map(|transition_case_id| self.candidate_raw_token_transition(transition_case_id))
            .collect::<Result<Vec<_>, _>>()?;
        let mut expanded_raw_token_ids = Vec::with_capacity(transitions.len());
        let mut transition_ids = Vec::with_capacity(transitions.len());
        for transition in &transitions {
            if transition.candidate_raw_token_id >= self.vocab_size {
                return Err(SyntaxBackendBoundaryError::new(
                    "target_neutral_state_api.bpe_raw_expansion.candidate_raw_token_id",
                    "BPE raw expansion candidate must remain below VOCAB_SIZE",
                ));
            }
            transition_ids.push(transition.case_id.clone());
            expanded_raw_token_ids.push(transition.candidate_raw_token_id);
        }

        let mut accepted_raw_token_count = 0usize;
        let mut last_accepted_transition: Option<&SyntaxGeneratedCodecTransitionCase> = None;
        let mut first_failure_transition: Option<&SyntaxGeneratedCodecTransitionCase> = None;
        let mut previous_consumed_raw_tokens = 0usize;

        for transition in transitions {
            if transition.consumed_raw_tokens < previous_consumed_raw_tokens {
                return Err(SyntaxBackendBoundaryError::new(
                    "target_neutral_state_api.bpe_raw_expansion.transition_order",
                    "multi-token BPE replay transitions must be ordered by consumed raw-token count",
                ));
            }

            if transition.accepted {
                accepted_raw_token_count += 1;
                previous_consumed_raw_tokens = transition.consumed_raw_tokens;
                last_accepted_transition = Some(transition);
            } else {
                first_failure_transition = Some(transition);
                break;
            }
        }

        let final_transition = last_accepted_transition
            .or(first_failure_transition)
            .ok_or_else(|| {
                SyntaxBackendBoundaryError::new(
                    "target_neutral_state_api.bpe_raw_expansion.transition_case_ids",
                    "missing transition replay result",
                )
            })?;
        let accepted = first_failure_transition.is_none()
            && accepted_raw_token_count == transition_case_ids.len();
        let first_failure_error_kind =
            first_failure_transition.and_then(|transition| transition.error_kind.clone());
        Ok(TargetNeutralBpeRawExpansionResult {
            transition_case_id: transition_ids[0].clone(),
            transition_case_ids: transition_ids,
            expanded_raw_token_count: expanded_raw_token_ids.len(),
            expanded_raw_token_ids,
            accepted_raw_token_count,
            accepted,
            observed_status: final_transition.observed_status.clone(),
            error_kind: first_failure_error_kind.clone(),
            final_prefix_state: format!(
                "status={},stack_depth={},active_rule={},active_field={},eof_valid={}",
                final_transition.observed_status,
                final_transition.stack_depth,
                final_transition.active_rule.as_deref().unwrap_or("<none>"),
                final_transition.active_field.as_deref().unwrap_or("<none>"),
                final_transition.eof_valid
            ),
            final_observed_status: final_transition.observed_status.clone(),
            final_payload_partial: false,
            final_eof_valid: final_transition.eof_valid,
            final_stack_depth: final_transition.stack_depth,
            final_active_rule: final_transition.active_rule.clone(),
            final_active_field: final_transition.active_field.clone(),
            first_failure_transition_case_id: first_failure_transition
                .map(|transition| transition.case_id.clone()),
            first_failure_error_kind,
            first_failure_diagnostic: first_failure_transition
                .map(|transition| transition.diagnostic.clone()),
            validity_authority: "target-neutral raw transition API".to_string(),
            bpe_layer_defines_validity: self.bpe_layer_defines_validity,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetNeutralBpeRawExpansionResult {
    pub transition_case_id: String,
    pub transition_case_ids: Vec<String>,
    pub expanded_raw_token_ids: Vec<u32>,
    pub expanded_raw_token_count: usize,
    pub accepted_raw_token_count: usize,
    pub accepted: bool,
    pub observed_status: String,
    pub error_kind: Option<String>,
    pub final_prefix_state: String,
    pub final_observed_status: String,
    pub final_payload_partial: bool,
    pub final_eof_valid: bool,
    pub final_stack_depth: usize,
    pub final_active_rule: Option<String>,
    pub final_active_field: Option<String>,
    pub first_failure_transition_case_id: Option<String>,
    pub first_failure_error_kind: Option<String>,
    pub first_failure_diagnostic: Option<String>,
    pub validity_authority: String,
    pub bpe_layer_defines_validity: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetNeutralAdmissibleTokenMaskResult {
    pub target: String,
    pub record_id: String,
    pub finite_mask_state_case_count: usize,
    pub finite_admissible_raw_token_count: usize,
    pub payload_range_state_case_id: String,
    pub payload_range_count: usize,
    pub payload_range_token_count: usize,
    pub payload_end_admissible: bool,
    pub eof_state_case_id: String,
    pub eof_valid: bool,
    pub eof_admissible_raw_token_count: usize,
    pub bpe_raw_expansion_replay_case_count: usize,
    pub bpe_raw_expansion_accepted_case_count: usize,
    pub bpe_raw_expansion_first_failure_case_count: usize,
    pub bpe_first_failure_diagnostic_count: usize,
    pub validity_authority: String,
    pub bpe_layer_defines_validity: bool,
    pub bpe_ids_used_as_syntax_validity_authority: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetNeutralPrefixStateContinuationResult {
    pub target: String,
    pub record_id: String,
    pub raw_token_count: usize,
    pub request_count: usize,
    pub response_count: usize,
    pub responses: Vec<TargetNeutralPrefixStateContinuationResponse>,
    pub finite_admissible_response_count: usize,
    pub payload_range_response_count: usize,
    pub eof_valid_response_count: usize,
    pub bpe_replay_responses: Vec<TargetNeutralPrefixStateContinuationBpeReplayResponse>,
    pub bpe_raw_expansion_replay_case_count: usize,
    pub bpe_raw_expansion_accepted_case_count: usize,
    pub bpe_raw_expansion_first_failure_case_count: usize,
    pub bpe_first_failure_diagnostic_count: usize,
    pub admissible_mask: TargetNeutralAdmissibleTokenMaskResult,
    pub validity_authority: String,
    pub bpe_layer_defines_validity: bool,
    pub bpe_ids_used_as_syntax_validity_authority: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetNeutralPrefixStateContinuationResponse {
    pub request_prefix_state_case_id: String,
    pub consumed_raw_tokens: usize,
    pub prefix_status: String,
    pub eof_valid: bool,
    pub active_rule: Option<String>,
    pub active_field: Option<String>,
    pub finite_admissible_raw_token_ids: Vec<u32>,
    pub payload_token_ranges: Vec<ProjectionTokenLayoutRangeSummary>,
    pub payload_end_admissible: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetNeutralPrefixStateContinuationBpeReplayResponse {
    pub case_id: String,
    pub expanded_raw_token_count: usize,
    pub accepted_raw_token_count: usize,
    pub accepted: bool,
    pub final_prefix_status: String,
    pub final_eof_valid: bool,
    pub first_failure_transition_case_id: Option<String>,
    pub first_failure_error_kind: Option<String>,
    pub first_failure_diagnostic: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetNeutralMatchedPresenceMaskContract {
    pub scope: String,
    pub syntax_level_boundary: String,
    pub target_neutral_state_api_consumed: bool,
    pub matched_record_count: usize,
    pub finite_mask_record_count: usize,
    pub payload_range_mask_record_count: usize,
    pub invalid_payload_rejection_count: usize,
    pub records: Vec<TargetNeutralPresenceMaskRecord>,
    pub validity_authority: String,
    pub arena_ir_used: bool,
    pub bpe_layer_defines_validity: bool,
    pub source_instance_segment_sidecars_used: bool,
    pub downstream_artifact_ladder_used: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetNeutralPresenceMaskRecord {
    pub target: String,
    pub record_id: String,
    pub owner_rule: String,
    pub owner_field: String,
    pub selected_node_kind: String,
    pub prefix_state_api_case: String,
    pub finite_admissible_raw_token_ids: Vec<u32>,
    pub payload_token_ranges: Vec<ProjectionTokenLayoutRangeSummary>,
    pub payload_end_admissible: bool,
    pub invalid_payload_rejected: bool,
    pub invalid_payload_error_kind: String,
    pub validity_authority: String,
}

pub fn csharp_nullable_marker_presence_record_query() -> GeneratedCodecStateApiIntegrationRecordQuery
{
    csharp_nullable_marker_presence_record_query_for_record_id(
        CSHARP_NULLABLE_MARKER_PRESENCE_RECORD_ID,
    )
}

pub fn csharp_nullable_marker_presence_record_query_for_record_id(
    record_id: impl Into<String>,
) -> GeneratedCodecStateApiIntegrationRecordQuery {
    GeneratedCodecStateApiIntegrationRecordQuery {
        target: CSHARP_STATE_MACHINE_PARITY_TARGET.to_string(),
        record_id: record_id.into(),
        prefix_state_case_ids: vec![
            "csharp_nullable_marker_after_terminal".to_string(),
            "csharp_nullable_marker_after_payload_byte".to_string(),
            "csharp_complete_stream".to_string(),
        ],
        eof_state_case_id: "csharp_complete_stream".to_string(),
        payload_range_state_case_id: "csharp_nullable_marker_after_payload_byte".to_string(),
        accepted_transition_case_id: "csharp_nullable_marker_question_payload_byte_is_accepted"
            .to_string(),
        rejected_transition_case_id: "csharp_nullable_marker_node_start_payload_byte_is_rejected"
            .to_string(),
        strict_accept_case_id: "csharp_strict_complete_stream_accepts_state_api_eof".to_string(),
        strict_reject_case_id: "csharp_strict_partial_prefix_rejects_eof".to_string(),
        recovery_success_case_id: "csharp_recovery_complete_stream_returns_strict_success"
            .to_string(),
        recovery_normalized_case_id:
            "csharp_recovery_trailing_pad_normalizes_after_state_api_complete".to_string(),
        bpe_raw_expansion_transition_case_ids: vec![
            "csharp_nullable_marker_question_payload_byte_is_accepted".to_string(),
            "csharp_nullable_marker_node_start_payload_byte_is_rejected".to_string(),
        ],
        bpe_raw_expansion_replay_cases: vec![
            GeneratedCodecStateApiIntegrationBpeReplayCase {
                case_id: "csharp_nullable_marker_question_then_payload_end_is_accepted".to_string(),
                transition_case_ids: vec![
                    "csharp_nullable_marker_question_payload_byte_is_accepted".to_string(),
                    "csharp_nullable_marker_payload_end_is_accepted".to_string(),
                ],
                expanded_raw_token_count: 2,
                expected_accepted_raw_token_count: 2,
                expected_accepted: true,
                expected_final_observed_status: "partial".to_string(),
                expected_final_payload_partial: false,
                expected_final_eof_valid: false,
                expected_first_failure_transition_case_id: None,
                expected_first_failure_error_kind: None,
            },
            GeneratedCodecStateApiIntegrationBpeReplayCase {
                case_id: "csharp_nullable_marker_question_then_pad_is_rejected".to_string(),
                transition_case_ids: vec![
                    "csharp_nullable_marker_question_payload_byte_is_accepted".to_string(),
                    "csharp_nullable_marker_after_payload_byte_pad_token_is_rejected".to_string(),
                ],
                expanded_raw_token_count: 2,
                expected_accepted_raw_token_count: 1,
                expected_accepted: false,
                expected_final_observed_status: "partial".to_string(),
                expected_final_payload_partial: false,
                expected_final_eof_valid: false,
                expected_first_failure_transition_case_id: Some(
                    "csharp_nullable_marker_after_payload_byte_pad_token_is_rejected".to_string(),
                ),
                expected_first_failure_error_kind: Some("pad_token_in_raw_stream".to_string()),
            },
        ],
    }
}

pub fn csharp_nullable_marker_presence_record_id(
    candidate_grammar_id: &str,
) -> Result<&'static str, SyntaxBackendBoundaryError> {
    match candidate_grammar_id {
        "csharp_local_nullable_return_type_syntax_v0" => {
            Ok(CSHARP_NULLABLE_MARKER_PRESENCE_RECORD_ID)
        }
        "csharp_local_nullable_declaration_syntax_v0" => {
            Ok(CSHARP_LOCAL_NULLABLE_DECLARATION_MARKER_PRESENCE_RECORD_ID)
        }
        other => Err(SyntaxBackendBoundaryError::new(
            "csharp_nullable_marker_presence_record_id.candidate_grammar_id",
            format!("missing nullable-marker record id mapping for {other}"),
        )),
    }
}

pub fn matched_go_csharp_presence_mask_contract(
    go_report: &GoAstDeclarationPresenceReplayReport,
    csharp_report: &SyntaxCSharpStateMachineParityReport,
) -> Result<TargetNeutralMatchedPresenceMaskContract, SyntaxBackendBoundaryError> {
    go_report.validate()?;
    csharp_report.validate()?;
    require_bool(
        "matched_presence_mask.go_report.arena_ir_used",
        go_report.arena_ir_used,
        false,
    )?;
    require_bool(
        "matched_presence_mask.go_report.bpe_layer_defines_validity",
        go_report.bpe_layer_defines_validity,
        false,
    )?;
    require_bool(
        "matched_presence_mask.csharp_report.arena_ir_used",
        csharp_report.arena_ir_used,
        false,
    )?;
    require_bool(
        "matched_presence_mask.csharp_report.bpe_layer_defines_validity",
        csharp_report.bpe_layer_defines_validity,
        false,
    )?;

    let go_decision = go_report
        .presence_decisions
        .iter()
        .find(|decision| decision.owner_rule == "TypeSpec" && decision.owner_field == "Assign")
        .ok_or_else(|| {
            SyntaxBackendBoundaryError::new(
                "matched_presence_mask.go_presence_decision",
                "missing Go TypeSpec.Assign presence decision",
            )
        })?;
    let csharp_decision = csharp_report
        .marker_presence_decisions
        .iter()
        .find(|decision| {
            decision.owner_rule == "NullableTypeSyntax" && decision.owner_field == "QuestionToken"
        })
        .ok_or_else(|| {
            SyntaxBackendBoundaryError::new(
                "matched_presence_mask.csharp_marker_presence_decision",
                "missing C# nullable marker presence decision",
            )
        })?;

    let go = go_presence_mask_record(go_report, go_decision)?;
    let csharp = csharp_presence_mask_record(csharp_report, csharp_decision)?;
    let records = vec![go, csharp];
    let finite_mask_record_count = records
        .iter()
        .filter(|record| !record.finite_admissible_raw_token_ids.is_empty())
        .count();
    let payload_range_mask_record_count = records
        .iter()
        .filter(|record| !record.payload_token_ranges.is_empty())
        .count();
    let invalid_payload_rejection_count = records
        .iter()
        .filter(|record| record.invalid_payload_rejected)
        .count();
    require_usize(
        "matched_presence_mask.matched_record_count",
        records.len(),
        2,
    )?;
    require_usize(
        "matched_presence_mask.finite_mask_record_count",
        finite_mask_record_count,
        2,
    )?;
    require_usize(
        "matched_presence_mask.payload_range_mask_record_count",
        payload_range_mask_record_count,
        1,
    )?;
    require_usize(
        "matched_presence_mask.invalid_payload_rejection_count",
        invalid_payload_rejection_count,
        2,
    )?;

    Ok(TargetNeutralMatchedPresenceMaskContract {
        scope: MATCHED_PRESENCE_MASK_SCOPE.to_string(),
        syntax_level_boundary:
            "parser-backed metadata -> GrammarIR -> StructuralModel -> token layout -> generated codec"
                .to_string(),
        target_neutral_state_api_consumed: true,
        matched_record_count: records.len(),
        finite_mask_record_count,
        payload_range_mask_record_count,
        invalid_payload_rejection_count,
        records,
        validity_authority: TARGET_NEUTRAL_RAW_STATE_API_AUTHORITY.to_string(),
        arena_ir_used: false,
        bpe_layer_defines_validity: false,
        source_instance_segment_sidecars_used: false,
        downstream_artifact_ladder_used: false,
    })
}

fn go_presence_mask_record(
    report: &GoAstDeclarationPresenceReplayReport,
    decision: &GoAstDeclarationPresenceDecision,
) -> Result<TargetNeutralPresenceMaskRecord, SyntaxBackendBoundaryError> {
    let structural_start = structural_range(&report.token_layout)?.start;
    let false_token = structural_start + RAW_BOOLEAN_FALSE_OFFSET;
    let true_token = structural_start + RAW_BOOLEAN_TRUE_OFFSET;
    require_bool(
        "matched_presence_mask.go.state_after_terminal_admits_false",
        decision.state_after_terminal_admits_false,
        true,
    )?;
    require_bool(
        "matched_presence_mask.go.state_after_terminal_admits_true",
        decision.state_after_terminal_admits_true,
        true,
    )?;
    require_u32(
        "matched_presence_mask.go.boolean_payload_raw_token_id",
        decision.boolean_payload_raw_token_id,
        true_token,
    )?;
    let finite_admissible_raw_token_ids = vec![false_token, true_token];
    validate_finite_mask_tokens(
        "matched_presence_mask.go.finite_admissible_raw_token_ids",
        &finite_admissible_raw_token_ids,
        &report.token_layout,
    )?;
    require_bool(
        "matched_presence_mask.go.invalid_boolean_payload_mutation.accepted",
        report.invalid_boolean_payload_mutation.accepted,
        false,
    )?;
    let invalid_payload_error_kind = report
        .invalid_boolean_payload_mutation
        .error_kind
        .as_deref()
        .unwrap_or_default();
    require_string(
        "matched_presence_mask.go.invalid_boolean_payload_mutation.error_kind",
        invalid_payload_error_kind,
        "invalid_payload_token",
    )?;
    Ok(TargetNeutralPresenceMaskRecord {
        target: report.target.clone(),
        record_id: format!(
            "{}.{}.{}",
            report.route, decision.owner_rule, decision.owner_field
        ),
        owner_rule: decision.owner_rule.clone(),
        owner_field: decision.owner_field.clone(),
        selected_node_kind: decision.selected_node_kind.clone(),
        prefix_state_api_case: "state_after_terminal".to_string(),
        finite_admissible_raw_token_ids,
        payload_token_ranges: Vec::new(),
        payload_end_admissible: false,
        invalid_payload_rejected: true,
        invalid_payload_error_kind: invalid_payload_error_kind.to_string(),
        validity_authority: TARGET_NEUTRAL_RAW_STATE_API_AUTHORITY.to_string(),
    })
}

fn csharp_presence_mask_record(
    report: &SyntaxCSharpStateMachineParityReport,
    decision: &CSharpMarkerPresenceDecision,
) -> Result<TargetNeutralPresenceMaskRecord, SyntaxBackendBoundaryError> {
    let record_id = csharp_nullable_marker_presence_record_id(&report.candidate_grammar_id)?;
    let structural_start = structural_range(&report.token_layout)?.start;
    let payload_end_token = structural_start + RAW_PAYLOAD_END_OFFSET;
    let api = TargetNeutralGeneratedCodecStateApi::from_csharp_parity_report(report)?;
    let record_query = csharp_nullable_marker_presence_record_query_for_record_id(record_id);
    let api_mask = api.admissible_token_mask_contract(&record_query)?;
    require_string(
        "matched_presence_mask.csharp.api_mask.record_id",
        &api_mask.record_id,
        record_id,
    )?;
    require_usize(
        "matched_presence_mask.csharp.api_mask.finite_admissible_raw_token_count",
        api_mask.finite_admissible_raw_token_count,
        1,
    )?;
    require_usize(
        "matched_presence_mask.csharp.api_mask.payload_range_count",
        api_mask.payload_range_count,
        1,
    )?;
    let _continuation = api.prefix_state_continuation_contract(&record_query)?;
    let selected_transition =
        api.candidate_raw_token_transition(&record_query.accepted_transition_case_id)?;
    require_bool(
        "matched_presence_mask.csharp.selected_transition.accepted",
        selected_transition.accepted,
        true,
    )?;
    let rejected_transition =
        api.candidate_raw_token_transition(&record_query.rejected_transition_case_id)?;
    require_bool(
        "matched_presence_mask.csharp.rejected_transition.accepted",
        rejected_transition.accepted,
        false,
    )?;
    require_string(
        "matched_presence_mask.csharp.rejected_transition.error_kind",
        rejected_transition
            .error_kind
            .as_deref()
            .unwrap_or_default(),
        "invalid_payload_token",
    )?;
    let strict_accept = api.strict_decode(&record_query.strict_accept_case_id)?;
    require_bool(
        "matched_presence_mask.csharp.strict_accept.accepted",
        strict_accept.accepted,
        true,
    )?;
    let recovery_success = api.recovery_decode(&record_query.recovery_success_case_id)?;
    require_string(
        "matched_presence_mask.csharp.recovery_success.observed_outcome",
        &recovery_success.observed_outcome,
        "strict_success",
    )?;

    require_bool(
        "matched_presence_mask.csharp.state_after_terminal_admits_utf8_payload_range",
        decision.state_after_terminal_admits_utf8_payload_range,
        true,
    )?;
    require_bool(
        "matched_presence_mask.csharp.state_after_payload_byte_admits_payload_end",
        decision.state_after_payload_byte_admits_payload_end,
        true,
    )?;
    let utf8_range = layout_range_by_role(&report.token_layout, "utf8_byte_payload")?.clone();
    if decision.first_payload_raw_token_id < utf8_range.start
        || decision.first_payload_raw_token_id >= utf8_range.end_exclusive
    {
        return Err(SyntaxBackendBoundaryError::new(
            "matched_presence_mask.csharp.first_payload_raw_token_id",
            "nullable marker payload token must fall inside the advertised UTF-8 payload range",
        ));
    }
    require_u32(
        "matched_presence_mask.csharp.selected_transition.candidate_raw_token_id",
        selected_transition.candidate_raw_token_id,
        decision.first_payload_raw_token_id,
    )?;
    validate_payload_ranges(
        "matched_presence_mask.csharp.payload_token_ranges",
        std::slice::from_ref(&utf8_range),
        &report.token_layout,
    )?;
    let finite_admissible_raw_token_ids = api
        .admissible_next_raw_token_ids(&record_query.payload_range_state_case_id)?
        .to_vec();
    require_usize(
        "matched_presence_mask.csharp.finite_admissible_raw_token_ids.len",
        finite_admissible_raw_token_ids.len(),
        1,
    )?;
    require_u32(
        "matched_presence_mask.csharp.finite_admissible_raw_token_ids.payload_end",
        finite_admissible_raw_token_ids[0],
        payload_end_token,
    )?;
    validate_finite_mask_tokens(
        "matched_presence_mask.csharp.finite_admissible_raw_token_ids",
        &finite_admissible_raw_token_ids,
        &report.token_layout,
    )?;
    let payload_token_ranges = api
        .admissible_next_raw_token_ranges(&record_query.payload_range_state_case_id)?
        .to_vec();
    validate_payload_ranges(
        "matched_presence_mask.csharp.api_payload_token_ranges",
        &payload_token_ranges,
        &report.token_layout,
    )?;
    let invalid_payload = report
        .marker_presence_invalid_payload_mutations
        .iter()
        .find(|case| case.case_id == "csharp_nullable_marker_invalid_payload_is_rejected")
        .ok_or_else(|| {
            SyntaxBackendBoundaryError::new(
                "matched_presence_mask.csharp.invalid_payload_mutation",
                "missing nullable marker invalid-payload mutation",
            )
        })?;
    require_bool(
        "matched_presence_mask.csharp.invalid_payload_mutation.accepted",
        invalid_payload.accepted,
        false,
    )?;
    let invalid_payload_error_kind = invalid_payload.error_kind.as_deref().unwrap_or_default();
    require_string(
        "matched_presence_mask.csharp.invalid_payload_mutation.error_kind",
        invalid_payload_error_kind,
        "invalid_payload_token",
    )?;
    Ok(TargetNeutralPresenceMaskRecord {
        target: CSHARP_STATE_MACHINE_PARITY_TARGET.to_string(),
        record_id: record_id.to_string(),
        owner_rule: decision.owner_rule.clone(),
        owner_field: decision.owner_field.clone(),
        selected_node_kind: decision.selected_node_kind.clone(),
        prefix_state_api_case: record_query.payload_range_state_case_id,
        finite_admissible_raw_token_ids,
        payload_token_ranges,
        payload_end_admissible: true,
        invalid_payload_rejected: true,
        invalid_payload_error_kind: invalid_payload_error_kind.to_string(),
        validity_authority: TARGET_NEUTRAL_RAW_STATE_API_AUTHORITY.to_string(),
    })
}

fn structural_range(
    layout: &crate::syntax_go_source_instance_projection::ProjectionTokenLayoutSummary,
) -> Result<&ProjectionTokenLayoutRangeSummary, SyntaxBackendBoundaryError> {
    layout_range_by_role(layout, "structural_decisions")
}

fn layout_range_by_role<'a>(
    layout: &'a crate::syntax_go_source_instance_projection::ProjectionTokenLayoutSummary,
    role: &str,
) -> Result<&'a ProjectionTokenLayoutRangeSummary, SyntaxBackendBoundaryError> {
    layout
        .ranges
        .iter()
        .find(|range| range.role == role)
        .ok_or_else(|| {
            SyntaxBackendBoundaryError::new(
                "matched_presence_mask.token_layout.range",
                format!("missing token-layout range role {role}"),
            )
        })
}

fn validate_finite_mask_tokens(
    field: &'static str,
    token_ids: &[u32],
    layout: &crate::syntax_go_source_instance_projection::ProjectionTokenLayoutSummary,
) -> Result<(), SyntaxBackendBoundaryError> {
    require_positive_usize(field, token_ids.len())?;
    for token_id in token_ids {
        if *token_id == layout.pad_token {
            return Err(SyntaxBackendBoundaryError::new(
                field,
                "presence mask must not admit PAD_TOKEN",
            ));
        }
        if *token_id >= layout.vocab_size {
            return Err(SyntaxBackendBoundaryError::new(
                field,
                "presence mask token must remain below VOCAB_SIZE",
            ));
        }
    }
    Ok(())
}

fn validate_payload_ranges(
    field: &'static str,
    ranges: &[ProjectionTokenLayoutRangeSummary],
    layout: &crate::syntax_go_source_instance_projection::ProjectionTokenLayoutSummary,
) -> Result<(), SyntaxBackendBoundaryError> {
    require_positive_usize(field, ranges.len())?;
    for range in ranges {
        if range.start <= layout.pad_token && layout.pad_token < range.end_exclusive {
            return Err(SyntaxBackendBoundaryError::new(
                field,
                "presence mask payload range must not contain PAD_TOKEN",
            ));
        }
        if range.end_exclusive > layout.vocab_size {
            return Err(SyntaxBackendBoundaryError::new(
                field,
                "presence mask payload range must remain below VOCAB_SIZE",
            ));
        }
    }
    Ok(())
}

fn validate_target_summaries(
    summaries: &[GeneratedCodecStateApiIntegrationTargetSummary],
    handoff_gate: &SyntaxGeneratedCodecStateApiHandoffGate,
) -> Result<(), SyntaxBackendBoundaryError> {
    require_usize(
        "target_summaries",
        summaries.len(),
        handoff_gate.evidence_sources.len(),
    )?;
    let observed_targets = summaries
        .iter()
        .map(|summary| summary.target.as_str())
        .collect::<BTreeSet<_>>();
    let expected_targets = REQUIRED_TARGETS.iter().copied().collect::<BTreeSet<_>>();
    if observed_targets != expected_targets {
        return Err(SyntaxBackendBoundaryError::new(
            "target_summaries.target",
            format!("expected targets {expected_targets:?}, got {observed_targets:?}"),
        ));
    }
    for summary in summaries {
        let source = handoff_gate
            .evidence_sources
            .iter()
            .find(|source| source.target == summary.target)
            .ok_or_else(|| {
                SyntaxBackendBoundaryError::new(
                    "target_summaries.target",
                    format!("missing handoff evidence source for {}", summary.target),
                )
            })?;
        summary.validate_against_handoff_source(source)?;
    }
    Ok(())
}

fn validate_record_query_targets(
    record_queries: &[GeneratedCodecStateApiIntegrationRecordQuery],
    handoff_gate: &SyntaxGeneratedCodecStateApiHandoffGate,
) -> Result<(), SyntaxBackendBoundaryError> {
    let go_count = record_queries
        .iter()
        .filter(|record| record.target == "go")
        .count();
    let csharp_count = record_queries
        .iter()
        .filter(|record| record.target == CSHARP_STATE_MACHINE_PARITY_TARGET)
        .count();
    for source in &handoff_gate.evidence_sources {
        let count = match source.target.as_str() {
            "go" => go_count,
            CSHARP_STATE_MACHINE_PARITY_TARGET => csharp_count,
            other => {
                return Err(SyntaxBackendBoundaryError::new(
                    "record_queries.target",
                    format!("unexpected handoff target {other}"),
                ));
            }
        };
        require_usize(
            "record_queries.target.source_record_count",
            count,
            source.source_record_count,
        )?;
    }
    let mut keys = BTreeSet::new();
    for record in record_queries {
        require_non_empty("record_queries.target", &record.target)?;
        require_non_empty("record_queries.record_id", &record.record_id)?;
        let source = handoff_gate
            .evidence_sources
            .iter()
            .find(|source| source.target == record.target)
            .ok_or_else(|| {
                SyntaxBackendBoundaryError::new(
                    "record_queries.target",
                    format!("missing handoff source for query target {}", record.target),
                )
            })?;
        if !source
            .supported_record_ids
            .iter()
            .any(|record_id| record_id == &record.record_id)
        {
            return Err(SyntaxBackendBoundaryError::new(
                "record_queries.record_id",
                format!(
                    "record query {}:{} is not present in the handoff supported records",
                    record.target, record.record_id
                ),
            ));
        }
        if !keys.insert((record.target.as_str(), record.record_id.as_str())) {
            return Err(SyntaxBackendBoundaryError::new(
                "record_queries.record_id",
                format!(
                    "duplicate record query {}:{}",
                    record.target, record.record_id
                ),
            ));
        }
    }
    Ok(())
}

fn validate_bpe_replay_query_shape(
    record_queries: &[GeneratedCodecStateApiIntegrationRecordQuery],
    contract: &GeneratedCodecStateApiIntegrationBpeContract,
) -> Result<(), SyntaxBackendBoundaryError> {
    let mut replay_targets = BTreeSet::new();
    let mut accepted_case_count = 0usize;
    let mut rejected_case_count = 0usize;
    for record in record_queries {
        for replay_case in &record.bpe_raw_expansion_replay_cases {
            replay_case.validate_shape()?;
            replay_targets.insert(record.target.as_str());
            if replay_case.expected_accepted {
                accepted_case_count += 1;
            } else {
                rejected_case_count += 1;
            }
        }
    }
    if contract.multi_token_raw_expansion_runtime_replay_ready {
        let required_targets = REQUIRED_TARGETS.iter().copied().collect::<BTreeSet<_>>();
        if !required_targets.is_subset(&replay_targets) {
            return Err(SyntaxBackendBoundaryError::new(
                "record_queries.bpe_raw_expansion_replay_cases.target",
                format!(
                    "multi-token BPE replay must cover targets {:?}",
                    required_targets
                        .difference(&replay_targets)
                        .copied()
                        .collect::<Vec<_>>()
                ),
            ));
        }
        require_positive_usize(
            "record_queries.bpe_raw_expansion_replay_cases.accepted",
            accepted_case_count,
        )?;
        require_positive_usize(
            "record_queries.bpe_raw_expansion_replay_cases.rejected",
            rejected_case_count,
        )?;
    }
    Ok(())
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

fn require_optional_string(
    field: &'static str,
    actual: Option<&str>,
    expected: Option<&str>,
) -> Result<(), SyntaxBackendBoundaryError> {
    match (actual, expected) {
        (Some(actual), Some(expected)) => require_string(field, actual, expected),
        (None, None) => Ok(()),
        (Some(actual), None) => Err(SyntaxBackendBoundaryError::new(
            field,
            format!("expected no value, got {actual}"),
        )),
        (None, Some(expected)) => Err(SyntaxBackendBoundaryError::new(
            field,
            format!("expected {expected}, got no value"),
        )),
    }
}

fn require_non_empty(field: &'static str, value: &str) -> Result<(), SyntaxBackendBoundaryError> {
    if value.is_empty() {
        Err(SyntaxBackendBoundaryError::new(
            field,
            "value must not be empty",
        ))
    } else {
        Ok(())
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

fn require_usize(
    field: &'static str,
    actual: usize,
    expected: usize,
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

fn require_positive_usize(
    field: &'static str,
    actual: usize,
) -> Result<(), SyntaxBackendBoundaryError> {
    if actual > 0 {
        Ok(())
    } else {
        Err(SyntaxBackendBoundaryError::new(
            field,
            "expected positive value",
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
    required: &[&str],
) -> Result<(), SyntaxBackendBoundaryError> {
    let actual = actual.iter().map(String::as_str).collect::<BTreeSet<_>>();
    let missing = required
        .iter()
        .copied()
        .filter(|required| !actual.contains(required))
        .collect::<Vec<_>>();
    if missing.is_empty() {
        Ok(())
    } else {
        Err(SyntaxBackendBoundaryError::new(
            field,
            format!("missing required entries: {}", missing.join(", ")),
        ))
    }
}
