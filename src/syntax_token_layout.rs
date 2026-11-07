//! Target-neutral syntax token-layout contract.
//!
//! Owns the shared token-layout summary types, the fixed alternative
//! node_start offset, the alternative node_start token assignment, and the
//! per-target layout constructors consumed by the raw syntax state machine,
//! the source-instance projection, and the generated-codec layers. This
//! module sits on the state-machine side of the boundary: it depends only on
//! `grammar_ir`/`grammar_ir_structural_model`, the backend-boundary error
//! type, and the checked Go inventory's pinned structural-token table, never
//! on Go/C# report modules or the generated codec.

use crate::go_ast_schema_inventory::{
    GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH, GO_STRUCTURAL_TOKEN_ASSIGNMENT_APPEND_RULE,
    go_pinned_alternative_node_start_tokens,
};
use crate::grammar_ir::GrammarTerminalKind;
use crate::grammar_ir_structural_model::{
    GrammarIntegerValueRange, GrammarStructuralModelSidecar, GrammarStructuralTokenLayout,
    GrammarStructuralTokenRange,
};
use crate::syntax_backend_boundary::SyntaxBackendBoundaryError;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// Structural-token offset where alternative node_start tokens begin; the ten
/// fixed structural role offsets occupy the offsets below it.
pub const RAW_TOKEN_ALTERNATIVE_NODE_START_OFFSET: u32 = 10;

/// Go syntax layout `VOCAB_SIZE` for both scratch and source-file layouts.
pub const GO_SYNTAX_LAYOUT_VOCAB_SIZE: u32 = 641;
/// Go syntax layout `PAD_TOKEN` for both scratch and source-file layouts.
pub const GO_SYNTAX_LAYOUT_PAD_TOKEN: u32 = 640;

/// Serializable summary of a structural token layout: vocabulary size, pad
/// token, and the contiguous token ranges.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectionTokenLayoutSummary {
    pub vocab_size: u32,
    pub pad_token: u32,
    pub ranges: Vec<ProjectionTokenLayoutRangeSummary>,
}

impl ProjectionTokenLayoutSummary {
    pub(crate) fn from_layout(layout: &GrammarStructuralTokenLayout) -> Self {
        Self {
            vocab_size: layout.vocab_size,
            pad_token: layout.pad_token,
            ranges: layout
                .ranges
                .iter()
                .map(|range| ProjectionTokenLayoutRangeSummary {
                    id: range.id.clone(),
                    role: range.role.clone(),
                    start: range.start,
                    end_exclusive: range.end_exclusive,
                })
                .collect(),
        }
    }

    pub(crate) fn validate(&self) -> Result<(), SyntaxBackendBoundaryError> {
        require_u32(
            "projection.token_layout_mapping.token_layout.vocab_size",
            self.vocab_size,
            GO_SYNTAX_LAYOUT_VOCAB_SIZE,
        )?;
        require_u32(
            "projection.token_layout_mapping.token_layout.pad_token",
            self.pad_token,
            GO_SYNTAX_LAYOUT_PAD_TOKEN,
        )?;
        require_token_range(
            &self.ranges,
            "go.structural_tokens",
            "structural_decisions",
            0,
            128,
        )?;
        require_token_range(
            &self.ranges,
            "go.utf8_byte_payload",
            "utf8_byte_payload",
            128,
            384,
        )?;
        require_token_range(
            &self.ranges,
            "go.integer_lexeme_payload",
            "integer_lexeme_payload",
            384,
            GO_SYNTAX_LAYOUT_PAD_TOKEN,
        )
    }
}

/// One contiguous token range of a layout summary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectionTokenLayoutRangeSummary {
    pub id: String,
    pub role: String,
    pub start: u32,
    pub end_exclusive: u32,
}

