//! Generated-codec state API route gate.
//!
//! This module promotes the checked target-neutral state API integration into
//! route-owned generated-codec metadata. It does not change generated decode
//! semantics, route `GrammarIR` or `StructuralModel` through the JSON Schema
//! compiler path, or promote source-program validity.

use crate::provenance::json_sha256;
use crate::syntax_backend_boundary::SyntaxBackendBoundaryError;
use crate::syntax_generated_codec::{
    GeneratedCodecPrefixStateApiMetadata, SyntaxGeneratedCodecArtifact,
    generated_codec_prefix_state_api_metadata,
};
use crate::syntax_generated_codec_state_api_integration::{
    ARTIFACT_KIND as INTEGRATION_ARTIFACT_KIND,
    COMPLETED_PROMOTION_CLASS as INTEGRATION_COMPLETED_PROMOTION_CLASS,
    FIXTURE_PATH as INTEGRATION_FIXTURE_PATH, GeneratedCodecStateApiIntegrationRecordQuery,
    MATCHED_PRESENCE_MASK_SCOPE,
    NEXT_MISSING_STABLE_PROMOTION_CLASS as INTEGRATION_NEXT_MISSING_STABLE_PROMOTION_CLASS,
    SyntaxGeneratedCodecStateApiIntegrationGate, TARGET_NEUTRAL_RAW_STATE_API_AUTHORITY,
    TargetNeutralMatchedPresenceMaskContract, TargetNeutralPrefixStateContinuationResult,
    VERSION as INTEGRATION_VERSION,
};
use crate::syntax_go_source_instance_projection::ProjectionTokenLayoutRangeSummary;
use crate::syntax_production_route::{
    ARTIFACT_KIND as PRODUCTION_ROUTE_ARTIFACT_KIND, FIXTURE_PATH as PRODUCTION_ROUTE_FIXTURE_PATH,
    ROUTE_ENTRYPOINT, ROUTE_GATE, STABLE_BACKEND_SCOPE,
    SYNTAX_COMPILER_ADMISSIBLE_TOKEN_MASK_ROUTE_COMPLETED_PROMOTION_CLASS,
    SYNTAX_COMPILER_ADMISSIBLE_TOKEN_MASK_ROUTE_NEXT_MISSING_STABLE_PROMOTION_CLASS,
    SYNTAX_COMPILER_PREFIX_STATE_CONTINUATION_ROUTE_COMPLETED_PROMOTION_CLASS,
    SYNTAX_COMPILER_PREFIX_STATE_CONTINUATION_ROUTE_NEXT_MISSING_STABLE_PROMOTION_CLASS,
    SYNTAX_COMPILER_PREFIX_STATE_CONTINUATION_ROUTE_RESPONSE_EVIDENCE_FIXTURE,
    SYNTAX_COMPILER_ROUTE_ENTRYPOINT, SYNTAX_COMPILER_STATE_API_ROUTE_COMPLETED_PROMOTION_CLASS,
    SYNTAX_COMPILER_STATE_API_ROUTE_NEXT_MISSING_STABLE_PROMOTION_CLASS,
    SyntaxProductionRouteImplementationGate, VERSION as PRODUCTION_ROUTE_VERSION,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeSet;

pub const TOOL: &str = "syntax_generated_codec_state_api_route_gate";
pub const VERSION: u32 = 1;
pub const ARTIFACT_KIND: &str = "tct_syntax_generated_codec_state_api_route_gate_v1";
pub const FIXTURE_PATH: &str =
    "tests/fixtures/syntax_backend/generated-codec-state-api-route-gate-v1.json";
pub const STATUS: &str = "generated_codec_state_api_route_gate_ready";
pub const COMPLETED_PROMOTION_CLASS: &str =
    SYNTAX_COMPILER_STATE_API_ROUTE_COMPLETED_PROMOTION_CLASS;
pub const NEXT_MISSING_STABLE_PROMOTION_CLASS: &str =
    SYNTAX_COMPILER_STATE_API_ROUTE_NEXT_MISSING_STABLE_PROMOTION_CLASS;
pub const LIVE_API_MATRIX_TOOL: &str = "syntax_generated_codec_prefix_state_live_api_matrix";
pub const LIVE_API_MATRIX_ARTIFACT_KIND: &str =
    "tct_syntax_generated_codec_prefix_state_live_api_matrix_v1";
pub const LIVE_API_MATRIX_FIXTURE_PATH: &str =
    "tests/fixtures/syntax_backend/generated-codec-prefix-state-live-api-matrix-v1.json";
pub const LIVE_API_MATRIX_STATUS: &str = "generated_codec_prefix_state_live_api_matrix_ready";
pub const LIVE_API_MATRIX_COMPLETED_PROMOTION_CLASS: &str =
    "syntax backend generated-codec prefix-state continuation live API matrix gate";
pub const LIVE_API_MATRIX_NEXT_MISSING_STABLE_PROMOTION_CLASS: &str =
    SYNTAX_COMPILER_PREFIX_STATE_CONTINUATION_ROUTE_NEXT_MISSING_STABLE_PROMOTION_CLASS;
pub const GO_SOURCE_INSTANCE_LIVE_API_ROUTE_BLOCKER: &str = "one or more checked Go source-instance generated codecs have prefix-state continuation response evidence but no registered generated-codec live prefix-state API route; consume the source-instance projection in reusable generated-codec renderer/report machinery before claiming live Go route consumption";
pub const MATCHED_PRESENCE_MASK_ROUTE_EVIDENCE_ID: &str =
    "route-owned-matched-go-csharp-presence-mask-v1";

const TARGET_AGNOSTIC_ROUTE_METADATA_FIELD: &str = "target_agnostic_compiler_route";
const STATE_API_ROUTE_METADATA_FIELD: &str = "generated_codec_state_api_route";
const ADMISSIBLE_TOKEN_MASK_METADATA_FIELD: &str = "admissible_token_mask_contract";
const PREFIX_STATE_CONTINUATION_METADATA_FIELD: &str = "prefix_state_continuation_contract";
const MATCHED_PRESENCE_MASK_SOURCE_CONTRACT: &str =
    "src/syntax_generated_codec_state_api_integration.rs::matched_go_csharp_presence_mask_contract";
const ADMISSIBLE_TOKEN_MASK_CONTRACT_ID: &str =
    "target-neutral-generated-codec-admissible-token-mask-v1";
const PREFIX_STATE_CONTINUATION_CONTRACT_ID: &str =
    "target-neutral-generated-codec-prefix-state-continuation-v1";
const ADMISSIBLE_TOKEN_MASK_SOURCE_QUERY_CONTRACT: &str =
    "target-neutral generated-codec state API integration record queries";
const ADMISSIBLE_TOKEN_MASK_VALIDITY_AUTHORITY: &str = "target-neutral raw transition API";
const GO_SOURCE_INSTANCE_GRAMMAR_IDS: &[&str] = &[
    "go_os_dir_unix_source_file_syntax_v0",
    "go_net_rpc_client_source_file_syntax_v0",
    "go_internal_coverage_cfile_testsupport_source_file_syntax_v0",
    "go_net_ipsock_plan9_source_file_syntax_v0",
    "go_go_ast_commentmap_source_file_syntax_v0",
];
const GO_SOURCE_INSTANCE_RECORDS: &[&str] = &[
    "tests/fixtures/go_syntax_corpus/src/os/dir_unix.go",
    "tests/fixtures/go_syntax_corpus/src/net/rpc/client.go",
    "tests/fixtures/go_syntax_corpus/src/internal/coverage/cfile/testsupport.go",
    "tests/fixtures/go_syntax_corpus/src/net/ipsock_plan9.go",
    "tests/fixtures/go_syntax_corpus/src/go/ast/commentmap.go",
];
const CSHARP_SOURCE_FILE_GRAMMAR_ID: &str = "csharp_compilation_unit_source_file_syntax_v0";
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
const REQUIRED_CSHARP_RECORDS: &[&str] = &[
    "pattern-matching/Shapes.cs",
    "delegates-and-events/Logger.cs",
    "events/VersionOne.cs",
    "iterators/IteratorMethods.cs",
    "serialization/Loan.cs",
];
const REQUIRED_PREFIX_STATE_CONTINUATION_REQUEST_FIELDS: &[&str] = &[
    "target",
    "record_id",
    "raw_prefix_token_count",
    "raw_prefix_token_ids_sha256",
    "prefix_state_case_id",
];
const REQUIRED_PREFIX_STATE_CONTINUATION_RESPONSE_FIELDS: &[&str] = &[
    "raw_prefix_token_ids",
    "consumed_raw_tokens",
    "prefix_status",
    "eof_valid",
    "finite_admissible_raw_token_ids",
    "payload_token_ranges",
    "bpe_raw_expansion_first_failure_diagnostic",
];
const REQUIRED_VALIDATION_COMMANDS: &[&str] = &[
    "cargo test --test syntax_backend_boundary_test syntax_backend_generated_codec_state_api_route_gate_consumes_integrated_route_metadata -- --exact",
    "cargo test --test syntax_state_machine_replay_test matched_go_csharp_presence_masks_use_target_neutral_state_api -- --exact",
    "cargo test --test syntax_state_machine_replay_test target_neutral_generated_codec_state_api_integration_consumes_shared_handoff_surface -- --exact",
    "cargo test --test syntax_generated_codec_gate_test csharp_compilation_unit_source_file_syntax_generated_codec_compiles_through_compiler_route -- --exact",
    "cargo test --test syntax_generated_codec_gate_test csharp_compilation_unit_iterator_methods_source_file_syntax_generated_codec_compiles_through_compiler_route -- --exact",
    "cargo test --test syntax_generated_codec_gate_test csharp_compilation_unit_loan_source_file_syntax_generated_codec_compiles_through_compiler_route -- --exact",
    "git diff --check",
];
const REQUIRED_NO_STABLE_CLAIMS: &[&str] = &[
    "no_arena_ir_claim",
    "no_bpe_layer_validity_claim",
    "no_generated_hot_path_claim",
    "no_paper_facing_claim",
    "no_source_program_validity_claim",
    "no_state_api_stable_backend_promotion_claim",
    "no_type_correctness_claim",
    "no_whole_source_language_validity_claim",
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxGeneratedCodecStateApiRouteGate {
    pub artifact_kind: String,
    pub version: u32,
    pub tool: String,
    pub status: String,
    pub source_state_api_integration: SyntaxGeneratedCodecStateApiRouteSourceIntegration,
    pub source_production_route: SyntaxGeneratedCodecStateApiRouteSourceProductionRoute,
    pub route_metadata_contract: SyntaxGeneratedCodecStateApiRouteMetadataContract,
    pub bpe_raw_expansion_contract: SyntaxGeneratedCodecStateApiRouteBpeContract,
    pub admissible_token_mask_contract: SyntaxGeneratedCodecStateApiRouteAdmissibleMaskContract,
    pub prefix_state_continuation_contract:
        SyntaxGeneratedCodecStateApiRoutePrefixStateContinuationContract,
    pub matched_presence_mask_contract:
        SyntaxGeneratedCodecStateApiRouteMatchedPresenceMaskContract,
    pub claim_boundary: SyntaxGeneratedCodecStateApiRouteClaimBoundary,
    pub validation_commands: Vec<String>,
    pub completed_promotion_class: String,
    pub next_missing_stable_promotion_class: String,
}

impl SyntaxGeneratedCodecStateApiRouteGate {
    pub fn validate_against_integration_and_route(
        &self,
        integration_gate: &SyntaxGeneratedCodecStateApiIntegrationGate,
        production_route_gate: &SyntaxProductionRouteImplementationGate,
        route_artifacts: &[SyntaxGeneratedCodecArtifact],
    ) -> Result<(), SyntaxBackendBoundaryError> {
        require_string("artifact_kind", &self.artifact_kind, ARTIFACT_KIND)?;
        require_u32("version", self.version, VERSION)?;
        require_string("tool", &self.tool, TOOL)?;
        require_string("status", &self.status, STATUS)?;
        self.source_state_api_integration
            .validate(integration_gate)?;
        self.source_production_route
            .validate(production_route_gate)?;
        self.route_metadata_contract
            .validate(integration_gate, route_artifacts)?;
        self.bpe_raw_expansion_contract.validate(integration_gate)?;
        self.admissible_token_mask_contract
            .validate(integration_gate)?;
        self.prefix_state_continuation_contract
            .validate(integration_gate)?;
        self.matched_presence_mask_contract.validate_shape()?;
        self.claim_boundary.validate()?;
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

    pub fn validate_against_matched_presence_mask_contract(
        &self,
        matched_contract: &TargetNeutralMatchedPresenceMaskContract,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        self.matched_presence_mask_contract
            .validate_against_matched_contract(matched_contract)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxGeneratedCodecStateApiRouteSourceIntegration {
    pub artifact_kind: String,
    pub version: u32,
    pub module_path: String,
    pub fixture_path: String,
    pub completed_promotion_class: String,
    pub next_missing_stable_promotion_class: String,
}

impl SyntaxGeneratedCodecStateApiRouteSourceIntegration {
    fn validate(
        &self,
        integration_gate: &SyntaxGeneratedCodecStateApiIntegrationGate,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        require_string(
            "source_state_api_integration.artifact_kind",
            &self.artifact_kind,
            INTEGRATION_ARTIFACT_KIND,
        )?;
        require_u32(
            "source_state_api_integration.version",
            self.version,
            INTEGRATION_VERSION,
        )?;
        require_string(
            "source_state_api_integration.module_path",
            &self.module_path,
            "src/syntax_generated_codec_state_api_integration.rs",
        )?;
        require_string(
            "source_state_api_integration.fixture_path",
            &self.fixture_path,
            INTEGRATION_FIXTURE_PATH,
        )?;
        require_string(
            "source_state_api_integration.completed_promotion_class",
            &self.completed_promotion_class,
            INTEGRATION_COMPLETED_PROMOTION_CLASS,
        )?;
        require_string(
            "source_state_api_integration.next_missing_stable_promotion_class",
            &self.next_missing_stable_promotion_class,
            INTEGRATION_NEXT_MISSING_STABLE_PROMOTION_CLASS,
        )?;
        require_string(
            "source_state_api_integration.source_fixture_completed_class",
            &integration_gate.completed_promotion_class,
            INTEGRATION_COMPLETED_PROMOTION_CLASS,
        )?;
        require_string(
            "source_state_api_integration.source_fixture_next_class",
            &integration_gate.next_missing_stable_promotion_class,
            INTEGRATION_NEXT_MISSING_STABLE_PROMOTION_CLASS,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxGeneratedCodecStateApiRouteSourceProductionRoute {
    pub artifact_kind: String,
    pub version: u32,
    pub module_path: String,
    pub fixture_path: String,
    pub route_entrypoint: String,
    pub compiler_route_entrypoint: String,
    pub route_gate: String,
    pub stable_backend_scope: String,
}

impl SyntaxGeneratedCodecStateApiRouteSourceProductionRoute {
    fn validate(
        &self,
        production_route_gate: &SyntaxProductionRouteImplementationGate,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        require_string(
            "source_production_route.artifact_kind",
            &self.artifact_kind,
            PRODUCTION_ROUTE_ARTIFACT_KIND,
        )?;
        require_u32(
            "source_production_route.version",
            self.version,
            PRODUCTION_ROUTE_VERSION,
        )?;
        require_string(
            "source_production_route.module_path",
            &self.module_path,
            "src/syntax_production_route.rs",
        )?;
        require_string(
            "source_production_route.fixture_path",
            &self.fixture_path,
            PRODUCTION_ROUTE_FIXTURE_PATH,
        )?;
        require_string(
            "source_production_route.route_entrypoint",
            &self.route_entrypoint,
            ROUTE_ENTRYPOINT,
        )?;
        require_string(
            "source_production_route.compiler_route_entrypoint",
            &self.compiler_route_entrypoint,
            SYNTAX_COMPILER_ROUTE_ENTRYPOINT,
        )?;
        require_string(
            "source_production_route.route_gate",
            &self.route_gate,
            ROUTE_GATE,
        )?;
        require_string(
            "source_production_route.stable_backend_scope",
            &self.stable_backend_scope,
            STABLE_BACKEND_SCOPE,
        )?;
        require_string(
            "source_production_route.source_fixture_artifact_kind",
            &production_route_gate.artifact_kind,
            PRODUCTION_ROUTE_ARTIFACT_KIND,
        )?;
        require_string(
            "source_production_route.source_fixture_entrypoint",
            &production_route_gate
                .route_contract
                .target_agnostic_compiler_entrypoint,
            SYNTAX_COMPILER_ROUTE_ENTRYPOINT,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxGeneratedCodecStateApiRouteMetadataContract {
    pub target_agnostic_compiler_route_metadata_field: String,
    pub state_api_route_metadata_field: String,
    pub target_ids: Vec<String>,
    pub capabilities: Vec<String>,
    pub route_evidence: Vec<String>,
    pub go_source_instance_records: Vec<String>,
    pub csharp_real_source_records: Vec<String>,
    pub record_query_count: usize,
    pub requires_route_artifact_metadata: bool,
    pub requires_integration_query_contract: bool,
}

impl SyntaxGeneratedCodecStateApiRouteMetadataContract {
    fn validate(
        &self,
        integration_gate: &SyntaxGeneratedCodecStateApiIntegrationGate,
        route_artifacts: &[SyntaxGeneratedCodecArtifact],
    ) -> Result<(), SyntaxBackendBoundaryError> {
        require_string(
            "route_metadata_contract.target_agnostic_compiler_route_metadata_field",
            &self.target_agnostic_compiler_route_metadata_field,
            TARGET_AGNOSTIC_ROUTE_METADATA_FIELD,
        )?;
        require_string(
            "route_metadata_contract.state_api_route_metadata_field",
            &self.state_api_route_metadata_field,
            STATE_API_ROUTE_METADATA_FIELD,
        )?;
        require_all(
            "route_metadata_contract.target_ids",
            &self.target_ids,
            REQUIRED_TARGETS,
        )?;
        require_all(
            "route_metadata_contract.capabilities",
            &self.capabilities,
            REQUIRED_CAPABILITIES,
        )?;
        require_all(
            "route_metadata_contract.route_evidence",
            &self.route_evidence,
            &required_route_evidence(),
        )?;
        require_all(
            "route_metadata_contract.go_source_instance_records",
            &self.go_source_instance_records,
            GO_SOURCE_INSTANCE_RECORDS,
        )?;
        require_all(
            "route_metadata_contract.csharp_real_source_records",
            &self.csharp_real_source_records,
            REQUIRED_CSHARP_RECORDS,
        )?;
        require_usize(
            "route_metadata_contract.record_query_count",
            self.record_query_count,
            route_owned_record_query_count(integration_gate)?,
        )?;
        require_bool(
            "route_metadata_contract.requires_route_artifact_metadata",
            self.requires_route_artifact_metadata,
            true,
        )?;
        require_bool(
            "route_metadata_contract.requires_integration_query_contract",
            self.requires_integration_query_contract,
            true,
        )?;
        validate_route_artifacts(route_artifacts, integration_gate)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxGeneratedCodecStateApiRouteBpeContract {
    pub one_token_raw_expansion_probe_ready: bool,
    pub multi_token_raw_expansion_runtime_replay_ready: bool,
    pub multi_token_raw_expansion_replay_case_count: usize,
    pub next_generated_codec_state_api_work: String,
    pub bpe_layer_defines_validity: bool,
}

impl SyntaxGeneratedCodecStateApiRouteBpeContract {
    fn validate(
        &self,
        integration_gate: &SyntaxGeneratedCodecStateApiIntegrationGate,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        require_bool(
            "bpe_raw_expansion_contract.one_token_raw_expansion_probe_ready",
            self.one_token_raw_expansion_probe_ready,
            integration_gate
                .bpe_raw_expansion_contract
                .one_token_raw_expansion_probe_ready,
        )?;
        require_bool(
            "bpe_raw_expansion_contract.multi_token_raw_expansion_runtime_replay_ready",
            self.multi_token_raw_expansion_runtime_replay_ready,
            true,
        )?;
        require_bool(
            "bpe_raw_expansion_contract.integration_multi_token_replay_ready",
            integration_gate
                .bpe_raw_expansion_contract
                .multi_token_raw_expansion_runtime_replay_ready,
            true,
        )?;
        require_usize(
            "bpe_raw_expansion_contract.multi_token_raw_expansion_replay_case_count",
            self.multi_token_raw_expansion_replay_case_count,
            multi_token_replay_case_count(integration_gate),
        )?;
        require_positive_usize(
            "bpe_raw_expansion_contract.multi_token_raw_expansion_replay_case_count",
            self.multi_token_raw_expansion_replay_case_count,
        )?;
        require_string(
            "bpe_raw_expansion_contract.next_generated_codec_state_api_work",
            &self.next_generated_codec_state_api_work,
            NEXT_MISSING_STABLE_PROMOTION_CLASS,
        )?;
        require_bool(
            "bpe_raw_expansion_contract.bpe_layer_defines_validity",
            self.bpe_layer_defines_validity,
            false,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxGeneratedCodecStateApiRouteAdmissibleMaskContract {
    pub mask_contract_id: String,
    pub completed_promotion_class: String,
    pub next_missing_stable_promotion_class: String,
    pub target_ids: Vec<String>,
    pub source_query_contract: String,
    pub target_neutral_state_api_authority: String,
    pub record_query_count: usize,
    pub finite_mask_record_count: usize,
    pub payload_range_mask_record_count: usize,
    pub eof_validity_record_count: usize,
    pub bpe_raw_expansion_replay_case_count: usize,
    pub bpe_raw_expansion_accepted_case_count: usize,
    pub bpe_raw_expansion_first_failure_case_count: usize,
    pub requires_first_failure_diagnostics: bool,
    pub bpe_ids_used_as_syntax_validity_authority: bool,
    pub bpe_layer_defines_validity: bool,
    pub source_program_validity_claim_allowed: bool,
    pub stable_backend_promotion_allowed: bool,
}

impl SyntaxGeneratedCodecStateApiRouteAdmissibleMaskContract {
    fn validate(
        &self,
        integration_gate: &SyntaxGeneratedCodecStateApiIntegrationGate,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        let summary = admissible_mask_summary(integration_gate)?;
        require_string(
            "admissible_token_mask_contract.mask_contract_id",
            &self.mask_contract_id,
            ADMISSIBLE_TOKEN_MASK_CONTRACT_ID,
        )?;
        require_string(
            "admissible_token_mask_contract.completed_promotion_class",
            &self.completed_promotion_class,
            SYNTAX_COMPILER_ADMISSIBLE_TOKEN_MASK_ROUTE_COMPLETED_PROMOTION_CLASS,
        )?;
        require_string(
            "admissible_token_mask_contract.next_missing_stable_promotion_class",
            &self.next_missing_stable_promotion_class,
            SYNTAX_COMPILER_ADMISSIBLE_TOKEN_MASK_ROUTE_NEXT_MISSING_STABLE_PROMOTION_CLASS,
        )?;
        require_all(
            "admissible_token_mask_contract.target_ids",
            &self.target_ids,
            REQUIRED_TARGETS,
        )?;
        require_string(
            "admissible_token_mask_contract.source_query_contract",
            &self.source_query_contract,
            ADMISSIBLE_TOKEN_MASK_SOURCE_QUERY_CONTRACT,
        )?;
        require_string(
            "admissible_token_mask_contract.target_neutral_state_api_authority",
            &self.target_neutral_state_api_authority,
            ADMISSIBLE_TOKEN_MASK_VALIDITY_AUTHORITY,
        )?;
        require_usize(
            "admissible_token_mask_contract.record_query_count",
            self.record_query_count,
            summary.record_query_count,
        )?;
        require_usize(
            "admissible_token_mask_contract.finite_mask_record_count",
            self.finite_mask_record_count,
            summary.finite_mask_record_count,
        )?;
        require_usize(
            "admissible_token_mask_contract.payload_range_mask_record_count",
            self.payload_range_mask_record_count,
            summary.payload_range_mask_record_count,
        )?;
        require_usize(
            "admissible_token_mask_contract.eof_validity_record_count",
            self.eof_validity_record_count,
            summary.eof_validity_record_count,
        )?;
        require_usize(
            "admissible_token_mask_contract.bpe_raw_expansion_replay_case_count",
            self.bpe_raw_expansion_replay_case_count,
            summary.bpe_raw_expansion_replay_case_count,
        )?;
        require_usize(
            "admissible_token_mask_contract.bpe_raw_expansion_accepted_case_count",
            self.bpe_raw_expansion_accepted_case_count,
            summary.bpe_raw_expansion_accepted_case_count,
        )?;
        require_usize(
            "admissible_token_mask_contract.bpe_raw_expansion_first_failure_case_count",
            self.bpe_raw_expansion_first_failure_case_count,
            summary.bpe_raw_expansion_first_failure_case_count,
        )?;
        require_positive_usize(
            "admissible_token_mask_contract.bpe_raw_expansion_first_failure_case_count",
            self.bpe_raw_expansion_first_failure_case_count,
        )?;
        require_bool(
            "admissible_token_mask_contract.requires_first_failure_diagnostics",
            self.requires_first_failure_diagnostics,
            true,
        )?;
        require_bool(
            "admissible_token_mask_contract.bpe_ids_used_as_syntax_validity_authority",
            self.bpe_ids_used_as_syntax_validity_authority,
            false,
        )?;
        require_bool(
            "admissible_token_mask_contract.bpe_layer_defines_validity",
            self.bpe_layer_defines_validity,
            false,
        )?;
        require_bool(
            "admissible_token_mask_contract.source_program_validity_claim_allowed",
            self.source_program_validity_claim_allowed,
            false,
        )?;
        require_bool(
            "admissible_token_mask_contract.stable_backend_promotion_allowed",
            self.stable_backend_promotion_allowed,
            false,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxGeneratedCodecStateApiRoutePrefixStateContinuationContract {
    pub continuation_contract_id: String,
    pub completed_promotion_class: String,
    pub next_missing_stable_promotion_class: String,
    pub source_admissible_token_mask_contract_id: String,
    pub source_admissible_token_mask_completed_promotion_class: String,
    pub target_ids: Vec<String>,
    pub source_query_contract: String,
    pub target_neutral_state_api_authority: String,
    pub request_fields: Vec<String>,
    pub response_fields: Vec<String>,
    pub record_query_count: usize,
    pub raw_prefix_request_count: usize,
    pub finite_admissible_response_record_count: usize,
    pub payload_range_response_record_count: usize,
    pub eof_validity_response_record_count: usize,
    pub bpe_raw_expansion_replay_case_count: usize,
    pub bpe_raw_expansion_first_failure_case_count: usize,
    pub response_evidence_fixture: String,
    pub response_evidence_record_count: usize,
    pub response_evidence_raw_prefix_response_count: usize,
    pub response_evidence_bpe_replay_response_count: usize,
    pub response_evidence_checked_against_target_neutral_api: bool,
    pub consumes_route_owned_admissible_mask_contract: bool,
    pub requires_consumed_raw_token_count: bool,
    pub requires_prefix_status: bool,
    pub requires_eof_validity: bool,
    pub requires_first_failure_diagnostics: bool,
    pub bpe_ids_used_as_syntax_validity_authority: bool,
    pub bpe_layer_defines_validity: bool,
    pub source_program_validity_claim_allowed: bool,
    pub stable_backend_promotion_allowed: bool,
    pub response_evidence_records:
        Vec<SyntaxGeneratedCodecStateApiRoutePrefixStateContinuationRecordEvidence>,
}

impl SyntaxGeneratedCodecStateApiRoutePrefixStateContinuationContract {
    fn validate(
        &self,
        integration_gate: &SyntaxGeneratedCodecStateApiIntegrationGate,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        let summary = prefix_state_continuation_summary(integration_gate)?;
        require_string(
            "prefix_state_continuation_contract.continuation_contract_id",
            &self.continuation_contract_id,
            PREFIX_STATE_CONTINUATION_CONTRACT_ID,
        )?;
        require_string(
            "prefix_state_continuation_contract.completed_promotion_class",
            &self.completed_promotion_class,
            SYNTAX_COMPILER_PREFIX_STATE_CONTINUATION_ROUTE_COMPLETED_PROMOTION_CLASS,
        )?;
        require_string(
            "prefix_state_continuation_contract.next_missing_stable_promotion_class",
            &self.next_missing_stable_promotion_class,
            SYNTAX_COMPILER_PREFIX_STATE_CONTINUATION_ROUTE_NEXT_MISSING_STABLE_PROMOTION_CLASS,
        )?;
        require_string(
            "prefix_state_continuation_contract.source_admissible_token_mask_contract_id",
            &self.source_admissible_token_mask_contract_id,
            ADMISSIBLE_TOKEN_MASK_CONTRACT_ID,
        )?;
        require_string(
            "prefix_state_continuation_contract.source_admissible_token_mask_completed_promotion_class",
            &self.source_admissible_token_mask_completed_promotion_class,
            SYNTAX_COMPILER_ADMISSIBLE_TOKEN_MASK_ROUTE_COMPLETED_PROMOTION_CLASS,
        )?;
        require_all(
            "prefix_state_continuation_contract.target_ids",
            &self.target_ids,
            REQUIRED_TARGETS,
        )?;
        require_string(
            "prefix_state_continuation_contract.source_query_contract",
            &self.source_query_contract,
            ADMISSIBLE_TOKEN_MASK_SOURCE_QUERY_CONTRACT,
        )?;
        require_string(
            "prefix_state_continuation_contract.target_neutral_state_api_authority",
            &self.target_neutral_state_api_authority,
            ADMISSIBLE_TOKEN_MASK_VALIDITY_AUTHORITY,
        )?;
        require_all(
            "prefix_state_continuation_contract.request_fields",
            &self.request_fields,
            REQUIRED_PREFIX_STATE_CONTINUATION_REQUEST_FIELDS,
        )?;
        require_all(
            "prefix_state_continuation_contract.response_fields",
            &self.response_fields,
            REQUIRED_PREFIX_STATE_CONTINUATION_RESPONSE_FIELDS,
        )?;
        require_usize(
            "prefix_state_continuation_contract.record_query_count",
            self.record_query_count,
            summary.record_query_count,
        )?;
        require_usize(
            "prefix_state_continuation_contract.raw_prefix_request_count",
            self.raw_prefix_request_count,
            summary.raw_prefix_request_count,
        )?;
        require_usize(
            "prefix_state_continuation_contract.finite_admissible_response_record_count",
            self.finite_admissible_response_record_count,
            summary.finite_admissible_response_record_count,
        )?;
        require_usize(
            "prefix_state_continuation_contract.payload_range_response_record_count",
            self.payload_range_response_record_count,
            summary.payload_range_response_record_count,
        )?;
        require_usize(
            "prefix_state_continuation_contract.eof_validity_response_record_count",
            self.eof_validity_response_record_count,
            summary.eof_validity_response_record_count,
        )?;
        require_usize(
            "prefix_state_continuation_contract.bpe_raw_expansion_replay_case_count",
            self.bpe_raw_expansion_replay_case_count,
            summary.bpe_raw_expansion_replay_case_count,
        )?;
        require_usize(
            "prefix_state_continuation_contract.bpe_raw_expansion_first_failure_case_count",
            self.bpe_raw_expansion_first_failure_case_count,
            summary.bpe_raw_expansion_first_failure_case_count,
        )?;
        require_string(
            "prefix_state_continuation_contract.response_evidence_fixture",
            &self.response_evidence_fixture,
            SYNTAX_COMPILER_PREFIX_STATE_CONTINUATION_ROUTE_RESPONSE_EVIDENCE_FIXTURE,
        )?;
        require_usize(
            "prefix_state_continuation_contract.response_evidence_record_count",
            self.response_evidence_record_count,
            summary.record_query_count,
        )?;
        require_usize(
            "prefix_state_continuation_contract.response_evidence_raw_prefix_response_count",
            self.response_evidence_raw_prefix_response_count,
            summary.raw_prefix_request_count,
        )?;
        require_usize(
            "prefix_state_continuation_contract.response_evidence_bpe_replay_response_count",
            self.response_evidence_bpe_replay_response_count,
            summary.bpe_raw_expansion_replay_case_count,
        )?;
        require_bool(
            "prefix_state_continuation_contract.response_evidence_checked_against_target_neutral_api",
            self.response_evidence_checked_against_target_neutral_api,
            true,
        )?;
        for (field, actual) in [
            (
                "prefix_state_continuation_contract.consumes_route_owned_admissible_mask_contract",
                self.consumes_route_owned_admissible_mask_contract,
            ),
            (
                "prefix_state_continuation_contract.requires_consumed_raw_token_count",
                self.requires_consumed_raw_token_count,
            ),
            (
                "prefix_state_continuation_contract.requires_prefix_status",
                self.requires_prefix_status,
            ),
            (
                "prefix_state_continuation_contract.requires_eof_validity",
                self.requires_eof_validity,
            ),
            (
                "prefix_state_continuation_contract.requires_first_failure_diagnostics",
                self.requires_first_failure_diagnostics,
            ),
        ] {
            require_bool(field, actual, true)?;
        }
        for (field, actual) in [
            (
                "prefix_state_continuation_contract.bpe_ids_used_as_syntax_validity_authority",
                self.bpe_ids_used_as_syntax_validity_authority,
            ),
            (
                "prefix_state_continuation_contract.bpe_layer_defines_validity",
                self.bpe_layer_defines_validity,
            ),
            (
                "prefix_state_continuation_contract.source_program_validity_claim_allowed",
                self.source_program_validity_claim_allowed,
            ),
            (
                "prefix_state_continuation_contract.stable_backend_promotion_allowed",
                self.stable_backend_promotion_allowed,
            ),
        ] {
            require_bool(field, actual, false)?;
        }
        self.validate_response_evidence_shape(integration_gate, &summary)?;
        Ok(())
    }

    fn validate_response_evidence_shape(
        &self,
        integration_gate: &SyntaxGeneratedCodecStateApiIntegrationGate,
        summary: &PrefixStateContinuationSummary,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        require_usize(
            "prefix_state_continuation_contract.response_evidence_records",
            self.response_evidence_records.len(),
            summary.record_query_count,
        )?;
        let mut observed_keys = BTreeSet::new();
        let mut response_count = 0usize;
        let mut finite_response_count = 0usize;
        let mut payload_range_response_count = 0usize;
        let mut eof_valid_response_count = 0usize;
        let mut bpe_replay_response_count = 0usize;
        let mut bpe_first_failure_response_count = 0usize;
        for record in &self.response_evidence_records {
            record.validate_shape()?;
            if !observed_keys.insert((record.target.as_str(), record.record_id.as_str())) {
                return Err(SyntaxBackendBoundaryError::new(
                    "prefix_state_continuation_contract.response_evidence_records",
                    format!(
                        "duplicate response-evidence record {}:{}",
                        record.target, record.record_id
                    ),
                ));
            }
            let query = integration_gate
                .record_queries
                .iter()
                .find(|query| query.target == record.target && query.record_id == record.record_id)
                .ok_or_else(|| {
                    SyntaxBackendBoundaryError::new(
                        "prefix_state_continuation_contract.response_evidence_records",
                        format!(
                            "response-evidence record {}:{} is not in integration queries",
                            record.target, record.record_id
                        ),
                    )
                })?;
            require_usize(
                "prefix_state_continuation_contract.response_evidence_records.request_count",
                record.request_count,
                query.prefix_state_case_ids.len(),
            )?;
            require_usize(
                "prefix_state_continuation_contract.response_evidence_records.response_count",
                record.response_count,
                query.prefix_state_case_ids.len(),
            )?;
            require_usize(
                "prefix_state_continuation_contract.response_evidence_records.responses",
                record.responses.len(),
                query.prefix_state_case_ids.len(),
            )?;
            require_usize(
                "prefix_state_continuation_contract.response_evidence_records.bpe_raw_expansion_responses",
                record.bpe_raw_expansion_responses.len(),
                query.bpe_raw_expansion_replay_cases.len(),
            )?;
            require_all(
                "prefix_state_continuation_contract.response_evidence_records.prefix_state_case_ids",
                &record
                    .responses
                    .iter()
                    .map(|response| response.prefix_state_case_id.clone())
                    .collect::<Vec<_>>(),
                &query
                    .prefix_state_case_ids
                    .iter()
                    .map(String::as_str)
                    .collect::<Vec<_>>(),
            )?;
            response_count += record.responses.len();
            if record.finite_admissible_response_count > 0 {
                finite_response_count += 1;
            }
            if record.payload_range_response_count > 0 {
                payload_range_response_count += 1;
            }
            if record.eof_validity_response_count > 0 {
                eof_valid_response_count += 1;
            }
            bpe_replay_response_count += record.bpe_raw_expansion_replay_case_count;
            bpe_first_failure_response_count += record.bpe_raw_expansion_first_failure_case_count;
        }
        require_usize(
            "prefix_state_continuation_contract.response_evidence_records.response_count",
            response_count,
            summary.raw_prefix_request_count,
        )?;
        require_usize(
            "prefix_state_continuation_contract.response_evidence_records.finite_admissible_response_count",
            finite_response_count,
            summary.finite_admissible_response_record_count,
        )?;
        require_usize(
            "prefix_state_continuation_contract.response_evidence_records.payload_range_response_count",
            payload_range_response_count,
            summary.payload_range_response_record_count,
        )?;
        require_usize(
            "prefix_state_continuation_contract.response_evidence_records.eof_validity_response_count",
            eof_valid_response_count,
            summary.eof_validity_response_record_count,
        )?;
        require_usize(
            "prefix_state_continuation_contract.response_evidence_records.bpe_raw_expansion_replay_case_count",
            bpe_replay_response_count,
            summary.bpe_raw_expansion_replay_case_count,
        )?;
        require_usize(
            "prefix_state_continuation_contract.response_evidence_records.bpe_raw_expansion_first_failure_case_count",
            bpe_first_failure_response_count,
            summary.bpe_raw_expansion_first_failure_case_count,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxGeneratedCodecStateApiRouteMatchedPresenceMaskContract {
    pub route_evidence_id: String,
    pub source_scope: String,
    pub source_contract: String,
    pub syntax_level_boundary: String,
    pub target_ids: Vec<String>,
    pub matched_record_count: usize,
    pub finite_mask_record_count: usize,
    pub payload_range_mask_record_count: usize,
    pub invalid_payload_rejection_count: usize,
    pub response_evidence_checked_against_target_neutral_api: bool,
    pub route_owned_evidence: bool,
    pub records: Vec<SyntaxGeneratedCodecStateApiRouteMatchedPresenceMaskRecord>,
    pub validity_authority: String,
    pub arena_ir_used: bool,
    pub bpe_layer_defines_validity: bool,
    pub source_instance_segment_sidecars_used: bool,
    pub downstream_artifact_ladder_used: bool,
}

impl SyntaxGeneratedCodecStateApiRouteMatchedPresenceMaskContract {
    fn validate_shape(&self) -> Result<(), SyntaxBackendBoundaryError> {
        require_string(
            "matched_presence_mask_contract.route_evidence_id",
            &self.route_evidence_id,
            MATCHED_PRESENCE_MASK_ROUTE_EVIDENCE_ID,
        )?;
        require_string(
            "matched_presence_mask_contract.source_scope",
            &self.source_scope,
            MATCHED_PRESENCE_MASK_SCOPE,
        )?;
        require_string(
            "matched_presence_mask_contract.source_contract",
            &self.source_contract,
            MATCHED_PRESENCE_MASK_SOURCE_CONTRACT,
        )?;
        require_string(
            "matched_presence_mask_contract.syntax_level_boundary",
            &self.syntax_level_boundary,
            "parser-backed metadata -> GrammarIR -> StructuralModel -> token layout -> generated codec",
        )?;
        require_all(
            "matched_presence_mask_contract.target_ids",
            &self.target_ids,
            REQUIRED_TARGETS,
        )?;
        require_usize(
            "matched_presence_mask_contract.matched_record_count",
            self.matched_record_count,
            self.records.len(),
        )?;
        require_usize(
            "matched_presence_mask_contract.matched_record_count",
            self.matched_record_count,
            2,
        )?;
        require_usize(
            "matched_presence_mask_contract.finite_mask_record_count",
            self.finite_mask_record_count,
            self.records
                .iter()
                .filter(|record| !record.finite_admissible_raw_token_ids.is_empty())
                .count(),
        )?;
        require_usize(
            "matched_presence_mask_contract.payload_range_mask_record_count",
            self.payload_range_mask_record_count,
            self.records
                .iter()
                .filter(|record| !record.payload_token_ranges.is_empty())
                .count(),
        )?;
        require_usize(
            "matched_presence_mask_contract.invalid_payload_rejection_count",
            self.invalid_payload_rejection_count,
            self.records
                .iter()
                .filter(|record| record.invalid_payload_rejected)
                .count(),
        )?;
        require_bool(
            "matched_presence_mask_contract.response_evidence_checked_against_target_neutral_api",
            self.response_evidence_checked_against_target_neutral_api,
            true,
        )?;
        require_bool(
            "matched_presence_mask_contract.route_owned_evidence",
            self.route_owned_evidence,
            true,
        )?;
        let mut observed_targets = BTreeSet::new();
        for record in &self.records {
            record.validate_shape()?;
            observed_targets.insert(record.target.as_str());
        }
        require_all(
            "matched_presence_mask_contract.records.targets",
            &observed_targets
                .iter()
                .copied()
                .map(str::to_string)
                .collect::<Vec<_>>(),
            REQUIRED_TARGETS,
        )?;
        require_string(
            "matched_presence_mask_contract.validity_authority",
            &self.validity_authority,
            TARGET_NEUTRAL_RAW_STATE_API_AUTHORITY,
        )?;
        for (field, actual) in [
            (
                "matched_presence_mask_contract.arena_ir_used",
                self.arena_ir_used,
            ),
            (
                "matched_presence_mask_contract.bpe_layer_defines_validity",
                self.bpe_layer_defines_validity,
            ),
            (
                "matched_presence_mask_contract.source_instance_segment_sidecars_used",
                self.source_instance_segment_sidecars_used,
            ),
            (
                "matched_presence_mask_contract.downstream_artifact_ladder_used",
                self.downstream_artifact_ladder_used,
            ),
        ] {
            require_bool(field, actual, false)?;
        }
        Ok(())
    }

    fn validate_against_matched_contract(
        &self,
        matched_contract: &TargetNeutralMatchedPresenceMaskContract,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        self.validate_shape()?;
        require_string(
            "matched_presence_mask_contract.source_scope",
            &self.source_scope,
            &matched_contract.scope,
        )?;
        require_string(
            "matched_presence_mask_contract.syntax_level_boundary",
            &self.syntax_level_boundary,
            &matched_contract.syntax_level_boundary,
        )?;
        require_usize(
            "matched_presence_mask_contract.matched_record_count",
            self.matched_record_count,
            matched_contract.matched_record_count,
        )?;
        require_usize(
            "matched_presence_mask_contract.finite_mask_record_count",
            self.finite_mask_record_count,
            matched_contract.finite_mask_record_count,
        )?;
        require_usize(
            "matched_presence_mask_contract.payload_range_mask_record_count",
            self.payload_range_mask_record_count,
            matched_contract.payload_range_mask_record_count,
        )?;
        require_usize(
            "matched_presence_mask_contract.invalid_payload_rejection_count",
            self.invalid_payload_rejection_count,
            matched_contract.invalid_payload_rejection_count,
        )?;
        require_string(
            "matched_presence_mask_contract.validity_authority",
            &self.validity_authority,
            &matched_contract.validity_authority,
        )?;
        require_bool(
            "matched_presence_mask_contract.target_neutral_state_api_consumed",
            matched_contract.target_neutral_state_api_consumed,
            true,
        )?;
        for (field, actual, expected) in [
            (
                "matched_presence_mask_contract.arena_ir_used",
                self.arena_ir_used,
                matched_contract.arena_ir_used,
            ),
            (
                "matched_presence_mask_contract.bpe_layer_defines_validity",
                self.bpe_layer_defines_validity,
                matched_contract.bpe_layer_defines_validity,
            ),
            (
                "matched_presence_mask_contract.source_instance_segment_sidecars_used",
                self.source_instance_segment_sidecars_used,
                matched_contract.source_instance_segment_sidecars_used,
            ),
            (
                "matched_presence_mask_contract.downstream_artifact_ladder_used",
                self.downstream_artifact_ladder_used,
                matched_contract.downstream_artifact_ladder_used,
            ),
        ] {
            require_bool(field, actual, expected)?;
        }
        for expected in &matched_contract.records {
            let actual = self
                .records
                .iter()
                .find(|record| {
                    record.target == expected.target && record.record_id == expected.record_id
                })
                .ok_or_else(|| {
                    SyntaxBackendBoundaryError::new(
                        "matched_presence_mask_contract.records",
                        format!(
                            "missing matched route evidence record {}:{}",
                            expected.target, expected.record_id
                        ),
                    )
                })?;
            actual.validate_against_matched_record(expected)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxGeneratedCodecStateApiRouteMatchedPresenceMaskRecord {
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

impl SyntaxGeneratedCodecStateApiRouteMatchedPresenceMaskRecord {
    fn validate_shape(&self) -> Result<(), SyntaxBackendBoundaryError> {
        require_non_empty("matched_presence_mask_record.target", &self.target)?;
        require_non_empty("matched_presence_mask_record.record_id", &self.record_id)?;
        require_non_empty("matched_presence_mask_record.owner_rule", &self.owner_rule)?;
        require_non_empty(
            "matched_presence_mask_record.owner_field",
            &self.owner_field,
        )?;
        require_non_empty(
            "matched_presence_mask_record.selected_node_kind",
            &self.selected_node_kind,
        )?;
        require_non_empty(
            "matched_presence_mask_record.prefix_state_api_case",
            &self.prefix_state_api_case,
        )?;
        require_positive_usize(
            "matched_presence_mask_record.finite_admissible_raw_token_ids",
            self.finite_admissible_raw_token_ids.len(),
        )?;
        require_bool(
            "matched_presence_mask_record.invalid_payload_rejected",
            self.invalid_payload_rejected,
            true,
        )?;
        require_string(
            "matched_presence_mask_record.invalid_payload_error_kind",
            &self.invalid_payload_error_kind,
            "invalid_payload_token",
        )?;
        require_string(
            "matched_presence_mask_record.validity_authority",
            &self.validity_authority,
            TARGET_NEUTRAL_RAW_STATE_API_AUTHORITY,
        )
    }

    fn validate_against_matched_record(
        &self,
        expected: &crate::syntax_generated_codec_state_api_integration::TargetNeutralPresenceMaskRecord,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        self.validate_shape()?;
        require_string(
            "matched_presence_mask_record.target",
            &self.target,
            &expected.target,
        )?;
        require_string(
            "matched_presence_mask_record.record_id",
            &self.record_id,
            &expected.record_id,
        )?;
        require_string(
            "matched_presence_mask_record.owner_rule",
            &self.owner_rule,
            &expected.owner_rule,
        )?;
        require_string(
            "matched_presence_mask_record.owner_field",
            &self.owner_field,
            &expected.owner_field,
        )?;
        require_string(
            "matched_presence_mask_record.selected_node_kind",
            &self.selected_node_kind,
            &expected.selected_node_kind,
        )?;
        require_string(
            "matched_presence_mask_record.prefix_state_api_case",
            &self.prefix_state_api_case,
            &expected.prefix_state_api_case,
        )?;
        require_u32_slice(
            "matched_presence_mask_record.finite_admissible_raw_token_ids",
            &self.finite_admissible_raw_token_ids,
            &expected.finite_admissible_raw_token_ids,
        )?;
        if self.payload_token_ranges != expected.payload_token_ranges {
            return Err(SyntaxBackendBoundaryError::new(
                "matched_presence_mask_record.payload_token_ranges",
                "matched route evidence payload ranges differ from target-neutral replay contract",
            ));
        }
        require_bool(
            "matched_presence_mask_record.payload_end_admissible",
            self.payload_end_admissible,
            expected.payload_end_admissible,
        )?;
        require_bool(
            "matched_presence_mask_record.invalid_payload_rejected",
            self.invalid_payload_rejected,
            expected.invalid_payload_rejected,
        )?;
        require_string(
            "matched_presence_mask_record.invalid_payload_error_kind",
            &self.invalid_payload_error_kind,
            &expected.invalid_payload_error_kind,
        )?;
        require_string(
            "matched_presence_mask_record.validity_authority",
            &self.validity_authority,
            &expected.validity_authority,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxGeneratedCodecStateApiRoutePrefixStateContinuationRecordEvidence {
    pub target: String,
    pub record_id: String,
    pub raw_token_count: usize,
    pub raw_token_ids_sha256: String,
    pub request_count: usize,
    pub response_count: usize,
    pub responses: Vec<SyntaxGeneratedCodecStateApiRoutePrefixStateContinuationResponseEvidence>,
    pub finite_admissible_response_count: usize,
    pub payload_range_response_count: usize,
    pub eof_validity_response_count: usize,
    pub bpe_raw_expansion_responses:
        Vec<SyntaxGeneratedCodecStateApiRoutePrefixStateContinuationBpeResponseEvidence>,
    pub bpe_raw_expansion_replay_case_count: usize,
    pub bpe_raw_expansion_accepted_case_count: usize,
    pub bpe_raw_expansion_first_failure_case_count: usize,
    pub bpe_first_failure_diagnostic_count: usize,
    pub validity_authority: String,
    pub bpe_layer_defines_validity: bool,
    pub bpe_ids_used_as_syntax_validity_authority: bool,
}

impl SyntaxGeneratedCodecStateApiRoutePrefixStateContinuationRecordEvidence {
    fn validate_shape(&self) -> Result<(), SyntaxBackendBoundaryError> {
        require_non_empty(
            "prefix_state_continuation_response_evidence.target",
            &self.target,
        )?;
        require_non_empty(
            "prefix_state_continuation_response_evidence.record_id",
            &self.record_id,
        )?;
        require_positive_usize(
            "prefix_state_continuation_response_evidence.raw_token_count",
            self.raw_token_count,
        )?;
        require_non_empty(
            "prefix_state_continuation_response_evidence.raw_token_ids_sha256",
            &self.raw_token_ids_sha256,
        )?;
        require_usize(
            "prefix_state_continuation_response_evidence.response_count",
            self.response_count,
            self.responses.len(),
        )?;
        require_usize(
            "prefix_state_continuation_response_evidence.request_count",
            self.request_count,
            self.responses.len(),
        )?;
        let finite_count = self
            .responses
            .iter()
            .filter(|response| !response.finite_admissible_raw_token_ids.is_empty())
            .count();
        let payload_count = self
            .responses
            .iter()
            .filter(|response| !response.payload_token_ranges.is_empty())
            .count();
        let eof_count = self
            .responses
            .iter()
            .filter(|response| response.eof_valid)
            .count();
        require_usize(
            "prefix_state_continuation_response_evidence.finite_admissible_response_count",
            self.finite_admissible_response_count,
            finite_count,
        )?;
        require_usize(
            "prefix_state_continuation_response_evidence.payload_range_response_count",
            self.payload_range_response_count,
            payload_count,
        )?;
        require_usize(
            "prefix_state_continuation_response_evidence.eof_validity_response_count",
            self.eof_validity_response_count,
            eof_count,
        )?;
        for response in &self.responses {
            response.validate_shape()?;
        }
        for response in &self.bpe_raw_expansion_responses {
            response.validate_shape()?;
        }
        require_usize(
            "prefix_state_continuation_response_evidence.bpe_raw_expansion_replay_case_count",
            self.bpe_raw_expansion_replay_case_count,
            self.bpe_raw_expansion_responses.len(),
        )?;
        require_usize(
            "prefix_state_continuation_response_evidence.bpe_raw_expansion_accepted_case_count",
            self.bpe_raw_expansion_accepted_case_count,
            self.bpe_raw_expansion_responses
                .iter()
                .filter(|response| response.accepted)
                .count(),
        )?;
        let first_failure_count = self
            .bpe_raw_expansion_responses
            .iter()
            .filter(|response| !response.accepted)
            .count();
        require_usize(
            "prefix_state_continuation_response_evidence.bpe_raw_expansion_first_failure_case_count",
            self.bpe_raw_expansion_first_failure_case_count,
            first_failure_count,
        )?;
        require_usize(
            "prefix_state_continuation_response_evidence.bpe_first_failure_diagnostic_count",
            self.bpe_first_failure_diagnostic_count,
            self.bpe_raw_expansion_responses
                .iter()
                .filter(|response| {
                    response
                        .bpe_raw_expansion_first_failure_diagnostic
                        .as_deref()
                        .is_some_and(|diagnostic| !diagnostic.is_empty())
                })
                .count(),
        )?;
        require_string(
            "prefix_state_continuation_response_evidence.validity_authority",
            &self.validity_authority,
            ADMISSIBLE_TOKEN_MASK_VALIDITY_AUTHORITY,
        )?;
        require_bool(
            "prefix_state_continuation_response_evidence.bpe_layer_defines_validity",
            self.bpe_layer_defines_validity,
            false,
        )?;
        require_bool(
            "prefix_state_continuation_response_evidence.bpe_ids_used_as_syntax_validity_authority",
            self.bpe_ids_used_as_syntax_validity_authority,
            false,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxGeneratedCodecStateApiRoutePrefixStateContinuationResponseEvidence {
    pub target: String,
    pub record_id: String,
    pub prefix_state_case_id: String,
    pub raw_prefix_token_count: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub raw_prefix_token_ids: Option<Vec<u32>>,
    pub raw_prefix_token_ids_sha256: String,
    pub consumed_raw_tokens: usize,
    pub prefix_status: String,
    pub eof_valid: bool,
    pub active_rule: Option<String>,
    pub active_field: Option<String>,
    pub finite_admissible_raw_token_ids: Vec<u32>,
    pub payload_token_ranges: Vec<ProjectionTokenLayoutRangeSummary>,
    pub payload_end_admissible: bool,
}

impl SyntaxGeneratedCodecStateApiRoutePrefixStateContinuationResponseEvidence {
    fn validate_shape(&self) -> Result<(), SyntaxBackendBoundaryError> {
        require_non_empty("prefix_state_continuation_response.target", &self.target)?;
        require_non_empty(
            "prefix_state_continuation_response.record_id",
            &self.record_id,
        )?;
        require_non_empty(
            "prefix_state_continuation_response.prefix_state_case_id",
            &self.prefix_state_case_id,
        )?;
        require_usize(
            "prefix_state_continuation_response.raw_prefix_token_count",
            self.raw_prefix_token_count,
            self.consumed_raw_tokens,
        )?;
        require_non_empty(
            "prefix_state_continuation_response.raw_prefix_token_ids_sha256",
            &self.raw_prefix_token_ids_sha256,
        )?;
        if self.finite_admissible_raw_token_ids.is_empty() && self.payload_token_ranges.is_empty() {
            require_bool(
                "prefix_state_continuation_response.raw_prefix_token_ids_required",
                self.raw_prefix_token_ids.is_none(),
                true,
            )?;
        } else {
            let raw_prefix_token_ids = self.raw_prefix_token_ids.as_ref().ok_or_else(|| {
                SyntaxBackendBoundaryError::new(
                    "prefix_state_continuation_response.raw_prefix_token_ids",
                    "candidate-bearing prefix responses must preserve raw prefix token IDs for stepwise transition evidence",
                )
            })?;
            require_usize(
                "prefix_state_continuation_response.raw_prefix_token_ids",
                raw_prefix_token_ids.len(),
                self.raw_prefix_token_count,
            )?;
            require_json_hash(
                "prefix_state_continuation_response.raw_prefix_token_ids_sha256",
                raw_prefix_token_ids,
                &self.raw_prefix_token_ids_sha256,
            )?;
        }
        require_non_empty(
            "prefix_state_continuation_response.prefix_status",
            &self.prefix_status,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxGeneratedCodecStateApiRoutePrefixStateContinuationBpeResponseEvidence {
    pub target: String,
    pub record_id: String,
    pub case_id: String,
    pub expanded_raw_token_count: usize,
    pub accepted_raw_token_count: usize,
    pub accepted: bool,
    pub final_prefix_status: String,
    pub final_eof_valid: bool,
    pub first_failure_transition_case_id: Option<String>,
    pub first_failure_error_kind: Option<String>,
    pub bpe_raw_expansion_first_failure_diagnostic: Option<String>,
}

impl SyntaxGeneratedCodecStateApiRoutePrefixStateContinuationBpeResponseEvidence {
    fn validate_shape(&self) -> Result<(), SyntaxBackendBoundaryError> {
        require_non_empty(
            "prefix_state_continuation_bpe_response.target",
            &self.target,
        )?;
        require_non_empty(
            "prefix_state_continuation_bpe_response.record_id",
            &self.record_id,
        )?;
        require_non_empty(
            "prefix_state_continuation_bpe_response.case_id",
            &self.case_id,
        )?;
        require_positive_usize(
            "prefix_state_continuation_bpe_response.expanded_raw_token_count",
            self.expanded_raw_token_count,
        )?;
        require_non_empty(
            "prefix_state_continuation_bpe_response.final_prefix_status",
            &self.final_prefix_status,
        )?;
        if self.accepted {
            require_optional_string(
                "prefix_state_continuation_bpe_response.first_failure_transition_case_id",
                self.first_failure_transition_case_id.as_deref(),
                None,
            )?;
            require_optional_string(
                "prefix_state_continuation_bpe_response.first_failure_error_kind",
                self.first_failure_error_kind.as_deref(),
                None,
            )?;
            require_optional_string(
                "prefix_state_continuation_bpe_response.first_failure_diagnostic",
                self.bpe_raw_expansion_first_failure_diagnostic.as_deref(),
                None,
            )
        } else {
            require_non_empty(
                "prefix_state_continuation_bpe_response.first_failure_transition_case_id",
                self.first_failure_transition_case_id
                    .as_deref()
                    .unwrap_or_default(),
            )?;
            require_non_empty(
                "prefix_state_continuation_bpe_response.first_failure_error_kind",
                self.first_failure_error_kind.as_deref().unwrap_or_default(),
            )?;
            require_non_empty(
                "prefix_state_continuation_bpe_response.first_failure_diagnostic",
                self.bpe_raw_expansion_first_failure_diagnostic
                    .as_deref()
                    .unwrap_or_default(),
            )
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct SyntaxGeneratedCodecStateApiRouteRawTokenStream<'a> {
    pub target: &'a str,
    pub record_id: &'a str,
    pub raw_token_ids: &'a [u32],
    pub raw_token_ids_sha256: &'a str,
}

pub fn build_prefix_state_continuation_response_evidence(
    integration_gate: &SyntaxGeneratedCodecStateApiIntegrationGate,
    continuation_results: &[TargetNeutralPrefixStateContinuationResult],
    raw_token_streams: &[SyntaxGeneratedCodecStateApiRouteRawTokenStream<'_>],
) -> Result<
    Vec<SyntaxGeneratedCodecStateApiRoutePrefixStateContinuationRecordEvidence>,
    SyntaxBackendBoundaryError,
> {
    route_owned_record_queries(integration_gate)
        .into_iter()
        .map(|query| {
            let result = continuation_results
                .iter()
                .find(|result| result.target == query.target && result.record_id == query.record_id)
                .ok_or_else(|| {
                    SyntaxBackendBoundaryError::new(
                        "prefix_state_continuation_response_evidence.result",
                        format!(
                            "missing continuation result for {}:{}",
                            query.target, query.record_id
                        ),
                    )
                })?;
            let raw_stream = raw_token_streams
                .iter()
                .find(|stream| stream.target == query.target && stream.record_id == query.record_id)
                .ok_or_else(|| {
                    SyntaxBackendBoundaryError::new(
                        "prefix_state_continuation_response_evidence.raw_token_stream",
                        format!(
                            "missing raw token stream for {}:{}",
                            query.target, query.record_id
                        ),
                    )
                })?;
            build_prefix_state_continuation_record_evidence(query, result, *raw_stream)
        })
        .collect()
}

pub fn validate_prefix_state_continuation_response_evidence(
    route_gate: &SyntaxGeneratedCodecStateApiRouteGate,
    integration_gate: &SyntaxGeneratedCodecStateApiIntegrationGate,
    continuation_results: &[TargetNeutralPrefixStateContinuationResult],
    raw_token_streams: &[SyntaxGeneratedCodecStateApiRouteRawTokenStream<'_>],
) -> Result<(), SyntaxBackendBoundaryError> {
    let expected = build_prefix_state_continuation_response_evidence(
        integration_gate,
        continuation_results,
        raw_token_streams,
    )?;
    if route_gate
        .prefix_state_continuation_contract
        .response_evidence_records
        == expected
    {
        Ok(())
    } else {
        Err(SyntaxBackendBoundaryError::new(
            "prefix_state_continuation_contract.response_evidence_records",
            "checked route-owned prefix-state continuation responses differ from target-neutral API evidence",
        ))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxGeneratedCodecPrefixStateLiveApiMatrixGate {
    pub artifact_kind: String,
    pub version: u32,
    pub tool: String,
    pub status: String,
    pub source_route_gate: SyntaxGeneratedCodecPrefixStateLiveApiMatrixSourceRouteGate,
    pub live_api_contract: SyntaxGeneratedCodecPrefixStateLiveApiMatrixContract,
    pub records: Vec<SyntaxGeneratedCodecPrefixStateLiveApiMatrixRecord>,
    pub claim_boundary: SyntaxGeneratedCodecPrefixStateLiveApiMatrixClaimBoundary,
    pub validation_commands: Vec<String>,
    pub completed_promotion_class: String,
    pub next_missing_stable_promotion_class: String,
}

impl SyntaxGeneratedCodecPrefixStateLiveApiMatrixGate {
    pub fn validate_against_route_gate(
        &self,
        route_gate: &SyntaxGeneratedCodecStateApiRouteGate,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        require_string(
            "artifact_kind",
            &self.artifact_kind,
            LIVE_API_MATRIX_ARTIFACT_KIND,
        )?;
        require_u32("version", self.version, VERSION)?;
        require_string("tool", &self.tool, LIVE_API_MATRIX_TOOL)?;
        require_string("status", &self.status, LIVE_API_MATRIX_STATUS)?;
        self.source_route_gate.validate(route_gate)?;
        self.live_api_contract.validate(route_gate, &self.records)?;
        self.validate_records_against_route_gate(route_gate)?;
        self.claim_boundary.validate()?;
        require_all(
            "validation_commands",
            &self.validation_commands,
            &[
                "cargo test --test syntax_generated_codec_gate_test generated_codec_state_api_exposes_admissible_raw_tokens_for_checked_go_source_instance -- --exact",
                "cargo test --test syntax_backend_boundary_test syntax_backend_generated_codec_state_api_route_gate_consumes_integrated_route_metadata -- --exact",
                "cargo test --test syntax_state_machine_replay_test target_neutral_generated_codec_state_api_integration_consumes_shared_handoff_surface -- --exact",
                "cargo test --test syntax_generated_codec_gate_test csharp_compilation_unit_source_file_syntax_generated_codec_compiles_through_compiler_route -- --exact",
                "cargo test --test syntax_generated_codec_gate_test csharp_compilation_unit_iterator_methods_source_file_syntax_generated_codec_compiles_through_compiler_route -- --exact",
                "cargo test --test syntax_generated_codec_gate_test csharp_compilation_unit_loan_source_file_syntax_generated_codec_compiles_through_compiler_route -- --exact",
                "git diff --check",
            ],
        )?;
        require_string(
            "completed_promotion_class",
            &self.completed_promotion_class,
            LIVE_API_MATRIX_COMPLETED_PROMOTION_CLASS,
        )?;
        require_string(
            "next_missing_stable_promotion_class",
            &self.next_missing_stable_promotion_class,
            LIVE_API_MATRIX_NEXT_MISSING_STABLE_PROMOTION_CLASS,
        )
    }

    fn validate_records_against_route_gate(
        &self,
        route_gate: &SyntaxGeneratedCodecStateApiRouteGate,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        let mut observed_targets = BTreeSet::new();
        let mut observed_keys = BTreeSet::new();
        let mut available_count = 0usize;
        let mut blocked_count = 0usize;
        for record in &self.records {
            record.validate_shape()?;
            observed_targets.insert(record.target.as_str());
            if !observed_keys.insert((record.target.as_str(), record.record_id.as_str())) {
                return Err(SyntaxBackendBoundaryError::new(
                    "live_api_matrix.records",
                    format!(
                        "duplicate live API matrix record {}:{}",
                        record.target, record.record_id
                    ),
                ));
            }
            if record.generated_codec_prefix_state_api_available {
                available_count += 1;
            } else {
                blocked_count += 1;
            }
            let expected = route_gate
                .prefix_state_continuation_contract
                .response_evidence_records
                .iter()
                .find(|expected| {
                    expected.target == record.target && expected.record_id == record.record_id
                })
                .ok_or_else(|| {
                    SyntaxBackendBoundaryError::new(
                        "live_api_matrix.records.route_response_evidence",
                        format!(
                            "missing checked route response evidence for {}:{}",
                            record.target, record.record_id
                        ),
                    )
                })?;
            if &record.route_response_evidence != expected {
                return Err(SyntaxBackendBoundaryError::new(
                    "live_api_matrix.records.route_response_evidence",
                    format!(
                        "live API matrix record {}:{} does not match checked route response evidence",
                        record.target, record.record_id
                    ),
                ));
            }
        }
        let expected_keys = route_gate
            .prefix_state_continuation_contract
            .response_evidence_records
            .iter()
            .map(|record| (record.target.as_str(), record.record_id.as_str()))
            .collect::<BTreeSet<_>>();
        if observed_keys != expected_keys {
            let missing = expected_keys
                .difference(&observed_keys)
                .map(|key| format!("{}:{}", key.0, key.1))
                .collect::<Vec<_>>();
            let unexpected = observed_keys
                .difference(&expected_keys)
                .map(|key| format!("{}:{}", key.0, key.1))
                .collect::<Vec<_>>();
            return Err(SyntaxBackendBoundaryError::new(
                "live_api_matrix.records",
                format!(
                    "live API matrix must include every route-owned response-evidence record; missing [{}], unexpected [{}]",
                    missing.join(", "),
                    unexpected.join(", ")
                ),
            ));
        }
        require_usize(
            "live_api_matrix.records.available_count",
            available_count,
            self.live_api_contract
                .live_generated_codec_api_available_record_count,
        )?;
        require_usize(
            "live_api_matrix.records.blocked_count",
            blocked_count,
            self.live_api_contract
                .live_generated_codec_api_blocked_record_count,
        )?;
        require_usize(
            "live_api_matrix.records.total_count",
            available_count + blocked_count,
            route_gate
                .prefix_state_continuation_contract
                .response_evidence_record_count,
        )?;
        require_positive_usize("live_api_matrix.records.available_count", available_count)?;
        let required_targets = REQUIRED_TARGETS.iter().copied().collect::<BTreeSet<_>>();
        if !required_targets.is_subset(&observed_targets) {
            return Err(SyntaxBackendBoundaryError::new(
                "live_api_matrix.records.target",
                "live API matrix must include at least one Go record and one C# record",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxGeneratedCodecPrefixStateLiveApiMatrixSourceRouteGate {
    pub artifact_kind: String,
    pub fixture_path: String,
    pub completed_promotion_class: String,
    pub next_missing_stable_promotion_class: String,
    pub response_evidence_fixture: String,
    pub response_evidence_record_count: usize,
    pub response_evidence_raw_prefix_response_count: usize,
}

impl SyntaxGeneratedCodecPrefixStateLiveApiMatrixSourceRouteGate {
    fn validate(
        &self,
        route_gate: &SyntaxGeneratedCodecStateApiRouteGate,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        require_string(
            "source_route_gate.artifact_kind",
            &self.artifact_kind,
            ARTIFACT_KIND,
        )?;
        require_string(
            "source_route_gate.fixture_path",
            &self.fixture_path,
            FIXTURE_PATH,
        )?;
        require_string(
            "source_route_gate.completed_promotion_class",
            &self.completed_promotion_class,
            COMPLETED_PROMOTION_CLASS,
        )?;
        require_string(
            "source_route_gate.next_missing_stable_promotion_class",
            &self.next_missing_stable_promotion_class,
            NEXT_MISSING_STABLE_PROMOTION_CLASS,
        )?;
        require_string(
            "source_route_gate.response_evidence_fixture",
            &self.response_evidence_fixture,
            &route_gate
                .prefix_state_continuation_contract
                .response_evidence_fixture,
        )?;
        require_usize(
            "source_route_gate.response_evidence_record_count",
            self.response_evidence_record_count,
            route_gate
                .prefix_state_continuation_contract
                .response_evidence_record_count,
        )?;
        require_usize(
            "source_route_gate.response_evidence_raw_prefix_response_count",
            self.response_evidence_raw_prefix_response_count,
            route_gate
                .prefix_state_continuation_contract
                .response_evidence_raw_prefix_response_count,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxGeneratedCodecPrefixStateLiveApiMatrixContract {
    pub matrix_id: String,
    pub target_ids: Vec<String>,
    pub selected_record_count: usize,
    pub selected_raw_prefix_response_count: usize,
    pub selected_bpe_replay_response_count: usize,
    pub request_fields: Vec<String>,
    pub response_fields: Vec<String>,
    pub generated_codec_prefix_state_api_surface: String,
    pub target_neutral_state_api_authority: String,
    pub route_owned_response_evidence_consumed: bool,
    pub csharp_generated_codec_live_api_available: bool,
    pub go_source_instance_live_api_route_blocked: bool,
    pub go_source_instance_live_api_route_blocker: Option<String>,
    pub live_generated_codec_api_available_record_count: usize,
    pub live_generated_codec_api_blocked_record_count: usize,
    pub bpe_first_failure_diagnostics_preserved: bool,
    pub bpe_ids_used_as_syntax_validity_authority: bool,
    pub bpe_layer_defines_validity: bool,
    pub source_program_validity_claim_allowed: bool,
    pub stable_backend_promotion_allowed: bool,
}

impl SyntaxGeneratedCodecPrefixStateLiveApiMatrixContract {
    fn validate(
        &self,
        route_gate: &SyntaxGeneratedCodecStateApiRouteGate,
        records: &[SyntaxGeneratedCodecPrefixStateLiveApiMatrixRecord],
    ) -> Result<(), SyntaxBackendBoundaryError> {
        require_string(
            "live_api_contract.matrix_id",
            &self.matrix_id,
            "route-owned-prefix-state-continuation-live-api-matrix-v1",
        )?;
        require_all(
            "live_api_contract.target_ids",
            &self.target_ids,
            REQUIRED_TARGETS,
        )?;
        require_usize(
            "live_api_contract.selected_record_count",
            self.selected_record_count,
            records.len(),
        )?;
        require_usize(
            "live_api_contract.selected_record_count.route_owned_response_evidence",
            self.selected_record_count,
            route_gate
                .prefix_state_continuation_contract
                .response_evidence_record_count,
        )?;
        require_positive_usize(
            "live_api_contract.selected_record_count",
            self.selected_record_count,
        )?;
        require_usize(
            "live_api_contract.selected_raw_prefix_response_count",
            self.selected_raw_prefix_response_count,
            records
                .iter()
                .map(|record| record.route_response_evidence.response_count)
                .sum(),
        )?;
        require_usize(
            "live_api_contract.selected_raw_prefix_response_count.route_owned_response_evidence",
            self.selected_raw_prefix_response_count,
            route_gate
                .prefix_state_continuation_contract
                .response_evidence_raw_prefix_response_count,
        )?;
        require_usize(
            "live_api_contract.selected_bpe_replay_response_count",
            self.selected_bpe_replay_response_count,
            records
                .iter()
                .map(|record| {
                    record
                        .route_response_evidence
                        .bpe_raw_expansion_replay_case_count
                })
                .sum(),
        )?;
        require_usize(
            "live_api_contract.selected_bpe_replay_response_count.route_owned_response_evidence",
            self.selected_bpe_replay_response_count,
            route_gate
                .prefix_state_continuation_contract
                .response_evidence_bpe_replay_response_count,
        )?;
        require_all(
            "live_api_contract.request_fields",
            &self.request_fields,
            REQUIRED_PREFIX_STATE_CONTINUATION_REQUEST_FIELDS,
        )?;
        require_all(
            "live_api_contract.response_fields",
            &self.response_fields,
            REQUIRED_PREFIX_STATE_CONTINUATION_RESPONSE_FIELDS,
        )?;
        require_string(
            "live_api_contract.generated_codec_prefix_state_api_surface",
            &self.generated_codec_prefix_state_api_surface,
            "src/syntax_generated_codec.rs::generated_codec_prefix_state_api_metadata",
        )?;
        require_string(
            "live_api_contract.target_neutral_state_api_authority",
            &self.target_neutral_state_api_authority,
            ADMISSIBLE_TOKEN_MASK_VALIDITY_AUTHORITY,
        )?;
        require_bool(
            "live_api_contract.route_owned_response_evidence_consumed",
            self.route_owned_response_evidence_consumed,
            true,
        )?;
        require_bool(
            "live_api_contract.csharp_generated_codec_live_api_available",
            self.csharp_generated_codec_live_api_available,
            true,
        )?;
        require_bool(
            "live_api_contract.go_source_instance_live_api_route_blocked",
            self.go_source_instance_live_api_route_blocked,
            false,
        )?;
        require_optional_string(
            "live_api_contract.go_source_instance_live_api_route_blocker",
            self.go_source_instance_live_api_route_blocker.as_deref(),
            None,
        )?;
        require_positive_usize(
            "live_api_contract.live_generated_codec_api_available_record_count",
            self.live_generated_codec_api_available_record_count,
        )?;
        require_usize(
            "live_api_contract.live_generated_codec_api_total_record_count",
            self.live_generated_codec_api_available_record_count
                + self.live_generated_codec_api_blocked_record_count,
            self.selected_record_count,
        )?;
        require_bool(
            "live_api_contract.bpe_first_failure_diagnostics_preserved",
            self.bpe_first_failure_diagnostics_preserved,
            records
                .iter()
                .filter(|record| {
                    !record
                        .route_response_evidence
                        .bpe_raw_expansion_responses
                        .is_empty()
                })
                .all(|record| {
                    record
                        .route_response_evidence
                        .bpe_first_failure_diagnostic_count
                        > 0
                }),
        )?;
        for (field, actual) in [
            (
                "live_api_contract.bpe_ids_used_as_syntax_validity_authority",
                self.bpe_ids_used_as_syntax_validity_authority,
            ),
            (
                "live_api_contract.bpe_layer_defines_validity",
                self.bpe_layer_defines_validity,
            ),
            (
                "live_api_contract.source_program_validity_claim_allowed",
                self.source_program_validity_claim_allowed,
            ),
            (
                "live_api_contract.stable_backend_promotion_allowed",
                self.stable_backend_promotion_allowed,
            ),
        ] {
            require_bool(field, actual, false)?;
        }
        require_string(
            "live_api_contract.source_route_gate_next",
            &route_gate.next_missing_stable_promotion_class,
            LIVE_API_MATRIX_NEXT_MISSING_STABLE_PROMOTION_CLASS,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxGeneratedCodecPrefixStateLiveApiMatrixRecord {
    pub target: String,
    pub record_id: String,
    pub generated_codec: String,
    pub generated_codec_prefix_state_api_available: bool,
    pub generated_codec_prefix_state_api_metadata: Option<GeneratedCodecPrefixStateApiMetadata>,
    pub live_api_status: String,
    pub route_response_evidence:
        SyntaxGeneratedCodecStateApiRoutePrefixStateContinuationRecordEvidence,
    pub next_missing_generated_codec_live_api_work: Option<String>,
}

impl SyntaxGeneratedCodecPrefixStateLiveApiMatrixRecord {
    fn validate_shape(&self) -> Result<(), SyntaxBackendBoundaryError> {
        require_non_empty("live_api_matrix.record.target", &self.target)?;
        require_non_empty("live_api_matrix.record.record_id", &self.record_id)?;
        require_non_empty(
            "live_api_matrix.record.generated_codec",
            &self.generated_codec,
        )?;
        self.route_response_evidence.validate_shape()?;
        require_string(
            "live_api_matrix.record.route_response.target",
            &self.route_response_evidence.target,
            &self.target,
        )?;
        require_string(
            "live_api_matrix.record.route_response.record_id",
            &self.route_response_evidence.record_id,
            &self.record_id,
        )?;
        match (
            self.generated_codec_prefix_state_api_available,
            self.generated_codec_prefix_state_api_metadata.as_ref(),
            self.next_missing_generated_codec_live_api_work.as_deref(),
        ) {
            (true, Some(metadata), None) => {
                let expected = generated_codec_prefix_state_api_metadata(&self.generated_codec)
                    .ok_or_else(|| {
                        SyntaxBackendBoundaryError::new(
                            "live_api_matrix.record.generated_codec_prefix_state_api_metadata",
                            format!(
                                "missing generated-codec prefix-state API metadata for {}",
                                self.generated_codec
                            ),
                        )
                    })?;
                if *metadata != expected {
                    return Err(SyntaxBackendBoundaryError::new(
                        "live_api_matrix.record.generated_codec_prefix_state_api_metadata",
                        format!(
                            "stale generated-codec prefix-state API metadata for {}",
                            self.generated_codec
                        ),
                    ));
                }
                require_string(
                    "live_api_matrix.record.live_api_status",
                    &self.live_api_status,
                    "live_generated_codec_prefix_state_api_available",
                )
            }
            (false, None, Some(blocker)) => {
                require_string(
                    "live_api_matrix.record.next_missing_generated_codec_live_api_work",
                    blocker,
                    GO_SOURCE_INSTANCE_LIVE_API_ROUTE_BLOCKER,
                )?;
                require_string(
                    "live_api_matrix.record.live_api_status",
                    &self.live_api_status,
                    "blocked_pending_generated_codec_live_api_route",
                )
            }
            (true, _, _) => Err(SyntaxBackendBoundaryError::new(
                "live_api_matrix.record.generated_codec_prefix_state_api_metadata",
                "available live API records must carry metadata and no blocker",
            )),
            (false, _, _) => Err(SyntaxBackendBoundaryError::new(
                "live_api_matrix.record.next_missing_generated_codec_live_api_work",
                "blocked live API records must carry the precise generated-codec route blocker",
            )),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxGeneratedCodecPrefixStateLiveApiMatrixClaimBoundary {
    pub syntax_level_boundary: String,
    pub route_owned_live_api_matrix: bool,
    pub checked_response_evidence_consumed: bool,
    pub tct_compiler_routing_changed: bool,
    pub grammar_ir_or_structural_model_routed_through_json_schema_compiler: bool,
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

impl SyntaxGeneratedCodecPrefixStateLiveApiMatrixClaimBoundary {
    fn validate(&self) -> Result<(), SyntaxBackendBoundaryError> {
        require_string(
            "live_api_matrix.claim_boundary.syntax_level_boundary",
            &self.syntax_level_boundary,
            "parser-backed metadata -> GrammarIR -> StructuralModel -> token layout -> generated codec",
        )?;
        for (field, actual, expected) in [
            (
                "live_api_matrix.claim_boundary.route_owned_live_api_matrix",
                self.route_owned_live_api_matrix,
                true,
            ),
            (
                "live_api_matrix.claim_boundary.checked_response_evidence_consumed",
                self.checked_response_evidence_consumed,
                true,
            ),
            (
                "live_api_matrix.claim_boundary.tct_compiler_routing_changed",
                self.tct_compiler_routing_changed,
                false,
            ),
            (
                "live_api_matrix.claim_boundary.grammar_ir_or_structural_model_routed_through_json_schema_compiler",
                self.grammar_ir_or_structural_model_routed_through_json_schema_compiler,
                false,
            ),
            (
                "live_api_matrix.claim_boundary.source_program_validity_claim_allowed",
                self.source_program_validity_claim_allowed,
                false,
            ),
            (
                "live_api_matrix.claim_boundary.type_correctness_claim_allowed",
                self.type_correctness_claim_allowed,
                false,
            ),
            (
                "live_api_matrix.claim_boundary.generated_hot_path_claim_allowed",
                self.generated_hot_path_claim_allowed,
                false,
            ),
            (
                "live_api_matrix.claim_boundary.paper_facing_claim_allowed",
                self.paper_facing_claim_allowed,
                false,
            ),
            (
                "live_api_matrix.claim_boundary.stable_backend_promotion_allowed",
                self.stable_backend_promotion_allowed,
                false,
            ),
            (
                "live_api_matrix.claim_boundary.arena_ir_proof_used",
                self.arena_ir_proof_used,
                false,
            ),
            (
                "live_api_matrix.claim_boundary.bpe_layer_validity_claim_allowed",
                self.bpe_layer_validity_claim_allowed,
                false,
            ),
            (
                "live_api_matrix.claim_boundary.json_schema_backend_changed",
                self.json_schema_backend_changed,
                false,
            ),
        ] {
            require_bool(field, actual, expected)?;
        }
        require_all(
            "live_api_matrix.claim_boundary.no_stable_claims",
            &self.no_stable_claims,
            REQUIRED_NO_STABLE_CLAIMS,
        )
    }
}

fn build_prefix_state_continuation_record_evidence(
    query: &crate::syntax_generated_codec_state_api_integration::GeneratedCodecStateApiIntegrationRecordQuery,
    result: &TargetNeutralPrefixStateContinuationResult,
    raw_stream: SyntaxGeneratedCodecStateApiRouteRawTokenStream<'_>,
) -> Result<
    SyntaxGeneratedCodecStateApiRoutePrefixStateContinuationRecordEvidence,
    SyntaxBackendBoundaryError,
> {
    require_string(
        "prefix_state_continuation_response_evidence.target",
        &result.target,
        &query.target,
    )?;
    require_string(
        "prefix_state_continuation_response_evidence.record_id",
        &result.record_id,
        &query.record_id,
    )?;
    require_usize(
        "prefix_state_continuation_response_evidence.raw_token_count",
        result.raw_token_count,
        raw_stream.raw_token_ids.len(),
    )?;
    require_usize(
        "prefix_state_continuation_response_evidence.request_count",
        result.request_count,
        query.prefix_state_case_ids.len(),
    )?;
    require_usize(
        "prefix_state_continuation_response_evidence.response_count",
        result.response_count,
        query.prefix_state_case_ids.len(),
    )?;
    require_string(
        "prefix_state_continuation_response_evidence.raw_stream.target",
        raw_stream.target,
        &query.target,
    )?;
    require_string(
        "prefix_state_continuation_response_evidence.raw_stream.record_id",
        raw_stream.record_id,
        &query.record_id,
    )?;
    require_string(
        "prefix_state_continuation_response_evidence.validity_authority",
        &result.validity_authority,
        ADMISSIBLE_TOKEN_MASK_VALIDITY_AUTHORITY,
    )?;
    require_bool(
        "prefix_state_continuation_response_evidence.bpe_layer_defines_validity",
        result.bpe_layer_defines_validity,
        false,
    )?;
    require_bool(
        "prefix_state_continuation_response_evidence.bpe_ids_used_as_syntax_validity_authority",
        result.bpe_ids_used_as_syntax_validity_authority,
        false,
    )?;

    let responses = result
        .responses
        .iter()
        .map(|response| {
            let raw_prefix = raw_stream
                .raw_token_ids
                .get(..response.consumed_raw_tokens)
                .ok_or_else(|| {
                    SyntaxBackendBoundaryError::new(
                        "prefix_state_continuation_response.raw_prefix_token_ids",
                        "prefix response consumed more tokens than the raw stream contains",
                    )
                })?
                .to_vec();
            let raw_prefix_token_ids_sha256 = json_sha256(&raw_prefix).map_err(|error| {
                SyntaxBackendBoundaryError::new(
                    "prefix_state_continuation_response.raw_prefix_token_ids_sha256",
                    format!("failed to hash raw prefix token IDs: {error}"),
                )
            })?;
            Ok(
                SyntaxGeneratedCodecStateApiRoutePrefixStateContinuationResponseEvidence {
                    target: result.target.clone(),
                    record_id: result.record_id.clone(),
                    prefix_state_case_id: response.request_prefix_state_case_id.clone(),
                    raw_prefix_token_count: raw_prefix.len(),
                    raw_prefix_token_ids: (!response.finite_admissible_raw_token_ids.is_empty()
                        || !response.payload_token_ranges.is_empty())
                    .then_some(raw_prefix),
                    raw_prefix_token_ids_sha256,
                    consumed_raw_tokens: response.consumed_raw_tokens,
                    prefix_status: response.prefix_status.clone(),
                    eof_valid: response.eof_valid,
                    active_rule: response.active_rule.clone(),
                    active_field: response.active_field.clone(),
                    finite_admissible_raw_token_ids: response
                        .finite_admissible_raw_token_ids
                        .clone(),
                    payload_token_ranges: response.payload_token_ranges.clone(),
                    payload_end_admissible: response.payload_end_admissible,
                },
            )
        })
        .collect::<Result<Vec<_>, SyntaxBackendBoundaryError>>()?;
    let bpe_raw_expansion_responses = result
        .bpe_replay_responses
        .iter()
        .map(|response| {
            SyntaxGeneratedCodecStateApiRoutePrefixStateContinuationBpeResponseEvidence {
                target: result.target.clone(),
                record_id: result.record_id.clone(),
                case_id: response.case_id.clone(),
                expanded_raw_token_count: response.expanded_raw_token_count,
                accepted_raw_token_count: response.accepted_raw_token_count,
                accepted: response.accepted,
                final_prefix_status: response.final_prefix_status.clone(),
                final_eof_valid: response.final_eof_valid,
                first_failure_transition_case_id: response.first_failure_transition_case_id.clone(),
                first_failure_error_kind: response.first_failure_error_kind.clone(),
                bpe_raw_expansion_first_failure_diagnostic: response
                    .first_failure_diagnostic
                    .clone(),
            }
        })
        .collect();

    let record = SyntaxGeneratedCodecStateApiRoutePrefixStateContinuationRecordEvidence {
        target: result.target.clone(),
        record_id: result.record_id.clone(),
        raw_token_count: result.raw_token_count,
        raw_token_ids_sha256: raw_stream.raw_token_ids_sha256.to_string(),
        request_count: result.request_count,
        response_count: result.response_count,
        responses,
        finite_admissible_response_count: result.finite_admissible_response_count,
        payload_range_response_count: result.payload_range_response_count,
        eof_validity_response_count: result.eof_valid_response_count,
        bpe_raw_expansion_responses,
        bpe_raw_expansion_replay_case_count: result.bpe_raw_expansion_replay_case_count,
        bpe_raw_expansion_accepted_case_count: result.bpe_raw_expansion_accepted_case_count,
        bpe_raw_expansion_first_failure_case_count: result
            .bpe_raw_expansion_first_failure_case_count,
        bpe_first_failure_diagnostic_count: result.bpe_first_failure_diagnostic_count,
        validity_authority: result.validity_authority.clone(),
        bpe_layer_defines_validity: result.bpe_layer_defines_validity,
        bpe_ids_used_as_syntax_validity_authority: result.bpe_ids_used_as_syntax_validity_authority,
    };
    record.validate_shape()?;
    Ok(record)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxGeneratedCodecStateApiRouteClaimBoundary {
    pub syntax_level_boundary: String,
    pub generated_codec_state_api_route_gate: bool,
    pub existing_feature_limited_syntax_route_metadata_already_promoted: bool,
    pub state_api_stable_backend_promotion_allowed: bool,
    pub source_program_validity_claim_allowed: bool,
    pub type_correctness_claim_allowed: bool,
    pub generated_hot_path_claim_allowed: bool,
    pub paper_facing_claim_allowed: bool,
    pub arena_ir_proof_used: bool,
    pub bpe_layer_validity_claim_allowed: bool,
    pub grammar_ir_or_structural_model_routed_through_json_schema_compiler: bool,
    pub json_schema_backend_changed: bool,
    pub no_stable_claims: Vec<String>,
}

impl SyntaxGeneratedCodecStateApiRouteClaimBoundary {
    fn validate(&self) -> Result<(), SyntaxBackendBoundaryError> {
        require_string(
            "claim_boundary.syntax_level_boundary",
            &self.syntax_level_boundary,
            "parser-backed metadata -> GrammarIR -> StructuralModel -> token layout -> generated codec",
        )?;
        require_bool(
            "claim_boundary.generated_codec_state_api_route_gate",
            self.generated_codec_state_api_route_gate,
            true,
        )?;
        require_bool(
            "claim_boundary.existing_feature_limited_syntax_route_metadata_already_promoted",
            self.existing_feature_limited_syntax_route_metadata_already_promoted,
            true,
        )?;
        require_bool(
            "claim_boundary.state_api_stable_backend_promotion_allowed",
            self.state_api_stable_backend_promotion_allowed,
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
            "claim_boundary.grammar_ir_or_structural_model_routed_through_json_schema_compiler",
            self.grammar_ir_or_structural_model_routed_through_json_schema_compiler,
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

fn validate_route_artifacts(
    route_artifacts: &[SyntaxGeneratedCodecArtifact],
    integration_gate: &SyntaxGeneratedCodecStateApiIntegrationGate,
) -> Result<(), SyntaxBackendBoundaryError> {
    require_positive_usize("route_artifacts", route_artifacts.len())?;
    let mut observed_targets = BTreeSet::new();
    for artifact in route_artifacts {
        if REQUIRED_TARGETS.contains(&artifact.target) {
            observed_targets.insert(artifact.target);
        }
        let route = artifact
            .metadata
            .get(TARGET_AGNOSTIC_ROUTE_METADATA_FIELD)
            .ok_or_else(|| {
                SyntaxBackendBoundaryError::new(
                    "route_artifacts.target_agnostic_compiler_route",
                    "missing target-agnostic compiler route metadata",
                )
            })?;
        require_json_string(
            "route_artifacts.target_agnostic_compiler_route.entrypoint",
            route,
            "entrypoint",
            SYNTAX_COMPILER_ROUTE_ENTRYPOINT,
        )?;
        require_json_string(
            "route_artifacts.target_agnostic_compiler_route.delegates_to",
            route,
            "delegates_to",
            ROUTE_ENTRYPOINT,
        )?;
        require_json_string(
            "route_artifacts.target_agnostic_compiler_route.route_gate",
            route,
            "route_gate",
            ROUTE_GATE,
        )?;
        require_json_bool(
            "route_artifacts.target_agnostic_compiler_route.rejects_arena_ir_boundaries",
            route,
            "rejects_arena_ir_boundaries",
            true,
        )?;

        let state_api_route = route.get(STATE_API_ROUTE_METADATA_FIELD).ok_or_else(|| {
            SyntaxBackendBoundaryError::new(
                "route_artifacts.generated_codec_state_api_route",
                "missing generated-codec state API route metadata",
            )
        })?;
        validate_state_api_route_metadata(state_api_route, integration_gate)?;
    }

    let required_targets = REQUIRED_TARGETS.iter().copied().collect::<BTreeSet<_>>();
    if !required_targets.is_subset(&observed_targets) {
        return Err(SyntaxBackendBoundaryError::new(
            "route_artifacts.target",
            format!(
                "missing route artifact targets {:?}",
                required_targets
                    .difference(&observed_targets)
                    .copied()
                    .collect::<Vec<_>>()
            ),
        ));
    }
    Ok(())
}

fn validate_state_api_route_metadata(
    state_api_route: &Value,
    integration_gate: &SyntaxGeneratedCodecStateApiIntegrationGate,
) -> Result<(), SyntaxBackendBoundaryError> {
    require_json_string(
        "state_api_route.source_integration_fixture",
        state_api_route,
        "source_integration_fixture",
        INTEGRATION_FIXTURE_PATH,
    )?;
    require_json_string(
        "state_api_route.source_integration_artifact_kind",
        state_api_route,
        "source_integration_artifact_kind",
        INTEGRATION_ARTIFACT_KIND,
    )?;
    require_json_string(
        "state_api_route.source_integration_completed_promotion_class",
        state_api_route,
        "source_integration_completed_promotion_class",
        INTEGRATION_COMPLETED_PROMOTION_CLASS,
    )?;
    require_json_string(
        "state_api_route.target_neutral_surface_id",
        state_api_route,
        "target_neutral_surface_id",
        &integration_gate.integration_surface.surface_id,
    )?;
    require_json_string(
        "state_api_route.completed_promotion_class",
        state_api_route,
        "completed_promotion_class",
        COMPLETED_PROMOTION_CLASS,
    )?;
    require_json_string(
        "state_api_route.next_missing_stable_promotion_class",
        state_api_route,
        "next_missing_stable_promotion_class",
        NEXT_MISSING_STABLE_PROMOTION_CLASS,
    )?;
    require_json_all(
        "state_api_route.target_ids",
        state_api_route,
        "target_ids",
        REQUIRED_TARGETS,
    )?;
    require_json_all(
        "state_api_route.capabilities",
        state_api_route,
        "capabilities",
        REQUIRED_CAPABILITIES,
    )?;
    require_json_all(
        "state_api_route.route_evidence",
        state_api_route,
        "route_evidence",
        &required_route_evidence(),
    )?;
    require_json_all(
        "state_api_route.go_source_instance_records",
        state_api_route,
        "go_source_instance_records",
        GO_SOURCE_INSTANCE_RECORDS,
    )?;
    require_json_all(
        "state_api_route.csharp_real_source_records",
        state_api_route,
        "csharp_real_source_records",
        REQUIRED_CSHARP_RECORDS,
    )?;
    require_json_usize(
        "state_api_route.record_query_count",
        state_api_route,
        "record_query_count",
        route_owned_record_query_count(integration_gate)?,
    )?;
    for (field, expected) in [
        ("query_contract_consumed_from_integration", true),
        ("prefix_state_queries_ready", true),
        ("transition_queries_ready", true),
        ("eof_queries_ready", true),
        ("strict_recovery_decode_queries_ready", true),
        ("consumed_token_accounting_queries_ready", true),
        ("admissible_raw_token_set_queries_ready", true),
        ("admissible_raw_token_range_queries_ready", true),
        ("one_token_bpe_raw_expansion_probe_ready", true),
        ("multi_token_bpe_raw_expansion_runtime_replay_ready", true),
        ("state_api_stable_backend_promotion_allowed", false),
        ("source_program_validity_claim_allowed", false),
        ("arena_ir_proof_used", false),
    ] {
        require_json_bool("state_api_route.boolean", state_api_route, field, expected)?;
    }
    require_json_usize(
        "state_api_route.multi_token_bpe_raw_expansion_replay_case_count",
        state_api_route,
        "multi_token_bpe_raw_expansion_replay_case_count",
        multi_token_replay_case_count(integration_gate),
    )?;
    require_json_string(
        "state_api_route.next_generated_codec_state_api_work",
        state_api_route,
        "next_generated_codec_state_api_work",
        NEXT_MISSING_STABLE_PROMOTION_CLASS,
    )?;
    let mask_contract = state_api_route
        .get(ADMISSIBLE_TOKEN_MASK_METADATA_FIELD)
        .ok_or_else(|| {
            SyntaxBackendBoundaryError::new(
                "state_api_route.admissible_token_mask_contract",
                "missing route-owned admissible-token mask contract metadata",
            )
        })?;
    validate_admissible_token_mask_metadata(mask_contract, integration_gate)?;
    let continuation_contract = state_api_route
        .get(PREFIX_STATE_CONTINUATION_METADATA_FIELD)
        .ok_or_else(|| {
            SyntaxBackendBoundaryError::new(
                "state_api_route.prefix_state_continuation_contract",
                "missing route-owned prefix-state continuation contract metadata",
            )
        })?;
    validate_prefix_state_continuation_metadata(continuation_contract, integration_gate)?;
    Ok(())
}

fn multi_token_replay_case_count(
    integration_gate: &SyntaxGeneratedCodecStateApiIntegrationGate,
) -> usize {
    route_owned_record_queries(integration_gate)
        .into_iter()
        .map(|record| record.bpe_raw_expansion_replay_cases.len())
        .sum()
}

fn route_owned_record_queries(
    integration_gate: &SyntaxGeneratedCodecStateApiIntegrationGate,
) -> Vec<&GeneratedCodecStateApiIntegrationRecordQuery> {
    integration_gate
        .record_queries
        .iter()
        .filter(|record| match record.target.as_str() {
            "go" => GO_SOURCE_INSTANCE_RECORDS.contains(&record.record_id.as_str()),
            "csharp" => REQUIRED_CSHARP_RECORDS.contains(&record.record_id.as_str()),
            _ => false,
        })
        .collect()
}

fn route_owned_record_query_count(
    integration_gate: &SyntaxGeneratedCodecStateApiIntegrationGate,
) -> Result<usize, SyntaxBackendBoundaryError> {
    let count = route_owned_record_queries(integration_gate).len();
    require_usize(
        "route_owned_record_queries",
        count,
        GO_SOURCE_INSTANCE_RECORDS.len() + REQUIRED_CSHARP_RECORDS.len(),
    )?;
    Ok(count)
}

fn required_route_evidence() -> Vec<&'static str> {
    GO_SOURCE_INSTANCE_GRAMMAR_IDS
        .iter()
        .copied()
        .chain(std::iter::once(CSHARP_SOURCE_FILE_GRAMMAR_ID))
        .collect()
}

fn validate_admissible_token_mask_metadata(
    mask_contract: &Value,
    integration_gate: &SyntaxGeneratedCodecStateApiIntegrationGate,
) -> Result<(), SyntaxBackendBoundaryError> {
    let summary = admissible_mask_summary(integration_gate)?;
    require_json_string(
        "state_api_route.admissible_token_mask_contract.mask_contract_id",
        mask_contract,
        "mask_contract_id",
        ADMISSIBLE_TOKEN_MASK_CONTRACT_ID,
    )?;
    require_json_string(
        "state_api_route.admissible_token_mask_contract.completed_promotion_class",
        mask_contract,
        "completed_promotion_class",
        SYNTAX_COMPILER_ADMISSIBLE_TOKEN_MASK_ROUTE_COMPLETED_PROMOTION_CLASS,
    )?;
    require_json_string(
        "state_api_route.admissible_token_mask_contract.next_missing_stable_promotion_class",
        mask_contract,
        "next_missing_stable_promotion_class",
        SYNTAX_COMPILER_ADMISSIBLE_TOKEN_MASK_ROUTE_NEXT_MISSING_STABLE_PROMOTION_CLASS,
    )?;
    require_json_all(
        "state_api_route.admissible_token_mask_contract.target_ids",
        mask_contract,
        "target_ids",
        REQUIRED_TARGETS,
    )?;
    require_json_string(
        "state_api_route.admissible_token_mask_contract.source_query_contract",
        mask_contract,
        "source_query_contract",
        ADMISSIBLE_TOKEN_MASK_SOURCE_QUERY_CONTRACT,
    )?;
    require_json_string(
        "state_api_route.admissible_token_mask_contract.target_neutral_state_api_authority",
        mask_contract,
        "target_neutral_state_api_authority",
        ADMISSIBLE_TOKEN_MASK_VALIDITY_AUTHORITY,
    )?;
    for (field, expected) in [
        ("record_query_count", summary.record_query_count),
        ("finite_mask_record_count", summary.finite_mask_record_count),
        (
            "payload_range_mask_record_count",
            summary.payload_range_mask_record_count,
        ),
        (
            "eof_validity_record_count",
            summary.eof_validity_record_count,
        ),
        (
            "bpe_raw_expansion_replay_case_count",
            summary.bpe_raw_expansion_replay_case_count,
        ),
        (
            "bpe_raw_expansion_accepted_case_count",
            summary.bpe_raw_expansion_accepted_case_count,
        ),
        (
            "bpe_raw_expansion_first_failure_case_count",
            summary.bpe_raw_expansion_first_failure_case_count,
        ),
    ] {
        require_json_usize(
            "state_api_route.admissible_token_mask_contract.summary",
            mask_contract,
            field,
            expected,
        )?;
    }
    for (field, expected) in [
        ("requires_first_failure_diagnostics", true),
        ("bpe_ids_used_as_syntax_validity_authority", false),
        ("bpe_layer_defines_validity", false),
        ("source_program_validity_claim_allowed", false),
        ("stable_backend_promotion_allowed", false),
    ] {
        require_json_bool(
            "state_api_route.admissible_token_mask_contract.boolean",
            mask_contract,
            field,
            expected,
        )?;
    }
    Ok(())
}

fn validate_prefix_state_continuation_metadata(
    continuation_contract: &Value,
    integration_gate: &SyntaxGeneratedCodecStateApiIntegrationGate,
) -> Result<(), SyntaxBackendBoundaryError> {
    let summary = prefix_state_continuation_summary(integration_gate)?;
    require_json_string(
        "state_api_route.prefix_state_continuation_contract.continuation_contract_id",
        continuation_contract,
        "continuation_contract_id",
        PREFIX_STATE_CONTINUATION_CONTRACT_ID,
    )?;
    require_json_string(
        "state_api_route.prefix_state_continuation_contract.completed_promotion_class",
        continuation_contract,
        "completed_promotion_class",
        SYNTAX_COMPILER_PREFIX_STATE_CONTINUATION_ROUTE_COMPLETED_PROMOTION_CLASS,
    )?;
    require_json_string(
        "state_api_route.prefix_state_continuation_contract.next_missing_stable_promotion_class",
        continuation_contract,
        "next_missing_stable_promotion_class",
        SYNTAX_COMPILER_PREFIX_STATE_CONTINUATION_ROUTE_NEXT_MISSING_STABLE_PROMOTION_CLASS,
    )?;
    require_json_string(
        "state_api_route.prefix_state_continuation_contract.source_admissible_token_mask_contract_id",
        continuation_contract,
        "source_admissible_token_mask_contract_id",
        ADMISSIBLE_TOKEN_MASK_CONTRACT_ID,
    )?;
    require_json_string(
        "state_api_route.prefix_state_continuation_contract.source_admissible_token_mask_completed_promotion_class",
        continuation_contract,
        "source_admissible_token_mask_completed_promotion_class",
        SYNTAX_COMPILER_ADMISSIBLE_TOKEN_MASK_ROUTE_COMPLETED_PROMOTION_CLASS,
    )?;
    require_json_all(
        "state_api_route.prefix_state_continuation_contract.target_ids",
        continuation_contract,
        "target_ids",
        REQUIRED_TARGETS,
    )?;
    require_json_string(
        "state_api_route.prefix_state_continuation_contract.source_query_contract",
        continuation_contract,
        "source_query_contract",
        ADMISSIBLE_TOKEN_MASK_SOURCE_QUERY_CONTRACT,
    )?;
    require_json_string(
        "state_api_route.prefix_state_continuation_contract.target_neutral_state_api_authority",
        continuation_contract,
        "target_neutral_state_api_authority",
        ADMISSIBLE_TOKEN_MASK_VALIDITY_AUTHORITY,
    )?;
    require_json_all(
        "state_api_route.prefix_state_continuation_contract.request_fields",
        continuation_contract,
        "request_fields",
        REQUIRED_PREFIX_STATE_CONTINUATION_REQUEST_FIELDS,
    )?;
    require_json_all(
        "state_api_route.prefix_state_continuation_contract.response_fields",
        continuation_contract,
        "response_fields",
        REQUIRED_PREFIX_STATE_CONTINUATION_RESPONSE_FIELDS,
    )?;
    for (field, expected) in [
        ("record_query_count", summary.record_query_count),
        ("raw_prefix_request_count", summary.raw_prefix_request_count),
        (
            "finite_admissible_response_record_count",
            summary.finite_admissible_response_record_count,
        ),
        (
            "payload_range_response_record_count",
            summary.payload_range_response_record_count,
        ),
        (
            "eof_validity_response_record_count",
            summary.eof_validity_response_record_count,
        ),
        (
            "bpe_raw_expansion_replay_case_count",
            summary.bpe_raw_expansion_replay_case_count,
        ),
        (
            "bpe_raw_expansion_first_failure_case_count",
            summary.bpe_raw_expansion_first_failure_case_count,
        ),
        ("response_evidence_record_count", summary.record_query_count),
        (
            "response_evidence_raw_prefix_response_count",
            summary.raw_prefix_request_count,
        ),
        (
            "response_evidence_bpe_replay_response_count",
            summary.bpe_raw_expansion_replay_case_count,
        ),
    ] {
        require_json_usize(
            "state_api_route.prefix_state_continuation_contract.summary",
            continuation_contract,
            field,
            expected,
        )?;
    }
    require_json_string(
        "state_api_route.prefix_state_continuation_contract.response_evidence_fixture",
        continuation_contract,
        "response_evidence_fixture",
        SYNTAX_COMPILER_PREFIX_STATE_CONTINUATION_ROUTE_RESPONSE_EVIDENCE_FIXTURE,
    )?;
    for (field, expected) in [
        ("response_evidence_checked_against_target_neutral_api", true),
        ("consumes_route_owned_admissible_mask_contract", true),
        ("requires_consumed_raw_token_count", true),
        ("requires_prefix_status", true),
        ("requires_eof_validity", true),
        ("requires_first_failure_diagnostics", true),
        ("bpe_ids_used_as_syntax_validity_authority", false),
        ("bpe_layer_defines_validity", false),
        ("source_program_validity_claim_allowed", false),
        ("stable_backend_promotion_allowed", false),
    ] {
        require_json_bool(
            "state_api_route.prefix_state_continuation_contract.boolean",
            continuation_contract,
            field,
            expected,
        )?;
    }
    Ok(())
}

#[derive(Debug, Clone, Copy)]
struct AdmissibleMaskSummary {
    record_query_count: usize,
    finite_mask_record_count: usize,
    payload_range_mask_record_count: usize,
    eof_validity_record_count: usize,
    bpe_raw_expansion_replay_case_count: usize,
    bpe_raw_expansion_accepted_case_count: usize,
    bpe_raw_expansion_first_failure_case_count: usize,
}

#[derive(Debug, Clone, Copy)]
struct PrefixStateContinuationSummary {
    record_query_count: usize,
    raw_prefix_request_count: usize,
    finite_admissible_response_record_count: usize,
    payload_range_response_record_count: usize,
    eof_validity_response_record_count: usize,
    bpe_raw_expansion_replay_case_count: usize,
    bpe_raw_expansion_first_failure_case_count: usize,
}

fn admissible_mask_summary(
    integration_gate: &SyntaxGeneratedCodecStateApiIntegrationGate,
) -> Result<AdmissibleMaskSummary, SyntaxBackendBoundaryError> {
    let record_queries = route_owned_record_queries(integration_gate);
    let mut summary = AdmissibleMaskSummary {
        record_query_count: record_queries.len(),
        finite_mask_record_count: 0,
        payload_range_mask_record_count: 0,
        eof_validity_record_count: 0,
        bpe_raw_expansion_replay_case_count: 0,
        bpe_raw_expansion_accepted_case_count: 0,
        bpe_raw_expansion_first_failure_case_count: 0,
    };
    require_positive_usize(
        "admissible_token_mask_contract.record_query_count",
        summary.record_query_count,
    )?;
    route_owned_record_query_count(integration_gate)?;

    for record_query in record_queries {
        require_positive_usize(
            "admissible_token_mask_contract.prefix_state_case_ids",
            record_query.prefix_state_case_ids.len(),
        )?;
        summary.finite_mask_record_count += 1;
        if record_query.payload_range_state_case_id.is_empty() {
            return Err(SyntaxBackendBoundaryError::new(
                "admissible_token_mask_contract.payload_range_state_case_id",
                "record query must identify a payload-range mask state",
            ));
        }
        summary.payload_range_mask_record_count += 1;
        if record_query.eof_state_case_id.is_empty() {
            return Err(SyntaxBackendBoundaryError::new(
                "admissible_token_mask_contract.eof_state_case_id",
                "record query must identify an EOF-valid mask state",
            ));
        }
        summary.eof_validity_record_count += 1;

        for replay_case in &record_query.bpe_raw_expansion_replay_cases {
            summary.bpe_raw_expansion_replay_case_count += 1;
            require_positive_usize(
                "admissible_token_mask_contract.bpe_transition_case_ids",
                replay_case.transition_case_ids.len(),
            )?;
            if replay_case.expected_accepted {
                summary.bpe_raw_expansion_accepted_case_count += 1;
            } else if replay_case
                .expected_first_failure_transition_case_id
                .is_some()
                && replay_case.expected_first_failure_error_kind.is_some()
            {
                summary.bpe_raw_expansion_first_failure_case_count += 1;
            } else {
                return Err(SyntaxBackendBoundaryError::new(
                    "admissible_token_mask_contract.bpe_first_failure",
                    "rejected BPE replay cases must name first-failure diagnostics",
                ));
            }
        }
    }

    require_positive_usize(
        "admissible_token_mask_contract.bpe_raw_expansion_replay_case_count",
        summary.bpe_raw_expansion_replay_case_count,
    )?;
    require_positive_usize(
        "admissible_token_mask_contract.bpe_raw_expansion_first_failure_case_count",
        summary.bpe_raw_expansion_first_failure_case_count,
    )?;
    Ok(summary)
}

fn prefix_state_continuation_summary(
    integration_gate: &SyntaxGeneratedCodecStateApiIntegrationGate,
) -> Result<PrefixStateContinuationSummary, SyntaxBackendBoundaryError> {
    let mask = admissible_mask_summary(integration_gate)?;
    let raw_prefix_request_count = route_owned_record_queries(integration_gate)
        .into_iter()
        .map(|record| record.prefix_state_case_ids.len())
        .sum();
    require_positive_usize(
        "prefix_state_continuation_contract.raw_prefix_request_count",
        raw_prefix_request_count,
    )?;
    Ok(PrefixStateContinuationSummary {
        record_query_count: mask.record_query_count,
        raw_prefix_request_count,
        finite_admissible_response_record_count: mask.finite_mask_record_count,
        payload_range_response_record_count: mask.payload_range_mask_record_count,
        eof_validity_response_record_count: mask.eof_validity_record_count,
        bpe_raw_expansion_replay_case_count: mask.bpe_raw_expansion_replay_case_count,
        bpe_raw_expansion_first_failure_case_count: mask.bpe_raw_expansion_first_failure_case_count,
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

fn require_u32_slice(
    field: &'static str,
    actual: &[u32],
    expected: &[u32],
) -> Result<(), SyntaxBackendBoundaryError> {
    if actual == expected {
        Ok(())
    } else {
        Err(SyntaxBackendBoundaryError::new(
            field,
            format!("expected {:?}, got {:?}", expected, actual),
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

fn require_json_hash<T: Serialize>(
    field: &'static str,
    value: &T,
    expected: &str,
) -> Result<(), SyntaxBackendBoundaryError> {
    let actual = json_sha256(value).map_err(|error| {
        SyntaxBackendBoundaryError::new(field, format!("failed to hash JSON value: {error}"))
    })?;
    require_string(field, &actual, expected)
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
        .filter(|entry| !actual.contains(entry))
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

fn require_json_string(
    field: &'static str,
    object: &Value,
    key: &str,
    expected: &str,
) -> Result<(), SyntaxBackendBoundaryError> {
    let actual = object.get(key).and_then(Value::as_str).ok_or_else(|| {
        SyntaxBackendBoundaryError::new(field, format!("missing string field {key}"))
    })?;
    require_string(field, actual, expected)
}

fn require_json_bool(
    field: &'static str,
    object: &Value,
    key: &str,
    expected: bool,
) -> Result<(), SyntaxBackendBoundaryError> {
    let actual = object.get(key).and_then(Value::as_bool).ok_or_else(|| {
        SyntaxBackendBoundaryError::new(field, format!("missing bool field {key}"))
    })?;
    require_bool(field, actual, expected)
}

fn require_json_usize(
    field: &'static str,
    object: &Value,
    key: &str,
    expected: usize,
) -> Result<(), SyntaxBackendBoundaryError> {
    let actual = object.get(key).and_then(Value::as_u64).ok_or_else(|| {
        SyntaxBackendBoundaryError::new(field, format!("missing integer field {key}"))
    })?;
    require_usize(field, actual as usize, expected)
}

fn require_json_all(
    field: &'static str,
    object: &Value,
    key: &str,
    required: &[&str],
) -> Result<(), SyntaxBackendBoundaryError> {
    let values = object
        .get(key)
        .and_then(Value::as_array)
        .ok_or_else(|| {
            SyntaxBackendBoundaryError::new(field, format!("missing array field {key}"))
        })?
        .iter()
        .map(|value| {
            value.as_str().map(str::to_string).ok_or_else(|| {
                SyntaxBackendBoundaryError::new(field, format!("array field {key} is not strings"))
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    require_all(field, &values, required)
}
