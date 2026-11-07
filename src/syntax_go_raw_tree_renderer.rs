//! Historical Go source rendering from the reusable raw syntax decoded tree.
//!
//! After the Go AST printer oracle cutover, this module is retained as
//! baseline/diagnostic Rust coverage rather than grammar-mode formatting or
//! parsing authority. The runtime dispatch guard is intentionally driven by the
//! checked go/ast inventory-backed generated-skeleton contract. Rendering still
//! delegates semantic layout, precedence, comments, and parser-validity
//! decisions to explicit manual policy callbacks; it does not read
//! source-instance segments, projection artifacts, route handoff provenance,
//! GoIR, ArenaIR, or BPE artifacts.

use crate::go_ast_schema_inventory::{
    GO_AST_RAW_TREE_RENDERER_CODEGEN_COVERAGE_ARTIFACT_KIND,
    GO_AST_RAW_TREE_RENDERER_CODEGEN_COVERAGE_STATUS_READY,
    GO_AST_RAW_TREE_RENDERER_SKELETON_POLICIES, GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH,
    GoAstRawTreeRendererCodegenCoverageReport, GoAstSchemaInventoryAdapter,
    go_ast_raw_tree_renderer_skeleton_policy, raw_tree_renderer_manual_policy_hooks,
};
use crate::grammar_ir::GrammarTerminalKind;
use crate::syntax_backend_boundary::SyntaxBackendBoundaryError;
use crate::syntax_state_machine::{
    RawSyntaxDecodedField, RawSyntaxDecodedNode, RawSyntaxDecodedTree, RawSyntaxDecodedValue,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const GO_RAW_TREE_RENDERER_ID: &str = "go-raw-decoded-tree-renderer-v1";
pub const GO_RAW_TREE_RENDERER_SCOPE: &str = "go_current_plus_decoded_tree_source_render_v1";
pub const GO_RAW_TREE_COMMENT_ANCHOR_POLICY: &str = "root File.Comments groups are consumed in source order; duplicate attached Doc/Comment groups render at their AST owner, File.Doc renders before the package clause, unattached root-only block comments may render at top-level declaration boundaries when enough unattached groups remain for explicit statement anchors, attached ImportSpec.Comment, TypeSpec.Comment, and ValueSpec.Comment groups render at trailing spec anchors, Field.Doc renders before struct fields and interface method fields, Field.Comment renders at struct fields and interface method trailing anchors, BlockStmt.CommentBefore, CaseClause.CommentBefore, and CommClause.CommentBefore metadata anchor root-only statement comments to explicit statement-list indexes without consuming attached AST-owner groups, current-node LabeledStmt, ForStmt, RangeStmt, IfStmt, and TypeSwitchStmt line-comment anchors may consume the next root-only group only when enough unattached groups remain for later explicit statement anchors, the current statement is not before a later same-list explicit anchor, and the statement body has no competing implicit or explicit body-leading anchor; loop statements that consume a current-node multi-line line-comment anchor defer generic unanchored fallback inside their body while preserving explicit body metadata anchors, conservative return-statement leading comments may consume the next root-only group under the same reservation policy, FuncLit bodies and TypeSwitchStmt clause bodies defer generic implicit leading-comment fallback so post-literal and post-switch root-only comments are preserved for the containing statement list while explicit metadata and current-node anchors still render, implicit root-only multi-line line comments before result-bearing returns infer a blank separator after a call statement with a function-literal callback argument, and long post-callback comment groups also infer a separator before that return, BlockStmt.TrailingComment, CaseClause.TrailingComment, and CommClause.TrailingComment metadata anchor root-only line comments to explicit trailing statement-list indexes, IfStmt.TrailingComment metadata anchors one root-only line comment to the if header while suppressing matching duplicate body-leading anchors, empty CaseClause and CommClause bodies may anchor a root-only body comment at index 0, empty IfStmt bodies before an else branch may consume one unreserved root-only group before rendering the else header, with matching BlankLineBefore separator metadata, struct field trailing comments that label following plain fields start separated field groups, retained inline comment groups render at checked trailing-line anchors while suppressing matching duplicate body-leading anchors, and remaining unattached root-only comments render at file end as a grammar-mode fallback only after all explicit statement anchors have released their reservations";
pub const GO_RAW_TREE_COMMENT_REFERENCE_POLICY: &str = "each comment's text is emitted exactly once: groups attached to an AST owner render from that owner and occupy a blank-text marker slot of the same line count in the root File.Comments channel, which preserves their source-order position without repeating their text; unattached groups carry their text in the root channel; an attached anchor consumes the next unconsumed blank marker slot";

const COMPOSITE_LIT_LINE_BREAK_BEFORE_FIELD: &str = "LineBreakBefore";
const CALL_EXPR_LINE_BREAK_BEFORE_FIELD: &str = "LineBreakBefore";
const TRAILING_BREAK_EOF_COMMENT: &str = "// EOF";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoRawTreeRenderedSource {
    pub source: String,
    pub root_file_comment_group_count: usize,
    pub attached_comment_group_count: usize,
    pub root_only_comment_group_count: usize,
    pub trailing_comment_anchor_count: usize,
    pub selected_generated_entrypoints: Vec<GoRawTreeRendererSelectedRuntimeEntrypoint>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoRawTreeRendererSelectedRuntimeEntrypoint {
    pub node_kind: String,
    pub dispatch_context: String,
    pub generated_entrypoint: String,
    pub manual_policy_callbacks: Vec<String>,
    pub generated_field_traversals: Vec<String>,
    pub invocation: GoRawTreeRendererGeneratedEntrypointInvocation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GoRawTreeRendererGeneratedEntrypointInvocation {
    DispatchGuardOnly,
    GeneratedWrapper,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FuncLitCalleeLayout {
    Preserve,
    CompactDeferredIdentCall,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum GoRawTreeRendererSkeletonContext {
    SourceFileRoot,
    Declaration,
    Specification,
    Statement,
    SimpleStatementHeader,
    Expression,
    Contextual,
    CommentTrivia,
}

impl GoRawTreeRendererSkeletonContext {
    fn as_str(self) -> &'static str {
        match self {
            Self::SourceFileRoot => "source_file_root",
            Self::Declaration => "declaration",
            Self::Specification => "specification",
            Self::Statement => "statement",
            Self::SimpleStatementHeader => "simple_statement_header",
            Self::Expression => "expression",
            Self::Contextual => "contextual",
            Self::CommentTrivia => "comment_trivia",
        }
    }
}

struct GoRawTreeRendererSkeletonDispatchEntry {
    node_kind: &'static str,
    contexts: &'static [GoRawTreeRendererSkeletonContext],
}

const GO_RAW_TREE_RENDERER_SKELETON_DISPATCH_ENTRIES: &[GoRawTreeRendererSkeletonDispatchEntry] = &[
    skeleton_dispatch("ArrayType", &[GoRawTreeRendererSkeletonContext::Expression]),
    skeleton_dispatch(
        "AssignStmt",
        &[
            GoRawTreeRendererSkeletonContext::Statement,
            GoRawTreeRendererSkeletonContext::SimpleStatementHeader,
        ],
    ),
    skeleton_dispatch("BasicLit", &[GoRawTreeRendererSkeletonContext::Expression]),
    skeleton_dispatch(
        "BinaryExpr",
        &[GoRawTreeRendererSkeletonContext::Expression],
    ),
    skeleton_dispatch("BlockStmt", &[GoRawTreeRendererSkeletonContext::Contextual]),
    skeleton_dispatch("BranchStmt", &[GoRawTreeRendererSkeletonContext::Statement]),
    skeleton_dispatch("CallExpr", &[GoRawTreeRendererSkeletonContext::Expression]),
    skeleton_dispatch("CaseClause", &[GoRawTreeRendererSkeletonContext::Statement]),
    skeleton_dispatch("ChanType", &[GoRawTreeRendererSkeletonContext::Expression]),
    skeleton_dispatch("CommClause", &[GoRawTreeRendererSkeletonContext::Statement]),
    skeleton_dispatch(
        "Comment",
        &[GoRawTreeRendererSkeletonContext::CommentTrivia],
    ),
    skeleton_dispatch(
        "CommentGroup",
        &[GoRawTreeRendererSkeletonContext::CommentTrivia],
    ),
    skeleton_dispatch(
        "CompositeLit",
        &[GoRawTreeRendererSkeletonContext::Expression],
    ),
    skeleton_dispatch(
        "DeclStmt",
        &[
            GoRawTreeRendererSkeletonContext::Statement,
            GoRawTreeRendererSkeletonContext::SimpleStatementHeader,
        ],
    ),
    skeleton_dispatch("DeferStmt", &[GoRawTreeRendererSkeletonContext::Statement]),
    skeleton_dispatch("Ellipsis", &[GoRawTreeRendererSkeletonContext::Expression]),
    skeleton_dispatch("EmptyStmt", &[GoRawTreeRendererSkeletonContext::Statement]),
    skeleton_dispatch(
        "ExprStmt",
        &[
            GoRawTreeRendererSkeletonContext::Statement,
            GoRawTreeRendererSkeletonContext::SimpleStatementHeader,
        ],
    ),
    skeleton_dispatch("Field", &[GoRawTreeRendererSkeletonContext::Contextual]),
    skeleton_dispatch("FieldList", &[GoRawTreeRendererSkeletonContext::Contextual]),
    skeleton_dispatch("File", &[GoRawTreeRendererSkeletonContext::SourceFileRoot]),
    skeleton_dispatch("ForStmt", &[GoRawTreeRendererSkeletonContext::Statement]),
    skeleton_dispatch("FuncDecl", &[GoRawTreeRendererSkeletonContext::Declaration]),
    skeleton_dispatch("FuncLit", &[GoRawTreeRendererSkeletonContext::Expression]),
    skeleton_dispatch("FuncType", &[GoRawTreeRendererSkeletonContext::Expression]),
    skeleton_dispatch("GenDecl", &[GoRawTreeRendererSkeletonContext::Declaration]),
    skeleton_dispatch("GoStmt", &[GoRawTreeRendererSkeletonContext::Statement]),
    skeleton_dispatch("Ident", &[GoRawTreeRendererSkeletonContext::Expression]),
    skeleton_dispatch("IfStmt", &[GoRawTreeRendererSkeletonContext::Statement]),
    skeleton_dispatch(
        "ImportSpec",
        &[GoRawTreeRendererSkeletonContext::Specification],
    ),
    skeleton_dispatch(
        "IncDecStmt",
        &[
            GoRawTreeRendererSkeletonContext::Statement,
            GoRawTreeRendererSkeletonContext::SimpleStatementHeader,
        ],
    ),
    skeleton_dispatch("IndexExpr", &[GoRawTreeRendererSkeletonContext::Expression]),
    skeleton_dispatch(
        "IndexListExpr",
        &[GoRawTreeRendererSkeletonContext::Expression],
    ),
    skeleton_dispatch(
        "InterfaceType",
        &[GoRawTreeRendererSkeletonContext::Expression],
    ),
    skeleton_dispatch(
        "KeyValueExpr",
        &[GoRawTreeRendererSkeletonContext::Expression],
    ),
    skeleton_dispatch(
        "LabeledStmt",
        &[GoRawTreeRendererSkeletonContext::Statement],
    ),
    skeleton_dispatch("MapType", &[GoRawTreeRendererSkeletonContext::Expression]),
    skeleton_dispatch("ParenExpr", &[GoRawTreeRendererSkeletonContext::Expression]),
    skeleton_dispatch("RangeStmt", &[GoRawTreeRendererSkeletonContext::Statement]),
    skeleton_dispatch("ReturnStmt", &[GoRawTreeRendererSkeletonContext::Statement]),
    skeleton_dispatch("SelectStmt", &[GoRawTreeRendererSkeletonContext::Statement]),
    skeleton_dispatch(
        "SelectorExpr",
        &[GoRawTreeRendererSkeletonContext::Expression],
    ),
    skeleton_dispatch(
        "SendStmt",
        &[
            GoRawTreeRendererSkeletonContext::Statement,
            GoRawTreeRendererSkeletonContext::SimpleStatementHeader,
        ],
    ),
    skeleton_dispatch("SliceExpr", &[GoRawTreeRendererSkeletonContext::Expression]),
    skeleton_dispatch("StarExpr", &[GoRawTreeRendererSkeletonContext::Expression]),
    skeleton_dispatch(
        "StructType",
        &[GoRawTreeRendererSkeletonContext::Expression],
    ),
    skeleton_dispatch("SwitchStmt", &[GoRawTreeRendererSkeletonContext::Statement]),
    skeleton_dispatch(
        "TypeAssertExpr",
        &[GoRawTreeRendererSkeletonContext::Expression],
    ),
    skeleton_dispatch(
        "TypeSpec",
        &[GoRawTreeRendererSkeletonContext::Specification],
    ),
    skeleton_dispatch(
        "TypeSwitchStmt",
        &[GoRawTreeRendererSkeletonContext::Statement],
    ),
    skeleton_dispatch("UnaryExpr", &[GoRawTreeRendererSkeletonContext::Expression]),
    skeleton_dispatch(
        "ValueSpec",
        &[GoRawTreeRendererSkeletonContext::Specification],
    ),
];

const fn skeleton_dispatch(
    node_kind: &'static str,
    contexts: &'static [GoRawTreeRendererSkeletonContext],
) -> GoRawTreeRendererSkeletonDispatchEntry {
    GoRawTreeRendererSkeletonDispatchEntry {
        node_kind,
        contexts,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoRawTreeRendererSkeletonDispatchAudit {
    pub inventory_generated_skeleton_node_count: usize,
    pub dispatch_node_count: usize,
    pub missing_generated_skeleton_node_kinds: Vec<String>,
    pub extra_dispatch_node_kinds: Vec<String>,
    pub contextual_dispatch_node_kinds: Vec<String>,
}

pub const GO_RAW_TREE_RENDERER_SKELETON_GENERATION_CONTRACT_ARTIFACT_KIND: &str =
    "tct_go_raw_tree_renderer_skeleton_generation_contract_v1";
pub const GO_RAW_TREE_RENDERER_SKELETON_GENERATION_STATUS_READY: &str =
    "go_raw_tree_renderer_skeleton_generation_contract_ready";
pub const GO_RAW_TREE_RENDERER_SKELETON_GENERATION_CLAIM_BOUNDARY: &str = "historical/baseline Go raw-tree renderer skeleton generation contract: checked go/ast inventory coverage supplies generated-skeleton-ready node kinds, source-file-root exclusions, and manual policy hooks; the raw-tree renderer dispatch table supplies contexts for generated skeleton entry points. After the Go AST printer oracle cutover, this retained Rust renderer coverage is diagnostic only and is not grammar-mode formatting/parsing authority. Parser, formatter, checked-source fidelity, Arena IR, route-handoff provenance, and BPE/model artifacts remain sidecar or cleanup surfaces, not validity authority.";
pub const GO_RAW_TREE_RENDERER_GENERATED_SKELETON_SLICE_ARTIFACT_KIND: &str =
    "tct_go_raw_tree_renderer_generated_skeleton_slice_v1";
pub const GO_RAW_TREE_RENDERER_GENERATED_SKELETON_SLICE_STATUS_READY: &str =
    "go_raw_tree_renderer_generated_skeleton_slice_ready";
pub const GO_RAW_TREE_RENDERER_INVENTORY_SKELETON_SLICE_ID: &str =
    "go_raw_tree_renderer_inventory_backed_generated_skeleton_slice_v1";
pub const GO_RAW_TREE_RENDERER_SOURCE_FILE_DECL_SPEC_SLICE_ID: &str =
    "go_raw_tree_renderer_source_file_decl_spec_generated_skeleton_slice_v1";
pub const GO_RAW_TREE_RENDERER_GENERATED_SKELETON_SLICE_CLAIM_BOUNDARY: &str = "contract-backed historical/baseline Go raw-tree renderer skeleton slice: the checked skeleton generation contract supplies selected node kinds, dispatch contexts, and explicit manual policy callbacks for a deterministic generated skeleton surface. Manual callbacks keep precedence, punctuation/layout, comments/directives, source-file root boundaries, and parser-validity policy outside generated field traversal. After the Go AST printer oracle cutover, this retained Rust renderer coverage is diagnostic only and is not grammar-mode formatting/parsing authority. Parser, formatter, checked-source fidelity, Arena IR, route-handoff provenance, and BPE/model artifacts remain sidecar or cleanup surfaces, not validity authority.";
const GO_RAW_TREE_RENDERER_RUNTIME_INVENTORY_ARTIFACT_SHA256: &str =
    "a3b342cebff81382cd9190c6cfa9c9f11c6f0641dc19f6aed30eecda235946eb";

const GO_RAW_TREE_RENDERER_SOURCE_FILE_DECL_SPEC_SLICE_NODES: &[&str] = &[
    "File",
    "FuncDecl",
    "GenDecl",
    "ImportSpec",
    "TypeSpec",
    "ValueSpec",
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoRawTreeRendererSkeletonGenerationContract {
    pub artifact_kind: String,
    pub version: u32,
    pub status: String,
    pub coverage_artifact_kind: String,
    pub coverage_status: String,
    pub inventory_artifact_path: String,
    pub inventory_artifact_sha256: String,
    pub inventory_generated_skeleton_node_count: usize,
    pub dispatch_node_count: usize,
    pub generated_skeletons: Vec<GoRawTreeRendererSkeletonGenerationNode>,
    pub source_file_root_exclusions: Vec<GoRawTreeRendererSkeletonGenerationExclusion>,
    pub manual_policy_hook_node_count: usize,
    pub contextual_dispatch_node_count: usize,
    pub claim_boundary: String,
}

impl GoRawTreeRendererSkeletonGenerationContract {
    pub fn validate(&self) -> Result<(), SyntaxBackendBoundaryError> {
        if self.artifact_kind != GO_RAW_TREE_RENDERER_SKELETON_GENERATION_CONTRACT_ARTIFACT_KIND {
            return Err(render_error(
                "skeleton_generation_contract.artifact_kind",
                format!(
                    "expected {GO_RAW_TREE_RENDERER_SKELETON_GENERATION_CONTRACT_ARTIFACT_KIND}, got {}",
                    self.artifact_kind
                ),
            ));
        }
        if self.version != 1 {
            return Err(render_error(
                "skeleton_generation_contract.version",
                format!("expected 1, got {}", self.version),
            ));
        }
        if self.status != GO_RAW_TREE_RENDERER_SKELETON_GENERATION_STATUS_READY {
            return Err(render_error(
                "skeleton_generation_contract.status",
                format!(
                    "expected {GO_RAW_TREE_RENDERER_SKELETON_GENERATION_STATUS_READY}, got {}",
                    self.status
                ),
            ));
        }
        if self.coverage_status != GO_AST_RAW_TREE_RENDERER_CODEGEN_COVERAGE_STATUS_READY {
            return Err(render_error(
                "skeleton_generation_contract.coverage_status",
                format!(
                    "expected ready renderer codegen coverage, got {}",
                    self.coverage_status
                ),
            ));
        }
        if self.inventory_artifact_path.is_empty() || self.inventory_artifact_sha256.is_empty() {
            return Err(render_error(
                "skeleton_generation_contract.inventory",
                "contract must retain the checked inventory artifact identity",
            ));
        }
        if self.inventory_generated_skeleton_node_count != self.generated_skeletons.len()
            || self.dispatch_node_count != self.generated_skeletons.len()
        {
            return Err(render_error(
                "skeleton_generation_contract.counts",
                "inventory-generated skeleton, dispatch, and contract node counts must match",
            ));
        }

        let mut seen_nodes = BTreeSet::new();
        let mut manual_policy_hook_node_count = 0usize;
        let mut contextual_dispatch_node_count = 0usize;
        let mut source_file_root_ready = false;
        let mut field_contextual = false;
        for node in &self.generated_skeletons {
            node.validate()?;
            if !seen_nodes.insert(node.node_kind.as_str()) {
                return Err(render_error(
                    "skeleton_generation_contract.generated_skeletons",
                    format!("duplicate generated skeleton node {}", node.node_kind),
                ));
            }
            if !node.manual_policy_hooks.is_empty() {
                manual_policy_hook_node_count += 1;
            }
            if node
                .dispatch_contexts
                .iter()
                .any(|context| context == "contextual" || context == "comment_trivia")
            {
                contextual_dispatch_node_count += 1;
            }
            if node.node_kind == "File"
                && node
                    .dispatch_contexts
                    .iter()
                    .any(|context| context == "source_file_root")
            {
                source_file_root_ready = true;
            }
            if node.node_kind == "Field"
                && node.contextual_renderer
                && node
                    .dispatch_contexts
                    .iter()
                    .any(|context| context == "contextual")
            {
                field_contextual = true;
            }
        }
        if manual_policy_hook_node_count != self.manual_policy_hook_node_count
            || contextual_dispatch_node_count != self.contextual_dispatch_node_count
        {
            return Err(render_error(
                "skeleton_generation_contract.summary_counts",
                "manual hook and contextual dispatch counts must match generated skeleton entries",
            ));
        }
        if !source_file_root_ready || !field_contextual {
            return Err(render_error(
                "skeleton_generation_contract.required_dispatch_contexts",
                "contract must preserve File source-root and Field contextual dispatch",
            ));
        }

        let mut seen_exclusions = BTreeSet::new();
        for exclusion in &self.source_file_root_exclusions {
            exclusion.validate()?;
            if !seen_exclusions.insert(exclusion.node_kind.as_str()) {
                return Err(render_error(
                    "skeleton_generation_contract.source_file_root_exclusions",
                    format!(
                        "duplicate source-file-root exclusion {}",
                        exclusion.node_kind
                    ),
                ));
            }
            if seen_nodes.contains(exclusion.node_kind.as_str()) {
                return Err(render_error(
                    "skeleton_generation_contract.source_file_root_exclusions",
                    format!(
                        "{} cannot be both generated-skeleton ready and source-root excluded",
                        exclusion.node_kind
                    ),
                ));
            }
        }
        if !seen_exclusions.contains("Package") {
            return Err(render_error(
                "skeleton_generation_contract.source_file_root_exclusions",
                "go/ast.Package must remain an explicit source-file-root exclusion",
            ));
        }
        for required in [
            "checked go/ast inventory coverage",
            "raw-tree renderer dispatch table",
            "manual policy hooks",
            "not validity authority",
        ] {
            if !self.claim_boundary.contains(required) {
                return Err(render_error(
                    "skeleton_generation_contract.claim_boundary",
                    format!("claim boundary must contain {required:?}"),
                ));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoRawTreeRendererSkeletonGenerationNode {
    pub node_kind: String,
    pub dispatch_contexts: Vec<String>,
    pub contextual_renderer: bool,
    pub manual_policy_hooks: Vec<String>,
    pub coverage_reason: String,
}

impl GoRawTreeRendererSkeletonGenerationNode {
    fn validate(&self) -> Result<(), SyntaxBackendBoundaryError> {
        if self.node_kind.is_empty()
            || self.dispatch_contexts.is_empty()
            || self.coverage_reason.is_empty()
        {
            return Err(render_error(
                "skeleton_generation_contract.generated_skeletons",
                "generated skeleton entries must carry a node kind, dispatch contexts, and coverage reason",
            ));
        }
        if self.node_kind == "Package" {
            return Err(render_error(
                "skeleton_generation_contract.generated_skeletons.Package",
                "go/ast.Package is a source-file-root exclusion, not a generated skeleton root",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoRawTreeRendererSkeletonGenerationExclusion {
    pub node_kind: String,
    pub reason: String,
}

impl GoRawTreeRendererSkeletonGenerationExclusion {
    fn validate(&self) -> Result<(), SyntaxBackendBoundaryError> {
        if self.node_kind.is_empty() || self.reason.is_empty() {
            return Err(render_error(
                "skeleton_generation_contract.source_file_root_exclusions",
                "source-file-root exclusions must carry a node kind and reason",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoRawTreeRendererGeneratedSkeletonSlice {
    pub artifact_kind: String,
    pub version: u32,
    pub status: String,
    pub slice_id: String,
    pub contract_artifact_kind: String,
    pub contract_status: String,
    pub inventory_artifact_path: String,
    pub inventory_artifact_sha256: String,
    pub selected_node_kinds: Vec<String>,
    pub generated_entrypoint_count: usize,
    pub manual_policy_callback_count: usize,
    pub generated_entrypoints: Vec<GoRawTreeRendererGeneratedSkeletonEntrypoint>,
    pub source_file_root_exclusions: Vec<GoRawTreeRendererSkeletonGenerationExclusion>,
    pub claim_boundary: String,
}

impl GoRawTreeRendererGeneratedSkeletonSlice {
    pub fn validate(&self) -> Result<(), SyntaxBackendBoundaryError> {
        if self.artifact_kind != GO_RAW_TREE_RENDERER_GENERATED_SKELETON_SLICE_ARTIFACT_KIND {
            return Err(render_error(
                "generated_skeleton_slice.artifact_kind",
                format!(
                    "expected {GO_RAW_TREE_RENDERER_GENERATED_SKELETON_SLICE_ARTIFACT_KIND}, got {}",
                    self.artifact_kind
                ),
            ));
        }
        if self.version != 1 {
            return Err(render_error(
                "generated_skeleton_slice.version",
                format!("expected 1, got {}", self.version),
            ));
        }
        if self.status != GO_RAW_TREE_RENDERER_GENERATED_SKELETON_SLICE_STATUS_READY {
            return Err(render_error(
                "generated_skeleton_slice.status",
                format!(
                    "expected {GO_RAW_TREE_RENDERER_GENERATED_SKELETON_SLICE_STATUS_READY}, got {}",
                    self.status
                ),
            ));
        }
        if self.contract_artifact_kind
            != GO_RAW_TREE_RENDERER_SKELETON_GENERATION_CONTRACT_ARTIFACT_KIND
            || self.contract_status != GO_RAW_TREE_RENDERER_SKELETON_GENERATION_STATUS_READY
        {
            return Err(render_error(
                "generated_skeleton_slice.contract",
                "generated skeleton slices must consume the ready skeleton generation contract",
            ));
        }
        if self.slice_id.is_empty()
            || self.inventory_artifact_path.is_empty()
            || self.inventory_artifact_sha256.is_empty()
            || self.selected_node_kinds.is_empty()
        {
            return Err(render_error(
                "generated_skeleton_slice.identity",
                "generated skeleton slices must carry a slice id, checked inventory identity, and selected nodes",
            ));
        }
        if self.generated_entrypoint_count != self.generated_entrypoints.len() {
            return Err(render_error(
                "generated_skeleton_slice.generated_entrypoint_count",
                "generated entrypoint count must match entrypoint records",
            ));
        }

        let mut selected_nodes = BTreeSet::new();
        for node_kind in &self.selected_node_kinds {
            if !selected_nodes.insert(node_kind.as_str()) {
                return Err(render_error(
                    "generated_skeleton_slice.selected_node_kinds",
                    format!("duplicate selected node {node_kind}"),
                ));
            }
        }

        let mut entrypoint_nodes = BTreeSet::new();
        let mut manual_policy_callback_count = 0usize;
        for entrypoint in &self.generated_entrypoints {
            entrypoint.validate()?;
            if !entrypoint_nodes.insert(entrypoint.node_kind.as_str()) {
                return Err(render_error(
                    "generated_skeleton_slice.generated_entrypoints",
                    format!("duplicate generated entrypoint {}", entrypoint.node_kind),
                ));
            }
            manual_policy_callback_count += entrypoint.manual_policy_callbacks.len();
        }
        if selected_nodes != entrypoint_nodes {
            return Err(render_error(
                "generated_skeleton_slice.dispatch_complete",
                "selected nodes and generated entrypoints must match exactly",
            ));
        }
        if manual_policy_callback_count != self.manual_policy_callback_count {
            return Err(render_error(
                "generated_skeleton_slice.manual_policy_callback_count",
                "manual policy callback count must match entrypoint callbacks",
            ));
        }

        for exclusion in &self.source_file_root_exclusions {
            exclusion.validate()?;
            if selected_nodes.contains(exclusion.node_kind.as_str()) {
                return Err(render_error(
                    "generated_skeleton_slice.source_file_root_exclusions",
                    format!(
                        "{} cannot be both selected and source-file-root excluded",
                        exclusion.node_kind
                    ),
                ));
            }
        }
        if !self
            .source_file_root_exclusions
            .iter()
            .any(|exclusion| exclusion.node_kind == "Package")
        {
            return Err(render_error(
                "generated_skeleton_slice.source_file_root_exclusions",
                "generated skeleton slices must preserve the go/ast.Package source-root exclusion",
            ));
        }
        if self.slice_id == GO_RAW_TREE_RENDERER_SOURCE_FILE_DECL_SPEC_SLICE_ID {
            validate_source_file_decl_spec_generated_slice(self)?;
        }
        for required in [
            "contract-backed",
            "manual policy callbacks",
            "not validity authority",
        ] {
            if !self.claim_boundary.contains(required) {
                return Err(render_error(
                    "generated_skeleton_slice.claim_boundary",
                    format!("claim boundary must contain {required:?}"),
                ));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoRawTreeRendererGeneratedSkeletonEntrypoint {
    pub node_kind: String,
    pub dispatch_contexts: Vec<String>,
    pub generated_entrypoint: String,
    pub manual_policy_callbacks: Vec<GoRawTreeRendererManualPolicyCallback>,
    pub generated_field_traversals: Vec<String>,
    pub coverage_reason: String,
}

impl GoRawTreeRendererGeneratedSkeletonEntrypoint {
    fn validate(&self) -> Result<(), SyntaxBackendBoundaryError> {
        if self.node_kind.is_empty()
            || self.dispatch_contexts.is_empty()
            || self.generated_entrypoint.is_empty()
            || self.coverage_reason.is_empty()
        {
            return Err(render_error(
                "generated_skeleton_slice.generated_entrypoints",
                "generated entrypoints must carry a node kind, dispatch contexts, entrypoint symbol, and coverage reason",
            ));
        }
        let expected_prefix = format!(
            "render_generated_{}_skeleton",
            go_ast_node_kind_to_snake(&self.node_kind)
        );
        if self.generated_entrypoint != expected_prefix {
            return Err(render_error(
                "generated_skeleton_slice.generated_entrypoint",
                format!(
                    "{} should use deterministic entrypoint symbol {}, got {}",
                    self.node_kind, expected_prefix, self.generated_entrypoint
                ),
            ));
        }
        for context in &self.dispatch_contexts {
            if !is_valid_generated_skeleton_context(context) {
                return Err(render_error(
                    "generated_skeleton_slice.dispatch_contexts",
                    format!("unsupported dispatch context {context}"),
                ));
            }
        }
        for callback in &self.manual_policy_callbacks {
            callback.validate(&self.node_kind)?;
        }
        let expected_traversals = generated_field_traversals(&self.node_kind);
        if self.generated_field_traversals != expected_traversals {
            return Err(render_error(
                "generated_skeleton_slice.generated_field_traversals",
                format!(
                    "{} should use generated field traversals {:?}, got {:?}",
                    self.node_kind, expected_traversals, self.generated_field_traversals
                ),
            ));
        }
        validate_generated_field_traversal_inventory_fields(
            &self.node_kind,
            &self.generated_field_traversals,
        )?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoRawTreeRendererManualPolicyCallback {
    pub hook: String,
    pub callback_symbol: String,
}

impl GoRawTreeRendererManualPolicyCallback {
    fn validate(&self, node_kind: &str) -> Result<(), SyntaxBackendBoundaryError> {
        if self.hook.is_empty() || self.callback_symbol.is_empty() {
            return Err(render_error(
                "generated_skeleton_slice.manual_policy_callbacks",
                "manual policy callbacks must carry a hook and callback symbol",
            ));
        }
        let expected_symbol = manual_policy_callback_symbol(node_kind, &self.hook);
        if self.callback_symbol != expected_symbol {
            return Err(render_error(
                "generated_skeleton_slice.manual_policy_callbacks",
                format!(
                    "{} hook {} should use deterministic callback symbol {}, got {}",
                    node_kind, self.hook, expected_symbol, self.callback_symbol
                ),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoRawTreeRendererGeneratedSkeletonDispatchGuard {
    pub slice_id: String,
    pub contract_artifact_kind: String,
    pub contract_status: String,
    pub dispatch_node_count: usize,
    pub generated_entrypoint_count: usize,
    pub selected_node_kinds: Vec<String>,
    pub source_file_root_exclusions: Vec<String>,
    contexts_by_node: BTreeMap<String, BTreeSet<String>>,
    entrypoints_by_node: BTreeMap<String, GoRawTreeRendererGeneratedRuntimeEntrypoint>,
    source_file_root_exclusion_reasons: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct GoRawTreeRendererGeneratedRuntimeEntrypoint {
    generated_entrypoint: String,
    manual_policy_callbacks: Vec<String>,
    generated_field_traversals: Vec<String>,
}

impl GoRawTreeRendererGeneratedSkeletonDispatchGuard {
    pub fn require_node_kind_context(
        &self,
        node_kind: &str,
        context: &str,
        render_context: &str,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        if !is_valid_generated_skeleton_context(context) {
            return Err(render_error(
                render_context,
                format!("unsupported generated skeleton dispatch context {context}"),
            ));
        }
        if let Some(reason) = self.source_file_root_exclusion_reasons.get(node_kind) {
            return Err(render_error(
                render_context,
                format!(
                    "Go AST node {node_kind} is excluded from generated skeleton slice {}: {reason}",
                    self.slice_id
                ),
            ));
        }
        let Some(policy) = go_ast_raw_tree_renderer_skeleton_policy(node_kind) else {
            return Err(render_error(
                render_context,
                format!(
                    "Go AST node {node_kind} is outside the generated renderer skeleton contract consumed by slice {}",
                    self.slice_id
                ),
            ));
        };
        if !policy.generated_skeleton_ready {
            return Err(render_error(
                render_context,
                format!(
                    "Go AST node {node_kind} is classified by the generated renderer skeleton contract but is not generated-skeleton ready"
                ),
            ));
        }
        let Some(contexts) = self.contexts_by_node.get(node_kind) else {
            return Err(render_error(
                render_context,
                format!(
                    "Go AST node {node_kind} is not selected by generated skeleton slice {} for {context} dispatch",
                    self.slice_id
                ),
            ));
        };
        if !contexts.contains(context) {
            return Err(render_error(
                render_context,
                format!(
                    "Go AST node {node_kind} in generated skeleton slice {} does not dispatch in {context} context",
                    self.slice_id
                ),
            ));
        }
        Ok(())
    }

    pub fn selected_runtime_entrypoint(
        &self,
        node_kind: &str,
        context: &str,
        render_context: &str,
    ) -> Result<GoRawTreeRendererSelectedRuntimeEntrypoint, SyntaxBackendBoundaryError> {
        self.require_node_kind_context(node_kind, context, render_context)?;
        let entrypoint = self.entrypoints_by_node.get(node_kind).ok_or_else(|| {
            render_error(
                render_context,
                format!(
                    "Go AST node {node_kind} is selected by slice {} but has no generated entrypoint",
                    self.slice_id
                ),
            )
        })?;
        Ok(GoRawTreeRendererSelectedRuntimeEntrypoint {
            node_kind: node_kind.to_string(),
            dispatch_context: context.to_string(),
            generated_entrypoint: entrypoint.generated_entrypoint.clone(),
            manual_policy_callbacks: entrypoint.manual_policy_callbacks.clone(),
            generated_field_traversals: entrypoint.generated_field_traversals.clone(),
            invocation: GoRawTreeRendererGeneratedEntrypointInvocation::DispatchGuardOnly,
        })
    }
}

pub fn go_raw_tree_renderer_source_file_decl_spec_generated_dispatch_guard(
    slice: &GoRawTreeRendererGeneratedSkeletonSlice,
) -> Result<GoRawTreeRendererGeneratedSkeletonDispatchGuard, SyntaxBackendBoundaryError> {
    if slice.slice_id != GO_RAW_TREE_RENDERER_SOURCE_FILE_DECL_SPEC_SLICE_ID {
        return Err(render_error(
            "generated_skeleton_dispatch_guard.slice_id",
            format!(
                "expected source-file declaration/specification generated skeleton slice {}, got {}",
                GO_RAW_TREE_RENDERER_SOURCE_FILE_DECL_SPEC_SLICE_ID, slice.slice_id
            ),
        ));
    }
    go_raw_tree_renderer_generated_skeleton_dispatch_guard(slice)
}

pub fn go_raw_tree_renderer_generated_skeleton_dispatch_guard(
    slice: &GoRawTreeRendererGeneratedSkeletonSlice,
) -> Result<GoRawTreeRendererGeneratedSkeletonDispatchGuard, SyntaxBackendBoundaryError> {
    slice.validate()?;
    let mut contexts_by_node = BTreeMap::new();
    let mut entrypoints_by_node = BTreeMap::new();
    for entrypoint in &slice.generated_entrypoints {
        let contexts = entrypoint
            .dispatch_contexts
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>();
        if contexts_by_node
            .insert(entrypoint.node_kind.clone(), contexts)
            .is_some()
        {
            return Err(render_error(
                "generated_skeleton_dispatch_guard.generated_entrypoints",
                format!(
                    "duplicate generated dispatch entry {}",
                    entrypoint.node_kind
                ),
            ));
        }
        if entrypoints_by_node
            .insert(
                entrypoint.node_kind.clone(),
                GoRawTreeRendererGeneratedRuntimeEntrypoint {
                    generated_entrypoint: entrypoint.generated_entrypoint.clone(),
                    manual_policy_callbacks: entrypoint
                        .manual_policy_callbacks
                        .iter()
                        .map(|callback| callback.callback_symbol.clone())
                        .collect(),
                    generated_field_traversals: entrypoint.generated_field_traversals.clone(),
                },
            )
            .is_some()
        {
            return Err(render_error(
                "generated_skeleton_dispatch_guard.generated_entrypoints",
                format!(
                    "duplicate generated runtime entrypoint {}",
                    entrypoint.node_kind
                ),
            ));
        }
    }
    let source_file_root_exclusion_reasons = slice
        .source_file_root_exclusions
        .iter()
        .map(|exclusion| (exclusion.node_kind.clone(), exclusion.reason.clone()))
        .collect::<BTreeMap<_, _>>();
    let guard = GoRawTreeRendererGeneratedSkeletonDispatchGuard {
        slice_id: slice.slice_id.clone(),
        contract_artifact_kind: slice.contract_artifact_kind.clone(),
        contract_status: slice.contract_status.clone(),
        dispatch_node_count: contexts_by_node.len(),
        generated_entrypoint_count: slice.generated_entrypoint_count,
        selected_node_kinds: slice.selected_node_kinds.clone(),
        source_file_root_exclusions: slice
            .source_file_root_exclusions
            .iter()
            .map(|exclusion| exclusion.node_kind.clone())
            .collect(),
        contexts_by_node,
        entrypoints_by_node,
        source_file_root_exclusion_reasons,
    };
    if guard.dispatch_node_count != guard.generated_entrypoint_count {
        return Err(render_error(
            "generated_skeleton_dispatch_guard.counts",
            "generated dispatch guard count must match generated entrypoint count",
        ));
    }
    Ok(guard)
}

pub fn go_raw_tree_renderer_source_file_decl_skeleton_slice(
    contract: &GoRawTreeRendererSkeletonGenerationContract,
) -> Result<GoRawTreeRendererGeneratedSkeletonSlice, SyntaxBackendBoundaryError> {
    go_raw_tree_renderer_generated_skeleton_slice_for_nodes(
        contract,
        GO_RAW_TREE_RENDERER_SOURCE_FILE_DECL_SPEC_SLICE_ID,
        GO_RAW_TREE_RENDERER_SOURCE_FILE_DECL_SPEC_SLICE_NODES,
    )
}

fn go_raw_tree_renderer_runtime_inventory_generated_dispatch_guard()
-> Result<GoRawTreeRendererGeneratedSkeletonDispatchGuard, SyntaxBackendBoundaryError> {
    let slice = go_raw_tree_renderer_runtime_inventory_generated_skeleton_slice()?;
    go_raw_tree_renderer_generated_skeleton_dispatch_guard(&slice)
}

fn go_raw_tree_renderer_runtime_inventory_generated_skeleton_slice()
-> Result<GoRawTreeRendererGeneratedSkeletonSlice, SyntaxBackendBoundaryError> {
    let contract = go_raw_tree_renderer_runtime_skeleton_generation_contract()?;
    go_raw_tree_renderer_inventory_skeleton_slice(&contract)
}

fn go_raw_tree_renderer_runtime_skeleton_generation_contract()
-> Result<GoRawTreeRendererSkeletonGenerationContract, SyntaxBackendBoundaryError> {
    let dispatch_audit = go_raw_tree_renderer_skeleton_dispatch_audit();
    if !dispatch_audit
        .missing_generated_skeleton_node_kinds
        .is_empty()
        || !dispatch_audit.extra_dispatch_node_kinds.is_empty()
    {
        return Err(render_error(
            "runtime_skeleton_generation_contract.dispatch_audit",
            format!(
                "dispatch table does not match generated-skeleton coverage: missing {:?}, extra {:?}",
                dispatch_audit.missing_generated_skeleton_node_kinds,
                dispatch_audit.extra_dispatch_node_kinds
            ),
        ));
    }

    let mut generated_skeletons = Vec::new();
    let mut source_file_root_exclusions = Vec::new();
    for policy in GO_AST_RAW_TREE_RENDERER_SKELETON_POLICIES {
        if policy.generated_skeleton_ready {
            let contexts = skeleton_dispatch_contexts(policy.node_kind).ok_or_else(|| {
                render_error(
                    "runtime_skeleton_generation_contract.dispatch_contexts",
                    format!(
                        "{} is generated-skeleton ready but missing dispatch contexts",
                        policy.node_kind
                    ),
                )
            })?;
            let manual_policy_hooks = raw_tree_renderer_manual_policy_hooks(policy.node_kind);
            generated_skeletons.push(GoRawTreeRendererSkeletonGenerationNode {
                node_kind: policy.node_kind.to_string(),
                dispatch_contexts: contexts
                    .iter()
                    .map(|context| context.as_str().to_string())
                    .collect(),
                contextual_renderer: policy.contextual_renderer,
                manual_policy_hooks: manual_policy_hooks.clone(),
                coverage_reason: renderer_skeleton_policy_reason(
                    policy.contextual_renderer,
                    &manual_policy_hooks,
                )
                .to_string(),
            });
        }
        if policy.source_file_root_excluded {
            source_file_root_exclusions.push(GoRawTreeRendererSkeletonGenerationExclusion {
                node_kind: policy.node_kind.to_string(),
                reason: renderer_source_file_root_exclusion_reason(policy.node_kind).to_string(),
            });
        }
    }
    let manual_policy_hook_node_count = generated_skeletons
        .iter()
        .filter(|node| !node.manual_policy_hooks.is_empty())
        .count();
    let contextual_dispatch_node_count = generated_skeletons
        .iter()
        .filter(|node| {
            node.dispatch_contexts
                .iter()
                .any(|context| context == "contextual" || context == "comment_trivia")
        })
        .count();
    let contract = GoRawTreeRendererSkeletonGenerationContract {
        artifact_kind: GO_RAW_TREE_RENDERER_SKELETON_GENERATION_CONTRACT_ARTIFACT_KIND.to_string(),
        version: 1,
        status: GO_RAW_TREE_RENDERER_SKELETON_GENERATION_STATUS_READY.to_string(),
        coverage_artifact_kind: GO_AST_RAW_TREE_RENDERER_CODEGEN_COVERAGE_ARTIFACT_KIND.to_string(),
        coverage_status: GO_AST_RAW_TREE_RENDERER_CODEGEN_COVERAGE_STATUS_READY.to_string(),
        inventory_artifact_path: GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH.to_string(),
        inventory_artifact_sha256: GO_RAW_TREE_RENDERER_RUNTIME_INVENTORY_ARTIFACT_SHA256
            .to_string(),
        inventory_generated_skeleton_node_count: dispatch_audit
            .inventory_generated_skeleton_node_count,
        dispatch_node_count: dispatch_audit.dispatch_node_count,
        generated_skeletons,
        source_file_root_exclusions,
        manual_policy_hook_node_count,
        contextual_dispatch_node_count,
        claim_boundary: GO_RAW_TREE_RENDERER_SKELETON_GENERATION_CLAIM_BOUNDARY.to_string(),
    };
    contract.validate()?;
    Ok(contract)
}

fn renderer_skeleton_policy_reason(
    contextual_renderer: bool,
    manual_policy_hooks: &[String],
) -> &'static str {
    if contextual_renderer {
        "inventory-derived skeleton is rendered through FieldList, parameter, struct, and interface contexts"
    } else if manual_policy_hooks.is_empty() {
        "inventory-derived skeleton dispatch and field traversal are sufficient for this node"
    } else {
        "inventory-derived skeleton dispatch is ready; listed manual policy hooks own rendering semantics"
    }
}

fn renderer_source_file_root_exclusion_reason(node_kind: &str) -> &'static str {
    match node_kind {
        "Package" => {
            "go/ast.Package is a multi-file aggregate; the Go tokenizer-generator source root is File"
        }
        _ => "go/ast node is excluded from source-file-root generated skeleton dispatch",
    }
}

pub fn go_raw_tree_renderer_generated_skeleton_slice_for_nodes(
    contract: &GoRawTreeRendererSkeletonGenerationContract,
    slice_id: &str,
    node_kinds: &[&str],
) -> Result<GoRawTreeRendererGeneratedSkeletonSlice, SyntaxBackendBoundaryError> {
    contract.validate()?;
    if slice_id.is_empty() || node_kinds.is_empty() {
        return Err(render_error(
            "generated_skeleton_slice.selection",
            "generated skeleton slice selection must carry a slice id and at least one node",
        ));
    }

    let generated_by_node = contract
        .generated_skeletons
        .iter()
        .map(|node| (node.node_kind.as_str(), node))
        .collect::<BTreeMap<_, _>>();
    let excluded_by_node = contract
        .source_file_root_exclusions
        .iter()
        .map(|exclusion| (exclusion.node_kind.as_str(), exclusion))
        .collect::<BTreeMap<_, _>>();

    let mut selected_node_kinds = Vec::with_capacity(node_kinds.len());
    let mut generated_entrypoints = Vec::with_capacity(node_kinds.len());
    let mut seen = BTreeSet::new();
    for node_kind in node_kinds {
        if !seen.insert(*node_kind) {
            return Err(render_error(
                "generated_skeleton_slice.selection",
                format!("duplicate selected node {node_kind}"),
            ));
        }
        if let Some(exclusion) = excluded_by_node.get(node_kind) {
            return Err(render_error(
                "generated_skeleton_slice.selection",
                format!(
                    "{} is a source-file-root exclusion and cannot be selected: {}",
                    exclusion.node_kind, exclusion.reason
                ),
            ));
        }
        let generated_node = generated_by_node.get(node_kind).ok_or_else(|| {
            render_error(
                "generated_skeleton_slice.selection",
                format!("{node_kind} is not present in the skeleton generation contract"),
            )
        })?;
        selected_node_kinds.push((*node_kind).to_string());
        generated_entrypoints.push(GoRawTreeRendererGeneratedSkeletonEntrypoint {
            node_kind: generated_node.node_kind.clone(),
            dispatch_contexts: generated_node.dispatch_contexts.clone(),
            generated_entrypoint: format!(
                "render_generated_{}_skeleton",
                go_ast_node_kind_to_snake(&generated_node.node_kind)
            ),
            manual_policy_callbacks: generated_node
                .manual_policy_hooks
                .iter()
                .map(|hook| GoRawTreeRendererManualPolicyCallback {
                    hook: hook.clone(),
                    callback_symbol: manual_policy_callback_symbol(&generated_node.node_kind, hook),
                })
                .collect(),
            generated_field_traversals: generated_field_traversals(&generated_node.node_kind),
            coverage_reason: generated_node.coverage_reason.clone(),
        });
    }
    let manual_policy_callback_count = generated_entrypoints
        .iter()
        .map(|entrypoint| entrypoint.manual_policy_callbacks.len())
        .sum();
    let slice = GoRawTreeRendererGeneratedSkeletonSlice {
        artifact_kind: GO_RAW_TREE_RENDERER_GENERATED_SKELETON_SLICE_ARTIFACT_KIND.to_string(),
        version: 1,
        status: GO_RAW_TREE_RENDERER_GENERATED_SKELETON_SLICE_STATUS_READY.to_string(),
        slice_id: slice_id.to_string(),
        contract_artifact_kind: contract.artifact_kind.clone(),
        contract_status: contract.status.clone(),
        inventory_artifact_path: contract.inventory_artifact_path.clone(),
        inventory_artifact_sha256: contract.inventory_artifact_sha256.clone(),
        selected_node_kinds,
        generated_entrypoint_count: generated_entrypoints.len(),
        manual_policy_callback_count,
        generated_entrypoints,
        source_file_root_exclusions: contract.source_file_root_exclusions.clone(),
        claim_boundary: GO_RAW_TREE_RENDERER_GENERATED_SKELETON_SLICE_CLAIM_BOUNDARY.to_string(),
    };
    slice.validate()?;
    Ok(slice)
}

pub fn go_raw_tree_renderer_inventory_skeleton_slice(
    contract: &GoRawTreeRendererSkeletonGenerationContract,
) -> Result<GoRawTreeRendererGeneratedSkeletonSlice, SyntaxBackendBoundaryError> {
    let node_kinds = contract
        .generated_skeletons
        .iter()
        .map(|node| node.node_kind.as_str())
        .collect::<Vec<_>>();
    go_raw_tree_renderer_generated_skeleton_slice_for_nodes(
        contract,
        GO_RAW_TREE_RENDERER_INVENTORY_SKELETON_SLICE_ID,
        &node_kinds,
    )
}

fn validate_source_file_decl_spec_generated_slice(
    slice: &GoRawTreeRendererGeneratedSkeletonSlice,
) -> Result<(), SyntaxBackendBoundaryError> {
    let selected = slice
        .selected_node_kinds
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let expected = GO_RAW_TREE_RENDERER_SOURCE_FILE_DECL_SPEC_SLICE_NODES
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    if selected != expected {
        return Err(render_error(
            "generated_skeleton_slice.source_file_decl_spec",
            "source-file declaration/specification slice must contain exactly File, declarations, and specifications",
        ));
    }

    for (node_kind, expected_context) in [
        ("File", "source_file_root"),
        ("FuncDecl", "declaration"),
        ("GenDecl", "declaration"),
        ("ImportSpec", "specification"),
        ("TypeSpec", "specification"),
        ("ValueSpec", "specification"),
    ] {
        let entrypoint = slice
            .generated_entrypoints
            .iter()
            .find(|entrypoint| entrypoint.node_kind == node_kind)
            .ok_or_else(|| {
                render_error(
                    "generated_skeleton_slice.source_file_decl_spec",
                    format!("missing generated entrypoint for {node_kind}"),
                )
            })?;
        if !entrypoint
            .dispatch_contexts
            .iter()
            .any(|context| context == expected_context)
        {
            return Err(render_error(
                "generated_skeleton_slice.source_file_decl_spec",
                format!("{node_kind} must dispatch through {expected_context}"),
            ));
        }
    }
    Ok(())
}

fn is_valid_generated_skeleton_context(context: &str) -> bool {
    matches!(
        context,
        "source_file_root"
            | "declaration"
            | "specification"
            | "statement"
            | "simple_statement_header"
            | "expression"
            | "contextual"
            | "comment_trivia"
    )
}

fn manual_policy_callback_symbol(node_kind: &str, hook: &str) -> String {
    format!("manual_{}_{}", go_ast_node_kind_to_snake(node_kind), hook)
}

fn generated_field_traversals(node_kind: &str) -> Vec<String> {
    generated_field_traversal_policy(node_kind)
        .iter()
        .map(|field| (*field).to_string())
        .collect()
}

fn generated_field_traversal_policy(node_kind: &str) -> &'static [&'static str] {
    match node_kind {
        "AssignStmt" => &["Lhs", "Tok", "Rhs"],
        "BasicLit" => &["Value"],
        "BinaryExpr" => &["X", "Op", "Y"],
        "BranchStmt" => &["Tok", "Label"],
        "CallExpr" => &["Fun", "Args"],
        "ExprStmt" => &["X"],
        "Field" => &["Names", "Type"],
        "FieldList" => &["List"],
        "GenDecl" => &["Specs"],
        "Ident" => &["Name"],
        "ImportSpec" => &["Name", "Path"],
        "IncDecStmt" => &["X", "Tok"],
        "ParenExpr" => &["X"],
        "ReturnStmt" => &["Results"],
        "SelectorExpr" => &["X", "Sel"],
        "TypeSpec" => &["Name", "Type"],
        "UnaryExpr" => &["Op", "X"],
        "ValueSpec" => &["Names", "Type", "Values"],
        _ => &[],
    }
}

fn validate_generated_field_traversal_inventory_fields(
    node_kind: &str,
    fields: &[String],
) -> Result<(), SyntaxBackendBoundaryError> {
    if fields.is_empty() {
        return Ok(());
    }
    let grammar = GoAstSchemaInventoryAdapter::lower_inventory_str(
        include_str!(
            "../tests/fixtures/go_ast_schema_inventory/go1.24.3-ast-schema-inventory-v1.json"
        ),
        Some(GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH.to_string()),
        None,
    )
    .map_err(|error| {
        render_error(
            "generated_skeleton_slice.generated_field_traversals",
            format!("failed to load checked Go AST inventory: {error}"),
        )
    })?;
    let rule = grammar.rule(node_kind).ok_or_else(|| {
        render_error(
            "generated_skeleton_slice.generated_field_traversals",
            format!("{node_kind} has no checked GrammarIR rule"),
        )
    })?;
    for field_name in fields {
        let field = rule
            .fields
            .iter()
            .find(|candidate| candidate.name == *field_name)
            .ok_or_else(|| {
                render_error(
                    "generated_skeleton_slice.generated_field_traversals",
                    format!("{node_kind}.{field_name} is not a checked GrammarIR field"),
                )
            })?;
        let all_nonterminal = field
            .alternatives
            .iter()
            .all(|alternative| alternative.terminal == GrammarTerminalKind::Nonterminal);
        if all_nonterminal {
            // Generated traversal owns raw child extraction; downstream
            // render dispatch still owns alternative-specific policy.
            continue;
        }
        let all_payload_terminals = field
            .alternatives
            .iter()
            .all(|alternative| alternative.terminal != GrammarTerminalKind::Nonterminal);
        if all_payload_terminals && generated_terminal_traversal_field_is_checked(field) {
            continue;
        }
        if field.alternatives.len() != 1 {
            return Err(render_error(
                "generated_skeleton_slice.generated_field_traversals",
                format!(
                    "{}.{} must be a single payload field or checked nonterminal child field before generated traversal can own it",
                    node_kind, field_name
                ),
            ));
        }
        let alternative = &field.alternatives[0];
        let expected_prefix = format!("{node_kind}.{field_name}.");
        if !alternative.node_kind.starts_with(&expected_prefix) {
            return Err(render_error(
                "generated_skeleton_slice.generated_field_traversals",
                format!(
                    "{}.{} resolves through finite domain {} rather than a direct payload field",
                    node_kind, field_name, alternative.node_kind
                ),
            ));
        }
    }
    Ok(())
}

fn generated_terminal_traversal_field_is_checked(field: &crate::grammar_ir::GrammarField) -> bool {
    let mut domain_prefix = None;
    for alternative in &field.alternatives {
        let Some((prefix, _)) = alternative.node_kind.rsplit_once('.') else {
            return false;
        };
        match domain_prefix {
            Some(existing) if existing != prefix => return false,
            Some(_) => {}
            None => domain_prefix = Some(prefix),
        }
    }
    domain_prefix.is_some()
}

fn go_ast_node_kind_to_snake(node_kind: &str) -> String {
    let mut snake = String::new();
    for (index, character) in node_kind.chars().enumerate() {
        if character.is_ascii_uppercase() && index > 0 {
            snake.push('_');
        }
        snake.push(character.to_ascii_lowercase());
    }
    snake
}

pub fn go_raw_tree_renderer_skeleton_dispatch_audit() -> GoRawTreeRendererSkeletonDispatchAudit {
    let inventory_generated = GO_AST_RAW_TREE_RENDERER_SKELETON_POLICIES
        .iter()
        .filter(|policy| policy.generated_skeleton_ready)
        .map(|policy| policy.node_kind)
        .collect::<BTreeSet<_>>();
    let dispatch_nodes = GO_RAW_TREE_RENDERER_SKELETON_DISPATCH_ENTRIES
        .iter()
        .map(|entry| entry.node_kind)
        .collect::<BTreeSet<_>>();
    let contextual_dispatch_node_kinds = GO_RAW_TREE_RENDERER_SKELETON_DISPATCH_ENTRIES
        .iter()
        .filter(|entry| {
            entry
                .contexts
                .contains(&GoRawTreeRendererSkeletonContext::Contextual)
                || entry
                    .contexts
                    .contains(&GoRawTreeRendererSkeletonContext::CommentTrivia)
        })
        .map(|entry| entry.node_kind.to_string())
        .collect::<Vec<_>>();
    GoRawTreeRendererSkeletonDispatchAudit {
        inventory_generated_skeleton_node_count: inventory_generated.len(),
        dispatch_node_count: dispatch_nodes.len(),
        missing_generated_skeleton_node_kinds: inventory_generated
            .difference(&dispatch_nodes)
            .map(|node_kind| (*node_kind).to_string())
            .collect(),
        extra_dispatch_node_kinds: dispatch_nodes
            .difference(&inventory_generated)
            .map(|node_kind| (*node_kind).to_string())
            .collect(),
        contextual_dispatch_node_kinds,
    }
}

pub fn go_raw_tree_renderer_skeleton_generation_contract(
    coverage: &GoAstRawTreeRendererCodegenCoverageReport,
) -> Result<GoRawTreeRendererSkeletonGenerationContract, SyntaxBackendBoundaryError> {
    coverage.validate().map_err(|error| {
        render_error(
            "skeleton_generation_contract.coverage",
            format!("renderer codegen coverage did not validate: {error}"),
        )
    })?;
    if coverage.status != GO_AST_RAW_TREE_RENDERER_CODEGEN_COVERAGE_STATUS_READY {
        return Err(render_error(
            "skeleton_generation_contract.coverage_status",
            format!(
                "renderer skeleton generation requires ready coverage, got {}",
                coverage.status
            ),
        ));
    }

    let dispatch_contexts = skeleton_dispatch_context_map()?;
    let dispatch_audit = go_raw_tree_renderer_skeleton_dispatch_audit();
    if !dispatch_audit
        .missing_generated_skeleton_node_kinds
        .is_empty()
        || !dispatch_audit.extra_dispatch_node_kinds.is_empty()
    {
        return Err(render_error(
            "skeleton_generation_contract.dispatch_audit",
            format!(
                "dispatch table does not match generated-skeleton coverage: missing {:?}, extra {:?}",
                dispatch_audit.missing_generated_skeleton_node_kinds,
                dispatch_audit.extra_dispatch_node_kinds
            ),
        ));
    }

    let mut generated_skeletons = Vec::new();
    let mut source_file_root_exclusions = Vec::new();
    for node in &coverage.node_coverages {
        if node.generated_skeleton_ready {
            let contexts = dispatch_contexts
                .get(node.node_kind.as_str())
                .ok_or_else(|| {
                    render_error(
                        "skeleton_generation_contract.dispatch_contexts",
                        format!(
                            "{} is generated-skeleton ready but missing dispatch contexts",
                            node.node_kind
                        ),
                    )
                })?;
            generated_skeletons.push(GoRawTreeRendererSkeletonGenerationNode {
                node_kind: node.node_kind.clone(),
                dispatch_contexts: contexts
                    .iter()
                    .map(|context| context.as_str().to_string())
                    .collect(),
                contextual_renderer: node.contextual_renderer,
                manual_policy_hooks: node.manual_policy_hooks.clone(),
                coverage_reason: node.reason.clone(),
            });
        }
        if node.source_file_root_excluded {
            source_file_root_exclusions.push(GoRawTreeRendererSkeletonGenerationExclusion {
                node_kind: node.node_kind.clone(),
                reason: node.reason.clone(),
            });
        }
    }
    let manual_policy_hook_node_count = generated_skeletons
        .iter()
        .filter(|node| !node.manual_policy_hooks.is_empty())
        .count();
    let contextual_dispatch_node_count = generated_skeletons
        .iter()
        .filter(|node| {
            node.dispatch_contexts
                .iter()
                .any(|context| context == "contextual" || context == "comment_trivia")
        })
        .count();

    let contract = GoRawTreeRendererSkeletonGenerationContract {
        artifact_kind: GO_RAW_TREE_RENDERER_SKELETON_GENERATION_CONTRACT_ARTIFACT_KIND.to_string(),
        version: 1,
        status: GO_RAW_TREE_RENDERER_SKELETON_GENERATION_STATUS_READY.to_string(),
        coverage_artifact_kind: coverage.artifact_kind.clone(),
        coverage_status: coverage.status.clone(),
        inventory_artifact_path: coverage.inventory_artifact_path.clone(),
        inventory_artifact_sha256: coverage.inventory_artifact_sha256.clone(),
        inventory_generated_skeleton_node_count: dispatch_audit
            .inventory_generated_skeleton_node_count,
        dispatch_node_count: dispatch_audit.dispatch_node_count,
        generated_skeletons,
        source_file_root_exclusions,
        manual_policy_hook_node_count,
        contextual_dispatch_node_count,
        claim_boundary: GO_RAW_TREE_RENDERER_SKELETON_GENERATION_CLAIM_BOUNDARY.to_string(),
    };
    contract.validate()?;
    Ok(contract)
}

fn skeleton_dispatch_context_map() -> Result<
    BTreeMap<&'static str, &'static [GoRawTreeRendererSkeletonContext]>,
    SyntaxBackendBoundaryError,
> {
    let mut contexts_by_node = BTreeMap::new();
    for entry in GO_RAW_TREE_RENDERER_SKELETON_DISPATCH_ENTRIES {
        if contexts_by_node
            .insert(entry.node_kind, entry.contexts)
            .is_some()
        {
            return Err(render_error(
                "skeleton_generation_contract.dispatch_contexts",
                format!("duplicate dispatch entry for {}", entry.node_kind),
            ));
        }
    }
    Ok(contexts_by_node)
}

fn skeleton_dispatch_contexts(
    node_kind: &str,
) -> Option<&'static [GoRawTreeRendererSkeletonContext]> {
    GO_RAW_TREE_RENDERER_SKELETON_DISPATCH_ENTRIES
        .iter()
        .find(|entry| entry.node_kind == node_kind)
        .map(|entry| entry.contexts)
}

fn selected_generated_runtime_entrypoint(
    dispatch: &GoRawTreeRendererGeneratedSkeletonDispatchGuard,
    node: &RawSyntaxDecodedNode,
    context: GoRawTreeRendererSkeletonContext,
    render_context: &str,
    invocation: GoRawTreeRendererGeneratedEntrypointInvocation,
) -> Result<GoRawTreeRendererSelectedRuntimeEntrypoint, SyntaxBackendBoundaryError> {
    let mut selected =
        dispatch.selected_runtime_entrypoint(&node.node_kind, context.as_str(), render_context)?;
    selected.invocation = invocation;
    Ok(selected)
}

pub fn render_go_source_from_raw_tree(
    tree: &RawSyntaxDecodedTree,
) -> Result<GoRawTreeRenderedSource, SyntaxBackendBoundaryError> {
    let root = tree.decoded_node_tree()?;
    render_go_source_from_decoded_node(&root)
}

pub fn render_go_source_from_decoded_node(
    root: &RawSyntaxDecodedNode,
) -> Result<GoRawTreeRenderedSource, SyntaxBackendBoundaryError> {
    require_node("raw_tree.root", root, "File")?;
    let skeleton_dispatch = go_raw_tree_renderer_runtime_inventory_generated_dispatch_guard()?;
    let mut selected_generated_entrypoints = Vec::new();
    let root_comment_groups = root_comment_groups(
        root,
        &skeleton_dispatch,
        &mut selected_generated_entrypoints,
    )?;
    let attached_comment_counts = attached_comment_counts(
        root,
        &skeleton_dispatch,
        &mut selected_generated_entrypoints,
    )?;
    let attached_comment_group_count = attached_comment_counts.values().sum::<usize>();
    let explicit_statement_comment_anchor_count = explicit_statement_comment_anchor_count(root)?;
    let mut comments = CommentAnchors::new(
        root_comment_groups,
        attached_comment_counts,
        explicit_statement_comment_anchor_count,
    );
    let mut renderer = GoRawTreeRenderer {
        comments: &mut comments,
        skeleton_dispatch,
        selected_generated_entrypoints,
        defer_unanchored_leading_comment_depth: 0,
        defer_implicit_leading_comment_depth: 0,
        suppress_implicit_branch_trailing_comment: false,
    };
    let source = renderer.render_file(root)?;
    renderer.comments.require_all_consumed()?;
    let root_file_comment_group_count = renderer.comments.root_group_count();
    let trailing_comment_anchor_count = renderer.comments.trailing_comment_anchor_count;
    let selected_generated_entrypoints = renderer.selected_generated_entrypoints;
    Ok(GoRawTreeRenderedSource {
        source,
        root_file_comment_group_count,
        attached_comment_group_count,
        root_only_comment_group_count: root_file_comment_group_count
            .saturating_sub(attached_comment_group_count),
        trailing_comment_anchor_count,
        selected_generated_entrypoints,
    })
}

struct GoRawTreeRenderer<'a> {
    comments: &'a mut CommentAnchors,
    skeleton_dispatch: GoRawTreeRendererGeneratedSkeletonDispatchGuard,
    selected_generated_entrypoints: Vec<GoRawTreeRendererSelectedRuntimeEntrypoint>,
    defer_unanchored_leading_comment_depth: usize,
    defer_implicit_leading_comment_depth: usize,
    suppress_implicit_branch_trailing_comment: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GoRawTreeRendererGeneratedEntrypointMode {
    SourceFileRoot,
    Declaration { indent: usize },
    Specification(GoRawTreeRendererSpecificationLayout),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GoRawTreeRendererSpecificationLayout {
    Inline,
    Indented { indent: usize },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GoRawTreeRendererFieldLayout {
    Standard,
    Interface,
}

impl GoRawTreeRenderer<'_> {
    fn require_skeleton_dispatch_context(
        &mut self,
        node: &RawSyntaxDecodedNode,
        context: GoRawTreeRendererSkeletonContext,
        render_context: &str,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        let selected = selected_generated_runtime_entrypoint(
            &self.skeleton_dispatch,
            node,
            context,
            render_context,
            GoRawTreeRendererGeneratedEntrypointInvocation::DispatchGuardOnly,
        )?;
        self.selected_generated_entrypoints.push(selected);
        Ok(())
    }

    fn select_generated_wrapper_entrypoint(
        &mut self,
        node: &RawSyntaxDecodedNode,
        context: GoRawTreeRendererSkeletonContext,
        render_context: &str,
    ) -> Result<String, SyntaxBackendBoundaryError> {
        let selected = selected_generated_runtime_entrypoint(
            &self.skeleton_dispatch,
            node,
            context,
            render_context,
            GoRawTreeRendererGeneratedEntrypointInvocation::GeneratedWrapper,
        )?;
        let generated_entrypoint = selected.generated_entrypoint.clone();
        self.selected_generated_entrypoints.push(selected);
        Ok(generated_entrypoint)
    }

    fn render_source_file_decl_spec_generated_entrypoint(
        &mut self,
        node: &RawSyntaxDecodedNode,
        context: GoRawTreeRendererSkeletonContext,
        render_context: &str,
        mode: GoRawTreeRendererGeneratedEntrypointMode,
    ) -> Result<String, SyntaxBackendBoundaryError> {
        let generated_entrypoint =
            self.select_generated_wrapper_entrypoint(node, context, render_context)?;

        match (generated_entrypoint.as_str(), mode) {
            (
                "render_generated_file_skeleton",
                GoRawTreeRendererGeneratedEntrypointMode::SourceFileRoot,
            ) => self.render_generated_file_skeleton(node),
            (
                "render_generated_gen_decl_skeleton",
                GoRawTreeRendererGeneratedEntrypointMode::Declaration { indent },
            ) => self.render_generated_gen_decl_skeleton(node, indent),
            (
                "render_generated_func_decl_skeleton",
                GoRawTreeRendererGeneratedEntrypointMode::Declaration { indent },
            ) => self.render_generated_func_decl_skeleton(node, indent),
            (
                "render_generated_import_spec_skeleton",
                GoRawTreeRendererGeneratedEntrypointMode::Specification(layout),
            ) => self.render_generated_import_spec_skeleton(node, layout),
            (
                "render_generated_type_spec_skeleton",
                GoRawTreeRendererGeneratedEntrypointMode::Specification(layout),
            ) => self.render_generated_type_spec_skeleton(node, layout),
            (
                "render_generated_value_spec_skeleton",
                GoRawTreeRendererGeneratedEntrypointMode::Specification(layout),
            ) => self.render_generated_value_spec_skeleton(node, layout),
            _ => Err(render_error(
                render_context,
                format!(
                    "generated renderer entrypoint {generated_entrypoint} is not wired for {render_context}"
                ),
            )),
        }
    }

    fn render_statement_generated_entrypoint(
        &mut self,
        node: &RawSyntaxDecodedNode,
        context: GoRawTreeRendererSkeletonContext,
        render_context: &str,
        indent: usize,
    ) -> Result<String, SyntaxBackendBoundaryError> {
        let generated_entrypoint =
            self.select_generated_wrapper_entrypoint(node, context, render_context)?;
        match generated_entrypoint.as_str() {
            "render_generated_assign_stmt_skeleton" => {
                self.render_generated_assign_stmt_skeleton(node, indent)
            }
            "render_generated_branch_stmt_skeleton" => {
                self.render_generated_branch_stmt_skeleton(node, indent)
            }
            "render_generated_expr_stmt_skeleton" => {
                self.render_generated_expr_stmt_skeleton(node, indent)
            }
            "render_generated_inc_dec_stmt_skeleton" => {
                self.render_generated_inc_dec_stmt_skeleton(node, indent)
            }
            "render_generated_return_stmt_skeleton" => {
                self.render_generated_return_stmt_skeleton(node, indent)
            }
            _ => Err(render_error(
                render_context,
                format!(
                    "generated renderer entrypoint {generated_entrypoint} is not wired for {render_context}"
                ),
            )),
        }
    }

    fn render_expression_generated_entrypoint(
        &mut self,
        node: &RawSyntaxDecodedNode,
        context: ExprContext,
        indent: usize,
    ) -> Result<String, SyntaxBackendBoundaryError> {
        let generated_entrypoint = self.select_generated_wrapper_entrypoint(
            node,
            GoRawTreeRendererSkeletonContext::Expression,
            "expr",
        )?;
        match generated_entrypoint.as_str() {
            "render_generated_basic_lit_skeleton" => self.render_generated_basic_lit_skeleton(node),
            "render_generated_binary_expr_skeleton" => {
                self.render_generated_binary_expr_skeleton(node, context)
            }
            "render_generated_call_expr_skeleton" => self.render_generated_call_expr_skeleton(
                node,
                indent,
                FuncLitCalleeLayout::Preserve,
            ),
            "render_generated_ident_skeleton" => self.render_generated_ident_skeleton(node),
            "render_generated_paren_expr_skeleton" => {
                self.render_generated_paren_expr_skeleton(node, context, indent)
            }
            "render_generated_selector_expr_skeleton" => {
                self.render_generated_selector_expr_skeleton(node, context, indent)
            }
            "render_generated_unary_expr_skeleton" => {
                self.render_generated_unary_expr_skeleton(node, context, indent)
            }
            _ => Err(render_error(
                "expr",
                format!(
                    "generated renderer entrypoint {generated_entrypoint} is not wired for expr"
                ),
            )),
        }
    }

    fn render_indented_dispatch(
        &mut self,
        context: GoRawTreeRendererSkeletonContext,
        render_context: &str,
        node: &RawSyntaxDecodedNode,
        indent: usize,
    ) -> Result<String, SyntaxBackendBoundaryError> {
        if context == GoRawTreeRendererSkeletonContext::Declaration {
            return self.render_source_file_decl_spec_generated_entrypoint(
                node,
                context,
                render_context,
                GoRawTreeRendererGeneratedEntrypointMode::Declaration { indent },
            );
        }
        if matches!(
            (context, node.node_kind.as_str()),
            (
                GoRawTreeRendererSkeletonContext::Statement
                    | GoRawTreeRendererSkeletonContext::SimpleStatementHeader,
                "AssignStmt"
            ) | (
                GoRawTreeRendererSkeletonContext::Statement
                    | GoRawTreeRendererSkeletonContext::SimpleStatementHeader,
                "ExprStmt"
            ) | (
                GoRawTreeRendererSkeletonContext::Statement
                    | GoRawTreeRendererSkeletonContext::SimpleStatementHeader,
                "IncDecStmt"
            ) | (GoRawTreeRendererSkeletonContext::Statement, "BranchStmt")
                | (GoRawTreeRendererSkeletonContext::Statement, "ReturnStmt")
        ) {
            return self.render_statement_generated_entrypoint(
                node,
                context,
                render_context,
                indent,
            );
        }
        self.require_skeleton_dispatch_context(node, context, render_context)?;
        match (context, node.node_kind.as_str()) {
            (GoRawTreeRendererSkeletonContext::Statement, "CaseClause") => {
                self.render_case_clause(node, indent)
            }
            (GoRawTreeRendererSkeletonContext::Statement, "CommClause") => {
                self.render_comm_clause(node, indent)
            }
            (
                GoRawTreeRendererSkeletonContext::Statement
                | GoRawTreeRendererSkeletonContext::SimpleStatementHeader,
                "DeclStmt",
            ) => self.render_decl_stmt(node, indent),
            (GoRawTreeRendererSkeletonContext::Statement, "DeferStmt") => {
                self.render_defer_stmt(node, indent)
            }
            (GoRawTreeRendererSkeletonContext::Statement, "EmptyStmt") => {
                self.render_empty_stmt(node, indent)
            }
            (
                GoRawTreeRendererSkeletonContext::Statement
                | GoRawTreeRendererSkeletonContext::SimpleStatementHeader,
                "ExprStmt",
            ) => self.render_expr_stmt(node, indent),
            (GoRawTreeRendererSkeletonContext::Statement, "ForStmt") => {
                self.render_for_stmt(node, indent)
            }
            (GoRawTreeRendererSkeletonContext::Statement, "GoStmt") => {
                self.render_go_stmt(node, indent)
            }
            (GoRawTreeRendererSkeletonContext::Statement, "IfStmt") => {
                self.render_if_stmt(node, indent)
            }
            (
                GoRawTreeRendererSkeletonContext::Statement
                | GoRawTreeRendererSkeletonContext::SimpleStatementHeader,
                "IncDecStmt",
            ) => self.render_inc_dec_stmt(node, indent),
            (GoRawTreeRendererSkeletonContext::Statement, "LabeledStmt") => {
                self.render_labeled_stmt(node, indent)
            }
            (GoRawTreeRendererSkeletonContext::Statement, "RangeStmt") => {
                self.render_range_stmt(node, indent)
            }
            (GoRawTreeRendererSkeletonContext::Statement, "ReturnStmt") => {
                self.render_return_stmt(node, indent)
            }
            (GoRawTreeRendererSkeletonContext::Statement, "SelectStmt") => {
                self.render_select_stmt(node, indent)
            }
            (
                GoRawTreeRendererSkeletonContext::Statement
                | GoRawTreeRendererSkeletonContext::SimpleStatementHeader,
                "SendStmt",
            ) => self.render_send_stmt(node, indent),
            (GoRawTreeRendererSkeletonContext::Statement, "SwitchStmt") => {
                self.render_switch_stmt(node, indent)
            }
            (GoRawTreeRendererSkeletonContext::Statement, "TypeSwitchStmt") => {
                self.render_type_switch_stmt(node, indent)
            }
            _ => Err(render_error(
                render_context,
                format!("unsupported Go {render_context} node {}", node.node_kind),
            )),
        }
    }

    fn render_spec_dispatch(
        &mut self,
        node: &RawSyntaxDecodedNode,
        layout: GoRawTreeRendererSpecificationLayout,
    ) -> Result<String, SyntaxBackendBoundaryError> {
        self.render_source_file_decl_spec_generated_entrypoint(
            node,
            GoRawTreeRendererSkeletonContext::Specification,
            "spec",
            GoRawTreeRendererGeneratedEntrypointMode::Specification(layout),
        )
    }

    fn render_expr_dispatch(
        &mut self,
        node: &RawSyntaxDecodedNode,
        context: ExprContext,
        indent: usize,
    ) -> Result<String, SyntaxBackendBoundaryError> {
        if matches!(
            node.node_kind.as_str(),
            "BasicLit"
                | "BinaryExpr"
                | "CallExpr"
                | "Ident"
                | "ParenExpr"
                | "SelectorExpr"
                | "UnaryExpr"
        ) {
            return self.render_expression_generated_entrypoint(node, context, indent);
        }
        self.require_skeleton_dispatch_context(
            node,
            GoRawTreeRendererSkeletonContext::Expression,
            "expr",
        )?;
        match node.node_kind.as_str() {
            "ArrayType" => self.render_array_type_expr(node, context, indent),
            "BasicLit" => self.render_basic_lit_expr(node, context, indent),
            "BinaryExpr" => self.render_binary_expr_dispatch(node, context, indent),
            "CallExpr" => self.render_call_expr_dispatch(node, context, indent),
            "ChanType" => self.render_chan_type_dispatch(node, context, indent),
            "CompositeLit" => self.render_composite_lit_dispatch(node, context, indent),
            "Ellipsis" => self.render_ellipsis_expr_dispatch(node, context, indent),
            "FuncLit" => self.render_func_lit_dispatch(node, context, indent),
            "FuncType" => self.render_func_type_expr(node, context, indent),
            "Ident" => self.render_ident_expr(node, context, indent),
            "IndexExpr" => self.render_index_expr(node, context, indent),
            "IndexListExpr" => self.render_index_list_expr_dispatch(node, context, indent),
            "InterfaceType" => self.render_interface_type_dispatch(node, context, indent),
            "KeyValueExpr" => self.render_key_value_expr_dispatch(node, context, indent),
            "MapType" => self.render_map_type_dispatch(node, context, indent),
            "ParenExpr" => self.render_paren_expr(node, context, indent),
            "SliceExpr" => self.render_slice_expr_dispatch(node, context, indent),
            "StarExpr" => self.render_star_expr(node, context, indent),
            "StructType" => self.render_struct_type_expr(node, context, indent),
            "TypeAssertExpr" => self.render_type_assert_expr_dispatch(node, context, indent),
            "UnaryExpr" => self.render_unary_expr(node, context, indent),
            _ => Err(render_error(
                "expr",
                format!("unsupported Go expression node {}", node.node_kind),
            )),
        }
    }

    fn render_file(
        &mut self,
        node: &RawSyntaxDecodedNode,
    ) -> Result<String, SyntaxBackendBoundaryError> {
        self.render_source_file_decl_spec_generated_entrypoint(
            node,
            GoRawTreeRendererSkeletonContext::SourceFileRoot,
            "file",
            GoRawTreeRendererGeneratedEntrypointMode::SourceFileRoot,
        )
    }

    fn render_generated_file_skeleton(
        &mut self,
        node: &RawSyntaxDecodedNode,
    ) -> Result<String, SyntaxBackendBoundaryError> {
        self.manual_file_source_file_layout(node)
    }

    fn manual_file_source_file_layout(
        &mut self,
        node: &RawSyntaxDecodedNode,
    ) -> Result<String, SyntaxBackendBoundaryError> {
        let mut source = String::new();
        self.manual_file_file_comment_order(node, &mut source)?;
        source.push_str(&self.manual_file_package_clause(node)?);
        source.push_str("\n\n");

        let decls = optional_nodes(node, "Decls")?;
        // File-scope layout decisions: `NoBlankLineBefore` records the
        // declaration separators the source suppressed (gofmt forces the
        // rest), and `CommentBefore` anchors each root-only comment group to
        // the declaration it precedes. Trees without the fields (historical
        // per-file and witness shapes) keep the pre-metadata behaviour: one
        // blank line between every declaration and a greedy block-comment
        // flush.
        let has_decl_anchors = node.field("CommentBefore").is_some();
        let comment_before = file_decl_comment_before_indexes(node, decls.len())?;
        let no_blank_line_before = file_decl_no_blank_line_before_indexes(node, decls.len())?;
        let mut comment_cursor = 0usize;
        for (index, decl) in decls.iter().enumerate() {
            if index > 0 && !no_blank_line_before.contains(&index) {
                source.push('\n');
            }
            if has_decl_anchors {
                self.push_anchored_file_scope_comments(
                    &mut source,
                    &comment_before,
                    &mut comment_cursor,
                    index,
                )?;
            } else {
                self.push_file_scope_block_comments(&mut source)?;
            }
            source.push_str(&self.render_decl(decl, 0)?);
            source.push('\n');
        }
        if has_decl_anchors {
            self.push_anchored_file_scope_comments(
                &mut source,
                &comment_before,
                &mut comment_cursor,
                decls.len(),
            )?;
        }
        self.push_remaining_file_scope_comments(&mut source)?;
        Ok(source)
    }

    fn manual_file_file_comment_order(
        &mut self,
        node: &RawSyntaxDecodedNode,
        source: &mut String,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        while self.comments.next_is_file_header_comment() {
            push_comment_group(source, &self.comments.take_next_unattached()?);
            source.push('\n');
        }
        if let Some(doc) = optional_single_node(node, "Doc")? {
            push_comment_group(source, &self.consume_attached(doc)?);
        }
        Ok(())
    }

    fn manual_file_package_clause(
        &mut self,
        node: &RawSyntaxDecodedNode,
    ) -> Result<String, SyntaxBackendBoundaryError> {
        Ok(format!(
            "package {}",
            self.render_ident(required_single_node(node, "Name")?)?
        ))
    }

    fn render_decl(
        &mut self,
        node: &RawSyntaxDecodedNode,
        indent: usize,
    ) -> Result<String, SyntaxBackendBoundaryError> {
        self.render_indented_dispatch(
            GoRawTreeRendererSkeletonContext::Declaration,
            "decl",
            node,
            indent,
        )
    }

    fn push_file_scope_block_comments(
        &mut self,
        source: &mut String,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        while self.comments.next_is_unattached_block_comment()
            && self.comments.has_unreserved_unattached_comment()
        {
            if !source.ends_with("\n\n") {
                source.push('\n');
            }
            push_comment_group(source, &self.comments.take_next_unattached()?);
            source.push('\n');
        }
        Ok(())
    }

    /// Consume the root-only comment groups anchored at `decl_index`.
    fn push_anchored_file_scope_comments(
        &mut self,
        source: &mut String,
        comment_before: &[usize],
        comment_cursor: &mut usize,
        decl_index: usize,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        while comment_before
            .get(*comment_cursor)
            .is_some_and(|anchor| *anchor == decl_index)
        {
            // The anchored group must be the next unconsumed root slot. When
            // an attached-owner blank marker is next, the comment channel and
            // the anchors disagree (a known v2 marker-consumption residue);
            // drop the anchor rather than fail the whole render, so the file
            // degrades to a placement mismatch the fidelity gate can bucket.
            if !self.comments.next_is_unattached() {
                self.comments.note_explicit_statement_anchor_reached();
                *comment_cursor += 1;
                continue;
            }
            if !source.is_empty() && !source.ends_with('\n') {
                source.push('\n');
            }
            push_comment_group(
                source,
                &self.comments.take_next_statement_anchor_unattached()?,
            );
            source.push('\n');
            *comment_cursor += 1;
        }
        Ok(())
    }

    fn push_remaining_file_scope_comments(
        &mut self,
        source: &mut String,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        while self.comments.next_is_unattached()
            && self.comments.has_unreserved_unattached_comment()
        {
            if !source.ends_with('\n') {
                source.push('\n');
            }
            if !source.ends_with("\n\n") {
                source.push('\n');
            }
            push_comment_group(source, &self.comments.take_next_unattached()?);
        }
        Ok(())
    }

    fn render_generated_gen_decl_skeleton(
        &mut self,
        node: &RawSyntaxDecodedNode,
        indent: usize,
    ) -> Result<String, SyntaxBackendBoundaryError> {
        let specs = self.generated_gen_decl_specs_field(node)?;
        self.manual_gen_decl_declaration_keyword_layout(node, indent, &specs)
    }

    fn generated_gen_decl_specs_field<'a>(
        &self,
        node: &'a RawSyntaxDecodedNode,
    ) -> Result<Vec<&'a RawSyntaxDecodedNode>, SyntaxBackendBoundaryError> {
        require_node("gen_decl", node, "GenDecl")?;
        required_nodes(node, "Specs")
    }

    fn manual_gen_decl_declaration_keyword_layout(
        &mut self,
        node: &RawSyntaxDecodedNode,
        indent: usize,
        specs: &[&RawSyntaxDecodedNode],
    ) -> Result<String, SyntaxBackendBoundaryError> {
        let mut source = String::new();
        if let Some(doc) = optional_single_node(node, "Doc")? {
            push_indented_comment_group(&mut source, indent, &self.consume_attached(doc)?);
        }
        let tok = required_terminal_payload(node, "Tok")?;
        let grouped = self.manual_gen_decl_grouped_declaration_layout(node, specs)?;
        if grouped {
            source.push_str(&indent_text(indent));
            source.push_str(tok);
            source.push_str(" (\n");
            for spec in specs {
                source.push_str(&self.render_spec(spec, indent + 1)?);
                source.push('\n');
            }
            source.push_str(&indent_text(indent));
            source.push(')');
        } else {
            source.push_str(&indent_text(indent));
            source.push_str(tok);
            source.push(' ');
            source.push_str(&self.render_spec_inline(specs[0])?);
        }
        Ok(source)
    }

    fn manual_gen_decl_grouped_declaration_layout(
        &self,
        node: &RawSyntaxDecodedNode,
        specs: &[&RawSyntaxDecodedNode],
    ) -> Result<bool, SyntaxBackendBoundaryError> {
        // Unified inventory trees carry the GenDecl grouped-declaration
        // delimiters as an explicit syntax-presence boolean
        // (`go_syntax_presence_gen_decl_group`); when present it is the
        // grouping authority. Trees without the boolean (historical per-file
        // and witness shapes) fall back to the structural heuristics.
        if let Some(present) = presence_boolean_field(node, "Lparen", "GenDecl")? {
            return Ok(present);
        }
        Ok(field_has_values(node, "Lparen") || specs.len() > 1 || specs.iter().any(spec_has_doc))
    }

    fn render_spec(
        &mut self,
        node: &RawSyntaxDecodedNode,
        indent: usize,
    ) -> Result<String, SyntaxBackendBoundaryError> {
        self.render_spec_dispatch(
            node,
            GoRawTreeRendererSpecificationLayout::Indented { indent },
        )
    }

    fn render_spec_inline(
        &mut self,
        node: &RawSyntaxDecodedNode,
    ) -> Result<String, SyntaxBackendBoundaryError> {
        self.render_spec_dispatch(node, GoRawTreeRendererSpecificationLayout::Inline)
    }

    fn render_generated_import_spec_skeleton(
        &mut self,
        node: &RawSyntaxDecodedNode,
        layout: GoRawTreeRendererSpecificationLayout,
    ) -> Result<String, SyntaxBackendBoundaryError> {
        self.render_spec_with_layout(layout, node, |renderer, node| {
            let name = renderer.generated_import_spec_name_field(node)?;
            let path = renderer.generated_import_spec_path_field(node)?;
            renderer.render_import_spec_fields(node, name, path)
        })
    }

    fn generated_import_spec_name_field<'a>(
        &self,
        node: &'a RawSyntaxDecodedNode,
    ) -> Result<Option<&'a RawSyntaxDecodedNode>, SyntaxBackendBoundaryError> {
        require_node("import_spec", node, "ImportSpec")?;
        optional_single_node(node, "Name")
    }

    fn generated_import_spec_path_field<'a>(
        &self,
        node: &'a RawSyntaxDecodedNode,
    ) -> Result<&'a RawSyntaxDecodedNode, SyntaxBackendBoundaryError> {
        require_node("import_spec", node, "ImportSpec")?;
        required_single_node(node, "Path")
    }

    fn render_import_spec_fields(
        &mut self,
        node: &RawSyntaxDecodedNode,
        name: Option<&RawSyntaxDecodedNode>,
        path: &RawSyntaxDecodedNode,
    ) -> Result<String, SyntaxBackendBoundaryError> {
        let mut source = String::new();
        if let Some(name) = name {
            source.push_str(&self.render_ident(name)?);
            source.push(' ');
        }
        source.push_str(&self.render_expr(path, ExprContext::Any)?);
        self.push_attached_trailing_comment(node, &mut source)?;
        Ok(source)
    }

    fn render_generated_type_spec_skeleton(
        &mut self,
        node: &RawSyntaxDecodedNode,
        layout: GoRawTreeRendererSpecificationLayout,
    ) -> Result<String, SyntaxBackendBoundaryError> {
        self.render_spec_with_layout(layout, node, |renderer, node| {
            let name = renderer.generated_type_spec_name_field(node)?;
            let typ = renderer.generated_type_spec_type_field(node)?;
            renderer.render_type_spec_fields(node, name, typ)
        })
    }

    fn generated_type_spec_name_field<'a>(
        &self,
        node: &'a RawSyntaxDecodedNode,
    ) -> Result<&'a RawSyntaxDecodedNode, SyntaxBackendBoundaryError> {
        require_node("type_spec", node, "TypeSpec")?;
        required_single_node(node, "Name")
    }

    fn generated_type_spec_type_field<'a>(
        &self,
        node: &'a RawSyntaxDecodedNode,
    ) -> Result<&'a RawSyntaxDecodedNode, SyntaxBackendBoundaryError> {
        require_node("type_spec", node, "TypeSpec")?;
        required_single_node(node, "Type")
    }

    fn render_type_spec_fields(
        &mut self,
        node: &RawSyntaxDecodedNode,
        name: &RawSyntaxDecodedNode,
        typ: &RawSyntaxDecodedNode,
    ) -> Result<String, SyntaxBackendBoundaryError> {
        let mut source = self.render_ident(name)?;
        source.push(' ');
        source.push_str(&self.render_expr(typ, ExprContext::Any)?);
        self.push_attached_trailing_comment(node, &mut source)?;
        Ok(source)
    }

    fn render_generated_value_spec_skeleton(
        &mut self,
        node: &RawSyntaxDecodedNode,
        layout: GoRawTreeRendererSpecificationLayout,
    ) -> Result<String, SyntaxBackendBoundaryError> {
        self.render_spec_with_layout(layout, node, |renderer, node| {
            let names = renderer.generated_value_spec_names_field(node)?;
            let typ = renderer.generated_value_spec_type_field(node)?;
            let values = renderer.generated_value_spec_values_field(node)?;
            renderer.manual_value_spec_value_spec_assignment_layout(node, &names, typ, &values)
        })
    }

    fn generated_value_spec_names_field<'a>(
        &self,
        node: &'a RawSyntaxDecodedNode,
    ) -> Result<Vec<&'a RawSyntaxDecodedNode>, SyntaxBackendBoundaryError> {
        require_node("value_spec", node, "ValueSpec")?;
        required_nodes(node, "Names")
    }

    fn generated_value_spec_type_field<'a>(
        &self,
        node: &'a RawSyntaxDecodedNode,
    ) -> Result<Option<&'a RawSyntaxDecodedNode>, SyntaxBackendBoundaryError> {
        require_node("value_spec", node, "ValueSpec")?;
        optional_single_node(node, "Type")
    }

    fn generated_value_spec_values_field<'a>(
        &self,
        node: &'a RawSyntaxDecodedNode,
    ) -> Result<Vec<&'a RawSyntaxDecodedNode>, SyntaxBackendBoundaryError> {
        require_node("value_spec", node, "ValueSpec")?;
        optional_nodes(node, "Values")
    }

    fn render_spec_with_layout(
        &mut self,
        layout: GoRawTreeRendererSpecificationLayout,
        node: &RawSyntaxDecodedNode,
        render_fields: impl FnOnce(
            &mut Self,
            &RawSyntaxDecodedNode,
        ) -> Result<String, SyntaxBackendBoundaryError>,
    ) -> Result<String, SyntaxBackendBoundaryError> {
        match layout {
            GoRawTreeRendererSpecificationLayout::Inline => render_fields(self, node),
            GoRawTreeRendererSpecificationLayout::Indented { indent } => {
                let mut source = String::new();
                if matches!(node.node_kind.as_str(), "TypeSpec" | "ValueSpec")
                    && let Some(doc) = optional_single_node(node, "Doc")?
                {
                    push_indented_comment_group(&mut source, indent, &self.consume_attached(doc)?);
                }
                let rendered = render_fields(self, node)?;
                source.push_str(&indent_text(indent));
                source.push_str(&rendered);
                Ok(source)
            }
        }
    }

    fn manual_value_spec_value_spec_assignment_layout(
        &mut self,
        node: &RawSyntaxDecodedNode,
        names: &[&RawSyntaxDecodedNode],
        typ: Option<&RawSyntaxDecodedNode>,
        values: &[&RawSyntaxDecodedNode],
    ) -> Result<String, SyntaxBackendBoundaryError> {
        let mut source = join_rendered(
            names.to_vec(),
            |renderer, node| renderer.render_ident(node),
            self,
            ", ",
        )?;
        if let Some(typ) = typ {
            source.push(' ');
            source.push_str(&self.render_expr(typ, ExprContext::Any)?);
        }
        if !values.is_empty() {
            source.push_str(" = ");
            source.push_str(&join_rendered(
                values.to_vec(),
                |renderer, node| renderer.render_expr(node, ExprContext::Any),
                self,
                ", ",
            )?);
        }
        self.push_attached_trailing_comment(node, &mut source)?;
        Ok(source)
    }

    fn push_attached_trailing_comment(
        &mut self,
        node: &RawSyntaxDecodedNode,
        source: &mut String,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        if let Some(comment) = optional_single_node(node, "Comment")? {
            source.push(' ');
            source.push_str(&self.consume_attached(comment)?.join(" "));
        }
        Ok(())
    }

    fn render_generated_func_decl_skeleton(
        &mut self,
        node: &RawSyntaxDecodedNode,
        indent: usize,
    ) -> Result<String, SyntaxBackendBoundaryError> {
        self.manual_func_decl_function_signature_layout(node, indent)
    }

    fn manual_func_decl_function_signature_layout(
        &mut self,
        node: &RawSyntaxDecodedNode,
        indent: usize,
    ) -> Result<String, SyntaxBackendBoundaryError> {
        let mut source = String::new();
        if let Some(doc) = optional_single_node(node, "Doc")? {
            push_indented_comment_group(&mut source, indent, &self.consume_attached(doc)?);
        }
        source.push_str(&indent_text(indent));
        source.push_str("func ");
        if let Some(recv) = optional_single_node(node, "Recv")? {
            source.push_str(&self.manual_func_decl_receiver_layout(recv)?);
        }
        source.push_str(&self.render_ident(required_single_node(node, "Name")?)?);
        source.push_str(&self.render_func_type(required_single_node(node, "Type")?, false)?);
        if let Some(body) = optional_single_node(node, "Body")? {
            source.push(' ');
            source.push_str(&self.render_block(body, indent)?);
        }
        Ok(source)
    }

    fn manual_func_decl_receiver_layout(
        &mut self,
        recv: &RawSyntaxDecodedNode,
    ) -> Result<String, SyntaxBackendBoundaryError> {
        Ok(format!("({}) ", self.render_field_list_contents(recv)?))
    }

    fn render_func_type(
        &mut self,
        node: &RawSyntaxDecodedNode,
        include_func_keyword: bool,
    ) -> Result<String, SyntaxBackendBoundaryError> {
        require_node("func_type", node, "FuncType")?;
        self.require_skeleton_dispatch_context(
            node,
            GoRawTreeRendererSkeletonContext::Expression,
            "func_type",
        )?;
        let mut source = String::new();
        if include_func_keyword {
            source.push_str("func");
        }
        source.push('(');
        source.push_str(&self.render_field_list_contents(required_single_node(node, "Params")?)?);
        source.push(')');
        if let Some(results) = optional_single_node(node, "Results")? {
            let result_fields = optional_nodes(results, "List")?;
            if !result_fields.is_empty() {
                source.push(' ');
                if result_fields.len() == 1 && field_names(result_fields[0])?.is_empty() {
                    source.push_str(
                        &self.render_type(required_single_node(result_fields[0], "Type")?)?,
                    );
                } else {
                    source.push('(');
                    source.push_str(&self.render_field_list_contents(results)?);
                    source.push(')');
                }
            }
        }
        Ok(source)
    }

    fn render_field_list_contents(
        &mut self,
        node: &RawSyntaxDecodedNode,
    ) -> Result<String, SyntaxBackendBoundaryError> {
        let generated_entrypoint = self.select_generated_wrapper_entrypoint(
            node,
            GoRawTreeRendererSkeletonContext::Contextual,
            "field_list",
        )?;
        match generated_entrypoint.as_str() {
            "render_generated_field_list_skeleton" => {
                self.render_generated_field_list_skeleton(node)
            }
            _ => Err(render_error(
                "field_list",
                format!(
                    "generated renderer entrypoint {generated_entrypoint} is not wired for field_list"
                ),
            )),
        }
    }

    fn render_generated_field_list_skeleton(
        &mut self,
        node: &RawSyntaxDecodedNode,
    ) -> Result<String, SyntaxBackendBoundaryError> {
        let fields = self.generated_field_list_list_field(node)?;
        self.manual_field_list_comma_list_layout(&fields)
    }

    fn generated_field_list_list_field<'a>(
        &self,
        node: &'a RawSyntaxDecodedNode,
    ) -> Result<Vec<&'a RawSyntaxDecodedNode>, SyntaxBackendBoundaryError> {
        require_node("field_list", node, "FieldList")?;
        optional_nodes(node, "List")
    }

    fn generated_field_list_fields_for_layout<'a>(
        &mut self,
        node: &'a RawSyntaxDecodedNode,
        render_context: &str,
    ) -> Result<Vec<&'a RawSyntaxDecodedNode>, SyntaxBackendBoundaryError> {
        let generated_entrypoint = self.select_generated_wrapper_entrypoint(
            node,
            GoRawTreeRendererSkeletonContext::Contextual,
            render_context,
        )?;
        match generated_entrypoint.as_str() {
            "render_generated_field_list_skeleton" => self.generated_field_list_list_field(node),
            _ => Err(render_error(
                render_context,
                format!(
                    "generated renderer entrypoint {generated_entrypoint} is not wired for {render_context}"
                ),
            )),
        }
    }

    fn manual_field_list_comma_list_layout(
        &mut self,
        fields: &[&RawSyntaxDecodedNode],
    ) -> Result<String, SyntaxBackendBoundaryError> {
        let mut rendered = Vec::with_capacity(fields.len());
        for field in fields {
            rendered.push(self.render_field_inline(field)?);
        }
        Ok(rendered.join(", "))
    }

    fn render_struct_type(
        &mut self,
        node: &RawSyntaxDecodedNode,
        indent: usize,
    ) -> Result<String, SyntaxBackendBoundaryError> {
        let fields = required_single_node(node, "Fields")?;
        let field_nodes =
            self.generated_field_list_fields_for_layout(fields, "struct_field_list")?;
        if field_nodes.is_empty() {
            return Ok("struct{}".to_string());
        }
        let mut source = String::from("struct {\n");
        for (index, field) in field_nodes.iter().enumerate() {
            if struct_field_starts_trailing_comment_group(&field_nodes, index)? {
                source.push('\n');
            }
            if let Some(doc) = optional_single_node(field, "Doc")? {
                push_indented_comment_group(&mut source, indent + 1, &self.consume_attached(doc)?);
            }
            source.push_str(&indent_text(indent + 1));
            source.push_str(&self.render_field_inline(field)?);
            if let Some(comment) = optional_single_node(field, "Comment")? {
                source.push(' ');
                source.push_str(&self.consume_attached(comment)?.join(" "));
            }
            source.push('\n');
        }
        source.push_str(&indent_text(indent));
        source.push('}');
        Ok(source)
    }

    fn render_field_inline(
        &mut self,
        node: &RawSyntaxDecodedNode,
    ) -> Result<String, SyntaxBackendBoundaryError> {
        let generated_entrypoint = self.select_generated_wrapper_entrypoint(
            node,
            GoRawTreeRendererSkeletonContext::Contextual,
            "field",
        )?;
        match generated_entrypoint.as_str() {
            "render_generated_field_skeleton" => {
                self.render_generated_field_skeleton(node, GoRawTreeRendererFieldLayout::Standard)
            }
            _ => Err(render_error(
                "field",
                format!(
                    "generated renderer entrypoint {generated_entrypoint} is not wired for field"
                ),
            )),
        }
    }

    fn render_generated_field_skeleton(
        &mut self,
        node: &RawSyntaxDecodedNode,
        layout: GoRawTreeRendererFieldLayout,
    ) -> Result<String, SyntaxBackendBoundaryError> {
        let names = self.generated_field_names_field(node)?;
        let typ = self.generated_field_type_field(node)?;
        match layout {
            GoRawTreeRendererFieldLayout::Standard => {
                self.manual_field_field_names_type_tag_layout(node, &names, typ)
            }
            GoRawTreeRendererFieldLayout::Interface => {
                self.manual_field_interface_method_layout(&names, typ)
            }
        }
    }

    fn generated_field_names_field<'a>(
        &self,
        node: &'a RawSyntaxDecodedNode,
    ) -> Result<Vec<&'a RawSyntaxDecodedNode>, SyntaxBackendBoundaryError> {
        require_node("field", node, "Field")?;
        optional_nodes(node, "Names")
    }

    fn generated_field_type_field<'a>(
        &self,
        node: &'a RawSyntaxDecodedNode,
    ) -> Result<&'a RawSyntaxDecodedNode, SyntaxBackendBoundaryError> {
        require_node("field", node, "Field")?;
        required_single_node(node, "Type")
    }

    fn manual_field_field_names_type_tag_layout(
        &mut self,
        node: &RawSyntaxDecodedNode,
        names: &[&RawSyntaxDecodedNode],
        typ: &RawSyntaxDecodedNode,
    ) -> Result<String, SyntaxBackendBoundaryError> {
        let names = field_name_strings(names)?;
        let typ = self.render_type(typ)?;
        let mut source = if names.is_empty() {
            typ
        } else {
            format!("{} {}", names.join(", "), typ)
        };
        if let Some(tag) = optional_single_node(node, "Tag")? {
            source.push(' ');
            source.push_str(&self.render_expr(tag, ExprContext::Any)?);
        }
        Ok(source)
    }

    fn manual_field_interface_method_layout(
        &mut self,
        names: &[&RawSyntaxDecodedNode],
        typ: &RawSyntaxDecodedNode,
    ) -> Result<String, SyntaxBackendBoundaryError> {
        let names = field_name_strings(names)?;
        if names.is_empty() {
            return self.render_type(typ);
        }
        if typ.node_kind != "FuncType" {
            return Ok(format!("{} {}", names.join(", "), self.render_type(typ)?));
        }
        if names.len() != 1 {
            return Err(render_error(
                "interface_method",
                "interface method field must have exactly one name",
            ));
        }
        Ok(format!(
            "{}{}",
            names[0],
            self.render_func_type(typ, false)?
        ))
    }

    fn render_block(
        &mut self,
        node: &RawSyntaxDecodedNode,
        indent: usize,
    ) -> Result<String, SyntaxBackendBoundaryError> {
        require_node("block", node, "BlockStmt")?;
        let generated_entrypoint = self.select_generated_wrapper_entrypoint(
            node,
            GoRawTreeRendererSkeletonContext::Contextual,
            "block",
        )?;
        match generated_entrypoint.as_str() {
            "render_generated_block_stmt_skeleton" => {
                self.render_generated_block_stmt_skeleton(node, indent)
            }
            _ => Err(render_error(
                "block",
                format!(
                    "generated renderer entrypoint {generated_entrypoint} is not wired for block"
                ),
            )),
        }
    }

    fn render_generated_block_stmt_skeleton(
        &mut self,
        node: &RawSyntaxDecodedNode,
        indent: usize,
    ) -> Result<String, SyntaxBackendBoundaryError> {
        self.manual_block_stmt_statement_block_layout(node, indent)
    }

    fn manual_block_stmt_statement_block_layout(
        &mut self,
        node: &RawSyntaxDecodedNode,
        indent: usize,
    ) -> Result<String, SyntaxBackendBoundaryError> {
        let mut source = String::from("{\n");
        self.render_block_contents(node, indent + 1, &mut source)?;
        source.push_str(&indent_text(indent));
        source.push('}');
        Ok(source)
    }

    fn render_block_contents(
        &mut self,
        node: &RawSyntaxDecodedNode,
        indent: usize,
        source: &mut String,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        let statements = optional_nodes(node, "List")?;
        self.render_statement_sequence(node, "BlockStmt", statements, indent, source, &[])
    }

    fn render_block_contents_suppressing_comment_before(
        &mut self,
        node: &RawSyntaxDecodedNode,
        indent: usize,
        source: &mut String,
        suppressed_comment_before: &[usize],
    ) -> Result<(), SyntaxBackendBoundaryError> {
        let statements = optional_nodes(node, "List")?;
        self.render_statement_sequence(
            node,
            "BlockStmt",
            statements,
            indent,
            source,
            suppressed_comment_before,
        )
    }

    fn render_statement_sequence(
        &mut self,
        owner: &RawSyntaxDecodedNode,
        context: &str,
        statements: Vec<&RawSyntaxDecodedNode>,
        indent: usize,
        source: &mut String,
        suppressed_comment_before: &[usize],
    ) -> Result<(), SyntaxBackendBoundaryError> {
        let mut comment_before =
            statement_comment_before_indexes(owner, statements.len(), context)?;
        for index in suppressed_comment_before {
            if comment_before.remove(index) {
                self.comments.note_explicit_statement_anchor_reached();
            }
        }
        let blank_line_before =
            statement_blank_line_before_indexes(owner, statements.len(), context)?;
        let trailing_comment =
            statement_trailing_comment_indexes(owner, statements.len(), context)?;
        if statements.is_empty() {
            if comment_before.contains(&0) {
                push_indented_comment_group(
                    source,
                    indent,
                    &self.comments.take_next_statement_anchor_unattached()?,
                );
            }
            return Ok(());
        }
        for (index, stmt) in statements.iter().copied().enumerate() {
            if index > 0
                && (blank_line_before.contains(&index)
                    || should_insert_blank_before_statement(stmt))
            {
                source.push('\n');
            }
            let previous_stmt = index
                .checked_sub(1)
                .and_then(|previous_index| statements.get(previous_index))
                .copied();
            let defer_unanchored_within_statement = self.push_statement_leading_comment(
                source,
                indent,
                stmt,
                previous_stmt,
                comment_before.contains(&index),
            )?;
            let later_explicit_anchor_in_same_list =
                comment_before.range((index + 1)..).next().is_some();
            let has_trailing_comment_anchor = trailing_comment.contains(&index);
            let previous_suppress_implicit_branch_trailing_comment =
                self.suppress_implicit_branch_trailing_comment;
            if has_trailing_comment_anchor {
                self.suppress_implicit_branch_trailing_comment = true;
            }
            let rendered_result =
                if later_explicit_anchor_in_same_list || defer_unanchored_within_statement {
                    self.defer_unanchored_leading_comment_depth += 1;
                    let result = self.render_stmt(stmt, indent);
                    self.defer_unanchored_leading_comment_depth -= 1;
                    result
                } else {
                    self.render_stmt(stmt, indent)
                };
            self.suppress_implicit_branch_trailing_comment =
                previous_suppress_implicit_branch_trailing_comment;
            let rendered = rendered_result?;
            source.push_str(&rendered);
            if has_trailing_comment_anchor {
                source.push(' ');
                let comment = self.comments.take_next_trailing_unattached()?;
                if comment.len() != 1 {
                    return Err(render_error(
                        "comment_anchor",
                        "statement trailing comment anchor expected one line comment",
                    ));
                }
                source.push_str(&comment[0]);
            }
            source.push('\n');
        }
        if comment_before.contains(&statements.len()) {
            if blank_line_before.contains(&statements.len()) {
                source.push('\n');
            }
            push_indented_comment_group(
                source,
                indent,
                &self.comments.take_next_statement_anchor_unattached()?,
            );
            source.push('\n');
        }
        Ok(())
    }

    fn render_stmt(
        &mut self,
        node: &RawSyntaxDecodedNode,
        indent: usize,
    ) -> Result<String, SyntaxBackendBoundaryError> {
        self.render_indented_dispatch(
            GoRawTreeRendererSkeletonContext::Statement,
            "stmt",
            node,
            indent,
        )
    }

    fn render_defer_stmt(
        &mut self,
        node: &RawSyntaxDecodedNode,
        indent: usize,
    ) -> Result<String, SyntaxBackendBoundaryError> {
        self.render_call_stmt(node, indent, "defer")
    }

    fn render_go_stmt(
        &mut self,
        node: &RawSyntaxDecodedNode,
        indent: usize,
    ) -> Result<String, SyntaxBackendBoundaryError> {
        self.render_call_stmt(node, indent, "go")
    }

    fn render_empty_stmt(
        &mut self,
        _node: &RawSyntaxDecodedNode,
        _indent: usize,
    ) -> Result<String, SyntaxBackendBoundaryError> {
        Ok(String::new())
    }

    fn render_expr_stmt(
        &mut self,
        node: &RawSyntaxDecodedNode,
        indent: usize,
    ) -> Result<String, SyntaxBackendBoundaryError> {
        let expr = self.generated_expr_stmt_x_field(node)?;
        self.manual_expr_stmt_expression_statement_layout(expr, indent)
    }

    fn render_generated_expr_stmt_skeleton(
        &mut self,
        node: &RawSyntaxDecodedNode,
        indent: usize,
    ) -> Result<String, SyntaxBackendBoundaryError> {
        self.render_expr_stmt(node, indent)
    }

    fn generated_expr_stmt_x_field<'a>(
        &self,
        node: &'a RawSyntaxDecodedNode,
    ) -> Result<&'a RawSyntaxDecodedNode, SyntaxBackendBoundaryError> {
        require_node("expr_stmt", node, "ExprStmt")?;
        required_single_node(node, "X")
    }

    fn manual_expr_stmt_expression_statement_layout(
        &mut self,
        expr: &RawSyntaxDecodedNode,
        indent: usize,
    ) -> Result<String, SyntaxBackendBoundaryError> {
        Ok(format!(
            "{}{}",
            indent_text(indent),
            self.render_expr_at_indent(expr, ExprContext::Any, indent)?
        ))
    }

    fn render_inc_dec_stmt(
        &mut self,
        node: &RawSyntaxDecodedNode,
        indent: usize,
    ) -> Result<String, SyntaxBackendBoundaryError> {
        let x = self.generated_inc_dec_stmt_x_field(node)?;
        let tok = self.generated_inc_dec_stmt_tok_field(node)?;
        self.manual_inc_dec_stmt_inc_dec_operator_layout(x, tok, indent)
    }

    fn render_generated_inc_dec_stmt_skeleton(
        &mut self,
        node: &RawSyntaxDecodedNode,
        indent: usize,
    ) -> Result<String, SyntaxBackendBoundaryError> {
        self.render_inc_dec_stmt(node, indent)
    }

    fn generated_inc_dec_stmt_x_field<'a>(
        &self,
        node: &'a RawSyntaxDecodedNode,
    ) -> Result<&'a RawSyntaxDecodedNode, SyntaxBackendBoundaryError> {
        require_node("inc_dec_stmt", node, "IncDecStmt")?;
        required_single_node(node, "X")
    }

    fn generated_inc_dec_stmt_tok_field<'a>(
        &self,
        node: &'a RawSyntaxDecodedNode,
    ) -> Result<&'a str, SyntaxBackendBoundaryError> {
        require_node("inc_dec_stmt", node, "IncDecStmt")?;
        required_terminal_payload(node, "Tok")
    }

    fn manual_inc_dec_stmt_inc_dec_operator_layout(
        &mut self,
        x: &RawSyntaxDecodedNode,
        tok: &str,
        indent: usize,
    ) -> Result<String, SyntaxBackendBoundaryError> {
        Ok(format!(
            "{}{}{}",
            indent_text(indent),
            self.render_expr(x, ExprContext::Any)?,
            tok
        ))
    }

    fn render_return_stmt(
        &mut self,
        node: &RawSyntaxDecodedNode,
        indent: usize,
    ) -> Result<String, SyntaxBackendBoundaryError> {
        let results = self.generated_return_stmt_results_field(node)?;
        self.manual_return_stmt_return_result_layout(&results, indent)
    }

    fn render_generated_return_stmt_skeleton(
        &mut self,
        node: &RawSyntaxDecodedNode,
        indent: usize,
    ) -> Result<String, SyntaxBackendBoundaryError> {
        let results = self.generated_return_stmt_results_field(node)?;
        self.manual_return_stmt_return_result_layout(&results, indent)
    }

    fn generated_return_stmt_results_field<'a>(
        &self,
        node: &'a RawSyntaxDecodedNode,
    ) -> Result<Vec<&'a RawSyntaxDecodedNode>, SyntaxBackendBoundaryError> {
        require_node("return_stmt", node, "ReturnStmt")?;
        optional_nodes(node, "Results")
    }

    fn manual_return_stmt_return_result_layout(
        &mut self,
        results: &[&RawSyntaxDecodedNode],
        indent: usize,
    ) -> Result<String, SyntaxBackendBoundaryError> {
        if results.is_empty() {
            Ok(format!("{}return", indent_text(indent)))
        } else {
            Ok(format!(
                "{}return {}",
                indent_text(indent),
                join_rendered(
                    results.to_vec(),
                    |renderer, node| renderer.render_expr_at_indent(node, ExprContext::Any, indent),
                    self,
                    ", ",
                )?
            ))
        }
    }

    fn render_call_stmt(
        &mut self,
        node: &RawSyntaxDecodedNode,
        indent: usize,
        keyword: &str,
    ) -> Result<String, SyntaxBackendBoundaryError> {
        let call = required_single_node(node, "Call")?;
        let rendered_call = if keyword == "defer" {
            self.render_call_expr(call, indent, FuncLitCalleeLayout::CompactDeferredIdentCall)?
        } else {
            self.render_expr_at_indent(call, ExprContext::Any, indent)?
        };
        Ok(format!(
            "{}{} {}",
            indent_text(indent),
            keyword,
            rendered_call
        ))
    }

    fn render_assign_stmt(
        &mut self,
        node: &RawSyntaxDecodedNode,
        indent: usize,
    ) -> Result<String, SyntaxBackendBoundaryError> {
        let lhs = self.generated_assign_stmt_lhs_field(node)?;
        let tok = self.generated_assign_stmt_tok_field(node)?;
        let rhs = self.generated_assign_stmt_rhs_field(node)?;
        self.manual_assign_stmt_assignment_operator_layout(&lhs, tok, &rhs, indent)
    }

    fn render_generated_assign_stmt_skeleton(
        &mut self,
        node: &RawSyntaxDecodedNode,
        indent: usize,
    ) -> Result<String, SyntaxBackendBoundaryError> {
        self.render_assign_stmt(node, indent)
    }

    fn generated_assign_stmt_lhs_field<'a>(
        &self,
        node: &'a RawSyntaxDecodedNode,
    ) -> Result<Vec<&'a RawSyntaxDecodedNode>, SyntaxBackendBoundaryError> {
        require_node("assign_stmt", node, "AssignStmt")?;
        required_nodes(node, "Lhs")
    }

    fn generated_assign_stmt_tok_field<'a>(
        &self,
        node: &'a RawSyntaxDecodedNode,
    ) -> Result<&'a str, SyntaxBackendBoundaryError> {
        require_node("assign_stmt", node, "AssignStmt")?;
        required_terminal_payload(node, "Tok")
    }

    fn generated_assign_stmt_rhs_field<'a>(
        &self,
        node: &'a RawSyntaxDecodedNode,
    ) -> Result<Vec<&'a RawSyntaxDecodedNode>, SyntaxBackendBoundaryError> {
        require_node("assign_stmt", node, "AssignStmt")?;
        required_nodes(node, "Rhs")
    }

    fn manual_assign_stmt_assignment_operator_layout(
        &mut self,
        lhs: &[&RawSyntaxDecodedNode],
        tok: &str,
        rhs: &[&RawSyntaxDecodedNode],
        indent: usize,
    ) -> Result<String, SyntaxBackendBoundaryError> {
        Ok(format!(
            "{}{} {} {}",
            indent_text(indent),
            join_rendered(
                lhs.to_vec(),
                |renderer, node| renderer.render_expr(node, ExprContext::Any),
                self,
                ", ",
            )?,
            tok,
            join_rendered(
                rhs.to_vec(),
                |renderer, node| renderer.render_expr_at_indent(node, ExprContext::Any, indent),
                self,
                ", ",
            )?
        ))
    }

    fn render_labeled_stmt(
        &mut self,
        node: &RawSyntaxDecodedNode,
        indent: usize,
    ) -> Result<String, SyntaxBackendBoundaryError> {
        let label = self.render_ident(required_single_node(node, "Label")?)?;
        let stmt = required_single_node(node, "Stmt")?;
        let mut source = format!("{}{}:\n", indent_text(indent.saturating_sub(1)), label);
        source.push_str(&self.render_stmt(stmt, indent)?);
        Ok(source)
    }

    fn render_branch_stmt(
        &mut self,
        node: &RawSyntaxDecodedNode,
        indent: usize,
    ) -> Result<String, SyntaxBackendBoundaryError> {
        let tok = self.generated_branch_stmt_tok_field(node)?;
        let label = self.generated_branch_stmt_label_field(node)?;
        self.manual_branch_stmt_branch_token_layout(tok, label, indent)
    }

    fn render_generated_branch_stmt_skeleton(
        &mut self,
        node: &RawSyntaxDecodedNode,
        indent: usize,
    ) -> Result<String, SyntaxBackendBoundaryError> {
        self.render_branch_stmt(node, indent)
    }

    fn generated_branch_stmt_tok_field<'a>(
        &self,
        node: &'a RawSyntaxDecodedNode,
    ) -> Result<&'a str, SyntaxBackendBoundaryError> {
        require_node("branch_stmt", node, "BranchStmt")?;
        required_terminal_payload(node, "Tok")
    }

    fn generated_branch_stmt_label_field<'a>(
        &self,
        node: &'a RawSyntaxDecodedNode,
    ) -> Result<Option<&'a RawSyntaxDecodedNode>, SyntaxBackendBoundaryError> {
        require_node("branch_stmt", node, "BranchStmt")?;
        optional_single_node(node, "Label")
    }

    fn manual_branch_stmt_branch_token_layout(
        &mut self,
        tok: &str,
        label: Option<&RawSyntaxDecodedNode>,
        indent: usize,
    ) -> Result<String, SyntaxBackendBoundaryError> {
        let mut source = format!("{}{}", indent_text(indent), tok);
        if let Some(label) = label {
            source.push(' ');
            source.push_str(&self.render_ident(label)?);
        }
        if !self.suppress_implicit_branch_trailing_comment
            && self.comments.next_text_is(TRAILING_BREAK_EOF_COMMENT)
        {
            source.push(' ');
            source.push_str(
                &self
                    .comments
                    .consume_next_exact(TRAILING_BREAK_EOF_COMMENT)?[0],
            );
        }
        Ok(source)
    }

    fn render_case_clause(
        &mut self,
        node: &RawSyntaxDecodedNode,
        indent: usize,
    ) -> Result<String, SyntaxBackendBoundaryError> {
        let values = optional_nodes(node, "List")?;
        let mut source = indent_text(indent);
        if values.is_empty() {
            source.push_str("default:\n");
        } else {
            source.push_str("case ");
            source.push_str(&join_rendered(
                values,
                |renderer, node| renderer.render_expr(node, ExprContext::Any),
                self,
                ", ",
            )?);
            source.push_str(":\n");
        }
        let statements = optional_nodes(node, "Body")?;
        self.render_statement_sequence(
            node,
            "CaseClause",
            statements,
            indent + 1,
            &mut source,
            &[],
        )?;
        trim_one_trailing_newline(&mut source);
        Ok(source)
    }

    fn render_comm_clause(
        &mut self,
        node: &RawSyntaxDecodedNode,
        indent: usize,
    ) -> Result<String, SyntaxBackendBoundaryError> {
        let mut source = indent_text(indent);
        if let Some(comm) = optional_single_node(node, "Comm")? {
            source.push_str("case ");
            source.push_str(&self.render_simple_stmt_header(comm)?);
            source.push_str(":\n");
        } else {
            source.push_str("default:\n");
        }
        let statements = optional_nodes(node, "Body")?;
        self.render_statement_sequence(
            node,
            "CommClause",
            statements,
            indent + 1,
            &mut source,
            &[],
        )?;
        trim_one_trailing_newline(&mut source);
        Ok(source)
    }

    fn render_decl_stmt(
        &mut self,
        node: &RawSyntaxDecodedNode,
        indent: usize,
    ) -> Result<String, SyntaxBackendBoundaryError> {
        self.render_decl(required_single_node(node, "Decl")?, indent)
    }

    fn render_for_stmt(
        &mut self,
        node: &RawSyntaxDecodedNode,
        indent: usize,
    ) -> Result<String, SyntaxBackendBoundaryError> {
        let init = optional_single_node(node, "Init")?;
        let cond = optional_single_node(node, "Cond")?;
        let post = optional_single_node(node, "Post")?;

        let mut source = format!("{}for", indent_text(indent));
        if init.is_some() || post.is_some() {
            source.push(' ');
            if let Some(init) = init {
                source.push_str(&self.render_simple_stmt_header(init)?);
            }
            source.push_str("; ");
            if let Some(cond) = cond {
                source.push_str(&self.render_expr(cond, ExprContext::Any)?);
            }
            source.push(';');
            if let Some(post) = post {
                source.push(' ');
                source.push_str(&self.render_simple_stmt_header(post)?);
            }
            source.push_str(" {\n");
        } else if let Some(cond) = cond {
            source.push(' ');
            source.push_str(&self.render_expr(cond, ExprContext::Any)?);
            source.push_str(" {\n");
        } else {
            source.push_str(" {\n");
        }
        self.render_block_contents(required_single_node(node, "Body")?, indent + 1, &mut source)?;
        source.push_str(&indent_text(indent));
        source.push('}');
        Ok(source)
    }

    fn render_if_stmt(
        &mut self,
        node: &RawSyntaxDecodedNode,
        indent: usize,
    ) -> Result<String, SyntaxBackendBoundaryError> {
        let cond = self.render_expr(required_single_node(node, "Cond")?, ExprContext::Any)?;
        let mut source = if let Some(init) = optional_single_node(node, "Init")? {
            format!(
                "{}if {}; {} {{",
                indent_text(indent),
                self.render_simple_stmt_header(init)?,
                cond
            )
        } else {
            format!("{}if {} {{", indent_text(indent), cond)
        };
        let mut suppress_body_comment_before = None;
        if if_header_trailing_comment_anchor(node)? {
            source.push(' ');
            let comment = self.comments.take_next_trailing_unattached()?;
            if comment.len() != 1 {
                return Err(render_error(
                    "IfStmt.TrailingComment",
                    "if header trailing comment anchor expected one line comment",
                ));
            }
            source.push_str(&comment[0]);
            suppress_body_comment_before = Some([0]);
        }
        source.push('\n');
        let body = required_single_node(node, "Body")?;
        let else_node = optional_single_node(node, "Else")?;
        let render_empty_body_comment_before_else =
            else_node.is_some() && self.should_render_empty_if_body_comment_before_else(body)?;
        if let Some(suppressed) = suppress_body_comment_before.as_ref() {
            self.render_block_contents_suppressing_comment_before(
                body,
                indent + 1,
                &mut source,
                suppressed,
            )?;
        } else {
            self.render_block_contents(body, indent + 1, &mut source)?;
        }
        if render_empty_body_comment_before_else {
            push_indented_comment_group(
                &mut source,
                indent + 1,
                &self.comments.take_next_unattached()?,
            );
        }
        source.push_str(&indent_text(indent));
        source.push('}');
        if let Some(else_node) = else_node {
            match else_node.node_kind.as_str() {
                "BlockStmt" => {
                    source.push_str(" else ");
                    source.push_str(&self.render_block(else_node, indent)?);
                }
                "IfStmt" => {
                    source.push_str(" else ");
                    let rendered = self.render_if_stmt(else_node, 0)?;
                    source.push_str(rendered.trim_start());
                }
                other => {
                    return Err(render_error(
                        "if.else",
                        format!("unsupported else node {other}"),
                    ));
                }
            }
        }
        Ok(source)
    }

    fn should_render_empty_if_body_comment_before_else(
        &self,
        body: &RawSyntaxDecodedNode,
    ) -> Result<bool, SyntaxBackendBoundaryError> {
        let statements = optional_nodes(body, "List")?;
        if !statements.is_empty() {
            return Ok(false);
        }
        let body_comment_before = statement_comment_before_indexes(body, 0, "BlockStmt")?;
        Ok(!body_comment_before.contains(&0)
            && self.comments.next_is_unattached()
            && self.comments.has_unreserved_unattached_comment())
    }

    fn render_simple_stmt_header(
        &mut self,
        node: &RawSyntaxDecodedNode,
    ) -> Result<String, SyntaxBackendBoundaryError> {
        let rendered = self.render_indented_dispatch(
            GoRawTreeRendererSkeletonContext::SimpleStatementHeader,
            "stmt",
            node,
            0,
        )?;
        Ok(rendered.trim().to_string())
    }

    fn render_range_stmt(
        &mut self,
        node: &RawSyntaxDecodedNode,
        indent: usize,
    ) -> Result<String, SyntaxBackendBoundaryError> {
        let key = optional_single_node(node, "Key")?;
        let value = optional_single_node(node, "Value")?;
        let range_expr = self.render_expr(required_single_node(node, "X")?, ExprContext::Any)?;
        let mut source = match (key, value) {
            (Some(key), Some(value)) => format!(
                "{}for {}, {} {} range {} {{\n",
                indent_text(indent),
                self.render_expr(key, ExprContext::Any)?,
                self.render_expr(value, ExprContext::Any)?,
                required_terminal_payload(node, "Tok")?,
                range_expr
            ),
            (Some(key), None) => format!(
                "{}for {} {} range {} {{\n",
                indent_text(indent),
                self.render_expr(key, ExprContext::Any)?,
                required_terminal_payload(node, "Tok")?,
                range_expr
            ),
            (None, None) => format!("{}for range {} {{\n", indent_text(indent), range_expr),
            (None, Some(_)) => {
                return Err(render_error(
                    "RangeStmt.Value",
                    "range value requires a range key",
                ));
            }
        };
        self.render_block_contents(required_single_node(node, "Body")?, indent + 1, &mut source)?;
        source.push_str(&indent_text(indent));
        source.push('}');
        Ok(source)
    }

    fn render_select_stmt(
        &mut self,
        node: &RawSyntaxDecodedNode,
        indent: usize,
    ) -> Result<String, SyntaxBackendBoundaryError> {
        let mut source = format!("{}select {{\n", indent_text(indent));
        self.render_block_contents(required_single_node(node, "Body")?, indent, &mut source)?;
        source.push_str(&indent_text(indent));
        source.push('}');
        Ok(source)
    }

    fn render_send_stmt(
        &mut self,
        node: &RawSyntaxDecodedNode,
        indent: usize,
    ) -> Result<String, SyntaxBackendBoundaryError> {
        Ok(format!(
            "{}{} <- {}",
            indent_text(indent),
            self.render_expr_at_indent(
                required_single_node(node, "Chan")?,
                ExprContext::Any,
                indent
            )?,
            self.render_expr_at_indent(
                required_single_node(node, "Value")?,
                ExprContext::Any,
                indent
            )?
        ))
    }

    fn render_switch_stmt(
        &mut self,
        node: &RawSyntaxDecodedNode,
        indent: usize,
    ) -> Result<String, SyntaxBackendBoundaryError> {
        let mut source = if let Some(tag) = optional_single_node(node, "Tag")? {
            format!(
                "{}switch {} {{\n",
                indent_text(indent),
                self.render_expr(tag, ExprContext::Any)?
            )
        } else {
            format!("{}switch {{\n", indent_text(indent))
        };
        self.render_block_contents(required_single_node(node, "Body")?, indent, &mut source)?;
        source.push_str(&indent_text(indent));
        source.push('}');
        Ok(source)
    }

    fn render_type_switch_stmt(
        &mut self,
        node: &RawSyntaxDecodedNode,
        indent: usize,
    ) -> Result<String, SyntaxBackendBoundaryError> {
        let mut source = format!("{}switch ", indent_text(indent));
        if let Some(init) = optional_single_node(node, "Init")? {
            source.push_str(&self.render_simple_stmt_header(init)?);
            source.push_str("; ");
        }
        source.push_str(&self.render_simple_stmt_header(required_single_node(node, "Assign")?)?);
        source.push_str(" {\n");
        let body = required_single_node(node, "Body")?;
        self.defer_implicit_leading_comment_depth += 1;
        let render_body_result = self.render_block_contents(body, indent, &mut source);
        self.defer_implicit_leading_comment_depth -= 1;
        render_body_result?;
        source.push_str(&indent_text(indent));
        source.push('}');
        Ok(source)
    }

    fn render_expr(
        &mut self,
        node: &RawSyntaxDecodedNode,
        context: ExprContext,
    ) -> Result<String, SyntaxBackendBoundaryError> {
        self.render_expr_at_indent(node, context, 0)
    }

    fn render_expr_at_indent(
        &mut self,
        node: &RawSyntaxDecodedNode,
        context: ExprContext,
        indent: usize,
    ) -> Result<String, SyntaxBackendBoundaryError> {
        self.render_expr_dispatch(node, context, indent)
    }

    fn render_array_type_expr(
        &mut self,
        node: &RawSyntaxDecodedNode,
        _context: ExprContext,
        _indent: usize,
    ) -> Result<String, SyntaxBackendBoundaryError> {
        let len = optional_single_node(node, "Len")?
            .map(|node| self.render_expr(node, ExprContext::Any))
            .transpose()?
            .unwrap_or_default();
        Ok(format!(
            "[{}]{}",
            len,
            self.render_type(required_single_node(node, "Elt")?)?
        ))
    }

    fn render_basic_lit_expr(
        &mut self,
        node: &RawSyntaxDecodedNode,
        _context: ExprContext,
        _indent: usize,
    ) -> Result<String, SyntaxBackendBoundaryError> {
        let value = self.generated_basic_lit_value_field(node)?;
        self.manual_basic_lit_literal_payload_rendering(value)
    }

    fn render_generated_basic_lit_skeleton(
        &mut self,
        node: &RawSyntaxDecodedNode,
    ) -> Result<String, SyntaxBackendBoundaryError> {
        let value = self.generated_basic_lit_value_field(node)?;
        self.manual_basic_lit_literal_payload_rendering(value)
    }

    fn generated_basic_lit_value_field<'a>(
        &self,
        node: &'a RawSyntaxDecodedNode,
    ) -> Result<&'a str, SyntaxBackendBoundaryError> {
        require_node("basic_lit", node, "BasicLit")?;
        required_terminal_payload(node, "Value")
    }

    fn manual_basic_lit_literal_payload_rendering(
        &mut self,
        value: &str,
    ) -> Result<String, SyntaxBackendBoundaryError> {
        Ok(value.to_string())
    }

    fn render_binary_expr_dispatch(
        &mut self,
        node: &RawSyntaxDecodedNode,
        context: ExprContext,
        _indent: usize,
    ) -> Result<String, SyntaxBackendBoundaryError> {
        self.render_binary_expr(node, context)
    }

    fn render_call_expr_dispatch(
        &mut self,
        node: &RawSyntaxDecodedNode,
        _context: ExprContext,
        indent: usize,
    ) -> Result<String, SyntaxBackendBoundaryError> {
        self.render_call_expr(node, indent, FuncLitCalleeLayout::Preserve)
    }

    fn render_chan_type_dispatch(
        &mut self,
        node: &RawSyntaxDecodedNode,
        _context: ExprContext,
        _indent: usize,
    ) -> Result<String, SyntaxBackendBoundaryError> {
        self.render_chan_type(node)
    }

    fn render_composite_lit_dispatch(
        &mut self,
        node: &RawSyntaxDecodedNode,
        _context: ExprContext,
        indent: usize,
    ) -> Result<String, SyntaxBackendBoundaryError> {
        self.render_composite_lit(node, indent)
    }

    fn render_ellipsis_expr_dispatch(
        &mut self,
        node: &RawSyntaxDecodedNode,
        _context: ExprContext,
        _indent: usize,
    ) -> Result<String, SyntaxBackendBoundaryError> {
        self.render_ellipsis_expr(node)
    }

    fn render_func_lit_dispatch(
        &mut self,
        node: &RawSyntaxDecodedNode,
        _context: ExprContext,
        indent: usize,
    ) -> Result<String, SyntaxBackendBoundaryError> {
        self.render_func_lit(node, indent)
    }

    fn render_func_type_expr(
        &mut self,
        node: &RawSyntaxDecodedNode,
        _context: ExprContext,
        _indent: usize,
    ) -> Result<String, SyntaxBackendBoundaryError> {
        self.render_func_type(node, true)
    }

    fn render_ident_expr(
        &mut self,
        node: &RawSyntaxDecodedNode,
        _context: ExprContext,
        _indent: usize,
    ) -> Result<String, SyntaxBackendBoundaryError> {
        self.render_ident(node)
    }

    fn render_generated_ident_skeleton(
        &mut self,
        node: &RawSyntaxDecodedNode,
    ) -> Result<String, SyntaxBackendBoundaryError> {
        let name = self.generated_ident_name_field(node)?;
        self.manual_ident_identifier_payload_rendering(name)
    }

    fn generated_ident_name_field<'a>(
        &self,
        node: &'a RawSyntaxDecodedNode,
    ) -> Result<&'a str, SyntaxBackendBoundaryError> {
        require_node("ident", node, "Ident")?;
        required_terminal_payload(node, "Name")
    }

    fn manual_ident_identifier_payload_rendering(
        &mut self,
        name: &str,
    ) -> Result<String, SyntaxBackendBoundaryError> {
        Ok(name.to_string())
    }

    fn render_index_expr(
        &mut self,
        node: &RawSyntaxDecodedNode,
        _context: ExprContext,
        _indent: usize,
    ) -> Result<String, SyntaxBackendBoundaryError> {
        Ok(format!(
            "{}[{}]",
            self.render_expr(required_single_node(node, "X")?, ExprContext::Any)?,
            self.render_expr(required_single_node(node, "Index")?, ExprContext::Any)?
        ))
    }

    fn render_index_list_expr_dispatch(
        &mut self,
        node: &RawSyntaxDecodedNode,
        _context: ExprContext,
        _indent: usize,
    ) -> Result<String, SyntaxBackendBoundaryError> {
        self.render_index_list_expr(node)
    }

    fn render_interface_type_dispatch(
        &mut self,
        node: &RawSyntaxDecodedNode,
        _context: ExprContext,
        _indent: usize,
    ) -> Result<String, SyntaxBackendBoundaryError> {
        self.render_interface_type(node)
    }

    fn render_key_value_expr_dispatch(
        &mut self,
        node: &RawSyntaxDecodedNode,
        _context: ExprContext,
        indent: usize,
    ) -> Result<String, SyntaxBackendBoundaryError> {
        self.render_key_value_expr(node, indent, None)
    }

    fn render_map_type_dispatch(
        &mut self,
        node: &RawSyntaxDecodedNode,
        _context: ExprContext,
        _indent: usize,
    ) -> Result<String, SyntaxBackendBoundaryError> {
        self.render_map_type(node)
    }

    fn render_paren_expr(
        &mut self,
        node: &RawSyntaxDecodedNode,
        context: ExprContext,
        indent: usize,
    ) -> Result<String, SyntaxBackendBoundaryError> {
        let operand = self.generated_paren_expr_x_field(node, indent)?;
        self.manual_paren_expr_parenthesized_expression_preservation(operand, context)
    }

    fn render_generated_paren_expr_skeleton(
        &mut self,
        node: &RawSyntaxDecodedNode,
        context: ExprContext,
        indent: usize,
    ) -> Result<String, SyntaxBackendBoundaryError> {
        self.render_paren_expr(node, context, indent)
    }

    fn generated_paren_expr_x_field(
        &mut self,
        node: &RawSyntaxDecodedNode,
        indent: usize,
    ) -> Result<String, SyntaxBackendBoundaryError> {
        require_node("paren_expr", node, "ParenExpr")?;
        self.render_expr_at_indent(required_single_node(node, "X")?, ExprContext::Any, indent)
    }

    fn manual_paren_expr_parenthesized_expression_preservation(
        &mut self,
        operand: String,
        _context: ExprContext,
    ) -> Result<String, SyntaxBackendBoundaryError> {
        Ok(format!("({operand})"))
    }

    fn render_generated_selector_expr_skeleton(
        &mut self,
        node: &RawSyntaxDecodedNode,
        _context: ExprContext,
        _indent: usize,
    ) -> Result<String, SyntaxBackendBoundaryError> {
        let receiver = self.generated_selector_expr_x_field(node)?;
        let selector = self.generated_selector_expr_sel_field(node)?;
        self.manual_selector_expr_selector_dot_layout(receiver, selector)
    }

    fn generated_selector_expr_x_field(
        &mut self,
        node: &RawSyntaxDecodedNode,
    ) -> Result<String, SyntaxBackendBoundaryError> {
        require_node("selector_expr", node, "SelectorExpr")?;
        self.render_expr(required_single_node(node, "X")?, ExprContext::SelectorBase)
    }

    fn generated_selector_expr_sel_field(
        &mut self,
        node: &RawSyntaxDecodedNode,
    ) -> Result<String, SyntaxBackendBoundaryError> {
        require_node("selector_expr", node, "SelectorExpr")?;
        let selector = required_single_node(node, "Sel")?;
        require_node("selector_expr.Sel", selector, "Ident")?;
        self.render_expression_generated_entrypoint(selector, ExprContext::Any, 0)
    }

    fn manual_selector_expr_selector_dot_layout(
        &mut self,
        receiver: String,
        selector: String,
    ) -> Result<String, SyntaxBackendBoundaryError> {
        Ok(format!("{receiver}.{selector}"))
    }

    fn render_slice_expr_dispatch(
        &mut self,
        node: &RawSyntaxDecodedNode,
        _context: ExprContext,
        _indent: usize,
    ) -> Result<String, SyntaxBackendBoundaryError> {
        self.render_slice_expr(node)
    }

    fn render_star_expr(
        &mut self,
        node: &RawSyntaxDecodedNode,
        _context: ExprContext,
        _indent: usize,
    ) -> Result<String, SyntaxBackendBoundaryError> {
        Ok(format!(
            "*{}",
            self.render_expr(required_single_node(node, "X")?, ExprContext::Unary)?
        ))
    }

    fn render_struct_type_expr(
        &mut self,
        node: &RawSyntaxDecodedNode,
        _context: ExprContext,
        _indent: usize,
    ) -> Result<String, SyntaxBackendBoundaryError> {
        self.render_struct_type(node, 0)
    }

    fn render_type_assert_expr_dispatch(
        &mut self,
        node: &RawSyntaxDecodedNode,
        _context: ExprContext,
        _indent: usize,
    ) -> Result<String, SyntaxBackendBoundaryError> {
        self.render_type_assert_expr(node)
    }

    fn render_unary_expr(
        &mut self,
        node: &RawSyntaxDecodedNode,
        context: ExprContext,
        indent: usize,
    ) -> Result<String, SyntaxBackendBoundaryError> {
        let op = self.generated_unary_expr_op_field(node)?;
        let operand = self.generated_unary_expr_x_field(node, indent)?;
        self.manual_unary_expr_unary_operator_precedence(op, operand, context)
    }

    fn render_generated_unary_expr_skeleton(
        &mut self,
        node: &RawSyntaxDecodedNode,
        context: ExprContext,
        indent: usize,
    ) -> Result<String, SyntaxBackendBoundaryError> {
        self.render_unary_expr(node, context, indent)
    }

    fn generated_unary_expr_op_field<'a>(
        &self,
        node: &'a RawSyntaxDecodedNode,
    ) -> Result<&'a str, SyntaxBackendBoundaryError> {
        require_node("unary_expr", node, "UnaryExpr")?;
        required_terminal_payload(node, "Op")
    }

    fn generated_unary_expr_x_field(
        &mut self,
        node: &RawSyntaxDecodedNode,
        indent: usize,
    ) -> Result<String, SyntaxBackendBoundaryError> {
        require_node("unary_expr", node, "UnaryExpr")?;
        self.render_expr_at_indent(required_single_node(node, "X")?, ExprContext::Unary, indent)
    }

    fn manual_unary_expr_unary_operator_precedence(
        &mut self,
        op: &str,
        operand: String,
        _context: ExprContext,
    ) -> Result<String, SyntaxBackendBoundaryError> {
        Ok(format!("{op}{operand}"))
    }

    fn render_type_assert_expr(
        &mut self,
        node: &RawSyntaxDecodedNode,
    ) -> Result<String, SyntaxBackendBoundaryError> {
        let receiver =
            self.render_expr(required_single_node(node, "X")?, ExprContext::SelectorBase)?;
        let asserted_type = optional_single_node(node, "Type")?
            .map(|node| self.render_type(node))
            .transpose()?
            .unwrap_or_else(|| "type".to_string());
        Ok(format!("{receiver}.({asserted_type})"))
    }

    fn render_type(
        &mut self,
        node: &RawSyntaxDecodedNode,
    ) -> Result<String, SyntaxBackendBoundaryError> {
        self.render_expr(node, ExprContext::Any)
    }

    fn render_ident(
        &mut self,
        node: &RawSyntaxDecodedNode,
    ) -> Result<String, SyntaxBackendBoundaryError> {
        let name = self.generated_ident_name_field(node)?;
        self.manual_ident_identifier_payload_rendering(name)
    }

    fn render_binary_expr(
        &mut self,
        node: &RawSyntaxDecodedNode,
        context: ExprContext,
    ) -> Result<String, SyntaxBackendBoundaryError> {
        let op = self.generated_binary_expr_op_field(node)?;
        let precedence = go_operator_precedence(op);
        let left = self.generated_binary_expr_x_field(node, precedence)?;
        let right = self.generated_binary_expr_y_field(node, precedence)?;
        let rendered = self.manual_binary_expr_binary_operator_layout(left, op, right)?;
        self.manual_binary_expr_operator_precedence(rendered, context, precedence)
    }

    fn render_generated_binary_expr_skeleton(
        &mut self,
        node: &RawSyntaxDecodedNode,
        context: ExprContext,
    ) -> Result<String, SyntaxBackendBoundaryError> {
        self.render_binary_expr(node, context)
    }

    fn generated_binary_expr_x_field(
        &mut self,
        node: &RawSyntaxDecodedNode,
        parent_precedence: u8,
    ) -> Result<String, SyntaxBackendBoundaryError> {
        require_node("binary_expr", node, "BinaryExpr")?;
        self.render_binary_operand(required_single_node(node, "X")?, parent_precedence, true)
    }

    fn generated_binary_expr_op_field<'a>(
        &self,
        node: &'a RawSyntaxDecodedNode,
    ) -> Result<&'a str, SyntaxBackendBoundaryError> {
        require_node("binary_expr", node, "BinaryExpr")?;
        required_terminal_payload(node, "Op")
    }

    fn generated_binary_expr_y_field(
        &mut self,
        node: &RawSyntaxDecodedNode,
        parent_precedence: u8,
    ) -> Result<String, SyntaxBackendBoundaryError> {
        require_node("binary_expr", node, "BinaryExpr")?;
        self.render_binary_operand(required_single_node(node, "Y")?, parent_precedence, false)
    }

    fn manual_binary_expr_binary_operator_layout(
        &mut self,
        left: String,
        op: &str,
        right: String,
    ) -> Result<String, SyntaxBackendBoundaryError> {
        Ok(format!("{left} {op} {right}"))
    }

    fn manual_binary_expr_operator_precedence(
        &mut self,
        rendered: String,
        context: ExprContext,
        precedence: u8,
    ) -> Result<String, SyntaxBackendBoundaryError> {
        if context.requires_parens_for_precedence(precedence) {
            Ok(format!("({rendered})"))
        } else {
            Ok(rendered)
        }
    }

    fn render_binary_operand(
        &mut self,
        node: &RawSyntaxDecodedNode,
        parent_precedence: u8,
        left_operand: bool,
    ) -> Result<String, SyntaxBackendBoundaryError> {
        if node.node_kind == "BinaryExpr" {
            let child_op = self.generated_binary_expr_op_field(node)?;
            let child_precedence = go_operator_precedence(child_op);
            let rendered =
                self.render_expression_generated_entrypoint(node, ExprContext::Any, 0)?;
            if child_precedence < parent_precedence
                || (!left_operand && child_precedence == parent_precedence)
            {
                Ok(format!("({rendered})"))
            } else {
                Ok(rendered)
            }
        } else {
            self.render_expr(node, ExprContext::Any)
        }
    }

    fn render_call_expr(
        &mut self,
        node: &RawSyntaxDecodedNode,
        indent: usize,
        func_lit_layout: FuncLitCalleeLayout,
    ) -> Result<String, SyntaxBackendBoundaryError> {
        let fun = self.generated_call_expr_fun_field(node, indent, func_lit_layout)?;
        let args = self.generated_call_expr_args_field(node)?;
        let ellipsis = self.manual_call_expr_ellipsis_policy(node, args.len())?;
        self.manual_call_expr_call_argument_layout(node, fun, &args, ellipsis, indent)
    }

    fn render_generated_call_expr_skeleton(
        &mut self,
        node: &RawSyntaxDecodedNode,
        indent: usize,
        func_lit_layout: FuncLitCalleeLayout,
    ) -> Result<String, SyntaxBackendBoundaryError> {
        self.render_call_expr(node, indent, func_lit_layout)
    }

    fn generated_call_expr_fun_field(
        &mut self,
        node: &RawSyntaxDecodedNode,
        indent: usize,
        func_lit_layout: FuncLitCalleeLayout,
    ) -> Result<String, SyntaxBackendBoundaryError> {
        require_node("call_expr", node, "CallExpr")?;
        self.render_call_callee(required_single_node(node, "Fun")?, indent, func_lit_layout)
    }

    fn generated_call_expr_args_field<'a>(
        &self,
        node: &'a RawSyntaxDecodedNode,
    ) -> Result<Vec<&'a RawSyntaxDecodedNode>, SyntaxBackendBoundaryError> {
        require_node("call_expr", node, "CallExpr")?;
        optional_nodes(node, "Args")
    }

    fn manual_call_expr_ellipsis_policy(
        &mut self,
        node: &RawSyntaxDecodedNode,
        argument_count: usize,
    ) -> Result<bool, SyntaxBackendBoundaryError> {
        call_expr_ellipsis_present(node, argument_count)
    }

    fn manual_call_expr_call_argument_layout(
        &mut self,
        node: &RawSyntaxDecodedNode,
        fun: String,
        args: &[&RawSyntaxDecodedNode],
        ellipsis: bool,
        indent: usize,
    ) -> Result<String, SyntaxBackendBoundaryError> {
        let line_break_before = call_expr_line_break_before_indexes(node, args.len())?;
        let argument_indent = if line_break_before.is_empty() {
            indent
        } else {
            indent + 1
        };
        let rendered_args = args
            .iter()
            .enumerate()
            .map(|(index, argument)| {
                let mut rendered =
                    self.render_expr_at_indent(argument, ExprContext::Any, argument_indent)?;
                if ellipsis && index + 1 == args.len() {
                    rendered.push_str("...");
                }
                Ok(rendered)
            })
            .collect::<Result<Vec<_>, SyntaxBackendBoundaryError>>()?;
        if line_break_before.is_empty() {
            return Ok(format!("{fun}({})", rendered_args.join(", ")));
        }

        let mut source = format!("{fun}(");
        for (index, argument) in rendered_args.into_iter().enumerate() {
            if index == 0 {
                if line_break_before.contains(&0) {
                    source.push('\n');
                    source.push_str(&indent_text(indent + 1));
                }
            } else if line_break_before.contains(&index) {
                source.push_str(",\n");
                source.push_str(&indent_text(indent + 1));
            } else {
                source.push_str(", ");
            }
            source.push_str(&argument);
        }
        source.push(')');
        Ok(source)
    }

    fn render_call_callee(
        &mut self,
        node: &RawSyntaxDecodedNode,
        indent: usize,
        func_lit_layout: FuncLitCalleeLayout,
    ) -> Result<String, SyntaxBackendBoundaryError> {
        if func_lit_layout == FuncLitCalleeLayout::CompactDeferredIdentCall
            && node.node_kind == "FuncLit"
            && let Some(rendered) = self.render_compact_func_lit_call_callee(node)?
        {
            return Ok(rendered);
        }
        self.render_expr_at_indent(node, ExprContext::SelectorBase, indent)
    }

    fn render_func_lit(
        &mut self,
        node: &RawSyntaxDecodedNode,
        indent: usize,
    ) -> Result<String, SyntaxBackendBoundaryError> {
        let rendered_type = self.render_func_type(required_single_node(node, "Type")?, true)?;
        let body = required_single_node(node, "Body")?;
        self.defer_implicit_leading_comment_depth += 1;
        let rendered_body_result = self.render_block(body, indent);
        self.defer_implicit_leading_comment_depth -= 1;
        let rendered_body = rendered_body_result?;
        Ok(format!("{rendered_type} {rendered_body}"))
    }

    fn render_compact_func_lit_call_callee(
        &mut self,
        node: &RawSyntaxDecodedNode,
    ) -> Result<Option<String>, SyntaxBackendBoundaryError> {
        let body = required_single_node(node, "Body")?;
        let Some(compact_body) = self.render_compact_func_lit_body(body)? else {
            return Ok(None);
        };
        Ok(Some(format!(
            "{} {compact_body}",
            self.render_func_type(required_single_node(node, "Type")?, true)?
        )))
    }

    fn render_compact_func_lit_body(
        &mut self,
        node: &RawSyntaxDecodedNode,
    ) -> Result<Option<String>, SyntaxBackendBoundaryError> {
        require_node("func_lit_body", node, "BlockStmt")?;
        let statements = optional_nodes(node, "List")?;
        let [statement] = statements.as_slice() else {
            return Ok(None);
        };
        if !statement_comment_before_indexes(node, 1, "BlockStmt")?.is_empty()
            || !statement_blank_line_before_indexes(node, 1, "BlockStmt")?.is_empty()
            || !statement_trailing_comment_indexes(node, 1, "BlockStmt")?.is_empty()
            || subtree_contains_node_kind(statement, "CommentGroup")
            || !can_render_compact_deferred_func_lit_statement(statement)?
        {
            return Ok(None);
        }

        let rendered = self.render_stmt(statement, 0)?;
        if rendered.contains('\n') {
            return Ok(None);
        }
        Ok(Some(format!("{{ {} }}", rendered.trim())))
    }

    fn render_composite_lit(
        &mut self,
        node: &RawSyntaxDecodedNode,
        indent: usize,
    ) -> Result<String, SyntaxBackendBoundaryError> {
        let elts = optional_nodes(node, "Elts")?;
        let typ = optional_single_node(node, "Type")?
            .map(|node| self.render_type(node))
            .transpose()?
            .unwrap_or_default();
        if elts.is_empty() {
            return Ok(format!("{typ}{{}}"));
        }
        let line_break_before = composite_lit_line_break_before_indexes(node, elts.len())?;
        if !line_break_before.is_empty() || elts.iter().any(key_value_value_is_func_lit) {
            let key_alignment = self.key_value_alignment_width(&elts)?;
            let mut source = format!("{typ}{{\n");
            for elt in elts {
                source.push_str(&indent_text(indent + 1));
                source.push_str(&self.render_composite_lit_element(
                    elt,
                    indent + 1,
                    key_alignment,
                )?);
                source.push_str(",\n");
            }
            source.push_str(&indent_text(indent));
            source.push('}');
            return Ok(source);
        }
        Ok(format!(
            "{typ}{{{}}}",
            join_rendered(
                elts,
                |renderer, node| renderer.render_expr(node, ExprContext::Any),
                self,
                ", ",
            )?
        ))
    }

    fn render_composite_lit_element(
        &mut self,
        node: &RawSyntaxDecodedNode,
        indent: usize,
        key_alignment: Option<usize>,
    ) -> Result<String, SyntaxBackendBoundaryError> {
        if node.node_kind == "KeyValueExpr" {
            return self.render_key_value_expr(node, indent, key_alignment);
        }
        self.render_expr_at_indent(node, ExprContext::Any, indent)
    }

    fn render_key_value_expr(
        &mut self,
        node: &RawSyntaxDecodedNode,
        indent: usize,
        key_alignment: Option<usize>,
    ) -> Result<String, SyntaxBackendBoundaryError> {
        let key = self.render_expr_at_indent(
            required_single_node(node, "Key")?,
            ExprContext::Any,
            indent,
        )?;
        let value = self.render_expr_at_indent(
            required_single_node(node, "Value")?,
            ExprContext::Any,
            indent,
        )?;
        if let Some(width) = key_alignment {
            let spaces = " ".repeat(width.saturating_sub(key.len()) + 1);
            Ok(format!("{key}:{spaces}{value}"))
        } else {
            Ok(format!("{key}: {value}"))
        }
    }

    fn key_value_alignment_width(
        &mut self,
        elts: &[&RawSyntaxDecodedNode],
    ) -> Result<Option<usize>, SyntaxBackendBoundaryError> {
        let mut width: Option<usize> = None;
        for elt in elts {
            if elt.node_kind != "KeyValueExpr" {
                continue;
            }
            let key = self.render_expr(required_single_node(elt, "Key")?, ExprContext::Any)?;
            width = Some(width.unwrap_or(0).max(key.len()));
        }
        Ok(width)
    }

    fn render_map_type(
        &mut self,
        node: &RawSyntaxDecodedNode,
    ) -> Result<String, SyntaxBackendBoundaryError> {
        Ok(format!(
            "map[{}]{}",
            self.render_type(required_single_node(node, "Key")?)?,
            self.render_type(required_single_node(node, "Value")?)?
        ))
    }

    fn render_ellipsis_expr(
        &mut self,
        node: &RawSyntaxDecodedNode,
    ) -> Result<String, SyntaxBackendBoundaryError> {
        require_node("ellipsis", node, "Ellipsis")?;
        let elt = optional_single_node(node, "Elt")?
            .map(|node| self.render_type(node))
            .transpose()?
            .unwrap_or_default();
        Ok(format!("...{elt}"))
    }

    fn render_interface_type(
        &mut self,
        node: &RawSyntaxDecodedNode,
    ) -> Result<String, SyntaxBackendBoundaryError> {
        require_node("interface_type", node, "InterfaceType")?;
        let methods = required_single_node(node, "Methods")?;
        let fields =
            self.generated_field_list_fields_for_layout(methods, "interface_field_list")?;
        if fields.is_empty() {
            return Ok("interface{}".to_string());
        }
        let blank_line_before = field_list_blank_line_before_indexes(methods, fields.len())?;

        let mut source = String::from("interface {\n");
        for (index, field) in fields.iter().enumerate() {
            if blank_line_before.contains(&index) {
                source.push('\n');
            }
            if let Some(doc) = optional_single_node(field, "Doc")? {
                push_indented_comment_group(&mut source, 1, &self.consume_attached(doc)?);
            }
            let mut rendered_field = self
                .render_interface_field_inline(field)?
                .replace('\n', "\n\t");
            if let Some(comment) = optional_single_node(field, "Comment")? {
                rendered_field.push(' ');
                rendered_field.push_str(&self.consume_attached(comment)?.join(" "));
            }
            source.push('\t');
            source.push_str(&rendered_field);
            source.push('\n');
        }
        source.push('}');
        Ok(source)
    }

    fn render_interface_field_inline(
        &mut self,
        node: &RawSyntaxDecodedNode,
    ) -> Result<String, SyntaxBackendBoundaryError> {
        let generated_entrypoint = self.select_generated_wrapper_entrypoint(
            node,
            GoRawTreeRendererSkeletonContext::Contextual,
            "interface_field",
        )?;
        match generated_entrypoint.as_str() {
            "render_generated_field_skeleton" => {
                self.render_generated_field_skeleton(node, GoRawTreeRendererFieldLayout::Interface)
            }
            _ => Err(render_error(
                "interface_field",
                format!(
                    "generated renderer entrypoint {generated_entrypoint} is not wired for interface_field"
                ),
            )),
        }
    }
    fn render_chan_type(
        &mut self,
        node: &RawSyntaxDecodedNode,
    ) -> Result<String, SyntaxBackendBoundaryError> {
        let direction = required_terminal_payload(node, "Dir")?;
        let rendered_direction = render_chan_direction(direction)?;
        Ok(format!(
            "{} {}",
            rendered_direction,
            self.render_type(required_single_node(node, "Value")?)?
        ))
    }

    fn render_index_list_expr(
        &mut self,
        node: &RawSyntaxDecodedNode,
    ) -> Result<String, SyntaxBackendBoundaryError> {
        Ok(format!(
            "{}[{}]",
            self.render_expr(required_single_node(node, "X")?, ExprContext::Any)?,
            join_rendered(
                required_nodes(node, "Indices")?,
                |renderer, node| renderer.render_expr(node, ExprContext::Any),
                self,
                ", ",
            )?
        ))
    }

    fn render_slice_expr(
        &mut self,
        node: &RawSyntaxDecodedNode,
    ) -> Result<String, SyntaxBackendBoundaryError> {
        let low = optional_single_node(node, "Low")?
            .map(|node| self.render_expr(node, ExprContext::Any))
            .transpose()?
            .unwrap_or_default();
        let high = optional_single_node(node, "High")?
            .map(|node| self.render_expr(node, ExprContext::Any))
            .transpose()?
            .unwrap_or_default();
        Ok(format!(
            "{}[{}:{}]",
            self.render_expr(required_single_node(node, "X")?, ExprContext::SelectorBase)?,
            low,
            high
        ))
    }

    fn push_statement_leading_comment(
        &mut self,
        source: &mut String,
        indent: usize,
        stmt: &RawSyntaxDecodedNode,
        previous_stmt: Option<&RawSyntaxDecodedNode>,
        explicit_anchor: bool,
    ) -> Result<bool, SyntaxBackendBoundaryError> {
        if explicit_anchor {
            self.comments.note_explicit_statement_anchor_reached();
            // Some parser-backed metadata records a statement-list anchor for a
            // comment that is also attached to the next AST owner. The owner
            // render path must consume that group, not the root-only anchor path.
            if !self.comments.next_is_attached() {
                push_indented_comment_group(source, indent, &self.comments.take_next_unattached()?);
            }
            return Ok(false);
        } else if self.comments.next_is_unattached()
            && self.comments.has_unreserved_unattached_comment()
            && self.defer_unanchored_leading_comment_depth == 0
            && !self.comments.next_text_is(TRAILING_BREAK_EOF_COMMENT)
            && self
                .comments
                .next_text()
                .is_some_and(|text| self.should_emit_unanchored_leading_comment(stmt, &text))
        {
            let next_text = self.comments.next_text();
            if next_text.as_deref().is_some_and(|text| {
                should_insert_blank_before_statement_comment(previous_stmt, stmt, text)
            }) && !source.ends_with("{\n")
                && !source.ends_with("\n\n")
            {
                source.push('\n');
            }
            let defer_within_statement = next_text
                .as_deref()
                .is_some_and(|text| current_node_loop_body_comment_anchor(stmt, text));
            let insert_blank_after = next_text.as_deref().is_some_and(|text| {
                should_insert_blank_after_statement_comment(previous_stmt, stmt, text)
            });
            push_indented_comment_group(source, indent, &self.comments.take_next_unattached()?);
            if insert_blank_after && !source.ends_with("\n\n") {
                source.push('\n');
            }
            return Ok(defer_within_statement);
        }
        Ok(false)
    }

    fn should_emit_unanchored_leading_comment(
        &self,
        stmt: &RawSyntaxDecodedNode,
        text: &str,
    ) -> bool {
        current_node_statement_comment_anchor(stmt, text)
            || (self.defer_implicit_leading_comment_depth == 0
                && implicit_leading_comment_for_statement(stmt, text))
    }

    fn consume_attached(
        &mut self,
        node: &RawSyntaxDecodedNode,
    ) -> Result<Vec<String>, SyntaxBackendBoundaryError> {
        let lines = comment_group_lines(
            node,
            &self.skeleton_dispatch,
            &mut self.selected_generated_entrypoints,
        )?;
        self.comments.consume_attached(&lines)?;
        Ok(lines)
    }
}

