//! Syntax token-layout diff: the syntax-side analog of `tct-token-diff`.
//!
//! Compares two syntax token-layout descriptors (ranges, fixed role offsets,
//! the pinned alternative node_start token table, pad, and vocab size) and
//! classifies the change with the same conservative compatibility classes the
//! JSON `tct-token-diff` report uses. Layout-affecting syntax changes must go
//! through this diff so renumbering can never land silently; the checked pin
//! fixtures (Go and C#) and their boundary gates are the forcing function.
//!
//! Go pins live under [`GO_TOKEN_LAYOUT_PIN_FIXTURE_PATH`] (elided stream
//! contract). C# pins live under [`CSHARP_TOKEN_LAYOUT_PIN_FIXTURE_PATH`]
//! (expanded stream contract). Both share this module's descriptor types and
//! [`diff_syntax_token_layouts`]; the C# pin is a token-layout lock only, not
//! a tree-sitter grammar pin (the project adds no new tree-sitter
//! language/grammar pins, but it does lock the C# structural layout).

use crate::syntax_backend_boundary::SyntaxBackendBoundaryError;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Checked pin fixture recording the current Go syntax token layouts, the
/// superseded previous layouts, and the classified transition between them.
/// `tests/syntax_backend_boundary_test.rs` enforces it against the live
/// layouts and regenerates it via the ignored
/// `regenerate_go_token_layout_pin_fixture` test.
pub const GO_TOKEN_LAYOUT_PIN_FIXTURE_PATH: &str =
    "tests/fixtures/syntax_structural_model/go-token-layout-pin-v1.json";

/// Checked pin fixture recording the current C# product syntax token layout
/// (`csharp_syntax_token_layout`). Absolute renumber protection for the
/// syntax-level cross-target check; enforced by
/// `csharp_token_layout_pin_fixture_matches_live_layout` and regenerated via
/// the ignored `regenerate_csharp_token_layout_pin_fixture` test.
///
/// This is a **token-layout pin**, not a tree-sitter grammar pin. The project
/// adds no new tree-sitter language/grammar pins; it does lock the C#
/// structural token layout against silent renumbering.
/// C# stays on the expanded stream contract (`raw-syntax-stream-expanded-v1`);
/// Go canonical streams use elided-v1 — the two pins must not be confused.
pub const CSHARP_TOKEN_LAYOUT_PIN_FIXTURE_PATH: &str =
    "tests/fixtures/syntax_structural_model/csharp-token-layout-pin-v1.json";

/// Canonical raw-syntax stream contract for fully expanded streams: every
/// structural decision, including positions whose admissible mask is a
/// singleton, is present in the encoded stream. Every descriptor serialized
/// before the `stream_contract` field existed carried this contract
/// implicitly, so it is also the serde default.
pub const RAW_SYNTAX_STREAM_CONTRACT_EXPANDED_V1: &str = "raw-syntax-stream-expanded-v1";

/// Canonical raw-syntax stream contract for elided streams: positions whose
/// admissible mask is a singleton are omitted from the encoded stream and
/// re-derived from the state machine on decode
/// (`CompiledSyntaxModel::elide_forced_raw_tokens` /
/// `expand_elided_raw_tokens`).
pub const RAW_SYNTAX_STREAM_CONTRACT_ELIDED_V1: &str = "raw-syntax-stream-elided-v1";

/// Elided stream contract with the SQL structural join-ON policy active.
///
/// Token ranges are identical to [`RAW_SYNTAX_STREAM_CONTRACT_ELIDED_V1`]
/// layouts, but join-key column admissibility narrows, so mask-forced
/// positions (and therefore canonical elided streams) differ. A policy flip
/// with unchanged ranges must classify as `breaking_change` — never
/// `exact_reuse` — for downstream corpora, BPE artifacts, and pins.
pub const RAW_SYNTAX_STREAM_CONTRACT_ELIDED_V1_JOINON_STRUCTURAL: &str =
    "raw-syntax-stream-elided-v1+joinon-structural";

