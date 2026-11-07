//! C# real-source generated-codec corpus completion handoff gate.
//!
//! This gate consumes the completed checked C# real-source generated-codec
//! corpus and records the source-corpus/source-program handoff input for the
//! five supported complete-source records. The evidence remains sidecar-only:
//! it does not promote source-program validity, type correctness, arena IR,
//! generated hot paths, paper claims, model-quality claims, or stable backend
//! status.
//!
//! Audit markers: `pattern-matching/Shapes.cs`, `delegates-and-events/Logger.cs`,
//! `events/VersionOne.cs`, `iterators/IteratorMethods.cs`,
//! `serialization/Loan.cs`, `csharp_compilation_unit_source_file_syntax_v0`,
//! `generated_csharp_compilation_unit_source_file_round_trips`,
//! `target/tct-syntax-real-source-demo/csharp-loan-generated-codec-demo.json`.

use crate::syntax_backend_boundary::SyntaxBackendBoundaryError;
use crate::syntax_csharp_real_source_generated_codec_corpus::{
    ARTIFACT_KIND as CORPUS_ARTIFACT_KIND, COMPLETED_PROMOTION_CLASS as CORPUS_COMPLETED_CLASS,
    CORPUS_ID, CORPUS_MANIFEST_PATH, CSharpGeneratedCodecCorpusSupportedRecord,
    FIXTURE_PATH as CORPUS_FIXTURE_PATH, GENERATED_CODEC_REPORT_COMMAND,
    GENERATED_CODEC_REPORT_PATH, ITERATOR_METHODS_GENERATED_CODEC_REPORT_COMMAND,
    ITERATOR_METHODS_GENERATED_CODEC_REPORT_PATH, LOAN_GENERATED_CODEC_REPORT_COMMAND,
    LOAN_GENERATED_CODEC_REPORT_PATH, LOGGER_GENERATED_CODEC_REPORT_COMMAND,
    LOGGER_GENERATED_CODEC_REPORT_PATH, NEXT_MISSING_STABLE_PROMOTION_CLASS as CORPUS_NEXT_CLASS,
    SyntaxCSharpRealSourceGeneratedCodecCorpusGate, TOOL as CORPUS_TOOL, VERSION as CORPUS_VERSION,
    VERSION_ONE_GENERATED_CODEC_REPORT_COMMAND, VERSION_ONE_GENERATED_CODEC_REPORT_PATH,
    WHOLE_FILE_GENERATED_CODEC_CLAIM, WHOLE_FILE_GENERATED_CODEC_CLAIM_SCOPE,
};
use crate::syntax_csharp_real_source_generated_codec_feasibility::{
    ITERATOR_METHODS_MANIFEST_PATH, LOAN_MANIFEST_PATH, LOGGER_MANIFEST_PATH,
    REQUIRED_WHOLE_SOURCE_GENERATED_CODEC_MARKER, REQUIRED_WHOLE_SOURCE_ROUTE_ID,
    SUPPORTED_MANIFEST_PATH, VERSION_ONE_MANIFEST_PATH,
};
use crate::syntax_source_corpus_evidence::{
    ARTIFACT_KIND as SOURCE_CORPUS_ARTIFACT_KIND, FIXTURE_PATH as SOURCE_CORPUS_FIXTURE_PATH,
    NEXT_MISSING_STABLE_PROMOTION_CLASS as SOURCE_CORPUS_NEXT_CLASS,
    SyntaxSourceCorpusEvidenceGate, TOOL as SOURCE_CORPUS_TOOL, VERSION as SOURCE_CORPUS_VERSION,
};
use crate::syntax_source_program_handoff::SYNTAX_LEVEL_BOUNDARY;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const TOOL: &str = "syntax_backend_csharp_real_source_generated_codec_completion_handoff_gate";
pub const VERSION: u32 = 1;
pub const ARTIFACT_KIND: &str =
    "tct_syntax_backend_csharp_real_source_generated_codec_completion_handoff_gate";
pub const FIXTURE_PATH: &str =
    "tests/fixtures/syntax_backend/csharp-real-source-generated-codec-completion-handoff-v1.json";
