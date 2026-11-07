//! I5 shared decision-token alphabet — checklist tranche 12 layout-diff gate.
//!
//! Records shared vs per-grammar layout summary for the JSON subset fixture
//! through the syntax token-layout diff machinery (`diff_syntax_token_layouts`).
//! Decision 4: mode switch is a stream-level `breaking_change` for absolute
//! consumers; default PerGrammar self-diff stays `exact_reuse` (no pin edits).
//!
//! Claim id: `i5_shared_alphabet_layout_diff_v0`
//! Boundary: `compiler_shared_alphabet_layout_only`
//!
//! Not a paper claim, production route, transfer experiment, pin rewrite, or
//! new hand-built subset codec.

use std::collections::BTreeMap;

use tct::shared_alphabet_layout::{
    SHARED_ALPHABET_CLAIM_BOUNDARY, SHARED_ALPHABET_PAD_TOKEN, SHARED_ALPHABET_VOCAB_SIZE,
    SyntaxTokenLayoutMode, shared_alphabet_token_layout,
};
use tct::syntax_state_machine::{
    CompiledSyntaxModel, RAW_BOOLEAN_FALSE_OFFSET, RAW_BOOLEAN_TRUE_OFFSET, RAW_FIELD_END_OFFSET,
    RAW_FIELD_MISSING_OFFSET, RAW_FIELD_START_OFFSET, RAW_NODE_END_OFFSET, RAW_NODE_START_OFFSET,
    RAW_PAYLOAD_END_OFFSET, RAW_REPEATED_EMPTY_OFFSET, RAW_TERMINAL_OFFSET, SyntaxTokenRoles,
};
use tct::syntax_token_layout_diff::{
    RAW_SYNTAX_STREAM_CONTRACT_EXPANDED_V1, SyntaxLayoutCompatibility, SyntaxTokenLayoutDescriptor,
    SyntaxTokenLayoutRangeDescriptor, diff_syntax_token_layouts,
};
use tct::tree_sitter_json_subset_codec::{TreeSitterJsonSubsetCodec, subset_token_layout};

const CLAIM_ID: &str = "i5_shared_alphabet_layout_diff_v0";
const CLAIM_BOUNDARY: &str = SHARED_ALPHABET_CLAIM_BOUNDARY;

/// Ten fixed structural role offsets shared by PerGrammar and SharedAlphabet.
fn fixed_role_offsets() -> BTreeMap<String, u32> {
    BTreeMap::from([
        ("node_start".to_string(), RAW_NODE_START_OFFSET),
        ("node_end".to_string(), RAW_NODE_END_OFFSET),
        ("field_start".to_string(), RAW_FIELD_START_OFFSET),
        ("field_end".to_string(), RAW_FIELD_END_OFFSET),
        ("field_missing".to_string(), RAW_FIELD_MISSING_OFFSET),
        ("repeated_empty".to_string(), RAW_REPEATED_EMPTY_OFFSET),
        ("terminal".to_string(), RAW_TERMINAL_OFFSET),
        ("payload_end".to_string(), RAW_PAYLOAD_END_OFFSET),
        ("boolean_false".to_string(), RAW_BOOLEAN_FALSE_OFFSET),
        ("boolean_true".to_string(), RAW_BOOLEAN_TRUE_OFFSET),
    ])
}

/// PerGrammar alternative identity: the residual global first-wins map is the
/// real identity under per-grammar free-slot assignment.
fn per_grammar_alternative_map(roles: &SyntaxTokenRoles) -> BTreeMap<String, u32> {
    roles.node_start_alternative_tokens.clone()
}

/// SharedAlphabet alternative identity for layout-diff evidence: derive from
/// the load-bearing `field_choice_table` (Decision 2/3), not the residual
/// first-wins global map (which is not a sound reverse identity under shared
/// mode because distinct fields reuse the same `CHOICE(n,k)` absolute ids).
///
/// Keys are unambiguous field-relative strings `"rule.field.node_kind"`.
fn shared_alphabet_alternative_map_from_field_choice_table(
    roles: &SyntaxTokenRoles,
) -> BTreeMap<String, u32> {
    let mut map = BTreeMap::new();
    for ((rule, field), alts) in &roles.field_choice_table {
        for alt in alts {
            let key = format!("{rule}.{field}.{}", alt.node_kind);
            // Field-relative keys are unique; a collision would mean the table
            // itself is ill-formed (duplicate node_kind in one field row).
            let previous = map.insert(key.clone(), alt.token_id);
            assert!(
                previous.is_none(),
                "field_choice_table key collision for {key}: previous={previous:?} new={}",
                alt.token_id
            );
        }
    }
    map
}

