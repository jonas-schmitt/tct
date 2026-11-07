//! I5 shared decision-token alphabet — mask/accept consistency regression.
//!
//! For every prefix of a valid stream, the admissible-token mask must equal
//! the set of tokens the raw state machine actually accepts. Divergence in
//! either direction is a defect:
//!
//! - accepted but not masked => constrained decoding is unsound
//! - masked but not accepted => constrained generation can deadlock
//!
//! Covers tree-sitter JSON and C subsets under SharedAlphabet, plus the
//! default PerGrammar models as a baseline so shared mode cannot silently
//! regress the historical path.
//!
//! Claim id: `i5_shared_alphabet_mask_accept_consistency_v0`
//! Boundary: `compiler_shared_alphabet_layout_only`
//!
//! Not a paper claim, production route, transfer experiment, pin rewrite, or
//! new hand-built subset codec.

use std::collections::BTreeSet;

use tct::shared_alphabet_layout::{
    SHARED_ALPHABET_CLAIM_BOUNDARY, SHARED_ALPHABET_VOCAB_SIZE, SyntaxTokenLayoutMode,
};
use tct::syntax_state_machine::{
    CompiledSyntaxModel, RawSyntaxAdmissibleTokens, RawSyntaxDecodedNode,
};
use tct::tree_sitter_c_subset_codec::{
    CSubsetProgram, TreeSitterCSubsetCodec, program_to_decoded_node,
};
use tct::tree_sitter_json_subset_codec::{
    JsonSubsetPair, JsonSubsetValue, TreeSitterJsonSubsetCodec, value_to_decoded_node,
};

const CLAIM_ID: &str = "i5_shared_alphabet_mask_accept_consistency_v0";
const CLAIM_BOUNDARY: &str = SHARED_ALPHABET_CLAIM_BOUNDARY;

/// Expand `admissible_next_raw_tokens` into the full finite admissible set.
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

/// Probe every id in `0..vocab_size` through `CompiledSyntaxModel::transition`.
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

/// For every prefix of `tokens` (including empty and complete), assert mask == accept.
fn assert_mask_accept_consistency(
    surface: &str,
    model: &CompiledSyntaxModel,
    tokens: &[u32],
    witness: &str,
) {
    let roles = model.token_roles();
    let payload_end = roles.payload_end;

    // Include the complete prefix so a post-complete mask drift cannot hide.
    for prefix_len in 0..=tokens.len() {
        let prefix = &tokens[..prefix_len];
        let admissible = model
            .admissible_next_raw_tokens(prefix)
            .unwrap_or_else(|error| {
                panic!(
                    "{surface}: admissible_next_raw_tokens failed at prefix_len={prefix_len} \
                 ({witness}; {CLAIM_BOUNDARY}): {error:?}"
                )
            });
        let masked = mask_set(&admissible, payload_end);
        let accepted = accepted_set(model, prefix);

        if masked != accepted {
            let only_masked: BTreeSet<_> = masked.difference(&accepted).copied().collect();
            let only_accepted: BTreeSet<_> = accepted.difference(&masked).copied().collect();
            panic!(
                "{surface}: mask/accept divergence at prefix_len={prefix_len}/{stream_len} \
                 ({witness}; claim {CLAIM_ID}; boundary {CLAIM_BOUNDARY}).\n\
                 only_masked (masked but not accepted — deadlock risk): {only_masked:?}\n\
                 only_accepted (accepted but not masked — unsound): {only_accepted:?}\n\
                 prefix_tail={prefix_tail:?}\n\
                 layout_mode={mode:?}",
                stream_len = tokens.len(),
                prefix_tail = if prefix_len == 0 {
                    Vec::new()
                } else {
                    prefix[prefix_len.saturating_sub(8)..].to_vec()
                },
                mode = model.layout_mode(),
            );
        }

        // The next stream token (when present) must be in the equal set.
        if prefix_len < tokens.len() {
            let next = tokens[prefix_len];
            assert!(
                masked.contains(&next),
                "{surface}: stream token {next} at offset {prefix_len} not in mask/accept set \
                 ({witness}); set={masked:?}"
            );
        }
    }
}

fn assert_tree_mask_accept(
    surface: &str,
    model: &CompiledSyntaxModel,
    tree: &RawSyntaxDecodedNode,
    witness: &str,
) {
    let tokens = model
        .encode_complete_tree(tree)
        .unwrap_or_else(|error| panic!("{surface}: encode failed for {witness}: {error:?}"));
    assert!(
        !tokens.is_empty(),
        "{surface}: encoded stream must be non-empty ({witness})"
    );
    for &token in &tokens {
        assert!(
            token < model.token_roles().pad_token && token < model.token_roles().vocab_size,
            "{surface}: token {token} must stay below PAD/VOCAB ({witness})"
        );
    }
    assert_mask_accept_consistency(surface, model, &tokens, witness);
}

