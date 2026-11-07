//! I5 shared decision-token alphabet — mixed-payload body-kind dispatch under
//! SharedAlphabet.
//!
//! The mixed-payload path (`apply_mixed_field_choice` /
//! `AlternativeBodyKind` dispatch) is the most intricate decode path and was
//! previously covered under shared mode only by encode-side inverse unit
//! assertions. This integration test constructs an equivalent mixed grammar
//! (same shape as the in-crate `mixed_payload_witness`, which is `pub(crate)`
//! and unreachable from integration tests) and asserts full encode/decode
//! inverse, mask/accept consistency, and consumed-token accounting per the
//! contract table in `docs/i5-shared-decision-token-alphabet-contract.md`.
//!
//! Placement: integration test with an equivalent grammar (not a `#[cfg(test)]`
//! module inside `src/`), so concurrent production edits to
//! `syntax_state_machine.rs` / `raw_replay.rs` do not merge-conflict with
//! this coverage, and so the witness shape is explicit in the test file.
//!
//! Claim id: `i5_shared_alphabet_mixed_payload_dispatch_v0`
//! Boundary: `compiler_shared_alphabet_layout_only`
//!
//! Not a paper claim, production route, transfer experiment, pin rewrite, or
//! new hand-built subset codec.

use std::collections::BTreeSet;

use tct::grammar_ir::{
    GrammarField, GrammarFieldAlternative, GrammarIr, GrammarIrProvenance, GrammarRule,
    GrammarTerminalKind,
};
use tct::grammar_ir_structural_model::{
    GrammarStructuralTokenLayout, lower_grammar_ir_to_structural_model,
};
use tct::shared_alphabet_layout::{
    SHARED_ALPHABET_CLAIM_BOUNDARY, SHARED_ALPHABET_VOCAB_SIZE, SyntaxTokenLayoutMode,
    shared_alphabet_choice_token_id,
};
use tct::syntax_state_machine::{
    CompiledSyntaxModel, RawSyntaxAdmissibleTokens, RawSyntaxDecodedField, RawSyntaxDecodedNode,
    RawSyntaxDecodedTerminal, RawSyntaxDecodedValue,
};
use tct::toy_grammar_frontend::ToyGrammarSpec;

const CLAIM_ID: &str = "i5_shared_alphabet_mixed_payload_dispatch_v0";
const CLAIM_BOUNDARY: &str = SHARED_ALPHABET_CLAIM_BOUNDARY;

/// Equivalent of `model_facts::mixed_payload_witness::mixed_payload_grammar_ir`.
///
/// Domain-stable sort of (node_kind, terminal_class):
/// child/nonterminal, false/boolean, number/integer, text/string, true/boolean
/// → CHOICE(5, 0..4).
fn mixed_payload_grammar_ir() -> GrammarIr {
    let mut grammar = GrammarIr::new(
        "i5_shared_alphabet_mixed_payload_dispatch_v0",
        GrammarIrProvenance {
            source_kind: "synthetic_i5_shared_alphabet_mixed_payload_dispatch".to_string(),
            source_path: Some(
                "tests/i5_shared_alphabet_mixed_payload_dispatch_test.rs".to_string(),
            ),
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
            name: "value".to_string(),
            required: true,
            multiple: false,
            alternatives: vec![
                GrammarFieldAlternative {
                    node_kind: "child".to_string(),
                    terminal: GrammarTerminalKind::Nonterminal,
                },
                GrammarFieldAlternative {
                    node_kind: "number".to_string(),
                    terminal: GrammarTerminalKind::IntegerPayload,
                },
                GrammarFieldAlternative {
                    node_kind: "text".to_string(),
                    terminal: GrammarTerminalKind::StringPayload,
                },
                GrammarFieldAlternative {
                    node_kind: "true".to_string(),
                    terminal: GrammarTerminalKind::BooleanPayload,
                },
                GrammarFieldAlternative {
                    node_kind: "false".to_string(),
                    terminal: GrammarTerminalKind::BooleanPayload,
                },
            ],
        }],
    });
    grammar.rules.push(GrammarRule {
        name: "child".to_string(),
        node_kind: "child".to_string(),
        fields: vec![],
    });
    grammar
}

