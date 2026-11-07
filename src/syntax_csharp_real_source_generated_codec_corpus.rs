//! C# real-source generated-codec corpus gate.
//!
//! This gate preserves checked C# real-source files that round trip through the
//! parser-backed generated syntax codec and keeps the remaining corpus files
//! blocked on their precise fixture or generated-codec coverage blockers. The
//! evidence is sidecar-only: it does not promote arena IR, source-program
//! validity, type correctness, generated hot-path behavior, or stable backend
//! status.
//!
//! Audit markers: `csharp_compilation_unit_source_file_syntax_v0`,
//! `generated_csharp_compilation_unit_source_file_round_trips`,
//! `all_checked_csharp_real_source_records_have_generated_codec_corpus_evidence`.

use crate::syntax_backend_boundary::SyntaxBackendBoundaryError;
use crate::syntax_csharp_real_source_generated_codec_feasibility::{
    ARTIFACT_KIND as FEASIBILITY_ARTIFACT_KIND,
    CHECKED_METADATA_BLOCKER as FEASIBILITY_CHECKED_METADATA_BLOCKER,
    COMPLETED_PROMOTION_CLASS as FEASIBILITY_COMPLETED_PROMOTION_CLASS,
    CSharpGeneratedCodecFeasibilityRecord, CSharpRealSourceCorpusManifest,
    CSharpRealSourceManifestEntry, FIXTURE_PATH as FEASIBILITY_FIXTURE_PATH,
    GENERATED_CODEC_FIXTURE_BLOCKER as FEASIBILITY_GENERATED_CODEC_FIXTURE_BLOCKER,
    GENERATED_CODEC_FIXTURE_BLOCKING_STAGE as FEASIBILITY_GENERATED_CODEC_FIXTURE_BLOCKING_STAGE,
    ITERATOR_METHODS_MANIFEST_PATH, LOAN_MANIFEST_PATH, LOGGER_MANIFEST_PATH,
    NEXT_MISSING_STABLE_PROMOTION_CLASS as FEASIBILITY_NEXT_MISSING_STABLE_PROMOTION_CLASS,
    REQUIRED_NEXT_ACTIONABLE_PREREQUISITE as FEASIBILITY_NEXT_ACTIONABLE_PREREQUISITE,
    REQUIRED_WHOLE_SOURCE_GENERATED_CODEC_MARKER, REQUIRED_WHOLE_SOURCE_ROUTE_ID,
    STATUS as FEASIBILITY_STATUS, SUPPORTED_MANIFEST_PATH,
    SyntaxCSharpRealSourceGeneratedCodecFeasibilityGate, TOOL as FEASIBILITY_TOOL,
    VERSION as FEASIBILITY_VERSION, VERSION_ONE_MANIFEST_PATH,
};
use crate::syntax_route_manifest::SyntaxProductionRouteManifest;
use crate::syntax_source_corpus_evidence::SyntaxSourceCorpusEvidenceGate;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const TOOL: &str = "syntax_backend_csharp_real_source_generated_codec_corpus_gate";
pub const VERSION: u32 = 1;
pub const ARTIFACT_KIND: &str = "tct_syntax_backend_csharp_real_source_generated_codec_corpus_gate";
pub const FIXTURE_PATH: &str =
    "tests/fixtures/syntax_backend/csharp-real-source-generated-codec-corpus-v1.json";
pub const STATUS: &str = "csharp_real_source_generated_codec_corpus_complete";
pub const COMPLETED_PROMOTION_CLASS: &str =
    "syntax backend C# real-source generated-codec corpus gate";
pub const NEXT_MISSING_STABLE_PROMOTION_CLASS: &str =
    "syntax backend C# real-source generated-codec corpus completion handoff tranche";

pub const CORPUS_ID: &str = "csharp-dotnet-samples-mini-v1";
pub const CORPUS_MANIFEST_PATH: &str = "corpora/csharp/dotnet-samples-mini/manifest-v1.json";
pub const SOURCE_ROOT: &str = "corpora/csharp/dotnet-samples-mini/src";
pub const GENERATED_CODEC_REPORT_ARTIFACT_KIND: &str =
    "tct_csharp_real_source_generated_codec_demo_v1";
pub const GENERATED_CODEC_REPORT_PATH: &str =
    "target/tct-syntax-real-source-demo/csharp-shapes-generated-codec-demo.json";
pub const GENERATED_CODEC_REPORT_ENV_VAR: &str =
    "TCT_CSHARP_COMPILATION_UNIT_SOURCE_FILE_REAL_SOURCE_CODEC_REPORT";
pub const GENERATED_CODEC_REPORT_COMMAND: &str = "cargo test --test syntax_generated_codec_gate_test csharp_compilation_unit_source_file_syntax_generated_codec_compiles_through_compiler_route -- --exact";
pub const LOGGER_GENERATED_CODEC_REPORT_PATH: &str =
    "target/tct-syntax-real-source-demo/csharp-logger-generated-codec-demo.json";
pub const LOGGER_GENERATED_CODEC_REPORT_ENV_VAR: &str =
    "TCT_CSHARP_COMPILATION_UNIT_LOGGER_SOURCE_FILE_REAL_SOURCE_CODEC_REPORT";
pub const LOGGER_GENERATED_CODEC_REPORT_COMMAND: &str = "cargo test --test syntax_generated_codec_gate_test csharp_compilation_unit_logger_source_file_syntax_generated_codec_compiles_through_compiler_route -- --exact";
pub const VERSION_ONE_GENERATED_CODEC_REPORT_PATH: &str =
    "target/tct-syntax-real-source-demo/csharp-version-one-generated-codec-demo.json";
