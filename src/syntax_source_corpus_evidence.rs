//! Parser-backed source corpus evidence for the syntax production route.
//!
//! This module records the smallest source corpus currently supported by the
//! feature-limited stable syntax production route. It derives evidence from
//! checked C# and Go parser metadata through the checked route manifest loader;
//! it keeps `SYNTAX_PRODUCTION_ROUTE_CONSTRUCTS` as a cross-check and does not
//! route syntax input through the JSON Schema backend, change generated JSON
//! Schema output, or make source-program/model-quality claims.

use crate::csharp_syntax_metadata::CSharpSyntaxMetadataAdapter;
use crate::go_syntax_metadata::GoSyntaxMetadataAdapter;
use crate::grammar_ir::{GrammarIr, GrammarParserProvenance};
use crate::grammar_ir_structural_model::{
    GrammarStructuralModelSidecar, lower_grammar_ir_to_structural_model,
};
use crate::provenance::{file_provenance, json_sha256};
use crate::syntax_backend_boundary::{SyntaxBackendBoundaryError, SyntaxBackendCompilerBoundary};
use crate::syntax_generated_codec::{csharp_syntax_token_layout, go_syntax_token_layout};
use crate::syntax_production_route::{
    self, SyntaxProductionRouteImplementationGate, SyntaxProductionRouteProjectorMetadataSetup,
    SyntaxProductionRouteRequest, SyntaxProductionRouteTarget,
    compile_checked_syntax_production_route,
};
use crate::syntax_route_manifest::SyntaxProductionRouteManifestConstruct;
use crate::syntax_route_manifest::{self, SyntaxProductionRouteManifest};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::BTreeSet;
use std::path::Path;

pub const TOOL: &str = "syntax_backend_source_corpus_evidence";
pub const VERSION: u32 = 1;
pub const ARTIFACT_KIND: &str = "tct_syntax_backend_source_corpus_evidence";
pub const FIXTURE_PATH: &str = "tests/fixtures/syntax_backend/source-corpus-evidence-v1.json";
pub const STATUS: &str = "sidecar_evidence_contract";
pub const COMPLETED_PROMOTION_CLASS: &str =
    "syntax backend parser-backed source corpus evidence tranche";
pub const REPORT_PRODUCER_COMPLETED_PROMOTION_CLASS: &str =
    "syntax backend parser-backed source corpus report producer tranche";
pub const REPORT_VALIDATOR_COMPLETED_PROMOTION_CLASS: &str =
    "syntax backend parser-backed source corpus report validator tranche";
pub const PROJECTOR_METADATA_SETUP_COMPLETED_PROMOTION_CLASS: &str =
    "syntax backend projector-driven metadata route setup tranche";
pub const GO_MINIMAL_FUNCTION_COMPLETED_PROMOTION_CLASS: &str =
    "syntax backend Go minimal function production route tranche";
pub const NEXT_MISSING_STABLE_PROMOTION_CLASS: &str =
    crate::syntax_roadmap_successors::GO_SOURCE_RECORD_RAW_TOKEN_GENERATOR_CORE_SUCCESSOR_TASK;
pub const NEXT_STABLE_PROMOTION_BLOCKER: &str = "Feature-limited stable syntax route metadata is promoted; source-program claims remain blocked until source-corpus reports distinguish generated-codec validity from whole-source-language or model-quality evidence.";
pub const REPORT_PRODUCER_COMMAND: &str = "cargo run --bin tct-syntax-source-corpus-evidence -- --route-manifest tests/fixtures/syntax_backend/production-route-manifest-v1.json --output target/tct-syntax-source-corpus-evidence/parser-backed-source-corpus-evidence-v1.json";
pub const REPORT_VALIDATOR_COMMAND: &str = "cargo run --bin tct-syntax-source-corpus-evidence -- --route-manifest tests/fixtures/syntax_backend/production-route-manifest-v1.json --validate-report target/tct-syntax-source-corpus-evidence/parser-backed-source-corpus-evidence-v1.json";
pub const REPORT_PATH: &str =
    "target/tct-syntax-source-corpus-evidence/parser-backed-source-corpus-evidence-v1.json";
pub const CORPUS_ID: &str = "syntax-production-route-parser-backed-source-corpus-v1";
pub const CSHARP_NO_GAP_METADATA_REPORT_ARTIFACT_KIND: &str =
    "tct_csharp_real_source_metadata_no_gap_report_contract";
pub const CSHARP_NO_GAP_METADATA_REPORT_TOOL: &str = "csharp_real_source_metadata_gaps";
pub const CSHARP_NO_GAP_METADATA_REPORT_PATH: &str =
    "target/tct-csharp-real-source-metadata-gaps/metadata-gaps.json";
pub const CSHARP_NO_GAP_METADATA_MARKDOWN_REPORT_PATH: &str =
    "target/tct-csharp-real-source-metadata-gaps/metadata-gaps.md";
pub const CSHARP_NO_GAP_METADATA_REPORT_COMMAND: &str =
    "./scripts/testing/test-csharp-real-source-metadata-gaps.sh";
pub const CSHARP_NO_GAP_SOURCE_MANIFEST_PATH: &str =
    "corpora/csharp/dotnet-samples-mini/sources.txt";
pub const CSHARP_NO_GAP_SOURCE_ROOT: &str = "corpora/csharp/dotnet-samples-mini/src";
pub const CSHARP_NO_GAP_CORPUS_MANIFEST_PATH: &str =
    "corpora/csharp/dotnet-samples-mini/manifest-v1.json";