/// Mask set built from `admissible_next_raw_tokens` equals the set accepted by
/// `transition` for every prefix, under SharedAlphabet (JSON + C) and
/// PerGrammar baseline.
#[test]
fn i5_shared_alphabet_mask_accept_consistency_json_and_c() {
    assert_eq!(CLAIM_BOUNDARY, "compiler_shared_alphabet_layout_only");
    assert_eq!(CLAIM_ID, "i5_shared_alphabet_mask_accept_consistency_v0");
    assert_eq!(
        SHARED_ALPHABET_CLAIM_BOUNDARY,
        "compiler_shared_alphabet_layout_only"
    );

    // --- JSON subset ---
    let json_codec = TreeSitterJsonSubsetCodec::from_pinned_fixture().expect("JSON subset codec");
    let json_shared = CompiledSyntaxModel::from_sidecar_with_mode(
        json_codec.sidecar(),
        SyntaxTokenLayoutMode::SharedAlphabet,
    )
    .expect("JSON subset SharedAlphabet compile");
    assert_eq!(
        json_shared.layout_mode(),
        SyntaxTokenLayoutMode::SharedAlphabet
    );
    assert_eq!(
        json_shared.token_roles().vocab_size,
        SHARED_ALPHABET_VOCAB_SIZE
    );

    let json_cases: Vec<(&str, JsonSubsetValue)> = vec![
        ("null", JsonSubsetValue::Null),
        ("true", JsonSubsetValue::True),
        ("number", JsonSubsetValue::Number(42)),
        // utf8 payload (multi-byte)
        ("utf8_string", JsonSubsetValue::String("café".to_string())),
        // nested / recursive object→array→object
        (
            "nested_object",
            JsonSubsetValue::Object(vec![
                JsonSubsetPair {
                    key: "outer".to_string(),
                    value: JsonSubsetValue::Object(vec![JsonSubsetPair {
                        key: "inner".to_string(),
                        value: JsonSubsetValue::Array(vec![
                            JsonSubsetValue::Number(1),
                            JsonSubsetValue::String("x".to_string()),
                            JsonSubsetValue::Null,
                        ]),
                    }]),
                },
                JsonSubsetPair {
                    key: "tail".to_string(),
                    value: JsonSubsetValue::True,
                },
            ]),
        ),
    ];

    for (name, value) in &json_cases {
        let tree = value_to_decoded_node(value);
        assert_tree_mask_accept("json_shared", &json_shared, &tree, name);
    }

    // PerGrammar baseline: same property on the default codec model (Decision 4).
    let json_pg = json_codec.model();
    assert_eq!(json_pg.layout_mode(), SyntaxTokenLayoutMode::PerGrammar);
    for (name, value) in &json_cases {
        let tree = value_to_decoded_node(value);
        assert_tree_mask_accept("json_per_grammar", json_pg, &tree, name);
    }

    // --- C subset ---
    let c_codec = TreeSitterCSubsetCodec::from_pinned_fixture().expect("C subset codec");
    let c_shared = CompiledSyntaxModel::from_sidecar_with_mode(
        c_codec.sidecar(),
        SyntaxTokenLayoutMode::SharedAlphabet,
    )
    .expect("C subset SharedAlphabet compile");
    assert_eq!(
        c_shared.layout_mode(),
        SyntaxTokenLayoutMode::SharedAlphabet
    );
    assert_eq!(
        c_shared.token_roles().vocab_size,
        SHARED_ALPHABET_VOCAB_SIZE
    );

    let c_sources = [
        ("minimal", "int f(void) { return 0; }"),
        ("return_42", "int main(void) { return 42; }"),
        // multi-statement body (utf8 identifiers + nested structure)
        ("multi_return", "int f(void) {\n  return 1;\n  return 2;\n}"),
        (
            "two_functions",
            "int main(void) {\n  return 0;\n}\nint foo(void) {\n  return 7;\n}",
        ),
    ];

    for (name, source) in c_sources {
        let program = CSubsetProgram::parse(source)
            .unwrap_or_else(|error| panic!("C parse failed for {name}: {error:?}"));
        let tree = program_to_decoded_node(&program)
            .unwrap_or_else(|error| panic!("program_to_decoded_node failed for {name}: {error:?}"));
        assert_tree_mask_accept("c_shared", &c_shared, &tree, name);
    }

    let c_pg = c_codec.model();
    assert_eq!(c_pg.layout_mode(), SyntaxTokenLayoutMode::PerGrammar);
    for (name, source) in c_sources {
        let program = CSubsetProgram::parse(source)
            .unwrap_or_else(|error| panic!("C parse failed for {name}: {error:?}"));
        let tree = program_to_decoded_node(&program)
            .unwrap_or_else(|error| panic!("program_to_decoded_node failed for {name}: {error:?}"));
        assert_tree_mask_accept("c_per_grammar", c_pg, &tree, name);
    }
}
