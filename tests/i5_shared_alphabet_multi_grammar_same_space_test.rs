//! I5 shared decision-token alphabet — checklist tranche 11 multi-grammar
//! same-space witness.
//!
//! Compiles JSON subset, C subset, and held-out rust under
//! `SyntaxTokenLayoutMode::SharedAlphabet` and asserts they share identical
//! fixed layout constants (vocab, pad, structural/payload ranges, bracket
//! roles) and the same absolute `CHOICE(n,k)` packing band.
//!
//! Claim id: `i5_shared_alphabet_multi_grammar_same_space_v0`
//! Boundary: `compiler_shared_alphabet_layout_only`
//!
//! Not a paper claim, production route, transfer experiment, pin rewrite, or
//! new hand-built subset codec. Default PerGrammar absolute goldens stay
//! unedited (Decision 4).

use std::collections::BTreeSet;

use tct::grammar_ir_structural_model::{
    GrammarIntegerValueRange, GrammarStructuralTokenLayout, GrammarStructuralTokenRange,
    lower_grammar_ir_to_structural_model,
};
use tct::shared_alphabet_layout::{
    SHARED_ALPHABET_CLAIM_BOUNDARY, SHARED_ALPHABET_PAD_TOKEN,
    SHARED_ALPHABET_STRUCTURAL_BAND_SIZE, SHARED_ALPHABET_VOCAB_SIZE, SyntaxTokenLayoutMode,
    shared_alphabet_choice_token_id, shared_alphabet_token_layout,
    shared_alphabet_unpack_choice_offset,
};
use tct::syntax_state_machine::{CompiledSyntaxModel, SyntaxTokenRoles};
use tct::syntax_token_layout::RAW_TOKEN_ALTERNATIVE_NODE_START_OFFSET;
use tct::tree_sitter_c_subset_codec::TreeSitterCSubsetCodec;
use tct::tree_sitter_json_subset_codec::TreeSitterJsonSubsetCodec;
use tct::tree_sitter_metadata::{TreeSitterGrammarPin, TreeSitterMetadataAdapter};

const CLAIM_ID: &str = "i5_shared_alphabet_multi_grammar_same_space_v0";
const CLAIM_BOUNDARY: &str = SHARED_ALPHABET_CLAIM_BOUNDARY;

/// Oversized structural band for held-out rust SM lower only — plumbing, not a
/// production layout. SharedAlphabet replaces ranges at compile time.
fn rust_held_out_sidecar_token_layout() -> GrammarStructuralTokenLayout {
    GrammarStructuralTokenLayout {
        vocab_size: 4097,
        pad_token: 4096,
        ranges: vec![
            GrammarStructuralTokenRange {
                id: "tree_sitter_rust.structural_tokens".to_string(),
                role: "structural_decisions".to_string(),
                start: 0,
                end_exclusive: 3072,
            },
            GrammarStructuralTokenRange {
                id: "tree_sitter_rust.utf8_byte_payload".to_string(),
                role: "utf8_byte_payload".to_string(),
                start: 3072,
                end_exclusive: 3328,
            },
            GrammarStructuralTokenRange {
                id: "tree_sitter_rust.u8_integer_payload".to_string(),
                role: "u8_integer_payload".to_string(),
                start: 3328,
                end_exclusive: 4096,
            },
        ],
        integer_value_range: Some(GrammarIntegerValueRange {
            id: "tree_sitter_rust.u8_integer_value".to_string(),
            valid_min: 0,
            valid_max: 255,
        }),
    }
}

/// Layout fingerprint that must be identical across grammars under SharedAlphabet.
#[derive(Debug, Clone, PartialEq, Eq)]
struct SharedLayoutFingerprint {
    vocab_size: u32,
    pad_token: u32,
    structural_start: u32,
    structural_end: u32,
    utf8_start: u32,
    utf8_end: u32,
    integer_start: u32,
    integer_end: u32,
    node_start: u32,
    node_end: u32,
    field_start: u32,
    field_end: u32,
    field_missing: u32,
    repeated_empty: u32,
    terminal: u32,
    payload_end: u32,
    boolean_false: u32,
    boolean_true: u32,
}