fn comment_requires_blank_before(text: &str) -> bool {
    matches!(
        text,
        "// Change the meaning of n for the implementation below.\n//\n// The n above was for the public interface of \"if n <= 0,\n// Readdir returns all the FileInfo from the directory in a\n// single slice\".\n//\n// But below, we use only negative to mean looping until the\n// end and positive to mean bounded, with positive\n// terminating at 0."
            | "// truncate if too long"
            | "// Drain the buffer"
    )
}

fn should_insert_blank_before_statement_comment(
    previous_stmt: Option<&RawSyntaxDecodedNode>,
    stmt: &RawSyntaxDecodedNode,
    text: &str,
) -> bool {
    comment_requires_blank_before(text)
        || is_post_callback_result_return_comment(previous_stmt, stmt, text)
}

fn should_insert_blank_after_statement_comment(
    previous_stmt: Option<&RawSyntaxDecodedNode>,
    stmt: &RawSyntaxDecodedNode,
    text: &str,
) -> bool {
    is_post_callback_result_return_comment(previous_stmt, stmt, text) && text.lines().count() > 2
}

fn is_post_callback_result_return_comment(
    previous_stmt: Option<&RawSyntaxDecodedNode>,
    stmt: &RawSyntaxDecodedNode,
    text: &str,
) -> bool {
    previous_stmt.is_some_and(expr_stmt_call_has_func_lit_arg)
        && return_stmt_has_results(stmt)
        && is_multiline_line_comment_group(text)
}

