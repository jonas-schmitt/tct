use super::{
    CSHARP_SYNTAX_GRAMMAR_COVERAGE_ARTIFACT_KIND,
    CSHARP_SYNTAX_GRAMMAR_COVERAGE_BLOCKER_NEXT_READY_TASK,
    CSHARP_SYNTAX_GRAMMAR_COVERAGE_CLAIM_BOUNDARY,
    CSHARP_SYNTAX_GRAMMAR_COVERAGE_CROSS_TARGET_FRAMING,
    CSHARP_SYNTAX_GRAMMAR_COVERAGE_NEXT_READY_TASK, CSHARP_SYNTAX_GRAMMAR_COVERAGE_SCOPE,
    CSHARP_SYNTAX_GRAMMAR_COVERAGE_STATUS_BLOCKED, CSHARP_SYNTAX_GRAMMAR_COVERAGE_STATUS_READY,
    CSHARP_SYNTAX_GRAMMAR_COVERAGE_VERSION, CSharpSyntaxGrammarCoverageBlocker,
    CSharpSyntaxGrammarCoverageEntry, CSharpSyntaxGrammarCoverageError,
    CSharpSyntaxGrammarCoverageRawStateMachine, CSharpSyntaxGrammarCoverageReport,
    CSharpSyntaxGrammarCoverageTokenLayout, CSharpSyntaxGrammarCoverageTotals,
};
use crate::grammar_ir::GrammarTerminalKind;
use crate::grammar_ir_structural_model::{
    GrammarStructuralModelSidecar, GrammarStructuralTokenLayout,
};
use crate::provenance::sha256_hex;
use crate::syntax_generated_codec::csharp_syntax_token_layout;
use crate::syntax_go_source_instance_projection::{
    RAW_TOKEN_ALTERNATIVE_NODE_START_OFFSET, node_start_alternative_token_ids_from_sidecar,
};
use crate::syntax_production_route::{
    CODEC_REGISTRY_SOURCE_OF_TRUTH, SYNTAX_CODEC_REGISTRY, SyntaxCodecRegistryEntry,
    lower_registered_parser_backed_syntax_entry,
};
use crate::syntax_state_machine::{
    CompiledSyntaxModel, RAW_NODE_START_OFFSET, RAW_SYNTAX_STATE_MACHINE_CORE_BOUNDARY,
    RAW_SYNTAX_STATE_MACHINE_CORE_BOUNDARY_ID,
};
use std::collections::BTreeSet;