pub const CSHARP_NO_GAP_CORPUS_ID: &str = "csharp-dotnet-samples-mini-v1";
pub const CSHARP_NO_GAP_EXPECTED_FILE_COUNT: usize = 5;

const REQUIRED_TARGETS: &[&str] = &["csharp", "go"];
const REQUIRED_RECORD_FIELDS: &[&str] = &[
    "target_id",
    "grammar_id",
    "completed_promotion_class",
    "source_path",
    "source_sha256",
    "metadata_path",
    "metadata_sha256",
    "parser_tool",
    "parser_version",
    "parser_package",
    "parser_package_version",
    "rendered_source_sha256",
    "generated_codec_metadata_sha256",
    "generated_crate_identity_sha256",
    "generation_options_sha256",
    "structural_model_sha256",
    "token_layout_sha256",
    "token_layout_metadata",
    "validation_hook_status",
    "sidecar_controls",
    "generated_test_markers",
    "projector_metadata_setup",
    "bpe_boundary",
    "bpe_merge_fingerprints",
];
const REQUIRED_PRESERVED_ARTIFACTS: &[&str] = &[
    "source_path",
    "source_sha256",
    "checked_metadata_path",
    "checked_metadata_sha256",
    "parser_tool_version",
    "generated_code_hashes",
    "token_layout_metadata",
    "generated_test_markers",
    "projector_command",
    "projector_generated_output_path",
    "route_manifest_fixture",
    "route_manifest_loader",
    "bpe_boundary_or_merge_fingerprints",
    "csharp_no_gap_metadata_report_contract",
    "csharp_real_source_manifest",
];
const REQUIRED_NO_STABLE_CLAIMS: &[&str] = &[
    "no_generated_json_schema_output_change",
    "no_core_compiler_path_change",
    "no_json_schema_compiler_pass_change",
    "no_model_quality_claim",
    "no_cross_repo_paper_claim",
];
const REQUIRED_MISSING_EVIDENCE: &[&str] =
    &["source_program_claim_handoff", "model_quality_evidence"];