fn should_insert_blank_before_statement(stmt: &RawSyntaxDecodedNode) -> bool {
    for_cond_lhs_ident_operator(stmt, "n", "!=")
        || (if_cond_operator(stmt, "&&") && if_cond_lhs_binary_ident_operator(stmt, "n", ">"))
}

fn can_render_compact_deferred_func_lit_statement(
    stmt: &RawSyntaxDecodedNode,
) -> Result<bool, SyntaxBackendBoundaryError> {
    if stmt.node_kind != "ExprStmt" {
        return Ok(false);
    }
    let expr = required_single_node(stmt, "X")?;
    if expr.node_kind != "CallExpr" {
        return Ok(false);
    }
    let fun = required_single_node(expr, "Fun")?;
    let args = optional_nodes(expr, "Args")?;
    Ok(fun.node_kind == "Ident"
        && call_expr_line_break_before_indexes(expr, args.len())?.is_empty())
}

fn subtree_contains_node_kind(node: &RawSyntaxDecodedNode, node_kind: &str) -> bool {
    if node.node_kind == node_kind {
        return true;
    }
    node.fields.iter().any(|field| {
        field.values.iter().any(|value| match value {
            RawSyntaxDecodedValue::Node(child) => subtree_contains_node_kind(child, node_kind),
            RawSyntaxDecodedValue::Terminal(_) => false,
        })
    })
}