/// Build a comparable layout descriptor from ranges + an alternative map.
///
/// Shared mode v0 is expanded-stream only (contract Decision 3 / elision out of
/// scope). Callers supply the alternative map that is load-bearing for the
/// mode under test (global map for PerGrammar; field_choice_table-derived map
/// for SharedAlphabet).
fn descriptor_from_layout_and_alternatives(
    layout_id: &str,
    layout_version: u32,
    ranges: &[(String, String, u32, u32)],
    vocab_size: u32,
    pad_token: u32,
    alternative_node_start_tokens: BTreeMap<String, u32>,
) -> SyntaxTokenLayoutDescriptor {
    SyntaxTokenLayoutDescriptor {
        layout_id: layout_id.to_string(),
        layout_version,
        stream_contract: RAW_SYNTAX_STREAM_CONTRACT_EXPANDED_V1.to_string(),
        vocab_size,
        pad_token,
        ranges: ranges
            .iter()
            .map(|(id, role, start, end)| SyntaxTokenLayoutRangeDescriptor {
                id: id.clone(),
                role: role.clone(),
                start: *start,
                end_exclusive: *end,
            })
            .collect(),
        role_offsets: fixed_role_offsets(),
        alternative_node_start_tokens,
    }
}

fn ranges_from_structural_layout(
    layout: &tct::grammar_ir_structural_model::GrammarStructuralTokenLayout,
) -> Vec<(String, String, u32, u32)> {
    layout
        .ranges
        .iter()
        .map(|range| {
            (
                range.id.clone(),
                range.role.clone(),
                range.start,
                range.end_exclusive,
            )
        })
        .collect()
}

