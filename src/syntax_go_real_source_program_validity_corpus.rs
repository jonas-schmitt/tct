//! Go real-source source-program validity corpus gate.
//!
//! This gate preserves the current parser-backed Go real-source corpus
//! eligibility boundary. The corpus check is sidecar evidence only: parser,
//! formatter, type-checker, and real-source eligibility reports do not promote
//! source-program validity, paper, model-quality, type correctness, arena IR,
//! external sidecar promotion, generated JSON Schema output, or generated
//! hot-path claims. Whole-file generated-codec validity is reserved for the
//! checked generated-codec corpus sidecar claim.

use crate::syntax_backend_boundary::SyntaxBackendBoundaryError;
use crate::syntax_source_program_handoff::{
    SYNTAX_LEVEL_BOUNDARY, SyntaxSourceProgramCSharpGeneratedCodecCompletionHandoffInput,
    SyntaxSourceProgramGoGeneratedCodecCompletionHandoffInput,
    SyntaxSourceProgramStableMetadataPromotionInput, TYPECHECK_PRESERVATION_REPORT_STABLE_FIELDS,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

// Identity pins for the retired source-program type-checker execution
// preservation gate (`src/syntax_source_program_typecheck_execution_preservation.rs`,
// removed 2026-06). The `source_typecheck_preservation` block in the corpus
// fixture is preserved historical metadata only; no live typecheck gate is
// consumed anymore.
const TYPECHECK_PRESERVATION_ARTIFACT_KIND: &str =
    "tct_syntax_backend_source_program_typecheck_execution_preservation_gate";
const TYPECHECK_PRESERVATION_VERSION: u32 = 1;
const TYPECHECK_PRESERVATION_TOOL: &str =
    "syntax_backend_source_program_typecheck_execution_preservation_gate";
const TYPECHECK_PRESERVATION_MODULE_PATH: &str =
    "src/syntax_source_program_typecheck_execution_preservation.rs";
const TYPECHECK_PRESERVATION_FIXTURE_PATH: &str =
    "tests/fixtures/syntax_backend/source-program-typecheck-execution-preservation-v1.json";
const TYPECHECK_PRESERVATION_STATUS: &str =
    "source_program_typecheck_execution_report_preservation_policy";
const TYPECHECK_PRESERVATION_COMPLETED_PROMOTION_CLASS: &str =
    "syntax backend source-program type-checker execution preservation gate";
const TYPECHECK_PRESERVATION_NEXT_MISSING_STABLE_PROMOTION_CLASS: &str =
    "syntax backend Go real-source source-program validity corpus gate";
const TYPECHECK_PRESERVATION_EXECUTION_REPORT_PATH: &str =
    "target/tct-syntax-source-program-typecheck-execution/execution-report-v1.json";
const TYPECHECK_PRESERVATION_FEASIBILITY_REPORT_PATH: &str =
    "target/tct-syntax-source-program-typecheck-feasibility/feasibility-report-v1.json";
const TYPECHECK_PRESERVATION_EXECUTION_COMPLETED_PROMOTION_CLASS: &str =
    "syntax backend source-program type-checker execution report gate";
const TYPECHECK_PRESERVATION_EXECUTION_NEXT_MISSING_STABLE_PROMOTION_CLASS: &str =
    "syntax backend source-program type-checker execution preservation gate";

pub const TOOL: &str = "syntax_backend_go_real_source_program_validity_corpus_gate";
pub const VERSION: u32 = 1;
pub const ARTIFACT_KIND: &str = "tct_syntax_backend_go_real_source_program_validity_corpus_gate";
pub const FIXTURE_PATH: &str =
    "tests/fixtures/syntax_backend/go-real-source-program-validity-corpus-v1.json";
pub const STATUS: &str = "go_real_source_program_validity_corpus_policy";
pub const COMPLETED_PROMOTION_CLASS: &str =
    "syntax backend Go real-source source-program validity corpus gate";
pub const NEXT_MISSING_STABLE_PROMOTION_CLASS: &str =
    "syntax backend Go real-source generated-codec corpus round-trip gate";
pub const CORPUS_MANIFEST_PATH: &str = "tests/fixtures/go_syntax_corpus/manifest-v1.json";
pub const CORPUS_SOURCE_MANIFEST_PATH: &str = "tests/fixtures/go_syntax_corpus/sources.txt";
pub const CORPUS_ID: &str = "go-stdlib-real-source-syntax-corpus-v1";
pub const CORPUS_ARTIFACT_KIND: &str = "tct_go_syntax_real_source_corpus_manifest_v1";
pub const CORPUS_SOURCE_ROOT: &str = "tests/fixtures/go_syntax_corpus/src";
pub const CORPUS_SOURCE_REVISION: &str = "go1.24.3";
pub const CORPUS_LICENSE: &str = "BSD-3-Clause";
pub const CORPUS_REPORT_ARTIFACT_KIND: &str = "tct_go_syntax_corpus_check_v1";
pub const CORPUS_REPORT_TOOL: &str = "go_syntax_corpus_check";
pub const CORPUS_REPORT_STATUS: &str = "ready";
pub const CORPUS_REPORT_PATH: &str = "target/tct-analysis/go-syntax-corpus-check/report.json";
pub const CORPUS_HELPER_PATH: &str = "scripts/analysis/go_syntax_corpus_check.go";
pub const CORPUS_HELPER_COMMAND: &str = "go run scripts/analysis/go_syntax_corpus_check.go --manifest tests/fixtures/go_syntax_corpus/manifest-v1.json --json-output target/tct-analysis/go-syntax-corpus-check/report.json";
pub const METADATA_PROJECTOR_COMMAND: &str =
    "go run scripts/analysis/go_syntax_metadata_smoke.go --self-test";
pub const NO_STABLE_PROMOTION_CLAIM: &str = "sidecar-only parser-backed corpus evidence; no TctCompiler route and no stable backend promotion";

const POLICY_ID: &str = "syntax-go-real-source-program-validity-corpus-v1";
const ROUTE_SOURCE_MANIFEST_TOOL: &str = "route_source_manifest";
const ROUTE_SOURCE_MANIFEST_TARGET: &str = "go-syntax";
const ROUTE_SOURCE_MANIFEST_MARKER: &str = "tct-go-syntax-route-corpus-v1";
const REQUIRED_ENTRY_COUNT: usize = 7;
const REQUIRED_OBSERVED_CONSTRUCT_COUNT: usize = 43;
const REQUIRED_ELIGIBILITY_STATUS: &str = "parser_backed_source_program_sidecar_eligible";
const REQUIRED_GENERATED_CODEC_STATUS: &str = "not_yet_checked";
const REQUIRED_STABLE_PROMOTION_BLOCKER: &str = "real_source_end_to_end_generated_codec_round_trip";
const REQUIRED_TYPECHECK_PRESERVATION_STABLE_REPORT_FIELDS: &[&str] = &[
    "source_feasibility_report.stable_metadata_promotion",
    "source_feasibility_report.go_generated_codec_completion_handoff",
    "source_feasibility_report.csharp_generated_codec_completion_handoff",
];
const REQUIRED_TYPECHECK_PRESERVATION_BLOCKED_CLAIMS: &[&str] = &[
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

const REQUIRED_TYPECHECK_PRESERVATION_NO_STABLE_CLAIMS: &[&str] = &[
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

const REQUIRED_STABLE_REPORT_FIELDS: &[&str] = &[
    "tool",
    "version",
    "artifact_kind",
    "status",
    "manifest.path",
    "source_manifest.path",
    "corpus_id",
    "source_root",
    "entry_count",
    "metadata_projector.command",
    "metadata_projector.role",
    "supported_route_fixtures",
    "supported_constructs",
    "observed_constructs.construct",
    "unsupported_constructs",
    "next_missing_construct",
    "files.path",
    "files.sha256",
    "files.bytes",
    "files.package",
    "files.constructs",
    "files.unsupported_constructs",
    "no_stable_promotion_claim",
    "stable_promotion_blockers",
];

const REQUIRED_VOLATILE_REPORT_FIELDS: &[&str] = &[
    "manifest.sha256",
    "manifest.bytes",
    "source_manifest.sha256",
    "source_manifest.bytes",
    "observed_constructs.count",
    "observed_constructs.files",
];

const REQUIRED_NO_STABLE_CLAIMS: &[&str] = &[
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

const REQUIRED_VALIDATION_COMMANDS: &[&str] = &[
    CORPUS_HELPER_COMMAND,
    "cargo test --test syntax_backend_boundary_test",
    "python3 scripts/testing/test-quality-audit.py --fail-on-gaps",
    "git diff --check",
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxGoRealSourceProgramValidityCorpusGate {
    pub artifact_kind: String,
    pub version: u32,
    pub tool: String,
    pub status: String,
    pub source_typecheck_preservation: SyntaxGoRealSourceTypecheckPreservationSource,
    pub corpus_manifest: SyntaxGoRealSourceCorpusManifestContract,
    pub corpus_check_report_contract: SyntaxGoRealSourceCorpusCheckReportContract,
    pub sidecar_validity_policy: SyntaxGoRealSourceProgramValidityPolicy,
    pub corpus_eligibility: SyntaxGoRealSourceProgramValidityEligibility,
    pub no_stable_claims: Vec<String>,
    pub validation_commands: Vec<String>,
    pub completed_promotion_class: String,
    pub next_missing_stable_promotion_class: String,
}

impl SyntaxGoRealSourceProgramValidityCorpusGate {
    pub fn validate_against_manifest(
        &self,
        manifest: &GoSyntaxCorpusManifest,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        require_string("artifact_kind", &self.artifact_kind, ARTIFACT_KIND)?;
        require_u32("version", self.version, VERSION)?;
        require_string("tool", &self.tool, TOOL)?;
        require_string("status", &self.status, STATUS)?;
        self.source_typecheck_preservation.validate()?;
        self.corpus_manifest.validate(manifest)?;
        self.corpus_check_report_contract.validate()?;
        self.sidecar_validity_policy.validate()?;
        self.corpus_eligibility.validate(manifest)?;
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
pub struct SyntaxGoRealSourceTypecheckPreservationSource {
    pub artifact_kind: String,
    pub version: u32,
    pub tool: String,
    pub module_path: String,
    pub fixture_path: String,
    pub status: String,
    pub source_typecheck_execution_report_path: String,
    pub source_typecheck_feasibility_report_path: String,
    pub source_typecheck_execution_completed_promotion_class: String,
    pub source_typecheck_execution_next_missing_stable_promotion_class: String,
    pub stable_metadata_promotion: SyntaxSourceProgramStableMetadataPromotionInput,
    pub go_generated_codec_completion_handoff:
        SyntaxSourceProgramGoGeneratedCodecCompletionHandoffInput,
    pub csharp_generated_codec_completion_handoff:
        SyntaxSourceProgramCSharpGeneratedCodecCompletionHandoffInput,
    pub preserved_stable_report_fields: Vec<String>,
    pub preserved_report_contract_stable_report_fields: Vec<String>,
    pub preserved_required_blocked_claims: Vec<String>,
    pub preserved_no_stable_claims: Vec<String>,
    pub execution_pass_envelope_required_record_count: usize,
    pub execution_pass_envelope_required_record_status: String,
    pub execution_pass_envelope_required_records_all_passed: bool,
    pub execution_pass_envelope_required_record_ids: Vec<String>,
    pub execution_evidence_typechecker_results_sidecar_only: bool,
    pub execution_evidence_type_correctness_claim_allowed: bool,
    pub execution_evidence_arena_ir_proof_used: bool,
    pub execution_evidence_json_schema_backend_unchanged: bool,
    pub completed_promotion_class: String,
    pub next_missing_stable_promotion_class: String,
}

impl SyntaxGoRealSourceTypecheckPreservationSource {
    fn validate(&self) -> Result<(), SyntaxBackendBoundaryError> {
        require_string(
            "source_typecheck_preservation.artifact_kind",
            &self.artifact_kind,
            TYPECHECK_PRESERVATION_ARTIFACT_KIND,
        )?;
        require_u32(
            "source_typecheck_preservation.version",
            self.version,
            TYPECHECK_PRESERVATION_VERSION,
        )?;
        require_string(
            "source_typecheck_preservation.tool",
            &self.tool,
            TYPECHECK_PRESERVATION_TOOL,
        )?;
        require_string(
            "source_typecheck_preservation.module_path",
            &self.module_path,
            TYPECHECK_PRESERVATION_MODULE_PATH,
        )?;
        require_string(
            "source_typecheck_preservation.fixture_path",
            &self.fixture_path,
            TYPECHECK_PRESERVATION_FIXTURE_PATH,
        )?;
        require_string(
            "source_typecheck_preservation.status",
            &self.status,
            TYPECHECK_PRESERVATION_STATUS,
        )?;
        require_string(
            "source_typecheck_preservation.source_typecheck_execution_report_path",
            &self.source_typecheck_execution_report_path,
            TYPECHECK_PRESERVATION_EXECUTION_REPORT_PATH,
        )?;
        require_string(
            "source_typecheck_preservation.source_typecheck_feasibility_report_path",
            &self.source_typecheck_feasibility_report_path,
            TYPECHECK_PRESERVATION_FEASIBILITY_REPORT_PATH,
        )?;
        require_string(
            "source_typecheck_preservation.source_typecheck_execution_completed_promotion_class",
            &self.source_typecheck_execution_completed_promotion_class,
            TYPECHECK_PRESERVATION_EXECUTION_COMPLETED_PROMOTION_CLASS,
        )?;
        require_string(
            "source_typecheck_preservation.source_typecheck_execution_next_missing_stable_promotion_class",
            &self.source_typecheck_execution_next_missing_stable_promotion_class,
            TYPECHECK_PRESERVATION_EXECUTION_NEXT_MISSING_STABLE_PROMOTION_CLASS,
        )?;
        require_all(
            "source_typecheck_preservation.stable_metadata_promotion.source_completion_handoff_source_program_preserved_typecheck_report_contract_stable_report_fields",
            &self
                .stable_metadata_promotion
                .source_completion_handoff_source_program_preserved_typecheck_report_contract_stable_report_fields,
            TYPECHECK_PRESERVATION_REPORT_STABLE_FIELDS,
        )?;
        require_all(
            "source_typecheck_preservation.preserved_stable_report_fields",
            &self.preserved_stable_report_fields,
            REQUIRED_TYPECHECK_PRESERVATION_STABLE_REPORT_FIELDS,
        )?;
        require_all(
            "source_typecheck_preservation.preserved_report_contract_stable_report_fields",
            &self.preserved_report_contract_stable_report_fields,
            TYPECHECK_PRESERVATION_REPORT_STABLE_FIELDS,
        )?;
        require_all(
            "source_typecheck_preservation.preserved_required_blocked_claims",
            &self.preserved_required_blocked_claims,
            REQUIRED_TYPECHECK_PRESERVATION_BLOCKED_CLAIMS,
        )?;
        require_all(
            "source_typecheck_preservation.preserved_no_stable_claims",
            &self.preserved_no_stable_claims,
            REQUIRED_TYPECHECK_PRESERVATION_NO_STABLE_CLAIMS,
        )?;
        require_bool(
            "source_typecheck_preservation.execution_evidence_typechecker_results_sidecar_only",
            self.execution_evidence_typechecker_results_sidecar_only,
            true,
        )?;
        require_bool(
            "source_typecheck_preservation.execution_evidence_type_correctness_claim_allowed",
            self.execution_evidence_type_correctness_claim_allowed,
            false,
        )?;
        require_bool(
            "source_typecheck_preservation.execution_evidence_arena_ir_proof_used",
            self.execution_evidence_arena_ir_proof_used,
            false,
        )?;
        require_bool(
            "source_typecheck_preservation.execution_evidence_json_schema_backend_unchanged",
            self.execution_evidence_json_schema_backend_unchanged,
            true,
        )?;
        require_string(
            "source_typecheck_preservation.completed_promotion_class",
            &self.completed_promotion_class,
            TYPECHECK_PRESERVATION_COMPLETED_PROMOTION_CLASS,
        )?;
        require_string(
            "source_typecheck_preservation.next_missing_stable_promotion_class",
            &self.next_missing_stable_promotion_class,
            TYPECHECK_PRESERVATION_NEXT_MISSING_STABLE_PROMOTION_CLASS,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxGoRealSourceCorpusManifestContract {
    pub manifest_path: String,
    pub manifest_artifact_kind: String,
    pub manifest_version: u32,
    pub corpus_id: String,
    pub source_root: String,
    pub source_manifest_path: String,
    pub source_manifest_marker: String,
    pub route_source_manifest_tool: String,
    pub route_source_manifest_target: String,
    pub route_source_manifest_fixture_scope: bool,
    pub source_revision: String,
    pub license: String,
    pub expected_entry_count: usize,
    pub expected_next_missing_construct: String,
    pub no_stable_promotion_claim: String,
    pub source_records: Vec<SyntaxGoRealSourceRecordIdentity>,
}

impl SyntaxGoRealSourceCorpusManifestContract {
    fn validate(
        &self,
        manifest: &GoSyntaxCorpusManifest,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        require_string(
            "corpus_manifest.manifest_path",
            &self.manifest_path,
            CORPUS_MANIFEST_PATH,
        )?;
        require_string(
            "corpus_manifest.manifest_artifact_kind",
            &self.manifest_artifact_kind,
            CORPUS_ARTIFACT_KIND,
        )?;
        require_u32(
            "corpus_manifest.manifest_version",
            self.manifest_version,
            VERSION,
        )?;
        require_string("corpus_manifest.corpus_id", &self.corpus_id, CORPUS_ID)?;
        require_string(
            "corpus_manifest.source_root",
            &self.source_root,
            CORPUS_SOURCE_ROOT,
        )?;
        require_string(
            "corpus_manifest.source_manifest_path",
            &self.source_manifest_path,
            CORPUS_SOURCE_MANIFEST_PATH,
        )?;
        require_string(
            "corpus_manifest.source_manifest_marker",
            &self.source_manifest_marker,
            ROUTE_SOURCE_MANIFEST_MARKER,
        )?;
        require_string(
            "corpus_manifest.route_source_manifest_tool",
            &self.route_source_manifest_tool,
            ROUTE_SOURCE_MANIFEST_TOOL,
        )?;
        require_string(
            "corpus_manifest.route_source_manifest_target",
            &self.route_source_manifest_target,
            ROUTE_SOURCE_MANIFEST_TARGET,
        )?;
        require_bool(
            "corpus_manifest.route_source_manifest_fixture_scope",
            self.route_source_manifest_fixture_scope,
            true,
        )?;
        require_string(
            "corpus_manifest.source_revision",
            &self.source_revision,
            CORPUS_SOURCE_REVISION,
        )?;
        require_string("corpus_manifest.license", &self.license, CORPUS_LICENSE)?;
        require_usize(
            "corpus_manifest.expected_entry_count",
            self.expected_entry_count,
            REQUIRED_ENTRY_COUNT,
        )?;
        require_string(
            "corpus_manifest.expected_next_missing_construct",
            &self.expected_next_missing_construct,
            "",
        )?;
        require_string(
            "corpus_manifest.no_stable_promotion_claim",
            &self.no_stable_promotion_claim,
            NO_STABLE_PROMOTION_CLAIM,
        )?;
        require_string(
            "manifest.artifact_kind",
            &manifest.artifact_kind,
            CORPUS_ARTIFACT_KIND,
        )?;
        require_u32("manifest.version", manifest.version, VERSION)?;
        require_string("manifest.corpus_id", &manifest.corpus_id, CORPUS_ID)?;
        require_string(
            "manifest.source_root",
            &manifest.source_root,
            CORPUS_SOURCE_ROOT,
        )?;
        require_string(
            "manifest.source_manifest",
            &manifest.source_manifest,
            CORPUS_SOURCE_MANIFEST_PATH,
        )?;
        manifest.route_source_manifest_shape.validate()?;
        manifest.selection.validate(self.expected_entry_count)?;
        require_string(
            "manifest.no_stable_promotion_claim",
            &manifest.no_stable_promotion_claim,
            NO_STABLE_PROMOTION_CLAIM,
        )?;
        require_usize(
            "manifest.entries",
            manifest.entries.len(),
            self.expected_entry_count,
        )?;
        require_usize(
            "corpus_manifest.source_records",
            self.source_records.len(),
            manifest.entries.len(),
        )?;
        validate_source_records(&self.source_records, manifest)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxGoRealSourceRecordIdentity {
    pub manifest_path: String,
    pub source_path: String,
    pub sha256: String,
    pub bytes: usize,
    pub license: String,
}

impl SyntaxGoRealSourceRecordIdentity {
    fn validate(&self, entry: &GoSyntaxCorpusEntry) -> Result<(), SyntaxBackendBoundaryError> {
        require_string(
            "corpus_manifest.source_records.manifest_path",
            &self.manifest_path,
            &entry.manifest_path,
        )?;
        require_string(
            "corpus_manifest.source_records.source_path",
            &self.source_path,
            &entry.file.path,
        )?;
        require_string(
            "corpus_manifest.source_records.sha256",
            &self.sha256,
            &entry.file.sha256,
        )?;
        require_usize(
            "corpus_manifest.source_records.bytes",
            self.bytes,
            entry.file.bytes,
        )?;
        require_string(
            "corpus_manifest.source_records.license",
            &self.license,
            CORPUS_LICENSE,
        )?;
        require_string("manifest.entries.license", &entry.license, CORPUS_LICENSE)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxGoRealSourceCorpusCheckReportContract {
    pub helper_path: String,
    pub report_artifact_kind: String,
    pub report_tool: String,
    pub report_version: u32,
    pub report_status_when_all_constructs_supported: String,
    pub report_path: String,
    pub report_command: String,
    pub metadata_projector_command: String,
    pub report_generated_under_target: bool,
    pub report_checked_in: bool,
    pub stable_report_fields: Vec<String>,
    pub volatile_report_fields: Vec<String>,
    pub stable_promotion_blockers: Vec<String>,
}

impl SyntaxGoRealSourceCorpusCheckReportContract {
    fn validate(&self) -> Result<(), SyntaxBackendBoundaryError> {
        require_string(
            "corpus_check_report_contract.helper_path",
            &self.helper_path,
            CORPUS_HELPER_PATH,
        )?;
        require_string(
            "corpus_check_report_contract.report_artifact_kind",
            &self.report_artifact_kind,
            CORPUS_REPORT_ARTIFACT_KIND,
        )?;
        require_string(
            "corpus_check_report_contract.report_tool",
            &self.report_tool,
            CORPUS_REPORT_TOOL,
        )?;
        require_u32(
            "corpus_check_report_contract.report_version",
            self.report_version,
            VERSION,
        )?;
        require_string(
            "corpus_check_report_contract.report_status_when_all_constructs_supported",
            &self.report_status_when_all_constructs_supported,
            CORPUS_REPORT_STATUS,
        )?;
        require_string(
            "corpus_check_report_contract.report_path",
            &self.report_path,
            CORPUS_REPORT_PATH,
        )?;
        require_string(
            "corpus_check_report_contract.report_command",
            &self.report_command,
            CORPUS_HELPER_COMMAND,
        )?;
        require_string(
            "corpus_check_report_contract.metadata_projector_command",
            &self.metadata_projector_command,
            METADATA_PROJECTOR_COMMAND,
        )?;
        require_bool(
            "corpus_check_report_contract.report_generated_under_target",
            self.report_generated_under_target,
            true,
        )?;
        require_bool(
            "corpus_check_report_contract.report_checked_in",
            self.report_checked_in,
            false,
        )?;
        if !self.report_path.starts_with("target/") {
            return Err(SyntaxBackendBoundaryError::new(
                "corpus_check_report_contract.report_path",
                "Go corpus check reports must stay under target/",
            ));
        }
        require_all(
            "corpus_check_report_contract.stable_report_fields",
            &self.stable_report_fields,
            REQUIRED_STABLE_REPORT_FIELDS,
        )?;
        require_all(
            "corpus_check_report_contract.volatile_report_fields",
            &self.volatile_report_fields,
            REQUIRED_VOLATILE_REPORT_FIELDS,
        )?;
        require_all(
            "corpus_check_report_contract.stable_promotion_blockers",
            &self.stable_promotion_blockers,
            &[REQUIRED_STABLE_PROMOTION_BLOCKER],
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxGoRealSourceProgramValidityPolicy {
    pub policy_id: String,
    pub syntax_level_boundary: String,
    pub corpus_evidence_sidecar_only: bool,
    pub parser_and_formatter_controls_sidecar_only: bool,
    pub typechecker_controls_sidecar_only: bool,
    pub real_source_corpus_eligibility_only: bool,
    pub generated_codec_corpus_round_trip_required_before_claim: bool,
    pub source_program_validity_claim_allowed: bool,
    pub whole_language_claim_allowed: bool,
    pub paper_claim_allowed: bool,
    pub model_quality_claim_allowed: bool,
    pub functional_correctness_claim_allowed: bool,
    pub type_correctness_claim_allowed: bool,
    pub completion_or_infill_claim_allowed: bool,
    pub arena_ir_proof_used: bool,
    pub generated_hot_path_claim_allowed: bool,
    pub json_schema_backend_unchanged: bool,
}

impl SyntaxGoRealSourceProgramValidityPolicy {
    fn validate(&self) -> Result<(), SyntaxBackendBoundaryError> {
        require_string(
            "sidecar_validity_policy.policy_id",
            &self.policy_id,
            POLICY_ID,
        )?;
        require_string(
            "sidecar_validity_policy.syntax_level_boundary",
            &self.syntax_level_boundary,
            SYNTAX_LEVEL_BOUNDARY,
        )?;
        for (field, actual, expected) in [
            (
                "sidecar_validity_policy.corpus_evidence_sidecar_only",
                self.corpus_evidence_sidecar_only,
                true,
            ),
            (
                "sidecar_validity_policy.parser_and_formatter_controls_sidecar_only",
                self.parser_and_formatter_controls_sidecar_only,
                true,
            ),
            (
                "sidecar_validity_policy.typechecker_controls_sidecar_only",
                self.typechecker_controls_sidecar_only,
                true,
            ),
            (
                "sidecar_validity_policy.real_source_corpus_eligibility_only",
                self.real_source_corpus_eligibility_only,
                true,
            ),
            (
                "sidecar_validity_policy.generated_codec_corpus_round_trip_required_before_claim",
                self.generated_codec_corpus_round_trip_required_before_claim,
                true,
            ),
            (
                "sidecar_validity_policy.source_program_validity_claim_allowed",
                self.source_program_validity_claim_allowed,
                false,
            ),
            (
                "sidecar_validity_policy.whole_language_claim_allowed",
                self.whole_language_claim_allowed,
                false,
            ),
            (
                "sidecar_validity_policy.paper_claim_allowed",
                self.paper_claim_allowed,
                false,
            ),
            (
                "sidecar_validity_policy.model_quality_claim_allowed",
                self.model_quality_claim_allowed,
                false,
            ),
            (
                "sidecar_validity_policy.functional_correctness_claim_allowed",
                self.functional_correctness_claim_allowed,
                false,
            ),
            (
                "sidecar_validity_policy.type_correctness_claim_allowed",
                self.type_correctness_claim_allowed,
                false,
            ),
            (
                "sidecar_validity_policy.completion_or_infill_claim_allowed",
                self.completion_or_infill_claim_allowed,
                false,
            ),
            (
                "sidecar_validity_policy.arena_ir_proof_used",
                self.arena_ir_proof_used,
                false,
            ),
            (
                "sidecar_validity_policy.generated_hot_path_claim_allowed",
                self.generated_hot_path_claim_allowed,
                false,
            ),
            (
                "sidecar_validity_policy.json_schema_backend_unchanged",
                self.json_schema_backend_unchanged,
                true,
            ),
        ] {
            require_bool(field, actual, expected)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxGoRealSourceProgramValidityEligibility {
    pub corpus_id: String,
    pub eligible_record_count: usize,
    pub unsupported_record_count: usize,
    pub required_status: String,
    pub observed_construct_count: usize,
    pub supported_construct_count: usize,
    pub expected_next_missing_construct: String,
    pub eligible_records: Vec<SyntaxGoRealSourceProgramValidityRecord>,
    pub unsupported_records: Vec<SyntaxGoRealSourceProgramUnsupportedRecord>,
}

impl SyntaxGoRealSourceProgramValidityEligibility {
    fn validate(
        &self,
        manifest: &GoSyntaxCorpusManifest,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        require_string("corpus_eligibility.corpus_id", &self.corpus_id, CORPUS_ID)?;
        require_usize(
            "corpus_eligibility.eligible_record_count",
            self.eligible_record_count,
            REQUIRED_ENTRY_COUNT,
        )?;
        require_usize(
            "corpus_eligibility.eligible_records",
            self.eligible_records.len(),
            self.eligible_record_count,
        )?;
        require_usize(
            "corpus_eligibility.unsupported_record_count",
            self.unsupported_record_count,
            0,
        )?;
        require_usize(
            "corpus_eligibility.unsupported_records",
            self.unsupported_records.len(),
            self.unsupported_record_count,
        )?;
        require_string(
            "corpus_eligibility.required_status",
            &self.required_status,
            REQUIRED_ELIGIBILITY_STATUS,
        )?;
        require_usize(
            "corpus_eligibility.observed_construct_count",
            self.observed_construct_count,
            REQUIRED_OBSERVED_CONSTRUCT_COUNT,
        )?;
        require_usize(
            "corpus_eligibility.supported_construct_count",
            self.supported_construct_count,
            REQUIRED_OBSERVED_CONSTRUCT_COUNT,
        )?;
        require_string(
            "corpus_eligibility.expected_next_missing_construct",
            &self.expected_next_missing_construct,
            "",
        )?;
        let entries_by_path = manifest.entries_by_manifest_path();
        let mut seen = BTreeSet::new();
        for record in &self.eligible_records {
            let entry = entries_by_path
                .get(record.manifest_path.as_str())
                .ok_or_else(|| {
                    SyntaxBackendBoundaryError::new(
                        "corpus_eligibility.eligible_records",
                        format!("missing manifest entry {}", record.manifest_path),
                    )
                })?;
            record.validate(entry)?;
            seen.insert(record.manifest_path.as_str());
        }
        for entry in &manifest.entries {
            if !seen.contains(entry.manifest_path.as_str()) {
                return Err(SyntaxBackendBoundaryError::new(
                    "corpus_eligibility.eligible_records",
                    format!("missing eligible record {}", entry.manifest_path),
                ));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxGoRealSourceProgramValidityRecord {
    pub manifest_path: String,
    pub source_path: String,
    pub package: String,
    pub sha256: String,
    pub bytes: usize,
    pub status: String,
    pub unsupported_constructs: Vec<String>,
    pub source_program_validity_claim_allowed: bool,
    pub generated_codec_corpus_round_trip_status: String,
    pub stable_promotion_allowed: bool,
}

impl SyntaxGoRealSourceProgramValidityRecord {
    fn validate(&self, entry: &GoSyntaxCorpusEntry) -> Result<(), SyntaxBackendBoundaryError> {
        require_string(
            "corpus_eligibility.record.manifest_path",
            &self.manifest_path,
            &entry.manifest_path,
        )?;
        require_string(
            "corpus_eligibility.record.source_path",
            &self.source_path,
            &entry.file.path,
        )?;
        if self.package.is_empty() {
            return Err(SyntaxBackendBoundaryError::new(
                "corpus_eligibility.record.package",
                "Go corpus eligibility records must preserve the parsed package name",
            ));
        }
        require_string(
            "corpus_eligibility.record.sha256",
            &self.sha256,
            &entry.file.sha256,
        )?;
        require_usize(
            "corpus_eligibility.record.bytes",
            self.bytes,
            entry.file.bytes,
        )?;
        require_string(
            "corpus_eligibility.record.status",
            &self.status,
            REQUIRED_ELIGIBILITY_STATUS,
        )?;
        if !self.unsupported_constructs.is_empty() {
            return Err(SyntaxBackendBoundaryError::new(
                "corpus_eligibility.record.unsupported_constructs",
                "current Go real-source corpus records must not carry unsupported constructs",
            ));
        }
        require_bool(
            "corpus_eligibility.record.source_program_validity_claim_allowed",
            self.source_program_validity_claim_allowed,
            false,
        )?;
        require_string(
            "corpus_eligibility.record.generated_codec_corpus_round_trip_status",
            &self.generated_codec_corpus_round_trip_status,
            REQUIRED_GENERATED_CODEC_STATUS,
        )?;
        require_bool(
            "corpus_eligibility.record.stable_promotion_allowed",
            self.stable_promotion_allowed,
            false,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxGoRealSourceProgramUnsupportedRecord {
    pub manifest_path: String,
    pub unsupported_constructs: Vec<String>,
    pub blocker: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoSyntaxCorpusManifest {
    pub artifact_kind: String,
    pub version: u32,
    pub corpus_id: String,
    pub source_root: String,
    pub source_manifest: String,
    pub route_source_manifest_shape: GoRouteSourceManifestShape,
    pub selection: GoSyntaxCorpusSelection,
    pub entries: Vec<GoSyntaxCorpusEntry>,
    pub no_stable_promotion_claim: String,
}

impl GoSyntaxCorpusManifest {
    fn entries_by_manifest_path(&self) -> BTreeMap<&str, &GoSyntaxCorpusEntry> {
        self.entries
            .iter()
            .map(|entry| (entry.manifest_path.as_str(), entry))
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoRouteSourceManifestShape {
    pub tool: String,
    pub version: u32,
    pub target: String,
    pub marker: String,
    pub route_decision_marker: String,
    pub corpus_fingerprint_sha256: String,
    pub source_list_sha256: String,
    pub minimum_entry_count: usize,
    pub fixture_scope: bool,
}

impl GoRouteSourceManifestShape {
    fn validate(&self) -> Result<(), SyntaxBackendBoundaryError> {
        require_string(
            "manifest.route_source_manifest_shape.tool",
            &self.tool,
            ROUTE_SOURCE_MANIFEST_TOOL,
        )?;
        require_u32(
            "manifest.route_source_manifest_shape.version",
            self.version,
            VERSION,
        )?;
        require_string(
            "manifest.route_source_manifest_shape.target",
            &self.target,
            ROUTE_SOURCE_MANIFEST_TARGET,
        )?;
        require_string(
            "manifest.route_source_manifest_shape.marker",
            &self.marker,
            ROUTE_SOURCE_MANIFEST_MARKER,
        )?;
        require_string(
            "manifest.route_source_manifest_shape.route_decision_marker",
            &self.route_decision_marker,
            ROUTE_SOURCE_MANIFEST_MARKER,
        )?;
        require_usize(
            "manifest.route_source_manifest_shape.minimum_entry_count",
            self.minimum_entry_count,
            REQUIRED_ENTRY_COUNT,
        )?;
        require_bool(
            "manifest.route_source_manifest_shape.fixture_scope",
            self.fixture_scope,
            true,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoSyntaxCorpusSelection {
    pub source: String,
    pub source_revision: String,
    pub license: String,
    pub license_text_path: String,
    pub selection_note: String,
    pub expected_next_missing_construct: String,
    pub expected_entry_count: usize,
    pub representative_status: String,
}

impl GoSyntaxCorpusSelection {
    fn validate(&self, expected_entry_count: usize) -> Result<(), SyntaxBackendBoundaryError> {
        require_string(
            "manifest.selection.source_revision",
            &self.source_revision,
            CORPUS_SOURCE_REVISION,
        )?;
        require_string("manifest.selection.license", &self.license, CORPUS_LICENSE)?;
        require_string(
            "manifest.selection.license_text_path",
            &self.license_text_path,
            "tests/fixtures/go_syntax_corpus/LICENSE.go-BSD-3-Clause.txt",
        )?;
        require_string(
            "manifest.selection.expected_next_missing_construct",
            &self.expected_next_missing_construct,
            "",
        )?;
        require_usize(
            "manifest.selection.expected_entry_count",
            self.expected_entry_count,
            expected_entry_count,
        )?;
        require_string(
            "manifest.selection.representative_status",
            &self.representative_status,
            "real_source_stdlib_subset",
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoSyntaxCorpusEntry {
    pub manifest_path: String,
    pub file: GoSyntaxCorpusFile,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_span: Option<GoSyntaxCorpusSourceSpan>,
    pub license: String,
    pub source_uri: String,
    pub selection_note: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoSyntaxCorpusFile {
    pub path: String,
    pub sha256: String,
    pub bytes: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoSyntaxCorpusSourceSpan {
    pub selection_report: String,
    pub span_id: String,
    pub line_start: usize,
    pub line_end: usize,
    pub byte_start: usize,
    pub byte_end: usize,
    pub bytes: usize,
    pub sha256: String,
}

fn validate_source_records(
    source_records: &[SyntaxGoRealSourceRecordIdentity],
    manifest: &GoSyntaxCorpusManifest,
) -> Result<(), SyntaxBackendBoundaryError> {
    let entries_by_path = manifest.entries_by_manifest_path();
    let mut seen = BTreeSet::new();
    for record in source_records {
        let entry = entries_by_path
            .get(record.manifest_path.as_str())
            .ok_or_else(|| {
                SyntaxBackendBoundaryError::new(
                    "corpus_manifest.source_records",
                    format!("missing manifest entry {}", record.manifest_path),
                )
            })?;
        record.validate(entry)?;
        seen.insert(record.manifest_path.as_str());
    }
    for entry in &manifest.entries {
        if !seen.contains(entry.manifest_path.as_str()) {
            return Err(SyntaxBackendBoundaryError::new(
                "corpus_manifest.source_records",
                format!("missing source record {}", entry.manifest_path),
            ));
        }
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
    for expected in required {
        if !actual.iter().any(|candidate| candidate == expected) {
            return Err(SyntaxBackendBoundaryError::new(
                field,
                format!("missing {expected}"),
            ));
        }
    }
    Ok(())
}
