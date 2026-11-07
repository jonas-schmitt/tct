//! SQL schema-linking syntax token layout (T2 + shared payload pool).
//!
//! SQL opts out of the cross-language disjoint payload bands. Structural
//! decisions stay in `[0, 64)`; UTF-8 bytes, u8 integer digits, and menu-index
//! ordinals all share one 256-wide payload pool at `[64, 320)`. Disambiguation
//! is entirely by the admissible mask at each free decision (band-mixing
//! census: the only multi-band free mask is structural+utf8).
//!
//! This is a token-layout pin for the SQL frontend only. It does **not** edit
//! Go or C# golden absolute layout pins and is not a production route.
//! Cross-language token compatibility for payload content is deliberately
//! given up for SQL (accepted campaign trade).

use crate::grammar_ir_structural_model::{
    GrammarIntegerValueRange, GrammarStructuralTokenLayout, GrammarStructuralTokenRange,
};

/// Fixed width of the menu-index role within the shared payload pool.
/// Catalogs/scopes exceeding capacity fail closed at binding construction (T7).
/// Widening after the pin lands is a `tct-token-diff` breaking change (**P9**).
pub const SQL_MENU_BAND_WIDTH: u32 = 256;

/// Structural decision band width (fixed roles + multi-alt headroom).
pub const SQL_STRUCTURAL_BAND_WIDTH: u32 = 64;
/// Shared payload pool width (UTF-8 bytes, integer digits, menu ordinals).
pub const SQL_SHARED_PAYLOAD_POOL_WIDTH: u32 = 256;
/// UTF-8 payload role width (aliases the shared pool).
pub const SQL_UTF8_BAND_WIDTH: u32 = SQL_SHARED_PAYLOAD_POOL_WIDTH;
/// u8 integer payload role width (aliases the shared pool).
pub const SQL_U8_INTEGER_BAND_WIDTH: u32 = SQL_SHARED_PAYLOAD_POOL_WIDTH;

/// Absolute start of the structural band.
pub const SQL_STRUCTURAL_START: u32 = 0;
/// Absolute start of the shared payload pool (and of every payload role).
pub const SQL_SHARED_PAYLOAD_START: u32 = SQL_STRUCTURAL_START + SQL_STRUCTURAL_BAND_WIDTH;
/// Exclusive end of the shared payload pool / raw base vocab before PAD.
pub const SQL_SHARED_PAYLOAD_END_EXCLUSIVE: u32 =
    SQL_SHARED_PAYLOAD_START + SQL_SHARED_PAYLOAD_POOL_WIDTH;
/// Absolute start of the UTF-8 byte payload role (shared pool).
pub const SQL_UTF8_START: u32 = SQL_SHARED_PAYLOAD_START;
/// Absolute start of the u8 integer payload role (shared pool).
pub const SQL_U8_INTEGER_START: u32 = SQL_SHARED_PAYLOAD_START;
/// Absolute start of the menu-index role (shared pool).
pub const SQL_MENU_START: u32 = SQL_SHARED_PAYLOAD_START;
/// Exclusive end of every payload role / shared pool.
pub const SQL_MENU_END_EXCLUSIVE: u32 = SQL_SHARED_PAYLOAD_END_EXCLUSIVE;

/// PAD sits immediately after the shared payload pool.
pub const SQL_SYNTAX_LAYOUT_PAD_TOKEN: u32 = SQL_SHARED_PAYLOAD_END_EXCLUSIVE;
/// VOCAB_SIZE is catalog-independent (**P4**).
pub const SQL_SYNTAX_LAYOUT_VOCAB_SIZE: u32 = SQL_SYNTAX_LAYOUT_PAD_TOKEN + 1;

/// Stable layout id recorded in the pin fixture and handoff.
///
/// Bumped to v1 with the shared payload pool collapse (`breaking_change` vs
/// the pre-collapse four-band pin; rematerialize raw-token corpora / BPE).
pub const SQL_SYNTAX_TOKEN_LAYOUT_ID: &str = "sql-schema-linking-token-layout-v1";
/// Layout version for the pin / handoff envelope.
pub const SQL_SYNTAX_TOKEN_LAYOUT_VERSION: u32 = 1;
/// Pin fixture path (token-layout pin only; not a tree-sitter grammar pin).
pub const SQL_TOKEN_LAYOUT_PIN_FIXTURE_PATH: &str =
    "tests/fixtures/syntax_structural_model/sql-token-layout-pin-v0.json";