pub const STATUS: &str = "csharp_real_source_generated_codec_completion_handoff";
pub const HANDOFF_ID: &str = "csharp-real-source-generated-codec-completion-handoff-v1";
pub const COMPLETED_PROMOTION_CLASS: &str =
    "syntax backend C# real-source generated-codec corpus completion handoff gate";
pub const NEXT_MISSING_STABLE_PROMOTION_CLASS: &str =
    "syntax backend C# real-source generated-codec source-program validation control tranche";

pub const REQUIRED_SUPPORTED_RECORD_COUNT: usize = 5;
pub const REQUIRED_BLOCKED_RECORD_COUNT: usize = 0;
pub const REQUIRED_SUPPORTED_MANIFEST_PATHS: &[&str] = &[
    SUPPORTED_MANIFEST_PATH,
    LOGGER_MANIFEST_PATH,
    VERSION_ONE_MANIFEST_PATH,
    ITERATOR_METHODS_MANIFEST_PATH,
    LOAN_MANIFEST_PATH,
];
pub const REQUIRED_GENERATED_CODEC_ROUTES: &[&str] = &[REQUIRED_WHOLE_SOURCE_ROUTE_ID];
pub const REQUIRED_GENERATED_CODEC_MARKERS: &[&str] =
    &[REQUIRED_WHOLE_SOURCE_GENERATED_CODEC_MARKER];
pub const REQUIRED_GENERATED_CODEC_REPORT_PATHS: &[&str] = &[
    GENERATED_CODEC_REPORT_PATH,
    LOGGER_GENERATED_CODEC_REPORT_PATH,
    VERSION_ONE_GENERATED_CODEC_REPORT_PATH,
    ITERATOR_METHODS_GENERATED_CODEC_REPORT_PATH,
    LOAN_GENERATED_CODEC_REPORT_PATH,
];
pub const REQUIRED_GENERATED_CODEC_REPORT_COMMANDS: &[&str] = &[
    GENERATED_CODEC_REPORT_COMMAND,
    LOGGER_GENERATED_CODEC_REPORT_COMMAND,
    VERSION_ONE_GENERATED_CODEC_REPORT_COMMAND,
    ITERATOR_METHODS_GENERATED_CODEC_REPORT_COMMAND,
    LOAN_GENERATED_CODEC_REPORT_COMMAND,
];

pub const CSHARP_COMPLETION_EVIDENCE_MARKER: &str =
    "csharp_real_source_generated_codec_corpus_completion_evidence";

const REQUIRED_REMAINING_SOURCE_CORPUS_MISSING_EVIDENCE: &[&str] =
    &["source_program_claim_handoff", "model_quality_evidence"];

const REQUIRED_ALLOWED_CLAIMS: &[&str] = &[
    "generated_codec_validity_for_supported_csharp_real_source_records",
    WHOLE_FILE_GENERATED_CODEC_CLAIM,
    "parser_backed_source_corpus_identity_preserved",
    "all_checked_csharp_real_source_records_have_generated_codec_corpus_evidence",
    "csharp_real_source_generated_codec_corpus_completion_evidence_consumed",
];

const REQUIRED_BLOCKED_CLAIMS: &[&str] = &[
    "source_program_validity_claim",
    "whole_source_language_validity_claim",
    "type_correctness_claim",
    "functional_correctness_claim",
    "generated_hot_path_claim",
    "arena_ir_promotion",
    "stable_backend_promotion",
    "cross_repo_paper_claim",
    "model_quality_claim",
    "generated_json_schema_output_change",
];

const REQUIRED_NO_STABLE_CLAIMS: &[&str] = &[
    "no_source_program_validity_claim",
    "no_whole_source_language_validity_claim",
    "no_type_correctness_claim",
    "no_functional_correctness_claim",
    "no_generated_hot_path_claim",
    "no_arena_ir_promotion",
    "no_stable_backend_promotion",
    "no_cross_repo_paper_claim",
    "no_model_quality_claim",
    "no_generated_json_schema_output_change",
];