/// Elided stream contract with SQL factored column addressing active.
///
/// A column reference is emitted as two positional menu digits (catalog table,
/// then column inside that table) instead of one radix code over the flat
/// scope-relative concatenation. Token ranges are unchanged, but both the
/// *values* and the *token count* at column sites change, so streams and BPE
/// artifacts from the two addressing policies are never interchangeable.
pub const RAW_SYNTAX_STREAM_CONTRACT_ELIDED_V1_FACTORED_COLUMNS: &str =
    "raw-syntax-stream-elided-v1+factored-columns";

/// Elided stream contract with **both** SQL policies active.
///
/// SQL policies compose, so the contract tag must express the combination
/// rather than either half: a corpus encoded under structural join-ON masking
/// plus factored column addressing must not be mistaken for one encoded under
/// either policy alone. Tag order follows the order the policies landed in.
pub const RAW_SYNTAX_STREAM_CONTRACT_ELIDED_V1_JOINON_STRUCTURAL_FACTORED_COLUMNS: &str =
    "raw-syntax-stream-elided-v1+joinon-structural+factored-columns";

/// Elided stream contract with unique-declared-FK join-ON shrink (flat columns).
///
/// Token ranges match the structural join-ON layout; the right-hand ON menu
/// may collapse from a table slice to a single FK column, so mask-forced
/// positions and canonical elided streams differ. Classify as
/// `breaking_change`, never `exact_reuse`.
pub const RAW_SYNTAX_STREAM_CONTRACT_ELIDED_V1_JOINON_FK_DECLARED: &str =
    "raw-syntax-stream-elided-v1+joinon-fk-declared";

/// Elided stream contract with unique-declared-FK join-ON shrink plus factored
/// column addressing.
pub const RAW_SYNTAX_STREAM_CONTRACT_ELIDED_V1_JOINON_FK_DECLARED_FACTORED_COLUMNS: &str =
    "raw-syntax-stream-elided-v1+joinon-fk-declared+factored-columns";

/// Elided stream contract with SQL visit-all remaining fields (rank-1 merge).
///
/// Token ranges are unchanged. `node_end` is no longer admissible while any
/// field remains, so the historical skip-vs-visit free decision (`rule._`)
/// becomes mask-forced and is elided. Trailing optional fields keep their
/// grammar alternatives (`field_missing` vs content). Classify as
/// `breaking_change`, never `exact_reuse`.
pub const RAW_SYNTAX_STREAM_CONTRACT_ELIDED_V1_VISIT_ALL_FIELDS: &str =
    "raw-syntax-stream-elided-v1+visit-all-fields";

/// Visit-all remaining fields plus structural join-ON.
pub const RAW_SYNTAX_STREAM_CONTRACT_ELIDED_V1_JOINON_STRUCTURAL_VISIT_ALL_FIELDS: &str =
    "raw-syntax-stream-elided-v1+joinon-structural+visit-all-fields";

/// Visit-all remaining fields plus factored column addressing.
pub const RAW_SYNTAX_STREAM_CONTRACT_ELIDED_V1_FACTORED_COLUMNS_VISIT_ALL_FIELDS: &str =
    "raw-syntax-stream-elided-v1+factored-columns+visit-all-fields";

/// Visit-all remaining fields plus structural join-ON and factored columns.
///
/// This is the locked L6-v1 successor contract for the rank-1 merge cell.
pub const RAW_SYNTAX_STREAM_CONTRACT_ELIDED_V1_JOINON_STRUCTURAL_FACTORED_COLUMNS_VISIT_ALL_FIELDS: &str =
    "raw-syntax-stream-elided-v1+joinon-structural+factored-columns+visit-all-fields";

/// Visit-all remaining fields plus unique-declared-FK join-ON (flat columns).
pub const RAW_SYNTAX_STREAM_CONTRACT_ELIDED_V1_JOINON_FK_DECLARED_VISIT_ALL_FIELDS: &str =
    "raw-syntax-stream-elided-v1+joinon-fk-declared+visit-all-fields";

