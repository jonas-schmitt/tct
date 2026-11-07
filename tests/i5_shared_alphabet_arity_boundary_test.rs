//! I5 shared decision-token alphabet — arity boundary at MAX=40 end-to-end.
//!
//! Tranche 3 unit tests hard-error at `MAX+1`, but nothing exercised arity
//! exactly 40 — the boundary where `CHOICE(40,39)` lands on the final slot of
//! the structural band (`SHARED_ALPHABET_STRUCTURAL_BAND_SIZE - 1`). This file
//! locks that boundary for every ordinal `k in 0..40` and keeps the `MAX+1`
//! overflow hard-error assertion for contrast.
//!
//! Claim id: `i5_shared_alphabet_arity_boundary_v0`
//! Boundary: `compiler_shared_alphabet_layout_only`
//!
//! Not a paper claim, production route, transfer experiment, pin rewrite, or
//! new hand-built subset codec.

use tct::grammar_ir::{
    GrammarField, GrammarFieldAlternative, GrammarIr, GrammarIrProvenance, GrammarRule,
    GrammarTerminalKind,
};
use tct::grammar_ir_structural_model::{
    GrammarStructuralTokenLayout, lower_grammar_ir_to_structural_model,
};
use tct::shared_alphabet_layout::{
    SHARED_ALPHABET_CLAIM_BOUNDARY, SHARED_ALPHABET_MAX_BRANCH_ARITY,
    SHARED_ALPHABET_STRUCTURAL_BAND_SIZE, SHARED_ALPHABET_VOCAB_SIZE, SyntaxTokenLayoutMode,
    shared_alphabet_choice_token_id, shared_alphabet_unpack_choice_offset,
};
use tct::syntax_state_machine::{
    CompiledSyntaxModel, RawSyntaxDecodedField, RawSyntaxDecodedNode, RawSyntaxDecodedValue,
};
use tct::toy_grammar_frontend::ToyGrammarSpec;

const CLAIM_ID: &str = "i5_shared_alphabet_arity_boundary_v0";
const CLAIM_BOUNDARY: &str = SHARED_ALPHABET_CLAIM_BOUNDARY;

/// Build a Nonterminal multi-alt grammar with the given alternative kinds.
///
/// Zero-padded kinds (`alt000`..`alt039`) keep alphabetical
/// `(node_kind, terminal_class)` order aligned with numeric ordinal order.
fn multi_alt_grammar(grammar_id: &str, alt_kinds: &[String]) -> GrammarIr {
    let mut grammar = GrammarIr::new(
        grammar_id,
        GrammarIrProvenance {
            source_kind: "synthetic_i5_shared_alphabet_arity_boundary".to_string(),
            source_path: Some("tests/i5_shared_alphabet_arity_boundary_test.rs".to_string()),
            source_sha256: None,
            metadata_path: None,
            metadata_sha256: None,
            parser: None,
        },
    );
    grammar.rules.push(GrammarRule {
        name: "wrapper".to_string(),
        node_kind: "wrapper".to_string(),
        fields: vec![GrammarField {
            name: "body".to_string(),
            required: true,
            multiple: false,
            alternatives: alt_kinds
                .iter()
                .map(|kind| GrammarFieldAlternative {
                    node_kind: kind.clone(),
                    terminal: GrammarTerminalKind::Nonterminal,
                })
                .collect(),
        }],
    });
    for kind in alt_kinds {
        grammar.rules.push(GrammarRule {
            name: kind.clone(),
            node_kind: kind.clone(),
            fields: vec![],
        });
    }
    grammar
}

fn compile_shared(grammar: &GrammarIr) -> Result<CompiledSyntaxModel, String> {
    let layout = ToyGrammarSpec::assignment_language_v1().layout;
    let sidecar = lower_grammar_ir_to_structural_model(
        grammar,
        GrammarStructuralTokenLayout::from_toy_assignment(&layout),
    )
    .map_err(|error| format!("lower: {error}"))?;
    CompiledSyntaxModel::from_sidecar_with_mode(&sidecar, SyntaxTokenLayoutMode::SharedAlphabet)
        .map_err(|error| format!("SharedAlphabet compile: {error}"))
}