/// Role name for the menu-index payload (**P4**); absolute ids share the pool.
pub const SQL_MENU_INDEX_PAYLOAD_ROLE: &str = "menu_index_payload";
/// Role name documented for the shared payload pool (informational; live
/// encode/decode still resolve via the three concrete payload roles).
pub const SQL_SHARED_PAYLOAD_POOL_ROLE: &str = "shared_payload_pool";

/// SQL schema-linking syntax token layout (shared payload pool).
///
/// Ranges (absolute):
/// - `structural_decisions` `[0, 64)` — disjoint from the pool
/// - `utf8_byte_payload` `[64, 320)` — shared pool role
/// - `u8_integer_payload` `[64, 320)` — shared pool role (exact coincidence)
/// - `menu_index_payload` `[64, 320)` — shared pool role (exact coincidence)
/// - `PAD_TOKEN` = 320, `VOCAB_SIZE` = 321
///
/// Exact-coincident multi-role ranges are intentional: the admissible mask
/// selects the active role at each free decision. Structural stays disjoint
/// because free masks can mix structural + utf8 (string terminate vs emit).
pub fn sql_syntax_token_layout() -> GrammarStructuralTokenLayout {
    GrammarStructuralTokenLayout {
        vocab_size: SQL_SYNTAX_LAYOUT_VOCAB_SIZE,
        pad_token: SQL_SYNTAX_LAYOUT_PAD_TOKEN,
        ranges: vec![
            GrammarStructuralTokenRange {
                id: "sql.structural_tokens".to_string(),
                role: "structural_decisions".to_string(),
                start: SQL_STRUCTURAL_START,
                end_exclusive: SQL_SHARED_PAYLOAD_START,
            },
            GrammarStructuralTokenRange {
                id: "sql.utf8_byte_payload".to_string(),
                role: "utf8_byte_payload".to_string(),
                start: SQL_SHARED_PAYLOAD_START,
                end_exclusive: SQL_SHARED_PAYLOAD_END_EXCLUSIVE,
            },
            GrammarStructuralTokenRange {
                id: "sql.u8_integer_payload".to_string(),
                role: "u8_integer_payload".to_string(),
                start: SQL_SHARED_PAYLOAD_START,
                end_exclusive: SQL_SHARED_PAYLOAD_END_EXCLUSIVE,
            },
            GrammarStructuralTokenRange {
                id: "sql.menu_index_payload".to_string(),
                role: SQL_MENU_INDEX_PAYLOAD_ROLE.to_string(),
                start: SQL_SHARED_PAYLOAD_START,
                end_exclusive: SQL_SHARED_PAYLOAD_END_EXCLUSIVE,
            },
        ],
        integer_value_range: Some(GrammarIntegerValueRange {
            id: "sql.u8_integer_value".to_string(),
            valid_min: 0,
            // Value domain under variable-width escape multi-digit on the
            // 256-wide shared pool (band width unchanged; pool start moved).
            valid_max: i64::from(crate::syntax_state_machine::INTEGER_PAYLOAD_CAPACITY_MAX),
        }),
    }
}

/// Menu band range from a live layout (start, end_exclusive).
pub fn menu_band_range(layout: &GrammarStructuralTokenLayout) -> Option<(u32, u32)> {
    layout.ranges.iter().find_map(|range| {
        if range.role == SQL_MENU_INDEX_PAYLOAD_ROLE {
            Some((range.start, range.end_exclusive))
        } else {
            None
        }
    })
}

/// u8 integer band range from a live layout (start, end_exclusive).
pub fn u8_integer_band_range(layout: &GrammarStructuralTokenLayout) -> Option<(u32, u32)> {
    layout.ranges.iter().find_map(|range| {
        if range.role == "u8_integer_payload" {
            Some((range.start, range.end_exclusive))
        } else {
            None
        }
    })
}

/// UTF-8 byte payload band range from a live layout (start, end_exclusive).
pub fn utf8_band_range(layout: &GrammarStructuralTokenLayout) -> Option<(u32, u32)> {
    layout.ranges.iter().find_map(|range| {
        if range.role == "utf8_byte_payload" {
            Some((range.start, range.end_exclusive))
        } else {
            None
        }
    })
}

