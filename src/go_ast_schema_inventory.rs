//! Checked `go/ast` schema inventory adapter for Go syntax generation.
//!
//! The inventory is parser-side evidence for the Go tokenizer-generator
//! workstream. It lowers a pinned `go/ast` surface into the existing GrammarIR
//! structs without routing syntax metadata through the public JSON compiler.

use crate::grammar_ir::{
    GrammarField, GrammarFieldAlternative, GrammarIr, GrammarIrProvenance, GrammarParserProvenance,
    GrammarRule, GrammarTerminalKind,
};
use crate::provenance::{file_provenance, sha256_hex};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;
use std::path::Path;
use std::sync::OnceLock;

mod grammar_coverage;

pub use grammar_coverage::{
    GO_AST_SCHEMA_GRAMMAR_COVERAGE_ARTIFACT_KIND,
    GO_AST_SCHEMA_GRAMMAR_COVERAGE_BLOCKER_NEXT_READY_TASK,
    GO_AST_SCHEMA_GRAMMAR_COVERAGE_CLAIM_BOUNDARY, GO_AST_SCHEMA_GRAMMAR_COVERAGE_NEXT_READY_TASK,
    GO_AST_SCHEMA_GRAMMAR_COVERAGE_STATUS_BLOCKED, GO_AST_SCHEMA_GRAMMAR_COVERAGE_STATUS_READY,
    GoAstSchemaGrammarCoverageBlocker, GoAstSchemaGrammarCoverageRawStateMachine,
    GoAstSchemaGrammarCoverageReport, GoAstSchemaGrammarCoverageTokenLayout,
};

pub const GO_AST_SCHEMA_INVENTORY_ARTIFACT_KIND: &str = "tct_go_ast_schema_inventory_v1";
pub const GO_AST_SCHEMA_INVENTORY_VERSION: u32 = 1;
pub const GO_AST_SCHEMA_INVENTORY_PINNED_GO_VERSION: &str = "go1.24.3";
pub const GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH: &str =
    "tests/fixtures/go_ast_schema_inventory/go1.24.3-ast-schema-inventory-v1.json";
pub const GO_STRUCTURAL_TOKEN_ASSIGNMENT_VERSION: u32 = 1;
pub const GO_STRUCTURAL_TOKEN_ASSIGNMENT_APPEND_RULE: &str =
    "new node kinds append at max assigned token id + 1; existing ids never renumber";
pub const GO_AST_SCHEMA_INVENTORY_RENDERER_POSITION_FIELD_COUNT: usize = 64;
pub const GO_AST_RAW_TREE_RENDERER_CODEGEN_COVERAGE_ARTIFACT_KIND: &str =
    "tct_go_ast_raw_tree_renderer_codegen_coverage_v1";
pub const GO_AST_RAW_TREE_RENDERER_CODEGEN_COVERAGE_STATUS_READY: &str =
    "go_ast_raw_tree_renderer_codegen_coverage_ready";
pub const GO_AST_RAW_TREE_RENDERER_CODEGEN_COVERAGE_STATUS_BLOCKED: &str =
    "go_ast_raw_tree_renderer_codegen_coverage_blocked";
pub const GO_AST_RAW_TREE_RENDERER_CODEGEN_COVERAGE_NEXT_READY_TASK: &str =
    crate::syntax_roadmap_successors::GO_SOURCE_RECORD_RAW_TOKEN_GENERATOR_CORE_SUCCESSOR_TASK;
pub const GO_AST_RAW_TREE_RENDERER_CODEGEN_COVERAGE_CLAIM_BOUNDARY: &str = "inventory-backed Go raw-tree renderer codegen coverage gate: the checked pinned go/ast node inventory is the source of truth for generated renderer skeleton dispatch and field traversal coverage, while manual policy hooks are reserved for precedence, punctuation/layout, comments/directives, source-file root boundaries, and parser-validity constraints. After the Go AST printer oracle cutover, this Rust renderer coverage is historical/baseline-only diagnostic evidence and is not grammar-mode formatting/parsing authority. Source batches are regression evidence after this gate, not the default scaling driver. This is not checked-source canonical comment/layout fidelity, not Arena IR evidence, not route-handoff provenance, not BPE validity authority, and not stable-backend promotion.";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GoAstRawTreeRendererSkeletonPolicy {
    pub node_kind: &'static str,
    pub generated_skeleton_ready: bool,
    pub contextual_renderer: bool,
    pub source_file_root_excluded: bool,
}

pub const GO_AST_RAW_TREE_RENDERER_SKELETON_POLICIES: &[GoAstRawTreeRendererSkeletonPolicy] = &[
    generated_renderer_skeleton_policy("ArrayType"),
    generated_renderer_skeleton_policy("AssignStmt"),
    generated_renderer_skeleton_policy("BasicLit"),
    generated_renderer_skeleton_policy("BinaryExpr"),
    generated_renderer_skeleton_policy("BlockStmt"),
    generated_renderer_skeleton_policy("BranchStmt"),
    generated_renderer_skeleton_policy("CallExpr"),
    generated_renderer_skeleton_policy("CaseClause"),
    generated_renderer_skeleton_policy("ChanType"),
    generated_renderer_skeleton_policy("CommClause"),
    generated_renderer_skeleton_policy("Comment"),
    generated_renderer_skeleton_policy("CommentGroup"),
    generated_renderer_skeleton_policy("CompositeLit"),
    generated_renderer_skeleton_policy("DeclStmt"),
    generated_renderer_skeleton_policy("DeferStmt"),
    generated_renderer_skeleton_policy("Ellipsis"),
    generated_renderer_skeleton_policy("EmptyStmt"),
    generated_renderer_skeleton_policy("ExprStmt"),
    GoAstRawTreeRendererSkeletonPolicy {
        node_kind: "Field",
        generated_skeleton_ready: true,
        contextual_renderer: true,
        source_file_root_excluded: false,
    },
    generated_renderer_skeleton_policy("FieldList"),
    generated_renderer_skeleton_policy("File"),
    generated_renderer_skeleton_policy("ForStmt"),
    generated_renderer_skeleton_policy("FuncDecl"),
    generated_renderer_skeleton_policy("FuncLit"),
    generated_renderer_skeleton_policy("FuncType"),
    generated_renderer_skeleton_policy("GenDecl"),
    generated_renderer_skeleton_policy("GoStmt"),
    generated_renderer_skeleton_policy("Ident"),
    generated_renderer_skeleton_policy("IfStmt"),
    generated_renderer_skeleton_policy("ImportSpec"),
    generated_renderer_skeleton_policy("IncDecStmt"),
    generated_renderer_skeleton_policy("IndexExpr"),
    generated_renderer_skeleton_policy("IndexListExpr"),
    generated_renderer_skeleton_policy("InterfaceType"),
    generated_renderer_skeleton_policy("KeyValueExpr"),
    generated_renderer_skeleton_policy("LabeledStmt"),
    generated_renderer_skeleton_policy("MapType"),
    GoAstRawTreeRendererSkeletonPolicy {
        node_kind: "Package",
        generated_skeleton_ready: false,
        contextual_renderer: false,
        source_file_root_excluded: true,
    },
    generated_renderer_skeleton_policy("ParenExpr"),
    generated_renderer_skeleton_policy("RangeStmt"),
    generated_renderer_skeleton_policy("ReturnStmt"),
    generated_renderer_skeleton_policy("SelectStmt"),
    generated_renderer_skeleton_policy("SelectorExpr"),
    generated_renderer_skeleton_policy("SendStmt"),
    generated_renderer_skeleton_policy("SliceExpr"),
    generated_renderer_skeleton_policy("StarExpr"),
    generated_renderer_skeleton_policy("StructType"),
    generated_renderer_skeleton_policy("SwitchStmt"),
    generated_renderer_skeleton_policy("TypeAssertExpr"),
    generated_renderer_skeleton_policy("TypeSpec"),
    generated_renderer_skeleton_policy("TypeSwitchStmt"),
    generated_renderer_skeleton_policy("UnaryExpr"),
    generated_renderer_skeleton_policy("ValueSpec"),
];