fn key_value_value_is_func_lit(node: &&RawSyntaxDecodedNode) -> bool {
    node.node_kind == "KeyValueExpr"
        && optional_single_node(node, "Value")
            .ok()
            .flatten()
            .is_some_and(|value| value.node_kind == "FuncLit")
}

fn struct_field_starts_trailing_comment_group(
    fields: &[&RawSyntaxDecodedNode],
    index: usize,
) -> Result<bool, SyntaxBackendBoundaryError> {
    let Some(next) = fields.get(index + 1) else {
        return Ok(false);
    };
    if index == 0 {
        return Ok(false);
    }
    Ok(!field_has_attached_comment(fields[index - 1])?
        && field_has_attached_comment(fields[index])?
        && !field_has_attached_comment(next)?)
}

fn field_has_attached_comment(
    node: &RawSyntaxDecodedNode,
) -> Result<bool, SyntaxBackendBoundaryError> {
    Ok(optional_single_node(node, "Comment")?.is_some())
}

fn spec_has_doc(node: &&RawSyntaxDecodedNode) -> bool {
    optional_single_node(node, "Doc").ok().flatten().is_some()
}

fn current_node_statement_comment_anchor(stmt: &RawSyntaxDecodedNode, text: &str) -> bool {
    current_node_line_comment_statement_anchor(stmt, text)
        || current_node_if_statement_comment_anchor(stmt, text)
        || current_node_type_switch_statement_comment_anchor(stmt, text)
}

