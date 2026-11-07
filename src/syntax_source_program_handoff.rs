//! Source-program claim handoff profile for the syntax production route.
//!
//! This gate records the claim boundary after feature-limited stable syntax
//! metadata promotion. It allows generated-codec validity claims for checked
//! parser-backed source-corpus records only. It does not promote arena IR
//! controls, parser/formatter/type-checker sidecars, whole-source-language
//! validity, cross-repo paper evidence, or model-quality evidence.

use crate::provenance::json_sha256;
use crate::syntax_backend_boundary::SyntaxBackendBoundaryError;
use crate::syntax_csharp_real_source_generated_codec_completion_handoff::{
    self, SyntaxCSharpRealSourceGeneratedCodecCompletionHandoffGate,
};
use crate::syntax_go_real_source_generated_codec_completion_handoff::{
    self, SyntaxGoRealSourceGeneratedCodecCompletionHandoffGate,
};
use crate::syntax_go_real_source_generated_codec_corpus::{
    GeneratedCodecCorpusSupportedRecord, WHOLE_FILE_GENERATED_CODEC_CLAIM,
    WHOLE_FILE_GENERATED_CODEC_CLAIM_SCOPE,
};
use crate::syntax_production_route::{self, SyntaxProductionRouteImplementationGate};
use crate::syntax_route_manifest::{self, SyntaxProductionRouteManifest};
use crate::syntax_source_corpus_evidence::{
    self, SyntaxSourceCorpusCSharpNoGapMetadataReport, SyntaxSourceCorpusEvidenceGate,
    SyntaxSourceCorpusEvidenceRecord, SyntaxSourceCorpusEvidenceReport,
    SyntaxSourceCorpusRouteEvidence,
};
use crate::syntax_stable_metadata_promotion::{
    self, SyntaxStableMetadataPromotionImplementationGate,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeSet;

pub const TOOL: &str = "syntax_backend_source_program_claim_handoff";
pub const VERSION: u32 = 1;
pub const ARTIFACT_KIND: &str = "tct_syntax_backend_source_program_claim_handoff";
pub const FIXTURE_PATH: &str = "tests/fixtures/syntax_backend/source-program-claim-handoff-v1.json";
pub const STATUS: &str = "checked_handoff_profile";
pub const PROFILE_ID: &str = "syntax-source-program-claim-handoff-v1";
pub const CLAIM_LEVEL: &str = "compiler_evidence_source_program_handoff_profile";
pub const COMPLETED_PROMOTION_CLASS: &str = "syntax backend source-program claim handoff gate";
pub const REPORT_PRODUCER_COMPLETED_PROMOTION_CLASS: &str =
    "syntax backend source-program handoff report producer gate";
pub const NEXT_MISSING_STABLE_PROMOTION_CLASS: &str =
    "syntax backend source-program external validation control evidence gate";
pub const REPORT_ARTIFACT_KIND: &str = "tct_syntax_backend_source_program_claim_handoff_report";
pub const REPORT_STATUS: &str = "generated_handoff_report";
pub const REPORT_PATH: &str =
    "target/tct-syntax-source-program-handoff/source-program-claim-handoff-v1.json";
pub const REPORT_PRODUCER_COMMAND: &str = "cargo run --bin tct-syntax-source-program-handoff -- --source-corpus-report target/tct-syntax-source-corpus-evidence/parser-backed-source-corpus-evidence-v1.json --output target/tct-syntax-source-program-handoff/source-program-claim-handoff-v1.json";
pub const REPORT_VALIDATOR_COMMAND: &str = "cargo run --bin tct-syntax-source-program-handoff -- --validate-report target/tct-syntax-source-program-handoff/source-program-claim-handoff-v1.json";
pub const SYNTAX_LEVEL_BOUNDARY: &str =
    "parser-backed metadata -> GrammarIR -> StructuralModel -> token layout -> generated codec";

/// Stable report-field names preserved from the retired source-program
/// type-checker execution evidence family
/// (`src/syntax_source_program_typecheck_execution_preservation.rs`, removed
/// 2026-06). The retained handoff and corpus gates still pin these field names
/// as historical fixture metadata; no live typecheck report is produced or
/// consumed anymore.
pub(crate) const TYPECHECK_PRESERVATION_REPORT_STABLE_FIELDS: &[&str] = &[
    "artifact_kind",
    "version",
    "tool",
    "status",
    "source_feasibility_report.report_path",
    "source_feasibility_report.artifact_kind",
    "source_feasibility_report.tool",
    "source_feasibility_report.status",
    "source_feasibility_report.record_count",
    "source_feasibility_report.records_sha256",
    "source_feasibility_report.stable_metadata_promotion",
    "source_feasibility_report.go_generated_codec_completion_handoff",
    "source_feasibility_report.csharp_generated_codec_completion_handoff",
    "source_feasibility_report.stable_metadata_promotion.source_completion_handoff_source_program_typecheck_execution_report_path",
    "source_feasibility_report.stable_metadata_promotion.source_completion_handoff_source_program_typecheck_feasibility_report_path",
    "source_feasibility_report.stable_metadata_promotion.source_completion_handoff_source_program_typecheck_execution_completed_promotion_class",
    "source_feasibility_report.stable_metadata_promotion.source_completion_handoff_source_program_typecheck_execution_next_missing_stable_promotion_class",
    "source_feasibility_report.stable_metadata_promotion.source_completion_handoff_source_program_preserved_typecheck_stable_report_fields",
    "source_feasibility_report.stable_metadata_promotion.source_completion_handoff_source_program_preserved_typecheck_report_contract_stable_report_fields",
    "source_feasibility_report.stable_metadata_promotion.source_completion_handoff_source_program_preserved_typecheck_blocked_claims",
    "source_feasibility_report.stable_metadata_promotion.source_completion_handoff_source_program_preserved_typecheck_no_stable_claims",
    "source_feasibility_report.stable_metadata_promotion.source_completion_handoff_source_program_execution_pass_envelope_required_record_count",
    "source_feasibility_report.stable_metadata_promotion.source_completion_handoff_source_program_execution_pass_envelope_required_record_status",
    "source_feasibility_report.stable_metadata_promotion.source_completion_handoff_source_program_execution_pass_envelope_required_records_all_passed",
    "source_feasibility_report.stable_metadata_promotion.source_completion_handoff_source_program_execution_pass_envelope_required_record_ids",
    "source_feasibility_report.stable_metadata_promotion.source_completion_handoff_source_program_typechecker_results_sidecar_only",
    "source_feasibility_report.stable_metadata_promotion.source_completion_handoff_source_program_type_correctness_claim_allowed",
    "source_feasibility_report.stable_metadata_promotion.source_completion_handoff_source_program_typechecker_arena_ir_proof_used",
    "source_feasibility_report.stable_metadata_promotion.source_completion_handoff_source_program_typechecker_json_schema_backend_unchanged",
    "source_feasibility_report.stable_metadata_promotion.source_completion_handoff_source_program_corpus_check_report_path",
    "source_feasibility_report.stable_metadata_promotion.source_completion_handoff_source_program_corpus_eligible_record_count",
    "source_feasibility_report.stable_metadata_promotion.source_completion_handoff_source_program_corpus_unsupported_record_count",
    "source_feasibility_report.stable_metadata_promotion.source_completion_handoff_source_program_sidecar_validity_claim_allowed",
    "source_feasibility_report.stable_metadata_promotion.source_completion_handoff_source_program_sidecar_arena_ir_proof_used",
    "source_feasibility_report.stable_metadata_promotion.source_completion_handoff_source_program_sidecar_json_schema_backend_unchanged",
    "source_feasibility_report.stable_metadata_promotion.source_completion_handoff_source_program_no_stable_claims",
    "source_feasibility_report.stable_metadata_promotion.source_completion_handoff_generated_codec_report_paths",
    "source_feasibility_report.stable_metadata_promotion.source_completion_handoff_generated_codec_report_commands",
    "source_feasibility_report.stable_metadata_promotion.source_completion_handoff_generated_codec_round_trip_records",
    "source_feasibility_report.stable_metadata_promotion.source_completion_handoff_generated_codec_allowed_claims",
    "source_feasibility_report.stable_metadata_promotion.source_completion_handoff_generated_codec_whole_file_claim_scope",
    "source_feasibility_report.stable_metadata_promotion.source_completion_handoff_generated_codec_no_stable_claims",
    "source_feasibility_report.completed_promotion_class",
    "source_feasibility_report.next_missing_stable_promotion_class",
    "control_boundary.syntax_level_boundary",
    "control_boundary.control_evidence_only",
    "control_boundary.generated_codec_validity_claim_only",
    "control_boundary.source_program_claim_allowed",
    "control_boundary.paper_claim_allowed",
    "control_boundary.arena_ir_proof_allowed",
    "control_boundary.external_sidecar_stable_promotion_allowed",
    "control_boundary.generated_hot_path_external_checks_allowed",
    "control_boundary.json_schema_backend_preserved",
    "tool_envelope.tct_version",
    "tool_envelope.observed_tools.tool_id",
    "tool_envelope.observed_tools.command",
    "tool_envelope.observed_tools.available",
    "target_summary.target_id",
    "target_summary.tool_id",
    "target_summary.record_count",
    "target_summary.ran_records",
    "target_summary.passed_records",
    "target_summary.failed_records",
    "target_summary.blocked_records",
    "target_summary.execution_status",
    "record_count",
    "records.target_id",
    "records.grammar_id",
    "records.source_path",
    "records.source_sha256",
    "records.metadata_path",
    "records.metadata_sha256",
    "records.typechecker_tool",
    "records.planned_command_shape",
    "records.command",
    "records.wrapping_harness_kind",
    "records.ran",
    "records.success",
    "records.exit_code",
    "records.execution_status",
    "records.blockers",
    "records.type_correctness_claim_allowed",
    "records.stable_promotion_allowed",
    "records.generated_hot_path_allowed",
    "blocked_claims",
    "validation_commands",
    "completed_promotion_class",
    "next_missing_stable_promotion_class",
];

const REQUIRED_ALLOWED_CLAIMS: &[&str] = &[
    "generated_codec_validity_for_checked_parser_backed_syntax_route",
    "generated_codec_validity_for_checked_go_real_source_corpus_records",
    "generated_codec_validity_for_supported_csharp_real_source_records",
    WHOLE_FILE_GENERATED_CODEC_CLAIM,
    "parser_backed_source_corpus_identity_preserved",
    "csharp_real_source_generated_codec_corpus_completion_evidence_consumed",
    "stable_route_metadata_promoted",
    "route_owned_stable_metadata_preserved",
];
const REQUIRED_BLOCKED_CLAIMS: &[&str] = &[
    "whole_source_language_validity",
    "cross_repo_paper_claim",
    "model_quality_claim",
    "functional_correctness_claim",
    "source_completion_or_infill_claim",
    "type_correctness_claim",
    "arena_ir_promotion",
    "external_sidecar_stable_promotion",
    "generated_hot_path_claim",
    "generated_json_schema_output_change",
];
const REQUIRED_EVIDENCE: &[&str] = &[
    "parser_backed_metadata",
    "GrammarIR",
    "StructuralModel",
    "token_layout",
    "generated_codec",
    "source_corpus_report",
    "csharp_no_gap_metadata_report",
    "source_sha256",
    "metadata_sha256",
    "parser_tool_identity",
    "generated_artifact_hashes",
    "route_metadata",
    "validation_commands",
    "json_schema_backend_preservation",
    "no_arena_ir_proof",
    "stable_metadata_promotion_implementation_gate",
    "go_real_source_generated_codec_completion_handoff",
    "csharp_real_source_generated_codec_completion_handoff",
];
const REQUIRED_SUMMARY_FIELDS: &[&str] = &[
    "artifact_kind",
    "version",
    "tool",
    "status",
    "profile_id",
    "source_route",
    "stable_metadata_promotion",
    "go_generated_codec_completion_handoff",
    "csharp_generated_codec_completion_handoff",
    "source_corpus",
    "source_corpus.csharp_no_gap_metadata_report",
    "claim_boundary",
    "record_count",
    "records_sha256",
    "records",
    "allowed_claims",
    "blocked_claims",
    "validation_commands",
    "completed_promotion_class",
    "next_missing_stable_promotion_class",
];
const REQUIRED_RECORD_FIELDS: &[&str] = &[
    "target_id",
    "grammar_id",
    "source_path",
    "source_sha256",
    "metadata_path",
    "metadata_sha256",
    "parser_tool",
    "parser_version",
    "structural_model_sha256",
    "token_layout_sha256",
    "rendered_source_sha256",
    "generated_codec_metadata_sha256",
    "generated_crate_identity_sha256",
    "generation_options_sha256",
    "route_metadata",
    "validation_hook_status",
    "sidecar_controls",
];

const REQUIRED_STABLE_METADATA_SOURCE_MANIFEST_PATHS: &[&str] = &[
    "strings/compare.go",
    "crypto/internal/sysrand/rand_windows.go",
    "strings/clone.go",
    "sort/example_interface_test.go",
    "sort/example_search_test.go",
    "strings/search.go",
    "path/match.go",
];

const REQUIRED_STABLE_METADATA_GENERATED_CODEC_ROUTES: &[&str] = &[
    "go_strings_compare_source_file_syntax_v0",
    "go_crypto_sysrand_rand_windows_source_file_syntax_v0",
    "go_strings_clone_source_file_syntax_v0",
    "go_sort_interface_source_file_syntax_v0",
    "go_sort_search_source_file_syntax_v0",
    "go_strings_search_source_file_syntax_v0",
    "go_path_match_source_file_syntax_v0",
];

const REQUIRED_STABLE_METADATA_GENERATED_CODEC_MARKERS: &[&str] = &[
    "generated_go_strings_compare_source_file_round_trips",
    "generated_go_crypto_sysrand_rand_windows_source_file_round_trips",
    "generated_go_strings_clone_source_file_round_trips",
    "generated_go_sort_interface_source_file_round_trips",
    "generated_go_sort_search_source_file_round_trips",
    "generated_go_strings_search_source_file_round_trips",
    "generated_go_path_match_source_file_round_trips",
];

const REQUIRED_STABLE_METADATA_SOURCE_PROGRAM_REPORT_FIELDS: &[&str] = &[
    "source_feasibility_report.stable_metadata_promotion",
    "source_feasibility_report.go_generated_codec_completion_handoff",
    "source_feasibility_report.csharp_generated_codec_completion_handoff",
];

const REQUIRED_STABLE_METADATA_SOURCE_PROGRAM_BLOCKED_CLAIMS: &[&str] = &[
    "whole_source_language_validity",
    "cross_repo_paper_claim",
    "model_quality_claim",
    "functional_correctness_claim",
    "source_completion_or_infill_claim",
    "type_correctness_claim",
    "arena_ir_promotion",
    "external_sidecar_stable_promotion",
    "generated_hot_path_claim",
    "generated_json_schema_output_change",
];

const REQUIRED_STABLE_METADATA_SOURCE_PROGRAM_NO_STABLE_CLAIMS: &[&str] = &[
    "no_source_program_validity_claim",
    "no_whole_source_language_validity_claim",
    "no_cross_repo_paper_claim",
    "no_model_quality_claim",
    "no_functional_correctness_claim",
    "no_type_correctness_claim",
    "no_completion_or_infill_claim",
    "no_arena_ir_promotion",
    "no_external_sidecar_stable_promotion",
    "no_generated_hot_path_claim",
    "no_generated_json_schema_output_change",
];

const REQUIRED_STABLE_METADATA_SOURCE_PROGRAM_TYPECHECK_NO_STABLE_CLAIMS: &[&str] = &[
    "no_source_program_validity_claim",
    "no_whole_source_language_validity_claim",
    "no_cross_repo_paper_claim",
    "no_model_quality_claim",
    "no_functional_correctness_claim",
    "no_type_correctness_claim",
    "no_completion_or_infill_claim",
    "no_arena_ir_promotion",
    "no_external_sidecar_stable_promotion",
    "no_generated_hot_path_typechecker",
    "no_generated_json_schema_output_change",
];

const REQUIRED_STABLE_METADATA_GENERATED_CODEC_NO_STABLE_CLAIMS: &[&str] = &[
    "no_source_program_validity_claim",
    "no_whole_source_language_validity_claim",
    "no_cross_repo_paper_claim",
    "no_model_quality_claim",
    "no_functional_correctness_claim",
    "no_type_correctness_claim",
    "no_completion_or_infill_claim",
    "no_arena_ir_promotion",
    "no_external_sidecar_stable_promotion",
    "no_generated_hot_path_claim",
    "no_generated_json_schema_output_change",
];

const REQUIRED_STABLE_METADATA_GENERATED_CODEC_ALLOWED_CLAIMS: &[&str] = &[
    "generated_codec_validity_for_checked_go_real_source_corpus_records",
    WHOLE_FILE_GENERATED_CODEC_CLAIM,
    "parser_backed_source_corpus_identity_preserved",
    "stable_metadata_promotion_prerequisite_identified",
];

const REQUIRED_VALIDATION_COMMANDS: &[&str] = &[
    "cargo test --test syntax_backend_boundary_test",
    "cargo test --test maintained_cli_binaries_test syntax_source_program_handoff_cli_writes_report -- --exact",
    "python3 scripts/testing/test-quality-audit.py --fail-on-gaps",
    "git diff --check",
];

const REQUIRED_GENERATED_CODEC_REPORT_PATHS: &[&str] = &[
    "target/tct-syntax-real-source-demo/go-strings-compare-generated-codec-demo.json",
    "target/tct-syntax-real-source-demo/go-crypto-sysrand-rand-windows-generated-codec-demo.json",
    "target/tct-syntax-real-source-demo/go-strings-clone-generated-codec-demo.json",
    "target/tct-syntax-real-source-demo/go-sort-interface-generated-codec-demo.json",
    "target/tct-syntax-real-source-demo/go-sort-search-generated-codec-demo.json",
    "target/tct-syntax-real-source-demo/go-strings-search-generated-codec-demo.json",
    "target/tct-syntax-real-source-demo/go-path-match-generated-codec-demo.json",
];

const REQUIRED_GENERATED_CODEC_REPORT_COMMANDS: &[&str] = &[
    "cargo test --test syntax_generated_codec_gate_test go_strings_compare_source_file_syntax_generated_codec_compiles_at_sidecar_boundary -- --exact",
    "cargo test --test syntax_generated_codec_gate_test go_crypto_sysrand_rand_windows_source_file_syntax_generated_codec_compiles_at_sidecar_boundary -- --exact",
    "cargo test --test syntax_generated_codec_gate_test go_strings_clone_source_file_syntax_generated_codec_compiles_at_sidecar_boundary -- --exact",
    "cargo test --test syntax_generated_codec_gate_test go_sort_interface_source_file_syntax_generated_codec_compiles_at_sidecar_boundary -- --exact",
    "cargo test --test syntax_generated_codec_gate_test go_sort_search_source_file_syntax_generated_codec_compiles_at_sidecar_boundary -- --exact",
    "cargo test --test syntax_generated_codec_gate_test go_strings_search_source_file_syntax_generated_codec_compiles_at_sidecar_boundary -- --exact",
    "cargo test --test syntax_generated_codec_gate_test go_path_match_source_file_syntax_generated_codec_compiles_at_sidecar_boundary -- --exact",
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxSourceProgramClaimHandoffGate {
    pub artifact_kind: String,
    pub version: u32,
    pub tool: String,
    pub status: String,
    pub profile_id: String,
    pub source_route: SyntaxSourceProgramHandoffSourceRoute,
    pub stable_metadata_promotion: SyntaxSourceProgramStableMetadataPromotionInput,
    pub go_generated_codec_completion_handoff:
        SyntaxSourceProgramGoGeneratedCodecCompletionHandoffInput,
    pub csharp_generated_codec_completion_handoff:
        SyntaxSourceProgramCSharpGeneratedCodecCompletionHandoffInput,
    pub claim_profile: SyntaxSourceProgramClaimProfile,
    pub handoff_report_contract: SyntaxSourceProgramHandoffReportContract,
    pub claim_boundary: SyntaxSourceProgramClaimBoundary,
    pub completed_promotion_class: String,
    pub next_missing_stable_promotion_class: String,
}

impl SyntaxSourceProgramClaimHandoffGate {
    pub fn validate_against_source_corpus(
        &self,
        route_gate: &SyntaxProductionRouteImplementationGate,
        route_manifest: &SyntaxProductionRouteManifest,
        source_corpus_gate: &SyntaxSourceCorpusEvidenceGate,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        source_corpus_gate.validate_against_production_route_gate(route_gate, route_manifest)?;
        require_string("artifact_kind", &self.artifact_kind, ARTIFACT_KIND)?;
        require_u32("version", self.version, VERSION)?;
        require_string("tool", &self.tool, TOOL)?;
        require_string("status", &self.status, STATUS)?;
        require_string("profile_id", &self.profile_id, PROFILE_ID)?;
        self.source_route
            .validate(route_gate, route_manifest, source_corpus_gate)?;
        self.stable_metadata_promotion.validate_static()?;
        self.go_generated_codec_completion_handoff
            .validate_static()?;
        self.csharp_generated_codec_completion_handoff
            .validate_static()?;
        self.claim_profile.validate()?;
        self.handoff_report_contract.validate(source_corpus_gate)?;
        self.claim_boundary.validate()?;
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

    pub fn validate_against_promotion_inputs(
        &self,
        route_gate: &SyntaxProductionRouteImplementationGate,
        route_manifest: &SyntaxProductionRouteManifest,
        source_corpus_gate: &SyntaxSourceCorpusEvidenceGate,
        stable_metadata_gate: &SyntaxStableMetadataPromotionImplementationGate,
        go_completion_handoff_gate: &SyntaxGoRealSourceGeneratedCodecCompletionHandoffGate,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        self.validate_against_source_corpus(route_gate, route_manifest, source_corpus_gate)?;
        self.stable_metadata_promotion
            .validate_against_gate(stable_metadata_gate, go_completion_handoff_gate)?;
        self.go_generated_codec_completion_handoff
            .validate_against_gate(go_completion_handoff_gate)?;
        require_string(
            "stable_metadata_promotion.source_fixture_next_missing_stable_promotion_class",
            &stable_metadata_gate.next_missing_stable_promotion_class,
            syntax_stable_metadata_promotion::NEXT_MISSING_STABLE_PROMOTION_CLASS,
        )?;
        require_string(
            "go_generated_codec_completion_handoff.source_fixture_completed_promotion_class",
            &go_completion_handoff_gate.completed_promotion_class,
            &self
                .stable_metadata_promotion
                .source_completion_handoff_completed_class,
        )
    }

    pub fn validate_against_csharp_generated_codec_completion_handoff(
        &self,
        csharp_completion_handoff_gate: &SyntaxCSharpRealSourceGeneratedCodecCompletionHandoffGate,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        self.csharp_generated_codec_completion_handoff
            .validate_against_gate(csharp_completion_handoff_gate)
    }

    pub fn build_report(
        &self,
        route_gate: &SyntaxProductionRouteImplementationGate,
        route_manifest: &SyntaxProductionRouteManifest,
        source_corpus_gate: &SyntaxSourceCorpusEvidenceGate,
        source_corpus_report_path: &str,
        source_corpus_report: &SyntaxSourceCorpusEvidenceReport,
    ) -> Result<SyntaxSourceProgramHandoffReport, SyntaxBackendBoundaryError> {
        self.validate_against_source_corpus(route_gate, route_manifest, source_corpus_gate)?;
        source_corpus_gate.validate_report(route_gate, route_manifest, source_corpus_report)?;
        let source_corpus_report_sha256 = json_sha256(source_corpus_report).map_err(|error| {
            SyntaxBackendBoundaryError::new(
                "source_corpus_report",
                format!("failed to hash source corpus report: {error}"),
            )
        })?;
        let records = source_corpus_report
            .records
            .iter()
            .map(source_program_handoff_record)
            .collect::<Result<Vec<_>, _>>()?;
        let records_sha256 = json_sha256(&records).map_err(|error| {
            SyntaxBackendBoundaryError::new(
                "records",
                format!("failed to hash source-program handoff records: {error}"),
            )
        })?;
        Ok(SyntaxSourceProgramHandoffReport {
            artifact_kind: REPORT_ARTIFACT_KIND.to_string(),
            version: VERSION,
            tool: TOOL.to_string(),
            status: REPORT_STATUS.to_string(),
            profile_id: self.profile_id.clone(),
            source_route: self.source_route.clone(),
            stable_metadata_promotion: self.stable_metadata_promotion.clone(),
            go_generated_codec_completion_handoff: self
                .go_generated_codec_completion_handoff
                .clone(),
            csharp_generated_codec_completion_handoff: self
                .csharp_generated_codec_completion_handoff
                .clone(),
            source_corpus: SyntaxSourceProgramHandoffSourceCorpusReport {
                report_path: source_corpus_report_path.to_string(),
                report_sha256: source_corpus_report_sha256,
                artifact_kind: source_corpus_report.artifact_kind.clone(),
                tool: source_corpus_report.tool.clone(),
                status: source_corpus_report.status.clone(),
                corpus_id: source_corpus_report.corpus_id.clone(),
                record_count: source_corpus_report.record_count,
                records_sha256: source_corpus_report.records_sha256.clone(),
                missing_evidence: source_corpus_report.missing_evidence.clone(),
                csharp_no_gap_metadata_report: source_corpus_report
                    .csharp_no_gap_metadata_report
                    .clone(),
                next_missing_stable_promotion_class: source_corpus_report
                    .next_missing_stable_promotion_class
                    .clone(),
            },
            claim_boundary: self.claim_boundary.clone(),
            record_count: records.len(),
            records_sha256,
            records,
            allowed_claims: self.claim_profile.allowed_claims.clone(),
            blocked_claims: self.claim_profile.blocked_claims.clone(),
            validation_commands: self
                .handoff_report_contract
                .required_validation_commands
                .clone(),
            completed_promotion_class: REPORT_PRODUCER_COMPLETED_PROMOTION_CLASS.to_string(),
            next_missing_stable_promotion_class: self.next_missing_stable_promotion_class.clone(),
        })
    }

    pub fn validate_report(
        &self,
        report: &SyntaxSourceProgramHandoffReport,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        require_string(
            "report.artifact_kind",
            &report.artifact_kind,
            REPORT_ARTIFACT_KIND,
        )?;
        require_u32("report.version", report.version, VERSION)?;
        require_string("report.tool", &report.tool, TOOL)?;
        require_string("report.status", &report.status, REPORT_STATUS)?;
        require_string("report.profile_id", &report.profile_id, PROFILE_ID)?;
        require_source_route_equal(
            "report.source_route",
            &report.source_route,
            &self.source_route,
        )?;
        require_stable_metadata_promotion_equal(
            "report.stable_metadata_promotion",
            &report.stable_metadata_promotion,
            &self.stable_metadata_promotion,
        )?;
        require_go_completion_handoff_equal(
            "report.go_generated_codec_completion_handoff",
            &report.go_generated_codec_completion_handoff,
            &self.go_generated_codec_completion_handoff,
        )?;
        require_csharp_completion_handoff_equal(
            "report.csharp_generated_codec_completion_handoff",
            &report.csharp_generated_codec_completion_handoff,
            &self.csharp_generated_codec_completion_handoff,
        )?;
        report.source_corpus.validate()?;
        require_claim_boundary_equal(
            "report.claim_boundary",
            &report.claim_boundary,
            &self.claim_boundary,
        )?;
        require_usize(
            "report.record_count",
            report.record_count,
            report.records.len(),
        )?;
        let records_sha256 = json_sha256(&report.records).map_err(|error| {
            SyntaxBackendBoundaryError::new(
                "report.records",
                format!("failed to hash source-program handoff records: {error}"),
            )
        })?;
        require_string(
            "report.records_sha256",
            &report.records_sha256,
            &records_sha256,
        )?;
        validate_handoff_records(&report.records)?;
        require_vec_equal(
            "report.allowed_claims",
            &report.allowed_claims,
            &self.claim_profile.allowed_claims,
        )?;
        require_vec_equal(
            "report.blocked_claims",
            &report.blocked_claims,
            &self.claim_profile.blocked_claims,
        )?;
        require_vec_equal(
            "report.validation_commands",
            &report.validation_commands,
            &self.handoff_report_contract.required_validation_commands,
        )?;
        require_string(
            "report.completed_promotion_class",
            &report.completed_promotion_class,
            REPORT_PRODUCER_COMPLETED_PROMOTION_CLASS,
        )?;
        require_string(
            "report.next_missing_stable_promotion_class",
            &report.next_missing_stable_promotion_class,
            &self.next_missing_stable_promotion_class,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SyntaxSourceProgramHandoffReport {
    pub artifact_kind: String,
    pub version: u32,
    pub tool: String,
    pub status: String,
    pub profile_id: String,
    pub source_route: SyntaxSourceProgramHandoffSourceRoute,
    pub stable_metadata_promotion: SyntaxSourceProgramStableMetadataPromotionInput,
    pub go_generated_codec_completion_handoff:
        SyntaxSourceProgramGoGeneratedCodecCompletionHandoffInput,
    pub csharp_generated_codec_completion_handoff:
        SyntaxSourceProgramCSharpGeneratedCodecCompletionHandoffInput,
    pub source_corpus: SyntaxSourceProgramHandoffSourceCorpusReport,
    pub claim_boundary: SyntaxSourceProgramClaimBoundary,
    pub record_count: usize,
    pub records_sha256: String,
    pub records: Vec<SyntaxSourceProgramHandoffRecord>,
    pub allowed_claims: Vec<String>,
    pub blocked_claims: Vec<String>,
    pub validation_commands: Vec<String>,
    pub completed_promotion_class: String,
    pub next_missing_stable_promotion_class: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxSourceProgramStableMetadataPromotionInput {
    pub artifact_kind: String,
    pub version: u32,
    pub tool: String,
    pub fixture_path: String,
    pub promotion_id: String,
    pub completed_promotion_class: String,
    pub next_missing_stable_promotion_class: String,
    pub source_completion_handoff_completed_class: String,
    pub source_completion_handoff_supported_record_count: usize,
    pub source_completion_handoff_blocked_record_count: usize,
    pub source_completion_handoff_supported_manifest_paths: Vec<String>,
    pub source_completion_handoff_generated_codec_routes: Vec<String>,
    pub source_completion_handoff_generated_codec_markers: Vec<String>,
    #[serde(default)]
    pub source_completion_handoff_source_program_typecheck_execution_report_path: String,
    #[serde(default)]
    pub source_completion_handoff_source_program_typecheck_feasibility_report_path: String,
    #[serde(default)]
    pub source_completion_handoff_source_program_typecheck_execution_completed_promotion_class:
        String,
    #[serde(default)]
    pub source_completion_handoff_source_program_typecheck_execution_next_missing_stable_promotion_class:
        String,
    pub source_completion_handoff_source_program_preserved_typecheck_stable_report_fields:
        Vec<String>,
    #[serde(default)]
    pub source_completion_handoff_source_program_preserved_typecheck_report_contract_stable_report_fields:
        Vec<String>,
    pub source_completion_handoff_source_program_preserved_typecheck_blocked_claims: Vec<String>,
    #[serde(default)]
    pub source_completion_handoff_source_program_preserved_typecheck_no_stable_claims: Vec<String>,
    #[serde(default)]
    pub source_completion_handoff_source_program_execution_pass_envelope_required_record_count:
        usize,
    #[serde(default)]
    pub source_completion_handoff_source_program_execution_pass_envelope_required_record_status:
        String,
    #[serde(default)]
    pub source_completion_handoff_source_program_execution_pass_envelope_required_records_all_passed:
        bool,
    #[serde(default)]
    pub source_completion_handoff_source_program_execution_pass_envelope_required_record_ids:
        Vec<String>,
    #[serde(default)]
    pub source_completion_handoff_source_program_typechecker_results_sidecar_only: bool,
    #[serde(default)]
    pub source_completion_handoff_source_program_type_correctness_claim_allowed: bool,
    #[serde(default)]
    pub source_completion_handoff_source_program_typechecker_arena_ir_proof_used: bool,
    #[serde(default)]
    pub source_completion_handoff_source_program_typechecker_json_schema_backend_unchanged: bool,
    #[serde(default)]
    pub source_completion_handoff_source_program_corpus_check_report_path: String,
    #[serde(default)]
    pub source_completion_handoff_source_program_corpus_eligible_record_count: usize,
    #[serde(default)]
    pub source_completion_handoff_source_program_corpus_unsupported_record_count: usize,
    #[serde(default)]
    pub source_completion_handoff_source_program_sidecar_validity_claim_allowed: bool,
    #[serde(default)]
    pub source_completion_handoff_source_program_sidecar_arena_ir_proof_used: bool,
    #[serde(default)]
    pub source_completion_handoff_source_program_sidecar_json_schema_backend_unchanged: bool,
    pub source_completion_handoff_source_program_no_stable_claims: Vec<String>,
    #[serde(default)]
    pub source_completion_handoff_generated_codec_report_paths: Vec<String>,
    #[serde(default)]
    pub source_completion_handoff_generated_codec_report_commands: Vec<String>,
    #[serde(default)]
    pub source_completion_handoff_generated_codec_round_trip_records:
        Vec<GeneratedCodecCorpusSupportedRecord>,
    #[serde(default)]
    pub source_completion_handoff_generated_codec_allowed_claims: Vec<String>,
    #[serde(default)]
    pub source_completion_handoff_generated_codec_whole_file_claim_scope: String,
    pub source_completion_handoff_generated_codec_no_stable_claims: Vec<String>,
    pub completion_handoff_consumed: bool,
    pub final_decision_consumed: bool,
    pub stable_route_metadata_promoted: bool,
    pub preserves_json_schema_backend: bool,
    pub generated_json_schema_output_changed: bool,
    pub source_program_claim_allowed: bool,
    pub arena_ir_proof_used: bool,
}

impl SyntaxSourceProgramStableMetadataPromotionInput {
    fn validate_static(&self) -> Result<(), SyntaxBackendBoundaryError> {
        require_string(
            "stable_metadata_promotion.artifact_kind",
            &self.artifact_kind,
            syntax_stable_metadata_promotion::ARTIFACT_KIND,
        )?;
        require_u32(
            "stable_metadata_promotion.version",
            self.version,
            syntax_stable_metadata_promotion::VERSION,
        )?;
        require_string(
            "stable_metadata_promotion.tool",
            &self.tool,
            syntax_stable_metadata_promotion::TOOL,
        )?;
        require_string(
            "stable_metadata_promotion.fixture_path",
            &self.fixture_path,
            syntax_stable_metadata_promotion::FIXTURE_PATH,
        )?;
        require_string(
            "stable_metadata_promotion.promotion_id",
            &self.promotion_id,
            syntax_stable_metadata_promotion::PROMOTION_ID,
        )?;
        require_string(
            "stable_metadata_promotion.completed_promotion_class",
            &self.completed_promotion_class,
            syntax_stable_metadata_promotion::COMPLETED_PROMOTION_CLASS,
        )?;
        require_string(
            "stable_metadata_promotion.next_missing_stable_promotion_class",
            &self.next_missing_stable_promotion_class,
            syntax_stable_metadata_promotion::NEXT_MISSING_STABLE_PROMOTION_CLASS,
        )?;
        require_string(
            "stable_metadata_promotion.source_completion_handoff_completed_class",
            &self.source_completion_handoff_completed_class,
            syntax_go_real_source_generated_codec_completion_handoff::COMPLETED_PROMOTION_CLASS,
        )?;
        require_usize(
            "stable_metadata_promotion.source_completion_handoff_supported_record_count",
            self.source_completion_handoff_supported_record_count,
            7,
        )?;
        require_usize(
            "stable_metadata_promotion.source_completion_handoff_blocked_record_count",
            self.source_completion_handoff_blocked_record_count,
            0,
        )?;
        require_set(
            "stable_metadata_promotion.source_completion_handoff_supported_manifest_paths",
            &self.source_completion_handoff_supported_manifest_paths,
            REQUIRED_STABLE_METADATA_SOURCE_MANIFEST_PATHS,
        )?;
        require_set(
            "stable_metadata_promotion.source_completion_handoff_generated_codec_routes",
            &self.source_completion_handoff_generated_codec_routes,
            REQUIRED_STABLE_METADATA_GENERATED_CODEC_ROUTES,
        )?;
        require_set(
            "stable_metadata_promotion.source_completion_handoff_generated_codec_markers",
            &self.source_completion_handoff_generated_codec_markers,
            REQUIRED_STABLE_METADATA_GENERATED_CODEC_MARKERS,
        )?;
        require_string(
            "stable_metadata_promotion.source_completion_handoff_source_program_typecheck_execution_report_path",
            &self.source_completion_handoff_source_program_typecheck_execution_report_path,
            "target/tct-syntax-source-program-typecheck-execution/execution-report-v1.json",
        )?;
        require_string(
            "stable_metadata_promotion.source_completion_handoff_source_program_typecheck_feasibility_report_path",
            &self.source_completion_handoff_source_program_typecheck_feasibility_report_path,
            "target/tct-syntax-source-program-typecheck-feasibility/feasibility-report-v1.json",
        )?;
        require_string(
            "stable_metadata_promotion.source_completion_handoff_source_program_typecheck_execution_completed_promotion_class",
            &self
                .source_completion_handoff_source_program_typecheck_execution_completed_promotion_class,
            "syntax backend source-program type-checker execution report gate",
        )?;
        require_string(
            "stable_metadata_promotion.source_completion_handoff_source_program_typecheck_execution_next_missing_stable_promotion_class",
            &self
                .source_completion_handoff_source_program_typecheck_execution_next_missing_stable_promotion_class,
            "syntax backend source-program type-checker execution preservation gate",
        )?;
        require_set(
            "stable_metadata_promotion.source_completion_handoff_source_program_preserved_typecheck_stable_report_fields",
            &self.source_completion_handoff_source_program_preserved_typecheck_stable_report_fields,
            REQUIRED_STABLE_METADATA_SOURCE_PROGRAM_REPORT_FIELDS,
        )?;
        require_set(
            "stable_metadata_promotion.source_completion_handoff_source_program_preserved_typecheck_report_contract_stable_report_fields",
            &self
                .source_completion_handoff_source_program_preserved_typecheck_report_contract_stable_report_fields,
            TYPECHECK_PRESERVATION_REPORT_STABLE_FIELDS,
        )?;
        require_set(
            "stable_metadata_promotion.source_completion_handoff_source_program_preserved_typecheck_blocked_claims",
            &self.source_completion_handoff_source_program_preserved_typecheck_blocked_claims,
            REQUIRED_STABLE_METADATA_SOURCE_PROGRAM_BLOCKED_CLAIMS,
        )?;
        require_set(
            "stable_metadata_promotion.source_completion_handoff_source_program_preserved_typecheck_no_stable_claims",
            &self.source_completion_handoff_source_program_preserved_typecheck_no_stable_claims,
            REQUIRED_STABLE_METADATA_SOURCE_PROGRAM_TYPECHECK_NO_STABLE_CLAIMS,
        )?;
        require_usize(
            "stable_metadata_promotion.source_completion_handoff_source_program_execution_pass_envelope_required_record_count",
            self.source_completion_handoff_source_program_execution_pass_envelope_required_record_count,
            84,
        )?;
        require_string(
            "stable_metadata_promotion.source_completion_handoff_source_program_execution_pass_envelope_required_record_status",
            &self.source_completion_handoff_source_program_execution_pass_envelope_required_record_status,
            "passed_sidecar_typecheck",
        )?;
        require_bool(
            "stable_metadata_promotion.source_completion_handoff_source_program_execution_pass_envelope_required_records_all_passed",
            self.source_completion_handoff_source_program_execution_pass_envelope_required_records_all_passed,
            true,
        )?;
        require_set(
            "stable_metadata_promotion.source_completion_handoff_source_program_execution_pass_envelope_required_record_ids",
            &self.source_completion_handoff_source_program_execution_pass_envelope_required_record_ids,
            &[
                "go_local_call_return_expression_syntax_v0",
                "go_local_argument_call_return_expression_syntax_v0",
                "csharp_local_nullable_return_type_syntax_v0",
                "csharp_local_nullable_declaration_syntax_v0",
                "csharp_nullable_property_declaration_syntax_v0",
                "csharp_nullable_parameter_declaration_syntax_v0",
            ],
        )?;
        require_bool(
            "stable_metadata_promotion.source_completion_handoff_source_program_typechecker_results_sidecar_only",
            self.source_completion_handoff_source_program_typechecker_results_sidecar_only,
            true,
        )?;
        require_bool(
            "stable_metadata_promotion.source_completion_handoff_source_program_type_correctness_claim_allowed",
            self.source_completion_handoff_source_program_type_correctness_claim_allowed,
            false,
        )?;
        require_bool(
            "stable_metadata_promotion.source_completion_handoff_source_program_typechecker_arena_ir_proof_used",
            self.source_completion_handoff_source_program_typechecker_arena_ir_proof_used,
            false,
        )?;
        require_bool(
            "stable_metadata_promotion.source_completion_handoff_source_program_typechecker_json_schema_backend_unchanged",
            self.source_completion_handoff_source_program_typechecker_json_schema_backend_unchanged,
            true,
        )?;
        require_string(
            "stable_metadata_promotion.source_completion_handoff_source_program_corpus_check_report_path",
            &self.source_completion_handoff_source_program_corpus_check_report_path,
            "target/tct-analysis/go-syntax-corpus-check/report.json",
        )?;
        require_usize(
            "stable_metadata_promotion.source_completion_handoff_source_program_corpus_eligible_record_count",
            self.source_completion_handoff_source_program_corpus_eligible_record_count,
            7,
        )?;
        require_usize(
            "stable_metadata_promotion.source_completion_handoff_source_program_corpus_unsupported_record_count",
            self.source_completion_handoff_source_program_corpus_unsupported_record_count,
            0,
        )?;
        require_bool(
            "stable_metadata_promotion.source_completion_handoff_source_program_sidecar_validity_claim_allowed",
            self.source_completion_handoff_source_program_sidecar_validity_claim_allowed,
            false,
        )?;
        require_bool(
            "stable_metadata_promotion.source_completion_handoff_source_program_sidecar_arena_ir_proof_used",
            self.source_completion_handoff_source_program_sidecar_arena_ir_proof_used,
            false,
        )?;
        require_bool(
            "stable_metadata_promotion.source_completion_handoff_source_program_sidecar_json_schema_backend_unchanged",
            self.source_completion_handoff_source_program_sidecar_json_schema_backend_unchanged,
            true,
        )?;
        require_set(
            "stable_metadata_promotion.source_completion_handoff_source_program_no_stable_claims",
            &self.source_completion_handoff_source_program_no_stable_claims,
            REQUIRED_STABLE_METADATA_SOURCE_PROGRAM_NO_STABLE_CLAIMS,
        )?;
        require_set(
            "stable_metadata_promotion.source_completion_handoff_generated_codec_report_paths",
            &self.source_completion_handoff_generated_codec_report_paths,
            REQUIRED_GENERATED_CODEC_REPORT_PATHS,
        )?;
        require_set(
            "stable_metadata_promotion.source_completion_handoff_generated_codec_report_commands",
            &self.source_completion_handoff_generated_codec_report_commands,
            REQUIRED_GENERATED_CODEC_REPORT_COMMANDS,
        )?;
        require_usize(
            "stable_metadata_promotion.source_completion_handoff_generated_codec_round_trip_records",
            self.source_completion_handoff_generated_codec_round_trip_records
                .len(),
            7,
        )?;
        require_set(
            "stable_metadata_promotion.source_completion_handoff_generated_codec_allowed_claims",
            &self.source_completion_handoff_generated_codec_allowed_claims,
            REQUIRED_STABLE_METADATA_GENERATED_CODEC_ALLOWED_CLAIMS,
        )?;
        require_string(
            "stable_metadata_promotion.source_completion_handoff_generated_codec_whole_file_claim_scope",
            &self.source_completion_handoff_generated_codec_whole_file_claim_scope,
            WHOLE_FILE_GENERATED_CODEC_CLAIM_SCOPE,
        )?;
        require_set(
            "stable_metadata_promotion.source_completion_handoff_generated_codec_no_stable_claims",
            &self.source_completion_handoff_generated_codec_no_stable_claims,
            REQUIRED_STABLE_METADATA_GENERATED_CODEC_NO_STABLE_CLAIMS,
        )?;
        require_bool(
            "stable_metadata_promotion.completion_handoff_consumed",
            self.completion_handoff_consumed,
            true,
        )?;
        require_bool(
            "stable_metadata_promotion.final_decision_consumed",
            self.final_decision_consumed,
            true,
        )?;
        require_bool(
            "stable_metadata_promotion.stable_route_metadata_promoted",
            self.stable_route_metadata_promoted,
            true,
        )?;
        require_bool(
            "stable_metadata_promotion.preserves_json_schema_backend",
            self.preserves_json_schema_backend,
            true,
        )?;
        require_bool(
            "stable_metadata_promotion.generated_json_schema_output_changed",
            self.generated_json_schema_output_changed,
            false,
        )?;
        require_bool(
            "stable_metadata_promotion.source_program_claim_allowed",
            self.source_program_claim_allowed,
            false,
        )?;
        require_bool(
            "stable_metadata_promotion.arena_ir_proof_used",
            self.arena_ir_proof_used,
            false,
        )
    }

    fn validate_against_gate(
        &self,
        stable_metadata_gate: &SyntaxStableMetadataPromotionImplementationGate,
        go_completion_handoff_gate: &SyntaxGoRealSourceGeneratedCodecCompletionHandoffGate,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        self.validate_static()?;
        require_string(
            "stable_metadata_promotion.source_fixture_artifact_kind",
            &stable_metadata_gate.artifact_kind,
            &self.artifact_kind,
        )?;
        require_string(
            "stable_metadata_promotion.source_fixture_completed_promotion_class",
            &stable_metadata_gate.completed_promotion_class,
            &self.completed_promotion_class,
        )?;
        require_string(
            "stable_metadata_promotion.source_fixture_source_completion_handoff_completed_class",
            &stable_metadata_gate
                .source_completion_handoff
                .completed_promotion_class,
            &go_completion_handoff_gate.completed_promotion_class,
        )?;
        require_usize(
            "stable_metadata_promotion.source_fixture_source_completion_handoff_supported_record_count",
            stable_metadata_gate
                .source_completion_handoff
                .supported_record_count,
            self.source_completion_handoff_supported_record_count,
        )?;
        require_usize(
            "stable_metadata_promotion.source_fixture_source_completion_handoff_blocked_record_count",
            stable_metadata_gate
                .source_completion_handoff
                .blocked_record_count,
            self.source_completion_handoff_blocked_record_count,
        )?;
        require_same_set(
            "stable_metadata_promotion.source_fixture_source_completion_handoff_supported_manifest_paths",
            &self.source_completion_handoff_supported_manifest_paths,
            &stable_metadata_gate
                .source_completion_handoff
                .supported_manifest_paths,
        )?;
        require_same_set(
            "stable_metadata_promotion.source_fixture_source_completion_handoff_generated_codec_routes",
            &self.source_completion_handoff_generated_codec_routes,
            &stable_metadata_gate
                .source_completion_handoff
                .generated_codec_routes,
        )?;
        require_same_set(
            "stable_metadata_promotion.source_fixture_source_completion_handoff_generated_codec_markers",
            &self.source_completion_handoff_generated_codec_markers,
            &stable_metadata_gate
                .source_completion_handoff
                .generated_codec_markers,
        )?;
        require_string(
            "stable_metadata_promotion.source_fixture_source_completion_handoff_source_program_typecheck_execution_report_path",
            &self.source_completion_handoff_source_program_typecheck_execution_report_path,
            &stable_metadata_gate
                .source_completion_handoff
                .source_program_typecheck_execution_report_path,
        )?;
        require_string(
            "stable_metadata_promotion.source_fixture_source_completion_handoff_source_program_typecheck_feasibility_report_path",
            &self.source_completion_handoff_source_program_typecheck_feasibility_report_path,
            &stable_metadata_gate
                .source_completion_handoff
                .source_program_typecheck_feasibility_report_path,
        )?;
        require_string(
            "stable_metadata_promotion.source_fixture_source_completion_handoff_source_program_typecheck_execution_completed_promotion_class",
            &self
                .source_completion_handoff_source_program_typecheck_execution_completed_promotion_class,
            &stable_metadata_gate
                .source_completion_handoff
                .source_program_typecheck_execution_completed_promotion_class,
        )?;
        require_string(
            "stable_metadata_promotion.source_fixture_source_completion_handoff_source_program_typecheck_execution_next_missing_stable_promotion_class",
            &self
                .source_completion_handoff_source_program_typecheck_execution_next_missing_stable_promotion_class,
            &stable_metadata_gate
                .source_completion_handoff
                .source_program_typecheck_execution_next_missing_stable_promotion_class,
        )?;
        require_same_set(
            "stable_metadata_promotion.source_fixture_source_completion_handoff_source_program_preserved_typecheck_stable_report_fields",
            &self.source_completion_handoff_source_program_preserved_typecheck_stable_report_fields,
            &stable_metadata_gate
                .source_completion_handoff
                .source_program_preserved_typecheck_stable_report_fields,
        )?;
        require_same_set(
            "stable_metadata_promotion.source_fixture_source_completion_handoff_source_program_preserved_typecheck_report_contract_stable_report_fields",
            &self
                .source_completion_handoff_source_program_preserved_typecheck_report_contract_stable_report_fields,
            &stable_metadata_gate
                .source_completion_handoff
                .source_program_preserved_typecheck_report_contract_stable_report_fields,
        )?;
        require_same_set(
            "stable_metadata_promotion.source_fixture_source_completion_handoff_source_program_preserved_typecheck_blocked_claims",
            &self.source_completion_handoff_source_program_preserved_typecheck_blocked_claims,
            &stable_metadata_gate
                .source_completion_handoff
                .source_program_preserved_typecheck_blocked_claims,
        )?;
        require_same_set(
            "stable_metadata_promotion.source_fixture_source_completion_handoff_source_program_preserved_typecheck_no_stable_claims",
            &self.source_completion_handoff_source_program_preserved_typecheck_no_stable_claims,
            &stable_metadata_gate
                .source_completion_handoff
                .source_program_preserved_typecheck_no_stable_claims,
        )?;
        require_usize(
            "stable_metadata_promotion.source_fixture_source_completion_handoff_source_program_execution_pass_envelope_required_record_count",
            self.source_completion_handoff_source_program_execution_pass_envelope_required_record_count,
            stable_metadata_gate
                .source_completion_handoff
                .source_program_execution_pass_envelope_required_record_count,
        )?;
        require_string(
            "stable_metadata_promotion.source_fixture_source_completion_handoff_source_program_execution_pass_envelope_required_record_status",
            &self.source_completion_handoff_source_program_execution_pass_envelope_required_record_status,
            &stable_metadata_gate
                .source_completion_handoff
                .source_program_execution_pass_envelope_required_record_status,
        )?;
        require_bool(
            "stable_metadata_promotion.source_fixture_source_completion_handoff_source_program_execution_pass_envelope_required_records_all_passed",
            self.source_completion_handoff_source_program_execution_pass_envelope_required_records_all_passed,
            stable_metadata_gate
                .source_completion_handoff
                .source_program_execution_pass_envelope_required_records_all_passed,
        )?;
        require_same_set(
            "stable_metadata_promotion.source_fixture_source_completion_handoff_source_program_execution_pass_envelope_required_record_ids",
            &self.source_completion_handoff_source_program_execution_pass_envelope_required_record_ids,
            &stable_metadata_gate
                .source_completion_handoff
                .source_program_execution_pass_envelope_required_record_ids,
        )?;
        require_bool(
            "stable_metadata_promotion.source_fixture_source_completion_handoff_source_program_typechecker_results_sidecar_only",
            self.source_completion_handoff_source_program_typechecker_results_sidecar_only,
            stable_metadata_gate
                .source_completion_handoff
                .source_program_typechecker_results_sidecar_only,
        )?;
        require_bool(
            "stable_metadata_promotion.source_fixture_source_completion_handoff_source_program_type_correctness_claim_allowed",
            self.source_completion_handoff_source_program_type_correctness_claim_allowed,
            stable_metadata_gate
                .source_completion_handoff
                .source_program_type_correctness_claim_allowed,
        )?;
        require_bool(
            "stable_metadata_promotion.source_fixture_source_completion_handoff_source_program_typechecker_arena_ir_proof_used",
            self.source_completion_handoff_source_program_typechecker_arena_ir_proof_used,
            stable_metadata_gate
                .source_completion_handoff
                .source_program_typechecker_arena_ir_proof_used,
        )?;
        require_bool(
            "stable_metadata_promotion.source_fixture_source_completion_handoff_source_program_typechecker_json_schema_backend_unchanged",
            self.source_completion_handoff_source_program_typechecker_json_schema_backend_unchanged,
            stable_metadata_gate
                .source_completion_handoff
                .source_program_typechecker_json_schema_backend_unchanged,
        )?;
        require_string(
            "stable_metadata_promotion.source_fixture_source_completion_handoff_source_program_corpus_check_report_path",
            &self.source_completion_handoff_source_program_corpus_check_report_path,
            &stable_metadata_gate
                .source_completion_handoff
                .source_program_corpus_check_report_path,
        )?;
        require_usize(
            "stable_metadata_promotion.source_fixture_source_completion_handoff_source_program_corpus_eligible_record_count",
            self.source_completion_handoff_source_program_corpus_eligible_record_count,
            stable_metadata_gate
                .source_completion_handoff
                .source_program_corpus_eligible_record_count,
        )?;
        require_usize(
            "stable_metadata_promotion.source_fixture_source_completion_handoff_source_program_corpus_unsupported_record_count",
            self.source_completion_handoff_source_program_corpus_unsupported_record_count,
            stable_metadata_gate
                .source_completion_handoff
                .source_program_corpus_unsupported_record_count,
        )?;
        require_bool(
            "stable_metadata_promotion.source_fixture_source_completion_handoff_source_program_sidecar_validity_claim_allowed",
            self.source_completion_handoff_source_program_sidecar_validity_claim_allowed,
            stable_metadata_gate
                .source_completion_handoff
                .source_program_sidecar_validity_claim_allowed,
        )?;
        require_bool(
            "stable_metadata_promotion.source_fixture_source_completion_handoff_source_program_sidecar_arena_ir_proof_used",
            self.source_completion_handoff_source_program_sidecar_arena_ir_proof_used,
            stable_metadata_gate
                .source_completion_handoff
                .source_program_sidecar_arena_ir_proof_used,
        )?;
        require_bool(
            "stable_metadata_promotion.source_fixture_source_completion_handoff_source_program_sidecar_json_schema_backend_unchanged",
            self.source_completion_handoff_source_program_sidecar_json_schema_backend_unchanged,
            stable_metadata_gate
                .source_completion_handoff
                .source_program_sidecar_json_schema_backend_unchanged,
        )?;
        require_same_set(
            "stable_metadata_promotion.source_fixture_source_completion_handoff_source_program_no_stable_claims",
            &self.source_completion_handoff_source_program_no_stable_claims,
            &stable_metadata_gate
                .source_completion_handoff
                .source_program_no_stable_claims,
        )?;
        require_same_set(
            "stable_metadata_promotion.source_fixture_source_completion_handoff_generated_codec_report_paths",
            &self.source_completion_handoff_generated_codec_report_paths,
            &stable_metadata_gate
                .source_completion_handoff
                .generated_codec_report_paths,
        )?;
        require_same_set(
            "stable_metadata_promotion.source_fixture_source_completion_handoff_generated_codec_report_commands",
            &self.source_completion_handoff_generated_codec_report_commands,
            &stable_metadata_gate
                .source_completion_handoff
                .generated_codec_report_commands,
        )?;
        require_same_supported_records(
            "stable_metadata_promotion.source_fixture_source_completion_handoff_generated_codec_round_trip_records",
            &self.source_completion_handoff_generated_codec_round_trip_records,
            &stable_metadata_gate
                .source_completion_handoff
                .generated_codec_round_trip_records,
        )?;
        require_same_set(
            "stable_metadata_promotion.source_fixture_source_completion_handoff_generated_codec_allowed_claims",
            &self.source_completion_handoff_generated_codec_allowed_claims,
            &stable_metadata_gate
                .source_completion_handoff
                .generated_codec_allowed_claims,
        )?;
        require_string(
            "stable_metadata_promotion.source_fixture_source_completion_handoff_generated_codec_whole_file_claim_scope",
            &self.source_completion_handoff_generated_codec_whole_file_claim_scope,
            &stable_metadata_gate
                .source_completion_handoff
                .generated_codec_whole_file_claim_scope,
        )?;
        require_same_set(
            "stable_metadata_promotion.source_fixture_source_completion_handoff_generated_codec_no_stable_claims",
            &self.source_completion_handoff_generated_codec_no_stable_claims,
            &stable_metadata_gate
                .source_completion_handoff
                .generated_codec_no_stable_claims,
        )?;
        require_bool(
            "stable_metadata_promotion.source_fixture_completion_handoff_consumed",
            stable_metadata_gate
                .promotion_contract
                .completion_handoff_consumed,
            self.completion_handoff_consumed,
        )?;
        require_bool(
            "stable_metadata_promotion.source_fixture_final_decision_consumed",
            stable_metadata_gate
                .promotion_contract
                .final_decision_consumed,
            self.final_decision_consumed,
        )?;
        require_bool(
            "stable_metadata_promotion.source_fixture_stable_route_metadata_promoted",
            stable_metadata_gate
                .promotion_contract
                .stable_route_metadata_promoted,
            self.stable_route_metadata_promoted,
        )?;
        require_bool(
            "stable_metadata_promotion.source_fixture_preserves_json_schema_backend",
            stable_metadata_gate
                .promotion_contract
                .preserves_json_schema_backend,
            self.preserves_json_schema_backend,
        )?;
        require_bool(
            "stable_metadata_promotion.source_fixture_generated_json_schema_output_changed",
            stable_metadata_gate
                .claim_boundary
                .generated_json_schema_output_changed,
            self.generated_json_schema_output_changed,
        )?;
        require_bool(
            "stable_metadata_promotion.source_fixture_source_program_claim_allowed",
            stable_metadata_gate
                .claim_boundary
                .source_program_claim_allowed,
            self.source_program_claim_allowed,
        )?;
        require_bool(
            "stable_metadata_promotion.source_fixture_arena_ir_proof_used",
            stable_metadata_gate.claim_boundary.arena_ir_proof_used,
            self.arena_ir_proof_used,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxSourceProgramGoGeneratedCodecCompletionHandoffInput {
    pub artifact_kind: String,
    pub version: u32,
    pub tool: String,
    pub fixture_path: String,
    pub completed_promotion_class: String,
    pub next_missing_stable_promotion_class: String,
    pub supported_record_count: usize,
    pub blocked_record_count: usize,
    pub source_program_claim_allowed: bool,
    pub whole_file_generated_codec_claim_allowed: bool,
    pub arena_ir_proof_used: bool,
    pub generated_json_schema_output_changed: bool,
    pub json_schema_backend_preserved: bool,
}

impl SyntaxSourceProgramGoGeneratedCodecCompletionHandoffInput {
    fn validate_static(&self) -> Result<(), SyntaxBackendBoundaryError> {
        require_string(
            "go_generated_codec_completion_handoff.artifact_kind",
            &self.artifact_kind,
            syntax_go_real_source_generated_codec_completion_handoff::ARTIFACT_KIND,
        )?;
        require_u32(
            "go_generated_codec_completion_handoff.version",
            self.version,
            syntax_go_real_source_generated_codec_completion_handoff::VERSION,
        )?;
        require_string(
            "go_generated_codec_completion_handoff.tool",
            &self.tool,
            syntax_go_real_source_generated_codec_completion_handoff::TOOL,
        )?;
        require_string(
            "go_generated_codec_completion_handoff.fixture_path",
            &self.fixture_path,
            syntax_go_real_source_generated_codec_completion_handoff::FIXTURE_PATH,
        )?;
        require_string(
            "go_generated_codec_completion_handoff.completed_promotion_class",
            &self.completed_promotion_class,
            syntax_go_real_source_generated_codec_completion_handoff::COMPLETED_PROMOTION_CLASS,
        )?;
        require_string(
            "go_generated_codec_completion_handoff.next_missing_stable_promotion_class",
            &self.next_missing_stable_promotion_class,
            syntax_go_real_source_generated_codec_completion_handoff::NEXT_MISSING_STABLE_PROMOTION_CLASS,
        )?;
        require_usize(
            "go_generated_codec_completion_handoff.supported_record_count",
            self.supported_record_count,
            7,
        )?;
        require_usize(
            "go_generated_codec_completion_handoff.blocked_record_count",
            self.blocked_record_count,
            0,
        )?;
        require_bool(
            "go_generated_codec_completion_handoff.source_program_claim_allowed",
            self.source_program_claim_allowed,
            false,
        )?;
        require_bool(
            "go_generated_codec_completion_handoff.whole_file_generated_codec_claim_allowed",
            self.whole_file_generated_codec_claim_allowed,
            true,
        )?;
        require_bool(
            "go_generated_codec_completion_handoff.arena_ir_proof_used",
            self.arena_ir_proof_used,
            false,
        )?;
        require_bool(
            "go_generated_codec_completion_handoff.generated_json_schema_output_changed",
            self.generated_json_schema_output_changed,
            false,
        )?;
        require_bool(
            "go_generated_codec_completion_handoff.json_schema_backend_preserved",
            self.json_schema_backend_preserved,
            true,
        )
    }

    fn validate_against_gate(
        &self,
        go_completion_handoff_gate: &SyntaxGoRealSourceGeneratedCodecCompletionHandoffGate,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        self.validate_static()?;
        require_string(
            "go_generated_codec_completion_handoff.source_fixture_artifact_kind",
            &go_completion_handoff_gate.artifact_kind,
            &self.artifact_kind,
        )?;
        require_usize(
            "go_generated_codec_completion_handoff.source_fixture_supported_record_count",
            go_completion_handoff_gate
                .source_generated_codec_corpus
                .supported_record_count,
            self.supported_record_count,
        )?;
        require_usize(
            "go_generated_codec_completion_handoff.source_fixture_blocked_record_count",
            go_completion_handoff_gate
                .source_generated_codec_corpus
                .blocked_record_count,
            self.blocked_record_count,
        )?;
        require_bool(
            "go_generated_codec_completion_handoff.source_fixture_source_program_claim_allowed",
            go_completion_handoff_gate
                .claim_boundary
                .source_program_validity_claim_allowed,
            self.source_program_claim_allowed,
        )?;
        require_bool(
            "go_generated_codec_completion_handoff.source_fixture_whole_file_generated_codec_claim_allowed",
            go_completion_handoff_gate
                .claim_boundary
                .whole_file_generated_codec_claim_allowed,
            self.whole_file_generated_codec_claim_allowed,
        )?;
        require_bool(
            "go_generated_codec_completion_handoff.source_fixture_arena_ir_proof_used",
            go_completion_handoff_gate
                .claim_boundary
                .arena_ir_proof_used,
            self.arena_ir_proof_used,
        )?;
        require_bool(
            "go_generated_codec_completion_handoff.source_fixture_generated_json_schema_output_changed",
            go_completion_handoff_gate
                .claim_boundary
                .generated_json_schema_output_changed,
            self.generated_json_schema_output_changed,
        )?;
        require_bool(
            "go_generated_codec_completion_handoff.source_fixture_json_schema_backend_preserved",
            go_completion_handoff_gate
                .claim_boundary
                .json_schema_backend_preserved,
            self.json_schema_backend_preserved,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxSourceProgramCSharpGeneratedCodecCompletionHandoffInput {
    pub artifact_kind: String,
    pub version: u32,
    pub tool: String,
    pub fixture_path: String,
    pub completed_promotion_class: String,
    pub next_missing_stable_promotion_class: String,
    pub supported_record_count: usize,
    pub blocked_record_count: usize,
    pub supported_manifest_paths: Vec<String>,
    pub generated_codec_routes: Vec<String>,
    pub generated_codec_markers: Vec<String>,
    pub generated_codec_report_paths: Vec<String>,
    pub generated_codec_report_commands: Vec<String>,
    pub source_program_claim_allowed: bool,
    pub whole_file_generated_codec_claim_allowed: bool,
    pub type_correctness_claim_allowed: bool,
    pub arena_ir_proof_used: bool,
    pub generated_json_schema_output_changed: bool,
    pub json_schema_backend_preserved: bool,
}

impl SyntaxSourceProgramCSharpGeneratedCodecCompletionHandoffInput {
    fn validate_static(&self) -> Result<(), SyntaxBackendBoundaryError> {
        require_string(
            "csharp_generated_codec_completion_handoff.artifact_kind",
            &self.artifact_kind,
            syntax_csharp_real_source_generated_codec_completion_handoff::ARTIFACT_KIND,
        )?;
        require_u32(
            "csharp_generated_codec_completion_handoff.version",
            self.version,
            syntax_csharp_real_source_generated_codec_completion_handoff::VERSION,
        )?;
        require_string(
            "csharp_generated_codec_completion_handoff.tool",
            &self.tool,
            syntax_csharp_real_source_generated_codec_completion_handoff::TOOL,
        )?;
        require_string(
            "csharp_generated_codec_completion_handoff.fixture_path",
            &self.fixture_path,
            syntax_csharp_real_source_generated_codec_completion_handoff::FIXTURE_PATH,
        )?;
        require_string(
            "csharp_generated_codec_completion_handoff.completed_promotion_class",
            &self.completed_promotion_class,
            syntax_csharp_real_source_generated_codec_completion_handoff::COMPLETED_PROMOTION_CLASS,
        )?;
        require_string(
            "csharp_generated_codec_completion_handoff.next_missing_stable_promotion_class",
            &self.next_missing_stable_promotion_class,
            syntax_csharp_real_source_generated_codec_completion_handoff::NEXT_MISSING_STABLE_PROMOTION_CLASS,
        )?;
        require_usize(
            "csharp_generated_codec_completion_handoff.supported_record_count",
            self.supported_record_count,
            syntax_csharp_real_source_generated_codec_completion_handoff::REQUIRED_SUPPORTED_RECORD_COUNT,
        )?;
        require_usize(
            "csharp_generated_codec_completion_handoff.blocked_record_count",
            self.blocked_record_count,
            syntax_csharp_real_source_generated_codec_completion_handoff::REQUIRED_BLOCKED_RECORD_COUNT,
        )?;
        require_set(
            "csharp_generated_codec_completion_handoff.supported_manifest_paths",
            &self.supported_manifest_paths,
            syntax_csharp_real_source_generated_codec_completion_handoff::REQUIRED_SUPPORTED_MANIFEST_PATHS,
        )?;
        require_set(
            "csharp_generated_codec_completion_handoff.generated_codec_routes",
            &self.generated_codec_routes,
            syntax_csharp_real_source_generated_codec_completion_handoff::REQUIRED_GENERATED_CODEC_ROUTES,
        )?;
        require_set(
            "csharp_generated_codec_completion_handoff.generated_codec_markers",
            &self.generated_codec_markers,
            syntax_csharp_real_source_generated_codec_completion_handoff::REQUIRED_GENERATED_CODEC_MARKERS,
        )?;
        require_set(
            "csharp_generated_codec_completion_handoff.generated_codec_report_paths",
            &self.generated_codec_report_paths,
            syntax_csharp_real_source_generated_codec_completion_handoff::REQUIRED_GENERATED_CODEC_REPORT_PATHS,
        )?;
        require_set(
            "csharp_generated_codec_completion_handoff.generated_codec_report_commands",
            &self.generated_codec_report_commands,
            syntax_csharp_real_source_generated_codec_completion_handoff::REQUIRED_GENERATED_CODEC_REPORT_COMMANDS,
        )?;
        require_bool(
            "csharp_generated_codec_completion_handoff.source_program_claim_allowed",
            self.source_program_claim_allowed,
            false,
        )?;
        require_bool(
            "csharp_generated_codec_completion_handoff.whole_file_generated_codec_claim_allowed",
            self.whole_file_generated_codec_claim_allowed,
            true,
        )?;
        require_bool(
            "csharp_generated_codec_completion_handoff.type_correctness_claim_allowed",
            self.type_correctness_claim_allowed,
            false,
        )?;
        require_bool(
            "csharp_generated_codec_completion_handoff.arena_ir_proof_used",
            self.arena_ir_proof_used,
            false,
        )?;
        require_bool(
            "csharp_generated_codec_completion_handoff.generated_json_schema_output_changed",
            self.generated_json_schema_output_changed,
            false,
        )?;
        require_bool(
            "csharp_generated_codec_completion_handoff.json_schema_backend_preserved",
            self.json_schema_backend_preserved,
            true,
        )
    }

    fn validate_against_gate(
        &self,
        csharp_completion_handoff_gate: &SyntaxCSharpRealSourceGeneratedCodecCompletionHandoffGate,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        self.validate_static()?;
        require_string(
            "csharp_generated_codec_completion_handoff.source_fixture_artifact_kind",
            &csharp_completion_handoff_gate.artifact_kind,
            &self.artifact_kind,
        )?;
        require_usize(
            "csharp_generated_codec_completion_handoff.source_fixture_supported_record_count",
            csharp_completion_handoff_gate
                .source_generated_codec_corpus
                .supported_record_count,
            self.supported_record_count,
        )?;
        require_usize(
            "csharp_generated_codec_completion_handoff.source_fixture_blocked_record_count",
            csharp_completion_handoff_gate
                .source_generated_codec_corpus
                .blocked_record_count,
            self.blocked_record_count,
        )?;
        require_same_set(
            "csharp_generated_codec_completion_handoff.source_fixture_supported_manifest_paths",
            &self.supported_manifest_paths,
            &csharp_completion_handoff_gate
                .source_generated_codec_corpus
                .supported_manifest_paths,
        )?;
        require_same_set(
            "csharp_generated_codec_completion_handoff.source_fixture_generated_codec_routes",
            &self.generated_codec_routes,
            &csharp_completion_handoff_gate
                .source_generated_codec_corpus
                .generated_codec_routes,
        )?;
        require_same_set(
            "csharp_generated_codec_completion_handoff.source_fixture_generated_codec_markers",
            &self.generated_codec_markers,
            &csharp_completion_handoff_gate
                .source_generated_codec_corpus
                .generated_codec_markers,
        )?;
        require_same_set(
            "csharp_generated_codec_completion_handoff.source_fixture_generated_codec_report_paths",
            &self.generated_codec_report_paths,
            &csharp_completion_handoff_gate
                .source_generated_codec_corpus
                .generated_codec_report_paths,
        )?;
        require_same_set(
            "csharp_generated_codec_completion_handoff.source_fixture_generated_codec_report_commands",
            &self.generated_codec_report_commands,
            &csharp_completion_handoff_gate
                .source_generated_codec_corpus
                .generated_codec_report_commands,
        )?;
        require_bool(
            "csharp_generated_codec_completion_handoff.source_fixture_source_program_claim_allowed",
            csharp_completion_handoff_gate
                .claim_boundary
                .source_program_validity_claim_allowed,
            self.source_program_claim_allowed,
        )?;
        require_bool(
            "csharp_generated_codec_completion_handoff.source_fixture_whole_file_generated_codec_claim_allowed",
            csharp_completion_handoff_gate
                .claim_boundary
                .whole_file_generated_codec_claim_allowed,
            self.whole_file_generated_codec_claim_allowed,
        )?;
        require_bool(
            "csharp_generated_codec_completion_handoff.source_fixture_type_correctness_claim_allowed",
            csharp_completion_handoff_gate
                .claim_boundary
                .type_correctness_claim_allowed,
            self.type_correctness_claim_allowed,
        )?;
        require_bool(
            "csharp_generated_codec_completion_handoff.source_fixture_arena_ir_proof_used",
            csharp_completion_handoff_gate
                .claim_boundary
                .arena_ir_proof_used,
            self.arena_ir_proof_used,
        )?;
        require_bool(
            "csharp_generated_codec_completion_handoff.source_fixture_generated_json_schema_output_changed",
            csharp_completion_handoff_gate
                .claim_boundary
                .generated_json_schema_output_changed,
            self.generated_json_schema_output_changed,
        )?;
        require_bool(
            "csharp_generated_codec_completion_handoff.source_fixture_json_schema_backend_preserved",
            csharp_completion_handoff_gate
                .claim_boundary
                .json_schema_backend_preserved,
            self.json_schema_backend_preserved,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxSourceProgramHandoffSourceCorpusReport {
    pub report_path: String,
    pub report_sha256: String,
    pub artifact_kind: String,
    pub tool: String,
    pub status: String,
    pub corpus_id: String,
    pub record_count: usize,
    pub records_sha256: String,
    pub missing_evidence: Vec<String>,
    pub csharp_no_gap_metadata_report: SyntaxSourceCorpusCSharpNoGapMetadataReport,
    pub next_missing_stable_promotion_class: String,
}

impl SyntaxSourceProgramHandoffSourceCorpusReport {
    fn validate(&self) -> Result<(), SyntaxBackendBoundaryError> {
        require_sha256("report.source_corpus.report_sha256", &self.report_sha256)?;
        require_string(
            "report.source_corpus.artifact_kind",
            &self.artifact_kind,
            syntax_source_corpus_evidence::ARTIFACT_KIND,
        )?;
        require_string(
            "report.source_corpus.tool",
            &self.tool,
            syntax_source_corpus_evidence::TOOL,
        )?;
        require_string(
            "report.source_corpus.status",
            &self.status,
            "generated_sidecar_report",
        )?;
        require_string(
            "report.source_corpus.corpus_id",
            &self.corpus_id,
            syntax_source_corpus_evidence::CORPUS_ID,
        )?;
        require_sha256("report.source_corpus.records_sha256", &self.records_sha256)?;
        require_all(
            "report.source_corpus.missing_evidence",
            &self.missing_evidence,
            &["source_program_claim_handoff", "model_quality_evidence"],
        )?;
        self.csharp_no_gap_metadata_report.validate()?;
        require_string(
            "report.source_corpus.next_missing_stable_promotion_class",
            &self.next_missing_stable_promotion_class,
            syntax_source_corpus_evidence::NEXT_MISSING_STABLE_PROMOTION_CLASS,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SyntaxSourceProgramHandoffRecord {
    pub target_id: String,
    pub grammar_id: String,
    pub source_path: String,
    pub source_sha256: String,
    pub metadata_path: String,
    pub metadata_sha256: String,
    pub parser_tool: String,
    pub parser_version: String,
    pub parser_package: Option<String>,
    pub parser_package_version: Option<String>,
    pub structural_model_sha256: String,
    pub token_layout_sha256: String,
    pub rendered_source_sha256: String,
    pub generated_codec_metadata_sha256: String,
    pub generated_crate_identity_sha256: String,
    pub generation_options_sha256: String,
    pub route_metadata: SyntaxSourceCorpusRouteEvidence,
    pub validation_hook_status: Value,
    pub sidecar_controls: Value,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxSourceProgramHandoffSourceRoute {
    pub production_route_fixture_path: String,
    pub route_manifest_fixture_path: String,
    pub source_corpus_fixture_path: String,
    pub route_table: String,
    pub route_entrypoint: String,
    pub target_agnostic_compiler_entrypoint: String,
    pub route_gate: String,
    pub stable_backend_scope: String,
    pub current_missing_class: String,
    pub source_corpus_report_path: String,
}

impl SyntaxSourceProgramHandoffSourceRoute {
    fn validate(
        &self,
        route_gate: &SyntaxProductionRouteImplementationGate,
        route_manifest: &SyntaxProductionRouteManifest,
        source_corpus_gate: &SyntaxSourceCorpusEvidenceGate,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        require_string(
            "source_route.production_route_fixture_path",
            &self.production_route_fixture_path,
            syntax_production_route::FIXTURE_PATH,
        )?;
        require_string(
            "source_route.route_manifest_fixture_path",
            &self.route_manifest_fixture_path,
            syntax_route_manifest::FIXTURE_PATH,
        )?;
        require_string(
            "source_route.source_corpus_fixture_path",
            &self.source_corpus_fixture_path,
            syntax_source_corpus_evidence::FIXTURE_PATH,
        )?;
        require_string(
            "source_route.route_table",
            &self.route_table,
            syntax_production_route::ROUTE_SETUP_SOURCE_OF_TRUTH,
        )?;
        require_string(
            "source_route.route_entrypoint",
            &self.route_entrypoint,
            syntax_production_route::ROUTE_ENTRYPOINT,
        )?;
        require_string(
            "source_route.target_agnostic_compiler_entrypoint",
            &self.target_agnostic_compiler_entrypoint,
            syntax_production_route::SYNTAX_COMPILER_ROUTE_ENTRYPOINT,
        )?;
        require_string(
            "source_route.route_gate",
            &self.route_gate,
            syntax_production_route::ROUTE_GATE,
        )?;
        require_string(
            "source_route.stable_backend_scope",
            &self.stable_backend_scope,
            syntax_production_route::STABLE_BACKEND_SCOPE,
        )?;
        require_string(
            "source_route.current_missing_class",
            &self.current_missing_class,
            syntax_source_corpus_evidence::NEXT_MISSING_STABLE_PROMOTION_CLASS,
        )?;
        require_string(
            "source_route.source_corpus_report_path",
            &self.source_corpus_report_path,
            syntax_source_corpus_evidence::REPORT_PATH,
        )?;
        require_string(
            "route_gate.stable_metadata.stable_backend_scope",
            &route_gate.stable_metadata.stable_backend_scope,
            &self.stable_backend_scope,
        )?;
        require_string(
            "route_gate.stable_metadata.completed_promotion_class",
            &route_gate.stable_metadata.completed_promotion_class,
            syntax_production_route::STABLE_METADATA_PROMOTION_COMPLETED_CLASS,
        )?;
        require_string(
            "route_gate.stable_metadata.next_missing_stable_promotion_class",
            &route_gate
                .stable_metadata
                .next_missing_stable_promotion_class,
            &self.current_missing_class,
        )?;
        require_bool(
            "route_gate.route_state.stable_backend_promoted",
            route_gate.route_state.stable_backend_promoted,
            true,
        )?;
        require_bool(
            "route_gate.stable_metadata.generated_json_schema_output_changed",
            route_gate
                .stable_metadata
                .generated_json_schema_output_changed,
            false,
        )?;
        require_bool(
            "route_gate.stable_metadata.core_compiler_path_changed",
            route_gate.stable_metadata.core_compiler_path_changed,
            false,
        )?;
        require_bool(
            "route_gate.stable_metadata.arena_ir_proof_used",
            route_gate.stable_metadata.arena_ir_proof_used,
            false,
        )?;
        require_bool(
            "route_gate.stable_metadata.external_sidecars_promoted",
            route_gate.stable_metadata.external_sidecars_promoted,
            false,
        )?;
        require_string(
            "route_manifest.next_missing_stable_promotion_class",
            &route_manifest.next_missing_stable_promotion_class,
            &self.current_missing_class,
        )?;
        require_usize(
            "route_manifest.constructs.len",
            route_manifest.constructs.len(),
            source_corpus_gate.corpus.expected_construct_count,
        )?;
        require_all(
            "source_corpus.corpus.missing_evidence",
            &source_corpus_gate.corpus.missing_evidence,
            &["source_program_claim_handoff", "model_quality_evidence"],
        )?;
        require_bool(
            "source_corpus.report_policy.generated_reports_checked_in",
            source_corpus_gate
                .report_policy
                .generated_reports_checked_in,
            false,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxSourceProgramClaimProfile {
    pub claim_level: String,
    pub allowed_claims: Vec<String>,
    pub blocked_claims: Vec<String>,
    pub required_evidence: Vec<String>,
    pub claim_text: String,
}

impl SyntaxSourceProgramClaimProfile {
    fn validate(&self) -> Result<(), SyntaxBackendBoundaryError> {
        require_string("claim_profile.claim_level", &self.claim_level, CLAIM_LEVEL)?;
        require_all(
            "claim_profile.allowed_claims",
            &self.allowed_claims,
            REQUIRED_ALLOWED_CLAIMS,
        )?;
        require_all(
            "claim_profile.blocked_claims",
            &self.blocked_claims,
            REQUIRED_BLOCKED_CLAIMS,
        )?;
        require_all(
            "claim_profile.required_evidence",
            &self.required_evidence,
            REQUIRED_EVIDENCE,
        )?;
        for phrase in [
            "generated-codec validity",
            "checked parser-backed syntax route",
            "not whole-source-language",
            "not paper",
            "not model-quality",
        ] {
            if !self.claim_text.contains(phrase) {
                return Err(SyntaxBackendBoundaryError::new(
                    "claim_profile.claim_text",
                    format!("claim text must contain {phrase}"),
                ));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxSourceProgramHandoffReportContract {
    pub report_artifact_kind: String,
    pub generated_report_path: String,
    pub generated_reports_checked_in: bool,
    pub source_corpus_report_path: String,
    pub report_producer_command: String,
    pub report_producer_available: bool,
    pub report_validator_command: String,
    pub report_validator_available: bool,
    pub required_summary_fields: Vec<String>,
    pub required_record_fields: Vec<String>,
    pub required_validation_commands: Vec<String>,
}

impl SyntaxSourceProgramHandoffReportContract {
    fn validate(
        &self,
        source_corpus_gate: &SyntaxSourceCorpusEvidenceGate,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        require_string(
            "handoff_report_contract.report_artifact_kind",
            &self.report_artifact_kind,
            REPORT_ARTIFACT_KIND,
        )?;
        require_string(
            "handoff_report_contract.generated_report_path",
            &self.generated_report_path,
            REPORT_PATH,
        )?;
        if !self.generated_report_path.starts_with("target/") {
            return Err(SyntaxBackendBoundaryError::new(
                "handoff_report_contract.generated_report_path",
                "source-program handoff reports must stay under target/",
            ));
        }
        require_bool(
            "handoff_report_contract.generated_reports_checked_in",
            self.generated_reports_checked_in,
            false,
        )?;
        require_string(
            "handoff_report_contract.source_corpus_report_path",
            &self.source_corpus_report_path,
            syntax_source_corpus_evidence::REPORT_PATH,
        )?;
        require_string(
            "handoff_report_contract.source_corpus_gate.report_policy.generated_report_path",
            &source_corpus_gate.report_policy.generated_report_path,
            &self.source_corpus_report_path,
        )?;
        require_string(
            "handoff_report_contract.report_producer_command",
            &self.report_producer_command,
            REPORT_PRODUCER_COMMAND,
        )?;
        require_bool(
            "handoff_report_contract.report_producer_available",
            self.report_producer_available,
            true,
        )?;
        require_string(
            "handoff_report_contract.report_validator_command",
            &self.report_validator_command,
            REPORT_VALIDATOR_COMMAND,
        )?;
        require_bool(
            "handoff_report_contract.report_validator_available",
            self.report_validator_available,
            true,
        )?;
        require_all(
            "handoff_report_contract.required_summary_fields",
            &self.required_summary_fields,
            REQUIRED_SUMMARY_FIELDS,
        )?;
        require_all(
            "handoff_report_contract.required_record_fields",
            &self.required_record_fields,
            REQUIRED_RECORD_FIELDS,
        )?;
        require_all(
            "handoff_report_contract.required_validation_commands",
            &self.required_validation_commands,
            REQUIRED_VALIDATION_COMMANDS,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxSourceProgramClaimBoundary {
    pub syntax_level_boundary: String,
    pub allowed_evidence_boundary: String,
    pub generated_codec_validity_claim_allowed: bool,
    pub whole_file_generated_codec_claim_allowed: bool,
    pub source_program_claim_allowed: bool,
    pub whole_source_language_claim_allowed: bool,
    pub paper_claim_allowed: bool,
    pub model_quality_claim_allowed: bool,
    pub arena_ir_proof_used: bool,
    pub external_sidecars_promoted: bool,
    pub json_schema_backend_changed: bool,
    pub json_schema_backend_preserved: bool,
}

impl SyntaxSourceProgramClaimBoundary {
    fn validate(&self) -> Result<(), SyntaxBackendBoundaryError> {
        require_string(
            "claim_boundary.syntax_level_boundary",
            &self.syntax_level_boundary,
            SYNTAX_LEVEL_BOUNDARY,
        )?;
        if !self
            .allowed_evidence_boundary
            .contains("checked parser-backed source-corpus records")
        {
            return Err(SyntaxBackendBoundaryError::new(
                "claim_boundary.allowed_evidence_boundary",
                "allowed evidence boundary must be source-corpus scoped",
            ));
        }
        require_bool(
            "claim_boundary.generated_codec_validity_claim_allowed",
            self.generated_codec_validity_claim_allowed,
            true,
        )?;
        require_bool(
            "claim_boundary.whole_file_generated_codec_claim_allowed",
            self.whole_file_generated_codec_claim_allowed,
            true,
        )?;
        require_bool(
            "claim_boundary.source_program_claim_allowed",
            self.source_program_claim_allowed,
            false,
        )?;
        require_bool(
            "claim_boundary.whole_source_language_claim_allowed",
            self.whole_source_language_claim_allowed,
            false,
        )?;
        require_bool(
            "claim_boundary.paper_claim_allowed",
            self.paper_claim_allowed,
            false,
        )?;
        require_bool(
            "claim_boundary.model_quality_claim_allowed",
            self.model_quality_claim_allowed,
            false,
        )?;
        require_bool(
            "claim_boundary.arena_ir_proof_used",
            self.arena_ir_proof_used,
            false,
        )?;
        require_bool(
            "claim_boundary.external_sidecars_promoted",
            self.external_sidecars_promoted,
            false,
        )?;
        require_bool(
            "claim_boundary.json_schema_backend_changed",
            self.json_schema_backend_changed,
            false,
        )?;
        require_bool(
            "claim_boundary.json_schema_backend_preserved",
            self.json_schema_backend_preserved,
            true,
        )
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
    let actual_set = actual.iter().map(String::as_str).collect::<BTreeSet<_>>();
    let missing = expected
        .iter()
        .copied()
        .filter(|expected_value| !actual_set.contains(expected_value))
        .collect::<Vec<_>>();
    if missing.is_empty() {
        Ok(())
    } else {
        Err(SyntaxBackendBoundaryError::new(
            field,
            format!("missing required values: {}", missing.join(", ")),
        ))
    }
}

fn require_set(
    field: &'static str,
    actual: &[String],
    expected: &[&str],
) -> Result<(), SyntaxBackendBoundaryError> {
    let actual = actual.iter().map(String::as_str).collect::<BTreeSet<_>>();
    let expected = expected.iter().copied().collect::<BTreeSet<_>>();
    if actual == expected {
        Ok(())
    } else {
        Err(SyntaxBackendBoundaryError::new(
            field,
            format!("expected {expected:?}, got {actual:?}"),
        ))
    }
}

fn require_same_set(
    field: &'static str,
    left: &[String],
    right: &[String],
) -> Result<(), SyntaxBackendBoundaryError> {
    let left = left.iter().map(String::as_str).collect::<BTreeSet<_>>();
    let right = right.iter().map(String::as_str).collect::<BTreeSet<_>>();
    if left == right {
        Ok(())
    } else {
        Err(SyntaxBackendBoundaryError::new(
            field,
            format!("expected {right:?}, got {left:?}"),
        ))
    }
}

fn require_same_supported_records(
    field: &'static str,
    left: &[GeneratedCodecCorpusSupportedRecord],
    right: &[GeneratedCodecCorpusSupportedRecord],
) -> Result<(), SyntaxBackendBoundaryError> {
    if left == right {
        Ok(())
    } else {
        Err(SyntaxBackendBoundaryError::new(
            field,
            "generated-codec round-trip records do not match stable metadata source",
        ))
    }
}

fn require_sha256(field: &'static str, actual: &str) -> Result<(), SyntaxBackendBoundaryError> {
    if actual.len() == 64 && actual.chars().all(|ch| ch.is_ascii_hexdigit()) {
        Ok(())
    } else {
        Err(SyntaxBackendBoundaryError::new(
            field,
            format!("expected sha256 hex digest, got {actual}"),
        ))
    }
}

fn require_vec_equal(
    field: &'static str,
    actual: &[String],
    expected: &[String],
) -> Result<(), SyntaxBackendBoundaryError> {
    if actual == expected {
        Ok(())
    } else {
        Err(SyntaxBackendBoundaryError::new(
            field,
            "report value does not match checked handoff profile",
        ))
    }
}

fn require_source_route_equal(
    field: &'static str,
    actual: &SyntaxSourceProgramHandoffSourceRoute,
    expected: &SyntaxSourceProgramHandoffSourceRoute,
) -> Result<(), SyntaxBackendBoundaryError> {
    if actual == expected {
        Ok(())
    } else {
        Err(SyntaxBackendBoundaryError::new(
            field,
            "report source route does not match checked handoff profile",
        ))
    }
}

fn require_stable_metadata_promotion_equal(
    field: &'static str,
    actual: &SyntaxSourceProgramStableMetadataPromotionInput,
    expected: &SyntaxSourceProgramStableMetadataPromotionInput,
) -> Result<(), SyntaxBackendBoundaryError> {
    if actual == expected {
        Ok(())
    } else {
        Err(SyntaxBackendBoundaryError::new(
            field,
            "report stable metadata promotion input does not match checked handoff profile",
        ))
    }
}

fn require_go_completion_handoff_equal(
    field: &'static str,
    actual: &SyntaxSourceProgramGoGeneratedCodecCompletionHandoffInput,
    expected: &SyntaxSourceProgramGoGeneratedCodecCompletionHandoffInput,
) -> Result<(), SyntaxBackendBoundaryError> {
    if actual == expected {
        Ok(())
    } else {
        Err(SyntaxBackendBoundaryError::new(
            field,
            "report Go generated-codec completion handoff input does not match checked handoff profile",
        ))
    }
}

fn require_csharp_completion_handoff_equal(
    field: &'static str,
    actual: &SyntaxSourceProgramCSharpGeneratedCodecCompletionHandoffInput,
    expected: &SyntaxSourceProgramCSharpGeneratedCodecCompletionHandoffInput,
) -> Result<(), SyntaxBackendBoundaryError> {
    if actual == expected {
        Ok(())
    } else {
        Err(SyntaxBackendBoundaryError::new(
            field,
            "report C# generated-codec completion handoff input does not match checked handoff profile",
        ))
    }
}

fn require_claim_boundary_equal(
    field: &'static str,
    actual: &SyntaxSourceProgramClaimBoundary,
    expected: &SyntaxSourceProgramClaimBoundary,
) -> Result<(), SyntaxBackendBoundaryError> {
    if actual == expected {
        Ok(())
    } else {
        Err(SyntaxBackendBoundaryError::new(
            field,
            "report claim boundary does not match checked handoff profile",
        ))
    }
}

fn source_program_handoff_record(
    record: &SyntaxSourceCorpusEvidenceRecord,
) -> Result<SyntaxSourceProgramHandoffRecord, SyntaxBackendBoundaryError> {
    require_sha256("source_corpus_record.source_sha256", &record.source_sha256)?;
    require_sha256(
        "source_corpus_record.metadata_sha256",
        &record.metadata_sha256,
    )?;
    require_sha256(
        "source_corpus_record.structural_model_sha256",
        &record.structural_model_sha256,
    )?;
    require_sha256(
        "source_corpus_record.token_layout_sha256",
        &record.token_layout_sha256,
    )?;
    require_sha256(
        "source_corpus_record.rendered_source_sha256",
        &record.generated_codec.rendered_source_sha256,
    )?;
    require_sha256(
        "source_corpus_record.generated_codec_metadata_sha256",
        &record.generated_codec.generated_codec_metadata_sha256,
    )?;
    require_sha256(
        "source_corpus_record.generated_crate_identity_sha256",
        &record.generated_codec.generated_crate_identity_sha256,
    )?;
    require_sha256(
        "source_corpus_record.generation_options_sha256",
        &record.generated_codec.generation_options_sha256,
    )?;
    require_bool(
        "source_corpus_record.route_metadata.generated_json_schema_output_changed",
        record.route_metadata.generated_json_schema_output_changed,
        false,
    )?;
    require_bool(
        "source_corpus_record.route_metadata.core_compiler_path_changed",
        record.route_metadata.core_compiler_path_changed,
        false,
    )?;
    require_bool(
        "source_corpus_record.route_metadata.stable_backend_promoted",
        record.route_metadata.stable_backend_promoted,
        true,
    )?;
    require_bool(
        "source_corpus_record.sidecar_controls.arena_ir_proof_used",
        record
            .sidecar_controls
            .get("arena_ir_proof_used")
            .and_then(Value::as_bool)
            .unwrap_or(true),
        false,
    )?;
    require_bool(
        "source_corpus_record.sidecar_controls.external_sidecars_promoted",
        record
            .sidecar_controls
            .get("external_sidecars_promoted")
            .and_then(Value::as_bool)
            .unwrap_or(true),
        false,
    )?;
    Ok(SyntaxSourceProgramHandoffRecord {
        target_id: record.target_id.clone(),
        grammar_id: record.grammar_id.clone(),
        source_path: record.source_path.clone(),
        source_sha256: record.source_sha256.clone(),
        metadata_path: record.metadata_path.clone(),
        metadata_sha256: record.metadata_sha256.clone(),
        parser_tool: record.parser.tool.clone(),
        parser_version: record.parser.version.clone(),
        parser_package: record.parser.package.clone(),
        parser_package_version: record.parser.package_version.clone(),
        structural_model_sha256: record.structural_model_sha256.clone(),
        token_layout_sha256: record.token_layout_sha256.clone(),
        rendered_source_sha256: record.generated_codec.rendered_source_sha256.clone(),
        generated_codec_metadata_sha256: record
            .generated_codec
            .generated_codec_metadata_sha256
            .clone(),
        generated_crate_identity_sha256: record
            .generated_codec
            .generated_crate_identity_sha256
            .clone(),
        generation_options_sha256: record.generated_codec.generation_options_sha256.clone(),
        route_metadata: record.route_metadata.clone(),
        validation_hook_status: record.validation_hook_status.clone(),
        sidecar_controls: record.sidecar_controls.clone(),
    })
}

fn validate_handoff_records(
    records: &[SyntaxSourceProgramHandoffRecord],
) -> Result<(), SyntaxBackendBoundaryError> {
    if records.is_empty() {
        return Err(SyntaxBackendBoundaryError::new(
            "report.records",
            "source-program handoff report must contain parser-backed records",
        ));
    }
    let mut targets = BTreeSet::new();
    for record in records {
        targets.insert(record.target_id.as_str());
        require_sha256("report.records.source_sha256", &record.source_sha256)?;
        require_sha256("report.records.metadata_sha256", &record.metadata_sha256)?;
        require_sha256(
            "report.records.structural_model_sha256",
            &record.structural_model_sha256,
        )?;
        require_sha256(
            "report.records.token_layout_sha256",
            &record.token_layout_sha256,
        )?;
        require_sha256(
            "report.records.rendered_source_sha256",
            &record.rendered_source_sha256,
        )?;
        require_sha256(
            "report.records.generated_codec_metadata_sha256",
            &record.generated_codec_metadata_sha256,
        )?;
        require_sha256(
            "report.records.generated_crate_identity_sha256",
            &record.generated_crate_identity_sha256,
        )?;
        require_sha256(
            "report.records.generation_options_sha256",
            &record.generation_options_sha256,
        )?;
        require_bool(
            "report.records.route_metadata.generated_json_schema_output_changed",
            record.route_metadata.generated_json_schema_output_changed,
            false,
        )?;
        require_bool(
            "report.records.route_metadata.core_compiler_path_changed",
            record.route_metadata.core_compiler_path_changed,
            false,
        )?;
        require_bool(
            "report.records.sidecar_controls.arena_ir_proof_used",
            record
                .sidecar_controls
                .get("arena_ir_proof_used")
                .and_then(Value::as_bool)
                .unwrap_or(true),
            false,
        )?;
        require_bool(
            "report.records.sidecar_controls.external_sidecars_promoted",
            record
                .sidecar_controls
                .get("external_sidecars_promoted")
                .and_then(Value::as_bool)
                .unwrap_or(true),
            false,
        )?;
    }
    require_all(
        "report.records.targets",
        &targets.into_iter().map(str::to_string).collect::<Vec<_>>(),
        &["csharp", "go"],
    )
}