fn leaf_tree(kind: &str) -> RawSyntaxDecodedNode {
    RawSyntaxDecodedNode {
        rule_name: "wrapper".to_string(),
        node_kind: "wrapper".to_string(),
        fields: vec![RawSyntaxDecodedField {
            name: "body".to_string(),
            values: vec![RawSyntaxDecodedValue::Node(Box::new(
                RawSyntaxDecodedNode {
                    rule_name: kind.to_string(),
                    node_kind: kind.to_string(),
                    fields: vec![],
                },
            ))],
            missing: false,
            repeated_empty: false,
        }],
    }
}

/// End-to-end SharedAlphabet encode/decode for every ordinal of arity 40.
///
/// Also asserts `CHOICE(40,39) == STRUCTURAL_BAND_SIZE - 1` and that unpack
/// inverts it, so an off-by-one in the triangular packing cannot pass silently.
#[test]
fn i5_shared_alphabet_arity_exactly_max_branch_arity_end_to_end() {
    assert_eq!(CLAIM_BOUNDARY, "compiler_shared_alphabet_layout_only");
    assert_eq!(CLAIM_ID, "i5_shared_alphabet_arity_boundary_v0");
    assert_eq!(SHARED_ALPHABET_MAX_BRANCH_ARITY, 40);
    assert_eq!(SHARED_ALPHABET_STRUCTURAL_BAND_SIZE, 829);

    let n = SHARED_ALPHABET_MAX_BRANCH_ARITY;
    // Zero-padded so alphabetical (node_kind, terminal_class) order == ordinal.
    let kinds: Vec<String> = (0..n).map(|k| format!("alt{k:03}")).collect();
    let grammar = multi_alt_grammar("i5_shared_alphabet_arity_exactly_40_v0", &kinds);
    let model = compile_shared(&grammar).expect("arity 40 must compile under SharedAlphabet");
    assert_eq!(model.layout_mode(), SyntaxTokenLayoutMode::SharedAlphabet);
    let roles = model.token_roles();
    assert_eq!(roles.vocab_size, SHARED_ALPHABET_VOCAB_SIZE);
    assert_eq!(
        roles.structural_range.end_exclusive,
        SHARED_ALPHABET_STRUCTURAL_BAND_SIZE
    );

    // Final slot of the structural band is CHOICE(40,39).
    let choice_40_39 = shared_alphabet_choice_token_id(0, n, n - 1).expect("CHOICE(40,39)");
    assert_eq!(
        choice_40_39,
        SHARED_ALPHABET_STRUCTURAL_BAND_SIZE - 1,
        "CHOICE(40,39) must occupy the last structural-band slot"
    );
    let (unpacked_n, unpacked_k) =
        shared_alphabet_unpack_choice_offset(choice_40_39).expect("CHOICE(40,39) must unpack");
    assert_eq!(
        (unpacked_n, unpacked_k),
        (n, n - 1),
        "shared_alphabet_unpack_choice_offset must invert CHOICE(40,39)"
    );

    let row = roles
        .field_choice_row("wrapper", "body")
        .expect("wrapper.body field choice row");
    assert_eq!(row.len(), n as usize, "field choice row arity must be 40");

    for k in 0..n {
        let kind = &kinds[k as usize];
        let expected = shared_alphabet_choice_token_id(0, n, k).expect("CHOICE pack");
        let via_helper = roles
            .alternative_token_for_field("wrapper", "body", kind)
            .unwrap_or_else(|| panic!("missing alternative_token_for_field for {kind}"));
        assert_eq!(
            via_helper, expected,
            "alternative_token_for_field(wrapper, body, {kind}) must be CHOICE(40,{k})"
        );
        assert_eq!(
            row[k as usize].node_kind, *kind,
            "field_choice_table ordinal {k} must be {kind}"
        );
        assert_eq!(
            row[k as usize].token_id, expected,
            "field_choice_table[{k}] must be CHOICE(40,{k})"
        );

        // Unpack each absolute id for every ordinal.
        let offset = expected
            .checked_sub(roles.structural_range.start)
            .expect("choice above structural start");
        let (got_n, got_k) = shared_alphabet_unpack_choice_offset(offset)
            .unwrap_or_else(|error| panic!("unpack CHOICE(40,{k}): {error}"));
        assert_eq!((got_n, got_k), (n, k));

        let tree = leaf_tree(kind);
        let tokens = model
            .encode_complete_tree(&tree)
            .unwrap_or_else(|error| panic!("encode alt ordinal {k} ({kind}): {error:?}"));
        assert!(
            tokens.contains(&expected),
            "encoded stream for ordinal {k} must carry CHOICE(40,{k})={expected}; stream={tokens:?}"
        );
        for &token in &tokens {
            assert!(
                token < roles.pad_token && token < roles.vocab_size,
                "token {token} must stay below PAD/VOCAB for ordinal {k}"
            );
        }

        let decoded = model
            .decode_complete_tree(&tokens)
            .unwrap_or_else(|error| panic!("decode ordinal {k}: {error:?}"))
            .decoded_node_tree()
            .expect("decoded node tree");
        assert_eq!(
            decoded, tree,
            "decode(encode(tree)) must equal tree for ordinal {k} ({kind}; {CLAIM_BOUNDARY})"
        );

        // Expected stream shape: node_start, field_start, CHOICE, node_end, field_end, node_end.
        assert_eq!(
            tokens,
            vec![
                roles.node_start,
                roles.field_start,
                expected,
                roles.node_end,
                roles.field_end,
                roles.node_end,
            ],
            "homogeneous multi-alt leaf stream for ordinal {k}"
        );
    }
}

