//! Go source-record raw-token generator core.
//!
//! Owns the reusable source-record → raw-token generator-core boundary for the
//! pinned Go inventory artifact, so the tokenizer-generator megamodule only
//! supplies oracle evidence plus record-readiness / representative views when
//! building the gate, and only wires generated-codec parity, raw-tree render
//! attempts, and non-claim report-level glue when building records.
//!
//! # Evidence builders
//!
//! Empty-prefix, expanded selected-completion, elided selected-completion,
//! incomplete-prefix rejection, and token-only decode evidence.
//!
//! # Inventory scratch-seed surfaces
//!
//! - Token-only and expanded selected-completion sample-step walks.
//! - Elided selected-completion — canonical elided stream via
//!   [`RawScratchGeneratorCore::elided_selected_completion`].
//! - Incomplete-prefix rejection — multi-token seeds via shared
//!   [`RawScratchGeneratorCore::incomplete_prefix`].
//! - Inventory-model empty-prefix on the scratch-seed batch — shared
//!   [`RawScratchGeneratorCore::empty_prefix`] against the inventory compiled
//!   model / `GoSourceFile` root.
//! - Evidence summary construction — frontier coverage plus render/reparse
//!   report stamping.
//! - Batch assembly — inventory lower → structural model → compiled model →
//!   empty-prefix, then seed list → per-seed summaries → aggregates.
//! - Seed+empty-prefix sub-gate bundle — batch, generator-core API stamps, and
//!   fixed claim flags.
//!
//! # Manifest-depth aggregation
//!
//! - Per-record readiness: empty-prefix, expanded and elided
//!   selected-completion, incomplete-prefix, token-only, and render-reparse
//!   counts over record views.
//! - Full scratch-generation gate assembly: status/scope/claim constants,
//!   oracle cutover attachment, representative path stamping, and the
//!   inventory-sub + record-readiness merge into
//!   `GoTokenizerGeneratorManifestDepthScratchGenerationGate` — including
//!   `elided_selected_completion_ready` parallel to
//!   `selected_completion_smoke_ready`.
//! - Record source-field stamping: `GoSourceRecordManifestDepthRecordSourceFields`
//!   from `GoSourceRecordGeneratorEvidence` — API/stream identity plus
//!   empty-prefix, selected-completion, elided selected-completion,
//!   incomplete-prefix, and token-only decode-render.
//!
//! # Report-level readiness (derived, never hardcoded)
//!
//! - [`GoSourceRecordManifestDepthReportReadiness::from_scratch_generation_gate`]
//!   — expanded and elided selected-completion, incomplete-prefix, token-only.
//! - [`GoSourceRecordManifestDepthVocabSmCompleteness::from_record_views`] —
//!   `raw_tokens_below_vocab_size`, `pad_token_absent`,
//!   `raw_state_machine_complete`.
//! - [`GoSourceRecordManifestDepthCanonicalRenderReadiness::from_record_views`]
//!   — report `canonical_render_ready` plus pipeline `canonical_render_ready` /
//!   `go_reparse_ready`.
//! - [`GoSourceRecordManifestDepthEncodeDecodeReadiness::from_vocab_sm_completeness`]
//!   — `raw_token_encode_ready` / `raw_state_machine_decode_ready`.
//!
//! # Pipeline claim stamps
//!
//! - Process-only stage / exclusion constant bundles:
//!   [`GoSourceRecordPipelineProcessStageClaims::SUCCESS`] for parser-backed
//!   metadata, GrammarIR, StructuralModel, token layout, and raw-SM compile;
//!   [`GoSourceRecordPipelineProcessExclusionClaims::SUCCESS`] for the ArenaIR /
//!   route-handoff / BPE-authority flags that must stay false.
//! - [`GoSourceRecordPipelineFieldStamp`] folds process stages, exclusions,
//!   encode/decode, and canonical-render readiness into one view.
//! - [`GoSourceRecordManifestDepthReportClaimStamp::from_gate_and_record_views`]
//!   folds vocab/SM completeness, gate-derived readiness, canonical-render, and
//!   the full pipeline stamp, so the megamodule maps one report-claim stamp
//!   instead of composing four readiness aggregators plus the pipeline at each
//!   construction site.
//! - [`GoSourceRecordCanonicalRenderReadinessFlags::from_checked_report`] /
//!   [`from_canonical_render`](GoSourceRecordCanonicalRenderReadinessFlags::from_canonical_render)
//!   — gofmt, go_parser, matches_source, and `canonical_sha256 == source.sha256`
//!   conjoined into `canonical_render_ready` / `go_reparse_ready`, so the
//!   single-record report path, selected-completion smoke, and token-only
//!   decode/render cannot hardcode these independently or drift on whether
//!   sha256 is included.
//!
//! # Claim boundary
//!
//! C# stays an expanded-contract cross-check (`minimal_class`) only. This module
//! does not flip production-route promotion.

use crate::go_ast_schema_inventory::{
    GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH, GoAstSchemaGrammarCoverageReport,
    GoAstSchemaInventoryAdapter,
};
use crate::grammar_ir_structural_model::{
    GrammarStructuralModelSidecar, lower_grammar_ir_to_structural_model,
};
use crate::provenance::{json_sha256, sha256_hex};
use crate::syntax_backend_boundary::SyntaxBackendBoundaryError;
use crate::syntax_generated_codec::go_syntax_token_layout;
use crate::syntax_go_inventory_synthesis::GO_AST_INVENTORY_SYNTHESIS_CORE_API_ID;
use crate::syntax_go_raw_tree_renderer::render_go_source_from_raw_tree;
use crate::syntax_go_scratch_seed::{
    GO_AST_INVENTORY_SCRATCH_VAR_DECL_ROOT, GO_AST_INVENTORY_SCRATCH_VAR_DECL_SEED_ID,
    GoScratchSeedStream, go_ast_inventory_scratch_seeds,
};
use crate::syntax_go_source_instance_projection::{
    SourceInstanceCanonicalRender, SourceInstanceProjectionRawTokenIds,
    SyntaxGoSourceInstanceProjectionReport,
};
use crate::syntax_go_source_record_raw_token::{
    GO_SOURCE_RECORD_RAW_TOKEN_EMISSION_API_BOUNDARY, GO_SOURCE_RECORD_RAW_TOKEN_EMISSION_API_ID,
    GoSourceRecordRawTokenEmission,
};
use crate::syntax_roadmap_successors::GO_SOURCE_RECORD_RAW_TOKEN_GENERATOR_CORE_SUCCESSOR_TASK;
use crate::syntax_scratch_generation::{
    RAW_SCRATCH_GENERATOR_CORE_API_BOUNDARY, RAW_SCRATCH_GENERATOR_CORE_API_ID,
    RawScratchClaimBoundaryFlags, RawScratchGeneratorCore, RawScratchTokenOnlyDecodeResult,
    RawScratchTokenOnlyDecodeSummary,
};
use crate::syntax_state_machine::{
    CompiledSyntaxModel, RAW_SYNTAX_STATE_MACHINE_CORE_BOUNDARY,
    RAW_SYNTAX_STATE_MACHINE_CORE_BOUNDARY_ID, RawSyntaxDecodedNode, RawSyntaxDecodedTree,
    RawSyntaxDecodedValue, RawSyntaxStateMachineCore, RawSyntaxStatus,
};
use crate::syntax_token_layout_diff::LIVE_RAW_SYNTAX_STREAM_CONTRACT;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::path::Path;

/// Matches `syntax_go_tokenizer_generator::VERSION` for precompiled artifact
/// evidence stamped by the source-record generator core.
const PRECOMPILED_ARTIFACT_VERSION: u32 = 1;
const NEXT_READY_TASK: &str = GO_SOURCE_RECORD_RAW_TOKEN_GENERATOR_CORE_SUCCESSOR_TASK;

pub const GO_SOURCE_RECORD_GENERATOR_CORE_API_ID: &str = "go-source-record-generator-core-v1";
pub const GO_SOURCE_RECORD_GENERATOR_CORE_API_BOUNDARY: &str = "checked Go parser metadata/projection source record + precompiled syntax model -> raw scratch generator evidence bundle including expanded selected-completion and elided canonical-stream selected-completion; render/gofmt/reparse remain sidecar validation";
pub const GO_SOURCE_RECORD_ELIDED_SELECTED_COMPLETION_STATUS: &str =
    "go_source_record_elided_selected_completion_ready";
pub const GO_SOURCE_RECORD_ELIDED_SELECTED_COMPLETION_SCOPE: &str =
    "precompiled_go_source_file_elided_stream_selected_completion_v1";
pub const GO_SOURCE_RECORD_ELIDED_SELECTED_COMPLETION_CLAIM_BOUNDARY: &str = "syntax-level Go source-record generator-core elided selected-completion evidence: the checked source-record raw-token emission's canonical elided stream is walked through the shared RawScratchGeneratorCore::elided_selected_completion surface (CompiledSyntaxModel begin_elided_stream), every selected elided token is admitted at a non-forced position, expand_elided_raw_tokens reproduces the expanded stream, and token-only decode via the elided path yields a reusable raw syntax tree. Expanded selected-completion remains SM-native evidence. This is not per-output parser metadata, not source-instance segment sidecar authority, not Arena IR evidence, not route-handoff provenance, not BPE validity authority, and not stable-backend promotion.";
pub const EMPTY_PREFIX_GENERATION_STATUS: &str = "go_tokenizer_generator_empty_prefix_state_ready";
pub const EMPTY_PREFIX_GENERATION_SCOPE: &str = "precompiled_go_source_file_empty_prefix_v1";
pub const SELECTED_COMPLETION_SMOKE_STATUS: &str =
    "go_tokenizer_generator_selected_completion_smoke_ready";
pub const SELECTED_COMPLETION_SMOKE_SCOPE: &str =
    "precompiled_go_source_file_selected_completion_v1";
pub const INCOMPLETE_PREFIX_REJECTION_STATUS: &str =
    "go_tokenizer_generator_incomplete_prefix_rejection_ready";
pub const INCOMPLETE_PREFIX_REJECTION_SCOPE: &str =
    "precompiled_go_source_file_incomplete_prefix_rejection_v1";
pub const SELECTED_COMPLETION_STREAM_SOURCE: &str = "checked_token_only_stream_selected_by_mask";
pub const TOKEN_ONLY_DECODE_RENDER_STATUS: &str =
    "go_tokenizer_generator_token_only_decode_render_ready";
pub const TOKEN_ONLY_DECODE_RENDER_SCOPE: &str = "precompiled_go_source_file_raw_token_stream_v1";
pub const TOKEN_ONLY_RAW_TOKEN_STREAM_SOURCE: &str = "raw_token_ids_only";
pub const PRECOMPILED_GO_SYNTAX_TOKENIZER_ARTIFACT_KIND: &str =
    "precompiled_go_syntax_tokenizer_artifact_v1";
pub const EMPTY_PREFIX_ROOT: &str = "GoSourceFile";
pub const EMPTY_PREFIX_GENERATION_CLAIM_BOUNDARY: &str = "syntax-level Go tokenizer-generator empty-prefix evidence: inference starts from a precompiled Go syntax tokenizer artifact, constructs the empty GoSourceFile raw-prefix state through GrammarIR -> StructuralModel -> token layout -> raw syntax state machine, queries a finite admissible raw-token mask, rejects PAD and malformed raw tokens, and applies an admitted raw syntax transition without per-output parser metadata. This is not Arena IR evidence, not route-handoff provenance, not source-prefix table evidence, not BPE validity authority, and not stable-backend promotion.";
pub const SELECTED_COMPLETION_SMOKE_CLAIM_BOUNDARY: &str = "syntax-level Go tokenizer-generator selected-completion evidence: inference starts from a precompiled Go syntax tokenizer artifact and an empty GoSourceFile prefix, repeatedly queries the raw syntax state machine for a finite admissible raw-token mask, selects only admitted checked-stream raw tokens, reaches an EOF-valid complete raw syntax stream, and ties token-only decode plus render/reparse sidecar validation to the same raw token hash. This is not per-output parser metadata, not source-instance segment sidecar authority, not source-prefix table evidence, not Arena IR evidence, not route-handoff provenance, not BPE validity authority, and not stable-backend promotion.";
pub const INCOMPLETE_PREFIX_REJECTION_CLAIM_BOUNDARY: &str = "syntax-level Go tokenizer-generator incomplete-prefix evidence: a selected token-only prefix from the same precompiled Go syntax tokenizer artifact is accepted as partial, rejected by strict completion semantics before EOF is admitted, preserves consumed-token accounting, exposes the finite continuation mask, and then continues through an admitted selected raw token toward the same complete render/reparse stream. This is not per-output parser metadata, not source-instance segment sidecar authority, not source-prefix table evidence, not Arena IR evidence, not route-handoff provenance, not BPE validity authority, and not stable-backend promotion.";
pub const TOKEN_ONLY_DECODE_RENDER_CLAIM_BOUNDARY: &str = "syntax-level Go tokenizer-generator token-only decode/render evidence: a raw token-only stream for a precompiled Go syntax tokenizer artifact decodes through the target-neutral raw syntax state machine into a reusable raw syntax tree, then ties canonical Go render, gofmt, and go/parser reparse sidecar validation to the same raw token hash. This is not per-output parser metadata, not source-instance segment sidecar authority, not Arena IR evidence, not route-handoff provenance, not BPE validity authority, and not stable-backend promotion.";

// Full scratch-generation gate constants (generator-core owned assembly).
/// Full empty-prefix scratch-generation gate status stamped on
/// [`GoTokenizerGeneratorManifestDepthScratchGenerationGate`].
pub const SCRATCH_GENERATION_GATE_STATUS: &str =
    "go_tokenizer_generator_empty_prefix_scratch_generation_gate_ready";
pub const SCRATCH_GENERATION_GATE_SCOPE: &str =
    "manifest_depth_empty_prefix_scratch_generation_gate_v1";
pub const SCRATCH_GENERATION_GATE_RENDER_AUTHORITY_SOURCE: &str =
    "accepted_go_ast_printer_oracle_cutover";
pub const SCRATCH_GENERATION_GATE_NEXT_READY_TASK: &str =
    GO_SOURCE_RECORD_RAW_TOKEN_GENERATOR_CORE_SUCCESSOR_TASK;
pub const SCRATCH_GENERATION_GATE_CLAIM_BOUNDARY: &str = "syntax-level Go tokenizer-generator empty-prefix scratch-generation gate: manifest-depth evidence consumes the accepted grammar-mode Go AST printer oracle cutover as render/reparse authority, starts inference from a precompiled Go syntax tokenizer artifact and an empty GoSourceFile raw prefix, records finite admissible-token masks, admitted selected transitions, strict incomplete-prefix rejection, token-only decode, canonical render, and parser sidecar validation, and keeps canonical-fidelity-only gaps diagnostic. This is raw syntax state-machine runtime evidence only, not per-output parser metadata, not source-instance segment sidecar authority, not source-prefix table evidence, not Arena IR evidence, not route-handoff provenance, not BPE validity authority, not a source-program validity claim, and not stable-backend promotion.";
/// Grammar-mode oracle cutover accepted status required by the scratch-generation gate.
pub const SCRATCH_GENERATION_GATE_ORACLE_CUTOVER_STATUS_ACCEPTED: &str =
    "go_ast_printer_oracle_grammar_mode_cutover_accepted";

// Inventory scratch-seed evidence summary constants (generator-core owned).
pub const GO_AST_INVENTORY_SCRATCH_SEED_STATUS: &str =
    "go_ast_inventory_scratch_seed_manifest_depth_ready";
pub const GO_AST_INVENTORY_SCRATCH_SEED_SCOPE: &str =
    "manifest_depth_go_ast_inventory_scratch_seed_v1";
pub const GO_AST_INVENTORY_SCRATCH_SEED_STREAM_SOURCE: &str =
    "go_ast_inventory_scratch_var_decl_seed";
pub const GO_AST_INVENTORY_SCRATCH_SEED_CLAIM_BOUNDARY: &str = "syntax-level Go tokenizer-generator scratch seed evidence: the manifest-depth gate consumes Go AST inventory token-only package var-declaration, alias import-spec, function-return, function-type result, array-type return, channel-type return, selector-call/member-access, index-expression, index-list expression, slice-expression, star-expression, struct-type, type-assertion expression, interface-type, key-value expression, assignment-statement, binary-expression, unary-expression, parenthesized-expression, composite-literal, array-literal Ellipsis, function-literal expression, declaration-statement, defer-statement, go-statement, explicit empty-statement, expression-statement, increment/decrement statement, branch-statement, range-statement, send-statement, labeled-statement, case-clause, type-switch statement, and comm-clause seeds, selects each raw token from raw state-machine admissible masks, decodes each token-only stream into a reusable raw syntax tree, renders canonical Go, and keeps gofmt/go-parser validation as sidecar evidence. This is not per-output parser metadata, not source-instance segment sidecar authority, not source-prefix table evidence, not Arena IR evidence, not route-handoff provenance, not BPE validity authority, not a source-program validity claim, and not stable-backend promotion.";
pub const GO_AST_INVENTORY_SCRATCH_SEED_ELIDED_SELECTED_COMPLETION_STATUS: &str =
    "go_ast_inventory_scratch_seed_elided_selected_completion_ready";
pub const GO_AST_INVENTORY_SCRATCH_SEED_ELIDED_SELECTED_COMPLETION_SCOPE: &str =
    "manifest_depth_go_ast_inventory_scratch_seed_elided_stream_selected_completion_v1";
pub const GO_AST_INVENTORY_SCRATCH_SEED_ELIDED_SELECTED_COMPLETION_CLAIM_BOUNDARY: &str = "syntax-level Go inventory scratch-seed generator-core elided selected-completion evidence: each inventory seed's expanded SM-native raw-token stream is elided via CompiledSyntaxModel::elide_forced_raw_tokens, walked through the shared RawScratchGeneratorCore::elided_selected_completion surface (begin_elided_stream), every selected elided token is admitted at a non-forced position, expand_elided_raw_tokens reproduces the expanded seed stream, and token-only decode via the elided path yields a reusable raw syntax tree. Expanded seed admission remains SM-native evidence. This is not per-output parser metadata, not source-instance segment sidecar authority, not Arena IR evidence, not route-handoff provenance, not BPE validity authority, not a source-program validity claim, and not stable-backend promotion.";
pub const GO_AST_INVENTORY_SCRATCH_SEED_INCOMPLETE_PREFIX_REJECTION_STATUS: &str =
    "go_ast_inventory_scratch_seed_incomplete_prefix_rejection_ready";
pub const GO_AST_INVENTORY_SCRATCH_SEED_INCOMPLETE_PREFIX_REJECTION_SCOPE: &str =
    "manifest_depth_go_ast_inventory_scratch_seed_incomplete_prefix_rejection_v1";
pub const GO_AST_INVENTORY_SCRATCH_SEED_INCOMPLETE_PREFIX_REJECTION_CLAIM_BOUNDARY: &str = "syntax-level Go inventory scratch-seed generator-core incomplete-prefix evidence: a multi-token inventory seed's selected token-only prefix is accepted as partial, rejected by strict completion semantics before EOF is admitted, preserves consumed-token accounting, exposes the finite continuation mask, and then continues through an admitted selected raw token toward the same complete seed stream via the shared RawScratchGeneratorCore::incomplete_prefix surface. Expanded seed admission remains SM-native evidence. This is not per-output parser metadata, not source-instance segment sidecar authority, not source-prefix table evidence, not Arena IR evidence, not route-handoff provenance, not BPE validity authority, not a source-program validity claim, and not stable-backend promotion.";
pub const GO_AST_INVENTORY_SCRATCH_SEED_EMPTY_PREFIX_STATUS: &str =
    "go_ast_inventory_scratch_seed_empty_prefix_ready";
pub const GO_AST_INVENTORY_SCRATCH_SEED_EMPTY_PREFIX_SCOPE: &str =
    "manifest_depth_go_ast_inventory_scratch_seed_empty_prefix_v1";
pub const GO_AST_INVENTORY_SCRATCH_SEED_EMPTY_PREFIX_CLAIM_BOUNDARY: &str = "syntax-level Go inventory scratch-seed generator-core empty-prefix evidence: the inventory compiled model constructs the empty GoSourceFile raw-prefix state through the shared RawScratchGeneratorCore::empty_prefix surface, queries a finite admissible raw-token mask, rejects PAD and malformed raw tokens, and applies an admitted raw syntax transition without per-output parser metadata. Inventory seeds share this generator-core batch surface rather than only source-record empty-prefix. Expanded seed admission remains SM-native evidence. This is not per-output parser metadata, not source-instance segment sidecar authority, not source-prefix table evidence, not Arena IR evidence, not route-handoff provenance, not BPE validity authority, not a source-program validity claim, and not stable-backend promotion.";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoTokenizerGeneratorDecodedTreeSummary {
    pub event_count: usize,
    pub node_start_count: usize,
    pub field_start_count: usize,
    pub payload_event_count: usize,
    pub payload_token_byte_count: usize,
    pub payload_value_count: usize,
    pub root_rule: String,
    pub root_node_kind: String,
    pub consumed_raw_tokens: usize,
    pub eof_valid: bool,
}

impl GoTokenizerGeneratorDecodedTreeSummary {
    pub(crate) fn from_tree(
        tree: &RawSyntaxDecodedTree,
    ) -> Result<Self, SyntaxBackendBoundaryError> {
        let root_rule = tree.root_rule().ok_or_else(|| {
            SyntaxBackendBoundaryError::new(
                "raw_state_machine_tree.root_rule",
                "decoded raw syntax tree must include a root node",
            )
        })?;
        let root_node_kind = tree.root_node_kind().ok_or_else(|| {
            SyntaxBackendBoundaryError::new(
                "raw_state_machine_tree.root_node_kind",
                "decoded raw syntax tree must include a root node kind",
            )
        })?;
        Ok(Self {
            event_count: tree.events.len(),
            node_start_count: tree.node_start_count(),
            field_start_count: tree.field_start_count(),
            payload_event_count: tree.payload_event_count(),
            payload_token_byte_count: tree.payload_token_byte_count(),
            payload_value_count: tree.payload_value_count(),
            root_rule: root_rule.to_string(),
            root_node_kind: root_node_kind.to_string(),
            consumed_raw_tokens: tree.state.consumed_raw_tokens,
            eof_valid: tree.state.eof_valid,
        })
    }