const fn generated_renderer_skeleton_policy(
    node_kind: &'static str,
) -> GoAstRawTreeRendererSkeletonPolicy {
    GoAstRawTreeRendererSkeletonPolicy {
        node_kind,
        generated_skeleton_ready: true,
        contextual_renderer: false,
        source_file_root_excluded: false,
    }
}

pub fn go_ast_raw_tree_renderer_skeleton_policy(
    node_kind: &str,
) -> Option<&'static GoAstRawTreeRendererSkeletonPolicy> {
    GO_AST_RAW_TREE_RENDERER_SKELETON_POLICIES
        .iter()
        .find(|policy| policy.node_kind == node_kind)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum GoAstRendererPositionPolicy {
    BadNodeSourceRangeUnsupported,
    CommentTriviaPositionSidecar,
    ParserSourcePositionSidecar,
    FixedSyntaxTokenRendererMetadata,
    FiniteDomainTokenRendererMetadata,
    SyntaxPresenceRendererMetadata,
}

impl GoAstRendererPositionPolicy {
    const ALL: [Self; 6] = [
        Self::BadNodeSourceRangeUnsupported,
        Self::CommentTriviaPositionSidecar,
        Self::ParserSourcePositionSidecar,
        Self::FixedSyntaxTokenRendererMetadata,
        Self::FiniteDomainTokenRendererMetadata,
        Self::SyntaxPresenceRendererMetadata,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::BadNodeSourceRangeUnsupported => "bad_node_source_range_unsupported",
            Self::CommentTriviaPositionSidecar => "comment_trivia_position_sidecar",
            Self::ParserSourcePositionSidecar => "parser_source_position_sidecar",
            Self::FixedSyntaxTokenRendererMetadata => "fixed_syntax_token_renderer_metadata",
            Self::FiniteDomainTokenRendererMetadata => "finite_domain_token_renderer_metadata",
            Self::SyntaxPresenceRendererMetadata => "syntax_presence_renderer_metadata",
        }
    }

    fn from_inventory_str(value: &str) -> Option<Self> {
        match value {
            "bad_node_source_range_unsupported" => Some(Self::BadNodeSourceRangeUnsupported),
            "comment_trivia_position_sidecar" => Some(Self::CommentTriviaPositionSidecar),
            "parser_source_position_sidecar" => Some(Self::ParserSourcePositionSidecar),
            "fixed_syntax_token_renderer_metadata" => Some(Self::FixedSyntaxTokenRendererMetadata),
            "finite_domain_token_renderer_metadata" => {
                Some(Self::FiniteDomainTokenRendererMetadata)
            }
            "syntax_presence_renderer_metadata" => Some(Self::SyntaxPresenceRendererMetadata),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoAstRendererPositionField {
    pub node_name: String,
    pub field_name: String,
    pub go_type: String,
    pub policy: GoAstRendererPositionPolicy,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoAstRawTreeRendererCodegenCoverageReport {
    pub artifact_kind: String,
    pub version: u32,
    pub status: String,
    pub inventory_artifact_path: String,
    pub inventory_artifact_sha256: String,
    pub pinned_go_version: String,
    pub tokenizable_node_count: usize,
    pub classified_node_count: usize,
    pub generated_skeleton_ready_count: usize,
    pub manual_policy_hook_node_count: usize,
    pub contextual_renderer_count: usize,
    pub source_file_root_excluded_count: usize,
    pub missing_policy_count: usize,
    pub node_coverages: Vec<GoAstRawTreeRendererCodegenNodeCoverage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_blocker: Option<GoAstRawTreeRendererCodegenBlocker>,
    pub next_ready_task: String,
    pub claim_boundary: String,
}

impl GoAstRawTreeRendererCodegenCoverageReport {
    pub fn validate(&self) -> Result<(), GoAstSchemaInventoryError> {
        if self.artifact_kind != GO_AST_RAW_TREE_RENDERER_CODEGEN_COVERAGE_ARTIFACT_KIND {
            return Err(GoAstSchemaInventoryError::new(
                "validate",
                Some("renderer_codegen_coverage.artifact_kind".to_string()),
                format!(
                    "expected artifact kind {GO_AST_RAW_TREE_RENDERER_CODEGEN_COVERAGE_ARTIFACT_KIND}, got {}",
                    self.artifact_kind
                ),
            ));
        }
        if self.version != GO_AST_SCHEMA_INVENTORY_VERSION {
            return Err(GoAstSchemaInventoryError::new(
                "validate",
                Some("renderer_codegen_coverage.version".to_string()),
                format!(
                    "expected version {GO_AST_SCHEMA_INVENTORY_VERSION}, got {}",
                    self.version
                ),
            ));
        }
        if self.pinned_go_version != GO_AST_SCHEMA_INVENTORY_PINNED_GO_VERSION {
            return Err(GoAstSchemaInventoryError::new(
                "validate",
                Some("renderer_codegen_coverage.pinned_go_version".to_string()),
                format!(
                    "expected pinned Go version {GO_AST_SCHEMA_INVENTORY_PINNED_GO_VERSION}, got {}",
                    self.pinned_go_version
                ),
            ));
        }
        if self.inventory_artifact_path.is_empty() || self.inventory_artifact_sha256.is_empty() {
            return Err(GoAstSchemaInventoryError::new(
                "validate",
                Some("renderer_codegen_coverage.inventory".to_string()),
                "renderer codegen coverage must identify the checked inventory artifact",
            ));
        }
        if self.tokenizable_node_count == 0
            || self.classified_node_count != self.node_coverages.len()
            || self.tokenizable_node_count != self.node_coverages.len()
        {
            return Err(GoAstSchemaInventoryError::new(
                "validate",
                Some("renderer_codegen_coverage.counts".to_string()),
                "renderer codegen coverage counts must match tokenizable node coverages",
            ));
        }

        let mut seen = BTreeSet::new();
        let mut generated_skeleton_ready_count = 0usize;
        let mut manual_policy_hook_node_count = 0usize;
        let mut contextual_renderer_count = 0usize;
        let mut source_file_root_excluded_count = 0usize;
        let mut missing_policy_count = 0usize;
        let mut first_missing = None;
        let mut package_excluded = false;
        let mut field_contextual = false;
        let mut empty_stmt_policy = false;
        for coverage in &self.node_coverages {
            coverage.validate()?;
            if !seen.insert(coverage.node_kind.as_str()) {
                return Err(GoAstSchemaInventoryError::new(
                    "validate",
                    Some("renderer_codegen_coverage.node_coverages".to_string()),
                    format!("duplicate node coverage for {}", coverage.node_kind),
                ));
            }
            if coverage.generated_skeleton_ready {
                generated_skeleton_ready_count += 1;
            }
            if !coverage.manual_policy_hooks.is_empty() {
                manual_policy_hook_node_count += 1;
            }
            if coverage.contextual_renderer {
                contextual_renderer_count += 1;
            }
            if coverage.source_file_root_excluded {
                source_file_root_excluded_count += 1;
            }
            if coverage.missing_policy {
                missing_policy_count += 1;
                if first_missing.is_none() {
                    first_missing = Some(GoAstRawTreeRendererCodegenBlocker {
                        node_kind: coverage.node_kind.clone(),
                        reason: coverage.reason.clone(),
                    });
                }
            }
            if coverage.node_kind == "Package" {
                package_excluded = coverage.source_file_root_excluded
                    && !coverage.generated_skeleton_ready
                    && !coverage.missing_policy;
            }
            if coverage.node_kind == "Field" {
                field_contextual = coverage.contextual_renderer
                    && coverage.generated_skeleton_ready
                    && !coverage.missing_policy;
            }
            if coverage.node_kind == "EmptyStmt" {
                empty_stmt_policy = coverage
                    .manual_policy_hooks
                    .iter()
                    .any(|hook| hook == "fixed_semicolon_grammar_mode")
                    && coverage.generated_skeleton_ready
                    && !coverage.missing_policy;
            }
        }
        if generated_skeleton_ready_count != self.generated_skeleton_ready_count
            || manual_policy_hook_node_count != self.manual_policy_hook_node_count
            || contextual_renderer_count != self.contextual_renderer_count
            || source_file_root_excluded_count != self.source_file_root_excluded_count
            || missing_policy_count != self.missing_policy_count
        {
            return Err(GoAstSchemaInventoryError::new(
                "validate",
                Some("renderer_codegen_coverage.summary_counts".to_string()),
                "renderer codegen coverage summary counts must match node coverages",
            ));
        }
        if !package_excluded || !field_contextual || !empty_stmt_policy {
            return Err(GoAstSchemaInventoryError::new(
                "validate",
                Some("renderer_codegen_coverage.required_node_policies".to_string()),
                "renderer codegen coverage must classify Package, Field, and EmptyStmt explicitly",
            ));
        }
        match self.status.as_str() {
            GO_AST_RAW_TREE_RENDERER_CODEGEN_COVERAGE_STATUS_READY => {
                if self.missing_policy_count != 0 || self.first_blocker.is_some() {
                    return Err(GoAstSchemaInventoryError::new(
                        "validate",
                        Some("renderer_codegen_coverage.first_blocker".to_string()),
                        "ready renderer codegen coverage must have no missing policy blocker",
                    ));
                }
            }
            GO_AST_RAW_TREE_RENDERER_CODEGEN_COVERAGE_STATUS_BLOCKED => {
                let blocker = self.first_blocker.as_ref().ok_or_else(|| {
                    GoAstSchemaInventoryError::new(
                        "validate",
                        Some("renderer_codegen_coverage.first_blocker".to_string()),
                        "blocked renderer codegen coverage must record the first missing policy",
                    )
                })?;
                if first_missing.as_ref() != Some(blocker) {
                    return Err(GoAstSchemaInventoryError::new(
                        "validate",
                        Some("renderer_codegen_coverage.first_blocker".to_string()),
                        "renderer codegen coverage blocker must match the first missing policy",
                    ));
                }
            }
            other => {
                return Err(GoAstSchemaInventoryError::new(
                    "validate",
                    Some("renderer_codegen_coverage.status".to_string()),
                    format!("unsupported renderer codegen coverage status {other}"),
                ));
            }
        }
        if self.next_ready_task != GO_AST_RAW_TREE_RENDERER_CODEGEN_COVERAGE_NEXT_READY_TASK {
            return Err(GoAstSchemaInventoryError::new(
                "validate",
                Some("renderer_codegen_coverage.next_ready_task".to_string()),
                "renderer codegen coverage must name the generated skeleton contract follow-up",
            ));
        }
        for required in [
            "generated renderer skeleton",
            "manual policy hooks",
            "historical/baseline-only diagnostic evidence",
            "not grammar-mode formatting/parsing authority",
            "Source batches are regression evidence",
            "not Arena IR",
            "not BPE",
        ] {
            if !self.claim_boundary.contains(required) {
                return Err(GoAstSchemaInventoryError::new(
                    "validate",
                    Some("renderer_codegen_coverage.claim_boundary".to_string()),
                    format!("claim boundary must contain {required:?}"),
                ));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoAstRawTreeRendererCodegenNodeCoverage {
    pub node_kind: String,
    pub generated_skeleton_ready: bool,
    pub contextual_renderer: bool,
    pub source_file_root_excluded: bool,
    pub manual_policy_hooks: Vec<String>,
    pub missing_policy: bool,
    pub reason: String,
}

impl GoAstRawTreeRendererCodegenNodeCoverage {
    fn validate(&self) -> Result<(), GoAstSchemaInventoryError> {
        if self.node_kind.is_empty() || self.reason.is_empty() {
            return Err(GoAstSchemaInventoryError::new(
                "validate",
                Some("renderer_codegen_coverage.node_coverages".to_string()),
                "node coverage entries must carry a node kind and reason",
            ));
        }
        if self.source_file_root_excluded && self.generated_skeleton_ready {
            return Err(GoAstSchemaInventoryError::new(
                "validate",
                Some(format!(
                    "renderer_codegen_coverage.node_coverages.{}",
                    self.node_kind
                )),
                "source-file root exclusions must not claim generated source-file skeleton support",
            ));
        }
        if self.missing_policy
            && (self.generated_skeleton_ready
                || self.contextual_renderer
                || self.source_file_root_excluded
                || !self.manual_policy_hooks.is_empty())
        {
            return Err(GoAstSchemaInventoryError::new(
                "validate",
                Some(format!(
                    "renderer_codegen_coverage.node_coverages.{}",
                    self.node_kind
                )),
                "missing policy nodes must not also claim codegen or manual hook coverage",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoAstRawTreeRendererCodegenBlocker {
    pub node_kind: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoAstSchemaInventoryAdapter;

impl GoAstSchemaInventoryAdapter {
    pub fn lower_inventory_file(path: &Path) -> Result<GrammarIr, GoAstSchemaInventoryError> {
        let contents = std::fs::read_to_string(path).map_err(|error| {
            GoAstSchemaInventoryError::new(
                "read",
                Some(path.display().to_string()),
                format!("failed to read inventory: {error}"),
            )
        })?;
        let provenance = file_provenance(path).map_err(|error| {
            GoAstSchemaInventoryError::new(
                "provenance",
                Some(path.display().to_string()),
                error.to_string(),
            )
        })?;
        Self::lower_inventory_str(
            &contents,
            Some(path.display().to_string()),
            Some(provenance.sha256),
        )
    }

    pub fn lower_inventory_str(
        contents: &str,
        metadata_path: Option<String>,
        metadata_sha256: Option<String>,
    ) -> Result<GrammarIr, GoAstSchemaInventoryError> {
        let inventory = parse_inventory_str(contents)?;
        validate_inventory(&inventory)?;

        let source = inventory
            .sources
            .iter()
            .find(|source| source.role == "go_ast")
            .ok_or_else(|| {
                GoAstSchemaInventoryError::new(
                    "validate",
                    Some("sources".to_string()),
                    "inventory must include a go_ast source record",
                )
            })?;

        let mut grammar = GrammarIr::new(
            inventory.grammar_id,
            GrammarIrProvenance {
                source_kind: "go_ast_schema_inventory".to_string(),
                source_path: Some(source.path.clone()),
                source_sha256: Some(source.sha256.clone()),
                metadata_path,
                metadata_sha256,
                parser: Some(GrammarParserProvenance {
                    tool: "go".to_string(),
                    version: inventory.go.version,
                    package: Some(inventory.go.ast_package),
                    package_version: Some(GO_AST_SCHEMA_INVENTORY_PINNED_GO_VERSION.to_string()),
                }),
            },
        );
        let mut rules = inventory
            .grammar_ir_rules
            .into_iter()
            .enumerate()
            .map(lower_rule)
            .collect::<Result<Vec<_>, _>>()?;
        promote_source_file_root_rule(&mut rules)?;
        grammar.rules = rules;
        Ok(grammar)
    }

    pub fn renderer_position_fields_file(
        path: &Path,
    ) -> Result<Vec<GoAstRendererPositionField>, GoAstSchemaInventoryError> {
        let contents = std::fs::read_to_string(path).map_err(|error| {
            GoAstSchemaInventoryError::new(
                "read",
                Some(path.display().to_string()),
                format!("failed to read inventory: {error}"),
            )
        })?;
        Self::renderer_position_fields_str(&contents)
    }

    pub fn renderer_position_fields_str(
        contents: &str,
    ) -> Result<Vec<GoAstRendererPositionField>, GoAstSchemaInventoryError> {
        let inventory = parse_inventory_str(contents)?;
        validate_inventory(&inventory)?;
        collect_renderer_position_fields(&inventory)
    }

    pub fn raw_tree_renderer_codegen_coverage_file(
        path: &Path,
    ) -> Result<GoAstRawTreeRendererCodegenCoverageReport, GoAstSchemaInventoryError> {
        let contents = std::fs::read_to_string(path).map_err(|error| {
            GoAstSchemaInventoryError::new(
                "read",
                Some(path.display().to_string()),
                format!("failed to read inventory: {error}"),
            )
        })?;
        Self::raw_tree_renderer_codegen_coverage_str(
            &contents,
            Some(path.display().to_string()),
            Some(sha256_hex(contents.as_bytes())),
        )
    }

    pub fn raw_tree_renderer_codegen_coverage_str(
        contents: &str,
        metadata_path: Option<String>,
        metadata_sha256: Option<String>,
    ) -> Result<GoAstRawTreeRendererCodegenCoverageReport, GoAstSchemaInventoryError> {
        let inventory = parse_inventory_str(contents)?;
        validate_inventory(&inventory)?;
        let inventory_artifact_path = metadata_path.unwrap_or_else(|| "<memory>".to_string());
        let inventory_artifact_sha256 =
            metadata_sha256.unwrap_or_else(|| sha256_hex(contents.as_bytes()));
        let tokenizable_nodes = inventory
            .node_structs
            .iter()
            .filter(|node| node.tokenizable)
            .collect::<Vec<_>>();
        let node_coverages = tokenizable_nodes
            .iter()
            .map(|node| raw_tree_renderer_codegen_node_coverage(&node.name))
            .collect::<Vec<_>>();
        let generated_skeleton_ready_count = node_coverages
            .iter()
            .filter(|coverage| coverage.generated_skeleton_ready)
            .count();
        let manual_policy_hook_node_count = node_coverages
            .iter()
            .filter(|coverage| !coverage.manual_policy_hooks.is_empty())
            .count();
        let contextual_renderer_count = node_coverages
            .iter()
            .filter(|coverage| coverage.contextual_renderer)
            .count();
        let source_file_root_excluded_count = node_coverages
            .iter()
            .filter(|coverage| coverage.source_file_root_excluded)
            .count();
        let missing_policy_count = node_coverages
            .iter()
            .filter(|coverage| coverage.missing_policy)
            .count();
        let first_blocker = node_coverages
            .iter()
            .find(|coverage| coverage.missing_policy)
            .map(|coverage| GoAstRawTreeRendererCodegenBlocker {
                node_kind: coverage.node_kind.clone(),
                reason: coverage.reason.clone(),
            });
        let status = if first_blocker.is_some() {
            GO_AST_RAW_TREE_RENDERER_CODEGEN_COVERAGE_STATUS_BLOCKED
        } else {
            GO_AST_RAW_TREE_RENDERER_CODEGEN_COVERAGE_STATUS_READY
        };
        let report = GoAstRawTreeRendererCodegenCoverageReport {
            artifact_kind: GO_AST_RAW_TREE_RENDERER_CODEGEN_COVERAGE_ARTIFACT_KIND.to_string(),
            version: GO_AST_SCHEMA_INVENTORY_VERSION,
            status: status.to_string(),
            inventory_artifact_path,
            inventory_artifact_sha256,
            pinned_go_version: inventory.go.version,
            tokenizable_node_count: tokenizable_nodes.len(),
            classified_node_count: node_coverages.len(),
            generated_skeleton_ready_count,
            manual_policy_hook_node_count,
            contextual_renderer_count,
            source_file_root_excluded_count,
            missing_policy_count,
            node_coverages,
            first_blocker,
            next_ready_task: GO_AST_RAW_TREE_RENDERER_CODEGEN_COVERAGE_NEXT_READY_TASK.to_string(),
            claim_boundary: GO_AST_RAW_TREE_RENDERER_CODEGEN_COVERAGE_CLAIM_BOUNDARY.to_string(),
        };
        report.validate()?;
        Ok(report)
    }
}

fn promote_source_file_root_rule(
    rules: &mut Vec<GrammarRule>,
) -> Result<(), GoAstSchemaInventoryError> {
    let Some(index) = rules
        .iter()
        .position(|rule| rule.name == "File" && rule.node_kind == "File")
    else {
        return Err(GoAstSchemaInventoryError::new(
            "lower",
            Some("grammar_ir_rules.File".to_string()),
            "Go AST inventory lowering requires File to be the GoSourceFile root",
        ));
    };
    if index > 0 {
        let root = rules.remove(index);
        rules.insert(0, root);
    }
    Ok(())
}

fn raw_tree_renderer_codegen_node_coverage(
    node_kind: &str,
) -> GoAstRawTreeRendererCodegenNodeCoverage {
    let Some(policy) = go_ast_raw_tree_renderer_skeleton_policy(node_kind) else {
        return GoAstRawTreeRendererCodegenNodeCoverage {
            node_kind: node_kind.to_string(),
            generated_skeleton_ready: false,
            contextual_renderer: false,
            source_file_root_excluded: false,
            manual_policy_hooks: Vec::new(),
            missing_policy: true,
            reason: "tokenizable go/ast node is not classified for generated renderer skeleton coverage or a manual policy hook".to_string(),
        };
    };
    if policy.source_file_root_excluded {
        return GoAstRawTreeRendererCodegenNodeCoverage {
            node_kind: node_kind.to_string(),
            generated_skeleton_ready: false,
            contextual_renderer: false,
            source_file_root_excluded: true,
            manual_policy_hooks: Vec::new(),
            missing_policy: false,
            reason: "go/ast.Package is a multi-file aggregate; the Go tokenizer-generator source root is File".to_string(),
        };
    }
    let manual_policy_hooks = raw_tree_renderer_manual_policy_hooks(node_kind);
    let reason = if policy.contextual_renderer {
        "inventory-derived skeleton is rendered through FieldList, parameter, struct, and interface contexts"
    } else if manual_policy_hooks.is_empty() {
        "inventory-derived skeleton dispatch and field traversal are sufficient for this node"
    } else {
        "inventory-derived skeleton dispatch is ready; listed manual policy hooks own rendering semantics"
    };
    GoAstRawTreeRendererCodegenNodeCoverage {
        node_kind: node_kind.to_string(),
        generated_skeleton_ready: policy.generated_skeleton_ready,
        contextual_renderer: policy.contextual_renderer,
        source_file_root_excluded: false,
        manual_policy_hooks,
        missing_policy: false,
        reason: reason.to_string(),
    }
}

pub(crate) fn raw_tree_renderer_manual_policy_hooks(node_kind: &str) -> Vec<String> {
    let hooks: &[&str] = match node_kind {
        "File" => &["source_file_layout", "package_clause", "file_comment_order"],
        "GenDecl" => &["declaration_keyword_layout", "grouped_declaration_layout"],
        "FuncDecl" => &["function_signature_layout", "receiver_layout"],
        "FuncType" => &["function_type_layout", "type_parameter_layout"],
        "FieldList" => &["comma_list_layout", "empty_field_list_layout"],
        "Field" => &[
            "field_names_type_tag_layout",
            "field_comment_layout",
            "interface_method_layout",
        ],
        "BlockStmt" => &["statement_block_layout"],
        "AssignStmt" => &["assignment_operator_layout"],
        "BranchStmt" => &["branch_token_layout"],
        "CaseClause" => &["case_clause_layout", "case_comment_layout"],
        "CommClause" => &["select_clause_layout", "comm_comment_layout"],
        "DeclStmt" => &["nested_declaration_statement_layout"],
        "DeferStmt" | "GoStmt" => &["call_statement_keyword_layout"],
        "EmptyStmt" => &["fixed_semicolon_grammar_mode"],
        "ExprStmt" => &["expression_statement_layout"],
        "ForStmt" => &["for_header_layout", "loop_comment_policy"],
        "IfStmt" => &["if_header_layout", "else_layout", "branch_comment_policy"],
        "IncDecStmt" => &["inc_dec_operator_layout"],
        "LabeledStmt" => &["label_colon_layout"],
        "RangeStmt" => &["range_header_layout"],
        "ReturnStmt" => &["return_result_layout"],
        "SelectStmt" => &["select_body_layout"],
        "SendStmt" => &["send_operator_layout"],
        "SwitchStmt" => &["switch_header_layout"],
        "TypeSwitchStmt" => &["type_switch_header_layout"],
        "BasicLit" => &["literal_payload_rendering"],
        "BinaryExpr" => &["operator_precedence", "binary_operator_layout"],
        "CallExpr" => &["call_argument_layout", "ellipsis_policy"],
        "ChanType" => &["channel_direction_token_layout"],
        "Comment" | "CommentGroup" => &["comment_trivia_policy"],
        "CompositeLit" => &["literal_element_layout", "composite_line_break_policy"],
        "Ellipsis" => &["ellipsis_token_layout"],
        "FuncLit" => &["function_literal_body_layout"],
        "Ident" => &["identifier_payload_rendering"],
        "IndexExpr" | "IndexListExpr" => &["index_bracket_layout"],
        "KeyValueExpr" => &["key_value_colon_layout"],
        "ParenExpr" => &["parenthesized_expression_preservation"],
        "SelectorExpr" => &["selector_dot_layout"],
        "SliceExpr" => &["slice_bounds_layout"],
        "TypeAssertExpr" => &["type_assertion_layout"],
        "UnaryExpr" => &["unary_operator_precedence"],
        "ValueSpec" => &["value_spec_assignment_layout"],
        _ => &[],
    };
    hooks.iter().map(|hook| (*hook).to_string()).collect()
}

fn parse_inventory_str(contents: &str) -> Result<GoAstSchemaInventory, GoAstSchemaInventoryError> {
    serde_json::from_str(contents).map_err(|error| {
        GoAstSchemaInventoryError::new(
            "parse",
            None,
            format!("Go AST schema inventory JSON is invalid: {error}"),
        )
    })
}

/// The pinned Go alternative node_start token table from the checked
/// inventory fixture, keyed by node kind.
///
/// Token IDs are offsets from the start of the structural token range. Every
/// Go sidecar (full inventory and per-record) must resolve multi-alternative
/// nonterminal node kinds through this table so per-record alphabetical
/// assignment can never diverge from the global layout.
pub fn go_pinned_alternative_node_start_tokens()
-> Result<&'static BTreeMap<String, u32>, GoAstSchemaInventoryError> {
    static PINNED: OnceLock<Result<BTreeMap<String, u32>, GoAstSchemaInventoryError>> =
        OnceLock::new();
    PINNED
        .get_or_init(|| {
            let inventory = parse_inventory_str(include_str!(
                "../tests/fixtures/go_ast_schema_inventory/go1.24.3-ast-schema-inventory-v1.json"
            ))?;
            validate_inventory(&inventory)?;
            let assignment = inventory.structural_token_assignment.ok_or_else(|| {
                GoAstSchemaInventoryError::new(
                    "validate",
                    Some("structural_token_assignment".to_string()),
                    format!(
                        "checked inventory {GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH} must pin the structural token assignment"
                    ),
                )
            })?;
            Ok(assignment
                .alternative_node_start_tokens
                .into_iter()
                .map(|token| (token.node_kind, token.token_id))
                .collect())
        })
        .as_ref()
        .map_err(Clone::clone)
}

fn validate_structural_token_assignment(
    inventory: &GoAstSchemaInventory,
) -> Result<(), GoAstSchemaInventoryError> {
    let Some(assignment) = &inventory.structural_token_assignment else {
        return Err(GoAstSchemaInventoryError::new(
            "validate",
            Some("structural_token_assignment".to_string()),
            format!(
                "inventory must pin the alternative node_start token assignment ({GO_STRUCTURAL_TOKEN_ASSIGNMENT_APPEND_RULE})"
            ),
        ));
    };
    if assignment.assignment_version != GO_STRUCTURAL_TOKEN_ASSIGNMENT_VERSION {
        return Err(GoAstSchemaInventoryError::new(
            "validate",
            Some("structural_token_assignment.assignment_version".to_string()),
            format!(
                "expected assignment version {GO_STRUCTURAL_TOKEN_ASSIGNMENT_VERSION}, got {}",
                assignment.assignment_version
            ),
        ));
    }
    let offset =
        crate::syntax_go_source_instance_projection::RAW_TOKEN_ALTERNATIVE_NODE_START_OFFSET;
    if assignment.alternative_node_start_offset != offset {
        return Err(GoAstSchemaInventoryError::new(
            "validate",
            Some("structural_token_assignment.alternative_node_start_offset".to_string()),
            format!(
                "expected alternative node_start offset {offset}, got {}",
                assignment.alternative_node_start_offset
            ),
        ));
    }
    if assignment.append_rule != GO_STRUCTURAL_TOKEN_ASSIGNMENT_APPEND_RULE {
        return Err(GoAstSchemaInventoryError::new(
            "validate",
            Some("structural_token_assignment.append_rule".to_string()),
            format!(
                "expected append rule {GO_STRUCTURAL_TOKEN_ASSIGNMENT_APPEND_RULE:?}, got {:?}",
                assignment.append_rule
            ),
        ));
    }

    let mut multi_alternative_kinds = BTreeSet::new();
    for rule in &inventory.grammar_ir_rules {
        for field in &rule.fields {
            if field.alternatives.len() <= 1 {
                continue;
            }
            for alternative in &field.alternatives {
                if alternative.terminal == GrammarTerminalKind::Nonterminal {
                    multi_alternative_kinds.insert(alternative.node_kind.as_str());
                }
            }
        }
    }

    let mut seen_kinds = BTreeSet::new();
    let mut seen_ids = BTreeSet::new();
    for token in &assignment.alternative_node_start_tokens {
        if token.node_kind.is_empty() {
            return Err(GoAstSchemaInventoryError::new(
                "validate",
                Some("structural_token_assignment.alternative_node_start_tokens".to_string()),
                "assigned token entries must carry a node kind",
            ));
        }
        if !seen_kinds.insert(token.node_kind.as_str()) {
            return Err(GoAstSchemaInventoryError::new(
                "validate",
                Some("structural_token_assignment.alternative_node_start_tokens".to_string()),
                format!("duplicate assigned node kind {}", token.node_kind),
            ));
        }
        if !seen_ids.insert(token.token_id) {
            return Err(GoAstSchemaInventoryError::new(
                "validate",
                Some("structural_token_assignment.alternative_node_start_tokens".to_string()),
                format!(
                    "duplicate assigned token id {} for {}",
                    token.token_id, token.node_kind
                ),
            ));
        }
        if token.token_id < assignment.alternative_node_start_offset {
            return Err(GoAstSchemaInventoryError::new(
                "validate",
                Some("structural_token_assignment.alternative_node_start_tokens".to_string()),
                format!(
                    "assigned token id {} for {} is below the alternative node_start offset {}",
                    token.token_id, token.node_kind, assignment.alternative_node_start_offset
                ),
            ));
        }
        if !multi_alternative_kinds.contains(token.node_kind.as_str()) {
            return Err(GoAstSchemaInventoryError::new(
                "validate",
                Some("structural_token_assignment.alternative_node_start_tokens".to_string()),
                format!(
                    "assigned node kind {} is not a multi-alternative nonterminal node kind in this inventory",
                    token.node_kind
                ),
            ));
        }
    }
    for kind in multi_alternative_kinds {
        if !seen_kinds.contains(kind) {
            return Err(GoAstSchemaInventoryError::new(
                "validate",
                Some("structural_token_assignment.alternative_node_start_tokens".to_string()),
                format!(
                    "multi-alternative nonterminal node kind {kind} has no pinned token id in {GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH}; {GO_STRUCTURAL_TOKEN_ASSIGNMENT_APPEND_RULE}"
                ),
            ));
        }
    }
    Ok(())
}

fn validate_inventory(inventory: &GoAstSchemaInventory) -> Result<(), GoAstSchemaInventoryError> {
    if inventory.artifact_kind != GO_AST_SCHEMA_INVENTORY_ARTIFACT_KIND {
        return Err(GoAstSchemaInventoryError::new(
            "validate",
            Some("artifact_kind".to_string()),
            format!(
                "expected artifact kind {GO_AST_SCHEMA_INVENTORY_ARTIFACT_KIND}, got {}",
                inventory.artifact_kind
            ),
        ));
    }
    if inventory.version != GO_AST_SCHEMA_INVENTORY_VERSION {
        return Err(GoAstSchemaInventoryError::new(
            "validate",
            Some("version".to_string()),
            format!(
                "expected version {GO_AST_SCHEMA_INVENTORY_VERSION}, got {}",
                inventory.version
            ),
        ));
    }
    if inventory.go.version != GO_AST_SCHEMA_INVENTORY_PINNED_GO_VERSION {
        return Err(GoAstSchemaInventoryError::new(
            "validate",
            Some("go.version".to_string()),
            format!(
                "expected pinned Go version {GO_AST_SCHEMA_INVENTORY_PINNED_GO_VERSION}, got {}",
                inventory.go.version
            ),
        ));
    }
    if inventory.go.ast_package != "go/ast" || inventory.go.parser_package != "go/parser" {
        return Err(GoAstSchemaInventoryError::new(
            "validate",
            Some("go".to_string()),
            "inventory must identify go/ast and go/parser",
        ));
    }
    if inventory.status != "ready_with_blockers" {
        return Err(GoAstSchemaInventoryError::new(
            "validate",
            Some("status".to_string()),
            "inventory must keep sidecar blockers explicit",
        ));
    }
    if inventory.grammar_id.is_empty() {
        return Err(GoAstSchemaInventoryError::new(
            "validate",
            Some("grammar_id".to_string()),
            "grammar_id must be present",
        ));
    }
    if inventory.grammar_ir_rules.is_empty() {
        return Err(GoAstSchemaInventoryError::new(
            "validate",
            Some("grammar_ir_rules".to_string()),
            "inventory must include GrammarIR lowering rules",
        ));
    }
    validate_required_families(inventory)?;
    validate_required_domains(inventory)?;
    validate_renderer_position_fields(inventory)?;
    validate_structural_token_assignment(inventory)?;
    if inventory.stable_promotion_blockers.is_empty()
        || inventory.no_stable_promotion_claim.is_empty()
    {
        return Err(GoAstSchemaInventoryError::new(
            "validate",
            Some("stable_promotion_blockers".to_string()),
            "inventory must keep stable-promotion blockers explicit",
        ));
    }
    Ok(())
}

fn validate_renderer_position_fields(
    inventory: &GoAstSchemaInventory,
) -> Result<(), GoAstSchemaInventoryError> {
    let position_fields = collect_renderer_position_fields(inventory)?;
    if position_fields.len() != GO_AST_SCHEMA_INVENTORY_RENDERER_POSITION_FIELD_COUNT {
        return Err(GoAstSchemaInventoryError::new(
            "validate",
            Some("node_structs[].position_fields".to_string()),
            format!(
                "expected {GO_AST_SCHEMA_INVENTORY_RENDERER_POSITION_FIELD_COUNT} token.Pos renderer-position fields, got {}",
                position_fields.len()
            ),
        ));
    }

    let seen = position_fields
        .iter()
        .map(|field| field.policy)
        .collect::<BTreeSet<_>>();
    for required in GoAstRendererPositionPolicy::ALL {
        if !seen.contains(&required) {
            return Err(GoAstSchemaInventoryError::new(
                "validate",
                Some("node_structs[].position_fields[].policy".to_string()),
                format!(
                    "missing renderer-position policy category {}",
                    required.as_str()
                ),
            ));
        }
    }

    Ok(())
}

fn collect_renderer_position_fields(
    inventory: &GoAstSchemaInventory,
) -> Result<Vec<GoAstRendererPositionField>, GoAstSchemaInventoryError> {
    if inventory.node_structs.is_empty() {
        return Err(GoAstSchemaInventoryError::new(
            "validate",
            Some("node_structs".to_string()),
            "inventory must include node_structs for renderer-position metadata",
        ));
    }

    let mut position_fields = Vec::new();
    for (node_index, node) in inventory.node_structs.iter().enumerate() {
        if node.name.is_empty() {
            return Err(GoAstSchemaInventoryError::new(
                "validate",
                Some(format!("node_structs[{node_index}].name")),
                "node name must be present",
            ));
        }

        let fields = node.position_fields.as_deref().unwrap_or(&[]);
        for (field_index, field) in fields.iter().enumerate() {
            if field.name.is_empty() {
                return Err(GoAstSchemaInventoryError::new(
                    "validate",
                    Some(format!(
                        "node_structs[{node_index}].position_fields[{field_index}].name"
                    )),
                    "position field name must be present",
                ));
            }

            let path = format!("{}.{}", node.name, field.name);
            if field.go_type != "token.Pos" {
                return Err(GoAstSchemaInventoryError::new(
                    "validate",
                    Some(path),
                    format!(
                        "position field must retain go_type token.Pos, got {}",
                        field.go_type
                    ),
                ));
            }
            if field.policy.is_empty() {
                return Err(GoAstSchemaInventoryError::new(
                    "validate",
                    Some(path),
                    "position field must have a renderer-position policy",
                ));
            }
            let policy = GoAstRendererPositionPolicy::from_inventory_str(&field.policy)
                .ok_or_else(|| {
                    GoAstSchemaInventoryError::new(
                        "validate",
                        Some(path.clone()),
                        format!(
                            "position field has unsupported renderer-position policy {}",
                            field.policy
                        ),
                    )
                })?;
            position_fields.push(GoAstRendererPositionField {
                node_name: node.name.clone(),
                field_name: field.name.clone(),
                go_type: field.go_type.clone(),
                policy,
            });
        }
    }

    Ok(position_fields)
}

fn validate_required_families(
    inventory: &GoAstSchemaInventory,
) -> Result<(), GoAstSchemaInventoryError> {
    let families: BTreeSet<_> = inventory
        .interface_families
        .iter()
        .map(|family| family.name.as_str())
        .collect();
    for required in ["Expr", "Stmt", "Decl", "Spec"] {
        if !families.contains(required) {
            return Err(GoAstSchemaInventoryError::new(
                "validate",
                Some("interface_families".to_string()),
                format!("missing interface family {required}"),
            ));
        }
    }
    Ok(())
}

fn validate_required_domains(
    inventory: &GoAstSchemaInventory,
) -> Result<(), GoAstSchemaInventoryError> {
    let domains: BTreeSet<_> = inventory
        .finite_domains
        .iter()
        .map(|domain| domain.name.as_str())
        .collect();
    for required in [
        "go_token_declaration_tokens",
        "go_token_branch_tokens",
        "go_token_basic_literal_kinds",
        "go_ast_channel_directions",
        "go_syntax_presence_call_expr_ellipsis",
        "go_syntax_presence_func_type_func",
        "go_syntax_presence_gen_decl_group",
        "go_syntax_presence_type_spec_alias",
    ] {
        if !domains.contains(required) {
            return Err(GoAstSchemaInventoryError::new(
                "validate",
                Some("finite_domains".to_string()),
                format!("missing finite domain {required}"),
            ));
        }
    }
    Ok(())
}

fn lower_rule(
    (rule_index, rule): (usize, GoAstSchemaInventoryRule),
) -> Result<GrammarRule, GoAstSchemaInventoryError> {
    if rule.name.is_empty() || rule.node_kind.is_empty() {
        return Err(GoAstSchemaInventoryError::new(
            "lower",
            Some(format!("grammar_ir_rules[{rule_index}]")),
            "rule name and node_kind must be present",
        ));
    }
    let fields = rule
        .fields
        .into_iter()
        .enumerate()
        .map(|(field_index, field)| lower_field(rule_index, field_index, field))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(GrammarRule {
        name: rule.name,
        node_kind: rule.node_kind,
        fields,
    })
}

fn lower_field(
    rule_index: usize,
    field_index: usize,
    field: GoAstSchemaInventoryField,
) -> Result<GrammarField, GoAstSchemaInventoryError> {
    if field.name.is_empty() {
        return Err(GoAstSchemaInventoryError::new(
            "lower",
            Some(format!(
                "grammar_ir_rules[{rule_index}].fields[{field_index}]"
            )),
            "field name must be present",
        ));
    }
    if field.alternatives.is_empty() {
        return Err(GoAstSchemaInventoryError::new(
            "lower",
            Some(format!(
                "grammar_ir_rules[{rule_index}].fields[{field_index}].alternatives"
            )),
            "field has no alternatives",
        ));
    }
    let alternatives = field
        .alternatives
        .into_iter()
        .map(|alternative| GrammarFieldAlternative {
            node_kind: alternative.node_kind,
            terminal: alternative.terminal,
        })
        .collect();
    Ok(GrammarField {
        name: field.name,
        required: field.required,
        multiple: field.multiple,
        alternatives,
    })
}

#[derive(Debug, Deserialize)]
struct GoAstSchemaInventory {
    artifact_kind: String,
    version: u32,
    status: String,
    grammar_id: String,
    go: GoAstSchemaInventoryToolchain,
    sources: Vec<GoAstSchemaInventorySource>,
    #[serde(default)]
    node_structs: Vec<GoAstSchemaInventoryNodeStruct>,
    #[serde(default)]
    interface_families: Vec<GoAstSchemaInventoryFamily>,
    #[serde(default)]
    finite_domains: Vec<GoAstSchemaInventoryDomain>,
    #[serde(default)]
    grammar_ir_rules: Vec<GoAstSchemaInventoryRule>,
    #[serde(default)]
    structural_token_assignment: Option<GoAstSchemaInventoryStructuralTokenAssignment>,
    #[serde(default)]
    stable_promotion_blockers: Vec<String>,
    #[serde(default)]
    no_stable_promotion_claim: String,
}

#[derive(Debug, Deserialize)]
struct GoAstSchemaInventoryStructuralTokenAssignment {
    assignment_version: u32,
    alternative_node_start_offset: u32,
    #[serde(default)]
    append_rule: String,
    #[serde(default)]
    alternative_node_start_tokens: Vec<GoAstSchemaInventoryAssignedToken>,
}

#[derive(Debug, Deserialize)]
struct GoAstSchemaInventoryAssignedToken {
    node_kind: String,
    token_id: u32,
}

#[derive(Debug, Deserialize)]
struct GoAstSchemaInventoryToolchain {
    version: String,
    parser_package: String,
    ast_package: String,
}

#[derive(Debug, Deserialize)]
struct GoAstSchemaInventorySource {
    role: String,
    path: String,
    sha256: String,
}

#[derive(Debug, Deserialize)]
struct GoAstSchemaInventoryFamily {
    name: String,
}

#[derive(Debug, Deserialize)]
struct GoAstSchemaInventoryDomain {
    name: String,
}

#[derive(Debug, Deserialize)]
struct GoAstSchemaInventoryNodeStruct {
    name: String,
    #[serde(default)]
    tokenizable: bool,
    #[serde(default)]
    position_fields: Option<Vec<GoAstSchemaInventoryPositionField>>,
}

#[derive(Debug, Deserialize)]
struct GoAstSchemaInventoryPositionField {
    name: String,
    #[serde(default)]
    go_type: String,
    #[serde(default)]
    policy: String,
}

#[derive(Debug, Deserialize)]
struct GoAstSchemaInventoryRule {
    name: String,
    node_kind: String,
    #[serde(default)]
    fields: Vec<GoAstSchemaInventoryField>,
}

#[derive(Debug, Deserialize)]
struct GoAstSchemaInventoryField {
    name: String,
    #[serde(default)]
    required: bool,
    #[serde(default)]
    multiple: bool,
    #[serde(default)]
    alternatives: Vec<GoAstSchemaInventoryAlternative>,
}

#[derive(Debug, Deserialize)]
struct GoAstSchemaInventoryAlternative {
    node_kind: String,
    terminal: GrammarTerminalKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoAstSchemaInventoryError {
    phase: &'static str,
    path: Option<String>,
    message: String,
}

impl GoAstSchemaInventoryError {
    fn new(phase: &'static str, path: Option<String>, message: impl Into<String>) -> Self {
        Self {
            phase,
            path,
            message: message.into(),
        }
    }
}

impl fmt::Display for GoAstSchemaInventoryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "Go AST schema inventory {} error", self.phase)?;
        if let Some(path) = &self.path {
            write!(formatter, " at {path}")?;
        }
        write!(formatter, ": {}", self.message)
    }
}

impl Error for GoAstSchemaInventoryError {}