/// Assign alternative node_start token IDs for a structural-model sidecar.
///
/// Go sidecars (the full pinned inventory grammar and every per-record
/// `go_*_syntax_v0` grammar) resolve each multi-alternative nonterminal node
/// kind through the global pinned table in the checked inventory fixture, so
/// per-record token maps can never diverge from the full-inventory layout.
/// Toy and C# grammars keep the alphabetical fallback assignment.
pub fn node_start_alternative_token_ids_from_sidecar(
    sidecar: &GrammarStructuralModelSidecar,
) -> Result<BTreeMap<String, u32>, SyntaxBackendBoundaryError> {
    let structural_range = token_range_by_role(
        &sidecar.structural_model.token_ranges,
        "structural_decisions",
    )?;
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
    let capacity = structural_range
        .end_exclusive
        .checked_sub(structural_range.start + RAW_TOKEN_ALTERNATIVE_NODE_START_OFFSET)
        .ok_or_else(|| {
            SyntaxBackendBoundaryError::new(
                "projection.token_layout_mapping.alternative_node_start_tokens",
                "structural token range cannot host alternative node_start tokens",
            )
        })?;
    if node_kinds.len() > capacity as usize {
        return Err(SyntaxBackendBoundaryError::new(
            "projection.token_layout_mapping.alternative_node_start_tokens",
            format!(
                "{} multi-alternative nonterminal node kinds exceed structural token capacity {capacity}",
                node_kinds.len()
            ),
        ));
    }
    // Checked Go grammar IDs are `go_ast_schema_inventory_go1_24_3` for the
    // full inventory and `go_*_syntax_v0` for per-record grammars; toy and C#
    // grammars use `toy_*` and `csharp_*`.
    if sidecar.input.grammar_id.starts_with("go_") {
        let pinned = go_pinned_alternative_node_start_tokens().map_err(|error| {
            SyntaxBackendBoundaryError::new(
                "projection.token_layout_mapping.alternative_node_start_tokens",
                format!("failed to load the pinned Go structural token assignment: {error}"),
            )
        })?;
        let mut assignments = BTreeMap::new();
        for node_kind in node_kinds {
            let Some(token_id) = pinned.get(&node_kind) else {
                return Err(SyntaxBackendBoundaryError::new(
                    "projection.token_layout_mapping.alternative_node_start_tokens",
                    format!(
                        "Go node kind {node_kind} has no pinned alternative node_start token in {GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH}; {GO_STRUCTURAL_TOKEN_ASSIGNMENT_APPEND_RULE}"
                    ),
                ));
            };
            let token = structural_range.start + *token_id;
            if token >= structural_range.end_exclusive {
                return Err(SyntaxBackendBoundaryError::new(
                    "projection.token_layout_mapping.alternative_node_start_tokens",
                    format!(
                        "pinned alternative node_start token {token} for {node_kind} does not fit structural range {}..{}",
                        structural_range.start, structural_range.end_exclusive
                    ),
                ));
            }
            assignments.insert(node_kind, token);
        }
        return Ok(assignments);
    }
    Ok(node_kinds
        .into_iter()
        .enumerate()
        .map(|(index, node_kind)| {
            (
                node_kind,
                structural_range.start + RAW_TOKEN_ALTERNATIVE_NODE_START_OFFSET + index as u32,
            )
        })
        .collect())
}

pub(crate) fn token_range_by_role(
    layout: &GrammarStructuralTokenLayout,
    role: &str,
) -> Result<ProjectionTokenLayoutRangeSummary, SyntaxBackendBoundaryError> {
    layout
        .ranges
        .iter()
        .find(|range| range.role == role)
        .map(|range| ProjectionTokenLayoutRangeSummary {
            id: range.id.clone(),
            role: range.role.clone(),
            start: range.start,
            end_exclusive: range.end_exclusive,
        })
        .ok_or_else(|| {
            SyntaxBackendBoundaryError::new(
                "projection.token_layout_mapping.token_range",
                format!("missing token-layout role {role}"),
            )
        })
}

fn require_token_range(
    ranges: &[ProjectionTokenLayoutRangeSummary],
    expected_id: &str,
    expected_role: &str,
    expected_start: u32,
    expected_end_exclusive: u32,
) -> Result<(), SyntaxBackendBoundaryError> {
    let Some(range) = ranges.iter().find(|range| range.id == expected_id) else {
        return Err(SyntaxBackendBoundaryError::new(
            "projection.token_layout_mapping.token_layout.ranges",
            format!("missing token range {expected_id}"),
        ));
    };
    require_string(
        "projection.token_layout_mapping.token_layout.range.role",
        &range.role,
        expected_role,
    )?;
    require_u32(
        "projection.token_layout_mapping.token_layout.range.start",
        range.start,
        expected_start,
    )?;
    require_u32(
        "projection.token_layout_mapping.token_layout.range.end_exclusive",
        range.end_exclusive,
        expected_end_exclusive,
    )
}

fn require_string(
    field: &'static str,
    actual: &str,
    expected: &str,
) -> Result<(), SyntaxBackendBoundaryError> {
    if actual == expected {
        Ok(())
    } else {
        Err(SyntaxBackendBoundaryError::new(
            field,
            format!("expected {expected}, got {actual}"),
        ))
    }
}