    pub(crate) fn validate(
        &self,
        raw_token_count: usize,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        require_positive_usize("raw_state_machine_tree.event_count", self.event_count)?;
        require_positive_usize(
            "raw_state_machine_tree.node_start_count",
            self.node_start_count,
        )?;
        require_positive_usize(
            "raw_state_machine_tree.field_start_count",
            self.field_start_count,
        )?;
        require_positive_usize(
            "raw_state_machine_tree.payload_event_count",
            self.payload_event_count,
        )?;
        require_positive_usize(
            "raw_state_machine_tree.payload_token_byte_count",
            self.payload_token_byte_count,
        )?;
        require_positive_usize(
            "raw_state_machine_tree.payload_value_count",
            self.payload_value_count,
        )?;
        if self.payload_token_byte_count > self.payload_event_count {
            return Err(SyntaxBackendBoundaryError::new(
                "raw_state_machine_tree.payload_token_byte_count",
                "payload byte events cannot exceed total payload events",
            ));
        }
        if self.payload_value_count > self.payload_event_count {
            return Err(SyntaxBackendBoundaryError::new(
                "raw_state_machine_tree.payload_value_count",
                "payload value events cannot exceed total payload events",
            ));
        }
        require_non_empty("raw_state_machine_tree.root_rule", &self.root_rule)?;
        require_non_empty(
            "raw_state_machine_tree.root_node_kind",
            &self.root_node_kind,
        )?;
        require_usize(
            "raw_state_machine_tree.consumed_raw_tokens",
            self.consumed_raw_tokens,
            raw_token_count,
        )?;
        require_bool("raw_state_machine_tree.eof_valid", self.eof_valid, true)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoTokenizerGeneratorTokenOnlyDecodeRenderEvidence {
    pub status: String,
    pub scope: String,
    pub precompiled_artifact_kind: String,
    pub precompiled_artifact_version: u32,
    pub precompiled_artifact_grammar_id: String,
    pub precompiled_artifact_metadata_path: String,
    pub precompiled_artifact_metadata_sha256: String,
    pub precompiled_artifact_structural_model_id: String,
    pub raw_state_machine_core_boundary_id: String,
    pub raw_state_machine_core_boundary: String,
    pub generator_core_api_id: String,
    pub generator_core_api_boundary: String,
    pub raw_token_stream_source: String,
    pub raw_token_count: usize,
    pub raw_token_ids_sha256: String,
    pub token_layout_vocab_size: u32,
    pub pad_token_id: u32,
    pub decoded_from_token_only_stream: bool,
    pub decoded_tree_reusable: bool,
    pub decoded_tree: GoTokenizerGeneratorDecodedTreeSummary,
    pub canonical_render: SourceInstanceCanonicalRender,
    pub canonical_render_ready: bool,
    pub gofmt_validation: String,
    pub go_parser_reparse_validation: String,
    pub render_reparse_sidecar_validation: bool,
    pub per_output_parser_metadata_required: bool,
    pub source_instance_segment_sidecar_used: bool,
    pub source_prefix_table_used: bool,
    pub arena_ir_used: bool,
    pub route_handoff_provenance_used: bool,
    pub bpe_validity_authority_used: bool,
    pub stable_backend_claimed: bool,
    pub next_ready_task: String,
    pub claim_boundary: String,
}

impl GoTokenizerGeneratorTokenOnlyDecodeRenderEvidence {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn validate(
        &self,
        candidate_grammar_id: &str,
        metadata_path: &str,
        metadata_sha256: &str,
        structural_model_id: &str,
        token_layout_vocab_size: u32,
        token_layout_pad_token: u32,
        raw_token_count: usize,
        raw_token_ids_sha256: &str,
        source_sha256: &str,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        require_string(
            "token_only_decode_render.status",
            &self.status,
            TOKEN_ONLY_DECODE_RENDER_STATUS,
        )?;
        require_string(
            "token_only_decode_render.scope",
            &self.scope,
            TOKEN_ONLY_DECODE_RENDER_SCOPE,
        )?;
        require_string(
            "token_only_decode_render.precompiled_artifact_kind",
            &self.precompiled_artifact_kind,
            PRECOMPILED_GO_SYNTAX_TOKENIZER_ARTIFACT_KIND,
        )?;
        require_u32(
            "token_only_decode_render.precompiled_artifact_version",
            self.precompiled_artifact_version,
            PRECOMPILED_ARTIFACT_VERSION,
        )?;
        require_string(
            "token_only_decode_render.precompiled_artifact_grammar_id",
            &self.precompiled_artifact_grammar_id,
            candidate_grammar_id,
        )?;
        require_non_empty(
            "token_only_decode_render.precompiled_artifact_metadata_path",
            &self.precompiled_artifact_metadata_path,
        )?;
        require_string(
            "token_only_decode_render.precompiled_artifact_metadata_path",
            &self.precompiled_artifact_metadata_path,
            metadata_path,
        )?;
        require_sha256(
            "token_only_decode_render.precompiled_artifact_metadata_sha256",
            &self.precompiled_artifact_metadata_sha256,
        )?;
        require_string(
            "token_only_decode_render.precompiled_artifact_metadata_sha256",
            &self.precompiled_artifact_metadata_sha256,
            metadata_sha256,
        )?;
        require_non_empty(
            "token_only_decode_render.precompiled_artifact_structural_model_id",
            &self.precompiled_artifact_structural_model_id,
        )?;
        require_string(
            "token_only_decode_render.precompiled_artifact_structural_model_id",
            &self.precompiled_artifact_structural_model_id,
            structural_model_id,
        )?;
        require_string(
            "token_only_decode_render.raw_state_machine_core_boundary_id",
            &self.raw_state_machine_core_boundary_id,
            RAW_SYNTAX_STATE_MACHINE_CORE_BOUNDARY_ID,
        )?;
        require_string(
            "token_only_decode_render.raw_state_machine_core_boundary",
            &self.raw_state_machine_core_boundary,
            RAW_SYNTAX_STATE_MACHINE_CORE_BOUNDARY,
        )?;
        require_string(
            "token_only_decode_render.generator_core_api_id",
            &self.generator_core_api_id,
            RAW_SCRATCH_GENERATOR_CORE_API_ID,
        )?;
        require_string(
            "token_only_decode_render.generator_core_api_boundary",
            &self.generator_core_api_boundary,
            RAW_SCRATCH_GENERATOR_CORE_API_BOUNDARY,
        )?;
        require_string(
            "token_only_decode_render.raw_token_stream_source",
            &self.raw_token_stream_source,
            TOKEN_ONLY_RAW_TOKEN_STREAM_SOURCE,
        )?;
        require_usize(
            "token_only_decode_render.raw_token_count",
            self.raw_token_count,
            raw_token_count,
        )?;
        require_string(
            "token_only_decode_render.raw_token_ids_sha256",
            &self.raw_token_ids_sha256,
            raw_token_ids_sha256,
        )?;
        require_u32(
            "token_only_decode_render.token_layout_vocab_size",
            self.token_layout_vocab_size,
            token_layout_vocab_size,
        )?;
        require_u32(
            "token_only_decode_render.pad_token_id",
            self.pad_token_id,
            token_layout_pad_token,
        )?;
        require_bool(
            "token_only_decode_render.decoded_from_token_only_stream",
            self.decoded_from_token_only_stream,
            true,
        )?;
        require_bool(
            "token_only_decode_render.decoded_tree_reusable",
            self.decoded_tree_reusable,
            true,
        )?;
        self.decoded_tree.validate(raw_token_count)?;
        require_string(
            "token_only_decode_render.canonical_render.formatter",
            &self.canonical_render.formatter,
            "go/format.Source",
        )?;
        require_string(
            "token_only_decode_render.canonical_render.gofmt",
            &self.canonical_render.gofmt,
            "passed",
        )?;
        require_string(
            "token_only_decode_render.canonical_render.go_parser",
            &self.canonical_render.go_parser,
            "passed",
        )?;
        require_bool(
            "token_only_decode_render.canonical_render.matches_source",
            self.canonical_render.matches_source,
            true,
        )?;
        require_sha256(
            "token_only_decode_render.canonical_render.canonical_sha256",
            &self.canonical_render.canonical_sha256,
        )?;
        require_string(
            "token_only_decode_render.canonical_render.canonical_sha256",
            &self.canonical_render.canonical_sha256,
            source_sha256,
        )?;
        require_positive_usize(
            "token_only_decode_render.canonical_render.canonical_byte_count",
            self.canonical_render.canonical_byte_count,
        )?;
        require_bool(
            "token_only_decode_render.canonical_render_ready",
            self.canonical_render_ready,
            true,
        )?;
        require_string(
            "token_only_decode_render.gofmt_validation",
            &self.gofmt_validation,
            "passed",
        )?;
        require_string(
            "token_only_decode_render.go_parser_reparse_validation",
            &self.go_parser_reparse_validation,
            "passed",
        )?;
        require_bool(
            "token_only_decode_render.render_reparse_sidecar_validation",
            self.render_reparse_sidecar_validation,
            true,
        )?;
        require_bool(
            "token_only_decode_render.per_output_parser_metadata_required",
            self.per_output_parser_metadata_required,
            false,
        )?;
        require_bool(
            "token_only_decode_render.source_instance_segment_sidecar_used",
            self.source_instance_segment_sidecar_used,
            false,
        )?;
        require_bool(
            "token_only_decode_render.source_prefix_table_used",
            self.source_prefix_table_used,
            false,
        )?;
        require_bool(
            "token_only_decode_render.arena_ir_used",
            self.arena_ir_used,
            false,
        )?;
        require_bool(
            "token_only_decode_render.route_handoff_provenance_used",
            self.route_handoff_provenance_used,
            false,
        )?;
        require_bool(
            "token_only_decode_render.bpe_validity_authority_used",
            self.bpe_validity_authority_used,
            false,
        )?;
        require_bool(
            "token_only_decode_render.stable_backend_claimed",
            self.stable_backend_claimed,
            false,
        )?;
        require_string(
            "token_only_decode_render.next_ready_task",
            &self.next_ready_task,
            NEXT_READY_TASK,
        )?;
        require_str_contains(
            "token_only_decode_render.claim_boundary",
            &self.claim_boundary,
            "syntax-level",
        )?;
        require_str_contains(
            "token_only_decode_render.claim_boundary",
            &self.claim_boundary,
            "not per-output parser metadata",
        )?;
        require_str_contains(
            "token_only_decode_render.claim_boundary",
            &self.claim_boundary,
            "not source-instance segment sidecar",
        )?;
        require_str_contains(
            "token_only_decode_render.claim_boundary",
            &self.claim_boundary,
            "not Arena IR",
        )?;
        require_str_contains(
            "token_only_decode_render.claim_boundary",
            &self.claim_boundary,
            "not BPE",
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoTokenizerGeneratorEmptyPrefixGenerationEvidence {
    pub status: String,
    pub scope: String,
    pub precompiled_artifact_kind: String,
    pub precompiled_artifact_version: u32,
    pub precompiled_artifact_grammar_id: String,
    pub precompiled_artifact_metadata_path: String,
    pub precompiled_artifact_metadata_sha256: String,
    pub precompiled_artifact_structural_model_id: String,
    pub raw_state_machine_core_boundary_id: String,
    pub raw_state_machine_core_boundary: String,
    pub generator_core_api_id: String,
    pub generator_core_api_boundary: String,
    pub token_layout_vocab_size: u32,
    pub pad_token_id: u32,
    pub empty_prefix_root: String,
    pub structural_root_rule: String,
    pub structural_root_node_kind: String,
    pub initial_state_observed_status: String,
    pub initial_state_consumed_raw_tokens: usize,
    pub initial_state_total_raw_tokens: usize,
    pub initial_state_eof_valid: bool,
    pub initial_state_stack_depth: usize,
    pub initial_state_active_rule: Option<String>,
    pub initial_state_active_field: Option<String>,
    pub finite_admissible_raw_token_count: usize,
    pub finite_admissible_raw_token_ids: Vec<u32>,
    pub finite_admissible_raw_token_ids_sha256: String,
    pub selected_raw_token_id: u32,
    pub selected_raw_token_admitted: bool,
    pub selected_transition_accepted: bool,
    pub selected_transition_observed_status: String,
    pub selected_transition_consumed_raw_tokens: usize,
    pub selected_transition_eof_valid: bool,
    pub selected_transition_stack_depth: usize,
    pub selected_transition_active_rule: Option<String>,
    pub selected_transition_active_field: Option<String>,
    pub pad_token_rejected: bool,
    pub pad_token_error_kind: Option<String>,
    pub malformed_raw_token_id: u32,
    pub malformed_raw_token_rejected: bool,
    pub malformed_raw_token_error_kind: Option<String>,
    pub per_output_parser_metadata_required: bool,
    pub source_instance_segment_sidecar_used: bool,
    pub source_prefix_table_used: bool,
    pub arena_ir_used: bool,
    pub route_handoff_provenance_used: bool,
    pub bpe_validity_authority_used: bool,
    pub stable_backend_claimed: bool,
    pub next_ready_task: String,
    pub claim_boundary: String,
}

impl GoTokenizerGeneratorEmptyPrefixGenerationEvidence {
    pub(crate) fn validate(
        &self,
        candidate_grammar_id: &str,
        metadata_path: &str,
        metadata_sha256: &str,
        structural_model_id: &str,
        token_layout_vocab_size: u32,
        token_layout_pad_token: u32,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        require_string(
            "empty_prefix_generation.status",
            &self.status,
            EMPTY_PREFIX_GENERATION_STATUS,
        )?;
        require_string(
            "empty_prefix_generation.scope",
            &self.scope,
            EMPTY_PREFIX_GENERATION_SCOPE,
        )?;
        require_string(
            "empty_prefix_generation.precompiled_artifact_kind",
            &self.precompiled_artifact_kind,
            PRECOMPILED_GO_SYNTAX_TOKENIZER_ARTIFACT_KIND,
        )?;
        require_u32(
            "empty_prefix_generation.precompiled_artifact_version",
            self.precompiled_artifact_version,
            PRECOMPILED_ARTIFACT_VERSION,
        )?;
        require_string(
            "empty_prefix_generation.precompiled_artifact_grammar_id",
            &self.precompiled_artifact_grammar_id,
            candidate_grammar_id,
        )?;
        require_string(
            "empty_prefix_generation.precompiled_artifact_metadata_path",
            &self.precompiled_artifact_metadata_path,
            metadata_path,
        )?;
        require_string(
            "empty_prefix_generation.precompiled_artifact_metadata_sha256",
            &self.precompiled_artifact_metadata_sha256,
            metadata_sha256,
        )?;
        require_string(
            "empty_prefix_generation.precompiled_artifact_structural_model_id",
            &self.precompiled_artifact_structural_model_id,
            structural_model_id,
        )?;
        require_string(
            "empty_prefix_generation.raw_state_machine_core_boundary_id",
            &self.raw_state_machine_core_boundary_id,
            RAW_SYNTAX_STATE_MACHINE_CORE_BOUNDARY_ID,
        )?;
        require_string(
            "empty_prefix_generation.raw_state_machine_core_boundary",
            &self.raw_state_machine_core_boundary,
            RAW_SYNTAX_STATE_MACHINE_CORE_BOUNDARY,
        )?;
        require_string(
            "empty_prefix_generation.generator_core_api_id",
            &self.generator_core_api_id,
            RAW_SCRATCH_GENERATOR_CORE_API_ID,
        )?;
        require_string(
            "empty_prefix_generation.generator_core_api_boundary",
            &self.generator_core_api_boundary,
            RAW_SCRATCH_GENERATOR_CORE_API_BOUNDARY,
        )?;
        require_u32(
            "empty_prefix_generation.token_layout_vocab_size",
            self.token_layout_vocab_size,
            token_layout_vocab_size,
        )?;
        require_u32(
            "empty_prefix_generation.pad_token_id",
            self.pad_token_id,
            token_layout_pad_token,
        )?;
        require_string(
            "empty_prefix_generation.empty_prefix_root",
            &self.empty_prefix_root,
            EMPTY_PREFIX_ROOT,
        )?;
        require_string(
            "empty_prefix_generation.structural_root_rule",
            &self.structural_root_rule,
            "File",
        )?;
        require_string(
            "empty_prefix_generation.structural_root_node_kind",
            &self.structural_root_node_kind,
            "File",
        )?;
        require_string(
            "empty_prefix_generation.initial_state_observed_status",
            &self.initial_state_observed_status,
            "partial",
        )?;
        require_usize(
            "empty_prefix_generation.initial_state_consumed_raw_tokens",
            self.initial_state_consumed_raw_tokens,
            0,
        )?;
        require_usize(
            "empty_prefix_generation.initial_state_total_raw_tokens",
            self.initial_state_total_raw_tokens,
            0,
        )?;
        require_bool(
            "empty_prefix_generation.initial_state_eof_valid",
            self.initial_state_eof_valid,
            false,
        )?;
        require_usize(
            "empty_prefix_generation.initial_state_stack_depth",
            self.initial_state_stack_depth,
            0,
        )?;
        require_none(
            "empty_prefix_generation.initial_state_active_rule",
            self.initial_state_active_rule.as_deref(),
        )?;
        require_none(
            "empty_prefix_generation.initial_state_active_field",
            self.initial_state_active_field.as_deref(),
        )?;
        require_positive_usize(
            "empty_prefix_generation.finite_admissible_raw_token_ids",
            self.finite_admissible_raw_token_ids.len(),
        )?;
        require_usize(
            "empty_prefix_generation.finite_admissible_raw_token_count",
            self.finite_admissible_raw_token_count,
            self.finite_admissible_raw_token_ids.len(),
        )?;
        let finite_hash = json_sha256(&self.finite_admissible_raw_token_ids).map_err(|error| {
            SyntaxBackendBoundaryError::new(
                "empty_prefix_generation.finite_admissible_raw_token_ids_sha256",
                format!("failed to hash empty-prefix admissible raw-token IDs: {error}"),
            )
        })?;
        require_string(
            "empty_prefix_generation.finite_admissible_raw_token_ids_sha256",
            &self.finite_admissible_raw_token_ids_sha256,
            &finite_hash,
        )?;
        if self.selected_raw_token_id >= token_layout_vocab_size {
            return Err(SyntaxBackendBoundaryError::new(
                "empty_prefix_generation.selected_raw_token_id",
                "selected raw token must be below VOCAB_SIZE",
            ));
        }
        if !self
            .finite_admissible_raw_token_ids
            .contains(&self.selected_raw_token_id)
        {
            return Err(SyntaxBackendBoundaryError::new(
                "empty_prefix_generation.selected_raw_token_id",
                "selected raw token must be admitted by the empty-prefix finite mask",
            ));
        }
        require_bool(
            "empty_prefix_generation.selected_raw_token_admitted",
            self.selected_raw_token_admitted,
            true,
        )?;
        require_bool(
            "empty_prefix_generation.selected_transition_accepted",
            self.selected_transition_accepted,
            true,
        )?;
        require_string(
            "empty_prefix_generation.selected_transition_observed_status",
            &self.selected_transition_observed_status,
            "partial",
        )?;
        require_usize(
            "empty_prefix_generation.selected_transition_consumed_raw_tokens",
            self.selected_transition_consumed_raw_tokens,
            1,
        )?;
        require_bool(
            "empty_prefix_generation.selected_transition_eof_valid",
            self.selected_transition_eof_valid,
            false,
        )?;
        require_usize(
            "empty_prefix_generation.selected_transition_stack_depth",
            self.selected_transition_stack_depth,
            1,
        )?;
        require_string(
            "empty_prefix_generation.selected_transition_active_rule",
            self.selected_transition_active_rule
                .as_deref()
                .unwrap_or(""),
            "File",
        )?;
        require_none(
            "empty_prefix_generation.selected_transition_active_field",
            self.selected_transition_active_field.as_deref(),
        )?;
        require_bool(
            "empty_prefix_generation.pad_token_rejected",
            self.pad_token_rejected,
            true,
        )?;
        require_string(
            "empty_prefix_generation.pad_token_error_kind",
            self.pad_token_error_kind.as_deref().unwrap_or(""),
            "pad_token_in_raw_stream",
        )?;
        require_u32(
            "empty_prefix_generation.malformed_raw_token_id",
            self.malformed_raw_token_id,
            token_layout_vocab_size,
        )?;
        require_bool(
            "empty_prefix_generation.malformed_raw_token_rejected",
            self.malformed_raw_token_rejected,
            true,
        )?;
        require_string(
            "empty_prefix_generation.malformed_raw_token_error_kind",
            self.malformed_raw_token_error_kind.as_deref().unwrap_or(""),
            "token_id_out_of_vocab",
        )?;
        require_bool(
            "empty_prefix_generation.per_output_parser_metadata_required",
            self.per_output_parser_metadata_required,
            false,
        )?;
        require_bool(
            "empty_prefix_generation.source_instance_segment_sidecar_used",
            self.source_instance_segment_sidecar_used,
            false,
        )?;
        require_bool(
            "empty_prefix_generation.source_prefix_table_used",
            self.source_prefix_table_used,
            false,
        )?;
        require_bool(
            "empty_prefix_generation.arena_ir_used",
            self.arena_ir_used,
            false,
        )?;
        require_bool(
            "empty_prefix_generation.route_handoff_provenance_used",
            self.route_handoff_provenance_used,
            false,
        )?;
        require_bool(
            "empty_prefix_generation.bpe_validity_authority_used",
            self.bpe_validity_authority_used,
            false,
        )?;
        require_bool(
            "empty_prefix_generation.stable_backend_claimed",
            self.stable_backend_claimed,
            false,
        )?;
        require_string(
            "empty_prefix_generation.next_ready_task",
            &self.next_ready_task,
            NEXT_READY_TASK,
        )?;
        require_str_contains(
            "empty_prefix_generation.claim_boundary",
            &self.claim_boundary,
            "syntax-level",
        )?;
        require_str_contains(
            "empty_prefix_generation.claim_boundary",
            &self.claim_boundary,
            "not Arena IR",
        )?;
        require_str_contains(
            "empty_prefix_generation.claim_boundary",
            &self.claim_boundary,
            "not BPE",
        )?;
        require_str_contains(
            "empty_prefix_generation.claim_boundary",
            &self.claim_boundary,
            "not source-prefix table",
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoTokenizerGeneratorSelectedCompletionSmokeEvidence {
    pub status: String,
    pub scope: String,
    pub precompiled_artifact_kind: String,
    pub precompiled_artifact_version: u32,
    pub precompiled_artifact_grammar_id: String,
    pub precompiled_artifact_metadata_path: String,
    pub precompiled_artifact_metadata_sha256: String,
    pub precompiled_artifact_structural_model_id: String,
    pub raw_state_machine_core_boundary_id: String,
    pub raw_state_machine_core_boundary: String,
    pub generator_core_api_id: String,
    pub generator_core_api_boundary: String,
    pub selection_loop_source: String,
    pub token_layout_vocab_size: u32,
    pub pad_token_id: u32,
    pub empty_prefix_root: String,
    pub raw_token_count: usize,
    pub transition_count: usize,
    pub raw_token_ids_sha256: String,
    pub admissible_mask_query_count: usize,
    pub finite_admissible_raw_token_mask_available: bool,
    pub every_selected_raw_token_admitted: bool,
    pub selected_transition_accepted_count: usize,
    pub first_selected_raw_token_id: u32,
    pub final_selected_raw_token_id: u32,
    pub final_state_observed_status: String,
    pub final_state_consumed_raw_tokens: usize,
    pub final_state_eof_valid: bool,
    pub final_state_stack_depth: usize,
    pub decoded_tree_reusable: bool,
    pub decoded_tree: GoTokenizerGeneratorDecodedTreeSummary,
    pub canonical_render: SourceInstanceCanonicalRender,
    pub canonical_render_ready: bool,
    pub gofmt_validation: String,
    pub go_parser_reparse_validation: String,
    pub render_reparse_sidecar_validation: bool,
    pub step_trace_sample: Vec<GoTokenizerGeneratorSelectedCompletionStepSample>,
    pub per_output_parser_metadata_required: bool,
    pub source_instance_segment_sidecar_used: bool,
    pub source_prefix_table_used: bool,
    pub arena_ir_used: bool,
    pub route_handoff_provenance_used: bool,
    pub bpe_validity_authority_used: bool,
    pub stable_backend_claimed: bool,
    pub next_ready_task: String,
    pub claim_boundary: String,
}

impl GoTokenizerGeneratorSelectedCompletionSmokeEvidence {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn validate(
        &self,
        candidate_grammar_id: &str,
        metadata_path: &str,
        metadata_sha256: &str,
        structural_model_id: &str,
        token_layout_vocab_size: u32,
        token_layout_pad_token: u32,
        raw_token_count: usize,
        raw_token_ids_sha256: &str,
        source_sha256: &str,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        require_string(
            "selected_completion_smoke.status",
            &self.status,
            SELECTED_COMPLETION_SMOKE_STATUS,
        )?;
        require_string(
            "selected_completion_smoke.scope",
            &self.scope,
            SELECTED_COMPLETION_SMOKE_SCOPE,
        )?;
        require_string(
            "selected_completion_smoke.precompiled_artifact_kind",
            &self.precompiled_artifact_kind,
            PRECOMPILED_GO_SYNTAX_TOKENIZER_ARTIFACT_KIND,
        )?;
        require_u32(
            "selected_completion_smoke.precompiled_artifact_version",
            self.precompiled_artifact_version,
            PRECOMPILED_ARTIFACT_VERSION,
        )?;
        require_string(
            "selected_completion_smoke.precompiled_artifact_grammar_id",
            &self.precompiled_artifact_grammar_id,
            candidate_grammar_id,
        )?;
        require_string(
            "selected_completion_smoke.precompiled_artifact_metadata_path",
            &self.precompiled_artifact_metadata_path,
            metadata_path,
        )?;
        require_string(
            "selected_completion_smoke.precompiled_artifact_metadata_sha256",
            &self.precompiled_artifact_metadata_sha256,
            metadata_sha256,
        )?;
        require_string(
            "selected_completion_smoke.precompiled_artifact_structural_model_id",
            &self.precompiled_artifact_structural_model_id,
            structural_model_id,
        )?;
        require_string(
            "selected_completion_smoke.raw_state_machine_core_boundary_id",
            &self.raw_state_machine_core_boundary_id,
            RAW_SYNTAX_STATE_MACHINE_CORE_BOUNDARY_ID,
        )?;
        require_string(
            "selected_completion_smoke.raw_state_machine_core_boundary",
            &self.raw_state_machine_core_boundary,
            RAW_SYNTAX_STATE_MACHINE_CORE_BOUNDARY,
        )?;
        require_string(
            "selected_completion_smoke.generator_core_api_id",
            &self.generator_core_api_id,
            RAW_SCRATCH_GENERATOR_CORE_API_ID,
        )?;
        require_string(
            "selected_completion_smoke.generator_core_api_boundary",
            &self.generator_core_api_boundary,
            RAW_SCRATCH_GENERATOR_CORE_API_BOUNDARY,
        )?;
        require_string(
            "selected_completion_smoke.selection_loop_source",
            &self.selection_loop_source,
            SELECTED_COMPLETION_STREAM_SOURCE,
        )?;
        require_u32(
            "selected_completion_smoke.token_layout_vocab_size",
            self.token_layout_vocab_size,
            token_layout_vocab_size,
        )?;
        require_u32(
            "selected_completion_smoke.pad_token_id",
            self.pad_token_id,
            token_layout_pad_token,
        )?;
        require_string(
            "selected_completion_smoke.empty_prefix_root",
            &self.empty_prefix_root,
            EMPTY_PREFIX_ROOT,
        )?;
        require_usize(
            "selected_completion_smoke.raw_token_count",
            self.raw_token_count,
            raw_token_count,
        )?;
        require_usize(
            "selected_completion_smoke.transition_count",
            self.transition_count,
            raw_token_count,
        )?;
        require_string(
            "selected_completion_smoke.raw_token_ids_sha256",
            &self.raw_token_ids_sha256,
            raw_token_ids_sha256,
        )?;
        require_usize(
            "selected_completion_smoke.admissible_mask_query_count",
            self.admissible_mask_query_count,
            raw_token_count,
        )?;
        require_bool(
            "selected_completion_smoke.finite_admissible_raw_token_mask_available",
            self.finite_admissible_raw_token_mask_available,
            true,
        )?;
        require_bool(
            "selected_completion_smoke.every_selected_raw_token_admitted",
            self.every_selected_raw_token_admitted,
            true,
        )?;
        require_usize(
            "selected_completion_smoke.selected_transition_accepted_count",
            self.selected_transition_accepted_count,
            raw_token_count,
        )?;
        if self.first_selected_raw_token_id >= token_layout_vocab_size {
            return Err(SyntaxBackendBoundaryError::new(
                "selected_completion_smoke.first_selected_raw_token_id",
                "first selected raw token must be below VOCAB_SIZE",
            ));
        }
        if self.final_selected_raw_token_id >= token_layout_vocab_size {
            return Err(SyntaxBackendBoundaryError::new(
                "selected_completion_smoke.final_selected_raw_token_id",
                "final selected raw token must be below VOCAB_SIZE",
            ));
        }
        require_string(
            "selected_completion_smoke.final_state_observed_status",
            &self.final_state_observed_status,
            "complete",
        )?;
        require_usize(
            "selected_completion_smoke.final_state_consumed_raw_tokens",
            self.final_state_consumed_raw_tokens,
            raw_token_count,
        )?;
        require_bool(
            "selected_completion_smoke.final_state_eof_valid",
            self.final_state_eof_valid,
            true,
        )?;
        require_usize(
            "selected_completion_smoke.final_state_stack_depth",
            self.final_state_stack_depth,
            0,
        )?;
        require_bool(
            "selected_completion_smoke.decoded_tree_reusable",
            self.decoded_tree_reusable,
            true,
        )?;
        self.decoded_tree.validate(raw_token_count)?;
        validate_canonical_render(&self.canonical_render, source_sha256)?;
        require_bool(
            "selected_completion_smoke.canonical_render_ready",
            self.canonical_render_ready,
            true,
        )?;
        require_string(
            "selected_completion_smoke.gofmt_validation",
            &self.gofmt_validation,
            "passed",
        )?;
        require_string(
            "selected_completion_smoke.go_parser_reparse_validation",
            &self.go_parser_reparse_validation,
            "passed",
        )?;
        require_bool(
            "selected_completion_smoke.render_reparse_sidecar_validation",
            self.render_reparse_sidecar_validation,
            true,
        )?;
        require_positive_usize(
            "selected_completion_smoke.step_trace_sample",
            self.step_trace_sample.len(),
        )?;
        if self.step_trace_sample.len() > 6 {
            return Err(SyntaxBackendBoundaryError::new(
                "selected_completion_smoke.step_trace_sample",
                "selected completion smoke keeps at most six diagnostic samples",
            ));
        }
        let mut seen_steps = BTreeSet::new();
        for sample in &self.step_trace_sample {
            sample.validate(raw_token_count, token_layout_vocab_size)?;
            if !seen_steps.insert(sample.step_index) {
                return Err(SyntaxBackendBoundaryError::new(
                    "selected_completion_smoke.step_trace_sample",
                    format!(
                        "duplicate selected completion sample step {}",
                        sample.step_index
                    ),
                ));
            }
        }
        require_bool(
            "selected_completion_smoke.per_output_parser_metadata_required",
            self.per_output_parser_metadata_required,
            false,
        )?;
        require_bool(
            "selected_completion_smoke.source_instance_segment_sidecar_used",
            self.source_instance_segment_sidecar_used,
            false,
        )?;
        require_bool(
            "selected_completion_smoke.source_prefix_table_used",
            self.source_prefix_table_used,
            false,
        )?;
        require_bool(
            "selected_completion_smoke.arena_ir_used",
            self.arena_ir_used,
            false,
        )?;
        require_bool(
            "selected_completion_smoke.route_handoff_provenance_used",
            self.route_handoff_provenance_used,
            false,
        )?;
        require_bool(
            "selected_completion_smoke.bpe_validity_authority_used",
            self.bpe_validity_authority_used,
            false,
        )?;
        require_bool(
            "selected_completion_smoke.stable_backend_claimed",
            self.stable_backend_claimed,
            false,
        )?;
        require_string(
            "selected_completion_smoke.next_ready_task",
            &self.next_ready_task,
            NEXT_READY_TASK,
        )?;
        require_str_contains(
            "selected_completion_smoke.claim_boundary",
            &self.claim_boundary,
            "syntax-level",
        )?;
        require_str_contains(
            "selected_completion_smoke.claim_boundary",
            &self.claim_boundary,
            "empty GoSourceFile prefix",
        )?;
        require_str_contains(
            "selected_completion_smoke.claim_boundary",
            &self.claim_boundary,
            "not per-output parser metadata",
        )?;
        require_str_contains(
            "selected_completion_smoke.claim_boundary",
            &self.claim_boundary,
            "not source-prefix table",
        )?;
        require_str_contains(
            "selected_completion_smoke.claim_boundary",
            &self.claim_boundary,
            "not Arena IR",
        )?;
        require_str_contains(
            "selected_completion_smoke.claim_boundary",
            &self.claim_boundary,
            "not BPE",
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoTokenizerGeneratorSelectedCompletionStepSample {
    pub step_index: usize,
    pub prefix_raw_token_count: usize,
    pub selected_raw_token_id: u32,
    pub finite_admissible_raw_token_count: usize,
    pub selected_raw_token_admitted: bool,
    pub selected_transition_accepted: bool,
    pub transition_observed_status: String,
    pub transition_consumed_raw_tokens: usize,
    pub transition_eof_valid: bool,
    pub transition_stack_depth: usize,
    pub transition_active_rule: Option<String>,
    pub transition_active_field: Option<String>,
}

impl GoTokenizerGeneratorSelectedCompletionStepSample {
    fn validate(
        &self,
        raw_token_count: usize,
        token_layout_vocab_size: u32,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        if self.step_index >= raw_token_count {
            return Err(SyntaxBackendBoundaryError::new(
                "selected_completion_smoke.step_trace_sample.step_index",
                "selected completion sample step must be inside the raw token stream",
            ));
        }
        require_usize(
            "selected_completion_smoke.step_trace_sample.prefix_raw_token_count",
            self.prefix_raw_token_count,
            self.step_index,
        )?;
        if self.selected_raw_token_id >= token_layout_vocab_size {
            return Err(SyntaxBackendBoundaryError::new(
                "selected_completion_smoke.step_trace_sample.selected_raw_token_id",
                "selected sample raw token must be below VOCAB_SIZE",
            ));
        }
        require_positive_usize(
            "selected_completion_smoke.step_trace_sample.finite_admissible_raw_token_count",
            self.finite_admissible_raw_token_count,
        )?;
        require_bool(
            "selected_completion_smoke.step_trace_sample.selected_raw_token_admitted",
            self.selected_raw_token_admitted,
            true,
        )?;
        require_bool(
            "selected_completion_smoke.step_trace_sample.selected_transition_accepted",
            self.selected_transition_accepted,
            true,
        )?;
        if self.transition_observed_status != "partial"
            && self.transition_observed_status != "complete"
        {
            return Err(SyntaxBackendBoundaryError::new(
                "selected_completion_smoke.step_trace_sample.transition_observed_status",
                "selected sample transition status must be partial or complete",
            ));
        }
        require_usize(
            "selected_completion_smoke.step_trace_sample.transition_consumed_raw_tokens",
            self.transition_consumed_raw_tokens,
            self.step_index + 1,
        )?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoSourceRecordElidedSelectedCompletionEvidence {
    pub status: String,
    pub scope: String,
    pub stream_contract: String,
    pub canonical_token_count: usize,
    pub canonical_tokens_sha256: String,
    pub canonical_max_token_id: Option<u32>,
    pub expanded_token_count: usize,
    pub forced_token_count: usize,
    pub expansion_matches_expanded_stream: bool,
    pub every_selected_elided_token_admitted: bool,
    pub every_elided_position_non_forced: bool,
    pub transition_count: usize,
    pub admissible_mask_query_count: usize,
    pub first_selected_elided_raw_token_id: Option<u32>,
    pub final_selected_elided_raw_token_id: Option<u32>,
    pub final_state_eof_valid: bool,
    pub final_state_consumed_raw_tokens: usize,
    pub decoded_tree_reusable: bool,
    pub decoded_tree_consumed_raw_tokens: usize,
    pub claim_boundary: String,
}

impl GoSourceRecordElidedSelectedCompletionEvidence {
    /// Validate stamped elided selected-completion evidence against stream
    /// identity available on the tokenizer-generator report / manifest-depth
    /// record (without requiring the full emission object).
    pub fn validate_against_stream_identity(
        &self,
        stream_contract: &str,
        canonical_token_count: usize,
        canonical_tokens_sha256: &str,
        canonical_max_token_id: Option<u32>,
        expanded_token_count: usize,
        forced_token_count: usize,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        require_string(
            "elided_selected_completion.status",
            &self.status,
            GO_SOURCE_RECORD_ELIDED_SELECTED_COMPLETION_STATUS,
        )?;
        require_string(
            "elided_selected_completion.scope",
            &self.scope,
            GO_SOURCE_RECORD_ELIDED_SELECTED_COMPLETION_SCOPE,
        )?;
        require_string(
            "elided_selected_completion.stream_contract",
            &self.stream_contract,
            LIVE_RAW_SYNTAX_STREAM_CONTRACT,
        )?;
        require_string(
            "elided_selected_completion.stream_contract",
            &self.stream_contract,
            stream_contract,
        )?;
        require_usize(
            "elided_selected_completion.canonical_token_count",
            self.canonical_token_count,
            canonical_token_count,
        )?;
        require_string(
            "elided_selected_completion.canonical_tokens_sha256",
            &self.canonical_tokens_sha256,
            canonical_tokens_sha256,
        )?;
        if self.canonical_max_token_id != canonical_max_token_id {
            return Err(SyntaxBackendBoundaryError::new(
                "elided_selected_completion.canonical_max_token_id",
                "elided selected-completion max token id must match stream identity",
            ));
        }
        require_usize(
            "elided_selected_completion.expanded_token_count",
            self.expanded_token_count,
            expanded_token_count,
        )?;
        require_usize(
            "elided_selected_completion.forced_token_count",
            self.forced_token_count,
            forced_token_count,
        )?;
        require_bool(
            "elided_selected_completion.expansion_matches_expanded_stream",
            self.expansion_matches_expanded_stream,
            true,
        )?;
        require_bool(
            "elided_selected_completion.every_selected_elided_token_admitted",
            self.every_selected_elided_token_admitted,
            true,
        )?;
        require_bool(
            "elided_selected_completion.every_elided_position_non_forced",
            self.every_elided_position_non_forced,
            true,
        )?;
        require_usize(
            "elided_selected_completion.transition_count",
            self.transition_count,
            canonical_token_count,
        )?;
        require_usize(
            "elided_selected_completion.admissible_mask_query_count",
            self.admissible_mask_query_count,
            canonical_token_count,
        )?;
        require_bool(
            "elided_selected_completion.final_state_eof_valid",
            self.final_state_eof_valid,
            true,
        )?;
        require_usize(
            "elided_selected_completion.final_state_consumed_raw_tokens",
            self.final_state_consumed_raw_tokens,
            expanded_token_count,
        )?;
        require_bool(
            "elided_selected_completion.decoded_tree_reusable",
            self.decoded_tree_reusable,
            true,
        )?;
        require_usize(
            "elided_selected_completion.decoded_tree_consumed_raw_tokens",
            self.decoded_tree_consumed_raw_tokens,
            expanded_token_count,
        )?;
        require_string(
            "elided_selected_completion.claim_boundary",
            &self.claim_boundary,
            GO_SOURCE_RECORD_ELIDED_SELECTED_COMPLETION_CLAIM_BOUNDARY,
        )?;
        Ok(())
    }

    fn validate(
        &self,
        raw_token_emission: &GoSourceRecordRawTokenEmission,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        self.validate_against_stream_identity(
            &raw_token_emission.stream_contract,
            raw_token_emission.canonical_token_count,
            &raw_token_emission.canonical_tokens_sha256,
            raw_token_emission.canonical_max_token_id,
            raw_token_emission.expanded_token_count,
            raw_token_emission.forced_token_count,
        )?;
        if self.first_selected_elided_raw_token_id
            != raw_token_emission.canonical_token_ids.first().copied()
        {
            return Err(SyntaxBackendBoundaryError::new(
                "elided_selected_completion.first_selected_elided_raw_token_id",
                "first selected elided token must match the emission canonical stream",
            ));
        }
        if self.final_selected_elided_raw_token_id
            != raw_token_emission.canonical_token_ids.last().copied()
        {
            return Err(SyntaxBackendBoundaryError::new(
                "elided_selected_completion.final_selected_elided_raw_token_id",
                "final selected elided token must match the emission canonical stream",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoSourceRecordGeneratorEvidence {
    pub source_record_generator_core_api_id: String,
    pub source_record_generator_core_api_boundary: String,
    pub source_record_raw_token_emission_api_id: String,
    pub source_record_raw_token_emission_api_boundary: String,
    pub delegated_raw_scratch_generator_core_api_id: String,
    pub delegated_raw_scratch_generator_core_api_boundary: String,
    /// Elided-stream identity owned by the source-record emission path; the
    /// tokenizer-generator report sources these fields from generator-core
    /// evidence rather than re-deriving them only from the generated-codec
    /// report.
    pub stream_contract: String,
    pub canonical_token_count: usize,
    pub canonical_tokens_sha256: String,
    pub canonical_max_token_id: Option<u32>,
    pub expanded_token_count: usize,
    pub forced_token_count: usize,
    pub empty_prefix_generation: GoTokenizerGeneratorEmptyPrefixGenerationEvidence,
    pub selected_completion_smoke: GoTokenizerGeneratorSelectedCompletionSmokeEvidence,
    pub elided_selected_completion: GoSourceRecordElidedSelectedCompletionEvidence,
    pub incomplete_prefix_rejection: GoTokenizerGeneratorIncompletePrefixRejectionEvidence,
    pub token_only_decode_render: GoTokenizerGeneratorTokenOnlyDecodeRenderEvidence,
}

impl GoSourceRecordGeneratorEvidence {
    pub fn validate(
        &self,
        checked_report: &SyntaxGoSourceInstanceProjectionReport,
        raw_token_emission: &GoSourceRecordRawTokenEmission,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        raw_token_emission.validate(checked_report)?;
        let raw_tokens = &raw_token_emission.raw_tokens;
        require_string(
            "source_record_generator_core.api_id",
            &self.source_record_generator_core_api_id,
            GO_SOURCE_RECORD_GENERATOR_CORE_API_ID,
        )?;
        require_string(
            "source_record_generator_core.api_boundary",
            &self.source_record_generator_core_api_boundary,
            GO_SOURCE_RECORD_GENERATOR_CORE_API_BOUNDARY,
        )?;
        require_string(
            "source_record_generator_core.raw_token_emission_api_id",
            &self.source_record_raw_token_emission_api_id,
            &raw_token_emission.api_id,
        )?;
        require_string(
            "source_record_generator_core.raw_token_emission_api_boundary",
            &self.source_record_raw_token_emission_api_boundary,
            &raw_token_emission.api_boundary,
        )?;
        require_string(
            "source_record_generator_core.delegated_raw_scratch_generator_core_api_id",
            &self.delegated_raw_scratch_generator_core_api_id,
            RAW_SCRATCH_GENERATOR_CORE_API_ID,
        )?;
        require_string(
            "source_record_generator_core.delegated_raw_scratch_generator_core_api_boundary",
            &self.delegated_raw_scratch_generator_core_api_boundary,
            RAW_SCRATCH_GENERATOR_CORE_API_BOUNDARY,
        )?;
        require_string(
            "source_record_generator_core.stream_contract",
            &self.stream_contract,
            &raw_token_emission.stream_contract,
        )?;
        require_usize(
            "source_record_generator_core.canonical_token_count",
            self.canonical_token_count,
            raw_token_emission.canonical_token_count,
        )?;
        require_string(
            "source_record_generator_core.canonical_tokens_sha256",
            &self.canonical_tokens_sha256,
            &raw_token_emission.canonical_tokens_sha256,
        )?;
        if self.canonical_max_token_id != raw_token_emission.canonical_max_token_id {
            return Err(SyntaxBackendBoundaryError::new(
                "source_record_generator_core.canonical_max_token_id",
                "generator-core canonical max token id must match the emission",
            ));
        }
        require_usize(
            "source_record_generator_core.expanded_token_count",
            self.expanded_token_count,
            raw_token_emission.expanded_token_count,
        )?;
        require_usize(
            "source_record_generator_core.forced_token_count",
            self.forced_token_count,
            raw_token_emission.forced_token_count,
        )?;
        self.empty_prefix_generation.validate(
            &checked_report.candidate_grammar_id,
            &checked_report.metadata.path,
            &checked_report.metadata.sha256,
            &raw_tokens.structural_model_id,
            raw_tokens.token_layout.vocab_size,
            raw_tokens.token_layout.pad_token,
        )?;
        self.selected_completion_smoke.validate(
            &checked_report.candidate_grammar_id,
            &checked_report.metadata.path,
            &checked_report.metadata.sha256,
            &raw_tokens.structural_model_id,
            raw_tokens.token_layout.vocab_size,
            raw_tokens.token_layout.pad_token,
            raw_tokens.raw_token_count,
            &raw_tokens.raw_token_ids_sha256,
            &checked_report.source.sha256,
        )?;
        self.elided_selected_completion
            .validate(raw_token_emission)?;
        self.incomplete_prefix_rejection.validate(
            &checked_report.candidate_grammar_id,
            &checked_report.metadata.path,
            &checked_report.metadata.sha256,
            &raw_tokens.structural_model_id,
            raw_tokens.token_layout.vocab_size,
            raw_tokens.token_layout.pad_token,
            raw_tokens.raw_token_count,
            &raw_tokens.raw_token_ids_sha256,
        )?;
        self.token_only_decode_render.validate(
            &checked_report.candidate_grammar_id,
            &checked_report.metadata.path,
            &checked_report.metadata.sha256,
            &raw_tokens.structural_model_id,
            raw_tokens.token_layout.vocab_size,
            raw_tokens.token_layout.pad_token,
            raw_tokens.raw_token_count,
            &raw_tokens.raw_token_ids_sha256,
            &checked_report.source.sha256,
        )
    }
}

pub struct GoSourceRecordGeneratorCore<'a> {
    raw_scratch_core: RawScratchGeneratorCore<'a>,
    checked_report: &'a SyntaxGoSourceInstanceProjectionReport,
    raw_token_emission: &'a GoSourceRecordRawTokenEmission,
}

impl<'a> GoSourceRecordGeneratorCore<'a> {
    pub fn new(
        checked_report: &'a SyntaxGoSourceInstanceProjectionReport,
        raw_token_emission: &'a GoSourceRecordRawTokenEmission,
    ) -> Self {
        Self {
            raw_scratch_core: RawScratchGeneratorCore::new(&raw_token_emission.compiled_model),
            checked_report,
            raw_token_emission,
        }
    }

    pub fn api_id(&self) -> &'static str {
        GO_SOURCE_RECORD_GENERATOR_CORE_API_ID
    }

    pub fn api_boundary(&self) -> &'static str {
        GO_SOURCE_RECORD_GENERATOR_CORE_API_BOUNDARY
    }

    pub fn build_evidence(
        &self,
    ) -> Result<GoSourceRecordGeneratorEvidence, SyntaxBackendBoundaryError> {
        let raw_tokens = &self.raw_token_emission.raw_tokens;
        let evidence = GoSourceRecordGeneratorEvidence {
            source_record_generator_core_api_id: self.api_id().to_string(),
            source_record_generator_core_api_boundary: self.api_boundary().to_string(),
            source_record_raw_token_emission_api_id: self.raw_token_emission.api_id.clone(),
            source_record_raw_token_emission_api_boundary: self
                .raw_token_emission
                .api_boundary
                .clone(),
            delegated_raw_scratch_generator_core_api_id: self.raw_scratch_core.api_id().to_string(),
            delegated_raw_scratch_generator_core_api_boundary: self
                .raw_scratch_core
                .api_boundary()
                .to_string(),
            stream_contract: self.raw_token_emission.stream_contract.clone(),
            canonical_token_count: self.raw_token_emission.canonical_token_count,
            canonical_tokens_sha256: self.raw_token_emission.canonical_tokens_sha256.clone(),
            canonical_max_token_id: self.raw_token_emission.canonical_max_token_id,
            expanded_token_count: self.raw_token_emission.expanded_token_count,
            forced_token_count: self.raw_token_emission.forced_token_count,
            empty_prefix_generation: go_tokenizer_generator_empty_prefix_generation_evidence(
                &self.raw_scratch_core,
                self.checked_report,
                raw_tokens,
                &self.raw_token_emission.decoded_tree,
            )?,
            selected_completion_smoke: go_tokenizer_generator_selected_completion_smoke_evidence(
                &self.raw_scratch_core,
                self.checked_report,
                raw_tokens,
            )?,
            elided_selected_completion: go_source_record_elided_selected_completion_evidence(
                self.raw_token_emission,
            )?,
            incomplete_prefix_rejection:
                go_tokenizer_generator_incomplete_prefix_rejection_evidence(
                    &self.raw_scratch_core,
                    self.checked_report,
                    raw_tokens,
                )?,
            token_only_decode_render: go_tokenizer_generator_token_only_decode_render_evidence(
                &self.raw_scratch_core,
                self.checked_report,
                raw_tokens,
            )?,
        };
        evidence.validate(self.checked_report, self.raw_token_emission)?;
        Ok(evidence)
    }
}

/// Inventory scratch-seed elided selected-completion evidence (generator-core).
///
/// Canonical elided walk for one inventory seed's expanded SM-native stream via
/// [`RawScratchGeneratorCore::elided_selected_completion`]. Expanded seed
/// admission remains on the parent seed summary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoInventoryScratchSeedElidedSelectedCompletionEvidence {
    pub status: String,
    pub scope: String,
    pub stream_contract: String,
    pub canonical_token_count: usize,
    pub canonical_tokens_sha256: String,
    pub canonical_max_token_id: Option<u32>,
    pub expanded_token_count: usize,
    pub forced_token_count: usize,
    pub expansion_matches_expanded_stream: bool,
    pub every_selected_elided_token_admitted: bool,
    pub every_elided_position_non_forced: bool,
    pub transition_count: usize,
    pub admissible_mask_query_count: usize,
    pub first_selected_elided_raw_token_id: Option<u32>,
    pub final_selected_elided_raw_token_id: Option<u32>,
    pub final_state_eof_valid: bool,
    pub final_state_consumed_raw_tokens: usize,
    pub decoded_tree_reusable: bool,
    pub decoded_tree_consumed_raw_tokens: usize,
    pub claim_boundary: String,
}

impl GoInventoryScratchSeedElidedSelectedCompletionEvidence {
    fn validate(&self, expanded_token_count: usize) -> Result<(), SyntaxBackendBoundaryError> {
        let field = "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.elided_selected_completion";
        require_string(
            "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.elided_selected_completion.status",
            &self.status,
            GO_AST_INVENTORY_SCRATCH_SEED_ELIDED_SELECTED_COMPLETION_STATUS,
        )?;
        require_string(
            "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.elided_selected_completion.scope",
            &self.scope,
            GO_AST_INVENTORY_SCRATCH_SEED_ELIDED_SELECTED_COMPLETION_SCOPE,
        )?;
        require_string(
            "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.elided_selected_completion.stream_contract",
            &self.stream_contract,
            LIVE_RAW_SYNTAX_STREAM_CONTRACT,
        )?;
        require_positive_usize(
            "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.elided_selected_completion.canonical_token_count",
            self.canonical_token_count,
        )?;
        require_sha256(
            "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.elided_selected_completion.canonical_tokens_sha256",
            &self.canonical_tokens_sha256,
        )?;
        require_usize(
            "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.elided_selected_completion.expanded_token_count",
            self.expanded_token_count,
            expanded_token_count,
        )?;
        require_usize(
            "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.elided_selected_completion.forced_token_count",
            self.forced_token_count,
            self.expanded_token_count
                .checked_sub(self.canonical_token_count)
                .ok_or_else(|| {
                    SyntaxBackendBoundaryError::new(
                        field,
                        "canonical elided token count must not exceed expanded seed stream length",
                    )
                })?,
        )?;
        require_bool(
            "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.elided_selected_completion.expansion_matches_expanded_stream",
            self.expansion_matches_expanded_stream,
            true,
        )?;
        require_bool(
            "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.elided_selected_completion.every_selected_elided_token_admitted",
            self.every_selected_elided_token_admitted,
            true,
        )?;
        require_bool(
            "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.elided_selected_completion.every_elided_position_non_forced",
            self.every_elided_position_non_forced,
            true,
        )?;
        require_usize(
            "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.elided_selected_completion.transition_count",
            self.transition_count,
            self.canonical_token_count,
        )?;
        require_usize(
            "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.elided_selected_completion.admissible_mask_query_count",
            self.admissible_mask_query_count,
            self.canonical_token_count,
        )?;
        if self.canonical_token_count > 0 {
            if self.first_selected_elided_raw_token_id.is_none() {
                return Err(SyntaxBackendBoundaryError::new(
                    "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.elided_selected_completion.first_selected_elided_raw_token_id",
                    "non-empty elided seed stream must expose a first selected token",
                ));
            }
            if self.final_selected_elided_raw_token_id.is_none() {
                return Err(SyntaxBackendBoundaryError::new(
                    "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.elided_selected_completion.final_selected_elided_raw_token_id",
                    "non-empty elided seed stream must expose a final selected token",
                ));
            }
        }
        require_bool(
            "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.elided_selected_completion.final_state_eof_valid",
            self.final_state_eof_valid,
            true,
        )?;
        require_usize(
            "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.elided_selected_completion.final_state_consumed_raw_tokens",
            self.final_state_consumed_raw_tokens,
            expanded_token_count,
        )?;
        require_bool(
            "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.elided_selected_completion.decoded_tree_reusable",
            self.decoded_tree_reusable,
            true,
        )?;
        require_usize(
            "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.elided_selected_completion.decoded_tree_consumed_raw_tokens",
            self.decoded_tree_consumed_raw_tokens,
            expanded_token_count,
        )?;
        require_string(
            "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.elided_selected_completion.claim_boundary",
            &self.claim_boundary,
            GO_AST_INVENTORY_SCRATCH_SEED_ELIDED_SELECTED_COMPLETION_CLAIM_BOUNDARY,
        )?;
        require_str_contains(
            "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.elided_selected_completion.claim_boundary",
            &self.claim_boundary,
            "RawScratchGeneratorCore::elided_selected_completion",
        )?;
        Ok(())
    }
}

/// Inventory scratch-seed incomplete-prefix rejection evidence (generator-core).
///
/// Multi-token inventory seeds only: partial prefix accepted, strict complete
/// rejected, selected continuation admitted via
/// [`RawScratchGeneratorCore::incomplete_prefix`]. Expanded seed admission
/// remains on the parent seed summary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoInventoryScratchSeedIncompletePrefixRejectionEvidence {
    pub status: String,
    pub scope: String,
    pub selection_loop_source: String,
    pub complete_raw_token_count: usize,
    pub prefix_raw_token_count: usize,
    pub prefix_raw_token_ids_sha256: String,
    pub prefix_observed_status: String,
    pub prefix_consumed_raw_tokens: usize,
    pub prefix_eof_valid: bool,
    pub prefix_stack_depth: usize,
    pub prefix_finite_admissible_raw_token_count: usize,
    pub prefix_finite_admissible_raw_token_ids: Vec<u32>,
    pub prefix_admissible_raw_token_range_count: usize,
    pub prefix_payload_end_admissible: bool,
    pub continuation_mask_available: bool,
    pub strict_completion_rejected: bool,
    pub strict_completion_error_kind: String,
    pub strict_completion_consumed_raw_tokens: usize,
    pub strict_completion_failing_token: Option<u32>,
    pub selected_next_raw_token_id: u32,
    pub selected_next_raw_token_admitted: bool,
    pub selected_next_transition_accepted: bool,
    pub selected_next_transition_observed_status: String,
    pub selected_next_transition_consumed_raw_tokens: usize,
    pub selected_next_transition_eof_valid: bool,
    pub final_completion_observed_status: String,
    pub final_completion_eof_valid: bool,
    pub claim_boundary: String,
}

impl GoInventoryScratchSeedIncompletePrefixRejectionEvidence {
    fn validate(
        &self,
        complete_raw_token_count: usize,
        token_layout_vocab_size: u32,
        token_layout_pad_token: u32,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        let field = "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.incomplete_prefix_rejection";
        require_string(
            "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.incomplete_prefix_rejection.status",
            &self.status,
            GO_AST_INVENTORY_SCRATCH_SEED_INCOMPLETE_PREFIX_REJECTION_STATUS,
        )?;
        require_string(
            "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.incomplete_prefix_rejection.scope",
            &self.scope,
            GO_AST_INVENTORY_SCRATCH_SEED_INCOMPLETE_PREFIX_REJECTION_SCOPE,
        )?;
        require_string(
            "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.incomplete_prefix_rejection.selection_loop_source",
            &self.selection_loop_source,
            SELECTED_COMPLETION_STREAM_SOURCE,
        )?;
        require_usize(
            "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.incomplete_prefix_rejection.complete_raw_token_count",
            self.complete_raw_token_count,
            complete_raw_token_count,
        )?;
        require_positive_usize(
            "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.incomplete_prefix_rejection.prefix_raw_token_count",
            self.prefix_raw_token_count,
        )?;
        if self.prefix_raw_token_count >= complete_raw_token_count {
            return Err(SyntaxBackendBoundaryError::new(
                "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.incomplete_prefix_rejection.prefix_raw_token_count",
                "inventory seed incomplete prefix must be shorter than the complete seed stream",
            ));
        }
        require_sha256(
            "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.incomplete_prefix_rejection.prefix_raw_token_ids_sha256",
            &self.prefix_raw_token_ids_sha256,
        )?;
        require_string(
            "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.incomplete_prefix_rejection.prefix_observed_status",
            &self.prefix_observed_status,
            "partial",
        )?;
        require_usize(
            "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.incomplete_prefix_rejection.prefix_consumed_raw_tokens",
            self.prefix_consumed_raw_tokens,
            self.prefix_raw_token_count,
        )?;
        require_bool(
            "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.incomplete_prefix_rejection.prefix_eof_valid",
            self.prefix_eof_valid,
            false,
        )?;
        require_positive_usize(
            "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.incomplete_prefix_rejection.prefix_stack_depth",
            self.prefix_stack_depth,
        )?;
        require_positive_usize(
            "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.incomplete_prefix_rejection.prefix_finite_admissible_raw_token_count",
            self.prefix_finite_admissible_raw_token_count,
        )?;
        require_usize(
            "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.incomplete_prefix_rejection.prefix_finite_admissible_raw_token_count",
            self.prefix_finite_admissible_raw_token_count,
            self.prefix_finite_admissible_raw_token_ids.len(),
        )?;
        for token_id in &self.prefix_finite_admissible_raw_token_ids {
            if *token_id == token_layout_pad_token {
                return Err(SyntaxBackendBoundaryError::new(
                    "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.incomplete_prefix_rejection.prefix_finite_admissible_raw_token_ids",
                    "seed continuation mask must not admit PAD_TOKEN",
                ));
            }
            if *token_id >= token_layout_vocab_size {
                return Err(SyntaxBackendBoundaryError::new(
                    "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.incomplete_prefix_rejection.prefix_finite_admissible_raw_token_ids",
                    "seed continuation mask token must remain below VOCAB_SIZE",
                ));
            }
        }
        require_usize(
            "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.incomplete_prefix_rejection.prefix_admissible_raw_token_range_count",
            self.prefix_admissible_raw_token_range_count,
            0,
        )?;
        require_bool(
            "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.incomplete_prefix_rejection.prefix_payload_end_admissible",
            self.prefix_payload_end_admissible,
            false,
        )?;
        require_bool(
            "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.incomplete_prefix_rejection.continuation_mask_available",
            self.continuation_mask_available,
            true,
        )?;
        require_bool(
            "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.incomplete_prefix_rejection.strict_completion_rejected",
            self.strict_completion_rejected,
            true,
        )?;
        require_string(
            "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.incomplete_prefix_rejection.strict_completion_error_kind",
            &self.strict_completion_error_kind,
            "premature_eof",
        )?;
        require_usize(
            "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.incomplete_prefix_rejection.strict_completion_consumed_raw_tokens",
            self.strict_completion_consumed_raw_tokens,
            self.prefix_raw_token_count,
        )?;
        if self.strict_completion_failing_token.is_some() {
            return Err(SyntaxBackendBoundaryError::new(
                "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.incomplete_prefix_rejection.strict_completion_failing_token",
                "premature EOF rejection for a seed token-only prefix must not report a failing token",
            ));
        }
        if self.selected_next_raw_token_id >= token_layout_vocab_size {
            return Err(SyntaxBackendBoundaryError::new(
                "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.incomplete_prefix_rejection.selected_next_raw_token_id",
                "selected seed continuation raw token must be below VOCAB_SIZE",
            ));
        }
        if !self
            .prefix_finite_admissible_raw_token_ids
            .contains(&self.selected_next_raw_token_id)
        {
            return Err(SyntaxBackendBoundaryError::new(
                "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.incomplete_prefix_rejection.selected_next_raw_token_id",
                "selected seed continuation raw token must be admitted by the prefix finite mask",
            ));
        }
        require_bool(
            "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.incomplete_prefix_rejection.selected_next_raw_token_admitted",
            self.selected_next_raw_token_admitted,
            true,
        )?;
        require_bool(
            "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.incomplete_prefix_rejection.selected_next_transition_accepted",
            self.selected_next_transition_accepted,
            true,
        )?;
        if self.selected_next_transition_observed_status != "partial"
            && self.selected_next_transition_observed_status != "complete"
        {
            return Err(SyntaxBackendBoundaryError::new(
                "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.incomplete_prefix_rejection.selected_next_transition_observed_status",
                "selected seed continuation transition status must be partial or complete",
            ));
        }
        require_usize(
            "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.incomplete_prefix_rejection.selected_next_transition_consumed_raw_tokens",
            self.selected_next_transition_consumed_raw_tokens,
            self.prefix_raw_token_count + 1,
        )?;
        require_string(
            "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.incomplete_prefix_rejection.final_completion_observed_status",
            &self.final_completion_observed_status,
            "complete",
        )?;
        require_bool(
            "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.incomplete_prefix_rejection.final_completion_eof_valid",
            self.final_completion_eof_valid,
            true,
        )?;
        require_string(
            "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.incomplete_prefix_rejection.claim_boundary",
            &self.claim_boundary,
            GO_AST_INVENTORY_SCRATCH_SEED_INCOMPLETE_PREFIX_REJECTION_CLAIM_BOUNDARY,
        )?;
        require_str_contains(
            field,
            &self.claim_boundary,
            "RawScratchGeneratorCore::incomplete_prefix",
        )?;
        Ok(())
    }
}

/// Inventory-model empty-prefix evidence (generator-core).
///
/// Nested on the inventory scratch-seed **batch** so inventory seeds and the
/// empty `GoSourceFile` start share one generator-core surface via
/// [`RawScratchGeneratorCore::empty_prefix`] against the inventory compiled
/// model (not only source-record precompiled-artifact empty-prefix).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoInventoryScratchSeedEmptyPrefixEvidence {
    pub status: String,
    pub scope: String,
    pub structural_model_id: String,
    pub raw_state_machine_core_boundary_id: String,
    pub raw_state_machine_core_boundary: String,
    pub generator_core_api_id: String,
    pub generator_core_api_boundary: String,
    pub token_layout_vocab_size: u32,
    pub pad_token_id: u32,
    pub empty_prefix_root: String,
    pub structural_root_rule: String,
    pub structural_root_node_kind: String,
    pub initial_state_observed_status: String,
    pub initial_state_consumed_raw_tokens: usize,
    pub initial_state_total_raw_tokens: usize,
    pub initial_state_eof_valid: bool,
    pub initial_state_stack_depth: usize,
    pub initial_state_active_rule: Option<String>,
    pub initial_state_active_field: Option<String>,
    pub finite_admissible_raw_token_count: usize,
    pub finite_admissible_raw_token_ids: Vec<u32>,
    pub finite_admissible_raw_token_ids_sha256: String,
    pub selected_raw_token_id: u32,
    pub selected_raw_token_admitted: bool,
    pub selected_transition_accepted: bool,
    pub selected_transition_observed_status: String,
    pub selected_transition_consumed_raw_tokens: usize,
    pub selected_transition_eof_valid: bool,
    pub selected_transition_stack_depth: usize,
    pub selected_transition_active_rule: Option<String>,
    pub selected_transition_active_field: Option<String>,
    pub pad_token_rejected: bool,
    pub pad_token_error_kind: String,
    pub malformed_raw_token_id: u32,
    pub malformed_raw_token_rejected: bool,
    pub malformed_raw_token_error_kind: String,
    pub claim_boundary: String,
}

impl GoInventoryScratchSeedEmptyPrefixEvidence {
    fn validate(
        &self,
        token_layout_vocab_size: u32,
        token_layout_pad_token: u32,
        structural_root_rule: &str,
        structural_root_node_kind: &str,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        let field = "manifest_depth.scratch_generation_gate.go_ast_inventory_empty_prefix";
        require_string(
            "manifest_depth.scratch_generation_gate.go_ast_inventory_empty_prefix.status",
            &self.status,
            GO_AST_INVENTORY_SCRATCH_SEED_EMPTY_PREFIX_STATUS,
        )?;
        require_string(
            "manifest_depth.scratch_generation_gate.go_ast_inventory_empty_prefix.scope",
            &self.scope,
            GO_AST_INVENTORY_SCRATCH_SEED_EMPTY_PREFIX_SCOPE,
        )?;
        require_non_empty(
            "manifest_depth.scratch_generation_gate.go_ast_inventory_empty_prefix.structural_model_id",
            &self.structural_model_id,
        )?;
        require_string(
            "manifest_depth.scratch_generation_gate.go_ast_inventory_empty_prefix.raw_state_machine_core_boundary_id",
            &self.raw_state_machine_core_boundary_id,
            RAW_SYNTAX_STATE_MACHINE_CORE_BOUNDARY_ID,
        )?;
        require_string(
            "manifest_depth.scratch_generation_gate.go_ast_inventory_empty_prefix.raw_state_machine_core_boundary",
            &self.raw_state_machine_core_boundary,
            RAW_SYNTAX_STATE_MACHINE_CORE_BOUNDARY,
        )?;
        require_string(
            "manifest_depth.scratch_generation_gate.go_ast_inventory_empty_prefix.generator_core_api_id",
            &self.generator_core_api_id,
            RAW_SCRATCH_GENERATOR_CORE_API_ID,
        )?;
        require_string(
            "manifest_depth.scratch_generation_gate.go_ast_inventory_empty_prefix.generator_core_api_boundary",
            &self.generator_core_api_boundary,
            RAW_SCRATCH_GENERATOR_CORE_API_BOUNDARY,
        )?;
        require_u32(
            "manifest_depth.scratch_generation_gate.go_ast_inventory_empty_prefix.token_layout_vocab_size",
            self.token_layout_vocab_size,
            token_layout_vocab_size,
        )?;
        require_u32(
            "manifest_depth.scratch_generation_gate.go_ast_inventory_empty_prefix.pad_token_id",
            self.pad_token_id,
            token_layout_pad_token,
        )?;
        require_string(
            "manifest_depth.scratch_generation_gate.go_ast_inventory_empty_prefix.empty_prefix_root",
            &self.empty_prefix_root,
            EMPTY_PREFIX_ROOT,
        )?;
        require_string(
            "manifest_depth.scratch_generation_gate.go_ast_inventory_empty_prefix.structural_root_rule",
            &self.structural_root_rule,
            structural_root_rule,
        )?;
        require_string(
            "manifest_depth.scratch_generation_gate.go_ast_inventory_empty_prefix.structural_root_node_kind",
            &self.structural_root_node_kind,
            structural_root_node_kind,
        )?;
        require_string(
            "manifest_depth.scratch_generation_gate.go_ast_inventory_empty_prefix.initial_state_observed_status",
            &self.initial_state_observed_status,
            "partial",
        )?;
        require_usize(
            "manifest_depth.scratch_generation_gate.go_ast_inventory_empty_prefix.initial_state_consumed_raw_tokens",
            self.initial_state_consumed_raw_tokens,
            0,
        )?;
        require_usize(
            "manifest_depth.scratch_generation_gate.go_ast_inventory_empty_prefix.initial_state_total_raw_tokens",
            self.initial_state_total_raw_tokens,
            0,
        )?;
        require_bool(
            "manifest_depth.scratch_generation_gate.go_ast_inventory_empty_prefix.initial_state_eof_valid",
            self.initial_state_eof_valid,
            false,
        )?;
        require_usize(
            "manifest_depth.scratch_generation_gate.go_ast_inventory_empty_prefix.initial_state_stack_depth",
            self.initial_state_stack_depth,
            0,
        )?;
        if self.initial_state_active_rule.is_some() {
            return Err(SyntaxBackendBoundaryError::new(
                "manifest_depth.scratch_generation_gate.go_ast_inventory_empty_prefix.initial_state_active_rule",
                "empty inventory prefix must not expose an active rule",
            ));
        }
        if self.initial_state_active_field.is_some() {
            return Err(SyntaxBackendBoundaryError::new(
                "manifest_depth.scratch_generation_gate.go_ast_inventory_empty_prefix.initial_state_active_field",
                "empty inventory prefix must not expose an active field",
            ));
        }
        require_positive_usize(
            "manifest_depth.scratch_generation_gate.go_ast_inventory_empty_prefix.finite_admissible_raw_token_count",
            self.finite_admissible_raw_token_count,
        )?;
        require_usize(
            "manifest_depth.scratch_generation_gate.go_ast_inventory_empty_prefix.finite_admissible_raw_token_ids",
            self.finite_admissible_raw_token_ids.len(),
            self.finite_admissible_raw_token_count,
        )?;
        let expected_mask_sha256 = json_sha256(&self.finite_admissible_raw_token_ids).map_err(
            |error| {
                SyntaxBackendBoundaryError::new(
                    "manifest_depth.scratch_generation_gate.go_ast_inventory_empty_prefix.finite_admissible_raw_token_ids_sha256",
                    format!("failed to hash inventory empty-prefix admissible raw-token IDs: {error}"),
                )
            },
        )?;
        require_string(
            "manifest_depth.scratch_generation_gate.go_ast_inventory_empty_prefix.finite_admissible_raw_token_ids_sha256",
            &self.finite_admissible_raw_token_ids_sha256,
            &expected_mask_sha256,
        )?;
        if self.selected_raw_token_id >= token_layout_vocab_size {
            return Err(SyntaxBackendBoundaryError::new(
                "manifest_depth.scratch_generation_gate.go_ast_inventory_empty_prefix.selected_raw_token_id",
                "selected inventory empty-prefix raw token must be below VOCAB_SIZE",
            ));
        }
        if !self
            .finite_admissible_raw_token_ids
            .contains(&self.selected_raw_token_id)
        {
            return Err(SyntaxBackendBoundaryError::new(
                "manifest_depth.scratch_generation_gate.go_ast_inventory_empty_prefix.selected_raw_token_id",
                "selected inventory empty-prefix raw token must be admitted by the empty-prefix finite mask",
            ));
        }
        require_bool(
            "manifest_depth.scratch_generation_gate.go_ast_inventory_empty_prefix.selected_raw_token_admitted",
            self.selected_raw_token_admitted,
            true,
        )?;
        require_bool(
            "manifest_depth.scratch_generation_gate.go_ast_inventory_empty_prefix.selected_transition_accepted",
            self.selected_transition_accepted,
            true,
        )?;
        require_string(
            "manifest_depth.scratch_generation_gate.go_ast_inventory_empty_prefix.selected_transition_observed_status",
            &self.selected_transition_observed_status,
            "partial",
        )?;
        require_usize(
            "manifest_depth.scratch_generation_gate.go_ast_inventory_empty_prefix.selected_transition_consumed_raw_tokens",
            self.selected_transition_consumed_raw_tokens,
            1,
        )?;
        require_bool(
            "manifest_depth.scratch_generation_gate.go_ast_inventory_empty_prefix.selected_transition_eof_valid",
            self.selected_transition_eof_valid,
            false,
        )?;
        require_positive_usize(
            "manifest_depth.scratch_generation_gate.go_ast_inventory_empty_prefix.selected_transition_stack_depth",
            self.selected_transition_stack_depth,
        )?;
        require_bool(
            "manifest_depth.scratch_generation_gate.go_ast_inventory_empty_prefix.pad_token_rejected",
            self.pad_token_rejected,
            true,
        )?;
        require_non_empty(
            "manifest_depth.scratch_generation_gate.go_ast_inventory_empty_prefix.pad_token_error_kind",
            &self.pad_token_error_kind,
        )?;
        require_u32(
            "manifest_depth.scratch_generation_gate.go_ast_inventory_empty_prefix.malformed_raw_token_id",
            self.malformed_raw_token_id,
            token_layout_vocab_size,
        )?;
        require_bool(
            "manifest_depth.scratch_generation_gate.go_ast_inventory_empty_prefix.malformed_raw_token_rejected",
            self.malformed_raw_token_rejected,
            true,
        )?;
        require_non_empty(
            "manifest_depth.scratch_generation_gate.go_ast_inventory_empty_prefix.malformed_raw_token_error_kind",
            &self.malformed_raw_token_error_kind,
        )?;
        require_string(
            "manifest_depth.scratch_generation_gate.go_ast_inventory_empty_prefix.claim_boundary",
            &self.claim_boundary,
            GO_AST_INVENTORY_SCRATCH_SEED_EMPTY_PREFIX_CLAIM_BOUNDARY,
        )?;
        require_str_contains(
            field,
            &self.claim_boundary,
            "RawScratchGeneratorCore::empty_prefix",
        )?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoTokenizerGeneratorManifestDepthScratchSeedSummary {
    pub seed_id: String,
    pub grammar_category: String,
    pub status: String,
    pub scope: String,
    pub raw_token_stream_source: String,
    pub contract: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inventory_synthesis_api_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inventory_synthesis_core_api_id: Option<String>,
    pub precompiled_artifact_kind: String,
    pub precompiled_artifact_version: u32,
    pub precompiled_artifact_grammar_id: String,
    pub precompiled_artifact_metadata_path: String,
    pub precompiled_artifact_metadata_sha256: String,
    pub precompiled_artifact_structural_model_id: String,
    pub raw_state_machine_core_boundary_id: String,
    pub raw_state_machine_core_boundary: String,
    pub generator_core_api_id: String,
    pub generator_core_api_boundary: String,
    pub token_layout_vocab_size: u32,
    pub pad_token_id: u32,
    pub empty_prefix_root: String,
    pub raw_token_count: usize,
    pub raw_token_ids: Vec<u32>,
    pub raw_token_ids_sha256: String,
    pub admissible_mask_query_count: usize,
    pub finite_admissible_raw_token_mask_available: bool,
    pub every_selected_raw_token_admitted: bool,
    pub selected_transition_accepted_count: usize,
    pub first_selected_raw_token_id: u32,
    pub final_selected_raw_token_id: u32,
    pub final_state_observed_status: String,
    pub final_state_consumed_raw_tokens: usize,
    pub final_state_eof_valid: bool,
    pub final_state_stack_depth: usize,
    pub decoded_from_token_only_stream: bool,
    pub decoded_tree_reusable: bool,
    pub decoded_tree: GoTokenizerGeneratorDecodedTreeSummary,
    pub covered_node_kinds: Vec<String>,
    pub covered_field_families: Vec<String>,
    pub token_only_root_rule: String,
    pub token_only_root_node_kind: String,
    pub token_only_node_start_count: usize,
    pub token_only_payload_value_count: usize,
    pub expected_rendered_source: String,
    pub rendered_source_sha256: String,
    pub rendered_source_byte_count: usize,
    pub rendered_source_matches_expected: bool,
    pub gofmt_validation: String,
    pub go_parser_reparse_validation: String,
    pub render_reparse_sidecar_validation_ready: bool,
    pub step_trace_sample: Vec<GoTokenizerGeneratorManifestDepthScratchSeedStepSample>,
    /// Canonical elided selected-completion for this seed's expanded stream.
    pub elided_selected_completion: GoInventoryScratchSeedElidedSelectedCompletionEvidence,
    /// Incomplete-prefix rejection for this multi-token seed stream.
    pub incomplete_prefix_rejection: GoInventoryScratchSeedIncompletePrefixRejectionEvidence,
    pub per_output_parser_metadata_required: bool,
    pub source_instance_segment_sidecar_used: bool,
    pub source_prefix_table_used: bool,
    pub arena_ir_used: bool,
    pub route_handoff_provenance_used: bool,
    pub bpe_validity_authority_used: bool,
    pub source_program_validity_claim_allowed: bool,
    pub stable_backend_claimed: bool,
    pub next_ready_task: String,
    pub claim_boundary: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoTokenizerGeneratorManifestDepthScratchSeedStepSample {
    pub step_index: usize,
    pub prefix_raw_token_count: usize,
    pub selected_raw_token_id: u32,
    pub finite_admissible_raw_token_count: usize,
    pub admissible_raw_token_range_count: usize,
    pub payload_end_admissible: bool,
    pub selected_raw_token_admitted: bool,
    pub selected_transition_accepted: bool,
    pub transition_observed_status: String,
    pub transition_consumed_raw_tokens: usize,
    pub transition_eof_valid: bool,
    pub transition_stack_depth: usize,
    pub transition_active_rule: Option<String>,
    pub transition_active_field: Option<String>,
}

impl GoTokenizerGeneratorManifestDepthScratchSeedStepSample {
    fn validate(
        &self,
        raw_token_count: usize,
        token_layout_vocab_size: u32,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        if self.step_index >= raw_token_count {
            return Err(SyntaxBackendBoundaryError::new(
                "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.step_trace_sample.step_index",
                "scratch seed sample step must be inside the raw token stream",
            ));
        }
        require_usize(
            "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.step_trace_sample.prefix_raw_token_count",
            self.prefix_raw_token_count,
            self.step_index,
        )?;
        if self.selected_raw_token_id >= token_layout_vocab_size {
            return Err(SyntaxBackendBoundaryError::new(
                "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.step_trace_sample.selected_raw_token_id",
                "scratch seed sample raw token must be below VOCAB_SIZE",
            ));
        }
        require_positive_usize(
            "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.step_trace_sample.finite_admissible_raw_token_count",
            self.finite_admissible_raw_token_count,
        )?;
        require_bool(
            "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.step_trace_sample.selected_raw_token_admitted",
            self.selected_raw_token_admitted,
            true,
        )?;
        require_bool(
            "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.step_trace_sample.selected_transition_accepted",
            self.selected_transition_accepted,
            true,
        )?;
        if self.transition_observed_status != "partial"
            && self.transition_observed_status != "complete"
        {
            return Err(SyntaxBackendBoundaryError::new(
                "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.step_trace_sample.transition_observed_status",
                "scratch seed sample transition status must be partial or complete",
            ));
        }
        require_usize(
            "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.step_trace_sample.transition_consumed_raw_tokens",
            self.transition_consumed_raw_tokens,
            self.step_index + 1,
        )
    }
}

impl GoTokenizerGeneratorManifestDepthScratchSeedSummary {
    /// Build inventory scratch-seed evidence summary through the source-record
    /// generator-core surface (token-only walk, frontier coverage, render/reparse).
    pub fn from_inventory_seed(
        grammar_coverage: &GoAstSchemaGrammarCoverageReport,
        sidecar: &GrammarStructuralModelSidecar,
        model: &CompiledSyntaxModel,
        seed: GoScratchSeedStream,
    ) -> Result<Self, SyntaxBackendBoundaryError> {
        let raw_token_ids_sha256 = json_sha256(&seed.raw_token_ids).map_err(|error| {
            SyntaxBackendBoundaryError::new(
                "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.raw_token_ids_sha256",
                format!("failed to hash Go AST inventory scratch seed raw token IDs: {error}"),
            )
        })?;
        // Token-only decode, expanded selected-completion sample steps, elided
        // selected-completion, and incomplete-prefix rejection come from the
        // source-record generator-core walk surface (not a megamodule-local
        // RawScratchGeneratorCore construction).
        let seed_walk = GoSourceRecordInventoryScratchSeedWalk::run(model, &seed.raw_token_ids)?;
        require_string(
            "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.raw_token_ids_sha256",
            &seed_walk.token_only_decode.summary.raw_token_ids_sha256,
            &raw_token_ids_sha256,
        )?;
        require_usize(
            "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.elided_selected_completion.expanded_token_count",
            seed_walk.elided_selected_completion.expanded_token_count,
            seed.raw_token_ids.len(),
        )?;
        require_usize(
            "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.incomplete_prefix_rejection.complete_raw_token_count",
            seed_walk
                .incomplete_prefix_rejection
                .complete_raw_token_count,
            seed.raw_token_ids.len(),
        )?;
        let decoded_tree = seed_walk.token_only_decode.decoded_tree;
        let token_only = seed_walk.token_only_decode.summary;
        let elided_selected_completion = seed_walk.elided_selected_completion;
        let incomplete_prefix_rejection = seed_walk.incomplete_prefix_rejection;
        let decoded_node_tree = decoded_tree.decoded_node_tree().map_err(|error| {
            SyntaxBackendBoundaryError::new(
                "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.decoded_tree",
                format!("Go AST inventory scratch seed should expose a decoded node tree: {error}"),
            )
        })?;
        let (covered_node_kinds, covered_field_families) =
            decoded_tree_frontier_coverage(&decoded_node_tree);
        let rendered = render_go_source_from_raw_tree(&decoded_tree).map_err(|error| {
            SyntaxBackendBoundaryError::new(
                "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.render",
                format!("Go AST inventory scratch seed should render from raw tree: {error}"),
            )
        })?;
        let formatted = gofmt_raw_tree_rendered_source(&rendered.source)?;
        let rendered_source_sha256 = sha256_hex(rendered.source.as_bytes());
        let rendered_source_byte_count = rendered.source.len();
        let rendered_source_matches_expected = rendered.source == seed.expected_rendered_source
            && formatted == seed.expected_rendered_source;
        let claim_flags = RawScratchClaimBoundaryFlags::raw_state_machine_only();
        let step_trace_sample = scratch_seed_step_samples(&seed, &seed_walk.sample_steps);
        let first_selected_raw_token_id = seed.raw_token_ids.first().copied().ok_or_else(|| {
            SyntaxBackendBoundaryError::new(
                "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.raw_token_ids",
                "scratch seed stream must contain at least one raw token",
            )
        })?;
        let final_selected_raw_token_id = seed.raw_token_ids.last().copied().ok_or_else(|| {
            SyntaxBackendBoundaryError::new(
                "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.raw_token_ids",
                "scratch seed stream must contain at least one raw token",
            )
        })?;

        let summary = Self {
            seed_id: seed.seed_id.clone(),
            grammar_category: seed.grammar_category,
            status: GO_AST_INVENTORY_SCRATCH_SEED_STATUS.to_string(),
            scope: GO_AST_INVENTORY_SCRATCH_SEED_SCOPE.to_string(),
            raw_token_stream_source: seed.seed_id,
            contract: seed.contract,
            inventory_synthesis_api_id: seed.inventory_synthesis_api_id,
            inventory_synthesis_core_api_id: seed.inventory_synthesis_core_api_id,
            precompiled_artifact_kind: PRECOMPILED_GO_SYNTAX_TOKENIZER_ARTIFACT_KIND.to_string(),
            precompiled_artifact_version: PRECOMPILED_ARTIFACT_VERSION,
            precompiled_artifact_grammar_id: grammar_coverage.grammar_id.clone(),
            precompiled_artifact_metadata_path: grammar_coverage.inventory_artifact_path.clone(),
            precompiled_artifact_metadata_sha256: grammar_coverage
                .inventory_artifact_sha256
                .clone(),
            precompiled_artifact_structural_model_id: sidecar.structural_model.model_id.clone(),
            raw_state_machine_core_boundary_id: model.boundary_id().to_string(),
            raw_state_machine_core_boundary: model.boundary_contract().to_string(),
            generator_core_api_id: seed_walk.generator_core_api_id.to_string(),
            generator_core_api_boundary: seed_walk.generator_core_api_boundary.to_string(),
            token_layout_vocab_size: model.token_roles().vocab_size,
            pad_token_id: model.token_roles().pad_token,
            empty_prefix_root: seed.root,
            raw_token_count: seed.admissibility.raw_token_count,
            raw_token_ids: seed.raw_token_ids,
            raw_token_ids_sha256,
            admissible_mask_query_count: seed.admissibility.admissible_mask_query_count,
            finite_admissible_raw_token_mask_available: seed
                .admissibility
                .finite_admissible_mask_available,
            every_selected_raw_token_admitted: seed.admissibility.every_selected_raw_token_admitted,
            selected_transition_accepted_count: seed
                .admissibility
                .selected_transition_accepted_count,
            first_selected_raw_token_id,
            final_selected_raw_token_id,
            final_state_observed_status: seed.admissibility.final_state.status.as_str().to_string(),
            final_state_consumed_raw_tokens: seed.admissibility.final_state.consumed_raw_tokens,
            final_state_eof_valid: seed.admissibility.final_state.eof_valid,
            final_state_stack_depth: seed.admissibility.final_state.stack_depth,
            decoded_from_token_only_stream: true,
            decoded_tree_reusable: true,
            decoded_tree: GoTokenizerGeneratorDecodedTreeSummary::from_tree(&decoded_tree)?,
            covered_node_kinds,
            covered_field_families,
            token_only_root_rule: token_only.root_rule,
            token_only_root_node_kind: token_only.root_node_kind,
            token_only_node_start_count: token_only.node_start_count,
            token_only_payload_value_count: token_only.payload_value_count,
            expected_rendered_source: seed.expected_rendered_source,
            rendered_source_sha256,
            rendered_source_byte_count,
            rendered_source_matches_expected,
            gofmt_validation: "passed".to_string(),
            go_parser_reparse_validation: "passed".to_string(),
            render_reparse_sidecar_validation_ready: rendered_source_matches_expected,
            step_trace_sample,
            elided_selected_completion,
            incomplete_prefix_rejection,
            per_output_parser_metadata_required: claim_flags.per_output_parser_metadata_required,
            source_instance_segment_sidecar_used: claim_flags.source_instance_segment_sidecar_used,
            source_prefix_table_used: claim_flags.source_prefix_table_used,
            arena_ir_used: claim_flags.arena_ir_used,
            route_handoff_provenance_used: claim_flags.route_handoff_provenance_used,
            bpe_validity_authority_used: claim_flags.bpe_validity_authority_used,
            source_program_validity_claim_allowed: false,
            stable_backend_claimed: claim_flags.stable_backend_claimed,
            next_ready_task: NEXT_READY_TASK.to_string(),
            claim_boundary: GO_AST_INVENTORY_SCRATCH_SEED_CLAIM_BOUNDARY.to_string(),
        };
        summary.validate(grammar_coverage)?;
        Ok(summary)
    }

    pub fn validate(
        &self,
        grammar_coverage: &GoAstSchemaGrammarCoverageReport,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        require_non_empty(
            "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.seed_id",
            &self.seed_id,
        )?;
        require_non_empty(
            "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.grammar_category",
            &self.grammar_category,
        )?;
        require_string(
            "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.status",
            &self.status,
            GO_AST_INVENTORY_SCRATCH_SEED_STATUS,
        )?;
        require_string(
            "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.scope",
            &self.scope,
            GO_AST_INVENTORY_SCRATCH_SEED_SCOPE,
        )?;
        require_string(
            "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.raw_token_stream_source",
            &self.raw_token_stream_source,
            &self.seed_id,
        )?;
        require_non_empty(
            "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.contract",
            &self.contract,
        )?;
        match (
            self.inventory_synthesis_api_id.as_deref(),
            self.inventory_synthesis_core_api_id.as_deref(),
        ) {
            (Some(api_id), Some(core_api_id)) => {
                require_non_empty(
                    "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.inventory_synthesis_api_id",
                    api_id,
                )?;
                require_string(
                    "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.inventory_synthesis_core_api_id",
                    core_api_id,
                    GO_AST_INVENTORY_SYNTHESIS_CORE_API_ID,
                )?;
            }
            (None, None) => {}
            _ => {
                return Err(SyntaxBackendBoundaryError::new(
                    "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.inventory_synthesis_core_api_id",
                    "inventory synthesis API and core API metadata must be present together",
                ));
            }
        }
        require_string(
            "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.precompiled_artifact_kind",
            &self.precompiled_artifact_kind,
            PRECOMPILED_GO_SYNTAX_TOKENIZER_ARTIFACT_KIND,
        )?;
        require_u32(
            "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.precompiled_artifact_version",
            self.precompiled_artifact_version,
            PRECOMPILED_ARTIFACT_VERSION,
        )?;
        require_string(
            "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.precompiled_artifact_grammar_id",
            &self.precompiled_artifact_grammar_id,
            &grammar_coverage.grammar_id,
        )?;
        require_string(
            "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.precompiled_artifact_metadata_path",
            &self.precompiled_artifact_metadata_path,
            &grammar_coverage.inventory_artifact_path,
        )?;
        require_string(
            "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.precompiled_artifact_metadata_sha256",
            &self.precompiled_artifact_metadata_sha256,
            &grammar_coverage.inventory_artifact_sha256,
        )?;
        require_non_empty(
            "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.precompiled_artifact_structural_model_id",
            &self.precompiled_artifact_structural_model_id,
        )?;
        require_string(
            "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.raw_state_machine_core_boundary_id",
            &self.raw_state_machine_core_boundary_id,
            RAW_SYNTAX_STATE_MACHINE_CORE_BOUNDARY_ID,
        )?;
        require_string(
            "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.raw_state_machine_core_boundary",
            &self.raw_state_machine_core_boundary,
            RAW_SYNTAX_STATE_MACHINE_CORE_BOUNDARY,
        )?;
        require_string(
            "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.generator_core_api_id",
            &self.generator_core_api_id,
            RAW_SCRATCH_GENERATOR_CORE_API_ID,
        )?;
        require_string(
            "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.generator_core_api_boundary",
            &self.generator_core_api_boundary,
            RAW_SCRATCH_GENERATOR_CORE_API_BOUNDARY,
        )?;
        require_u32(
            "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.token_layout_vocab_size",
            self.token_layout_vocab_size,
            grammar_coverage.token_layout.vocab_size,
        )?;
        require_u32(
            "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.pad_token_id",
            self.pad_token_id,
            grammar_coverage.token_layout.pad_token,
        )?;
        require_string(
            "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.empty_prefix_root",
            &self.empty_prefix_root,
            GO_AST_INVENTORY_SCRATCH_VAR_DECL_ROOT,
        )?;
        require_positive_usize(
            "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.raw_token_count",
            self.raw_token_count,
        )?;
        require_usize(
            "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.raw_token_ids",
            self.raw_token_ids.len(),
            self.raw_token_count,
        )?;
        for token_id in &self.raw_token_ids {
            if *token_id >= self.token_layout_vocab_size {
                return Err(SyntaxBackendBoundaryError::new(
                    "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.raw_token_ids",
                    "scratch seed raw token IDs must be below VOCAB_SIZE",
                ));
            }
            if *token_id == self.pad_token_id {
                return Err(SyntaxBackendBoundaryError::new(
                    "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.raw_token_ids",
                    "scratch seed raw token IDs must not contain PAD_TOKEN",
                ));
            }
        }
        let raw_token_ids_sha256 = json_sha256(&self.raw_token_ids).map_err(|error| {
            SyntaxBackendBoundaryError::new(
                "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.raw_token_ids_sha256",
                format!("failed to hash Go AST inventory scratch seed raw token IDs: {error}"),
            )
        })?;
        require_string(
            "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.raw_token_ids_sha256",
            &self.raw_token_ids_sha256,
            &raw_token_ids_sha256,
        )?;
        require_usize(
            "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.admissible_mask_query_count",
            self.admissible_mask_query_count,
            self.raw_token_count,
        )?;
        require_bool(
            "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.finite_admissible_raw_token_mask_available",
            self.finite_admissible_raw_token_mask_available,
            true,
        )?;
        require_bool(
            "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.every_selected_raw_token_admitted",
            self.every_selected_raw_token_admitted,
            true,
        )?;
        require_usize(
            "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.selected_transition_accepted_count",
            self.selected_transition_accepted_count,
            self.raw_token_count,
        )?;
        if self.first_selected_raw_token_id != self.raw_token_ids[0] {
            return Err(SyntaxBackendBoundaryError::new(
                "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.first_selected_raw_token_id",
                "first selected raw token must match the seed stream",
            ));
        }
        if self.final_selected_raw_token_id != self.raw_token_ids[self.raw_token_count - 1] {
            return Err(SyntaxBackendBoundaryError::new(
                "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.final_selected_raw_token_id",
                "final selected raw token must match the seed stream",
            ));
        }
        require_string(
            "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.final_state_observed_status",
            &self.final_state_observed_status,
            "complete",
        )?;
        require_usize(
            "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.final_state_consumed_raw_tokens",
            self.final_state_consumed_raw_tokens,
            self.raw_token_count,
        )?;
        require_bool(
            "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.final_state_eof_valid",
            self.final_state_eof_valid,
            true,
        )?;
        require_usize(
            "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.final_state_stack_depth",
            self.final_state_stack_depth,
            0,
        )?;
        require_bool(
            "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.decoded_from_token_only_stream",
            self.decoded_from_token_only_stream,
            true,
        )?;
        require_bool(
            "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.decoded_tree_reusable",
            self.decoded_tree_reusable,
            true,
        )?;
        self.decoded_tree.validate(self.raw_token_count)?;
        validate_sorted_unique_non_empty(
            "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.covered_node_kinds",
            &self.covered_node_kinds,
        )?;
        validate_sorted_unique_non_empty(
            "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.covered_field_families",
            &self.covered_field_families,
        )?;
        require_contains(
            "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.covered_node_kinds",
            &self.covered_node_kinds,
            "File",
        )?;
        require_string(
            "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.token_only_root_rule",
            &self.token_only_root_rule,
            "File",
        )?;
        require_string(
            "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.token_only_root_node_kind",
            &self.token_only_root_node_kind,
            "File",
        )?;
        require_positive_usize(
            "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.token_only_node_start_count",
            self.token_only_node_start_count,
        )?;
        require_positive_usize(
            "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.token_only_payload_value_count",
            self.token_only_payload_value_count,
        )?;
        require_non_empty(
            "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.expected_rendered_source",
            &self.expected_rendered_source,
        )?;
        require_sha256(
            "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.rendered_source_sha256",
            &self.rendered_source_sha256,
        )?;
        require_string(
            "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.rendered_source_sha256",
            &self.rendered_source_sha256,
            &sha256_hex(self.expected_rendered_source.as_bytes()),
        )?;
        require_positive_usize(
            "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.rendered_source_byte_count",
            self.rendered_source_byte_count,
        )?;
        require_bool(
            "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.rendered_source_matches_expected",
            self.rendered_source_matches_expected,
            true,
        )?;
        require_string(
            "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.gofmt_validation",
            &self.gofmt_validation,
            "passed",
        )?;
        require_string(
            "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.go_parser_reparse_validation",
            &self.go_parser_reparse_validation,
            "passed",
        )?;
        require_bool(
            "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.render_reparse_sidecar_validation_ready",
            self.render_reparse_sidecar_validation_ready,
            true,
        )?;
        require_positive_usize(
            "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.step_trace_sample",
            self.step_trace_sample.len(),
        )?;
        if self.step_trace_sample.len() > 6 {
            return Err(SyntaxBackendBoundaryError::new(
                "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.step_trace_sample",
                "scratch seed keeps at most six diagnostic samples",
            ));
        }
        let mut seen_steps = BTreeSet::new();
        for sample in &self.step_trace_sample {
            sample.validate(self.raw_token_count, self.token_layout_vocab_size)?;
            if !seen_steps.insert(sample.step_index) {
                return Err(SyntaxBackendBoundaryError::new(
                    "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.step_trace_sample",
                    format!("duplicate scratch seed sample step {}", sample.step_index),
                ));
            }
        }
        self.elided_selected_completion
            .validate(self.raw_token_count)?;
        self.incomplete_prefix_rejection.validate(
            self.raw_token_count,
            self.token_layout_vocab_size,
            self.pad_token_id,
        )?;
        require_bool(
            "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.per_output_parser_metadata_required",
            self.per_output_parser_metadata_required,
            false,
        )?;
        require_bool(
            "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.source_instance_segment_sidecar_used",
            self.source_instance_segment_sidecar_used,
            false,
        )?;
        require_bool(
            "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.source_prefix_table_used",
            self.source_prefix_table_used,
            false,
        )?;
        require_bool(
            "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.arena_ir_used",
            self.arena_ir_used,
            false,
        )?;
        require_bool(
            "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.route_handoff_provenance_used",
            self.route_handoff_provenance_used,
            false,
        )?;
        require_bool(
            "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.bpe_validity_authority_used",
            self.bpe_validity_authority_used,
            false,
        )?;
        require_bool(
            "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.source_program_validity_claim_allowed",
            self.source_program_validity_claim_allowed,
            false,
        )?;
        require_bool(
            "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.stable_backend_claimed",
            self.stable_backend_claimed,
            false,
        )?;
        require_string(
            "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.next_ready_task",
            &self.next_ready_task,
            NEXT_READY_TASK,
        )?;
        require_str_contains(
            "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.claim_boundary",
            &self.claim_boundary,
            "Go AST inventory token-only",
        )?;
        require_str_contains(
            "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.claim_boundary",
            &self.claim_boundary,
            "raw state-machine admissible masks",
        )?;
        require_str_contains(
            "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.claim_boundary",
            &self.claim_boundary,
            "not source-prefix table",
        )?;
        require_str_contains(
            "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.claim_boundary",
            &self.claim_boundary,
            "not Arena IR",
        )?;
        require_str_contains(
            "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.claim_boundary",
            &self.claim_boundary,
            "not BPE",
        )
    }
}

/// Inventory scratch-seed batch assembled through the source-record generator core.
///
/// Owns inventory lower → structural model → compiled model → inventory-model
/// empty-prefix ([`RawScratchGeneratorCore::empty_prefix`]) →
/// [`go_ast_inventory_scratch_seeds`] → per-seed
/// [`GoTokenizerGeneratorManifestDepthScratchSeedSummary::from_inventory_seed`],
/// the representative package-var seed selection, and seed aggregate counts so
/// the tokenizer-generator megamodule only stamps gate-level fields from this
/// batch result plus record/oracle readiness.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoSourceRecordInventoryScratchSeedBatch {
    /// Inventory-model empty-prefix start shared with inventory seeds.
    pub empty_prefix: GoInventoryScratchSeedEmptyPrefixEvidence,
    pub go_ast_inventory_scratch_seed: GoTokenizerGeneratorManifestDepthScratchSeedSummary,
    pub go_ast_inventory_scratch_seeds: Vec<GoTokenizerGeneratorManifestDepthScratchSeedSummary>,
    pub scratch_seed_count: usize,
    pub scratch_seed_admitted_count: usize,
    pub scratch_seed_render_reparse_ready_count: usize,
    pub inventory_synthesis_backed_seed_count: usize,
    pub first_non_inventory_synthesis_seed_id: Option<String>,
    /// True when every seed decodes from a token-only stream into a reusable tree.
    pub all_seeds_token_only_decode_render_ready: bool,
    /// True when every seed is render/reparse-sidecar ready
    /// (`scratch_seed_render_reparse_ready_count == scratch_seed_count`).
    pub all_seeds_render_reparse_ready: bool,
}

impl GoSourceRecordInventoryScratchSeedBatch {
    /// Build the full inventory scratch-seed batch for the pinned Go AST inventory.
    pub fn from_inventory(
        grammar_coverage: &GoAstSchemaGrammarCoverageReport,
    ) -> Result<Self, SyntaxBackendBoundaryError> {
        let inventory_path = Path::new(GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH);
        let grammar = GoAstSchemaInventoryAdapter::lower_inventory_file(inventory_path).map_err(
            |error| {
                SyntaxBackendBoundaryError::new(
                    "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.inventory",
                    format!("failed to lower checked Go AST inventory for scratch seed: {error}"),
                )
            },
        )?;
        let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout())
            .map_err(|error| {
                SyntaxBackendBoundaryError::new(
                    "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.structural_model",
                    format!("failed to lower Go AST inventory scratch seed model: {error}"),
                )
            })?;
        let model = CompiledSyntaxModel::from_sidecar(&sidecar).map_err(|error| {
            SyntaxBackendBoundaryError::new(
                "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.raw_state_machine",
                format!("failed to compile Go AST inventory scratch seed model: {error}"),
            )
        })?;
        Self::from_compiled_inventory(grammar_coverage, &sidecar, &model)
    }

    /// Build the batch from an already-lowered inventory model (test / reuse hook).
    pub fn from_compiled_inventory(
        grammar_coverage: &GoAstSchemaGrammarCoverageReport,
        sidecar: &GrammarStructuralModelSidecar,
        model: &CompiledSyntaxModel,
    ) -> Result<Self, SyntaxBackendBoundaryError> {
        let empty_prefix = inventory_scratch_seed_empty_prefix_evidence(model, sidecar)?;
        let go_ast_inventory_scratch_seeds = go_ast_inventory_scratch_seeds(model)?
            .into_iter()
            .map(|seed| {
                GoTokenizerGeneratorManifestDepthScratchSeedSummary::from_inventory_seed(
                    grammar_coverage,
                    sidecar,
                    model,
                    seed,
                )
            })
            .collect::<Result<Vec<_>, _>>()?;
        let go_ast_inventory_scratch_seed = go_ast_inventory_scratch_seeds
            .iter()
            .find(|seed| seed.seed_id == GO_AST_INVENTORY_SCRATCH_VAR_DECL_SEED_ID)
            .cloned()
            .ok_or_else(|| {
                SyntaxBackendBoundaryError::new(
                    "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seeds",
                    "package-var scratch seed must remain present in the multi-seed report",
                )
            })?;
        let scratch_seed_count = go_ast_inventory_scratch_seeds.len();
        let scratch_seed_admitted_count = go_ast_inventory_scratch_seeds
            .iter()
            .filter(|seed| {
                seed.every_selected_raw_token_admitted
                    && seed.selected_transition_accepted_count == seed.raw_token_count
                    && seed.final_state_eof_valid
            })
            .count();
        let scratch_seed_render_reparse_ready_count = go_ast_inventory_scratch_seeds
            .iter()
            .filter(|seed| seed.render_reparse_sidecar_validation_ready)
            .count();
        let inventory_synthesis_backed_seed_count = go_ast_inventory_scratch_seeds
            .iter()
            .filter(|seed| seed.inventory_synthesis_api_id.is_some())
            .count();
        let first_non_inventory_synthesis_seed_id = go_ast_inventory_scratch_seeds
            .iter()
            .find(|seed| seed.inventory_synthesis_api_id.is_none())
            .map(|seed| seed.seed_id.clone());
        let all_seeds_token_only_decode_render_ready = go_ast_inventory_scratch_seeds
            .iter()
            .all(|seed| seed.decoded_from_token_only_stream && seed.decoded_tree_reusable);
        let all_seeds_render_reparse_ready =
            scratch_seed_render_reparse_ready_count == scratch_seed_count;

        let batch = Self {
            empty_prefix,
            go_ast_inventory_scratch_seed,
            go_ast_inventory_scratch_seeds,
            scratch_seed_count,
            scratch_seed_admitted_count,
            scratch_seed_render_reparse_ready_count,
            inventory_synthesis_backed_seed_count,
            first_non_inventory_synthesis_seed_id,
            all_seeds_token_only_decode_render_ready,
            all_seeds_render_reparse_ready,
        };
        batch.validate(grammar_coverage)?;
        Ok(batch)
    }

    /// Validate empty-prefix, seed-list identity, aggregate counts, and per-seed
    /// summaries. Every inventory seed must start from the same empty-prefix
    /// root and admit its first selected token under the inventory empty-prefix
    /// finite mask.
    pub fn validate(
        &self,
        grammar_coverage: &GoAstSchemaGrammarCoverageReport,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        let representative = self.go_ast_inventory_scratch_seeds.first().ok_or_else(|| {
            SyntaxBackendBoundaryError::new(
                "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seeds",
                "inventory scratch-seed batch requires at least one seed",
            )
        })?;
        self.empty_prefix.validate(
            representative.token_layout_vocab_size,
            representative.pad_token_id,
            &representative.token_only_root_rule,
            &representative.token_only_root_node_kind,
        )?;
        // Re-validate structural roots against every seed so batch empty-prefix
        // stays coupled to seed decode roots (all inventory seeds are File).
        for seed in &self.go_ast_inventory_scratch_seeds {
            require_string(
                "manifest_depth.scratch_generation_gate.go_ast_inventory_empty_prefix.structural_root_rule",
                &self.empty_prefix.structural_root_rule,
                &seed.token_only_root_rule,
            )?;
            require_string(
                "manifest_depth.scratch_generation_gate.go_ast_inventory_empty_prefix.structural_root_node_kind",
                &self.empty_prefix.structural_root_node_kind,
                &seed.token_only_root_node_kind,
            )?;
            require_string(
                "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.empty_prefix_root",
                &seed.empty_prefix_root,
                &self.empty_prefix.empty_prefix_root,
            )?;
            if !self
                .empty_prefix
                .finite_admissible_raw_token_ids
                .contains(&seed.first_selected_raw_token_id)
            {
                return Err(SyntaxBackendBoundaryError::new(
                    "manifest_depth.scratch_generation_gate.go_ast_inventory_empty_prefix.seed_first_selected_raw_token_id",
                    format!(
                        "inventory seed {} first selected raw token {} must be admitted by the inventory empty-prefix finite mask",
                        seed.seed_id, seed.first_selected_raw_token_id
                    ),
                ));
            }
            require_u32(
                "manifest_depth.scratch_generation_gate.go_ast_inventory_empty_prefix.token_layout_vocab_size",
                self.empty_prefix.token_layout_vocab_size,
                seed.token_layout_vocab_size,
            )?;
            require_u32(
                "manifest_depth.scratch_generation_gate.go_ast_inventory_empty_prefix.pad_token_id",
                self.empty_prefix.pad_token_id,
                seed.pad_token_id,
            )?;
        }
        require_usize(
            "manifest_depth.scratch_generation_gate.scratch_seed_count",
            self.scratch_seed_count,
            self.go_ast_inventory_scratch_seeds.len(),
        )?;
        require_usize(
            "manifest_depth.scratch_generation_gate.scratch_seed_admitted_count",
            self.scratch_seed_admitted_count,
            self.go_ast_inventory_scratch_seeds
                .iter()
                .filter(|seed| {
                    seed.every_selected_raw_token_admitted
                        && seed.selected_transition_accepted_count == seed.raw_token_count
                        && seed.final_state_eof_valid
                })
                .count(),
        )?;
        require_usize(
            "manifest_depth.scratch_generation_gate.scratch_seed_render_reparse_ready_count",
            self.scratch_seed_render_reparse_ready_count,
            self.go_ast_inventory_scratch_seeds
                .iter()
                .filter(|seed| seed.render_reparse_sidecar_validation_ready)
                .count(),
        )?;
        let inventory_synthesis_backed_seed_count = self
            .go_ast_inventory_scratch_seeds
            .iter()
            .filter(|seed| seed.inventory_synthesis_api_id.is_some())
            .count();
        require_usize(
            "manifest_depth.scratch_generation_gate.inventory_synthesis_backed_seed_count",
            self.inventory_synthesis_backed_seed_count,
            inventory_synthesis_backed_seed_count,
        )?;
        let first_non_inventory_synthesis_seed_id = self
            .go_ast_inventory_scratch_seeds
            .iter()
            .find(|seed| seed.inventory_synthesis_api_id.is_none())
            .map(|seed| seed.seed_id.clone());
        if self.first_non_inventory_synthesis_seed_id != first_non_inventory_synthesis_seed_id {
            return Err(SyntaxBackendBoundaryError::new(
                "manifest_depth.scratch_generation_gate.first_non_inventory_synthesis_seed_id",
                "first non-inventory-synthesis seed must match registry order",
            ));
        }
        let expected_token_only = self
            .go_ast_inventory_scratch_seeds
            .iter()
            .all(|seed| seed.decoded_from_token_only_stream && seed.decoded_tree_reusable);
        require_bool(
            "manifest_depth.scratch_generation_gate.all_seeds_token_only_decode_render_ready",
            self.all_seeds_token_only_decode_render_ready,
            expected_token_only,
        )?;
        require_bool(
            "manifest_depth.scratch_generation_gate.all_seeds_render_reparse_ready",
            self.all_seeds_render_reparse_ready,
            self.scratch_seed_render_reparse_ready_count == self.scratch_seed_count,
        )?;
        let mut seed_ids = BTreeSet::new();
        for seed in &self.go_ast_inventory_scratch_seeds {
            seed.validate(grammar_coverage)?;
            if !seed_ids.insert(seed.seed_id.clone()) {
                return Err(SyntaxBackendBoundaryError::new(
                    "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seeds",
                    format!("duplicate scratch seed {}", seed.seed_id),
                ));
            }
        }
        let package_var_seed = self
            .go_ast_inventory_scratch_seeds
            .iter()
            .find(|seed| seed.seed_id == GO_AST_INVENTORY_SCRATCH_VAR_DECL_SEED_ID)
            .ok_or_else(|| {
                SyntaxBackendBoundaryError::new(
                    "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seeds",
                    "package-var scratch seed must remain present",
                )
            })?;
        if package_var_seed != &self.go_ast_inventory_scratch_seed {
            return Err(SyntaxBackendBoundaryError::new(
                "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed",
                "legacy package-var scratch seed field must match the multi-seed package-var entry",
            ));
        }
        self.go_ast_inventory_scratch_seed
            .validate(grammar_coverage)?;
        Ok(())
    }
}

/// Inventory seed+empty-prefix sub-bundle for the scratch-generation gate.
///
/// Generator-core owns the inventory batch (empty-prefix + seed list +
/// aggregates), generator-core / raw-token emission API stamps, empty-prefix
/// root, and fixed claim flags that the inventory scratch path always asserts.
/// Per-record readiness aggregation is separately owned by
/// [`GoSourceRecordManifestDepthRecordReadiness`]. The tokenizer-generator
/// megamodule only supplies oracle + record readiness/representative views when
/// assembling [`GoTokenizerGeneratorManifestDepthScratchGenerationGate`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoSourceRecordInventoryScratchSubGate {
    pub empty_prefix_root: String,
    pub batch: GoSourceRecordInventoryScratchSeedBatch,
    pub source_record_generator_core_api_id: String,
    pub source_record_generator_core_api_boundary: String,
    pub source_record_raw_token_emission_api_id: String,
    pub source_record_raw_token_emission_api_boundary: String,
    pub delegated_raw_scratch_generator_core_api_id: String,
    pub delegated_raw_scratch_generator_core_api_boundary: String,
    pub generator_core_api_id: String,
    pub generator_core_api_boundary: String,
    pub raw_state_machine_runtime_boundary_only: bool,
    pub per_output_parser_metadata_required: bool,
    pub source_instance_segment_sidecar_used: bool,
    pub source_prefix_table_used: bool,
    pub arena_ir_used: bool,
    pub route_handoff_provenance_used: bool,
    pub bpe_validity_authority_used: bool,
}

impl GoSourceRecordInventoryScratchSubGate {
    /// Build the inventory seed+empty-prefix sub-gate for the pinned Go AST inventory.
    pub fn from_inventory(
        grammar_coverage: &GoAstSchemaGrammarCoverageReport,
    ) -> Result<Self, SyntaxBackendBoundaryError> {
        let batch = GoSourceRecordInventoryScratchSeedBatch::from_inventory(grammar_coverage)?;
        Self::from_batch(batch)
    }

    /// Stamp generator-core API / claim fields onto an already-built inventory batch.
    pub fn from_batch(
        batch: GoSourceRecordInventoryScratchSeedBatch,
    ) -> Result<Self, SyntaxBackendBoundaryError> {
        let sub = Self {
            empty_prefix_root: EMPTY_PREFIX_ROOT.to_string(),
            batch,
            source_record_generator_core_api_id: GO_SOURCE_RECORD_GENERATOR_CORE_API_ID.to_string(),
            source_record_generator_core_api_boundary: GO_SOURCE_RECORD_GENERATOR_CORE_API_BOUNDARY
                .to_string(),
            source_record_raw_token_emission_api_id: GO_SOURCE_RECORD_RAW_TOKEN_EMISSION_API_ID
                .to_string(),
            source_record_raw_token_emission_api_boundary:
                GO_SOURCE_RECORD_RAW_TOKEN_EMISSION_API_BOUNDARY.to_string(),
            delegated_raw_scratch_generator_core_api_id: RAW_SCRATCH_GENERATOR_CORE_API_ID
                .to_string(),
            delegated_raw_scratch_generator_core_api_boundary:
                RAW_SCRATCH_GENERATOR_CORE_API_BOUNDARY.to_string(),
            generator_core_api_id: RAW_SCRATCH_GENERATOR_CORE_API_ID.to_string(),
            generator_core_api_boundary: RAW_SCRATCH_GENERATOR_CORE_API_BOUNDARY.to_string(),
            raw_state_machine_runtime_boundary_only: true,
            per_output_parser_metadata_required: false,
            source_instance_segment_sidecar_used: false,
            source_prefix_table_used: false,
            arena_ir_used: false,
            route_handoff_provenance_used: false,
            bpe_validity_authority_used: false,
        };
        sub.validate_stamps()?;
        Ok(sub)
    }

    /// Rebuild the sub-gate from fields already stamped on a full scratch-generation gate.
    ///
    /// Used by gate validation so seed/API/claim identity stays generator-core-owned
    /// rather than re-checked field-by-field in the megamodule.
    #[allow(clippy::too_many_arguments)]
    pub fn from_stamped_fields(
        empty_prefix_root: String,
        batch: GoSourceRecordInventoryScratchSeedBatch,
        source_record_generator_core_api_id: String,
        source_record_generator_core_api_boundary: String,
        source_record_raw_token_emission_api_id: String,
        source_record_raw_token_emission_api_boundary: String,
        delegated_raw_scratch_generator_core_api_id: String,
        delegated_raw_scratch_generator_core_api_boundary: String,
        generator_core_api_id: String,
        generator_core_api_boundary: String,
        raw_state_machine_runtime_boundary_only: bool,
        per_output_parser_metadata_required: bool,
        source_instance_segment_sidecar_used: bool,
        source_prefix_table_used: bool,
        arena_ir_used: bool,
        route_handoff_provenance_used: bool,
        bpe_validity_authority_used: bool,
    ) -> Self {
        Self {
            empty_prefix_root,
            batch,
            source_record_generator_core_api_id,
            source_record_generator_core_api_boundary,
            source_record_raw_token_emission_api_id,
            source_record_raw_token_emission_api_boundary,
            delegated_raw_scratch_generator_core_api_id,
            delegated_raw_scratch_generator_core_api_boundary,
            generator_core_api_id,
            generator_core_api_boundary,
            raw_state_machine_runtime_boundary_only,
            per_output_parser_metadata_required,
            source_instance_segment_sidecar_used,
            source_prefix_table_used,
            arena_ir_used,
            route_handoff_provenance_used,
            bpe_validity_authority_used,
        }
    }

    /// Validate inventory batch identity plus generator-core API / claim stamps.
    pub fn validate(
        &self,
        grammar_coverage: &GoAstSchemaGrammarCoverageReport,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        self.validate_stamps()?;
        self.batch.validate(grammar_coverage)?;
        // Gate-level readiness still requires full seed admission/render counts.
        require_usize(
            "manifest_depth.scratch_generation_gate.scratch_seed_admitted_count",
            self.batch.scratch_seed_admitted_count,
            self.batch.scratch_seed_count,
        )?;
        require_usize(
            "manifest_depth.scratch_generation_gate.scratch_seed_render_reparse_ready_count",
            self.batch.scratch_seed_render_reparse_ready_count,
            self.batch.scratch_seed_count,
        )?;
        Ok(())
    }

    fn validate_stamps(&self) -> Result<(), SyntaxBackendBoundaryError> {
        require_string(
            "manifest_depth.scratch_generation_gate.empty_prefix_root",
            &self.empty_prefix_root,
            EMPTY_PREFIX_ROOT,
        )?;
        require_string(
            "manifest_depth.scratch_generation_gate.empty_prefix_root",
            &self.empty_prefix_root,
            &self.batch.empty_prefix.empty_prefix_root,
        )?;
        require_string(
            "manifest_depth.scratch_generation_gate.source_record_generator_core_api_id",
            &self.source_record_generator_core_api_id,
            GO_SOURCE_RECORD_GENERATOR_CORE_API_ID,
        )?;
        require_string(
            "manifest_depth.scratch_generation_gate.source_record_generator_core_api_boundary",
            &self.source_record_generator_core_api_boundary,
            GO_SOURCE_RECORD_GENERATOR_CORE_API_BOUNDARY,
        )?;
        require_string(
            "manifest_depth.scratch_generation_gate.source_record_raw_token_emission_api_id",
            &self.source_record_raw_token_emission_api_id,
            GO_SOURCE_RECORD_RAW_TOKEN_EMISSION_API_ID,
        )?;
        require_string(
            "manifest_depth.scratch_generation_gate.source_record_raw_token_emission_api_boundary",
            &self.source_record_raw_token_emission_api_boundary,
            GO_SOURCE_RECORD_RAW_TOKEN_EMISSION_API_BOUNDARY,
        )?;
        require_string(
            "manifest_depth.scratch_generation_gate.delegated_raw_scratch_generator_core_api_id",
            &self.delegated_raw_scratch_generator_core_api_id,
            RAW_SCRATCH_GENERATOR_CORE_API_ID,
        )?;
        require_string(
            "manifest_depth.scratch_generation_gate.delegated_raw_scratch_generator_core_api_boundary",
            &self.delegated_raw_scratch_generator_core_api_boundary,
            RAW_SCRATCH_GENERATOR_CORE_API_BOUNDARY,
        )?;
        require_string(
            "manifest_depth.scratch_generation_gate.generator_core_api_id",
            &self.generator_core_api_id,
            RAW_SCRATCH_GENERATOR_CORE_API_ID,
        )?;
        require_string(
            "manifest_depth.scratch_generation_gate.generator_core_api_boundary",
            &self.generator_core_api_boundary,
            RAW_SCRATCH_GENERATOR_CORE_API_BOUNDARY,
        )?;
        require_bool(
            "manifest_depth.scratch_generation_gate.raw_state_machine_runtime_boundary_only",
            self.raw_state_machine_runtime_boundary_only,
            true,
        )?;
        require_bool(
            "manifest_depth.scratch_generation_gate.per_output_parser_metadata_required",
            self.per_output_parser_metadata_required,
            false,
        )?;
        require_bool(
            "manifest_depth.scratch_generation_gate.source_instance_segment_sidecar_used",
            self.source_instance_segment_sidecar_used,
            false,
        )?;
        require_bool(
            "manifest_depth.scratch_generation_gate.source_prefix_table_used",
            self.source_prefix_table_used,
            false,
        )?;
        require_bool(
            "manifest_depth.scratch_generation_gate.arena_ir_used",
            self.arena_ir_used,
            false,
        )?;
        require_bool(
            "manifest_depth.scratch_generation_gate.route_handoff_provenance_used",
            self.route_handoff_provenance_used,
            false,
        )?;
        require_bool(
            "manifest_depth.scratch_generation_gate.bpe_validity_authority_used",
            self.bpe_validity_authority_used,
            false,
        )?;
        Ok(())
    }
}

/// Borrowed per-record fields needed to aggregate manifest-depth readiness.
///
/// The megamodule maps each
/// [`crate::syntax_go_tokenizer_generator::GoTokenizerGeneratorManifestDepthRecord`]
/// into this view so generator-core owns the aggregation predicates without
/// depending on the full record type.
#[derive(Debug, Clone, Copy)]
pub struct GoSourceRecordManifestDepthRecordReadinessView<'a> {
    pub empty_prefix_generation: &'a GoTokenizerGeneratorEmptyPrefixGenerationEvidence,
    pub selected_completion_smoke: &'a GoTokenizerGeneratorSelectedCompletionSmokeEvidence,
    /// Elided-path selected-completion (canonical stream); parallel to expanded smoke.
    pub elided_selected_completion: &'a GoSourceRecordElidedSelectedCompletionEvidence,
    pub incomplete_prefix_rejection: &'a GoTokenizerGeneratorIncompletePrefixRejectionEvidence,
    pub token_only_decode_render: &'a GoTokenizerGeneratorTokenOnlyDecodeRenderEvidence,
    pub gofmt_validation: &'a str,
    pub go_parser_reparse_validation: &'a str,
    pub source_program_validity_claim_allowed: bool,
    pub stable_promotion_allowed: bool,
}

impl<'a> GoSourceRecordManifestDepthRecordReadinessView<'a> {
    /// Build a readiness view from source-record generator-core evidence plus
    /// record-level gofmt/reparse and claim flags.
    pub fn from_source_record_evidence(
        evidence: &'a GoSourceRecordGeneratorEvidence,
        gofmt_validation: &'a str,
        go_parser_reparse_validation: &'a str,
        source_program_validity_claim_allowed: bool,
        stable_promotion_allowed: bool,
    ) -> Self {
        Self {
            empty_prefix_generation: &evidence.empty_prefix_generation,
            selected_completion_smoke: &evidence.selected_completion_smoke,
            elided_selected_completion: &evidence.elided_selected_completion,
            incomplete_prefix_rejection: &evidence.incomplete_prefix_rejection,
            token_only_decode_render: &evidence.token_only_decode_render,
            gofmt_validation,
            go_parser_reparse_validation,
            source_program_validity_claim_allowed,
            stable_promotion_allowed,
        }
    }

    fn empty_prefix_ready(self) -> bool {
        self.empty_prefix_generation.status == EMPTY_PREFIX_GENERATION_STATUS
            && self.empty_prefix_generation.empty_prefix_root == EMPTY_PREFIX_ROOT
    }

    fn finite_admissible_mask_ready(self) -> bool {
        self.empty_prefix_generation.empty_prefix_root == EMPTY_PREFIX_ROOT
            && self
                .empty_prefix_generation
                .finite_admissible_raw_token_count
                > 0
    }

    fn selected_transition_accepted_ready(self) -> bool {
        self.empty_prefix_generation.selected_transition_accepted
            && self.empty_prefix_generation.selected_raw_token_admitted
    }

    fn selected_completion_smoke_ready(self) -> bool {
        self.selected_completion_smoke.status == SELECTED_COMPLETION_SMOKE_STATUS
            && self
                .selected_completion_smoke
                .every_selected_raw_token_admitted
            && self.selected_completion_smoke.final_state_eof_valid
    }

    /// Elided-path selected-completion readiness (status + admission + expand
    /// parity + EOF + reusable decode), parallel to expanded smoke readiness.
    fn elided_selected_completion_ready(self) -> bool {
        self.elided_selected_completion.status == GO_SOURCE_RECORD_ELIDED_SELECTED_COMPLETION_STATUS
            && self.elided_selected_completion.stream_contract == LIVE_RAW_SYNTAX_STREAM_CONTRACT
            && self
                .elided_selected_completion
                .expansion_matches_expanded_stream
            && self
                .elided_selected_completion
                .every_selected_elided_token_admitted
            && self
                .elided_selected_completion
                .every_elided_position_non_forced
            && self.elided_selected_completion.final_state_eof_valid
            && self.elided_selected_completion.decoded_tree_reusable
    }

    fn incomplete_prefix_rejection_ready(self) -> bool {
        self.incomplete_prefix_rejection.status == INCOMPLETE_PREFIX_REJECTION_STATUS
            && self.incomplete_prefix_rejection.continuation_mask_available
            && self.incomplete_prefix_rejection.strict_completion_rejected
            && self
                .incomplete_prefix_rejection
                .selected_next_transition_accepted
    }

    fn token_only_decode_render_ready(self) -> bool {
        self.token_only_decode_render.status == TOKEN_ONLY_DECODE_RENDER_STATUS
            && self.token_only_decode_render.decoded_from_token_only_stream
            && self.token_only_decode_render.decoded_tree_reusable
    }

    fn render_reparse_sidecar_validation_ready(self) -> bool {
        self.selected_completion_smoke
            .render_reparse_sidecar_validation
            && self
                .token_only_decode_render
                .render_reparse_sidecar_validation
            && self.gofmt_validation == "passed"
            && self.go_parser_reparse_validation == "passed"
    }
}

/// Aggregated per-record readiness over a manifest-depth record slice.
///
/// Generator-core owns empty-prefix / expanded selected-completion / elided
/// selected-completion / incomplete-prefix / token-only / render-reparse counts
/// and readiness flags. Full gate assembly
/// ([`GoTokenizerGeneratorManifestDepthScratchGenerationGate::from_oracle_and_record_views`])
/// merges this aggregation with inventory-sub + oracle attachment. Inventory
/// seed conjunction for final token-only / render-reparse readiness uses
/// [`Self::with_inventory_seed_readiness`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoSourceRecordManifestDepthRecordReadiness {
    pub supported_record_count: usize,
    pub finite_admissible_mask_record_count: usize,
    pub selected_transition_accepted_record_count: usize,
    pub empty_prefix_generation_ready: bool,
    pub selected_completion_smoke_ready: bool,
    /// Per-record elided selected-completion readiness (all records).
    pub elided_selected_completion_ready: bool,
    pub incomplete_prefix_rejection_ready: bool,
    /// Per-record token-only readiness before inventory-seed conjunction.
    pub record_token_only_decode_render_ready: bool,
    /// Per-record render/reparse readiness before inventory-seed conjunction.
    pub record_render_reparse_sidecar_validation_ready: bool,
    pub source_program_validity_claim_allowed: bool,
    pub stable_backend_claimed: bool,
}

impl GoSourceRecordManifestDepthRecordReadiness {
    /// Aggregate readiness over one or more borrowed record views.
    pub fn from_record_views<'a>(
        views: impl IntoIterator<Item = GoSourceRecordManifestDepthRecordReadinessView<'a>>,
    ) -> Result<Self, SyntaxBackendBoundaryError> {
        let views: Vec<_> = views.into_iter().collect();
        if views.is_empty() {
            return Err(SyntaxBackendBoundaryError::new(
                "manifest_depth.scratch_generation_gate.records",
                "scratch-generation gate requires at least one manifest-depth record",
            ));
        }
        let supported_record_count = views.len();
        let finite_admissible_mask_record_count = views
            .iter()
            .filter(|view| view.finite_admissible_mask_ready())
            .count();
        let selected_transition_accepted_record_count = views
            .iter()
            .filter(|view| view.selected_transition_accepted_ready())
            .count();
        let empty_prefix_generation_ready = views.iter().all(|view| view.empty_prefix_ready());
        let selected_completion_smoke_ready = views
            .iter()
            .all(|view| view.selected_completion_smoke_ready());
        let elided_selected_completion_ready = views
            .iter()
            .all(|view| view.elided_selected_completion_ready());
        let incomplete_prefix_rejection_ready = views
            .iter()
            .all(|view| view.incomplete_prefix_rejection_ready());
        let record_token_only_decode_render_ready = views
            .iter()
            .all(|view| view.token_only_decode_render_ready());
        let record_render_reparse_sidecar_validation_ready = views
            .iter()
            .all(|view| view.render_reparse_sidecar_validation_ready());
        let source_program_validity_claim_allowed = views
            .iter()
            .any(|view| view.source_program_validity_claim_allowed);
        let stable_backend_claimed = views.iter().any(|view| view.stable_promotion_allowed);

        Ok(Self {
            supported_record_count,
            finite_admissible_mask_record_count,
            selected_transition_accepted_record_count,
            empty_prefix_generation_ready,
            selected_completion_smoke_ready,
            elided_selected_completion_ready,
            incomplete_prefix_rejection_ready,
            record_token_only_decode_render_ready,
            record_render_reparse_sidecar_validation_ready,
            source_program_validity_claim_allowed,
            stable_backend_claimed,
        })
    }

    /// Final token-only and render/reparse readiness after AND with inventory
    /// seed-batch flags (the conjunction the scratch-generation gate stamps).
    pub fn with_inventory_seed_readiness(
        &self,
        batch: &GoSourceRecordInventoryScratchSeedBatch,
    ) -> (bool, bool) {
        (
            self.record_token_only_decode_render_ready
                && batch.all_seeds_token_only_decode_render_ready,
            self.record_render_reparse_sidecar_validation_ready
                && batch.all_seeds_render_reparse_ready,
        )
    }

    /// Check gate-stamped readiness fields against this aggregation (including
    /// inventory seed conjunction for final token-only / render-reparse flags).
    #[allow(clippy::too_many_arguments)]
    pub fn validate_gate_fields(
        &self,
        batch: &GoSourceRecordInventoryScratchSeedBatch,
        empty_prefix_generation_ready: bool,
        finite_admissible_mask_record_count: usize,
        selected_transition_accepted_record_count: usize,
        selected_completion_smoke_ready: bool,
        elided_selected_completion_ready: bool,
        incomplete_prefix_rejection_ready: bool,
        token_only_decode_render_ready: bool,
        render_reparse_sidecar_validation_ready: bool,
        source_program_validity_claim_allowed: bool,
        stable_backend_claimed: bool,
        supported_record_count: usize,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        let (expected_token_only, expected_render_reparse) =
            self.with_inventory_seed_readiness(batch);
        require_usize(
            "manifest_depth.scratch_generation_gate.supported_record_count",
            supported_record_count,
            self.supported_record_count,
        )?;
        require_bool(
            "manifest_depth.scratch_generation_gate.empty_prefix_generation_ready",
            empty_prefix_generation_ready,
            self.empty_prefix_generation_ready,
        )?;
        require_usize(
            "manifest_depth.scratch_generation_gate.finite_admissible_mask_record_count",
            finite_admissible_mask_record_count,
            self.finite_admissible_mask_record_count,
        )?;
        require_usize(
            "manifest_depth.scratch_generation_gate.selected_transition_accepted_record_count",
            selected_transition_accepted_record_count,
            self.selected_transition_accepted_record_count,
        )?;
        require_bool(
            "manifest_depth.scratch_generation_gate.selected_completion_smoke_ready",
            selected_completion_smoke_ready,
            self.selected_completion_smoke_ready,
        )?;
        require_bool(
            "manifest_depth.scratch_generation_gate.elided_selected_completion_ready",
            elided_selected_completion_ready,
            self.elided_selected_completion_ready,
        )?;
        require_bool(
            "manifest_depth.scratch_generation_gate.incomplete_prefix_rejection_ready",
            incomplete_prefix_rejection_ready,
            self.incomplete_prefix_rejection_ready,
        )?;
        require_bool(
            "manifest_depth.scratch_generation_gate.token_only_decode_render_ready",
            token_only_decode_render_ready,
            expected_token_only,
        )?;
        require_bool(
            "manifest_depth.scratch_generation_gate.render_reparse_sidecar_validation_ready",
            render_reparse_sidecar_validation_ready,
            expected_render_reparse,
        )?;
        require_bool(
            "manifest_depth.scratch_generation_gate.source_program_validity_claim_allowed",
            source_program_validity_claim_allowed,
            self.source_program_validity_claim_allowed,
        )?;
        require_bool(
            "manifest_depth.scratch_generation_gate.stable_backend_claimed",
            stable_backend_claimed,
            self.stable_backend_claimed,
        )?;
        // Gate contract still requires fully-ready current-plus records.
        require_bool(
            "manifest_depth.scratch_generation_gate.empty_prefix_generation_ready",
            self.empty_prefix_generation_ready,
            true,
        )?;
        require_usize(
            "manifest_depth.scratch_generation_gate.finite_admissible_mask_record_count",
            self.finite_admissible_mask_record_count,
            self.supported_record_count,
        )?;
        require_usize(
            "manifest_depth.scratch_generation_gate.selected_transition_accepted_record_count",
            self.selected_transition_accepted_record_count,
            self.supported_record_count,
        )?;
        require_bool(
            "manifest_depth.scratch_generation_gate.selected_completion_smoke_ready",
            self.selected_completion_smoke_ready,
            true,
        )?;
        require_bool(
            "manifest_depth.scratch_generation_gate.elided_selected_completion_ready",
            self.elided_selected_completion_ready,
            true,
        )?;
        require_bool(
            "manifest_depth.scratch_generation_gate.incomplete_prefix_rejection_ready",
            self.incomplete_prefix_rejection_ready,
            true,
        )?;
        require_bool(
            "manifest_depth.scratch_generation_gate.token_only_decode_render_ready",
            expected_token_only,
            true,
        )?;
        require_bool(
            "manifest_depth.scratch_generation_gate.render_reparse_sidecar_validation_ready",
            expected_render_reparse,
            true,
        )?;
        require_bool(
            "manifest_depth.scratch_generation_gate.source_program_validity_claim_allowed",
            self.source_program_validity_claim_allowed,
            false,
        )?;
        require_bool(
            "manifest_depth.scratch_generation_gate.stable_backend_claimed",
            self.stable_backend_claimed,
            false,
        )?;
        Ok(())
    }
}

/// Source-record evidence fields stamped onto a manifest-depth record (and
/// mirrored onto the tokenizer-generator report).
///
/// Generator-core owns API/stream identity plus empty-prefix / selected-
/// completion / elided selected-completion / incomplete-prefix / token-only
/// decode-render stamping from [`GoSourceRecordGeneratorEvidence`] so the
/// megamodule only wires generated-codec parity, raw-tree render attempts, and
/// report-level glue when building records. Expanded raw-token counts, SM
/// decode summaries, raw-tree render blockers, and gofmt/reparse claim flags
/// remain megamodule-owned.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoSourceRecordManifestDepthRecordSourceFields {
    pub source_record_generator_core_api_id: String,
    pub source_record_generator_core_api_boundary: String,
    pub source_record_raw_token_emission_api_id: String,
    pub source_record_raw_token_emission_api_boundary: String,
    pub delegated_raw_scratch_generator_core_api_id: String,
    pub delegated_raw_scratch_generator_core_api_boundary: String,
    pub stream_contract: String,
    pub canonical_token_count: usize,
    pub canonical_tokens_sha256: String,
    pub canonical_max_token_id: Option<u32>,
    pub forced_token_count: usize,
    pub empty_prefix_generation: GoTokenizerGeneratorEmptyPrefixGenerationEvidence,
    pub selected_completion_smoke: GoTokenizerGeneratorSelectedCompletionSmokeEvidence,
    /// Elided-path selected-completion (not only evidence-bundle-internal).
    pub elided_selected_completion: GoSourceRecordElidedSelectedCompletionEvidence,
    pub incomplete_prefix_rejection: GoTokenizerGeneratorIncompletePrefixRejectionEvidence,
    pub token_only_decode_render: GoTokenizerGeneratorTokenOnlyDecodeRenderEvidence,
}

impl GoSourceRecordManifestDepthRecordSourceFields {
    /// Stamp manifest-depth source-record fields from generator-core evidence.
    ///
    /// Prefer this when `GoSourceRecordGeneratorEvidence` is still in hand
    /// (artifact construction and manifest-depth record construction that
    /// retains evidence on artifacts).
    pub fn from_generator_evidence(evidence: &GoSourceRecordGeneratorEvidence) -> Self {
        Self {
            source_record_generator_core_api_id: evidence
                .source_record_generator_core_api_id
                .clone(),
            source_record_generator_core_api_boundary: evidence
                .source_record_generator_core_api_boundary
                .clone(),
            source_record_raw_token_emission_api_id: evidence
                .source_record_raw_token_emission_api_id
                .clone(),
            source_record_raw_token_emission_api_boundary: evidence
                .source_record_raw_token_emission_api_boundary
                .clone(),
            delegated_raw_scratch_generator_core_api_id: evidence
                .delegated_raw_scratch_generator_core_api_id
                .clone(),
            delegated_raw_scratch_generator_core_api_boundary: evidence
                .delegated_raw_scratch_generator_core_api_boundary
                .clone(),
            stream_contract: evidence.stream_contract.clone(),
            canonical_token_count: evidence.canonical_token_count,
            canonical_tokens_sha256: evidence.canonical_tokens_sha256.clone(),
            canonical_max_token_id: evidence.canonical_max_token_id,
            forced_token_count: evidence.forced_token_count,
            empty_prefix_generation: evidence.empty_prefix_generation.clone(),
            selected_completion_smoke: evidence.selected_completion_smoke.clone(),
            elided_selected_completion: evidence.elided_selected_completion.clone(),
            incomplete_prefix_rejection: evidence.incomplete_prefix_rejection.clone(),
            token_only_decode_render: evidence.token_only_decode_render.clone(),
        }
    }

    /// Validate generator-core / emission / delegated-scratch API identity and
    /// the live elided stream contract for stamped source-record fields.
    pub fn validate_api_and_stream_identity(&self) -> Result<(), SyntaxBackendBoundaryError> {
        require_string(
            "manifest_depth.record.source_record_generator_core_api_id",
            &self.source_record_generator_core_api_id,
            GO_SOURCE_RECORD_GENERATOR_CORE_API_ID,
        )?;
        require_string(
            "manifest_depth.record.source_record_generator_core_api_boundary",
            &self.source_record_generator_core_api_boundary,
            GO_SOURCE_RECORD_GENERATOR_CORE_API_BOUNDARY,
        )?;
        require_string(
            "manifest_depth.record.source_record_raw_token_emission_api_id",
            &self.source_record_raw_token_emission_api_id,
            GO_SOURCE_RECORD_RAW_TOKEN_EMISSION_API_ID,
        )?;
        require_string(
            "manifest_depth.record.source_record_raw_token_emission_api_boundary",
            &self.source_record_raw_token_emission_api_boundary,
            GO_SOURCE_RECORD_RAW_TOKEN_EMISSION_API_BOUNDARY,
        )?;
        require_string(
            "manifest_depth.record.delegated_raw_scratch_generator_core_api_id",
            &self.delegated_raw_scratch_generator_core_api_id,
            RAW_SCRATCH_GENERATOR_CORE_API_ID,
        )?;
        require_string(
            "manifest_depth.record.delegated_raw_scratch_generator_core_api_boundary",
            &self.delegated_raw_scratch_generator_core_api_boundary,
            RAW_SCRATCH_GENERATOR_CORE_API_BOUNDARY,
        )?;
        require_string(
            "manifest_depth.record.stream_contract",
            &self.stream_contract,
            LIVE_RAW_SYNTAX_STREAM_CONTRACT,
        )?;
        require_positive_usize(
            "manifest_depth.record.canonical_token_count",
            self.canonical_token_count,
        )?;
        require_sha256(
            "manifest_depth.record.canonical_tokens_sha256",
            &self.canonical_tokens_sha256,
        )?;
        Ok(())
    }

    /// Require a report surface that absorbed generator-core evidence still
    /// matches this stamp (manifest-depth record parity against the report).
    #[allow(clippy::too_many_arguments)]
    pub fn require_absorbed_report_parity(
        &self,
        source_record_generator_core_api_id: &str,
        source_record_generator_core_api_boundary: &str,
        source_record_raw_token_emission_api_id: &str,
        source_record_raw_token_emission_api_boundary: &str,
        delegated_raw_scratch_generator_core_api_id: &str,
        delegated_raw_scratch_generator_core_api_boundary: &str,
        stream_contract: &str,
        canonical_token_count: usize,
        canonical_tokens_sha256: &str,
        canonical_max_token_id: Option<u32>,
        forced_token_count: usize,
        empty_prefix_generation: &GoTokenizerGeneratorEmptyPrefixGenerationEvidence,
        selected_completion_smoke: &GoTokenizerGeneratorSelectedCompletionSmokeEvidence,
        elided_selected_completion: &GoSourceRecordElidedSelectedCompletionEvidence,
        incomplete_prefix_rejection: &GoTokenizerGeneratorIncompletePrefixRejectionEvidence,
        token_only_decode_render: &GoTokenizerGeneratorTokenOnlyDecodeRenderEvidence,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        require_string(
            "manifest_depth.record.report.source_record_generator_core_api_id",
            source_record_generator_core_api_id,
            &self.source_record_generator_core_api_id,
        )?;
        require_string(
            "manifest_depth.record.report.source_record_generator_core_api_boundary",
            source_record_generator_core_api_boundary,
            &self.source_record_generator_core_api_boundary,
        )?;
        require_string(
            "manifest_depth.record.report.source_record_raw_token_emission_api_id",
            source_record_raw_token_emission_api_id,
            &self.source_record_raw_token_emission_api_id,
        )?;
        require_string(
            "manifest_depth.record.report.source_record_raw_token_emission_api_boundary",
            source_record_raw_token_emission_api_boundary,
            &self.source_record_raw_token_emission_api_boundary,
        )?;
        require_string(
            "manifest_depth.record.report.delegated_raw_scratch_generator_core_api_id",
            delegated_raw_scratch_generator_core_api_id,
            &self.delegated_raw_scratch_generator_core_api_id,
        )?;
        require_string(
            "manifest_depth.record.report.delegated_raw_scratch_generator_core_api_boundary",
            delegated_raw_scratch_generator_core_api_boundary,
            &self.delegated_raw_scratch_generator_core_api_boundary,
        )?;
        require_string(
            "manifest_depth.record.report.stream_contract",
            stream_contract,
            &self.stream_contract,
        )?;
        require_usize(
            "manifest_depth.record.report.canonical_token_count",
            canonical_token_count,
            self.canonical_token_count,
        )?;
        require_string(
            "manifest_depth.record.report.canonical_tokens_sha256",
            canonical_tokens_sha256,
            &self.canonical_tokens_sha256,
        )?;
        if canonical_max_token_id != self.canonical_max_token_id {
            return Err(SyntaxBackendBoundaryError::new(
                "manifest_depth.record.report.canonical_max_token_id",
                "report canonical max token id must match source-record generator-core stamp",
            ));
        }
        require_usize(
            "manifest_depth.record.report.forced_token_count",
            forced_token_count,
            self.forced_token_count,
        )?;
        if empty_prefix_generation != &self.empty_prefix_generation {
            return Err(SyntaxBackendBoundaryError::new(
                "manifest_depth.record.report.empty_prefix_generation",
                "report empty-prefix evidence must match source-record generator-core stamp",
            ));
        }
        if selected_completion_smoke != &self.selected_completion_smoke {
            return Err(SyntaxBackendBoundaryError::new(
                "manifest_depth.record.report.selected_completion_smoke",
                "report selected-completion evidence must match source-record generator-core stamp",
            ));
        }
        if elided_selected_completion != &self.elided_selected_completion {
            return Err(SyntaxBackendBoundaryError::new(
                "manifest_depth.record.report.elided_selected_completion",
                "report elided selected-completion evidence must match source-record generator-core stamp",
            ));
        }
        if incomplete_prefix_rejection != &self.incomplete_prefix_rejection {
            return Err(SyntaxBackendBoundaryError::new(
                "manifest_depth.record.report.incomplete_prefix_rejection",
                "report incomplete-prefix evidence must match source-record generator-core stamp",
            ));
        }
        if token_only_decode_render != &self.token_only_decode_render {
            return Err(SyntaxBackendBoundaryError::new(
                "manifest_depth.record.report.token_only_decode_render",
                "report token-only decode/render evidence must match source-record generator-core stamp",
            ));
        }
        Ok(())
    }
}

/// Borrowed oracle cutover fields the full scratch-generation gate stamps and
/// re-validates against.
///
/// The megamodule maps
/// `GoTokenizerGeneratorManifestDepthOracleCutoverEvidence` into this view so
/// generator-core owns gate assembly without depending on oracle report parsing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GoSourceRecordOracleCutoverAttachmentView<'a> {
    pub report_path: &'a str,
    pub report_sha256: &'a str,
    pub status: &'a str,
    pub accepted: bool,
    pub record_count: usize,
    pub canonical_fidelity_only_gap_count: usize,
    pub claim_boundary: &'a str,
}

/// Borrowed representative manifest-depth record paths stamped on the full gate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GoSourceRecordManifestDepthRepresentativeView<'a> {
    pub manifest_path: &'a str,
    pub source_path: &'a str,
    pub raw_token_ids_sha256: &'a str,
}

/// Full empty-prefix scratch-generation gate for manifest-depth evidence.
///
/// Generator-core owns status/scope/claim constants, oracle cutover attachment,
/// representative path stamping, inventory seed+empty-prefix sub-gate merge,
/// and per-record readiness aggregation. The tokenizer-generator megamodule only
/// supplies oracle attachment + readiness/representative views (and grammar
/// coverage for inventory rebuild).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoTokenizerGeneratorManifestDepthScratchGenerationGate {
    pub status: String,
    pub scope: String,
    pub precompiled_artifact_kind: String,
    pub empty_prefix_root: String,
    pub grammar_mode_render_authority_source: String,
    pub oracle_report_path: String,
    pub oracle_report_sha256: String,
    pub oracle_cutover_status: String,
    pub oracle_cutover_accepted: bool,
    pub oracle_record_count: usize,
    pub oracle_canonical_fidelity_only_gap_count: usize,
    pub canonical_fidelity_gaps_diagnostic_only: bool,
    /// Inventory-model empty-prefix from generator-core seed batch.
    pub go_ast_inventory_empty_prefix: GoInventoryScratchSeedEmptyPrefixEvidence,
    pub go_ast_inventory_scratch_seed: GoTokenizerGeneratorManifestDepthScratchSeedSummary,
    pub go_ast_inventory_scratch_seeds: Vec<GoTokenizerGeneratorManifestDepthScratchSeedSummary>,
    pub scratch_seed_count: usize,
    pub scratch_seed_admitted_count: usize,
    pub scratch_seed_render_reparse_ready_count: usize,
    pub inventory_synthesis_backed_seed_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_non_inventory_synthesis_seed_id: Option<String>,
    pub representative_manifest_path: String,
    pub representative_source_path: String,
    pub representative_raw_token_ids_sha256: String,
    pub supported_record_count: usize,
    pub source_record_generator_core_api_id: String,
    pub source_record_generator_core_api_boundary: String,
    pub source_record_raw_token_emission_api_id: String,
    pub source_record_raw_token_emission_api_boundary: String,
    pub delegated_raw_scratch_generator_core_api_id: String,
    pub delegated_raw_scratch_generator_core_api_boundary: String,
    pub generator_core_api_id: String,
    pub generator_core_api_boundary: String,
    pub empty_prefix_generation_ready: bool,
    pub finite_admissible_mask_record_count: usize,
    pub selected_transition_accepted_record_count: usize,
    pub selected_completion_smoke_ready: bool,
    /// Elided-path selected-completion readiness over the record slice.
    pub elided_selected_completion_ready: bool,
    pub incomplete_prefix_rejection_ready: bool,
    pub token_only_decode_render_ready: bool,
    pub render_reparse_sidecar_validation_ready: bool,
    pub raw_state_machine_runtime_boundary_only: bool,
    pub per_output_parser_metadata_required: bool,
    pub source_instance_segment_sidecar_used: bool,
    pub source_prefix_table_used: bool,
    pub arena_ir_used: bool,
    pub route_handoff_provenance_used: bool,
    pub bpe_validity_authority_used: bool,
    pub source_program_validity_claim_allowed: bool,
    pub stable_backend_claimed: bool,
    pub next_ready_task: String,
    pub claim_boundary: String,
}

impl GoTokenizerGeneratorManifestDepthScratchGenerationGate {
    /// Assemble the full scratch-generation gate from oracle attachment, inventory
    /// grammar coverage, per-record readiness views, and representative paths.
    pub fn from_oracle_and_record_views<'a>(
        oracle: GoSourceRecordOracleCutoverAttachmentView<'_>,
        grammar_coverage: &GoAstSchemaGrammarCoverageReport,
        record_views: impl IntoIterator<Item = GoSourceRecordManifestDepthRecordReadinessView<'a>>,
        representative: GoSourceRecordManifestDepthRepresentativeView<'_>,
    ) -> Result<Self, SyntaxBackendBoundaryError> {
        let record_readiness =
            GoSourceRecordManifestDepthRecordReadiness::from_record_views(record_views)?;
        let inventory_sub =
            GoSourceRecordInventoryScratchSubGate::from_inventory(grammar_coverage)?;
        let (token_only_decode_render_ready, render_reparse_sidecar_validation_ready) =
            record_readiness.with_inventory_seed_readiness(&inventory_sub.batch);

        let gate = Self {
            status: SCRATCH_GENERATION_GATE_STATUS.to_string(),
            scope: SCRATCH_GENERATION_GATE_SCOPE.to_string(),
            precompiled_artifact_kind: PRECOMPILED_GO_SYNTAX_TOKENIZER_ARTIFACT_KIND.to_string(),
            empty_prefix_root: inventory_sub.empty_prefix_root.clone(),
            grammar_mode_render_authority_source: SCRATCH_GENERATION_GATE_RENDER_AUTHORITY_SOURCE
                .to_string(),
            oracle_report_path: oracle.report_path.to_string(),
            oracle_report_sha256: oracle.report_sha256.to_string(),
            oracle_cutover_status: oracle.status.to_string(),
            oracle_cutover_accepted: oracle.accepted,
            oracle_record_count: oracle.record_count,
            oracle_canonical_fidelity_only_gap_count: oracle.canonical_fidelity_only_gap_count,
            canonical_fidelity_gaps_diagnostic_only: oracle
                .claim_boundary
                .contains("source fidelity remains diagnostic"),
            go_ast_inventory_empty_prefix: inventory_sub.batch.empty_prefix.clone(),
            go_ast_inventory_scratch_seed: inventory_sub
                .batch
                .go_ast_inventory_scratch_seed
                .clone(),
            go_ast_inventory_scratch_seeds: inventory_sub
                .batch
                .go_ast_inventory_scratch_seeds
                .clone(),
            scratch_seed_count: inventory_sub.batch.scratch_seed_count,
            scratch_seed_admitted_count: inventory_sub.batch.scratch_seed_admitted_count,
            scratch_seed_render_reparse_ready_count: inventory_sub
                .batch
                .scratch_seed_render_reparse_ready_count,
            inventory_synthesis_backed_seed_count: inventory_sub
                .batch
                .inventory_synthesis_backed_seed_count,
            first_non_inventory_synthesis_seed_id: inventory_sub
                .batch
                .first_non_inventory_synthesis_seed_id
                .clone(),
            representative_manifest_path: representative.manifest_path.to_string(),
            representative_source_path: representative.source_path.to_string(),
            representative_raw_token_ids_sha256: representative.raw_token_ids_sha256.to_string(),
            supported_record_count: record_readiness.supported_record_count,
            source_record_generator_core_api_id: inventory_sub
                .source_record_generator_core_api_id
                .clone(),
            source_record_generator_core_api_boundary: inventory_sub
                .source_record_generator_core_api_boundary
                .clone(),
            source_record_raw_token_emission_api_id: inventory_sub
                .source_record_raw_token_emission_api_id
                .clone(),
            source_record_raw_token_emission_api_boundary: inventory_sub
                .source_record_raw_token_emission_api_boundary
                .clone(),
            delegated_raw_scratch_generator_core_api_id: inventory_sub
                .delegated_raw_scratch_generator_core_api_id
                .clone(),
            delegated_raw_scratch_generator_core_api_boundary: inventory_sub
                .delegated_raw_scratch_generator_core_api_boundary
                .clone(),
            generator_core_api_id: inventory_sub.generator_core_api_id.clone(),
            generator_core_api_boundary: inventory_sub.generator_core_api_boundary.clone(),
            empty_prefix_generation_ready: record_readiness.empty_prefix_generation_ready,
            finite_admissible_mask_record_count: record_readiness
                .finite_admissible_mask_record_count,
            selected_transition_accepted_record_count: record_readiness
                .selected_transition_accepted_record_count,
            selected_completion_smoke_ready: record_readiness.selected_completion_smoke_ready,
            elided_selected_completion_ready: record_readiness.elided_selected_completion_ready,
            incomplete_prefix_rejection_ready: record_readiness.incomplete_prefix_rejection_ready,
            token_only_decode_render_ready,
            render_reparse_sidecar_validation_ready,
            raw_state_machine_runtime_boundary_only: inventory_sub
                .raw_state_machine_runtime_boundary_only,
            per_output_parser_metadata_required: inventory_sub.per_output_parser_metadata_required,
            source_instance_segment_sidecar_used: inventory_sub
                .source_instance_segment_sidecar_used,
            source_prefix_table_used: inventory_sub.source_prefix_table_used,
            arena_ir_used: inventory_sub.arena_ir_used,
            route_handoff_provenance_used: inventory_sub.route_handoff_provenance_used,
            bpe_validity_authority_used: inventory_sub.bpe_validity_authority_used,
            source_program_validity_claim_allowed: record_readiness
                .source_program_validity_claim_allowed,
            stable_backend_claimed: record_readiness.stable_backend_claimed,
            next_ready_task: SCRATCH_GENERATION_GATE_NEXT_READY_TASK.to_string(),
            claim_boundary: SCRATCH_GENERATION_GATE_CLAIM_BOUNDARY.to_string(),
        };
        gate.validate_with_readiness(oracle, grammar_coverage, representative, &record_readiness)?;
        Ok(gate)
    }

    /// Validate a stamped gate against oracle attachment, inventory coverage,
    /// readiness views, and representative paths.
    pub fn validate<'a>(
        &self,
        oracle: GoSourceRecordOracleCutoverAttachmentView<'_>,
        grammar_coverage: &GoAstSchemaGrammarCoverageReport,
        record_views: impl IntoIterator<Item = GoSourceRecordManifestDepthRecordReadinessView<'a>>,
        representative: GoSourceRecordManifestDepthRepresentativeView<'_>,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        let record_readiness =
            GoSourceRecordManifestDepthRecordReadiness::from_record_views(record_views)?;
        self.validate_with_readiness(oracle, grammar_coverage, representative, &record_readiness)
    }

    fn validate_with_readiness(
        &self,
        oracle: GoSourceRecordOracleCutoverAttachmentView<'_>,
        grammar_coverage: &GoAstSchemaGrammarCoverageReport,
        representative: GoSourceRecordManifestDepthRepresentativeView<'_>,
        record_readiness: &GoSourceRecordManifestDepthRecordReadiness,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        require_string(
            "manifest_depth.scratch_generation_gate.status",
            &self.status,
            SCRATCH_GENERATION_GATE_STATUS,
        )?;
        require_string(
            "manifest_depth.scratch_generation_gate.scope",
            &self.scope,
            SCRATCH_GENERATION_GATE_SCOPE,
        )?;
        require_string(
            "manifest_depth.scratch_generation_gate.precompiled_artifact_kind",
            &self.precompiled_artifact_kind,
            PRECOMPILED_GO_SYNTAX_TOKENIZER_ARTIFACT_KIND,
        )?;
        require_string(
            "manifest_depth.scratch_generation_gate.grammar_mode_render_authority_source",
            &self.grammar_mode_render_authority_source,
            SCRATCH_GENERATION_GATE_RENDER_AUTHORITY_SOURCE,
        )?;
        require_string(
            "manifest_depth.scratch_generation_gate.oracle_report_path",
            &self.oracle_report_path,
            oracle.report_path,
        )?;
        require_string(
            "manifest_depth.scratch_generation_gate.oracle_report_sha256",
            &self.oracle_report_sha256,
            oracle.report_sha256,
        )?;
        require_string(
            "manifest_depth.scratch_generation_gate.oracle_cutover_status",
            &self.oracle_cutover_status,
            SCRATCH_GENERATION_GATE_ORACLE_CUTOVER_STATUS_ACCEPTED,
        )?;
        require_string(
            "manifest_depth.scratch_generation_gate.oracle_cutover_status",
            &self.oracle_cutover_status,
            oracle.status,
        )?;
        require_bool(
            "manifest_depth.scratch_generation_gate.oracle_cutover_accepted",
            self.oracle_cutover_accepted,
            true,
        )?;
        require_bool(
            "manifest_depth.scratch_generation_gate.oracle_cutover_accepted",
            self.oracle_cutover_accepted,
            oracle.accepted,
        )?;
        require_usize(
            "manifest_depth.scratch_generation_gate.oracle_record_count",
            self.oracle_record_count,
            oracle.record_count,
        )?;
        require_usize(
            "manifest_depth.scratch_generation_gate.oracle_canonical_fidelity_only_gap_count",
            self.oracle_canonical_fidelity_only_gap_count,
            oracle.canonical_fidelity_only_gap_count,
        )?;
        require_bool(
            "manifest_depth.scratch_generation_gate.canonical_fidelity_gaps_diagnostic_only",
            self.canonical_fidelity_gaps_diagnostic_only,
            true,
        )?;
        // Inventory seed+empty-prefix sub-bundle (batch, API stamps, fixed claim
        // flags) is validated by the generator-core sub-gate type (rebuilt from
        // stamped gate fields).
        let inventory_sub = GoSourceRecordInventoryScratchSubGate::from_stamped_fields(
            self.empty_prefix_root.clone(),
            GoSourceRecordInventoryScratchSeedBatch {
                empty_prefix: self.go_ast_inventory_empty_prefix.clone(),
                go_ast_inventory_scratch_seed: self.go_ast_inventory_scratch_seed.clone(),
                go_ast_inventory_scratch_seeds: self.go_ast_inventory_scratch_seeds.clone(),
                scratch_seed_count: self.scratch_seed_count,
                scratch_seed_admitted_count: self.scratch_seed_admitted_count,
                scratch_seed_render_reparse_ready_count: self
                    .scratch_seed_render_reparse_ready_count,
                inventory_synthesis_backed_seed_count: self.inventory_synthesis_backed_seed_count,
                first_non_inventory_synthesis_seed_id: self
                    .first_non_inventory_synthesis_seed_id
                    .clone(),
                all_seeds_token_only_decode_render_ready: self
                    .go_ast_inventory_scratch_seeds
                    .iter()
                    .all(|seed| seed.decoded_from_token_only_stream && seed.decoded_tree_reusable),
                all_seeds_render_reparse_ready: self.scratch_seed_render_reparse_ready_count
                    == self.scratch_seed_count,
            },
            self.source_record_generator_core_api_id.clone(),
            self.source_record_generator_core_api_boundary.clone(),
            self.source_record_raw_token_emission_api_id.clone(),
            self.source_record_raw_token_emission_api_boundary.clone(),
            self.delegated_raw_scratch_generator_core_api_id.clone(),
            self.delegated_raw_scratch_generator_core_api_boundary
                .clone(),
            self.generator_core_api_id.clone(),
            self.generator_core_api_boundary.clone(),
            self.raw_state_machine_runtime_boundary_only,
            self.per_output_parser_metadata_required,
            self.source_instance_segment_sidecar_used,
            self.source_prefix_table_used,
            self.arena_ir_used,
            self.route_handoff_provenance_used,
            self.bpe_validity_authority_used,
        );
        inventory_sub.validate(grammar_coverage)?;
        require_string(
            "manifest_depth.scratch_generation_gate.representative_manifest_path",
            &self.representative_manifest_path,
            representative.manifest_path,
        )?;
        require_string(
            "manifest_depth.scratch_generation_gate.representative_source_path",
            &self.representative_source_path,
            representative.source_path,
        )?;
        require_string(
            "manifest_depth.scratch_generation_gate.representative_raw_token_ids_sha256",
            &self.representative_raw_token_ids_sha256,
            representative.raw_token_ids_sha256,
        )?;
        require_positive_usize(
            "manifest_depth.scratch_generation_gate.supported_record_count",
            self.supported_record_count,
        )?;
        record_readiness.validate_gate_fields(
            &inventory_sub.batch,
            self.empty_prefix_generation_ready,
            self.finite_admissible_mask_record_count,
            self.selected_transition_accepted_record_count,
            self.selected_completion_smoke_ready,
            self.elided_selected_completion_ready,
            self.incomplete_prefix_rejection_ready,
            self.token_only_decode_render_ready,
            self.render_reparse_sidecar_validation_ready,
            self.source_program_validity_claim_allowed,
            self.stable_backend_claimed,
            self.supported_record_count,
        )?;
        require_string(
            "manifest_depth.scratch_generation_gate.next_ready_task",
            &self.next_ready_task,
            SCRATCH_GENERATION_GATE_NEXT_READY_TASK,
        )?;
        require_str_contains(
            "manifest_depth.scratch_generation_gate.claim_boundary",
            &self.claim_boundary,
            "accepted grammar-mode Go AST printer oracle cutover",
        )?;
        require_str_contains(
            "manifest_depth.scratch_generation_gate.claim_boundary",
            &self.claim_boundary,
            "empty GoSourceFile raw prefix",
        )?;
        require_str_contains(
            "manifest_depth.scratch_generation_gate.claim_boundary",
            &self.claim_boundary,
            "raw syntax state-machine runtime evidence",
        )?;
        require_str_contains(
            "manifest_depth.scratch_generation_gate.claim_boundary",
            &self.claim_boundary,
            "not source-prefix table",
        )?;
        require_str_contains(
            "manifest_depth.scratch_generation_gate.claim_boundary",
            &self.claim_boundary,
            "not Arena IR",
        )?;
        require_str_contains(
            "manifest_depth.scratch_generation_gate.claim_boundary",
            &self.claim_boundary,
            "not BPE",
        )
    }
}

/// Manifest-depth **report-level** readiness flags derived from the assembled
/// scratch-generation gate.
///
/// Generator-core owns the gate → report mapping so top-level report claims
/// cannot diverge from gate readiness. Covers expanded selected-completion,
/// elided selected-completion, incomplete-prefix rejection, and token-only
/// decode/render. Report/pipeline `canonical_render_ready` and pipeline
/// `go_reparse_ready` are owned separately by
/// [`GoSourceRecordManifestDepthCanonicalRenderReadiness`] (record/sidecar
/// render authority, not gate readiness). Pipeline
/// `raw_token_encode_ready` / `raw_state_machine_decode_ready` are owned by
/// [`GoSourceRecordManifestDepthEncodeDecodeReadiness`] (token-emission + SM
/// completeness aggregates, not gate readiness).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoSourceRecordManifestDepthReportReadiness {
    pub selected_completion_smoke_ready: bool,
    pub elided_selected_completion_ready: bool,
    pub incomplete_prefix_rejection_ready: bool,
    pub token_only_decode_render_ready: bool,
}

impl GoSourceRecordManifestDepthReportReadiness {
    /// Derive report-level readiness from the assembled scratch-generation gate.
    pub fn from_scratch_generation_gate(
        gate: &GoTokenizerGeneratorManifestDepthScratchGenerationGate,
    ) -> Self {
        Self {
            selected_completion_smoke_ready: gate.selected_completion_smoke_ready,
            elided_selected_completion_ready: gate.elided_selected_completion_ready,
            incomplete_prefix_rejection_ready: gate.incomplete_prefix_rejection_ready,
            token_only_decode_render_ready: gate.token_only_decode_render_ready,
        }
    }

    /// Require report-stamped readiness fields match this gate-derived view.
    pub fn validate_report_fields(
        &self,
        selected_completion_smoke_ready: bool,
        elided_selected_completion_ready: bool,
        incomplete_prefix_rejection_ready: bool,
        token_only_decode_render_ready: bool,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        require_bool(
            "manifest_depth.report.selected_completion_smoke_ready",
            selected_completion_smoke_ready,
            self.selected_completion_smoke_ready,
        )?;
        require_bool(
            "manifest_depth.report.elided_selected_completion_ready",
            elided_selected_completion_ready,
            self.elided_selected_completion_ready,
        )?;
        require_bool(
            "manifest_depth.report.incomplete_prefix_rejection_ready",
            incomplete_prefix_rejection_ready,
            self.incomplete_prefix_rejection_ready,
        )?;
        require_bool(
            "manifest_depth.report.token_only_decode_render_ready",
            token_only_decode_render_ready,
            self.token_only_decode_render_ready,
        )?;
        Ok(())
    }

    /// Validate a report's readiness flags against the gate they claim to mirror.
    pub fn validate_against_gate(
        gate: &GoTokenizerGeneratorManifestDepthScratchGenerationGate,
        selected_completion_smoke_ready: bool,
        elided_selected_completion_ready: bool,
        incomplete_prefix_rejection_ready: bool,
        token_only_decode_render_ready: bool,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        Self::from_scratch_generation_gate(gate).validate_report_fields(
            selected_completion_smoke_ready,
            elided_selected_completion_ready,
            incomplete_prefix_rejection_ready,
            token_only_decode_render_ready,
        )
    }
}

/// Borrowed per-record vocabulary / raw-SM completeness fields for report-level
/// aggregation.
///
/// The megamodule maps each manifest-depth record into this view so
/// generator-core owns the aggregate predicates without depending on the full
/// record type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GoSourceRecordManifestDepthVocabSmCompletenessView {
    pub raw_tokens_below_vocab_size: bool,
    pub pad_token_absent: bool,
    pub raw_state_machine_complete: bool,
    pub max_raw_token_id: Option<u32>,
    pub token_layout_vocab_size: u32,
    pub token_layout_pad_token: u32,
}

/// Manifest-depth **report-level** vocabulary / raw-SM completeness flags
/// derived from record aggregates.
///
/// Generator-core owns the record → report mapping so top-level
/// `raw_tokens_below_vocab_size`, `pad_token_absent`, and
/// `raw_state_machine_complete` claims cannot hardcode independently of
/// actual record token accounting and vocab bounds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoSourceRecordManifestDepthVocabSmCompleteness {
    pub supported_record_count: usize,
    pub max_raw_token_id: Option<u32>,
    pub token_layout_vocab_size: u32,
    pub token_layout_pad_token: u32,
    pub raw_tokens_below_vocab_size: bool,
    pub pad_token_absent: bool,
    pub raw_state_machine_complete: bool,
}