fn fingerprint(roles: &SyntaxTokenRoles) -> SharedLayoutFingerprint {
    SharedLayoutFingerprint {
        vocab_size: roles.vocab_size,
        pad_token: roles.pad_token,
        structural_start: roles.structural_range.start,
        structural_end: roles.structural_range.end_exclusive,
        utf8_start: roles.utf8_range.start,
        utf8_end: roles.utf8_range.end_exclusive,
        integer_start: roles.integer_range.start,
        integer_end: roles.integer_range.end_exclusive,
        node_start: roles.node_start,
        node_end: roles.node_end,
        field_start: roles.field_start,
        field_end: roles.field_end,
        field_missing: roles.field_missing,
        repeated_empty: roles.repeated_empty,
        terminal: roles.terminal,
        payload_end: roles.payload_end,
        boolean_false: roles.boolean_false,
        boolean_true: roles.boolean_true,
    }
}

/// Expected fingerprint from Decision 1 / `shared_alphabet_token_layout()`.
fn expected_shared_fingerprint() -> SharedLayoutFingerprint {
    let layout = shared_alphabet_token_layout();
    let structural = layout
        .ranges
        .iter()
        .find(|range| range.role == "structural_decisions")
        .expect("shared layout structural_decisions");
    let utf8 = layout
        .ranges
        .iter()
        .find(|range| range.role == "utf8_byte_payload")
        .expect("shared layout utf8_byte_payload");
    let integer = layout
        .ranges
        .iter()
        .find(|range| range.role == "u8_integer_payload")
        .expect("shared layout u8_integer_payload");
    SharedLayoutFingerprint {
        vocab_size: SHARED_ALPHABET_VOCAB_SIZE,
        pad_token: SHARED_ALPHABET_PAD_TOKEN,
        structural_start: structural.start,
        structural_end: structural.end_exclusive,
        utf8_start: utf8.start,
        utf8_end: utf8.end_exclusive,
        integer_start: integer.start,
        integer_end: integer.end_exclusive,
        node_start: structural.start,   // RAW_NODE_START_OFFSET = 0
        node_end: structural.start + 1, // RAW_NODE_END_OFFSET = 1
        field_start: structural.start + 2,
        field_end: structural.start + 3,
        field_missing: structural.start + 4,
        repeated_empty: structural.start + 5,
        terminal: structural.start + 6,
        payload_end: structural.start + 7,
        boolean_false: structural.start + 8,
        boolean_true: structural.start + 9,
    }
}