pub const VERSION_ONE_GENERATED_CODEC_REPORT_ENV_VAR: &str =
    "TCT_CSHARP_COMPILATION_UNIT_VERSION_ONE_SOURCE_FILE_REAL_SOURCE_CODEC_REPORT";
pub const VERSION_ONE_GENERATED_CODEC_REPORT_COMMAND: &str = "cargo test --test syntax_generated_codec_gate_test csharp_compilation_unit_version_one_source_file_syntax_generated_codec_compiles_through_compiler_route -- --exact";
pub const ITERATOR_METHODS_GENERATED_CODEC_REPORT_PATH: &str =
    "target/tct-syntax-real-source-demo/csharp-iterator-methods-generated-codec-demo.json";
pub const ITERATOR_METHODS_GENERATED_CODEC_REPORT_ENV_VAR: &str =
    "TCT_CSHARP_COMPILATION_UNIT_ITERATOR_METHODS_SOURCE_FILE_REAL_SOURCE_CODEC_REPORT";
pub const ITERATOR_METHODS_GENERATED_CODEC_REPORT_COMMAND: &str = "cargo test --test syntax_generated_codec_gate_test csharp_compilation_unit_iterator_methods_source_file_syntax_generated_codec_compiles_through_compiler_route -- --exact";
pub const LOAN_GENERATED_CODEC_REPORT_PATH: &str =
    "target/tct-syntax-real-source-demo/csharp-loan-generated-codec-demo.json";
pub const LOAN_GENERATED_CODEC_REPORT_ENV_VAR: &str =
    "TCT_CSHARP_COMPILATION_UNIT_LOAN_SOURCE_FILE_REAL_SOURCE_CODEC_REPORT";
pub const LOAN_GENERATED_CODEC_REPORT_COMMAND: &str = "cargo test --test syntax_generated_codec_gate_test csharp_compilation_unit_loan_source_file_syntax_generated_codec_compiles_through_compiler_route -- --exact";
pub const SUPPORTED_SOURCE_PATH: &str =
    "corpora/csharp/dotnet-samples-mini/src/pattern-matching/Shapes.cs";
pub const SUPPORTED_SOURCE_SHA256: &str =
    "2714e85da9030b88559ce8548a3cce2e6f446681002380213c0bbf3e0d4a454e";
pub const LOGGER_SOURCE_PATH: &str =
    "corpora/csharp/dotnet-samples-mini/src/delegates-and-events/Logger.cs";
pub const LOGGER_SOURCE_SHA256: &str =
    "fe3c59d6da3c21872acb23a4d7aae0476a9ef977bbf30b11eda9aa21e1c97a0f";
pub const VERSION_ONE_SOURCE_PATH: &str =
    "corpora/csharp/dotnet-samples-mini/src/events/VersionOne.cs";
pub const VERSION_ONE_SOURCE_SHA256: &str =
    "ee5faea8bc7d22031adba420d7662a001386a09af49858fcce81a8aced49ece7";
pub const ITERATOR_METHODS_SOURCE_PATH: &str =
    "corpora/csharp/dotnet-samples-mini/src/iterators/IteratorMethods.cs";
pub const ITERATOR_METHODS_SOURCE_SHA256: &str =
    "2f47b8449ee3d6056fa341da57ac691a8c3b13a2776d7fd0b03e847ea8f4e976";
pub const LOAN_SOURCE_PATH: &str = "corpora/csharp/dotnet-samples-mini/src/serialization/Loan.cs";
pub const LOAN_SOURCE_SHA256: &str =
    "2435ba265f5db79ff296c9514d6c3480d053b348539a80d6bc63e8b2177ed609";
pub const GENERATED_CODEC_ROUND_TRIP_STATUS: &str = "generated_codec_corpus_round_trip_passed";
pub const BLOCKED_RECORD_STATUS: &str = "generated_codec_corpus_round_trip_blocked";
pub const WHOLE_FILE_GENERATED_CODEC_CLAIM: &str =
    "whole_file_generated_codec_claim_for_supported_csharp_real_source_record";
pub const WHOLE_FILE_GENERATED_CODEC_CLAIM_SCOPE: &str =
    "checked_csharp_real_source_generated_codec_corpus_sidecar_only";
pub const ITERATOR_METHODS_REPLAY_METADATA_SEGMENT_BLOCKER: &str =
    "csharp_payload_sequence_generated_codec_lacks_structural_model_segment_bridge";

const REQUIRED_RECORD_COUNT: usize = 5;
const REQUIRED_SUPPORTED_RECORD_COUNT: usize = 5;
const REQUIRED_BLOCKED_RECORD_COUNT: usize = 0;
const REQUIRED_SUPPORTED_TCT_TOKEN_COUNT: usize = 77;
const REQUIRED_SUPPORTED_PAYLOAD_SEQUENCE_CONTROL_TOKEN_COUNT: usize = 282;
const REQUIRED_SUPPORTED_BASELINE_BYTE_TOKEN_COUNT: usize = 1003;
const REQUIRED_LOGGER_TCT_TOKEN_COUNT: usize = 29;
const REQUIRED_LOGGER_PAYLOAD_SEQUENCE_CONTROL_TOKEN_COUNT: usize = 340;
const REQUIRED_LOGGER_BASELINE_BYTE_TOKEN_COUNT: usize = 553;
const REQUIRED_VERSION_ONE_TCT_TOKEN_COUNT: usize = 54;
const REQUIRED_VERSION_ONE_PAYLOAD_SEQUENCE_CONTROL_TOKEN_COUNT: usize = 337;
const REQUIRED_VERSION_ONE_BASELINE_BYTE_TOKEN_COUNT: usize = 497;
const REQUIRED_ITERATOR_METHODS_TCT_TOKEN_COUNT: usize = 121;
const REQUIRED_ITERATOR_METHODS_PAYLOAD_SEQUENCE_CONTROL_TOKEN_COUNT: usize = 482;
const REQUIRED_ITERATOR_METHODS_BASELINE_BYTE_TOKEN_COUNT: usize = 1883;
const REQUIRED_LOAN_TCT_TOKEN_COUNT: usize = 100;
const REQUIRED_LOAN_PAYLOAD_SEQUENCE_CONTROL_TOKEN_COUNT: usize = 555;
const REQUIRED_LOAN_BASELINE_BYTE_TOKEN_COUNT: usize = 1139;