fn compile_mixed_shared() -> CompiledSyntaxModel {
    let layout = ToyGrammarSpec::assignment_language_v1().layout;
    let sidecar = lower_grammar_ir_to_structural_model(
        &mixed_payload_grammar_ir(),
        GrammarStructuralTokenLayout::from_toy_assignment(&layout),
    )
    .expect("mixed-payload grammar should lower to StructuralModel");
    CompiledSyntaxModel::from_sidecar_with_mode(&sidecar, SyntaxTokenLayoutMode::SharedAlphabet)
        .expect("mixed-payload grammar must compile under SharedAlphabet")
}

fn wrapper_tree(value: RawSyntaxDecodedValue) -> RawSyntaxDecodedNode {
    RawSyntaxDecodedNode {
        rule_name: "wrapper".to_string(),
        node_kind: "wrapper".to_string(),
        fields: vec![RawSyntaxDecodedField {
            name: "value".to_string(),
            values: vec![value],
            missing: false,
            repeated_empty: false,
        }],
    }
}

fn mask_set(admissible: &RawSyntaxAdmissibleTokens, payload_end: u32) -> BTreeSet<u32> {
    let mut set = BTreeSet::new();
    for &id in &admissible.finite_raw_token_ids {
        set.insert(id);
    }
    for range in &admissible.raw_token_ranges {
        for id in range.start..range.end_exclusive {
            set.insert(id);
        }
    }
    if admissible.payload_end_admissible {
        set.insert(payload_end);
    }
    set
}

fn accepted_set(model: &CompiledSyntaxModel, prefix: &[u32]) -> BTreeSet<u32> {
    let vocab = model.token_roles().vocab_size;
    let mut set = BTreeSet::new();
    for id in 0..vocab {
        if model.transition(prefix, id).accepted {
            set.insert(id);
        }
    }
    set
}

fn assert_mask_accept_consistency(model: &CompiledSyntaxModel, tokens: &[u32], witness: &str) {
    let payload_end = model.token_roles().payload_end;
    for prefix_len in 0..=tokens.len() {
        let prefix = &tokens[..prefix_len];
        let admissible = model
            .admissible_next_raw_tokens(prefix)
            .unwrap_or_else(|error| {
                panic!(
                    "admissible_next_raw_tokens failed at prefix_len={prefix_len} ({witness}): {error:?}"
                )
            });
        let masked = mask_set(&admissible, payload_end);
        let accepted = accepted_set(model, prefix);
        if masked != accepted {
            let only_masked: BTreeSet<_> = masked.difference(&accepted).copied().collect();
            let only_accepted: BTreeSet<_> = accepted.difference(&masked).copied().collect();
            panic!(
                "mask/accept divergence at prefix_len={prefix_len} ({witness}; {CLAIM_BOUNDARY}).\n\
                 only_masked={only_masked:?}\n\
                 only_accepted={only_accepted:?}"
            );
        }
        if prefix_len < tokens.len() {
            assert!(
                masked.contains(&tokens[prefix_len]),
                "stream token {} at offset {prefix_len} not in mask/accept ({witness})",
                tokens[prefix_len]
            );
        }
    }
}

fn assert_stream_tree_inverses(
    model: &CompiledSyntaxModel,
    tree: &RawSyntaxDecodedNode,
    witness: &str,
) -> Vec<u32> {
    let roles = model.token_roles();
    let tokens = model
        .encode_complete_tree(tree)
        .unwrap_or_else(|error| panic!("encode failed for {witness}: {error:?}"));
    for &token in &tokens {
        assert!(
            token < roles.pad_token && token < roles.vocab_size,
            "token {token} must stay below PAD/VOCAB for {witness}"
        );
    }
    let decoded = model
        .decode_complete_tree(&tokens)
        .unwrap_or_else(|error| panic!("decode failed for {witness}: {error:?}"))
        .decoded_node_tree()
        .expect("decoded node tree");
    assert_eq!(
        &decoded, tree,
        "decode(encode(tree)) must equal tree for {witness} ({CLAIM_BOUNDARY})"
    );
    let reencoded = model
        .encode_complete_tree(&decoded)
        .unwrap_or_else(|error| panic!("re-encode failed for {witness}: {error:?}"));
    assert_eq!(
        reencoded, tokens,
        "encode(decode(tokens)) must equal tokens for {witness}"
    );
    assert_mask_accept_consistency(model, &tokens, witness);
    tokens
}