impl GoSourceRecordManifestDepthVocabSmCompleteness {
    /// Aggregate vocabulary / raw-SM completeness over one or more record views.
    pub fn from_record_views(
        views: impl IntoIterator<Item = GoSourceRecordManifestDepthVocabSmCompletenessView>,
    ) -> Result<Self, SyntaxBackendBoundaryError> {
        let views: Vec<_> = views.into_iter().collect();
        if views.is_empty() {
            return Err(SyntaxBackendBoundaryError::new(
                "manifest_depth.vocab_sm_completeness.records",
                "vocabulary / raw-SM completeness requires at least one manifest-depth record",
            ));
        }

        let token_layout_vocab_size = views[0].token_layout_vocab_size;
        let token_layout_pad_token = views[0].token_layout_pad_token;
        for view in &views[1..] {
            if view.token_layout_vocab_size != token_layout_vocab_size {
                return Err(SyntaxBackendBoundaryError::new(
                    "manifest_depth.vocab_sm_completeness.token_layout_vocab_size",
                    "all manifest-depth records must use the same token layout VOCAB_SIZE",
                ));
            }
            if view.token_layout_pad_token != token_layout_pad_token {
                return Err(SyntaxBackendBoundaryError::new(
                    "manifest_depth.vocab_sm_completeness.token_layout_pad_token",
                    "all manifest-depth records must use the same token layout PAD_TOKEN",
                ));
            }
        }

        let mut max_raw_token_id = None;
        for view in &views {
            max_raw_token_id = max_optional_token(max_raw_token_id, view.max_raw_token_id);
        }

        let all_raw_tokens_below_vocab = views.iter().all(|view| view.raw_tokens_below_vocab_size);
        let aggregate_max_below_vocab = match max_raw_token_id {
            Some(token_id) => token_id < token_layout_vocab_size,
            None => true,
        };
        let raw_tokens_below_vocab_size = all_raw_tokens_below_vocab && aggregate_max_below_vocab;
        let pad_token_absent = views.iter().all(|view| view.pad_token_absent);
        let raw_state_machine_complete = views.iter().all(|view| view.raw_state_machine_complete);

        Ok(Self {
            supported_record_count: views.len(),
            max_raw_token_id,
            token_layout_vocab_size,
            token_layout_pad_token,
            raw_tokens_below_vocab_size,
            pad_token_absent,
            raw_state_machine_complete,
        })
    }