impl CSharpSyntaxGrammarCoverageReport {
    pub fn from_registered_syntax_codec_entries() -> Result<Self, CSharpSyntaxGrammarCoverageError>
    {
        let registered_entries = SYNTAX_CODEC_REGISTRY
            .iter()
            .filter(|entry| entry.target_id == "csharp")
            .collect::<Vec<_>>();
        if registered_entries.is_empty() {
            return Err(CSharpSyntaxGrammarCoverageError::new(
                "build",
                Some("syntax_codec_registry".to_string()),
                "C# grammar coverage requires at least one registered C# syntax codec entry",
            ));
        }

        let shared_token_layout =
            grammar_coverage_token_layout(&csharp_syntax_token_layout(), None, None)?;
        let mut entries = Vec::with_capacity(registered_entries.len());
        let mut first_blocker = None;

        for entry in registered_entries {
            let coverage_entry = grammar_coverage_entry(entry, &shared_token_layout)?;
            if first_blocker.is_none()
                && let Some(blocker) = &coverage_entry.first_blocker
            {
                first_blocker = Some(blocker.clone());
            }
            entries.push(coverage_entry);
        }

        let blocked_entry_count = entries
            .iter()
            .filter(|entry| entry.first_blocker.is_some())
            .count();
        let covered_entry_count = entries.len() - blocked_entry_count;
        let mut shared_token_layout = shared_token_layout;
        shared_token_layout.max_multi_alternative_nonterminal_node_kind_count = entries
            .iter()
            .map(|entry| entry.multi_alternative_nonterminal_node_kind_count)
            .max()
            .unwrap_or(0);
        shared_token_layout.max_multi_alternative_grammar_id = entries
            .iter()
            .find(|entry| {
                entry.multi_alternative_nonterminal_node_kind_count
                    == shared_token_layout.max_multi_alternative_nonterminal_node_kind_count
            })
            .map(|entry| entry.grammar_id.clone());

        let status = if blocked_entry_count == 0 {
            CSHARP_SYNTAX_GRAMMAR_COVERAGE_STATUS_READY
        } else {
            CSHARP_SYNTAX_GRAMMAR_COVERAGE_STATUS_BLOCKED
        };
        let grammar_ids = entries
            .iter()
            .map(|entry| entry.grammar_id.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        let report = Self {
            artifact_kind: CSHARP_SYNTAX_GRAMMAR_COVERAGE_ARTIFACT_KIND.to_string(),
            version: CSHARP_SYNTAX_GRAMMAR_COVERAGE_VERSION,
            status: status.to_string(),
            target_id: "csharp".to_string(),
            coverage_scope: CSHARP_SYNTAX_GRAMMAR_COVERAGE_SCOPE.to_string(),
            syntax_codec_registry_source_of_truth: CODEC_REGISTRY_SOURCE_OF_TRUTH.to_string(),
            registered_grammar_ids_sha256: sha256_hex(grammar_ids.as_bytes()),
            registry_entry_count: entries.len(),
            covered_entry_count,
            blocked_entry_count,
            shared_token_layout,
            totals: CSharpSyntaxGrammarCoverageTotals::from_entries(&entries),
            entries,
            first_blocker,
            cross_target_framing: CSHARP_SYNTAX_GRAMMAR_COVERAGE_CROSS_TARGET_FRAMING.to_string(),
            claim_boundary: CSHARP_SYNTAX_GRAMMAR_COVERAGE_CLAIM_BOUNDARY.to_string(),
            arena_ir_proof_used: false,
            source_program_type_correctness_claim_allowed: false,
            bpe_layer_defines_validity: false,
            stable_backend_promotion_claim_allowed: false,
            json_schema_backend_changed: false,
            next_ready_task: CSHARP_SYNTAX_GRAMMAR_COVERAGE_NEXT_READY_TASK.to_string(),
        };
        report.validate()?;
        Ok(report)
    }
}

fn grammar_coverage_entry(
    entry: &SyntaxCodecRegistryEntry,
    shared_token_layout: &CSharpSyntaxGrammarCoverageTokenLayout,
) -> Result<CSharpSyntaxGrammarCoverageEntry, CSharpSyntaxGrammarCoverageError> {
    let (grammar, sidecar) =
        lower_registered_parser_backed_syntax_entry(entry).map_err(|error| {
            CSharpSyntaxGrammarCoverageError::new(
                "lower",
                Some(entry.grammar_id.to_string()),
                format!("failed to lower registered C# syntax entry: {error}"),
            )
        })?;
    let token_layout = grammar_coverage_token_layout(
        &sidecar.structural_model.token_ranges,
        Some(&sidecar),
        Some(entry.grammar_id),
    )?;
    if token_layout.vocab_size != shared_token_layout.vocab_size
        || token_layout.pad_token != shared_token_layout.pad_token
        || token_layout.structural_range_id != shared_token_layout.structural_range_id
        || token_layout.structural_range_start != shared_token_layout.structural_range_start
        || token_layout.structural_range_end_exclusive
            != shared_token_layout.structural_range_end_exclusive
    {
        return Err(CSharpSyntaxGrammarCoverageError::new(
            "coverage",
            Some(format!("entries.{}.token_layout", entry.grammar_id)),
            "registered C# syntax entries must share the C# syntax token layout",
        ));
    }
    let (raw_state_machine, first_blocker) =
        grammar_coverage_raw_state_machine(entry, &sidecar, &token_layout);

    Ok(CSharpSyntaxGrammarCoverageEntry {
        target_id: entry.target_id.to_string(),
        grammar_id: entry.grammar_id.to_string(),
        metadata_path: entry.metadata_path.to_string(),
        metadata_sha256: grammar
            .provenance
            .metadata_sha256
            .clone()
            .unwrap_or_default(),
        completed_promotion_class: entry.completed_promotion_class.to_string(),
        generated_test_marker_count: entry.generated_test_markers.len(),
        grammar_ir_rule_count: grammar.rules.len(),
        grammar_ir_field_count: grammar
            .rules
            .iter()
            .map(|rule| rule.fields.len())
            .sum::<usize>(),
        structural_model_record_count: sidecar.structural_model.records.len(),
        structural_model_field_count: sidecar
            .structural_model
            .records
            .iter()
            .map(|record| record.fields.len())
            .sum::<usize>(),
        finite_domain_count: sidecar.structural_model.finite_domains.len(),
        list_count: sidecar.structural_model.lists.len(),
        optional_count: sidecar.structural_model.optionals.len(),
        reference_count: sidecar.structural_model.references.len(),
        payload_leaf_count: sidecar.structural_model.payload_leaves.len(),
        decision_graph_node_count: sidecar.structural_model.decision_graph.nodes.len(),
        decision_graph_edge_count: sidecar.structural_model.decision_graph.edges.len(),
        multi_alternative_nonterminal_node_kind_count: token_layout
            .max_multi_alternative_nonterminal_node_kind_count,
        raw_state_machine,
        first_blocker,
    })
}

fn grammar_coverage_token_layout(
    token_layout: &GrammarStructuralTokenLayout,
    sidecar: Option<&GrammarStructuralModelSidecar>,
    grammar_id: Option<&str>,
) -> Result<CSharpSyntaxGrammarCoverageTokenLayout, CSharpSyntaxGrammarCoverageError> {
    let structural_range = token_layout
        .ranges
        .iter()
        .find(|range| range.role == "structural_decisions")
        .ok_or_else(|| {
            CSharpSyntaxGrammarCoverageError::new(
                "coverage",
                grammar_id.map(|id| format!("entries.{id}.token_layout.structural_decisions")),
                "C# grammar coverage requires a structural_decisions token range",
            )
        })?;
    let first_alternative_token = structural_range
        .start
        .saturating_add(RAW_TOKEN_ALTERNATIVE_NODE_START_OFFSET);
    let alternative_node_start_capacity = structural_range
        .end_exclusive
        .saturating_sub(first_alternative_token) as usize;
    let multi_alternative_nonterminal_node_kind_count = sidecar
        .map(multi_alternative_nonterminal_node_kinds)
        .unwrap_or_default()
        .len();

    Ok(CSharpSyntaxGrammarCoverageTokenLayout {
        vocab_size: token_layout.vocab_size,
        pad_token: token_layout.pad_token,
        structural_range_id: structural_range.id.clone(),
        structural_range_start: structural_range.start,
        structural_range_end_exclusive: structural_range.end_exclusive,
        alternative_node_start_offset: RAW_TOKEN_ALTERNATIVE_NODE_START_OFFSET,
        alternative_node_start_capacity,
        max_multi_alternative_nonterminal_node_kind_count:
            multi_alternative_nonterminal_node_kind_count,
        max_multi_alternative_grammar_id: grammar_id.map(str::to_string),
    })
}

fn grammar_coverage_raw_state_machine(
    entry: &SyntaxCodecRegistryEntry,
    sidecar: &GrammarStructuralModelSidecar,
    token_layout: &CSharpSyntaxGrammarCoverageTokenLayout,
) -> (
    CSharpSyntaxGrammarCoverageRawStateMachine,
    Option<CSharpSyntaxGrammarCoverageBlocker>,
) {
    let mut raw_state_machine = CSharpSyntaxGrammarCoverageRawStateMachine {
        raw_state_machine_core_boundary_id: RAW_SYNTAX_STATE_MACHINE_CORE_BOUNDARY_ID.to_string(),
        raw_state_machine_core_boundary: RAW_SYNTAX_STATE_MACHINE_CORE_BOUNDARY.to_string(),
        raw_state_machine_compile_ready: false,
        initial_state_ready: false,
        initial_state_admissible_raw_token_count: 0,
        initial_state_contains_node_start: false,
    };

    if token_layout.max_multi_alternative_nonterminal_node_kind_count
        > token_layout.alternative_node_start_capacity
    {
        return (
            raw_state_machine,
            Some(CSharpSyntaxGrammarCoverageBlocker {
                grammar_id: entry.grammar_id.to_string(),
                metadata_path: entry.metadata_path.to_string(),
                kind: "raw_state_machine_alternative_node_start_capacity".to_string(),
                path: "token_layout.structural_decisions".to_string(),
                reason: format!(
                    "{} multi-alternative nonterminal node kinds exceed structural token capacity {} in {}",
                    token_layout.max_multi_alternative_nonterminal_node_kind_count,
                    token_layout.alternative_node_start_capacity,
                    token_layout.structural_range_id
                ),
                next_ready_task: CSHARP_SYNTAX_GRAMMAR_COVERAGE_BLOCKER_NEXT_READY_TASK.to_string(),
            }),
        );
    }

    if let Err(error) = node_start_alternative_token_ids_from_sidecar(sidecar) {
        return (
            raw_state_machine,
            Some(CSharpSyntaxGrammarCoverageBlocker {
                grammar_id: entry.grammar_id.to_string(),
                metadata_path: entry.metadata_path.to_string(),
                kind: "raw_state_machine_alternative_node_start_tokens".to_string(),
                path: "token_layout.structural_decisions".to_string(),
                reason: error.to_string(),
                next_ready_task: CSHARP_SYNTAX_GRAMMAR_COVERAGE_BLOCKER_NEXT_READY_TASK.to_string(),
            }),
        );
    }

    let model = match CompiledSyntaxModel::from_sidecar(sidecar) {
        Ok(model) => model,
        Err(error) => {
            return (
                raw_state_machine,
                Some(CSharpSyntaxGrammarCoverageBlocker {
                    grammar_id: entry.grammar_id.to_string(),
                    metadata_path: entry.metadata_path.to_string(),
                    kind: "raw_state_machine_compile".to_string(),
                    path: "raw_state_machine.compile".to_string(),
                    reason: error.to_string(),
                    next_ready_task: CSHARP_SYNTAX_GRAMMAR_COVERAGE_BLOCKER_NEXT_READY_TASK
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
                Some(CSharpSyntaxGrammarCoverageBlocker {
                    grammar_id: entry.grammar_id.to_string(),
                    metadata_path: entry.metadata_path.to_string(),
                    kind: "raw_state_machine_initial_state".to_string(),
                    path: "raw_state_machine.empty_prefix".to_string(),
                    reason: format!("{error:?}"),
                    next_ready_task: CSHARP_SYNTAX_GRAMMAR_COVERAGE_BLOCKER_NEXT_READY_TASK
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
            Some(CSharpSyntaxGrammarCoverageBlocker {
                grammar_id: entry.grammar_id.to_string(),
                metadata_path: entry.metadata_path.to_string(),
                kind: "raw_state_machine_initial_admissible_tokens".to_string(),
                path: "raw_state_machine.empty_prefix.admissible".to_string(),
                reason: "empty-prefix raw state did not expose the root node-start token"
                    .to_string(),
                next_ready_task: CSHARP_SYNTAX_GRAMMAR_COVERAGE_BLOCKER_NEXT_READY_TASK.to_string(),
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