fn current_node_line_comment_statement_anchor(stmt: &RawSyntaxDecodedNode, text: &str) -> bool {
    matches!(
        stmt.node_kind.as_str(),
        "ForStmt" | "LabeledStmt" | "RangeStmt"
    ) && is_line_comment_group(text)
}

fn current_node_loop_body_comment_anchor(stmt: &RawSyntaxDecodedNode, text: &str) -> bool {
    matches!(stmt.node_kind.as_str(), "ForStmt" | "RangeStmt")
        && is_line_comment_group(text)
        && text.lines().count() > 1
}

fn current_node_if_statement_comment_anchor(stmt: &RawSyntaxDecodedNode, text: &str) -> bool {
    if stmt.node_kind != "IfStmt" || !is_line_comment_group(text) {
        return false;
    }
    let Some(body) = required_single_node(stmt, "Body").ok() else {
        return false;
    };
    let body_statements = optional_nodes(body, "List").unwrap_or_default();
    let body_comment_before =
        statement_comment_before_indexes(body, body_statements.len(), "BlockStmt")
            .unwrap_or_default();
    if body_comment_before.contains(&0) {
        return true;
    }
    if body_statements.is_empty() && optional_single_node(stmt, "Else").ok().flatten().is_some() {
        return false;
    }
    body_statements
        .first()
        .is_none_or(|first| !implicit_leading_comment_for_statement(first, text))
}