    /// Require report-stamped completeness fields match this record-derived view.
    pub fn validate_report_fields(
        &self,
        raw_tokens_below_vocab_size: bool,
        pad_token_absent: bool,
        raw_state_machine_complete: bool,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        require_bool(
            "manifest_depth.report.raw_tokens_below_vocab_size",
            raw_tokens_below_vocab_size,
            self.raw_tokens_below_vocab_size,
        )?;
        require_bool(
            "manifest_depth.report.pad_token_absent",
            pad_token_absent,
            self.pad_token_absent,
        )?;
        require_bool(
            "manifest_depth.report.raw_state_machine_complete",
            raw_state_machine_complete,
            self.raw_state_machine_complete,
        )?;
        Ok(())
    }

    /// Validate report completeness flags against the record slice they claim
    /// to summarize.
    pub fn validate_against_record_views(
        views: impl IntoIterator<Item = GoSourceRecordManifestDepthVocabSmCompletenessView>,
        raw_tokens_below_vocab_size: bool,
        pad_token_absent: bool,
        raw_state_machine_complete: bool,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        Self::from_record_views(views)?.validate_report_fields(
            raw_tokens_below_vocab_size,
            pad_token_absent,
            raw_state_machine_complete,
        )
    }
}

/// Single-record canonical-render / go-reparse readiness pure conjunctions
/// derived from checked projection sidecar fields.
///
/// Generator-core owns this pure conjunction so the single-record generator
/// report, selected-completion smoke, and token-only decode/render evidence
/// stamp the same readiness claims and cannot drift on whether
/// `canonical_sha256 == source.sha256` is part of the formula. Matches the
/// emission sidecar readiness used by
/// [`crate::syntax_go_source_record_raw_token::GoSourceRecordRenderReparseSidecarEvidence`].
///
/// - `canonical_render_ready`: gofmt passed ∧ go_parser passed ∧
///   matches_source ∧ canonical_sha256 == source.sha256
/// - `go_reparse_ready`: go_parser passed
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoSourceRecordCanonicalRenderReadinessFlags {
    pub gofmt_passed: bool,
    pub go_parser_reparse_passed: bool,
    pub matches_source: bool,
    pub canonical_sha256_matches_source: bool,
    pub canonical_render_ready: bool,
    pub go_reparse_ready: bool,
}