/// Mixed BooleanPayload / IntegerPayload / Utf8 / Nonterminal under SharedAlphabet:
/// inverse + mask/accept + consumed-token accounting per body kind.
#[test]
fn i5_shared_alphabet_mixed_payload_body_kind_dispatch() {
    assert_eq!(CLAIM_BOUNDARY, "compiler_shared_alphabet_layout_only");
    assert_eq!(CLAIM_ID, "i5_shared_alphabet_mixed_payload_dispatch_v0");

    let model = compile_mixed_shared();
    assert_eq!(model.layout_mode(), SyntaxTokenLayoutMode::SharedAlphabet);
    let roles = model.token_roles();
    assert_eq!(roles.vocab_size, SHARED_ALPHABET_VOCAB_SIZE);

    // Domain-stable CHOICE(5,k) packing.
    let child = shared_alphabet_choice_token_id(0, 5, 0).expect("CHOICE(5,0)");
    let false_tok = shared_alphabet_choice_token_id(0, 5, 1).expect("CHOICE(5,1)");
    let number = shared_alphabet_choice_token_id(0, 5, 2).expect("CHOICE(5,2)");
    let text_tok = shared_alphabet_choice_token_id(0, 5, 3).expect("CHOICE(5,3)");
    let true_tok = shared_alphabet_choice_token_id(0, 5, 4).expect("CHOICE(5,4)");
    assert_eq!(
        roles.alternative_token_for_field("wrapper", "value", "child"),
        Some(child)
    );
    assert_eq!(
        roles.alternative_token_for_field("wrapper", "value", "false"),
        Some(false_tok)
    );
    assert_eq!(
        roles.alternative_token_for_field("wrapper", "value", "number"),
        Some(number)
    );
    assert_eq!(
        roles.alternative_token_for_field("wrapper", "value", "text"),
        Some(text_tok)
    );
    assert_eq!(
        roles.alternative_token_for_field("wrapper", "value", "true"),
        Some(true_tok)
    );

    // --- Mixed BooleanPayload: CHOICE(n,k) only (no generic terminal, no payload bytes) ---
    let true_tree = wrapper_tree(RawSyntaxDecodedValue::Terminal(RawSyntaxDecodedTerminal {
        node_kind: "true".to_string(),
        terminal: "boolean".to_string(),
        payload: "true".to_string(),
    }));
    let true_tokens = assert_stream_tree_inverses(&model, &true_tree, "mixed_boolean_true");
    assert_eq!(
        true_tokens,
        vec![
            roles.node_start,
            roles.field_start,
            true_tok,
            roles.field_end,
            roles.node_end,
        ],
        "mixed BooleanPayload true is CHOICE only"
    );
    assert!(
        !true_tokens.contains(&roles.terminal),
        "mixed BooleanPayload must not emit generic terminal"
    );
    assert!(
        !true_tokens.contains(&roles.payload_end),
        "mixed BooleanPayload must not emit payload_end"
    );
    // Field-value body = CHOICE only (between field_start and field_end).
    assert_eq!(
        &true_tokens[2..3],
        &[true_tok],
        "consumed body for mixed BooleanPayload is CHOICE only"
    );

    let false_tree = wrapper_tree(RawSyntaxDecodedValue::Terminal(RawSyntaxDecodedTerminal {
        node_kind: "false".to_string(),
        terminal: "boolean".to_string(),
        payload: "false".to_string(),
    }));
    let false_tokens = assert_stream_tree_inverses(&model, &false_tree, "mixed_boolean_false");
    assert_eq!(
        false_tokens,
        vec![
            roles.node_start,
            roles.field_start,
            false_tok,
            roles.field_end,
            roles.node_end,
        ],
        "mixed BooleanPayload false is CHOICE only"
    );

    // --- Mixed IntegerPayload: CHOICE + payload + payload_end ---
    let int_tree = wrapper_tree(RawSyntaxDecodedValue::Terminal(RawSyntaxDecodedTerminal {
        node_kind: "number".to_string(),
        terminal: "integer".to_string(),
        payload: "7".to_string(),
    }));
    let int_tokens = assert_stream_tree_inverses(&model, &int_tree, "mixed_integer");
    let expected_int = vec![
        roles.node_start,
        roles.field_start,
        number,
        roles.integer_range.start + 7,
        roles.payload_end,
        roles.field_end,
        roles.node_end,
    ];
    assert_eq!(
        int_tokens, expected_int,
        "mixed IntegerPayload is CHOICE + payload + payload_end"
    );
    assert!(
        !int_tokens.contains(&roles.terminal),
        "mixed IntegerPayload must not emit generic terminal"
    );
    // Body between field_start and field_end: CHOICE + integer byte + payload_end.
    assert_eq!(
        &int_tokens[2..5],
        &[number, roles.integer_range.start + 7, roles.payload_end]
    );

    // --- Mixed Utf8 / StringPayload: CHOICE + payload bytes + payload_end ---
    let text = "ab";
    let utf8_tree = wrapper_tree(RawSyntaxDecodedValue::Terminal(RawSyntaxDecodedTerminal {
        node_kind: "text".to_string(),
        terminal: "string".to_string(),
        payload: text.to_string(),
    }));
    let utf8_tokens = assert_stream_tree_inverses(&model, &utf8_tree, "mixed_utf8");
    let mut expected_utf8 = vec![roles.node_start, roles.field_start, text_tok];
    for byte in text.as_bytes() {
        expected_utf8.push(roles.utf8_range.start + u32::from(*byte));
    }
    expected_utf8.push(roles.payload_end);
    expected_utf8.push(roles.field_end);
    expected_utf8.push(roles.node_end);
    assert_eq!(
        utf8_tokens, expected_utf8,
        "mixed Utf8 is CHOICE + payload + payload_end"
    );
    assert!(
        !utf8_tokens.contains(&roles.terminal),
        "mixed Utf8 must not emit generic terminal"
    );
    // Body: CHOICE + two utf8 bytes + payload_end.
    assert_eq!(
        &utf8_tokens[2..6],
        &[
            text_tok,
            roles.utf8_range.start + u32::from(b'a'),
            roles.utf8_range.start + u32::from(b'b'),
            roles.payload_end,
        ]
    );

    // Multi-byte utf8 payload case.
    let utf8_mb_tree = wrapper_tree(RawSyntaxDecodedValue::Terminal(RawSyntaxDecodedTerminal {
        node_kind: "text".to_string(),
        terminal: "string".to_string(),
        payload: "é".to_string(),
    }));
    let utf8_mb_tokens = assert_stream_tree_inverses(&model, &utf8_mb_tree, "mixed_utf8_multibyte");
    assert!(utf8_mb_tokens.contains(&text_tok));
    assert!(utf8_mb_tokens.contains(&roles.payload_end));
    assert!(!utf8_mb_tokens.contains(&roles.terminal));

    // --- Mixed Nonterminal: CHOICE(n,k) + child record body ---
    let child_tree = wrapper_tree(RawSyntaxDecodedValue::Node(Box::new(
        RawSyntaxDecodedNode {
            rule_name: "child".to_string(),
            node_kind: "child".to_string(),
            fields: vec![],
        },
    )));
    let child_tokens = assert_stream_tree_inverses(&model, &child_tree, "mixed_nonterminal");
    // Field-less child: CHOICE (= child open) + node_end.
    assert_eq!(
        child_tokens,
        vec![
            roles.node_start,
            roles.field_start,
            child,
            roles.node_end,
            roles.field_end,
            roles.node_end,
        ],
        "mixed Nonterminal is CHOICE + rest of child record"
    );
    // Body between field_start and field_end: CHOICE + child node_end.
    assert_eq!(&child_tokens[2..4], &[child, roles.node_end]);

    // Open-field mask at choice state is exactly CHOICE(5,*) (Decision 3).
    let open_prefix = vec![roles.node_start, roles.field_start];
    let open_mask = mask_set(
        &model
            .admissible_next_raw_tokens(&open_prefix)
            .expect("open mixed field"),
        roles.payload_end,
    );
    let expected_choices: BTreeSet<u32> = [child, false_tok, number, text_tok, true_tok]
        .into_iter()
        .collect();
    assert_eq!(
        open_mask, expected_choices,
        "open mixed field under SharedAlphabet must admit exactly CHOICE(5,*)"
    );
    assert!(!open_mask.contains(&roles.terminal));
    assert!(!open_mask.contains(&roles.payload_end));
}