const REQUIRED_CSHARP_NO_GAP_NO_STABLE_CLAIMS: &[&str] = &[
    "no_arena_ir_promotion",
    "no_source_program_validity_claim",
    "no_type_correctness_claim",
    "no_generated_codec_real_source_claim",
    "no_generated_hot_path_claim",
    "no_stable_backend_promotion",
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxSourceCorpusEvidenceGate {
    pub artifact_kind: String,
    pub version: u32,
    pub tool: String,
    pub status: String,
    pub evidence_level: String,
    pub source_production_route: SyntaxSourceCorpusProductionRoute,
    pub corpus: SyntaxSourceCorpusContract,
    pub csharp_no_gap_metadata_report: SyntaxSourceCorpusCSharpNoGapMetadataReport,
    pub report_policy: SyntaxSourceCorpusReportPolicy,
    pub compiler_boundary: SyntaxBackendCompilerBoundary,
    pub no_stable_claims: Vec<String>,
    pub completed_promotion_class: String,
    pub next_missing_stable_promotion_class: String,
}

impl SyntaxSourceCorpusEvidenceGate {
    pub fn validate_against_production_route_gate(
        &self,
        route_gate: &SyntaxProductionRouteImplementationGate,
        route_manifest: &SyntaxProductionRouteManifest,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        require_string("artifact_kind", &self.artifact_kind, ARTIFACT_KIND)?;
        require_u32("version", self.version, VERSION)?;
        require_string("tool", &self.tool, TOOL)?;
        require_string("status", &self.status, STATUS)?;
        require_string(
            "evidence_level",
            &self.evidence_level,
            "parser-backed source-corpus evidence for the feature-limited stable syntax route, not source-program evidence, not model-quality evidence",
        )?;
        self.source_production_route
            .validate(route_gate, route_manifest)?;
        self.corpus.validate()?;
        self.csharp_no_gap_metadata_report.validate()?;
        self.report_policy.validate()?;
        require_production_route_boundary(&self.compiler_boundary)?;
        require_all(
            "no_stable_claims",
            &self.no_stable_claims,
            REQUIRED_NO_STABLE_CLAIMS,
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
        )?;
        route_manifest.validate_against_route_gate_and_source_corpus(route_gate, self)
    }

    pub fn build_report(
        &self,
        route_gate: &SyntaxProductionRouteImplementationGate,
        route_manifest: &SyntaxProductionRouteManifest,
    ) -> Result<SyntaxSourceCorpusEvidenceReport, SyntaxBackendBoundaryError> {
        self.validate_against_production_route_gate(route_gate, route_manifest)?;
        let records = parser_backed_source_corpus_records(route_manifest, route_gate, self)?;
        let corpus_sha256 = json_sha256(&records).map_err(|error| {
            SyntaxBackendBoundaryError::new(
                "records",
                format!("failed to hash source corpus records: {error}"),
            )
        })?;
        Ok(SyntaxSourceCorpusEvidenceReport {
            artifact_kind: ARTIFACT_KIND.to_string(),
            version: VERSION,
            tool: TOOL.to_string(),
            status: "generated_sidecar_report".to_string(),
            corpus_id: self.corpus.corpus_id.clone(),
            source_production_route: self.source_production_route.clone(),
            record_count: records.len(),
            records_sha256: corpus_sha256,
            records,
            missing_evidence: self.corpus.missing_evidence.clone(),
            csharp_no_gap_metadata_report: self.csharp_no_gap_metadata_report.clone(),
            report_policy: self.report_policy.clone(),
            no_stable_claims: self.no_stable_claims.clone(),
            completed_promotion_class: self.completed_promotion_class.clone(),
            next_missing_stable_promotion_class: self.next_missing_stable_promotion_class.clone(),
        })
    }

    pub fn validate_report(
        &self,
        route_gate: &SyntaxProductionRouteImplementationGate,
        route_manifest: &SyntaxProductionRouteManifest,
        report: &SyntaxSourceCorpusEvidenceReport,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        let expected = self.build_report(route_gate, route_manifest)?;
        require_string("report.artifact_kind", &report.artifact_kind, ARTIFACT_KIND)?;
        require_u32("report.version", report.version, VERSION)?;
        require_string("report.tool", &report.tool, TOOL)?;
        require_string("report.status", &report.status, "generated_sidecar_report")?;
        require_string("report.corpus_id", &report.corpus_id, &expected.corpus_id)?;
        require_route_equal(
            "report.source_production_route",
            &report.source_production_route,
            &expected.source_production_route,
        )?;
        require_usize(
            "report.record_count",
            report.record_count,
            expected.record_count,
        )?;
        require_string(
            "report.records_sha256",
            &report.records_sha256,
            &expected.records_sha256,
        )?;
        require_records_equal("report.records", &report.records, &expected.records)?;
        require_vec_equal(
            "report.missing_evidence",
            &report.missing_evidence,
            &expected.missing_evidence,
        )?;
        require_csharp_no_gap_metadata_report_equal(
            "report.csharp_no_gap_metadata_report",
            &report.csharp_no_gap_metadata_report,
            &expected.csharp_no_gap_metadata_report,
        )?;
        require_report_policy_equal(
            "report.report_policy",
            &report.report_policy,
            &expected.report_policy,
        )?;
        require_vec_equal(
            "report.no_stable_claims",
            &report.no_stable_claims,
            &expected.no_stable_claims,
        )?;
        require_string(
            "report.completed_promotion_class",
            &report.completed_promotion_class,
            &expected.completed_promotion_class,
        )?;
        require_string(
            "report.next_missing_stable_promotion_class",
            &report.next_missing_stable_promotion_class,
            &expected.next_missing_stable_promotion_class,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxSourceCorpusProductionRoute {
    pub artifact_kind: String,
    pub version: u32,
    pub module_path: String,
    pub fixture_path: String,
    pub route_table: String,
    pub route_entrypoint: String,
    pub route_gate: String,
    pub route_manifest_fixture_path: String,
    pub route_manifest_loader: String,
    pub route_manifest_loader_completed_promotion_class: String,
    pub projector_metadata_setup_source: String,
    pub projector_metadata_setup_completed_promotion_class: String,
    pub completed_promotion_class: String,
    pub current_next_missing_stable_promotion_class: String,
}

impl SyntaxSourceCorpusProductionRoute {
    fn validate(
        &self,
        route_gate: &SyntaxProductionRouteImplementationGate,
        route_manifest: &SyntaxProductionRouteManifest,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        require_string(
            "source_production_route.artifact_kind",
            &self.artifact_kind,
            syntax_production_route::ARTIFACT_KIND,
        )?;
        require_u32(
            "source_production_route.version",
            self.version,
            syntax_production_route::VERSION,
        )?;
        require_string(
            "source_production_route.module_path",
            &self.module_path,
            "src/syntax_production_route.rs",
        )?;
        require_string(
            "source_production_route.fixture_path",
            &self.fixture_path,
            syntax_production_route::FIXTURE_PATH,
        )?;
        require_string(
            "source_production_route.route_table",
            &self.route_table,
            syntax_production_route::ROUTE_SETUP_SOURCE_OF_TRUTH,
        )?;
        require_string(
            "source_production_route.route_entrypoint",
            &self.route_entrypoint,
            syntax_production_route::ROUTE_ENTRYPOINT,
        )?;
        require_string(
            "source_production_route.route_gate",
            &self.route_gate,
            syntax_production_route::ROUTE_GATE,
        )?;
        require_string(
            "source_production_route.route_manifest_fixture_path",
            &self.route_manifest_fixture_path,
            syntax_route_manifest::FIXTURE_PATH,
        )?;
        require_string(
            "source_production_route.route_manifest_loader",
            &self.route_manifest_loader,
            syntax_route_manifest::ROUTE_MANIFEST_LOADER,
        )?;
        require_string(
            "source_production_route.route_manifest_loader_completed_promotion_class",
            &self.route_manifest_loader_completed_promotion_class,
            syntax_route_manifest::LOADER_COMPLETED_PROMOTION_CLASS,
        )?;
        require_string(
            "source_production_route.projector_metadata_setup_source",
            &self.projector_metadata_setup_source,
            syntax_production_route::PROJECTOR_METADATA_SETUP_SOURCE_OF_TRUTH,
        )?;
        require_string(
            "source_production_route.projector_metadata_setup_completed_promotion_class",
            &self.projector_metadata_setup_completed_promotion_class,
            PROJECTOR_METADATA_SETUP_COMPLETED_PROMOTION_CLASS,
        )?;
        require_string(
            "source_production_route.fixture_route_manifest_loader",
            &route_manifest.route_manifest_loader,
            syntax_route_manifest::ROUTE_MANIFEST_LOADER,
        )?;
        require_string(
            "source_production_route.fixture_projector_metadata_setup_source",
            &route_manifest.projector_metadata_setup_source,
            syntax_production_route::PROJECTOR_METADATA_SETUP_SOURCE_OF_TRUTH,
        )?;
        require_string(
            "source_production_route.fixture_projector_metadata_setup_completed_promotion_class",
            &route_manifest.projector_metadata_setup_completed_promotion_class,
            PROJECTOR_METADATA_SETUP_COMPLETED_PROMOTION_CLASS,
        )?;
        require_string(
            "source_production_route.completed_promotion_class",
            &self.completed_promotion_class,
            syntax_production_route::COMPLETED_PROMOTION_CLASS,
        )?;
        require_string(
            "source_production_route.current_next_missing_stable_promotion_class",
            &self.current_next_missing_stable_promotion_class,
            syntax_production_route::NEXT_MISSING_STABLE_PROMOTION_CLASS,
        )?;
        require_string(
            "source_production_route.fixture_next_missing_stable_promotion_class",
            &route_gate.next_missing_stable_promotion_class,
            syntax_production_route::NEXT_MISSING_STABLE_PROMOTION_CLASS,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxSourceCorpusContract {
    pub corpus_id: String,
    pub source_of_truth: String,
    pub selection_policy: String,
    pub supported_targets: Vec<String>,
    pub expected_construct_count: usize,
    pub required_record_fields: Vec<String>,
    pub preserved_artifacts: Vec<String>,
    pub missing_evidence: Vec<String>,
    pub next_stable_promotion_blocker: String,
}

impl SyntaxSourceCorpusContract {
    fn validate(&self) -> Result<(), SyntaxBackendBoundaryError> {
        require_string("corpus.corpus_id", &self.corpus_id, CORPUS_ID)?;
        require_string(
            "corpus.source_of_truth",
            &self.source_of_truth,
            syntax_production_route::ROUTE_SETUP_SOURCE_OF_TRUTH,
        )?;
        if !self
            .selection_policy
            .contains("checked C# syntax fixtures and checked Go syntax fixtures")
        {
            return Err(SyntaxBackendBoundaryError::new(
                "corpus.selection_policy",
                "selection policy must name the current parser-backed fixture subset",
            ));
        }
        require_all(
            "corpus.supported_targets",
            &self.supported_targets,
            REQUIRED_TARGETS,
        )?;
        require_usize(
            "corpus.expected_construct_count",
            self.expected_construct_count,
            syntax_production_route::SYNTAX_PRODUCTION_ROUTE_CONSTRUCTS.len(),
        )?;
        require_all(
            "corpus.required_record_fields",
            &self.required_record_fields,
            REQUIRED_RECORD_FIELDS,
        )?;
        require_all(
            "corpus.preserved_artifacts",
            &self.preserved_artifacts,
            REQUIRED_PRESERVED_ARTIFACTS,
        )?;
        require_all(
            "corpus.missing_evidence",
            &self.missing_evidence,
            REQUIRED_MISSING_EVIDENCE,
        )?;
        require_string(
            "corpus.next_stable_promotion_blocker",
            &self.next_stable_promotion_blocker,
            NEXT_STABLE_PROMOTION_BLOCKER,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxSourceCorpusReportPolicy {
    pub generated_report_path: String,
    pub generated_reports_checked_in: bool,
    pub checked_fixture_path: String,
    pub source_report_producer_command: String,
    pub source_report_producer_available: bool,
    pub source_report_validator_command: String,
    pub source_report_validator_available: bool,
}

impl SyntaxSourceCorpusReportPolicy {
    fn validate(&self) -> Result<(), SyntaxBackendBoundaryError> {
        require_string(
            "report_policy.generated_report_path",
            &self.generated_report_path,
            REPORT_PATH,
        )?;
        if !self.generated_report_path.starts_with("target/") {
            return Err(SyntaxBackendBoundaryError::new(
                "report_policy.generated_report_path",
                "generated source corpus reports must stay under target/",
            ));
        }
        require_bool(
            "report_policy.generated_reports_checked_in",
            self.generated_reports_checked_in,
            false,
        )?;
        require_string(
            "report_policy.checked_fixture_path",
            &self.checked_fixture_path,
            FIXTURE_PATH,
        )?;
        require_string(
            "report_policy.source_report_producer_command",
            &self.source_report_producer_command,
            REPORT_PRODUCER_COMMAND,
        )?;
        require_bool(
            "report_policy.source_report_producer_available",
            self.source_report_producer_available,
            true,
        )?;
        require_string(
            "report_policy.source_report_validator_command",
            &self.source_report_validator_command,
            REPORT_VALIDATOR_COMMAND,
        )?;
        require_bool(
            "report_policy.source_report_validator_available",
            self.source_report_validator_available,
            true,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxSourceCorpusCSharpNoGapMetadataReport {
    pub artifact_kind: String,
    pub version: u32,
    pub tool: String,
    pub report_path: String,
    pub markdown_report_path: String,
    pub report_command: String,
    pub source_manifest_path: String,
    pub source_root: String,
    pub corpus_manifest_path: String,
    pub corpus_id: String,
    pub expected_file_count: usize,
    pub expected_parsed_files: usize,
    pub expected_parse_diagnostics: usize,
    pub expected_unsupported_node_kind_count: usize,
    pub expected_unsupported_gap_count: usize,
    pub expected_next_smallest_tranche: Option<String>,
    pub parser_backed_gap_report_ready_required: bool,
    pub parser_package: String,
    pub parser_package_version: String,
    pub target_framework: String,
    pub evidence_boundary: String,
    pub no_stable_claims: Vec<String>,
}

impl SyntaxSourceCorpusCSharpNoGapMetadataReport {
    pub(crate) fn validate(&self) -> Result<(), SyntaxBackendBoundaryError> {
        require_string(
            "csharp_no_gap_metadata_report.artifact_kind",
            &self.artifact_kind,
            CSHARP_NO_GAP_METADATA_REPORT_ARTIFACT_KIND,
        )?;
        require_u32(
            "csharp_no_gap_metadata_report.version",
            self.version,
            VERSION,
        )?;
        require_string(
            "csharp_no_gap_metadata_report.tool",
            &self.tool,
            CSHARP_NO_GAP_METADATA_REPORT_TOOL,
        )?;
        require_string(
            "csharp_no_gap_metadata_report.report_path",
            &self.report_path,
            CSHARP_NO_GAP_METADATA_REPORT_PATH,
        )?;
        if !self.report_path.starts_with("target/") {
            return Err(SyntaxBackendBoundaryError::new(
                "csharp_no_gap_metadata_report.report_path",
                "C# no-gap metadata reports must stay under target/",
            ));
        }
        require_string(
            "csharp_no_gap_metadata_report.markdown_report_path",
            &self.markdown_report_path,
            CSHARP_NO_GAP_METADATA_MARKDOWN_REPORT_PATH,
        )?;
        if !self.markdown_report_path.starts_with("target/") {
            return Err(SyntaxBackendBoundaryError::new(
                "csharp_no_gap_metadata_report.markdown_report_path",
                "C# no-gap metadata markdown reports must stay under target/",
            ));
        }
        require_string(
            "csharp_no_gap_metadata_report.report_command",
            &self.report_command,
            CSHARP_NO_GAP_METADATA_REPORT_COMMAND,
        )?;
        require_string(
            "csharp_no_gap_metadata_report.source_manifest_path",
            &self.source_manifest_path,
            CSHARP_NO_GAP_SOURCE_MANIFEST_PATH,
        )?;
        require_string(
            "csharp_no_gap_metadata_report.source_root",
            &self.source_root,
            CSHARP_NO_GAP_SOURCE_ROOT,
        )?;
        require_string(
            "csharp_no_gap_metadata_report.corpus_manifest_path",
            &self.corpus_manifest_path,
            CSHARP_NO_GAP_CORPUS_MANIFEST_PATH,
        )?;
        require_string(
            "csharp_no_gap_metadata_report.corpus_id",
            &self.corpus_id,
            CSHARP_NO_GAP_CORPUS_ID,
        )?;
        require_usize(
            "csharp_no_gap_metadata_report.expected_file_count",
            self.expected_file_count,
            CSHARP_NO_GAP_EXPECTED_FILE_COUNT,
        )?;
        require_usize(
            "csharp_no_gap_metadata_report.expected_parsed_files",
            self.expected_parsed_files,
            CSHARP_NO_GAP_EXPECTED_FILE_COUNT,
        )?;
        require_usize(
            "csharp_no_gap_metadata_report.expected_parse_diagnostics",
            self.expected_parse_diagnostics,
            0,
        )?;
        require_usize(
            "csharp_no_gap_metadata_report.expected_unsupported_node_kind_count",
            self.expected_unsupported_node_kind_count,
            0,
        )?;
        require_usize(
            "csharp_no_gap_metadata_report.expected_unsupported_gap_count",
            self.expected_unsupported_gap_count,
            0,
        )?;
        if self.expected_next_smallest_tranche.is_some() {
            return Err(SyntaxBackendBoundaryError::new(
                "csharp_no_gap_metadata_report.expected_next_smallest_tranche",
                "C# no-gap metadata handoff must not name a remaining route tranche",
            ));
        }
        require_bool(
            "csharp_no_gap_metadata_report.parser_backed_gap_report_ready_required",
            self.parser_backed_gap_report_ready_required,
            true,
        )?;
        require_string(
            "csharp_no_gap_metadata_report.parser_package",
            &self.parser_package,
            "Microsoft.CodeAnalysis.CSharp",
        )?;
        require_string(
            "csharp_no_gap_metadata_report.parser_package_version",
            &self.parser_package_version,
            "5.3.0",
        )?;
        require_string(
            "csharp_no_gap_metadata_report.target_framework",
            &self.target_framework,
            "net10.0",
        )?;
        require_string(
            "csharp_no_gap_metadata_report.evidence_boundary",
            &self.evidence_boundary,
            "sidecar parser-backed C# real-source syntax-metadata gap evidence only",
        )?;
        require_all(
            "csharp_no_gap_metadata_report.no_stable_claims",
            &self.no_stable_claims,
            REQUIRED_CSHARP_NO_GAP_NO_STABLE_CLAIMS,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SyntaxSourceCorpusEvidenceReport {
    pub artifact_kind: String,
    pub version: u32,
    pub tool: String,
    pub status: String,
    pub corpus_id: String,
    pub source_production_route: SyntaxSourceCorpusProductionRoute,
    pub record_count: usize,
    pub records_sha256: String,
    pub records: Vec<SyntaxSourceCorpusEvidenceRecord>,
    pub missing_evidence: Vec<String>,
    pub csharp_no_gap_metadata_report: SyntaxSourceCorpusCSharpNoGapMetadataReport,
    pub report_policy: SyntaxSourceCorpusReportPolicy,
    pub no_stable_claims: Vec<String>,
    pub completed_promotion_class: String,
    pub next_missing_stable_promotion_class: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SyntaxSourceCorpusEvidenceRecord {
    pub target_id: String,
    pub grammar_id: String,
    pub completed_promotion_class: String,
    pub source_path: String,
    pub source_sha256: String,
    pub source_bytes: u64,
    pub metadata_path: String,
    pub metadata_sha256: String,
    pub metadata_bytes: u64,
    pub parser: SyntaxSourceCorpusParserEvidence,
    pub generated_codec: SyntaxSourceCorpusGeneratedCodecEvidence,
    pub structural_model_sha256: String,
    pub token_layout_sha256: String,
    pub token_layout: Value,
    pub validation_hook_status: Value,
    pub sidecar_controls: Value,
    pub generated_test_markers: Vec<String>,
    pub projector_metadata_setup: SyntaxProductionRouteProjectorMetadataSetup,
    pub bpe: SyntaxSourceCorpusBpeEvidence,
    pub route_metadata: SyntaxSourceCorpusRouteEvidence,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxSourceCorpusParserEvidence {
    pub tool: String,
    pub version: String,
    pub package: Option<String>,
    pub package_version: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxSourceCorpusGeneratedCodecEvidence {
    pub renderer: String,
    pub rendered_source_sha256: String,
    pub generated_codec_metadata_sha256: String,
    pub generated_crate_identity_sha256: String,
    pub generation_options_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SyntaxSourceCorpusBpeEvidence {
    pub bpe_boundary: Value,
    pub merge_fingerprints: Vec<String>,
    pub merge_artifact_used: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxSourceCorpusRouteEvidence {
    pub accepted_input: String,
    pub route_entrypoint: String,
    pub route_gate: String,
    pub stable_backend_promoted: bool,
    pub generated_json_schema_output_changed: bool,
    pub core_compiler_path_changed: bool,
}

pub fn parser_backed_source_corpus_records(
    route_manifest: &SyntaxProductionRouteManifest,
    route_gate: &SyntaxProductionRouteImplementationGate,
    source_corpus_gate: &SyntaxSourceCorpusEvidenceGate,
) -> Result<Vec<SyntaxSourceCorpusEvidenceRecord>, SyntaxBackendBoundaryError> {
    syntax_route_manifest::checked_syntax_production_route_manifest_constructs(
        route_manifest,
        route_gate,
        source_corpus_gate,
    )?
    .iter()
    .map(parser_backed_source_corpus_record)
    .collect()
}

fn parser_backed_source_corpus_record(
    construct: &SyntaxProductionRouteManifestConstruct,
) -> Result<SyntaxSourceCorpusEvidenceRecord, SyntaxBackendBoundaryError> {
    let (target, grammar, sidecar) = route_input_for_construct(construct)?;
    let artifact = compile_checked_syntax_production_route(SyntaxProductionRouteRequest {
        target,
        grammar: &grammar,
        sidecar: &sidecar,
    })?;
    let source_path = require_option(
        "record.source_path",
        grammar.provenance.source_path.clone(),
        &construct.grammar_id,
    )?;
    let source_sha256 = require_option(
        "record.source_sha256",
        grammar.provenance.source_sha256.clone(),
        &construct.grammar_id,
    )?;
    let metadata_path = require_option(
        "record.metadata_path",
        grammar.provenance.metadata_path.clone(),
        &construct.grammar_id,
    )?;
    let metadata_sha256 = require_option(
        "record.metadata_sha256",
        grammar.provenance.metadata_sha256.clone(),
        &construct.grammar_id,
    )?;
    require_string(
        "record.metadata_path",
        &metadata_path,
        &construct.metadata_path,
    )?;
    require_string(
        "record.projector_metadata_setup.checked_metadata_path",
        &construct.projector_metadata_setup.checked_metadata_path,
        &metadata_path,
    )?;
    if !construct
        .projector_metadata_setup
        .generated_output_path
        .starts_with("target/")
    {
        return Err(SyntaxBackendBoundaryError::new(
            "record.projector_metadata_setup.generated_output_path",
            format!(
                "{} projector metadata output must stay under target/",
                construct.grammar_id
            ),
        ));
    }
    if !construct
        .projector_metadata_setup
        .route_manifest_projector_backed
    {
        return Err(SyntaxBackendBoundaryError::new(
            "record.projector_metadata_setup.route_manifest_projector_backed",
            format!(
                "{} must be route-manifest projector-backed",
                construct.grammar_id
            ),
        ));
    }
    if construct
        .projector_metadata_setup
        .projector_command
        .is_empty()
    {
        return Err(SyntaxBackendBoundaryError::new(
            "record.projector_metadata_setup.projector_command",
            format!("{} missing projector command", construct.grammar_id),
        ));
    }
    let source_file = file_provenance(&source_path).map_err(|error| {
        SyntaxBackendBoundaryError::new(
            "record.source_path",
            format!(
                "failed to read source path for {}: {error}",
                construct.grammar_id
            ),
        )
    })?;
    require_string("record.source_sha256", &source_file.sha256, &source_sha256)?;
    let metadata_file = file_provenance(&metadata_path).map_err(|error| {
        SyntaxBackendBoundaryError::new(
            "record.metadata_path",
            format!(
                "failed to read checked metadata path for {}: {error}",
                construct.grammar_id
            ),
        )
    })?;
    require_string(
        "record.metadata_sha256",
        &metadata_file.sha256,
        &metadata_sha256,
    )?;
    let parser = grammar
        .provenance
        .parser
        .as_ref()
        .ok_or_else(|| {
            SyntaxBackendBoundaryError::new(
                "record.parser",
                format!("missing parser provenance for {}", construct.grammar_id),
            )
        })
        .map(parser_evidence)?;
    let generated_codec = SyntaxSourceCorpusGeneratedCodecEvidence {
        renderer: require_json_string(
            "record.generated_codec.renderer",
            &artifact.metadata["generated_crate_identity"]["renderer"],
        )?,
        rendered_source_sha256: require_json_string(
            "record.generated_codec.rendered_source_sha256",
            &artifact.metadata["rendered_source_sha256"],
        )?,
        generated_codec_metadata_sha256: artifact.metadata_sha256,
        generated_crate_identity_sha256: require_json_string(
            "record.generated_codec.generated_crate_identity_sha256",
            &artifact.metadata["generated_crate_identity"]["sha256"],
        )?,
        generation_options_sha256: require_json_string(
            "record.generated_codec.generation_options_sha256",
            &artifact.metadata["generation_options_sha256"],
        )?,
    };
    let structural_model_sha256 = require_json_string(
        "record.structural_model_sha256",
        &artifact.metadata["structural_model_sha256"],
    )?;
    let token_layout_sha256 = require_json_string(
        "record.token_layout_sha256",
        &artifact.metadata["token_layout_sha256"],
    )?;
    let token_layout = artifact.metadata["token_layout"].clone();
    let token_layout_metadata_sha256 =
        require_json_string("record.token_layout.sha256", &token_layout["sha256"])?;
    require_string(
        "record.token_layout.sha256",
        &token_layout_metadata_sha256,
        &token_layout_sha256,
    )?;
    let bpe_boundary = token_layout["bpe_boundary"].clone();
    let validation_hook_status = json!({
        "validation_hook_contract_sha256": artifact.metadata["validation_hook_contract_sha256"].clone(),
        "validation_hook_sidecar_evidence_sha256": artifact.metadata["validation_hook_sidecar_evidence_sha256"].clone(),
        "source_reparse_or_equivalent_check": artifact.metadata["validation_hook_contract"]["source_reparse_or_equivalent_check"].clone(),
        "renderer_formatter_status": artifact.metadata["validation_hook_contract"]["renderer_formatter_status"].clone(),
        "external_check_policy": artifact.metadata["validation_hook_contract"]["external_check_policy"].clone(),
        "validation_surfaces": artifact.metadata["validation_hook_contract"]["validation_surfaces"].clone(),
    });
    let sidecar_controls = json!({
        "arena_ir_proof_used": false,
        "external_sidecars_promoted": false,
        "validation_hook_sidecar_status": artifact.metadata["validation_hook_sidecar_evidence"]["sidecar_status"].clone(),
        "generated_artifact_replay_compiler_boundary": artifact.metadata["generated_artifact_replay"]["compiler_boundary"].clone(),
        "input_diagnostics_contract_sidecar_status": artifact.metadata["input_diagnostics_contract"]["sidecar_status"].clone(),
        "input_diagnostics_negative_fixture_sidecar_status": artifact.metadata["input_diagnostics_negative_fixture"]["sidecar_status"].clone(),
    });
    let route_metadata = SyntaxSourceCorpusRouteEvidence {
        accepted_input: require_json_string(
            "record.route_metadata.accepted_input",
            &artifact.metadata["production_route"]["accepted_input"],
        )?,
        route_entrypoint: require_json_string(
            "record.route_metadata.route_entrypoint",
            &artifact.metadata["production_route"]["route_entrypoint"],
        )?,
        route_gate: require_json_string(
            "record.route_metadata.route_gate",
            &artifact.metadata["production_route"]["route_gate"],
        )?,
        stable_backend_promoted: require_json_bool(
            "record.route_metadata.stable_backend_promoted",
            &artifact.metadata["production_route"]["stable_backend_promoted"],
        )?,
        generated_json_schema_output_changed: require_json_bool(
            "record.route_metadata.generated_json_schema_output_changed",
            &artifact.metadata["production_route"]["generated_json_schema_output_changed"],
        )?,
        core_compiler_path_changed: require_json_bool(
            "record.route_metadata.core_compiler_path_changed",
            &artifact.metadata["production_route"]["core_compiler_path_changed"],
        )?,
    };
    if route_metadata.generated_json_schema_output_changed
        || route_metadata.core_compiler_path_changed
    {
        return Err(SyntaxBackendBoundaryError::new(
            "record.route_metadata",
            format!(
                "{} claims JSON Schema or core compiler route changes",
                construct.grammar_id
            ),
        ));
    }

    Ok(SyntaxSourceCorpusEvidenceRecord {
        target_id: construct.target_id.clone(),
        grammar_id: construct.grammar_id.clone(),
        completed_promotion_class: construct.completed_promotion_class.clone(),
        source_path,
        source_sha256,
        source_bytes: source_file.bytes,
        metadata_path,
        metadata_sha256,
        metadata_bytes: metadata_file.bytes,
        parser,
        generated_codec,
        structural_model_sha256,
        token_layout_sha256,
        token_layout,
        validation_hook_status,
        sidecar_controls,
        generated_test_markers: construct.generated_test_markers.clone(),
        projector_metadata_setup: construct.projector_metadata_setup.clone(),
        bpe: SyntaxSourceCorpusBpeEvidence {
            bpe_boundary,
            merge_fingerprints: Vec::new(),
            merge_artifact_used: false,
        },
        route_metadata,
    })
}

fn route_input_for_construct(
    construct: &SyntaxProductionRouteManifestConstruct,
) -> Result<
    (
        SyntaxProductionRouteTarget,
        GrammarIr,
        GrammarStructuralModelSidecar,
    ),
    SyntaxBackendBoundaryError,
> {
    match construct.target_id.as_str() {
        "csharp" => {
            let grammar = CSharpSyntaxMetadataAdapter::new(construct.grammar_id.as_str())
                .lower_metadata_file(Path::new(&construct.metadata_path))
                .map_err(|error| {
                    SyntaxBackendBoundaryError::new(
                        "record.metadata_path",
                        format!("failed to lower C# metadata: {error}"),
                    )
                })?;
            let sidecar =
                lower_grammar_ir_to_structural_model(&grammar, csharp_syntax_token_layout())
                    .map_err(|error| {
                        SyntaxBackendBoundaryError::new(
                            "record.structural_model",
                            format!("failed to lower C# StructuralModel: {error}"),
                        )
                    })?;
            Ok((SyntaxProductionRouteTarget::CSharp, grammar, sidecar))
        }
        "go" => {
            let grammar = GoSyntaxMetadataAdapter::new(construct.grammar_id.as_str())
                .lower_metadata_file(Path::new(&construct.metadata_path))
                .map_err(|error| {
                    SyntaxBackendBoundaryError::new(
                        "record.metadata_path",
                        format!("failed to lower Go metadata: {error}"),
                    )
                })?;
            let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout())
                .map_err(|error| {
                    SyntaxBackendBoundaryError::new(
                        "record.structural_model",
                        format!("failed to lower Go StructuralModel: {error}"),
                    )
                })?;
            Ok((SyntaxProductionRouteTarget::Go, grammar, sidecar))
        }
        _ => Err(SyntaxBackendBoundaryError::new(
            "record.target_id",
            format!(
                "unsupported parser-backed corpus target {}",
                construct.target_id
            ),
        )),
    }
}

fn parser_evidence(parser: &GrammarParserProvenance) -> SyntaxSourceCorpusParserEvidence {
    SyntaxSourceCorpusParserEvidence {
        tool: parser.tool.clone(),
        version: parser.version.clone(),
        package: parser.package.clone(),
        package_version: parser.package_version.clone(),
    }
}

fn require_production_route_boundary(
    boundary: &SyntaxBackendCompilerBoundary,
) -> Result<(), SyntaxBackendBoundaryError> {
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

fn require_option(
    field: &'static str,
    value: Option<String>,
    grammar_id: &str,
) -> Result<String, SyntaxBackendBoundaryError> {
    value.ok_or_else(|| {
        SyntaxBackendBoundaryError::new(field, format!("missing value for {grammar_id}"))
    })
}

fn require_json_string(
    field: &'static str,
    value: &Value,
) -> Result<String, SyntaxBackendBoundaryError> {
    value
        .as_str()
        .map(ToString::to_string)
        .ok_or_else(|| SyntaxBackendBoundaryError::new(field, "expected string JSON value"))
}

fn require_json_bool(
    field: &'static str,
    value: &Value,
) -> Result<bool, SyntaxBackendBoundaryError> {
    value
        .as_bool()
        .ok_or_else(|| SyntaxBackendBoundaryError::new(field, "expected boolean JSON value"))
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
            "generated report drifted from the checked source-corpus evidence gate",
        ))
    }
}

fn require_route_equal(
    field: &'static str,
    actual: &SyntaxSourceCorpusProductionRoute,
    expected: &SyntaxSourceCorpusProductionRoute,
) -> Result<(), SyntaxBackendBoundaryError> {
    if actual == expected {
        Ok(())
    } else {
        Err(SyntaxBackendBoundaryError::new(
            field,
            "generated report route metadata drifted from the current production route gate",
        ))
    }
}

fn require_report_policy_equal(
    field: &'static str,
    actual: &SyntaxSourceCorpusReportPolicy,
    expected: &SyntaxSourceCorpusReportPolicy,
) -> Result<(), SyntaxBackendBoundaryError> {
    if actual == expected {
        Ok(())
    } else {
        Err(SyntaxBackendBoundaryError::new(
            field,
            "generated report policy drifted from the checked source-corpus evidence gate",
        ))
    }
}

fn require_csharp_no_gap_metadata_report_equal(
    field: &'static str,
    actual: &SyntaxSourceCorpusCSharpNoGapMetadataReport,
    expected: &SyntaxSourceCorpusCSharpNoGapMetadataReport,
) -> Result<(), SyntaxBackendBoundaryError> {
    if actual == expected {
        Ok(())
    } else {
        Err(SyntaxBackendBoundaryError::new(
            field,
            "generated report no-gap C# metadata contract drifted from the checked source-corpus evidence gate",
        ))
    }
}

fn require_records_equal(
    field: &'static str,
    actual: &[SyntaxSourceCorpusEvidenceRecord],
    expected: &[SyntaxSourceCorpusEvidenceRecord],
) -> Result<(), SyntaxBackendBoundaryError> {
    if actual == expected {
        Ok(())
    } else {
        Err(SyntaxBackendBoundaryError::new(
            field,
            "generated records drifted from the current route table, checked parser metadata, or generated codec metadata",
        ))
    }
}