/// Every field-choice token must sit in the shared CHOICE band and unpack to
/// `(arity, ordinal)` with arity matching the field row length.
fn assert_choice_band_usage(surface: &str, roles: &SyntaxTokenRoles) {
    assert_eq!(
        roles.layout_mode,
        SyntaxTokenLayoutMode::SharedAlphabet,
        "{surface} must be SharedAlphabet"
    );
    assert!(
        !roles.field_choice_table.is_empty(),
        "{surface} SharedAlphabet must populate field_choice_table for multi-alt fields"
    );

    // Structural band is always [0, STRUCTURAL_BAND_SIZE) under the shared ctor.
    assert_eq!(roles.structural_range.start, 0);
    assert_eq!(
        roles.structural_range.end_exclusive,
        SHARED_ALPHABET_STRUCTURAL_BAND_SIZE
    );
    let choice_band_start = RAW_TOKEN_ALTERNATIVE_NODE_START_OFFSET;
    let choice_band_end = SHARED_ALPHABET_STRUCTURAL_BAND_SIZE;

    for ((rule, field), alts) in &roles.field_choice_table {
        let n = alts.len() as u32;
        assert!(
            n >= 2,
            "{surface} field {rule}.{field} choice row must have arity >= 2, got {n}"
        );
        for (k, alt) in alts.iter().enumerate() {
            let k = k as u32;
            assert!(
                alt.token_id >= choice_band_start && alt.token_id < choice_band_end,
                "{surface} {rule}.{field}[{k}] token {} is outside CHOICE band [{choice_band_start}, {choice_band_end})",
                alt.token_id
            );
            let offset = alt
                .token_id
                .checked_sub(roles.structural_range.start)
                .expect("token above structural start");
            let (unpacked_n, unpacked_k) = shared_alphabet_unpack_choice_offset(offset)
                .unwrap_or_else(|error| {
                    panic!(
                        "{surface} {rule}.{field}[{k}] token {} failed CHOICE unpack: {error}",
                        alt.token_id
                    )
                });
            assert_eq!(
                (unpacked_n, unpacked_k),
                (n, k),
                "{surface} {rule}.{field}[{k}] must pack as CHOICE({n},{k}); got CHOICE({unpacked_n},{unpacked_k})"
            );
            let expected = shared_alphabet_choice_token_id(roles.structural_range.start, n, k)
                .expect("CHOICE pack");
            assert_eq!(
                alt.token_id, expected,
                "{surface} {rule}.{field}[{k}] absolute id must match shared packing"
            );
        }
    }
}

/// Collect absolute CHOICE ids used by any multi-alt field of each arity.
fn choice_ids_by_arity(roles: &SyntaxTokenRoles) -> BTreeSet<(u32, u32, u32)> {
    let mut out = BTreeSet::new();
    for alts in roles.field_choice_table.values() {
        let n = alts.len() as u32;
        for (k, alt) in alts.iter().enumerate() {
            out.insert((n, k as u32, alt.token_id));
        }
    }
    out
}

