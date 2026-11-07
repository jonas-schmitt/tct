//! I5 shared decision-token alphabet — layout constants and constructor.
//!
//! Checklist tranches 2–3 of `docs/i5-shared-decision-token-alphabet-contract.md`
//! (claim `i5_shared_decision_token_alphabet_contract_v0`, boundary
//! `compiler_shared_alphabet_layout_only`).
//!
//! This module owns the fixed shared ranges, bijective `CHOICE(n,k)` packing,
//! and the selectable layout-mode enum (Decision 4). Role assignment under
//! `SharedAlphabet` is wired from `CompiledSyntaxModel`; this module does
//! **not** register a production route or change default per-grammar pin
//! paths.

use crate::grammar_ir_structural_model::{
    GrammarIntegerValueRange, GrammarStructuralTokenLayout, GrammarStructuralTokenRange,
};
use crate::syntax_backend_boundary::SyntaxBackendBoundaryError;
use crate::syntax_token_layout::RAW_TOKEN_ALTERNATIVE_NODE_START_OFFSET;
use serde::{Deserialize, Serialize};

/// Selectable syntax token-layout mode (I5 Decision 4).
///
/// `PerGrammar` is the default production-adjacent path: absolute Go/C# and
/// subset golden pins stay on per-grammar free-slot / pin-table assignment.
/// `SharedAlphabet` is an opt-in second layout mode that draws multi-alt
/// choice tokens only from the shared `CHOICE(n,k)` band.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SyntaxTokenLayoutMode {
    /// Per-grammar layout: pinned Go table or alphabetical free-slot scan.
    #[default]
    PerGrammar,
    /// Shared decision-token alphabet: domain-stable `CHOICE(n,k)` ordinals.
    SharedAlphabet,
}

/// Census-settled `MAX_BRANCH_ARITY` for the shared decision-token alphabet.
///
/// Evidence: `tests/i5_shared_alphabet_arity_census_test.rs` (overall max 40
/// from tree-sitter-rust held-out `_expression.subtype`). Raising this constant
/// is a deliberate shared-alphabet revision, not a per-grammar spill.
pub const SHARED_ALPHABET_MAX_BRANCH_ARITY: u32 = 40;

/// Choice-token count: `sum_{n=2}^{MAX} n = MAX·(MAX+1)/2 − 1`.
pub const SHARED_ALPHABET_CHOICE_TOKEN_COUNT: u32 =
    SHARED_ALPHABET_MAX_BRANCH_ARITY * (SHARED_ALPHABET_MAX_BRANCH_ARITY + 1) / 2 - 1;

/// Structural band width: fixed bracket roles + `CHOICE(n,k)` pool.
pub const SHARED_ALPHABET_STRUCTURAL_BAND_SIZE: u32 =
    RAW_TOKEN_ALTERNATIVE_NODE_START_OFFSET + SHARED_ALPHABET_CHOICE_TOKEN_COUNT;

/// UTF-8 byte payload band width (shared across grammars).
pub const SHARED_ALPHABET_UTF8_PAYLOAD_WIDTH: u32 = 256;

/// u8 integer payload band width (shared across grammars).
pub const SHARED_ALPHABET_U8_INTEGER_PAYLOAD_WIDTH: u32 = 256;

/// PAD token for the shared alphabet (immediately after raw base ranges).
pub const SHARED_ALPHABET_PAD_TOKEN: u32 = SHARED_ALPHABET_STRUCTURAL_BAND_SIZE
    + SHARED_ALPHABET_UTF8_PAYLOAD_WIDTH
    + SHARED_ALPHABET_U8_INTEGER_PAYLOAD_WIDTH;

/// Shared alphabet `VOCAB_SIZE` (PAD + one).
pub const SHARED_ALPHABET_VOCAB_SIZE: u32 = SHARED_ALPHABET_PAD_TOKEN + 1;

/// Claim boundary string for compiler-side shared-alphabet evidence.
pub const SHARED_ALPHABET_CLAIM_BOUNDARY: &str = "compiler_shared_alphabet_layout_only";