fn require_u32(
    field: &'static str,
    actual: u32,
    expected: u32,
) -> Result<(), SyntaxBackendBoundaryError> {
    if actual == expected {
        Ok(())
    } else {
        Err(SyntaxBackendBoundaryError::new(
            field,
            format!("expected {expected}, got {actual}"),
        ))
    }
}

/// The C# syntax token layout (64 structural tokens, UTF-8 byte payload, u8
/// integer payload).
pub fn csharp_syntax_token_layout() -> GrammarStructuralTokenLayout {
    GrammarStructuralTokenLayout {
        vocab_size: 577,
        pad_token: 576,
        ranges: vec![
            GrammarStructuralTokenRange {
                id: "csharp.structural_tokens".to_string(),
                role: "structural_decisions".to_string(),
                start: 0,
                end_exclusive: 64,
            },
            GrammarStructuralTokenRange {
                id: "csharp.utf8_byte_payload".to_string(),
                role: "utf8_byte_payload".to_string(),
                start: 64,
                end_exclusive: 320,
            },
            GrammarStructuralTokenRange {
                id: "csharp.u8_integer_payload".to_string(),
                role: "u8_integer_payload".to_string(),
                start: 320,
                end_exclusive: 576,
            },
        ],
        integer_value_range: Some(GrammarIntegerValueRange {
            id: "csharp.u8_integer_value".to_string(),
            valid_min: 0,
            valid_max: 255,
        }),
    }
}

/// The Go syntax token layout for per-construct scratch grammars (128
/// structural tokens, UTF-8 byte payload, u8 integer payload).
pub fn go_syntax_token_layout() -> GrammarStructuralTokenLayout {
    GrammarStructuralTokenLayout {
        vocab_size: GO_SYNTAX_LAYOUT_VOCAB_SIZE,
        pad_token: GO_SYNTAX_LAYOUT_PAD_TOKEN,
        ranges: vec![
            GrammarStructuralTokenRange {
                id: "go.structural_tokens".to_string(),
                role: "structural_decisions".to_string(),
                start: 0,
                end_exclusive: 128,
            },
            GrammarStructuralTokenRange {
                id: "go.utf8_byte_payload".to_string(),
                role: "utf8_byte_payload".to_string(),
                start: 128,
                end_exclusive: 384,
            },
            GrammarStructuralTokenRange {
                id: "go.u8_integer_payload".to_string(),
                role: "u8_integer_payload".to_string(),
                start: 384,
                end_exclusive: GO_SYNTAX_LAYOUT_PAD_TOKEN,
            },
        ],
        integer_value_range: Some(GrammarIntegerValueRange {
            id: "go.u8_integer_value".to_string(),
            valid_min: 0,
            valid_max: 255,
        }),
    }
}

