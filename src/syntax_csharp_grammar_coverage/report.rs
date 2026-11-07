use super::{
    CSHARP_SYNTAX_GRAMMAR_COVERAGE_ARTIFACT_KIND,
    CSHARP_SYNTAX_GRAMMAR_COVERAGE_BLOCKER_NEXT_READY_TASK,
    CSHARP_SYNTAX_GRAMMAR_COVERAGE_CLAIM_BOUNDARY,
    CSHARP_SYNTAX_GRAMMAR_COVERAGE_CROSS_TARGET_FRAMING,
    CSHARP_SYNTAX_GRAMMAR_COVERAGE_NEXT_READY_TASK, CSHARP_SYNTAX_GRAMMAR_COVERAGE_SCOPE,
    CSHARP_SYNTAX_GRAMMAR_COVERAGE_STATUS_BLOCKED, CSHARP_SYNTAX_GRAMMAR_COVERAGE_STATUS_READY,
    CSHARP_SYNTAX_GRAMMAR_COVERAGE_VERSION, CSharpSyntaxGrammarCoverageError,
};
use crate::syntax_go_source_instance_projection::RAW_TOKEN_ALTERNATIVE_NODE_START_OFFSET;
use crate::syntax_production_route::CODEC_REGISTRY_SOURCE_OF_TRUTH;
use crate::syntax_state_machine::{
    RAW_SYNTAX_STATE_MACHINE_CORE_BOUNDARY, RAW_SYNTAX_STATE_MACHINE_CORE_BOUNDARY_ID,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CSharpSyntaxGrammarCoverageReport {
    pub artifact_kind: String,
    pub version: u32,
    pub status: String,
    pub target_id: String,
    pub coverage_scope: String,
    pub syntax_codec_registry_source_of_truth: String,
    pub registered_grammar_ids_sha256: String,
    pub registry_entry_count: usize,
    pub covered_entry_count: usize,
    pub blocked_entry_count: usize,
    pub shared_token_layout: CSharpSyntaxGrammarCoverageTokenLayout,
    pub totals: CSharpSyntaxGrammarCoverageTotals,
    pub entries: Vec<CSharpSyntaxGrammarCoverageEntry>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_blocker: Option<CSharpSyntaxGrammarCoverageBlocker>,
    pub cross_target_framing: String,
    pub claim_boundary: String,
    pub arena_ir_proof_used: bool,
    pub source_program_type_correctness_claim_allowed: bool,
    pub bpe_layer_defines_validity: bool,
    pub stable_backend_promotion_claim_allowed: bool,
    pub json_schema_backend_changed: bool,
    pub next_ready_task: String,
}

impl CSharpSyntaxGrammarCoverageReport {
    pub fn validate(&self) -> Result<(), CSharpSyntaxGrammarCoverageError> {
        if self.artifact_kind != CSHARP_SYNTAX_GRAMMAR_COVERAGE_ARTIFACT_KIND {
            return Err(CSharpSyntaxGrammarCoverageError::new(
                "validate",
                Some("grammar_coverage.artifact_kind".to_string()),
                format!(
                    "expected artifact kind {CSHARP_SYNTAX_GRAMMAR_COVERAGE_ARTIFACT_KIND}, got {}",
                    self.artifact_kind
                ),
            ));
        }
        if self.version != CSHARP_SYNTAX_GRAMMAR_COVERAGE_VERSION {
            return Err(CSharpSyntaxGrammarCoverageError::new(
                "validate",
                Some("grammar_coverage.version".to_string()),
                format!(
                    "expected version {CSHARP_SYNTAX_GRAMMAR_COVERAGE_VERSION}, got {}",
                    self.version
                ),
            ));
        }
        if self.target_id != "csharp" || self.coverage_scope != CSHARP_SYNTAX_GRAMMAR_COVERAGE_SCOPE
        {
            return Err(CSharpSyntaxGrammarCoverageError::new(
                "validate",
                Some("grammar_coverage.scope".to_string()),
                "C# grammar coverage must be scoped to registered C# syntax codec entries",
            ));
        }
        if self.syntax_codec_registry_source_of_truth != CODEC_REGISTRY_SOURCE_OF_TRUTH {
            return Err(CSharpSyntaxGrammarCoverageError::new(
                "validate",
                Some("grammar_coverage.syntax_codec_registry_source_of_truth".to_string()),
                "grammar coverage must point at the syntax codec registry source of truth",
            ));
        }
        if self.registry_entry_count == 0
            || self.registry_entry_count != self.entries.len()
            || self.covered_entry_count + self.blocked_entry_count != self.registry_entry_count
            || self.registered_grammar_ids_sha256.len() != 64
        {
            return Err(CSharpSyntaxGrammarCoverageError::new(
                "validate",
                Some("grammar_coverage.registry_counts".to_string()),
                "registry counts and grammar id hash must describe the covered C# entries",
            ));
        }
        self.shared_token_layout.validate()?;
        for entry in &self.entries {
            entry.validate()?;
        }
        self.totals.validate(self)?;
        self.validate_status()?;
        self.validate_claim_boundaries()?;
        if self.next_ready_task != CSHARP_SYNTAX_GRAMMAR_COVERAGE_NEXT_READY_TASK {
            return Err(CSharpSyntaxGrammarCoverageError::new(
                "validate",
                Some("grammar_coverage.next_ready_task".to_string()),
                "C# grammar coverage must name the next C# syntax parity task",
            ));
        }
        Ok(())
    }

    fn validate_status(&self) -> Result<(), CSharpSyntaxGrammarCoverageError> {
        match self.status.as_str() {
            CSHARP_SYNTAX_GRAMMAR_COVERAGE_STATUS_READY => {
                if self.blocked_entry_count != 0 || self.first_blocker.is_some() {
                    return Err(CSharpSyntaxGrammarCoverageError::new(
                        "validate",
                        Some("grammar_coverage.first_blocker".to_string()),
                        "ready C# grammar coverage must not carry blockers",
                    ));
                }
            }
            CSHARP_SYNTAX_GRAMMAR_COVERAGE_STATUS_BLOCKED => {
                if self.blocked_entry_count == 0 || self.first_blocker.is_none() {
                    return Err(CSharpSyntaxGrammarCoverageError::new(
                        "validate",
                        Some("grammar_coverage.first_blocker".to_string()),
                        "blocked C# grammar coverage must record the first precise blocker",
                    ));
                }
            }
            other => {
                return Err(CSharpSyntaxGrammarCoverageError::new(
                    "validate",
                    Some("grammar_coverage.status".to_string()),
                    format!("unsupported C# grammar coverage status {other}"),
                ));
            }
        }
        Ok(())
    }

    fn validate_claim_boundaries(&self) -> Result<(), CSharpSyntaxGrammarCoverageError> {
        for required in [
            "Go AST grammar coverage",
            "shared raw state-machine boundary",
            "GrammarIR -> StructuralModel -> token layout",
            "first precise syntax-level blocker",
        ] {
            if !self.cross_target_framing.contains(required) {
                return Err(CSharpSyntaxGrammarCoverageError::new(
                    "validate",
                    Some("grammar_coverage.cross_target_framing".to_string()),
                    format!("cross-target framing must contain {required:?}"),
                ));
            }
        }
        for required in [
            "GrammarIR -> StructuralModel -> token layout",
            "raw syntax state-machine boundary",
            "not C# arena IR",
            "not source-program type correctness",
            "not BPE",
            "not stable-backend promotion",
        ] {
            if !self.claim_boundary.contains(required) {
                return Err(CSharpSyntaxGrammarCoverageError::new(
                    "validate",
                    Some("grammar_coverage.claim_boundary".to_string()),
                    format!("claim boundary must contain {required:?}"),
                ));
            }
        }
        if self.cross_target_framing != CSHARP_SYNTAX_GRAMMAR_COVERAGE_CROSS_TARGET_FRAMING
            || self.claim_boundary != CSHARP_SYNTAX_GRAMMAR_COVERAGE_CLAIM_BOUNDARY
            || self.arena_ir_proof_used
            || self.source_program_type_correctness_claim_allowed
            || self.bpe_layer_defines_validity
            || self.stable_backend_promotion_claim_allowed
            || self.json_schema_backend_changed
        {
            return Err(CSharpSyntaxGrammarCoverageError::new(
                "validate",
                Some("grammar_coverage.claim_controls".to_string()),
                "C# grammar coverage must keep arena IR, type correctness, BPE, stable backend, and JSON backend claims out of scope",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CSharpSyntaxGrammarCoverageEntry {
    pub target_id: String,
    pub grammar_id: String,
    pub metadata_path: String,
    pub metadata_sha256: String,
    pub completed_promotion_class: String,
    pub generated_test_marker_count: usize,
    pub grammar_ir_rule_count: usize,
    pub grammar_ir_field_count: usize,
    pub structural_model_record_count: usize,
    pub structural_model_field_count: usize,
    pub finite_domain_count: usize,
    pub list_count: usize,
    pub optional_count: usize,
    pub reference_count: usize,
    pub payload_leaf_count: usize,
    pub decision_graph_node_count: usize,
    pub decision_graph_edge_count: usize,
    pub multi_alternative_nonterminal_node_kind_count: usize,
    pub raw_state_machine: CSharpSyntaxGrammarCoverageRawStateMachine,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_blocker: Option<CSharpSyntaxGrammarCoverageBlocker>,
}

impl CSharpSyntaxGrammarCoverageEntry {
    fn validate(&self) -> Result<(), CSharpSyntaxGrammarCoverageError> {
        if self.target_id != "csharp"
            || self.grammar_id.is_empty()
            || self.metadata_path.is_empty()
            || self.metadata_sha256.len() != 64
            || self.completed_promotion_class.is_empty()
            || self.generated_test_marker_count == 0
        {
            return Err(CSharpSyntaxGrammarCoverageError::new(
                "validate",
                Some(format!("entries.{}", self.grammar_id)),
                "C# coverage entries must preserve registry metadata",
            ));
        }
        if self.grammar_ir_rule_count == 0
            || self.grammar_ir_rule_count != self.structural_model_record_count
            || self.grammar_ir_field_count != self.structural_model_field_count
            || self.finite_domain_count == 0
            || self.decision_graph_node_count == 0
            || self.decision_graph_edge_count == 0
        {
            return Err(CSharpSyntaxGrammarCoverageError::new(
                "validate",
                Some(format!("entries.{}.counts", self.grammar_id)),
                "C# coverage entries must lower through GrammarIR and StructuralModel",
            ));
        }
        self.raw_state_machine.validate(&self.grammar_id)?;
        if let Some(blocker) = &self.first_blocker {
            blocker.validate()?;
        } else if !self.raw_state_machine.raw_state_machine_compile_ready
            || !self.raw_state_machine.initial_state_ready
        {
            return Err(CSharpSyntaxGrammarCoverageError::new(
                "validate",
                Some(format!("entries.{}.raw_state_machine", self.grammar_id)),
                "unblocked C# coverage entries must compile and initialize the raw state machine",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CSharpSyntaxGrammarCoverageTotals {
    pub grammar_ir_rule_count: usize,
    pub grammar_ir_field_count: usize,
    pub structural_model_record_count: usize,
    pub structural_model_field_count: usize,
    pub finite_domain_count: usize,
    pub list_count: usize,
    pub optional_count: usize,
    pub reference_count: usize,
    pub payload_leaf_count: usize,
    pub decision_graph_node_count: usize,
    pub decision_graph_edge_count: usize,
}

impl CSharpSyntaxGrammarCoverageTotals {
    pub(crate) fn from_entries(entries: &[CSharpSyntaxGrammarCoverageEntry]) -> Self {
        Self {
            grammar_ir_rule_count: entries
                .iter()
                .map(|entry| entry.grammar_ir_rule_count)
                .sum(),
            grammar_ir_field_count: entries
                .iter()
                .map(|entry| entry.grammar_ir_field_count)
                .sum(),
            structural_model_record_count: entries
                .iter()
                .map(|entry| entry.structural_model_record_count)
                .sum(),
            structural_model_field_count: entries
                .iter()
                .map(|entry| entry.structural_model_field_count)
                .sum(),
            finite_domain_count: entries.iter().map(|entry| entry.finite_domain_count).sum(),
            list_count: entries.iter().map(|entry| entry.list_count).sum(),
            optional_count: entries.iter().map(|entry| entry.optional_count).sum(),
            reference_count: entries.iter().map(|entry| entry.reference_count).sum(),
            payload_leaf_count: entries.iter().map(|entry| entry.payload_leaf_count).sum(),
            decision_graph_node_count: entries
                .iter()
                .map(|entry| entry.decision_graph_node_count)
                .sum(),
            decision_graph_edge_count: entries
                .iter()
                .map(|entry| entry.decision_graph_edge_count)
                .sum(),
        }
    }

    fn validate(
        &self,
        report: &CSharpSyntaxGrammarCoverageReport,
    ) -> Result<(), CSharpSyntaxGrammarCoverageError> {
        if self != &Self::from_entries(&report.entries) {
            return Err(CSharpSyntaxGrammarCoverageError::new(
                "validate",
                Some("grammar_coverage.totals".to_string()),
                "C# grammar coverage totals must equal the covered entries",
            ));
        }
        if self.grammar_ir_rule_count == 0
            || self.grammar_ir_rule_count != self.structural_model_record_count
            || self.grammar_ir_field_count != self.structural_model_field_count
            || self.finite_domain_count == 0
        {
            return Err(CSharpSyntaxGrammarCoverageError::new(
                "validate",
                Some("grammar_coverage.totals".to_string()),
                "C# grammar coverage totals must prove GrammarIR and StructuralModel parity",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CSharpSyntaxGrammarCoverageTokenLayout {
    pub vocab_size: u32,
    pub pad_token: u32,
    pub structural_range_id: String,
    pub structural_range_start: u32,
    pub structural_range_end_exclusive: u32,
    pub alternative_node_start_offset: u32,
    pub alternative_node_start_capacity: usize,
    pub max_multi_alternative_nonterminal_node_kind_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_multi_alternative_grammar_id: Option<String>,
}

impl CSharpSyntaxGrammarCoverageTokenLayout {
    pub(crate) fn validate(&self) -> Result<(), CSharpSyntaxGrammarCoverageError> {
        if self.structural_range_id.is_empty()
            || self.structural_range_start >= self.structural_range_end_exclusive
            || self.structural_range_end_exclusive > self.vocab_size
            || self.pad_token >= self.vocab_size
        {
            return Err(CSharpSyntaxGrammarCoverageError::new(
                "validate",
                Some("grammar_coverage.token_layout".to_string()),
                "C# grammar coverage token layout must retain a valid structural range",
            ));
        }
        if self.alternative_node_start_offset != RAW_TOKEN_ALTERNATIVE_NODE_START_OFFSET {
            return Err(CSharpSyntaxGrammarCoverageError::new(
                "validate",
                Some("grammar_coverage.token_layout.alternative_node_start_offset".to_string()),
                "C# grammar coverage token layout offset must match the raw state-machine contract",
            ));
        }
        if self.max_multi_alternative_nonterminal_node_kind_count
            > self.alternative_node_start_capacity
        {
            return Err(CSharpSyntaxGrammarCoverageError::new(
                "validate",
                Some("grammar_coverage.token_layout.alternative_node_start_capacity".to_string()),
                "C# grammar coverage exceeds the structural token layout capacity",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CSharpSyntaxGrammarCoverageRawStateMachine {
    pub raw_state_machine_core_boundary_id: String,
    pub raw_state_machine_core_boundary: String,
    pub raw_state_machine_compile_ready: bool,
    pub initial_state_ready: bool,
    pub initial_state_admissible_raw_token_count: usize,
    pub initial_state_contains_node_start: bool,
}

impl CSharpSyntaxGrammarCoverageRawStateMachine {
    fn validate(&self, grammar_id: &str) -> Result<(), CSharpSyntaxGrammarCoverageError> {
        if self.raw_state_machine_core_boundary_id != RAW_SYNTAX_STATE_MACHINE_CORE_BOUNDARY_ID
            || self.raw_state_machine_core_boundary != RAW_SYNTAX_STATE_MACHINE_CORE_BOUNDARY
        {
            return Err(CSharpSyntaxGrammarCoverageError::new(
                "validate",
                Some(format!("entries.{grammar_id}.raw_state_machine.boundary")),
                "C# grammar coverage must use the raw syntax state-machine core boundary",
            ));
        }
        if !self.raw_state_machine_compile_ready
            && (self.initial_state_ready
                || self.initial_state_admissible_raw_token_count != 0
                || self.initial_state_contains_node_start)
        {
            return Err(CSharpSyntaxGrammarCoverageError::new(
                "validate",
                Some(format!(
                    "entries.{grammar_id}.raw_state_machine.initial_state"
                )),
                "blocked C# raw state-machine coverage cannot report an initialized state",
            ));
        }
        if self.initial_state_ready
            && (!self.raw_state_machine_compile_ready
                || self.initial_state_admissible_raw_token_count == 0
                || !self.initial_state_contains_node_start)
        {
            return Err(CSharpSyntaxGrammarCoverageError::new(
                "validate",
                Some(format!(
                    "entries.{grammar_id}.raw_state_machine.initial_state"
                )),
                "initialized C# raw state-machine coverage must expose the root node-start token",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CSharpSyntaxGrammarCoverageBlocker {
    pub grammar_id: String,
    pub metadata_path: String,
    pub kind: String,
    pub path: String,
    pub reason: String,
    pub next_ready_task: String,
}

impl CSharpSyntaxGrammarCoverageBlocker {
    fn validate(&self) -> Result<(), CSharpSyntaxGrammarCoverageError> {
        if self.grammar_id.is_empty()
            || self.metadata_path.is_empty()
            || self.kind.is_empty()
            || self.path.is_empty()
            || self.reason.is_empty()
            || self.next_ready_task != CSHARP_SYNTAX_GRAMMAR_COVERAGE_BLOCKER_NEXT_READY_TASK
        {
            return Err(CSharpSyntaxGrammarCoverageError::new(
                "validate",
                Some("grammar_coverage.first_blocker".to_string()),
                "C# grammar coverage blocker must name a grammar id, path, reason, and next task",
            ));
        }
        Ok(())
    }
}