const REQUIRED_STABLE_REPORT_FIELDS: &[&str] = &[
    "artifact_kind",
    "version",
    "status",
    "target",
    "route",
    "source_path",
    "source_sha256",
    "corpus_manifest",
    "generated_codec",
    "generated_codec_marker",
    "parser_validation.roslyn_metadata",
    "compiler_boundary",
    "no_stable_promotion_claim",
    "next_missing_stable_promotion_class",
];

const REQUIRED_VOLATILE_REPORT_FIELDS: &[&str] = &[
    "tct_token_count",
    "baseline_byte_token_count",
    "baseline_tokenizer",
];

pub const REQUIRED_REPLAY_METADATA_REPORT_FIELDS: &[&str] = &[
    "tct_tokens",
    "raw_token_ids_sha256",
    "structural_replay.raw_token_ids",
    "structural_replay.raw_token_ids_sha256",
    "structural_replay.raw_token_segments",
    "structural_replay.raw_token_segments_sha256",
    "raw_token_offset_mapping",
];

const REQUIRED_RETARGETED_REPLAY_METADATA_REPORT_FIELDS: &[&str] = &[
    "payload_sequence_control_token_count",
    "payload_sequence_control_tokens",
    "payload_sequence_control_tokens_sha256",
];

const REQUIRED_ALLOWED_CLAIMS: &[&str] = &[
    "generated_codec_validity_for_supported_csharp_real_source_record",
    WHOLE_FILE_GENERATED_CODEC_CLAIM,
    "parser_backed_source_corpus_identity_preserved",
    "all_checked_csharp_real_source_records_have_generated_codec_corpus_evidence",
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
];