impl GoSourceRecordCanonicalRenderReadinessFlags {
    /// Derive single-record render/reparse readiness from canonical-render
    /// sidecar fields plus the checked source sha256.
    pub fn from_canonical_render(
        canonical: &SourceInstanceCanonicalRender,
        source_sha256: &str,
    ) -> Self {
        let gofmt_passed = canonical.gofmt == "passed";
        let go_parser_reparse_passed = canonical.go_parser == "passed";
        let matches_source = canonical.matches_source;
        let canonical_sha256_matches_source = canonical.canonical_sha256 == source_sha256;
        let go_reparse_ready = go_parser_reparse_passed;
        let canonical_render_ready = gofmt_passed
            && go_parser_reparse_passed
            && matches_source
            && canonical_sha256_matches_source;
        Self {
            gofmt_passed,
            go_parser_reparse_passed,
            matches_source,
            canonical_sha256_matches_source,
            canonical_render_ready,
            go_reparse_ready,
        }
    }

    /// Derive single-record render/reparse readiness from a checked projection
    /// report (canonical_render + source.sha256).
    pub fn from_checked_report(report: &SyntaxGoSourceInstanceProjectionReport) -> Self {
        Self::from_canonical_render(&report.canonical_render, &report.source.sha256)
    }

    /// Map into the manifest-depth per-record view used by aggregate readiness.
    pub fn readiness_view(self) -> GoSourceRecordManifestDepthCanonicalRenderReadinessView {
        GoSourceRecordManifestDepthCanonicalRenderReadinessView {
            canonical_render_ready: self.canonical_render_ready,
            gofmt_passed: self.gofmt_passed,
            go_parser_reparse_passed: self.go_parser_reparse_passed,
        }
    }
}

