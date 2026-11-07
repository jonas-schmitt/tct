//! First Go raw syntax-token tokenizer generator report.
//!
//! This is the first CLI-facing slice of the Go tokenizer generator path. It
//! consumes checked parser-backed Go source-instance projection evidence,
//! lowers metadata through `GrammarIR -> StructuralModel -> token layout`,
//! emits raw syntax tokens, and validates the complete stream with the raw
//! syntax state machine. The canonical render and reparse evidence remains
//! parser-sidecar evidence from the checked projection report; Arena IR,
//! route-handoff provenance, BPE, and stable-backend claims are intentionally
//! out of scope.

use crate::go_ast_schema_inventory::{
    GO_AST_RAW_TREE_RENDERER_CODEGEN_COVERAGE_STATUS_READY,
    GO_AST_SCHEMA_GRAMMAR_COVERAGE_STATUS_BLOCKED, GO_AST_SCHEMA_GRAMMAR_COVERAGE_STATUS_READY,
    GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH, GO_AST_SCHEMA_INVENTORY_RENDERER_POSITION_FIELD_COUNT,
    GoAstRawTreeRendererCodegenCoverageReport, GoAstRendererPositionField,
    GoAstSchemaGrammarCoverageReport, GoAstSchemaInventoryAdapter,
};
use crate::grammar_ir::{GrammarIr, GrammarTerminalKind};
use crate::provenance::{json_sha256, sha256_hex};
use crate::syntax_backend_boundary::SyntaxBackendBoundaryError;
use crate::syntax_generated_codec::{
    GO_FUNCTION_LITERAL_GENERATED_TEST_MARKERS, GO_FUNCTION_LITERAL_METADATA_PATH,
    GO_LOCAL_VARIADIC_ELLIPSIS_GENERATED_TEST_MARKERS, GO_LOCAL_VARIADIC_ELLIPSIS_METADATA_PATH,
    GO_TYPE_SPEC_ASSIGN_GENERATED_TEST_MARKERS, GO_TYPE_SPEC_ASSIGN_METADATA_PATH,
    GO_TYPE_SPEC_DOC_GENERATED_TEST_MARKERS, GO_TYPE_SPEC_DOC_METADATA_PATH,
    SyntaxGeneratedCodecArtifact, build_registered_parser_backed_syntax_generated_codec_artifact,
    go_syntax_token_layout,
};
use crate::syntax_go_inventory_synthesis::{
    GO_AST_INVENTORY_DECLARATIVE_EXPRESSION_SPEC_KIND,
    GO_AST_INVENTORY_FUNCTION_RETURN_EXPR_TEMPLATE_ID,
    GO_AST_INVENTORY_FUNCTION_STATEMENT_LIST_TEMPLATE_ID,
    GO_AST_INVENTORY_PACKAGE_IMPORT_SPEC_TEMPLATE_ID,
    GO_AST_INVENTORY_PACKAGE_TYPE_EXPR_TEMPLATE_ID,
    GO_AST_INVENTORY_PACKAGE_VAR_TYPE_EXPR_TEMPLATE_ID,
    GO_AST_INVENTORY_PACKAGE_VAR_VALUE_EXPR_TEMPLATE_ID, GO_AST_INVENTORY_SYNTHESIS_CORE_API_ID,
    GO_AST_INVENTORY_TEMPLATE_SYNTHESIS_API_ID, GO_AST_INVENTORY_TEMPLATE_SYNTHESIS_BOUNDARY,
    check_go_ast_inventory_declarative_expression_spec_from_inventory,
    go_ast_inventory_declarative_template_coverage_specs,
};
use crate::syntax_go_raw_tree_renderer::{
    GO_RAW_TREE_COMMENT_ANCHOR_POLICY, GO_RAW_TREE_COMMENT_REFERENCE_POLICY,
    GO_RAW_TREE_RENDERER_ID, GO_RAW_TREE_RENDERER_SCOPE, render_go_source_from_raw_tree,
};
use crate::syntax_go_scratch_seed::{
    GO_AST_INVENTORY_SCRATCH_VAR_DECL_SEED_ID, go_ast_inventory_scratch_witness_specs,
};
use crate::syntax_go_source_instance_projection::{
    GENERATED_CODEC_REPORT_ARTIFACT_KIND, GENERATED_CODEC_REPORT_ROUTE,
    SourceInstanceCanonicalRender, SourceInstanceGeneratedCodecReport,
    SourceInstanceProjectionArtifact, SourceInstanceProjectionRawTokenIds,
    SyntaxGoSourceInstanceProjectionReport, parse_checked_report, parse_projection_artifact,
};
pub use crate::syntax_go_source_record_generator_core::{
    EMPTY_PREFIX_GENERATION_CLAIM_BOUNDARY, EMPTY_PREFIX_GENERATION_SCOPE,
    EMPTY_PREFIX_GENERATION_STATUS, EMPTY_PREFIX_ROOT,
    GO_AST_INVENTORY_SCRATCH_SEED_CLAIM_BOUNDARY,
    GO_AST_INVENTORY_SCRATCH_SEED_ELIDED_SELECTED_COMPLETION_CLAIM_BOUNDARY,
    GO_AST_INVENTORY_SCRATCH_SEED_ELIDED_SELECTED_COMPLETION_SCOPE,
    GO_AST_INVENTORY_SCRATCH_SEED_ELIDED_SELECTED_COMPLETION_STATUS,
    GO_AST_INVENTORY_SCRATCH_SEED_EMPTY_PREFIX_SCOPE,
    GO_AST_INVENTORY_SCRATCH_SEED_EMPTY_PREFIX_STATUS,
    GO_AST_INVENTORY_SCRATCH_SEED_INCOMPLETE_PREFIX_REJECTION_SCOPE,
    GO_AST_INVENTORY_SCRATCH_SEED_INCOMPLETE_PREFIX_REJECTION_STATUS,
    GO_AST_INVENTORY_SCRATCH_SEED_SCOPE, GO_AST_INVENTORY_SCRATCH_SEED_STATUS,
    GO_AST_INVENTORY_SCRATCH_SEED_STREAM_SOURCE,
    GO_SOURCE_RECORD_ELIDED_SELECTED_COMPLETION_CLAIM_BOUNDARY,
    GO_SOURCE_RECORD_ELIDED_SELECTED_COMPLETION_SCOPE,
    GO_SOURCE_RECORD_ELIDED_SELECTED_COMPLETION_STATUS,
    GO_SOURCE_RECORD_GENERATOR_CORE_API_BOUNDARY, GO_SOURCE_RECORD_GENERATOR_CORE_API_ID,
    GoInventoryScratchSeedElidedSelectedCompletionEvidence,
    GoInventoryScratchSeedEmptyPrefixEvidence,
    GoInventoryScratchSeedIncompletePrefixRejectionEvidence,
    GoSourceRecordCanonicalRenderReadinessFlags, GoSourceRecordElidedSelectedCompletionEvidence,
    GoSourceRecordGeneratorCore, GoSourceRecordGeneratorEvidence,
    GoSourceRecordInventoryScratchSeedBatch, GoSourceRecordInventoryScratchSeedWalk,
    GoSourceRecordInventoryScratchSubGate, GoSourceRecordManifestDepthCanonicalRenderReadinessView,
    GoSourceRecordManifestDepthEncodeDecodeReadiness, GoSourceRecordManifestDepthRecordReadiness,
    GoSourceRecordManifestDepthRecordReadinessView, GoSourceRecordManifestDepthRecordSourceFields,
    GoSourceRecordManifestDepthReportClaimStamp, GoSourceRecordManifestDepthRepresentativeView,
    GoSourceRecordManifestDepthVocabSmCompletenessView, GoSourceRecordOracleCutoverAttachmentView,
    GoSourceRecordPipelineFieldStamp, GoSourceRecordPipelineProcessExclusionClaims,
    GoSourceRecordPipelineProcessStageClaims, GoTokenizerGeneratorDecodedTreeSummary,
    GoTokenizerGeneratorEmptyPrefixGenerationEvidence,
    GoTokenizerGeneratorIncompletePrefixRejectionEvidence,
    GoTokenizerGeneratorManifestDepthScratchGenerationGate,
    GoTokenizerGeneratorManifestDepthScratchSeedStepSample,
    GoTokenizerGeneratorManifestDepthScratchSeedSummary,
    GoTokenizerGeneratorSelectedCompletionSmokeEvidence,
    GoTokenizerGeneratorSelectedCompletionStepSample,
    GoTokenizerGeneratorTokenOnlyDecodeRenderEvidence, INCOMPLETE_PREFIX_REJECTION_CLAIM_BOUNDARY,
    INCOMPLETE_PREFIX_REJECTION_SCOPE, INCOMPLETE_PREFIX_REJECTION_STATUS,
    PRECOMPILED_GO_SYNTAX_TOKENIZER_ARTIFACT_KIND, SCRATCH_GENERATION_GATE_CLAIM_BOUNDARY,
    SCRATCH_GENERATION_GATE_NEXT_READY_TASK,
    SCRATCH_GENERATION_GATE_ORACLE_CUTOVER_STATUS_ACCEPTED,
    SCRATCH_GENERATION_GATE_RENDER_AUTHORITY_SOURCE, SCRATCH_GENERATION_GATE_SCOPE,
    SCRATCH_GENERATION_GATE_STATUS, SELECTED_COMPLETION_SMOKE_CLAIM_BOUNDARY,
    SELECTED_COMPLETION_SMOKE_SCOPE, SELECTED_COMPLETION_SMOKE_STATUS,
    SELECTED_COMPLETION_STREAM_SOURCE, TOKEN_ONLY_DECODE_RENDER_CLAIM_BOUNDARY,
    TOKEN_ONLY_DECODE_RENDER_SCOPE, TOKEN_ONLY_DECODE_RENDER_STATUS,
    TOKEN_ONLY_RAW_TOKEN_STREAM_SOURCE,
};
pub use crate::syntax_go_source_record_raw_token::{
    GO_SOURCE_RECORD_RAW_TOKEN_EMISSION_API_BOUNDARY, GO_SOURCE_RECORD_RAW_TOKEN_EMISSION_API_ID,
    GoSourceRecordRawTokenEmission, GoSourceRecordRenderReparseSidecarEvidence,
    emit_go_source_record_raw_tokens,
};

use crate::syntax_go_unified_artifact::{
    GO_UNIFIED_GRAMMAR_ARTIFACT_ID, GO_UNIFIED_GRAMMAR_ID, go_unified_grammar_artifact,
};
use crate::syntax_scratch_generation::{
    RAW_SCRATCH_GENERATOR_CORE_API_BOUNDARY, RAW_SCRATCH_GENERATOR_CORE_API_ID,
};
use crate::syntax_state_machine::{
    RAW_SYNTAX_STATE_MACHINE_CORE_BOUNDARY, RAW_SYNTAX_STATE_MACHINE_CORE_BOUNDARY_ID,
    RawSyntaxDecodedNode, RawSyntaxDecodedTree, RawSyntaxDecodedValue, RawSyntaxState,
    RawSyntaxStatus,
};
use crate::syntax_state_machine_replay::{
    GO_AST_DECLARATION_PRESENCE_REPLAY_ARTIFACT_KIND,
    GO_AST_DECLARATION_PRESENCE_REPLAY_NEXT_READY_TASK, GoAstDeclarationPresenceDecision,
    SyntaxStateMachineReplayReport, build_go_ast_declaration_presence_replay_report,
    build_go_source_instance_replay_report,
};
use crate::syntax_token_layout_diff::LIVE_RAW_SYNTAX_STREAM_CONTRACT;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::{Command as ProcessCommand, Stdio};

pub const TOOL: &str = "syntax_go_tokenizer_generator";
pub const VERSION: u32 = 1;
pub const ARTIFACT_KIND: &str = "tct_syntax_go_tokenizer_generator_report_v1";
pub const STATUS: &str = "go_raw_syntax_tokenizer_generator_fixture_ready";
pub const TARGET: &str = "go";
pub const ROUTE: &str = "go_parser_metadata_raw_syntax_state_machine";
pub const DEFAULT_CHECKED_REPORT_PATH: &str =
    "tests/fixtures/go_syntax_corpus/source-instance-projection-report-os-dir-unix-v1.json";
pub const DEFAULT_OUTPUT_PATH: &str =
    "target/tct-syntax-go-tokenizer-generator/go-os-dir-unix-tokenizer-generator-report-v1.json";
pub const DEFAULT_REPORT_INGESTION_MANIFEST_PATH: &str =
    "tests/fixtures/go_syntax_corpus/training-report-ingestion-current-plus-v1.json";
pub const DEFAULT_MANIFEST_DEPTH_OUTPUT_PATH: &str = "target/tct-syntax-go-tokenizer-generator/current-plus-tokenizer-generator-depth-report-v1.json";
pub const DEFAULT_GO_RAW_TREE_AST_PRINTER_ORACLE_REPORT_PATH: &str =
    "tests/fixtures/go_syntax_corpus/go-raw-tree-ast-printer-oracle-full-adoption-decision-v1.json";
pub const MANIFEST_DEPTH_ARTIFACT_KIND: &str =
    "tct_syntax_go_tokenizer_generator_manifest_depth_report_v1";
pub const MANIFEST_DEPTH_STATUS: &str = "go_tokenizer_generator_manifest_depth_ready";
pub const REPORT_INGESTION_MANIFEST_ARTIFACT_KIND: &str =
    "tct_syntax_go_generated_codec_round_trip_report_ingestion_manifest";
pub const REPORT_INGESTION_MANIFEST_TOOL: &str = "go_syntax_generated_codec_round_trip_gate";
pub const CURRENT_PLUS_GENERATED_CODEC_ROLE: &str = "current_plus_generated_codec_round_trip";
pub const SOURCE_INSTANCE_GENERATED_CODEC_EVIDENCE_SCOPE: &str = "source_instance_generated_codec";
pub const RAW_DECODED_TREE_RENDER_BLOCKER_STATUS: &str =
    "go_tokenizer_generator_raw_decoded_tree_render_blocked";
pub const RAW_DECODED_TREE_RENDER_READY_STATUS: &str =
    "go_tokenizer_generator_raw_decoded_tree_render_ready";
pub const RAW_DECODED_TREE_RENDER_BLOCKER_SCOPE: &str =
    "precompiled_go_source_file_decoded_tree_render_v1";
pub const RAW_DECODED_TREE_RENDER_BLOCKER_NEXT_READY_TASK: &str =
    "materialize Go comment/trivia render policy before raw decoded-tree source render smoke";
pub const GO_SOURCE_RECORD_RAW_TOKEN_EMISSION_NEXT_READY_TASK: &str =
    crate::syntax_roadmap_successors::GO_SOURCE_RECORD_RAW_TOKEN_GENERATOR_CORE_SUCCESSOR_TASK;
pub const GO_TOKENIZER_GENERATOR_STATE_API_SUCCESSOR_NEXT_READY_TASK: &str =
    GO_SOURCE_RECORD_RAW_TOKEN_EMISSION_NEXT_READY_TASK;
pub const RAW_DECODED_TREE_RENDER_NEXT_READY_TASK: &str =
    GO_TOKENIZER_GENERATOR_STATE_API_SUCCESSOR_NEXT_READY_TASK;
pub const GO_AST_GENERATION_FRONTIER_ARTIFACT_KIND: &str =
    "tct_go_ast_generation_frontier_report_v1";
pub const GO_AST_GENERATION_FRONTIER_STATUS_READY: &str = "go_ast_generation_frontier_closed";
pub const GO_AST_GENERATION_FRONTIER_STATUS_BLOCKED: &str = "go_ast_generation_frontier_blocked";
pub const GO_AST_GENERATION_FRONTIER_CATEGORY_READY: &str = "ready";
pub const GO_AST_GENERATION_FRONTIER_CATEGORY_BLOCKED: &str = "blocked";
pub const GO_AST_GENERATION_FRONTIER_CATEGORY_DEFERRED_NON_CORE: &str = "deferred_non_core";
pub const GO_AST_GENERATION_FRONTIER_NEXT_READY_TASK: &str =
    GO_TOKENIZER_GENERATOR_STATE_API_SUCCESSOR_NEXT_READY_TASK;
pub const GO_AST_GENERATION_FRONTIER_BLOCKER_NEXT_READY_TASK: &str =
    "add the next minimal Go AST frontier witness from the first blocked inventory category";
pub const GO_AST_GENERATION_FRONTIER_CLAIM_BOUNDARY: &str = "parser-level Go syntactic correctness frontier: pinned go/parser/go/ast inventory lowers through GrammarIR -> StructuralModel -> token layout -> RawSyntaxStateMachineCore, generation starts from an empty GoSourceFile raw prefix, witnesses select only admitted raw tokens, strict-decode token-only trees, and rely on accepted Go AST printer oracle render/reparse sidecar authority. Canonical formatting and nonessential trivia gaps are diagnostic-only; essential parser/build directives remain sidecar policy. This is not source-prefix table evidence, not per-output parser metadata, not Arena IR or GoIR evidence, not BPE validity authority, not type correctness, and not stable-backend promotion.";
pub const GO_AST_INVENTORY_TEMPLATE_COVERAGE_FRONTIER_ARTIFACT_KIND: &str =
    "tct_go_ast_inventory_template_coverage_frontier_v1";
pub const GO_AST_INVENTORY_TEMPLATE_COVERAGE_FRONTIER_STATUS_BLOCKED: &str =
    "go_ast_inventory_template_coverage_frontier_blocked";
pub const GO_AST_INVENTORY_TEMPLATE_COVERAGE_FRONTIER_STATUS_READY: &str =
    "go_ast_inventory_template_coverage_frontier_ready";
pub const GO_AST_INVENTORY_TEMPLATE_COVERAGE_FRONTIER_BLOCKER_NEXT_READY_TASK: &str =
    "add the Go inventory template capability named by first_missing_template_capability";
pub const GO_AST_INVENTORY_TEMPLATE_COVERAGE_FRONTIER_NEXT_READY_TASK: &str =
    crate::syntax_roadmap_successors::GO_SOURCE_RECORD_RAW_TOKEN_GENERATOR_CORE_SUCCESSOR_TASK;
pub const GO_AST_INVENTORY_TEMPLATE_COVERAGE_FRONTIER_CLAIM_BOUNDARY: &str = "checked go/ast inventory-template coverage frontier: supported template kinds, covered rules, covered fields, and covered finite-domain choices are computed from template-backed token-only decoded streams plus the checked inventory. The next task is selected from reusable template capability gaps, not scratch-witness registry order, not C# breadth, not source-file batches, not Arena IR or GoIR, not BPE, and not report ladders.";
pub const NEXT_READY_TASK: &str = SCRATCH_GENERATION_GATE_NEXT_READY_TASK;
pub const RAW_TREE_RENDER_BROADENING_DECISION_SOURCE: &str =
    "manifest_depth.raw_tree_render_attempt_summary";
pub const RAW_TREE_RENDER_BROADENING_NEXT_CANDIDATE_STATUS: &str =
    "go_tokenizer_generator_raw_tree_render_broadening_next_candidate_ready";
pub const RAW_TREE_RENDER_BROADENING_DIAGNOSTIC_BLOCKED_STATUS: &str =
    "go_tokenizer_generator_raw_tree_render_broadening_diagnostic_blocked";
pub const RAW_TREE_RENDER_BROADENING_GRAMMAR_MODE_READY_STATUS: &str =
    "go_tokenizer_generator_go_ast_printer_oracle_cutover_accepted";
pub const RAW_TREE_RENDER_BROADENING_NO_CANDIDATE_STATUS: &str =
    "go_tokenizer_generator_raw_tree_render_broadening_no_candidate";
pub const RAW_TREE_RENDER_BROADENING_MANIFEST_ONLY_ACTION: &str =
    "set_raw_tree_render_attempt_in_report_ingestion_manifest";
pub const RAW_TREE_RENDER_BROADENING_DIAGNOSTIC_ACTION: &str =
    "fix_structured_raw_tree_renderer_diagnostic";
pub const RAW_TREE_RENDER_BROADENING_GRAMMAR_MODE_ACTION: &str =
    "accept_go_ast_printer_oracle_grammar_mode_cutover";
pub const RAW_TREE_RENDER_BROADENING_NO_CANDIDATE_ACTION: &str =
    "refresh_manifest_depth_prerequisites_or_broaden_current_plus_manifest";
pub const RAW_TREE_RENDER_BROADENING_NEXT_READY_TASK: &str =
    "prove the Go AST printer oracle cutover gate before selecting another raw-tree render source";
pub const GO_RAW_TREE_AST_PRINTER_ORACLE_REPORT_ARTIFACT_KIND: &str =
    "tct_go_raw_tree_ast_printer_oracle_decision_v1";
pub const GO_RAW_TREE_AST_PRINTER_ORACLE_REPORT_VERSION: u32 = 1;
pub const GO_RAW_TREE_AST_PRINTER_ORACLE_REPORT_TOOL: &str = "go_raw_tree_ast_printer_oracle";
pub const GO_RAW_TREE_AST_PRINTER_ORACLE_REPORT_STATUS_PASSED: &str = "passed";
pub const GO_RAW_TREE_AST_PRINTER_ORACLE_REPORT_MODE_GRAMMAR: &str = "grammar-mode";
pub const GO_RAW_TREE_AST_PRINTER_ORACLE_REPORT_RECOMMENDATION_ACCEPTED: &str =
    "accept_go_ast_printer_oracle_grammar_mode_cutover";
pub const GO_RAW_TREE_AST_PRINTER_ORACLE_CUTOVER_STATUS_ACCEPTED: &str =
    SCRATCH_GENERATION_GATE_ORACLE_CUTOVER_STATUS_ACCEPTED;
pub const GO_RAW_TREE_AST_PRINTER_ORACLE_CUTOVER_NEXT_READY_TASK: &str =
    GO_TOKENIZER_GENERATOR_STATE_API_SUCCESSOR_NEXT_READY_TASK;
pub const GO_RAW_TREE_AST_PRINTER_ORACLE_CUTOVER_CLAIM_BOUNDARY: &str = "syntax-level Go raw-tree AST printer oracle cutover decision: grammar-mode reports must materialize decoded raw trees, format through go/format, reparse through go/parser, and preserve essential Go comments or directives. Canonical source fidelity remains diagnostic: checked-source comments, exact layout, and blank-line fidelity are diagnostic only. The handwritten Rust raw-tree renderer is demoted to historical/baseline diagnostic evidence and is not grammar-mode formatting/parsing authority. This is not generated-hot-path code, not a production Rust API, not source-program validity authority, not Arena IR evidence, not route-handoff provenance, not BPE validity authority, and not stable-backend promotion.";
pub const RAW_TREE_RENDER_BROADENING_CLAIM_BOUNDARY: &str = "syntax-level Go raw-tree oracle cutover decision: current-plus report-ingestion records and regenerated tokenizer-generator manifest-depth evidence select either the first structured historical/baseline Rust renderer diagnostic to fix, the Go AST printer oracle as grammar-mode authority when selected attempts are grammar-mode ready, or the next manifest-only raw-tree render attempt candidate when grammar-mode evidence is not yet complete. Canonical source fidelity remains diagnostic; existing Rust raw-tree renderer evidence is historical/baseline-only diagnostics and is not grammar-mode formatting/parsing authority. This is not Arena IR evidence, not route-handoff provenance, not BPE validity authority, not a source-program validity claim, and not stable-backend promotion.";
pub const GO_OS_DIR_UNIX_SOURCE_FILE_GRAMMAR_ID: &str = "go_os_dir_unix_source_file_syntax_v0";
pub const GO_FE_ARM64_NOASM_SOURCE_FILE_GRAMMAR_ID: &str =
    "go_crypto_internal_fips140_edwards25519_field_fe_arm64_noasm_source_file_syntax_v0";
pub const DECLARATION_PRESENCE_STATUS: &str =
    "go_ast_optional_token_presence_preserved_through_generated_render_report";
pub const STATE_MASK_SELECTION_STATUS: &str =
    "go_type_spec_assign_state_mask_selected_raw_token_ready";
pub const STATE_MASK_SELECTION_SCOPE: &str =
    "go_type_spec_assign_presence_mask_state_machine_selection_v1";
pub const DECLARATION_PRESENCE_CLAIM_BOUNDARY: &str = "syntax-level Go optional-token presence evidence: raw replay consumes inventory-backed GenDecl grouped-declaration, TypeSpec.Assign, FuncType.Func, and CallExpr.Ellipsis presence decisions, and registered generated codecs preserve those decisions through encode/decode/render plus gofmt/go parser sidecar tests. This is not Arena IR evidence, not route-handoff provenance, not BPE validity authority, and not stable-backend promotion.";
pub const STATE_MASK_SELECTION_CLAIM_BOUNDARY: &str = "syntax-level Go tokenizer-generator state-mask selection evidence: the Go TypeSpec.Assign presence decision consumes the target-neutral raw state machine's finite admissible raw-token mask, selects an admitted raw syntax token, decodes the complete replay stream, and ties render plus go/parser reparse evidence to the registered generated TypeSpec.Assign codec fixture. This is not Arena IR evidence, not route-handoff provenance, not BPE validity authority, and not stable-backend promotion.";
pub const RAW_DECODED_TREE_RENDER_BLOCKER_CLAIM_BOUNDARY: &str = "syntax-level Go tokenizer-generator raw decoded-tree render blocker: the token-only stream now decodes into a reusable raw syntax tree with target-neutral payload values materialized from raw token IDs and with a nested node/field tree materialized from raw token IDs, but source rendering from that tree must wait for Go comment/trivia position anchors and comment-group reference or deduplication policy instead of reusing checked projection canonical-render data. This is not per-output parser metadata, not source-instance segment sidecar authority, not Arena IR evidence, not route-handoff provenance, not BPE validity authority, and not stable-backend promotion.";
pub const RAW_DECODED_TREE_RENDER_CLAIM_BOUNDARY: &str = "syntax-level Go tokenizer-generator raw decoded-tree render baseline evidence: selected current-plus token-only streams including os/dir_unix.go and fe_arm64_noasm.go decode into reusable raw syntax trees, materialize payload values and comment anchors from raw token IDs, and the historical Rust renderer renders parseable Go source from those decoded trees while tying gofmt/go-parser sidecar validation to the same raw token hash. The Go AST printer oracle is the accepted grammar-mode formatting/parsing authority; the Rust renderer result is retained as historical/baseline diagnostics. Exact checked-source comment/layout fidelity is reported separately as canonical-fidelity evidence and is not required for grammar-mode readiness. This is not checked projection canonical-render reuse, not per-output parser metadata, not source-instance segment sidecar authority, not Arena IR evidence, not route-handoff provenance, not BPE validity authority, and not stable-backend promotion.";
pub const RAW_DECODED_TREE_RENDER_DIAGNOSTIC_CLAIM_BOUNDARY: &str = "syntax-level Go tokenizer-generator raw decoded-tree render diagnostic: the selected current-plus token-only stream decodes into a reusable raw syntax tree and attempts historical/baseline Rust renderer source materialization, then records the first unsupported raw-tree renderer node, expression, statement, comment-anchor, or formatting policy as structured data without falling back to checked projection canonical-render data. The Go AST printer oracle remains grammar-mode formatting/parsing authority. This is not per-output parser metadata, not source-instance segment sidecar authority, not Arena IR evidence, not route-handoff provenance, not BPE validity authority, and not stable-backend promotion.";
pub const PROJECTION_REPORT_MODE_CHECKED_FIXTURE: &str = "checked_fixture";
pub const PROJECTION_REPORT_MODE_SOURCE_DRIVEN_GENERATED: &str = "source_driven_generated";
pub const CLAIM_BOUNDARY: &str = "syntax-level Go tokenizer generator fixture evidence: parser-backed metadata is lowered through GrammarIR, StructuralModel, token layout, contextual raw syntax-token emission, the raw syntax state machine, and token-only decode into a reusable raw syntax tree; canonical render and go/parser reparse evidence are sidecar checks tied to the same raw token stream. Scratch generation starts from a precompiled Go syntax tokenizer artifact and an empty GoSourceFile token-space prefix, not per-output parser metadata. This is not Arena IR evidence, not route-handoff provenance, not BPE validity authority, and not stable-backend promotion.";
const GO_TYPE_SPEC_ASSIGN_PREFIX_STATE_TEST_MARKER: &str =
    "generated_go_type_spec_assign_prefix_state_accepts_selected_live_continuations";
const GO_TYPE_SPEC_ASSIGN_RENDER_TEST_MARKER: &str =
    "generated_go_type_spec_assign_renders_canonical_source";
const GO_TYPE_SPEC_ASSIGN_REPARSE_TEST_MARKER: &str =
    "generated_go_type_spec_assign_render_passes_go_parser_sidecar";
const GO_TYPE_SPEC_ASSIGN_EXPECTED_SOURCE_SNIPPET: &str = "type Counter = int";
const REPRESENTATIVE_RENDERER_POSITION_FIELDS: &[(&str, &str)] = &[
    ("ArrayType", "Lbrack"),
    ("BranchStmt", "TokPos"),
    ("CaseClause", "Case"),
    ("FieldList", "Opening"),
    ("Ident", "NamePos"),
    ("ImportSpec", "EndPos"),
    ("ParenExpr", "Lparen"),
    ("UnaryExpr", "OpPos"),
];
const RENDERER_POSITION_POLICY_CATEGORIES: &[&str] = &[
    "bad_node_source_range_unsupported",
    "comment_trivia_position_sidecar",
    "parser_source_position_sidecar",
    "fixed_syntax_token_renderer_metadata",
    "finite_domain_token_renderer_metadata",
    "syntax_presence_renderer_metadata",
];

const NO_STABLE_CLAIMS: &[&str] = &[
    "no_arena_ir_claim",
    "no_arena_ir_promotion",
    "no_bpe_layer_validity_claim",
    "no_broad_go_language_claim",
    "no_generated_hot_path_claim",
    "no_paper_facing_claim",
    "no_route_handoff_provenance_claim",
    "no_source_validity_by_construction_claim",
    "no_stable_backend_claim",
    "no_type_correctness_claim",
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoTokenizerGeneratorReport {
    pub artifact_kind: String,
    pub version: u32,
    pub tool: String,
    pub status: String,
    pub target: String,
    pub route: String,
    pub candidate_grammar_id: String,
    pub checked_projection_report_path: String,
    pub projection_report_mode: String,
    pub projection_artifact_path: String,
    pub projection_artifact_sha256: String,
    pub source_path: String,
    pub source_sha256: String,
    pub metadata_path: String,
    pub metadata_sha256: String,
    pub parser_tool: String,
    pub parser_version: String,
    pub source_record_generator_core_api_id: String,
    pub source_record_generator_core_api_boundary: String,
    pub source_record_raw_token_emission_api_id: String,
    pub source_record_raw_token_emission_api_boundary: String,
    pub delegated_raw_scratch_generator_core_api_id: String,
    pub delegated_raw_scratch_generator_core_api_boundary: String,
    pub renderer_position_policy_summary: GoTokenizerGeneratorRendererPositionPolicySummary,
    pub structural_model_id: String,
    pub grammar_rule_count: usize,
    pub grammar_field_count: usize,
    pub token_layout_vocab_size: u32,
    pub token_layout_pad_token: u32,
    pub raw_token_count: usize,
    pub raw_token_ids_sha256: String,
    pub max_raw_token_id: Option<u32>,
    pub raw_tokens_below_vocab_size: bool,
    pub pad_token_absent: bool,
    /// Canonical encoded stream contract (`LIVE_RAW_SYNTAX_STREAM_CONTRACT`,
    /// elided-v1). `raw_token_*` above stay expanded-stream evidence; the
    /// canonical fields below identify the encoded stream downstream corpora
    /// and BPE artifacts consume.
    pub stream_contract: String,
    pub canonical_token_count: usize,
    pub canonical_tokens_sha256: String,
    pub canonical_max_token_id: Option<u32>,
    pub forced_token_count: usize,
    pub raw_state_machine_complete: bool,
    pub raw_state_machine_decode: GoTokenizerGeneratorDecodeSummary,
    pub raw_state_machine_tree: GoTokenizerGeneratorDecodedTreeSummary,
    pub token_only_decode_render: GoTokenizerGeneratorTokenOnlyDecodeRenderEvidence,
    pub empty_prefix_generation: GoTokenizerGeneratorEmptyPrefixGenerationEvidence,
    pub selected_completion_smoke: GoTokenizerGeneratorSelectedCompletionSmokeEvidence,
    /// Source-record elided selected-completion (generator-core stamped).
    pub elided_selected_completion: GoSourceRecordElidedSelectedCompletionEvidence,
    pub incomplete_prefix_rejection: GoTokenizerGeneratorIncompletePrefixRejectionEvidence,
    pub raw_decoded_tree_render_blocker: GoTokenizerGeneratorRawDecodedTreeRenderBlocker,
    pub generated_codec_report_artifact_kind: String,
    pub generated_codec_report_route: String,
    pub generated_codec_report_path: String,
    pub replay_report_artifact_kind: String,
    pub replay_report_complete: bool,
    pub replay_report_prefix_state_api_ready: bool,
    pub canonical_render: SourceInstanceCanonicalRender,
    pub canonical_render_ready: bool,
    pub gofmt_validation: String,
    pub go_parser_reparse_validation: String,
    pub declaration_presence: GoTokenizerGeneratorDeclarationPresenceEvidence,
    pub state_mask_selection: GoTokenizerGeneratorStateMaskSelectionEvidence,
    pub pipeline: GoTokenizerGeneratorPipeline,
    pub next_ready_task: String,
    pub validation_commands: Vec<String>,
    pub claim_boundary: String,
    pub no_stable_claims: Vec<String>,
}

impl GoTokenizerGeneratorReport {
    pub fn validate(&self) -> Result<(), SyntaxBackendBoundaryError> {
        require_string("artifact_kind", &self.artifact_kind, ARTIFACT_KIND)?;
        require_u32("version", self.version, VERSION)?;
        require_string("tool", &self.tool, TOOL)?;
        require_string("status", &self.status, STATUS)?;
        require_string("target", &self.target, TARGET)?;
        require_string("route", &self.route, ROUTE)?;
        require_non_empty("candidate_grammar_id", &self.candidate_grammar_id)?;
        require_non_empty(
            "checked_projection_report_path",
            &self.checked_projection_report_path,
        )?;
        require_projection_report_mode(&self.projection_report_mode)?;
        require_non_empty("projection_artifact_path", &self.projection_artifact_path)?;
        require_sha256(
            "projection_artifact_sha256",
            &self.projection_artifact_sha256,
        )?;
        require_non_empty("source_path", &self.source_path)?;
        require_sha256("source_sha256", &self.source_sha256)?;
        require_non_empty("metadata_path", &self.metadata_path)?;
        require_sha256("metadata_sha256", &self.metadata_sha256)?;
        require_non_empty("parser_tool", &self.parser_tool)?;
        require_non_empty("parser_version", &self.parser_version)?;
        require_string(
            "source_record_generator_core_api_id",
            &self.source_record_generator_core_api_id,
            GO_SOURCE_RECORD_GENERATOR_CORE_API_ID,
        )?;
        require_string(
            "source_record_generator_core_api_boundary",
            &self.source_record_generator_core_api_boundary,
            GO_SOURCE_RECORD_GENERATOR_CORE_API_BOUNDARY,
        )?;
        require_string(
            "source_record_raw_token_emission_api_id",
            &self.source_record_raw_token_emission_api_id,
            GO_SOURCE_RECORD_RAW_TOKEN_EMISSION_API_ID,
        )?;
        require_string(
            "source_record_raw_token_emission_api_boundary",
            &self.source_record_raw_token_emission_api_boundary,
            GO_SOURCE_RECORD_RAW_TOKEN_EMISSION_API_BOUNDARY,
        )?;
        require_string(
            "delegated_raw_scratch_generator_core_api_id",
            &self.delegated_raw_scratch_generator_core_api_id,
            RAW_SCRATCH_GENERATOR_CORE_API_ID,
        )?;
        require_string(
            "delegated_raw_scratch_generator_core_api_boundary",
            &self.delegated_raw_scratch_generator_core_api_boundary,
            RAW_SCRATCH_GENERATOR_CORE_API_BOUNDARY,
        )?;
        self.renderer_position_policy_summary.validate()?;
        require_non_empty("structural_model_id", &self.structural_model_id)?;
        require_positive_usize("grammar_rule_count", self.grammar_rule_count)?;
        require_positive_usize("grammar_field_count", self.grammar_field_count)?;
        require_u32(
            "token_layout_pad_token",
            self.token_layout_pad_token,
            self.token_layout_vocab_size.saturating_sub(1),
        )?;
        require_positive_usize("raw_token_count", self.raw_token_count)?;
        require_sha256("raw_token_ids_sha256", &self.raw_token_ids_sha256)?;
        if let Some(max_raw_token_id) = self.max_raw_token_id
            && max_raw_token_id >= self.token_layout_vocab_size
        {
            return Err(SyntaxBackendBoundaryError::new(
                "max_raw_token_id",
                "max raw token ID must be below VOCAB_SIZE",
            ));
        }
        require_bool(
            "raw_tokens_below_vocab_size",
            self.raw_tokens_below_vocab_size,
            true,
        )?;
        require_bool("pad_token_absent", self.pad_token_absent, true)?;
        require_string(
            "stream_contract",
            &self.stream_contract,
            LIVE_RAW_SYNTAX_STREAM_CONTRACT,
        )?;
        require_sha256("canonical_tokens_sha256", &self.canonical_tokens_sha256)?;
        require_usize(
            "forced_token_count",
            self.forced_token_count,
            self.raw_token_count - self.canonical_token_count,
        )?;
        if let Some(canonical_max_token_id) = self.canonical_max_token_id
            && canonical_max_token_id >= self.token_layout_vocab_size
        {
            return Err(SyntaxBackendBoundaryError::new(
                "canonical_max_token_id",
                "canonical max token ID must be below VOCAB_SIZE",
            ));
        }
        require_bool(
            "raw_state_machine_complete",
            self.raw_state_machine_complete,
            true,
        )?;
        self.raw_state_machine_decode
            .validate(self.raw_token_count)?;
        self.raw_state_machine_tree.validate(self.raw_token_count)?;
        self.token_only_decode_render.validate(
            &self.candidate_grammar_id,
            &self.metadata_path,
            &self.metadata_sha256,
            &self.structural_model_id,
            self.token_layout_vocab_size,
            self.token_layout_pad_token,
            self.raw_token_count,
            &self.raw_token_ids_sha256,
            &self.source_sha256,
        )?;
        self.empty_prefix_generation.validate(
            &self.candidate_grammar_id,
            &self.metadata_path,
            &self.metadata_sha256,
            &self.structural_model_id,
            self.token_layout_vocab_size,
            self.token_layout_pad_token,
        )?;
        self.selected_completion_smoke.validate(
            &self.candidate_grammar_id,
            &self.metadata_path,
            &self.metadata_sha256,
            &self.structural_model_id,
            self.token_layout_vocab_size,
            self.token_layout_pad_token,
            self.raw_token_count,
            &self.raw_token_ids_sha256,
            &self.source_sha256,
        )?;
        self.elided_selected_completion
            .validate_against_stream_identity(
                &self.stream_contract,
                self.canonical_token_count,
                &self.canonical_tokens_sha256,
                self.canonical_max_token_id,
                self.raw_token_count,
                self.forced_token_count,
            )?;
        self.incomplete_prefix_rejection.validate(
            &self.candidate_grammar_id,
            &self.metadata_path,
            &self.metadata_sha256,
            &self.structural_model_id,
            self.token_layout_vocab_size,
            self.token_layout_pad_token,
            self.raw_token_count,
            &self.raw_token_ids_sha256,
        )?;
        self.raw_decoded_tree_render_blocker.validate(
            self.raw_token_count,
            &self.raw_token_ids_sha256,
            &self.source_sha256,
            self.raw_state_machine_tree.payload_event_count,
            self.raw_state_machine_tree.payload_token_byte_count,
            self.raw_state_machine_tree.payload_value_count,
        )?;
        require_string(
            "generated_codec_report_artifact_kind",
            &self.generated_codec_report_artifact_kind,
            GENERATED_CODEC_REPORT_ARTIFACT_KIND,
        )?;
        require_string(
            "generated_codec_report_route",
            &self.generated_codec_report_route,
            GENERATED_CODEC_REPORT_ROUTE,
        )?;
        require_non_empty(
            "generated_codec_report_path",
            &self.generated_codec_report_path,
        )?;
        require_non_empty(
            "replay_report_artifact_kind",
            &self.replay_report_artifact_kind,
        )?;
        require_bool("replay_report_complete", self.replay_report_complete, true)?;
        require_bool(
            "replay_report_prefix_state_api_ready",
            self.replay_report_prefix_state_api_ready,
            true,
        )?;
        validate_canonical_render(&self.canonical_render, &self.source_sha256)?;
        require_bool("canonical_render_ready", self.canonical_render_ready, true)?;
        require_string("gofmt_validation", &self.gofmt_validation, "passed")?;
        require_string(
            "go_parser_reparse_validation",
            &self.go_parser_reparse_validation,
            "passed",
        )?;
        self.declaration_presence.validate()?;
        self.state_mask_selection
            .validate(self.token_layout_vocab_size)?;
        self.pipeline.validate()?;
        require_string("next_ready_task", &self.next_ready_task, NEXT_READY_TASK)?;
        require_contains(
            "validation_commands",
            &self.validation_commands,
            "tct-syntax-go-tokenizer-generator",
        )?;
        require_contains(
            "validation_commands",
            &self.validation_commands,
            "cargo test --test syntax_go_source_instance_projection_test",
        )?;
        require_contains(
            "claim_boundary",
            std::slice::from_ref(&self.claim_boundary),
            "syntax-level",
        )?;
        require_contains(
            "claim_boundary",
            std::slice::from_ref(&self.claim_boundary),
            "not Arena IR",
        )?;
        require_contains(
            "claim_boundary",
            std::slice::from_ref(&self.claim_boundary),
            "not BPE",
        )?;
        require_all("no_stable_claims", &self.no_stable_claims, NO_STABLE_CLAIMS)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoTokenizerGeneratorRendererPositionPolicySummary {
    pub inventory_artifact_path: String,
    pub total_field_count: usize,
    pub category_counts: Vec<GoTokenizerGeneratorRendererPositionCategoryCount>,
    pub representative_field_policies: Vec<GoTokenizerGeneratorRendererPositionFieldPolicy>,
}

impl GoTokenizerGeneratorRendererPositionPolicySummary {
    fn from_fields(
        inventory_artifact_path: String,
        fields: &[GoAstRendererPositionField],
    ) -> Result<Self, SyntaxBackendBoundaryError> {
        let mut category_counts = BTreeMap::<String, usize>::new();
        let mut fields_by_path = BTreeMap::<String, &GoAstRendererPositionField>::new();
        for field in fields {
            let category = field.policy.as_str().to_string();
            *category_counts.entry(category).or_default() += 1;
            let path = format!("{}.{}", field.node_name, field.field_name);
            if fields_by_path.insert(path.clone(), field).is_some() {
                return Err(SyntaxBackendBoundaryError::new(
                    "renderer_position_policy_summary.representative_field_policies",
                    format!("duplicate renderer-position field path {path}"),
                ));
            }
        }

        let category_counts = RENDERER_POSITION_POLICY_CATEGORIES
            .iter()
            .map(
                |category| GoTokenizerGeneratorRendererPositionCategoryCount {
                    category: (*category).to_string(),
                    field_count: category_counts.get(*category).copied().unwrap_or(0),
                },
            )
            .collect::<Vec<_>>();
        let representative_field_policies = REPRESENTATIVE_RENDERER_POSITION_FIELDS
            .iter()
            .map(|(node_name, field_name)| {
                let path = format!("{node_name}.{field_name}");
                let field = fields_by_path.get(&path).ok_or_else(|| {
                    SyntaxBackendBoundaryError::new(
                        "renderer_position_policy_summary.representative_field_policies",
                        format!("missing representative renderer-position field {path}"),
                    )
                })?;
                Ok(GoTokenizerGeneratorRendererPositionFieldPolicy {
                    node_name: (*node_name).to_string(),
                    field_name: (*field_name).to_string(),
                    go_type: field.go_type.clone(),
                    policy: field.policy.as_str().to_string(),
                })
            })
            .collect::<Result<Vec<_>, SyntaxBackendBoundaryError>>()?;

        Ok(Self {
            inventory_artifact_path,
            total_field_count: fields.len(),
            category_counts,
            representative_field_policies,
        })
    }

    fn validate(&self) -> Result<(), SyntaxBackendBoundaryError> {
        let expected = go_ast_renderer_position_policy_summary()?;
        require_string(
            "renderer_position_policy_summary.inventory_artifact_path",
            &self.inventory_artifact_path,
            &expected.inventory_artifact_path,
        )?;
        require_usize(
            "renderer_position_policy_summary.total_field_count",
            self.total_field_count,
            GO_AST_SCHEMA_INVENTORY_RENDERER_POSITION_FIELD_COUNT,
        )?;
        if self.category_counts != expected.category_counts {
            return Err(SyntaxBackendBoundaryError::new(
                "renderer_position_policy_summary.category_counts",
                "category counts must match the checked Go AST inventory",
            ));
        }
        if self.representative_field_policies != expected.representative_field_policies {
            return Err(SyntaxBackendBoundaryError::new(
                "renderer_position_policy_summary.representative_field_policies",
                "representative field policies must match the checked Go AST inventory",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoTokenizerGeneratorRendererPositionCategoryCount {
    pub category: String,
    pub field_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoTokenizerGeneratorRendererPositionFieldPolicy {
    pub node_name: String,
    pub field_name: String,
    pub go_type: String,
    pub policy: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoTokenizerGeneratorDecodeSummary {
    pub observed_status: String,
    pub consumed_raw_tokens: usize,
    pub total_raw_tokens: usize,
    pub eof_valid: bool,
    pub stack_depth: usize,
    pub active_rule: Option<String>,
    pub active_field: Option<String>,
    pub diagnostic: String,
}

impl GoTokenizerGeneratorDecodeSummary {
    fn from_state(state: RawSyntaxState) -> Self {
        Self {
            observed_status: state.status.as_str().to_string(),
            consumed_raw_tokens: state.consumed_raw_tokens,
            total_raw_tokens: state.total_raw_tokens,
            eof_valid: state.eof_valid,
            stack_depth: state.stack_depth,
            active_rule: state.active_rule,
            active_field: state.active_field,
            diagnostic: state.diagnostic,
        }
    }

    fn validate(&self, raw_token_count: usize) -> Result<(), SyntaxBackendBoundaryError> {
        require_string("decode.observed_status", &self.observed_status, "complete")?;
        require_usize(
            "decode.consumed_raw_tokens",
            self.consumed_raw_tokens,
            raw_token_count,
        )?;
        require_usize(
            "decode.total_raw_tokens",
            self.total_raw_tokens,
            raw_token_count,
        )?;
        require_bool("decode.eof_valid", self.eof_valid, true)?;
        require_usize("decode.stack_depth", self.stack_depth, 0)?;
        require_non_empty("decode.diagnostic", &self.diagnostic)
    }
}

const RAW_DECODED_TREE_RENDER_MISSING_CAPABILITIES: &[&str] = &[
    "go_file_comments_position_anchor_policy",
    "go_file_comments_position_and_trivia_render_policy",
    "go_comment_group_reference_or_deduplication_policy",
];
pub const RAW_TREE_RENDER_ATTEMPT_SELECTION_SOURCE: &str =
    "report_ingestion_manifest.raw_tree_render_attempt";
pub const RAW_TREE_ESSENTIAL_COMMENT_POLICY_NOT_EVALUATED_STATUS: &str =
    "essential_comments_not_evaluated";
pub const RAW_TREE_ESSENTIAL_COMMENT_POLICY_READY_STATUS: &str =
    "essential_comments_preserved_or_absent";
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoTokenizerGeneratorRawDecodedTreeRenderBlocker {
    pub status: String,
    pub scope: String,
    pub decoded_tree_source: String,
    pub raw_tree_render_attempted: bool,
    #[serde(default = "default_raw_tree_render_attempt_source")]
    pub raw_tree_render_attempt_source: String,
    #[serde(default)]
    pub raw_tree_render_attempt_manifest_path: Option<String>,
    #[serde(default)]
    pub raw_tree_render_attempt_manifest_sha256: Option<String>,
    pub renderer_id: Option<String>,
    pub renderer_scope: Option<String>,
    pub renderer_diagnostic: Option<GoTokenizerGeneratorRawTreeRendererDiagnostic>,
    pub raw_token_count: usize,
    pub raw_token_ids_sha256: String,
    pub source_sha256: Option<String>,
    pub decoded_tree_reusable: bool,
    pub decoded_tree_payload_event_count: usize,
    pub decoded_tree_payload_token_byte_count: usize,
    pub decoded_tree_payload_value_count: usize,
    pub raw_tree_payload_values_materialized: bool,
    pub raw_tree_node_tree_materialized: bool,
    pub root_file_comment_group_count: usize,
    pub attached_comment_group_count: usize,
    pub root_only_comment_group_count: usize,
    pub total_comment_group_node_count: usize,
    pub non_file_comment_group_node_count: usize,
    pub raw_tree_comment_position_policy_materialized: bool,
    pub raw_tree_comment_group_reference_policy_materialized: bool,
    pub raw_tree_comment_anchor_policy: Option<String>,
    pub raw_tree_comment_group_reference_policy: Option<String>,
    pub trailing_comment_anchor_count: usize,
    pub comment_trivia_policy_status: String,
    pub renderer_policy_status: String,
    pub canonical_render_reused_from_checked_projection: bool,
    #[serde(default)]
    pub raw_tree_grammar_mode_render_ready: bool,
    #[serde(default)]
    pub raw_tree_canonical_fidelity_ready: bool,
    #[serde(default)]
    pub essential_comment_policy_ready: bool,
    #[serde(default)]
    pub essential_comment_policy_status: String,
    #[serde(default)]
    pub essential_comment_count: usize,
    #[serde(default)]
    pub essential_comment_categories: Vec<String>,
    pub raw_tree_canonical_render_ready: bool,
    pub raw_tree_rendered_source_sha256: Option<String>,
    pub raw_tree_rendered_source_byte_count: Option<usize>,
    pub raw_tree_formatted_source_sha256: Option<String>,
    pub raw_tree_formatted_source_byte_count: Option<usize>,
    pub raw_tree_render_matches_canonical_source: bool,
    #[serde(default)]
    pub canonical_fidelity_diff_summary: Option<String>,
    pub gofmt_validation: Option<String>,
    pub go_parser_reparse_validation: Option<String>,
    pub missing_capabilities: Vec<String>,
    pub next_ready_task: String,
    pub claim_boundary: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoTokenizerGeneratorRawTreeRendererDiagnostic {
    pub diagnostic_kind: String,
    pub unsupported_category: String,
    pub field: String,
    pub message: String,
    pub missing_capability: String,
}

fn default_raw_tree_render_attempt_source() -> String {
    RAW_TREE_RENDER_ATTEMPT_SELECTION_SOURCE.to_string()
}

impl GoTokenizerGeneratorRawTreeRendererDiagnostic {
    fn validate(&self) -> Result<(), SyntaxBackendBoundaryError> {
        require_string(
            "raw_decoded_tree_render_blocker.renderer_diagnostic.diagnostic_kind",
            &self.diagnostic_kind,
            "raw_tree_renderer_attempt_failed",
        )?;
        require_non_empty(
            "raw_decoded_tree_render_blocker.renderer_diagnostic.unsupported_category",
            &self.unsupported_category,
        )?;
        require_non_empty(
            "raw_decoded_tree_render_blocker.renderer_diagnostic.field",
            &self.field,
        )?;
        require_non_empty(
            "raw_decoded_tree_render_blocker.renderer_diagnostic.message",
            &self.message,
        )?;
        require_str_contains(
            "raw_decoded_tree_render_blocker.renderer_diagnostic.missing_capability",
            &self.missing_capability,
            "go_raw_tree_renderer_",
        )
    }
}

impl GoTokenizerGeneratorRawDecodedTreeRenderBlocker {
    fn validate(
        &self,
        raw_token_count: usize,
        raw_token_ids_sha256: &str,
        source_sha256: &str,
        payload_event_count: usize,
        payload_token_byte_count: usize,
        payload_value_count: usize,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        if self.status != RAW_DECODED_TREE_RENDER_BLOCKER_STATUS
            && self.status != RAW_DECODED_TREE_RENDER_READY_STATUS
        {
            return Err(SyntaxBackendBoundaryError::new(
                "raw_decoded_tree_render_blocker.status",
                "raw decoded-tree render status must be blocked or ready",
            ));
        }
        require_string(
            "raw_decoded_tree_render_blocker.scope",
            &self.scope,
            RAW_DECODED_TREE_RENDER_BLOCKER_SCOPE,
        )?;
        require_string(
            "raw_decoded_tree_render_blocker.decoded_tree_source",
            &self.decoded_tree_source,
            TOKEN_ONLY_RAW_TOKEN_STREAM_SOURCE,
        )?;
        require_string(
            "raw_decoded_tree_render_blocker.raw_tree_render_attempt_source",
            &self.raw_tree_render_attempt_source,
            RAW_TREE_RENDER_ATTEMPT_SELECTION_SOURCE,
        )?;
        if let Some(path) = &self.raw_tree_render_attempt_manifest_path {
            require_non_empty(
                "raw_decoded_tree_render_blocker.raw_tree_render_attempt_manifest_path",
                path,
            )?;
        }
        if let Some(sha256) = &self.raw_tree_render_attempt_manifest_sha256 {
            require_sha256(
                "raw_decoded_tree_render_blocker.raw_tree_render_attempt_manifest_sha256",
                sha256,
            )?;
        }
        let raw_tree_render_ready = self.status == RAW_DECODED_TREE_RENDER_READY_STATUS;
        if raw_tree_render_ready {
            require_bool(
                "raw_decoded_tree_render_blocker.raw_tree_render_attempted",
                self.raw_tree_render_attempted,
                true,
            )?;
        }
        if raw_tree_render_ready || self.raw_tree_render_attempted {
            if self.raw_tree_render_attempt_manifest_path.is_none() {
                return Err(SyntaxBackendBoundaryError::new(
                    "raw_decoded_tree_render_blocker.raw_tree_render_attempt_manifest_path",
                    "attempted raw decoded-tree render evidence must carry the checked selecting manifest path",
                ));
            }
            if self.raw_tree_render_attempt_manifest_sha256.is_none() {
                return Err(SyntaxBackendBoundaryError::new(
                    "raw_decoded_tree_render_blocker.raw_tree_render_attempt_manifest_sha256",
                    "attempted raw decoded-tree render evidence must carry the checked selecting manifest hash",
                ));
            }
            require_string(
                "raw_decoded_tree_render_blocker.renderer_id",
                self.renderer_id.as_deref().unwrap_or(""),
                GO_RAW_TREE_RENDERER_ID,
            )?;
            require_string(
                "raw_decoded_tree_render_blocker.renderer_scope",
                self.renderer_scope.as_deref().unwrap_or(""),
                GO_RAW_TREE_RENDERER_SCOPE,
            )?;
        } else {
            require_bool(
                "raw_decoded_tree_render_blocker.raw_tree_render_attempted",
                self.raw_tree_render_attempted,
                false,
            )?;
            require_none(
                "raw_decoded_tree_render_blocker.renderer_id",
                self.renderer_id.as_deref(),
            )?;
            require_none(
                "raw_decoded_tree_render_blocker.renderer_scope",
                self.renderer_scope.as_deref(),
            )?;
        }
        match (
            raw_tree_render_ready,
            self.raw_tree_render_attempted,
            &self.renderer_diagnostic,
        ) {
            (true, _, Some(_)) => {
                return Err(SyntaxBackendBoundaryError::new(
                    "raw_decoded_tree_render_blocker.renderer_diagnostic",
                    "ready raw decoded-tree render evidence must not carry a renderer diagnostic",
                ));
            }
            (false, true, Some(diagnostic)) => diagnostic.validate()?,
            (false, true, None) => {
                return Err(SyntaxBackendBoundaryError::new(
                    "raw_decoded_tree_render_blocker.renderer_diagnostic",
                    "blocked renderer-attempt evidence must carry a structured diagnostic",
                ));
            }
            (false, false, Some(_)) => {
                return Err(SyntaxBackendBoundaryError::new(
                    "raw_decoded_tree_render_blocker.renderer_diagnostic",
                    "unattempted raw decoded-tree blocker must not carry a renderer diagnostic",
                ));
            }
            _ => {}
        }
        require_usize(
            "raw_decoded_tree_render_blocker.raw_token_count",
            self.raw_token_count,
            raw_token_count,
        )?;
        require_string(
            "raw_decoded_tree_render_blocker.raw_token_ids_sha256",
            &self.raw_token_ids_sha256,
            raw_token_ids_sha256,
        )?;
        if raw_tree_render_ready || self.raw_tree_render_attempted {
            require_string(
                "raw_decoded_tree_render_blocker.source_sha256",
                self.source_sha256.as_deref().unwrap_or(""),
                source_sha256,
            )?;
        } else {
            require_none(
                "raw_decoded_tree_render_blocker.source_sha256",
                self.source_sha256.as_deref(),
            )?;
        }
        require_bool(
            "raw_decoded_tree_render_blocker.decoded_tree_reusable",
            self.decoded_tree_reusable,
            true,
        )?;
        require_usize(
            "raw_decoded_tree_render_blocker.decoded_tree_payload_event_count",
            self.decoded_tree_payload_event_count,
            payload_event_count,
        )?;
        require_usize(
            "raw_decoded_tree_render_blocker.decoded_tree_payload_token_byte_count",
            self.decoded_tree_payload_token_byte_count,
            payload_token_byte_count,
        )?;
        require_usize(
            "raw_decoded_tree_render_blocker.decoded_tree_payload_value_count",
            self.decoded_tree_payload_value_count,
            payload_value_count,
        )?;
        require_positive_usize(
            "raw_decoded_tree_render_blocker.decoded_tree_payload_value_count",
            self.decoded_tree_payload_value_count,
        )?;
        require_bool(
            "raw_decoded_tree_render_blocker.raw_tree_payload_values_materialized",
            self.raw_tree_payload_values_materialized,
            true,
        )?;
        require_bool(
            "raw_decoded_tree_render_blocker.raw_tree_node_tree_materialized",
            self.raw_tree_node_tree_materialized,
            true,
        )?;
        if self.attached_comment_group_count > self.root_file_comment_group_count {
            return Err(SyntaxBackendBoundaryError::new(
                "raw_decoded_tree_render_blocker.attached_comment_group_count",
                "attached comment groups cannot exceed root File.Comments groups",
            ));
        }
        require_usize(
            "raw_decoded_tree_render_blocker.root_only_comment_group_count",
            self.root_only_comment_group_count,
            self.root_file_comment_group_count - self.attached_comment_group_count,
        )?;
        if self.total_comment_group_node_count < self.root_file_comment_group_count {
            return Err(SyntaxBackendBoundaryError::new(
                "raw_decoded_tree_render_blocker.total_comment_group_node_count",
                "total CommentGroup nodes must include root File.Comments groups",
            ));
        }
        require_usize(
            "raw_decoded_tree_render_blocker.non_file_comment_group_node_count",
            self.non_file_comment_group_node_count,
            self.total_comment_group_node_count - self.root_file_comment_group_count,
        )?;
        if self.status == RAW_DECODED_TREE_RENDER_READY_STATUS {
            require_bool(
                "raw_decoded_tree_render_blocker.raw_tree_comment_position_policy_materialized",
                self.raw_tree_comment_position_policy_materialized,
                true,
            )?;
            require_bool(
                "raw_decoded_tree_render_blocker.raw_tree_comment_group_reference_policy_materialized",
                self.raw_tree_comment_group_reference_policy_materialized,
                true,
            )?;
            require_string(
                "raw_decoded_tree_render_blocker.raw_tree_comment_anchor_policy",
                self.raw_tree_comment_anchor_policy.as_deref().unwrap_or(""),
                GO_RAW_TREE_COMMENT_ANCHOR_POLICY,
            )?;
            require_string(
                "raw_decoded_tree_render_blocker.raw_tree_comment_group_reference_policy",
                self.raw_tree_comment_group_reference_policy
                    .as_deref()
                    .unwrap_or(""),
                GO_RAW_TREE_COMMENT_REFERENCE_POLICY,
            )?;
            if self.trailing_comment_anchor_count > self.root_file_comment_group_count {
                return Err(SyntaxBackendBoundaryError::new(
                    "raw_decoded_tree_render_blocker.trailing_comment_anchor_count",
                    "trailing comment anchors cannot exceed root File.Comments groups",
                ));
            }
            require_string(
                "raw_decoded_tree_render_blocker.comment_trivia_policy_status",
                &self.comment_trivia_policy_status,
                "raw_tree_comment_anchor_policy_materialized",
            )?;
            require_string(
                "raw_decoded_tree_render_blocker.renderer_policy_status",
                &self.renderer_policy_status,
                "raw_tree_decoded_source_render_ready",
            )?;
            require_bool(
                "raw_decoded_tree_render_blocker.canonical_render_reused_from_checked_projection",
                self.canonical_render_reused_from_checked_projection,
                false,
            )?;
            require_bool(
                "raw_decoded_tree_render_blocker.raw_tree_grammar_mode_render_ready",
                self.raw_tree_grammar_mode_render_ready,
                true,
            )?;
            require_bool(
                "raw_decoded_tree_render_blocker.essential_comment_policy_ready",
                self.essential_comment_policy_ready,
                true,
            )?;
            require_string(
                "raw_decoded_tree_render_blocker.essential_comment_policy_status",
                &self.essential_comment_policy_status,
                RAW_TREE_ESSENTIAL_COMMENT_POLICY_READY_STATUS,
            )?;
            if self.essential_comment_count == 0 {
                if !self.essential_comment_categories.is_empty() {
                    return Err(SyntaxBackendBoundaryError::new(
                        "raw_decoded_tree_render_blocker.essential_comment_categories",
                        "records without essential Go comments must not report essential comment categories",
                    ));
                }
            } else if self.essential_comment_categories.is_empty() {
                return Err(SyntaxBackendBoundaryError::new(
                    "raw_decoded_tree_render_blocker.essential_comment_categories",
                    "records with essential Go comments must report at least one essential comment category",
                ));
            }
            require_bool(
                "raw_decoded_tree_render_blocker.raw_tree_canonical_fidelity_ready",
                self.raw_tree_canonical_fidelity_ready,
                self.raw_tree_render_matches_canonical_source,
            )?;
            require_bool(
                "raw_decoded_tree_render_blocker.raw_tree_canonical_render_ready",
                self.raw_tree_canonical_render_ready,
                self.raw_tree_canonical_fidelity_ready,
            )?;
            require_sha256(
                "raw_decoded_tree_render_blocker.raw_tree_rendered_source_sha256",
                self.raw_tree_rendered_source_sha256
                    .as_deref()
                    .unwrap_or(""),
            )?;
            require_positive_usize(
                "raw_decoded_tree_render_blocker.raw_tree_rendered_source_byte_count",
                self.raw_tree_rendered_source_byte_count.unwrap_or(0),
            )?;
            require_sha256(
                "raw_decoded_tree_render_blocker.raw_tree_formatted_source_sha256",
                self.raw_tree_formatted_source_sha256
                    .as_deref()
                    .unwrap_or(""),
            )?;
            require_positive_usize(
                "raw_decoded_tree_render_blocker.raw_tree_formatted_source_byte_count",
                self.raw_tree_formatted_source_byte_count.unwrap_or(0),
            )?;
            if self.raw_tree_render_matches_canonical_source {
                require_string(
                    "raw_decoded_tree_render_blocker.raw_tree_formatted_source_sha256",
                    self.raw_tree_formatted_source_sha256
                        .as_deref()
                        .unwrap_or(""),
                    source_sha256,
                )?;
                require_none(
                    "raw_decoded_tree_render_blocker.canonical_fidelity_diff_summary",
                    self.canonical_fidelity_diff_summary.as_deref(),
                )?;
            } else {
                require_non_empty(
                    "raw_decoded_tree_render_blocker.canonical_fidelity_diff_summary",
                    self.canonical_fidelity_diff_summary
                        .as_deref()
                        .unwrap_or(""),
                )?;
            }
            require_string(
                "raw_decoded_tree_render_blocker.gofmt_validation",
                self.gofmt_validation.as_deref().unwrap_or(""),
                "passed",
            )?;
            require_string(
                "raw_decoded_tree_render_blocker.go_parser_reparse_validation",
                self.go_parser_reparse_validation.as_deref().unwrap_or(""),
                "passed",
            )?;
            if !self.missing_capabilities.is_empty() {
                return Err(SyntaxBackendBoundaryError::new(
                    "raw_decoded_tree_render_blocker.missing_capabilities",
                    "ready raw decoded-tree render evidence must not list missing capabilities",
                ));
            }
            require_string(
                "raw_decoded_tree_render_blocker.next_ready_task",
                &self.next_ready_task,
                RAW_DECODED_TREE_RENDER_NEXT_READY_TASK,
            )?;
            require_str_contains(
                "raw_decoded_tree_render_blocker.claim_boundary",
                &self.claim_boundary,
                "syntax-level",
            )?;
            require_str_contains(
                "raw_decoded_tree_render_blocker.claim_boundary",
                &self.claim_boundary,
                "renders parseable Go source",
            )?;
            require_str_contains(
                "raw_decoded_tree_render_blocker.claim_boundary",
                &self.claim_boundary,
                "Exact checked-source comment/layout fidelity is reported separately",
            )?;
            require_str_contains(
                "raw_decoded_tree_render_blocker.claim_boundary",
                &self.claim_boundary,
                "not checked projection canonical-render reuse",
            )?;
            require_str_contains(
                "raw_decoded_tree_render_blocker.claim_boundary",
                &self.claim_boundary,
                "not Arena IR",
            )?;
            return require_str_contains(
                "raw_decoded_tree_render_blocker.claim_boundary",
                &self.claim_boundary,
                "not BPE",
            );
        }
        require_bool(
            "raw_decoded_tree_render_blocker.raw_tree_comment_position_policy_materialized",
            self.raw_tree_comment_position_policy_materialized,
            false,
        )?;
        require_bool(
            "raw_decoded_tree_render_blocker.raw_tree_comment_group_reference_policy_materialized",
            self.raw_tree_comment_group_reference_policy_materialized,
            false,
        )?;
        require_none(
            "raw_decoded_tree_render_blocker.raw_tree_comment_anchor_policy",
            self.raw_tree_comment_anchor_policy.as_deref(),
        )?;
        require_none(
            "raw_decoded_tree_render_blocker.raw_tree_comment_group_reference_policy",
            self.raw_tree_comment_group_reference_policy.as_deref(),
        )?;
        require_usize(
            "raw_decoded_tree_render_blocker.trailing_comment_anchor_count",
            self.trailing_comment_anchor_count,
            0,
        )?;
        if self.raw_tree_render_attempted {
            require_string(
                "raw_decoded_tree_render_blocker.comment_trivia_policy_status",
                &self.comment_trivia_policy_status,
                "raw_tree_renderer_attempted_unsupported_policy",
            )?;
            require_string(
                "raw_decoded_tree_render_blocker.renderer_policy_status",
                &self.renderer_policy_status,
                "blocked_on_raw_tree_renderer_diagnostic",
            )?;
            require_bool(
                "raw_decoded_tree_render_blocker.canonical_render_reused_from_checked_projection",
                self.canonical_render_reused_from_checked_projection,
                false,
            )?;
        } else {
            require_string(
                "raw_decoded_tree_render_blocker.comment_trivia_policy_status",
                &self.comment_trivia_policy_status,
                "raw_tree_node_values_ready_comment_anchor_policy_blocked",
            )?;
            require_string(
                "raw_decoded_tree_render_blocker.renderer_policy_status",
                &self.renderer_policy_status,
                "blocked_on_comment_anchor_and_reference_policy",
            )?;
            require_bool(
                "raw_decoded_tree_render_blocker.canonical_render_reused_from_checked_projection",
                self.canonical_render_reused_from_checked_projection,
                true,
            )?;
        }
        require_bool(
            "raw_decoded_tree_render_blocker.raw_tree_grammar_mode_render_ready",
            self.raw_tree_grammar_mode_render_ready,
            false,
        )?;
        require_bool(
            "raw_decoded_tree_render_blocker.raw_tree_canonical_fidelity_ready",
            self.raw_tree_canonical_fidelity_ready,
            false,
        )?;
        require_bool(
            "raw_decoded_tree_render_blocker.essential_comment_policy_ready",
            self.essential_comment_policy_ready,
            false,
        )?;
        require_string(
            "raw_decoded_tree_render_blocker.essential_comment_policy_status",
            &self.essential_comment_policy_status,
            RAW_TREE_ESSENTIAL_COMMENT_POLICY_NOT_EVALUATED_STATUS,
        )?;
        require_usize(
            "raw_decoded_tree_render_blocker.essential_comment_count",
            self.essential_comment_count,
            0,
        )?;
        if !self.essential_comment_categories.is_empty() {
            return Err(SyntaxBackendBoundaryError::new(
                "raw_decoded_tree_render_blocker.essential_comment_categories",
                "blocked raw decoded-tree render evidence must not carry essential comment categories",
            ));
        }
        require_bool(
            "raw_decoded_tree_render_blocker.raw_tree_canonical_render_ready",
            self.raw_tree_canonical_render_ready,
            false,
        )?;
        require_none(
            "raw_decoded_tree_render_blocker.raw_tree_rendered_source_sha256",
            self.raw_tree_rendered_source_sha256.as_deref(),
        )?;
        if self.raw_tree_rendered_source_byte_count.is_some() {
            return Err(SyntaxBackendBoundaryError::new(
                "raw_decoded_tree_render_blocker.raw_tree_rendered_source_byte_count",
                "blocked raw decoded-tree render evidence must not carry rendered source bytes",
            ));
        }
        require_none(
            "raw_decoded_tree_render_blocker.raw_tree_formatted_source_sha256",
            self.raw_tree_formatted_source_sha256.as_deref(),
        )?;
        if self.raw_tree_formatted_source_byte_count.is_some() {
            return Err(SyntaxBackendBoundaryError::new(
                "raw_decoded_tree_render_blocker.raw_tree_formatted_source_byte_count",
                "blocked raw decoded-tree render evidence must not carry formatted source bytes",
            ));
        }
        require_bool(
            "raw_decoded_tree_render_blocker.raw_tree_render_matches_canonical_source",
            self.raw_tree_render_matches_canonical_source,
            false,
        )?;
        require_none(
            "raw_decoded_tree_render_blocker.canonical_fidelity_diff_summary",
            self.canonical_fidelity_diff_summary.as_deref(),
        )?;
        require_none(
            "raw_decoded_tree_render_blocker.gofmt_validation",
            self.gofmt_validation.as_deref(),
        )?;
        require_none(
            "raw_decoded_tree_render_blocker.go_parser_reparse_validation",
            self.go_parser_reparse_validation.as_deref(),
        )?;
        if self.raw_tree_render_attempted {
            let diagnostic = self.renderer_diagnostic.as_ref().ok_or_else(|| {
                SyntaxBackendBoundaryError::new(
                    "raw_decoded_tree_render_blocker.renderer_diagnostic",
                    "blocked renderer attempt must carry a structured diagnostic",
                )
            })?;
            require_contains(
                "raw_decoded_tree_render_blocker.missing_capabilities",
                &self.missing_capabilities,
                &diagnostic.missing_capability,
            )?;
            require_string(
                "raw_decoded_tree_render_blocker.next_ready_task",
                &self.next_ready_task,
                RAW_DECODED_TREE_RENDER_NEXT_READY_TASK,
            )?;
            require_str_contains(
                "raw_decoded_tree_render_blocker.claim_boundary",
                &self.claim_boundary,
                "syntax-level",
            )?;
            require_str_contains(
                "raw_decoded_tree_render_blocker.claim_boundary",
                &self.claim_boundary,
                "structured data",
            )?;
            require_str_contains(
                "raw_decoded_tree_render_blocker.claim_boundary",
                &self.claim_boundary,
                "without falling back to checked projection canonical-render data",
            )?;
        } else {
            require_all(
                "raw_decoded_tree_render_blocker.missing_capabilities",
                &self.missing_capabilities,
                RAW_DECODED_TREE_RENDER_MISSING_CAPABILITIES,
            )?;
            require_string(
                "raw_decoded_tree_render_blocker.next_ready_task",
                &self.next_ready_task,
                RAW_DECODED_TREE_RENDER_BLOCKER_NEXT_READY_TASK,
            )?;
            require_str_contains(
                "raw_decoded_tree_render_blocker.claim_boundary",
                &self.claim_boundary,
                "syntax-level",
            )?;
            require_str_contains(
                "raw_decoded_tree_render_blocker.claim_boundary",
                &self.claim_boundary,
                "payload values materialized",
            )?;
            require_str_contains(
                "raw_decoded_tree_render_blocker.claim_boundary",
                &self.claim_boundary,
                "comment/trivia position anchors",
            )?;
        }
        require_str_contains(
            "raw_decoded_tree_render_blocker.claim_boundary",
            &self.claim_boundary,
            "not Arena IR",
        )?;
        require_str_contains(
            "raw_decoded_tree_render_blocker.claim_boundary",
            &self.claim_boundary,
            "not BPE",
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoTokenizerGeneratorDeclarationPresenceEvidence {
    pub status: String,
    pub raw_replay_artifact_kind: String,
    pub raw_replay_next_ready_task: String,
    pub raw_replay_decisions_consumed: bool,
    pub raw_replay_raw_token_ids_sha256: String,
    pub generated_render_decisions_preserved: bool,
    pub render_reparse_checks_ready: bool,
    pub presence_decisions: Vec<GoTokenizerGeneratorDeclarationPresenceDecision>,
    pub generated_evidence: Vec<GoTokenizerGeneratorDeclarationGeneratedEvidence>,
    pub next_ready_task: String,
    pub claim_boundary: String,
}

impl GoTokenizerGeneratorDeclarationPresenceEvidence {
    fn validate(&self) -> Result<(), SyntaxBackendBoundaryError> {
        require_string(
            "declaration_presence.status",
            &self.status,
            DECLARATION_PRESENCE_STATUS,
        )?;
        require_string(
            "declaration_presence.raw_replay_artifact_kind",
            &self.raw_replay_artifact_kind,
            GO_AST_DECLARATION_PRESENCE_REPLAY_ARTIFACT_KIND,
        )?;
        require_string(
            "declaration_presence.raw_replay_next_ready_task",
            &self.raw_replay_next_ready_task,
            GO_AST_DECLARATION_PRESENCE_REPLAY_NEXT_READY_TASK,
        )?;
        require_bool(
            "declaration_presence.raw_replay_decisions_consumed",
            self.raw_replay_decisions_consumed,
            true,
        )?;
        require_sha256(
            "declaration_presence.raw_replay_raw_token_ids_sha256",
            &self.raw_replay_raw_token_ids_sha256,
        )?;
        require_bool(
            "declaration_presence.generated_render_decisions_preserved",
            self.generated_render_decisions_preserved,
            true,
        )?;
        require_bool(
            "declaration_presence.render_reparse_checks_ready",
            self.render_reparse_checks_ready,
            true,
        )?;
        require_usize(
            "declaration_presence.presence_decisions",
            self.presence_decisions.len(),
            5,
        )?;
        let mut observed = BTreeSet::new();
        for decision in &self.presence_decisions {
            decision.validate()?;
            observed.insert(format!("{}.{}", decision.owner_rule, decision.owner_field));
        }
        for expected in [
            "GenDecl.Lparen",
            "GenDecl.Rparen",
            "TypeSpec.Assign",
            "FuncType.Func",
            "CallExpr.Ellipsis",
        ] {
            if !observed.contains(expected) {
                return Err(SyntaxBackendBoundaryError::new(
                    "declaration_presence.presence_decisions",
                    format!("missing declaration presence decision {expected}"),
                ));
            }
        }
        require_usize(
            "declaration_presence.generated_evidence",
            self.generated_evidence.len(),
            4,
        )?;
        let mut generated = BTreeSet::new();
        for evidence in &self.generated_evidence {
            evidence.validate()?;
            generated.insert(evidence.grammar_id.as_str());
        }
        for expected in [
            "go_type_spec_assign_syntax_v0",
            "go_type_spec_doc_syntax_v0",
            "go_function_literal_syntax_v0",
            "go_local_variadic_ellipsis_syntax_v0",
        ] {
            if !generated.contains(expected) {
                return Err(SyntaxBackendBoundaryError::new(
                    "declaration_presence.generated_evidence",
                    format!("missing generated declaration evidence {expected}"),
                ));
            }
        }
        require_string(
            "declaration_presence.next_ready_task",
            &self.next_ready_task,
            NEXT_READY_TASK,
        )?;
        require_str_contains(
            "declaration_presence.claim_boundary",
            &self.claim_boundary,
            "syntax-level",
        )?;
        require_str_contains(
            "declaration_presence.claim_boundary",
            &self.claim_boundary,
            "not Arena IR",
        )?;
        require_str_contains(
            "declaration_presence.claim_boundary",
            &self.claim_boundary,
            "not BPE",
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoTokenizerGeneratorDeclarationPresenceDecision {
    pub owner_rule: String,
    pub owner_field: String,
    pub domain_id: String,
    pub selected_node_kind: String,
    pub boolean_payload_kind: String,
    pub generated_fixture_grammar_id: String,
    pub generated_fixture_metadata_path: String,
    pub generated_presence_test_marker: String,
    pub generated_render_test_marker: String,
    pub generated_reparse_test_marker: String,
    pub generated_render_preserves_presence: bool,
    pub generated_reparse_check_present: bool,
}

impl GoTokenizerGeneratorDeclarationPresenceDecision {
    fn validate(&self) -> Result<(), SyntaxBackendBoundaryError> {
        require_non_empty("declaration_presence.decision.owner_rule", &self.owner_rule)?;
        require_non_empty(
            "declaration_presence.decision.owner_field",
            &self.owner_field,
        )?;
        require_non_empty("declaration_presence.decision.domain_id", &self.domain_id)?;
        require_str_contains(
            "declaration_presence.decision.selected_node_kind",
            &self.selected_node_kind,
            ".PRESENT",
        )?;
        require_string(
            "declaration_presence.decision.boolean_payload_kind",
            &self.boolean_payload_kind,
            "boolean",
        )?;
        require_non_empty(
            "declaration_presence.decision.generated_fixture_grammar_id",
            &self.generated_fixture_grammar_id,
        )?;
        require_non_empty(
            "declaration_presence.decision.generated_fixture_metadata_path",
            &self.generated_fixture_metadata_path,
        )?;
        require_non_empty(
            "declaration_presence.decision.generated_presence_test_marker",
            &self.generated_presence_test_marker,
        )?;
        require_non_empty(
            "declaration_presence.decision.generated_render_test_marker",
            &self.generated_render_test_marker,
        )?;
        require_non_empty(
            "declaration_presence.decision.generated_reparse_test_marker",
            &self.generated_reparse_test_marker,
        )?;
        require_bool(
            "declaration_presence.decision.generated_render_preserves_presence",
            self.generated_render_preserves_presence,
            true,
        )?;
        require_bool(
            "declaration_presence.decision.generated_reparse_check_present",
            self.generated_reparse_check_present,
            true,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoTokenizerGeneratorDeclarationGeneratedEvidence {
    pub grammar_id: String,
    pub metadata_path: String,
    pub generated_crate_name: String,
    pub presence_test_marker: String,
    pub render_test_marker: String,
    pub reparse_test_marker: String,
    pub expected_source_snippet: String,
    pub rendered_source_sha256: String,
    pub generated_source_contains_presence_test: bool,
    pub generated_source_contains_render_test: bool,
    pub generated_source_contains_reparse_test: bool,
    pub generated_source_contains_expected_snippet: bool,
    pub registry_marker_consumed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoTokenizerGeneratorStateMaskSelectionEvidence {
    pub status: String,
    pub scope: String,
    pub replay_artifact_kind: String,
    pub generated_fixture_grammar_id: String,
    pub generated_fixture_metadata_path: String,
    pub owner_rule: String,
    pub owner_field: String,
    pub domain_id: String,
    pub selected_node_kind: String,
    pub boolean_payload_kind: String,
    pub terminal_raw_token_offset: usize,
    pub selected_raw_token_offset: usize,
    pub selected_raw_token_id: u32,
    pub finite_admissible_raw_token_ids: Vec<u32>,
    pub selected_raw_token_admitted_by_mask: bool,
    pub decoded_selected_raw_token: bool,
    pub complete_stream_decoded: bool,
    pub generated_prefix_state_marker: String,
    pub generated_render_marker: String,
    pub generated_reparse_marker: String,
    pub generated_prefix_state_marker_consumed: bool,
    pub generated_render_reparse_ready: bool,
    pub expected_source_snippet: String,
    pub generated_source_contains_expected_snippet: bool,
    pub arena_ir_used: bool,
    pub route_handoff_provenance_used: bool,
    pub bpe_validity_authority_used: bool,
    pub stable_backend_claimed: bool,
    pub claim_boundary: String,
}

impl GoTokenizerGeneratorStateMaskSelectionEvidence {
    fn validate(&self, token_layout_vocab_size: u32) -> Result<(), SyntaxBackendBoundaryError> {
        require_string(
            "state_mask_selection.status",
            &self.status,
            STATE_MASK_SELECTION_STATUS,
        )?;
        require_string(
            "state_mask_selection.scope",
            &self.scope,
            STATE_MASK_SELECTION_SCOPE,
        )?;
        require_string(
            "state_mask_selection.replay_artifact_kind",
            &self.replay_artifact_kind,
            GO_AST_DECLARATION_PRESENCE_REPLAY_ARTIFACT_KIND,
        )?;
        require_string(
            "state_mask_selection.generated_fixture_grammar_id",
            &self.generated_fixture_grammar_id,
            "go_type_spec_assign_syntax_v0",
        )?;
        require_string(
            "state_mask_selection.generated_fixture_metadata_path",
            &self.generated_fixture_metadata_path,
            GO_TYPE_SPEC_ASSIGN_METADATA_PATH,
        )?;
        require_string(
            "state_mask_selection.owner_rule",
            &self.owner_rule,
            "TypeSpec",
        )?;
        require_string(
            "state_mask_selection.owner_field",
            &self.owner_field,
            "Assign",
        )?;
        require_non_empty("state_mask_selection.domain_id", &self.domain_id)?;
        require_string(
            "state_mask_selection.selected_node_kind",
            &self.selected_node_kind,
            "go_syntax_presence_type_spec_alias.PRESENT",
        )?;
        require_string(
            "state_mask_selection.boolean_payload_kind",
            &self.boolean_payload_kind,
            "boolean",
        )?;
        if self.selected_raw_token_offset != self.terminal_raw_token_offset + 1 {
            return Err(SyntaxBackendBoundaryError::new(
                "state_mask_selection.selected_raw_token_offset",
                "selected raw token must immediately follow the terminal token",
            ));
        }
        require_positive_usize(
            "state_mask_selection.finite_admissible_raw_token_ids",
            self.finite_admissible_raw_token_ids.len(),
        )?;
        if self.selected_raw_token_id >= token_layout_vocab_size {
            return Err(SyntaxBackendBoundaryError::new(
                "state_mask_selection.selected_raw_token_id",
                "selected raw token must remain below the generator token layout VOCAB_SIZE",
            ));
        }
        if !self
            .finite_admissible_raw_token_ids
            .contains(&self.selected_raw_token_id)
        {
            return Err(SyntaxBackendBoundaryError::new(
                "state_mask_selection.finite_admissible_raw_token_ids",
                "finite admissible-token mask must contain the selected raw token",
            ));
        }
        require_bool(
            "state_mask_selection.selected_raw_token_admitted_by_mask",
            self.selected_raw_token_admitted_by_mask,
            true,
        )?;
        require_bool(
            "state_mask_selection.decoded_selected_raw_token",
            self.decoded_selected_raw_token,
            true,
        )?;
        require_bool(
            "state_mask_selection.complete_stream_decoded",
            self.complete_stream_decoded,
            true,
        )?;
        require_string(
            "state_mask_selection.generated_prefix_state_marker",
            &self.generated_prefix_state_marker,
            GO_TYPE_SPEC_ASSIGN_PREFIX_STATE_TEST_MARKER,
        )?;
        require_string(
            "state_mask_selection.generated_render_marker",
            &self.generated_render_marker,
            GO_TYPE_SPEC_ASSIGN_RENDER_TEST_MARKER,
        )?;
        require_string(
            "state_mask_selection.generated_reparse_marker",
            &self.generated_reparse_marker,
            GO_TYPE_SPEC_ASSIGN_REPARSE_TEST_MARKER,
        )?;
        require_bool(
            "state_mask_selection.generated_prefix_state_marker_consumed",
            self.generated_prefix_state_marker_consumed,
            true,
        )?;
        require_bool(
            "state_mask_selection.generated_render_reparse_ready",
            self.generated_render_reparse_ready,
            true,
        )?;
        require_string(
            "state_mask_selection.expected_source_snippet",
            &self.expected_source_snippet,
            GO_TYPE_SPEC_ASSIGN_EXPECTED_SOURCE_SNIPPET,
        )?;
        require_bool(
            "state_mask_selection.generated_source_contains_expected_snippet",
            self.generated_source_contains_expected_snippet,
            true,
        )?;
        require_bool(
            "state_mask_selection.arena_ir_used",
            self.arena_ir_used,
            false,
        )?;
        require_bool(
            "state_mask_selection.route_handoff_provenance_used",
            self.route_handoff_provenance_used,
            false,
        )?;
        require_bool(
            "state_mask_selection.bpe_validity_authority_used",
            self.bpe_validity_authority_used,
            false,
        )?;
        require_bool(
            "state_mask_selection.stable_backend_claimed",
            self.stable_backend_claimed,
            false,
        )?;
        require_str_contains(
            "state_mask_selection.claim_boundary",
            &self.claim_boundary,
            "syntax-level",
        )?;
        require_str_contains(
            "state_mask_selection.claim_boundary",
            &self.claim_boundary,
            "not Arena IR",
        )?;
        require_str_contains(
            "state_mask_selection.claim_boundary",
            &self.claim_boundary,
            "not BPE",
        )
    }
}

impl GoTokenizerGeneratorDeclarationGeneratedEvidence {
    fn validate(&self) -> Result<(), SyntaxBackendBoundaryError> {
        require_non_empty(
            "declaration_presence.generated.grammar_id",
            &self.grammar_id,
        )?;
        require_non_empty(
            "declaration_presence.generated.metadata_path",
            &self.metadata_path,
        )?;
        require_non_empty(
            "declaration_presence.generated.generated_crate_name",
            &self.generated_crate_name,
        )?;
        require_non_empty(
            "declaration_presence.generated.presence_test_marker",
            &self.presence_test_marker,
        )?;
        require_non_empty(
            "declaration_presence.generated.render_test_marker",
            &self.render_test_marker,
        )?;
        require_non_empty(
            "declaration_presence.generated.reparse_test_marker",
            &self.reparse_test_marker,
        )?;
        require_non_empty(
            "declaration_presence.generated.expected_source_snippet",
            &self.expected_source_snippet,
        )?;
        require_sha256(
            "declaration_presence.generated.rendered_source_sha256",
            &self.rendered_source_sha256,
        )?;
        require_bool(
            "declaration_presence.generated.generated_source_contains_presence_test",
            self.generated_source_contains_presence_test,
            true,
        )?;
        require_bool(
            "declaration_presence.generated.generated_source_contains_render_test",
            self.generated_source_contains_render_test,
            true,
        )?;
        require_bool(
            "declaration_presence.generated.generated_source_contains_reparse_test",
            self.generated_source_contains_reparse_test,
            true,
        )?;
        require_bool(
            "declaration_presence.generated.generated_source_contains_expected_snippet",
            self.generated_source_contains_expected_snippet,
            true,
        )?;
        require_bool(
            "declaration_presence.generated.registry_marker_consumed",
            self.registry_marker_consumed,
            true,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoTokenizerGeneratorPipeline {
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

impl GoTokenizerGeneratorPipeline {
    /// Map a generator-core full-pipeline field stamp into the report pipeline
    /// surface (single stamp → single construction, no dual-site composition).
    fn from_field_stamp(stamp: GoSourceRecordPipelineFieldStamp) -> Self {
        Self {
            parser_backed_metadata_loaded: stamp.parser_backed_metadata_loaded,
            grammar_ir_lowered: stamp.grammar_ir_lowered,
            structural_model_lowered: stamp.structural_model_lowered,
            token_layout_compiled: stamp.token_layout_compiled,
            raw_state_machine_compiled: stamp.raw_state_machine_compiled,
            raw_token_encode_ready: stamp.raw_token_encode_ready,
            raw_state_machine_decode_ready: stamp.raw_state_machine_decode_ready,
            canonical_render_ready: stamp.canonical_render_ready,
            go_reparse_ready: stamp.go_reparse_ready,
            arena_ir_used: stamp.arena_ir_used,
            route_handoff_provenance_used: stamp.route_handoff_provenance_used,
            bpe_validity_authority_used: stamp.bpe_validity_authority_used,
        }
    }

    fn validate(&self) -> Result<(), SyntaxBackendBoundaryError> {
        // Process-only compile stages + exclusions come from the generator-core
        // constant bundles (not ad-hoc true/false literals at each site).
        GoSourceRecordPipelineProcessStageClaims::validate_against_success_bundle(
            self.parser_backed_metadata_loaded,
            self.grammar_ir_lowered,
            self.structural_model_lowered,
            self.token_layout_compiled,
            self.raw_state_machine_compiled,
        )?;
        require_bool(
            "pipeline.raw_token_encode_ready",
            self.raw_token_encode_ready,
            true,
        )?;
        require_bool(
            "pipeline.raw_state_machine_decode_ready",
            self.raw_state_machine_decode_ready,
            true,
        )?;
        require_bool(
            "pipeline.canonical_render_ready",
            self.canonical_render_ready,
            true,
        )?;
        require_bool("pipeline.go_reparse_ready", self.go_reparse_ready, true)?;
        GoSourceRecordPipelineProcessExclusionClaims::validate_against_success_bundle(
            self.arena_ir_used,
            self.route_handoff_provenance_used,
            self.bpe_validity_authority_used,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoTokenizerGeneratorArtifacts {
    pub report: GoTokenizerGeneratorReport,
    pub generated_codec_report: SourceInstanceGeneratedCodecReport,
    pub replay_report: SyntaxStateMachineReplayReport,
    /// Retained source-record generator-core evidence so manifest-depth record
    /// construction stamps API/stream + empty-prefix / selected-completion /
    /// elided selected-completion / incomplete-prefix / token-only fields from
    /// evidence rather than only re-cloning the absorbed report surface.
    pub source_record_generation: GoSourceRecordGeneratorEvidence,
}

#[derive(Debug, Clone, Copy)]
pub struct GoTokenizerGeneratorManifestDepthReportInput<'a> {
    pub checked_projection_report_path: &'a Path,
    pub artifacts: &'a GoTokenizerGeneratorArtifacts,
}

#[derive(Debug, Clone, Copy)]
pub struct GoRawTreeAstPrinterOracleCutoverReportInput<'a> {
    pub report_path: &'a Path,
    pub report_contents: &'a str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoRawTreeAstPrinterOracleCutoverDecision {
    pub status: String,
    pub accepted: bool,
    pub record_count: usize,
    pub canonical_fidelity_only_gap_count: usize,
    pub next_ready_task: String,
    pub claim_boundary: String,
}

impl GoRawTreeAstPrinterOracleCutoverDecision {
    pub fn validate(&self) -> Result<(), SyntaxBackendBoundaryError> {
        require_string(
            "go_raw_tree_ast_printer_oracle_cutover.status",
            &self.status,
            GO_RAW_TREE_AST_PRINTER_ORACLE_CUTOVER_STATUS_ACCEPTED,
        )?;
        require_bool(
            "go_raw_tree_ast_printer_oracle_cutover.accepted",
            self.accepted,
            true,
        )?;
        require_positive_usize(
            "go_raw_tree_ast_printer_oracle_cutover.record_count",
            self.record_count,
        )?;
        require_string(
            "go_raw_tree_ast_printer_oracle_cutover.next_ready_task",
            &self.next_ready_task,
            GO_RAW_TREE_AST_PRINTER_ORACLE_CUTOVER_NEXT_READY_TASK,
        )?;
        require_str_contains(
            "go_raw_tree_ast_printer_oracle_cutover.claim_boundary",
            &self.claim_boundary,
            "source fidelity",
        )?;
        require_str_contains(
            "go_raw_tree_ast_printer_oracle_cutover.claim_boundary",
            &self.claim_boundary,
            "diagnostic",
        )?;
        require_str_contains(
            "go_raw_tree_ast_printer_oracle_cutover.claim_boundary",
            &self.claim_boundary,
            "not generated-hot-path",
        )
    }
}

#[derive(Debug, Clone, Deserialize)]
struct GoRawTreeAstPrinterOracleCutoverReport {
    artifact_kind: Option<String>,
    version: Option<u32>,
    tool: Option<String>,
    status: Option<String>,
    mode: Option<String>,
    record_count: Option<usize>,
    structural_blocked_count: Option<usize>,
    comment_directive_blocked_count: Option<usize>,
    full_adoption_possible: Option<bool>,
    structural_blockers: Option<Vec<serde_json::Value>>,
    essential_comment_blockers: Option<Vec<serde_json::Value>>,
    parser_blockers: Option<Vec<serde_json::Value>>,
    canonical_fidelity_only_gaps: Option<Vec<serde_json::Value>>,
    records: Option<Vec<serde_json::Value>>,
    decision: Option<GoRawTreeAstPrinterOracleCutoverReportDecision>,
    claim_boundary: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct GoRawTreeAstPrinterOracleCutoverReportDecision {
    recommendation: Option<String>,
}

impl GoRawTreeAstPrinterOracleCutoverReport {
    fn cutover_decision(
        &self,
    ) -> Result<GoRawTreeAstPrinterOracleCutoverDecision, SyntaxBackendBoundaryError> {
        let artifact_kind =
            required_oracle_cutover_field("oracle_report.artifact_kind", &self.artifact_kind)?;
        require_string(
            "oracle_report.artifact_kind",
            &artifact_kind,
            GO_RAW_TREE_AST_PRINTER_ORACLE_REPORT_ARTIFACT_KIND,
        )?;
        let version = required_oracle_cutover_field("oracle_report.version", &self.version)?;
        require_u32(
            "oracle_report.version",
            version,
            GO_RAW_TREE_AST_PRINTER_ORACLE_REPORT_VERSION,
        )?;
        let tool = required_oracle_cutover_field("oracle_report.tool", &self.tool)?;
        require_string(
            "oracle_report.tool",
            &tool,
            GO_RAW_TREE_AST_PRINTER_ORACLE_REPORT_TOOL,
        )?;
        let status = required_oracle_cutover_field("oracle_report.status", &self.status)?;
        require_string(
            "oracle_report.status",
            &status,
            GO_RAW_TREE_AST_PRINTER_ORACLE_REPORT_STATUS_PASSED,
        )?;
        let mode = required_oracle_cutover_field("oracle_report.mode", &self.mode)?;
        require_string(
            "oracle_report.mode",
            &mode,
            GO_RAW_TREE_AST_PRINTER_ORACLE_REPORT_MODE_GRAMMAR,
        )?;

        let record_count =
            required_oracle_cutover_field("oracle_report.record_count", &self.record_count)?;
        require_positive_usize("oracle_report.record_count", record_count)?;
        let records = required_oracle_cutover_field("oracle_report.records", &self.records)?;
        require_usize("oracle_report.records", records.len(), record_count)?;

        let structural_blocked_count = required_oracle_cutover_field(
            "oracle_report.structural_blocked_count",
            &self.structural_blocked_count,
        )?;
        let comment_directive_blocked_count = required_oracle_cutover_field(
            "oracle_report.comment_directive_blocked_count",
            &self.comment_directive_blocked_count,
        )?;
        let structural_blockers = required_oracle_cutover_field(
            "oracle_report.structural_blockers",
            &self.structural_blockers,
        )?;
        let essential_comment_blockers = required_oracle_cutover_field(
            "oracle_report.essential_comment_blockers",
            &self.essential_comment_blockers,
        )?;
        let parser_blockers =
            required_oracle_cutover_field("oracle_report.parser_blockers", &self.parser_blockers)?;
        let canonical_fidelity_only_gaps = required_oracle_cutover_field(
            "oracle_report.canonical_fidelity_only_gaps",
            &self.canonical_fidelity_only_gaps,
        )?;
        let full_adoption_possible = required_oracle_cutover_field(
            "oracle_report.full_adoption_possible",
            &self.full_adoption_possible,
        )?;
        let decision = required_oracle_cutover_field("oracle_report.decision", &self.decision)?;
        let recommendation = required_oracle_cutover_field(
            "oracle_report.decision.recommendation",
            &decision.recommendation,
        )?;
        require_string(
            "oracle_report.decision.recommendation",
            &recommendation,
            GO_RAW_TREE_AST_PRINTER_ORACLE_REPORT_RECOMMENDATION_ACCEPTED,
        )?;
        let claim_boundary =
            required_oracle_cutover_field("oracle_report.claim_boundary", &self.claim_boundary)?;
        require_str_contains(
            "oracle_report.claim_boundary",
            &claim_boundary,
            "grammar-mode",
        )?;
        require_str_contains(
            "oracle_report.claim_boundary",
            &claim_boundary,
            "source fidelity remains diagnostic",
        )?;
        require_str_contains(
            "oracle_report.claim_boundary",
            &claim_boundary,
            "historical/baseline diagnostic",
        )?;
        require_str_contains("oracle_report.claim_boundary", &claim_boundary, "not BPE")?;
        require_str_contains(
            "oracle_report.claim_boundary",
            &claim_boundary,
            "not Arena IR",
        )?;

        if structural_blocked_count != 0 || !structural_blockers.is_empty() {
            return Err(SyntaxBackendBoundaryError::new(
                "go_raw_tree_ast_printer_oracle_cutover.structural_blockers",
                "oracle cutover requires zero structural/grammar materialization blockers",
            ));
        }
        if comment_directive_blocked_count != 0 || !essential_comment_blockers.is_empty() {
            return Err(SyntaxBackendBoundaryError::new(
                "go_raw_tree_ast_printer_oracle_cutover.essential_comment_blockers",
                "oracle cutover requires zero essential-comment or directive blockers",
            ));
        }
        if !parser_blockers.is_empty() {
            return Err(SyntaxBackendBoundaryError::new(
                "go_raw_tree_ast_printer_oracle_cutover.parser_blockers",
                "oracle cutover requires zero go/format or go/parser blockers",
            ));
        }
        if !full_adoption_possible {
            return Err(SyntaxBackendBoundaryError::new(
                "go_raw_tree_ast_printer_oracle_cutover.full_adoption_possible",
                "oracle cutover fails closed unless full_adoption_possible is true",
            ));
        }

        let decision = GoRawTreeAstPrinterOracleCutoverDecision {
            status: GO_RAW_TREE_AST_PRINTER_ORACLE_CUTOVER_STATUS_ACCEPTED.to_string(),
            accepted: true,
            record_count,
            canonical_fidelity_only_gap_count: canonical_fidelity_only_gaps.len(),
            next_ready_task: GO_RAW_TREE_AST_PRINTER_ORACLE_CUTOVER_NEXT_READY_TASK.to_string(),
            claim_boundary,
        };
        decision.validate()?;
        Ok(decision)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoTokenizerGeneratorManifestDepthOracleCutoverEvidence {
    pub report_path: String,
    pub report_sha256: String,
    pub status: String,
    pub accepted: bool,
    pub record_count: usize,
    pub canonical_fidelity_only_gap_count: usize,
    pub claim_boundary: String,
}

impl GoTokenizerGeneratorManifestDepthOracleCutoverEvidence {
    /// Borrowed attachment view for generator-core scratch-generation gate assembly.
    pub fn attachment_view(&self) -> GoSourceRecordOracleCutoverAttachmentView<'_> {
        GoSourceRecordOracleCutoverAttachmentView {
            report_path: &self.report_path,
            report_sha256: &self.report_sha256,
            status: &self.status,
            accepted: self.accepted,
            record_count: self.record_count,
            canonical_fidelity_only_gap_count: self.canonical_fidelity_only_gap_count,
            claim_boundary: &self.claim_boundary,
        }
    }

    fn from_input(
        input: GoRawTreeAstPrinterOracleCutoverReportInput<'_>,
    ) -> Result<Self, SyntaxBackendBoundaryError> {
        let decision =
            parse_go_raw_tree_ast_printer_oracle_cutover_decision(input.report_contents)?;
        let evidence = Self {
            report_path: input.report_path.display().to_string(),
            report_sha256: sha256_hex(input.report_contents.as_bytes()),
            status: decision.status,
            accepted: decision.accepted,
            record_count: decision.record_count,
            canonical_fidelity_only_gap_count: decision.canonical_fidelity_only_gap_count,
            claim_boundary: decision.claim_boundary,
        };
        evidence.validate_common()?;
        Ok(evidence)
    }

    fn validate(
        &self,
        summary: &GoTokenizerGeneratorManifestDepthRawTreeRenderAttemptSummary,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        self.validate_common()?;
        require_usize(
            "go_raw_tree_ast_printer_oracle_cutover.record_count",
            self.record_count,
            summary.manifest_selected_attempt_count,
        )
    }

    fn validate_common(&self) -> Result<(), SyntaxBackendBoundaryError> {
        require_non_empty(
            "go_raw_tree_ast_printer_oracle_cutover.report_path",
            &self.report_path,
        )?;
        require_sha256(
            "go_raw_tree_ast_printer_oracle_cutover.report_sha256",
            &self.report_sha256,
        )?;
        require_string(
            "go_raw_tree_ast_printer_oracle_cutover.status",
            &self.status,
            GO_RAW_TREE_AST_PRINTER_ORACLE_CUTOVER_STATUS_ACCEPTED,
        )?;
        require_bool(
            "go_raw_tree_ast_printer_oracle_cutover.accepted",
            self.accepted,
            true,
        )?;
        require_positive_usize(
            "go_raw_tree_ast_printer_oracle_cutover.record_count",
            self.record_count,
        )?;
        require_str_contains(
            "go_raw_tree_ast_printer_oracle_cutover.claim_boundary",
            &self.claim_boundary,
            "grammar-mode",
        )?;
        require_str_contains(
            "go_raw_tree_ast_printer_oracle_cutover.claim_boundary",
            &self.claim_boundary,
            "source fidelity remains diagnostic",
        )?;
        require_str_contains(
            "go_raw_tree_ast_printer_oracle_cutover.claim_boundary",
            &self.claim_boundary,
            "not BPE",
        )?;
        require_str_contains(
            "go_raw_tree_ast_printer_oracle_cutover.claim_boundary",
            &self.claim_boundary,
            "not Arena IR",
        )
    }
}

/// Thin megamodule adapter: map oracle evidence + records into generator-core
/// views and assemble/validate
/// [`GoTokenizerGeneratorManifestDepthScratchGenerationGate`].
fn assemble_scratch_generation_gate(
    oracle: &GoTokenizerGeneratorManifestDepthOracleCutoverEvidence,
    grammar_coverage: &GoAstSchemaGrammarCoverageReport,
    records: &[GoTokenizerGeneratorManifestDepthRecord],
) -> Result<GoTokenizerGeneratorManifestDepthScratchGenerationGate, SyntaxBackendBoundaryError> {
    let representative = records.first().ok_or_else(|| {
        SyntaxBackendBoundaryError::new(
            "manifest_depth.scratch_generation_gate.records",
            "scratch-generation gate requires at least one manifest-depth record",
        )
    })?;
    GoTokenizerGeneratorManifestDepthScratchGenerationGate::from_oracle_and_record_views(
        oracle.attachment_view(),
        grammar_coverage,
        records
            .iter()
            .map(GoTokenizerGeneratorManifestDepthRecord::readiness_view),
        representative.representative_view(),
    )
}

fn validate_scratch_generation_gate(
    gate: &GoTokenizerGeneratorManifestDepthScratchGenerationGate,
    oracle: &GoTokenizerGeneratorManifestDepthOracleCutoverEvidence,
    grammar_coverage: &GoAstSchemaGrammarCoverageReport,
    records: &[GoTokenizerGeneratorManifestDepthRecord],
) -> Result<(), SyntaxBackendBoundaryError> {
    let representative = records.first().ok_or_else(|| {
        SyntaxBackendBoundaryError::new(
            "manifest_depth.scratch_generation_gate.records",
            "scratch-generation gate requires at least one manifest-depth record",
        )
    })?;
    gate.validate(
        oracle.attachment_view(),
        grammar_coverage,
        records
            .iter()
            .map(GoTokenizerGeneratorManifestDepthRecord::readiness_view),
        representative.representative_view(),
    )
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoAstInventoryTemplateCoverageFrontierReport {
    pub artifact_kind: String,
    pub version: u32,
    pub status: String,
    pub grammar_id: String,
    pub inventory_artifact_path: String,
    pub inventory_artifact_sha256: String,
    pub template_api_id: String,
    pub template_api_boundary: String,
    pub supported_template_ids: Vec<String>,
    pub supported_declaration_template_kinds: Vec<String>,
    pub supported_expression_template_kinds: Vec<String>,
    pub template_backed_synthesis_api_ids: Vec<String>,
    pub template_backed_witness_ids: Vec<String>,
    pub inventory_rule_count: usize,
    pub inventory_field_count: usize,
    pub inventory_required_field_count: usize,
    pub inventory_optional_field_count: usize,
    pub inventory_repeated_field_count: usize,
    pub inventory_finite_domain_field_count: usize,
    pub covered_rule_count: usize,
    pub covered_rules: Vec<String>,
    pub covered_field_count: usize,
    pub covered_fields: Vec<String>,
    pub covered_finite_domain_choice_count: usize,
    pub covered_finite_domain_choices: Vec<GoAstInventoryTemplateFiniteDomainChoice>,
    pub unsupported_optional_field_policy_count: usize,
    pub unsupported_repeated_field_policy_count: usize,
    pub first_unsupported_repeated_or_optional_policy: Option<GoAstInventoryTemplatePolicyGap>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_missing_template_capability: Option<GoAstInventoryTemplateCoverageBlocker>,
    pub next_ready_task: String,
    pub validation_commands: Vec<String>,
    pub claim_boundary: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoAstInventoryTemplateFiniteDomainChoice {
    pub rule_name: String,
    pub field_name: String,
    pub alternative: String,
    pub terminal_kind: GrammarTerminalKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoAstInventoryTemplatePolicyGap {
    pub field_family: String,
    pub policy: String,
    pub reason: String,
    pub next_template_capability: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoAstInventoryTemplateCoverageBlocker {
    pub kind: String,
    pub rule_name: String,
    pub field_name: String,
    pub alternative: String,
    pub reason: String,
    pub next_ready_task: String,
}

impl GoAstInventoryTemplateCoverageFrontierReport {
    pub fn validate(&self) -> Result<(), SyntaxBackendBoundaryError> {
        require_string(
            "go_ast_inventory_template_coverage_frontier.artifact_kind",
            &self.artifact_kind,
            GO_AST_INVENTORY_TEMPLATE_COVERAGE_FRONTIER_ARTIFACT_KIND,
        )?;
        require_u32(
            "go_ast_inventory_template_coverage_frontier.version",
            self.version,
            VERSION,
        )?;
        if self.status != GO_AST_INVENTORY_TEMPLATE_COVERAGE_FRONTIER_STATUS_BLOCKED
            && self.status != GO_AST_INVENTORY_TEMPLATE_COVERAGE_FRONTIER_STATUS_READY
        {
            return Err(SyntaxBackendBoundaryError::new(
                "go_ast_inventory_template_coverage_frontier.status",
                "template coverage frontier status must be blocked or ready",
            ));
        }
        require_non_empty(
            "go_ast_inventory_template_coverage_frontier.grammar_id",
            &self.grammar_id,
        )?;
        require_non_empty(
            "go_ast_inventory_template_coverage_frontier.inventory_artifact_path",
            &self.inventory_artifact_path,
        )?;
        require_sha256(
            "go_ast_inventory_template_coverage_frontier.inventory_artifact_sha256",
            &self.inventory_artifact_sha256,
        )?;
        require_string(
            "go_ast_inventory_template_coverage_frontier.template_api_id",
            &self.template_api_id,
            GO_AST_INVENTORY_TEMPLATE_SYNTHESIS_API_ID,
        )?;
        require_string(
            "go_ast_inventory_template_coverage_frontier.template_api_boundary",
            &self.template_api_boundary,
            GO_AST_INVENTORY_TEMPLATE_SYNTHESIS_BOUNDARY,
        )?;
        require_exact_strings(
            "go_ast_inventory_template_coverage_frontier.supported_template_ids",
            &self.supported_template_ids,
            &[
                GO_AST_INVENTORY_FUNCTION_RETURN_EXPR_TEMPLATE_ID,
                GO_AST_INVENTORY_FUNCTION_STATEMENT_LIST_TEMPLATE_ID,
                GO_AST_INVENTORY_PACKAGE_IMPORT_SPEC_TEMPLATE_ID,
                GO_AST_INVENTORY_PACKAGE_TYPE_EXPR_TEMPLATE_ID,
                GO_AST_INVENTORY_PACKAGE_VAR_TYPE_EXPR_TEMPLATE_ID,
                GO_AST_INVENTORY_PACKAGE_VAR_VALUE_EXPR_TEMPLATE_ID,
            ],
        )?;
        require_exact_strings(
            "go_ast_inventory_template_coverage_frontier.supported_declaration_template_kinds",
            &self.supported_declaration_template_kinds,
            &[
                "FunctionReturnExpression",
                "FunctionStatementList",
                "FunctionStatementListNoResult",
                "FunctionStatementListReturnExpression",
                "PackageImportSpec",
                "PackageTypeExpression",
                "PackageVarTypeExpression",
                "PackageVarValueExpression",
            ],
        )?;
        require_exact_strings(
            "go_ast_inventory_template_coverage_frontier.supported_expression_template_kinds",
            &self.supported_expression_template_kinds,
            &[
                "ArrayTypeIdent",
                "ArrayTypeEllipsisIdent",
                "BasicLit",
                "BinaryExpr",
                "ChanTypeIdent",
                "CompositeLit",
                GO_AST_INVENTORY_DECLARATIVE_EXPRESSION_SPEC_KIND,
                "FuncLit",
                "FuncType",
                "Ident",
                "IndexExpr",
                "SelectorCall",
            ],
        )?;
        require_exact_strings(
            "go_ast_inventory_template_coverage_frontier.template_backed_synthesis_api_ids",
            &self.template_backed_synthesis_api_ids,
            &template_backed_synthesis_api_ids_from_scratch_registry(),
        )?;
        require_exact_strings(
            "go_ast_inventory_template_coverage_frontier.template_backed_witness_ids",
            &self.template_backed_witness_ids,
            &template_backed_witness_ids_from_scratch_registry(),
        )?;
        require_positive_usize(
            "go_ast_inventory_template_coverage_frontier.inventory_rule_count",
            self.inventory_rule_count,
        )?;
        require_positive_usize(
            "go_ast_inventory_template_coverage_frontier.inventory_field_count",
            self.inventory_field_count,
        )?;
        require_usize(
            "go_ast_inventory_template_coverage_frontier.inventory_field_count",
            self.inventory_required_field_count + self.inventory_optional_field_count,
            self.inventory_field_count,
        )?;
        if self.inventory_repeated_field_count > self.inventory_field_count {
            return Err(SyntaxBackendBoundaryError::new(
                "go_ast_inventory_template_coverage_frontier.inventory_repeated_field_count",
                "repeated field count cannot exceed inventory field count",
            ));
        }
        if self.inventory_finite_domain_field_count > self.inventory_field_count {
            return Err(SyntaxBackendBoundaryError::new(
                "go_ast_inventory_template_coverage_frontier.inventory_finite_domain_field_count",
                "finite-domain field count cannot exceed inventory field count",
            ));
        }
        validate_sorted_unique_non_empty(
            "go_ast_inventory_template_coverage_frontier.covered_rules",
            &self.covered_rules,
        )?;
        require_usize(
            "go_ast_inventory_template_coverage_frontier.covered_rule_count",
            self.covered_rule_count,
            self.covered_rules.len(),
        )?;
        validate_sorted_unique_non_empty(
            "go_ast_inventory_template_coverage_frontier.covered_fields",
            &self.covered_fields,
        )?;
        require_usize(
            "go_ast_inventory_template_coverage_frontier.covered_field_count",
            self.covered_field_count,
            self.covered_fields.len(),
        )?;
        require_usize(
            "go_ast_inventory_template_coverage_frontier.covered_finite_domain_choice_count",
            self.covered_finite_domain_choice_count,
            self.covered_finite_domain_choices.len(),
        )?;
        validate_template_finite_domain_choices(&self.covered_finite_domain_choices)?;
        if self.unsupported_optional_field_policy_count > self.inventory_optional_field_count {
            return Err(SyntaxBackendBoundaryError::new(
                "go_ast_inventory_template_coverage_frontier.unsupported_optional_field_policy_count",
                "unsupported optional-field policy count cannot exceed inventory optional-field count",
            ));
        }
        if self.unsupported_repeated_field_policy_count > self.inventory_repeated_field_count {
            return Err(SyntaxBackendBoundaryError::new(
                "go_ast_inventory_template_coverage_frontier.unsupported_repeated_field_policy_count",
                "unsupported repeated-field policy count cannot exceed inventory repeated-field count",
            ));
        }
        let unsupported_field_policy_count = self.unsupported_optional_field_policy_count
            + self.unsupported_repeated_field_policy_count;
        match (
            unsupported_field_policy_count,
            &self.first_unsupported_repeated_or_optional_policy,
        ) {
            (0, None) => {}
            (0, Some(_)) => {
                return Err(SyntaxBackendBoundaryError::new(
                    "go_ast_inventory_template_coverage_frontier.first_unsupported_repeated_or_optional_policy",
                    "exhausted optional/repeated field policies must not carry a first policy gap",
                ));
            }
            (_, Some(policy_gap)) => policy_gap.validate()?,
            (_, None) => {
                return Err(SyntaxBackendBoundaryError::new(
                    "go_ast_inventory_template_coverage_frontier.first_unsupported_repeated_or_optional_policy",
                    "unsupported optional/repeated field policy count requires a first policy gap",
                ));
            }
        }
        match (
            self.status.as_str(),
            &self.first_missing_template_capability,
        ) {
            (GO_AST_INVENTORY_TEMPLATE_COVERAGE_FRONTIER_STATUS_BLOCKED, Some(blocker)) => {
                blocker.validate()?;
                require_string(
                    "go_ast_inventory_template_coverage_frontier.next_ready_task",
                    &self.next_ready_task,
                    &blocker.next_ready_task,
                )?;
            }
            (GO_AST_INVENTORY_TEMPLATE_COVERAGE_FRONTIER_STATUS_BLOCKED, None) => {
                return Err(SyntaxBackendBoundaryError::new(
                    "go_ast_inventory_template_coverage_frontier.first_missing_template_capability",
                    "blocked template coverage frontier requires a first missing capability",
                ));
            }
            (GO_AST_INVENTORY_TEMPLATE_COVERAGE_FRONTIER_STATUS_READY, None) => {
                require_string(
                    "go_ast_inventory_template_coverage_frontier.next_ready_task",
                    &self.next_ready_task,
                    GO_AST_INVENTORY_TEMPLATE_COVERAGE_FRONTIER_NEXT_READY_TASK,
                )?;
            }
            (GO_AST_INVENTORY_TEMPLATE_COVERAGE_FRONTIER_STATUS_READY, Some(_)) => {
                return Err(SyntaxBackendBoundaryError::new(
                    "go_ast_inventory_template_coverage_frontier.first_missing_template_capability",
                    "ready template coverage frontier must not carry a first missing capability",
                ));
            }
            _ => unreachable!("template coverage frontier status was checked above"),
        }
        require_contains(
            "go_ast_inventory_template_coverage_frontier.validation_commands",
            &self.validation_commands,
            "go_tokenizer_generator_manifest_depth_report_validates_current_plus_supported_records",
        )?;
        require_contains(
            "go_ast_inventory_template_coverage_frontier.validation_commands",
            &self.validation_commands,
            "cargo test --test go_ast_schema_inventory_test",
        )?;
        for required in [
            "inventory-template coverage frontier",
            "template-backed token-only decoded streams",
            "checked inventory",
            "not scratch-witness registry order",
            "not Arena IR",
            "not BPE",
        ] {
            require_str_contains(
                "go_ast_inventory_template_coverage_frontier.claim_boundary",
                &self.claim_boundary,
                required,
            )?;
        }
        Ok(())
    }
}

impl GoAstInventoryTemplatePolicyGap {
    fn validate(&self) -> Result<(), SyntaxBackendBoundaryError> {
        require_non_empty(
            "go_ast_inventory_template_coverage_frontier.first_unsupported_repeated_or_optional_policy.field_family",
            &self.field_family,
        )?;
        if self.policy != "optional_field_policy" && self.policy != "repeated_field_policy" {
            return Err(SyntaxBackendBoundaryError::new(
                "go_ast_inventory_template_coverage_frontier.first_unsupported_repeated_or_optional_policy.policy",
                "policy must be optional_field_policy or repeated_field_policy",
            ));
        }
        require_non_empty(
            "go_ast_inventory_template_coverage_frontier.first_unsupported_repeated_or_optional_policy.reason",
            &self.reason,
        )?;
        require_non_empty(
            "go_ast_inventory_template_coverage_frontier.first_unsupported_repeated_or_optional_policy.next_template_capability",
            &self.next_template_capability,
        )
    }
}

impl GoAstInventoryTemplateCoverageBlocker {
    fn validate(&self) -> Result<(), SyntaxBackendBoundaryError> {
        require_non_empty(
            "go_ast_inventory_template_coverage_frontier.first_missing_template_capability.kind",
            &self.kind,
        )?;
        require_non_empty(
            "go_ast_inventory_template_coverage_frontier.first_missing_template_capability.rule_name",
            &self.rule_name,
        )?;
        require_non_empty(
            "go_ast_inventory_template_coverage_frontier.first_missing_template_capability.field_name",
            &self.field_name,
        )?;
        require_non_empty(
            "go_ast_inventory_template_coverage_frontier.first_missing_template_capability.alternative",
            &self.alternative,
        )?;
        require_non_empty(
            "go_ast_inventory_template_coverage_frontier.first_missing_template_capability.reason",
            &self.reason,
        )?;
        require_string(
            "go_ast_inventory_template_coverage_frontier.first_missing_template_capability.next_ready_task",
            &self.next_ready_task,
            GO_AST_INVENTORY_TEMPLATE_COVERAGE_FRONTIER_BLOCKER_NEXT_READY_TASK,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoAstGenerationFrontierReport {
    pub artifact_kind: String,
    pub version: u32,
    pub status: String,
    pub pinned_go_version: String,
    pub grammar_id: String,
    pub inventory_artifact_path: String,
    pub inventory_artifact_sha256: String,
    pub render_reparse_authority_source: String,
    pub oracle_cutover_accepted: bool,
    pub oracle_record_count: usize,
    pub oracle_canonical_fidelity_only_gap_count: usize,
    pub canonical_fidelity_gaps_diagnostic_only: bool,
    pub essential_parser_directives_sidecar_policy: bool,
    pub raw_state_machine_core_boundary_id: String,
    pub raw_state_machine_core_boundary: String,
    pub raw_state_machine_compile_ready: bool,
    pub initial_state_ready: bool,
    pub token_layout_vocab_size: u32,
    pub token_layout_pad_token: u32,
    pub grammar_rule_count: usize,
    pub grammar_field_family_count: usize,
    pub category_record_count: usize,
    pub category_records: Vec<GoAstGenerationFrontierCategoryRecord>,
    pub ready_count: usize,
    pub blocked_count: usize,
    pub deferred_non_core_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_blocker: Option<GoAstGenerationFrontierBlocker>,
    pub witnesses: Vec<GoAstGenerationFrontierWitnessRecord>,
    pub witness_hashes: Vec<GoAstGenerationFrontierWitnessHash>,
    pub inventory_synthesis_witness_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_non_inventory_synthesis_witness_id: Option<String>,
    pub no_parser_metadata_at_inference_time: bool,
    pub source_prefix_table_used: bool,
    pub arena_ir_used: bool,
    pub go_ir_used: bool,
    pub bpe_validity_authority_used: bool,
    pub type_correctness_claim_allowed: bool,
    pub typecheck_sidecar_diagnostic_failures: usize,
    pub stable_backend_claimed: bool,
    pub next_ready_task: String,
    pub validation_commands: Vec<String>,
    pub claim_boundary: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoAstGenerationFrontierCategoryRecord {
    pub category_id: String,
    pub category_kind: String,
    pub rule_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field_name: Option<String>,
    pub status: String,
    pub witness_ids: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_blocker: Option<GoAstGenerationFrontierBlocker>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deferred_reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoAstGenerationFrontierBlocker {
    pub kind: String,
    pub category_id: String,
    pub reason: String,
    pub next_ready_task: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoAstGenerationFrontierWitnessRecord {
    pub witness_id: String,
    pub grammar_category: String,
    pub root: String,
    pub raw_token_count: usize,
    pub raw_token_ids_sha256: String,
    pub rendered_source_sha256: String,
    pub covered_node_kinds: Vec<String>,
    pub covered_field_families: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inventory_synthesis_api_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inventory_synthesis_core_api_id: Option<String>,
    pub empty_prefix_start: bool,
    pub every_selected_token_admitted: bool,
    pub strict_decode_complete: bool,
    pub token_only_tree_decode_complete: bool,
    pub render_reparse_authority_source: String,
    pub gofmt_validation: String,
    pub go_parser_reparse_validation: String,
    pub render_reparse_sidecar_passed: bool,
    pub no_parser_metadata_at_inference_time: bool,
    pub source_prefix_table_used: bool,
    pub source_instance_segment_sidecar_used: bool,
    pub arena_ir_used: bool,
    pub go_ir_used: bool,
    pub route_handoff_provenance_used: bool,
    pub bpe_validity_authority_used: bool,
    pub source_program_validity_claim_allowed: bool,
    pub type_correctness_claim_allowed: bool,
    pub stable_backend_claimed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoAstGenerationFrontierWitnessHash {
    pub witness_id: String,
    pub raw_token_ids_sha256: String,
    pub rendered_source_sha256: String,
}

impl GoAstGenerationFrontierReport {
    pub fn validate(&self) -> Result<(), SyntaxBackendBoundaryError> {
        require_string(
            "go_ast_generation_frontier.artifact_kind",
            &self.artifact_kind,
            GO_AST_GENERATION_FRONTIER_ARTIFACT_KIND,
        )?;
        require_u32("go_ast_generation_frontier.version", self.version, VERSION)?;
        if self.status != GO_AST_GENERATION_FRONTIER_STATUS_READY
            && self.status != GO_AST_GENERATION_FRONTIER_STATUS_BLOCKED
        {
            return Err(SyntaxBackendBoundaryError::new(
                "go_ast_generation_frontier.status",
                "frontier status must be ready or blocked",
            ));
        }
        require_string(
            "go_ast_generation_frontier.pinned_go_version",
            &self.pinned_go_version,
            crate::go_ast_schema_inventory::GO_AST_SCHEMA_INVENTORY_PINNED_GO_VERSION,
        )?;
        require_non_empty("go_ast_generation_frontier.grammar_id", &self.grammar_id)?;
        require_non_empty(
            "go_ast_generation_frontier.inventory_artifact_path",
            &self.inventory_artifact_path,
        )?;
        require_sha256(
            "go_ast_generation_frontier.inventory_artifact_sha256",
            &self.inventory_artifact_sha256,
        )?;
        require_string(
            "go_ast_generation_frontier.render_reparse_authority_source",
            &self.render_reparse_authority_source,
            SCRATCH_GENERATION_GATE_RENDER_AUTHORITY_SOURCE,
        )?;
        require_bool(
            "go_ast_generation_frontier.oracle_cutover_accepted",
            self.oracle_cutover_accepted,
            true,
        )?;
        require_positive_usize(
            "go_ast_generation_frontier.oracle_record_count",
            self.oracle_record_count,
        )?;
        require_bool(
            "go_ast_generation_frontier.canonical_fidelity_gaps_diagnostic_only",
            self.canonical_fidelity_gaps_diagnostic_only,
            true,
        )?;
        require_bool(
            "go_ast_generation_frontier.essential_parser_directives_sidecar_policy",
            self.essential_parser_directives_sidecar_policy,
            true,
        )?;
        require_string(
            "go_ast_generation_frontier.raw_state_machine_core_boundary_id",
            &self.raw_state_machine_core_boundary_id,
            RAW_SYNTAX_STATE_MACHINE_CORE_BOUNDARY_ID,
        )?;
        require_string(
            "go_ast_generation_frontier.raw_state_machine_core_boundary",
            &self.raw_state_machine_core_boundary,
            RAW_SYNTAX_STATE_MACHINE_CORE_BOUNDARY,
        )?;
        require_bool(
            "go_ast_generation_frontier.raw_state_machine_compile_ready",
            self.raw_state_machine_compile_ready,
            true,
        )?;
        require_bool(
            "go_ast_generation_frontier.initial_state_ready",
            self.initial_state_ready,
            true,
        )?;
        require_u32(
            "go_ast_generation_frontier.token_layout_pad_token",
            self.token_layout_pad_token,
            self.token_layout_vocab_size.saturating_sub(1),
        )?;
        require_positive_usize(
            "go_ast_generation_frontier.grammar_rule_count",
            self.grammar_rule_count,
        )?;
        require_positive_usize(
            "go_ast_generation_frontier.grammar_field_family_count",
            self.grammar_field_family_count,
        )?;
        require_usize(
            "go_ast_generation_frontier.category_record_count",
            self.category_record_count,
            self.category_records.len(),
        )?;
        require_positive_usize(
            "go_ast_generation_frontier.category_records",
            self.category_records.len(),
        )?;
        require_positive_usize("go_ast_generation_frontier.witnesses", self.witnesses.len())?;

        let mut witness_ids = BTreeSet::new();
        let expected_hashes = self
            .witnesses
            .iter()
            .map(|witness| {
                witness.validate()?;
                if !witness_ids.insert(witness.witness_id.as_str()) {
                    return Err(SyntaxBackendBoundaryError::new(
                        "go_ast_generation_frontier.witnesses",
                        format!("duplicate witness {}", witness.witness_id),
                    ));
                }
                Ok(GoAstGenerationFrontierWitnessHash {
                    witness_id: witness.witness_id.clone(),
                    raw_token_ids_sha256: witness.raw_token_ids_sha256.clone(),
                    rendered_source_sha256: witness.rendered_source_sha256.clone(),
                })
            })
            .collect::<Result<Vec<_>, SyntaxBackendBoundaryError>>()?;
        if self.witness_hashes != expected_hashes {
            return Err(SyntaxBackendBoundaryError::new(
                "go_ast_generation_frontier.witness_hashes",
                "frontier witness hashes must match witness records in order",
            ));
        }
        let inventory_synthesis_witness_count = self
            .witnesses
            .iter()
            .filter(|witness| witness.inventory_synthesis_api_id.is_some())
            .count();
        require_usize(
            "go_ast_generation_frontier.inventory_synthesis_witness_count",
            self.inventory_synthesis_witness_count,
            inventory_synthesis_witness_count,
        )?;
        let first_non_inventory_synthesis_witness_id = self
            .witnesses
            .iter()
            .find(|witness| witness.inventory_synthesis_api_id.is_none())
            .map(|witness| witness.witness_id.clone());
        if self.first_non_inventory_synthesis_witness_id != first_non_inventory_synthesis_witness_id
        {
            return Err(SyntaxBackendBoundaryError::new(
                "go_ast_generation_frontier.first_non_inventory_synthesis_witness_id",
                "first non-inventory-synthesis witness must match frontier witness order",
            ));
        }

        let mut ready_count = 0usize;
        let mut blocked_count = 0usize;
        let mut deferred_non_core_count = 0usize;
        let mut rule_count = 0usize;
        let mut field_family_count = 0usize;
        let mut seen_categories = BTreeSet::new();
        let mut first_blocker = None;
        for record in &self.category_records {
            record.validate(&witness_ids)?;
            if !seen_categories.insert(record.category_id.as_str()) {
                return Err(SyntaxBackendBoundaryError::new(
                    "go_ast_generation_frontier.category_records",
                    format!("duplicate category {}", record.category_id),
                ));
            }
            match record.category_kind.as_str() {
                "ast_rule" => rule_count += 1,
                "ast_field_family" => field_family_count += 1,
                "parser_error_placeholder" => {}
                other => {
                    return Err(SyntaxBackendBoundaryError::new(
                        "go_ast_generation_frontier.category_records.category_kind",
                        format!("unsupported frontier category kind {other}"),
                    ));
                }
            }
            match record.status.as_str() {
                GO_AST_GENERATION_FRONTIER_CATEGORY_READY => ready_count += 1,
                GO_AST_GENERATION_FRONTIER_CATEGORY_BLOCKED => {
                    blocked_count += 1;
                    if first_blocker.is_none() {
                        first_blocker = record.first_blocker.clone();
                    }
                }
                GO_AST_GENERATION_FRONTIER_CATEGORY_DEFERRED_NON_CORE => {
                    deferred_non_core_count += 1
                }
                other => {
                    return Err(SyntaxBackendBoundaryError::new(
                        "go_ast_generation_frontier.category_records.status",
                        format!("unsupported frontier category status {other}"),
                    ));
                }
            }
        }
        require_usize(
            "go_ast_generation_frontier.grammar_rule_count",
            rule_count,
            self.grammar_rule_count,
        )?;
        require_usize(
            "go_ast_generation_frontier.grammar_field_family_count",
            field_family_count,
            self.grammar_field_family_count,
        )?;
        require_usize(
            "go_ast_generation_frontier.ready_count",
            self.ready_count,
            ready_count,
        )?;
        require_usize(
            "go_ast_generation_frontier.blocked_count",
            self.blocked_count,
            blocked_count,
        )?;
        require_usize(
            "go_ast_generation_frontier.deferred_non_core_count",
            self.deferred_non_core_count,
            deferred_non_core_count,
        )?;
        require_usize(
            "go_ast_generation_frontier.category_record_count",
            self.ready_count + self.blocked_count + self.deferred_non_core_count,
            self.category_record_count,
        )?;
        if blocked_count == 0 {
            require_string(
                "go_ast_generation_frontier.status",
                &self.status,
                GO_AST_GENERATION_FRONTIER_STATUS_READY,
            )?;
            if self.first_blocker.is_some() {
                return Err(SyntaxBackendBoundaryError::new(
                    "go_ast_generation_frontier.first_blocker",
                    "closed frontier must not carry a blocker",
                ));
            }
        } else {
            require_string(
                "go_ast_generation_frontier.status",
                &self.status,
                GO_AST_GENERATION_FRONTIER_STATUS_BLOCKED,
            )?;
            if self.first_blocker != first_blocker {
                return Err(SyntaxBackendBoundaryError::new(
                    "go_ast_generation_frontier.first_blocker",
                    "frontier first blocker must match the first blocked category",
                ));
            }
        }
        require_bool(
            "go_ast_generation_frontier.no_parser_metadata_at_inference_time",
            self.no_parser_metadata_at_inference_time,
            true,
        )?;
        require_bool(
            "go_ast_generation_frontier.source_prefix_table_used",
            self.source_prefix_table_used,
            false,
        )?;
        require_bool(
            "go_ast_generation_frontier.arena_ir_used",
            self.arena_ir_used,
            false,
        )?;
        require_bool(
            "go_ast_generation_frontier.go_ir_used",
            self.go_ir_used,
            false,
        )?;
        require_bool(
            "go_ast_generation_frontier.bpe_validity_authority_used",
            self.bpe_validity_authority_used,
            false,
        )?;
        require_bool(
            "go_ast_generation_frontier.type_correctness_claim_allowed",
            self.type_correctness_claim_allowed,
            false,
        )?;
        require_bool(
            "go_ast_generation_frontier.stable_backend_claimed",
            self.stable_backend_claimed,
            false,
        )?;
        require_string(
            "go_ast_generation_frontier.next_ready_task",
            &self.next_ready_task,
            GO_AST_GENERATION_FRONTIER_NEXT_READY_TASK,
        )?;
        require_contains(
            "go_ast_generation_frontier.validation_commands",
            &self.validation_commands,
            "go_tokenizer_generator_manifest_depth_report_validates_current_plus_supported_records",
        )?;
        require_contains(
            "go_ast_generation_frontier.validation_commands",
            &self.validation_commands,
            "cargo test --test go_ast_schema_inventory_test",
        )?;
        for required in [
            "parser-level Go syntactic correctness frontier",
            "empty GoSourceFile",
            "RawSyntaxStateMachineCore",
            "oracle render/reparse",
            "diagnostic-only",
            "not source-prefix table",
            "not Arena IR or GoIR",
            "not BPE",
            "not type correctness",
        ] {
            require_str_contains(
                "go_ast_generation_frontier.claim_boundary",
                &self.claim_boundary,
                required,
            )?;
        }
        Ok(())
    }
}

impl GoAstGenerationFrontierCategoryRecord {
    fn validate(&self, witness_ids: &BTreeSet<&str>) -> Result<(), SyntaxBackendBoundaryError> {
        require_non_empty(
            "go_ast_generation_frontier.category_records.category_id",
            &self.category_id,
        )?;
        require_non_empty(
            "go_ast_generation_frontier.category_records.rule_name",
            &self.rule_name,
        )?;
        for witness_id in &self.witness_ids {
            if !witness_ids.contains(witness_id.as_str()) {
                return Err(SyntaxBackendBoundaryError::new(
                    "go_ast_generation_frontier.category_records.witness_ids",
                    format!("unknown frontier witness {witness_id}"),
                ));
            }
        }
        match self.status.as_str() {
            GO_AST_GENERATION_FRONTIER_CATEGORY_READY => {
                require_positive_usize(
                    "go_ast_generation_frontier.category_records.witness_ids",
                    self.witness_ids.len(),
                )?;
                if self.first_blocker.is_some() || self.deferred_reason.is_some() {
                    return Err(SyntaxBackendBoundaryError::new(
                        "go_ast_generation_frontier.category_records",
                        "ready frontier categories cannot carry blockers or deferred reasons",
                    ));
                }
            }
            GO_AST_GENERATION_FRONTIER_CATEGORY_BLOCKED => {
                let blocker = self.first_blocker.as_ref().ok_or_else(|| {
                    SyntaxBackendBoundaryError::new(
                        "go_ast_generation_frontier.category_records.first_blocker",
                        "blocked frontier category must carry a precise blocker",
                    )
                })?;
                blocker.validate()?;
                if blocker.category_id != self.category_id {
                    return Err(SyntaxBackendBoundaryError::new(
                        "go_ast_generation_frontier.category_records.first_blocker",
                        "category blocker must point back to the blocked category",
                    ));
                }
                if self.deferred_reason.is_some() {
                    return Err(SyntaxBackendBoundaryError::new(
                        "go_ast_generation_frontier.category_records.deferred_reason",
                        "blocked frontier categories cannot carry deferred reasons",
                    ));
                }
            }
            GO_AST_GENERATION_FRONTIER_CATEGORY_DEFERRED_NON_CORE => {
                require_non_empty(
                    "go_ast_generation_frontier.category_records.deferred_reason",
                    self.deferred_reason.as_deref().unwrap_or(""),
                )?;
                if !self.witness_ids.is_empty() || self.first_blocker.is_some() {
                    return Err(SyntaxBackendBoundaryError::new(
                        "go_ast_generation_frontier.category_records",
                        "deferred non-core categories cannot carry witnesses or blockers",
                    ));
                }
            }
            _ => {
                return Err(SyntaxBackendBoundaryError::new(
                    "go_ast_generation_frontier.category_records.status",
                    "unsupported frontier category status",
                ));
            }
        }
        Ok(())
    }
}

impl GoAstGenerationFrontierBlocker {
    fn validate(&self) -> Result<(), SyntaxBackendBoundaryError> {
        require_non_empty("go_ast_generation_frontier.blocker.kind", &self.kind)?;
        require_non_empty(
            "go_ast_generation_frontier.blocker.category_id",
            &self.category_id,
        )?;
        require_non_empty("go_ast_generation_frontier.blocker.reason", &self.reason)?;
        require_string(
            "go_ast_generation_frontier.blocker.next_ready_task",
            &self.next_ready_task,
            GO_AST_GENERATION_FRONTIER_BLOCKER_NEXT_READY_TASK,
        )
    }
}

impl GoAstGenerationFrontierWitnessRecord {
    fn from_scratch_seed(seed: &GoTokenizerGeneratorManifestDepthScratchSeedSummary) -> Self {
        Self {
            witness_id: seed.seed_id.clone(),
            grammar_category: seed.grammar_category.clone(),
            root: seed.empty_prefix_root.clone(),
            raw_token_count: seed.raw_token_count,
            raw_token_ids_sha256: seed.raw_token_ids_sha256.clone(),
            rendered_source_sha256: seed.rendered_source_sha256.clone(),
            covered_node_kinds: seed.covered_node_kinds.clone(),
            covered_field_families: seed.covered_field_families.clone(),
            inventory_synthesis_api_id: seed.inventory_synthesis_api_id.clone(),
            inventory_synthesis_core_api_id: seed.inventory_synthesis_core_api_id.clone(),
            empty_prefix_start: seed.empty_prefix_root == EMPTY_PREFIX_ROOT
                && seed.first_selected_raw_token_id == seed.raw_token_ids[0],
            every_selected_token_admitted: seed.every_selected_raw_token_admitted
                && seed.selected_transition_accepted_count == seed.raw_token_count,
            strict_decode_complete: seed.final_state_observed_status == "complete"
                && seed.final_state_consumed_raw_tokens == seed.raw_token_count
                && seed.final_state_eof_valid
                && seed.final_state_stack_depth == 0,
            token_only_tree_decode_complete: seed.decoded_from_token_only_stream
                && seed.decoded_tree_reusable
                && seed.token_only_root_rule == "File"
                && seed.token_only_root_node_kind == "File",
            render_reparse_authority_source: SCRATCH_GENERATION_GATE_RENDER_AUTHORITY_SOURCE
                .to_string(),
            gofmt_validation: seed.gofmt_validation.clone(),
            go_parser_reparse_validation: seed.go_parser_reparse_validation.clone(),
            render_reparse_sidecar_passed: seed.render_reparse_sidecar_validation_ready,
            no_parser_metadata_at_inference_time: !seed.per_output_parser_metadata_required,
            source_prefix_table_used: seed.source_prefix_table_used,
            source_instance_segment_sidecar_used: seed.source_instance_segment_sidecar_used,
            arena_ir_used: seed.arena_ir_used,
            go_ir_used: false,
            route_handoff_provenance_used: seed.route_handoff_provenance_used,
            bpe_validity_authority_used: seed.bpe_validity_authority_used,
            source_program_validity_claim_allowed: seed.source_program_validity_claim_allowed,
            type_correctness_claim_allowed: false,
            stable_backend_claimed: seed.stable_backend_claimed,
        }
    }

    fn validate(&self) -> Result<(), SyntaxBackendBoundaryError> {
        require_non_empty(
            "go_ast_generation_frontier.witnesses.witness_id",
            &self.witness_id,
        )?;
        require_non_empty(
            "go_ast_generation_frontier.witnesses.grammar_category",
            &self.grammar_category,
        )?;
        require_string(
            "go_ast_generation_frontier.witnesses.root",
            &self.root,
            EMPTY_PREFIX_ROOT,
        )?;
        require_positive_usize(
            "go_ast_generation_frontier.witnesses.raw_token_count",
            self.raw_token_count,
        )?;
        require_sha256(
            "go_ast_generation_frontier.witnesses.raw_token_ids_sha256",
            &self.raw_token_ids_sha256,
        )?;
        require_sha256(
            "go_ast_generation_frontier.witnesses.rendered_source_sha256",
            &self.rendered_source_sha256,
        )?;
        validate_sorted_unique_non_empty(
            "go_ast_generation_frontier.witnesses.covered_node_kinds",
            &self.covered_node_kinds,
        )?;
        validate_sorted_unique_non_empty(
            "go_ast_generation_frontier.witnesses.covered_field_families",
            &self.covered_field_families,
        )?;
        match (
            self.inventory_synthesis_api_id.as_deref(),
            self.inventory_synthesis_core_api_id.as_deref(),
        ) {
            (Some(api_id), Some(core_api_id)) => {
                require_non_empty(
                    "go_ast_generation_frontier.witnesses.inventory_synthesis_api_id",
                    api_id,
                )?;
                require_string(
                    "go_ast_generation_frontier.witnesses.inventory_synthesis_core_api_id",
                    core_api_id,
                    GO_AST_INVENTORY_SYNTHESIS_CORE_API_ID,
                )?;
            }
            (None, None) => {}
            _ => {
                return Err(SyntaxBackendBoundaryError::new(
                    "go_ast_generation_frontier.witnesses.inventory_synthesis_core_api_id",
                    "inventory synthesis API and core API metadata must be present together",
                ));
            }
        }
        require_bool(
            "go_ast_generation_frontier.witnesses.empty_prefix_start",
            self.empty_prefix_start,
            true,
        )?;
        require_bool(
            "go_ast_generation_frontier.witnesses.every_selected_token_admitted",
            self.every_selected_token_admitted,
            true,
        )?;
        require_bool(
            "go_ast_generation_frontier.witnesses.strict_decode_complete",
            self.strict_decode_complete,
            true,
        )?;
        require_bool(
            "go_ast_generation_frontier.witnesses.token_only_tree_decode_complete",
            self.token_only_tree_decode_complete,
            true,
        )?;
        require_string(
            "go_ast_generation_frontier.witnesses.render_reparse_authority_source",
            &self.render_reparse_authority_source,
            SCRATCH_GENERATION_GATE_RENDER_AUTHORITY_SOURCE,
        )?;
        require_string(
            "go_ast_generation_frontier.witnesses.gofmt_validation",
            &self.gofmt_validation,
            "passed",
        )?;
        require_string(
            "go_ast_generation_frontier.witnesses.go_parser_reparse_validation",
            &self.go_parser_reparse_validation,
            "passed",
        )?;
        require_bool(
            "go_ast_generation_frontier.witnesses.render_reparse_sidecar_passed",
            self.render_reparse_sidecar_passed,
            true,
        )?;
        require_bool(
            "go_ast_generation_frontier.witnesses.no_parser_metadata_at_inference_time",
            self.no_parser_metadata_at_inference_time,
            true,
        )?;
        require_bool(
            "go_ast_generation_frontier.witnesses.source_prefix_table_used",
            self.source_prefix_table_used,
            false,
        )?;
        require_bool(
            "go_ast_generation_frontier.witnesses.source_instance_segment_sidecar_used",
            self.source_instance_segment_sidecar_used,
            false,
        )?;
        require_bool(
            "go_ast_generation_frontier.witnesses.arena_ir_used",
            self.arena_ir_used,
            false,
        )?;
        require_bool(
            "go_ast_generation_frontier.witnesses.go_ir_used",
            self.go_ir_used,
            false,
        )?;
        require_bool(
            "go_ast_generation_frontier.witnesses.route_handoff_provenance_used",
            self.route_handoff_provenance_used,
            false,
        )?;
        require_bool(
            "go_ast_generation_frontier.witnesses.bpe_validity_authority_used",
            self.bpe_validity_authority_used,
            false,
        )?;
        require_bool(
            "go_ast_generation_frontier.witnesses.source_program_validity_claim_allowed",
            self.source_program_validity_claim_allowed,
            false,
        )?;
        require_bool(
            "go_ast_generation_frontier.witnesses.type_correctness_claim_allowed",
            self.type_correctness_claim_allowed,
            false,
        )?;
        require_bool(
            "go_ast_generation_frontier.witnesses.stable_backend_claimed",
            self.stable_backend_claimed,
            false,
        )
    }
}

fn go_ast_inventory_template_coverage_frontier_report(
    grammar_coverage: &GoAstSchemaGrammarCoverageReport,
    scratch_generation_gate: &GoTokenizerGeneratorManifestDepthScratchGenerationGate,
) -> Result<GoAstInventoryTemplateCoverageFrontierReport, SyntaxBackendBoundaryError> {
    let grammar = GoAstSchemaInventoryAdapter::lower_inventory_file(Path::new(
        GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH,
    ))
    .map_err(|error| {
        SyntaxBackendBoundaryError::new(
            "go_ast_inventory_template_coverage_frontier.inventory",
            format!("failed to lower checked Go AST inventory for template coverage: {error}"),
        )
    })?;
    require_string(
        "go_ast_inventory_template_coverage_frontier.grammar_id",
        &grammar.grammar_id,
        &grammar_coverage.grammar_id,
    )?;

    let expected_template_backed_witness_ids = template_backed_witness_ids_from_scratch_registry();
    let expected_witness_backed_synthesis_api_ids =
        witness_backed_template_synthesis_api_ids_from_scratch_registry();
    let expected_template_backed_synthesis_api_ids =
        template_backed_synthesis_api_ids_from_scratch_registry();
    let mut covered_rules = BTreeSet::new();
    let mut covered_fields = BTreeSet::new();
    for (witness_id, synthesis_api_id) in expected_template_backed_witness_ids
        .iter()
        .zip(expected_witness_backed_synthesis_api_ids.iter())
    {
        let witness = scratch_generation_gate
            .go_ast_inventory_scratch_seeds
            .iter()
            .find(|seed| seed.seed_id == *witness_id)
            .ok_or_else(|| {
                SyntaxBackendBoundaryError::new(
                    "go_ast_inventory_template_coverage_frontier.template_backed_witness_ids",
                    format!("missing template-backed witness {witness_id}"),
                )
            })?;
        require_string(
            "go_ast_inventory_template_coverage_frontier.template_backed_synthesis_api_ids",
            witness.inventory_synthesis_api_id.as_deref().unwrap_or(""),
            synthesis_api_id,
        )?;
        for node_kind in &witness.covered_node_kinds {
            covered_rules.insert(node_kind.clone());
        }
        for field_family in &witness.covered_field_families {
            covered_fields.insert(field_family.clone());
        }
    }

    let covered_fields = covered_fields.into_iter().collect::<Vec<_>>();
    let covered_field_set = covered_fields
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let inventory_field_count = grammar
        .rules
        .iter()
        .map(|rule| rule.fields.len())
        .sum::<usize>();
    let inventory_required_field_count = grammar
        .rules
        .iter()
        .flat_map(|rule| &rule.fields)
        .filter(|field| field.required)
        .count();
    let inventory_optional_field_count = inventory_field_count - inventory_required_field_count;
    let inventory_repeated_field_count = grammar
        .rules
        .iter()
        .flat_map(|rule| &rule.fields)
        .filter(|field| field.multiple)
        .count();
    let inventory_finite_domain_field_count = grammar
        .rules
        .iter()
        .flat_map(|rule| &rule.fields)
        .filter(|field| field.alternatives.len() > 1)
        .count();
    let unsupported_optional_field_policy_count =
        unsupported_template_policy_count(&grammar, &covered_field_set, |field| !field.required);
    let unsupported_repeated_field_policy_count =
        unsupported_template_policy_count(&grammar, &covered_field_set, |field| field.multiple);
    let first_unsupported_repeated_or_optional_policy =
        first_unsupported_template_policy_gap(&grammar, &covered_field_set);
    let first_missing_template_capability =
        first_missing_template_capability(&grammar, &covered_rules, &covered_field_set)?;
    let status = if first_missing_template_capability.is_some() {
        GO_AST_INVENTORY_TEMPLATE_COVERAGE_FRONTIER_STATUS_BLOCKED
    } else {
        GO_AST_INVENTORY_TEMPLATE_COVERAGE_FRONTIER_STATUS_READY
    };
    let next_ready_task = first_missing_template_capability
        .as_ref()
        .map(|blocker| blocker.next_ready_task.clone())
        .unwrap_or_else(|| GO_AST_INVENTORY_TEMPLATE_COVERAGE_FRONTIER_NEXT_READY_TASK.to_string());
    let covered_finite_domain_choices = template_covered_finite_domain_choices(&grammar)?;
    let report = GoAstInventoryTemplateCoverageFrontierReport {
        artifact_kind: GO_AST_INVENTORY_TEMPLATE_COVERAGE_FRONTIER_ARTIFACT_KIND.to_string(),
        version: VERSION,
        status: status.to_string(),
        grammar_id: grammar_coverage.grammar_id.clone(),
        inventory_artifact_path: grammar_coverage.inventory_artifact_path.clone(),
        inventory_artifact_sha256: grammar_coverage.inventory_artifact_sha256.clone(),
        template_api_id: GO_AST_INVENTORY_TEMPLATE_SYNTHESIS_API_ID.to_string(),
        template_api_boundary: GO_AST_INVENTORY_TEMPLATE_SYNTHESIS_BOUNDARY.to_string(),
        supported_template_ids: vec![
            GO_AST_INVENTORY_FUNCTION_RETURN_EXPR_TEMPLATE_ID.to_string(),
            GO_AST_INVENTORY_FUNCTION_STATEMENT_LIST_TEMPLATE_ID.to_string(),
            GO_AST_INVENTORY_PACKAGE_IMPORT_SPEC_TEMPLATE_ID.to_string(),
            GO_AST_INVENTORY_PACKAGE_TYPE_EXPR_TEMPLATE_ID.to_string(),
            GO_AST_INVENTORY_PACKAGE_VAR_TYPE_EXPR_TEMPLATE_ID.to_string(),
            GO_AST_INVENTORY_PACKAGE_VAR_VALUE_EXPR_TEMPLATE_ID.to_string(),
        ],
        supported_declaration_template_kinds: vec![
            "FunctionReturnExpression".to_string(),
            "FunctionStatementList".to_string(),
            "FunctionStatementListNoResult".to_string(),
            "FunctionStatementListReturnExpression".to_string(),
            "PackageImportSpec".to_string(),
            "PackageTypeExpression".to_string(),
            "PackageVarTypeExpression".to_string(),
            "PackageVarValueExpression".to_string(),
        ],
        supported_expression_template_kinds: vec![
            "ArrayTypeIdent".to_string(),
            "ArrayTypeEllipsisIdent".to_string(),
            "BasicLit".to_string(),
            "BinaryExpr".to_string(),
            "ChanTypeIdent".to_string(),
            "CompositeLit".to_string(),
            GO_AST_INVENTORY_DECLARATIVE_EXPRESSION_SPEC_KIND.to_string(),
            "FuncLit".to_string(),
            "FuncType".to_string(),
            "Ident".to_string(),
            "IndexExpr".to_string(),
            "SelectorCall".to_string(),
        ],
        template_backed_synthesis_api_ids: expected_template_backed_synthesis_api_ids
            .iter()
            .map(|api_id| (*api_id).to_string())
            .collect(),
        template_backed_witness_ids: expected_template_backed_witness_ids
            .iter()
            .map(|witness_id| (*witness_id).to_string())
            .collect(),
        inventory_rule_count: grammar.rules.len(),
        inventory_field_count,
        inventory_required_field_count,
        inventory_optional_field_count,
        inventory_repeated_field_count,
        inventory_finite_domain_field_count,
        covered_rule_count: covered_rules.len(),
        covered_rules: covered_rules.into_iter().collect(),
        covered_field_count: covered_fields.len(),
        covered_fields,
        covered_finite_domain_choice_count: covered_finite_domain_choices.len(),
        covered_finite_domain_choices,
        unsupported_optional_field_policy_count,
        unsupported_repeated_field_policy_count,
        first_unsupported_repeated_or_optional_policy,
        first_missing_template_capability,
        next_ready_task,
        validation_commands: vec![
            "cargo test --test go_ast_schema_inventory_test".to_string(),
            "cargo test --test syntax_go_source_instance_projection_test go_tokenizer_generator_manifest_depth_report_validates_current_plus_supported_records -- --exact".to_string(),
            "python3 scripts/testing/test-quality-audit.py --changed-from origin/main".to_string(),
            "git diff --check".to_string(),
        ],
        claim_boundary: GO_AST_INVENTORY_TEMPLATE_COVERAGE_FRONTIER_CLAIM_BOUNDARY.to_string(),
    };
    report.validate()?;
    Ok(report)
}

fn unsupported_template_policy_count(
    grammar: &GrammarIr,
    covered_field_set: &BTreeSet<&str>,
    is_policy_field: impl Fn(&crate::grammar_ir::GrammarField) -> bool,
) -> usize {
    grammar
        .rules
        .iter()
        .flat_map(|rule| {
            rule.fields.iter().map(|field| {
                (
                    format!("{}.{}", rule.node_kind, field.name),
                    is_policy_field(field),
                    template_coverage_field_is_deferred_non_core(&rule.node_kind, &field.name),
                )
            })
        })
        .filter(|(field_family, is_policy, is_deferred_non_core)| {
            *is_policy
                && !*is_deferred_non_core
                && !covered_field_set.contains(field_family.as_str())
        })
        .count()
}

fn first_unsupported_template_policy_gap(
    grammar: &GrammarIr,
    covered_field_set: &BTreeSet<&str>,
) -> Option<GoAstInventoryTemplatePolicyGap> {
    for rule in &grammar.rules {
        for field in &rule.fields {
            let field_family = format!("{}.{}", rule.node_kind, field.name);
            if template_coverage_field_is_deferred_non_core(&rule.node_kind, &field.name) {
                continue;
            }
            if covered_field_set.contains(field_family.as_str())
                || (field.required && !field.multiple)
            {
                continue;
            }
            let (policy, next_template_capability) = if field.multiple {
                (
                    "repeated_field_policy",
                    "add reusable repeated-field cardinality policy to the Go inventory template API",
                )
            } else {
                (
                    "optional_field_policy",
                    "add reusable optional-field presence policy to the Go inventory template API",
                )
            };
            return Some(GoAstInventoryTemplatePolicyGap {
                field_family: field_family.clone(),
                policy: policy.to_string(),
                reason: format!(
                    "current template coverage does not yet expose a reusable {policy} for {field_family}"
                ),
                next_template_capability: next_template_capability.to_string(),
            });
        }
    }
    None
}

fn first_missing_template_capability(
    grammar: &GrammarIr,
    covered_rules: &BTreeSet<String>,
    covered_field_set: &BTreeSet<&str>,
) -> Result<Option<GoAstInventoryTemplateCoverageBlocker>, SyntaxBackendBoundaryError> {
    let results = grammar
        .rules
        .iter()
        .find(|rule| rule.node_kind == "ReturnStmt")
        .and_then(|rule| rule.fields.iter().find(|field| field.name == "Results"))
        .ok_or_else(|| {
            SyntaxBackendBoundaryError::new(
                "go_ast_inventory_template_coverage_frontier.first_missing_template_capability",
                "checked inventory must expose ReturnStmt.Results",
            )
        })?;
    let mut supported_return_expression_rules = [
        "ArrayType",
        "BasicLit",
        "BinaryExpr",
        "CallExpr",
        "ChanType",
        "CompositeLit",
        "Ellipsis",
        "FuncLit",
        "FuncType",
        "Ident",
        "IndexExpr",
    ]
    .into_iter()
    .map(str::to_string)
    .collect::<BTreeSet<_>>();
    for spec in go_ast_inventory_declarative_template_coverage_specs() {
        supported_return_expression_rules.insert(spec.root_node_kind().to_string());
    }
    for covered_rule in covered_rules {
        supported_return_expression_rules.insert(covered_rule.clone());
    }
    let alternative = results
        .alternatives
        .iter()
        .find(|alternative| {
            alternative.terminal == GrammarTerminalKind::Nonterminal
                && !supported_return_expression_rules.contains(alternative.node_kind.as_str())
        })
        .map(|alternative| GoAstInventoryTemplateCoverageBlocker {
            kind: "missing_expression_template_alternative".to_string(),
            rule_name: "ReturnStmt".to_string(),
            field_name: "Results".to_string(),
            alternative: alternative.node_kind.clone(),
            reason: format!(
                "template-backed source contexts do not yet support {}, the first unsupported ReturnStmt.Results alternative in the checked inventory",
                alternative.node_kind
            ),
            next_ready_task: GO_AST_INVENTORY_TEMPLATE_COVERAGE_FRONTIER_BLOCKER_NEXT_READY_TASK
                .to_string(),
        });
    if let Some(blocker) = alternative {
        return Ok(Some(blocker));
    }
    if let Some(policy_gap) = first_unsupported_template_policy_gap(grammar, covered_field_set) {
        let (rule_name, field_name) = policy_gap.field_family.split_once('.').ok_or_else(|| {
            SyntaxBackendBoundaryError::new(
                "go_ast_inventory_template_coverage_frontier.first_missing_template_capability",
                format!(
                    "unsupported template policy field family must be Rule.Field, got {}",
                    policy_gap.field_family
                ),
            )
        })?;
        return Ok(Some(GoAstInventoryTemplateCoverageBlocker {
            kind: "missing_template_field_policy".to_string(),
            rule_name: rule_name.to_string(),
            field_name: field_name.to_string(),
            alternative: policy_gap.policy.clone(),
            reason: format!(
                "all ReturnStmt.Results alternatives are template-covered; the next missing reusable template capability is {} for {}",
                policy_gap.policy, policy_gap.field_family
            ),
            next_ready_task: GO_AST_INVENTORY_TEMPLATE_COVERAGE_FRONTIER_BLOCKER_NEXT_READY_TASK
                .to_string(),
        }));
    }
    first_uncovered_required_template_field_gap(grammar, covered_field_set)
}

fn first_uncovered_required_template_field_gap(
    grammar: &GrammarIr,
    covered_field_set: &BTreeSet<&str>,
) -> Result<Option<GoAstInventoryTemplateCoverageBlocker>, SyntaxBackendBoundaryError> {
    for rule in &grammar.rules {
        for field in &rule.fields {
            let field_family = format!("{}.{}", rule.node_kind, field.name);
            if template_coverage_field_is_deferred_non_core(&rule.node_kind, &field.name) {
                continue;
            }
            if field.required
                && !field.multiple
                && !covered_field_set.contains(field_family.as_str())
            {
                return Ok(Some(GoAstInventoryTemplateCoverageBlocker {
                    kind: "missing_template_required_field_coverage".to_string(),
                    rule_name: rule.node_kind.clone(),
                    field_name: field.name.clone(),
                    alternative: "required_field_coverage".to_string(),
                    reason: format!(
                        "ReturnStmt.Results alternatives and optional/repeated field policies are template-covered; the next missing reusable template capability is required field coverage for {field_family}"
                    ),
                    next_ready_task:
                        GO_AST_INVENTORY_TEMPLATE_COVERAGE_FRONTIER_BLOCKER_NEXT_READY_TASK
                            .to_string(),
                }));
            }
        }
    }
    Ok(None)
}

fn template_coverage_field_is_deferred_non_core(rule_name: &str, field_name: &str) -> bool {
    go_ast_generation_frontier_deferred_non_core_reason(rule_name, Some(field_name)).is_some()
}

fn template_backed_witness_ids_from_scratch_registry() -> Vec<&'static str> {
    go_ast_inventory_scratch_witness_specs()
        .iter()
        .filter(|spec| scratch_witness_spec_is_template_backed(spec.seed_id))
        .map(|spec| spec.seed_id)
        .collect()
}

fn witness_backed_template_synthesis_api_ids_from_scratch_registry() -> Vec<&'static str> {
    go_ast_inventory_scratch_witness_specs()
        .iter()
        .filter(|spec| scratch_witness_spec_is_template_backed(spec.seed_id))
        .filter_map(|spec| spec.inventory_synthesis_api_id)
        .collect()
}

fn template_backed_synthesis_api_ids_from_scratch_registry() -> Vec<&'static str> {
    witness_backed_template_synthesis_api_ids_from_scratch_registry()
}

fn scratch_witness_spec_is_template_backed(seed_id: &str) -> bool {
    go_ast_inventory_scratch_witness_specs()
        .iter()
        .find(|spec| spec.seed_id == seed_id)
        .and_then(|spec| spec.inventory_synthesis_api_id)
        .is_some()
        && seed_id != GO_AST_INVENTORY_SCRATCH_VAR_DECL_SEED_ID
}

fn template_covered_finite_domain_choices(
    grammar: &GrammarIr,
) -> Result<Vec<GoAstInventoryTemplateFiniteDomainChoice>, SyntaxBackendBoundaryError> {
    let mut choices = vec![
        template_finite_domain_choice(
            grammar,
            "BasicLit",
            "Kind",
            "go_token_basic_literal_kinds.INT",
            GrammarTerminalKind::IdentifierPayload,
        )?,
        template_finite_domain_choice(
            grammar,
            "ArrayType",
            "Elt",
            "Ident",
            GrammarTerminalKind::Nonterminal,
        )?,
        template_finite_domain_choice(
            grammar,
            "ArrayType",
            "Len",
            "Ellipsis",
            GrammarTerminalKind::Nonterminal,
        )?,
        template_finite_domain_choice(
            grammar,
            "BinaryExpr",
            "Op",
            "go_token_binary_operator_tokens.ADD",
            GrammarTerminalKind::IdentifierPayload,
        )?,
        template_finite_domain_choice(
            grammar,
            "BinaryExpr",
            "X",
            "BasicLit",
            GrammarTerminalKind::Nonterminal,
        )?,
        template_finite_domain_choice(
            grammar,
            "BinaryExpr",
            "Y",
            "BasicLit",
            GrammarTerminalKind::Nonterminal,
        )?,
        template_finite_domain_choice(
            grammar,
            "CallExpr",
            "Ellipsis",
            "go_syntax_presence_call_expr_ellipsis.ABSENT",
            GrammarTerminalKind::BooleanPayload,
        )?,
        template_finite_domain_choice(
            grammar,
            "CallExpr",
            "Fun",
            "SelectorExpr",
            GrammarTerminalKind::Nonterminal,
        )?,
        template_finite_domain_choice(
            grammar,
            "CompositeLit",
            "Elts",
            "BasicLit",
            GrammarTerminalKind::Nonterminal,
        )?,
        template_finite_domain_choice(
            grammar,
            "CompositeLit",
            "Incomplete",
            "CompositeLit.Incomplete.bool",
            GrammarTerminalKind::BooleanPayload,
        )?,
        template_finite_domain_choice(
            grammar,
            "CompositeLit",
            "Type",
            "ArrayType",
            GrammarTerminalKind::Nonterminal,
        )?,
        template_finite_domain_choice(
            grammar,
            "ChanType",
            "Dir",
            "go_ast_channel_directions.SEND_RECV",
            GrammarTerminalKind::IdentifierPayload,
        )?,
        template_finite_domain_choice(
            grammar,
            "Field",
            "Type",
            "FuncType",
            GrammarTerminalKind::Nonterminal,
        )?,
        template_finite_domain_choice(
            grammar,
            "FuncType",
            "Func",
            "go_syntax_presence_func_type_func.PRESENT",
            GrammarTerminalKind::BooleanPayload,
        )?,
        template_finite_domain_choice(
            grammar,
            "GenDecl",
            "Specs",
            "ImportSpec",
            GrammarTerminalKind::Nonterminal,
        )?,
        template_finite_domain_choice(
            grammar,
            "GenDecl",
            "Specs",
            "TypeSpec",
            GrammarTerminalKind::Nonterminal,
        )?,
        template_finite_domain_choice(
            grammar,
            "GenDecl",
            "Specs",
            "ValueSpec",
            GrammarTerminalKind::Nonterminal,
        )?,
        template_finite_domain_choice(
            grammar,
            "GenDecl",
            "Tok",
            "go_token_declaration_tokens.IMPORT",
            GrammarTerminalKind::IdentifierPayload,
        )?,
        template_finite_domain_choice(
            grammar,
            "GenDecl",
            "Tok",
            "go_token_declaration_tokens.TYPE",
            GrammarTerminalKind::IdentifierPayload,
        )?,
        template_finite_domain_choice(
            grammar,
            "GenDecl",
            "Tok",
            "go_token_declaration_tokens.VAR",
            GrammarTerminalKind::IdentifierPayload,
        )?,
        template_finite_domain_choice(
            grammar,
            "SelectorExpr",
            "Sel",
            "Ident",
            GrammarTerminalKind::Nonterminal,
        )?,
        template_finite_domain_choice(
            grammar,
            "SelectorExpr",
            "X",
            "Ident",
            GrammarTerminalKind::Nonterminal,
        )?,
        template_finite_domain_choice(
            grammar,
            "ValueSpec",
            "Values",
            "CompositeLit",
            GrammarTerminalKind::Nonterminal,
        )?,
        template_finite_domain_choice(
            grammar,
            "ValueSpec",
            "Values",
            "FuncLit",
            GrammarTerminalKind::Nonterminal,
        )?,
        template_finite_domain_choice(
            grammar,
            "IndexExpr",
            "Index",
            "BasicLit",
            GrammarTerminalKind::Nonterminal,
        )?,
        template_finite_domain_choice(
            grammar,
            "IndexExpr",
            "X",
            "Ident",
            GrammarTerminalKind::Nonterminal,
        )?,
        template_finite_domain_choice(
            grammar,
            "ReturnStmt",
            "Results",
            "Ident",
            GrammarTerminalKind::Nonterminal,
        )?,
        template_finite_domain_choice(
            grammar,
            "ReturnStmt",
            "Results",
            "IndexExpr",
            GrammarTerminalKind::Nonterminal,
        )?,
        template_finite_domain_choice(
            grammar,
            "SliceExpr",
            "Slice3",
            "SliceExpr.Slice3.bool",
            GrammarTerminalKind::BooleanPayload,
        )?,
    ];
    for spec in go_ast_inventory_declarative_template_coverage_specs() {
        let checked = check_go_ast_inventory_declarative_expression_spec_from_inventory(
            Path::new(GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH),
            Some(&spec.parent_field),
            &spec.expression,
        )?;
        for requirement in checked.finite_domain_requirements {
            choices.push(template_finite_domain_choice(
                grammar,
                &requirement.rule_name,
                &requirement.field_name,
                &requirement.selected_alternative,
                requirement.terminal,
            )?);
        }
    }
    choices.sort_by(template_finite_domain_choice_cmp);
    let mut unique_choices = Vec::with_capacity(choices.len());
    let mut previous_key: Option<String> = None;
    for choice in choices {
        let key = template_finite_domain_choice_key(&choice);
        if previous_key.as_deref() != Some(key.as_str()) {
            previous_key = Some(key);
            unique_choices.push(choice);
        }
    }
    Ok(unique_choices)
}

fn template_finite_domain_choice(
    grammar: &GrammarIr,
    rule_name: &str,
    field_name: &str,
    alternative: &str,
    terminal_kind: GrammarTerminalKind,
) -> Result<GoAstInventoryTemplateFiniteDomainChoice, SyntaxBackendBoundaryError> {
    let field = grammar
        .rules
        .iter()
        .find(|rule| rule.node_kind == rule_name)
        .and_then(|rule| rule.fields.iter().find(|field| field.name == field_name))
        .ok_or_else(|| {
            SyntaxBackendBoundaryError::new(
                "go_ast_inventory_template_coverage_frontier.covered_finite_domain_choices",
                format!("checked inventory is missing {rule_name}.{field_name}"),
            )
        })?;
    let matched = field
        .alternatives
        .iter()
        .any(|candidate| candidate.node_kind == alternative && candidate.terminal == terminal_kind);
    if !matched {
        return Err(SyntaxBackendBoundaryError::new(
            "go_ast_inventory_template_coverage_frontier.covered_finite_domain_choices",
            format!(
                "checked inventory is missing finite-domain alternative {rule_name}.{field_name}={alternative}"
            ),
        ));
    }
    Ok(GoAstInventoryTemplateFiniteDomainChoice {
        rule_name: rule_name.to_string(),
        field_name: field_name.to_string(),
        alternative: alternative.to_string(),
        terminal_kind,
    })
}

fn go_ast_generation_frontier_report(
    grammar_coverage: &GoAstSchemaGrammarCoverageReport,
    oracle: &GoTokenizerGeneratorManifestDepthOracleCutoverEvidence,
    scratch_generation_gate: &GoTokenizerGeneratorManifestDepthScratchGenerationGate,
) -> Result<GoAstGenerationFrontierReport, SyntaxBackendBoundaryError> {
    let grammar = GoAstSchemaInventoryAdapter::lower_inventory_file(Path::new(
        GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH,
    ))
    .map_err(|error| {
        SyntaxBackendBoundaryError::new(
            "go_ast_generation_frontier.inventory",
            format!("failed to lower checked Go AST inventory for frontier report: {error}"),
        )
    })?;
    require_string(
        "go_ast_generation_frontier.grammar_id",
        &grammar.grammar_id,
        &grammar_coverage.grammar_id,
    )?;

    let witnesses = scratch_generation_gate
        .go_ast_inventory_scratch_seeds
        .iter()
        .map(GoAstGenerationFrontierWitnessRecord::from_scratch_seed)
        .collect::<Vec<_>>();
    let witness_hashes = witnesses
        .iter()
        .map(|witness| GoAstGenerationFrontierWitnessHash {
            witness_id: witness.witness_id.clone(),
            raw_token_ids_sha256: witness.raw_token_ids_sha256.clone(),
            rendered_source_sha256: witness.rendered_source_sha256.clone(),
        })
        .collect::<Vec<_>>();
    let inventory_synthesis_witness_count = witnesses
        .iter()
        .filter(|witness| witness.inventory_synthesis_api_id.is_some())
        .count();
    let first_non_inventory_synthesis_witness_id = witnesses
        .iter()
        .find(|witness| witness.inventory_synthesis_api_id.is_none())
        .map(|witness| witness.witness_id.clone());
    let category_records = go_ast_generation_frontier_category_records(&grammar, &witnesses);
    let ready_count = category_records
        .iter()
        .filter(|record| record.status == GO_AST_GENERATION_FRONTIER_CATEGORY_READY)
        .count();
    let blocked_count = category_records
        .iter()
        .filter(|record| record.status == GO_AST_GENERATION_FRONTIER_CATEGORY_BLOCKED)
        .count();
    let deferred_non_core_count = category_records
        .iter()
        .filter(|record| record.status == GO_AST_GENERATION_FRONTIER_CATEGORY_DEFERRED_NON_CORE)
        .count();
    let first_blocker = category_records
        .iter()
        .find(|record| record.status == GO_AST_GENERATION_FRONTIER_CATEGORY_BLOCKED)
        .and_then(|record| record.first_blocker.clone());
    let status = if blocked_count == 0 {
        GO_AST_GENERATION_FRONTIER_STATUS_READY
    } else {
        GO_AST_GENERATION_FRONTIER_STATUS_BLOCKED
    };
    let grammar_field_family_count = grammar
        .rules
        .iter()
        .map(|rule| rule.fields.len())
        .sum::<usize>();

    let report = GoAstGenerationFrontierReport {
        artifact_kind: GO_AST_GENERATION_FRONTIER_ARTIFACT_KIND.to_string(),
        version: VERSION,
        status: status.to_string(),
        pinned_go_version: grammar_coverage.pinned_go_version.clone(),
        grammar_id: grammar_coverage.grammar_id.clone(),
        inventory_artifact_path: grammar_coverage.inventory_artifact_path.clone(),
        inventory_artifact_sha256: grammar_coverage.inventory_artifact_sha256.clone(),
        render_reparse_authority_source: SCRATCH_GENERATION_GATE_RENDER_AUTHORITY_SOURCE
            .to_string(),
        oracle_cutover_accepted: oracle.accepted,
        oracle_record_count: oracle.record_count,
        oracle_canonical_fidelity_only_gap_count: oracle.canonical_fidelity_only_gap_count,
        canonical_fidelity_gaps_diagnostic_only: scratch_generation_gate
            .canonical_fidelity_gaps_diagnostic_only,
        essential_parser_directives_sidecar_policy: true,
        raw_state_machine_core_boundary_id: grammar_coverage
            .raw_state_machine
            .raw_state_machine_core_boundary_id
            .clone(),
        raw_state_machine_core_boundary: grammar_coverage
            .raw_state_machine
            .raw_state_machine_core_boundary
            .clone(),
        raw_state_machine_compile_ready: grammar_coverage
            .raw_state_machine
            .raw_state_machine_compile_ready,
        initial_state_ready: grammar_coverage.raw_state_machine.initial_state_ready,
        token_layout_vocab_size: grammar_coverage.token_layout.vocab_size,
        token_layout_pad_token: grammar_coverage.token_layout.pad_token,
        grammar_rule_count: grammar.rules.len(),
        grammar_field_family_count,
        category_record_count: category_records.len(),
        category_records,
        ready_count,
        blocked_count,
        deferred_non_core_count,
        first_blocker,
        witnesses,
        witness_hashes,
        inventory_synthesis_witness_count,
        first_non_inventory_synthesis_witness_id,
        no_parser_metadata_at_inference_time: true,
        source_prefix_table_used: false,
        arena_ir_used: false,
        go_ir_used: false,
        bpe_validity_authority_used: false,
        type_correctness_claim_allowed: false,
        typecheck_sidecar_diagnostic_failures: 0,
        stable_backend_claimed: false,
        next_ready_task: GO_AST_GENERATION_FRONTIER_NEXT_READY_TASK.to_string(),
        validation_commands: vec![
            "cargo test --test go_ast_schema_inventory_test".to_string(),
            "cargo test --test syntax_go_source_instance_projection_test go_tokenizer_generator_manifest_depth_report_validates_current_plus_supported_records -- --exact".to_string(),
            "python3 scripts/testing/test-quality-audit.py --changed-from origin/main".to_string(),
            "git diff --check".to_string(),
        ],
        claim_boundary: GO_AST_GENERATION_FRONTIER_CLAIM_BOUNDARY.to_string(),
    };
    report.validate()?;
    Ok(report)
}

fn go_ast_generation_frontier_category_records(
    grammar: &crate::grammar_ir::GrammarIr,
    witnesses: &[GoAstGenerationFrontierWitnessRecord],
) -> Vec<GoAstGenerationFrontierCategoryRecord> {
    let mut records = Vec::new();
    for rule in &grammar.rules {
        let witness_ids = witnesses_covering_node_kind(witnesses, &rule.node_kind);
        records.push(go_ast_generation_frontier_category_record(
            format!("go_ast.rule.{}", rule.node_kind),
            "ast_rule",
            rule.node_kind.clone(),
            None,
            witness_ids,
        ));
        for field in &rule.fields {
            let field_family = format!("{}.{}", rule.node_kind, field.name);
            let witness_ids = witnesses_covering_field_family(witnesses, &field_family);
            records.push(go_ast_generation_frontier_category_record(
                format!("go_ast.field.{}.{}", rule.node_kind, field.name),
                "ast_field_family",
                rule.node_kind.clone(),
                Some(field.name.clone()),
                witness_ids,
            ));
        }
    }
    for placeholder in ["BadDecl", "BadExpr", "BadStmt"] {
        records.push(GoAstGenerationFrontierCategoryRecord {
            category_id: format!("go_ast.parser_error_placeholder.{placeholder}"),
            category_kind: "parser_error_placeholder".to_string(),
            rule_name: placeholder.to_string(),
            field_name: None,
            status: GO_AST_GENERATION_FRONTIER_CATEGORY_DEFERRED_NON_CORE.to_string(),
            witness_ids: Vec::new(),
            first_blocker: None,
            deferred_reason: Some(
                "parser-error placeholder node is outside the valid-Go syntactic generation claim"
                    .to_string(),
            ),
        });
    }
    records
}

fn go_ast_generation_frontier_category_record(
    category_id: String,
    category_kind: &str,
    rule_name: String,
    field_name: Option<String>,
    witness_ids: Vec<String>,
) -> GoAstGenerationFrontierCategoryRecord {
    if let Some(deferred_reason) =
        go_ast_generation_frontier_deferred_non_core_reason(&rule_name, field_name.as_deref())
    {
        return GoAstGenerationFrontierCategoryRecord {
            category_id,
            category_kind: category_kind.to_string(),
            rule_name,
            field_name,
            status: GO_AST_GENERATION_FRONTIER_CATEGORY_DEFERRED_NON_CORE.to_string(),
            witness_ids: Vec::new(),
            first_blocker: None,
            deferred_reason: Some(deferred_reason.to_string()),
        };
    }

    if witness_ids.is_empty() {
        let reason = match &field_name {
            Some(field_name) => format!(
                "no empty-prefix Go AST generation witness currently covers {rule_name}.{field_name}"
            ),
            None => {
                format!("no empty-prefix Go AST generation witness currently covers {rule_name}")
            }
        };
        GoAstGenerationFrontierCategoryRecord {
            category_id: category_id.clone(),
            category_kind: category_kind.to_string(),
            rule_name,
            field_name,
            status: GO_AST_GENERATION_FRONTIER_CATEGORY_BLOCKED.to_string(),
            witness_ids,
            first_blocker: Some(GoAstGenerationFrontierBlocker {
                kind: "missing_empty_prefix_generation_witness".to_string(),
                category_id,
                reason,
                next_ready_task: GO_AST_GENERATION_FRONTIER_BLOCKER_NEXT_READY_TASK.to_string(),
            }),
            deferred_reason: None,
        }
    } else {
        GoAstGenerationFrontierCategoryRecord {
            category_id,
            category_kind: category_kind.to_string(),
            rule_name,
            field_name,
            status: GO_AST_GENERATION_FRONTIER_CATEGORY_READY.to_string(),
            witness_ids,
            first_blocker: None,
            deferred_reason: None,
        }
    }
}

fn go_ast_generation_frontier_deferred_non_core_reason(
    rule_name: &str,
    field_name: Option<&str>,
) -> Option<&'static str> {
    if rule_name == "Package" {
        return Some(
            "go/ast Package is a parser.ParseDir package aggregate outside the single GoSourceFile scratch-generation root",
        );
    }
    if rule_name == "Comment" || rule_name == "CommentGroup" {
        return Some(
            "go/ast comment nodes are nonessential trivia for parser-level generation frontier coverage; essential parser/build directives remain explicit sidecar policy",
        );
    }
    if matches!(field_name, Some("Comments" | "Doc" | "Comment")) {
        return Some(
            "go/ast comment-bearing fields are nonessential trivia for parser-level generation frontier coverage; essential parser/build directives remain explicit sidecar policy",
        );
    }
    None
}

fn witnesses_covering_node_kind(
    witnesses: &[GoAstGenerationFrontierWitnessRecord],
    node_kind: &str,
) -> Vec<String> {
    witnesses
        .iter()
        .filter(|witness| {
            witness
                .covered_node_kinds
                .iter()
                .any(|covered| covered == node_kind)
        })
        .map(|witness| witness.witness_id.clone())
        .collect()
}

fn witnesses_covering_field_family(
    witnesses: &[GoAstGenerationFrontierWitnessRecord],
    field_family: &str,
) -> Vec<String> {
    witnesses
        .iter()
        .filter(|witness| {
            witness
                .covered_field_families
                .iter()
                .any(|covered| covered == field_family)
        })
        .map(|witness| witness.witness_id.clone())
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoTokenizerGeneratorManifestDepthReport {
    pub artifact_kind: String,
    pub version: u32,
    pub tool: String,
    pub status: String,
    pub target: String,
    pub route: String,
    pub report_ingestion_manifest_path: String,
    pub report_ingestion_manifest_sha256: String,
    pub report_ingestion_manifest_artifact_kind: String,
    pub report_ingestion_manifest_tool: String,
    pub corpus_id: String,
    pub source_corpus_manifest_path: String,
    pub renderer_position_policy_summary: GoTokenizerGeneratorRendererPositionPolicySummary,
    pub go_ast_grammar_coverage: GoAstSchemaGrammarCoverageReport,
    pub go_ast_raw_tree_renderer_codegen_coverage: GoAstRawTreeRendererCodegenCoverageReport,
    pub raw_tree_render_attempt_summary:
        GoTokenizerGeneratorManifestDepthRawTreeRenderAttemptSummary,
    pub raw_tree_render_broadening_decision:
        GoTokenizerGeneratorManifestDepthRawTreeRenderBroadeningDecision,
    pub go_raw_tree_ast_printer_oracle_cutover:
        GoTokenizerGeneratorManifestDepthOracleCutoverEvidence,
    pub scratch_generation_gate: GoTokenizerGeneratorManifestDepthScratchGenerationGate,
    pub go_ast_inventory_template_coverage_frontier: GoAstInventoryTemplateCoverageFrontierReport,
    pub go_ast_generation_frontier: GoAstGenerationFrontierReport,
    pub record_count: usize,
    pub supported_record_count: usize,
    pub records: Vec<GoTokenizerGeneratorManifestDepthRecord>,
    pub total_raw_token_count: usize,
    /// Canonical (elided) stream contract and aggregate identity covering
    /// every record; bound to the pinned unified Go grammar artifact.
    pub stream_contract: String,
    pub total_canonical_token_count: usize,
    pub total_forced_token_count: usize,
    pub grammar_artifact_id: String,
    pub grammar_artifact_content_sha256: String,
    pub raw_token_ids_sha256_by_manifest_path: Vec<GoTokenizerGeneratorManifestDepthTokenDigest>,
    pub raw_token_digest_set_sha256: String,
    pub max_raw_token_id: Option<u32>,
    pub token_layout_vocab_size: u32,
    pub token_layout_pad_token: u32,
    /// Vocabulary bound claim from the generator-core report claim stamp
    /// ([`GoSourceRecordManifestDepthReportClaimStamp`]).
    pub raw_tokens_below_vocab_size: bool,
    /// PAD absence claim from the generator-core report claim stamp.
    pub pad_token_absent: bool,
    /// Raw-SM complete claim from the generator-core report claim stamp.
    pub raw_state_machine_complete: bool,
    /// Expanded selected-completion readiness from the report claim stamp
    /// (gate-derived).
    pub selected_completion_smoke_ready: bool,
    /// Elided selected-completion readiness from the report claim stamp.
    pub elided_selected_completion_ready: bool,
    /// Incomplete-prefix rejection readiness from the report claim stamp.
    pub incomplete_prefix_rejection_ready: bool,
    /// Token-only decode/render readiness from the report claim stamp.
    pub token_only_decode_render_ready: bool,
    /// Canonical-render readiness from the report claim stamp (record/sidecar
    /// authority).
    pub canonical_render_ready: bool,
    /// Full pipeline surface from the report claim stamp's nested
    /// [`GoSourceRecordPipelineFieldStamp`].
    pub pipeline: GoTokenizerGeneratorPipeline,
    pub next_ready_task: String,
    pub validation_commands: Vec<String>,
    pub claim_boundary: String,
    pub no_stable_claims: Vec<String>,
}

impl GoTokenizerGeneratorManifestDepthReport {
    pub fn validate(&self) -> Result<(), SyntaxBackendBoundaryError> {
        require_string(
            "artifact_kind",
            &self.artifact_kind,
            MANIFEST_DEPTH_ARTIFACT_KIND,
        )?;
        require_u32("version", self.version, VERSION)?;
        require_string("tool", &self.tool, TOOL)?;
        require_string("status", &self.status, MANIFEST_DEPTH_STATUS)?;
        require_string("target", &self.target, TARGET)?;
        require_string("route", &self.route, ROUTE)?;
        require_non_empty(
            "report_ingestion_manifest_path",
            &self.report_ingestion_manifest_path,
        )?;
        require_sha256(
            "report_ingestion_manifest_sha256",
            &self.report_ingestion_manifest_sha256,
        )?;
        require_string(
            "report_ingestion_manifest_artifact_kind",
            &self.report_ingestion_manifest_artifact_kind,
            REPORT_INGESTION_MANIFEST_ARTIFACT_KIND,
        )?;
        require_string(
            "report_ingestion_manifest_tool",
            &self.report_ingestion_manifest_tool,
            REPORT_INGESTION_MANIFEST_TOOL,
        )?;
        require_non_empty("corpus_id", &self.corpus_id)?;
        require_non_empty(
            "source_corpus_manifest_path",
            &self.source_corpus_manifest_path,
        )?;
        self.renderer_position_policy_summary.validate()?;
        self.go_ast_grammar_coverage.validate().map_err(|error| {
            SyntaxBackendBoundaryError::new(
                "go_ast_grammar_coverage",
                format!("Go AST grammar coverage report is invalid: {error}"),
            )
        })?;
        self.go_ast_raw_tree_renderer_codegen_coverage
            .validate()
            .map_err(|error| {
                SyntaxBackendBoundaryError::new(
                    "go_ast_raw_tree_renderer_codegen_coverage",
                    format!("Go AST raw-tree renderer codegen coverage report is invalid: {error}"),
                )
            })?;
        if self.raw_tree_render_broadening_decision.next_action
            == RAW_TREE_RENDER_BROADENING_GRAMMAR_MODE_ACTION
        {
            require_bool(
                "go_ast_grammar_coverage.inventory_surface_checked",
                self.go_ast_grammar_coverage.inventory_surface_checked,
                true,
            )?;
            match self.go_ast_grammar_coverage.status.as_str() {
                GO_AST_SCHEMA_GRAMMAR_COVERAGE_STATUS_READY => {
                    require_bool(
                        "go_ast_grammar_coverage.raw_state_machine_compile_ready",
                        self.go_ast_grammar_coverage
                            .raw_state_machine
                            .raw_state_machine_compile_ready,
                        true,
                    )?;
                    require_bool(
                        "go_ast_grammar_coverage.initial_state_ready",
                        self.go_ast_grammar_coverage
                            .raw_state_machine
                            .initial_state_ready,
                        true,
                    )?;
                }
                GO_AST_SCHEMA_GRAMMAR_COVERAGE_STATUS_BLOCKED => {
                    if self.go_ast_grammar_coverage.first_blocker.is_none() {
                        return Err(SyntaxBackendBoundaryError::new(
                            "go_ast_grammar_coverage.first_blocker",
                            "grammar-mode broadening requires the Go AST coverage gate to record a precise blocker when the full inventory is not raw-state ready",
                        ));
                    }
                }
                other => {
                    return Err(SyntaxBackendBoundaryError::new(
                        "go_ast_grammar_coverage.status",
                        format!("unsupported Go AST grammar coverage status {other}"),
                    ));
                }
            }
            require_string(
                "go_ast_raw_tree_renderer_codegen_coverage.status",
                &self.go_ast_raw_tree_renderer_codegen_coverage.status,
                GO_AST_RAW_TREE_RENDERER_CODEGEN_COVERAGE_STATUS_READY,
            )?;
            require_usize(
                "go_ast_raw_tree_renderer_codegen_coverage.missing_policy_count",
                self.go_ast_raw_tree_renderer_codegen_coverage
                    .missing_policy_count,
                0,
            )?;
        }
        require_positive_usize("record_count", self.record_count)?;
        require_usize(
            "supported_record_count",
            self.supported_record_count,
            self.records.len(),
        )?;
        require_usize("record_count", self.record_count, self.records.len())?;
        require_usize(
            "raw_token_ids_sha256_by_manifest_path",
            self.raw_token_ids_sha256_by_manifest_path.len(),
            self.records.len(),
        )?;
        let mut total_raw_token_count = 0usize;
        let mut total_canonical_token_count = 0usize;
        let mut total_forced_token_count = 0usize;
        let mut max_raw_token_id = None;
        let mut seen_manifest_paths = BTreeSet::new();
        let mut expected_digests = Vec::new();
        for record in &self.records {
            record.validate()?;
            if !seen_manifest_paths.insert(record.manifest_path.as_str()) {
                return Err(SyntaxBackendBoundaryError::new(
                    "manifest_depth.records",
                    format!("duplicate manifest path {}", record.manifest_path),
                ));
            }
            total_raw_token_count += record.raw_token_count;
            total_canonical_token_count += record.canonical_token_count;
            total_forced_token_count += record.forced_token_count;
            max_raw_token_id = max_optional_token(max_raw_token_id, record.max_raw_token_id);
            expected_digests.push(GoTokenizerGeneratorManifestDepthTokenDigest {
                manifest_path: record.manifest_path.clone(),
                canonical_tokens_sha256: record.canonical_tokens_sha256.clone(),
            });
        }
        self.raw_tree_render_attempt_summary.validate(
            &self.report_ingestion_manifest_path,
            &self.report_ingestion_manifest_sha256,
            &self.records,
        )?;
        self.raw_tree_render_broadening_decision.validate(
            &self.raw_tree_render_attempt_summary,
            &self.records,
            &self.go_ast_grammar_coverage,
            &self.go_ast_raw_tree_renderer_codegen_coverage,
        )?;
        self.go_raw_tree_ast_printer_oracle_cutover
            .validate(&self.raw_tree_render_attempt_summary)?;
        validate_scratch_generation_gate(
            &self.scratch_generation_gate,
            &self.go_raw_tree_ast_printer_oracle_cutover,
            &self.go_ast_grammar_coverage,
            &self.records,
        )?;
        self.go_ast_inventory_template_coverage_frontier
            .validate()?;
        require_string(
            "go_ast_inventory_template_coverage_frontier.inventory_artifact_path",
            &self
                .go_ast_inventory_template_coverage_frontier
                .inventory_artifact_path,
            &self.go_ast_grammar_coverage.inventory_artifact_path,
        )?;
        require_string(
            "go_ast_inventory_template_coverage_frontier.inventory_artifact_sha256",
            &self
                .go_ast_inventory_template_coverage_frontier
                .inventory_artifact_sha256,
            &self.go_ast_grammar_coverage.inventory_artifact_sha256,
        )?;
        self.go_ast_generation_frontier.validate()?;
        require_string(
            "go_ast_generation_frontier.inventory_artifact_path",
            &self.go_ast_generation_frontier.inventory_artifact_path,
            &self.go_ast_grammar_coverage.inventory_artifact_path,
        )?;
        require_string(
            "go_ast_generation_frontier.inventory_artifact_sha256",
            &self.go_ast_generation_frontier.inventory_artifact_sha256,
            &self.go_ast_grammar_coverage.inventory_artifact_sha256,
        )?;
        require_usize(
            "go_ast_generation_frontier.witnesses",
            self.go_ast_generation_frontier.witnesses.len(),
            self.scratch_generation_gate.scratch_seed_count,
        )?;
        require_bool(
            "go_ast_generation_frontier.type_correctness_claim_allowed",
            self.go_ast_generation_frontier
                .type_correctness_claim_allowed,
            false,
        )?;
        require_usize(
            "total_raw_token_count",
            self.total_raw_token_count,
            total_raw_token_count,
        )?;
        require_string(
            "stream_contract",
            &self.stream_contract,
            LIVE_RAW_SYNTAX_STREAM_CONTRACT,
        )?;
        require_usize(
            "total_canonical_token_count",
            self.total_canonical_token_count,
            total_canonical_token_count,
        )?;
        require_usize(
            "total_forced_token_count",
            self.total_forced_token_count,
            total_forced_token_count,
        )?;
        let unified_artifact = go_unified_grammar_artifact()?;
        require_string(
            "grammar_artifact_id",
            &self.grammar_artifact_id,
            unified_artifact.artifact_id,
        )?;
        require_string(
            "grammar_artifact_content_sha256",
            &self.grammar_artifact_content_sha256,
            &unified_artifact.content_sha256,
        )?;
        if self.raw_token_ids_sha256_by_manifest_path != expected_digests {
            return Err(SyntaxBackendBoundaryError::new(
                "raw_token_ids_sha256_by_manifest_path",
                "digest summary must match manifest record order",
            ));
        }
        let digest_set_hash =
            json_sha256(&self.raw_token_ids_sha256_by_manifest_path).map_err(|error| {
                SyntaxBackendBoundaryError::new(
                    "raw_token_digest_set_sha256",
                    format!("failed to hash raw token digest set: {error}"),
                )
            })?;
        require_string(
            "raw_token_digest_set_sha256",
            &self.raw_token_digest_set_sha256,
            &digest_set_hash,
        )?;
        // Report-level claim fields (vocab/SM + gate readiness + canonical-render
        // + full pipeline) must mirror gate/record aggregates through one
        // generator-core stamp (not hardcode independently of those surfaces).
        GoSourceRecordManifestDepthReportClaimStamp::validate_against_gate_and_record_views(
            &self.scratch_generation_gate,
            self.records
                .iter()
                .map(GoTokenizerGeneratorManifestDepthRecord::vocab_sm_completeness_view),
            self.records
                .iter()
                .map(GoTokenizerGeneratorManifestDepthRecord::canonical_render_readiness_view),
            self.raw_tokens_below_vocab_size,
            self.pad_token_absent,
            self.raw_state_machine_complete,
            self.selected_completion_smoke_ready,
            self.elided_selected_completion_ready,
            self.incomplete_prefix_rejection_ready,
            self.token_only_decode_render_ready,
            self.canonical_render_ready,
            self.pipeline.parser_backed_metadata_loaded,
            self.pipeline.grammar_ir_lowered,
            self.pipeline.structural_model_lowered,
            self.pipeline.token_layout_compiled,
            self.pipeline.raw_state_machine_compiled,
            self.pipeline.raw_token_encode_ready,
            self.pipeline.raw_state_machine_decode_ready,
            self.pipeline.canonical_render_ready,
            self.pipeline.go_reparse_ready,
            self.pipeline.arena_ir_used,
            self.pipeline.route_handoff_provenance_used,
            self.pipeline.bpe_validity_authority_used,
        )?;
        // A successful manifest-depth report still requires complete token
        // accounting and render readiness for every supported record.
        require_bool(
            "raw_tokens_below_vocab_size",
            self.raw_tokens_below_vocab_size,
            true,
        )?;
        require_bool("pad_token_absent", self.pad_token_absent, true)?;
        require_bool(
            "raw_state_machine_complete",
            self.raw_state_machine_complete,
            true,
        )?;
        require_bool("canonical_render_ready", self.canonical_render_ready, true)?;
        if self.max_raw_token_id != max_raw_token_id {
            return Err(SyntaxBackendBoundaryError::new(
                "max_raw_token_id",
                "aggregate max raw token ID must match records",
            ));
        }
        if let Some(max_raw_token_id) = self.max_raw_token_id
            && max_raw_token_id >= self.token_layout_vocab_size
        {
            return Err(SyntaxBackendBoundaryError::new(
                "max_raw_token_id",
                "max raw token ID must be below VOCAB_SIZE",
            ));
        }
        require_u32(
            "token_layout_pad_token",
            self.token_layout_pad_token,
            self.token_layout_vocab_size.saturating_sub(1),
        )?;
        self.pipeline.validate()?;
        require_string(
            "next_ready_task",
            &self.next_ready_task,
            expected_manifest_depth_next_ready_task(
                &self.raw_tree_render_broadening_decision,
                &self.go_ast_grammar_coverage,
                &self.go_ast_raw_tree_renderer_codegen_coverage,
            ),
        )?;
        require_contains(
            "validation_commands",
            &self.validation_commands,
            "tct-syntax-go-tokenizer-generator -- --report-ingestion-manifest",
        )?;
        require_contains(
            "validation_commands",
            &self.validation_commands,
            "cargo test --test go_ast_schema_inventory_test",
        )?;
        require_contains(
            "validation_commands",
            &self.validation_commands,
            "test-go-generated-codec-round-trip-gate.sh",
        )?;
        require_contains(
            "claim_boundary",
            std::slice::from_ref(&self.claim_boundary),
            "syntax-level",
        )?;
        require_contains(
            "claim_boundary",
            std::slice::from_ref(&self.claim_boundary),
            "not Arena IR",
        )?;
        require_contains(
            "claim_boundary",
            std::slice::from_ref(&self.claim_boundary),
            "not BPE",
        )?;
        require_all("no_stable_claims", &self.no_stable_claims, NO_STABLE_CLAIMS)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoTokenizerGeneratorManifestDepthRawTreeRenderAttemptSummary {
    pub selection_source: String,
    pub selecting_report_ingestion_manifest_path: String,
    pub selecting_report_ingestion_manifest_sha256: String,
    pub manifest_selected_attempt_count: usize,
    pub ready_count: usize,
    #[serde(default)]
    pub grammar_mode_ready_count: usize,
    #[serde(default)]
    pub canonical_fidelity_ready_count: usize,
    pub diagnostic_count: usize,
    pub attempted_manifest_paths: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoTokenizerGeneratorManifestDepthRawTreeRenderBroadeningDecision {
    pub status: String,
    pub decision_source: String,
    pub selected_attempt_count: usize,
    pub ready_attempt_count: usize,
    #[serde(default)]
    pub grammar_mode_ready_attempt_count: usize,
    #[serde(default)]
    pub canonical_fidelity_ready_attempt_count: usize,
    pub diagnostic_count: usize,
    pub attempted_manifest_paths: Vec<String>,
    pub next_action: String,
    pub next_candidate: Option<GoTokenizerGeneratorManifestDepthRawTreeRenderCandidate>,
    pub first_blocker: Option<GoTokenizerGeneratorManifestDepthRawTreeRenderBlocker>,
    pub manifest_only_attempt_ready: bool,
    pub rust_side_setup_required: bool,
    pub next_ready_task: String,
    pub claim_boundary: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoTokenizerGeneratorManifestDepthRawTreeRenderCandidate {
    pub manifest_path: String,
    pub source_path: String,
    pub source_sha256: String,
    pub checked_projection_report_path: String,
    pub candidate_grammar_id: String,
    pub generated_codec_report_path: String,
    pub raw_token_count: usize,
    pub raw_token_ids_sha256: String,
    pub token_layout_vocab_size: u32,
    pub token_layout_pad_token: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoTokenizerGeneratorManifestDepthRawTreeRenderBlocker {
    pub manifest_path: String,
    pub source_path: String,
    pub diagnostic: GoTokenizerGeneratorRawTreeRendererDiagnostic,
    pub missing_capability: String,
    pub renderer_category: String,
}

impl GoTokenizerGeneratorManifestDepthRawTreeRenderBroadeningDecision {
    fn from_records(
        summary: &GoTokenizerGeneratorManifestDepthRawTreeRenderAttemptSummary,
        records: &[GoTokenizerGeneratorManifestDepthRecord],
    ) -> Self {
        if let Some((blocked_record, diagnostic)) = records.iter().find_map(|record| {
            if !record.raw_tree_render_attempt_selected_by_manifest {
                return None;
            }
            record
                .raw_decoded_tree_render_blocker
                .renderer_diagnostic
                .clone()
                .map(|diagnostic| (record, diagnostic))
        }) {
            return Self {
                status: RAW_TREE_RENDER_BROADENING_DIAGNOSTIC_BLOCKED_STATUS.to_string(),
                decision_source: RAW_TREE_RENDER_BROADENING_DECISION_SOURCE.to_string(),
                selected_attempt_count: summary.manifest_selected_attempt_count,
                ready_attempt_count: summary.ready_count,
                grammar_mode_ready_attempt_count: summary.grammar_mode_ready_count,
                canonical_fidelity_ready_attempt_count: summary.canonical_fidelity_ready_count,
                diagnostic_count: summary.diagnostic_count,
                attempted_manifest_paths: summary.attempted_manifest_paths.clone(),
                next_action: RAW_TREE_RENDER_BROADENING_DIAGNOSTIC_ACTION.to_string(),
                next_candidate: None,
                first_blocker: Some(GoTokenizerGeneratorManifestDepthRawTreeRenderBlocker {
                    manifest_path: blocked_record.manifest_path.clone(),
                    source_path: blocked_record.source_path.clone(),
                    missing_capability: diagnostic.missing_capability.clone(),
                    renderer_category: diagnostic.unsupported_category.clone(),
                    diagnostic,
                }),
                manifest_only_attempt_ready: false,
                rust_side_setup_required: true,
                next_ready_task: RAW_TREE_RENDER_BROADENING_NEXT_READY_TASK.to_string(),
                claim_boundary: RAW_TREE_RENDER_BROADENING_CLAIM_BOUNDARY.to_string(),
            };
        }

        if summary.manifest_selected_attempt_count > 0
            && summary.grammar_mode_ready_count == summary.manifest_selected_attempt_count
        {
            return Self {
                status: RAW_TREE_RENDER_BROADENING_GRAMMAR_MODE_READY_STATUS.to_string(),
                decision_source: RAW_TREE_RENDER_BROADENING_DECISION_SOURCE.to_string(),
                selected_attempt_count: summary.manifest_selected_attempt_count,
                ready_attempt_count: summary.ready_count,
                grammar_mode_ready_attempt_count: summary.grammar_mode_ready_count,
                canonical_fidelity_ready_attempt_count: summary.canonical_fidelity_ready_count,
                diagnostic_count: summary.diagnostic_count,
                attempted_manifest_paths: summary.attempted_manifest_paths.clone(),
                next_action: RAW_TREE_RENDER_BROADENING_GRAMMAR_MODE_ACTION.to_string(),
                next_candidate: None,
                first_blocker: None,
                manifest_only_attempt_ready: false,
                rust_side_setup_required: false,
                next_ready_task: RAW_TREE_RENDER_BROADENING_NEXT_READY_TASK.to_string(),
                claim_boundary: RAW_TREE_RENDER_BROADENING_CLAIM_BOUNDARY.to_string(),
            };
        }

        if let Some(candidate_record) = records
            .iter()
            .find(|record| !record.raw_tree_render_attempt_selected_by_manifest)
        {
            return Self {
                status: RAW_TREE_RENDER_BROADENING_NEXT_CANDIDATE_STATUS.to_string(),
                decision_source: RAW_TREE_RENDER_BROADENING_DECISION_SOURCE.to_string(),
                selected_attempt_count: summary.manifest_selected_attempt_count,
                ready_attempt_count: summary.ready_count,
                grammar_mode_ready_attempt_count: summary.grammar_mode_ready_count,
                canonical_fidelity_ready_attempt_count: summary.canonical_fidelity_ready_count,
                diagnostic_count: summary.diagnostic_count,
                attempted_manifest_paths: summary.attempted_manifest_paths.clone(),
                next_action: RAW_TREE_RENDER_BROADENING_MANIFEST_ONLY_ACTION.to_string(),
                next_candidate: Some(
                    GoTokenizerGeneratorManifestDepthRawTreeRenderCandidate::from_record(
                        candidate_record,
                    ),
                ),
                first_blocker: None,
                manifest_only_attempt_ready: true,
                rust_side_setup_required: false,
                next_ready_task: RAW_TREE_RENDER_BROADENING_NEXT_READY_TASK.to_string(),
                claim_boundary: RAW_TREE_RENDER_BROADENING_CLAIM_BOUNDARY.to_string(),
            };
        }

        Self {
            status: RAW_TREE_RENDER_BROADENING_NO_CANDIDATE_STATUS.to_string(),
            decision_source: RAW_TREE_RENDER_BROADENING_DECISION_SOURCE.to_string(),
            selected_attempt_count: summary.manifest_selected_attempt_count,
            ready_attempt_count: summary.ready_count,
            grammar_mode_ready_attempt_count: summary.grammar_mode_ready_count,
            canonical_fidelity_ready_attempt_count: summary.canonical_fidelity_ready_count,
            diagnostic_count: summary.diagnostic_count,
            attempted_manifest_paths: summary.attempted_manifest_paths.clone(),
            next_action: RAW_TREE_RENDER_BROADENING_NO_CANDIDATE_ACTION.to_string(),
            next_candidate: None,
            first_blocker: None,
            manifest_only_attempt_ready: false,
            rust_side_setup_required: true,
            next_ready_task: RAW_TREE_RENDER_BROADENING_NEXT_READY_TASK.to_string(),
            claim_boundary: RAW_TREE_RENDER_BROADENING_CLAIM_BOUNDARY.to_string(),
        }
    }

    fn validate(
        &self,
        summary: &GoTokenizerGeneratorManifestDepthRawTreeRenderAttemptSummary,
        records: &[GoTokenizerGeneratorManifestDepthRecord],
        go_ast_grammar_coverage: &GoAstSchemaGrammarCoverageReport,
        go_ast_raw_tree_renderer_codegen_coverage: &GoAstRawTreeRendererCodegenCoverageReport,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        require_string(
            "manifest_depth.raw_tree_render_broadening_decision.decision_source",
            &self.decision_source,
            RAW_TREE_RENDER_BROADENING_DECISION_SOURCE,
        )?;
        require_usize(
            "manifest_depth.raw_tree_render_broadening_decision.selected_attempt_count",
            self.selected_attempt_count,
            summary.manifest_selected_attempt_count,
        )?;
        require_usize(
            "manifest_depth.raw_tree_render_broadening_decision.ready_attempt_count",
            self.ready_attempt_count,
            summary.ready_count,
        )?;
        require_usize(
            "manifest_depth.raw_tree_render_broadening_decision.grammar_mode_ready_attempt_count",
            self.grammar_mode_ready_attempt_count,
            summary.grammar_mode_ready_count,
        )?;
        require_usize(
            "manifest_depth.raw_tree_render_broadening_decision.canonical_fidelity_ready_attempt_count",
            self.canonical_fidelity_ready_attempt_count,
            summary.canonical_fidelity_ready_count,
        )?;
        require_usize(
            "manifest_depth.raw_tree_render_broadening_decision.diagnostic_count",
            self.diagnostic_count,
            summary.diagnostic_count,
        )?;
        if self.attempted_manifest_paths != summary.attempted_manifest_paths {
            return Err(SyntaxBackendBoundaryError::new(
                "manifest_depth.raw_tree_render_broadening_decision.attempted_manifest_paths",
                "broadening decision attempted paths must match raw-tree render attempt summary",
            ));
        }
        require_string(
            "manifest_depth.raw_tree_render_broadening_decision.next_ready_task",
            &self.next_ready_task,
            expected_raw_tree_render_broadening_next_ready_task(
                self,
                go_ast_grammar_coverage,
                go_ast_raw_tree_renderer_codegen_coverage,
            ),
        )?;
        require_str_contains(
            "manifest_depth.raw_tree_render_broadening_decision.claim_boundary",
            &self.claim_boundary,
            "syntax-level",
        )?;
        require_str_contains(
            "manifest_depth.raw_tree_render_broadening_decision.claim_boundary",
            &self.claim_boundary,
            "not Arena IR",
        )?;
        require_str_contains(
            "manifest_depth.raw_tree_render_broadening_decision.claim_boundary",
            &self.claim_boundary,
            "not BPE",
        )?;

        let expected_blocker = records
            .iter()
            .find_map(GoTokenizerGeneratorManifestDepthRawTreeRenderBlocker::from_record);
        if let Some(expected_blocker) = expected_blocker {
            require_string(
                "manifest_depth.raw_tree_render_broadening_decision.status",
                &self.status,
                RAW_TREE_RENDER_BROADENING_DIAGNOSTIC_BLOCKED_STATUS,
            )?;
            require_string(
                "manifest_depth.raw_tree_render_broadening_decision.next_action",
                &self.next_action,
                RAW_TREE_RENDER_BROADENING_DIAGNOSTIC_ACTION,
            )?;
            if self.first_blocker.as_ref() != Some(&expected_blocker) {
                return Err(SyntaxBackendBoundaryError::new(
                    "manifest_depth.raw_tree_render_broadening_decision.first_blocker",
                    "broadening blocker must match the first manifest-selected renderer diagnostic",
                ));
            }
            require_none(
                "manifest_depth.raw_tree_render_broadening_decision.next_candidate",
                self.next_candidate
                    .as_ref()
                    .map(|candidate| candidate.manifest_path.as_str()),
            )?;
            require_bool(
                "manifest_depth.raw_tree_render_broadening_decision.manifest_only_attempt_ready",
                self.manifest_only_attempt_ready,
                false,
            )?;
            return require_bool(
                "manifest_depth.raw_tree_render_broadening_decision.rust_side_setup_required",
                self.rust_side_setup_required,
                true,
            );
        }

        if summary.manifest_selected_attempt_count > 0
            && summary.grammar_mode_ready_count == summary.manifest_selected_attempt_count
        {
            require_string(
                "manifest_depth.raw_tree_render_broadening_decision.status",
                &self.status,
                RAW_TREE_RENDER_BROADENING_GRAMMAR_MODE_READY_STATUS,
            )?;
            require_string(
                "manifest_depth.raw_tree_render_broadening_decision.next_action",
                &self.next_action,
                RAW_TREE_RENDER_BROADENING_GRAMMAR_MODE_ACTION,
            )?;
            require_none(
                "manifest_depth.raw_tree_render_broadening_decision.next_candidate",
                self.next_candidate
                    .as_ref()
                    .map(|candidate| candidate.manifest_path.as_str()),
            )?;
            require_none(
                "manifest_depth.raw_tree_render_broadening_decision.first_blocker",
                self.first_blocker
                    .as_ref()
                    .map(|blocker| blocker.manifest_path.as_str()),
            )?;
            require_bool(
                "manifest_depth.raw_tree_render_broadening_decision.manifest_only_attempt_ready",
                self.manifest_only_attempt_ready,
                false,
            )?;
            return require_bool(
                "manifest_depth.raw_tree_render_broadening_decision.rust_side_setup_required",
                self.rust_side_setup_required,
                false,
            );
        }

        let expected_candidate = records
            .iter()
            .find(|record| !record.raw_tree_render_attempt_selected_by_manifest)
            .map(GoTokenizerGeneratorManifestDepthRawTreeRenderCandidate::from_record);
        if let Some(expected_candidate) = expected_candidate {
            require_string(
                "manifest_depth.raw_tree_render_broadening_decision.status",
                &self.status,
                RAW_TREE_RENDER_BROADENING_NEXT_CANDIDATE_STATUS,
            )?;
            require_string(
                "manifest_depth.raw_tree_render_broadening_decision.next_action",
                &self.next_action,
                RAW_TREE_RENDER_BROADENING_MANIFEST_ONLY_ACTION,
            )?;
            if self.next_candidate.as_ref() != Some(&expected_candidate) {
                return Err(SyntaxBackendBoundaryError::new(
                    "manifest_depth.raw_tree_render_broadening_decision.next_candidate",
                    "broadening candidate must match the first unattempted manifest-depth record",
                ));
            }
            require_none(
                "manifest_depth.raw_tree_render_broadening_decision.first_blocker",
                self.first_blocker
                    .as_ref()
                    .map(|blocker| blocker.manifest_path.as_str()),
            )?;
            require_bool(
                "manifest_depth.raw_tree_render_broadening_decision.manifest_only_attempt_ready",
                self.manifest_only_attempt_ready,
                true,
            )?;
            return require_bool(
                "manifest_depth.raw_tree_render_broadening_decision.rust_side_setup_required",
                self.rust_side_setup_required,
                false,
            );
        }

        require_string(
            "manifest_depth.raw_tree_render_broadening_decision.status",
            &self.status,
            RAW_TREE_RENDER_BROADENING_NO_CANDIDATE_STATUS,
        )?;
        require_string(
            "manifest_depth.raw_tree_render_broadening_decision.next_action",
            &self.next_action,
            RAW_TREE_RENDER_BROADENING_NO_CANDIDATE_ACTION,
        )?;
        require_none(
            "manifest_depth.raw_tree_render_broadening_decision.next_candidate",
            self.next_candidate
                .as_ref()
                .map(|candidate| candidate.manifest_path.as_str()),
        )?;
        require_none(
            "manifest_depth.raw_tree_render_broadening_decision.first_blocker",
            self.first_blocker
                .as_ref()
                .map(|blocker| blocker.manifest_path.as_str()),
        )?;
        require_bool(
            "manifest_depth.raw_tree_render_broadening_decision.manifest_only_attempt_ready",
            self.manifest_only_attempt_ready,
            false,
        )?;
        require_bool(
            "manifest_depth.raw_tree_render_broadening_decision.rust_side_setup_required",
            self.rust_side_setup_required,
            true,
        )
    }
}

impl GoTokenizerGeneratorManifestDepthRawTreeRenderCandidate {
    fn from_record(record: &GoTokenizerGeneratorManifestDepthRecord) -> Self {
        Self {
            manifest_path: record.manifest_path.clone(),
            source_path: record.source_path.clone(),
            source_sha256: record.source_sha256.clone(),
            checked_projection_report_path: record.checked_projection_report_path.clone(),
            candidate_grammar_id: record.candidate_grammar_id.clone(),
            generated_codec_report_path: record.generated_codec_report_path.clone(),
            raw_token_count: record.raw_token_count,
            raw_token_ids_sha256: record.raw_token_ids_sha256.clone(),
            token_layout_vocab_size: record.token_layout_vocab_size,
            token_layout_pad_token: record.token_layout_pad_token,
        }
    }
}

impl GoTokenizerGeneratorManifestDepthRawTreeRenderBlocker {
    fn from_record(record: &GoTokenizerGeneratorManifestDepthRecord) -> Option<Self> {
        if !record.raw_tree_render_attempt_selected_by_manifest {
            return None;
        }
        let diagnostic = record
            .raw_decoded_tree_render_blocker
            .renderer_diagnostic
            .clone()?;
        Some(Self {
            manifest_path: record.manifest_path.clone(),
            source_path: record.source_path.clone(),
            missing_capability: diagnostic.missing_capability.clone(),
            renderer_category: diagnostic.unsupported_category.clone(),
            diagnostic,
        })
    }
}

fn expected_raw_tree_render_broadening_next_ready_task<'a>(
    decision: &GoTokenizerGeneratorManifestDepthRawTreeRenderBroadeningDecision,
    _go_ast_grammar_coverage: &'a GoAstSchemaGrammarCoverageReport,
    _go_ast_raw_tree_renderer_codegen_coverage: &'a GoAstRawTreeRendererCodegenCoverageReport,
) -> &'a str {
    if decision.next_action == RAW_TREE_RENDER_BROADENING_GRAMMAR_MODE_ACTION {
        GO_RAW_TREE_AST_PRINTER_ORACLE_CUTOVER_NEXT_READY_TASK
    } else {
        RAW_TREE_RENDER_BROADENING_NEXT_READY_TASK
    }
}

fn expected_manifest_depth_next_ready_task<'a>(
    decision: &'a GoTokenizerGeneratorManifestDepthRawTreeRenderBroadeningDecision,
    _go_ast_grammar_coverage: &'a GoAstSchemaGrammarCoverageReport,
    _go_ast_raw_tree_renderer_codegen_coverage: &'a GoAstRawTreeRendererCodegenCoverageReport,
) -> &'a str {
    if decision.next_action == RAW_TREE_RENDER_BROADENING_GRAMMAR_MODE_ACTION {
        SCRATCH_GENERATION_GATE_NEXT_READY_TASK
    } else {
        decision.next_ready_task.as_str()
    }
}

impl GoTokenizerGeneratorManifestDepthRawTreeRenderAttemptSummary {
    fn from_records(
        report_ingestion_manifest_path: &str,
        report_ingestion_manifest_sha256: &str,
        records: &[GoTokenizerGeneratorManifestDepthRecord],
    ) -> Self {
        let mut attempted_manifest_paths = Vec::new();
        let mut ready_count = 0usize;
        let mut grammar_mode_ready_count = 0usize;
        let mut canonical_fidelity_ready_count = 0usize;
        let mut diagnostic_count = 0usize;
        for record in records {
            if !record.raw_tree_render_attempt_selected_by_manifest {
                continue;
            }
            attempted_manifest_paths.push(record.manifest_path.clone());
            if record.raw_decoded_tree_render_blocker.status == RAW_DECODED_TREE_RENDER_READY_STATUS
            {
                ready_count += 1;
            }
            if record
                .raw_decoded_tree_render_blocker
                .raw_tree_grammar_mode_render_ready
            {
                grammar_mode_ready_count += 1;
            }
            if record
                .raw_decoded_tree_render_blocker
                .raw_tree_canonical_fidelity_ready
            {
                canonical_fidelity_ready_count += 1;
            }
            if record
                .raw_decoded_tree_render_blocker
                .renderer_diagnostic
                .is_some()
            {
                diagnostic_count += 1;
            }
        }
        Self {
            selection_source: RAW_TREE_RENDER_ATTEMPT_SELECTION_SOURCE.to_string(),
            selecting_report_ingestion_manifest_path: report_ingestion_manifest_path.to_string(),
            selecting_report_ingestion_manifest_sha256: report_ingestion_manifest_sha256
                .to_string(),
            manifest_selected_attempt_count: attempted_manifest_paths.len(),
            ready_count,
            grammar_mode_ready_count,
            canonical_fidelity_ready_count,
            diagnostic_count,
            attempted_manifest_paths,
        }
    }

    fn validate(
        &self,
        report_ingestion_manifest_path: &str,
        report_ingestion_manifest_sha256: &str,
        records: &[GoTokenizerGeneratorManifestDepthRecord],
    ) -> Result<(), SyntaxBackendBoundaryError> {
        require_string(
            "manifest_depth.raw_tree_render_attempt_summary.selection_source",
            &self.selection_source,
            RAW_TREE_RENDER_ATTEMPT_SELECTION_SOURCE,
        )?;
        require_string(
            "manifest_depth.raw_tree_render_attempt_summary.selecting_report_ingestion_manifest_path",
            &self.selecting_report_ingestion_manifest_path,
            report_ingestion_manifest_path,
        )?;
        require_string(
            "manifest_depth.raw_tree_render_attempt_summary.selecting_report_ingestion_manifest_sha256",
            &self.selecting_report_ingestion_manifest_sha256,
            report_ingestion_manifest_sha256,
        )?;

        let mut expected_attempted_manifest_paths = Vec::new();
        let mut expected_ready_count = 0usize;
        let mut expected_grammar_mode_ready_count = 0usize;
        let mut expected_canonical_fidelity_ready_count = 0usize;
        let mut expected_diagnostic_count = 0usize;
        for record in records {
            let selected = record.raw_tree_render_attempt_selected_by_manifest;
            let blocker = &record.raw_decoded_tree_render_blocker;
            require_bool(
                "manifest_depth.raw_tree_render_attempt_summary.record_attempted",
                blocker.raw_tree_render_attempted,
                selected,
            )?;
            if !selected {
                continue;
            }
            require_string(
                "manifest_depth.raw_tree_render_attempt_summary.record_manifest_path",
                blocker
                    .raw_tree_render_attempt_manifest_path
                    .as_deref()
                    .unwrap_or(""),
                report_ingestion_manifest_path,
            )?;
            require_string(
                "manifest_depth.raw_tree_render_attempt_summary.record_manifest_sha256",
                blocker
                    .raw_tree_render_attempt_manifest_sha256
                    .as_deref()
                    .unwrap_or(""),
                report_ingestion_manifest_sha256,
            )?;
            expected_attempted_manifest_paths.push(record.manifest_path.clone());
            if blocker.status == RAW_DECODED_TREE_RENDER_READY_STATUS {
                expected_ready_count += 1;
            }
            if blocker.raw_tree_grammar_mode_render_ready {
                expected_grammar_mode_ready_count += 1;
            }
            if blocker.raw_tree_canonical_fidelity_ready {
                expected_canonical_fidelity_ready_count += 1;
            }
            if blocker.renderer_diagnostic.is_some() {
                expected_diagnostic_count += 1;
            }
        }
        require_usize(
            "manifest_depth.raw_tree_render_attempt_summary.manifest_selected_attempt_count",
            self.manifest_selected_attempt_count,
            expected_attempted_manifest_paths.len(),
        )?;
        require_usize(
            "manifest_depth.raw_tree_render_attempt_summary.ready_count",
            self.ready_count,
            expected_ready_count,
        )?;
        require_usize(
            "manifest_depth.raw_tree_render_attempt_summary.grammar_mode_ready_count",
            self.grammar_mode_ready_count,
            expected_grammar_mode_ready_count,
        )?;
        require_usize(
            "manifest_depth.raw_tree_render_attempt_summary.canonical_fidelity_ready_count",
            self.canonical_fidelity_ready_count,
            expected_canonical_fidelity_ready_count,
        )?;
        require_usize(
            "manifest_depth.raw_tree_render_attempt_summary.diagnostic_count",
            self.diagnostic_count,
            expected_diagnostic_count,
        )?;
        if self.attempted_manifest_paths != expected_attempted_manifest_paths {
            return Err(SyntaxBackendBoundaryError::new(
                "manifest_depth.raw_tree_render_attempt_summary.attempted_manifest_paths",
                "attempted manifest paths must match manifest-selected record order",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoTokenizerGeneratorManifestDepthRecord {
    pub role: String,
    pub evidence_scope: String,
    pub manifest_path: String,
    pub source_path: String,
    pub source_sha256: String,
    pub metadata_path: String,
    pub metadata_sha256: String,
    pub checked_projection_report_path: String,
    pub candidate_grammar_id: String,
    pub source_record_generator_core_api_id: String,
    pub source_record_generator_core_api_boundary: String,
    pub source_record_raw_token_emission_api_id: String,
    pub source_record_raw_token_emission_api_boundary: String,
    pub delegated_raw_scratch_generator_core_api_id: String,
    pub delegated_raw_scratch_generator_core_api_boundary: String,
    pub generated_codec_report_path: String,
    pub generated_codec_report_artifact_kind: String,
    pub generated_codec_route: String,
    pub expected_tct_token_count: usize,
    /// Canonical (elided) stream identity; matches the manifest
    /// `expected_tct_*` expectations.
    pub stream_contract: String,
    pub canonical_token_count: usize,
    pub canonical_tokens_sha256: String,
    pub canonical_max_token_id: Option<u32>,
    pub forced_token_count: usize,
    /// Expanded (state-machine-native) stream evidence.
    pub raw_token_count: usize,
    pub expected_tct_tokens_sha256: String,
    pub raw_token_ids_sha256: String,
    pub expected_max_raw_token_id: Option<u32>,
    pub max_raw_token_id: Option<u32>,
    pub expected_vocab_size: u32,
    pub token_layout_vocab_size: u32,
    pub expected_pad_token: u32,
    pub token_layout_pad_token: u32,
    pub expected_pad_token_absent: bool,
    pub pad_token_absent: bool,
    pub raw_tokens_below_vocab_size: bool,
    pub raw_state_machine_complete: bool,
    pub raw_state_machine_decode: GoTokenizerGeneratorDecodeSummary,
    pub raw_state_machine_tree: GoTokenizerGeneratorDecodedTreeSummary,
    pub empty_prefix_generation: GoTokenizerGeneratorEmptyPrefixGenerationEvidence,
    pub selected_completion_smoke: GoTokenizerGeneratorSelectedCompletionSmokeEvidence,
    /// Source-record elided selected-completion (generator-core stamped).
    pub elided_selected_completion: GoSourceRecordElidedSelectedCompletionEvidence,
    pub incomplete_prefix_rejection: GoTokenizerGeneratorIncompletePrefixRejectionEvidence,
    pub token_only_decode_render: GoTokenizerGeneratorTokenOnlyDecodeRenderEvidence,
    pub raw_tree_render_attempt_selected_by_manifest: bool,
    pub raw_decoded_tree_render_blocker: GoTokenizerGeneratorRawDecodedTreeRenderBlocker,
    pub replay_report_complete: bool,
    pub replay_report_prefix_state_api_ready: bool,
    pub canonical_render_ready: bool,
    pub gofmt_validation: String,
    pub go_parser_reparse_validation: String,
    pub source_program_validity_claim_allowed: bool,
    pub stable_promotion_allowed: bool,
}

impl GoTokenizerGeneratorManifestDepthRecord {
    /// Borrowed readiness view for generator-core per-record aggregation.
    pub fn readiness_view(&self) -> GoSourceRecordManifestDepthRecordReadinessView<'_> {
        GoSourceRecordManifestDepthRecordReadinessView {
            empty_prefix_generation: &self.empty_prefix_generation,
            selected_completion_smoke: &self.selected_completion_smoke,
            elided_selected_completion: &self.elided_selected_completion,
            incomplete_prefix_rejection: &self.incomplete_prefix_rejection,
            token_only_decode_render: &self.token_only_decode_render,
            gofmt_validation: &self.gofmt_validation,
            go_parser_reparse_validation: &self.go_parser_reparse_validation,
            source_program_validity_claim_allowed: self.source_program_validity_claim_allowed,
            stable_promotion_allowed: self.stable_promotion_allowed,
        }
    }

    /// Borrowed vocabulary / raw-SM completeness view for report-level aggregation.
    pub fn vocab_sm_completeness_view(&self) -> GoSourceRecordManifestDepthVocabSmCompletenessView {
        GoSourceRecordManifestDepthVocabSmCompletenessView {
            raw_tokens_below_vocab_size: self.raw_tokens_below_vocab_size,
            pad_token_absent: self.pad_token_absent,
            raw_state_machine_complete: self.raw_state_machine_complete,
            max_raw_token_id: self.max_raw_token_id,
            token_layout_vocab_size: self.token_layout_vocab_size,
            token_layout_pad_token: self.token_layout_pad_token,
        }
    }

    /// Borrowed canonical-render / go-reparse readiness view for report-level aggregation.
    pub fn canonical_render_readiness_view(
        &self,
    ) -> GoSourceRecordManifestDepthCanonicalRenderReadinessView {
        GoSourceRecordManifestDepthCanonicalRenderReadinessView {
            canonical_render_ready: self.canonical_render_ready,
            gofmt_passed: self.gofmt_validation == "passed",
            go_parser_reparse_passed: self.go_parser_reparse_validation == "passed",
        }
    }

    /// Borrowed representative paths for generator-core scratch-generation gate assembly.
    pub fn representative_view(&self) -> GoSourceRecordManifestDepthRepresentativeView<'_> {
        GoSourceRecordManifestDepthRepresentativeView {
            manifest_path: &self.manifest_path,
            source_path: &self.source_path,
            raw_token_ids_sha256: &self.raw_token_ids_sha256,
        }
    }

    fn validate(&self) -> Result<(), SyntaxBackendBoundaryError> {
        require_string(
            "manifest_depth.record.role",
            &self.role,
            CURRENT_PLUS_GENERATED_CODEC_ROLE,
        )?;
        require_string(
            "manifest_depth.record.evidence_scope",
            &self.evidence_scope,
            SOURCE_INSTANCE_GENERATED_CODEC_EVIDENCE_SCOPE,
        )?;
        require_non_empty("manifest_depth.record.manifest_path", &self.manifest_path)?;
        require_non_empty("manifest_depth.record.source_path", &self.source_path)?;
        require_sha256("manifest_depth.record.source_sha256", &self.source_sha256)?;
        require_non_empty("manifest_depth.record.metadata_path", &self.metadata_path)?;
        require_sha256(
            "manifest_depth.record.metadata_sha256",
            &self.metadata_sha256,
        )?;
        require_non_empty(
            "manifest_depth.record.checked_projection_report_path",
            &self.checked_projection_report_path,
        )?;
        require_non_empty(
            "manifest_depth.record.candidate_grammar_id",
            &self.candidate_grammar_id,
        )?;
        // Forward gate (M1): the manifest-depth gate accepts only the unified
        // inventory grammar id. Retired per-file go_*_source_file_syntax_v0
        // registrations stay historical evidence behind the checked-report
        // per_file_grammar_blocker escape hatch and must not enter the
        // forward manifest.
        if self.candidate_grammar_id != GO_UNIFIED_GRAMMAR_ID {
            return Err(SyntaxBackendBoundaryError::new(
                "manifest_depth.record.candidate_grammar_id",
                format!(
                    "manifest-depth record {} carries grammar id {}; the forward gate accepts \
                     only the unified inventory grammar id {GO_UNIFIED_GRAMMAR_ID} pinned by \
                     grammar artifact {GO_UNIFIED_GRAMMAR_ARTIFACT_ID}; retired per-file \
                     go_*_source_file_syntax_v0 registrations stay historical evidence behind \
                     the checked-report per_file_grammar_blocker escape hatch",
                    self.manifest_path, self.candidate_grammar_id
                ),
            ));
        }
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
        require_non_empty(
            "manifest_depth.record.generated_codec_report_path",
            &self.generated_codec_report_path,
        )?;
        require_string(
            "manifest_depth.record.generated_codec_report_artifact_kind",
            &self.generated_codec_report_artifact_kind,
            GENERATED_CODEC_REPORT_ARTIFACT_KIND,
        )?;
        require_string(
            "manifest_depth.record.generated_codec_route",
            &self.generated_codec_route,
            GENERATED_CODEC_REPORT_ROUTE,
        )?;
        require_positive_usize(
            "manifest_depth.record.expected_tct_token_count",
            self.expected_tct_token_count,
        )?;
        require_string(
            "manifest_depth.record.stream_contract",
            &self.stream_contract,
            LIVE_RAW_SYNTAX_STREAM_CONTRACT,
        )?;
        require_usize(
            "manifest_depth.record.canonical_token_count",
            self.canonical_token_count,
            self.expected_tct_token_count,
        )?;
        require_usize(
            "manifest_depth.record.forced_token_count",
            self.forced_token_count,
            self.raw_token_count - self.canonical_token_count,
        )?;
        require_sha256(
            "manifest_depth.record.expected_tct_tokens_sha256",
            &self.expected_tct_tokens_sha256,
        )?;
        require_string(
            "manifest_depth.record.canonical_tokens_sha256",
            &self.canonical_tokens_sha256,
            &self.expected_tct_tokens_sha256,
        )?;
        require_sha256(
            "manifest_depth.record.raw_token_ids_sha256",
            &self.raw_token_ids_sha256,
        )?;
        if self.canonical_max_token_id != self.expected_max_raw_token_id {
            return Err(SyntaxBackendBoundaryError::new(
                "manifest_depth.record.canonical_max_token_id",
                "canonical max token ID must match report-ingestion expectation",
            ));
        }
        require_u32(
            "manifest_depth.record.token_layout_vocab_size",
            self.token_layout_vocab_size,
            self.expected_vocab_size,
        )?;
        require_u32(
            "manifest_depth.record.token_layout_pad_token",
            self.token_layout_pad_token,
            self.expected_pad_token,
        )?;
        require_bool(
            "manifest_depth.record.expected_pad_token_absent",
            self.expected_pad_token_absent,
            true,
        )?;
        require_bool(
            "manifest_depth.record.pad_token_absent",
            self.pad_token_absent,
            self.expected_pad_token_absent,
        )?;
        require_bool(
            "manifest_depth.record.raw_tokens_below_vocab_size",
            self.raw_tokens_below_vocab_size,
            true,
        )?;
        require_bool(
            "manifest_depth.record.raw_state_machine_complete",
            self.raw_state_machine_complete,
            true,
        )?;
        self.raw_state_machine_decode
            .validate(self.raw_token_count)?;
        self.raw_state_machine_tree.validate(self.raw_token_count)?;
        self.empty_prefix_generation.validate(
            &self.candidate_grammar_id,
            &self.metadata_path,
            &self.metadata_sha256,
            &self
                .empty_prefix_generation
                .precompiled_artifact_structural_model_id,
            self.token_layout_vocab_size,
            self.token_layout_pad_token,
        )?;
        self.selected_completion_smoke.validate(
            &self.candidate_grammar_id,
            &self.metadata_path,
            &self.metadata_sha256,
            &self
                .selected_completion_smoke
                .precompiled_artifact_structural_model_id,
            self.token_layout_vocab_size,
            self.token_layout_pad_token,
            self.raw_token_count,
            &self.raw_token_ids_sha256,
            &self.source_sha256,
        )?;
        self.elided_selected_completion
            .validate_against_stream_identity(
                &self.stream_contract,
                self.canonical_token_count,
                &self.canonical_tokens_sha256,
                self.canonical_max_token_id,
                self.raw_token_count,
                self.forced_token_count,
            )?;
        self.incomplete_prefix_rejection.validate(
            &self.candidate_grammar_id,
            &self.metadata_path,
            &self.metadata_sha256,
            &self
                .incomplete_prefix_rejection
                .precompiled_artifact_structural_model_id,
            self.token_layout_vocab_size,
            self.token_layout_pad_token,
            self.raw_token_count,
            &self.raw_token_ids_sha256,
        )?;
        self.token_only_decode_render.validate(
            &self.candidate_grammar_id,
            &self.metadata_path,
            &self.metadata_sha256,
            &self
                .token_only_decode_render
                .precompiled_artifact_structural_model_id,
            self.token_layout_vocab_size,
            self.token_layout_pad_token,
            self.raw_token_count,
            &self.raw_token_ids_sha256,
            &self.source_sha256,
        )?;
        self.raw_decoded_tree_render_blocker.validate(
            self.raw_token_count,
            &self.raw_token_ids_sha256,
            &self.source_sha256,
            self.raw_state_machine_tree.payload_event_count,
            self.raw_state_machine_tree.payload_token_byte_count,
            self.raw_state_machine_tree.payload_value_count,
        )?;
        require_bool(
            "manifest_depth.record.raw_tree_render_attempt_selected_by_manifest",
            self.raw_decoded_tree_render_blocker
                .raw_tree_render_attempted,
            self.raw_tree_render_attempt_selected_by_manifest,
        )?;
        require_bool(
            "manifest_depth.record.replay_report_complete",
            self.replay_report_complete,
            true,
        )?;
        require_bool(
            "manifest_depth.record.replay_report_prefix_state_api_ready",
            self.replay_report_prefix_state_api_ready,
            true,
        )?;
        require_bool(
            "manifest_depth.record.canonical_render_ready",
            self.canonical_render_ready,
            true,
        )?;
        require_string(
            "manifest_depth.record.gofmt_validation",
            &self.gofmt_validation,
            "passed",
        )?;
        require_string(
            "manifest_depth.record.go_parser_reparse_validation",
            &self.go_parser_reparse_validation,
            "passed",
        )?;
        require_bool(
            "manifest_depth.record.source_program_validity_claim_allowed",
            self.source_program_validity_claim_allowed,
            false,
        )?;
        require_bool(
            "manifest_depth.record.stable_promotion_allowed",
            self.stable_promotion_allowed,
            false,
        )
    }
}

/// Per-record canonical (elided) stream digest; the digest set identifies
/// the encoded corpus downstream raw-token corpora and BPE artifacts consume.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoTokenizerGeneratorManifestDepthTokenDigest {
    pub manifest_path: String,
    pub canonical_tokens_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoTokenizerGeneratorReportIngestionSource {
    pub manifest_path: String,
    pub report_path: String,
    pub source_path: String,
    pub source_sha256: String,
}

pub fn build_go_tokenizer_generator_artifacts(
    checked_projection_report_path: impl AsRef<Path>,
    projection_artifact_path: impl AsRef<Path>,
    projection_artifact_contents: &str,
) -> Result<GoTokenizerGeneratorArtifacts, SyntaxBackendBoundaryError> {
    build_go_tokenizer_generator_artifacts_with_projection_report_mode(
        checked_projection_report_path,
        projection_artifact_path,
        projection_artifact_contents,
        PROJECTION_REPORT_MODE_CHECKED_FIXTURE,
    )
}

pub fn decode_go_tokenizer_generator_raw_syntax_tree(
    checked_projection_report_path: impl AsRef<Path>,
    projection_artifact_contents: &str,
) -> Result<RawSyntaxDecodedTree, SyntaxBackendBoundaryError> {
    let checked_projection_report_path = checked_projection_report_path.as_ref();
    let checked_report_contents =
        fs::read_to_string(checked_projection_report_path).map_err(|error| {
            SyntaxBackendBoundaryError::new(
                "checked_projection_report_path",
                format!(
                    "failed to read checked Go source-instance projection report {}: {error}",
                    checked_projection_report_path.display()
                ),
            )
        })?;
    let checked_report = parse_checked_report(&checked_report_contents)?;
    checked_report.validate()?;
    let projection_artifact = parse_projection_artifact(projection_artifact_contents)?;
    let raw_token_emission =
        emit_go_source_record_raw_tokens(&checked_report, &projection_artifact)?;
    Ok(raw_token_emission.decoded_tree)
}

pub fn build_go_tokenizer_generator_artifacts_with_projection_report_mode(
    checked_projection_report_path: impl AsRef<Path>,
    projection_artifact_path: impl AsRef<Path>,
    projection_artifact_contents: &str,
    projection_report_mode: &str,
) -> Result<GoTokenizerGeneratorArtifacts, SyntaxBackendBoundaryError> {
    require_projection_report_mode(projection_report_mode)?;
    let checked_projection_report_path = checked_projection_report_path.as_ref();
    let projection_artifact_path = projection_artifact_path.as_ref();
    let checked_report_contents =
        fs::read_to_string(checked_projection_report_path).map_err(|error| {
            SyntaxBackendBoundaryError::new(
                "checked_projection_report_path",
                format!(
                    "failed to read checked Go source-instance projection report {}: {error}",
                    checked_projection_report_path.display()
                ),
            )
        })?;
    let checked_report = parse_checked_report(&checked_report_contents)?;
    let projection_artifact = parse_projection_artifact(projection_artifact_contents)?;
    build_go_tokenizer_generator_artifacts_from_parts_with_projection_report_mode(
        checked_projection_report_path,
        projection_artifact_path,
        projection_artifact_contents,
        &checked_report,
        &projection_artifact,
        projection_report_mode,
    )
}

pub fn build_go_tokenizer_generator_artifacts_from_parts(
    checked_projection_report_path: &Path,
    projection_artifact_path: &Path,
    projection_artifact_contents: &str,
    checked_report: &SyntaxGoSourceInstanceProjectionReport,
    projection_artifact: &SourceInstanceProjectionArtifact,
) -> Result<GoTokenizerGeneratorArtifacts, SyntaxBackendBoundaryError> {
    build_go_tokenizer_generator_artifacts_from_parts_with_projection_report_mode(
        checked_projection_report_path,
        projection_artifact_path,
        projection_artifact_contents,
        checked_report,
        projection_artifact,
        PROJECTION_REPORT_MODE_CHECKED_FIXTURE,
    )
}

pub fn build_go_tokenizer_generator_artifacts_from_parts_with_projection_report_mode(
    checked_projection_report_path: &Path,
    projection_artifact_path: &Path,
    projection_artifact_contents: &str,
    checked_report: &SyntaxGoSourceInstanceProjectionReport,
    projection_artifact: &SourceInstanceProjectionArtifact,
    projection_report_mode: &str,
) -> Result<GoTokenizerGeneratorArtifacts, SyntaxBackendBoundaryError> {
    require_projection_report_mode(projection_report_mode)?;
    checked_report.validate()?;
    projection_artifact.validate_against_report(checked_report)?;
    let raw_token_emission = emit_go_source_record_raw_tokens(checked_report, projection_artifact)?;
    let raw_tokens = &raw_token_emission.raw_tokens;
    let generated_codec_report =
        projection_artifact.generated_codec_report_against_report(checked_report)?;
    generated_codec_report.validate()?;
    let decoded_tree = raw_token_emission.decoded_tree.clone();
    let decoded = raw_token_emission.raw_state_machine_decode.clone();
    let replay_report = build_go_source_instance_replay_report(
        checked_report,
        raw_tokens,
        &generated_codec_report,
    )?;
    replay_report.validate()?;
    let source_record_generator =
        GoSourceRecordGeneratorCore::new(checked_report, &raw_token_emission);
    let source_record_generation = source_record_generator.build_evidence()?;

    let report = go_tokenizer_generator_report(GoTokenizerGeneratorReportInputs {
        checked_projection_report_path,
        projection_artifact_path,
        projection_artifact_contents,
        checked_report,
        raw_tokens,
        generated_codec_report: &generated_codec_report,
        replay_report: &replay_report,
        decoded,
        decoded_tree,
        source_record_generation: &source_record_generation,
        projection_report_mode,
        grammar_rule_count: raw_token_emission.grammar_rule_count,
        grammar_field_count: raw_token_emission.grammar_field_count,
    })?;
    report.validate()?;

    Ok(GoTokenizerGeneratorArtifacts {
        report,
        generated_codec_report,
        replay_report,
        source_record_generation,
    })
}

pub fn parse_go_tokenizer_generator_report(
    contents: &str,
) -> Result<GoTokenizerGeneratorReport, SyntaxBackendBoundaryError> {
    let report: GoTokenizerGeneratorReport = serde_json::from_str(contents).map_err(|error| {
        SyntaxBackendBoundaryError::new(
            "go_tokenizer_generator_report",
            format!("failed to parse Go tokenizer generator report: {error}"),
        )
    })?;
    report.validate()?;
    Ok(report)
}

pub fn parse_go_raw_tree_ast_printer_oracle_cutover_decision(
    contents: &str,
) -> Result<GoRawTreeAstPrinterOracleCutoverDecision, SyntaxBackendBoundaryError> {
    let report: GoRawTreeAstPrinterOracleCutoverReport =
        serde_json::from_str(contents).map_err(|error| {
            SyntaxBackendBoundaryError::new(
                "go_raw_tree_ast_printer_oracle_cutover_report",
                format!("failed to parse Go raw-tree AST printer oracle report: {error}"),
            )
        })?;
    report.cutover_decision()
}

pub fn build_go_tokenizer_generator_manifest_depth_report(
    report_ingestion_manifest_path: impl AsRef<Path>,
    report_ingestion_manifest_contents: &str,
    oracle_cutover_report_input: GoRawTreeAstPrinterOracleCutoverReportInput<'_>,
    artifact_inputs: &[GoTokenizerGeneratorManifestDepthReportInput<'_>],
) -> Result<GoTokenizerGeneratorManifestDepthReport, SyntaxBackendBoundaryError> {
    let report_ingestion_manifest_path = report_ingestion_manifest_path.as_ref();
    let manifest =
        parse_go_tokenizer_generator_report_ingestion_manifest(report_ingestion_manifest_contents)?;
    validate_go_tokenizer_generator_report_ingestion_manifest(&manifest)?;
    // Depth gate: the current-plus manifest must pin the unified grammar
    // artifact and the canonical stream contract at manifest level (the
    // per-field equality against the live identity is enforced by the
    // package-metadata validator above once the fields are non-empty).
    require_non_empty(
        "manifest_depth.package_metadata.stream_contract",
        &manifest.package_metadata.stream_contract,
    )?;
    require_non_empty(
        "manifest_depth.package_metadata.grammar_artifact_id",
        &manifest.package_metadata.grammar_artifact_id,
    )?;
    require_non_empty(
        "manifest_depth.package_metadata.grammar_artifact_content_sha256",
        &manifest.package_metadata.grammar_artifact_content_sha256,
    )?;
    let go_raw_tree_ast_printer_oracle_cutover =
        GoTokenizerGeneratorManifestDepthOracleCutoverEvidence::from_input(
            oracle_cutover_report_input,
        )?;

    let mut inputs_by_source = BTreeMap::new();
    for input in artifact_inputs {
        input.artifacts.report.validate()?;
        input.artifacts.generated_codec_report.validate()?;
        input.artifacts.replay_report.validate()?;
        let key = (
            input.artifacts.report.source_path.as_str(),
            input.artifacts.report.source_sha256.as_str(),
        );
        if inputs_by_source.insert(key, *input).is_some() {
            return Err(SyntaxBackendBoundaryError::new(
                "manifest_depth.inputs",
                format!(
                    "duplicate tokenizer-generator artifact input for {}",
                    input.artifacts.report.source_path
                ),
            ));
        }
    }

    let mut records = Vec::with_capacity(manifest.records.len());
    let mut total_raw_token_count = 0usize;
    let mut total_canonical_token_count = 0usize;
    let mut total_forced_token_count = 0usize;
    let mut max_raw_token_id = None;
    let mut token_layout_vocab_size = None;
    let mut token_layout_pad_token = None;
    for manifest_record in &manifest.records {
        let key = (
            manifest_record.source_path.as_str(),
            manifest_record.source_sha256.as_str(),
        );
        let input = inputs_by_source.remove(&key).ok_or_else(|| {
            SyntaxBackendBoundaryError::new(
                "manifest_depth.records",
                format!(
                    "missing checked tokenizer-generator artifact for {}",
                    manifest_record.manifest_path
                ),
            )
        })?;
        let record = go_tokenizer_generator_manifest_depth_record(manifest_record, input)?;
        total_raw_token_count += record.raw_token_count;
        total_canonical_token_count += record.canonical_token_count;
        total_forced_token_count += record.forced_token_count;
        max_raw_token_id = max_optional_token(max_raw_token_id, record.max_raw_token_id);
        token_layout_vocab_size = Some(match token_layout_vocab_size {
            Some(expected) if expected != record.token_layout_vocab_size => {
                return Err(SyntaxBackendBoundaryError::new(
                    "manifest_depth.token_layout_vocab_size",
                    "all manifest-depth records must use the same token layout VOCAB_SIZE",
                ));
            }
            Some(expected) => expected,
            None => record.token_layout_vocab_size,
        });
        token_layout_pad_token = Some(match token_layout_pad_token {
            Some(expected) if expected != record.token_layout_pad_token => {
                return Err(SyntaxBackendBoundaryError::new(
                    "manifest_depth.token_layout_pad_token",
                    "all manifest-depth records must use the same token layout PAD_TOKEN",
                ));
            }
            Some(expected) => expected,
            None => record.token_layout_pad_token,
        });
        records.push(record);
    }
    if !inputs_by_source.is_empty() {
        let extra = inputs_by_source
            .values()
            .map(|input| input.artifacts.report.source_path.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        return Err(SyntaxBackendBoundaryError::new(
            "manifest_depth.inputs",
            format!("tokenizer-generator artifact inputs not consumed by manifest: {extra}"),
        ));
    }

    let report_ingestion_manifest_path = report_ingestion_manifest_path.display().to_string();
    let report_ingestion_manifest_sha256 =
        sha256_hex(report_ingestion_manifest_contents.as_bytes());
    let raw_tree_render_attempt_summary =
        GoTokenizerGeneratorManifestDepthRawTreeRenderAttemptSummary::from_records(
            &report_ingestion_manifest_path,
            &report_ingestion_manifest_sha256,
            &records,
        );
    go_raw_tree_ast_printer_oracle_cutover.validate(&raw_tree_render_attempt_summary)?;
    let mut raw_tree_render_broadening_decision =
        GoTokenizerGeneratorManifestDepthRawTreeRenderBroadeningDecision::from_records(
            &raw_tree_render_attempt_summary,
            &records,
        );
    let go_ast_grammar_coverage = GoAstSchemaInventoryAdapter::grammar_coverage_file(
        Path::new(GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH),
        go_syntax_token_layout(),
    )
    .map_err(|error| {
        SyntaxBackendBoundaryError::new(
            "go_ast_grammar_coverage",
            format!("failed to build Go AST grammar coverage report: {error}"),
        )
    })?;
    let go_ast_raw_tree_renderer_codegen_coverage =
        GoAstSchemaInventoryAdapter::raw_tree_renderer_codegen_coverage_file(Path::new(
            GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH,
        ))
        .map_err(|error| {
            SyntaxBackendBoundaryError::new(
                "go_ast_raw_tree_renderer_codegen_coverage",
                format!(
                    "failed to build Go AST raw-tree renderer codegen coverage report: {error}"
                ),
            )
        })?;
    raw_tree_render_broadening_decision.next_ready_task =
        expected_raw_tree_render_broadening_next_ready_task(
            &raw_tree_render_broadening_decision,
            &go_ast_grammar_coverage,
            &go_ast_raw_tree_renderer_codegen_coverage,
        )
        .to_string();

    let raw_token_ids_sha256_by_manifest_path = records
        .iter()
        .map(|record| GoTokenizerGeneratorManifestDepthTokenDigest {
            manifest_path: record.manifest_path.clone(),
            canonical_tokens_sha256: record.canonical_tokens_sha256.clone(),
        })
        .collect::<Vec<_>>();
    let unified_artifact = go_unified_grammar_artifact()?;
    let raw_token_digest_set_sha256 =
        json_sha256(&raw_token_ids_sha256_by_manifest_path).map_err(|error| {
            SyntaxBackendBoundaryError::new(
                "raw_token_digest_set_sha256",
                format!("failed to hash raw token digest set: {error}"),
            )
        })?;
    let scratch_generation_gate = assemble_scratch_generation_gate(
        &go_raw_tree_ast_printer_oracle_cutover,
        &go_ast_grammar_coverage,
        &records,
    )?;
    let go_ast_inventory_template_coverage_frontier =
        go_ast_inventory_template_coverage_frontier_report(
            &go_ast_grammar_coverage,
            &scratch_generation_gate,
        )?;
    let go_ast_generation_frontier = go_ast_generation_frontier_report(
        &go_ast_grammar_coverage,
        &go_raw_tree_ast_printer_oracle_cutover,
        &scratch_generation_gate,
    )?;
    let manifest_depth_next_ready_task = expected_manifest_depth_next_ready_task(
        &raw_tree_render_broadening_decision,
        &go_ast_grammar_coverage,
        &go_ast_raw_tree_renderer_codegen_coverage,
    )
    .to_string();
    // Report-level claim fields (vocab/SM + gate readiness + canonical-render +
    // full pipeline) come from one generator-core stamp.
    let report_claim_stamp =
        GoSourceRecordManifestDepthReportClaimStamp::from_gate_and_record_views(
            &scratch_generation_gate,
            records
                .iter()
                .map(GoTokenizerGeneratorManifestDepthRecord::vocab_sm_completeness_view),
            records
                .iter()
                .map(GoTokenizerGeneratorManifestDepthRecord::canonical_render_readiness_view),
        )?;

    let report = GoTokenizerGeneratorManifestDepthReport {
        artifact_kind: MANIFEST_DEPTH_ARTIFACT_KIND.to_string(),
        version: VERSION,
        tool: TOOL.to_string(),
        status: MANIFEST_DEPTH_STATUS.to_string(),
        target: TARGET.to_string(),
        route: ROUTE.to_string(),
        report_ingestion_manifest_path,
        report_ingestion_manifest_sha256,
        report_ingestion_manifest_artifact_kind: manifest.artifact_kind,
        report_ingestion_manifest_tool: manifest.tool,
        corpus_id: manifest.corpus_id,
        source_corpus_manifest_path: manifest.source_corpus_manifest_path,
        renderer_position_policy_summary: go_ast_renderer_position_policy_summary()?,
        go_ast_grammar_coverage,
        go_ast_raw_tree_renderer_codegen_coverage,
        raw_tree_render_attempt_summary,
        raw_tree_render_broadening_decision,
        go_raw_tree_ast_printer_oracle_cutover,
        scratch_generation_gate,
        go_ast_inventory_template_coverage_frontier,
        go_ast_generation_frontier,
        record_count: records.len(),
        supported_record_count: records.len(),
        records,
        total_raw_token_count,
        stream_contract: LIVE_RAW_SYNTAX_STREAM_CONTRACT.to_string(),
        total_canonical_token_count,
        total_forced_token_count,
        grammar_artifact_id: unified_artifact.artifact_id.to_string(),
        grammar_artifact_content_sha256: unified_artifact.content_sha256.clone(),
        raw_token_ids_sha256_by_manifest_path,
        raw_token_digest_set_sha256,
        max_raw_token_id,
        token_layout_vocab_size: token_layout_vocab_size.unwrap_or(0),
        token_layout_pad_token: token_layout_pad_token.unwrap_or(0),
        raw_tokens_below_vocab_size: report_claim_stamp.raw_tokens_below_vocab_size,
        pad_token_absent: report_claim_stamp.pad_token_absent,
        raw_state_machine_complete: report_claim_stamp.raw_state_machine_complete,
        selected_completion_smoke_ready: report_claim_stamp.selected_completion_smoke_ready,
        elided_selected_completion_ready: report_claim_stamp.elided_selected_completion_ready,
        incomplete_prefix_rejection_ready: report_claim_stamp
            .incomplete_prefix_rejection_ready,
        token_only_decode_render_ready: report_claim_stamp.token_only_decode_render_ready,
        canonical_render_ready: report_claim_stamp.canonical_render_ready,
        pipeline: GoTokenizerGeneratorPipeline::from_field_stamp(report_claim_stamp.pipeline),
        next_ready_task: manifest_depth_next_ready_task,
        validation_commands: vec![
            format!(
                "cargo run --bin tct-syntax-go-tokenizer-generator -- --report-ingestion-manifest {} --go-ast-printer-oracle-report {} --output {}",
                DEFAULT_REPORT_INGESTION_MANIFEST_PATH,
                DEFAULT_GO_RAW_TREE_AST_PRINTER_ORACLE_REPORT_PATH,
                DEFAULT_MANIFEST_DEPTH_OUTPUT_PATH
            ),
            "go test scripts/analysis/go_raw_tree_ast_printer_oracle.go scripts/analysis/go_raw_tree_ast_printer_oracle_test.go -run TestGoRawTreeAstPrinterOracle".to_string(),
            "cargo test --test syntax_go_source_instance_projection_test go_raw_tree_ast_printer_oracle_exports_current_plus_full_adoption_decision_report -- --exact --nocapture".to_string(),
            "cargo test --test syntax_go_source_instance_projection_test go_scratch_generation_incomplete_prefix_rejection_validates_continuation_mask -- --exact".to_string(),
            "cargo test --test syntax_go_source_instance_projection_test go_tokenizer_generator_manifest_depth_report_validates_current_plus_supported_records -- --exact".to_string(),
            "cargo test --test go_ast_schema_inventory_test".to_string(),
            "./scripts/testing/test-go-generated-codec-round-trip-gate.sh".to_string(),
            "python3 scripts/testing/test-quality-audit.py --changed-from origin/main".to_string(),
            "git diff --check".to_string(),
        ],
        claim_boundary: CLAIM_BOUNDARY.to_string(),
        no_stable_claims: NO_STABLE_CLAIMS
            .iter()
            .map(|claim| (*claim).to_string())
            .collect(),
    };
    report.validate()?;
    Ok(report)
}

pub fn parse_go_tokenizer_generator_manifest_depth_report(
    contents: &str,
) -> Result<GoTokenizerGeneratorManifestDepthReport, SyntaxBackendBoundaryError> {
    let report: GoTokenizerGeneratorManifestDepthReport =
        serde_json::from_str(contents).map_err(|error| {
            SyntaxBackendBoundaryError::new(
                "go_tokenizer_generator_manifest_depth_report",
                format!("failed to parse Go tokenizer generator manifest-depth report: {error}"),
            )
        })?;
    report.validate()?;
    Ok(report)
}

pub fn go_tokenizer_generator_report_ingestion_sources(
    report_ingestion_manifest_contents: &str,
) -> Result<Vec<GoTokenizerGeneratorReportIngestionSource>, SyntaxBackendBoundaryError> {
    let manifest =
        parse_go_tokenizer_generator_report_ingestion_manifest(report_ingestion_manifest_contents)?;
    validate_go_tokenizer_generator_report_ingestion_manifest(&manifest)?;
    Ok(manifest
        .records
        .into_iter()
        .map(|record| GoTokenizerGeneratorReportIngestionSource {
            manifest_path: record.manifest_path,
            report_path: record.report_path,
            source_path: record.source_path,
            source_sha256: record.source_sha256,
        })
        .collect())
}

fn go_tokenizer_generator_manifest_depth_record(
    manifest_record: &GoTokenizerGeneratorReportIngestionRecord,
    input: GoTokenizerGeneratorManifestDepthReportInput<'_>,
) -> Result<GoTokenizerGeneratorManifestDepthRecord, SyntaxBackendBoundaryError> {
    let report = &input.artifacts.report;
    let generated_report = &input.artifacts.generated_codec_report;
    // Generator-core stamps source-record evidence fields from retained
    // evidence; megamodule only wires generated-codec parity, raw-tree render
    // selection, and report-level glue below.
    let source_fields = GoSourceRecordManifestDepthRecordSourceFields::from_generator_evidence(
        &input.artifacts.source_record_generation,
    );
    source_fields.validate_api_and_stream_identity()?;
    source_fields.require_absorbed_report_parity(
        &report.source_record_generator_core_api_id,
        &report.source_record_generator_core_api_boundary,
        &report.source_record_raw_token_emission_api_id,
        &report.source_record_raw_token_emission_api_boundary,
        &report.delegated_raw_scratch_generator_core_api_id,
        &report.delegated_raw_scratch_generator_core_api_boundary,
        &report.stream_contract,
        report.canonical_token_count,
        &report.canonical_tokens_sha256,
        report.canonical_max_token_id,
        report.forced_token_count,
        &report.empty_prefix_generation,
        &report.selected_completion_smoke,
        &report.elided_selected_completion,
        &report.incomplete_prefix_rejection,
        &report.token_only_decode_render,
    )?;
    require_string(
        "manifest_depth.record.manifest_path",
        &generated_report.manifest_path,
        &manifest_record.manifest_path,
    )?;
    require_string(
        "manifest_depth.record.generated_codec_report_path",
        &report.generated_codec_report_path,
        &manifest_record.report_path,
    )?;
    require_string(
        "manifest_depth.record.generated_codec_report_path",
        &generated_report.report_path,
        &manifest_record.report_path,
    )?;
    require_string(
        "manifest_depth.record.generated_codec_route",
        &manifest_record.generated_codec_route,
        GENERATED_CODEC_REPORT_ROUTE,
    )?;
    require_string(
        "manifest_depth.record.generated_codec_report_artifact_kind",
        &manifest_record.generated_codec_report_artifact_kind,
        GENERATED_CODEC_REPORT_ARTIFACT_KIND,
    )?;
    require_usize(
        "manifest_depth.record.canonical_token_count",
        source_fields.canonical_token_count,
        manifest_record.expected_tct_token_count,
    )?;
    require_string(
        "manifest_depth.record.canonical_tokens_sha256",
        &source_fields.canonical_tokens_sha256,
        &manifest_record.expected_tct_tokens_sha256,
    )?;
    require_u32(
        "manifest_depth.record.token_layout_vocab_size",
        report.token_layout_vocab_size,
        manifest_record.expected_vocab_size,
    )?;
    require_u32(
        "manifest_depth.record.token_layout_pad_token",
        report.token_layout_pad_token,
        manifest_record.expected_pad_token,
    )?;
    if source_fields.canonical_max_token_id != manifest_record.expected_max_raw_token_id {
        return Err(SyntaxBackendBoundaryError::new(
            "manifest_depth.record.canonical_max_token_id",
            "tokenizer-generator canonical max token ID must match report-ingestion expectation",
        ));
    }
    validate_report_ingestion_claim_boundary(&manifest_record.expected_claim_boundary)?;

    let record = GoTokenizerGeneratorManifestDepthRecord {
        role: manifest_record.role.clone(),
        evidence_scope: manifest_record.evidence_scope.clone(),
        manifest_path: manifest_record.manifest_path.clone(),
        source_path: manifest_record.source_path.clone(),
        source_sha256: manifest_record.source_sha256.clone(),
        metadata_path: report.metadata_path.clone(),
        metadata_sha256: report.metadata_sha256.clone(),
        checked_projection_report_path: input.checked_projection_report_path.display().to_string(),
        candidate_grammar_id: report.candidate_grammar_id.clone(),
        source_record_generator_core_api_id: source_fields.source_record_generator_core_api_id,
        source_record_generator_core_api_boundary: source_fields
            .source_record_generator_core_api_boundary,
        source_record_raw_token_emission_api_id: source_fields
            .source_record_raw_token_emission_api_id,
        source_record_raw_token_emission_api_boundary: source_fields
            .source_record_raw_token_emission_api_boundary,
        delegated_raw_scratch_generator_core_api_id: source_fields
            .delegated_raw_scratch_generator_core_api_id,
        delegated_raw_scratch_generator_core_api_boundary: source_fields
            .delegated_raw_scratch_generator_core_api_boundary,
        generated_codec_report_path: report.generated_codec_report_path.clone(),
        generated_codec_report_artifact_kind: manifest_record
            .generated_codec_report_artifact_kind
            .clone(),
        generated_codec_route: manifest_record.generated_codec_route.clone(),
        expected_tct_token_count: manifest_record.expected_tct_token_count,
        stream_contract: source_fields.stream_contract,
        canonical_token_count: source_fields.canonical_token_count,
        canonical_tokens_sha256: source_fields.canonical_tokens_sha256,
        canonical_max_token_id: source_fields.canonical_max_token_id,
        forced_token_count: source_fields.forced_token_count,
        raw_token_count: report.raw_token_count,
        expected_tct_tokens_sha256: manifest_record.expected_tct_tokens_sha256.clone(),
        raw_token_ids_sha256: report.raw_token_ids_sha256.clone(),
        expected_max_raw_token_id: manifest_record.expected_max_raw_token_id,
        max_raw_token_id: report.max_raw_token_id,
        expected_vocab_size: manifest_record.expected_vocab_size,
        token_layout_vocab_size: report.token_layout_vocab_size,
        expected_pad_token: manifest_record.expected_pad_token,
        token_layout_pad_token: report.token_layout_pad_token,
        expected_pad_token_absent: manifest_record.expected_pad_token_absent,
        pad_token_absent: report.pad_token_absent,
        raw_tokens_below_vocab_size: report.raw_tokens_below_vocab_size,
        raw_state_machine_complete: report.raw_state_machine_complete,
        raw_state_machine_decode: report.raw_state_machine_decode.clone(),
        raw_state_machine_tree: report.raw_state_machine_tree.clone(),
        empty_prefix_generation: source_fields.empty_prefix_generation,
        selected_completion_smoke: source_fields.selected_completion_smoke,
        elided_selected_completion: source_fields.elided_selected_completion,
        incomplete_prefix_rejection: source_fields.incomplete_prefix_rejection,
        token_only_decode_render: source_fields.token_only_decode_render,
        raw_tree_render_attempt_selected_by_manifest: manifest_record.raw_tree_render_attempt,
        raw_decoded_tree_render_blocker: report.raw_decoded_tree_render_blocker.clone(),
        replay_report_complete: report.replay_report_complete,
        replay_report_prefix_state_api_ready: report.replay_report_prefix_state_api_ready,
        canonical_render_ready: report.canonical_render_ready,
        gofmt_validation: report.gofmt_validation.clone(),
        go_parser_reparse_validation: report.go_parser_reparse_validation.clone(),
        source_program_validity_claim_allowed: manifest_record
            .source_program_validity_claim_allowed,
        stable_promotion_allowed: manifest_record.stable_promotion_allowed,
    };
    record.validate()?;
    Ok(record)
}

struct GoTokenizerGeneratorReportInputs<'a> {
    checked_projection_report_path: &'a Path,
    projection_artifact_path: &'a Path,
    projection_artifact_contents: &'a str,
    checked_report: &'a SyntaxGoSourceInstanceProjectionReport,
    raw_tokens: &'a SourceInstanceProjectionRawTokenIds,
    generated_codec_report: &'a SourceInstanceGeneratedCodecReport,
    replay_report: &'a SyntaxStateMachineReplayReport,
    decoded: RawSyntaxState,
    decoded_tree: RawSyntaxDecodedTree,
    source_record_generation: &'a GoSourceRecordGeneratorEvidence,
    projection_report_mode: &'a str,
    grammar_rule_count: usize,
    grammar_field_count: usize,
}

fn go_ast_renderer_position_policy_summary()
-> Result<GoTokenizerGeneratorRendererPositionPolicySummary, SyntaxBackendBoundaryError> {
    let inventory_path = Path::new(GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH);
    let fields = GoAstSchemaInventoryAdapter::renderer_position_fields_file(inventory_path)
        .map_err(|error| {
            SyntaxBackendBoundaryError::new(
                "renderer_position_policy_summary",
                format!("failed to load Go AST renderer-position policies: {error}"),
            )
        })?;
    GoTokenizerGeneratorRendererPositionPolicySummary::from_fields(
        inventory_path.display().to_string(),
        &fields,
    )
}

fn go_tokenizer_generator_raw_decoded_tree_render_blocker(
    checked_report: &SyntaxGoSourceInstanceProjectionReport,
    raw_tokens: &SourceInstanceProjectionRawTokenIds,
    decoded_tree: &RawSyntaxDecodedTree,
) -> Result<GoTokenizerGeneratorRawDecodedTreeRenderBlocker, SyntaxBackendBoundaryError> {
    let decoded_node_tree = decoded_tree.decoded_node_tree()?;
    let root_file_comment_group_count = raw_tree_root_file_comment_group_count(&decoded_node_tree)?;
    let total_comment_group_node_count = decoded_node_tree.node_kind_count("CommentGroup");
    let non_file_comment_group_node_count =
        total_comment_group_node_count.saturating_sub(root_file_comment_group_count);
    let render_attempt_selection = raw_tree_render_attempt_selection(checked_report)?;
    let raw_tree_render_attempted = render_attempt_selection.attempted;
    let renderer_diagnostic = if raw_tree_render_attempted {
        match go_tokenizer_generator_raw_tree_render_ready_evidence(checked_report, decoded_tree) {
            Ok(render) => {
                let evidence = GoTokenizerGeneratorRawDecodedTreeRenderBlocker {
                    status: RAW_DECODED_TREE_RENDER_READY_STATUS.to_string(),
                    scope: RAW_DECODED_TREE_RENDER_BLOCKER_SCOPE.to_string(),
                    decoded_tree_source: TOKEN_ONLY_RAW_TOKEN_STREAM_SOURCE.to_string(),
                    raw_tree_render_attempted: true,
                    raw_tree_render_attempt_source: RAW_TREE_RENDER_ATTEMPT_SELECTION_SOURCE
                        .to_string(),
                    raw_tree_render_attempt_manifest_path: render_attempt_selection
                        .manifest_path
                        .clone(),
                    raw_tree_render_attempt_manifest_sha256: render_attempt_selection
                        .manifest_sha256
                        .clone(),
                    renderer_id: Some(GO_RAW_TREE_RENDERER_ID.to_string()),
                    renderer_scope: Some(GO_RAW_TREE_RENDERER_SCOPE.to_string()),
                    renderer_diagnostic: None,
                    raw_token_count: raw_tokens.raw_token_count,
                    raw_token_ids_sha256: raw_tokens.raw_token_ids_sha256.clone(),
                    source_sha256: Some(checked_report.source.sha256.clone()),
                    decoded_tree_reusable: true,
                    decoded_tree_payload_event_count: decoded_tree.payload_event_count(),
                    decoded_tree_payload_token_byte_count: decoded_tree.payload_token_byte_count(),
                    decoded_tree_payload_value_count: decoded_tree.payload_value_count(),
                    raw_tree_payload_values_materialized: true,
                    raw_tree_node_tree_materialized: true,
                    root_file_comment_group_count: render.root_file_comment_group_count,
                    attached_comment_group_count: render.attached_comment_group_count,
                    root_only_comment_group_count: render.root_only_comment_group_count,
                    total_comment_group_node_count,
                    non_file_comment_group_node_count,
                    raw_tree_comment_position_policy_materialized: true,
                    raw_tree_comment_group_reference_policy_materialized: true,
                    raw_tree_comment_anchor_policy: Some(
                        GO_RAW_TREE_COMMENT_ANCHOR_POLICY.to_string(),
                    ),
                    raw_tree_comment_group_reference_policy: Some(
                        GO_RAW_TREE_COMMENT_REFERENCE_POLICY.to_string(),
                    ),
                    trailing_comment_anchor_count: render.trailing_comment_anchor_count,
                    comment_trivia_policy_status: "raw_tree_comment_anchor_policy_materialized"
                        .to_string(),
                    renderer_policy_status: "raw_tree_decoded_source_render_ready".to_string(),
                    canonical_render_reused_from_checked_projection: false,
                    raw_tree_grammar_mode_render_ready: true,
                    raw_tree_canonical_fidelity_ready: render.matches_canonical_source,
                    essential_comment_policy_ready: true,
                    essential_comment_policy_status: RAW_TREE_ESSENTIAL_COMMENT_POLICY_READY_STATUS
                        .to_string(),
                    essential_comment_count: render.essential_comment_count,
                    essential_comment_categories: render.essential_comment_categories,
                    raw_tree_canonical_render_ready: render.matches_canonical_source,
                    raw_tree_rendered_source_sha256: Some(render.rendered_source_sha256),
                    raw_tree_rendered_source_byte_count: Some(render.rendered_source_byte_count),
                    raw_tree_formatted_source_sha256: Some(render.formatted_source_sha256),
                    raw_tree_formatted_source_byte_count: Some(render.formatted_source_byte_count),
                    raw_tree_render_matches_canonical_source: render.matches_canonical_source,
                    canonical_fidelity_diff_summary: render.canonical_fidelity_diff_summary,
                    gofmt_validation: Some("passed".to_string()),
                    go_parser_reparse_validation: Some("passed".to_string()),
                    missing_capabilities: Vec::new(),
                    next_ready_task: RAW_DECODED_TREE_RENDER_NEXT_READY_TASK.to_string(),
                    claim_boundary: RAW_DECODED_TREE_RENDER_CLAIM_BOUNDARY.to_string(),
                };
                evidence.validate(
                    raw_tokens.raw_token_count,
                    &raw_tokens.raw_token_ids_sha256,
                    &checked_report.source.sha256,
                    decoded_tree.payload_event_count(),
                    decoded_tree.payload_token_byte_count(),
                    decoded_tree.payload_value_count(),
                )?;
                return Ok(evidence);
            }
            Err(error) => Some(raw_tree_renderer_diagnostic(error)),
        }
    } else {
        None
    };
    let attempted_missing_capabilities = renderer_diagnostic
        .as_ref()
        .map(|diagnostic| vec![diagnostic.missing_capability.clone()]);
    let attempted_blocked = renderer_diagnostic.is_some();
    let evidence = GoTokenizerGeneratorRawDecodedTreeRenderBlocker {
        status: RAW_DECODED_TREE_RENDER_BLOCKER_STATUS.to_string(),
        scope: RAW_DECODED_TREE_RENDER_BLOCKER_SCOPE.to_string(),
        decoded_tree_source: TOKEN_ONLY_RAW_TOKEN_STREAM_SOURCE.to_string(),
        raw_tree_render_attempted,
        raw_tree_render_attempt_source: RAW_TREE_RENDER_ATTEMPT_SELECTION_SOURCE.to_string(),
        raw_tree_render_attempt_manifest_path: render_attempt_selection.manifest_path,
        raw_tree_render_attempt_manifest_sha256: render_attempt_selection.manifest_sha256,
        renderer_id: raw_tree_render_attempted.then(|| GO_RAW_TREE_RENDERER_ID.to_string()),
        renderer_scope: raw_tree_render_attempted.then(|| GO_RAW_TREE_RENDERER_SCOPE.to_string()),
        renderer_diagnostic,
        raw_token_count: raw_tokens.raw_token_count,
        raw_token_ids_sha256: raw_tokens.raw_token_ids_sha256.clone(),
        source_sha256: raw_tree_render_attempted.then(|| checked_report.source.sha256.clone()),
        decoded_tree_reusable: true,
        decoded_tree_payload_event_count: decoded_tree.payload_event_count(),
        decoded_tree_payload_token_byte_count: decoded_tree.payload_token_byte_count(),
        decoded_tree_payload_value_count: decoded_tree.payload_value_count(),
        raw_tree_payload_values_materialized: true,
        raw_tree_node_tree_materialized: true,
        root_file_comment_group_count,
        attached_comment_group_count: non_file_comment_group_node_count,
        root_only_comment_group_count: root_file_comment_group_count
            .saturating_sub(non_file_comment_group_node_count),
        total_comment_group_node_count,
        non_file_comment_group_node_count,
        raw_tree_comment_position_policy_materialized: false,
        raw_tree_comment_group_reference_policy_materialized: false,
        raw_tree_comment_anchor_policy: None,
        raw_tree_comment_group_reference_policy: None,
        trailing_comment_anchor_count: 0,
        comment_trivia_policy_status: if attempted_blocked {
            "raw_tree_renderer_attempted_unsupported_policy"
        } else {
            "raw_tree_node_values_ready_comment_anchor_policy_blocked"
        }
        .to_string(),
        renderer_policy_status: if attempted_blocked {
            "blocked_on_raw_tree_renderer_diagnostic"
        } else {
            "blocked_on_comment_anchor_and_reference_policy"
        }
        .to_string(),
        canonical_render_reused_from_checked_projection: !attempted_blocked,
        raw_tree_grammar_mode_render_ready: false,
        raw_tree_canonical_fidelity_ready: false,
        essential_comment_policy_ready: false,
        essential_comment_policy_status: RAW_TREE_ESSENTIAL_COMMENT_POLICY_NOT_EVALUATED_STATUS
            .to_string(),
        essential_comment_count: 0,
        essential_comment_categories: Vec::new(),
        raw_tree_canonical_render_ready: false,
        raw_tree_rendered_source_sha256: None,
        raw_tree_rendered_source_byte_count: None,
        raw_tree_formatted_source_sha256: None,
        raw_tree_formatted_source_byte_count: None,
        raw_tree_render_matches_canonical_source: false,
        canonical_fidelity_diff_summary: None,
        gofmt_validation: None,
        go_parser_reparse_validation: None,
        missing_capabilities: attempted_missing_capabilities.unwrap_or_else(|| {
            RAW_DECODED_TREE_RENDER_MISSING_CAPABILITIES
                .iter()
                .map(|capability| (*capability).to_string())
                .collect()
        }),
        next_ready_task: if attempted_blocked {
            RAW_DECODED_TREE_RENDER_NEXT_READY_TASK
        } else {
            RAW_DECODED_TREE_RENDER_BLOCKER_NEXT_READY_TASK
        }
        .to_string(),
        claim_boundary: if attempted_blocked {
            RAW_DECODED_TREE_RENDER_DIAGNOSTIC_CLAIM_BOUNDARY
        } else {
            RAW_DECODED_TREE_RENDER_BLOCKER_CLAIM_BOUNDARY
        }
        .to_string(),
    };
    evidence.validate(
        raw_tokens.raw_token_count,
        &raw_tokens.raw_token_ids_sha256,
        &checked_report.source.sha256,
        decoded_tree.payload_event_count(),
        decoded_tree.payload_token_byte_count(),
        decoded_tree.payload_value_count(),
    )?;
    Ok(evidence)
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RawTreeRenderAttemptSelection {
    attempted: bool,
    manifest_path: Option<String>,
    manifest_sha256: Option<String>,
}

fn raw_tree_render_attempt_selection(
    checked_report: &SyntaxGoSourceInstanceProjectionReport,
) -> Result<RawTreeRenderAttemptSelection, SyntaxBackendBoundaryError> {
    let manifest_path = checked_report.report_ingestion_manifest_path.trim();
    if manifest_path.is_empty() {
        return Ok(RawTreeRenderAttemptSelection {
            attempted: false,
            manifest_path: None,
            manifest_sha256: None,
        });
    }
    let contents = fs::read_to_string(manifest_path).map_err(|error| {
        SyntaxBackendBoundaryError::new(
            "raw_decoded_tree_render_blocker.raw_tree_render_attempt_manifest_path",
            format!("failed to read Go raw-tree render attempt manifest {manifest_path}: {error}"),
        )
    })?;
    let manifest = parse_go_tokenizer_generator_report_ingestion_manifest(&contents)?;
    validate_go_tokenizer_generator_report_ingestion_manifest(&manifest)?;
    let manifest_sha256 = sha256_hex(contents.as_bytes());
    let attempted = manifest
        .records
        .iter()
        .find(|record| {
            record.manifest_path == checked_report.manifest_path
                && record.source_path == checked_report.source.path
                && record.source_sha256 == checked_report.source.sha256
        })
        .is_some_and(|record| record.raw_tree_render_attempt);
    Ok(RawTreeRenderAttemptSelection {
        attempted,
        manifest_path: Some(manifest_path.to_string()),
        manifest_sha256: Some(manifest_sha256),
    })
}

fn raw_tree_renderer_diagnostic(
    error: SyntaxBackendBoundaryError,
) -> GoTokenizerGeneratorRawTreeRendererDiagnostic {
    let field = error.field().to_string();
    let message = error.to_string();
    let unsupported_category = raw_tree_renderer_unsupported_category(&field, &message).to_string();
    GoTokenizerGeneratorRawTreeRendererDiagnostic {
        diagnostic_kind: "raw_tree_renderer_attempt_failed".to_string(),
        missing_capability: format!("go_raw_tree_renderer_{unsupported_category}"),
        unsupported_category,
        field,
        message,
    }
}

fn raw_tree_renderer_unsupported_category(field: &str, message: &str) -> &'static str {
    if message.contains("comment_anchor") {
        return "unsupported_comment_anchor_policy";
    }
    if message.contains("comment_reference") {
        return "unsupported_comment_reference_policy";
    }
    match field {
        "decl" => "unsupported_declaration",
        "spec" => "unsupported_spec",
        "stmt" => "unsupported_statement",
        "expr" | "if.else" => "unsupported_expression",
        "comment_anchor" => "unsupported_comment_anchor_policy",
        "comment_reference" => "unsupported_comment_reference_policy",
        "raw_decoded_tree_render_blocker.gofmt_validation" => "unsupported_formatting_policy",
        "raw_decoded_tree_render_blocker.raw_tree_render_matches_canonical_source" => {
            "unsupported_canonical_render_policy"
        }
        _ => "unsupported_renderer_policy",
    }
}

struct GoTokenizerGeneratorRawTreeRenderReadyEvidence {
    rendered_source_sha256: String,
    rendered_source_byte_count: usize,
    formatted_source_sha256: String,
    formatted_source_byte_count: usize,
    matches_canonical_source: bool,
    canonical_fidelity_diff_summary: Option<String>,
    essential_comment_count: usize,
    essential_comment_categories: Vec<String>,
    root_file_comment_group_count: usize,
    attached_comment_group_count: usize,
    root_only_comment_group_count: usize,
    trailing_comment_anchor_count: usize,
}

fn go_tokenizer_generator_raw_tree_render_ready_evidence(
    checked_report: &SyntaxGoSourceInstanceProjectionReport,
    decoded_tree: &RawSyntaxDecodedTree,
) -> Result<GoTokenizerGeneratorRawTreeRenderReadyEvidence, SyntaxBackendBoundaryError> {
    let rendered = render_go_source_from_raw_tree(decoded_tree)?;
    let rendered_source_sha256 = sha256_hex(rendered.source.as_bytes());
    let rendered_source_byte_count = rendered.source.len();
    let formatted = gofmt_raw_tree_rendered_source(&rendered.source)?;
    let formatted_source_sha256 = sha256_hex(formatted.as_bytes());
    let formatted_source_byte_count = formatted.len();
    let matches_canonical_source = formatted_source_sha256 == checked_report.source.sha256
        && formatted_source_byte_count == checked_report.source.bytes;
    let canonical_fidelity_diff_summary = if matches_canonical_source {
        None
    } else {
        let expected_source = fs::read_to_string(&checked_report.source.path).map_err(|error| {
            SyntaxBackendBoundaryError::new(
                "raw_decoded_tree_render_blocker.raw_tree_render_matches_canonical_source",
                format!(
                    "failed to read checked Go source {} for raw render diff: {error}",
                    checked_report.source.path
                ),
            )
        })?;
        Some(format!(
            "raw decoded-tree render hash {} ({} bytes) did not match checked source hash {} ({} bytes): {}",
            formatted_source_sha256,
            formatted_source_byte_count,
            checked_report.source.sha256,
            checked_report.source.bytes,
            first_source_diff_summary(&expected_source, &formatted),
        ))
    };
    let decoded_node_tree = decoded_tree.decoded_node_tree()?;
    let essential_comments = raw_tree_essential_comments(&decoded_node_tree);
    ensure_essential_comments_preserved(&formatted, &essential_comments)?;
    let essential_comment_categories = essential_comments
        .iter()
        .flat_map(|comment| essential_comment_categories(comment))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    Ok(GoTokenizerGeneratorRawTreeRenderReadyEvidence {
        rendered_source_sha256,
        rendered_source_byte_count,
        formatted_source_sha256,
        formatted_source_byte_count,
        matches_canonical_source,
        canonical_fidelity_diff_summary,
        essential_comment_count: essential_comments.len(),
        essential_comment_categories,
        root_file_comment_group_count: rendered.root_file_comment_group_count,
        attached_comment_group_count: rendered.attached_comment_group_count,
        root_only_comment_group_count: rendered.root_only_comment_group_count,
        trailing_comment_anchor_count: rendered.trailing_comment_anchor_count,
    })
}

fn raw_tree_essential_comments(root: &RawSyntaxDecodedNode) -> Vec<String> {
    let mut comments = Vec::new();
    collect_raw_tree_essential_comments(root, &mut comments);
    comments
}

fn collect_raw_tree_essential_comments(node: &RawSyntaxDecodedNode, comments: &mut Vec<String>) {
    if node.node_kind == "Comment"
        && let Some(text) = node.field("Text").and_then(|field| {
            field.values.iter().find_map(|value| match value {
                RawSyntaxDecodedValue::Terminal(terminal) => Some(terminal.payload.clone()),
                RawSyntaxDecodedValue::Node(_) => None,
            })
        })
        && is_essential_go_comment(&text)
    {
        comments.push(text);
    }
    for field in &node.fields {
        for value in &field.values {
            if let RawSyntaxDecodedValue::Node(child) = value {
                collect_raw_tree_essential_comments(child, comments);
            }
        }
    }
}

fn ensure_essential_comments_preserved(
    formatted_source: &str,
    comments: &[String],
) -> Result<(), SyntaxBackendBoundaryError> {
    for comment in comments {
        if !formatted_source.contains(comment) {
            return Err(SyntaxBackendBoundaryError::new(
                "raw_decoded_tree_render_blocker.essential_comment_policy_status",
                format!("raw decoded-tree render dropped essential Go comment {comment:?}"),
            ));
        }
    }
    Ok(())
}

fn is_essential_go_comment(comment: &str) -> bool {
    let trimmed = comment.trim_start();
    trimmed.starts_with("//go:build")
        || trimmed.starts_with("// +build")
        || trimmed.starts_with("//go:")
        || trimmed.starts_with("//line ")
        || trimmed.starts_with("/*line ")
        || is_cgo_preamble_comment(trimmed)
}

fn essential_comment_categories(comment: &str) -> Vec<String> {
    let trimmed = comment.trim_start();
    let mut categories = Vec::new();
    if trimmed.starts_with("//go:build") {
        categories.push("go_build_constraint".to_string());
    }
    if trimmed.starts_with("// +build") {
        categories.push("legacy_build_constraint".to_string());
    }
    if trimmed.starts_with("//go:") && !trimmed.starts_with("//go:build") {
        categories.push("go_directive".to_string());
    }
    if trimmed.starts_with("//line ") || trimmed.starts_with("/*line ") {
        categories.push("line_directive".to_string());
    }
    if is_cgo_preamble_comment(trimmed) {
        categories.push("cgo_preamble".to_string());
    }
    categories
}

fn is_cgo_preamble_comment(trimmed: &str) -> bool {
    trimmed.starts_with("// #cgo")
        || trimmed.starts_with("//#cgo")
        || trimmed.starts_with("// #include")
        || trimmed.starts_with("//#include")
        || trimmed.starts_with("// #define")
        || trimmed.starts_with("//#define")
        || (trimmed.starts_with("/*")
            && (trimmed.contains("#cgo")
                || trimmed.contains("#include")
                || trimmed.contains("#define")))
}

fn first_source_diff_summary(expected: &str, actual: &str) -> String {
    let expected_lines = expected.lines().collect::<Vec<_>>();
    let actual_lines = actual.lines().collect::<Vec<_>>();
    let max_len = expected_lines.len().max(actual_lines.len());
    for index in 0..max_len {
        let expected_line = expected_lines.get(index).copied().unwrap_or("<missing>");
        let actual_line = actual_lines.get(index).copied().unwrap_or("<missing>");
        if expected_line != actual_line {
            return format!(
                "first differing line {} expected {:?}, got {:?}",
                index + 1,
                expected_line,
                actual_line
            );
        }
    }
    "line content matched but trailing newline or byte content differed".to_string()
}

fn gofmt_raw_tree_rendered_source(source: &str) -> Result<String, SyntaxBackendBoundaryError> {
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
                "raw_decoded_tree_render_blocker.gofmt_validation",
                format!("failed to start gofmt for raw decoded-tree render: {error}"),
            )
        })?;
    child
        .stdin
        .as_mut()
        .ok_or_else(|| {
            SyntaxBackendBoundaryError::new(
                "raw_decoded_tree_render_blocker.gofmt_validation",
                "failed to open gofmt stdin for raw decoded-tree render",
            )
        })?
        .write_all(source.as_bytes())
        .map_err(|error| {
            SyntaxBackendBoundaryError::new(
                "raw_decoded_tree_render_blocker.gofmt_validation",
                format!("failed to write raw decoded-tree render to gofmt: {error}"),
            )
        })?;
    let output = child.wait_with_output().map_err(|error| {
        SyntaxBackendBoundaryError::new(
            "raw_decoded_tree_render_blocker.gofmt_validation",
            format!("failed to read gofmt output for raw decoded-tree render: {error}"),
        )
    })?;
    if !output.status.success() {
        return Err(SyntaxBackendBoundaryError::new(
            "raw_decoded_tree_render_blocker.gofmt_validation",
            format!(
                "gofmt rejected raw decoded-tree render: {}",
                String::from_utf8_lossy(&output.stderr)
            ),
        ));
    }
    String::from_utf8(output.stdout).map_err(|error| {
        SyntaxBackendBoundaryError::new(
            "raw_decoded_tree_render_blocker.gofmt_validation",
            format!("gofmt output was not valid UTF-8: {error}"),
        )
    })
}

/// Token-only canonical-render fidelity for one Go source file.
///
/// This is the round trip the Go elided-stream losslessness claim needs:
/// canonical elided tokens -> `decode_complete_tree_elided` ->
/// `render_go_source_from_raw_tree` -> `gofmt` -> byte comparison against the
/// checked (gofmt-canonical) source. It deliberately shares nothing with the
/// Go-side `canonical_render.matches_source` evidence, which only certifies
/// that the *source* is gofmt-idempotent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoTokenOnlyCanonicalRenderFidelity {
    /// `gofmt(render(decode(tokens)))` equals the checked source bytes.
    pub matches_canonical_source: bool,
    pub rendered_source_sha256: Option<String>,
    pub rendered_source_byte_count: Option<usize>,
    pub formatted_source_sha256: Option<String>,
    pub formatted_source_byte_count: Option<usize>,
    /// Decode, render, or gofmt failure, or the observed/expected hash pair.
    pub diagnostic: Option<String>,
}

/// Run the token-only canonical-render round trip for one canonical elided
/// stream. Never panics and never returns `Err`: a decode, render, or gofmt
/// failure is reported as `matches_canonical_source == false` plus a
/// diagnostic, so corpus-scale callers can bucket the residue.
pub fn go_token_only_canonical_render_fidelity(
    canonical_elided_tokens: &[u32],
    source_sha256: &str,
    source_byte_count: usize,
) -> GoTokenOnlyCanonicalRenderFidelity {
    let blocked = |diagnostic: String| GoTokenOnlyCanonicalRenderFidelity {
        matches_canonical_source: false,
        rendered_source_sha256: None,
        rendered_source_byte_count: None,
        formatted_source_sha256: None,
        formatted_source_byte_count: None,
        diagnostic: Some(diagnostic),
    };
    let artifact = match crate::syntax_go_unified_artifact::go_unified_grammar_artifact() {
        Ok(artifact) => artifact,
        Err(error) => return blocked(format!("unified Go grammar artifact unavailable: {error}")),
    };
    let decoded = match artifact
        .compiled_model
        .decode_complete_tree_elided(canonical_elided_tokens)
    {
        Ok(decoded) => decoded,
        Err(error) => return blocked(format!("elided decode failed: {error:?}")),
    };
    let rendered = match render_go_source_from_raw_tree(&decoded) {
        Ok(rendered) => rendered,
        Err(error) => return blocked(format!("raw-tree render failed: {error}")),
    };
    let rendered_source_sha256 = sha256_hex(rendered.source.as_bytes());
    let rendered_source_byte_count = rendered.source.len();
    let formatted = match gofmt_raw_tree_rendered_source(&rendered.source) {
        Ok(formatted) => formatted,
        Err(error) => {
            return GoTokenOnlyCanonicalRenderFidelity {
                matches_canonical_source: false,
                rendered_source_sha256: Some(rendered_source_sha256),
                rendered_source_byte_count: Some(rendered_source_byte_count),
                formatted_source_sha256: None,
                formatted_source_byte_count: None,
                diagnostic: Some(format!("gofmt of raw-tree render failed: {error}")),
            };
        }
    };
    let formatted_source_sha256 = sha256_hex(formatted.as_bytes());
    let formatted_source_byte_count = formatted.len();
    let matches_canonical_source = formatted_source_sha256 == source_sha256
        && formatted_source_byte_count == source_byte_count;
    let diagnostic = if matches_canonical_source {
        None
    } else {
        Some(format!(
            "token-only render hash {formatted_source_sha256} ({formatted_source_byte_count} bytes) did not match checked source hash {source_sha256} ({source_byte_count} bytes)"
        ))
    };
    GoTokenOnlyCanonicalRenderFidelity {
        matches_canonical_source,
        rendered_source_sha256: Some(rendered_source_sha256),
        rendered_source_byte_count: Some(rendered_source_byte_count),
        formatted_source_sha256: Some(formatted_source_sha256),
        formatted_source_byte_count: Some(formatted_source_byte_count),
        diagnostic,
    }
}

fn raw_tree_root_file_comment_group_count(
    root: &RawSyntaxDecodedNode,
) -> Result<usize, SyntaxBackendBoundaryError> {
    require_string("raw_decoded_tree.root.node_kind", &root.node_kind, "File")?;
    let comments = root.field("Comments").ok_or_else(|| {
        SyntaxBackendBoundaryError::new(
            "raw_decoded_tree_render_blocker.root_file_comment_group_count",
            "retained Go raw decoded tree must include File.Comments",
        )
    })?;
    Ok(comments
        .values
        .iter()
        .filter(|value| match value {
            RawSyntaxDecodedValue::Node(node) => node.node_kind == "CommentGroup",
            RawSyntaxDecodedValue::Terminal(_) => false,
        })
        .count())
}

fn go_tokenizer_generator_report(
    inputs: GoTokenizerGeneratorReportInputs<'_>,
) -> Result<GoTokenizerGeneratorReport, SyntaxBackendBoundaryError> {
    let projection_artifact_sha256 = sha256_hex(inputs.projection_artifact_contents.as_bytes());
    let raw_token_ids_sha256 = json_sha256(&inputs.raw_tokens.raw_token_ids).map_err(|error| {
        SyntaxBackendBoundaryError::new(
            "raw_token_ids_sha256",
            format!("failed to hash raw syntax tokens: {error}"),
        )
    })?;
    require_string(
        "raw_token_ids_sha256",
        &raw_token_ids_sha256,
        &inputs.raw_tokens.raw_token_ids_sha256,
    )?;
    let renderer_position_policy_summary = go_ast_renderer_position_policy_summary()?;
    let raw_state_machine_tree =
        GoTokenizerGeneratorDecodedTreeSummary::from_tree(&inputs.decoded_tree)?;
    let raw_decoded_tree_render_blocker = go_tokenizer_generator_raw_decoded_tree_render_blocker(
        inputs.checked_report,
        inputs.raw_tokens,
        &inputs.decoded_tree,
    )?;
    // Source-record generator core stamps API/stream identity and empty-prefix /
    // selected-completion / elided selected-completion / incomplete-prefix /
    // token-only evidence. Generated-codec report must stay in parity (same
    // emission-derived elision).
    let source_fields = GoSourceRecordManifestDepthRecordSourceFields::from_generator_evidence(
        inputs.source_record_generation,
    );
    source_fields.validate_api_and_stream_identity()?;
    require_string(
        "generated_codec_report.stream_contract",
        &inputs.generated_codec_report.stream_contract,
        &source_fields.stream_contract,
    )?;
    require_usize(
        "generated_codec_report.tct_token_count",
        inputs.generated_codec_report.tct_token_count,
        source_fields.canonical_token_count,
    )?;
    require_string(
        "generated_codec_report.tct_tokens_sha256",
        &inputs.generated_codec_report.tct_tokens_sha256,
        &source_fields.canonical_tokens_sha256,
    )?;
    if inputs.generated_codec_report.max_raw_token_id != source_fields.canonical_max_token_id {
        return Err(SyntaxBackendBoundaryError::new(
            "generated_codec_report.max_raw_token_id",
            "generated-codec report max token id must match source-record generator-core elided identity",
        ));
    }
    require_usize(
        "generated_codec_report.forced_token_count",
        inputs.generated_codec_report.forced_token_count,
        source_fields.forced_token_count,
    )?;

    // Single-record render/reparse pure conjunction is generator-core-owned
    // (includes canonical_sha256 == source.sha256; matches emission sidecar).
    let render_flags =
        GoSourceRecordCanonicalRenderReadinessFlags::from_checked_report(inputs.checked_report);
    let canonical_render_ready = render_flags.canonical_render_ready;
    let go_reparse_ready = render_flags.go_reparse_ready;
    let raw_state_machine_complete = inputs.decoded.status == RawSyntaxStatus::Complete;
    let encode_decode_readiness =
        GoSourceRecordManifestDepthEncodeDecodeReadiness::from_token_emission_and_sm_complete(
            inputs.raw_tokens.raw_tokens_below_vocab_size,
            inputs.raw_tokens.pad_token_absent,
            raw_state_machine_complete,
        );

    Ok(GoTokenizerGeneratorReport {
        artifact_kind: ARTIFACT_KIND.to_string(),
        version: VERSION,
        tool: TOOL.to_string(),
        status: STATUS.to_string(),
        target: TARGET.to_string(),
        route: ROUTE.to_string(),
        candidate_grammar_id: inputs.checked_report.candidate_grammar_id.clone(),
        checked_projection_report_path: inputs
            .checked_projection_report_path
            .display()
            .to_string(),
        projection_report_mode: inputs.projection_report_mode.to_string(),
        projection_artifact_path: inputs.projection_artifact_path.display().to_string(),
        projection_artifact_sha256,
        source_path: inputs.checked_report.source.path.clone(),
        source_sha256: inputs.checked_report.source.sha256.clone(),
        metadata_path: inputs.checked_report.metadata.path.clone(),
        metadata_sha256: inputs.checked_report.metadata.sha256.clone(),
        parser_tool: inputs.checked_report.parser.metadata_tool.clone(),
        parser_version: inputs.checked_report.parser.metadata_version.clone(),
        source_record_generator_core_api_id: source_fields
            .source_record_generator_core_api_id
            .clone(),
        source_record_generator_core_api_boundary: source_fields
            .source_record_generator_core_api_boundary
            .clone(),
        source_record_raw_token_emission_api_id: source_fields
            .source_record_raw_token_emission_api_id
            .clone(),
        source_record_raw_token_emission_api_boundary: source_fields
            .source_record_raw_token_emission_api_boundary
            .clone(),
        delegated_raw_scratch_generator_core_api_id: source_fields
            .delegated_raw_scratch_generator_core_api_id
            .clone(),
        delegated_raw_scratch_generator_core_api_boundary: source_fields
            .delegated_raw_scratch_generator_core_api_boundary
            .clone(),
        renderer_position_policy_summary,
        structural_model_id: inputs.raw_tokens.structural_model_id.clone(),
        grammar_rule_count: inputs.grammar_rule_count,
        grammar_field_count: inputs.grammar_field_count,
        token_layout_vocab_size: inputs.raw_tokens.token_layout.vocab_size,
        token_layout_pad_token: inputs.raw_tokens.token_layout.pad_token,
        raw_token_count: inputs.raw_tokens.raw_token_count,
        raw_token_ids_sha256: raw_token_ids_sha256.clone(),
        max_raw_token_id: inputs.raw_tokens.max_raw_token_id,
        raw_tokens_below_vocab_size: inputs.raw_tokens.raw_tokens_below_vocab_size,
        pad_token_absent: inputs.raw_tokens.pad_token_absent,
        stream_contract: source_fields.stream_contract.clone(),
        canonical_token_count: source_fields.canonical_token_count,
        canonical_tokens_sha256: source_fields.canonical_tokens_sha256.clone(),
        canonical_max_token_id: source_fields.canonical_max_token_id,
        forced_token_count: source_fields.forced_token_count,
        raw_state_machine_complete,
        raw_state_machine_decode: GoTokenizerGeneratorDecodeSummary::from_state(inputs.decoded),
        raw_state_machine_tree,
        token_only_decode_render: source_fields.token_only_decode_render.clone(),
        empty_prefix_generation: source_fields.empty_prefix_generation.clone(),
        selected_completion_smoke: source_fields.selected_completion_smoke.clone(),
        elided_selected_completion: source_fields.elided_selected_completion.clone(),
        incomplete_prefix_rejection: source_fields.incomplete_prefix_rejection.clone(),
        raw_decoded_tree_render_blocker,
        generated_codec_report_artifact_kind: inputs.generated_codec_report.artifact_kind.clone(),
        generated_codec_report_route: inputs.generated_codec_report.route.clone(),
        generated_codec_report_path: inputs.generated_codec_report.report_path.clone(),
        replay_report_artifact_kind: inputs.replay_report.artifact_kind.clone(),
        replay_report_complete: inputs.replay_report.complete_stream.accepted,
        replay_report_prefix_state_api_ready: inputs.replay_report.prefix_inspection_state_api_ready,
        canonical_render: inputs.checked_report.canonical_render.clone(),
        canonical_render_ready,
        gofmt_validation: inputs.checked_report.canonical_render.gofmt.clone(),
        go_parser_reparse_validation: inputs.checked_report.canonical_render.go_parser.clone(),
        declaration_presence: go_tokenizer_generator_declaration_presence_evidence()?,
        state_mask_selection: go_tokenizer_generator_state_mask_selection_evidence()?,
        // Full pipeline field values come from one generator-core stamp
        // (process stages + exclusions + encode/decode + canonical-render).
        pipeline: GoTokenizerGeneratorPipeline::from_field_stamp(
            GoSourceRecordPipelineFieldStamp::from_encode_decode_and_render_flags(
                &encode_decode_readiness,
                canonical_render_ready,
                go_reparse_ready,
            ),
        ),
        next_ready_task: NEXT_READY_TASK.to_string(),
        validation_commands: vec![
            "cargo run --bin tct-syntax-go-tokenizer-generator".to_string(),
            "cargo test --test go_ast_schema_inventory_test".to_string(),
            "go run scripts/analysis/go_ast_schema_inventory.go --self-test".to_string(),
            "cargo test --test syntax_state_machine_replay_test".to_string(),
            "cargo test --test syntax_generated_codec_gate_test".to_string(),
            "cargo test --test syntax_backend_boundary_test".to_string(),
            "cargo test --test syntax_go_source_instance_projection_test go_empty_prefix_scratch_generation_smoke_validates_token_space_boundary -- --exact".to_string(),
            "cargo test --test syntax_go_source_instance_projection_test go_scratch_generation_incomplete_prefix_rejection_validates_continuation_mask -- --exact".to_string(),
            "TCT_GO_BIN=\"${TCT_GO_BIN:-go}\" cargo test --test syntax_go_source_instance_projection_test"
                .to_string(),
            "python3 scripts/testing/test-quality-audit.py --changed-from origin/main".to_string(),
            "cargo fmt --all -- --check".to_string(),
            "cargo clippy --all-targets".to_string(),
            "git diff --check".to_string(),
        ],
        claim_boundary: CLAIM_BOUNDARY.to_string(),
        no_stable_claims: NO_STABLE_CLAIMS
            .iter()
            .map(|claim| (*claim).to_string())
            .collect(),
    })
}

fn go_tokenizer_generator_declaration_presence_evidence()
-> Result<GoTokenizerGeneratorDeclarationPresenceEvidence, SyntaxBackendBoundaryError> {
    let replay_report = build_go_ast_declaration_presence_replay_report()?;
    replay_report.validate()?;
    let type_assign_evidence = go_declaration_presence_generated_evidence(
        "go_type_spec_assign_syntax_v0",
        GO_TYPE_SPEC_ASSIGN_METADATA_PATH,
        "go-type-spec-assign-syntax-generated-codec",
        "generated_go_type_spec_assign_preserves_inventory_type_spec_assign_presence",
        "generated_go_type_spec_assign_renders_canonical_source",
        "generated_go_type_spec_assign_render_passes_go_parser_sidecar",
        "type Counter = int",
        GO_TYPE_SPEC_ASSIGN_GENERATED_TEST_MARKERS,
    )?;
    let type_doc_evidence = go_declaration_presence_generated_evidence(
        "go_type_spec_doc_syntax_v0",
        GO_TYPE_SPEC_DOC_METADATA_PATH,
        "go-type-spec-doc-syntax-generated-codec",
        "generated_go_type_spec_doc_preserves_inventory_gen_decl_group_presence",
        "generated_go_type_spec_doc_renders_canonical_source",
        "generated_go_type_spec_doc_render_passes_go_parser_sidecar",
        "type (",
        GO_TYPE_SPEC_DOC_GENERATED_TEST_MARKERS,
    )?;
    let function_literal_evidence = go_declaration_presence_generated_evidence(
        "go_function_literal_syntax_v0",
        GO_FUNCTION_LITERAL_METADATA_PATH,
        "go-function-literal-syntax-generated-codec",
        "generated_go_function_literal_preserves_inventory_func_type_func_presence",
        "generated_go_function_literal_renders_canonical_source",
        "generated_go_function_literal_render_passes_go_parser_sidecar",
        "return func() int",
        GO_FUNCTION_LITERAL_GENERATED_TEST_MARKERS,
    )?;
    let variadic_ellipsis_evidence = go_declaration_presence_generated_evidence(
        "go_local_variadic_ellipsis_syntax_v0",
        GO_LOCAL_VARIADIC_ELLIPSIS_METADATA_PATH,
        "go-local-variadic-ellipsis-syntax-generated-codec",
        "generated_go_local_variadic_ellipsis_preserves_inventory_call_expr_ellipsis_presence",
        "generated_go_local_variadic_ellipsis_renders_canonical_source",
        "generated_go_local_variadic_ellipsis_render_passes_go_parser_sidecar",
        "values...",
        GO_LOCAL_VARIADIC_ELLIPSIS_GENERATED_TEST_MARKERS,
    )?;
    let generated_evidence = vec![
        type_assign_evidence.clone(),
        type_doc_evidence.clone(),
        function_literal_evidence.clone(),
        variadic_ellipsis_evidence.clone(),
    ];
    let mut presence_decisions = Vec::with_capacity(replay_report.presence_decisions.len());
    for decision in &replay_report.presence_decisions {
        let generated = match (decision.owner_rule.as_str(), decision.owner_field.as_str()) {
            ("TypeSpec", "Assign") => &type_assign_evidence,
            ("GenDecl", "Lparen") | ("GenDecl", "Rparen") => &type_doc_evidence,
            ("FuncType", "Func") => &function_literal_evidence,
            ("CallExpr", "Ellipsis") => &variadic_ellipsis_evidence,
            (owner_rule, owner_field) => {
                return Err(SyntaxBackendBoundaryError::new(
                    "declaration_presence.decision",
                    format!("unsupported declaration presence decision {owner_rule}.{owner_field}"),
                ));
            }
        };
        presence_decisions.push(go_tokenizer_generator_declaration_presence_decision(
            decision, generated,
        ));
    }
    let evidence = GoTokenizerGeneratorDeclarationPresenceEvidence {
        status: DECLARATION_PRESENCE_STATUS.to_string(),
        raw_replay_artifact_kind: replay_report.artifact_kind,
        raw_replay_next_ready_task: replay_report.next_ready_task,
        raw_replay_decisions_consumed: replay_report.declaration_presence_decisions_consumed,
        raw_replay_raw_token_ids_sha256: replay_report.raw_token_ids_sha256,
        generated_render_decisions_preserved: true,
        render_reparse_checks_ready: true,
        presence_decisions,
        generated_evidence,
        next_ready_task: NEXT_READY_TASK.to_string(),
        claim_boundary: DECLARATION_PRESENCE_CLAIM_BOUNDARY.to_string(),
    };
    evidence.validate()?;
    Ok(evidence)
}

fn go_tokenizer_generator_state_mask_selection_evidence()
-> Result<GoTokenizerGeneratorStateMaskSelectionEvidence, SyntaxBackendBoundaryError> {
    let replay_report = build_go_ast_declaration_presence_replay_report()?;
    replay_report.validate()?;
    let decision = replay_report
        .presence_decisions
        .iter()
        .find(|decision| decision.owner_rule == "TypeSpec" && decision.owner_field == "Assign")
        .ok_or_else(|| {
            SyntaxBackendBoundaryError::new(
                "state_mask_selection.replay_decision",
                "missing Go TypeSpec.Assign presence decision",
            )
        })?;
    let generated = go_declaration_presence_generated_evidence(
        "go_type_spec_assign_syntax_v0",
        GO_TYPE_SPEC_ASSIGN_METADATA_PATH,
        "go-type-spec-assign-syntax-generated-codec",
        "generated_go_type_spec_assign_preserves_inventory_type_spec_assign_presence",
        GO_TYPE_SPEC_ASSIGN_RENDER_TEST_MARKER,
        GO_TYPE_SPEC_ASSIGN_REPARSE_TEST_MARKER,
        GO_TYPE_SPEC_ASSIGN_EXPECTED_SOURCE_SNIPPET,
        GO_TYPE_SPEC_ASSIGN_GENERATED_TEST_MARKERS,
    )?;
    let artifact = build_registered_parser_backed_syntax_generated_codec_artifact(
        "go_type_spec_assign_syntax_v0",
    )?;
    let evidence = GoTokenizerGeneratorStateMaskSelectionEvidence {
        status: STATE_MASK_SELECTION_STATUS.to_string(),
        scope: STATE_MASK_SELECTION_SCOPE.to_string(),
        replay_artifact_kind: replay_report.artifact_kind,
        generated_fixture_grammar_id: generated.grammar_id,
        generated_fixture_metadata_path: generated.metadata_path,
        owner_rule: decision.owner_rule.clone(),
        owner_field: decision.owner_field.clone(),
        domain_id: decision.domain_id.clone(),
        selected_node_kind: decision.selected_node_kind.clone(),
        boolean_payload_kind: decision.boolean_payload_kind.clone(),
        terminal_raw_token_offset: decision.terminal_raw_token_offset,
        selected_raw_token_offset: decision.boolean_payload_raw_token_offset,
        selected_raw_token_id: decision.boolean_payload_raw_token_id,
        finite_admissible_raw_token_ids: decision
            .state_after_terminal_finite_admissible_raw_token_ids
            .clone(),
        selected_raw_token_admitted_by_mask: decision
            .state_after_terminal_finite_admissible_raw_token_ids
            .contains(&decision.boolean_payload_raw_token_id),
        decoded_selected_raw_token: decision.boolean_payload_decoded,
        complete_stream_decoded: replay_report.complete_stream.accepted,
        generated_prefix_state_marker: GO_TYPE_SPEC_ASSIGN_PREFIX_STATE_TEST_MARKER.to_string(),
        generated_render_marker: GO_TYPE_SPEC_ASSIGN_RENDER_TEST_MARKER.to_string(),
        generated_reparse_marker: GO_TYPE_SPEC_ASSIGN_REPARSE_TEST_MARKER.to_string(),
        generated_prefix_state_marker_consumed: artifact
            .source
            .contains(GO_TYPE_SPEC_ASSIGN_PREFIX_STATE_TEST_MARKER),
        generated_render_reparse_ready: generated.generated_source_contains_render_test
            && generated.generated_source_contains_reparse_test,
        expected_source_snippet: GO_TYPE_SPEC_ASSIGN_EXPECTED_SOURCE_SNIPPET.to_string(),
        generated_source_contains_expected_snippet: generated
            .generated_source_contains_expected_snippet,
        arena_ir_used: false,
        route_handoff_provenance_used: false,
        bpe_validity_authority_used: false,
        stable_backend_claimed: false,
        claim_boundary: STATE_MASK_SELECTION_CLAIM_BOUNDARY.to_string(),
    };
    evidence.validate(replay_report.token_layout.vocab_size)?;
    Ok(evidence)
}

#[allow(clippy::too_many_arguments)]
fn go_declaration_presence_generated_evidence(
    grammar_id: &str,
    metadata_path: &str,
    generated_crate_name: &str,
    presence_test_marker: &str,
    render_test_marker: &str,
    reparse_test_marker: &str,
    expected_source_snippet: &str,
    generated_test_markers: &[&str],
) -> Result<GoTokenizerGeneratorDeclarationGeneratedEvidence, SyntaxBackendBoundaryError> {
    let artifact = build_registered_parser_backed_syntax_generated_codec_artifact(grammar_id)?;
    validate_declaration_presence_artifact(
        &artifact,
        grammar_id,
        metadata_path,
        generated_crate_name,
        presence_test_marker,
        render_test_marker,
        reparse_test_marker,
        expected_source_snippet,
    )?;
    if !generated_test_markers.contains(&presence_test_marker) {
        return Err(SyntaxBackendBoundaryError::new(
            "declaration_presence.generated.registry_marker",
            format!("registry markers for {grammar_id} do not include {presence_test_marker}"),
        ));
    }
    Ok(GoTokenizerGeneratorDeclarationGeneratedEvidence {
        grammar_id: grammar_id.to_string(),
        metadata_path: metadata_path.to_string(),
        generated_crate_name: generated_crate_name.to_string(),
        presence_test_marker: presence_test_marker.to_string(),
        render_test_marker: render_test_marker.to_string(),
        reparse_test_marker: reparse_test_marker.to_string(),
        expected_source_snippet: expected_source_snippet.to_string(),
        rendered_source_sha256: sha256_hex(artifact.source.as_bytes()),
        generated_source_contains_presence_test: artifact.source.contains(presence_test_marker),
        generated_source_contains_render_test: artifact.source.contains(render_test_marker),
        generated_source_contains_reparse_test: artifact.source.contains(reparse_test_marker),
        generated_source_contains_expected_snippet: artifact
            .source
            .contains(expected_source_snippet),
        registry_marker_consumed: true,
    })
}

#[allow(clippy::too_many_arguments)]
fn validate_declaration_presence_artifact(
    artifact: &SyntaxGeneratedCodecArtifact,
    grammar_id: &str,
    metadata_path: &str,
    generated_crate_name: &str,
    presence_test_marker: &str,
    render_test_marker: &str,
    reparse_test_marker: &str,
    expected_source_snippet: &str,
) -> Result<(), SyntaxBackendBoundaryError> {
    require_string(
        "declaration_presence.generated.target",
        artifact.target,
        TARGET,
    )?;
    require_string(
        "declaration_presence.generated.grammar_id",
        artifact
            .metadata
            .get("grammar_input")
            .and_then(|value| value.get("grammar_id"))
            .and_then(|value| value.as_str())
            .unwrap_or(""),
        grammar_id,
    )?;
    require_string(
        "declaration_presence.generated.metadata_path",
        artifact
            .metadata
            .get("source_metadata_provenance")
            .and_then(|value| value.get("metadata_path"))
            .and_then(|value| value.as_str())
            .unwrap_or(""),
        metadata_path,
    )?;
    require_string(
        "declaration_presence.generated.crate_name",
        artifact
            .metadata
            .get("generated_crate")
            .and_then(|value| value.get("crate_name"))
            .and_then(|value| value.as_str())
            .unwrap_or(""),
        generated_crate_name,
    )?;
    for (field, marker) in [
        ("presence_test_marker", presence_test_marker),
        ("render_test_marker", render_test_marker),
        ("reparse_test_marker", reparse_test_marker),
    ] {
        if !artifact.source.contains(marker) {
            return Err(SyntaxBackendBoundaryError::new(
                field,
                format!("generated source for {grammar_id} is missing {marker}"),
            ));
        }
    }
    if !artifact.source.contains(expected_source_snippet) {
        return Err(SyntaxBackendBoundaryError::new(
            "declaration_presence.generated.expected_source_snippet",
            format!("generated source for {grammar_id} is missing {expected_source_snippet:?}"),
        ));
    }
    Ok(())
}

fn go_tokenizer_generator_declaration_presence_decision(
    replay_decision: &GoAstDeclarationPresenceDecision,
    generated: &GoTokenizerGeneratorDeclarationGeneratedEvidence,
) -> GoTokenizerGeneratorDeclarationPresenceDecision {
    GoTokenizerGeneratorDeclarationPresenceDecision {
        owner_rule: replay_decision.owner_rule.clone(),
        owner_field: replay_decision.owner_field.clone(),
        domain_id: replay_decision.domain_id.clone(),
        selected_node_kind: replay_decision.selected_node_kind.clone(),
        boolean_payload_kind: replay_decision.boolean_payload_kind.clone(),
        generated_fixture_grammar_id: generated.grammar_id.clone(),
        generated_fixture_metadata_path: generated.metadata_path.clone(),
        generated_presence_test_marker: generated.presence_test_marker.clone(),
        generated_render_test_marker: generated.render_test_marker.clone(),
        generated_reparse_test_marker: generated.reparse_test_marker.clone(),
        generated_render_preserves_presence: generated.generated_source_contains_expected_snippet,
        generated_reparse_check_present: generated.generated_source_contains_reparse_test,
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct GoTokenizerGeneratorReportIngestionManifest {
    artifact_kind: String,
    version: u32,
    tool: String,
    corpus_id: String,
    source_corpus_manifest_path: String,
    package_metadata: GoTokenizerGeneratorReportIngestionPackageMetadata,
    record_order_policy: String,
    records: Vec<GoTokenizerGeneratorReportIngestionRecord>,
    no_stable_promotion_claim: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct GoTokenizerGeneratorReportIngestionPackageMetadata {
    status: String,
    completed_promotion_class: String,
    next_missing_workstream_blocker: String,
    /// Canonical stream contract of every record's `expected_tct_*` stream.
    /// Required for current-plus manifests (enforced by the depth gate);
    /// historical pre-unification manifests omit it.
    #[serde(default)]
    stream_contract: String,
    /// Pinned unified Go grammar artifact identity covering every record.
    #[serde(default)]
    grammar_artifact_id: String,
    #[serde(default)]
    grammar_artifact_content_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct GoTokenizerGeneratorReportIngestionRecord {
    role: String,
    evidence_scope: String,
    manifest_path: String,
    report_path: String,
    #[serde(default)]
    raw_tree_render_attempt: bool,
    generated_codec_route: String,
    source_path: String,
    source_sha256: String,
    source_bytes: usize,
    package: String,
    source_function: String,
    expected_tct_token_count: usize,
    expected_tct_tokens_sha256: String,
    expected_max_raw_token_id: Option<u32>,
    expected_vocab_size: u32,
    expected_pad_token: u32,
    expected_pad_token_absent: bool,
    go_parser_version: String,
    generated_codec_report_artifact_kind: String,
    canonical_render: GoTokenizerGeneratorReportIngestionCanonicalRender,
    expected_claim_boundary: String,
    source_program_validity_claim_allowed: bool,
    stable_promotion_allowed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct GoTokenizerGeneratorReportIngestionCanonicalRender {
    bytes: usize,
    sha256: String,
}

fn parse_go_tokenizer_generator_report_ingestion_manifest(
    contents: &str,
) -> Result<GoTokenizerGeneratorReportIngestionManifest, SyntaxBackendBoundaryError> {
    serde_json::from_str(contents).map_err(|error| {
        SyntaxBackendBoundaryError::new(
            "go_tokenizer_generator_report_ingestion_manifest",
            format!("failed to parse Go tokenizer generator report-ingestion manifest: {error}"),
        )
    })
}

fn validate_go_tokenizer_generator_report_ingestion_manifest(
    manifest: &GoTokenizerGeneratorReportIngestionManifest,
) -> Result<(), SyntaxBackendBoundaryError> {
    require_string(
        "report_ingestion_manifest.artifact_kind",
        &manifest.artifact_kind,
        REPORT_INGESTION_MANIFEST_ARTIFACT_KIND,
    )?;
    require_u32(
        "report_ingestion_manifest.version",
        manifest.version,
        VERSION,
    )?;
    require_string(
        "report_ingestion_manifest.tool",
        &manifest.tool,
        REPORT_INGESTION_MANIFEST_TOOL,
    )?;
    require_non_empty("report_ingestion_manifest.corpus_id", &manifest.corpus_id)?;
    require_non_empty(
        "report_ingestion_manifest.source_corpus_manifest_path",
        &manifest.source_corpus_manifest_path,
    )?;
    validate_go_tokenizer_generator_report_ingestion_package_metadata(&manifest.package_metadata)?;
    require_string(
        "report_ingestion_manifest.record_order_policy",
        &manifest.record_order_policy,
        "records array order is the generated-codec gate evidence order; it is not a training JSONL package order",
    )?;
    require_positive_usize("report_ingestion_manifest.records", manifest.records.len())?;
    for record in &manifest.records {
        validate_go_tokenizer_generator_report_ingestion_record(record)?;
    }
    require_contains(
        "report_ingestion_manifest.no_stable_promotion_claim",
        std::slice::from_ref(&manifest.no_stable_promotion_claim),
        "no TctCompiler route",
    )?;
    require_contains(
        "report_ingestion_manifest.no_stable_promotion_claim",
        std::slice::from_ref(&manifest.no_stable_promotion_claim),
        "BPE validity authority",
    )?;
    require_contains(
        "report_ingestion_manifest.no_stable_promotion_claim",
        std::slice::from_ref(&manifest.no_stable_promotion_claim),
        "stable backend promotion",
    )
}

fn validate_go_tokenizer_generator_report_ingestion_package_metadata(
    metadata: &GoTokenizerGeneratorReportIngestionPackageMetadata,
) -> Result<(), SyntaxBackendBoundaryError> {
    require_non_empty(
        "report_ingestion_manifest.package_metadata.status",
        &metadata.status,
    )?;
    require_non_empty(
        "report_ingestion_manifest.package_metadata.completed_promotion_class",
        &metadata.completed_promotion_class,
    )?;
    require_non_empty(
        "report_ingestion_manifest.package_metadata.next_missing_workstream_blocker",
        &metadata.next_missing_workstream_blocker,
    )?;
    // Historical pre-unification manifests omit the unified-artifact block;
    // when present it must match the live pinned identity exactly.
    if !metadata.stream_contract.is_empty()
        || !metadata.grammar_artifact_id.is_empty()
        || !metadata.grammar_artifact_content_sha256.is_empty()
    {
        require_string(
            "report_ingestion_manifest.package_metadata.stream_contract",
            &metadata.stream_contract,
            LIVE_RAW_SYNTAX_STREAM_CONTRACT,
        )?;
        let unified_artifact = go_unified_grammar_artifact()?;
        require_string(
            "report_ingestion_manifest.package_metadata.grammar_artifact_id",
            &metadata.grammar_artifact_id,
            unified_artifact.artifact_id,
        )?;
        require_string(
            "report_ingestion_manifest.package_metadata.grammar_artifact_content_sha256",
            &metadata.grammar_artifact_content_sha256,
            &unified_artifact.content_sha256,
        )?;
    }
    Ok(())
}

fn validate_go_tokenizer_generator_report_ingestion_record(
    record: &GoTokenizerGeneratorReportIngestionRecord,
) -> Result<(), SyntaxBackendBoundaryError> {
    require_string(
        "report_ingestion_manifest.record.role",
        &record.role,
        CURRENT_PLUS_GENERATED_CODEC_ROLE,
    )?;
    require_string(
        "report_ingestion_manifest.record.evidence_scope",
        &record.evidence_scope,
        SOURCE_INSTANCE_GENERATED_CODEC_EVIDENCE_SCOPE,
    )?;
    require_non_empty(
        "report_ingestion_manifest.record.manifest_path",
        &record.manifest_path,
    )?;
    require_non_empty(
        "report_ingestion_manifest.record.report_path",
        &record.report_path,
    )?;
    require_string(
        "report_ingestion_manifest.record.generated_codec_route",
        &record.generated_codec_route,
        GENERATED_CODEC_REPORT_ROUTE,
    )?;
    require_non_empty(
        "report_ingestion_manifest.record.source_path",
        &record.source_path,
    )?;
    require_sha256(
        "report_ingestion_manifest.record.source_sha256",
        &record.source_sha256,
    )?;
    require_positive_usize(
        "report_ingestion_manifest.record.source_bytes",
        record.source_bytes,
    )?;
    require_non_empty("report_ingestion_manifest.record.package", &record.package)?;
    require_string(
        "report_ingestion_manifest.record.source_function",
        &record.source_function,
        "source_file",
    )?;
    require_positive_usize(
        "report_ingestion_manifest.record.expected_tct_token_count",
        record.expected_tct_token_count,
    )?;
    require_sha256(
        "report_ingestion_manifest.record.expected_tct_tokens_sha256",
        &record.expected_tct_tokens_sha256,
    )?;
    require_u32(
        "report_ingestion_manifest.record.expected_pad_token",
        record.expected_pad_token,
        record.expected_vocab_size.saturating_sub(1),
    )?;
    require_bool(
        "report_ingestion_manifest.record.expected_pad_token_absent",
        record.expected_pad_token_absent,
        true,
    )?;
    require_non_empty(
        "report_ingestion_manifest.record.go_parser_version",
        &record.go_parser_version,
    )?;
    require_string(
        "report_ingestion_manifest.record.generated_codec_report_artifact_kind",
        &record.generated_codec_report_artifact_kind,
        GENERATED_CODEC_REPORT_ARTIFACT_KIND,
    )?;
    require_positive_usize(
        "report_ingestion_manifest.record.canonical_render.bytes",
        record.canonical_render.bytes,
    )?;
    require_string(
        "report_ingestion_manifest.record.canonical_render.sha256",
        &record.canonical_render.sha256,
        &record.source_sha256,
    )?;
    validate_report_ingestion_claim_boundary(&record.expected_claim_boundary)?;
    require_bool(
        "report_ingestion_manifest.record.source_program_validity_claim_allowed",
        record.source_program_validity_claim_allowed,
        false,
    )?;
    require_bool(
        "report_ingestion_manifest.record.stable_promotion_allowed",
        record.stable_promotion_allowed,
        false,
    )
}

fn validate_report_ingestion_claim_boundary(
    claim_boundary: &str,
) -> Result<(), SyntaxBackendBoundaryError> {
    require_str_contains(
        "report_ingestion_manifest.record.expected_claim_boundary",
        claim_boundary,
        "Parser-backed Go metadata",
    )?;
    require_str_contains(
        "report_ingestion_manifest.record.expected_claim_boundary",
        claim_boundary,
        "GrammarIR",
    )?;
    require_str_contains(
        "report_ingestion_manifest.record.expected_claim_boundary",
        claim_boundary,
        "StructuralModel",
    )?;
    require_str_contains(
        "report_ingestion_manifest.record.expected_claim_boundary",
        claim_boundary,
        "contextual raw tokens",
    )?;
    require_str_contains(
        "report_ingestion_manifest.record.expected_claim_boundary",
        claim_boundary,
        "not arena-IR",
    )?;
    require_str_contains(
        "report_ingestion_manifest.record.expected_claim_boundary",
        claim_boundary,
        "not stable-backend",
    )
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

fn max_optional_token(left: Option<u32>, right: Option<u32>) -> Option<u32> {
    match (left, right) {
        (Some(left), Some(right)) => Some(left.max(right)),
        (Some(value), None) | (None, Some(value)) => Some(value),
        (None, None) => None,
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

fn required_oracle_cutover_field<T: Clone>(
    field: &'static str,
    actual: &Option<T>,
) -> Result<T, SyntaxBackendBoundaryError> {
    actual.clone().ok_or_else(|| {
        SyntaxBackendBoundaryError::new(field, "oracle cutover report is missing required field")
    })
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

fn require_projection_report_mode(actual: &str) -> Result<(), SyntaxBackendBoundaryError> {
    match actual {
        PROJECTION_REPORT_MODE_CHECKED_FIXTURE | PROJECTION_REPORT_MODE_SOURCE_DRIVEN_GENERATED => {
            Ok(())
        }
        other => Err(SyntaxBackendBoundaryError::new(
            "projection_report_mode",
            format!(
                "expected {PROJECTION_REPORT_MODE_CHECKED_FIXTURE} or {PROJECTION_REPORT_MODE_SOURCE_DRIVEN_GENERATED}, got {other}"
            ),
        )),
    }
}

fn require_positive_usize(
    field: &'static str,
    actual: usize,
) -> Result<(), SyntaxBackendBoundaryError> {
    if actual == 0 {
        Err(SyntaxBackendBoundaryError::new(
            field,
            "value must be positive",
        ))
    } else {
        Ok(())
    }
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

fn require_exact_strings(
    field: &'static str,
    actual: &[String],
    expected: &[&str],
) -> Result<(), SyntaxBackendBoundaryError> {
    if actual.len() != expected.len() {
        return Err(SyntaxBackendBoundaryError::new(
            field,
            format!("expected {} values, got {}", expected.len(), actual.len()),
        ));
    }
    for (actual, expected) in actual.iter().zip(expected) {
        require_string(field, actual, expected)?;
    }
    Ok(())
}

fn validate_template_finite_domain_choices(
    choices: &[GoAstInventoryTemplateFiniteDomainChoice],
) -> Result<(), SyntaxBackendBoundaryError> {
    require_positive_usize(
        "go_ast_inventory_template_coverage_frontier.covered_finite_domain_choices",
        choices.len(),
    )?;
    let mut previous: Option<String> = None;
    for choice in choices {
        require_non_empty(
            "go_ast_inventory_template_coverage_frontier.covered_finite_domain_choices.rule_name",
            &choice.rule_name,
        )?;
        require_non_empty(
            "go_ast_inventory_template_coverage_frontier.covered_finite_domain_choices.field_name",
            &choice.field_name,
        )?;
        require_non_empty(
            "go_ast_inventory_template_coverage_frontier.covered_finite_domain_choices.alternative",
            &choice.alternative,
        )?;
        let key = template_finite_domain_choice_key(choice);
        if let Some(previous) = previous
            && previous >= key
        {
            return Err(SyntaxBackendBoundaryError::new(
                "go_ast_inventory_template_coverage_frontier.covered_finite_domain_choices",
                "finite-domain choices must be sorted and unique",
            ));
        }
        previous = Some(key);
    }
    Ok(())
}

fn template_finite_domain_choice_cmp(
    left: &GoAstInventoryTemplateFiniteDomainChoice,
    right: &GoAstInventoryTemplateFiniteDomainChoice,
) -> std::cmp::Ordering {
    template_finite_domain_choice_key(left).cmp(&template_finite_domain_choice_key(right))
}

fn template_finite_domain_choice_key(choice: &GoAstInventoryTemplateFiniteDomainChoice) -> String {
    format!(
        "{}.{}.{}.{:?}",
        choice.rule_name, choice.field_name, choice.alternative, choice.terminal_kind
    )
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