/// True when menu and u8 integer bands are pairwise disjoint (legacy shape).
///
/// Under the shared payload pool this is **false** (exact coincidence).
pub fn menu_band_disjoint_from_u8_integer(layout: &GrammarStructuralTokenLayout) -> bool {
    match (menu_band_range(layout), u8_integer_band_range(layout)) {
        (Some((m0, m1)), Some((i0, i1))) => m1 <= i0 || i1 <= m0,
        _ => false,
    }
}

/// True when utf8, u8 integer, and menu roles occupy the same absolute pool.
pub fn payload_roles_share_pool(layout: &GrammarStructuralTokenLayout) -> bool {
    match (
        utf8_band_range(layout),
        u8_integer_band_range(layout),
        menu_band_range(layout),
    ) {
        (Some(u), Some(i), Some(m)) => u == i && i == m,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::syntax_token_layout::{
        RAW_TOKEN_ALTERNATIVE_NODE_START_OFFSET, csharp_syntax_token_layout, go_syntax_token_layout,
    };
    use crate::syntax_token_layout_diff::{
        CSHARP_TOKEN_LAYOUT_PIN_FIXTURE_PATH, GO_TOKEN_LAYOUT_PIN_FIXTURE_PATH,
        RAW_SYNTAX_STREAM_CONTRACT_ELIDED_V1,
        RAW_SYNTAX_STREAM_CONTRACT_ELIDED_V1_FACTORED_COLUMNS,
        RAW_SYNTAX_STREAM_CONTRACT_ELIDED_V1_FACTORED_COLUMNS_VISIT_ALL_FIELDS,
        RAW_SYNTAX_STREAM_CONTRACT_ELIDED_V1_JOINON_FK_DECLARED,
        RAW_SYNTAX_STREAM_CONTRACT_ELIDED_V1_JOINON_FK_DECLARED_FACTORED_COLUMNS,
        RAW_SYNTAX_STREAM_CONTRACT_ELIDED_V1_JOINON_FK_DECLARED_FACTORED_COLUMNS_VISIT_ALL_FIELDS,
        RAW_SYNTAX_STREAM_CONTRACT_ELIDED_V1_JOINON_FK_DECLARED_VISIT_ALL_FIELDS,
        RAW_SYNTAX_STREAM_CONTRACT_ELIDED_V1_JOINON_STRUCTURAL,
        RAW_SYNTAX_STREAM_CONTRACT_ELIDED_V1_JOINON_STRUCTURAL_FACTORED_COLUMNS,
        RAW_SYNTAX_STREAM_CONTRACT_ELIDED_V1_JOINON_STRUCTURAL_FACTORED_COLUMNS_VISIT_ALL_FIELDS,
        RAW_SYNTAX_STREAM_CONTRACT_ELIDED_V1_JOINON_STRUCTURAL_VISIT_ALL_FIELDS,
        RAW_SYNTAX_STREAM_CONTRACT_ELIDED_V1_VISIT_ALL_FIELDS,
        RAW_SYNTAX_STREAM_CONTRACT_EXPANDED_V1, SyntaxLayoutCompatibility,
        SyntaxTokenLayoutDescriptor, SyntaxTokenLayoutRangeDescriptor, diff_syntax_token_layouts,
    };
    use std::collections::BTreeMap;
    use std::path::Path;

    fn live_range_descriptors(
        layout: &GrammarStructuralTokenLayout,
    ) -> Vec<SyntaxTokenLayoutRangeDescriptor> {
        layout
            .ranges
            .iter()
            .map(|range| SyntaxTokenLayoutRangeDescriptor {
                id: range.id.clone(),
                role: range.role.clone(),
                start: range.start,
                end_exclusive: range.end_exclusive,
            })
            .collect()
    }

    #[test]
    fn sql_layout_constructor_matches_token_layout_pin_fixture() {
        let fixture_path =
            Path::new(env!("CARGO_MANIFEST_DIR")).join(SQL_TOKEN_LAYOUT_PIN_FIXTURE_PATH);
        let fixture: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(&fixture_path)
                .expect("SQL token-layout pin fixture should be readable"),
        )
        .expect("SQL token-layout pin fixture should be JSON");
        assert_eq!(
            fixture["artifact_kind"].as_str(),
            Some("tct_sql_schema_linking_token_layout_pin")
        );
        let descriptor: SyntaxTokenLayoutDescriptor =
            serde_json::from_value(fixture["current"]["token_layout"].clone())
                .expect("SQL pin fixture current token_layout should deserialize");
        let layout = sql_syntax_token_layout();

        assert_eq!(layout.vocab_size, descriptor.vocab_size);
        assert_eq!(layout.pad_token, descriptor.pad_token);
        assert_eq!(live_range_descriptors(&layout), descriptor.ranges);
        assert_eq!(
            descriptor.stream_contract, RAW_SYNTAX_STREAM_CONTRACT_ELIDED_V1,
            "SQL pin records canonical elided stream contract"
        );
        assert_eq!(descriptor.layout_id, SQL_SYNTAX_TOKEN_LAYOUT_ID);
        assert_eq!(descriptor.layout_version, SQL_SYNTAX_TOKEN_LAYOUT_VERSION);
        assert!(
            descriptor.alternative_node_start_tokens.is_empty(),
            "SQL product layout pin has no global alternative table"
        );
        for (role, offset) in &descriptor.role_offsets {
            assert!(
                *offset < RAW_TOKEN_ALTERNATIVE_NODE_START_OFFSET,
                "fixed role offset {role}={offset} must stay below the alternative node_start offset"
            );
        }

        // Stream-contract flip with identical ranges is breaking_change (non-exact_reuse).
        let mut previous = descriptor.clone();
        previous.stream_contract = RAW_SYNTAX_STREAM_CONTRACT_EXPANDED_V1.to_string();
        let diff = diff_syntax_token_layouts(&previous, &descriptor)
            .expect("layout diff over stream-contract-only change");
        assert_eq!(
            diff.compatibility,
            SyntaxLayoutCompatibility::BreakingChange,
            "expanded→elided with unchanged ranges must be breaking_change"
        );
        assert_eq!(
            diff.stream_contract_change
                .as_ref()
                .map(|(f, t)| (f.as_str(), t.as_str())),
            Some((
                RAW_SYNTAX_STREAM_CONTRACT_EXPANDED_V1,
                RAW_SYNTAX_STREAM_CONTRACT_ELIDED_V1
            )),
        );
        assert!(diff.moved_ranges.is_empty());
        assert!(diff.vocab_size_change.is_none());
        assert!(diff.pad_token_change.is_none());
    }

    #[test]
    fn join_on_policy_contract_flip_with_identical_ranges_is_breaking_change() {
        // The structural join-ON policy leaves every token range untouched but
        // changes which positions are mask-forced (and therefore elided). A
        // mechanical range-only diff would misreport exact_reuse; the
        // policy-tagged stream contract makes the flip a breaking_change so
        // downstream corpora, BPE artifacts, and pins cannot silently mix
        // policies.
        let descriptor = SyntaxTokenLayoutDescriptor {
            layout_id: SQL_SYNTAX_TOKEN_LAYOUT_ID.to_string(),
            layout_version: SQL_SYNTAX_TOKEN_LAYOUT_VERSION,
            stream_contract: RAW_SYNTAX_STREAM_CONTRACT_ELIDED_V1.to_string(),
            vocab_size: SQL_SYNTAX_LAYOUT_VOCAB_SIZE,
            pad_token: SQL_SYNTAX_LAYOUT_PAD_TOKEN,
            ranges: live_range_descriptors(&sql_syntax_token_layout()),
            role_offsets: BTreeMap::new(),
            alternative_node_start_tokens: BTreeMap::new(),
        };
        let mut structural = descriptor.clone();
        structural.stream_contract =
            RAW_SYNTAX_STREAM_CONTRACT_ELIDED_V1_JOINON_STRUCTURAL.to_string();
        let diff = diff_syntax_token_layouts(&descriptor, &structural)
            .expect("layout diff over join-on policy contract flip");
        assert_eq!(
            diff.compatibility,
            SyntaxLayoutCompatibility::BreakingChange,
            "off→structural join-ON policy with unchanged ranges must be breaking_change"
        );
        assert_eq!(
            diff.stream_contract_change
                .as_ref()
                .map(|(f, t)| (f.as_str(), t.as_str())),
            Some((
                RAW_SYNTAX_STREAM_CONTRACT_ELIDED_V1,
                RAW_SYNTAX_STREAM_CONTRACT_ELIDED_V1_JOINON_STRUCTURAL
            )),
        );
        assert!(diff.moved_ranges.is_empty());
        assert!(diff.vocab_size_change.is_none());
        assert!(diff.pad_token_change.is_none());
    }

    #[test]
    fn every_sql_policy_contract_pair_is_a_breaking_change_with_identical_ranges() {
        // Factored column addressing leaves the token layout untouched but
        // changes both the value and the count of column tokens, and it
        // composes with the join-ON policy. Every distinct policy pair must be
        // a distinct tag, and any flip between them must classify as
        // breaking_change so corpora and BPE artifacts cannot silently mix.
        let base = SyntaxTokenLayoutDescriptor {
            layout_id: SQL_SYNTAX_TOKEN_LAYOUT_ID.to_string(),
            layout_version: SQL_SYNTAX_TOKEN_LAYOUT_VERSION,
            stream_contract: RAW_SYNTAX_STREAM_CONTRACT_ELIDED_V1.to_string(),
            vocab_size: SQL_SYNTAX_LAYOUT_VOCAB_SIZE,
            pad_token: SQL_SYNTAX_LAYOUT_PAD_TOKEN,
            ranges: live_range_descriptors(&sql_syntax_token_layout()),
            role_offsets: BTreeMap::new(),
            alternative_node_start_tokens: BTreeMap::new(),
        };
        let contracts = [
            RAW_SYNTAX_STREAM_CONTRACT_ELIDED_V1,
            RAW_SYNTAX_STREAM_CONTRACT_ELIDED_V1_JOINON_STRUCTURAL,
            RAW_SYNTAX_STREAM_CONTRACT_ELIDED_V1_FACTORED_COLUMNS,
            RAW_SYNTAX_STREAM_CONTRACT_ELIDED_V1_JOINON_STRUCTURAL_FACTORED_COLUMNS,
            RAW_SYNTAX_STREAM_CONTRACT_ELIDED_V1_JOINON_FK_DECLARED,
            RAW_SYNTAX_STREAM_CONTRACT_ELIDED_V1_JOINON_FK_DECLARED_FACTORED_COLUMNS,
            RAW_SYNTAX_STREAM_CONTRACT_ELIDED_V1_VISIT_ALL_FIELDS,
            RAW_SYNTAX_STREAM_CONTRACT_ELIDED_V1_JOINON_STRUCTURAL_VISIT_ALL_FIELDS,
            RAW_SYNTAX_STREAM_CONTRACT_ELIDED_V1_FACTORED_COLUMNS_VISIT_ALL_FIELDS,
            RAW_SYNTAX_STREAM_CONTRACT_ELIDED_V1_JOINON_STRUCTURAL_FACTORED_COLUMNS_VISIT_ALL_FIELDS,
            RAW_SYNTAX_STREAM_CONTRACT_ELIDED_V1_JOINON_FK_DECLARED_VISIT_ALL_FIELDS,
            RAW_SYNTAX_STREAM_CONTRACT_ELIDED_V1_JOINON_FK_DECLARED_FACTORED_COLUMNS_VISIT_ALL_FIELDS,
        ];
        let distinct: std::collections::BTreeSet<&str> = contracts.iter().copied().collect();
        assert_eq!(distinct.len(), 12, "policy contract tags must be distinct");

        for from in contracts {
            for to in contracts {
                let mut old = base.clone();
                old.stream_contract = from.to_string();
                let mut new = base.clone();
                new.stream_contract = to.to_string();
                let diff = diff_syntax_token_layouts(&old, &new).expect("layout diff");
                assert!(diff.moved_ranges.is_empty());
                if from == to {
                    assert!(diff.stream_contract_change.is_none());
                    assert_eq!(diff.compatibility, SyntaxLayoutCompatibility::ExactReuse);
                } else {
                    assert_eq!(
                        diff.compatibility,
                        SyntaxLayoutCompatibility::BreakingChange,
                        "{from} -> {to} must be breaking_change"
                    );
                }
            }
        }
    }

    #[test]
    fn sql_layout_shared_payload_pool_ranges_validate_and_coincide() {
        let layout = sql_syntax_token_layout();
        layout
            .validate()
            .expect("SQL shared-pool layout must pass GrammarStructuralTokenLayout::validate");

        assert_eq!(layout.vocab_size, SQL_SYNTAX_LAYOUT_VOCAB_SIZE);
        assert_eq!(layout.pad_token, SQL_SYNTAX_LAYOUT_PAD_TOKEN);
        assert_eq!(SQL_MENU_BAND_WIDTH, 256);
        assert_eq!(SQL_SHARED_PAYLOAD_POOL_WIDTH, 256);
        assert_eq!(layout.pad_token + 1, layout.vocab_size);
        assert_eq!(layout.vocab_size, 321);
        assert_eq!(layout.pad_token, 320);

        let (menu_start, menu_end) =
            menu_band_range(&layout).expect("menu_index_payload range required");
        let (u8_start, u8_end) =
            u8_integer_band_range(&layout).expect("u8_integer_payload range required");
        let (utf8_start, utf8_end) =
            utf8_band_range(&layout).expect("utf8_byte_payload range required");
        assert_eq!(menu_end - menu_start, SQL_MENU_BAND_WIDTH);
        assert_eq!(u8_end - u8_start, SQL_U8_INTEGER_BAND_WIDTH);
        assert_eq!(utf8_end - utf8_start, SQL_UTF8_BAND_WIDTH);
        assert!(
            payload_roles_share_pool(&layout),
            "utf8, u8 integer, and menu must occupy the same shared payload pool"
        );
        assert!(
            !menu_band_disjoint_from_u8_integer(&layout),
            "shared pool deliberately co-locates menu and integer roles"
        );
        // Concrete pin numbers for auditability.
        assert_eq!(utf8_start, 64);
        assert_eq!(utf8_end, 320);
        assert_eq!(menu_start, 64);
        assert_eq!(menu_end, 320);
        assert_eq!(u8_start, 64);
        assert_eq!(u8_end, 320);
        // Structural stays disjoint from the pool.
        assert_eq!(SQL_STRUCTURAL_START, 0);
        assert_eq!(SQL_SHARED_PAYLOAD_START, 64);
    }

    #[test]
    fn sql_shared_pool_vs_prior_four_band_layout_is_breaking_change() {
        // Previous four-band pin numbers (pre-collapse) for layout-diff class.
        let previous = SyntaxTokenLayoutDescriptor {
            layout_id: "sql-schema-linking-token-layout-v0".to_string(),
            layout_version: 0,
            stream_contract: RAW_SYNTAX_STREAM_CONTRACT_ELIDED_V1.to_string(),
            vocab_size: 833,
            pad_token: 832,
            ranges: vec![
                SyntaxTokenLayoutRangeDescriptor {
                    id: "sql.structural_tokens".into(),
                    role: "structural_decisions".into(),
                    start: 0,
                    end_exclusive: 64,
                },
                SyntaxTokenLayoutRangeDescriptor {
                    id: "sql.utf8_byte_payload".into(),
                    role: "utf8_byte_payload".into(),
                    start: 64,
                    end_exclusive: 320,
                },
                SyntaxTokenLayoutRangeDescriptor {
                    id: "sql.u8_integer_payload".into(),
                    role: "u8_integer_payload".into(),
                    start: 320,
                    end_exclusive: 576,
                },
                SyntaxTokenLayoutRangeDescriptor {
                    id: "sql.menu_index_payload".into(),
                    role: "menu_index_payload".into(),
                    start: 576,
                    end_exclusive: 832,
                },
            ],
            role_offsets: BTreeMap::new(),
            alternative_node_start_tokens: BTreeMap::new(),
        };
        let current = SyntaxTokenLayoutDescriptor {
            layout_id: SQL_SYNTAX_TOKEN_LAYOUT_ID.to_string(),
            layout_version: SQL_SYNTAX_TOKEN_LAYOUT_VERSION,
            stream_contract: RAW_SYNTAX_STREAM_CONTRACT_ELIDED_V1.to_string(),
            vocab_size: SQL_SYNTAX_LAYOUT_VOCAB_SIZE,
            pad_token: SQL_SYNTAX_LAYOUT_PAD_TOKEN,
            ranges: live_range_descriptors(&sql_syntax_token_layout()),
            role_offsets: BTreeMap::new(),
            alternative_node_start_tokens: BTreeMap::new(),
        };
        let diff = diff_syntax_token_layouts(&previous, &current)
            .expect("layout diff four-band → shared pool");
        assert_eq!(
            diff.compatibility,
            SyntaxLayoutCompatibility::BreakingChange,
            "shared payload pool collapse must be breaking_change (rematerialize corpora/BPE)"
        );
        assert!(diff.vocab_size_change.is_some());
        assert!(diff.pad_token_change.is_some());
        assert!(
            !diff.moved_ranges.is_empty()
                || !diff.added_ranges.is_empty()
                || !diff.removed_ranges.is_empty()
                || diff.vocab_size_change.is_some(),
            "diff must record the range/vocab movement"
        );
    }

    #[test]
    fn sql_layout_tranche_does_not_change_go_or_csharp_live_constructors() {
        // Prove SQL shared-pool work does not edit default Go/C# absolute layout
        // constructors. Full pin equality is enforced by existing
        // syntax_token_layout tests and syntax_backend_boundary_test.
        let go = go_syntax_token_layout();
        let csharp = csharp_syntax_token_layout();
        assert_eq!(go.vocab_size, 641);
        assert_eq!(go.pad_token, 640);
        assert_eq!(csharp.vocab_size, 577);
        assert_eq!(csharp.pad_token, 576);
        assert!(
            go.ranges
                .iter()
                .all(|r| r.role != SQL_MENU_INDEX_PAYLOAD_ROLE)
        );
        assert!(
            csharp
                .ranges
                .iter()
                .all(|r| r.role != SQL_MENU_INDEX_PAYLOAD_ROLE)
        );

        // Pin fixture files for Go/C# must still be present and parseable, and
        // must remain byte-identical to the pre-SQL-shared-pool pins (checked
        // by not editing them in this campaign).
        for path in [
            GO_TOKEN_LAYOUT_PIN_FIXTURE_PATH,
            CSHARP_TOKEN_LAYOUT_PIN_FIXTURE_PATH,
        ] {
            let full = Path::new(env!("CARGO_MANIFEST_DIR")).join(path);
            let fixture: serde_json::Value =
                serde_json::from_str(&std::fs::read_to_string(&full).unwrap_or_else(|e| {
                    panic!("Go/C# pin fixture {path} must remain readable: {e}")
                }))
                .unwrap_or_else(|e| panic!("Go/C# pin fixture {path} must remain JSON: {e}"));
            assert!(
                fixture["current"]["token_layout"].is_object(),
                "{path} current.token_layout missing"
            );
        }
    }

    #[test]
    fn sql_menu_band_width_is_pinned_once() {
        // Contract P9: fix SQL_MENU_BAND_WIDTH once; do not widen casually.
        assert_eq!(SQL_MENU_BAND_WIDTH, 256);
        let layout = sql_syntax_token_layout();
        let (start, end) = menu_band_range(&layout).unwrap();
        assert_eq!(end - start, SQL_MENU_BAND_WIDTH);
        // Role offsets map shape matches Go/C# fixed ten.
        let mut expected_offsets = BTreeMap::new();
        expected_offsets.insert("node_start".to_string(), 0u32);
        expected_offsets.insert("node_end".to_string(), 1);
        expected_offsets.insert("field_start".to_string(), 2);
        expected_offsets.insert("field_end".to_string(), 3);
        expected_offsets.insert("field_missing".to_string(), 4);
        expected_offsets.insert("repeated_empty".to_string(), 5);
        expected_offsets.insert("terminal".to_string(), 6);
        expected_offsets.insert("payload_end".to_string(), 7);
        expected_offsets.insert("boolean_false".to_string(), 8);
        expected_offsets.insert("boolean_true".to_string(), 9);
        let fixture_path =
            Path::new(env!("CARGO_MANIFEST_DIR")).join(SQL_TOKEN_LAYOUT_PIN_FIXTURE_PATH);
        let fixture: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(fixture_path).unwrap()).unwrap();
        let descriptor: SyntaxTokenLayoutDescriptor =
            serde_json::from_value(fixture["current"]["token_layout"].clone()).unwrap();
        assert_eq!(descriptor.role_offsets, expected_offsets);
    }
}