/// Borrowed per-record canonical-render / go-reparse readiness fields for
/// report-level aggregation.
///
/// The megamodule maps each manifest-depth record into this view so
/// generator-core owns the aggregate predicates without depending on the full
/// record type. Sidecar strings are reduced to booleans at the view boundary
/// (`gofmt` / `go_parser` == `"passed"`). Prefer
/// [`GoSourceRecordCanonicalRenderReadinessFlags::readiness_view`] when the
/// source is a checked projection report.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GoSourceRecordManifestDepthCanonicalRenderReadinessView {
    pub canonical_render_ready: bool,
    pub gofmt_passed: bool,
    pub go_parser_reparse_passed: bool,
}

/// Manifest-depth **report-level** canonical-render / go-reparse readiness
/// derived from record aggregates and sidecar render authority.
///
/// Generator-core owns the record → report/pipeline mapping so top-level
/// `canonical_render_ready` and pipeline `canonical_render_ready` /
/// `go_reparse_ready` claims cannot hardcode independently of per-record
/// gofmt + go-parser reparse evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoSourceRecordManifestDepthCanonicalRenderReadiness {
    pub supported_record_count: usize,
    pub canonical_render_ready: bool,
    pub go_reparse_ready: bool,
}

impl GoSourceRecordManifestDepthCanonicalRenderReadiness {
    /// Aggregate canonical-render / go-reparse readiness over one or more
    /// record views.
    pub fn from_record_views(
        views: impl IntoIterator<Item = GoSourceRecordManifestDepthCanonicalRenderReadinessView>,
    ) -> Result<Self, SyntaxBackendBoundaryError> {
        let views: Vec<_> = views.into_iter().collect();
        if views.is_empty() {
            return Err(SyntaxBackendBoundaryError::new(
                "manifest_depth.canonical_render_readiness.records",
                "canonical-render readiness requires at least one manifest-depth record",
            ));
        }

        let gofmt_all_passed = views.iter().all(|view| view.gofmt_passed);
        let go_reparse_ready = views.iter().all(|view| view.go_parser_reparse_passed);
        // Require both the explicit record flag and the sidecar pass bits so a
        // true claim cannot float free of gofmt/go-parser authority.
        let canonical_render_ready = views.iter().all(|view| view.canonical_render_ready)
            && gofmt_all_passed
            && go_reparse_ready;

        Ok(Self {
            supported_record_count: views.len(),
            canonical_render_ready,
            go_reparse_ready,
        })
    }

    /// Require report-stamped render readiness fields match this record-derived
    /// view (report top-level + pipeline render/reparse flags).
    pub fn validate_report_fields(
        &self,
        canonical_render_ready: bool,
        pipeline_canonical_render_ready: bool,
        pipeline_go_reparse_ready: bool,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        require_bool(
            "manifest_depth.report.canonical_render_ready",
            canonical_render_ready,
            self.canonical_render_ready,
        )?;
        require_bool(
            "manifest_depth.report.pipeline.canonical_render_ready",
            pipeline_canonical_render_ready,
            self.canonical_render_ready,
        )?;
        require_bool(
            "manifest_depth.report.pipeline.go_reparse_ready",
            pipeline_go_reparse_ready,
            self.go_reparse_ready,
        )?;
        Ok(())
    }

    /// Validate report render readiness against the record slice they claim to
    /// summarize.
    pub fn validate_against_record_views(
        views: impl IntoIterator<Item = GoSourceRecordManifestDepthCanonicalRenderReadinessView>,
        canonical_render_ready: bool,
        pipeline_canonical_render_ready: bool,
        pipeline_go_reparse_ready: bool,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        Self::from_record_views(views)?.validate_report_fields(
            canonical_render_ready,
            pipeline_canonical_render_ready,
            pipeline_go_reparse_ready,
        )
    }
}

/// Process-only pipeline **compile-stage** claims for the Go source-record
/// raw-token path.
///
/// These five flags document that a successful generator run completed the
/// parser-backed metadata → GrammarIR → StructuralModel → token layout → raw
/// syntax state-machine compile stages. They are **process claims**, not pure
/// conjunctions of record/gate aggregates: no independent evidence surface
/// currently names them as record-derived predicates, and inventing fake
/// evidence would be wrong. Generator-core owns the constant bundle so the
/// megamodule stamps the same literals rather than inventing alternate process
/// claims at each construction site.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoSourceRecordPipelineProcessStageClaims {
    pub parser_backed_metadata_loaded: bool,
    pub grammar_ir_lowered: bool,
    pub structural_model_lowered: bool,
    pub token_layout_compiled: bool,
    pub raw_state_machine_compiled: bool,
}

impl GoSourceRecordPipelineProcessStageClaims {
    /// Canonical process-stage claim bundle for a successful inventory-path
    /// source-record generator run.
    pub const SUCCESS: Self = Self {
        parser_backed_metadata_loaded: true,
        grammar_ir_lowered: true,
        structural_model_lowered: true,
        token_layout_compiled: true,
        raw_state_machine_compiled: true,
    };

    /// Return the canonical process-stage claim bundle.
    pub fn process_success() -> Self {
        Self::SUCCESS
    }

    /// Require pipeline-stamped process-stage flags match this bundle.
    pub fn validate_pipeline_fields(
        &self,
        parser_backed_metadata_loaded: bool,
        grammar_ir_lowered: bool,
        structural_model_lowered: bool,
        token_layout_compiled: bool,
        raw_state_machine_compiled: bool,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        require_bool(
            "pipeline.parser_backed_metadata_loaded",
            parser_backed_metadata_loaded,
            self.parser_backed_metadata_loaded,
        )?;
        require_bool(
            "pipeline.grammar_ir_lowered",
            grammar_ir_lowered,
            self.grammar_ir_lowered,
        )?;
        require_bool(
            "pipeline.structural_model_lowered",
            structural_model_lowered,
            self.structural_model_lowered,
        )?;
        require_bool(
            "pipeline.token_layout_compiled",
            token_layout_compiled,
            self.token_layout_compiled,
        )?;
        require_bool(
            "pipeline.raw_state_machine_compiled",
            raw_state_machine_compiled,
            self.raw_state_machine_compiled,
        )?;
        Ok(())
    }

    /// Validate pipeline process-stage flags against the canonical success
    /// bundle (rejects divergence from the generator-core constant).
    pub fn validate_against_success_bundle(
        parser_backed_metadata_loaded: bool,
        grammar_ir_lowered: bool,
        structural_model_lowered: bool,
        token_layout_compiled: bool,
        raw_state_machine_compiled: bool,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        Self::SUCCESS.validate_pipeline_fields(
            parser_backed_metadata_loaded,
            grammar_ir_lowered,
            structural_model_lowered,
            token_layout_compiled,
            raw_state_machine_compiled,
        )
    }
}

/// Process-only pipeline **exclusion** claims for the Go source-record
/// raw-token path.
///
/// These three flags document that ArenaIR, route-handoff provenance ladders,
/// and BPE validity authority are **not** used as authority on this path.
/// They are process exclusions (must stay false), not record-derived
/// aggregates. Generator-core owns the constant bundle so the megamodule
/// stamps the same literals at each construction site.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoSourceRecordPipelineProcessExclusionClaims {
    pub arena_ir_used: bool,
    pub route_handoff_provenance_used: bool,
    pub bpe_validity_authority_used: bool,
}

impl GoSourceRecordPipelineProcessExclusionClaims {
    /// Canonical process-exclusion claim bundle for a successful inventory-path
    /// source-record generator run (ArenaIR / route-handoff / BPE off).
    pub const SUCCESS: Self = Self {
        arena_ir_used: false,
        route_handoff_provenance_used: false,
        bpe_validity_authority_used: false,
    };

    /// Return the canonical process-exclusion claim bundle.
    pub fn process_success() -> Self {
        Self::SUCCESS
    }

    /// Require pipeline-stamped exclusion flags match this bundle.
    pub fn validate_pipeline_fields(
        &self,
        arena_ir_used: bool,
        route_handoff_provenance_used: bool,
        bpe_validity_authority_used: bool,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        require_bool("pipeline.arena_ir_used", arena_ir_used, self.arena_ir_used)?;
        require_bool(
            "pipeline.route_handoff_provenance_used",
            route_handoff_provenance_used,
            self.route_handoff_provenance_used,
        )?;
        require_bool(
            "pipeline.bpe_validity_authority_used",
            bpe_validity_authority_used,
            self.bpe_validity_authority_used,
        )?;
        Ok(())
    }

    /// Validate pipeline exclusion flags against the canonical success bundle.
    pub fn validate_against_success_bundle(
        arena_ir_used: bool,
        route_handoff_provenance_used: bool,
        bpe_validity_authority_used: bool,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        Self::SUCCESS.validate_pipeline_fields(
            arena_ir_used,
            route_handoff_provenance_used,
            bpe_validity_authority_used,
        )
    }
}

/// Manifest-depth **pipeline** encode/decode stage readiness derived from
/// vocabulary / raw-SM completeness aggregates (token emission + SM complete).
///
/// Generator-core owns the completeness → pipeline mapping so
/// `pipeline.raw_token_encode_ready` and `pipeline.raw_state_machine_decode_ready`
/// cannot hardcode independently of record token-emission integrity and
/// raw-SM completeness. Process-only pipeline stages are owned by
/// [`GoSourceRecordPipelineProcessStageClaims`]; process exclusions by
/// [`GoSourceRecordPipelineProcessExclusionClaims`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoSourceRecordManifestDepthEncodeDecodeReadiness {
    pub supported_record_count: usize,
    pub raw_token_encode_ready: bool,
    pub raw_state_machine_decode_ready: bool,
}

impl GoSourceRecordManifestDepthEncodeDecodeReadiness {
    /// Derive pipeline encode/decode readiness from the vocab/SM completeness
    /// aggregate (pure conjunctions of token-emission + SM-complete flags).
    ///
    /// - `raw_token_encode_ready`: every record's emitted raw tokens stay below
    ///   VOCAB_SIZE and omit PAD (token-emission integrity).
    /// - `raw_state_machine_decode_ready`: every record's raw SM decode is
    ///   complete.
    pub fn from_vocab_sm_completeness(
        completeness: &GoSourceRecordManifestDepthVocabSmCompleteness,
    ) -> Self {
        Self {
            supported_record_count: completeness.supported_record_count,
            raw_token_encode_ready: completeness.raw_tokens_below_vocab_size
                && completeness.pad_token_absent,
            raw_state_machine_decode_ready: completeness.raw_state_machine_complete,
        }
    }

    /// Aggregate encode/decode readiness over record completeness views.
    pub fn from_record_views(
        views: impl IntoIterator<Item = GoSourceRecordManifestDepthVocabSmCompletenessView>,
    ) -> Result<Self, SyntaxBackendBoundaryError> {
        Ok(Self::from_vocab_sm_completeness(
            &GoSourceRecordManifestDepthVocabSmCompleteness::from_record_views(views)?,
        ))
    }

    /// Derive encode/decode readiness from a single record's token-emission and
    /// SM-complete flags (single-record generator report path).
    pub fn from_token_emission_and_sm_complete(
        raw_tokens_below_vocab_size: bool,
        pad_token_absent: bool,
        raw_state_machine_complete: bool,
    ) -> Self {
        Self {
            supported_record_count: 1,
            raw_token_encode_ready: raw_tokens_below_vocab_size && pad_token_absent,
            raw_state_machine_decode_ready: raw_state_machine_complete,
        }
    }

    /// Require pipeline-stamped encode/decode flags match this aggregate view.
    pub fn validate_pipeline_fields(
        &self,
        pipeline_raw_token_encode_ready: bool,
        pipeline_raw_state_machine_decode_ready: bool,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        require_bool(
            "manifest_depth.report.pipeline.raw_token_encode_ready",
            pipeline_raw_token_encode_ready,
            self.raw_token_encode_ready,
        )?;
        require_bool(
            "manifest_depth.report.pipeline.raw_state_machine_decode_ready",
            pipeline_raw_state_machine_decode_ready,
            self.raw_state_machine_decode_ready,
        )?;
        Ok(())
    }

    /// Validate pipeline encode/decode flags against the completeness aggregate
    /// they claim to mirror.
    pub fn validate_against_vocab_sm_completeness(
        completeness: &GoSourceRecordManifestDepthVocabSmCompleteness,
        pipeline_raw_token_encode_ready: bool,
        pipeline_raw_state_machine_decode_ready: bool,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        Self::from_vocab_sm_completeness(completeness).validate_pipeline_fields(
            pipeline_raw_token_encode_ready,
            pipeline_raw_state_machine_decode_ready,
        )
    }

    /// Validate pipeline encode/decode flags against the record completeness
    /// views they claim to summarize.
    pub fn validate_against_record_views(
        views: impl IntoIterator<Item = GoSourceRecordManifestDepthVocabSmCompletenessView>,
        pipeline_raw_token_encode_ready: bool,
        pipeline_raw_state_machine_decode_ready: bool,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        Self::from_record_views(views)?.validate_pipeline_fields(
            pipeline_raw_token_encode_ready,
            pipeline_raw_state_machine_decode_ready,
        )
    }
}

/// Full pipeline field stamp for the Go source-record raw-token path.
///
/// Folds the four generator-core pipeline surfaces into one view so the
/// megamodule maps a single stamp at each construction site rather than
/// re-composing process stages, process exclusions, encode/decode readiness,
/// and canonical-render readiness independently:
///
/// - process stages: [`GoSourceRecordPipelineProcessStageClaims::SUCCESS`]
/// - process exclusions: [`GoSourceRecordPipelineProcessExclusionClaims::SUCCESS`]
/// - encode/decode: [`GoSourceRecordManifestDepthEncodeDecodeReadiness`]
/// - canonical-render / go-reparse: record-aggregate or single-record flags
///
/// Process stages/exclusions remain constant process claims (not invented
/// record evidence). Encode/decode and render flags stay pure conjunctions of
/// their existing aggregates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoSourceRecordPipelineFieldStamp {
    pub parser_backed_metadata_loaded: bool,
    pub grammar_ir_lowered: bool,
    pub structural_model_lowered: bool,
    pub token_layout_compiled: bool,
    pub raw_state_machine_compiled: bool,
    pub raw_token_encode_ready: bool,
    pub raw_state_machine_decode_ready: bool,
    pub canonical_render_ready: bool,
    pub go_reparse_ready: bool,
    pub arena_ir_used: bool,
    pub route_handoff_provenance_used: bool,
    pub bpe_validity_authority_used: bool,
}

impl GoSourceRecordPipelineFieldStamp {
    /// Assemble the full pipeline stamp from encode/decode readiness plus
    /// explicit canonical-render / go-reparse flags (single-record path, or any
    /// caller that already reduced render readiness to booleans).
    ///
    /// Process stage and exclusion bundles come from the generator-core
    /// success constants — never ad-hoc literals at the call site.
    pub fn from_encode_decode_and_render_flags(
        encode_decode: &GoSourceRecordManifestDepthEncodeDecodeReadiness,
        canonical_render_ready: bool,
        go_reparse_ready: bool,
    ) -> Self {
        let process_stages = GoSourceRecordPipelineProcessStageClaims::SUCCESS;
        let process_exclusions = GoSourceRecordPipelineProcessExclusionClaims::SUCCESS;
        Self {
            parser_backed_metadata_loaded: process_stages.parser_backed_metadata_loaded,
            grammar_ir_lowered: process_stages.grammar_ir_lowered,
            structural_model_lowered: process_stages.structural_model_lowered,
            token_layout_compiled: process_stages.token_layout_compiled,
            raw_state_machine_compiled: process_stages.raw_state_machine_compiled,
            raw_token_encode_ready: encode_decode.raw_token_encode_ready,
            raw_state_machine_decode_ready: encode_decode.raw_state_machine_decode_ready,
            canonical_render_ready,
            go_reparse_ready,
            arena_ir_used: process_exclusions.arena_ir_used,
            route_handoff_provenance_used: process_exclusions.route_handoff_provenance_used,
            bpe_validity_authority_used: process_exclusions.bpe_validity_authority_used,
        }
    }

    /// Assemble the full pipeline stamp from encode/decode readiness and the
    /// record-aggregate canonical-render readiness view (manifest-depth path).
    pub fn from_encode_decode_and_canonical_render(
        encode_decode: &GoSourceRecordManifestDepthEncodeDecodeReadiness,
        canonical_render: &GoSourceRecordManifestDepthCanonicalRenderReadiness,
    ) -> Self {
        Self::from_encode_decode_and_render_flags(
            encode_decode,
            canonical_render.canonical_render_ready,
            canonical_render.go_reparse_ready,
        )
    }

    /// Require pipeline-stamped fields match this assembled stamp.
    #[allow(clippy::too_many_arguments)]
    pub fn validate_pipeline_fields(
        &self,
        parser_backed_metadata_loaded: bool,
        grammar_ir_lowered: bool,
        structural_model_lowered: bool,
        token_layout_compiled: bool,
        raw_state_machine_compiled: bool,
        raw_token_encode_ready: bool,
        raw_state_machine_decode_ready: bool,
        canonical_render_ready: bool,
        go_reparse_ready: bool,
        arena_ir_used: bool,
        route_handoff_provenance_used: bool,
        bpe_validity_authority_used: bool,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        require_bool(
            "pipeline.parser_backed_metadata_loaded",
            parser_backed_metadata_loaded,
            self.parser_backed_metadata_loaded,
        )?;
        require_bool(
            "pipeline.grammar_ir_lowered",
            grammar_ir_lowered,
            self.grammar_ir_lowered,
        )?;
        require_bool(
            "pipeline.structural_model_lowered",
            structural_model_lowered,
            self.structural_model_lowered,
        )?;
        require_bool(
            "pipeline.token_layout_compiled",
            token_layout_compiled,
            self.token_layout_compiled,
        )?;
        require_bool(
            "pipeline.raw_state_machine_compiled",
            raw_state_machine_compiled,
            self.raw_state_machine_compiled,
        )?;
        require_bool(
            "pipeline.raw_token_encode_ready",
            raw_token_encode_ready,
            self.raw_token_encode_ready,
        )?;
        require_bool(
            "pipeline.raw_state_machine_decode_ready",
            raw_state_machine_decode_ready,
            self.raw_state_machine_decode_ready,
        )?;
        require_bool(
            "pipeline.canonical_render_ready",
            canonical_render_ready,
            self.canonical_render_ready,
        )?;
        require_bool(
            "pipeline.go_reparse_ready",
            go_reparse_ready,
            self.go_reparse_ready,
        )?;
        require_bool("pipeline.arena_ir_used", arena_ir_used, self.arena_ir_used)?;
        require_bool(
            "pipeline.route_handoff_provenance_used",
            route_handoff_provenance_used,
            self.route_handoff_provenance_used,
        )?;
        require_bool(
            "pipeline.bpe_validity_authority_used",
            bpe_validity_authority_used,
            self.bpe_validity_authority_used,
        )?;
        Ok(())
    }
}