/// Structural-relative offset of the first `CHOICE(n, *)` token for arity `n`.
///
/// Packing (contract Decision 1):
/// `choice_base(n) = RAW_TOKEN_ALTERNATIVE_NODE_START_OFFSET + (n·(n-1)/2 − 1)`
/// for `n ∈ 2..=MAX_BRANCH_ARITY`.
pub fn shared_alphabet_choice_base_offset(arity: u32) -> Result<u32, SyntaxBackendBoundaryError> {
    if !(2..=SHARED_ALPHABET_MAX_BRANCH_ARITY).contains(&arity) {
        return Err(SyntaxBackendBoundaryError::new(
            "shared_alphabet.choice_base",
            format!(
                "arity {arity} is outside 2..={} (SHARED_ALPHABET_MAX_BRANCH_ARITY)",
                SHARED_ALPHABET_MAX_BRANCH_ARITY
            ),
        ));
    }
    // n*(n-1)/2 - 1 is non-negative for n >= 2.
    let triangular_prefix = arity * (arity - 1) / 2 - 1;
    Ok(RAW_TOKEN_ALTERNATIVE_NODE_START_OFFSET + triangular_prefix)
}

/// Structural-relative offset of `CHOICE(n, k)` for `0 <= k < n`.
pub fn shared_alphabet_choice_offset(
    arity: u32,
    ordinal: u32,
) -> Result<u32, SyntaxBackendBoundaryError> {
    if ordinal >= arity {
        return Err(SyntaxBackendBoundaryError::new(
            "shared_alphabet.choice_offset",
            format!("ordinal {ordinal} is not strictly below arity {arity}"),
        ));
    }
    let base = shared_alphabet_choice_base_offset(arity)?;
    Ok(base + ordinal)
}

/// Absolute token id of `CHOICE(n, k)` inside a structural range that starts at
/// `structural_range_start` (normally 0 for the shared layout constructor).
pub fn shared_alphabet_choice_token_id(
    structural_range_start: u32,
    arity: u32,
    ordinal: u32,
) -> Result<u32, SyntaxBackendBoundaryError> {
    let offset = shared_alphabet_choice_offset(arity, ordinal)?;
    structural_range_start.checked_add(offset).ok_or_else(|| {
        SyntaxBackendBoundaryError::new(
            "shared_alphabet.choice_token_id",
            format!(
                "token id overflow for structural_start={structural_range_start} arity={arity} ordinal={ordinal}"
            ),
        )
    })
}

/// Invert a structural-relative offset in the choice band to `(arity, ordinal)`.
///
/// Offsets below [`RAW_TOKEN_ALTERNATIVE_NODE_START_OFFSET`] are fixed bracket
/// roles and are rejected. Offsets at or above the structural band end are
/// rejected.
pub fn shared_alphabet_unpack_choice_offset(
    offset: u32,
) -> Result<(u32, u32), SyntaxBackendBoundaryError> {
    if offset < RAW_TOKEN_ALTERNATIVE_NODE_START_OFFSET {
        return Err(SyntaxBackendBoundaryError::new(
            "shared_alphabet.unpack_choice",
            format!(
                "offset {offset} is below the choice band start {}",
                RAW_TOKEN_ALTERNATIVE_NODE_START_OFFSET
            ),
        ));
    }
    if offset >= SHARED_ALPHABET_STRUCTURAL_BAND_SIZE {
        return Err(SyntaxBackendBoundaryError::new(
            "shared_alphabet.unpack_choice",
            format!(
                "offset {offset} is at or above the structural band end {}",
                SHARED_ALPHABET_STRUCTURAL_BAND_SIZE
            ),
        ));
    }
    // Scan is O(MAX_BRANCH_ARITY) and exact; closed form is optional.
    for arity in 2..=SHARED_ALPHABET_MAX_BRANCH_ARITY {
        // Loop bounds already constrain arity into the supported range.
        let Ok(base) = shared_alphabet_choice_base_offset(arity) else {
            continue;
        };
        if offset >= base && offset < base + arity {
            return Ok((arity, offset - base));
        }
    }
    Err(SyntaxBackendBoundaryError::new(
        "shared_alphabet.unpack_choice",
        format!("offset {offset} did not resolve to any CHOICE(n, k) packing slot"),
    ))
}

