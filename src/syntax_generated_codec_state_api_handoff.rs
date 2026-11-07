//! Shared Go/C# generated-codec state API handoff gate.
//!
//! This checked gate consumes report-only Go and C# syntax state-machine
//! evidence and pins the target-neutral generated-codec handoff surface. It is
//! not a stable backend promotion and it does not route `GrammarIR` or
//! `StructuralModel` through `TctCompiler`.

use crate::provenance::json_sha256;
use crate::syntax_backend_boundary::SyntaxBackendBoundaryError;
use crate::syntax_state_machine_replay::{
    CSHARP_REAL_SOURCE_STATE_MACHINE_PARITY_ARTIFACT_KIND,
    CSHARP_REAL_SOURCE_STATE_MACHINE_PARITY_STATUS,
    CSHARP_SOURCE_RECORD_RAW_TOKEN_EMISSION_API_BOUNDARY,
    CSHARP_SOURCE_RECORD_RAW_TOKEN_EMISSION_API_ID, CSHARP_STATE_MACHINE_PARITY_TARGET,
    GENERATED_CODEC_STATE_API_ARTIFACT_KIND, GENERATED_CODEC_STATE_API_STATUS,
    GENERATED_CODEC_STRICT_RECOVERY_DECODE_ARTIFACT_KIND,
    GENERATED_CODEC_STRICT_RECOVERY_DECODE_STATUS, SyntaxCSharpRealSourceStateMachineParityReport,
    SyntaxGeneratedCodecStateApiReport, SyntaxGeneratedCodecStrictRecoveryDecodeReport,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const TOOL: &str = "syntax_generated_codec_state_api_handoff_gate";
pub const VERSION: u32 = 1;
pub const ARTIFACT_KIND: &str = "tct_syntax_generated_codec_state_api_handoff_gate_v1";
pub const FIXTURE_PATH: &str =
    "tests/fixtures/syntax_backend/shared-generated-codec-state-api-handoff-v1.json";
pub const STATUS: &str = "shared_go_csharp_generated_codec_state_api_handoff_ready";
pub const HANDOFF_ID: &str = "shared-generated-codec-state-api-handoff-v1";
pub const COMPLETED_PROMOTION_CLASS: &str =
    "syntax backend shared Go/C# generated-codec state API handoff gate";
pub const NEXT_MISSING_STABLE_PROMOTION_CLASS: &str =
    "syntax backend target-neutral generated-codec state API integration tranche";
pub const SYNTAX_BOUNDARY: &str =
    "parser-backed metadata -> GrammarIR -> StructuralModel -> token layout -> generated codec";
pub const BPE_RAW_EXPANSION_POLICY: &str = "bpe-raw-expansion-v1: BPE tokens are downstream compression only; generated codecs evaluate each BPE expansion as raw syntax tokens through the same target-neutral transition API, report accepted raw-token count and partial payload state, and never use BPE ids as syntax-validity authority";

const REQUIRED_TARGETS: &[&str] = &["csharp", "go"];
const REQUIRED_SURFACE_CAPABILITIES: &[&str] = &[
    "state_after_raw_prefix",
    "candidate_raw_token_transition",
    "eof_validity",
    "strict_decode",
    "recovery_decode",
    "consumed_raw_token_accounting",
    "admissible_next_raw_token_ids",
    "admissible_next_raw_token_ranges",
    "bpe_raw_expansion_policy",
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxGeneratedCodecStateApiHandoffGate {
    pub artifact_kind: String,
    pub version: u32,
    pub tool: String,
    pub status: String,
    pub handoff_id: String,
    pub syntax_boundary: String,
    pub target_neutral_surface: TargetNeutralStateApiSurface,
    pub evidence_sources: Vec<GeneratedCodecStateApiHandoffEvidenceSource>,
    pub claim_boundary: GeneratedCodecStateApiHandoffClaimBoundary,
    pub validation_commands: Vec<String>,
    pub completed_promotion_class: String,
    pub next_missing_stable_promotion_class: String,
}

impl SyntaxGeneratedCodecStateApiHandoffGate {
    pub fn validate(&self) -> Result<(), SyntaxBackendBoundaryError> {
        require_string("artifact_kind", &self.artifact_kind, ARTIFACT_KIND)?;
        require_u32("version", self.version, VERSION)?;
        require_string("tool", &self.tool, TOOL)?;
        require_string("status", &self.status, STATUS)?;
        require_string("handoff_id", &self.handoff_id, HANDOFF_ID)?;
        require_string("syntax_boundary", &self.syntax_boundary, SYNTAX_BOUNDARY)?;
        self.target_neutral_surface.validate()?;
        require_usize(
            "evidence_sources",
            self.evidence_sources.len(),
            REQUIRED_TARGETS.len(),
        )?;
        let observed_targets = self
            .evidence_sources
            .iter()
            .map(|source| source.target.as_str())
            .collect::<BTreeSet<_>>();
        let expected_targets = REQUIRED_TARGETS.iter().copied().collect::<BTreeSet<_>>();
        if observed_targets != expected_targets {
            return Err(SyntaxBackendBoundaryError::new(
                "evidence_sources.target",
                format!("expected targets {expected_targets:?}, got {observed_targets:?}"),
            ));
        }
        for source in &self.evidence_sources {
            source.validate()?;
        }
        self.claim_boundary.validate()?;
        require_all(
            "validation_commands",
            &self.validation_commands,
            &[
                "cargo test --test syntax_state_machine_replay_test",
                "cargo test --test syntax_backend_boundary_test syntax_backend_shared_generated_codec_state_api_handoff_consumes_go_and_csharp_state_api_evidence -- --exact",
                "cargo test --test syntax_generated_codec_gate_test csharp_compilation_unit_source_file_syntax_generated_codec_compiles_through_compiler_route -- --exact",
                "cargo test --test syntax_generated_codec_gate_test csharp_compilation_unit_logger_source_file_syntax_generated_codec_compiles_through_compiler_route -- --exact",
                "cargo test --test syntax_generated_codec_gate_test csharp_compilation_unit_version_one_source_file_syntax_generated_codec_compiles_through_compiler_route -- --exact",
                "cargo test --test syntax_generated_codec_gate_test csharp_compilation_unit_iterator_methods_source_file_syntax_generated_codec_compiles_through_compiler_route -- --exact",
                "cargo test --test syntax_generated_codec_gate_test csharp_compilation_unit_loan_source_file_syntax_generated_codec_compiles_through_compiler_route -- --exact",
                "git diff --check",
            ],
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

    pub fn validate_against_reports(
        &self,
        go_state_api: &SyntaxGeneratedCodecStateApiReport,
        go_strict_recovery: &SyntaxGeneratedCodecStrictRecoveryDecodeReport,
        csharp_real_source: &SyntaxCSharpRealSourceStateMachineParityReport,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        self.validate_against_report_sets(
            std::slice::from_ref(go_state_api),
            std::slice::from_ref(go_strict_recovery),
            csharp_real_source,
        )
    }

    pub fn validate_against_report_sets(
        &self,
        go_state_api_reports: &[SyntaxGeneratedCodecStateApiReport],
        go_strict_recovery_reports: &[SyntaxGeneratedCodecStrictRecoveryDecodeReport],
        csharp_real_source: &SyntaxCSharpRealSourceStateMachineParityReport,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        self.validate()?;
        csharp_real_source.validate()?;

        let go_source = self.source_for_target("go")?;
        go_source
            .validate_against_go_report_set(go_state_api_reports, go_strict_recovery_reports)?;

        let csharp_source = self.source_for_target(CSHARP_STATE_MACHINE_PARITY_TARGET)?;
        csharp_source.validate_against_csharp_report(csharp_real_source)?;
        Ok(())
    }

    fn source_for_target(
        &self,
        target: &'static str,
    ) -> Result<&GeneratedCodecStateApiHandoffEvidenceSource, SyntaxBackendBoundaryError> {
        self.evidence_sources
            .iter()
            .find(|source| source.target == target)
            .ok_or_else(|| {
                SyntaxBackendBoundaryError::new(
                    "evidence_sources.target",
                    format!("missing {target} handoff evidence source"),
                )
            })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TargetNeutralStateApiSurface {
    pub surface_id: String,
    pub capabilities: Vec<String>,
    pub prefix_state_api_ready: bool,
    pub transition_api_ready: bool,
    pub eof_api_ready: bool,
    pub strict_recovery_decode_ready: bool,
    pub consumed_token_accounting_ready: bool,
    pub admissible_raw_token_sets_ready: bool,
    pub admissible_raw_token_ranges_ready: bool,
    pub bpe_raw_expansion_policy: String,
    pub bpe_expansion_uses_transition_api: bool,
    pub bpe_layer_defines_validity: bool,
}

impl TargetNeutralStateApiSurface {
    fn validate(&self) -> Result<(), SyntaxBackendBoundaryError> {
        require_string(
            "target_neutral_surface.surface_id",
            &self.surface_id,
            "syntax-generated-codec-state-api-surface-v1",
        )?;
        require_all(
            "target_neutral_surface.capabilities",
            &self.capabilities,
            REQUIRED_SURFACE_CAPABILITIES,
        )?;
        require_bool(
            "target_neutral_surface.prefix_state_api_ready",
            self.prefix_state_api_ready,
            true,
        )?;
        require_bool(
            "target_neutral_surface.transition_api_ready",
            self.transition_api_ready,
            true,
        )?;
        require_bool(
            "target_neutral_surface.eof_api_ready",
            self.eof_api_ready,
            true,
        )?;
        require_bool(
            "target_neutral_surface.strict_recovery_decode_ready",
            self.strict_recovery_decode_ready,
            true,
        )?;
        require_bool(
            "target_neutral_surface.consumed_token_accounting_ready",
            self.consumed_token_accounting_ready,
            true,
        )?;
        require_bool(
            "target_neutral_surface.admissible_raw_token_sets_ready",
            self.admissible_raw_token_sets_ready,
            true,
        )?;
        require_bool(
            "target_neutral_surface.admissible_raw_token_ranges_ready",
            self.admissible_raw_token_ranges_ready,
            true,
        )?;
        require_string(
            "target_neutral_surface.bpe_raw_expansion_policy",
            &self.bpe_raw_expansion_policy,
            BPE_RAW_EXPANSION_POLICY,
        )?;
        require_bool(
            "target_neutral_surface.bpe_expansion_uses_transition_api",
            self.bpe_expansion_uses_transition_api,
            true,
        )?;
        require_bool(
            "target_neutral_surface.bpe_layer_defines_validity",
            self.bpe_layer_defines_validity,
            false,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeneratedCodecStateApiHandoffEvidenceSource {
    pub target: String,
    pub evidence_artifact_kind: String,
    pub evidence_status: String,
    pub source_report_paths: Vec<String>,
    pub route: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_record_raw_token_emission_api_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_record_raw_token_emission_api_boundary: Option<String>,
    pub source_record_count: usize,
    pub raw_token_count: usize,
    pub raw_token_ids_sha256: String,
    pub state_case_count: usize,
    pub transition_case_count: usize,
    pub strict_decode_case_count: usize,
    pub recovery_decode_case_count: usize,
    pub payload_range_state_count: usize,
    pub eof_valid_state_count: usize,
    pub supported_record_ids: Vec<String>,
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
    pub source_program_validity_claim_allowed: bool,
    pub type_correctness_claim_allowed: bool,
    pub stable_promotion_allowed: bool,
}

impl GeneratedCodecStateApiHandoffEvidenceSource {
    fn validate(&self) -> Result<(), SyntaxBackendBoundaryError> {
        require_non_empty("evidence_source.target", &self.target)?;
        match self.target.as_str() {
            "go" => {
                require_string(
                    "evidence_source.evidence_artifact_kind",
                    &self.evidence_artifact_kind,
                    GENERATED_CODEC_STATE_API_ARTIFACT_KIND,
                )?;
                require_string(
                    "evidence_source.evidence_status",
                    &self.evidence_status,
                    GENERATED_CODEC_STATE_API_STATUS,
                )?;
                require_usize(
                    "evidence_source.source_report_paths",
                    self.source_report_paths.len(),
                    self.source_record_count.saturating_mul(2),
                )?;
            }
            CSHARP_STATE_MACHINE_PARITY_TARGET => {
                require_string(
                    "evidence_source.evidence_artifact_kind",
                    &self.evidence_artifact_kind,
                    CSHARP_REAL_SOURCE_STATE_MACHINE_PARITY_ARTIFACT_KIND,
                )?;
                require_string(
                    "evidence_source.evidence_status",
                    &self.evidence_status,
                    CSHARP_REAL_SOURCE_STATE_MACHINE_PARITY_STATUS,
                )?;
                require_usize(
                    "evidence_source.source_report_paths",
                    self.source_report_paths.len(),
                    self.source_record_count,
                )?;
                require_usize(
                    "evidence_source.source_record_count",
                    self.source_record_count,
                    5,
                )?;
                require_string(
                    "evidence_source.source_record_raw_token_emission_api_id",
                    self.source_record_raw_token_emission_api_id
                        .as_deref()
                        .unwrap_or_default(),
                    CSHARP_SOURCE_RECORD_RAW_TOKEN_EMISSION_API_ID,
                )?;
                require_string(
                    "evidence_source.source_record_raw_token_emission_api_boundary",
                    self.source_record_raw_token_emission_api_boundary
                        .as_deref()
                        .unwrap_or_default(),
                    CSHARP_SOURCE_RECORD_RAW_TOKEN_EMISSION_API_BOUNDARY,
                )?;
            }
            other => {
                return Err(SyntaxBackendBoundaryError::new(
                    "evidence_source.target",
                    format!("unexpected target {other}"),
                ));
            }
        }
        require_non_empty(
            "evidence_source.evidence_artifact_kind",
            &self.evidence_artifact_kind,
        )?;
        require_non_empty("evidence_source.evidence_status", &self.evidence_status)?;
        require_positive_usize(
            "evidence_source.source_report_paths",
            self.source_report_paths.len(),
        )?;
        for path in &self.source_report_paths {
            require_contains("evidence_source.source_report_paths", path, "target/")?;
        }
        require_non_empty("evidence_source.route", &self.route)?;
        require_positive_usize(
            "evidence_source.source_record_count",
            self.source_record_count,
        )?;
        require_positive_usize("evidence_source.raw_token_count", self.raw_token_count)?;
        require_sha256(
            "evidence_source.raw_token_ids_sha256",
            &self.raw_token_ids_sha256,
        )?;
        require_positive_usize("evidence_source.state_case_count", self.state_case_count)?;
        require_positive_usize(
            "evidence_source.transition_case_count",
            self.transition_case_count,
        )?;
        require_positive_usize(
            "evidence_source.strict_decode_case_count",
            self.strict_decode_case_count,
        )?;
        require_positive_usize(
            "evidence_source.recovery_decode_case_count",
            self.recovery_decode_case_count,
        )?;
        require_positive_usize(
            "evidence_source.payload_range_state_count",
            self.payload_range_state_count,
        )?;
        require_positive_usize(
            "evidence_source.eof_valid_state_count",
            self.eof_valid_state_count,
        )?;
        require_usize(
            "evidence_source.supported_record_ids",
            self.supported_record_ids.len(),
            self.source_record_count,
        )?;
        require_bool(
            "evidence_source.prefix_state_api_ready",
            self.prefix_state_api_ready,
            true,
        )?;
        require_bool(
            "evidence_source.transition_api_ready",
            self.transition_api_ready,
            true,
        )?;
        require_bool(
            "evidence_source.eof_state_api_ready",
            self.eof_state_api_ready,
            true,
        )?;
        require_bool(
            "evidence_source.strict_recovery_decode_ready",
            self.strict_recovery_decode_ready,
            true,
        )?;
        require_bool(
            "evidence_source.consumed_token_accounting_ok",
            self.consumed_token_accounting_ok,
            true,
        )?;
        require_bool(
            "evidence_source.admissible_next_raw_token_sets_ready",
            self.admissible_next_raw_token_sets_ready,
            true,
        )?;
        require_bool(
            "evidence_source.admissible_next_raw_token_ranges_ready",
            self.admissible_next_raw_token_ranges_ready,
            true,
        )?;
        require_bool(
            "evidence_source.bpe_layer_defines_validity",
            self.bpe_layer_defines_validity,
            false,
        )?;
        require_bool(
            "evidence_source.target_metadata_sidecar_only",
            self.target_metadata_sidecar_only,
            true,
        )?;
        require_bool("evidence_source.arena_ir_used", self.arena_ir_used, false)?;
        require_bool(
            "evidence_source.source_program_validity_claim_allowed",
            self.source_program_validity_claim_allowed,
            false,
        )?;
        require_bool(
            "evidence_source.type_correctness_claim_allowed",
            self.type_correctness_claim_allowed,
            false,
        )?;
        require_bool(
            "evidence_source.stable_promotion_allowed",
            self.stable_promotion_allowed,
            false,
        )
    }

    fn validate_against_go_report_set(
        &self,
        state_api_reports: &[SyntaxGeneratedCodecStateApiReport],
        strict_recovery_reports: &[SyntaxGeneratedCodecStrictRecoveryDecodeReport],
    ) -> Result<(), SyntaxBackendBoundaryError> {
        require_string("go_source.target", &self.target, "go")?;
        require_string(
            "go_source.evidence_artifact_kind",
            &self.evidence_artifact_kind,
            GENERATED_CODEC_STATE_API_ARTIFACT_KIND,
        )?;
        require_string(
            "go_source.evidence_status",
            &self.evidence_status,
            GENERATED_CODEC_STATE_API_STATUS,
        )?;
        require_usize(
            "go_source.source_record_count",
            state_api_reports.len(),
            self.source_record_count,
        )?;
        require_usize(
            "go_source.strict_recovery_source_record_count",
            strict_recovery_reports.len(),
            self.source_record_count,
        )?;
        let mut raw_token_hashes = Vec::with_capacity(state_api_reports.len());
        let mut supported_record_ids = Vec::with_capacity(state_api_reports.len());
        let mut raw_token_count = 0usize;
        let mut state_case_count = 0usize;
        let mut transition_case_count = 0usize;
        let mut strict_decode_case_count = 0usize;
        let mut recovery_decode_case_count = 0usize;
        let mut aggregate_payload_range_state_count = 0usize;
        let mut aggregate_eof_valid_state_count = 0usize;
        for (state_api, strict_recovery) in
            state_api_reports.iter().zip(strict_recovery_reports.iter())
        {
            state_api.validate()?;
            strict_recovery.validate()?;
            require_string("go_source.route", &self.route, &state_api.route)?;
            require_string(
                "go_source.strict_recovery_artifact_kind",
                &strict_recovery.artifact_kind,
                GENERATED_CODEC_STRICT_RECOVERY_DECODE_ARTIFACT_KIND,
            )?;
            require_string(
                "go_source.strict_recovery_status",
                &strict_recovery.status,
                GENERATED_CODEC_STRICT_RECOVERY_DECODE_STATUS,
            )?;
            require_string(
                "go_source.strict_recovery.route",
                &strict_recovery.route,
                &state_api.route,
            )?;
            require_string(
                "go_source.strict_recovery.source_path",
                &strict_recovery.source_path,
                &state_api.source_path,
            )?;
            require_string(
                "go_source.strict_recovery.raw_token_ids_sha256",
                &strict_recovery.raw_token_ids_sha256,
                &state_api.raw_token_ids_sha256,
            )?;
            require_bool(
                "go_source.prefix_state_api_ready",
                state_api.prefix_state_api_ready,
                true,
            )?;
            require_bool(
                "go_source.strict_recovery_decode_ready",
                strict_recovery.strict_recovery_decode_wiring_ready,
                true,
            )?;
            require_bool(
                "go_source.consumed_token_accounting_ok",
                strict_recovery.consumed_token_accounting_ok,
                true,
            )?;
            require_bool(
                "go_source.bpe_layer_defines_validity",
                state_api.bpe_layer_defines_validity || strict_recovery.bpe_layer_defines_validity,
                false,
            )?;
            raw_token_hashes.push(state_api.raw_token_ids_sha256.clone());
            supported_record_ids.push(state_api.source_path.as_str());
            raw_token_count += state_api.raw_token_count;
            state_case_count += state_api.state_cases.len();
            transition_case_count += state_api.transition_cases.len();
            strict_decode_case_count += strict_recovery.strict_cases.len();
            recovery_decode_case_count += strict_recovery.recovery_cases.len();
            aggregate_payload_range_state_count +=
                payload_range_state_count(&state_api.state_cases);
            aggregate_eof_valid_state_count += eof_valid_state_count(&state_api.state_cases);
        }
        require_usize(
            "go_source.raw_token_count",
            self.raw_token_count,
            raw_token_count,
        )?;
        let expected_raw_token_hash = json_sha256(&raw_token_hashes).map_err(|error| {
            SyntaxBackendBoundaryError::new(
                "go_source.raw_token_ids_sha256",
                format!("failed to hash Go state API record hashes: {error}"),
            )
        })?;
        require_string(
            "go_source.raw_token_ids_sha256",
            &self.raw_token_ids_sha256,
            &expected_raw_token_hash,
        )?;
        require_usize(
            "go_source.state_case_count",
            self.state_case_count,
            state_case_count,
        )?;
        require_usize(
            "go_source.transition_case_count",
            self.transition_case_count,
            transition_case_count,
        )?;
        require_usize(
            "go_source.strict_decode_case_count",
            self.strict_decode_case_count,
            strict_decode_case_count,
        )?;
        require_usize(
            "go_source.recovery_decode_case_count",
            self.recovery_decode_case_count,
            recovery_decode_case_count,
        )?;
        require_usize(
            "go_source.payload_range_state_count",
            self.payload_range_state_count,
            aggregate_payload_range_state_count,
        )?;
        require_usize(
            "go_source.eof_valid_state_count",
            self.eof_valid_state_count,
            aggregate_eof_valid_state_count,
        )?;
        require_all(
            "go_source.supported_record_ids",
            &self.supported_record_ids,
            &supported_record_ids,
        )
    }

    fn validate_against_csharp_report(
        &self,
        report: &SyntaxCSharpRealSourceStateMachineParityReport,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        require_string(
            "csharp_source.target",
            &self.target,
            CSHARP_STATE_MACHINE_PARITY_TARGET,
        )?;
        require_string(
            "csharp_source.evidence_artifact_kind",
            &self.evidence_artifact_kind,
            CSHARP_REAL_SOURCE_STATE_MACHINE_PARITY_ARTIFACT_KIND,
        )?;
        require_string(
            "csharp_source.evidence_status",
            &self.evidence_status,
            CSHARP_REAL_SOURCE_STATE_MACHINE_PARITY_STATUS,
        )?;
        require_exact_strings(
            "csharp_source.source_report_paths",
            &self.source_report_paths,
            &report.source_report_paths,
        )?;
        require_string("csharp_source.route", &self.route, &report.route)?;
        require_string(
            "csharp_source.source_record_raw_token_emission_api_id",
            self.source_record_raw_token_emission_api_id
                .as_deref()
                .unwrap_or_default(),
            &report.source_record_raw_token_emission_api_id,
        )?;
        require_string(
            "csharp_source.source_record_raw_token_emission_api_boundary",
            self.source_record_raw_token_emission_api_boundary
                .as_deref()
                .unwrap_or_default(),
            &report.source_record_raw_token_emission_api_boundary,
        )?;
        require_usize(
            "csharp_source.source_record_count",
            self.source_record_count,
            report.corpus_replay_record_count,
        )?;
        require_usize(
            "csharp_source.raw_token_count",
            self.raw_token_count,
            report
                .corpus_replay_records
                .iter()
                .map(|record| record.structural_replay_token_stream.raw_token_count)
                .sum(),
        )?;
        let csharp_record_hashes = report
            .corpus_replay_records
            .iter()
            .map(|record| {
                record
                    .structural_replay_token_stream
                    .raw_token_ids_sha256
                    .clone()
            })
            .collect::<Vec<_>>();
        let expected_hash = json_sha256(&csharp_record_hashes).map_err(|error| {
            SyntaxBackendBoundaryError::new(
                "csharp_source.raw_token_ids_sha256",
                format!("failed to hash C# corpus replay record hashes: {error}"),
            )
        })?;
        require_string(
            "csharp_source.raw_token_ids_sha256",
            &self.raw_token_ids_sha256,
            &expected_hash,
        )?;
        require_usize(
            "csharp_source.state_case_count",
            self.state_case_count,
            report
                .corpus_replay_records
                .iter()
                .map(|record| record.state_cases.len())
                .sum(),
        )?;
        require_usize(
            "csharp_source.transition_case_count",
            self.transition_case_count,
            report
                .corpus_replay_records
                .iter()
                .map(|record| record.transition_cases.len())
                .sum(),
        )?;
        require_usize(
            "csharp_source.strict_decode_case_count",
            self.strict_decode_case_count,
            report
                .corpus_replay_records
                .iter()
                .map(|record| record.strict_cases.len())
                .sum(),
        )?;
        require_usize(
            "csharp_source.recovery_decode_case_count",
            self.recovery_decode_case_count,
            report
                .corpus_replay_records
                .iter()
                .map(|record| record.recovery_cases.len())
                .sum(),
        )?;
        require_usize(
            "csharp_source.payload_range_state_count",
            self.payload_range_state_count,
            report
                .corpus_replay_records
                .iter()
                .map(|record| payload_range_state_count(&record.state_cases))
                .sum(),
        )?;
        require_usize(
            "csharp_source.eof_valid_state_count",
            self.eof_valid_state_count,
            report
                .corpus_replay_records
                .iter()
                .map(|record| eof_valid_state_count(&record.state_cases))
                .sum(),
        )?;
        let expected_records = report
            .corpus_replay_records
            .iter()
            .map(|record| record.manifest_path.as_str())
            .collect::<Vec<_>>();
        require_all(
            "csharp_source.supported_record_ids",
            &self.supported_record_ids,
            &expected_records,
        )?;
        require_bool(
            "csharp_source.prefix_state_api_ready",
            report
                .corpus_replay_records
                .iter()
                .all(|record| record.prefix_state_api_ready),
            true,
        )?;
        require_bool(
            "csharp_source.admissible_next_raw_tokens_ready",
            report
                .corpus_replay_records
                .iter()
                .all(|record| record.admissible_next_raw_tokens_ready),
            true,
        )?;
        require_bool(
            "csharp_source.eof_state_api_ready",
            report
                .corpus_replay_records
                .iter()
                .all(|record| record.eof_state_api_ready),
            true,
        )?;
        require_bool(
            "csharp_source.strict_recovery_decode_ready",
            report
                .corpus_replay_records
                .iter()
                .all(|record| record.strict_recovery_decode_agreement_ready),
            true,
        )?;
        require_bool(
            "csharp_source.consumed_token_accounting_ok",
            report
                .corpus_replay_records
                .iter()
                .all(|record| record.consumed_token_accounting_ok),
            true,
        )?;
        require_bool(
            "csharp_source.bpe_layer_defines_validity",
            report.bpe_layer_defines_validity(),
            false,
        )?;
        require_bool("csharp_source.arena_ir_used", report.arena_ir_used, false)?;
        require_bool(
            "csharp_source.source_program_validity_claim_allowed",
            report.source_program_validity_claim_allowed,
            false,
        )?;
        require_bool(
            "csharp_source.type_correctness_claim_allowed",
            report.type_correctness_claim_allowed,
            false,
        )?;
        require_bool(
            "csharp_source.stable_promotion_allowed",
            report.stable_promotion_allowed,
            false,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeneratedCodecStateApiHandoffClaimBoundary {
    pub syntax_level_boundary: String,
    pub target_neutral_handoff_only: bool,
    pub tct_compiler_routing_used: bool,
    pub grammar_ir_or_structural_model_routed_through_tct_compiler: bool,
    pub source_program_validity_claim_allowed: bool,
    pub type_correctness_claim_allowed: bool,
    pub generated_hot_path_claim_allowed: bool,
    pub stable_backend_promotion_allowed: bool,
    pub arena_ir_proof_used: bool,
    pub bpe_layer_validity_claim_allowed: bool,
    pub json_schema_backend_changed: bool,
    pub no_stable_claims: Vec<String>,
}

impl GeneratedCodecStateApiHandoffClaimBoundary {
    fn validate(&self) -> Result<(), SyntaxBackendBoundaryError> {
        require_string(
            "claim_boundary.syntax_level_boundary",
            &self.syntax_level_boundary,
            SYNTAX_BOUNDARY,
        )?;
        require_bool(
            "claim_boundary.target_neutral_handoff_only",
            self.target_neutral_handoff_only,
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

fn payload_range_state_count(
    state_cases: &[crate::syntax_state_machine_replay::SyntaxGeneratedCodecStateCase],
) -> usize {
    state_cases
        .iter()
        .filter(|case| !case.admissible_next_raw_token_ranges.is_empty())
        .count()
}

fn eof_valid_state_count(
    state_cases: &[crate::syntax_state_machine_replay::SyntaxGeneratedCodecStateCase],
) -> usize {
    state_cases.iter().filter(|case| case.eof_valid).count()
}

trait CSharpReportBpeValidity {
    fn bpe_layer_defines_validity(&self) -> bool;
}

impl CSharpReportBpeValidity for SyntaxCSharpRealSourceStateMachineParityReport {
    fn bpe_layer_defines_validity(&self) -> bool {
        false
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

fn require_contains(
    field: &'static str,
    actual: &str,
    expected_substring: &str,
) -> Result<(), SyntaxBackendBoundaryError> {
    if actual.contains(expected_substring) {
        Ok(())
    } else {
        Err(SyntaxBackendBoundaryError::new(
            field,
            format!("expected value to contain {expected_substring:?}"),
        ))
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

fn require_sha256(field: &'static str, value: &str) -> Result<(), SyntaxBackendBoundaryError> {
    if value.len() == 64 && value.chars().all(|c| c.is_ascii_hexdigit()) {
        Ok(())
    } else {
        Err(SyntaxBackendBoundaryError::new(
            field,
            "value must be a SHA-256 hex digest",
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

fn require_exact_strings(
    field: &'static str,
    actual: &[String],
    expected: &[String],
) -> Result<(), SyntaxBackendBoundaryError> {
    if actual == expected {
        Ok(())
    } else {
        Err(SyntaxBackendBoundaryError::new(
            field,
            format!("expected {expected:?}, got {actual:?}"),
        ))
    }
}