/// Full **report-level claim field stamp** for the manifest-depth
/// tokenizer-generator report.
///
/// Folds the four generator-core report-claim surfaces plus the full pipeline
/// stamp into one view so the megamodule maps a single stamp onto report claim
/// fields rather than re-composing:
///
/// - vocab/SM completeness: [`GoSourceRecordManifestDepthVocabSmCompleteness`]
/// - gate-derived readiness: [`GoSourceRecordManifestDepthReportReadiness`]
/// - canonical-render readiness: [`GoSourceRecordManifestDepthCanonicalRenderReadiness`]
/// - full pipeline: [`GoSourceRecordPipelineFieldStamp`]
///   (process stages/exclusions + encode/decode + render)
///
/// Process stages/exclusions remain constant process claims inside the pipeline
/// stamp. Encode/decode, vocab/SM, gate readiness, and render flags stay pure
/// conjunctions of their existing gate/record aggregates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoSourceRecordManifestDepthReportClaimStamp {
    pub raw_tokens_below_vocab_size: bool,
    pub pad_token_absent: bool,
    pub raw_state_machine_complete: bool,
    pub selected_completion_smoke_ready: bool,
    pub elided_selected_completion_ready: bool,
    pub incomplete_prefix_rejection_ready: bool,
    pub token_only_decode_render_ready: bool,
    pub canonical_render_ready: bool,
    pub pipeline: GoSourceRecordPipelineFieldStamp,
}

impl GoSourceRecordManifestDepthReportClaimStamp {
    /// Assemble the full report-claim stamp from the scratch-generation gate
    /// plus per-record vocab/SM and canonical-render views.
    pub fn from_gate_and_record_views(
        gate: &GoTokenizerGeneratorManifestDepthScratchGenerationGate,
        vocab_views: impl IntoIterator<Item = GoSourceRecordManifestDepthVocabSmCompletenessView>,
        render_views: impl IntoIterator<Item = GoSourceRecordManifestDepthCanonicalRenderReadinessView>,
    ) -> Result<Self, SyntaxBackendBoundaryError> {
        let vocab_sm =
            GoSourceRecordManifestDepthVocabSmCompleteness::from_record_views(vocab_views)?;
        let encode_decode =
            GoSourceRecordManifestDepthEncodeDecodeReadiness::from_vocab_sm_completeness(&vocab_sm);
        let report_readiness =
            GoSourceRecordManifestDepthReportReadiness::from_scratch_generation_gate(gate);
        let canonical_render =
            GoSourceRecordManifestDepthCanonicalRenderReadiness::from_record_views(render_views)?;
        let pipeline = GoSourceRecordPipelineFieldStamp::from_encode_decode_and_canonical_render(
            &encode_decode,
            &canonical_render,
        );
        Ok(Self {
            raw_tokens_below_vocab_size: vocab_sm.raw_tokens_below_vocab_size,
            pad_token_absent: vocab_sm.pad_token_absent,
            raw_state_machine_complete: vocab_sm.raw_state_machine_complete,
            selected_completion_smoke_ready: report_readiness.selected_completion_smoke_ready,
            elided_selected_completion_ready: report_readiness.elided_selected_completion_ready,
            incomplete_prefix_rejection_ready: report_readiness.incomplete_prefix_rejection_ready,
            token_only_decode_render_ready: report_readiness.token_only_decode_render_ready,
            canonical_render_ready: canonical_render.canonical_render_ready,
            pipeline,
        })
    }

    /// Require report-stamped claim fields match this assembled stamp
    /// (top-level readiness booleans + full pipeline surface).
    #[allow(clippy::too_many_arguments)]
    pub fn validate_report_claim_fields(
        &self,
        raw_tokens_below_vocab_size: bool,
        pad_token_absent: bool,
        raw_state_machine_complete: bool,
        selected_completion_smoke_ready: bool,
        elided_selected_completion_ready: bool,
        incomplete_prefix_rejection_ready: bool,
        token_only_decode_render_ready: bool,
        canonical_render_ready: bool,
        pipeline_parser_backed_metadata_loaded: bool,
        pipeline_grammar_ir_lowered: bool,
        pipeline_structural_model_lowered: bool,
        pipeline_token_layout_compiled: bool,
        pipeline_raw_state_machine_compiled: bool,
        pipeline_raw_token_encode_ready: bool,
        pipeline_raw_state_machine_decode_ready: bool,
        pipeline_canonical_render_ready: bool,
        pipeline_go_reparse_ready: bool,
        pipeline_arena_ir_used: bool,
        pipeline_route_handoff_provenance_used: bool,
        pipeline_bpe_validity_authority_used: bool,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        require_bool(
            "manifest_depth.report.raw_tokens_below_vocab_size",
            raw_tokens_below_vocab_size,
            self.raw_tokens_below_vocab_size,
        )?;
        require_bool(
            "manifest_depth.report.pad_token_absent",
            pad_token_absent,
            self.pad_token_absent,
        )?;
        require_bool(
            "manifest_depth.report.raw_state_machine_complete",
            raw_state_machine_complete,
            self.raw_state_machine_complete,
        )?;
        require_bool(
            "manifest_depth.report.selected_completion_smoke_ready",
            selected_completion_smoke_ready,
            self.selected_completion_smoke_ready,
        )?;
        require_bool(
            "manifest_depth.report.elided_selected_completion_ready",
            elided_selected_completion_ready,
            self.elided_selected_completion_ready,
        )?;
        require_bool(
            "manifest_depth.report.incomplete_prefix_rejection_ready",
            incomplete_prefix_rejection_ready,
            self.incomplete_prefix_rejection_ready,
        )?;
        require_bool(
            "manifest_depth.report.token_only_decode_render_ready",
            token_only_decode_render_ready,
            self.token_only_decode_render_ready,
        )?;
        require_bool(
            "manifest_depth.report.canonical_render_ready",
            canonical_render_ready,
            self.canonical_render_ready,
        )?;
        self.pipeline.validate_pipeline_fields(
            pipeline_parser_backed_metadata_loaded,
            pipeline_grammar_ir_lowered,
            pipeline_structural_model_lowered,
            pipeline_token_layout_compiled,
            pipeline_raw_state_machine_compiled,
            pipeline_raw_token_encode_ready,
            pipeline_raw_state_machine_decode_ready,
            pipeline_canonical_render_ready,
            pipeline_go_reparse_ready,
            pipeline_arena_ir_used,
            pipeline_route_handoff_provenance_used,
            pipeline_bpe_validity_authority_used,
        )
    }

    /// Validate a report's claim fields against gate + record views (rebuilds
    /// the expected stamp so dual-site hardcoding cannot drift from aggregates).
    #[allow(clippy::too_many_arguments)]
    pub fn validate_against_gate_and_record_views(
        gate: &GoTokenizerGeneratorManifestDepthScratchGenerationGate,
        vocab_views: impl IntoIterator<Item = GoSourceRecordManifestDepthVocabSmCompletenessView>,
        render_views: impl IntoIterator<Item = GoSourceRecordManifestDepthCanonicalRenderReadinessView>,
        raw_tokens_below_vocab_size: bool,
        pad_token_absent: bool,
        raw_state_machine_complete: bool,
        selected_completion_smoke_ready: bool,
        elided_selected_completion_ready: bool,
        incomplete_prefix_rejection_ready: bool,
        token_only_decode_render_ready: bool,
        canonical_render_ready: bool,
        pipeline_parser_backed_metadata_loaded: bool,
        pipeline_grammar_ir_lowered: bool,
        pipeline_structural_model_lowered: bool,
        pipeline_token_layout_compiled: bool,
        pipeline_raw_state_machine_compiled: bool,
        pipeline_raw_token_encode_ready: bool,
        pipeline_raw_state_machine_decode_ready: bool,
        pipeline_canonical_render_ready: bool,
        pipeline_go_reparse_ready: bool,
        pipeline_arena_ir_used: bool,
        pipeline_route_handoff_provenance_used: bool,
        pipeline_bpe_validity_authority_used: bool,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        Self::from_gate_and_record_views(gate, vocab_views, render_views)?
            .validate_report_claim_fields(
                raw_tokens_below_vocab_size,
                pad_token_absent,
                raw_state_machine_complete,
                selected_completion_smoke_ready,
                elided_selected_completion_ready,
                incomplete_prefix_rejection_ready,
                token_only_decode_render_ready,
                canonical_render_ready,
                pipeline_parser_backed_metadata_loaded,
                pipeline_grammar_ir_lowered,
                pipeline_structural_model_lowered,
                pipeline_token_layout_compiled,
                pipeline_raw_state_machine_compiled,
                pipeline_raw_token_encode_ready,
                pipeline_raw_state_machine_decode_ready,
                pipeline_canonical_render_ready,
                pipeline_go_reparse_ready,
                pipeline_arena_ir_used,
                pipeline_route_handoff_provenance_used,
                pipeline_bpe_validity_authority_used,
            )
    }
}

fn max_optional_token(left: Option<u32>, right: Option<u32>) -> Option<u32> {
    match (left, right) {
        (Some(left), Some(right)) => Some(left.max(right)),
        (Some(left), None) => Some(left),
        (None, Some(right)) => Some(right),
        (None, None) => None,
    }
}

fn decoded_tree_frontier_coverage(root: &RawSyntaxDecodedNode) -> (Vec<String>, Vec<String>) {
    let mut node_kinds = BTreeSet::new();
    let mut field_families = BTreeSet::new();
    collect_decoded_tree_frontier_coverage(root, &mut node_kinds, &mut field_families);
    (
        node_kinds.into_iter().collect(),
        field_families.into_iter().collect(),
    )
}

fn collect_decoded_tree_frontier_coverage(
    node: &RawSyntaxDecodedNode,
    node_kinds: &mut BTreeSet<String>,
    field_families: &mut BTreeSet<String>,
) {
    node_kinds.insert(node.node_kind.clone());
    for field in &node.fields {
        field_families.insert(format!("{}.{}", node.node_kind, field.name));
        for value in &field.values {
            if let RawSyntaxDecodedValue::Node(child) = value {
                collect_decoded_tree_frontier_coverage(child, node_kinds, field_families);
            }
        }
    }
}

fn scratch_seed_step_samples(
    seed: &GoScratchSeedStream,
    sample_steps: &BTreeSet<usize>,
) -> Vec<GoTokenizerGeneratorManifestDepthScratchSeedStepSample> {
    seed.admissibility
        .steps
        .iter()
        .filter(|step| sample_steps.contains(&step.step_index))
        .map(
            |step| GoTokenizerGeneratorManifestDepthScratchSeedStepSample {
                step_index: step.step_index,
                prefix_raw_token_count: step.prefix_raw_token_count,
                selected_raw_token_id: step.selected_raw_token_id,
                finite_admissible_raw_token_count: step.finite_admissible_raw_token_count,
                admissible_raw_token_range_count: step.admissible_raw_token_range_count,
                payload_end_admissible: step.payload_end_admissible,
                selected_raw_token_admitted: step.selected_raw_token_admitted,
                selected_transition_accepted: step.selected_transition_accepted,
                transition_observed_status: step.transition_state.status.as_str().to_string(),
                transition_consumed_raw_tokens: step.transition_state.consumed_raw_tokens,
                transition_eof_valid: step.transition_state.eof_valid,
                transition_stack_depth: step.transition_state.stack_depth,
                transition_active_rule: step.transition_state.active_rule.clone(),
                transition_active_field: step.transition_state.active_field.clone(),
            },
        )
        .collect()
}

fn gofmt_raw_tree_rendered_source(source: &str) -> Result<String, SyntaxBackendBoundaryError> {
    use std::io::Write;
    use std::process::{Command as ProcessCommand, Stdio};

    let gofmt_bin = std::env::var_os("TCT_GOFMT_BIN")
        .or_else(|| crate::go_toolchain::gofmt_bin_os_string().ok())
        .unwrap_or_else(|| std::ffi::OsString::from("gofmt"));
    let mut child = ProcessCommand::new(&gofmt_bin)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| {
            SyntaxBackendBoundaryError::new(
                "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.gofmt_validation",
                format!("failed to start gofmt for inventory scratch seed render: {error}"),
            )
        })?;
    child
        .stdin
        .as_mut()
        .ok_or_else(|| {
            SyntaxBackendBoundaryError::new(
                "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.gofmt_validation",
                "failed to open gofmt stdin for inventory scratch seed render",
            )
        })?
        .write_all(source.as_bytes())
        .map_err(|error| {
            SyntaxBackendBoundaryError::new(
                "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.gofmt_validation",
                format!("failed to write inventory scratch seed render to gofmt: {error}"),
            )
        })?;
    let output = child.wait_with_output().map_err(|error| {
        SyntaxBackendBoundaryError::new(
            "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.gofmt_validation",
            format!("failed to read gofmt output for inventory scratch seed render: {error}"),
        )
    })?;
    if !output.status.success() {
        return Err(SyntaxBackendBoundaryError::new(
            "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.gofmt_validation",
            format!(
                "gofmt rejected inventory scratch seed render: {}",
                String::from_utf8_lossy(&output.stderr)
            ),
        ));
    }
    String::from_utf8(output.stdout).map_err(|error| {
        SyntaxBackendBoundaryError::new(
            "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.gofmt_validation",
            format!("gofmt output was not valid UTF-8: {error}"),
        )
    })
}

fn validate_sorted_unique_non_empty(
    field: &'static str,
    actual: &[String],
) -> Result<(), SyntaxBackendBoundaryError> {
    require_positive_usize(field, actual.len())?;
    let mut previous: Option<&str> = None;
    for value in actual {
        require_non_empty(field, value)?;
        if let Some(previous) = previous
            && previous >= value.as_str()
        {
            return Err(SyntaxBackendBoundaryError::new(
                field,
                "values must be sorted and unique",
            ));
        }
        previous = Some(value);
    }
    Ok(())
}

fn require_contains(
    field: &'static str,
    actual: &[String],
    expected: &str,
) -> Result<(), SyntaxBackendBoundaryError> {
    if actual.iter().any(|value| value.contains(expected)) {
        Ok(())
    } else {
        Err(SyntaxBackendBoundaryError::new(
            field,
            format!("missing value containing {expected}"),
        ))
    }
}

/// Inventory scratch-seed walk through the source-record generator-core surface.
///
/// Owns token-only decode, expanded selected-completion sample-step selection,
/// canonical elided selected-completion, and incomplete-prefix rejection so the
/// tokenizer-generator megamodule does not construct
/// [`RawScratchGeneratorCore`] or sample-step sets itself when stamping seed
/// evidence. Full seed evidence summary assembly (including render/reparse and
/// frontier coverage) lives on
/// [`GoTokenizerGeneratorManifestDepthScratchSeedSummary::from_inventory_seed`];
/// this walk is the reusable SM walk boundary for seed streams.
pub struct GoSourceRecordInventoryScratchSeedWalk {
    pub generator_core_api_id: &'static str,
    pub generator_core_api_boundary: &'static str,
    pub token_only_decode: RawScratchTokenOnlyDecodeResult,
    pub sample_steps: BTreeSet<usize>,
    pub elided_selected_completion: GoInventoryScratchSeedElidedSelectedCompletionEvidence,
    pub incomplete_prefix_rejection: GoInventoryScratchSeedIncompletePrefixRejectionEvidence,
}

impl GoSourceRecordInventoryScratchSeedWalk {
    /// Run expanded selected-completion (which includes token-only decode),
    /// elide the expanded stream, walk the shared elided selected-completion
    /// surface, probe incomplete-prefix rejection on multi-token streams, and
    /// expose sample steps for an inventory scratch-seed stream.
    ///
    /// Aggregate seed-admissibility proofs may still come from seed
    /// construction; this walk is the reusable generator-core surface the
    /// report path must use instead of constructing `RawScratchGeneratorCore`
    /// in the tokenizer-generator megamodule.
    pub fn run(
        model: &CompiledSyntaxModel,
        raw_token_ids: &[u32],
    ) -> Result<Self, SyntaxBackendBoundaryError> {
        let core = RawScratchGeneratorCore::new(model);
        let sample_steps = selected_completion_sample_steps(raw_token_ids.len());
        // Shared selected-completion surface already token-only decodes the
        // stream after the admission walk; reuse that tree for seed evidence.
        let selected_completion = core.selected_completion(
            raw_token_ids,
            &sample_steps,
            "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.selected_completion",
        )?;
        let token_only_summary = RawScratchTokenOnlyDecodeSummary::from_tree(
            raw_token_ids.len(),
            selected_completion.raw_token_ids_sha256.clone(),
            &selected_completion.decoded_tree,
            "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.token_only_decode",
        )?;
        let elided_selected_completion =
            inventory_scratch_seed_elided_selected_completion_evidence(model, raw_token_ids)?;
        let incomplete_prefix_rejection =
            inventory_scratch_seed_incomplete_prefix_rejection_evidence(
                &core,
                raw_token_ids,
                model.token_roles().vocab_size,
                model.token_roles().pad_token,
            )?;
        Ok(Self {
            generator_core_api_id: core.api_id(),
            generator_core_api_boundary: core.api_boundary(),
            token_only_decode: RawScratchTokenOnlyDecodeResult {
                summary: token_only_summary,
                decoded_tree: selected_completion.decoded_tree,
            },
            sample_steps,
            elided_selected_completion,
            incomplete_prefix_rejection,
        })
    }
}

/// Elide an inventory seed's expanded SM-native stream and walk the shared
/// elided selected-completion surface.
fn inventory_scratch_seed_elided_selected_completion_evidence(
    model: &CompiledSyntaxModel,
    expanded_raw_token_ids: &[u32],
) -> Result<GoInventoryScratchSeedElidedSelectedCompletionEvidence, SyntaxBackendBoundaryError> {
    let field = "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.elided_selected_completion";
    let canonical_token_ids = model
        .elide_forced_raw_tokens(expanded_raw_token_ids)
        .map_err(|error| {
            SyntaxBackendBoundaryError::new(
                field,
                format!(
                    "failed to elide mask-forced tokens from inventory scratch seed stream: {}",
                    error.message
                ),
            )
        })?;
    let run = RawScratchGeneratorCore::elided_selected_completion(
        model,
        &canonical_token_ids,
        expanded_raw_token_ids,
        field,
    )?;
    require_usize(
        "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.elided_selected_completion.canonical_token_count",
        run.elided_token_count,
        canonical_token_ids.len(),
    )?;
    require_usize(
        "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.elided_selected_completion.expanded_token_count",
        run.expanded_token_count,
        expanded_raw_token_ids.len(),
    )?;
    require_usize(
        "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.elided_selected_completion.forced_token_count",
        run.forced_token_count,
        expanded_raw_token_ids
            .len()
            .saturating_sub(canonical_token_ids.len()),
    )?;
    let evidence = GoInventoryScratchSeedElidedSelectedCompletionEvidence {
        status: GO_AST_INVENTORY_SCRATCH_SEED_ELIDED_SELECTED_COMPLETION_STATUS.to_string(),
        scope: GO_AST_INVENTORY_SCRATCH_SEED_ELIDED_SELECTED_COMPLETION_SCOPE.to_string(),
        stream_contract: LIVE_RAW_SYNTAX_STREAM_CONTRACT.to_string(),
        canonical_token_count: canonical_token_ids.len(),
        canonical_tokens_sha256: run.elided_tokens_sha256.clone(),
        canonical_max_token_id: canonical_token_ids.iter().copied().max(),
        expanded_token_count: expanded_raw_token_ids.len(),
        forced_token_count: run.forced_token_count,
        expansion_matches_expanded_stream: run.expansion_matches_expanded_stream,
        every_selected_elided_token_admitted: run.every_selected_elided_token_admitted,
        every_elided_position_non_forced: run.every_elided_position_non_forced,
        transition_count: run.transition_count,
        admissible_mask_query_count: run.admissible_mask_query_count,
        first_selected_elided_raw_token_id: run.first_selected_elided_raw_token_id,
        final_selected_elided_raw_token_id: run.final_selected_elided_raw_token_id,
        final_state_eof_valid: run.final_state.eof_valid,
        final_state_consumed_raw_tokens: run.final_state.consumed_raw_tokens,
        decoded_tree_reusable: true,
        decoded_tree_consumed_raw_tokens: run.decoded_tree.state.consumed_raw_tokens,
        claim_boundary: GO_AST_INVENTORY_SCRATCH_SEED_ELIDED_SELECTED_COMPLETION_CLAIM_BOUNDARY
            .to_string(),
    };
    evidence.validate(expanded_raw_token_ids.len())?;
    Ok(evidence)
}

/// Probe inventory-model empty-prefix through the shared generator-core surface.
fn inventory_scratch_seed_empty_prefix_evidence(
    model: &CompiledSyntaxModel,
    sidecar: &GrammarStructuralModelSidecar,
) -> Result<GoInventoryScratchSeedEmptyPrefixEvidence, SyntaxBackendBoundaryError> {
    let field = "manifest_depth.scratch_generation_gate.go_ast_inventory_empty_prefix";
    let generator_core = RawScratchGeneratorCore::new(model);
    let empty_prefix = generator_core.empty_prefix(field)?;
    let pad_rejection = generator_core.reject_empty_prefix_candidate(
        empty_prefix.pad_token_id,
        "manifest_depth.scratch_generation_gate.go_ast_inventory_empty_prefix.pad_token_rejected",
    )?;
    let malformed_raw_token_id = empty_prefix.token_layout_vocab_size;
    let malformed_rejection = generator_core.reject_empty_prefix_candidate(
        malformed_raw_token_id,
        "manifest_depth.scratch_generation_gate.go_ast_inventory_empty_prefix.malformed_raw_token_rejected",
    )?;
    let root_record = sidecar.structural_model.records.first().ok_or_else(|| {
        SyntaxBackendBoundaryError::new(
            "manifest_depth.scratch_generation_gate.go_ast_inventory_empty_prefix.structural_root",
            "inventory structural model must expose a root record for empty-prefix evidence",
        )
    })?;

    let evidence = GoInventoryScratchSeedEmptyPrefixEvidence {
        status: GO_AST_INVENTORY_SCRATCH_SEED_EMPTY_PREFIX_STATUS.to_string(),
        scope: GO_AST_INVENTORY_SCRATCH_SEED_EMPTY_PREFIX_SCOPE.to_string(),
        structural_model_id: sidecar.structural_model.model_id.clone(),
        raw_state_machine_core_boundary_id: empty_prefix.raw_state_machine_core_boundary_id,
        raw_state_machine_core_boundary: empty_prefix.raw_state_machine_core_boundary,
        generator_core_api_id: generator_core.api_id().to_string(),
        generator_core_api_boundary: generator_core.api_boundary().to_string(),
        token_layout_vocab_size: empty_prefix.token_layout_vocab_size,
        pad_token_id: empty_prefix.pad_token_id,
        empty_prefix_root: EMPTY_PREFIX_ROOT.to_string(),
        structural_root_rule: root_record.grammar_rule.clone(),
        structural_root_node_kind: root_record.record_type.clone(),
        initial_state_observed_status: empty_prefix.initial_state.observed_status,
        initial_state_consumed_raw_tokens: empty_prefix.initial_state.consumed_raw_tokens,
        initial_state_total_raw_tokens: empty_prefix.initial_state.total_raw_tokens,
        initial_state_eof_valid: empty_prefix.initial_state.eof_valid,
        initial_state_stack_depth: empty_prefix.initial_state.stack_depth,
        initial_state_active_rule: empty_prefix.initial_state.active_rule,
        initial_state_active_field: empty_prefix.initial_state.active_field,
        finite_admissible_raw_token_count: empty_prefix.finite_admissible_raw_token_count,
        finite_admissible_raw_token_ids: empty_prefix.finite_admissible_raw_token_ids,
        finite_admissible_raw_token_ids_sha256: empty_prefix.finite_admissible_raw_token_ids_sha256,
        selected_raw_token_id: empty_prefix.selected_raw_token_id,
        selected_raw_token_admitted: empty_prefix.selected_raw_token_admitted,
        selected_transition_accepted: empty_prefix.selected_transition_accepted,
        selected_transition_observed_status: empty_prefix.selected_transition_state.observed_status,
        selected_transition_consumed_raw_tokens: empty_prefix
            .selected_transition_state
            .consumed_raw_tokens,
        selected_transition_eof_valid: empty_prefix.selected_transition_state.eof_valid,
        selected_transition_stack_depth: empty_prefix.selected_transition_state.stack_depth,
        selected_transition_active_rule: empty_prefix.selected_transition_state.active_rule,
        selected_transition_active_field: empty_prefix.selected_transition_state.active_field,
        pad_token_rejected: pad_rejection.rejected,
        pad_token_error_kind: pad_rejection.error_kind,
        malformed_raw_token_id: malformed_rejection.candidate_raw_token_id,
        malformed_raw_token_rejected: malformed_rejection.rejected,
        malformed_raw_token_error_kind: malformed_rejection.error_kind,
        claim_boundary: GO_AST_INVENTORY_SCRATCH_SEED_EMPTY_PREFIX_CLAIM_BOUNDARY.to_string(),
    };
    evidence.validate(
        evidence.token_layout_vocab_size,
        evidence.pad_token_id,
        &evidence.structural_root_rule,
        &evidence.structural_root_node_kind,
    )?;
    Ok(evidence)
}

/// Probe incomplete-prefix rejection for a multi-token inventory seed stream
/// through the shared generator-core surface.
fn inventory_scratch_seed_incomplete_prefix_rejection_evidence(
    generator_core: &RawScratchGeneratorCore<'_>,
    raw_token_ids: &[u32],
    token_layout_vocab_size: u32,
    token_layout_pad_token: u32,
) -> Result<GoInventoryScratchSeedIncompletePrefixRejectionEvidence, SyntaxBackendBoundaryError> {
    let field = "manifest_depth.scratch_generation_gate.go_ast_inventory_scratch_seed.incomplete_prefix_rejection";
    if raw_token_ids.len() < 2 {
        return Err(SyntaxBackendBoundaryError::new(
            field,
            "inventory scratch-seed incomplete-prefix rejection requires a multi-token seed stream",
        ));
    }
    let prefix_raw_token_count = 1usize;
    let incomplete =
        generator_core.incomplete_prefix(raw_token_ids, prefix_raw_token_count, field)?;
    let prefix_state = &incomplete.prefix_state;
    if prefix_state.status != RawSyntaxStatus::Partial || prefix_state.eof_valid {
        return Err(SyntaxBackendBoundaryError::new(
            field,
            "selected inventory seed raw-token prefix must be partial and not EOF-valid",
        ));
    }
    let selected_next_state = &incomplete.selected_next_transition_state;
    let final_completion = &incomplete.final_completion_state;
    let evidence = GoInventoryScratchSeedIncompletePrefixRejectionEvidence {
        status: GO_AST_INVENTORY_SCRATCH_SEED_INCOMPLETE_PREFIX_REJECTION_STATUS.to_string(),
        scope: GO_AST_INVENTORY_SCRATCH_SEED_INCOMPLETE_PREFIX_REJECTION_SCOPE.to_string(),
        selection_loop_source: SELECTED_COMPLETION_STREAM_SOURCE.to_string(),
        complete_raw_token_count: raw_token_ids.len(),
        prefix_raw_token_count,
        prefix_raw_token_ids_sha256: incomplete.prefix_raw_token_ids_sha256,
        prefix_observed_status: prefix_state.status.as_str().to_string(),
        prefix_consumed_raw_tokens: prefix_state.consumed_raw_tokens,
        prefix_eof_valid: prefix_state.eof_valid,
        prefix_stack_depth: prefix_state.stack_depth,
        prefix_finite_admissible_raw_token_count: prefix_state
            .admissible
            .finite_raw_token_ids
            .len(),
        prefix_finite_admissible_raw_token_ids: prefix_state
            .admissible
            .finite_raw_token_ids
            .clone(),
        prefix_admissible_raw_token_range_count: prefix_state.admissible.raw_token_ranges.len(),
        prefix_payload_end_admissible: prefix_state.admissible.payload_end_admissible,
        continuation_mask_available: !prefix_state.admissible.finite_raw_token_ids.is_empty(),
        strict_completion_rejected: incomplete.strict_incomplete.rejected,
        strict_completion_error_kind: incomplete.strict_incomplete.error_kind,
        strict_completion_consumed_raw_tokens: incomplete.strict_incomplete.consumed_raw_tokens,
        strict_completion_failing_token: incomplete.strict_incomplete.failing_token,
        selected_next_raw_token_id: incomplete.selected_next_raw_token_id,
        selected_next_raw_token_admitted: incomplete.selected_next_raw_token_admitted,
        selected_next_transition_accepted: incomplete.selected_next_transition_accepted,
        selected_next_transition_observed_status: selected_next_state.status.as_str().to_string(),
        selected_next_transition_consumed_raw_tokens: selected_next_state.consumed_raw_tokens,
        selected_next_transition_eof_valid: selected_next_state.eof_valid,
        final_completion_observed_status: final_completion.status.as_str().to_string(),
        final_completion_eof_valid: final_completion.eof_valid,
        claim_boundary: GO_AST_INVENTORY_SCRATCH_SEED_INCOMPLETE_PREFIX_REJECTION_CLAIM_BOUNDARY
            .to_string(),
    };
    evidence.validate(
        raw_token_ids.len(),
        token_layout_vocab_size,
        token_layout_pad_token,
    )?;
    Ok(evidence)
}

fn go_source_record_elided_selected_completion_evidence(
    raw_token_emission: &GoSourceRecordRawTokenEmission,
) -> Result<GoSourceRecordElidedSelectedCompletionEvidence, SyntaxBackendBoundaryError> {
    let run = RawScratchGeneratorCore::elided_selected_completion(
        &raw_token_emission.compiled_model,
        &raw_token_emission.canonical_token_ids,
        &raw_token_emission.raw_tokens.raw_token_ids,
        "elided_selected_completion",
    )?;

    // Source-record evidence reuses the emission's canonical stream identity
    // (contract + sha256 + max token id) while the walk/expand/decode facts
    // come from the shared RawScratchGeneratorCore surface.
    require_usize(
        "elided_selected_completion.canonical_token_count",
        run.elided_token_count,
        raw_token_emission.canonical_token_count,
    )?;
    require_string(
        "elided_selected_completion.canonical_tokens_sha256",
        &run.elided_tokens_sha256,
        &raw_token_emission.canonical_tokens_sha256,
    )?;
    require_usize(
        "elided_selected_completion.expanded_token_count",
        run.expanded_token_count,
        raw_token_emission.expanded_token_count,
    )?;
    require_usize(
        "elided_selected_completion.forced_token_count",
        run.forced_token_count,
        raw_token_emission.forced_token_count,
    )?;

    let evidence = GoSourceRecordElidedSelectedCompletionEvidence {
        status: GO_SOURCE_RECORD_ELIDED_SELECTED_COMPLETION_STATUS.to_string(),
        scope: GO_SOURCE_RECORD_ELIDED_SELECTED_COMPLETION_SCOPE.to_string(),
        stream_contract: raw_token_emission.stream_contract.clone(),
        canonical_token_count: raw_token_emission.canonical_token_count,
        canonical_tokens_sha256: raw_token_emission.canonical_tokens_sha256.clone(),
        canonical_max_token_id: raw_token_emission.canonical_max_token_id,
        expanded_token_count: raw_token_emission.expanded_token_count,
        forced_token_count: raw_token_emission.forced_token_count,
        expansion_matches_expanded_stream: run.expansion_matches_expanded_stream,
        every_selected_elided_token_admitted: run.every_selected_elided_token_admitted,
        every_elided_position_non_forced: run.every_elided_position_non_forced,
        transition_count: run.transition_count,
        admissible_mask_query_count: run.admissible_mask_query_count,
        first_selected_elided_raw_token_id: run.first_selected_elided_raw_token_id,
        final_selected_elided_raw_token_id: run.final_selected_elided_raw_token_id,
        final_state_eof_valid: run.final_state.eof_valid,
        final_state_consumed_raw_tokens: run.final_state.consumed_raw_tokens,
        decoded_tree_reusable: true,
        decoded_tree_consumed_raw_tokens: run.decoded_tree.state.consumed_raw_tokens,
        claim_boundary: GO_SOURCE_RECORD_ELIDED_SELECTED_COMPLETION_CLAIM_BOUNDARY.to_string(),
    };
    evidence.validate(raw_token_emission)?;
    Ok(evidence)
}
fn go_tokenizer_generator_empty_prefix_generation_evidence(
    generator_core: &RawScratchGeneratorCore<'_>,
    checked_report: &SyntaxGoSourceInstanceProjectionReport,
    raw_tokens: &SourceInstanceProjectionRawTokenIds,
    decoded_tree: &RawSyntaxDecodedTree,
) -> Result<GoTokenizerGeneratorEmptyPrefixGenerationEvidence, SyntaxBackendBoundaryError> {
    let empty_prefix =
        generator_core.empty_prefix("empty_prefix_generation.raw_state_machine_summary")?;
    let claim_flags = RawScratchClaimBoundaryFlags::raw_state_machine_only();

    let pad_token_id = raw_tokens.token_layout.pad_token;
    let pad_rejection = generator_core.reject_empty_prefix_candidate(
        pad_token_id,
        "empty_prefix_generation.pad_token_rejected",
    )?;

    let malformed_raw_token_id = raw_tokens.token_layout.vocab_size;
    let malformed_rejection = generator_core.reject_empty_prefix_candidate(
        malformed_raw_token_id,
        "empty_prefix_generation.malformed_raw_token_rejected",
    )?;

    let structural_root_rule = decoded_tree.root_rule().ok_or_else(|| {
        SyntaxBackendBoundaryError::new(
            "empty_prefix_generation.structural_root_rule",
            "decoded checked stream must expose a structural root rule",
        )
    })?;
    let structural_root_node_kind = decoded_tree.root_node_kind().ok_or_else(|| {
        SyntaxBackendBoundaryError::new(
            "empty_prefix_generation.structural_root_node_kind",
            "decoded checked stream must expose a structural root node kind",
        )
    })?;

    let evidence = GoTokenizerGeneratorEmptyPrefixGenerationEvidence {
        status: EMPTY_PREFIX_GENERATION_STATUS.to_string(),
        scope: EMPTY_PREFIX_GENERATION_SCOPE.to_string(),
        precompiled_artifact_kind: PRECOMPILED_GO_SYNTAX_TOKENIZER_ARTIFACT_KIND.to_string(),
        precompiled_artifact_version: PRECOMPILED_ARTIFACT_VERSION,
        precompiled_artifact_grammar_id: checked_report.candidate_grammar_id.clone(),
        precompiled_artifact_metadata_path: checked_report.metadata.path.clone(),
        precompiled_artifact_metadata_sha256: checked_report.metadata.sha256.clone(),
        precompiled_artifact_structural_model_id: raw_tokens.structural_model_id.clone(),
        raw_state_machine_core_boundary_id: empty_prefix.raw_state_machine_core_boundary_id,
        raw_state_machine_core_boundary: empty_prefix.raw_state_machine_core_boundary,
        generator_core_api_id: generator_core.api_id().to_string(),
        generator_core_api_boundary: generator_core.api_boundary().to_string(),
        token_layout_vocab_size: empty_prefix.token_layout_vocab_size,
        pad_token_id: empty_prefix.pad_token_id,
        empty_prefix_root: EMPTY_PREFIX_ROOT.to_string(),
        structural_root_rule: structural_root_rule.to_string(),
        structural_root_node_kind: structural_root_node_kind.to_string(),
        initial_state_observed_status: empty_prefix.initial_state.observed_status,
        initial_state_consumed_raw_tokens: empty_prefix.initial_state.consumed_raw_tokens,
        initial_state_total_raw_tokens: empty_prefix.initial_state.total_raw_tokens,
        initial_state_eof_valid: empty_prefix.initial_state.eof_valid,
        initial_state_stack_depth: empty_prefix.initial_state.stack_depth,
        initial_state_active_rule: empty_prefix.initial_state.active_rule,
        initial_state_active_field: empty_prefix.initial_state.active_field,
        finite_admissible_raw_token_count: empty_prefix.finite_admissible_raw_token_count,
        finite_admissible_raw_token_ids: empty_prefix.finite_admissible_raw_token_ids,
        finite_admissible_raw_token_ids_sha256: empty_prefix.finite_admissible_raw_token_ids_sha256,
        selected_raw_token_id: empty_prefix.selected_raw_token_id,
        selected_raw_token_admitted: empty_prefix.selected_raw_token_admitted,
        selected_transition_accepted: empty_prefix.selected_transition_accepted,
        selected_transition_observed_status: empty_prefix.selected_transition_state.observed_status,
        selected_transition_consumed_raw_tokens: empty_prefix
            .selected_transition_state
            .consumed_raw_tokens,
        selected_transition_eof_valid: empty_prefix.selected_transition_state.eof_valid,
        selected_transition_stack_depth: empty_prefix.selected_transition_state.stack_depth,
        selected_transition_active_rule: empty_prefix.selected_transition_state.active_rule,
        selected_transition_active_field: empty_prefix.selected_transition_state.active_field,
        pad_token_rejected: pad_rejection.rejected,
        pad_token_error_kind: Some(pad_rejection.error_kind),
        malformed_raw_token_id: malformed_rejection.candidate_raw_token_id,
        malformed_raw_token_rejected: malformed_rejection.rejected,
        malformed_raw_token_error_kind: Some(malformed_rejection.error_kind),
        per_output_parser_metadata_required: claim_flags.per_output_parser_metadata_required,
        source_instance_segment_sidecar_used: claim_flags.source_instance_segment_sidecar_used,
        source_prefix_table_used: claim_flags.source_prefix_table_used,
        arena_ir_used: claim_flags.arena_ir_used,
        route_handoff_provenance_used: claim_flags.route_handoff_provenance_used,
        bpe_validity_authority_used: claim_flags.bpe_validity_authority_used,
        stable_backend_claimed: claim_flags.stable_backend_claimed,
        next_ready_task: NEXT_READY_TASK.to_string(),
        claim_boundary: EMPTY_PREFIX_GENERATION_CLAIM_BOUNDARY.to_string(),
    };
    evidence.validate(
        &checked_report.candidate_grammar_id,
        &checked_report.metadata.path,
        &checked_report.metadata.sha256,
        &raw_tokens.structural_model_id,
        raw_tokens.token_layout.vocab_size,
        raw_tokens.token_layout.pad_token,
    )?;
    Ok(evidence)
}