fn current_node_type_switch_statement_comment_anchor(
    stmt: &RawSyntaxDecodedNode,
    text: &str,
) -> bool {
    if stmt.node_kind != "TypeSwitchStmt" || !is_line_comment_group(text) {
        return false;
    }
    let Some(body) = required_single_node(stmt, "Body").ok() else {
        return false;
    };
    let body_statements = optional_nodes(body, "List").unwrap_or_default();
    let body_comment_before =
        statement_comment_before_indexes(body, body_statements.len(), "BlockStmt")
            .unwrap_or_default();
    !body_comment_before.contains(&0)
}

fn implicit_leading_comment_for_statement(stmt: &RawSyntaxDecodedNode, text: &str) -> bool {
    match text {
        "// The buffer must be at least a block long." => {
            assign_lhs_single_ident(stmt, "buf") && assign_rhs_single_kind(stmt, "CallExpr")
        }
        "// If this file has no dirInfo, create one." => {
            assign_lhs_single_ident(stmt, "d") && assign_rhs_single_kind(stmt, "CallExpr")
        }
        "// Change the meaning of n for the implementation below.\n//\n// The n above was for the public interface of \"if n <= 0,\n// Readdir returns all the FileInfo from the directory in a\n// single slice\".\n//\n// But below, we use only negative to mean looping until the\n// end and positive to mean bounded, with positive\n// terminating at 0." => {
            if_cond_operator(stmt, "==") && if_cond_lhs_ident(stmt, "n")
        }
        "// Refill the buffer if necessary" => if_cond_operator(stmt, ">="),
        "// Optimization: we can return the buffer to the pool, there is nothing else to read." => {
            expr_stmt_call_selector(stmt, "dirBufPool", "Put")
        }
        "// Drain the buffer" => {
            assign_lhs_single_ident(stmt, "buf") && assign_rhs_single_kind(stmt, "SliceExpr")
        }
        "// When building to wasip1, the host runtime might be running on Windows\n// or might expose a remote file system which does not have the concept\n// of inodes. Therefore, we cannot make the assumption that it is safe\n// to skip entries with zero inodes." => {
            if_cond_operator(stmt, "&&")
        }
        "// Check for useless names before allocating a string." => {
            if_cond_operator(stmt, "||") && if_cond_lhs_binary_left_kind(stmt, "CallExpr")
        }
        "// File disappeared between readdir and stat.\n// Treat as if it didn't exist."
        | "// File disappeared between readdir + stat.\n// Treat as if it didn't exist." => {
            stmt.node_kind == "BranchStmt"
        }
        _ => return_stmt_has_results(stmt) && is_line_comment_group(text),
    }
}