/// Fixed shared decision-token alphabet layout (no per-grammar free-slot size).
///
/// Ranges are independent of any grammar fixture. Default production paths do
/// **not** use this constructor; selectable shared mode wiring is later
/// checklist work. Absolute Go/C# and subset golden pins stay on per-grammar
/// layouts (Decision 4).
pub fn shared_alphabet_token_layout() -> GrammarStructuralTokenLayout {
    let structural_end = SHARED_ALPHABET_STRUCTURAL_BAND_SIZE;
    let utf8_end = structural_end + SHARED_ALPHABET_UTF8_PAYLOAD_WIDTH;
    let integer_end = utf8_end + SHARED_ALPHABET_U8_INTEGER_PAYLOAD_WIDTH;
    debug_assert_eq!(integer_end, SHARED_ALPHABET_PAD_TOKEN);
    debug_assert_eq!(SHARED_ALPHABET_PAD_TOKEN + 1, SHARED_ALPHABET_VOCAB_SIZE);
    GrammarStructuralTokenLayout {
        vocab_size: SHARED_ALPHABET_VOCAB_SIZE,
        pad_token: SHARED_ALPHABET_PAD_TOKEN,
        ranges: vec![
            GrammarStructuralTokenRange {
                id: "shared_alphabet.structural_tokens".to_string(),
                role: "structural_decisions".to_string(),
                start: 0,
                end_exclusive: structural_end,
            },
            GrammarStructuralTokenRange {
                id: "shared_alphabet.utf8_byte_payload".to_string(),
                role: "utf8_byte_payload".to_string(),
                start: structural_end,
                end_exclusive: utf8_end,
            },
            GrammarStructuralTokenRange {
                id: "shared_alphabet.u8_integer_payload".to_string(),
                role: "u8_integer_payload".to_string(),
                start: utf8_end,
                end_exclusive: integer_end,
            },
        ],
        integer_value_range: Some(GrammarIntegerValueRange {
            id: "shared_alphabet.u8_integer_value".to_string(),
            valid_min: 0,
            valid_max: 255,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::syntax_token_layout::{csharp_syntax_token_layout, go_syntax_token_layout};
    use std::collections::BTreeSet;

    /// I5 checklist tranche 2: bijective `CHOICE(n,k)` packing for all arities.
    #[test]
    fn shared_alphabet_choice_pack_unpack_is_bijective_for_all_arities() {
        assert_eq!(
            SHARED_ALPHABET_CLAIM_BOUNDARY,
            "compiler_shared_alphabet_layout_only"
        );
        assert_eq!(SHARED_ALPHABET_MAX_BRANCH_ARITY, 40);
        assert_eq!(SHARED_ALPHABET_CHOICE_TOKEN_COUNT, 819);
        assert_eq!(SHARED_ALPHABET_STRUCTURAL_BAND_SIZE, 829);

        // Contract examples: arity 2 at 10,11; arity 3 at 12,13,14.
        assert_eq!(shared_alphabet_choice_base_offset(2).unwrap(), 10);
        assert_eq!(shared_alphabet_choice_offset(2, 0).unwrap(), 10);
        assert_eq!(shared_alphabet_choice_offset(2, 1).unwrap(), 11);
        assert_eq!(shared_alphabet_choice_base_offset(3).unwrap(), 12);
        assert_eq!(shared_alphabet_choice_offset(3, 2).unwrap(), 14);

        let mut seen = BTreeSet::new();
        for arity in 2..=SHARED_ALPHABET_MAX_BRANCH_ARITY {
            for ordinal in 0..arity {
                let offset = shared_alphabet_choice_offset(arity, ordinal)
                    .unwrap_or_else(|error| panic!("pack CHOICE({arity},{ordinal}): {error}"));
                assert!(
                    seen.insert(offset),
                    "CHOICE({arity},{ordinal}) offset {offset} collides with another packing slot"
                );
                let (unpacked_arity, unpacked_ordinal) =
                    shared_alphabet_unpack_choice_offset(offset)
                        .unwrap_or_else(|error| panic!("unpack offset {offset}: {error}"));
                assert_eq!(
                    (unpacked_arity, unpacked_ordinal),
                    (arity, ordinal),
                    "pack/unpack round-trip failed for CHOICE({arity},{ordinal})"
                );
                let absolute = shared_alphabet_choice_token_id(0, arity, ordinal).unwrap();
                assert_eq!(absolute, offset);
                let shifted = shared_alphabet_choice_token_id(100, arity, ordinal).unwrap();
                assert_eq!(shifted, 100 + offset);
            }
        }
        assert_eq!(
            seen.len(),
            SHARED_ALPHABET_CHOICE_TOKEN_COUNT as usize,
            "every choice slot must be occupied exactly once"
        );
        assert_eq!(
            *seen.first().expect("non-empty"),
            RAW_TOKEN_ALTERNATIVE_NODE_START_OFFSET
        );
        assert_eq!(
            *seen.last().expect("non-empty"),
            SHARED_ALPHABET_STRUCTURAL_BAND_SIZE - 1
        );

        assert!(shared_alphabet_choice_base_offset(1).is_err());
        assert!(shared_alphabet_choice_base_offset(SHARED_ALPHABET_MAX_BRANCH_ARITY + 1).is_err());
        assert!(shared_alphabet_choice_offset(2, 2).is_err());
        assert!(shared_alphabet_unpack_choice_offset(0).is_err());
        assert!(
            shared_alphabet_unpack_choice_offset(SHARED_ALPHABET_STRUCTURAL_BAND_SIZE).is_err()
        );
    }

    /// I5 checklist tranche 2: shared layout ranges validate and are grammar-independent.
    #[test]
    fn shared_alphabet_token_layout_validates_and_is_grammar_independent() {
        assert_eq!(
            SHARED_ALPHABET_CLAIM_BOUNDARY,
            "compiler_shared_alphabet_layout_only"
        );

        let layout = shared_alphabet_token_layout();
        layout
            .validate()
            .expect("shared alphabet layout must pass validate_token_layout");

        // Fixed sizes from Decision 1 at MAX_BRANCH_ARITY=40.
        assert_eq!(layout.vocab_size, SHARED_ALPHABET_VOCAB_SIZE);
        assert_eq!(layout.pad_token, SHARED_ALPHABET_PAD_TOKEN);
        assert_eq!(layout.vocab_size, 1342);
        assert_eq!(layout.pad_token, 1341);
        assert_eq!(layout.raw_base_vocab_size(), SHARED_ALPHABET_PAD_TOKEN);

        let structural = layout
            .ranges
            .iter()
            .find(|range| range.role == "structural_decisions")
            .expect("structural_decisions range");
        assert_eq!(structural.start, 0);
        assert_eq!(
            structural.end_exclusive,
            SHARED_ALPHABET_STRUCTURAL_BAND_SIZE
        );
        assert_eq!(structural.end_exclusive, 829);
        assert_eq!(structural.id, "shared_alphabet.structural_tokens");

        let utf8 = layout
            .ranges
            .iter()
            .find(|range| range.role == "utf8_byte_payload")
            .expect("utf8_byte_payload range");
        assert_eq!(utf8.start, 829);
        assert_eq!(utf8.end_exclusive, 829 + 256);

        let integer = layout
            .ranges
            .iter()
            .find(|range| range.role == "u8_integer_payload")
            .expect("u8_integer_payload range");
        assert_eq!(integer.start, 829 + 256);
        assert_eq!(integer.end_exclusive, SHARED_ALPHABET_PAD_TOKEN);

        // Size is independent of any grammar fixture: constructor takes no model,
        // and a second call is bit-identical (no free-slot scan state).
        assert_eq!(layout, shared_alphabet_token_layout());

        // Shared layout must not silently alias default per-grammar constructors
        // (Decision 4: selectable mode; default pins stay on per-grammar ranges).
        assert_ne!(layout.vocab_size, go_syntax_token_layout().vocab_size);
        assert_ne!(layout.vocab_size, csharp_syntax_token_layout().vocab_size);
        assert_ne!(
            layout.ranges[0].end_exclusive,
            go_syntax_token_layout().ranges[0].end_exclusive
        );
    }
}
