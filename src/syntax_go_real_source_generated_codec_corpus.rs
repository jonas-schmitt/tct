//! Go real-source generated-codec corpus round-trip gate.
//!
//! This gate preserves the checked real-source Go corpus records that round
//! trip through generated syntax codecs and consumes the source-program corpus
//! gate's preserved type-checker handoff inputs. It is syntax-level evidence
//! for the supported corpus subset, while whole-file source validity and stable
//! backend promotion remain outside this sidecar-only boundary.

use crate::syntax_backend_boundary::SyntaxBackendBoundaryError;
use crate::syntax_go_real_source_program_validity_corpus::{
    ARTIFACT_KIND as SOURCE_PROGRAM_CORPUS_ARTIFACT_KIND,
    COMPLETED_PROMOTION_CLASS as SOURCE_PROGRAM_CORPUS_COMPLETED_PROMOTION_CLASS, CORPUS_ID,
    CORPUS_MANIFEST_PATH, CORPUS_REPORT_PATH, FIXTURE_PATH as SOURCE_PROGRAM_CORPUS_FIXTURE_PATH,
    GoSyntaxCorpusEntry, GoSyntaxCorpusManifest, STATUS as SOURCE_PROGRAM_CORPUS_STATUS,
    SyntaxGoRealSourceProgramValidityCorpusGate, TOOL as SOURCE_PROGRAM_CORPUS_TOOL,
    VERSION as SOURCE_PROGRAM_CORPUS_VERSION,
};
use crate::syntax_source_program_handoff::{
    SYNTAX_LEVEL_BOUNDARY, SyntaxSourceProgramCSharpGeneratedCodecCompletionHandoffInput,
    SyntaxSourceProgramGoGeneratedCodecCompletionHandoffInput,
    SyntaxSourceProgramStableMetadataPromotionInput, TYPECHECK_PRESERVATION_REPORT_STABLE_FIELDS,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const TOOL: &str = "syntax_backend_go_real_source_generated_codec_corpus_gate";
pub const VERSION: u32 = 1;
pub const ARTIFACT_KIND: &str = "tct_syntax_backend_go_real_source_generated_codec_corpus_gate";
pub const FIXTURE_PATH: &str =
    "tests/fixtures/syntax_backend/go-real-source-generated-codec-corpus-v1.json";
pub const STATUS: &str = "go_real_source_generated_codec_corpus_policy";
pub const COMPLETED_PROMOTION_CLASS: &str =
    "syntax backend Go real-source generated-codec corpus round-trip gate";
pub const NEXT_MISSING_STABLE_PROMOTION_CLASS: &str =
    "syntax backend Go real-source generated-codec corpus completion handoff gate";

pub const GENERATED_CODEC_ROUTE_ID: &str = "go_strings_compare_source_file_syntax_v0";
pub const GENERATED_CODEC_MARKER: &str = "generated_go_strings_compare_source_file_round_trips";
pub const CRYPTO_SYSRAND_RAND_WINDOWS_GENERATED_CODEC_ROUTE_ID: &str =
    "go_crypto_sysrand_rand_windows_source_file_syntax_v0";
pub const CRYPTO_SYSRAND_RAND_WINDOWS_GENERATED_CODEC_MARKER: &str =
    "generated_go_crypto_sysrand_rand_windows_source_file_round_trips";
pub const STRINGS_CLONE_GENERATED_CODEC_ROUTE_ID: &str = "go_strings_clone_source_file_syntax_v0";
pub const STRINGS_CLONE_GENERATED_CODEC_MARKER: &str =
    "generated_go_strings_clone_source_file_round_trips";
pub const SORT_INTERFACE_GENERATED_CODEC_ROUTE_ID: &str = "go_sort_interface_source_file_syntax_v0";
pub const SORT_INTERFACE_GENERATED_CODEC_MARKER: &str =
    "generated_go_sort_interface_source_file_round_trips";
pub const SORT_SEARCH_GENERATED_CODEC_ROUTE_ID: &str = "go_sort_search_source_file_syntax_v0";
pub const SORT_SEARCH_GENERATED_CODEC_MARKER: &str =
    "generated_go_sort_search_source_file_round_trips";
pub const STRINGS_SEARCH_GENERATED_CODEC_ROUTE_ID: &str = "go_strings_search_source_file_syntax_v0";
pub const STRINGS_SEARCH_GENERATED_CODEC_MARKER: &str =
    "generated_go_strings_search_source_file_round_trips";
pub const PATH_MATCH_GENERATED_CODEC_ROUTE_ID: &str = "go_path_match_source_file_syntax_v0";
pub const PATH_MATCH_GENERATED_CODEC_MARKER: &str =
    "generated_go_path_match_source_file_round_trips";
pub const GENERATED_CODEC_REPORT_ARTIFACT_KIND: &str = "tct_go_real_source_generated_codec_demo_v1";
pub const GENERATED_CODEC_REPORT_PATH: &str =
    "target/tct-syntax-real-source-demo/go-strings-compare-generated-codec-demo.json";
pub const GENERATED_CODEC_REPORT_COMMAND: &str = "cargo test --test syntax_generated_codec_gate_test go_strings_compare_source_file_syntax_generated_codec_compiles_at_sidecar_boundary -- --exact";
pub const CRYPTO_SYSRAND_RAND_WINDOWS_GENERATED_CODEC_REPORT_PATH: &str =
    "target/tct-syntax-real-source-demo/go-crypto-sysrand-rand-windows-generated-codec-demo.json";
pub const CRYPTO_SYSRAND_RAND_WINDOWS_GENERATED_CODEC_REPORT_COMMAND: &str = "cargo test --test syntax_generated_codec_gate_test go_crypto_sysrand_rand_windows_source_file_syntax_generated_codec_compiles_at_sidecar_boundary -- --exact";
pub const STRINGS_CLONE_GENERATED_CODEC_REPORT_PATH: &str =
    "target/tct-syntax-real-source-demo/go-strings-clone-generated-codec-demo.json";
pub const STRINGS_CLONE_GENERATED_CODEC_REPORT_COMMAND: &str = "cargo test --test syntax_generated_codec_gate_test go_strings_clone_source_file_syntax_generated_codec_compiles_at_sidecar_boundary -- --exact";
pub const SORT_INTERFACE_GENERATED_CODEC_REPORT_PATH: &str =
    "target/tct-syntax-real-source-demo/go-sort-interface-generated-codec-demo.json";
pub const SORT_INTERFACE_GENERATED_CODEC_REPORT_COMMAND: &str = "cargo test --test syntax_generated_codec_gate_test go_sort_interface_source_file_syntax_generated_codec_compiles_at_sidecar_boundary -- --exact";
pub const SORT_SEARCH_GENERATED_CODEC_REPORT_PATH: &str =
    "target/tct-syntax-real-source-demo/go-sort-search-generated-codec-demo.json";
pub const SORT_SEARCH_GENERATED_CODEC_REPORT_COMMAND: &str = "cargo test --test syntax_generated_codec_gate_test go_sort_search_source_file_syntax_generated_codec_compiles_at_sidecar_boundary -- --exact";
pub const STRINGS_SEARCH_GENERATED_CODEC_REPORT_PATH: &str =
    "target/tct-syntax-real-source-demo/go-strings-search-generated-codec-demo.json";
pub const STRINGS_SEARCH_GENERATED_CODEC_REPORT_COMMAND: &str = "cargo test --test syntax_generated_codec_gate_test go_strings_search_source_file_syntax_generated_codec_compiles_at_sidecar_boundary -- --exact";
pub const PATH_MATCH_GENERATED_CODEC_REPORT_PATH: &str =
    "target/tct-syntax-real-source-demo/go-path-match-generated-codec-demo.json";
pub const PATH_MATCH_GENERATED_CODEC_REPORT_COMMAND: &str = "cargo test --test syntax_generated_codec_gate_test go_path_match_source_file_syntax_generated_codec_compiles_at_sidecar_boundary -- --exact";
pub const SUPPORTED_SOURCE_PATH: &str = "tests/fixtures/go_syntax_corpus/src/strings/compare.go";
pub const SUPPORTED_MANIFEST_PATH: &str = "strings/compare.go";
pub const SUPPORTED_SOURCE_SHA256: &str =
    "af4e0fa7dfe63018d9808f89a4c511eeb8d773bd6f3b030f1c9592a1e0de7a85";
pub const SUPPORTED_FUNCTION: &str = "Compare";
pub const CRYPTO_SYSRAND_RAND_WINDOWS_SOURCE_PATH: &str =
    "tests/fixtures/go_syntax_corpus/src/crypto/internal/sysrand/rand_windows.go";
pub const CRYPTO_SYSRAND_RAND_WINDOWS_MANIFEST_PATH: &str =
    "crypto/internal/sysrand/rand_windows.go";
pub const CRYPTO_SYSRAND_RAND_WINDOWS_SOURCE_SHA256: &str =
    "8122c2f83776b2e76f45631835d8038853886e5db728e24fe97c60179197ed8b";
pub const CRYPTO_SYSRAND_RAND_WINDOWS_SOURCE_FUNCTION: &str = "read";
pub const STRINGS_CLONE_SOURCE_PATH: &str = "tests/fixtures/go_syntax_corpus/src/strings/clone.go";
pub const STRINGS_CLONE_MANIFEST_PATH: &str = "strings/clone.go";
pub const STRINGS_CLONE_SOURCE_SHA256: &str =
    "5c18eb3b45b7d9ae63d0cc43f9e337c077de05dd47fd5a1ea49dafeb7e1b862b";
pub const STRINGS_CLONE_SOURCE_FUNCTION: &str = "Clone";
pub const SORT_INTERFACE_SOURCE_PATH: &str =
    "tests/fixtures/go_syntax_corpus/src/sort/example_interface_test.go";
pub const SORT_INTERFACE_MANIFEST_PATH: &str = "sort/example_interface_test.go";
pub const SORT_INTERFACE_SOURCE_SHA256: &str =
    "74de942ab67e4d7725f9e06c2389e8face1118e80865536ae709a0fc3367d5f1";
pub const SORT_INTERFACE_SOURCE_FUNCTION: &str = "Example";
pub const SORT_SEARCH_SOURCE_PATH: &str =
    "tests/fixtures/go_syntax_corpus/src/sort/example_search_test.go";
pub const SORT_SEARCH_MANIFEST_PATH: &str = "sort/example_search_test.go";
pub const SORT_SEARCH_SOURCE_SHA256: &str =
    "c65e8f2f0dab440d8f27a42c27199279f37256d4ef276863a7fd556529f4274d";
pub const SORT_SEARCH_SOURCE_FUNCTION: &str = "ExampleSearch";
pub const STRINGS_SEARCH_SOURCE_PATH: &str =
    "tests/fixtures/go_syntax_corpus/src/strings/search.go";
pub const STRINGS_SEARCH_MANIFEST_PATH: &str = "strings/search.go";
pub const STRINGS_SEARCH_SOURCE_SHA256: &str =
    "4fdef1f79f6e854789d9140b17ce2c2bbdbf0d4067a801e42607cf7877698cd1";
pub const STRINGS_SEARCH_SOURCE_FUNCTION: &str = "makeStringFinder";
pub const PATH_MATCH_SOURCE_PATH: &str = "tests/fixtures/go_syntax_corpus/src/path/match.go";
pub const PATH_MATCH_MANIFEST_PATH: &str = "path/match.go";
pub const PATH_MATCH_SOURCE_SHA256: &str =
    "bb88f659463a04e290492cff8d19c4d7d5c7d551db9b90b3049d3e51188d5ec8";
pub const PATH_MATCH_SOURCE_FUNCTION: &str = "Match";
pub const GENERATED_CODEC_ROUND_TRIP_STATUS: &str = "generated_codec_corpus_round_trip_passed";
pub const BLOCKED_RECORD_STATUS: &str = "generated_codec_corpus_round_trip_blocked";
pub const GENERATED_CODEC_ROUTE_BLOCKER: &str = "unsupported_go_real_source_generated_codec_route";
pub const WHOLE_FILE_GENERATED_CODEC_CLAIM: &str =
    "whole_file_generated_codec_claim_for_checked_go_real_source_corpus";
pub const WHOLE_FILE_GENERATED_CODEC_CLAIM_SCOPE: &str =
    "checked_go_real_source_generated_codec_corpus_sidecar_only";

const REQUIRED_RECORD_COUNT: usize = 7;
const REQUIRED_SUPPORTED_RECORD_COUNT: usize = 7;
const REQUIRED_BLOCKED_RECORD_COUNT: usize = 0;
const REQUIRED_STRINGS_COMPARE_TCT_TOKEN_COUNT: usize = 587;
const REQUIRED_STRINGS_COMPARE_BASELINE_BYTE_TOKEN_COUNT: usize = 630;
const REQUIRED_CRYPTO_SYSRAND_RAND_WINDOWS_TCT_TOKEN_COUNT: usize = 241;
const REQUIRED_CRYPTO_SYSRAND_RAND_WINDOWS_BASELINE_BYTE_TOKEN_COUNT: usize = 273;
const REQUIRED_STRINGS_CLONE_TCT_TOKEN_COUNT: usize = 752;
const REQUIRED_STRINGS_CLONE_BASELINE_BYTE_TOKEN_COUNT: usize = 809;
const REQUIRED_SORT_INTERFACE_TCT_TOKEN_COUNT: usize = 898;
const REQUIRED_SORT_INTERFACE_BASELINE_BYTE_TOKEN_COUNT: usize = 1501;
const REQUIRED_SORT_SEARCH_TCT_TOKEN_COUNT: usize = 253;
const REQUIRED_SORT_SEARCH_BASELINE_BYTE_TOKEN_COUNT: usize = 3294;
const REQUIRED_STRINGS_SEARCH_TCT_TOKEN_COUNT: usize = 2753;
const REQUIRED_STRINGS_SEARCH_BASELINE_BYTE_TOKEN_COUNT: usize = 4227;
const REQUIRED_PATH_MATCH_TCT_TOKEN_COUNT: usize = 1484;
const REQUIRED_PATH_MATCH_BASELINE_BYTE_TOKEN_COUNT: usize = 5465;

const REQUIRED_SOURCE_PROGRAM_CORPUS_TYPECHECK_STABLE_REPORT_FIELDS: &[&str] = &[
    "source_feasibility_report.stable_metadata_promotion",
    "source_feasibility_report.go_generated_codec_completion_handoff",
    "source_feasibility_report.csharp_generated_codec_completion_handoff",
];

const REQUIRED_SOURCE_PROGRAM_CORPUS_TYPECHECK_BLOCKED_CLAIMS: &[&str] = &[
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

const REQUIRED_SOURCE_PROGRAM_CORPUS_NO_STABLE_CLAIMS: &[&str] = &[
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

const REQUIRED_SOURCE_PROGRAM_CORPUS_TYPECHECK_NO_STABLE_CLAIMS: &[&str] = &[
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
    "artifact_kind",
    "version",
    "status",
    "target",
    "route",
    "source_path",
    "source_sha256",
    "source_function",
    "corpus_manifest",
    "generated_codec",
    "parser_validation.gofmt",
    "parser_validation.go_parser",
    "compiler_boundary",
    "no_stable_promotion_claim",
    "next_missing_stable_promotion_class",
];

const REQUIRED_VOLATILE_REPORT_FIELDS: &[&str] = &[
    "tct_token_count",
    "tct_tokens",
    "baseline_byte_token_count",
    "baseline_tokenizer",
];

const REQUIRED_ALLOWED_CLAIMS: &[&str] = &[
    "generated_codec_validity_for_checked_go_real_source_corpus_records",
    WHOLE_FILE_GENERATED_CODEC_CLAIM,
    "parser_backed_source_corpus_identity_preserved",
    "stable_metadata_promotion_prerequisite_identified",
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
    GENERATED_CODEC_REPORT_COMMAND,
    CRYPTO_SYSRAND_RAND_WINDOWS_GENERATED_CODEC_REPORT_COMMAND,
    STRINGS_CLONE_GENERATED_CODEC_REPORT_COMMAND,
    SORT_INTERFACE_GENERATED_CODEC_REPORT_COMMAND,
    SORT_SEARCH_GENERATED_CODEC_REPORT_COMMAND,
    STRINGS_SEARCH_GENERATED_CODEC_REPORT_COMMAND,
    PATH_MATCH_GENERATED_CODEC_REPORT_COMMAND,
    "go run scripts/analysis/go_syntax_corpus_check.go --manifest tests/fixtures/go_syntax_corpus/manifest-v1.json --json-output target/tct-analysis/go-syntax-corpus-check/report.json",
    "cargo test --test syntax_backend_boundary_test",
    "python3 scripts/testing/test-quality-audit.py --fail-on-gaps",
    "git diff --check",
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxGoRealSourceGeneratedCodecCorpusGate {
    pub artifact_kind: String,
    pub version: u32,
    pub tool: String,
    pub status: String,
    pub source_program_corpus_gate: SourceProgramCorpusGateRef,
    pub generated_codec_report_contract: GeneratedCodecCorpusReportContract,
    pub corpus_round_trip: GeneratedCodecCorpusRoundTripEvidence,
    pub whole_file_generated_codec_claim_allowed: bool,
    pub whole_file_generated_codec_claim_scope: String,
    pub allowed_claims: Vec<String>,
    pub no_stable_claims: Vec<String>,
    pub validation_commands: Vec<String>,
    pub completed_promotion_class: String,
    pub next_missing_stable_promotion_class: String,
}

impl SyntaxGoRealSourceGeneratedCodecCorpusGate {
    pub fn validate_against_sources(
        &self,
        source_program_corpus_gate: &SyntaxGoRealSourceProgramValidityCorpusGate,
        manifest: &GoSyntaxCorpusManifest,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        require_string("artifact_kind", &self.artifact_kind, ARTIFACT_KIND)?;
        require_usize("version", self.version as usize, VERSION as usize)?;
        require_string("tool", &self.tool, TOOL)?;
        require_string("status", &self.status, STATUS)?;
        self.source_program_corpus_gate
            .validate(source_program_corpus_gate)?;
        self.generated_codec_report_contract.validate()?;
        self.corpus_round_trip
            .validate(source_program_corpus_gate, manifest)?;
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
pub struct SourceProgramCorpusGateRef {
    pub artifact_kind: String,
    pub version: u32,
    pub tool: String,
    pub status: String,
    pub module_path: String,
    pub fixture_path: String,
    pub source_typecheck_execution_report_path: String,
    pub source_typecheck_feasibility_report_path: String,
    pub source_typecheck_execution_completed_promotion_class: String,
    pub source_typecheck_execution_next_missing_stable_promotion_class: String,
    pub stable_metadata_promotion: SyntaxSourceProgramStableMetadataPromotionInput,
    pub go_generated_codec_completion_handoff:
        SyntaxSourceProgramGoGeneratedCodecCompletionHandoffInput,
    pub csharp_generated_codec_completion_handoff:
        SyntaxSourceProgramCSharpGeneratedCodecCompletionHandoffInput,
    pub preserved_typecheck_stable_report_fields: Vec<String>,
    pub preserved_typecheck_report_contract_stable_report_fields: Vec<String>,
    pub preserved_typecheck_blocked_claims: Vec<String>,
    pub preserved_typecheck_no_stable_claims: Vec<String>,
    pub execution_pass_envelope_required_record_count: usize,
    pub execution_pass_envelope_required_record_status: String,
    pub execution_pass_envelope_required_records_all_passed: bool,
    pub execution_pass_envelope_required_record_ids: Vec<String>,
    pub execution_evidence_typechecker_results_sidecar_only: bool,
    pub execution_evidence_type_correctness_claim_allowed: bool,
    pub execution_evidence_arena_ir_proof_used: bool,
    pub execution_evidence_json_schema_backend_unchanged: bool,
    pub corpus_manifest_path: String,
    pub corpus_check_report_path: String,
    pub corpus_eligible_record_count: usize,
    pub corpus_unsupported_record_count: usize,
    pub sidecar_source_program_validity_claim_allowed: bool,
    pub sidecar_arena_ir_proof_used: bool,
    pub sidecar_json_schema_backend_unchanged: bool,
    pub source_program_no_stable_claims: Vec<String>,
    pub completed_promotion_class: String,
    pub next_missing_stable_promotion_class: String,
}

impl SourceProgramCorpusGateRef {
    fn validate(
        &self,
        source_program_corpus_gate: &SyntaxGoRealSourceProgramValidityCorpusGate,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        require_string(
            "source_program_corpus_gate.artifact_kind",
            &self.artifact_kind,
            SOURCE_PROGRAM_CORPUS_ARTIFACT_KIND,
        )?;
        require_usize(
            "source_program_corpus_gate.version",
            self.version as usize,
            SOURCE_PROGRAM_CORPUS_VERSION as usize,
        )?;
        require_string(
            "source_program_corpus_gate.tool",
            &self.tool,
            SOURCE_PROGRAM_CORPUS_TOOL,
        )?;
        require_string(
            "source_program_corpus_gate.status",
            &self.status,
            SOURCE_PROGRAM_CORPUS_STATUS,
        )?;
        require_string(
            "source_program_corpus_gate.module_path",
            &self.module_path,
            "src/syntax_go_real_source_program_validity_corpus.rs",
        )?;
        require_string(
            "source_program_corpus_gate.fixture_path",
            &self.fixture_path,
            SOURCE_PROGRAM_CORPUS_FIXTURE_PATH,
        )?;
        require_string(
            "source_program_corpus_gate.source_typecheck_execution_report_path",
            &self.source_typecheck_execution_report_path,
            &source_program_corpus_gate
                .source_typecheck_preservation
                .source_typecheck_execution_report_path,
        )?;
        require_string(
            "source_program_corpus_gate.source_typecheck_feasibility_report_path",
            &self.source_typecheck_feasibility_report_path,
            &source_program_corpus_gate
                .source_typecheck_preservation
                .source_typecheck_feasibility_report_path,
        )?;
        require_string(
            "source_program_corpus_gate.source_typecheck_execution_completed_promotion_class",
            &self.source_typecheck_execution_completed_promotion_class,
            &source_program_corpus_gate
                .source_typecheck_preservation
                .source_typecheck_execution_completed_promotion_class,
        )?;
        require_string(
            "source_program_corpus_gate.source_typecheck_execution_next_missing_stable_promotion_class",
            &self.source_typecheck_execution_next_missing_stable_promotion_class,
            &source_program_corpus_gate
                .source_typecheck_preservation
                .source_typecheck_execution_next_missing_stable_promotion_class,
        )?;
        require_string_vec_equal(
            "source_program_corpus_gate.stable_metadata_promotion.source_completion_handoff_source_program_preserved_typecheck_report_contract_stable_report_fields",
            &self
                .stable_metadata_promotion
                .source_completion_handoff_source_program_preserved_typecheck_report_contract_stable_report_fields,
            &source_program_corpus_gate
                .source_typecheck_preservation
                .stable_metadata_promotion
                .source_completion_handoff_source_program_preserved_typecheck_report_contract_stable_report_fields,
        )?;
        require_stable_metadata_promotion_equal(
            "source_program_corpus_gate.stable_metadata_promotion",
            &self.stable_metadata_promotion,
            &source_program_corpus_gate
                .source_typecheck_preservation
                .stable_metadata_promotion,
        )?;
        require_go_completion_handoff_equal(
            "source_program_corpus_gate.go_generated_codec_completion_handoff",
            &self.go_generated_codec_completion_handoff,
            &source_program_corpus_gate
                .source_typecheck_preservation
                .go_generated_codec_completion_handoff,
        )?;
        require_csharp_completion_handoff_equal(
            "source_program_corpus_gate.csharp_generated_codec_completion_handoff",
            &self.csharp_generated_codec_completion_handoff,
            &source_program_corpus_gate
                .source_typecheck_preservation
                .csharp_generated_codec_completion_handoff,
        )?;
        require_all(
            "source_program_corpus_gate.preserved_typecheck_stable_report_fields",
            &self.preserved_typecheck_stable_report_fields,
            REQUIRED_SOURCE_PROGRAM_CORPUS_TYPECHECK_STABLE_REPORT_FIELDS,
        )?;
        require_all(
            "source_program_corpus_gate.actual_preserved_typecheck_stable_report_fields",
            &source_program_corpus_gate
                .source_typecheck_preservation
                .preserved_stable_report_fields,
            REQUIRED_SOURCE_PROGRAM_CORPUS_TYPECHECK_STABLE_REPORT_FIELDS,
        )?;
        require_all(
            "source_program_corpus_gate.preserved_typecheck_report_contract_stable_report_fields",
            &self.preserved_typecheck_report_contract_stable_report_fields,
            TYPECHECK_PRESERVATION_REPORT_STABLE_FIELDS,
        )?;
        require_all(
            "source_program_corpus_gate.actual_preserved_typecheck_report_contract_stable_report_fields",
            &source_program_corpus_gate
                .source_typecheck_preservation
                .preserved_report_contract_stable_report_fields,
            TYPECHECK_PRESERVATION_REPORT_STABLE_FIELDS,
        )?;
        require_string_vec_equal(
            "source_program_corpus_gate.preserved_typecheck_report_contract_stable_report_fields",
            &self.preserved_typecheck_report_contract_stable_report_fields,
            &source_program_corpus_gate
                .source_typecheck_preservation
                .preserved_report_contract_stable_report_fields,
        )?;
        require_all(
            "source_program_corpus_gate.preserved_typecheck_blocked_claims",
            &self.preserved_typecheck_blocked_claims,
            REQUIRED_SOURCE_PROGRAM_CORPUS_TYPECHECK_BLOCKED_CLAIMS,
        )?;
        require_all(
            "source_program_corpus_gate.actual_preserved_typecheck_blocked_claims",
            &source_program_corpus_gate
                .source_typecheck_preservation
                .preserved_required_blocked_claims,
            REQUIRED_SOURCE_PROGRAM_CORPUS_TYPECHECK_BLOCKED_CLAIMS,
        )?;
        require_all(
            "source_program_corpus_gate.preserved_typecheck_no_stable_claims",
            &self.preserved_typecheck_no_stable_claims,
            REQUIRED_SOURCE_PROGRAM_CORPUS_TYPECHECK_NO_STABLE_CLAIMS,
        )?;
        require_all(
            "source_program_corpus_gate.actual_preserved_typecheck_no_stable_claims",
            &source_program_corpus_gate
                .source_typecheck_preservation
                .preserved_no_stable_claims,
            REQUIRED_SOURCE_PROGRAM_CORPUS_TYPECHECK_NO_STABLE_CLAIMS,
        )?;
        require_usize(
            "source_program_corpus_gate.execution_pass_envelope_required_record_count",
            self.execution_pass_envelope_required_record_count,
            source_program_corpus_gate
                .source_typecheck_preservation
                .execution_pass_envelope_required_record_count,
        )?;
        require_string(
            "source_program_corpus_gate.execution_pass_envelope_required_record_status",
            &self.execution_pass_envelope_required_record_status,
            &source_program_corpus_gate
                .source_typecheck_preservation
                .execution_pass_envelope_required_record_status,
        )?;
        require_bool(
            "source_program_corpus_gate.execution_pass_envelope_required_records_all_passed",
            self.execution_pass_envelope_required_records_all_passed,
            source_program_corpus_gate
                .source_typecheck_preservation
                .execution_pass_envelope_required_records_all_passed,
        )?;
        require_all(
            "source_program_corpus_gate.execution_pass_envelope_required_record_ids",
            &self.execution_pass_envelope_required_record_ids,
            &source_program_corpus_gate
                .source_typecheck_preservation
                .execution_pass_envelope_required_record_ids
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>(),
        )?;
        require_bool(
            "source_program_corpus_gate.execution_evidence_typechecker_results_sidecar_only",
            self.execution_evidence_typechecker_results_sidecar_only,
            source_program_corpus_gate
                .source_typecheck_preservation
                .execution_evidence_typechecker_results_sidecar_only,
        )?;
        require_bool(
            "source_program_corpus_gate.execution_evidence_type_correctness_claim_allowed",
            self.execution_evidence_type_correctness_claim_allowed,
            source_program_corpus_gate
                .source_typecheck_preservation
                .execution_evidence_type_correctness_claim_allowed,
        )?;
        require_bool(
            "source_program_corpus_gate.execution_evidence_arena_ir_proof_used",
            self.execution_evidence_arena_ir_proof_used,
            source_program_corpus_gate
                .source_typecheck_preservation
                .execution_evidence_arena_ir_proof_used,
        )?;
        require_bool(
            "source_program_corpus_gate.execution_evidence_json_schema_backend_unchanged",
            self.execution_evidence_json_schema_backend_unchanged,
            source_program_corpus_gate
                .source_typecheck_preservation
                .execution_evidence_json_schema_backend_unchanged,
        )?;
        require_string(
            "source_program_corpus_gate.corpus_manifest_path",
            &self.corpus_manifest_path,
            CORPUS_MANIFEST_PATH,
        )?;
        require_string(
            "source_program_corpus_gate.actual_corpus_manifest_path",
            &source_program_corpus_gate.corpus_manifest.manifest_path,
            CORPUS_MANIFEST_PATH,
        )?;
        require_string(
            "source_program_corpus_gate.corpus_check_report_path",
            &self.corpus_check_report_path,
            CORPUS_REPORT_PATH,
        )?;
        require_string(
            "source_program_corpus_gate.actual_corpus_check_report_path",
            &source_program_corpus_gate
                .corpus_check_report_contract
                .report_path,
            CORPUS_REPORT_PATH,
        )?;
        require_usize(
            "source_program_corpus_gate.corpus_eligible_record_count",
            self.corpus_eligible_record_count,
            source_program_corpus_gate
                .corpus_eligibility
                .eligible_record_count,
        )?;
        require_usize(
            "source_program_corpus_gate.corpus_unsupported_record_count",
            self.corpus_unsupported_record_count,
            source_program_corpus_gate
                .corpus_eligibility
                .unsupported_record_count,
        )?;
        require_bool(
            "source_program_corpus_gate.sidecar_source_program_validity_claim_allowed",
            self.sidecar_source_program_validity_claim_allowed,
            source_program_corpus_gate
                .sidecar_validity_policy
                .source_program_validity_claim_allowed,
        )?;
        require_bool(
            "source_program_corpus_gate.sidecar_arena_ir_proof_used",
            self.sidecar_arena_ir_proof_used,
            source_program_corpus_gate
                .sidecar_validity_policy
                .arena_ir_proof_used,
        )?;
        require_bool(
            "source_program_corpus_gate.sidecar_json_schema_backend_unchanged",
            self.sidecar_json_schema_backend_unchanged,
            source_program_corpus_gate
                .sidecar_validity_policy
                .json_schema_backend_unchanged,
        )?;
        require_all(
            "source_program_corpus_gate.source_program_no_stable_claims",
            &self.source_program_no_stable_claims,
            REQUIRED_SOURCE_PROGRAM_CORPUS_NO_STABLE_CLAIMS,
        )?;
        require_all(
            "source_program_corpus_gate.actual_source_program_no_stable_claims",
            &source_program_corpus_gate.no_stable_claims,
            REQUIRED_SOURCE_PROGRAM_CORPUS_NO_STABLE_CLAIMS,
        )?;
        require_string(
            "source_program_corpus_gate.completed_promotion_class",
            &self.completed_promotion_class,
            SOURCE_PROGRAM_CORPUS_COMPLETED_PROMOTION_CLASS,
        )?;
        require_string(
            "source_program_corpus_gate.next_missing_stable_promotion_class",
            &self.next_missing_stable_promotion_class,
            COMPLETED_PROMOTION_CLASS,
        )?;
        require_string(
            "source_program_corpus_gate.actual_next_missing_stable_promotion_class",
            &source_program_corpus_gate.next_missing_stable_promotion_class,
            COMPLETED_PROMOTION_CLASS,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeneratedCodecCorpusReportContract {
    pub report_artifact_kind: String,
    pub report_path: String,
    pub report_command: String,
    pub report_generated_under_target: bool,
    pub report_checked_in: bool,
    pub expected_route: String,
    pub expected_generated_codec_marker: String,
    pub additional_report_paths: Vec<String>,
    pub additional_report_commands: Vec<String>,
    pub additional_expected_routes: Vec<String>,
    pub additional_expected_generated_codec_markers: Vec<String>,
    pub expected_next_missing_stable_promotion_class: String,
    pub stable_report_fields: Vec<String>,
    pub volatile_report_fields: Vec<String>,
}

impl GeneratedCodecCorpusReportContract {
    fn validate(&self) -> Result<(), SyntaxBackendBoundaryError> {
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
                "Go real-source generated-codec reports must stay under target/",
            ));
        }
        require_string(
            "generated_codec_report_contract.report_command",
            &self.report_command,
            GENERATED_CODEC_REPORT_COMMAND,
        )?;
        require_all(
            "generated_codec_report_contract.additional_report_paths",
            &self.additional_report_paths,
            &[
                CRYPTO_SYSRAND_RAND_WINDOWS_GENERATED_CODEC_REPORT_PATH,
                STRINGS_CLONE_GENERATED_CODEC_REPORT_PATH,
                SORT_INTERFACE_GENERATED_CODEC_REPORT_PATH,
                SORT_SEARCH_GENERATED_CODEC_REPORT_PATH,
                STRINGS_SEARCH_GENERATED_CODEC_REPORT_PATH,
                PATH_MATCH_GENERATED_CODEC_REPORT_PATH,
            ],
        )?;
        for path in &self.additional_report_paths {
            if !path.starts_with("target/") {
                return Err(SyntaxBackendBoundaryError::new(
                    "generated_codec_report_contract.additional_report_paths",
                    "Go real-source generated-codec reports must stay under target/",
                ));
            }
        }
        require_all(
            "generated_codec_report_contract.additional_report_commands",
            &self.additional_report_commands,
            &[
                CRYPTO_SYSRAND_RAND_WINDOWS_GENERATED_CODEC_REPORT_COMMAND,
                STRINGS_CLONE_GENERATED_CODEC_REPORT_COMMAND,
                SORT_INTERFACE_GENERATED_CODEC_REPORT_COMMAND,
                SORT_SEARCH_GENERATED_CODEC_REPORT_COMMAND,
                STRINGS_SEARCH_GENERATED_CODEC_REPORT_COMMAND,
                PATH_MATCH_GENERATED_CODEC_REPORT_COMMAND,
            ],
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
            GENERATED_CODEC_ROUTE_ID,
        )?;
        require_string(
            "generated_codec_report_contract.expected_generated_codec_marker",
            &self.expected_generated_codec_marker,
            GENERATED_CODEC_MARKER,
        )?;
        require_all(
            "generated_codec_report_contract.additional_expected_routes",
            &self.additional_expected_routes,
            &[
                CRYPTO_SYSRAND_RAND_WINDOWS_GENERATED_CODEC_ROUTE_ID,
                STRINGS_CLONE_GENERATED_CODEC_ROUTE_ID,
                SORT_INTERFACE_GENERATED_CODEC_ROUTE_ID,
                SORT_SEARCH_GENERATED_CODEC_ROUTE_ID,
                STRINGS_SEARCH_GENERATED_CODEC_ROUTE_ID,
                PATH_MATCH_GENERATED_CODEC_ROUTE_ID,
            ],
        )?;
        require_all(
            "generated_codec_report_contract.additional_expected_generated_codec_markers",
            &self.additional_expected_generated_codec_markers,
            &[
                CRYPTO_SYSRAND_RAND_WINDOWS_GENERATED_CODEC_MARKER,
                STRINGS_CLONE_GENERATED_CODEC_MARKER,
                SORT_INTERFACE_GENERATED_CODEC_MARKER,
                SORT_SEARCH_GENERATED_CODEC_MARKER,
                STRINGS_SEARCH_GENERATED_CODEC_MARKER,
                PATH_MATCH_GENERATED_CODEC_MARKER,
            ],
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
pub struct GeneratedCodecCorpusRoundTripEvidence {
    pub corpus_id: String,
    pub corpus_manifest_path: String,
    pub syntax_level_boundary: String,
    pub all_checked_records_are_complete_source_files: bool,
    pub total_corpus_record_count: usize,
    pub generated_codec_round_trip_record_count: usize,
    pub blocked_record_count: usize,
    pub supported_records: Vec<GeneratedCodecCorpusSupportedRecord>,
    pub blocked_records: Vec<GeneratedCodecCorpusBlockedRecord>,
}

impl GeneratedCodecCorpusRoundTripEvidence {
    fn validate(
        &self,
        source_program_corpus_gate: &SyntaxGoRealSourceProgramValidityCorpusGate,
        manifest: &GoSyntaxCorpusManifest,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        require_string("corpus_round_trip.corpus_id", &self.corpus_id, CORPUS_ID)?;
        require_string(
            "corpus_round_trip.corpus_manifest_path",
            &self.corpus_manifest_path,
            CORPUS_MANIFEST_PATH,
        )?;
        require_string(
            "corpus_round_trip.syntax_level_boundary",
            &self.syntax_level_boundary,
            SYNTAX_LEVEL_BOUNDARY,
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
            "manifest.entries",
            manifest.entries.len(),
            self.total_corpus_record_count,
        )?;
        require_usize(
            "source_program_corpus_gate.corpus_eligibility.eligible_record_count",
            source_program_corpus_gate
                .corpus_eligibility
                .eligible_record_count,
            self.total_corpus_record_count,
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

        let manifest_by_path = manifest_entries_by_path(manifest);
        let eligible_paths = source_program_corpus_gate
            .corpus_eligibility
            .eligible_records
            .iter()
            .map(|record| record.manifest_path.as_str())
            .collect::<BTreeSet<_>>();
        let mut accounted = BTreeSet::new();

        for record in &self.supported_records {
            let entry = manifest_entry(&manifest_by_path, &record.manifest_path)?;
            if !eligible_paths.contains(record.manifest_path.as_str()) {
                return Err(SyntaxBackendBoundaryError::new(
                    "corpus_round_trip.supported_records",
                    format!(
                        "unsupported source-program corpus record {}",
                        record.manifest_path
                    ),
                ));
            }
            record.validate(entry)?;
            accounted.insert(record.manifest_path.as_str());
        }

        for record in &self.blocked_records {
            let entry = manifest_entry(&manifest_by_path, &record.manifest_path)?;
            if !eligible_paths.contains(record.manifest_path.as_str()) {
                return Err(SyntaxBackendBoundaryError::new(
                    "corpus_round_trip.blocked_records",
                    format!(
                        "unknown source-program corpus record {}",
                        record.manifest_path
                    ),
                ));
            }
            record.validate(entry)?;
            accounted.insert(record.manifest_path.as_str());
        }

        for entry in &manifest.entries {
            if !accounted.contains(entry.manifest_path.as_str()) {
                return Err(SyntaxBackendBoundaryError::new(
                    "corpus_round_trip.records",
                    format!(
                        "missing corpus generated-codec disposition {}",
                        entry.manifest_path
                    ),
                ));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeneratedCodecCorpusSupportedRecord {
    pub manifest_path: String,
    pub source_path: String,
    pub source_sha256: String,
    pub package: String,
    pub source_function: String,
    pub generated_codec_route: String,
    pub generated_codec_marker: String,
    pub generated_codec_round_trip_status: String,
    pub tct_token_count: usize,
    pub baseline_byte_token_count: usize,
    pub gofmt_validation: String,
    pub go_parser_validation: String,
    pub source_program_validity_claim_allowed: bool,
    pub stable_promotion_allowed: bool,
}

impl GeneratedCodecCorpusSupportedRecord {
    fn validate(&self, entry: &GoSyntaxCorpusEntry) -> Result<(), SyntaxBackendBoundaryError> {
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
        require_string(
            "manifest.entry.source_path",
            &entry.file.path,
            &self.source_path,
        )?;
        require_string(
            "manifest.entry.sha256",
            &entry.file.sha256,
            &self.source_sha256,
        )?;
        require_string(
            "corpus_round_trip.supported_record.package",
            &self.package,
            expected.package,
        )?;
        require_string(
            "corpus_round_trip.supported_record.source_function",
            &self.source_function,
            expected.source_function,
        )?;
        require_string(
            "corpus_round_trip.supported_record.generated_codec_route",
            &self.generated_codec_route,
            expected.generated_codec_route,
        )?;
        require_string(
            "corpus_round_trip.supported_record.generated_codec_marker",
            &self.generated_codec_marker,
            expected.generated_codec_marker,
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
        require_usize(
            "corpus_round_trip.supported_record.baseline_byte_token_count",
            self.baseline_byte_token_count,
            expected.baseline_byte_token_count,
        )?;
        require_string(
            "corpus_round_trip.supported_record.gofmt_validation",
            &self.gofmt_validation,
            "passed",
        )?;
        require_string(
            "corpus_round_trip.supported_record.go_parser_validation",
            &self.go_parser_validation,
            "passed",
        )?;
        require_bool(
            "corpus_round_trip.supported_record.source_program_validity_claim_allowed",
            self.source_program_validity_claim_allowed,
            false,
        )?;
        require_bool(
            "corpus_round_trip.supported_record.stable_promotion_allowed",
            self.stable_promotion_allowed,
            false,
        )
    }
}

struct SupportedRecordPolicy {
    manifest_path: &'static str,
    source_path: &'static str,
    source_sha256: &'static str,
    package: &'static str,
    source_function: &'static str,
    generated_codec_route: &'static str,
    generated_codec_marker: &'static str,
    tct_token_count: usize,
    baseline_byte_token_count: usize,
}

fn supported_record_policy(
    manifest_path: &str,
) -> Result<SupportedRecordPolicy, SyntaxBackendBoundaryError> {
    match manifest_path {
        SUPPORTED_MANIFEST_PATH => Ok(SupportedRecordPolicy {
            manifest_path: SUPPORTED_MANIFEST_PATH,
            source_path: SUPPORTED_SOURCE_PATH,
            source_sha256: SUPPORTED_SOURCE_SHA256,
            package: "strings",
            source_function: SUPPORTED_FUNCTION,
            generated_codec_route: GENERATED_CODEC_ROUTE_ID,
            generated_codec_marker: GENERATED_CODEC_MARKER,
            tct_token_count: REQUIRED_STRINGS_COMPARE_TCT_TOKEN_COUNT,
            baseline_byte_token_count: REQUIRED_STRINGS_COMPARE_BASELINE_BYTE_TOKEN_COUNT,
        }),
        CRYPTO_SYSRAND_RAND_WINDOWS_MANIFEST_PATH => Ok(SupportedRecordPolicy {
            manifest_path: CRYPTO_SYSRAND_RAND_WINDOWS_MANIFEST_PATH,
            source_path: CRYPTO_SYSRAND_RAND_WINDOWS_SOURCE_PATH,
            source_sha256: CRYPTO_SYSRAND_RAND_WINDOWS_SOURCE_SHA256,
            package: "sysrand",
            source_function: CRYPTO_SYSRAND_RAND_WINDOWS_SOURCE_FUNCTION,
            generated_codec_route: CRYPTO_SYSRAND_RAND_WINDOWS_GENERATED_CODEC_ROUTE_ID,
            generated_codec_marker: CRYPTO_SYSRAND_RAND_WINDOWS_GENERATED_CODEC_MARKER,
            tct_token_count: REQUIRED_CRYPTO_SYSRAND_RAND_WINDOWS_TCT_TOKEN_COUNT,
            baseline_byte_token_count:
                REQUIRED_CRYPTO_SYSRAND_RAND_WINDOWS_BASELINE_BYTE_TOKEN_COUNT,
        }),
        STRINGS_CLONE_MANIFEST_PATH => Ok(SupportedRecordPolicy {
            manifest_path: STRINGS_CLONE_MANIFEST_PATH,
            source_path: STRINGS_CLONE_SOURCE_PATH,
            source_sha256: STRINGS_CLONE_SOURCE_SHA256,
            package: "strings",
            source_function: STRINGS_CLONE_SOURCE_FUNCTION,
            generated_codec_route: STRINGS_CLONE_GENERATED_CODEC_ROUTE_ID,
            generated_codec_marker: STRINGS_CLONE_GENERATED_CODEC_MARKER,
            tct_token_count: REQUIRED_STRINGS_CLONE_TCT_TOKEN_COUNT,
            baseline_byte_token_count: REQUIRED_STRINGS_CLONE_BASELINE_BYTE_TOKEN_COUNT,
        }),
        SORT_INTERFACE_MANIFEST_PATH => Ok(SupportedRecordPolicy {
            manifest_path: SORT_INTERFACE_MANIFEST_PATH,
            source_path: SORT_INTERFACE_SOURCE_PATH,
            source_sha256: SORT_INTERFACE_SOURCE_SHA256,
            package: "sort_test",
            source_function: SORT_INTERFACE_SOURCE_FUNCTION,
            generated_codec_route: SORT_INTERFACE_GENERATED_CODEC_ROUTE_ID,
            generated_codec_marker: SORT_INTERFACE_GENERATED_CODEC_MARKER,
            tct_token_count: REQUIRED_SORT_INTERFACE_TCT_TOKEN_COUNT,
            baseline_byte_token_count: REQUIRED_SORT_INTERFACE_BASELINE_BYTE_TOKEN_COUNT,
        }),
        SORT_SEARCH_MANIFEST_PATH => Ok(SupportedRecordPolicy {
            manifest_path: SORT_SEARCH_MANIFEST_PATH,
            source_path: SORT_SEARCH_SOURCE_PATH,
            source_sha256: SORT_SEARCH_SOURCE_SHA256,
            package: "sort_test",
            source_function: SORT_SEARCH_SOURCE_FUNCTION,
            generated_codec_route: SORT_SEARCH_GENERATED_CODEC_ROUTE_ID,
            generated_codec_marker: SORT_SEARCH_GENERATED_CODEC_MARKER,
            tct_token_count: REQUIRED_SORT_SEARCH_TCT_TOKEN_COUNT,
            baseline_byte_token_count: REQUIRED_SORT_SEARCH_BASELINE_BYTE_TOKEN_COUNT,
        }),
        STRINGS_SEARCH_MANIFEST_PATH => Ok(SupportedRecordPolicy {
            manifest_path: STRINGS_SEARCH_MANIFEST_PATH,
            source_path: STRINGS_SEARCH_SOURCE_PATH,
            source_sha256: STRINGS_SEARCH_SOURCE_SHA256,
            package: "strings",
            source_function: STRINGS_SEARCH_SOURCE_FUNCTION,
            generated_codec_route: STRINGS_SEARCH_GENERATED_CODEC_ROUTE_ID,
            generated_codec_marker: STRINGS_SEARCH_GENERATED_CODEC_MARKER,
            tct_token_count: REQUIRED_STRINGS_SEARCH_TCT_TOKEN_COUNT,
            baseline_byte_token_count: REQUIRED_STRINGS_SEARCH_BASELINE_BYTE_TOKEN_COUNT,
        }),
        PATH_MATCH_MANIFEST_PATH => Ok(SupportedRecordPolicy {
            manifest_path: PATH_MATCH_MANIFEST_PATH,
            source_path: PATH_MATCH_SOURCE_PATH,
            source_sha256: PATH_MATCH_SOURCE_SHA256,
            package: "path",
            source_function: PATH_MATCH_SOURCE_FUNCTION,
            generated_codec_route: PATH_MATCH_GENERATED_CODEC_ROUTE_ID,
            generated_codec_marker: PATH_MATCH_GENERATED_CODEC_MARKER,
            tct_token_count: REQUIRED_PATH_MATCH_TCT_TOKEN_COUNT,
            baseline_byte_token_count: REQUIRED_PATH_MATCH_BASELINE_BYTE_TOKEN_COUNT,
        }),
        other => Err(SyntaxBackendBoundaryError::new(
            "corpus_round_trip.supported_record.manifest_path",
            format!("unsupported generated-codec corpus record {other}"),
        )),
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeneratedCodecCorpusBlockedRecord {
    pub manifest_path: String,
    pub source_path: String,
    pub status: String,
    pub route_blocker: String,
    pub construct_blockers: Vec<String>,
    pub generated_codec_round_trip_status: String,
    pub source_program_validity_claim_allowed: bool,
    pub stable_promotion_allowed: bool,
}

impl GeneratedCodecCorpusBlockedRecord {
    fn validate(&self, entry: &GoSyntaxCorpusEntry) -> Result<(), SyntaxBackendBoundaryError> {
        require_string(
            "corpus_round_trip.blocked_record.source_path",
            &self.source_path,
            &entry.file.path,
        )?;
        require_string(
            "corpus_round_trip.blocked_record.status",
            &self.status,
            BLOCKED_RECORD_STATUS,
        )?;
        require_string(
            "corpus_round_trip.blocked_record.route_blocker",
            &self.route_blocker,
            GENERATED_CODEC_ROUTE_BLOCKER,
        )?;
        if self.construct_blockers.is_empty() {
            return Err(SyntaxBackendBoundaryError::new(
                "corpus_round_trip.blocked_record.construct_blockers",
                "blocked real-source corpus records must name at least one construct blocker",
            ));
        }
        require_string(
            "corpus_round_trip.blocked_record.generated_codec_round_trip_status",
            &self.generated_codec_round_trip_status,
            BLOCKED_RECORD_STATUS,
        )?;
        require_bool(
            "corpus_round_trip.blocked_record.source_program_validity_claim_allowed",
            self.source_program_validity_claim_allowed,
            false,
        )?;
        require_bool(
            "corpus_round_trip.blocked_record.stable_promotion_allowed",
            self.stable_promotion_allowed,
            false,
        )
    }
}

fn manifest_entries_by_path(
    manifest: &GoSyntaxCorpusManifest,
) -> BTreeMap<&str, &GoSyntaxCorpusEntry> {
    manifest
        .entries
        .iter()
        .map(|entry| (entry.manifest_path.as_str(), entry))
        .collect()
}

fn manifest_entry<'a>(
    entries_by_path: &'a BTreeMap<&str, &GoSyntaxCorpusEntry>,
    manifest_path: &str,
) -> Result<&'a GoSyntaxCorpusEntry, SyntaxBackendBoundaryError> {
    entries_by_path.get(manifest_path).copied().ok_or_else(|| {
        SyntaxBackendBoundaryError::new(
            "corpus_round_trip.records",
            format!("missing manifest entry {manifest_path}"),
        )
    })
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
            "Go generated-codec corpus gate must preserve the source-program corpus stable metadata promotion input summary",
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
            "Go generated-codec corpus gate must preserve the source-program corpus generated-codec completion handoff input summary",
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
            "Go generated-codec corpus gate must preserve the source-program corpus C# generated-codec completion handoff input summary",
        ))
    }
}

fn require_string_vec_equal(
    field: &'static str,
    actual: &[String],
    expected: &[String],
) -> Result<(), SyntaxBackendBoundaryError> {
    if actual == expected {
        Ok(())
    } else {
        Err(SyntaxBackendBoundaryError::new(
            field,
            "Go generated-codec corpus gate must preserve the source-program corpus type-checker preservation report fields",
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