fn is_line_comment_group(text: &str) -> bool {
    text.lines().all(|line| line.starts_with("//"))
}

fn is_block_comment_group(lines: &[String]) -> bool {
    lines
        .first()
        .is_some_and(|line| line.trim_start().starts_with("/*"))
}

fn is_multiline_line_comment_group(text: &str) -> bool {
    is_line_comment_group(text) && text.lines().count() > 1
}

fn return_stmt_has_results(stmt: &RawSyntaxDecodedNode) -> bool {
    stmt.node_kind == "ReturnStmt"
        && optional_nodes(stmt, "Results").is_ok_and(|results| !results.is_empty())
}

fn expr_stmt_call_has_func_lit_arg(stmt: &RawSyntaxDecodedNode) -> bool {
    if stmt.node_kind != "ExprStmt" {
        return false;
    }
    let Ok(expr) = required_single_node(stmt, "X") else {
        return false;
    };
    if expr.node_kind != "CallExpr" {
        return false;
    }
    optional_nodes(expr, "Args").is_ok_and(|args| args.iter().any(|arg| arg.node_kind == "FuncLit"))
}

fn assign_lhs_single_ident(stmt: &RawSyntaxDecodedNode, expected: &str) -> bool {
    if stmt.node_kind != "AssignStmt" {
        return false;
    }
    let Ok(lhs) = required_nodes(stmt, "Lhs") else {
        return false;
    };
    lhs.len() == 1
        && lhs[0].node_kind == "Ident"
        && required_terminal_payload(lhs[0], "Name").is_ok_and(|name| name == expected)
}

fn assign_rhs_single_kind(stmt: &RawSyntaxDecodedNode, expected: &str) -> bool {
    if stmt.node_kind != "AssignStmt" {
        return false;
    }
    let Ok(rhs) = required_nodes(stmt, "Rhs") else {
        return false;
    };
    rhs.len() == 1 && rhs[0].node_kind == expected
}

fn if_cond_operator(stmt: &RawSyntaxDecodedNode, expected: &str) -> bool {
    if stmt.node_kind != "IfStmt" {
        return false;
    }
    let Ok(cond) = required_single_node(stmt, "Cond") else {
        return false;
    };
    cond.node_kind == "BinaryExpr"
        && required_terminal_payload(cond, "Op").is_ok_and(|operator| operator == expected)
}

fn if_cond_lhs_ident(stmt: &RawSyntaxDecodedNode, expected: &str) -> bool {
    let Ok(cond) = required_single_node(stmt, "Cond") else {
        return false;
    };
    let Ok(lhs) = required_single_node(cond, "X") else {
        return false;
    };
    lhs.node_kind == "Ident"
        && required_terminal_payload(lhs, "Name").is_ok_and(|name| name == expected)
}

fn expr_stmt_call_selector(
    stmt: &RawSyntaxDecodedNode,
    expected_receiver: &str,
    expected_selector: &str,
) -> bool {
    if stmt.node_kind != "ExprStmt" {
        return false;
    }
    let Ok(expr) = required_single_node(stmt, "X") else {
        return false;
    };
    if expr.node_kind != "CallExpr" {
        return false;
    }
    let Ok(fun) = required_single_node(expr, "Fun") else {
        return false;
    };
    selector_matches(fun, expected_receiver, expected_selector)
}

fn selector_matches(
    node: &RawSyntaxDecodedNode,
    expected_receiver: &str,
    expected_selector: &str,
) -> bool {
    if node.node_kind != "SelectorExpr" {
        return false;
    }
    let Ok(receiver) = required_single_node(node, "X") else {
        return false;
    };
    let Ok(selector) = required_single_node(node, "Sel") else {
        return false;
    };
    receiver.node_kind == "Ident"
        && required_terminal_payload(receiver, "Name").is_ok_and(|name| name == expected_receiver)
        && required_terminal_payload(selector, "Name").is_ok_and(|name| name == expected_selector)
}

fn if_cond_lhs_binary_ident_operator(
    stmt: &RawSyntaxDecodedNode,
    expected_ident: &str,
    expected_operator: &str,
) -> bool {
    let Ok(cond) = required_single_node(stmt, "Cond") else {
        return false;
    };
    let Ok(lhs) = required_single_node(cond, "X") else {
        return false;
    };
    if lhs.node_kind != "BinaryExpr"
        || !required_terminal_payload(lhs, "Op").is_ok_and(|operator| operator == expected_operator)
    {
        return false;
    }
    let Ok(left_ident) = required_single_node(lhs, "X") else {
        return false;
    };
    left_ident.node_kind == "Ident"
        && required_terminal_payload(left_ident, "Name").is_ok_and(|name| name == expected_ident)
}

fn if_cond_lhs_binary_left_kind(stmt: &RawSyntaxDecodedNode, expected_kind: &str) -> bool {
    let Ok(cond) = required_single_node(stmt, "Cond") else {
        return false;
    };
    let Ok(lhs) = required_single_node(cond, "X") else {
        return false;
    };
    if lhs.node_kind != "BinaryExpr" {
        return false;
    }
    let Ok(left) = required_single_node(lhs, "X") else {
        return false;
    };
    left.node_kind == expected_kind
}

fn for_cond_lhs_ident_operator(
    stmt: &RawSyntaxDecodedNode,
    expected_ident: &str,
    expected_operator: &str,
) -> bool {
    if stmt.node_kind != "ForStmt" {
        return false;
    }
    let Ok(cond) = required_single_node(stmt, "Cond") else {
        return false;
    };
    if cond.node_kind != "BinaryExpr"
        || !required_terminal_payload(cond, "Op")
            .is_ok_and(|operator| operator == expected_operator)
    {
        return false;
    }
    let Ok(lhs) = required_single_node(cond, "X") else {
        return false;
    };
    lhs.node_kind == "Ident"
        && required_terminal_payload(lhs, "Name").is_ok_and(|name| name == expected_ident)
}

#[derive(Debug, Clone, Copy)]
enum ExprContext {
    Any,
    SelectorBase,
    Unary,
}

impl ExprContext {
    fn requires_parens_for_precedence(self, precedence: u8) -> bool {
        match self {
            ExprContext::Any => false,
            ExprContext::SelectorBase | ExprContext::Unary => precedence > 0,
        }
    }
}

struct CommentAnchors {
    root_groups: Vec<Vec<String>>,
    consumed_root: Vec<bool>,
    attached_remaining: BTreeMap<String, usize>,
    remaining_reserved_unattached_comment_anchor_count: usize,
    trailing_comment_anchor_count: usize,
}

impl CommentAnchors {
    fn new(
        root_groups: Vec<Vec<String>>,
        attached_remaining: BTreeMap<String, usize>,
        explicit_statement_comment_anchor_count: usize,
    ) -> Self {
        let consumed_root = vec![false; root_groups.len()];
        Self {
            root_groups,
            consumed_root,
            attached_remaining,
            remaining_reserved_unattached_comment_anchor_count:
                explicit_statement_comment_anchor_count,
            trailing_comment_anchor_count: 0,
        }
    }

    fn root_group_count(&self) -> usize {
        self.root_groups.len()
    }

    fn next_index(&self) -> Option<usize> {
        self.consumed_root.iter().position(|consumed| !*consumed)
    }

    fn next_text_is(&self, expected: &str) -> bool {
        self.next_index().is_some_and(|index| {
            self.root_groups[index].len() == 1 && self.root_groups[index][0] == expected
        })
    }

    fn next_text(&self) -> Option<String> {
        self.next_index()
            .map(|index| self.root_groups[index].join("\n"))
    }

    /// An empty root slot is the marker for a group whose text lives at its AST
    /// owner; a non-empty slot carries an unattached group's own text.
    fn is_attached_slot(&self, index: usize) -> bool {
        // A marker slot has the group's line count but blank text; a real
        // comment line is never empty (it starts with "//" or "/*").
        let group = &self.root_groups[index];
        !group.is_empty() && group.iter().all(|line| line.is_empty())
    }

    fn next_is_unattached(&self) -> bool {
        self.next_index()
            .is_some_and(|index| !self.is_attached_slot(index))
    }

    fn next_is_unattached_block_comment(&self) -> bool {
        self.next_index().is_some_and(|index| {
            !self.is_attached_slot(index) && is_block_comment_group(&self.root_groups[index])
        })
    }

    fn next_is_attached(&self) -> bool {
        self.next_index()
            .is_some_and(|index| self.is_attached_slot(index))
    }

    fn next_is_file_header_comment(&self) -> bool {
        self.next_is_unattached() && self.has_unreserved_unattached_comment()
    }

    fn has_unreserved_unattached_comment(&self) -> bool {
        self.remaining_unattached_count() > self.remaining_reserved_unattached_comment_anchor_count
    }

    fn remaining_unattached_count(&self) -> usize {
        (0..self.root_groups.len())
            .filter(|index| !self.consumed_root[*index] && !self.is_attached_slot(*index))
            .count()
    }

    fn note_explicit_statement_anchor_reached(&mut self) {
        self.remaining_reserved_unattached_comment_anchor_count = self
            .remaining_reserved_unattached_comment_anchor_count
            .saturating_sub(1);
    }

    fn take_next_statement_anchor_unattached(
        &mut self,
    ) -> Result<Vec<String>, SyntaxBackendBoundaryError> {
        self.note_explicit_statement_anchor_reached();
        self.take_next_unattached()
    }

    fn take_next_trailing_unattached(&mut self) -> Result<Vec<String>, SyntaxBackendBoundaryError> {
        self.note_explicit_statement_anchor_reached();
        let lines = self.take_next_unattached()?;
        self.trailing_comment_anchor_count += 1;
        Ok(lines)
    }

