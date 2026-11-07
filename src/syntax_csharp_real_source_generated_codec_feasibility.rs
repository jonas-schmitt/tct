//! C# real-source generated-codec corpus feasibility gate.
//!
//! This checked gate consumes the C# no-gap syntax-metadata handoff and the
//! pinned `csharp-dotnet-samples-mini-v1` source corpus, then records the first
//! generated-codec corpus blocker at the syntax level. It does not promote
//! arena IR, source-program validity, type correctness, generated hot paths, or
//! stable backend status.

use crate::syntax_backend_boundary::SyntaxBackendBoundaryError;
use crate::syntax_route_manifest::{self, SyntaxProductionRouteManifest};
use crate::syntax_source_corpus_evidence::{
    self, CSHARP_NO_GAP_CORPUS_ID, CSHARP_NO_GAP_CORPUS_MANIFEST_PATH,
    CSHARP_NO_GAP_EXPECTED_FILE_COUNT, CSHARP_NO_GAP_METADATA_REPORT_ARTIFACT_KIND,
    CSHARP_NO_GAP_METADATA_REPORT_COMMAND, CSHARP_NO_GAP_METADATA_REPORT_PATH,
    CSHARP_NO_GAP_METADATA_REPORT_TOOL, CSHARP_NO_GAP_SOURCE_MANIFEST_PATH,
    CSHARP_NO_GAP_SOURCE_ROOT, SyntaxSourceCorpusEvidenceGate,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const TOOL: &str = "syntax_backend_csharp_real_source_generated_codec_feasibility_gate";
pub const VERSION: u32 = 1;
pub const ARTIFACT_KIND: &str =
    "tct_syntax_backend_csharp_real_source_generated_codec_feasibility_gate";
pub const FIXTURE_PATH: &str =
    "tests/fixtures/syntax_backend/csharp-real-source-generated-codec-feasibility-v1.json";
pub const STATUS: &str = "csharp_real_source_generated_codec_feasibility_complete";
pub const COMPLETED_PROMOTION_CLASS: &str =
    "syntax backend C# real-source generated-codec corpus feasibility gate";
pub const NEXT_MISSING_STABLE_PROMOTION_CLASS: &str =
    "syntax backend C# real-source generated-codec corpus completion handoff tranche";

pub const REQUIRED_WHOLE_SOURCE_ROUTE_ID: &str = "csharp_compilation_unit_source_file_syntax_v0";
pub const REQUIRED_WHOLE_SOURCE_GENERATED_CODEC_MARKER: &str =
    "generated_csharp_compilation_unit_source_file_round_trips";
pub const CHECKED_METADATA_BLOCKER: &str =
    "missing_csharp_real_source_checked_route_metadata_fixture";
pub const CHECKED_METADATA_BLOCKING_STAGE: &str = "checked_metadata_fixture";
pub const GENERATED_CODEC_FIXTURE_BLOCKER: &str =
    "missing_csharp_real_source_generated_codec_fixture_coverage";
pub const GENERATED_CODEC_FIXTURE_BLOCKING_STAGE: &str = "generated_codec_fixture_coverage";
pub const REQUIRED_BLOCKER: &str = SUPPORTED_BLOCKER;
pub const REQUIRED_BLOCKING_STAGE: &str = SUPPORTED_BLOCKING_STAGE;
pub const SUPPORTED_MANIFEST_PATH: &str = "pattern-matching/Shapes.cs";
pub const LOGGER_MANIFEST_PATH: &str = "delegates-and-events/Logger.cs";
pub const VERSION_ONE_MANIFEST_PATH: &str = "events/VersionOne.cs";
pub const ITERATOR_METHODS_MANIFEST_PATH: &str = "iterators/IteratorMethods.cs";
pub const LOAN_MANIFEST_PATH: &str = "serialization/Loan.cs";
pub const SUPPORTED_BLOCKER: &str = "none";
pub const SUPPORTED_BLOCKING_STAGE: &str = "supported";
pub const REQUIRED_ROUTE_PREREQUISITE: &str = "registered csharp_compilation_unit_source_file_syntax_v0 in SYNTAX_PRODUCTION_ROUTE_CONSTRUCTS";
pub const REQUIRED_PROJECTOR_PREREQUISITE: &str =
    "add checked Roslyn CompilationUnitSyntax metadata fixture for this corpus file";
pub const REQUIRED_GENERATED_CODEC_PREREQUISITE: &str = "extend csharp_compilation_unit_source_file_syntax_v0 generated-codec fixture coverage to this corpus file";
pub const REQUIRED_NEXT_ACTIONABLE_PREREQUISITE: &str =
    "curate C# real-source generated-codec corpus completion handoff from five supported records";
pub const SUPPORTED_PROJECTOR_PREREQUISITE: &str =
    "checked Roslyn CompilationUnitSyntax metadata fixture exists for pattern-matching/Shapes.cs";
pub const SUPPORTED_GENERATED_CODEC_PREREQUISITE: &str =
    "generated codec route exists for pattern-matching/Shapes.cs";
pub const LOGGER_PROJECTOR_PREREQUISITE: &str = "checked Roslyn CompilationUnitSyntax metadata fixture exists for delegates-and-events/Logger.cs";
pub const LOGGER_GENERATED_CODEC_PREREQUISITE: &str =
    "generated codec route exists for delegates-and-events/Logger.cs";
pub const VERSION_ONE_PROJECTOR_PREREQUISITE: &str =
    "checked Roslyn CompilationUnitSyntax metadata fixture exists for events/VersionOne.cs";
pub const VERSION_ONE_GENERATED_CODEC_PREREQUISITE: &str =
    "generated codec route exists for events/VersionOne.cs";
pub const ITERATOR_METHODS_PROJECTOR_PREREQUISITE: &str =
    "checked Roslyn CompilationUnitSyntax metadata fixture exists for iterators/IteratorMethods.cs";
pub const ITERATOR_METHODS_GENERATED_CODEC_PREREQUISITE: &str =
    "generated codec route exists for iterators/IteratorMethods.cs";
pub const LOAN_PROJECTOR_PREREQUISITE: &str =
    "checked Roslyn CompilationUnitSyntax metadata fixture exists for serialization/Loan.cs";
pub const LOAN_GENERATED_CODEC_PREREQUISITE: &str =
    "generated codec route exists for serialization/Loan.cs";

const REQUIRED_FEASIBLE_RECORD_COUNT: usize = 5;
const REQUIRED_BLOCKED_RECORD_COUNT: usize = 0;
const REQUIRED_CORPUS_MANIFEST_ARTIFACT_KIND: &str = "tct_csharp_real_source_corpus_selection";
const REQUIRED_CORPUS_SELECTION_POLICY: &str = "Small standalone C# files from dotnet/samples chosen for parser/toolchain route planning; sidecar evidence only.";
const REQUIRED_SOURCE_MANIFEST_MARKER: &str = "tct-csharp-route-corpus-v1";
const REQUIRED_SOURCE_MANIFEST_SHA256: &str =
    "0d8d671a2783f1a7db675174c30f6daa90ba03c0f86644902edfb4cee6bdba21";

const REQUIRED_ALLOWED_CLAIMS: &[&str] = &[
    "metadata_gap_absence_for_checked_csharp_real_source_corpus",
    "generated_codec_feasibility_complete_for_checked_csharp_real_source_corpus",
    "single_csharp_whole_source_generated_codec_route_supported",
    "logger_csharp_whole_source_metadata_fixture_checked",
    "logger_csharp_whole_source_generated_codec_route_supported",
    "version_one_csharp_whole_source_metadata_fixture_checked",
    "version_one_csharp_whole_source_generated_codec_route_supported",
    "iterator_methods_csharp_whole_source_metadata_fixture_checked",
    "iterator_methods_csharp_whole_source_generated_codec_route_supported",
    "loan_csharp_whole_source_metadata_fixture_checked",
    "loan_csharp_whole_source_generated_codec_route_supported",
    "parser_backed_source_corpus_identity_preserved",
];
const REQUIRED_BLOCKED_CLAIMS: &[&str] = &[
    "source_program_validity",
    "whole_source_language_validity",
    "type_correctness",
    "functional_correctness",
    "generated_hot_path_behavior",
    "arena_ir_promotion",
    "stable_backend_promotion",
    "cross_repo_paper_claim",
    "model_quality_claim",
];
const REQUIRED_NO_STABLE_CLAIMS: &[&str] = &[
    "no_generated_codec_real_source_claim",
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
    "cargo test --test syntax_backend_boundary_test syntax_backend_csharp_real_source_generated_codec_feasibility_gate_records_first_supported_route -- --exact",
    "cargo test --test syntax_generated_codec_gate_test csharp_compilation_unit_source_file_syntax_generated_codec_compiles_through_compiler_route -- --exact",
    "cargo test --test syntax_generated_codec_gate_test csharp_compilation_unit_logger_source_file_syntax_generated_codec_compiles_through_compiler_route -- --exact",
    "cargo test --test syntax_generated_codec_gate_test csharp_compilation_unit_version_one_source_file_syntax_generated_codec_compiles_through_compiler_route -- --exact",
    "cargo test --test syntax_generated_codec_gate_test csharp_compilation_unit_iterator_methods_source_file_syntax_generated_codec_compiles_through_compiler_route -- --exact",
    "cargo test --test syntax_generated_codec_gate_test csharp_compilation_unit_loan_source_file_syntax_generated_codec_compiles_through_compiler_route -- --exact",
    "python3 -m unittest tests.python.test_structural_model_boundary_selection tests.python.test_syntax_backend_gate_strategy",
    "python3 scripts/testing/test-quality-audit.py --changed-from origin/main --fail-on-gaps",
    "git diff --check",
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxCSharpRealSourceGeneratedCodecFeasibilityGate {
    pub artifact_kind: String,
    pub version: u32,
    pub tool: String,
    pub status: String,
    pub source_corpus_evidence: CSharpFeasibilitySourceCorpusEvidence,
    pub production_route_manifest: CSharpFeasibilityProductionRouteManifest,
    pub corpus_manifest: CSharpFeasibilityCorpusManifestRef,
    pub feasibility: CSharpGeneratedCodecFeasibilitySummary,
    pub records: Vec<CSharpGeneratedCodecFeasibilityRecord>,
    pub allowed_claims: Vec<String>,
    pub blocked_claims: Vec<String>,
    pub no_stable_claims: Vec<String>,
    pub validation_commands: Vec<String>,
    pub completed_promotion_class: String,
    pub next_missing_stable_promotion_class: String,
}

impl SyntaxCSharpRealSourceGeneratedCodecFeasibilityGate {
    pub fn validate_against_sources(
        &self,
        source_corpus_gate: &SyntaxSourceCorpusEvidenceGate,
        route_manifest: &SyntaxProductionRouteManifest,
        corpus_manifest: &CSharpRealSourceCorpusManifest,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        require_string("artifact_kind", &self.artifact_kind, ARTIFACT_KIND)?;
        require_u32("version", self.version, VERSION)?;
        require_string("tool", &self.tool, TOOL)?;
        require_string("status", &self.status, STATUS)?;
        self.source_corpus_evidence.validate(source_corpus_gate)?;
        self.production_route_manifest.validate(route_manifest)?;
        self.corpus_manifest.validate(corpus_manifest)?;
        self.feasibility
            .validate(route_manifest, corpus_manifest, &self.records)?;
        validate_records(&self.records, corpus_manifest)?;
        require_all(
            "allowed_claims",
            &self.allowed_claims,
            REQUIRED_ALLOWED_CLAIMS,
        )?;
        require_all(
            "blocked_claims",
            &self.blocked_claims,
            REQUIRED_BLOCKED_CLAIMS,
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
pub struct CSharpFeasibilitySourceCorpusEvidence {
    pub artifact_kind: String,
    pub version: u32,
    pub fixture_path: String,
    pub completed_promotion_class: String,
    pub no_gap_metadata_report_artifact_kind: String,
    pub no_gap_metadata_report_tool: String,
    pub no_gap_metadata_report_path: String,
    pub no_gap_metadata_report_command: String,
    pub no_gap_corpus_id: String,
    pub no_gap_expected_file_count: usize,
    pub no_gap_expected_unsupported_node_kind_count: usize,
    pub no_gap_expected_unsupported_gap_count: usize,
    pub remaining_missing_evidence: Vec<String>,
}

impl CSharpFeasibilitySourceCorpusEvidence {
    fn validate(
        &self,
        source_corpus_gate: &SyntaxSourceCorpusEvidenceGate,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        require_string(
            "source_corpus_evidence.artifact_kind",
            &self.artifact_kind,
            syntax_source_corpus_evidence::ARTIFACT_KIND,
        )?;
        require_u32(
            "source_corpus_evidence.version",
            self.version,
            syntax_source_corpus_evidence::VERSION,
        )?;
        require_string(
            "source_corpus_evidence.fixture_path",
            &self.fixture_path,
            syntax_source_corpus_evidence::FIXTURE_PATH,
        )?;
        require_string(
            "source_corpus_evidence.completed_promotion_class",
            &self.completed_promotion_class,
            syntax_source_corpus_evidence::COMPLETED_PROMOTION_CLASS,
        )?;
        require_string(
            "source_corpus_evidence.no_gap_metadata_report_artifact_kind",
            &self.no_gap_metadata_report_artifact_kind,
            CSHARP_NO_GAP_METADATA_REPORT_ARTIFACT_KIND,
        )?;
        require_string(
            "source_corpus_evidence.no_gap_metadata_report_tool",
            &self.no_gap_metadata_report_tool,
            CSHARP_NO_GAP_METADATA_REPORT_TOOL,
        )?;
        require_string(
            "source_corpus_evidence.no_gap_metadata_report_path",
            &self.no_gap_metadata_report_path,
            CSHARP_NO_GAP_METADATA_REPORT_PATH,
        )?;
        require_string(
            "source_corpus_evidence.no_gap_metadata_report_command",
            &self.no_gap_metadata_report_command,
            CSHARP_NO_GAP_METADATA_REPORT_COMMAND,
        )?;
        require_string(
            "source_corpus_evidence.no_gap_corpus_id",
            &self.no_gap_corpus_id,
            CSHARP_NO_GAP_CORPUS_ID,
        )?;
        require_usize(
            "source_corpus_evidence.no_gap_expected_file_count",
            self.no_gap_expected_file_count,
            CSHARP_NO_GAP_EXPECTED_FILE_COUNT,
        )?;
        require_usize(
            "source_corpus_evidence.no_gap_expected_unsupported_node_kind_count",
            self.no_gap_expected_unsupported_node_kind_count,
            0,
        )?;
        require_usize(
            "source_corpus_evidence.no_gap_expected_unsupported_gap_count",
            self.no_gap_expected_unsupported_gap_count,
            0,
        )?;
        require_all(
            "source_corpus_evidence.remaining_missing_evidence",
            &self.remaining_missing_evidence,
            &["source_program_claim_handoff", "model_quality_evidence"],
        )?;
        let no_gap = &source_corpus_gate.csharp_no_gap_metadata_report;
        no_gap.validate()?;
        require_string(
            "source_corpus.csharp_no_gap_metadata_report.artifact_kind",
            &no_gap.artifact_kind,
            &self.no_gap_metadata_report_artifact_kind,
        )?;
        require_usize(
            "source_corpus.csharp_no_gap_metadata_report.expected_unsupported_node_kind_count",
            no_gap.expected_unsupported_node_kind_count,
            self.no_gap_expected_unsupported_node_kind_count,
        )?;
        require_usize(
            "source_corpus.csharp_no_gap_metadata_report.expected_unsupported_gap_count",
            no_gap.expected_unsupported_gap_count,
            self.no_gap_expected_unsupported_gap_count,
        )?;
        require_all(
            "source_corpus.corpus.missing_evidence",
            &source_corpus_gate.corpus.missing_evidence,
            &["source_program_claim_handoff", "model_quality_evidence"],
        )?;
        if source_corpus_gate
            .corpus
            .missing_evidence
            .iter()
            .any(|missing| {
                missing == "csharp_real_source_generated_codec_corpus_completion_evidence"
            })
        {
            return Err(SyntaxBackendBoundaryError::new(
                "source_corpus.corpus.missing_evidence",
                "C# generated-codec corpus completion evidence must not remain missing after handoff",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CSharpFeasibilityProductionRouteManifest {
    pub artifact_kind: String,
    pub version: u32,
    pub fixture_path: String,
    pub route_manifest_loader: String,
    pub registered_whole_source_route_id: String,
    pub registered_generated_codec_marker: String,
    pub remaining_blocker: String,
}

impl CSharpFeasibilityProductionRouteManifest {
    fn validate(
        &self,
        route_manifest: &SyntaxProductionRouteManifest,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        require_string(
            "production_route_manifest.artifact_kind",
            &self.artifact_kind,
            syntax_route_manifest::ARTIFACT_KIND,
        )?;
        require_u32(
            "production_route_manifest.version",
            self.version,
            syntax_route_manifest::VERSION,
        )?;
        require_string(
            "production_route_manifest.fixture_path",
            &self.fixture_path,
            syntax_route_manifest::FIXTURE_PATH,
        )?;
        require_string(
            "production_route_manifest.route_manifest_loader",
            &self.route_manifest_loader,
            syntax_route_manifest::ROUTE_MANIFEST_LOADER,
        )?;
        require_string(
            "production_route_manifest.registered_whole_source_route_id",
            &self.registered_whole_source_route_id,
            REQUIRED_WHOLE_SOURCE_ROUTE_ID,
        )?;
        require_string(
            "production_route_manifest.registered_generated_codec_marker",
            &self.registered_generated_codec_marker,
            REQUIRED_WHOLE_SOURCE_GENERATED_CODEC_MARKER,
        )?;
        require_string(
            "production_route_manifest.remaining_blocker",
            &self.remaining_blocker,
            REQUIRED_BLOCKER,
        )?;
        let whole_source_route = route_manifest
            .constructs
            .iter()
            .find(|construct| construct.grammar_id == REQUIRED_WHOLE_SOURCE_ROUTE_ID)
            .ok_or_else(|| {
                SyntaxBackendBoundaryError::new(
                    "production_route_manifest.registered_whole_source_route_id",
                    "C# generated-codec feasibility gate requires the whole-source route to be registered",
                )
            })?;
        require_contains(
            "production_route_manifest.registered_generated_codec_marker",
            &whole_source_route.generated_test_markers,
            REQUIRED_WHOLE_SOURCE_GENERATED_CODEC_MARKER,
        )?;
        // The C# route count is volatile under route promotions; the durable
        // structural requirements are the registered whole-source route and
        // its generated-codec marker checked above, plus a nonempty C# route
        // set.
        if !route_manifest
            .constructs
            .iter()
            .any(|construct| construct.target_id == "csharp")
        {
            return Err(SyntaxBackendBoundaryError::new(
                "production_route_manifest.csharp_routes",
                "C# generated-codec feasibility gate requires at least one registered C# route",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CSharpFeasibilityCorpusManifestRef {
    pub artifact_kind: String,
    pub version: u32,
    pub manifest_path: String,
    pub corpus_id: String,
    pub source_root: String,
    pub source_manifest_path: String,
    pub source_manifest_marker: String,
    pub source_manifest_sha256: String,
    pub expected_entry_count: usize,
    pub selection_policy: String,
}

impl CSharpFeasibilityCorpusManifestRef {
    fn validate(
        &self,
        corpus_manifest: &CSharpRealSourceCorpusManifest,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        corpus_manifest.validate()?;
        require_string(
            "corpus_manifest.artifact_kind",
            &self.artifact_kind,
            REQUIRED_CORPUS_MANIFEST_ARTIFACT_KIND,
        )?;
        require_u32("corpus_manifest.version", self.version, VERSION)?;
        require_string(
            "corpus_manifest.manifest_path",
            &self.manifest_path,
            CSHARP_NO_GAP_CORPUS_MANIFEST_PATH,
        )?;
        require_string(
            "corpus_manifest.corpus_id",
            &self.corpus_id,
            CSHARP_NO_GAP_CORPUS_ID,
        )?;
        require_string(
            "corpus_manifest.source_root",
            &self.source_root,
            CSHARP_NO_GAP_SOURCE_ROOT,
        )?;
        require_string(
            "corpus_manifest.source_manifest_path",
            &self.source_manifest_path,
            CSHARP_NO_GAP_SOURCE_MANIFEST_PATH,
        )?;
        require_string(
            "corpus_manifest.source_manifest_marker",
            &self.source_manifest_marker,
            REQUIRED_SOURCE_MANIFEST_MARKER,
        )?;
        require_string(
            "corpus_manifest.source_manifest_sha256",
            &self.source_manifest_sha256,
            REQUIRED_SOURCE_MANIFEST_SHA256,
        )?;
        require_usize(
            "corpus_manifest.expected_entry_count",
            self.expected_entry_count,
            CSHARP_NO_GAP_EXPECTED_FILE_COUNT,
        )?;
        require_string(
            "corpus_manifest.selection_policy",
            &self.selection_policy,
            REQUIRED_CORPUS_SELECTION_POLICY,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CSharpGeneratedCodecFeasibilitySummary {
    pub target_id: String,
    pub corpus_id: String,
    pub source_file_count: usize,
    pub no_gap_metadata_file_count: usize,
    pub feasible_record_count: usize,
    pub blocked_record_count: usize,
    pub first_blocker: String,
    pub first_blocking_stage: String,
    pub next_actionable_prerequisite: String,
    pub generated_codec_real_source_claim_allowed: bool,
    pub source_program_validity_claim_allowed: bool,
    pub type_correctness_claim_allowed: bool,
    pub arena_ir_proof_used: bool,
    pub stable_backend_promotion_allowed: bool,
}

impl CSharpGeneratedCodecFeasibilitySummary {
    fn validate(
        &self,
        _route_manifest: &SyntaxProductionRouteManifest,
        corpus_manifest: &CSharpRealSourceCorpusManifest,
        records: &[CSharpGeneratedCodecFeasibilityRecord],
    ) -> Result<(), SyntaxBackendBoundaryError> {
        require_string("feasibility.target_id", &self.target_id, "csharp")?;
        require_string(
            "feasibility.corpus_id",
            &self.corpus_id,
            CSHARP_NO_GAP_CORPUS_ID,
        )?;
        require_usize(
            "feasibility.source_file_count",
            self.source_file_count,
            corpus_manifest.source_manifest.entry_count,
        )?;
        require_usize(
            "feasibility.source_file_count",
            self.source_file_count,
            CSHARP_NO_GAP_EXPECTED_FILE_COUNT,
        )?;
        require_usize(
            "feasibility.no_gap_metadata_file_count",
            self.no_gap_metadata_file_count,
            CSHARP_NO_GAP_EXPECTED_FILE_COUNT,
        )?;
        // The live C# route count is volatile under route promotions and is
        // not pinned here; route-set structure is validated by the
        // production-route-manifest block.
        require_usize(
            "feasibility.feasible_record_count",
            self.feasible_record_count,
            REQUIRED_FEASIBLE_RECORD_COUNT,
        )?;
        let actual_feasible_record_count = records
            .iter()
            .filter(|record| record.generated_codec_possible_for_current_route)
            .count();
        require_usize(
            "feasibility.feasible_record_count",
            self.feasible_record_count,
            actual_feasible_record_count,
        )?;
        require_usize(
            "feasibility.blocked_record_count",
            self.blocked_record_count,
            REQUIRED_BLOCKED_RECORD_COUNT,
        )?;
        let actual_blocked_record_count = records.len() - actual_feasible_record_count;
        require_usize(
            "feasibility.blocked_record_count",
            self.blocked_record_count,
            actual_blocked_record_count,
        )?;
        require_string(
            "feasibility.first_blocker",
            &self.first_blocker,
            REQUIRED_BLOCKER,
        )?;
        require_string(
            "feasibility.first_blocking_stage",
            &self.first_blocking_stage,
            REQUIRED_BLOCKING_STAGE,
        )?;
        require_string(
            "feasibility.next_actionable_prerequisite",
            &self.next_actionable_prerequisite,
            REQUIRED_NEXT_ACTIONABLE_PREREQUISITE,
        )?;
        require_bool(
            "feasibility.generated_codec_real_source_claim_allowed",
            self.generated_codec_real_source_claim_allowed,
            false,
        )?;
        require_bool(
            "feasibility.source_program_validity_claim_allowed",
            self.source_program_validity_claim_allowed,
            false,
        )?;
        require_bool(
            "feasibility.type_correctness_claim_allowed",
            self.type_correctness_claim_allowed,
            false,
        )?;
        require_bool(
            "feasibility.arena_ir_proof_used",
            self.arena_ir_proof_used,
            false,
        )?;
        require_bool(
            "feasibility.stable_backend_promotion_allowed",
            self.stable_backend_promotion_allowed,
            false,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CSharpGeneratedCodecFeasibilityRecord {
    pub manifest_path: String,
    pub source_path: String,
    pub source_sha256: String,
    pub source_bytes: usize,
    pub parser_backed_metadata_available: bool,
    pub unsupported_node_kind_count: usize,
    pub unsupported_gap_count: usize,
    pub whole_source_route_registered: bool,
    pub grammar_ir_lowering_possible_for_current_route: bool,
    pub structural_model_lowering_possible_for_current_route: bool,
    pub token_layout_possible_for_current_route: bool,
    pub generated_codec_possible_for_current_route: bool,
    pub blocking_stage: String,
    pub blocker: String,
    pub route_id: String,
    pub generated_codec_marker: String,
    pub route_prerequisite: String,
    pub projector_prerequisite: String,
    pub generated_codec_prerequisite: String,
    pub no_stable_claims: Vec<String>,
}

impl CSharpGeneratedCodecFeasibilityRecord {
    fn validate(
        &self,
        entry: &CSharpRealSourceManifestEntry,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        require_string(
            "records.manifest_path",
            &self.manifest_path,
            &entry.manifest_path,
        )?;
        require_string(
            "records.source_path",
            &self.source_path,
            &format!("{CSHARP_NO_GAP_SOURCE_ROOT}/{}", entry.manifest_path),
        )?;
        require_string("records.source_sha256", &self.source_sha256, &entry.sha256)?;
        require_usize("records.source_bytes", self.source_bytes, entry.bytes)?;
        require_bool(
            "records.parser_backed_metadata_available",
            self.parser_backed_metadata_available,
            true,
        )?;
        require_usize(
            "records.unsupported_node_kind_count",
            self.unsupported_node_kind_count,
            0,
        )?;
        require_usize(
            "records.unsupported_gap_count",
            self.unsupported_gap_count,
            0,
        )?;
        let supported = self.manifest_path == SUPPORTED_MANIFEST_PATH;
        let logger = self.manifest_path == LOGGER_MANIFEST_PATH;
        let version_one = self.manifest_path == VERSION_ONE_MANIFEST_PATH;
        let iterator_methods = self.manifest_path == ITERATOR_METHODS_MANIFEST_PATH;
        let loan = self.manifest_path == LOAN_MANIFEST_PATH;
        let checked_metadata_available =
            supported || logger || version_one || iterator_methods || loan;
        let generated_codec_available =
            supported || logger || version_one || iterator_methods || loan;
        require_bool(
            "records.whole_source_route_registered",
            self.whole_source_route_registered,
            true,
        )?;
        require_bool(
            "records.grammar_ir_lowering_possible_for_current_route",
            self.grammar_ir_lowering_possible_for_current_route,
            checked_metadata_available,
        )?;
        require_bool(
            "records.structural_model_lowering_possible_for_current_route",
            self.structural_model_lowering_possible_for_current_route,
            checked_metadata_available,
        )?;
        require_bool(
            "records.token_layout_possible_for_current_route",
            self.token_layout_possible_for_current_route,
            checked_metadata_available,
        )?;
        require_bool(
            "records.generated_codec_possible_for_current_route",
            self.generated_codec_possible_for_current_route,
            generated_codec_available,
        )?;
        let expected_blocking_stage = if generated_codec_available {
            SUPPORTED_BLOCKING_STAGE
        } else if checked_metadata_available {
            GENERATED_CODEC_FIXTURE_BLOCKING_STAGE
        } else {
            CHECKED_METADATA_BLOCKING_STAGE
        };
        require_string(
            "records.blocking_stage",
            &self.blocking_stage,
            expected_blocking_stage,
        )?;
        let expected_blocker = if generated_codec_available {
            SUPPORTED_BLOCKER
        } else if checked_metadata_available {
            GENERATED_CODEC_FIXTURE_BLOCKER
        } else {
            CHECKED_METADATA_BLOCKER
        };
        require_string("records.blocker", &self.blocker, expected_blocker)?;
        require_string(
            "records.route_id",
            &self.route_id,
            REQUIRED_WHOLE_SOURCE_ROUTE_ID,
        )?;
        require_string(
            "records.generated_codec_marker",
            &self.generated_codec_marker,
            REQUIRED_WHOLE_SOURCE_GENERATED_CODEC_MARKER,
        )?;
        require_string(
            "records.route_prerequisite",
            &self.route_prerequisite,
            REQUIRED_ROUTE_PREREQUISITE,
        )?;
        let expected_projector_prerequisite = if supported {
            SUPPORTED_PROJECTOR_PREREQUISITE
        } else if logger {
            LOGGER_PROJECTOR_PREREQUISITE
        } else if version_one {
            VERSION_ONE_PROJECTOR_PREREQUISITE
        } else if iterator_methods {
            ITERATOR_METHODS_PROJECTOR_PREREQUISITE
        } else if loan {
            LOAN_PROJECTOR_PREREQUISITE
        } else {
            REQUIRED_PROJECTOR_PREREQUISITE
        };
        require_string(
            "records.projector_prerequisite",
            &self.projector_prerequisite,
            expected_projector_prerequisite,
        )?;
        let expected_generated_codec_prerequisite = if supported {
            SUPPORTED_GENERATED_CODEC_PREREQUISITE
        } else if logger {
            LOGGER_GENERATED_CODEC_PREREQUISITE
        } else if version_one {
            VERSION_ONE_GENERATED_CODEC_PREREQUISITE
        } else if iterator_methods {
            ITERATOR_METHODS_GENERATED_CODEC_PREREQUISITE
        } else if loan {
            LOAN_GENERATED_CODEC_PREREQUISITE
        } else {
            REQUIRED_GENERATED_CODEC_PREREQUISITE
        };
        require_string(
            "records.generated_codec_prerequisite",
            &self.generated_codec_prerequisite,
            expected_generated_codec_prerequisite,
        )?;
        require_all(
            "records.no_stable_claims",
            &self.no_stable_claims,
            REQUIRED_NO_STABLE_CLAIMS,
        )
    }
}

fn validate_records(
    records: &[CSharpGeneratedCodecFeasibilityRecord],
    corpus_manifest: &CSharpRealSourceCorpusManifest,
) -> Result<(), SyntaxBackendBoundaryError> {
    require_usize(
        "records.len",
        records.len(),
        corpus_manifest.source_manifest.entry_count,
    )?;
    let mut seen_paths = BTreeSet::new();
    for (record, entry) in records.iter().zip(&corpus_manifest.source_manifest.entries) {
        record.validate(entry)?;
        if !seen_paths.insert(record.manifest_path.as_str()) {
            return Err(SyntaxBackendBoundaryError::new(
                "records.manifest_path",
                format!("duplicate manifest path {}", record.manifest_path),
            ));
        }
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CSharpRealSourceCorpusManifest {
    pub artifact_kind: String,
    pub version: u32,
    pub corpus: CSharpRealSourceCorpusIdentity,
    pub source_manifest: CSharpRealSourceManifest,
    pub route_readiness_boundaries: CSharpRealSourceRouteReadinessBoundaries,
}

impl CSharpRealSourceCorpusManifest {
    pub fn validate(&self) -> Result<(), SyntaxBackendBoundaryError> {
        require_string(
            "csharp_corpus_manifest.artifact_kind",
            &self.artifact_kind,
            REQUIRED_CORPUS_MANIFEST_ARTIFACT_KIND,
        )?;
        require_u32("csharp_corpus_manifest.version", self.version, VERSION)?;
        require_string(
            "csharp_corpus_manifest.corpus.corpus_id",
            &self.corpus.corpus_id,
            CSHARP_NO_GAP_CORPUS_ID,
        )?;
        require_string(
            "csharp_corpus_manifest.corpus.source_root",
            &self.corpus.source_root,
            CSHARP_NO_GAP_SOURCE_ROOT,
        )?;
        require_string(
            "csharp_corpus_manifest.corpus.source_manifest",
            &self.corpus.source_manifest,
            CSHARP_NO_GAP_SOURCE_MANIFEST_PATH,
        )?;
        require_string(
            "csharp_corpus_manifest.corpus.selection_policy",
            &self.corpus.selection_policy,
            REQUIRED_CORPUS_SELECTION_POLICY,
        )?;
        require_string(
            "csharp_corpus_manifest.source_manifest.marker",
            &self.source_manifest.marker,
            REQUIRED_SOURCE_MANIFEST_MARKER,
        )?;
        require_string(
            "csharp_corpus_manifest.source_manifest.sha256",
            &self.source_manifest.sha256,
            REQUIRED_SOURCE_MANIFEST_SHA256,
        )?;
        require_usize(
            "csharp_corpus_manifest.source_manifest.entry_count",
            self.source_manifest.entry_count,
            CSHARP_NO_GAP_EXPECTED_FILE_COUNT,
        )?;
        require_usize(
            "csharp_corpus_manifest.source_manifest.entries",
            self.source_manifest.entries.len(),
            CSHARP_NO_GAP_EXPECTED_FILE_COUNT,
        )?;
        require_string(
            "csharp_corpus_manifest.route_readiness_boundaries.claim",
            &self.route_readiness_boundaries.claim,
            "sidecar C# real-source corpus-selection evidence only",
        )?;
        require_all(
            "csharp_corpus_manifest.route_readiness_boundaries.no_stable_claims",
            &self.route_readiness_boundaries.no_stable_claims,
            &[
                "no_arena_ir_promotion",
                "no_source_program_validity_claim",
                "no_type_correctness_claim",
                "no_generated_codec_real_source_claim",
                "no_stable_backend_promotion",
            ],
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CSharpRealSourceCorpusIdentity {
    pub corpus_id: String,
    pub source_root: String,
    pub source_manifest: String,
    pub selection_policy: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CSharpRealSourceManifest {
    pub marker: String,
    pub sha256: String,
    pub entry_count: usize,
    pub entries: Vec<CSharpRealSourceManifestEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CSharpRealSourceManifestEntry {
    pub manifest_path: String,
    pub sha256: String,
    pub bytes: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CSharpRealSourceRouteReadinessBoundaries {
    pub claim: String,
    pub no_stable_claims: Vec<String>,
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