/// I5 checklist tranche 11: JSON + C + held-out rust share one SharedAlphabet
/// token space (identical vocab/ranges/brackets; bijective CHOICE band).
/// Claim id: `i5_shared_alphabet_multi_grammar_same_space_v0`.
/// Boundary: `compiler_shared_alphabet_layout_only`.
#[test]
fn i5_shared_alphabet_multi_grammar_same_space_witness() {
    assert_eq!(CLAIM_ID, "i5_shared_alphabet_multi_grammar_same_space_v0");
    assert_eq!(CLAIM_BOUNDARY, "compiler_shared_alphabet_layout_only");
    assert_eq!(
        SHARED_ALPHABET_CLAIM_BOUNDARY,
        "compiler_shared_alphabet_layout_only"
    );

    // --- Compile three distinct grammars under SharedAlphabet ---
    let json_codec = TreeSitterJsonSubsetCodec::from_pinned_fixture().expect("JSON subset codec");
    assert_eq!(
        json_codec.model().layout_mode(),
        SyntaxTokenLayoutMode::PerGrammar,
        "default JSON codec must remain PerGrammar (Decision 4)"
    );
    let json_shared = CompiledSyntaxModel::from_sidecar_with_mode(
        json_codec.sidecar(),
        SyntaxTokenLayoutMode::SharedAlphabet,
    )
    .expect("JSON subset must compile under SharedAlphabet");

    let c_codec = TreeSitterCSubsetCodec::from_pinned_fixture().expect("C subset codec");
    assert_eq!(
        c_codec.model().layout_mode(),
        SyntaxTokenLayoutMode::PerGrammar,
        "default C codec must remain PerGrammar (Decision 4)"
    );
    let c_shared = CompiledSyntaxModel::from_sidecar_with_mode(
        c_codec.sidecar(),
        SyntaxTokenLayoutMode::SharedAlphabet,
    )
    .expect("C subset must compile under SharedAlphabet");

    let rust_adapter = TreeSitterMetadataAdapter::new("tree_sitter_rust_v0_24_0");
    let (rust_grammar, rust_pin) = rust_adapter
        .lower_pinned_grammar_dir(&TreeSitterGrammarPin::rust_fixture_dir())
        .expect("held-out rust pin must lower without language-keyed classification");
    assert_eq!(rust_pin.pin_id, "tree_sitter_rust_node_types_v0.24.0");
    let rust_sidecar =
        lower_grammar_ir_to_structural_model(&rust_grammar, rust_held_out_sidecar_token_layout())
            .expect("held-out rust GrammarIR must lower to StructuralModel");
    // Sidecar still carries oversized PerGrammar plumbing ranges; SharedAlphabet
    // replaces them at compile time (same pattern as tranche 10).
    assert_ne!(
        rust_sidecar.structural_model.token_ranges.vocab_size, SHARED_ALPHABET_VOCAB_SIZE,
        "rust sidecar plumbing layout must stay distinct from shared constants"
    );
    let rust_shared = CompiledSyntaxModel::from_sidecar_with_mode(
        &rust_sidecar,
        SyntaxTokenLayoutMode::SharedAlphabet,
    )
    .expect("held-out rust must compile under SharedAlphabet");

    let surfaces: [(&str, &CompiledSyntaxModel); 3] = [
        ("json_subset", &json_shared),
        ("c_subset", &c_shared),
        ("rust_held_out", &rust_shared),
    ];

    let expected = expected_shared_fingerprint();
    let mut fingerprints = Vec::new();
    let mut all_choice_triples: BTreeSet<(u32, u32, u32)> = BTreeSet::new();

    for (surface, model) in surfaces {
        assert_eq!(
            model.layout_mode(),
            SyntaxTokenLayoutMode::SharedAlphabet,
            "{surface} layout_mode"
        );
        let roles = model.token_roles();
        let fp = fingerprint(roles);
        assert_eq!(
            fp, expected,
            "{surface} SharedAlphabet layout fingerprint must match shared_alphabet_token_layout() \
             constants ({CLAIM_BOUNDARY})"
        );
        assert_choice_band_usage(surface, roles);
        all_choice_triples.extend(choice_ids_by_arity(roles));
        fingerprints.push((surface, fp));
    }

    // Pairwise equality (redundant with expected match, but names the multi-grammar claim).
    for window in fingerprints.windows(2) {
        let (a_name, a_fp) = &window[0];
        let (b_name, b_fp) = &window[1];
        assert_eq!(
            a_fp, b_fp,
            "{a_name} and {b_name} must share identical SharedAlphabet vocab/ranges/brackets"
        );
    }

    // Cross-grammar packing identity: every observed CHOICE(n,k) absolute id is
    // the same function of (n,k) regardless of which grammar emitted the row.
    for (n, k, token_id) in &all_choice_triples {
        let expected_id = shared_alphabet_choice_token_id(0, *n, *k).expect("pack");
        assert_eq!(
            *token_id, expected_id,
            "cross-grammar CHOICE({n},{k}) must use absolute id {expected_id}, got {token_id}"
        );
    }
    // Overlapping arities across at least two surfaces (JSON uses 7; C uses 2;
    // rust uses many including 2) prove the band is not grammar-local free slots.
    let json_arities: BTreeSet<u32> = json_shared
        .token_roles()
        .field_choice_table
        .values()
        .map(|alts| alts.len() as u32)
        .collect();
    let c_arities: BTreeSet<u32> = c_shared
        .token_roles()
        .field_choice_table
        .values()
        .map(|alts| alts.len() as u32)
        .collect();
    let rust_arities: BTreeSet<u32> = rust_shared
        .token_roles()
        .field_choice_table
        .values()
        .map(|alts| alts.len() as u32)
        .collect();
    assert!(
        json_arities.contains(&7),
        "JSON subset _value.subtype is arity 7 under SharedAlphabet"
    );
    assert!(
        c_arities.contains(&2),
        "C subset should expose arity-2 multi-alt fields"
    );
    assert!(
        rust_arities.contains(&2),
        "held-out rust should expose arity-2 multi-alt fields"
    );
    let shared_arity_2_id = shared_alphabet_choice_token_id(0, 2, 0).expect("CHOICE(2,0)");
    for (surface, model) in [("c_subset", &c_shared), ("rust_held_out", &rust_shared)] {
        let has_choice_2_0 = model.token_roles().field_choice_table.values().any(|alts| {
            alts.first()
                .is_some_and(|alt| alt.token_id == shared_arity_2_id)
        });
        assert!(
            has_choice_2_0,
            "{surface} must reuse absolute CHOICE(2,0)={shared_arity_2_id} for some field's ordinal 0"
        );
    }
    // JSON null is CHOICE(7,2) — same absolute id any grammar would get for (7,2).
    let json_null = json_shared
        .token_roles()
        .alternative_token_for_field("_value", "subtype", "null")
        .expect("JSON null choice under SharedAlphabet");
    assert_eq!(
        json_null,
        shared_alphabet_choice_token_id(0, 7, 2).expect("CHOICE(7,2)"),
        "JSON null must be absolute CHOICE(7,2) in the shared space"
    );

    // Contrast: default PerGrammar paths stay on grammar-local free-slot
    // layouts (not the shared CHOICE alphabet). Decision 4 pin safety — no
    // absolute golden edits.
    let json_pg = json_codec.model().token_roles();
    let c_pg = c_codec.model().token_roles();
    assert_eq!(json_pg.layout_mode, SyntaxTokenLayoutMode::PerGrammar);
    assert_eq!(c_pg.layout_mode, SyntaxTokenLayoutMode::PerGrammar);
    assert_ne!(
        json_pg.vocab_size, SHARED_ALPHABET_VOCAB_SIZE,
        "JSON PerGrammar vocab must stay distinct from shared alphabet"
    );
    assert_ne!(
        c_pg.vocab_size, SHARED_ALPHABET_VOCAB_SIZE,
        "C PerGrammar vocab must stay distinct from shared alphabet"
    );
    // PerGrammar alternative maps are grammar-local free slots, not CHOICE
    // packing — even when subset range widths coincide (both 577).
    let json_null_pg = json_pg
        .node_start_alternative_tokens
        .get("null")
        .copied()
        .expect("JSON PerGrammar null free-slot");
    assert_ne!(
        json_null_pg, json_null,
        "PerGrammar null free-slot must differ from SharedAlphabet CHOICE(7,2)"
    );
    assert!(
        shared_alphabet_unpack_choice_offset(
            json_null_pg
                .checked_sub(json_pg.structural_range.start)
                .expect("null above structural start")
        )
        .is_err()
            || shared_alphabet_unpack_choice_offset(
                json_null_pg
                    .checked_sub(json_pg.structural_range.start)
                    .expect("null above structural start")
            )
            .ok()
                != Some((7, 2)),
        "PerGrammar null must not be absolute CHOICE(7,2) under the subset structural band"
    );
    // Field-relative CHOICE tables exist only under SharedAlphabet; PerGrammar
    // keeps the historical global free-slot map. That is the Decision 4 split:
    // multi-grammar same-space is opt-in SharedAlphabet only.
    assert!(
        json_pg.field_choice_table.is_empty(),
        "PerGrammar must not populate SharedAlphabet field_choice_table"
    );
    assert!(
        c_pg.field_choice_table.is_empty(),
        "PerGrammar must not populate SharedAlphabet field_choice_table"
    );
    assert!(
        !json_shared.token_roles().field_choice_table.is_empty()
            && !c_shared.token_roles().field_choice_table.is_empty()
            && !rust_shared.token_roles().field_choice_table.is_empty(),
        "SharedAlphabet field_choice_table is the multi-grammar same-space surface"
    );
}