fn go_tokenizer_generator_selected_completion_smoke_evidence(
    generator_core: &RawScratchGeneratorCore<'_>,
    checked_report: &SyntaxGoSourceInstanceProjectionReport,
    raw_tokens: &SourceInstanceProjectionRawTokenIds,
) -> Result<GoTokenizerGeneratorSelectedCompletionSmokeEvidence, SyntaxBackendBoundaryError> {
    let sample_steps = selected_completion_sample_steps(raw_tokens.raw_token_ids.len());
    let selected_completion_run = generator_core.selected_completion(
        &raw_tokens.raw_token_ids,
        &sample_steps,
        "selected_completion_smoke.raw_state_machine_summary",
    )?;
    let selected_completion = &selected_completion_run.summary;
    let claim_flags = RawScratchClaimBoundaryFlags::raw_state_machine_only();

    require_string(
        "selected_completion_smoke.raw_token_ids_sha256",
        &selected_completion_run.raw_token_ids_sha256,
        &raw_tokens.raw_token_ids_sha256,
    )?;
    // Single-record render/reparse pure conjunction is generator-core-owned
    // (same formula as token-only + single-record report path).
    let render_flags =
        GoSourceRecordCanonicalRenderReadinessFlags::from_checked_report(checked_report);
    let canonical_render_ready = render_flags.canonical_render_ready;

    let evidence = GoTokenizerGeneratorSelectedCompletionSmokeEvidence {
        status: SELECTED_COMPLETION_SMOKE_STATUS.to_string(),
        scope: SELECTED_COMPLETION_SMOKE_SCOPE.to_string(),
        precompiled_artifact_kind: PRECOMPILED_GO_SYNTAX_TOKENIZER_ARTIFACT_KIND.to_string(),
        precompiled_artifact_version: PRECOMPILED_ARTIFACT_VERSION,
        precompiled_artifact_grammar_id: checked_report.candidate_grammar_id.clone(),
        precompiled_artifact_metadata_path: checked_report.metadata.path.clone(),
        precompiled_artifact_metadata_sha256: checked_report.metadata.sha256.clone(),
        precompiled_artifact_structural_model_id: raw_tokens.structural_model_id.clone(),
        raw_state_machine_core_boundary_id: RAW_SYNTAX_STATE_MACHINE_CORE_BOUNDARY_ID.to_string(),
        raw_state_machine_core_boundary: RAW_SYNTAX_STATE_MACHINE_CORE_BOUNDARY.to_string(),
        generator_core_api_id: generator_core.api_id().to_string(),
        generator_core_api_boundary: generator_core.api_boundary().to_string(),
        selection_loop_source: SELECTED_COMPLETION_STREAM_SOURCE.to_string(),
        token_layout_vocab_size: raw_tokens.token_layout.vocab_size,
        pad_token_id: raw_tokens.token_layout.pad_token,
        empty_prefix_root: EMPTY_PREFIX_ROOT.to_string(),
        raw_token_count: selected_completion.raw_token_count,
        transition_count: selected_completion.transition_count,
        raw_token_ids_sha256: selected_completion_run.raw_token_ids_sha256.clone(),
        admissible_mask_query_count: selected_completion.admissible_mask_query_count,
        finite_admissible_raw_token_mask_available: selected_completion
            .finite_admissible_raw_token_mask_available,
        every_selected_raw_token_admitted: selected_completion.every_selected_raw_token_admitted,
        selected_transition_accepted_count: selected_completion.selected_transition_accepted_count,
        first_selected_raw_token_id: selected_completion.first_selected_raw_token_id,
        final_selected_raw_token_id: selected_completion.final_selected_raw_token_id,
        final_state_observed_status: selected_completion.final_state.observed_status.clone(),
        final_state_consumed_raw_tokens: selected_completion.final_state.consumed_raw_tokens,
        final_state_eof_valid: selected_completion.final_state.eof_valid,
        final_state_stack_depth: selected_completion.final_state.stack_depth,
        decoded_tree_reusable: true,
        decoded_tree: GoTokenizerGeneratorDecodedTreeSummary::from_tree(
            &selected_completion_run.decoded_tree,
        )?,
        canonical_render: checked_report.canonical_render.clone(),
        canonical_render_ready,
        gofmt_validation: checked_report.canonical_render.gofmt.clone(),
        go_parser_reparse_validation: checked_report.canonical_render.go_parser.clone(),
        render_reparse_sidecar_validation: canonical_render_ready,
        step_trace_sample: selected_completion
            .step_samples
            .iter()
            .map(|sample| GoTokenizerGeneratorSelectedCompletionStepSample {
                step_index: sample.step_index,
                prefix_raw_token_count: sample.prefix_raw_token_count,
                selected_raw_token_id: sample.selected_raw_token_id,
                finite_admissible_raw_token_count: sample.finite_admissible_raw_token_count,
                selected_raw_token_admitted: sample.selected_raw_token_admitted,
                selected_transition_accepted: sample.selected_transition_accepted,
                transition_observed_status: sample.transition_state.observed_status.clone(),
                transition_consumed_raw_tokens: sample.transition_state.consumed_raw_tokens,
                transition_eof_valid: sample.transition_state.eof_valid,
                transition_stack_depth: sample.transition_state.stack_depth,
                transition_active_rule: sample.transition_state.active_rule.clone(),
                transition_active_field: sample.transition_state.active_field.clone(),
            })
            .collect(),
        per_output_parser_metadata_required: claim_flags.per_output_parser_metadata_required,
        source_instance_segment_sidecar_used: claim_flags.source_instance_segment_sidecar_used,
        source_prefix_table_used: claim_flags.source_prefix_table_used,
        arena_ir_used: claim_flags.arena_ir_used,
        route_handoff_provenance_used: claim_flags.route_handoff_provenance_used,
        bpe_validity_authority_used: claim_flags.bpe_validity_authority_used,
        stable_backend_claimed: claim_flags.stable_backend_claimed,
        next_ready_task: NEXT_READY_TASK.to_string(),
        claim_boundary: SELECTED_COMPLETION_SMOKE_CLAIM_BOUNDARY.to_string(),
    };
    evidence.validate(
        &checked_report.candidate_grammar_id,
        &checked_report.metadata.path,
        &checked_report.metadata.sha256,
        &raw_tokens.structural_model_id,
        raw_tokens.token_layout.vocab_size,
        raw_tokens.token_layout.pad_token,
        raw_tokens.raw_token_count,
        &raw_tokens.raw_token_ids_sha256,
        &checked_report.source.sha256,
    )?;
    Ok(evidence)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoTokenizerGeneratorIncompletePrefixRejectionEvidence {
    pub status: String,
    pub scope: String,
    pub precompiled_artifact_kind: String,
    pub precompiled_artifact_version: u32,
    pub precompiled_artifact_grammar_id: String,
    pub precompiled_artifact_metadata_path: String,
    pub precompiled_artifact_metadata_sha256: String,
    pub precompiled_artifact_structural_model_id: String,
    pub raw_state_machine_core_boundary_id: String,
    pub raw_state_machine_core_boundary: String,
    pub generator_core_api_id: String,
    pub generator_core_api_boundary: String,
    pub selection_loop_source: String,
    pub token_layout_vocab_size: u32,
    pub pad_token_id: u32,
    pub empty_prefix_root: String,
    pub raw_token_ids_sha256: String,
    pub complete_raw_token_count: usize,
    pub prefix_raw_token_count: usize,
    pub prefix_raw_token_ids_sha256: String,
    pub prefix_observed_status: String,
    pub prefix_consumed_raw_tokens: usize,
    pub prefix_total_raw_tokens: usize,
    pub prefix_eof_valid: bool,
    pub prefix_stack_depth: usize,
    pub prefix_active_rule: Option<String>,
    pub prefix_active_field: Option<String>,
    pub prefix_finite_admissible_raw_token_count: usize,
    pub prefix_finite_admissible_raw_token_ids: Vec<u32>,
    pub prefix_admissible_raw_token_range_count: usize,
    pub prefix_payload_end_admissible: bool,
    pub continuation_mask_available: bool,
    pub strict_completion_rejected: bool,
    pub strict_completion_error_kind: String,
    pub strict_completion_consumed_raw_tokens: usize,
    pub strict_completion_failing_token: Option<u32>,
    pub selected_next_raw_token_id: u32,
    pub selected_next_raw_token_admitted: bool,
    pub selected_next_transition_accepted: bool,
    pub selected_next_transition_observed_status: String,
    pub selected_next_transition_consumed_raw_tokens: usize,
    pub selected_next_transition_eof_valid: bool,
    pub final_completion_observed_status: String,
    pub final_completion_eof_valid: bool,
    pub per_output_parser_metadata_required: bool,
    pub source_instance_segment_sidecar_used: bool,
    pub source_prefix_table_used: bool,
    pub arena_ir_used: bool,
    pub route_handoff_provenance_used: bool,
    pub bpe_validity_authority_used: bool,
    pub stable_backend_claimed: bool,
    pub next_ready_task: String,
    pub claim_boundary: String,
}

impl GoTokenizerGeneratorIncompletePrefixRejectionEvidence {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn validate(
        &self,
        candidate_grammar_id: &str,
        metadata_path: &str,
        metadata_sha256: &str,
        structural_model_id: &str,
        token_layout_vocab_size: u32,
        token_layout_pad_token: u32,
        raw_token_count: usize,
        raw_token_ids_sha256: &str,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        require_string(
            "incomplete_prefix_rejection.status",
            &self.status,
            INCOMPLETE_PREFIX_REJECTION_STATUS,
        )?;
        require_string(
            "incomplete_prefix_rejection.scope",
            &self.scope,
            INCOMPLETE_PREFIX_REJECTION_SCOPE,
        )?;
        require_string(
            "incomplete_prefix_rejection.precompiled_artifact_kind",
            &self.precompiled_artifact_kind,
            PRECOMPILED_GO_SYNTAX_TOKENIZER_ARTIFACT_KIND,
        )?;
        require_u32(
            "incomplete_prefix_rejection.precompiled_artifact_version",
            self.precompiled_artifact_version,
            PRECOMPILED_ARTIFACT_VERSION,
        )?;
        require_string(
            "incomplete_prefix_rejection.precompiled_artifact_grammar_id",
            &self.precompiled_artifact_grammar_id,
            candidate_grammar_id,
        )?;
        require_string(
            "incomplete_prefix_rejection.precompiled_artifact_metadata_path",
            &self.precompiled_artifact_metadata_path,
            metadata_path,
        )?;
        require_string(
            "incomplete_prefix_rejection.precompiled_artifact_metadata_sha256",
            &self.precompiled_artifact_metadata_sha256,
            metadata_sha256,
        )?;
        require_string(
            "incomplete_prefix_rejection.precompiled_artifact_structural_model_id",
            &self.precompiled_artifact_structural_model_id,
            structural_model_id,
        )?;
        require_string(
            "incomplete_prefix_rejection.raw_state_machine_core_boundary_id",
            &self.raw_state_machine_core_boundary_id,
            RAW_SYNTAX_STATE_MACHINE_CORE_BOUNDARY_ID,
        )?;
        require_string(
            "incomplete_prefix_rejection.raw_state_machine_core_boundary",
            &self.raw_state_machine_core_boundary,
            RAW_SYNTAX_STATE_MACHINE_CORE_BOUNDARY,
        )?;
        require_string(
            "incomplete_prefix_rejection.generator_core_api_id",
            &self.generator_core_api_id,
            RAW_SCRATCH_GENERATOR_CORE_API_ID,
        )?;
        require_string(
            "incomplete_prefix_rejection.generator_core_api_boundary",
            &self.generator_core_api_boundary,
            RAW_SCRATCH_GENERATOR_CORE_API_BOUNDARY,
        )?;
        require_string(
            "incomplete_prefix_rejection.selection_loop_source",
            &self.selection_loop_source,
            SELECTED_COMPLETION_STREAM_SOURCE,
        )?;
        require_u32(
            "incomplete_prefix_rejection.token_layout_vocab_size",
            self.token_layout_vocab_size,
            token_layout_vocab_size,
        )?;
        require_u32(
            "incomplete_prefix_rejection.pad_token_id",
            self.pad_token_id,
            token_layout_pad_token,
        )?;
        require_string(
            "incomplete_prefix_rejection.empty_prefix_root",
            &self.empty_prefix_root,
            EMPTY_PREFIX_ROOT,
        )?;
        require_string(
            "incomplete_prefix_rejection.raw_token_ids_sha256",
            &self.raw_token_ids_sha256,
            raw_token_ids_sha256,
        )?;
        require_usize(
            "incomplete_prefix_rejection.complete_raw_token_count",
            self.complete_raw_token_count,
            raw_token_count,
        )?;
        require_positive_usize(
            "incomplete_prefix_rejection.prefix_raw_token_count",
            self.prefix_raw_token_count,
        )?;
        if self.prefix_raw_token_count >= raw_token_count {
            return Err(SyntaxBackendBoundaryError::new(
                "incomplete_prefix_rejection.prefix_raw_token_count",
                "incomplete prefix must be shorter than the complete raw token stream",
            ));
        }
        require_sha256(
            "incomplete_prefix_rejection.prefix_raw_token_ids_sha256",
            &self.prefix_raw_token_ids_sha256,
        )?;
        require_string(
            "incomplete_prefix_rejection.prefix_observed_status",
            &self.prefix_observed_status,
            "partial",
        )?;
        require_usize(
            "incomplete_prefix_rejection.prefix_consumed_raw_tokens",
            self.prefix_consumed_raw_tokens,
            self.prefix_raw_token_count,
        )?;
        require_usize(
            "incomplete_prefix_rejection.prefix_total_raw_tokens",
            self.prefix_total_raw_tokens,
            self.prefix_raw_token_count,
        )?;
        require_bool(
            "incomplete_prefix_rejection.prefix_eof_valid",
            self.prefix_eof_valid,
            false,
        )?;
        require_positive_usize(
            "incomplete_prefix_rejection.prefix_stack_depth",
            self.prefix_stack_depth,
        )?;
        require_positive_usize(
            "incomplete_prefix_rejection.prefix_finite_admissible_raw_token_count",
            self.prefix_finite_admissible_raw_token_count,
        )?;
        require_usize(
            "incomplete_prefix_rejection.prefix_finite_admissible_raw_token_count",
            self.prefix_finite_admissible_raw_token_count,
            self.prefix_finite_admissible_raw_token_ids.len(),
        )?;
        for token_id in &self.prefix_finite_admissible_raw_token_ids {
            if *token_id == token_layout_pad_token {
                return Err(SyntaxBackendBoundaryError::new(
                    "incomplete_prefix_rejection.prefix_finite_admissible_raw_token_ids",
                    "continuation mask must not admit PAD_TOKEN",
                ));
            }
            if *token_id >= token_layout_vocab_size {
                return Err(SyntaxBackendBoundaryError::new(
                    "incomplete_prefix_rejection.prefix_finite_admissible_raw_token_ids",
                    "continuation mask token must remain below VOCAB_SIZE",
                ));
            }
        }
        require_usize(
            "incomplete_prefix_rejection.prefix_admissible_raw_token_range_count",
            self.prefix_admissible_raw_token_range_count,
            0,
        )?;
        require_bool(
            "incomplete_prefix_rejection.prefix_payload_end_admissible",
            self.prefix_payload_end_admissible,
            false,
        )?;
        require_bool(
            "incomplete_prefix_rejection.continuation_mask_available",
            self.continuation_mask_available,
            true,
        )?;
        require_bool(
            "incomplete_prefix_rejection.strict_completion_rejected",
            self.strict_completion_rejected,
            true,
        )?;
        require_string(
            "incomplete_prefix_rejection.strict_completion_error_kind",
            &self.strict_completion_error_kind,
            "premature_eof",
        )?;
        require_usize(
            "incomplete_prefix_rejection.strict_completion_consumed_raw_tokens",
            self.strict_completion_consumed_raw_tokens,
            self.prefix_raw_token_count,
        )?;
        if self.strict_completion_failing_token.is_some() {
            return Err(SyntaxBackendBoundaryError::new(
                "incomplete_prefix_rejection.strict_completion_failing_token",
                "premature EOF rejection for a token-only prefix must not report a failing token",
            ));
        }
        if self.selected_next_raw_token_id >= token_layout_vocab_size {
            return Err(SyntaxBackendBoundaryError::new(
                "incomplete_prefix_rejection.selected_next_raw_token_id",
                "selected continuation raw token must be below VOCAB_SIZE",
            ));
        }
        if !self
            .prefix_finite_admissible_raw_token_ids
            .contains(&self.selected_next_raw_token_id)
        {
            return Err(SyntaxBackendBoundaryError::new(
                "incomplete_prefix_rejection.selected_next_raw_token_id",
                "selected continuation raw token must be admitted by the prefix finite mask",
            ));
        }
        require_bool(
            "incomplete_prefix_rejection.selected_next_raw_token_admitted",
            self.selected_next_raw_token_admitted,
            true,
        )?;
        require_bool(
            "incomplete_prefix_rejection.selected_next_transition_accepted",
            self.selected_next_transition_accepted,
            true,
        )?;
        require_usize(
            "incomplete_prefix_rejection.selected_next_transition_consumed_raw_tokens",
            self.selected_next_transition_consumed_raw_tokens,
            self.prefix_raw_token_count + 1,
        )?;
        if self.selected_next_transition_observed_status != "partial"
            && self.selected_next_transition_observed_status != "complete"
        {
            return Err(SyntaxBackendBoundaryError::new(
                "incomplete_prefix_rejection.selected_next_transition_observed_status",
                "selected continuation transition status must be partial or complete",
            ));
        }
        require_string(
            "incomplete_prefix_rejection.final_completion_observed_status",
            &self.final_completion_observed_status,
            "complete",
        )?;
        require_bool(
            "incomplete_prefix_rejection.final_completion_eof_valid",
            self.final_completion_eof_valid,
            true,
        )?;
        require_bool(
            "incomplete_prefix_rejection.per_output_parser_metadata_required",
            self.per_output_parser_metadata_required,
            false,
        )?;
        require_bool(
            "incomplete_prefix_rejection.source_instance_segment_sidecar_used",
            self.source_instance_segment_sidecar_used,
            false,
        )?;
        require_bool(
            "incomplete_prefix_rejection.source_prefix_table_used",
            self.source_prefix_table_used,
            false,
        )?;
        require_bool(
            "incomplete_prefix_rejection.arena_ir_used",
            self.arena_ir_used,
            false,
        )?;
        require_bool(
            "incomplete_prefix_rejection.route_handoff_provenance_used",
            self.route_handoff_provenance_used,
            false,
        )?;
        require_bool(
            "incomplete_prefix_rejection.bpe_validity_authority_used",
            self.bpe_validity_authority_used,
            false,
        )?;
        require_bool(
            "incomplete_prefix_rejection.stable_backend_claimed",
            self.stable_backend_claimed,
            false,
        )?;
        require_string(
            "incomplete_prefix_rejection.next_ready_task",
            &self.next_ready_task,
            NEXT_READY_TASK,
        )?;
        require_str_contains(
            "incomplete_prefix_rejection.claim_boundary",
            &self.claim_boundary,
            "syntax-level",
        )?;
        require_str_contains(
            "incomplete_prefix_rejection.claim_boundary",
            &self.claim_boundary,
            "incomplete-prefix",
        )?;
        require_str_contains(
            "incomplete_prefix_rejection.claim_boundary",
            &self.claim_boundary,
            "not source-prefix table",
        )?;
        require_str_contains(
            "incomplete_prefix_rejection.claim_boundary",
            &self.claim_boundary,
            "not Arena IR",
        )?;
        require_str_contains(
            "incomplete_prefix_rejection.claim_boundary",
            &self.claim_boundary,
            "not BPE",
        )
    }
}

fn go_tokenizer_generator_incomplete_prefix_rejection_evidence(
    generator_core: &RawScratchGeneratorCore<'_>,
    checked_report: &SyntaxGoSourceInstanceProjectionReport,
    raw_tokens: &SourceInstanceProjectionRawTokenIds,
) -> Result<GoTokenizerGeneratorIncompletePrefixRejectionEvidence, SyntaxBackendBoundaryError> {
    let prefix_raw_token_count = 1usize;
    let incomplete = generator_core.incomplete_prefix(
        &raw_tokens.raw_token_ids,
        prefix_raw_token_count,
        "incomplete_prefix_rejection.raw_state_machine_probe",
    )?;
    let prefix_state = &incomplete.prefix_state;
    if prefix_state.status != RawSyntaxStatus::Partial || prefix_state.eof_valid {
        return Err(SyntaxBackendBoundaryError::new(
            "incomplete_prefix_rejection.prefix_state",
            "selected Go raw-token prefix must be partial and not EOF-valid",
        ));
    }

    let claim_flags = RawScratchClaimBoundaryFlags::raw_state_machine_only();
    let selected_next_state = &incomplete.selected_next_transition_state;
    let final_completion = &incomplete.final_completion_state;

    let evidence = GoTokenizerGeneratorIncompletePrefixRejectionEvidence {
        status: INCOMPLETE_PREFIX_REJECTION_STATUS.to_string(),
        scope: INCOMPLETE_PREFIX_REJECTION_SCOPE.to_string(),
        precompiled_artifact_kind: PRECOMPILED_GO_SYNTAX_TOKENIZER_ARTIFACT_KIND.to_string(),
        precompiled_artifact_version: PRECOMPILED_ARTIFACT_VERSION,
        precompiled_artifact_grammar_id: checked_report.candidate_grammar_id.clone(),
        precompiled_artifact_metadata_path: checked_report.metadata.path.clone(),
        precompiled_artifact_metadata_sha256: checked_report.metadata.sha256.clone(),
        precompiled_artifact_structural_model_id: raw_tokens.structural_model_id.clone(),
        raw_state_machine_core_boundary_id: RAW_SYNTAX_STATE_MACHINE_CORE_BOUNDARY_ID.to_string(),
        raw_state_machine_core_boundary: RAW_SYNTAX_STATE_MACHINE_CORE_BOUNDARY.to_string(),
        generator_core_api_id: generator_core.api_id().to_string(),
        generator_core_api_boundary: generator_core.api_boundary().to_string(),
        selection_loop_source: SELECTED_COMPLETION_STREAM_SOURCE.to_string(),
        token_layout_vocab_size: raw_tokens.token_layout.vocab_size,
        pad_token_id: raw_tokens.token_layout.pad_token,
        empty_prefix_root: EMPTY_PREFIX_ROOT.to_string(),
        raw_token_ids_sha256: raw_tokens.raw_token_ids_sha256.clone(),
        complete_raw_token_count: raw_tokens.raw_token_count,
        prefix_raw_token_count,
        prefix_raw_token_ids_sha256: incomplete.prefix_raw_token_ids_sha256,
        prefix_observed_status: prefix_state.status.as_str().to_string(),
        prefix_consumed_raw_tokens: prefix_state.consumed_raw_tokens,
        prefix_total_raw_tokens: prefix_state.total_raw_tokens,
        prefix_eof_valid: prefix_state.eof_valid,
        prefix_stack_depth: prefix_state.stack_depth,
        prefix_active_rule: prefix_state.active_rule.clone(),
        prefix_active_field: prefix_state.active_field.clone(),
        prefix_finite_admissible_raw_token_count: prefix_state
            .admissible
            .finite_raw_token_ids
            .len(),
        prefix_finite_admissible_raw_token_ids: prefix_state
            .admissible
            .finite_raw_token_ids
            .clone(),
        prefix_admissible_raw_token_range_count: prefix_state.admissible.raw_token_ranges.len(),
        prefix_payload_end_admissible: prefix_state.admissible.payload_end_admissible,
        continuation_mask_available: !prefix_state.admissible.finite_raw_token_ids.is_empty(),
        strict_completion_rejected: incomplete.strict_incomplete.rejected,
        strict_completion_error_kind: incomplete.strict_incomplete.error_kind,
        strict_completion_consumed_raw_tokens: incomplete.strict_incomplete.consumed_raw_tokens,
        strict_completion_failing_token: incomplete.strict_incomplete.failing_token,
        selected_next_raw_token_id: incomplete.selected_next_raw_token_id,
        selected_next_raw_token_admitted: incomplete.selected_next_raw_token_admitted,
        selected_next_transition_accepted: incomplete.selected_next_transition_accepted,
        selected_next_transition_observed_status: selected_next_state.status.as_str().to_string(),
        selected_next_transition_consumed_raw_tokens: selected_next_state.consumed_raw_tokens,
        selected_next_transition_eof_valid: selected_next_state.eof_valid,
        final_completion_observed_status: final_completion.status.as_str().to_string(),
        final_completion_eof_valid: final_completion.eof_valid,
        per_output_parser_metadata_required: claim_flags.per_output_parser_metadata_required,
        source_instance_segment_sidecar_used: claim_flags.source_instance_segment_sidecar_used,
        source_prefix_table_used: claim_flags.source_prefix_table_used,
        arena_ir_used: claim_flags.arena_ir_used,
        route_handoff_provenance_used: claim_flags.route_handoff_provenance_used,
        bpe_validity_authority_used: claim_flags.bpe_validity_authority_used,
        stable_backend_claimed: claim_flags.stable_backend_claimed,
        next_ready_task: NEXT_READY_TASK.to_string(),
        claim_boundary: INCOMPLETE_PREFIX_REJECTION_CLAIM_BOUNDARY.to_string(),
    };
    evidence.validate(
        &checked_report.candidate_grammar_id,
        &checked_report.metadata.path,
        &checked_report.metadata.sha256,
        &raw_tokens.structural_model_id,
        raw_tokens.token_layout.vocab_size,
        raw_tokens.token_layout.pad_token,
        raw_tokens.raw_token_count,
        &raw_tokens.raw_token_ids_sha256,
    )?;
    Ok(evidence)
}

pub(crate) fn selected_completion_sample_steps(raw_token_count: usize) -> BTreeSet<usize> {
    let mut steps = BTreeSet::new();
    for step in 0..raw_token_count.min(3) {
        steps.insert(step);
    }
    for step in raw_token_count.saturating_sub(3)..raw_token_count {
        steps.insert(step);
    }
    steps
}

fn go_tokenizer_generator_token_only_decode_render_evidence(
    generator_core: &RawScratchGeneratorCore<'_>,
    checked_report: &SyntaxGoSourceInstanceProjectionReport,
    raw_tokens: &SourceInstanceProjectionRawTokenIds,
) -> Result<GoTokenizerGeneratorTokenOnlyDecodeRenderEvidence, SyntaxBackendBoundaryError> {
    let token_only_decode = generator_core.token_only_decode(
        &raw_tokens.raw_token_ids,
        "token_only_decode_render.raw_state_machine_summary",
    )?;
    require_string(
        "token_only_decode_render.raw_token_ids_sha256",
        &token_only_decode.summary.raw_token_ids_sha256,
        &raw_tokens.raw_token_ids_sha256,
    )?;
    let decoded_tree =
        GoTokenizerGeneratorDecodedTreeSummary::from_tree(&token_only_decode.decoded_tree)?;
    // Single-record render/reparse pure conjunction is generator-core-owned
    // (same formula as selected-completion + single-record report path).
    let render_flags =
        GoSourceRecordCanonicalRenderReadinessFlags::from_checked_report(checked_report);
    let canonical_render_ready = render_flags.canonical_render_ready;
    let claim_flags = RawScratchClaimBoundaryFlags::raw_state_machine_only();
    let evidence = GoTokenizerGeneratorTokenOnlyDecodeRenderEvidence {
        status: TOKEN_ONLY_DECODE_RENDER_STATUS.to_string(),
        scope: TOKEN_ONLY_DECODE_RENDER_SCOPE.to_string(),
        precompiled_artifact_kind: PRECOMPILED_GO_SYNTAX_TOKENIZER_ARTIFACT_KIND.to_string(),
        precompiled_artifact_version: PRECOMPILED_ARTIFACT_VERSION,
        precompiled_artifact_grammar_id: checked_report.candidate_grammar_id.clone(),
        precompiled_artifact_metadata_path: checked_report.metadata.path.clone(),
        precompiled_artifact_metadata_sha256: checked_report.metadata.sha256.clone(),
        precompiled_artifact_structural_model_id: raw_tokens.structural_model_id.clone(),
        raw_state_machine_core_boundary_id: RAW_SYNTAX_STATE_MACHINE_CORE_BOUNDARY_ID.to_string(),
        raw_state_machine_core_boundary: RAW_SYNTAX_STATE_MACHINE_CORE_BOUNDARY.to_string(),
        generator_core_api_id: generator_core.api_id().to_string(),
        generator_core_api_boundary: generator_core.api_boundary().to_string(),
        raw_token_stream_source: TOKEN_ONLY_RAW_TOKEN_STREAM_SOURCE.to_string(),
        raw_token_count: token_only_decode.summary.raw_token_count,
        raw_token_ids_sha256: token_only_decode.summary.raw_token_ids_sha256,
        token_layout_vocab_size: raw_tokens.token_layout.vocab_size,
        pad_token_id: raw_tokens.token_layout.pad_token,
        decoded_from_token_only_stream: true,
        decoded_tree_reusable: true,
        decoded_tree,
        canonical_render: checked_report.canonical_render.clone(),
        canonical_render_ready,
        gofmt_validation: checked_report.canonical_render.gofmt.clone(),
        go_parser_reparse_validation: checked_report.canonical_render.go_parser.clone(),
        render_reparse_sidecar_validation: canonical_render_ready,
        per_output_parser_metadata_required: claim_flags.per_output_parser_metadata_required,
        source_instance_segment_sidecar_used: claim_flags.source_instance_segment_sidecar_used,
        source_prefix_table_used: claim_flags.source_prefix_table_used,
        arena_ir_used: claim_flags.arena_ir_used,
        route_handoff_provenance_used: claim_flags.route_handoff_provenance_used,
        bpe_validity_authority_used: claim_flags.bpe_validity_authority_used,
        stable_backend_claimed: claim_flags.stable_backend_claimed,
        next_ready_task: NEXT_READY_TASK.to_string(),
        claim_boundary: TOKEN_ONLY_DECODE_RENDER_CLAIM_BOUNDARY.to_string(),
    };
    evidence.validate(
        &checked_report.candidate_grammar_id,
        &checked_report.metadata.path,
        &checked_report.metadata.sha256,
        &raw_tokens.structural_model_id,
        raw_tokens.token_layout.vocab_size,
        raw_tokens.token_layout.pad_token,
        raw_tokens.raw_token_count,
        &raw_tokens.raw_token_ids_sha256,
        &checked_report.source.sha256,
    )?;
    Ok(evidence)
}

fn validate_canonical_render(
    canonical: &SourceInstanceCanonicalRender,
    source_sha256: &str,
) -> Result<(), SyntaxBackendBoundaryError> {
    require_string(
        "canonical_render.formatter",
        &canonical.formatter,
        "go/format.Source",
    )?;
    require_string("canonical_render.gofmt", &canonical.gofmt, "passed")?;
    require_string("canonical_render.go_parser", &canonical.go_parser, "passed")?;
    require_bool(
        "canonical_render.matches_source",
        canonical.matches_source,
        true,
    )?;
    require_sha256(
        "canonical_render.canonical_sha256",
        &canonical.canonical_sha256,
    )?;
    require_string(
        "canonical_render.canonical_sha256",
        &canonical.canonical_sha256,
        source_sha256,
    )?;
    require_positive_usize(
        "canonical_render.canonical_byte_count",
        canonical.canonical_byte_count,
    )
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
            format!("expected {expected:?}, got {actual:?}"),
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

fn require_positive_usize(
    field: &'static str,
    actual: usize,
) -> Result<(), SyntaxBackendBoundaryError> {
    if actual > 0 {
        Ok(())
    } else {
        Err(SyntaxBackendBoundaryError::new(
            field,
            "expected a positive value",
        ))
    }
}

fn require_non_empty(field: &'static str, actual: &str) -> Result<(), SyntaxBackendBoundaryError> {
    if actual.is_empty() {
        Err(SyntaxBackendBoundaryError::new(
            field,
            "value must be non-empty",
        ))
    } else {
        Ok(())
    }
}

fn require_str_contains(
    field: &'static str,
    actual: &str,
    expected: &str,
) -> Result<(), SyntaxBackendBoundaryError> {
    if actual.contains(expected) {
        Ok(())
    } else {
        Err(SyntaxBackendBoundaryError::new(
            field,
            format!("missing value containing {expected}"),
        ))
    }
}

fn require_sha256(field: &'static str, actual: &str) -> Result<(), SyntaxBackendBoundaryError> {
    if actual.len() == 64 && actual.chars().all(|ch| ch.is_ascii_hexdigit()) {
        Ok(())
    } else {
        Err(SyntaxBackendBoundaryError::new(
            field,
            "value must be a lowercase SHA-256 hex digest",
        ))
    }
}

fn require_none(
    field: &'static str,
    actual: Option<&str>,
) -> Result<(), SyntaxBackendBoundaryError> {
    if actual.is_none() {
        Ok(())
    } else {
        Err(SyntaxBackendBoundaryError::new(
            field,
            "expected no active syntax state value",
        ))
    }
}