/// Visit-all remaining fields plus unique-declared-FK join-ON and factored columns.
pub const RAW_SYNTAX_STREAM_CONTRACT_ELIDED_V1_JOINON_FK_DECLARED_FACTORED_COLUMNS_VISIT_ALL_FIELDS: &str =
    "raw-syntax-stream-elided-v1+joinon-fk-declared+factored-columns+visit-all-fields";

/// The live canonical stream contract stamped on freshly built layout
/// descriptors and the unified Go grammar artifact identity. The deliberate
/// elision break flipped this to [`RAW_SYNTAX_STREAM_CONTRACT_ELIDED_V1`]:
/// canonical encoded Go streams contain no mask-forced tokens, the
/// layout-diff gate classifies the expanded -> elided transition as a
/// `breaking_change` with zero moved ranges, and the training-corpus handoff
/// records the retrain-required stream-contract transition for downstream
/// raw-token corpora and BPE artifacts.
pub const LIVE_RAW_SYNTAX_STREAM_CONTRACT: &str = RAW_SYNTAX_STREAM_CONTRACT_ELIDED_V1;

fn expanded_stream_contract() -> String {
    RAW_SYNTAX_STREAM_CONTRACT_EXPANDED_V1.to_string()
}

/// One contiguous token range of a syntax layout.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxTokenLayoutRangeDescriptor {
    pub id: String,
    pub role: String,
    pub start: u32,
    pub end_exclusive: u32,
}

/// A complete, comparable description of one syntax token layout.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxTokenLayoutDescriptor {
    pub layout_id: String,
    pub layout_version: u32,
    /// Canonical raw-syntax stream contract the layout's encoded streams
    /// follow. Defaults to [`RAW_SYNTAX_STREAM_CONTRACT_EXPANDED_V1`] so
    /// descriptors serialized before the field existed parse as
    /// expanded-era evidence.
    #[serde(default = "expanded_stream_contract")]
    pub stream_contract: String,
    pub vocab_size: u32,
    pub pad_token: u32,
    pub ranges: Vec<SyntaxTokenLayoutRangeDescriptor>,
    /// The ten fixed structural role offsets, name -> offset within the
    /// structural range.
    pub role_offsets: BTreeMap<String, u32>,
    /// Pinned alternative node_start assignment, node kind -> absolute token
    /// id.
    pub alternative_node_start_tokens: BTreeMap<String, u32>,
}

/// Conservative compatibility classes mirroring the JSON
/// `StableVocabularyDecision` contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SyntaxLayoutCompatibility {
    /// Descriptors are identical.
    ExactReuse,
    /// Ranges, role offsets, pad, vocab, and every existing alternative token
    /// are identical; only identifying metadata changed.
    LayoutOnlyUnchanged,
    /// Only new alternative tokens were appended inside unchanged ranges;
    /// every pre-existing assignment is identical.
    AdditiveCompatible,
    /// Any range, role offset, pad, vocab, or existing alternative token
    /// moved or disappeared.
    BreakingChange,
}

impl SyntaxLayoutCompatibility {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ExactReuse => "exact_reuse",
            Self::LayoutOnlyUnchanged => "layout_only_unchanged",
            Self::AdditiveCompatible => "additive_compatible",
            Self::BreakingChange => "breaking_change",
        }
    }
}