const REQUIRED_VALIDATION_COMMANDS: &[&str] = &[
    "cargo test --test syntax_backend_boundary_test syntax_backend_csharp_real_source_generated_codec_completion_handoff_consumes_completed_corpus -- --exact",
    "cargo test --test syntax_backend_boundary_test syntax_backend_source_program_handoff_consumes_csharp_generated_codec_completion_handoff -- --exact",
    "python3 -m unittest tests.python.test_syntax_backend_gate_strategy tests.python.test_structural_model_boundary_selection",
    "python3 scripts/testing/test-quality-audit.py --changed-from origin/main --fail-on-gaps",
    "git diff --check",
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxCSharpRealSourceGeneratedCodecCompletionHandoffGate {
    pub artifact_kind: String,
    pub version: u32,
    pub tool: String,
    pub status: String,
    pub handoff_id: String,
    pub source_generated_codec_corpus: CSharpGeneratedCodecCompletionSourceCorpus,
    pub source_corpus_handoff: CSharpGeneratedCodecCompletionSourceCorpusHandoff,
    pub completion_summary: CSharpGeneratedCodecCompletionSummary,
    pub claim_boundary: CSharpGeneratedCodecCompletionClaimBoundary,
    pub validation_commands: Vec<String>,
    pub completed_promotion_class: String,
    pub next_missing_stable_promotion_class: String,
}

impl SyntaxCSharpRealSourceGeneratedCodecCompletionHandoffGate {
    pub fn validate_against_corpus(
        &self,
        corpus_gate: &SyntaxCSharpRealSourceGeneratedCodecCorpusGate,
        source_corpus_gate: &SyntaxSourceCorpusEvidenceGate,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        require_string("artifact_kind", &self.artifact_kind, ARTIFACT_KIND)?;
        require_u32("version", self.version, VERSION)?;
        require_string("tool", &self.tool, TOOL)?;
        require_string("status", &self.status, STATUS)?;
        require_string("handoff_id", &self.handoff_id, HANDOFF_ID)?;
        self.source_generated_codec_corpus.validate(corpus_gate)?;
        self.source_corpus_handoff.validate(source_corpus_gate)?;
        self.completion_summary
            .validate(corpus_gate, source_corpus_gate)?;
        self.claim_boundary.validate(corpus_gate)?;
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
pub struct CSharpGeneratedCodecCompletionSourceCorpus {
    pub artifact_kind: String,
    pub version: u32,
    pub tool: String,
    pub module_path: String,
    pub fixture_path: String,
    pub completed_promotion_class: String,
    pub next_missing_stable_promotion_class: String,
    pub corpus_id: String,
    pub corpus_manifest_path: String,
    pub total_corpus_record_count: usize,
    pub supported_record_count: usize,
    pub blocked_record_count: usize,
    pub supported_manifest_paths: Vec<String>,
    pub generated_codec_routes: Vec<String>,
    pub generated_codec_markers: Vec<String>,
    pub generated_codec_report_paths: Vec<String>,
    pub generated_codec_report_commands: Vec<String>,
    pub generated_codec_round_trip_records: Vec<CSharpGeneratedCodecCorpusSupportedRecord>,
    pub whole_file_generated_codec_claim_allowed: bool,
    pub whole_file_generated_codec_claim_scope: String,
    pub allowed_claims: Vec<String>,
    pub no_stable_claims: Vec<String>,
    pub sidecar_only_completed: bool,
}

impl CSharpGeneratedCodecCompletionSourceCorpus {
    fn validate(
        &self,
        corpus_gate: &SyntaxCSharpRealSourceGeneratedCodecCorpusGate,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        require_string(
            "source_generated_codec_corpus.artifact_kind",
            &self.artifact_kind,
            CORPUS_ARTIFACT_KIND,
        )?;
        require_u32(
            "source_generated_codec_corpus.version",
            self.version,
            CORPUS_VERSION,
        )?;
        require_string(
            "source_generated_codec_corpus.tool",
            &self.tool,
            CORPUS_TOOL,
        )?;
        require_string(
            "source_generated_codec_corpus.module_path",
            &self.module_path,
            "src/syntax_csharp_real_source_generated_codec_corpus.rs",
        )?;
        require_string(
            "source_generated_codec_corpus.fixture_path",
            &self.fixture_path,
            CORPUS_FIXTURE_PATH,
        )?;
        require_string(
            "source_generated_codec_corpus.completed_promotion_class",
            &self.completed_promotion_class,
            CORPUS_COMPLETED_CLASS,
        )?;
        require_string(
            "source_generated_codec_corpus.next_missing_stable_promotion_class",
            &self.next_missing_stable_promotion_class,
            CORPUS_NEXT_CLASS,
        )?;
        require_string(
            "source_generated_codec_corpus.source_fixture_artifact_kind",
            &corpus_gate.artifact_kind,
            CORPUS_ARTIFACT_KIND,
        )?;
        require_string(
            "source_generated_codec_corpus.source_fixture_completed_class",
            &corpus_gate.completed_promotion_class,
            CORPUS_COMPLETED_CLASS,
        )?;
        require_string(
            "source_generated_codec_corpus.source_fixture_next_missing_class",
            &corpus_gate.next_missing_stable_promotion_class,
            CORPUS_NEXT_CLASS,
        )?;
        require_string(
            "source_generated_codec_corpus.corpus_id",
            &self.corpus_id,
            CORPUS_ID,
        )?;
        require_string(
            "source_generated_codec_corpus.corpus_manifest_path",
            &self.corpus_manifest_path,
            CORPUS_MANIFEST_PATH,
        )?;
        require_usize(
            "source_generated_codec_corpus.total_corpus_record_count",
            self.total_corpus_record_count,
            corpus_gate.corpus_round_trip.total_corpus_record_count,
        )?;
        require_usize(
            "source_generated_codec_corpus.supported_record_count",
            self.supported_record_count,
            REQUIRED_SUPPORTED_RECORD_COUNT,
        )?;
        require_usize(
            "source_generated_codec_corpus.supported_record_count.source_fixture",
            self.supported_record_count,
            corpus_gate.corpus_round_trip.supported_records.len(),
        )?;
        require_usize(
            "source_generated_codec_corpus.blocked_record_count",
            self.blocked_record_count,
            REQUIRED_BLOCKED_RECORD_COUNT,
        )?;
        require_usize(
            "source_generated_codec_corpus.blocked_record_count.source_fixture",
            self.blocked_record_count,
            corpus_gate.corpus_round_trip.blocked_records.len(),
        )?;
        require_set(
            "source_generated_codec_corpus.supported_manifest_paths",
            &self.supported_manifest_paths,
            REQUIRED_SUPPORTED_MANIFEST_PATHS,
        )?;
        let corpus_manifest_paths = corpus_gate
            .corpus_round_trip
            .supported_records
            .iter()
            .map(|record| record.manifest_path.clone())
            .collect::<Vec<_>>();
        require_same_set(
            "source_generated_codec_corpus.source_fixture_supported_manifest_paths",
            &self.supported_manifest_paths,
            &corpus_manifest_paths,
        )?;
        require_set(
            "source_generated_codec_corpus.generated_codec_routes",
            &self.generated_codec_routes,
            REQUIRED_GENERATED_CODEC_ROUTES,
        )?;
        require_set(
            "source_generated_codec_corpus.generated_codec_markers",
            &self.generated_codec_markers,
            REQUIRED_GENERATED_CODEC_MARKERS,
        )?;
        let corpus_routes = unique_strings(
            corpus_gate
                .corpus_round_trip
                .supported_records
                .iter()
                .map(|record| record.generated_codec_route.as_str()),
        );
        require_same_set(
            "source_generated_codec_corpus.source_fixture_generated_codec_routes",
            &self.generated_codec_routes,
            &corpus_routes,
        )?;
        let corpus_markers = unique_strings(
            corpus_gate
                .corpus_round_trip
                .supported_records
                .iter()
                .map(|record| record.generated_codec_marker.as_str()),
        );
        require_same_set(
            "source_generated_codec_corpus.source_fixture_generated_codec_markers",
            &self.generated_codec_markers,
            &corpus_markers,
        )?;
        require_set(
            "source_generated_codec_corpus.generated_codec_report_paths",
            &self.generated_codec_report_paths,
            REQUIRED_GENERATED_CODEC_REPORT_PATHS,
        )?;
        require_set(
            "source_generated_codec_corpus.generated_codec_report_commands",
            &self.generated_codec_report_commands,
            REQUIRED_GENERATED_CODEC_REPORT_COMMANDS,
        )?;
        let corpus_report_paths = corpus_gate
            .corpus_round_trip
            .supported_records
            .iter()
            .map(|record| record.report_path.clone())
            .collect::<Vec<_>>();
        require_same_set(
            "source_generated_codec_corpus.source_fixture_generated_codec_report_paths",
            &self.generated_codec_report_paths,
            &corpus_report_paths,
        )?;
        let corpus_report_commands = corpus_gate
            .corpus_round_trip
            .supported_records
            .iter()
            .map(|record| record.report_command.clone())
            .collect::<Vec<_>>();
        require_same_set(
            "source_generated_codec_corpus.source_fixture_generated_codec_report_commands",
            &self.generated_codec_report_commands,
            &corpus_report_commands,
        )?;
        require_same_supported_records(
            "source_generated_codec_corpus.generated_codec_round_trip_records",
            &self.generated_codec_round_trip_records,
            &corpus_gate.corpus_round_trip.supported_records,
        )?;
        require_bool(
            "source_generated_codec_corpus.whole_file_generated_codec_claim_allowed",
            self.whole_file_generated_codec_claim_allowed,
            true,
        )?;
        require_bool(
            "source_generated_codec_corpus.source_fixture_whole_file_generated_codec_claim_allowed",
            corpus_gate.whole_file_generated_codec_claim_allowed,
            true,
        )?;
        require_string(
            "source_generated_codec_corpus.whole_file_generated_codec_claim_scope",
            &self.whole_file_generated_codec_claim_scope,
            WHOLE_FILE_GENERATED_CODEC_CLAIM_SCOPE,
        )?;
        require_string(
            "source_generated_codec_corpus.source_fixture_whole_file_generated_codec_claim_scope",
            &corpus_gate.whole_file_generated_codec_claim_scope,
            WHOLE_FILE_GENERATED_CODEC_CLAIM_SCOPE,
        )?;
        require_all(
            "source_generated_codec_corpus.allowed_claims",
            &self.allowed_claims,
            REQUIRED_ALLOWED_CLAIMS,
        )?;
        require_all(
            "source_generated_codec_corpus.source_fixture_allowed_claims",
            &corpus_gate.allowed_claims,
            &[
                "generated_codec_validity_for_supported_csharp_real_source_record",
                WHOLE_FILE_GENERATED_CODEC_CLAIM,
                "parser_backed_source_corpus_identity_preserved",
                "all_checked_csharp_real_source_records_have_generated_codec_corpus_evidence",
            ],
        )?;
        require_all(
            "source_generated_codec_corpus.no_stable_claims",
            &self.no_stable_claims,
            REQUIRED_NO_STABLE_CLAIMS,
        )?;
        require_all(
            "source_generated_codec_corpus.source_fixture_no_stable_claims",
            &corpus_gate.no_stable_claims,
            &[
                "no_source_program_validity_claim",
                "no_whole_source_language_validity_claim",
                "no_type_correctness_claim",
                "no_functional_correctness_claim",
                "no_generated_hot_path_claim",
                "no_arena_ir_promotion",
                "no_stable_backend_promotion",
                "no_cross_repo_paper_claim",
                "no_model_quality_claim",
            ],
        )?;
        require_bool(
            "source_generated_codec_corpus.sidecar_only_completed",
            self.sidecar_only_completed,
            true,
        )?;
        for record in &corpus_gate.corpus_round_trip.supported_records {
            require_bool(
                "source_generated_codec_corpus.source_fixture_record_source_program_validity_claim_allowed",
                record.source_program_validity_claim_allowed,
                false,
            )?;
            require_bool(
                "source_generated_codec_corpus.source_fixture_record_type_correctness_claim_allowed",
                record.type_correctness_claim_allowed,
                false,
            )?;
            require_bool(
                "source_generated_codec_corpus.source_fixture_record_stable_promotion_allowed",
                record.stable_promotion_allowed,
                false,
            )?;
            require_bool(
                "source_generated_codec_corpus.source_fixture_record_arena_ir_proof_used",
                record.arena_ir_proof_used,
                false,
            )?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CSharpGeneratedCodecCompletionSourceCorpusHandoff {
    pub artifact_kind: String,
    pub version: u32,
    pub tool: String,
    pub fixture_path: String,
    pub completed_promotion_class: String,
    pub next_missing_stable_promotion_class: String,
    pub csharp_completion_evidence_consumed: bool,
    pub remaining_missing_evidence: Vec<String>,
    pub source_program_handoff_still_required: bool,
    pub model_quality_evidence_still_required: bool,
}

impl CSharpGeneratedCodecCompletionSourceCorpusHandoff {
    fn validate(
        &self,
        source_corpus_gate: &SyntaxSourceCorpusEvidenceGate,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        require_string(
            "source_corpus_handoff.artifact_kind",
            &self.artifact_kind,
            SOURCE_CORPUS_ARTIFACT_KIND,
        )?;
        require_u32(
            "source_corpus_handoff.version",
            self.version,
            SOURCE_CORPUS_VERSION,
        )?;
        require_string("source_corpus_handoff.tool", &self.tool, SOURCE_CORPUS_TOOL)?;
        require_string(
            "source_corpus_handoff.fixture_path",
            &self.fixture_path,
            SOURCE_CORPUS_FIXTURE_PATH,
        )?;
        require_string(
            "source_corpus_handoff.completed_promotion_class",
            &self.completed_promotion_class,
            crate::syntax_source_corpus_evidence::COMPLETED_PROMOTION_CLASS,
        )?;
        require_string(
            "source_corpus_handoff.next_missing_stable_promotion_class",
            &self.next_missing_stable_promotion_class,
            SOURCE_CORPUS_NEXT_CLASS,
        )?;
        require_bool(
            "source_corpus_handoff.csharp_completion_evidence_consumed",
            self.csharp_completion_evidence_consumed,
            true,
        )?;
        require_set(
            "source_corpus_handoff.remaining_missing_evidence",
            &self.remaining_missing_evidence,
            REQUIRED_REMAINING_SOURCE_CORPUS_MISSING_EVIDENCE,
        )?;
        require_same_set(
            "source_corpus_handoff.source_fixture_remaining_missing_evidence",
            &self.remaining_missing_evidence,
            &source_corpus_gate.corpus.missing_evidence,
        )?;
        if source_corpus_gate
            .corpus
            .missing_evidence
            .iter()
            .any(|missing| missing == CSHARP_COMPLETION_EVIDENCE_MARKER)
        {
            return Err(SyntaxBackendBoundaryError::new(
                "source_corpus_handoff.source_fixture_remaining_missing_evidence",
                "C# generated-codec corpus completion evidence must not remain missing",
            ));
        }
        require_bool(
            "source_corpus_handoff.source_program_handoff_still_required",
            self.source_program_handoff_still_required,
            true,
        )?;
        require_bool(
            "source_corpus_handoff.model_quality_evidence_still_required",
            self.model_quality_evidence_still_required,
            true,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CSharpGeneratedCodecCompletionSummary {
    pub source_language: String,
    pub corpus_id: String,
    pub corpus_manifest_path: String,
    pub checked_record_count: usize,
    pub supported_generated_codec_record_count: usize,
    pub blocked_generated_codec_record_count: usize,
    pub all_checked_records_supported: bool,
    pub route_scope: String,
    pub next_control_prerequisite: String,
}

impl CSharpGeneratedCodecCompletionSummary {
    fn validate(
        &self,
        corpus_gate: &SyntaxCSharpRealSourceGeneratedCodecCorpusGate,
        source_corpus_gate: &SyntaxSourceCorpusEvidenceGate,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        require_string(
            "completion_summary.source_language",
            &self.source_language,
            "csharp",
        )?;
        require_string(
            "completion_summary.corpus_id",
            &self.corpus_id,
            &corpus_gate.corpus_round_trip.corpus_id,
        )?;
        require_string(
            "completion_summary.corpus_manifest_path",
            &self.corpus_manifest_path,
            &corpus_gate.corpus_round_trip.corpus_manifest_path,
        )?;
        require_usize(
            "completion_summary.checked_record_count",
            self.checked_record_count,
            corpus_gate.corpus_round_trip.total_corpus_record_count,
        )?;
        require_usize(
            "completion_summary.supported_generated_codec_record_count",
            self.supported_generated_codec_record_count,
            corpus_gate
                .corpus_round_trip
                .generated_codec_round_trip_record_count,
        )?;
        require_usize(
            "completion_summary.blocked_generated_codec_record_count",
            self.blocked_generated_codec_record_count,
            corpus_gate.corpus_round_trip.blocked_record_count,
        )?;
        require_bool(
            "completion_summary.all_checked_records_supported",
            self.all_checked_records_supported,
            true,
        )?;
        require_string(
            "completion_summary.route_scope",
            &self.route_scope,
            "checked_csharp_real_source_generated_codec_sidecars",
        )?;
        require_string(
            "completion_summary.next_control_prerequisite",
            &self.next_control_prerequisite,
            NEXT_MISSING_STABLE_PROMOTION_CLASS,
        )?;
        require_set(
            "completion_summary.source_corpus_remaining_missing_evidence",
            &source_corpus_gate.corpus.missing_evidence,
            REQUIRED_REMAINING_SOURCE_CORPUS_MISSING_EVIDENCE,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CSharpGeneratedCodecCompletionClaimBoundary {
    pub syntax_level_boundary: String,
    pub allowed_claims: Vec<String>,
    pub blocked_claims: Vec<String>,
    pub parser_formatter_sidecars_promoted: bool,
    pub type_checker_sidecars_promoted: bool,
    pub source_program_validity_claim_allowed: bool,
    pub whole_file_generated_codec_claim_allowed: bool,
    pub type_correctness_claim_allowed: bool,
    pub arena_ir_proof_used: bool,
    pub generated_hot_path_claim_allowed: bool,
    pub generated_json_schema_output_changed: bool,
    pub json_schema_backend_preserved: bool,
}

impl CSharpGeneratedCodecCompletionClaimBoundary {
    fn validate(
        &self,
        corpus_gate: &SyntaxCSharpRealSourceGeneratedCodecCorpusGate,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        require_string(
            "claim_boundary.syntax_level_boundary",
            &self.syntax_level_boundary,
            SYNTAX_LEVEL_BOUNDARY,
        )?;
        require_all(
            "claim_boundary.allowed_claims",
            &self.allowed_claims,
            REQUIRED_ALLOWED_CLAIMS,
        )?;
        require_all(
            "claim_boundary.blocked_claims",
            &self.blocked_claims,
            REQUIRED_BLOCKED_CLAIMS,
        )?;
        require_bool(
            "claim_boundary.parser_formatter_sidecars_promoted",
            self.parser_formatter_sidecars_promoted,
            false,
        )?;
        require_bool(
            "claim_boundary.type_checker_sidecars_promoted",
            self.type_checker_sidecars_promoted,
            false,
        )?;
        require_bool(
            "claim_boundary.source_program_validity_claim_allowed",
            self.source_program_validity_claim_allowed,
            false,
        )?;
        require_bool(
            "claim_boundary.whole_file_generated_codec_claim_allowed",
            self.whole_file_generated_codec_claim_allowed,
            true,
        )?;
        require_bool(
            "claim_boundary.type_correctness_claim_allowed",
            self.type_correctness_claim_allowed,
            false,
        )?;
        require_bool(
            "claim_boundary.arena_ir_proof_used",
            self.arena_ir_proof_used,
            false,
        )?;
        require_bool(
            "claim_boundary.generated_hot_path_claim_allowed",
            self.generated_hot_path_claim_allowed,
            false,
        )?;
        require_bool(
            "claim_boundary.generated_json_schema_output_changed",
            self.generated_json_schema_output_changed,
            false,
        )?;
        require_bool(
            "claim_boundary.json_schema_backend_preserved",
            self.json_schema_backend_preserved,
            true,
        )?;
        require_all(
            "claim_boundary.source_fixture_no_stable_claims",
            &corpus_gate.no_stable_claims,
            &[
                "no_source_program_validity_claim",
                "no_type_correctness_claim",
                "no_generated_hot_path_claim",
                "no_arena_ir_promotion",
                "no_stable_backend_promotion",
            ],
        )
    }
}

fn unique_strings<'a>(values: impl Iterator<Item = &'a str>) -> Vec<String> {
    values
        .collect::<BTreeSet<_>>()
        .into_iter()
        .map(str::to_string)
        .collect()
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
    left: &[CSharpGeneratedCodecCorpusSupportedRecord],
    right: &[CSharpGeneratedCodecCorpusSupportedRecord],
) -> Result<(), SyntaxBackendBoundaryError> {
    if left == right {
        Ok(())
    } else {
        Err(SyntaxBackendBoundaryError::new(
            field,
            "C# generated-codec round-trip records do not match generated-codec corpus gate",
        ))
    }
}