const REQUIRED_VALIDATION_COMMANDS: &[&str] = &[
    "python3 scripts/analysis/csharp_syntax_metadata_smoke.py --fixture compilation-unit-source-file --self-test",
    "python3 scripts/analysis/csharp_syntax_metadata_smoke.py --fixture compilation-unit-logger-source-file --self-test",
    "python3 scripts/analysis/csharp_syntax_metadata_smoke.py --fixture compilation-unit-version-one-source-file --self-test",
    "python3 scripts/analysis/csharp_syntax_metadata_smoke.py --fixture compilation-unit-iterator-methods-source-file --self-test",
    "python3 scripts/analysis/csharp_syntax_metadata_smoke.py --fixture compilation-unit-loan-source-file --self-test",
    "./scripts/testing/test-csharp-real-source-metadata-gaps.sh",
    "cargo test --test csharp_syntax_metadata_adapter_test csharp_compilation_unit_version_one_source_file_metadata_lowers_to_structural_model_sidecar -- --exact",
    "cargo test --test csharp_syntax_metadata_adapter_test csharp_compilation_unit_iterator_methods_source_file_metadata_lowers_to_structural_model_sidecar -- --exact",
    "cargo test --test csharp_syntax_metadata_adapter_test csharp_compilation_unit_loan_source_file_metadata_lowers_to_structural_model_sidecar -- --exact",
    GENERATED_CODEC_REPORT_COMMAND,
    LOGGER_GENERATED_CODEC_REPORT_COMMAND,
    VERSION_ONE_GENERATED_CODEC_REPORT_COMMAND,
    ITERATOR_METHODS_GENERATED_CODEC_REPORT_COMMAND,
    LOAN_GENERATED_CODEC_REPORT_COMMAND,
    "cargo test --test syntax_backend_boundary_test syntax_backend_csharp_real_source_generated_codec_corpus_consumes_feasibility_gate -- --exact",
    "python3 scripts/testing/test-quality-audit.py --changed-from origin/main --fail-on-gaps",
    "git diff --check",
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxCSharpRealSourceGeneratedCodecCorpusGate {
    pub artifact_kind: String,
    pub version: u32,
    pub tool: String,
    pub status: String,
    pub feasibility_gate: CSharpGeneratedCodecCorpusFeasibilityRef,
    pub generated_codec_report_contract: CSharpGeneratedCodecReportContract,
    pub corpus_round_trip: CSharpGeneratedCodecCorpusRoundTripEvidence,
    pub whole_file_generated_codec_claim_allowed: bool,
    pub whole_file_generated_codec_claim_scope: String,
    pub allowed_claims: Vec<String>,
    pub no_stable_claims: Vec<String>,
    pub validation_commands: Vec<String>,
    pub completed_promotion_class: String,
    pub next_missing_stable_promotion_class: String,
}

impl SyntaxCSharpRealSourceGeneratedCodecCorpusGate {
    pub fn validate_against_sources(
        &self,
        source_corpus_gate: &SyntaxSourceCorpusEvidenceGate,
        route_manifest: &SyntaxProductionRouteManifest,
        corpus_manifest: &CSharpRealSourceCorpusManifest,
        feasibility_gate: &SyntaxCSharpRealSourceGeneratedCodecFeasibilityGate,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        feasibility_gate.validate_against_sources(
            source_corpus_gate,
            route_manifest,
            corpus_manifest,
        )?;
        require_string("artifact_kind", &self.artifact_kind, ARTIFACT_KIND)?;
        require_u32("version", self.version, VERSION)?;
        require_string("tool", &self.tool, TOOL)?;
        require_string("status", &self.status, STATUS)?;
        self.feasibility_gate.validate(feasibility_gate)?;
        self.generated_codec_report_contract
            .validate(route_manifest)?;
        self.corpus_round_trip
            .validate(corpus_manifest, feasibility_gate)?;
        require_bool(
            "whole_file_generated_codec_claim_allowed",
            self.whole_file_generated_codec_claim_allowed,
            true,
        )?;
        require_string(
            "whole_file_generated_codec_claim_scope",
            &self.whole_file_generated_codec_claim_scope,
            WHOLE_FILE_GENERATED_CODEC_CLAIM_SCOPE,
        )?;
        require_all(
            "allowed_claims",
            &self.allowed_claims,
            REQUIRED_ALLOWED_CLAIMS,
        )?;
        require_all(
            "no_stable_claims",
            &self.no_stable_claims,
            REQUIRED_NO_STABLE_CLAIMS,
        )?;
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
pub struct CSharpGeneratedCodecCorpusFeasibilityRef {
    pub artifact_kind: String,
    pub version: u32,
    pub tool: String,
    pub status: String,
    pub module_path: String,
    pub fixture_path: String,
    pub completed_promotion_class: String,
    pub next_missing_stable_promotion_class: String,
    pub feasible_record_count: usize,
    pub blocked_record_count: usize,
    pub supported_manifest_paths: Vec<String>,
    pub remaining_blocker: String,
    pub next_actionable_prerequisite: String,
}

impl CSharpGeneratedCodecCorpusFeasibilityRef {
    fn validate(
        &self,
        feasibility_gate: &SyntaxCSharpRealSourceGeneratedCodecFeasibilityGate,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        require_string(
            "feasibility_gate.artifact_kind",
            &self.artifact_kind,
            FEASIBILITY_ARTIFACT_KIND,
        )?;
        require_u32(
            "feasibility_gate.version",
            self.version,
            FEASIBILITY_VERSION,
        )?;
        require_string("feasibility_gate.tool", &self.tool, FEASIBILITY_TOOL)?;
        require_string("feasibility_gate.status", &self.status, FEASIBILITY_STATUS)?;
        require_string(
            "feasibility_gate.module_path",
            &self.module_path,
            "src/syntax_csharp_real_source_generated_codec_feasibility.rs",
        )?;
        require_string(
            "feasibility_gate.fixture_path",
            &self.fixture_path,
            FEASIBILITY_FIXTURE_PATH,
        )?;
        require_string(
            "feasibility_gate.completed_promotion_class",
            &self.completed_promotion_class,
            FEASIBILITY_COMPLETED_PROMOTION_CLASS,
        )?;
        require_string(
            "feasibility_gate.next_missing_stable_promotion_class",
            &self.next_missing_stable_promotion_class,
            FEASIBILITY_NEXT_MISSING_STABLE_PROMOTION_CLASS,
        )?;
        require_string(
            "feasibility_gate.actual_completed_promotion_class",
            &feasibility_gate.completed_promotion_class,
            FEASIBILITY_COMPLETED_PROMOTION_CLASS,
        )?;
        require_string(
            "feasibility_gate.actual_next_missing_stable_promotion_class",
            &feasibility_gate.next_missing_stable_promotion_class,
            FEASIBILITY_NEXT_MISSING_STABLE_PROMOTION_CLASS,
        )?;
        require_usize(
            "feasibility_gate.feasible_record_count",
            self.feasible_record_count,
            REQUIRED_SUPPORTED_RECORD_COUNT,
        )?;
        require_usize(
            "feasibility_gate.actual_feasible_record_count",
            feasibility_gate.feasibility.feasible_record_count,
            self.feasible_record_count,
        )?;
        require_usize(
            "feasibility_gate.blocked_record_count",
            self.blocked_record_count,
            REQUIRED_BLOCKED_RECORD_COUNT,
        )?;
        require_usize(
            "feasibility_gate.actual_blocked_record_count",
            feasibility_gate.feasibility.blocked_record_count,
            self.blocked_record_count,
        )?;
        require_all(
            "feasibility_gate.supported_manifest_paths",
            &self.supported_manifest_paths,
            &[
                SUPPORTED_MANIFEST_PATH,
                LOGGER_MANIFEST_PATH,
                VERSION_ONE_MANIFEST_PATH,
                ITERATOR_METHODS_MANIFEST_PATH,
                LOAN_MANIFEST_PATH,
            ],
        )?;
        require_string(
            "feasibility_gate.remaining_blocker",
            &self.remaining_blocker,
            "none",
        )?;
        require_string(
            "feasibility_gate.next_actionable_prerequisite",
            &self.next_actionable_prerequisite,
            FEASIBILITY_NEXT_ACTIONABLE_PREREQUISITE,
        )?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CSharpGeneratedCodecReportContract {
    pub report_artifact_kind: String,
    pub report_path: String,
    pub report_env_var: String,
    pub report_command: String,
    pub report_generated_under_target: bool,
    pub report_checked_in: bool,
    pub expected_route: String,
    pub expected_generated_codec_marker: String,
    pub expected_next_missing_stable_promotion_class: String,
    pub stable_report_fields: Vec<String>,
    pub volatile_report_fields: Vec<String>,
}

impl CSharpGeneratedCodecReportContract {
    fn validate(
        &self,
        route_manifest: &SyntaxProductionRouteManifest,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        require_string(
            "generated_codec_report_contract.report_artifact_kind",
            &self.report_artifact_kind,
            GENERATED_CODEC_REPORT_ARTIFACT_KIND,
        )?;
        require_string(
            "generated_codec_report_contract.report_path",
            &self.report_path,
            GENERATED_CODEC_REPORT_PATH,
        )?;
        if !self.report_path.starts_with("target/") {
            return Err(SyntaxBackendBoundaryError::new(
                "generated_codec_report_contract.report_path",
                "C# real-source generated-codec reports must stay under target/",
            ));
        }
        require_string(
            "generated_codec_report_contract.report_env_var",
            &self.report_env_var,
            GENERATED_CODEC_REPORT_ENV_VAR,
        )?;
        require_string(
            "generated_codec_report_contract.report_command",
            &self.report_command,
            GENERATED_CODEC_REPORT_COMMAND,
        )?;
        require_bool(
            "generated_codec_report_contract.report_generated_under_target",
            self.report_generated_under_target,
            true,
        )?;
        require_bool(
            "generated_codec_report_contract.report_checked_in",
            self.report_checked_in,
            false,
        )?;
        require_string(
            "generated_codec_report_contract.expected_route",
            &self.expected_route,
            REQUIRED_WHOLE_SOURCE_ROUTE_ID,
        )?;
        require_string(
            "generated_codec_report_contract.expected_generated_codec_marker",
            &self.expected_generated_codec_marker,
            REQUIRED_WHOLE_SOURCE_GENERATED_CODEC_MARKER,
        )?;
        let route = route_manifest
            .constructs
            .iter()
            .find(|construct| construct.grammar_id == REQUIRED_WHOLE_SOURCE_ROUTE_ID)
            .ok_or_else(|| {
                SyntaxBackendBoundaryError::new(
                    "generated_codec_report_contract.expected_route",
                    "C# corpus gate requires the whole-source generated-codec route",
                )
            })?;
        require_contains(
            "generated_codec_report_contract.expected_generated_codec_marker",
            &route.generated_test_markers,
            REQUIRED_WHOLE_SOURCE_GENERATED_CODEC_MARKER,
        )?;
        require_string(
            "generated_codec_report_contract.expected_next_missing_stable_promotion_class",
            &self.expected_next_missing_stable_promotion_class,
            COMPLETED_PROMOTION_CLASS,
        )?;
        require_all(
            "generated_codec_report_contract.stable_report_fields",
            &self.stable_report_fields,
            REQUIRED_STABLE_REPORT_FIELDS,
        )?;
        require_all(
            "generated_codec_report_contract.volatile_report_fields",
            &self.volatile_report_fields,
            REQUIRED_VOLATILE_REPORT_FIELDS,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CSharpGeneratedCodecCorpusRoundTripEvidence {
    pub corpus_id: String,
    pub corpus_manifest_path: String,
    pub all_checked_records_are_complete_source_files: bool,
    pub total_corpus_record_count: usize,
    pub generated_codec_round_trip_record_count: usize,
    pub blocked_record_count: usize,
    pub supported_records: Vec<CSharpGeneratedCodecCorpusSupportedRecord>,
    pub blocked_records: Vec<CSharpGeneratedCodecCorpusBlockedRecord>,
}

impl CSharpGeneratedCodecCorpusRoundTripEvidence {
    fn validate(
        &self,
        corpus_manifest: &CSharpRealSourceCorpusManifest,
        feasibility_gate: &SyntaxCSharpRealSourceGeneratedCodecFeasibilityGate,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        corpus_manifest.validate()?;
        require_string("corpus_round_trip.corpus_id", &self.corpus_id, CORPUS_ID)?;
        require_string(
            "corpus_round_trip.corpus_manifest_path",
            &self.corpus_manifest_path,
            CORPUS_MANIFEST_PATH,
        )?;
        require_bool(
            "corpus_round_trip.all_checked_records_are_complete_source_files",
            self.all_checked_records_are_complete_source_files,
            true,
        )?;
        require_usize(
            "corpus_round_trip.total_corpus_record_count",
            self.total_corpus_record_count,
            REQUIRED_RECORD_COUNT,
        )?;
        require_usize(
            "corpus_round_trip.total_corpus_record_count",
            self.total_corpus_record_count,
            corpus_manifest.source_manifest.entry_count,
        )?;
        require_usize(
            "corpus_round_trip.generated_codec_round_trip_record_count",
            self.generated_codec_round_trip_record_count,
            REQUIRED_SUPPORTED_RECORD_COUNT,
        )?;
        require_usize(
            "corpus_round_trip.supported_records",
            self.supported_records.len(),
            self.generated_codec_round_trip_record_count,
        )?;
        require_usize(
            "corpus_round_trip.blocked_record_count",
            self.blocked_record_count,
            REQUIRED_BLOCKED_RECORD_COUNT,
        )?;
        require_usize(
            "corpus_round_trip.blocked_records",
            self.blocked_records.len(),
            self.blocked_record_count,
        )?;

        let manifest_by_path = manifest_entries_by_path(corpus_manifest);
        let feasibility_by_path = feasibility_records_by_path(feasibility_gate);
        let mut accounted = BTreeSet::new();

        for record in &self.supported_records {
            let entry = manifest_entry(&manifest_by_path, &record.manifest_path)?;
            let feasibility_record =
                feasibility_record(&feasibility_by_path, &record.manifest_path)?;
            record.validate(entry, feasibility_record)?;
            accounted.insert(record.manifest_path.as_str());
        }

        for record in &self.blocked_records {
            let entry = manifest_entry(&manifest_by_path, &record.manifest_path)?;
            let feasibility_record =
                feasibility_record(&feasibility_by_path, &record.manifest_path)?;
            record.validate(entry, feasibility_record)?;
            accounted.insert(record.manifest_path.as_str());
        }

        for entry in &corpus_manifest.source_manifest.entries {
            if !accounted.contains(entry.manifest_path.as_str()) {
                return Err(SyntaxBackendBoundaryError::new(
                    "corpus_round_trip.records",
                    format!(
                        "missing C# corpus generated-codec disposition {}",
                        entry.manifest_path
                    ),
                ));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CSharpGeneratedCodecCorpusSupportedRecord {
    pub manifest_path: String,
    pub source_path: String,
    pub source_sha256: String,
    pub source_bytes: usize,
    pub generated_codec_route: String,
    pub generated_codec_marker: String,
    pub report_path: String,
    pub report_command: String,
    pub generated_codec_round_trip_status: String,
    pub tct_token_count: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payload_sequence_control_token_count: Option<usize>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub replay_metadata_report_fields: Vec<String>,
    pub baseline_byte_token_count: usize,
    pub parser_metadata_validation: String,
    pub source_program_validity_claim_allowed: bool,
    pub type_correctness_claim_allowed: bool,
    pub stable_promotion_allowed: bool,
    pub arena_ir_proof_used: bool,
}

impl CSharpGeneratedCodecCorpusSupportedRecord {
    fn validate(
        &self,
        entry: &CSharpRealSourceManifestEntry,
        feasibility_record: &CSharpGeneratedCodecFeasibilityRecord,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        let expected = supported_record_policy(&self.manifest_path)?;
        require_string(
            "corpus_round_trip.supported_record.manifest_path",
            &self.manifest_path,
            expected.manifest_path,
        )?;
        require_string(
            "corpus_round_trip.supported_record.source_path",
            &self.source_path,
            expected.source_path,
        )?;
        require_string(
            "corpus_round_trip.supported_record.source_sha256",
            &self.source_sha256,
            expected.source_sha256,
        )?;
        require_usize(
            "corpus_round_trip.supported_record.source_bytes",
            self.source_bytes,
            expected.baseline_byte_token_count,
        )?;
        require_string("manifest.entry.sha256", &entry.sha256, &self.source_sha256)?;
        require_usize("manifest.entry.bytes", entry.bytes, self.source_bytes)?;
        require_bool(
            "feasibility_record.generated_codec_possible_for_current_route",
            feasibility_record.generated_codec_possible_for_current_route,
            true,
        )?;
        require_string(
            "corpus_round_trip.supported_record.generated_codec_route",
            &self.generated_codec_route,
            REQUIRED_WHOLE_SOURCE_ROUTE_ID,
        )?;
        require_string(
            "corpus_round_trip.supported_record.generated_codec_marker",
            &self.generated_codec_marker,
            REQUIRED_WHOLE_SOURCE_GENERATED_CODEC_MARKER,
        )?;
        require_string(
            "corpus_round_trip.supported_record.report_path",
            &self.report_path,
            expected.report_path,
        )?;
        require_string(
            "corpus_round_trip.supported_record.report_command",
            &self.report_command,
            expected.report_command,
        )?;
        require_string(
            "corpus_round_trip.supported_record.generated_codec_round_trip_status",
            &self.generated_codec_round_trip_status,
            GENERATED_CODEC_ROUND_TRIP_STATUS,
        )?;
        require_usize(
            "corpus_round_trip.supported_record.tct_token_count",
            self.tct_token_count,
            expected.tct_token_count,
        )?;
        require_all(
            "corpus_round_trip.supported_record.replay_metadata_report_fields",
            &self.replay_metadata_report_fields,
            REQUIRED_REPLAY_METADATA_REPORT_FIELDS,
        )?;
        match (
            self.payload_sequence_control_token_count,
            expected.payload_sequence_control_token_count,
        ) {
            (Some(actual), Some(expected)) => {
                require_usize(
                    "corpus_round_trip.supported_record.payload_sequence_control_token_count",
                    actual,
                    expected,
                )?;
                require_all(
                    "corpus_round_trip.supported_record.retargeted_replay_metadata_report_fields",
                    &self.replay_metadata_report_fields,
                    REQUIRED_RETARGETED_REPLAY_METADATA_REPORT_FIELDS,
                )?;
            }
            (None, Some(_)) => {
                return Err(SyntaxBackendBoundaryError::new(
                    "corpus_round_trip.supported_record.payload_sequence_control_token_count",
                    "retargeted C# generated-codec records must preserve payload-sequence compatibility token count",
                ));
            }
            (Some(_), None) => {
                return Err(SyntaxBackendBoundaryError::new(
                    "corpus_round_trip.supported_record.payload_sequence_control_token_count",
                    "payload-sequence compatibility token count is only expected for retargeted C# generated-codec records",
                ));
            }
            (None, None) => {}
        }
        require_usize(
            "corpus_round_trip.supported_record.baseline_byte_token_count",
            self.baseline_byte_token_count,
            expected.baseline_byte_token_count,
        )?;
        require_string(
            "corpus_round_trip.supported_record.parser_metadata_validation",
            &self.parser_metadata_validation,
            "checked",
        )?;
        require_bool(
            "corpus_round_trip.supported_record.source_program_validity_claim_allowed",
            self.source_program_validity_claim_allowed,
            false,
        )?;
        require_bool(
            "corpus_round_trip.supported_record.type_correctness_claim_allowed",
            self.type_correctness_claim_allowed,
            false,
        )?;
        require_bool(
            "corpus_round_trip.supported_record.stable_promotion_allowed",
            self.stable_promotion_allowed,
            false,
        )?;
        require_bool(
            "corpus_round_trip.supported_record.arena_ir_proof_used",
            self.arena_ir_proof_used,
            false,
        )
    }
}

struct CSharpSupportedRecordPolicy {
    manifest_path: &'static str,
    source_path: &'static str,
    source_sha256: &'static str,
    report_path: &'static str,
    report_command: &'static str,
    tct_token_count: usize,
    payload_sequence_control_token_count: Option<usize>,
    baseline_byte_token_count: usize,
}

fn supported_record_policy(
    manifest_path: &str,
) -> Result<CSharpSupportedRecordPolicy, SyntaxBackendBoundaryError> {
    match manifest_path {
        SUPPORTED_MANIFEST_PATH => Ok(CSharpSupportedRecordPolicy {
            manifest_path: SUPPORTED_MANIFEST_PATH,
            source_path: SUPPORTED_SOURCE_PATH,
            source_sha256: SUPPORTED_SOURCE_SHA256,
            report_path: GENERATED_CODEC_REPORT_PATH,
            report_command: GENERATED_CODEC_REPORT_COMMAND,
            tct_token_count: REQUIRED_SUPPORTED_TCT_TOKEN_COUNT,
            payload_sequence_control_token_count: Some(
                REQUIRED_SUPPORTED_PAYLOAD_SEQUENCE_CONTROL_TOKEN_COUNT,
            ),
            baseline_byte_token_count: REQUIRED_SUPPORTED_BASELINE_BYTE_TOKEN_COUNT,
        }),
        LOGGER_MANIFEST_PATH => Ok(CSharpSupportedRecordPolicy {
            manifest_path: LOGGER_MANIFEST_PATH,
            source_path: LOGGER_SOURCE_PATH,
            source_sha256: LOGGER_SOURCE_SHA256,
            report_path: LOGGER_GENERATED_CODEC_REPORT_PATH,
            report_command: LOGGER_GENERATED_CODEC_REPORT_COMMAND,
            tct_token_count: REQUIRED_LOGGER_TCT_TOKEN_COUNT,
            payload_sequence_control_token_count: Some(
                REQUIRED_LOGGER_PAYLOAD_SEQUENCE_CONTROL_TOKEN_COUNT,
            ),
            baseline_byte_token_count: REQUIRED_LOGGER_BASELINE_BYTE_TOKEN_COUNT,
        }),
        VERSION_ONE_MANIFEST_PATH => Ok(CSharpSupportedRecordPolicy {
            manifest_path: VERSION_ONE_MANIFEST_PATH,
            source_path: VERSION_ONE_SOURCE_PATH,
            source_sha256: VERSION_ONE_SOURCE_SHA256,
            report_path: VERSION_ONE_GENERATED_CODEC_REPORT_PATH,
            report_command: VERSION_ONE_GENERATED_CODEC_REPORT_COMMAND,
            tct_token_count: REQUIRED_VERSION_ONE_TCT_TOKEN_COUNT,
            payload_sequence_control_token_count: Some(
                REQUIRED_VERSION_ONE_PAYLOAD_SEQUENCE_CONTROL_TOKEN_COUNT,
            ),
            baseline_byte_token_count: REQUIRED_VERSION_ONE_BASELINE_BYTE_TOKEN_COUNT,
        }),
        ITERATOR_METHODS_MANIFEST_PATH => Ok(CSharpSupportedRecordPolicy {
            manifest_path: ITERATOR_METHODS_MANIFEST_PATH,
            source_path: ITERATOR_METHODS_SOURCE_PATH,
            source_sha256: ITERATOR_METHODS_SOURCE_SHA256,
            report_path: ITERATOR_METHODS_GENERATED_CODEC_REPORT_PATH,
            report_command: ITERATOR_METHODS_GENERATED_CODEC_REPORT_COMMAND,
            tct_token_count: REQUIRED_ITERATOR_METHODS_TCT_TOKEN_COUNT,
            payload_sequence_control_token_count: Some(
                REQUIRED_ITERATOR_METHODS_PAYLOAD_SEQUENCE_CONTROL_TOKEN_COUNT,
            ),
            baseline_byte_token_count: REQUIRED_ITERATOR_METHODS_BASELINE_BYTE_TOKEN_COUNT,
        }),
        LOAN_MANIFEST_PATH => Ok(CSharpSupportedRecordPolicy {
            manifest_path: LOAN_MANIFEST_PATH,
            source_path: LOAN_SOURCE_PATH,
            source_sha256: LOAN_SOURCE_SHA256,
            report_path: LOAN_GENERATED_CODEC_REPORT_PATH,
            report_command: LOAN_GENERATED_CODEC_REPORT_COMMAND,
            tct_token_count: REQUIRED_LOAN_TCT_TOKEN_COUNT,
            payload_sequence_control_token_count: Some(
                REQUIRED_LOAN_PAYLOAD_SEQUENCE_CONTROL_TOKEN_COUNT,
            ),
            baseline_byte_token_count: REQUIRED_LOAN_BASELINE_BYTE_TOKEN_COUNT,
        }),
        other => Err(SyntaxBackendBoundaryError::new(
            "corpus_round_trip.supported_record.manifest_path",
            format!("unsupported C# generated-codec corpus record {other}"),
        )),
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CSharpGeneratedCodecCorpusBlockedRecord {
    pub manifest_path: String,
    pub source_path: String,
    pub source_sha256: String,
    pub source_bytes: usize,
    pub generated_codec_round_trip_status: String,
    pub blocker: String,
    pub route_id: String,
    pub generated_codec_marker: String,
    pub next_actionable_prerequisite: String,
    pub source_program_validity_claim_allowed: bool,
    pub type_correctness_claim_allowed: bool,
    pub stable_promotion_allowed: bool,
    pub arena_ir_proof_used: bool,
}

impl CSharpGeneratedCodecCorpusBlockedRecord {
    fn validate(
        &self,
        entry: &CSharpRealSourceManifestEntry,
        feasibility_record: &CSharpGeneratedCodecFeasibilityRecord,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        if self.manifest_path == SUPPORTED_MANIFEST_PATH
            || self.manifest_path == LOGGER_MANIFEST_PATH
            || self.manifest_path == VERSION_ONE_MANIFEST_PATH
            || self.manifest_path == ITERATOR_METHODS_MANIFEST_PATH
            || self.manifest_path == LOAN_MANIFEST_PATH
        {
            return Err(SyntaxBackendBoundaryError::new(
                "corpus_round_trip.blocked_record.manifest_path",
                "supported C# generated-codec record cannot be listed as blocked",
            ));
        }
        require_string(
            "corpus_round_trip.blocked_record.source_path",
            &self.source_path,
            &format!("{SOURCE_ROOT}/{}", self.manifest_path),
        )?;
        require_string(
            "manifest.entry.manifest_path",
            &entry.manifest_path,
            &self.manifest_path,
        )?;
        require_string("manifest.entry.sha256", &entry.sha256, &self.source_sha256)?;
        require_usize("manifest.entry.bytes", entry.bytes, self.source_bytes)?;
        require_bool(
            "feasibility_record.generated_codec_possible_for_current_route",
            feasibility_record.generated_codec_possible_for_current_route,
            false,
        )?;
        require_string(
            "corpus_round_trip.blocked_record.generated_codec_round_trip_status",
            &self.generated_codec_round_trip_status,
            BLOCKED_RECORD_STATUS,
        )?;
        require_string(
            "corpus_round_trip.blocked_record.blocker",
            &self.blocker,
            &feasibility_record.blocker,
        )?;
        require_string(
            "corpus_round_trip.blocked_record.route_id",
            &self.route_id,
            REQUIRED_WHOLE_SOURCE_ROUTE_ID,
        )?;
        require_string(
            "corpus_round_trip.blocked_record.generated_codec_marker",
            &self.generated_codec_marker,
            REQUIRED_WHOLE_SOURCE_GENERATED_CODEC_MARKER,
        )?;
        require_string(
            "corpus_round_trip.blocked_record.next_actionable_prerequisite",
            &self.next_actionable_prerequisite,
            expected_blocked_record_next_action(feasibility_record),
        )?;
        require_bool(
            "corpus_round_trip.blocked_record.source_program_validity_claim_allowed",
            self.source_program_validity_claim_allowed,
            false,
        )?;
        require_bool(
            "corpus_round_trip.blocked_record.type_correctness_claim_allowed",
            self.type_correctness_claim_allowed,
            false,
        )?;
        require_bool(
            "corpus_round_trip.blocked_record.stable_promotion_allowed",
            self.stable_promotion_allowed,
            false,
        )?;
        require_bool(
            "corpus_round_trip.blocked_record.arena_ir_proof_used",
            self.arena_ir_proof_used,
            false,
        )
    }
}

fn manifest_entries_by_path(
    corpus_manifest: &CSharpRealSourceCorpusManifest,
) -> BTreeMap<&str, &CSharpRealSourceManifestEntry> {
    corpus_manifest
        .source_manifest
        .entries
        .iter()
        .map(|entry| (entry.manifest_path.as_str(), entry))
        .collect()
}

fn feasibility_records_by_path(
    feasibility_gate: &SyntaxCSharpRealSourceGeneratedCodecFeasibilityGate,
) -> BTreeMap<&str, &CSharpGeneratedCodecFeasibilityRecord> {
    feasibility_gate
        .records
        .iter()
        .map(|record| (record.manifest_path.as_str(), record))
        .collect()
}

fn manifest_entry<'a>(
    manifest_by_path: &'a BTreeMap<&str, &'a CSharpRealSourceManifestEntry>,
    manifest_path: &str,
) -> Result<&'a CSharpRealSourceManifestEntry, SyntaxBackendBoundaryError> {
    manifest_by_path.get(manifest_path).copied().ok_or_else(|| {
        SyntaxBackendBoundaryError::new(
            "corpus_round_trip.records",
            format!("unknown C# manifest path {manifest_path}"),
        )
    })
}

fn feasibility_record<'a>(
    feasibility_by_path: &'a BTreeMap<&str, &'a CSharpGeneratedCodecFeasibilityRecord>,
    manifest_path: &str,
) -> Result<&'a CSharpGeneratedCodecFeasibilityRecord, SyntaxBackendBoundaryError> {
    feasibility_by_path
        .get(manifest_path)
        .copied()
        .ok_or_else(|| {
            SyntaxBackendBoundaryError::new(
                "corpus_round_trip.records",
                format!("missing C# feasibility record {manifest_path}"),
            )
        })
}

fn expected_blocked_record_next_action(
    feasibility_record: &CSharpGeneratedCodecFeasibilityRecord,
) -> &str {
    if feasibility_record.blocking_stage == FEASIBILITY_GENERATED_CODEC_FIXTURE_BLOCKING_STAGE
        || feasibility_record.blocker == FEASIBILITY_GENERATED_CODEC_FIXTURE_BLOCKER
    {
        &feasibility_record.generated_codec_prerequisite
    } else {
        debug_assert_eq!(
            feasibility_record.blocker,
            FEASIBILITY_CHECKED_METADATA_BLOCKER
        );
        &feasibility_record.projector_prerequisite
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

fn require_contains(
    field: &'static str,
    actual: &[String],
    expected: &str,
) -> Result<(), SyntaxBackendBoundaryError> {
    if actual.iter().any(|value| value == expected) {
        Ok(())
    } else {
        Err(SyntaxBackendBoundaryError::new(
            field,
            format!("missing required value {expected}"),
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