/// Diff between two syntax token layouts with the classified compatibility.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SyntaxTokenLayoutDiff {
    pub compatibility: SyntaxLayoutCompatibility,
    pub old_layout_id: String,
    pub new_layout_id: String,
    pub vocab_size_change: Option<(u32, u32)>,
    pub pad_token_change: Option<(u32, u32)>,
    /// `(old, new)` when the canonical stream contract changed; a contract
    /// change is breaking even when every token range is identical.
    pub stream_contract_change: Option<(String, String)>,
    pub moved_ranges: Vec<String>,
    pub added_ranges: Vec<String>,
    pub removed_ranges: Vec<String>,
    pub changed_role_offsets: Vec<String>,
    pub renumbered_alternatives: Vec<String>,
    pub removed_alternatives: Vec<String>,
    pub added_alternatives: Vec<String>,
    pub reasons: Vec<String>,
    pub evidence_scope: &'static str,
    pub downstream_claims_allowed: bool,
    pub notes: Vec<&'static str>,
}

/// Compare two layout descriptors and classify the change conservatively.
pub fn diff_syntax_token_layouts(
    old: &SyntaxTokenLayoutDescriptor,
    new: &SyntaxTokenLayoutDescriptor,
) -> Result<SyntaxTokenLayoutDiff, SyntaxBackendBoundaryError> {
    for descriptor in [old, new] {
        if descriptor.ranges.is_empty() {
            return Err(SyntaxBackendBoundaryError::new(
                "syntax_token_layout_diff.ranges",
                format!("layout {} has no ranges", descriptor.layout_id),
            ));
        }
    }

    let mut reasons = Vec::new();
    let mut moved_ranges = Vec::new();
    let mut added_ranges = Vec::new();
    let mut removed_ranges = Vec::new();

    let old_ranges: BTreeMap<&str, &SyntaxTokenLayoutRangeDescriptor> = old
        .ranges
        .iter()
        .map(|range| (range.role.as_str(), range))
        .collect();
    let new_ranges: BTreeMap<&str, &SyntaxTokenLayoutRangeDescriptor> = new
        .ranges
        .iter()
        .map(|range| (range.role.as_str(), range))
        .collect();
    for (role, old_range) in &old_ranges {
        match new_ranges.get(role) {
            None => {
                removed_ranges.push((*role).to_string());
                reasons.push(format!("range {role} removed"));
            }
            Some(new_range) => {
                if old_range.start != new_range.start
                    || old_range.end_exclusive != new_range.end_exclusive
                {
                    moved_ranges.push((*role).to_string());
                    reasons.push(format!(
                        "range {role} moved from {}..{} to {}..{}",
                        old_range.start,
                        old_range.end_exclusive,
                        new_range.start,
                        new_range.end_exclusive
                    ));
                }
            }
        }
    }
    for role in new_ranges.keys() {
        if !old_ranges.contains_key(role) {
            added_ranges.push((*role).to_string());
            reasons.push(format!("range {role} added"));
        }
    }

    let mut changed_role_offsets = Vec::new();
    for (name, old_offset) in &old.role_offsets {
        match new.role_offsets.get(name) {
            Some(new_offset) if new_offset == old_offset => {}
            Some(new_offset) => {
                changed_role_offsets.push(name.clone());
                reasons.push(format!(
                    "role offset {name} changed from {old_offset} to {new_offset}"
                ));
            }
            None => {
                changed_role_offsets.push(name.clone());
                reasons.push(format!("role offset {name} removed"));
            }
        }
    }
    for name in new.role_offsets.keys() {
        if !old.role_offsets.contains_key(name) {
            changed_role_offsets.push(name.clone());
            reasons.push(format!("role offset {name} added"));
        }
    }

    let mut renumbered_alternatives = Vec::new();
    let mut removed_alternatives = Vec::new();
    let mut added_alternatives = Vec::new();
    for (kind, old_token) in &old.alternative_node_start_tokens {
        match new.alternative_node_start_tokens.get(kind) {
            Some(new_token) if new_token == old_token => {}
            Some(new_token) => {
                renumbered_alternatives.push(kind.clone());
                reasons.push(format!(
                    "alternative {kind} renumbered from {old_token} to {new_token}"
                ));
            }
            None => {
                removed_alternatives.push(kind.clone());
                reasons.push(format!("alternative {kind} removed"));
            }
        }
    }
    for kind in new.alternative_node_start_tokens.keys() {
        if !old.alternative_node_start_tokens.contains_key(kind) {
            added_alternatives.push(kind.clone());
        }
    }

    let vocab_size_change =
        (old.vocab_size != new.vocab_size).then_some((old.vocab_size, new.vocab_size));
    if let Some((from, to)) = vocab_size_change {
        reasons.push(format!("vocab size changed from {from} to {to}"));
    }
    let pad_token_change =
        (old.pad_token != new.pad_token).then_some((old.pad_token, new.pad_token));
    if let Some((from, to)) = pad_token_change {
        reasons.push(format!("pad token changed from {from} to {to}"));
    }
    let stream_contract_change = (old.stream_contract != new.stream_contract)
        .then(|| (old.stream_contract.clone(), new.stream_contract.clone()));
    if let Some((from, to)) = &stream_contract_change {
        reasons.push(format!("stream contract changed from {from} to {to}"));
    }

    let breaking = !moved_ranges.is_empty()
        || !removed_ranges.is_empty()
        || !changed_role_offsets.is_empty()
        || !renumbered_alternatives.is_empty()
        || !removed_alternatives.is_empty()
        || vocab_size_change.is_some()
        || pad_token_change.is_some()
        || stream_contract_change.is_some()
        || !added_ranges.is_empty();
    let compatibility = if breaking {
        SyntaxLayoutCompatibility::BreakingChange
    } else if !added_alternatives.is_empty() {
        SyntaxLayoutCompatibility::AdditiveCompatible
    } else if old == new {
        SyntaxLayoutCompatibility::ExactReuse
    } else {
        SyntaxLayoutCompatibility::LayoutOnlyUnchanged
    };

    Ok(SyntaxTokenLayoutDiff {
        compatibility,
        old_layout_id: old.layout_id.clone(),
        new_layout_id: new.layout_id.clone(),
        vocab_size_change,
        pad_token_change,
        stream_contract_change,
        moved_ranges,
        added_ranges,
        removed_ranges,
        changed_role_offsets,
        renumbered_alternatives,
        removed_alternatives,
        added_alternatives,
        reasons,
        evidence_scope: "compiler_artifact_evidence_only",
        downstream_claims_allowed: false,
        notes: vec![
            "Syntax layout diffs are compiler-local compatibility evidence, not downstream model evidence.",
            "additive_compatible covers only appended alternative tokens inside unchanged ranges.",
            "breaking_change requires retraining or explicit migration of downstream raw-token corpora, BPE artifacts, and embeddings.",
            "A stream_contract change is a breaking_change even when every token range is identical: encoded streams are not interchangeable across stream contracts.",
        ],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_descriptor() -> SyntaxTokenLayoutDescriptor {
        SyntaxTokenLayoutDescriptor {
            layout_id: "go-syntax-source-file-v1".to_string(),
            layout_version: 1,
            stream_contract: RAW_SYNTAX_STREAM_CONTRACT_EXPANDED_V1.to_string(),
            vocab_size: 577,
            pad_token: 576,
            ranges: vec![
                SyntaxTokenLayoutRangeDescriptor {
                    id: "structural".to_string(),
                    role: "structural_decisions".to_string(),
                    start: 0,
                    end_exclusive: 64,
                },
                SyntaxTokenLayoutRangeDescriptor {
                    id: "utf8".to_string(),
                    role: "utf8_byte_payload".to_string(),
                    start: 64,
                    end_exclusive: 320,
                },
                SyntaxTokenLayoutRangeDescriptor {
                    id: "integer".to_string(),
                    role: "integer_lexeme_payload".to_string(),
                    start: 320,
                    end_exclusive: 576,
                },
            ],
            role_offsets: BTreeMap::from([
                ("node_start".to_string(), 0),
                ("node_end".to_string(), 1),
            ]),
            alternative_node_start_tokens: BTreeMap::from([
                ("AssignStmt".to_string(), 11),
                ("CallExpr".to_string(), 16),
            ]),
        }
    }

    #[test]
    fn identical_descriptors_are_exact_reuse() {
        let old = base_descriptor();
        let diff = diff_syntax_token_layouts(&old, &old.clone()).unwrap();
        assert_eq!(diff.compatibility, SyntaxLayoutCompatibility::ExactReuse);
        assert!(diff.reasons.is_empty());
    }

    #[test]
    fn metadata_only_change_is_layout_only_unchanged() {
        let old = base_descriptor();
        let mut new = old.clone();
        new.layout_version = 2;
        let diff = diff_syntax_token_layouts(&old, &new).unwrap();
        assert_eq!(
            diff.compatibility,
            SyntaxLayoutCompatibility::LayoutOnlyUnchanged
        );
    }

    #[test]
    fn appended_alternative_is_additive_compatible() {
        let old = base_descriptor();
        let mut new = old.clone();
        new.alternative_node_start_tokens
            .insert("DeferStmt".to_string(), 22);
        let diff = diff_syntax_token_layouts(&old, &new).unwrap();
        assert_eq!(
            diff.compatibility,
            SyntaxLayoutCompatibility::AdditiveCompatible
        );
        assert_eq!(diff.added_alternatives, vec!["DeferStmt".to_string()]);
    }

    #[test]
    fn renumbered_alternative_is_breaking() {
        let old = base_descriptor();
        let mut new = old.clone();
        new.alternative_node_start_tokens
            .insert("CallExpr".to_string(), 17);
        let diff = diff_syntax_token_layouts(&old, &new).unwrap();
        assert_eq!(
            diff.compatibility,
            SyntaxLayoutCompatibility::BreakingChange
        );
        assert_eq!(diff.renumbered_alternatives, vec!["CallExpr".to_string()]);
    }

    #[test]
    fn the_planned_widening_is_breaking_with_stable_structural_ids() {
        let old = base_descriptor();
        let mut new = old.clone();
        new.layout_version = 2;
        new.vocab_size = 641;
        new.pad_token = 640;
        new.ranges[0].end_exclusive = 128;
        new.ranges[1].start = 128;
        new.ranges[1].end_exclusive = 384;
        new.ranges[2].start = 384;
        new.ranges[2].end_exclusive = 640;
        let diff = diff_syntax_token_layouts(&old, &new).unwrap();
        assert_eq!(
            diff.compatibility,
            SyntaxLayoutCompatibility::BreakingChange
        );
        // Structural alternative IDs survive the widening; only payload
        // ranges, pad, and vocab move.
        assert!(diff.renumbered_alternatives.is_empty());
        assert_eq!(
            diff.moved_ranges,
            vec![
                "integer_lexeme_payload".to_string(),
                "structural_decisions".to_string(),
                "utf8_byte_payload".to_string()
            ]
        );
        assert!(diff.vocab_size_change.is_some());
        assert!(diff.pad_token_change.is_some());
        assert!(!diff.downstream_claims_allowed);
    }

    #[test]
    fn removed_range_is_breaking() {
        let old = base_descriptor();
        let mut new = old.clone();
        new.ranges.pop();
        let diff = diff_syntax_token_layouts(&old, &new).unwrap();
        assert_eq!(
            diff.compatibility,
            SyntaxLayoutCompatibility::BreakingChange
        );
        assert_eq!(
            diff.removed_ranges,
            vec!["integer_lexeme_payload".to_string()]
        );
    }

    #[test]
    fn empty_ranges_are_rejected() {
        let mut old = base_descriptor();
        old.ranges.clear();
        let error = diff_syntax_token_layouts(&old, &base_descriptor()).unwrap_err();
        assert_eq!(error.field(), "syntax_token_layout_diff.ranges");
    }

    /// Post-elision-break guard: the live canonical stream contract is the
    /// elided contract. Reverting to expanded-v1 (or minting a new contract)
    /// is itself a breaking stream-contract change and must regenerate the
    /// pin fixtures and the checked corpus again.
    #[test]
    fn live_stream_contract_is_elided_v1() {
        assert_eq!(
            LIVE_RAW_SYNTAX_STREAM_CONTRACT,
            RAW_SYNTAX_STREAM_CONTRACT_ELIDED_V1
        );
        assert_ne!(
            RAW_SYNTAX_STREAM_CONTRACT_EXPANDED_V1,
            RAW_SYNTAX_STREAM_CONTRACT_ELIDED_V1
        );
    }

    #[test]
    fn descriptor_without_stream_contract_field_parses_as_expanded() {
        let mut value = serde_json::to_value(base_descriptor()).unwrap();
        let removed = value
            .as_object_mut()
            .unwrap()
            .remove("stream_contract")
            .expect("serialized descriptor should carry stream_contract");
        assert_eq!(removed, RAW_SYNTAX_STREAM_CONTRACT_EXPANDED_V1);
        let parsed: SyntaxTokenLayoutDescriptor = serde_json::from_value(value).unwrap();
        assert_eq!(
            parsed.stream_contract,
            RAW_SYNTAX_STREAM_CONTRACT_EXPANDED_V1
        );
        assert_eq!(parsed, base_descriptor());
    }

    #[test]
    fn explicit_stream_contract_round_trips_through_serde() {
        let mut descriptor = base_descriptor();
        descriptor.stream_contract = RAW_SYNTAX_STREAM_CONTRACT_ELIDED_V1.to_string();
        let value = serde_json::to_value(&descriptor).unwrap();
        assert_eq!(
            value["stream_contract"],
            RAW_SYNTAX_STREAM_CONTRACT_ELIDED_V1
        );
        let parsed: SyntaxTokenLayoutDescriptor = serde_json::from_value(value).unwrap();
        assert_eq!(parsed, descriptor);
    }

    #[test]
    fn stream_contract_change_with_identical_ranges_is_breaking() {
        let old = base_descriptor();
        let mut new = old.clone();
        new.stream_contract = RAW_SYNTAX_STREAM_CONTRACT_ELIDED_V1.to_string();
        let diff = diff_syntax_token_layouts(&old, &new).unwrap();
        assert_eq!(
            diff.compatibility,
            SyntaxLayoutCompatibility::BreakingChange
        );
        assert_eq!(
            diff.stream_contract_change,
            Some((
                RAW_SYNTAX_STREAM_CONTRACT_EXPANDED_V1.to_string(),
                RAW_SYNTAX_STREAM_CONTRACT_ELIDED_V1.to_string()
            ))
        );
        assert!(diff.moved_ranges.is_empty());
        assert!(diff.added_ranges.is_empty());
        assert!(diff.removed_ranges.is_empty());
        assert!(diff.changed_role_offsets.is_empty());
        assert!(diff.renumbered_alternatives.is_empty());
        assert!(diff.removed_alternatives.is_empty());
        assert!(diff.vocab_size_change.is_none());
        assert!(diff.pad_token_change.is_none());
        assert_eq!(
            diff.reasons,
            vec![format!(
                "stream contract changed from {RAW_SYNTAX_STREAM_CONTRACT_EXPANDED_V1} to {RAW_SYNTAX_STREAM_CONTRACT_ELIDED_V1}"
            )]
        );
        assert!(!diff.downstream_claims_allowed);
    }

    #[test]
    fn identical_elided_descriptors_are_exact_reuse() {
        let mut old = base_descriptor();
        old.stream_contract = RAW_SYNTAX_STREAM_CONTRACT_ELIDED_V1.to_string();
        let diff = diff_syntax_token_layouts(&old, &old.clone()).unwrap();
        assert_eq!(diff.compatibility, SyntaxLayoutCompatibility::ExactReuse);
        assert!(diff.stream_contract_change.is_none());
        assert!(diff.reasons.is_empty());
    }
}