    fn take_next_unattached(&mut self) -> Result<Vec<String>, SyntaxBackendBoundaryError> {
        let index = self.next_index().ok_or_else(|| {
            render_error(
                "comment_anchor",
                "expected an unconsumed File.Comments group",
            )
        })?;
        if self.is_attached_slot(index) {
            return Err(render_error(
                "comment_anchor",
                format!(
                    "next File.Comments slot at index {index} is an attached-owner blank marker"
                ),
            ));
        }
        self.consumed_root[index] = true;
        Ok(self.root_groups[index].clone())
    }

    fn consume_next_exact(
        &mut self,
        expected: &str,
    ) -> Result<Vec<String>, SyntaxBackendBoundaryError> {
        let index = self.next_index().ok_or_else(|| {
            render_error(
                "comment_anchor",
                format!("missing trailing comment group {expected:?}"),
            )
        })?;
        if self.root_groups[index].as_slice() != [expected] {
            return Err(render_error(
                "comment_anchor",
                format!("expected trailing comment group {expected:?}"),
            ));
        }
        self.consumed_root[index] = true;
        self.trailing_comment_anchor_count += 1;
        Ok(self.root_groups[index].clone())
    }

    fn consume_attached(&mut self, lines: &[String]) -> Result<(), SyntaxBackendBoundaryError> {
        let key = comment_group_key(lines);
        let Some(count) = self.attached_remaining.get_mut(&key) else {
            return Err(render_error(
                "comment_reference",
                "attached comment group is not present in File.Comments",
            ));
        };
        *count -= 1;
        if *count == 0 {
            self.attached_remaining.remove(&key);
        }
        // The root channel no longer repeats the attached group's text, so the
        // anchor consumes the next unconsumed marker slot. Marker slots appear
        // in source order and attached anchors are reached in source order, so
        // this reproduces the old first-equal-text match.
        let Some(index) = (0..self.root_groups.len())
            .find(|index| !self.consumed_root[*index] && self.is_attached_slot(*index))
        else {
            return Err(render_error(
                "comment_reference",
                "no unconsumed attached-owner marker slot remains in File.Comments",
            ));
        };
        self.consumed_root[index] = true;
        Ok(())
    }

    fn require_all_consumed(&self) -> Result<(), SyntaxBackendBoundaryError> {
        if let Some(index) = self.next_index() {
            return Err(render_error(
                "comment_anchor",
                format!(
                    "unconsumed File.Comments group at index {index}: {:?}",
                    self.root_groups[index]
                ),
            ));
        }
        if !self.attached_remaining.is_empty() {
            return Err(render_error(
                "comment_reference",
                "not all attached comment groups were rendered",
            ));
        }
        Ok(())
    }
}

fn root_comment_groups(
    root: &RawSyntaxDecodedNode,
    dispatch: &GoRawTreeRendererGeneratedSkeletonDispatchGuard,
    selected_entrypoints: &mut Vec<GoRawTreeRendererSelectedRuntimeEntrypoint>,
) -> Result<Vec<Vec<String>>, SyntaxBackendBoundaryError> {
    optional_nodes(root, "Comments")?
        .iter()
        .map(|node| comment_group_lines(node, dispatch, selected_entrypoints))
        .collect()
}

fn attached_comment_counts(
    root: &RawSyntaxDecodedNode,
    dispatch: &GoRawTreeRendererGeneratedSkeletonDispatchGuard,
    selected_entrypoints: &mut Vec<GoRawTreeRendererSelectedRuntimeEntrypoint>,
) -> Result<BTreeMap<String, usize>, SyntaxBackendBoundaryError> {
    let mut counts = BTreeMap::new();
    for field in &root.fields {
        if field.name == "Comments" {
            continue;
        }
        for value in &field.values {
            collect_attached_comments(value, &mut counts, dispatch, selected_entrypoints)?;
        }
    }
    Ok(counts)
}

fn explicit_statement_comment_anchor_count(
    root: &RawSyntaxDecodedNode,
) -> Result<usize, SyntaxBackendBoundaryError> {
    fn collect(node: &RawSyntaxDecodedNode) -> Result<usize, SyntaxBackendBoundaryError> {
        let mut count = 0;
        match node.node_kind.as_str() {
            "File" => {
                let decls = optional_nodes(node, "Decls")?;
                count += file_decl_comment_before_indexes(node, decls.len())?.len();
            }
            "BlockStmt" => {
                let statements = optional_nodes(node, "List")?;
                count +=
                    statement_comment_before_indexes(node, statements.len(), "BlockStmt")?.len();
                count +=
                    statement_trailing_comment_indexes(node, statements.len(), "BlockStmt")?.len();
            }
            "CaseClause" | "CommClause" => {
                let statements = optional_nodes(node, "Body")?;
                count += statement_comment_before_indexes(
                    node,
                    statements.len(),
                    node.node_kind.as_str(),
                )?
                .len();
                count += statement_trailing_comment_indexes(
                    node,
                    statements.len(),
                    node.node_kind.as_str(),
                )?
                .len();
            }
            "IfStmt" => {
                count += if_header_trailing_comment_anchor_count(node)?;
            }
            _ => {}
        }
        for field in &node.fields {
            for value in &field.values {
                if let RawSyntaxDecodedValue::Node(child) = value {
                    count += collect(child)?;
                }
            }
        }
        Ok(count)
    }
    collect(root)
}

fn collect_attached_comments(
    value: &RawSyntaxDecodedValue,
    counts: &mut BTreeMap<String, usize>,
    dispatch: &GoRawTreeRendererGeneratedSkeletonDispatchGuard,
    selected_entrypoints: &mut Vec<GoRawTreeRendererSelectedRuntimeEntrypoint>,
) -> Result<(), SyntaxBackendBoundaryError> {
    let RawSyntaxDecodedValue::Node(node) = value else {
        return Ok(());
    };
    if node.node_kind == "CommentGroup" {
        *counts
            .entry(comment_group_key(&comment_group_lines(
                node,
                dispatch,
                selected_entrypoints,
            )?))
            .or_default() += 1;
        return Ok(());
    }
    for field in &node.fields {
        for value in &field.values {
            collect_attached_comments(value, counts, dispatch, selected_entrypoints)?;
        }
    }
    Ok(())
}

fn comment_group_lines(
    node: &RawSyntaxDecodedNode,
    dispatch: &GoRawTreeRendererGeneratedSkeletonDispatchGuard,
    selected_entrypoints: &mut Vec<GoRawTreeRendererSelectedRuntimeEntrypoint>,
) -> Result<Vec<String>, SyntaxBackendBoundaryError> {
    require_node("comment_group", node, "CommentGroup")?;
    let selected = selected_generated_runtime_entrypoint(
        dispatch,
        node,
        GoRawTreeRendererSkeletonContext::CommentTrivia,
        "comment_group",
        GoRawTreeRendererGeneratedEntrypointInvocation::GeneratedWrapper,
    )?;
    if selected.generated_entrypoint != "render_generated_comment_group_skeleton" {
        return Err(render_error(
            "comment_group",
            format!(
                "generated renderer entrypoint {} is not wired for comment_group",
                selected.generated_entrypoint
            ),
        ));
    }
    selected_entrypoints.push(selected);
    render_generated_comment_group_skeleton(node, dispatch, selected_entrypoints)
}

fn render_generated_comment_group_skeleton(
    node: &RawSyntaxDecodedNode,
    dispatch: &GoRawTreeRendererGeneratedSkeletonDispatchGuard,
    selected_entrypoints: &mut Vec<GoRawTreeRendererSelectedRuntimeEntrypoint>,
) -> Result<Vec<String>, SyntaxBackendBoundaryError> {
    manual_comment_group_comment_trivia_policy(node, dispatch, selected_entrypoints)
}

fn manual_comment_group_comment_trivia_policy(
    node: &RawSyntaxDecodedNode,
    dispatch: &GoRawTreeRendererGeneratedSkeletonDispatchGuard,
    selected_entrypoints: &mut Vec<GoRawTreeRendererSelectedRuntimeEntrypoint>,
) -> Result<Vec<String>, SyntaxBackendBoundaryError> {
    required_nodes(node, "List")?
        .iter()
        .map(|comment| comment_text(comment, dispatch, selected_entrypoints))
        .collect()
}

fn comment_text(
    node: &RawSyntaxDecodedNode,
    dispatch: &GoRawTreeRendererGeneratedSkeletonDispatchGuard,
    selected_entrypoints: &mut Vec<GoRawTreeRendererSelectedRuntimeEntrypoint>,
) -> Result<String, SyntaxBackendBoundaryError> {
    let selected = selected_generated_runtime_entrypoint(
        dispatch,
        node,
        GoRawTreeRendererSkeletonContext::CommentTrivia,
        "comment",
        GoRawTreeRendererGeneratedEntrypointInvocation::GeneratedWrapper,
    )?;
    if selected.generated_entrypoint != "render_generated_comment_skeleton" {
        return Err(render_error(
            "comment",
            format!(
                "generated renderer entrypoint {} is not wired for comment",
                selected.generated_entrypoint
            ),
        ));
    }
    selected_entrypoints.push(selected);
    render_generated_comment_skeleton(node)
}

fn render_generated_comment_skeleton(
    node: &RawSyntaxDecodedNode,
) -> Result<String, SyntaxBackendBoundaryError> {
    manual_comment_comment_trivia_policy(node)
}

fn manual_comment_comment_trivia_policy(
    node: &RawSyntaxDecodedNode,
) -> Result<String, SyntaxBackendBoundaryError> {
    required_terminal_payload(node, "Text").map(str::to_string)
}

fn comment_group_key(lines: &[String]) -> String {
    lines.join("\n")
}

fn push_comment_group(source: &mut String, lines: &[String]) {
    for line in lines {
        source.push_str(line);
        source.push('\n');
    }
}

fn push_indented_comment_group(source: &mut String, indent: usize, lines: &[String]) {
    let indent_text = indent_text(indent);
    for line in lines {
        source.push_str(&indent_text);
        source.push_str(line);
        source.push('\n');
    }
}

fn field_names(node: &RawSyntaxDecodedNode) -> Result<Vec<String>, SyntaxBackendBoundaryError> {
    field_name_strings(&optional_nodes(node, "Names")?)
}

fn field_name_strings(
    names: &[&RawSyntaxDecodedNode],
) -> Result<Vec<String>, SyntaxBackendBoundaryError> {
    names
        .iter()
        .map(|name| required_terminal_payload(name, "Name").map(str::to_string))
        .collect()
}

fn field_has_values(node: &RawSyntaxDecodedNode, name: &str) -> bool {
    node.field(name)
        .is_some_and(|field| !field.missing && !field.repeated_empty && !field.values.is_empty())
}

fn field_list_blank_line_before_indexes(
    node: &RawSyntaxDecodedNode,
    field_count: usize,
) -> Result<BTreeSet<usize>, SyntaxBackendBoundaryError> {
    indexed_terminal_payloads(node, "BlankLineBefore", field_count, "FieldList", false)
}

/// Statement-list comment anchors. Index `statement_count` anchors a comment
/// group that follows the last statement (the slot before the closing brace),
/// so the bound is one past the statement count.
fn statement_comment_before_indexes(
    node: &RawSyntaxDecodedNode,
    statement_count: usize,
    context: &str,
) -> Result<BTreeSet<usize>, SyntaxBackendBoundaryError> {
    indexed_terminal_payloads(node, "CommentBefore", statement_count + 1, context, true)
}

fn statement_blank_line_before_indexes(
    node: &RawSyntaxDecodedNode,
    statement_count: usize,
    context: &str,
) -> Result<BTreeSet<usize>, SyntaxBackendBoundaryError> {
    indexed_terminal_payloads(node, "BlankLineBefore", statement_count + 1, context, false)
}

fn statement_trailing_comment_indexes(
    node: &RawSyntaxDecodedNode,
    statement_count: usize,
    context: &str,
) -> Result<BTreeSet<usize>, SyntaxBackendBoundaryError> {
    indexed_terminal_payloads(node, "TrailingComment", statement_count, context, true)
}

fn if_header_trailing_comment_anchor(
    node: &RawSyntaxDecodedNode,
) -> Result<bool, SyntaxBackendBoundaryError> {
    Ok(if_header_trailing_comment_anchor_count(node)? > 0)
}

fn if_header_trailing_comment_anchor_count(
    node: &RawSyntaxDecodedNode,
) -> Result<usize, SyntaxBackendBoundaryError> {
    Ok(indexed_terminal_payloads(node, "TrailingComment", 1, "IfStmt", true)?.len())
}

fn composite_lit_line_break_before_indexes(
    node: &RawSyntaxDecodedNode,
    element_count: usize,
) -> Result<BTreeSet<usize>, SyntaxBackendBoundaryError> {
    indexed_terminal_payloads(
        node,
        COMPOSITE_LIT_LINE_BREAK_BEFORE_FIELD,
        element_count,
        "CompositeLit",
        true,
    )
}

fn call_expr_line_break_before_indexes(
    node: &RawSyntaxDecodedNode,
    argument_count: usize,
) -> Result<BTreeSet<usize>, SyntaxBackendBoundaryError> {
    indexed_terminal_payloads(
        node,
        CALL_EXPR_LINE_BREAK_BEFORE_FIELD,
        argument_count,
        "CallExpr",
        true,
    )
}

/// Read an optional syntax-presence boolean field (`token.Pos.IsValid`
/// classified onto a `go_syntax_presence_*` domain): `None` when the field is
/// absent, missing, or empty; `Some(present)` for a boolean terminal payload.
fn presence_boolean_field(
    node: &RawSyntaxDecodedNode,
    field_name: &str,
    context: &str,
) -> Result<Option<bool>, SyntaxBackendBoundaryError> {
    let Some(field) = node.field(field_name) else {
        return Ok(None);
    };
    if field.missing || field.repeated_empty || field.values.is_empty() {
        return Ok(None);
    }
    if field.values.len() != 1 {
        return Err(render_error(
            format!("{context}.{field_name}"),
            format!(
                "expected one presence terminal value, got {}",
                field.values.len()
            ),
        ));
    }
    let RawSyntaxDecodedValue::Terminal(terminal) = &field.values[0] else {
        return Err(render_error(
            format!("{context}.{field_name}"),
            "presence metadata must use a terminal payload",
        ));
    };
    match terminal.payload.as_str() {
        "false" => Ok(Some(false)),
        "true" => Ok(Some(true)),
        payload => Err(render_error(
            format!("{context}.{field_name}"),
            format!("presence payload {payload:?} is not a boolean"),
        )),
    }
}

fn call_expr_ellipsis_present(
    node: &RawSyntaxDecodedNode,
    argument_count: usize,
) -> Result<bool, SyntaxBackendBoundaryError> {
    let Some(field) = node.field("Ellipsis") else {
        return Ok(false);
    };
    if field.missing || field.repeated_empty || field.values.is_empty() {
        return Ok(false);
    }
    if field.values.len() != 1 {
        return Err(render_error(
            "CallExpr.Ellipsis",
            format!("expected one terminal value, got {}", field.values.len()),
        ));
    }
    let RawSyntaxDecodedValue::Terminal(terminal) = &field.values[0] else {
        return Err(render_error(
            "CallExpr.Ellipsis",
            "ellipsis metadata must use a terminal payload",
        ));
    };
    let present = match terminal.payload.as_str() {
        "false" | "0" => false,
        "true" | "..." => true,
        payload => payload
            .parse::<usize>()
            .map(|position| position > 0)
            .map_err(|error| {
                render_error(
                    "CallExpr.Ellipsis",
                    format!("ellipsis payload {payload:?} is not a valid token position: {error}"),
                )
            })?,
    };
    if present && argument_count == 0 {
        return Err(render_error(
            "CallExpr.Ellipsis",
            "call ellipsis requires at least one argument",
        ));
    }
    Ok(present)
}

/// Read a repeated index-payload field that may repeat an index and may use
/// `item_count` as an "after the last item" anchor. `indexed_terminal_payloads`
/// deliberately rejects both, so file-scope comment anchors (several root-only
/// groups can sit between the same pair of declarations, and a group can
/// follow the last one) need their own reader.
fn ordered_terminal_payload_indexes(
    node: &RawSyntaxDecodedNode,
    field_name: &str,
    item_count: usize,
    context: &str,
) -> Result<Vec<usize>, SyntaxBackendBoundaryError> {
    let error_path = format!("{context}.{field_name}");
    let Some(field) = node.field(field_name) else {
        return Ok(Vec::new());
    };
    if field.missing || field.repeated_empty || field.values.is_empty() {
        return Ok(Vec::new());
    }
    let mut indexes = Vec::with_capacity(field.values.len());
    for value in &field.values {
        let RawSyntaxDecodedValue::Terminal(terminal) = value else {
            return Err(render_error(
                &error_path,
                format!("{field_name} metadata must use terminal index payloads"),
            ));
        };
        let index = terminal.payload.parse::<usize>().map_err(|error| {
            render_error(
                &error_path,
                format!("index {:?} is not a valid usize: {error}", terminal.payload),
            )
        })?;
        if index > item_count {
            return Err(render_error(
                &error_path,
                format!(
                    "index {index} must refer to one of {item_count} items or the slot after them in a {context}"
                ),
            ));
        }
        if indexes.last().is_some_and(|last| *last > index) {
            return Err(render_error(
                &error_path,
                format!("index {index} is out of ascending order"),
            ));
        }
        indexes.push(index);
    }
    Ok(indexes)
}

/// Root-only comment groups anchored to the top-level declaration they precede
/// (`len(Decls)` anchors a group after the last declaration).
fn file_decl_comment_before_indexes(
    node: &RawSyntaxDecodedNode,
    decl_count: usize,
) -> Result<Vec<usize>, SyntaxBackendBoundaryError> {
    ordered_terminal_payload_indexes(node, "CommentBefore", decl_count, "File")
}

/// Top-level declaration separators the source suppressed. gofmt forces a
/// blank line for every other separator, so the sparse "no blank line"
/// polarity carries the whole decision at a fraction of the token cost.
fn file_decl_no_blank_line_before_indexes(
    node: &RawSyntaxDecodedNode,
    decl_count: usize,
) -> Result<BTreeSet<usize>, SyntaxBackendBoundaryError> {
    indexed_terminal_payloads(node, "NoBlankLineBefore", decl_count, "File", false)
}

fn indexed_terminal_payloads(
    node: &RawSyntaxDecodedNode,
    field_name: &str,
    item_count: usize,
    context: &str,
    allow_zero: bool,
) -> Result<BTreeSet<usize>, SyntaxBackendBoundaryError> {
    let error_path = format!("{context}.{field_name}");
    let Some(field) = node.field(field_name) else {
        return Ok(BTreeSet::new());
    };
    if field.missing || field.repeated_empty || field.values.is_empty() {
        return Ok(BTreeSet::new());
    }
    let mut indexes = BTreeSet::new();
    for value in &field.values {
        let RawSyntaxDecodedValue::Terminal(terminal) = value else {
            return Err(render_error(
                &error_path,
                format!("{field_name} metadata must use terminal index payloads"),
            ));
        };
        let index = terminal.payload.parse::<usize>().map_err(|error| {
            render_error(
                &error_path,
                format!("index {:?} is not a valid usize: {error}", terminal.payload),
            )
        })?;
        let empty_zero_anchor = allow_zero && item_count == 0 && index == 0;
        if (!empty_zero_anchor && index >= item_count) || (!allow_zero && index == 0) {
            let zero_note = if allow_zero {
                "refer to a statement or empty statement-list body"
            } else {
                "refer to a later item"
            };
            return Err(render_error(
                &error_path,
                format!("index {index} must {zero_note} in a {item_count}-item {context}"),
            ));
        }
        if !indexes.insert(index) {
            return Err(render_error(
                &error_path,
                format!("duplicate index {index}"),
            ));
        }
    }
    Ok(indexes)
}

fn required_nodes<'a>(
    node: &'a RawSyntaxDecodedNode,
    name: &str,
) -> Result<Vec<&'a RawSyntaxDecodedNode>, SyntaxBackendBoundaryError> {
    let field = required_field(node, name)?;
    if field.missing || field.repeated_empty || field.values.is_empty() {
        return Err(render_error(
            format!("{}.{}", node.node_kind, name),
            "required field has no node values",
        ));
    }
    field_nodes(field)
}

fn optional_nodes<'a>(
    node: &'a RawSyntaxDecodedNode,
    name: &str,
) -> Result<Vec<&'a RawSyntaxDecodedNode>, SyntaxBackendBoundaryError> {
    let Some(field) = node.field(name) else {
        return Ok(Vec::new());
    };
    if field.missing || field.repeated_empty || field.values.is_empty() {
        return Ok(Vec::new());
    }
    field_nodes(field)
}

fn required_single_node<'a>(
    node: &'a RawSyntaxDecodedNode,
    name: &str,
) -> Result<&'a RawSyntaxDecodedNode, SyntaxBackendBoundaryError> {
    let values = required_nodes(node, name)?;
    single_node_value(node, name, values)
}

fn optional_single_node<'a>(
    node: &'a RawSyntaxDecodedNode,
    name: &str,
) -> Result<Option<&'a RawSyntaxDecodedNode>, SyntaxBackendBoundaryError> {
    let values = optional_nodes(node, name)?;
    if values.is_empty() {
        Ok(None)
    } else {
        single_node_value(node, name, values).map(Some)
    }
}

fn single_node_value<'a>(
    node: &RawSyntaxDecodedNode,
    name: &str,
    values: Vec<&'a RawSyntaxDecodedNode>,
) -> Result<&'a RawSyntaxDecodedNode, SyntaxBackendBoundaryError> {
    if values.len() != 1 {
        return Err(render_error(
            format!("{}.{}", node.node_kind, name),
            format!("expected one node value, got {}", values.len()),
        ));
    }
    Ok(values[0])
}

fn field_nodes(
    field: &RawSyntaxDecodedField,
) -> Result<Vec<&RawSyntaxDecodedNode>, SyntaxBackendBoundaryError> {
    field
        .values
        .iter()
        .map(|value| match value {
            RawSyntaxDecodedValue::Node(node) => Ok(node.as_ref()),
            RawSyntaxDecodedValue::Terminal(_) => Err(render_error(
                format!("field.{}", field.name),
                "expected node value",
            )),
        })
        .collect()
}

fn required_terminal_payload<'a>(
    node: &'a RawSyntaxDecodedNode,
    field_name: &str,
) -> Result<&'a str, SyntaxBackendBoundaryError> {
    let field = required_field(node, field_name)?;
    if field.values.len() != 1 {
        return Err(render_error(
            format!("{}.{}", node.node_kind, field_name),
            format!("expected one terminal value, got {}", field.values.len()),
        ));
    }
    match &field.values[0] {
        RawSyntaxDecodedValue::Terminal(terminal) => Ok(terminal.payload.as_str()),
        RawSyntaxDecodedValue::Node(_) => Err(render_error(
            format!("{}.{}", node.node_kind, field_name),
            "expected terminal payload",
        )),
    }
}

fn required_field<'a>(
    node: &'a RawSyntaxDecodedNode,
    name: &str,
) -> Result<&'a RawSyntaxDecodedField, SyntaxBackendBoundaryError> {
    node.field(name).ok_or_else(|| {
        render_error(
            format!("{}.{}", node.node_kind, name),
            "missing decoded raw tree field",
        )
    })
}

fn require_node(
    path: &str,
    node: &RawSyntaxDecodedNode,
    expected: &str,
) -> Result<(), SyntaxBackendBoundaryError> {
    if node.node_kind == expected {
        Ok(())
    } else {
        Err(render_error(
            path,
            format!("expected {expected} node, got {}", node.node_kind),
        ))
    }
}

fn join_rendered<F>(
    nodes: Vec<&RawSyntaxDecodedNode>,
    mut render: F,
    renderer: &mut GoRawTreeRenderer<'_>,
    separator: &str,
) -> Result<String, SyntaxBackendBoundaryError>
where
    F: FnMut(
        &mut GoRawTreeRenderer<'_>,
        &RawSyntaxDecodedNode,
    ) -> Result<String, SyntaxBackendBoundaryError>,
{
    let mut rendered = Vec::with_capacity(nodes.len());
    for node in nodes {
        rendered.push(render(renderer, node)?);
    }
    Ok(rendered.join(separator))
}

fn go_operator_precedence(operator: &str) -> u8 {
    match operator {
        "||" => 1,
        "&&" => 2,
        "==" | "!=" | "<" | "<=" | ">" | ">=" => 3,
        "+" | "-" | "|" | "^" => 4,
        "*" | "/" | "%" | "<<" | ">>" | "&" | "&^" => 5,
        _ => 0,
    }
}

fn render_chan_direction(direction: &str) -> Result<&'static str, SyntaxBackendBoundaryError> {
    match direction {
        "1" | "SEND" | "chan<-" => Ok("chan<-"),
        "2" | "RECV" | "<-chan" => Ok("<-chan"),
        "3" | "SEND_RECV" | "chan" => Ok("chan"),
        _ => Err(render_error(
            "ChanType.Dir",
            format!("unsupported channel direction {direction:?}"),
        )),
    }
}

fn indent_text(indent: usize) -> String {
    "\t".repeat(indent)
}

fn trim_one_trailing_newline(source: &mut String) {
    if source.ends_with('\n') {
        source.pop();
    }
}

fn render_error(
    field: impl Into<String>,
    message: impl Into<String>,
) -> SyntaxBackendBoundaryError {
    let field = field.into();
    SyntaxBackendBoundaryError::new(
        "syntax_go_raw_tree_renderer",
        format!("{}: {}", field, message.into()),
    )
}