/// Contrast: arity `MAX+1` hard-errors at compile under SharedAlphabet.
///
/// Same assertion family as the in-crate tranche-3 unit test, kept here so
/// the boundary file documents both sides of the cut (pass at 40 / fail at 41).
#[test]
fn i5_shared_alphabet_arity_max_plus_one_overflow_hard_errors() {
    assert_eq!(CLAIM_BOUNDARY, "compiler_shared_alphabet_layout_only");
    assert_eq!(CLAIM_ID, "i5_shared_alphabet_arity_boundary_v0");

    let overflow = (SHARED_ALPHABET_MAX_BRANCH_ARITY + 1) as usize;
    // Zero-padded so kinds remain domain-stable if overflow ever became allowed.
    let kinds: Vec<String> = (0..overflow).map(|i| format!("alt{i:03}")).collect();
    let grammar = multi_alt_grammar("i5_shared_alphabet_arity_overflow_41_v0", &kinds);
    let err =
        compile_shared(&grammar).expect_err("arity MAX+1 must hard-error under SharedAlphabet");
    let text = err;
    assert!(
        text.contains("shared_alphabet.arity_overflow") || text.contains("arity"),
        "diagnostic must identify arity overflow: {text}"
    );
    assert!(
        text.contains("owner_rule=wrapper") || text.contains("wrapper"),
        "diagnostic must name owner rule: {text}"
    );
    assert!(
        text.contains("field=body") || text.contains("body"),
        "diagnostic must name field: {text}"
    );
    assert!(
        text.contains(&SHARED_ALPHABET_MAX_BRANCH_ARITY.to_string()),
        "diagnostic must name SHARED_ALPHABET_MAX_BRANCH_ARITY: {text}"
    );
    assert!(
        text.contains(&(SHARED_ALPHABET_MAX_BRANCH_ARITY + 1).to_string()),
        "diagnostic must name the overflowing arity: {text}"
    );
}