/// I5 checklist tranche 12: layout-diff classifies mode switch as breaking.
#[test]
fn i5_shared_alphabet_layout_diff_json_subset_mode_switch_is_breaking_change() {
    assert_eq!(CLAIM_BOUNDARY, "compiler_shared_alphabet_layout_only");
    assert_eq!(CLAIM_ID, "i5_shared_alphabet_layout_diff_v0");

    let codec = TreeSitterJsonSubsetCodec::from_pinned_fixture()
        .expect("JSON subset pin should load for layout-diff evidence");
    let per_grammar = codec.model();
    assert_eq!(
        per_grammar.layout_mode(),
        SyntaxTokenLayoutMode::PerGrammar,
        "default codec must stay on PerGrammar (Decision 4)"
    );

    let shared = CompiledSyntaxModel::from_sidecar_with_mode(
        codec.sidecar(),
        SyntaxTokenLayoutMode::SharedAlphabet,
    )
    .expect("JSON subset should compile under SharedAlphabet");
    assert_eq!(shared.layout_mode(), SyntaxTokenLayoutMode::SharedAlphabet);

    let per_layout = subset_token_layout();
    let shared_layout = shared_alphabet_token_layout();
    let per_roles = per_grammar.token_roles();
    let shared_roles = shared.token_roles();

    // SharedAlphabet must populate the field-relative table (Decision 2).
    assert!(
        !shared_roles.field_choice_table.is_empty(),
        "SharedAlphabet layout-diff evidence requires a non-empty field_choice_table"
    );
    assert!(
        per_roles.field_choice_table.is_empty(),
        "PerGrammar must keep field_choice_table empty (global map is its identity)"
    );

    let per_descriptor = descriptor_from_layout_and_alternatives(
        "tree_sitter_json_subset.per_grammar",
        1,
        &ranges_from_structural_layout(&per_layout),
        per_layout.vocab_size,
        per_layout.pad_token,
        per_grammar_alternative_map(per_roles),
    );
    let shared_descriptor = descriptor_from_layout_and_alternatives(
        "tree_sitter_json_subset.shared_alphabet",
        1,
        &ranges_from_structural_layout(&shared_layout),
        shared_layout.vocab_size,
        shared_layout.pad_token,
        shared_alphabet_alternative_map_from_field_choice_table(shared_roles),
    );
    // Sanity: shared-side map is non-empty and field-keyed (not bare node_kinds).
    assert!(
        !shared_descriptor.alternative_node_start_tokens.is_empty(),
        "SharedAlphabet field_choice_table-derived alternative map must be non-empty"
    );
    assert!(
        shared_descriptor
            .alternative_node_start_tokens
            .keys()
            .any(|k| k.contains('.')),
        "SharedAlphabet alternative keys must be field-relative (rule.field.node_kind); keys={:?}",
        shared_descriptor
            .alternative_node_start_tokens
            .keys()
            .collect::<Vec<_>>()
    );

    // Self-diffs are exact_reuse (no silent drift inside either mode).
    let per_self = diff_syntax_token_layouts(&per_descriptor, &per_descriptor)
        .expect("PerGrammar self-diff should succeed");
    assert_eq!(
        per_self.compatibility,
        SyntaxLayoutCompatibility::ExactReuse,
        "PerGrammar JSON subset layout must be exact_reuse against itself"
    );
    let shared_self = diff_syntax_token_layouts(&shared_descriptor, &shared_descriptor)
        .expect("SharedAlphabet self-diff should succeed");
    assert_eq!(
        shared_self.compatibility,
        SyntaxLayoutCompatibility::ExactReuse,
        "SharedAlphabet JSON subset layout must be exact_reuse against itself"
    );

    // Mode switch: stream-level breaking_change for absolute consumers.
    let mode_switch = diff_syntax_token_layouts(&per_descriptor, &shared_descriptor)
        .expect("mode-switch layout-diff should succeed");
    assert_eq!(
        mode_switch.compatibility,
        SyntaxLayoutCompatibility::BreakingChange,
        "PerGrammar → SharedAlphabet must classify as breaking_change (Decision 4); reasons: {:?}",
        mode_switch.reasons
    );
    assert_eq!(
        mode_switch.compatibility.as_str(),
        "breaking_change",
        "string form must match tct-token-diff class vocabulary"
    );
    assert!(
        !mode_switch.downstream_claims_allowed,
        "layout-diff evidence must not allow downstream model claims"
    );
    assert_eq!(
        mode_switch.evidence_scope, "compiler_artifact_evidence_only",
        "compiler-local evidence only"
    );

    // Range-level summary: structural band widens for CHOICE(n,k); payload bases move.
    assert!(
        mode_switch
            .moved_ranges
            .contains(&"structural_decisions".to_string()),
        "structural_decisions must move (shared band 0..829 vs subset 0..64); moved={:?}",
        mode_switch.moved_ranges
    );
    assert!(
        mode_switch
            .moved_ranges
            .contains(&"utf8_byte_payload".to_string()),
        "utf8_byte_payload base must move with structural widening; moved={:?}",
        mode_switch.moved_ranges
    );
    assert!(
        mode_switch
            .moved_ranges
            .contains(&"u8_integer_payload".to_string()),
        "u8_integer_payload base must move with structural widening; moved={:?}",
        mode_switch.moved_ranges
    );
    assert_eq!(
        mode_switch.vocab_size_change,
        Some((per_layout.vocab_size, SHARED_ALPHABET_VOCAB_SIZE)),
        "vocab must jump from subset PerGrammar to shared constants"
    );
    assert_eq!(
        mode_switch.pad_token_change,
        Some((per_layout.pad_token, SHARED_ALPHABET_PAD_TOKEN)),
        "pad must jump from subset PerGrammar to shared constants"
    );
    // Fixed bracket role *offsets* stay identical (absolute bases differ via ranges).
    assert!(
        mode_switch.changed_role_offsets.is_empty(),
        "fixed role offsets 0..9 must not change between modes; changed={:?}",
        mode_switch.changed_role_offsets
    );
    // Stream contract is expanded for both (shared v0 does not flip elision).
    assert!(
        mode_switch.stream_contract_change.is_none(),
        "mode switch must not silently change stream contract; change={:?}",
        mode_switch.stream_contract_change
    );
    // Multi-alt absolute IDs renumber (free-slot → CHOICE(n,k)).
    assert!(
        !mode_switch.renumbered_alternatives.is_empty()
            || !mode_switch.removed_alternatives.is_empty()
            || !mode_switch.added_alternatives.is_empty(),
        "choice/alternative absolute IDs must differ under SharedAlphabet; \
         renumbered={:?} removed={:?} added={:?}",
        mode_switch.renumbered_alternatives,
        mode_switch.removed_alternatives,
        mode_switch.added_alternatives
    );

    // Sanity: roles agree with layout constructors (no free-slot leak into shared).
    assert_eq!(shared_roles.vocab_size, SHARED_ALPHABET_VOCAB_SIZE);
    assert_eq!(shared_roles.pad_token, SHARED_ALPHABET_PAD_TOKEN);
    assert_eq!(shared_roles.structural_range.end_exclusive, 829);
    assert_ne!(
        per_roles.vocab_size, shared_roles.vocab_size,
        "PerGrammar and SharedAlphabet must not silently share vocab size"
    );
}