/// The Go source-file syntax token layout (128 structural tokens, UTF-8 byte
/// payload, integer lexeme payload).
pub fn go_syntax_source_file_token_layout() -> GrammarStructuralTokenLayout {
    GrammarStructuralTokenLayout {
        vocab_size: GO_SYNTAX_LAYOUT_VOCAB_SIZE,
        pad_token: GO_SYNTAX_LAYOUT_PAD_TOKEN,
        ranges: vec![
            GrammarStructuralTokenRange {
                id: "go.structural_tokens".to_string(),
                role: "structural_decisions".to_string(),
                start: 0,
                end_exclusive: 128,
            },
            GrammarStructuralTokenRange {
                id: "go.utf8_byte_payload".to_string(),
                role: "utf8_byte_payload".to_string(),
                start: 128,
                end_exclusive: 384,
            },
            GrammarStructuralTokenRange {
                id: "go.integer_lexeme_payload".to_string(),
                role: "integer_lexeme_payload".to_string(),
                start: 384,
                end_exclusive: GO_SYNTAX_LAYOUT_PAD_TOKEN,
            },
        ],
        integer_value_range: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::syntax_token_layout_diff::{
        CSHARP_TOKEN_LAYOUT_PIN_FIXTURE_PATH, GO_TOKEN_LAYOUT_PIN_FIXTURE_PATH,
        RAW_SYNTAX_STREAM_CONTRACT_EXPANDED_V1, SyntaxTokenLayoutDescriptor,
        SyntaxTokenLayoutRangeDescriptor,
    };

    fn pin_fixture_current_descriptor(key: &str) -> SyntaxTokenLayoutDescriptor {
        let fixture: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(GO_TOKEN_LAYOUT_PIN_FIXTURE_PATH)
                .expect("checked Go token-layout pin fixture should be readable"),
        )
        .expect("checked Go token-layout pin fixture should be JSON");
        serde_json::from_value(fixture["current"][key].clone())
            .expect("pin fixture current descriptor should deserialize")
    }

    fn assert_layout_matches_pin_descriptor(
        layout: &GrammarStructuralTokenLayout,
        descriptor: &SyntaxTokenLayoutDescriptor,
    ) {
        assert_eq!(layout.vocab_size, descriptor.vocab_size);
        assert_eq!(layout.pad_token, descriptor.pad_token);
        let live_ranges: Vec<SyntaxTokenLayoutRangeDescriptor> = layout
            .ranges
            .iter()
            .map(|range| SyntaxTokenLayoutRangeDescriptor {
                id: range.id.clone(),
                role: range.role.clone(),
                start: range.start,
                end_exclusive: range.end_exclusive,
            })
            .collect();
        assert_eq!(live_ranges, descriptor.ranges);
        let structural_range = layout
            .ranges
            .iter()
            .find(|range| range.role == "structural_decisions")
            .expect("Go layout should expose a structural_decisions range");
        let pinned = go_pinned_alternative_node_start_tokens()
            .expect("pinned Go alternative node_start table should load");
        let expected_alternatives: BTreeMap<String, u32> = pinned
            .iter()
            .map(|(node_kind, token_id)| (node_kind.clone(), structural_range.start + *token_id))
            .collect();
        assert_eq!(
            expected_alternatives,
            descriptor.alternative_node_start_tokens
        );
        for (role, offset) in &descriptor.role_offsets {
            assert!(
                *offset < RAW_TOKEN_ALTERNATIVE_NODE_START_OFFSET,
                "fixed role offset {role}={offset} must stay below the alternative node_start offset"
            );
        }
        for (node_kind, token) in &descriptor.alternative_node_start_tokens {
            assert!(
                *token >= structural_range.start + RAW_TOKEN_ALTERNATIVE_NODE_START_OFFSET
                    && *token < structural_range.end_exclusive,
                "alternative node_start token {node_kind}={token} must sit inside the structural range at or above the alternative offset"
            );
        }
    }

    #[test]
    fn go_layout_constructors_match_token_layout_pin_fixture_current_descriptors() {
        assert_layout_matches_pin_descriptor(
            &go_syntax_token_layout(),
            &pin_fixture_current_descriptor("token_layout"),
        );
        assert_layout_matches_pin_descriptor(
            &go_syntax_source_file_token_layout(),
            &pin_fixture_current_descriptor("source_file_token_layout"),
        );
    }

    /// Absolute renumber protection for `csharp_syntax_token_layout`.
    /// Updating the fixture is a `tct-token-diff` breaking change for
    /// downstream raw-token corpora and BPE artifacts and must not be done
    /// merely to make a refactor pass. This is a token-layout pin, not a
    /// tree-sitter grammar pin; C# remains on the expanded stream contract.
    #[test]
    fn csharp_layout_constructor_matches_token_layout_pin_fixture_current_descriptor() {
        let fixture: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(CSHARP_TOKEN_LAYOUT_PIN_FIXTURE_PATH)
                .expect("checked C# token-layout pin fixture should be readable"),
        )
        .expect("checked C# token-layout pin fixture should be JSON");
        let descriptor: SyntaxTokenLayoutDescriptor =
            serde_json::from_value(fixture["current"]["token_layout"].clone())
                .expect("C# pin fixture current token_layout should deserialize");
        let layout = csharp_syntax_token_layout();
        assert_eq!(layout.vocab_size, descriptor.vocab_size);
        assert_eq!(layout.pad_token, descriptor.pad_token);
        let live_ranges: Vec<SyntaxTokenLayoutRangeDescriptor> = layout
            .ranges
            .iter()
            .map(|range| SyntaxTokenLayoutRangeDescriptor {
                id: range.id.clone(),
                role: range.role.clone(),
                start: range.start,
                end_exclusive: range.end_exclusive,
            })
            .collect();
        assert_eq!(live_ranges, descriptor.ranges);
        assert_eq!(
            descriptor.stream_contract, RAW_SYNTAX_STREAM_CONTRACT_EXPANDED_V1,
            "C# pin must record expanded stream contract"
        );
        assert!(
            descriptor.alternative_node_start_tokens.is_empty(),
            "C# product layout pin has no global alternative table"
        );
        for (role, offset) in &descriptor.role_offsets {
            assert!(
                *offset < RAW_TOKEN_ALTERNATIVE_NODE_START_OFFSET,
                "fixed role offset {role}={offset} must stay below the alternative node_start offset"
            );
        }
    }
}
