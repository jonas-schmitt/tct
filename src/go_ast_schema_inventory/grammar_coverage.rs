use super::{
    GO_AST_SCHEMA_INVENTORY_PINNED_GO_VERSION,
    GO_AST_SCHEMA_INVENTORY_RENDERER_POSITION_FIELD_COUNT, GO_AST_SCHEMA_INVENTORY_VERSION,
    GoAstSchemaInventory, GoAstSchemaInventoryAdapter, GoAstSchemaInventoryError,
    collect_renderer_position_fields, parse_inventory_str, validate_inventory,
};
use crate::grammar_ir::{GrammarIr, GrammarTerminalKind};
use crate::grammar_ir_structural_model::{
    GrammarStructuralModelSidecar, GrammarStructuralTokenLayout,
    lower_grammar_ir_to_structural_model,
};
use crate::provenance::sha256_hex;
use crate::syntax_go_source_instance_projection::RAW_TOKEN_ALTERNATIVE_NODE_START_OFFSET;
use crate::syntax_state_machine::{
    CompiledSyntaxModel, RAW_NODE_START_OFFSET, RAW_SYNTAX_STATE_MACHINE_CORE_BOUNDARY,
    RAW_SYNTAX_STATE_MACHINE_CORE_BOUNDARY_ID,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::path::Path;

pub const GO_AST_SCHEMA_GRAMMAR_COVERAGE_ARTIFACT_KIND: &str =
    "tct_go_ast_schema_inventory_grammar_coverage_v1";
pub const GO_AST_SCHEMA_GRAMMAR_COVERAGE_STATUS_READY: &str =
    "go_ast_grammar_surface_raw_state_coverage_ready";
pub const GO_AST_SCHEMA_GRAMMAR_COVERAGE_STATUS_BLOCKED: &str =
    "go_ast_grammar_surface_raw_state_coverage_blocked";
pub const GO_AST_SCHEMA_GRAMMAR_COVERAGE_NEXT_READY_TASK: &str =
    crate::syntax_roadmap_successors::GO_SOURCE_RECORD_RAW_TOKEN_GENERATOR_CORE_SUCCESSOR_TASK;
pub const GO_AST_SCHEMA_GRAMMAR_COVERAGE_BLOCKER_NEXT_READY_TASK: &str = "expand Go syntax structural token range for inventory-backed raw state-machine node alternatives";
pub const GO_AST_SCHEMA_GRAMMAR_COVERAGE_CLAIM_BOUNDARY: &str = "inventory-backed Go grammar coverage gate: pinned parser-backed go/ast metadata lowers through GrammarIR -> StructuralModel -> token layout and then attempts the raw syntax state machine boundary. The gate either proves the whole valid go/ast grammar surface reaches raw state/decode coverage or records the first precise blocker. This is not checked-source canonical comment/layout fidelity, not Arena IR, not route-handoff provenance, not BPE validity authority, and not stable-backend promotion.";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoAstSchemaGrammarCoverageReport {
    pub artifact_kind: String,
    pub version: u32,
    pub status: String,
    pub inventory_artifact_path: String,
    pub inventory_artifact_sha256: String,
    pub grammar_id: String,
    pub pinned_go_version: String,
    pub inventory_surface_checked: bool,
    pub parser_error_placeholders_excluded: Vec<String>,
    pub tokenizable_node_count: usize,
    pub grammar_ir_rule_count: usize,
    pub tokenizable_field_count: usize,
    pub structural_model_lowered: bool,
    pub structural_model_record_count: usize,
    pub structural_model_field_count: usize,
    pub finite_domain_count: usize,
    pub renderer_position_policy_field_count: usize,
    pub token_layout: GoAstSchemaGrammarCoverageTokenLayout,
    pub raw_state_machine: GoAstSchemaGrammarCoverageRawStateMachine,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_blocker: Option<GoAstSchemaGrammarCoverageBlocker>,
    pub next_ready_task: String,
    pub claim_boundary: String,
}

impl GoAstSchemaGrammarCoverageReport {
    pub fn validate(&self) -> Result<(), GoAstSchemaInventoryError> {
        if self.artifact_kind != GO_AST_SCHEMA_GRAMMAR_COVERAGE_ARTIFACT_KIND {
            return Err(GoAstSchemaInventoryError::new(
                "validate",
                Some("grammar_coverage.artifact_kind".to_string()),
                format!(
                    "expected artifact kind {GO_AST_SCHEMA_GRAMMAR_COVERAGE_ARTIFACT_KIND}, got {}",
                    self.artifact_kind
                ),
            ));
        }
        if self.version != GO_AST_SCHEMA_INVENTORY_VERSION {
            return Err(GoAstSchemaInventoryError::new(
                "validate",
                Some("grammar_coverage.version".to_string()),
                format!(
                    "expected version {GO_AST_SCHEMA_INVENTORY_VERSION}, got {}",
                    self.version
                ),
            ));
        }
        if self.pinned_go_version != GO_AST_SCHEMA_INVENTORY_PINNED_GO_VERSION {
            return Err(GoAstSchemaInventoryError::new(
                "validate",
                Some("grammar_coverage.pinned_go_version".to_string()),
                format!(
                    "expected pinned Go version {GO_AST_SCHEMA_INVENTORY_PINNED_GO_VERSION}, got {}",
                    self.pinned_go_version
                ),
            ));
        }
        if self.inventory_artifact_path.is_empty() || self.inventory_artifact_sha256.is_empty() {
            return Err(GoAstSchemaInventoryError::new(
                "validate",
                Some("grammar_coverage.inventory".to_string()),
                "grammar coverage must identify the checked inventory artifact",
            ));
        }
        if self.grammar_id.is_empty() {
            return Err(GoAstSchemaInventoryError::new(
                "validate",
                Some("grammar_coverage.grammar_id".to_string()),
                "grammar coverage must carry a grammar id",
            ));
        }
        if !self.inventory_surface_checked {
            return Err(GoAstSchemaInventoryError::new(
                "validate",
                Some("grammar_coverage.inventory_surface_checked".to_string()),
                "grammar coverage must check the pinned inventory surface",
            ));
        }
        let excluded = self
            .parser_error_placeholders_excluded
            .iter()
            .map(String::as_str)
            .collect::<BTreeSet<_>>();
        let expected_excluded = ["BadDecl", "BadExpr", "BadStmt"]
            .into_iter()
            .collect::<BTreeSet<_>>();
        if excluded != expected_excluded {
            return Err(GoAstSchemaInventoryError::new(
                "validate",
                Some("grammar_coverage.parser_error_placeholders_excluded".to_string()),
                "only parser-error placeholder nodes may be excluded from valid-Go coverage",
            ));
        }
        if self.tokenizable_node_count == 0
            || self.grammar_ir_rule_count == 0
            || self.tokenizable_field_count == 0
        {
            return Err(GoAstSchemaInventoryError::new(
                "validate",
                Some("grammar_coverage.counts".to_string()),
                "grammar coverage counts must be positive",
            ));
        }
        if self.tokenizable_node_count != self.grammar_ir_rule_count
            || self.grammar_ir_rule_count != self.structural_model_record_count
        {
            return Err(GoAstSchemaInventoryError::new(
                "validate",
                Some("grammar_coverage.rule_counts".to_string()),
                "tokenizable nodes, GrammarIR rules, and StructuralModel records must match",
            ));
        }
        if self.tokenizable_field_count != self.structural_model_field_count {
            return Err(GoAstSchemaInventoryError::new(
                "validate",
                Some("grammar_coverage.field_counts".to_string()),
                "tokenizable fields and StructuralModel fields must match",
            ));
        }
        if !self.structural_model_lowered || self.finite_domain_count == 0 {
            return Err(GoAstSchemaInventoryError::new(
                "validate",
                Some("grammar_coverage.structural_model".to_string()),
                "grammar coverage must lower to a StructuralModel with finite domains",
            ));
        }
        if self.renderer_position_policy_field_count
            != GO_AST_SCHEMA_INVENTORY_RENDERER_POSITION_FIELD_COUNT
        {
            return Err(GoAstSchemaInventoryError::new(
                "validate",
                Some("grammar_coverage.renderer_position_policy_field_count".to_string()),
                format!(
                    "expected {GO_AST_SCHEMA_INVENTORY_RENDERER_POSITION_FIELD_COUNT} renderer-position fields, got {}",
                    self.renderer_position_policy_field_count
                ),
            ));
        }
        self.token_layout.validate()?;
        self.raw_state_machine.validate()?;
        match self.status.as_str() {
            GO_AST_SCHEMA_GRAMMAR_COVERAGE_STATUS_READY => {
                if self.first_blocker.is_some() {
                    return Err(GoAstSchemaInventoryError::new(
                        "validate",
                        Some("grammar_coverage.first_blocker".to_string()),
                        "ready grammar coverage must not carry a blocker",
                    ));
                }
                if !self.raw_state_machine.raw_state_machine_compile_ready
                    || !self.raw_state_machine.initial_state_ready
                {
                    return Err(GoAstSchemaInventoryError::new(
                        "validate",
                        Some("grammar_coverage.raw_state_machine".to_string()),
                        "ready grammar coverage must compile and initialize the raw state machine",
                    ));
                }
            }
            GO_AST_SCHEMA_GRAMMAR_COVERAGE_STATUS_BLOCKED => {
                let blocker = self.first_blocker.as_ref().ok_or_else(|| {
                    GoAstSchemaInventoryError::new(
                        "validate",
                        Some("grammar_coverage.first_blocker".to_string()),
                        "blocked grammar coverage must record the first precise blocker",
                    )
                })?;
                blocker.validate()?;
            }
            other => {
                return Err(GoAstSchemaInventoryError::new(
                    "validate",
                    Some("grammar_coverage.status".to_string()),
                    format!("unsupported grammar coverage status {other}"),
                ));
            }
        }
        if self.next_ready_task != GO_AST_SCHEMA_GRAMMAR_COVERAGE_NEXT_READY_TASK {
            return Err(GoAstSchemaInventoryError::new(
                "validate",
                Some("grammar_coverage.next_ready_task".to_string()),
                "grammar coverage must name the next syntax-level raw-tree broadening task",
            ));
        }
        for required in [
            "GrammarIR -> StructuralModel -> token layout",
            "first precise blocker",
            "not checked-source canonical comment/layout fidelity",
            "not Arena IR",
            "not BPE",
        ] {
            if !self.claim_boundary.contains(required) {
                return Err(GoAstSchemaInventoryError::new(
                    "validate",
                    Some("grammar_coverage.claim_boundary".to_string()),
                    format!("claim boundary must contain {required:?}"),
                ));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoAstSchemaGrammarCoverageTokenLayout {
    pub vocab_size: u32,
    pub pad_token: u32,
    pub structural_range_id: String,
    pub structural_range_start: u32,
    pub structural_range_end_exclusive: u32,
    pub alternative_node_start_offset: u32,
    pub alternative_node_start_capacity: usize,
    pub multi_alternative_nonterminal_node_kind_count: usize,
}

impl GoAstSchemaGrammarCoverageTokenLayout {
    fn validate(&self) -> Result<(), GoAstSchemaInventoryError> {
        if self.structural_range_id.is_empty()
            || self.structural_range_start >= self.structural_range_end_exclusive
            || self.structural_range_end_exclusive > self.vocab_size
            || self.pad_token >= self.vocab_size
        {
            return Err(GoAstSchemaInventoryError::new(
                "validate",
                Some("grammar_coverage.token_layout".to_string()),
                "grammar coverage token layout summary must retain a valid structural range",
            ));
        }
        if self.alternative_node_start_offset != RAW_TOKEN_ALTERNATIVE_NODE_START_OFFSET {
            return Err(GoAstSchemaInventoryError::new(
                "validate",
                Some("grammar_coverage.token_layout.alternative_node_start_offset".to_string()),
                "grammar coverage token layout offset must match the raw state-machine contract",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoAstSchemaGrammarCoverageRawStateMachine {
    pub raw_state_machine_core_boundary_id: String,
    pub raw_state_machine_core_boundary: String,
    pub raw_state_machine_compile_ready: bool,
    pub initial_state_ready: bool,
    pub initial_state_admissible_raw_token_count: usize,
    pub initial_state_contains_node_start: bool,
}

impl GoAstSchemaGrammarCoverageRawStateMachine {
    fn validate(&self) -> Result<(), GoAstSchemaInventoryError> {
        if self.raw_state_machine_core_boundary_id != RAW_SYNTAX_STATE_MACHINE_CORE_BOUNDARY_ID
            || self.raw_state_machine_core_boundary != RAW_SYNTAX_STATE_MACHINE_CORE_BOUNDARY
        {
            return Err(GoAstSchemaInventoryError::new(
                "validate",
                Some("grammar_coverage.raw_state_machine.boundary".to_string()),
                "grammar coverage must use the raw syntax state-machine core boundary",
            ));
        }
        if !self.raw_state_machine_compile_ready
            && (self.initial_state_ready
                || self.initial_state_admissible_raw_token_count != 0
                || self.initial_state_contains_node_start)
        {
            return Err(GoAstSchemaInventoryError::new(
                "validate",
                Some("grammar_coverage.raw_state_machine.initial_state".to_string()),
                "blocked raw state-machine coverage cannot report an initialized state",
            ));
        }
        if self.initial_state_ready
            && (!self.raw_state_machine_compile_ready
                || self.initial_state_admissible_raw_token_count == 0
                || !self.initial_state_contains_node_start)
        {
            return Err(GoAstSchemaInventoryError::new(
                "validate",
                Some("grammar_coverage.raw_state_machine.initial_state".to_string()),
                "initialized raw state-machine coverage must expose the root node-start token",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoAstSchemaGrammarCoverageBlocker {
    pub kind: String,
    pub path: String,
    pub reason: String,
    pub next_ready_task: String,
}

impl GoAstSchemaGrammarCoverageBlocker {
    fn validate(&self) -> Result<(), GoAstSchemaInventoryError> {
        if self.kind.is_empty()
            || self.path.is_empty()
            || self.reason.is_empty()
            || self.next_ready_task != GO_AST_SCHEMA_GRAMMAR_COVERAGE_BLOCKER_NEXT_READY_TASK
        {
            return Err(GoAstSchemaInventoryError::new(
                "validate",
                Some("grammar_coverage.first_blocker".to_string()),
                "grammar coverage blocker must name a kind, path, reason, and next task",
            ));
        }
        Ok(())
    }
}

impl GoAstSchemaInventoryAdapter {
    pub fn grammar_coverage_file(
        path: &Path,
        token_layout: GrammarStructuralTokenLayout,
    ) -> Result<GoAstSchemaGrammarCoverageReport, GoAstSchemaInventoryError> {
        let contents = std::fs::read_to_string(path).map_err(|error| {
            GoAstSchemaInventoryError::new(
                "read",
                Some(path.display().to_string()),
                format!("failed to read inventory: {error}"),
            )
        })?;
        Self::grammar_coverage_str(
            &contents,
            Some(path.display().to_string()),
            Some(sha256_hex(contents.as_bytes())),
            token_layout,
        )
    }

    pub fn grammar_coverage_str(
        contents: &str,
        metadata_path: Option<String>,
        metadata_sha256: Option<String>,
        token_layout: GrammarStructuralTokenLayout,
    ) -> Result<GoAstSchemaGrammarCoverageReport, GoAstSchemaInventoryError> {
        let inventory = parse_inventory_str(contents)?;
        validate_inventory(&inventory)?;
        let inventory_artifact_path = metadata_path.unwrap_or_else(|| "<memory>".to_string());
        let inventory_artifact_sha256 =
            metadata_sha256.unwrap_or_else(|| sha256_hex(contents.as_bytes()));
        let grammar = Self::lower_inventory_str(
            contents,
            Some(inventory_artifact_path.clone()),
            Some(inventory_artifact_sha256.clone()),
        )?;
        let sidecar =
            lower_grammar_ir_to_structural_model(&grammar, token_layout).map_err(|error| {
                GoAstSchemaInventoryError::new(
                    "coverage",
                    Some("grammar_coverage.structural_model".to_string()),
                    error.to_string(),
                )
            })?;
        let renderer_position_policy_field_count =
            collect_renderer_position_fields(&inventory)?.len();
        let token_layout_summary = grammar_coverage_token_layout(&sidecar)?;
        let parser_error_placeholders_excluded =
            parser_error_placeholders_excluded(&inventory, &grammar);
        let (raw_state_machine, first_blocker) =
            grammar_coverage_raw_state_machine(&sidecar, &token_layout_summary);
        let status = if first_blocker.is_none()
            && raw_state_machine.raw_state_machine_compile_ready
            && raw_state_machine.initial_state_ready
        {
            GO_AST_SCHEMA_GRAMMAR_COVERAGE_STATUS_READY
        } else {
            GO_AST_SCHEMA_GRAMMAR_COVERAGE_STATUS_BLOCKED
        };
        let report = GoAstSchemaGrammarCoverageReport {
            artifact_kind: GO_AST_SCHEMA_GRAMMAR_COVERAGE_ARTIFACT_KIND.to_string(),
            version: GO_AST_SCHEMA_INVENTORY_VERSION,
            status: status.to_string(),
            inventory_artifact_path,
            inventory_artifact_sha256,
            grammar_id: grammar.grammar_id.clone(),
            pinned_go_version: inventory.go.version,
            inventory_surface_checked: true,
            parser_error_placeholders_excluded,
            tokenizable_node_count: grammar.rules.len(),
            grammar_ir_rule_count: grammar.rules.len(),
            tokenizable_field_count: grammar
                .rules
                .iter()
                .map(|rule| rule.fields.len())
                .sum::<usize>(),
            structural_model_lowered: true,
            structural_model_record_count: sidecar.structural_model.records.len(),
            structural_model_field_count: sidecar
                .structural_model
                .records
                .iter()
                .map(|record| record.fields.len())
                .sum::<usize>(),
            finite_domain_count: sidecar.structural_model.finite_domains.len(),
            renderer_position_policy_field_count,
            token_layout: token_layout_summary,
            raw_state_machine,
            first_blocker,
            next_ready_task: GO_AST_SCHEMA_GRAMMAR_COVERAGE_NEXT_READY_TASK.to_string(),
            claim_boundary: GO_AST_SCHEMA_GRAMMAR_COVERAGE_CLAIM_BOUNDARY.to_string(),
        };
        report.validate()?;
        Ok(report)
    }
}

fn grammar_coverage_token_layout(
    sidecar: &GrammarStructuralModelSidecar,
) -> Result<GoAstSchemaGrammarCoverageTokenLayout, GoAstSchemaInventoryError> {
    let token_layout = &sidecar.structural_model.token_ranges;
    let structural_range = token_layout
        .ranges
        .iter()
        .find(|range| range.role == "structural_decisions")
        .ok_or_else(|| {
            GoAstSchemaInventoryError::new(
                "coverage",
                Some("grammar_coverage.token_layout.structural_decisions".to_string()),
                "Go AST grammar coverage requires a structural_decisions token range",
            )
        })?;
    let first_alternative_token = structural_range
        .start
        .saturating_add(RAW_TOKEN_ALTERNATIVE_NODE_START_OFFSET);
    let alternative_node_start_capacity = structural_range
        .end_exclusive
        .saturating_sub(first_alternative_token) as usize;
    let multi_alternative_nonterminal_node_kind_count =
        multi_alternative_nonterminal_node_kinds(sidecar).len();

    Ok(GoAstSchemaGrammarCoverageTokenLayout {
        vocab_size: token_layout.vocab_size,
        pad_token: token_layout.pad_token,
        structural_range_id: structural_range.id.clone(),
        structural_range_start: structural_range.start,
        structural_range_end_exclusive: structural_range.end_exclusive,
        alternative_node_start_offset: RAW_TOKEN_ALTERNATIVE_NODE_START_OFFSET,
        alternative_node_start_capacity,
        multi_alternative_nonterminal_node_kind_count,
    })
}

fn grammar_coverage_raw_state_machine(
    sidecar: &GrammarStructuralModelSidecar,
    token_layout: &GoAstSchemaGrammarCoverageTokenLayout,
) -> (
    GoAstSchemaGrammarCoverageRawStateMachine,
    Option<GoAstSchemaGrammarCoverageBlocker>,
) {
    let mut raw_state_machine = GoAstSchemaGrammarCoverageRawStateMachine {
        raw_state_machine_core_boundary_id: RAW_SYNTAX_STATE_MACHINE_CORE_BOUNDARY_ID.to_string(),
        raw_state_machine_core_boundary: RAW_SYNTAX_STATE_MACHINE_CORE_BOUNDARY.to_string(),
        raw_state_machine_compile_ready: false,
        initial_state_ready: false,
        initial_state_admissible_raw_token_count: 0,
        initial_state_contains_node_start: false,
    };

    if token_layout.multi_alternative_nonterminal_node_kind_count
        > token_layout.alternative_node_start_capacity
    {
        return (
            raw_state_machine,
            Some(GoAstSchemaGrammarCoverageBlocker {
                kind: "raw_state_machine_alternative_node_start_capacity".to_string(),
                path: "token_layout.structural_decisions".to_string(),
                reason: format!(
                    "{} multi-alternative nonterminal node kinds exceed structural token capacity {} in {}",
                    token_layout.multi_alternative_nonterminal_node_kind_count,
                    token_layout.alternative_node_start_capacity,
                    token_layout.structural_range_id
                ),
                next_ready_task: GO_AST_SCHEMA_GRAMMAR_COVERAGE_BLOCKER_NEXT_READY_TASK.to_string(),
            }),
        );
    }

    if let Err(error) =
        crate::syntax_go_source_instance_projection::node_start_alternative_token_ids_from_sidecar(
            sidecar,
        )
    {
        return (
            raw_state_machine,
            Some(GoAstSchemaGrammarCoverageBlocker {
                kind: "raw_state_machine_alternative_node_start_tokens".to_string(),
                path: "token_layout.structural_decisions".to_string(),
                reason: error.to_string(),
                next_ready_task: GO_AST_SCHEMA_GRAMMAR_COVERAGE_BLOCKER_NEXT_READY_TASK.to_string(),
            }),
        );
    }

    let model = match CompiledSyntaxModel::from_sidecar(sidecar) {
        Ok(model) => model,
        Err(error) => {
            return (
                raw_state_machine,
                Some(GoAstSchemaGrammarCoverageBlocker {
                    kind: "raw_state_machine_compile".to_string(),
                    path: "raw_state_machine.compile".to_string(),
                    reason: error.to_string(),
                    next_ready_task: GO_AST_SCHEMA_GRAMMAR_COVERAGE_BLOCKER_NEXT_READY_TASK
                        .to_string(),
                }),
            );
        }
    };
    raw_state_machine.raw_state_machine_compile_ready = true;

    let state = match model.state_from_prefix(&[]) {
        Ok(state) => state,
        Err(error) => {
            return (
                raw_state_machine,
                Some(GoAstSchemaGrammarCoverageBlocker {
                    kind: "raw_state_machine_initial_state".to_string(),
                    path: "raw_state_machine.empty_prefix".to_string(),
                    reason: format!("{error:?}"),
                    next_ready_task: GO_AST_SCHEMA_GRAMMAR_COVERAGE_BLOCKER_NEXT_READY_TASK
                        .to_string(),
                }),
            );
        }
    };
    let root_node_start = token_layout.structural_range_start + RAW_NODE_START_OFFSET;
    raw_state_machine.initial_state_admissible_raw_token_count =
        state.admissible.finite_raw_token_ids.len()
            + state
                .admissible
                .raw_token_ranges
                .iter()
                .map(|range| (range.end_exclusive - range.start) as usize)
                .sum::<usize>();
    raw_state_machine.initial_state_contains_node_start = state
        .admissible
        .finite_raw_token_ids
        .contains(&root_node_start);
    raw_state_machine.initial_state_ready =
        raw_state_machine.initial_state_admissible_raw_token_count > 0
            && raw_state_machine.initial_state_contains_node_start;

    if raw_state_machine.initial_state_ready {
        (raw_state_machine, None)
    } else {
        (
            raw_state_machine,
            Some(GoAstSchemaGrammarCoverageBlocker {
                kind: "raw_state_machine_initial_admissible_tokens".to_string(),
                path: "raw_state_machine.empty_prefix.admissible".to_string(),
                reason: "empty-prefix raw state did not expose the root node-start token"
                    .to_string(),
                next_ready_task: GO_AST_SCHEMA_GRAMMAR_COVERAGE_BLOCKER_NEXT_READY_TASK.to_string(),
            }),
        )
    }
}

fn multi_alternative_nonterminal_node_kinds(
    sidecar: &GrammarStructuralModelSidecar,
) -> BTreeSet<String> {
    let mut node_kinds = BTreeSet::new();
    for domain in &sidecar.structural_model.finite_domains {
        if domain.alternatives.len() <= 1 {
            continue;
        }
        for alternative in &domain.alternatives {
            if alternative.terminal == GrammarTerminalKind::Nonterminal {
                node_kinds.insert(alternative.node_kind.clone());
            }
        }
    }
    node_kinds
}

fn parser_error_placeholders_excluded(
    inventory: &GoAstSchemaInventory,
    grammar: &GrammarIr,
) -> Vec<String> {
    let grammar_rule_names = grammar
        .rules
        .iter()
        .map(|rule| rule.name.as_str())
        .collect::<BTreeSet<_>>();
    inventory
        .node_structs
        .iter()
        .filter(|node| !grammar_rule_names.contains(node.name.as_str()))
        .map(|node| node.name.clone())
        .collect()
}
