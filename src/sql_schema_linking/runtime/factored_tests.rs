//! Factored column-addressing policy tests (L4).
//!
//! Under [`SqlColumnAddressing::Factored`] a column reference is emitted as two
//! positional menu digits — catalog table index, then column index inside that
//! table — instead of one radix code over the flat scope-relative
//! concatenation. These tests pin the properties the policy exists for:
//! addressing stability across FROM order, a per-table digit-2 mask, scope
//! exclusion at digit 1, lossless round trips, mask==accept parity, and
//! bit-identical `Flat` output.
//!
//! Kept in a sibling module so the oversized `runtime/tests.rs` does not grow.

use super::*;
use crate::sql_schema_linking::catalog::{
    SQL_SCHEMA_LINKING_FIXTURE_DB_ID, SqlCatalog, fixture_dir,
};
use crate::sql_schema_linking::menu_binding::SqlColumnAddressing;
use crate::syntax_token_layout_diff::{
    RAW_SYNTAX_STREAM_CONTRACT_ELIDED_V1, RAW_SYNTAX_STREAM_CONTRACT_ELIDED_V1_FACTORED_COLUMNS,
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
};
use std::collections::BTreeSet;

/// concert_singer-shaped catalog: the exact schema the L4 measurement used.
///
/// Catalog order (and therefore table menu digits): `stadium`=0 (7 columns),
/// `singer`=1 (7), `concert`=2 (5), `singer_in_concert`=3 (2). `singer.Name` is
/// column 1 of table 1, so the factored pair is `(1, 1)` in every scope, while
/// the flat scope-relative ordinal is 1 / 8 / 6 depending on FROM order.
fn concert_singer_catalog() -> SqlCatalog {
    let tables_json = br#"[{
        "db_id": "concert_singer",
        "table_names_original": ["stadium", "singer", "concert", "singer_in_concert"],
        "table_names": ["stadium", "singer", "concert", "singer in concert"],
        "column_names_original": [
            [-1, "*"],
            [0, "Stadium_ID"], [0, "Location"], [0, "Name"], [0, "Capacity"],
            [0, "Highest"], [0, "Lowest"], [0, "Average"],
            [1, "Singer_ID"], [1, "Name"], [1, "Country"], [1, "Song_Name"],
            [1, "Song_release_year"], [1, "Age"], [1, "Is_male"],
            [2, "concert_ID"], [2, "concert_Name"], [2, "Theme"], [2, "Stadium_ID"],
            [2, "Year"],
            [3, "concert_ID"], [3, "Singer_ID"]
        ],
        "column_names": [
            [-1, "*"],
            [0, "stadium id"], [0, "location"], [0, "name"], [0, "capacity"],
            [0, "highest"], [0, "lowest"], [0, "average"],
            [1, "singer id"], [1, "name"], [1, "country"], [1, "song name"],
            [1, "song release year"], [1, "age"], [1, "is male"],
            [2, "concert id"], [2, "concert name"], [2, "theme"], [2, "stadium id"],
            [2, "year"],
            [3, "concert id"], [3, "singer id"]
        ],
        "column_types": [
            "text",
            "number", "text", "text", "number", "number", "number", "number",
            "number", "text", "text", "text", "text", "number", "text",
            "number", "text", "text", "number", "text",
            "number", "number"
        ],
        "primary_keys": [1, 8, 15],
        "foreign_keys": [[18, 1], [20, 15], [21, 8]]
    }]"#;
    SqlCatalog::parse_tables_json_unpinned(tables_json, "concert_singer")
        .expect("concert_singer catalog parses")
}

fn factored_runtime(catalog: SqlCatalog) -> SqlSchemaLinkingRuntime {
    SqlSchemaLinkingRuntime::from_catalog_with_policies(
        catalog,
        SqlJoinOnPolicy::Off,
        SqlColumnAddressing::Factored,
    )
    .expect("factored runtime")
}

fn flat_runtime(catalog: SqlCatalog) -> SqlSchemaLinkingRuntime {
    SqlSchemaLinkingRuntime::from_catalog(catalog).expect("flat runtime")
}

fn menu_band(rt: &SqlSchemaLinkingRuntime) -> (u32, u32) {
    let range = rt
        .codec()
        .model()
        .token_roles()
        .menu_range
        .as_ref()
        .expect("SQL layout declares a menu band");
    (range.start, range.end_exclusive)
}

/// Number of column menu sites in a query, from the authoritative menu trace.
///
/// Token-band counting cannot substitute for this: the SQL layout gives
/// `menu_index_payload` and `u8_integer_payload` the *same* range [64, 320)
/// (shared payload pool), so integer literals are indistinguishable from menu
/// digits by token id alone.
fn column_site_count(rt: &SqlSchemaLinkingRuntime, sql: &str) -> usize {
    let (_, trace) = rt.encode_with_menu_trace(sql).expect("encode with trace");
    trace.iter().filter(|site| site.kind == "column").count()
}

/// Menu-band digits of a stream, in order (the ordinals the model must produce).
///
/// NOTE: the menu band coincides with the u8 integer band in the SQL layout, so
/// this also picks up integer literals. Callers must only use it on queries
/// with no integer payloads, or to read a digit whose position they already
/// know. Use [`column_site_count`] to count column sites.
fn menu_digits(rt: &SqlSchemaLinkingRuntime, tokens: &[u32]) -> Vec<u32> {
    let (start, end) = menu_band(rt);
    tokens
        .iter()
        .filter(|&&t| t >= start && t < end)
        .map(|&t| t - start)
        .collect()
}

/// The three L4 scopes for `singer.Name`, as SQL.
const SCOPE_SINGER: &str = "SELECT singer.Name FROM singer";
const SCOPE_STADIUM_SINGER: &str =
    "SELECT singer.Name FROM stadium JOIN singer ON singer.Singer_ID = stadium.Stadium_ID";
const SCOPE_CONCERT_SINGER: &str =
    "SELECT singer.Name FROM concert JOIN singer ON singer.Singer_ID = concert.Stadium_ID";

#[test]
fn flat_addressing_reproduces_the_measured_from_order_dependence() {
    // Baseline the L4 finding: the identical column `singer.Name` (prompt `#1`)
    // is emitted as 1, 8, or 6 under the historical flat encoding.
    let rt = flat_runtime(concert_singer_catalog());
    let ordinals: Vec<u32> = [SCOPE_SINGER, SCOPE_STADIUM_SINGER, SCOPE_CONCERT_SINGER]
        .iter()
        .map(|sql| {
            let tokens = rt
                .encode(sql)
                .unwrap_or_else(|e| panic!("encode {sql}: {e}"));
            // Last menu digit is the select-list column reference.
            *menu_digits(&rt, &tokens)
                .last()
                .expect("column menu digit present")
        })
        .collect();
    assert_eq!(
        ordinals,
        vec![1, 8, 6],
        "flat scope-relative ordinals must stay FROM-order dependent (the measured tax)"
    );
}

#[test]
fn factored_addressing_emits_a_stable_table_column_pair_regardless_of_from_order() {
    // The property the policy exists for: the same (table, column) digit pair
    // in every scope, matching the prompt's own `table #1` / `#1` notation.
    let rt = factored_runtime(concert_singer_catalog());
    for sql in [SCOPE_SINGER, SCOPE_STADIUM_SINGER, SCOPE_CONCERT_SINGER] {
        let tokens = rt
            .encode(sql)
            .unwrap_or_else(|e| panic!("encode {sql}: {e}"));
        let digits = menu_digits(&rt, &tokens);
        let pair = digits.split_at(digits.len() - 2).1.to_vec();
        assert_eq!(
            pair,
            vec![1, 1],
            "singer.Name must be (table 1, column 1) under factored addressing; {sql} gave {digits:?}"
        );
    }
}

/// Stream indices of the menu-band tokens, in order.
fn menu_positions(rt: &SqlSchemaLinkingRuntime, tokens: &[u32]) -> Vec<usize> {
    let (start, end) = menu_band(rt);
    tokens
        .iter()
        .enumerate()
        .filter(|&(_, &t)| t >= start && t < end)
        .map(|(i, _)| i)
        .collect()
}

/// Mask at the prefix `tokens[..len]`.
fn mask_at_prefix(
    rt: &SqlSchemaLinkingRuntime,
    tokens: &[u32],
    len: usize,
) -> (SqlTctDecodeState, BTreeSet<u32>) {
    let mut state = rt.begin_tct_decode();
    for &token in tokens.iter().take(len) {
        state.apply(token).expect("apply prefix token");
    }
    let allowed = state
        .allowed_token_ids()
        .expect("mask")
        .into_iter()
        .collect();
    (state, allowed)
}

#[test]
fn factored_column_digit_admits_exactly_the_chosen_tables_columns() {
    // After the table digit, the admissible column set is that table's columns
    // only — not the union over in-scope tables.
    let rt = factored_runtime(concert_singer_catalog());
    let (menu_start, _) = menu_band(&rt);
    let tokens = rt.encode(SCOPE_STADIUM_SINGER).expect("encode");
    // The select-list `singer.Name` reference is the last column site, so its
    // (table, column) digits are the final two menu-band positions.
    let positions = menu_positions(&rt, &tokens);
    let column_digit_at = *positions.last().expect("a column digit is present");

    let (_, allowed) = mask_at_prefix(&rt, &tokens, column_digit_at);
    // Table digit just applied was `singer` (catalog index 1, 7 columns).
    let expected: BTreeSet<u32> = (0..7u32).map(|c| menu_start + c).collect();
    assert_eq!(
        allowed, expected,
        "digit 2 must admit exactly singer's 7 columns"
    );
    assert_eq!(
        tokens[column_digit_at] - menu_start,
        1,
        "singer.Name is column 1 of singer"
    );
}

#[test]
fn out_of_scope_table_is_inadmissible_at_the_table_digit() {
    // `concert` and `singer_in_concert` are not in scope for a
    // `stadium JOIN singer` query, so their table digits must be unwritable.
    let rt = factored_runtime(concert_singer_catalog());
    let (menu_start, _) = menu_band(&rt);
    let tokens = rt.encode(SCOPE_STADIUM_SINGER).expect("encode");
    let positions = menu_positions(&rt, &tokens);
    let table_digit_at = positions[positions.len() - 2];

    let (state, allowed) = mask_at_prefix(&rt, &tokens, table_digit_at);
    let expected: BTreeSet<u32> = [menu_start, menu_start + 1].into_iter().collect();
    assert_eq!(
        allowed, expected,
        "only stadium(0) and singer(1) may be addressed at the table digit"
    );
    for out_of_scope in [2u32, 3] {
        let mut probe = state.clone();
        assert!(
            probe.apply(menu_start + out_of_scope).is_err(),
            "out-of-scope table digit {out_of_scope} must fail apply"
        );
    }
}

#[test]
fn factored_round_trip_is_lossless_and_mask_equals_accept() {
    let rt = factored_runtime(concert_singer_catalog());
    let queries = [
        SCOPE_SINGER,
        SCOPE_STADIUM_SINGER,
        SCOPE_CONCERT_SINGER,
        "SELECT singer.Name, singer.Age FROM singer WHERE singer.Age > 20",
        "SELECT count ( singer.Singer_ID ) FROM singer",
        "SELECT concert.Theme FROM concert JOIN singer_in_concert ON singer_in_concert.concert_ID = concert.concert_ID",
    ];
    for sql in queries {
        let tokens = rt
            .encode(sql)
            .unwrap_or_else(|e| panic!("encode {sql}: {e}"));
        let rendered = rt
            .decode(&tokens)
            .unwrap_or_else(|e| panic!("decode {sql}: {e}"));
        let reencoded = rt
            .encode(&rendered)
            .unwrap_or_else(|e| panic!("re-encode {rendered}: {e}"));
        assert_eq!(reencoded, tokens, "render fixpoint for {sql}");

        // Elided round trip under the same policy.
        let elided = rt.encode_elided(sql).expect("elide");
        let expanded = rt.expand_elided_tokens(&elided).expect("expand");
        assert_eq!(expanded, tokens, "elided round trip for {sql}");

        // Full-vocab mask==accept sweep at every prefix.
        let mut state = rt.begin_tct_decode();
        for (i, &token) in tokens.iter().enumerate() {
            let allowed: BTreeSet<u32> = state
                .allowed_token_ids()
                .expect("mask")
                .into_iter()
                .collect();
            assert!(
                allowed.contains(&token),
                "{sql} prefix_len={i}: gold token {token} missing from mask"
            );
            let prefix = state.tokens().to_vec();
            for candidate in 0..rt.pad_token() {
                let in_mask = allowed.contains(&candidate);
                let accepted = rt.codec().model().transition(&prefix, candidate).accepted;
                assert_eq!(
                    in_mask, accepted,
                    "{sql} prefix_len={i} candidate={candidate}: mask={in_mask} accept={accepted}"
                );
            }
            state.apply(token).expect("apply");
        }
        assert!(state.is_complete().expect("complete"));
    }
}

#[test]
fn factored_policy_composes_with_structural_join_on() {
    let rt = SqlSchemaLinkingRuntime::from_catalog_with_policies(
        concert_singer_catalog(),
        SqlJoinOnPolicy::Structural,
        SqlColumnAddressing::Factored,
    )
    .expect("combined runtime");
    let sql = SCOPE_STADIUM_SINGER;
    let tokens = rt.encode(sql).expect("encode combined");
    let elided = rt.encode_elided(sql).expect("elide combined");
    assert_eq!(
        rt.expand_elided_tokens(&elided).expect("expand"),
        tokens,
        "combined-policy elided stream must invert losslessly"
    );
    assert_eq!(
        rt.stream_contract(),
        RAW_SYNTAX_STREAM_CONTRACT_ELIDED_V1_JOINON_STRUCTURAL_FACTORED_COLUMNS
    );

    // Under the structural pair constraint the right-hand join key must name a
    // table on the other side of the join, so its table digit is a singleton.
    let (menu_start, _) = menu_band(&rt);
    let mut state = rt.begin_tct_decode();
    let mut singleton_table_digit_seen = false;
    for &token in &tokens {
        let allowed = state.allowed_token_ids().expect("mask");
        if allowed.len() == 1 && allowed[0] == token && token >= menu_start {
            singleton_table_digit_seen = true;
        }
        state.apply(token).expect("apply");
    }
    assert!(
        singleton_table_digit_seen,
        "structural join-ON must force at least one menu digit under factored addressing"
    );
}

#[test]
fn single_table_scopes_pay_nothing_and_multi_table_scopes_pay_one_token_per_column() {
    // The cost model, pinned. With one table in scope the table digit has a
    // singleton mask, so it is mask-forced and vanishes from the canonical
    // elided stream — and the surviving column digit is the same per-table
    // index the flat encoding already emitted. Elided streams are therefore
    // BYTE-IDENTICAL for single-table queries; only multi-table scopes pay.
    let flat = flat_runtime(concert_singer_catalog());
    let factored = factored_runtime(concert_singer_catalog());

    for sql in [
        SCOPE_SINGER,
        "SELECT singer.Name, singer.Age, singer.Country FROM singer",
        "SELECT singer.Name FROM singer WHERE singer.Age > 20 ORDER BY singer.Name ASC",
    ] {
        let flat_elided = flat.encode_elided(sql).expect("flat elide");
        let factored_elided = factored.encode_elided(sql).expect("factored elide");
        assert_eq!(
            flat_elided, factored_elided,
            "single-table scope must cost nothing under factored addressing: {sql}"
        );
        // The expanded streams still differ by exactly one token per column
        // reference (the forced table digit is present before elision).
        assert_eq!(
            factored.encode(sql).expect("factored encode").len(),
            flat.encode(sql).expect("flat encode").len() + column_site_count(&flat, sql),
            "expanded stream grows by one token per column reference: {sql}"
        );
    }

    // Two tables in scope: the table digit becomes a real choice, so each
    // column reference costs exactly one extra elided token.
    for sql in [SCOPE_STADIUM_SINGER, SCOPE_CONCERT_SINGER] {
        let sites = column_site_count(&flat, sql);
        assert!(sites >= 3, "join query has ON pair + select column: {sql}");
        assert_eq!(
            factored.encode_elided(sql).expect("factored elide").len(),
            flat.encode_elided(sql).expect("flat elide").len() + sites,
            "each column reference in a multi-table scope costs one elided token: {sql}"
        );
    }
}

#[test]
fn flat_policy_output_is_byte_identical_to_the_default_runtime() {
    // P10-style guard: binding the new policy enum with `Flat` must not perturb
    // a single token relative to today's default construction path.
    let default_rt =
        SqlSchemaLinkingRuntime::from_catalog(concert_singer_catalog()).expect("default runtime");
    let explicit_flat = SqlSchemaLinkingRuntime::from_catalog_with_policies(
        concert_singer_catalog(),
        SqlJoinOnPolicy::Off,
        SqlColumnAddressing::Flat,
    )
    .expect("explicit flat runtime");
    for sql in [SCOPE_SINGER, SCOPE_STADIUM_SINGER, SCOPE_CONCERT_SINGER] {
        assert_eq!(
            default_rt.encode(sql).expect("default encode"),
            explicit_flat.encode(sql).expect("flat encode"),
            "explicit Flat must be bit-identical to the default for {sql}"
        );
        assert_eq!(
            default_rt.encode_elided(sql).expect("default elide"),
            explicit_flat.encode_elided(sql).expect("flat elide"),
        );
    }
    assert_eq!(explicit_flat.column_addressing(), SqlColumnAddressing::Flat);
    assert_eq!(
        default_rt.stream_contract(),
        RAW_SYNTAX_STREAM_CONTRACT_ELIDED_V1
    );
}

#[test]
fn gold_fixture_queries_round_trip_under_factored_addressing() {
    let path = fixture_dir().join("tables.json");
    let bytes = std::fs::read(&path).expect("read tables");
    let sha = crate::provenance::sha256_hex(&bytes);
    let rt = SqlSchemaLinkingRuntime::from_catalog_json_bytes_with_policies(
        &bytes,
        &sha,
        SQL_SCHEMA_LINKING_FIXTURE_DB_ID,
        SqlJoinOnPolicy::Off,
        SqlColumnAddressing::Factored,
    )
    .expect("factored fixture runtime");
    let flat = SqlSchemaLinkingRuntime::from_catalog_json_bytes(
        &bytes,
        &sha,
        SQL_SCHEMA_LINKING_FIXTURE_DB_ID,
    )
    .expect("flat fixture runtime");

    let gold: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(fixture_dir().join("gold_queries.json")).expect("read gold"),
    )
    .expect("gold json");
    let queries = gold["queries"].as_array().expect("gold queries array");
    assert!(!queries.is_empty());

    let mut factored_total = 0usize;
    let mut flat_total = 0usize;
    for query in queries {
        let sql = query["sql"].as_str().expect("gold sql");
        let tokens = rt
            .encode(sql)
            .unwrap_or_else(|e| panic!("factored encode {sql}: {e}"));
        let rendered = rt
            .decode(&tokens)
            .unwrap_or_else(|e| panic!("factored decode {sql}: {e}"));
        assert_eq!(
            rt.encode(&rendered).expect("re-encode"),
            tokens,
            "render fixpoint for {sql}"
        );
        let elided = rt.encode_elided(sql).expect("elide");
        assert_eq!(rt.expand_elided_tokens(&elided).expect("expand"), tokens);
        factored_total += elided.len();
        flat_total += flat.encode_elided(sql).expect("flat elide").len();
    }
    assert!(
        factored_total >= flat_total,
        "factored addressing may only add tokens (flat={flat_total}, factored={factored_total})"
    );
}

#[test]
fn stream_contract_is_tagged_for_every_policy_combination() {
    let cases = [
        (
            SqlJoinOnPolicy::Off,
            SqlColumnAddressing::Flat,
            false,
            RAW_SYNTAX_STREAM_CONTRACT_ELIDED_V1,
        ),
        (
            SqlJoinOnPolicy::Structural,
            SqlColumnAddressing::Flat,
            false,
            RAW_SYNTAX_STREAM_CONTRACT_ELIDED_V1_JOINON_STRUCTURAL,
        ),
        (
            SqlJoinOnPolicy::Off,
            SqlColumnAddressing::Factored,
            false,
            RAW_SYNTAX_STREAM_CONTRACT_ELIDED_V1_FACTORED_COLUMNS,
        ),
        (
            SqlJoinOnPolicy::Structural,
            SqlColumnAddressing::Factored,
            false,
            RAW_SYNTAX_STREAM_CONTRACT_ELIDED_V1_JOINON_STRUCTURAL_FACTORED_COLUMNS,
        ),
        (
            SqlJoinOnPolicy::FkDeclared,
            SqlColumnAddressing::Flat,
            false,
            RAW_SYNTAX_STREAM_CONTRACT_ELIDED_V1_JOINON_FK_DECLARED,
        ),
        (
            SqlJoinOnPolicy::FkDeclared,
            SqlColumnAddressing::Factored,
            false,
            RAW_SYNTAX_STREAM_CONTRACT_ELIDED_V1_JOINON_FK_DECLARED_FACTORED_COLUMNS,
        ),
        (
            SqlJoinOnPolicy::Off,
            SqlColumnAddressing::Flat,
            true,
            RAW_SYNTAX_STREAM_CONTRACT_ELIDED_V1_VISIT_ALL_FIELDS,
        ),
        (
            SqlJoinOnPolicy::Structural,
            SqlColumnAddressing::Flat,
            true,
            RAW_SYNTAX_STREAM_CONTRACT_ELIDED_V1_JOINON_STRUCTURAL_VISIT_ALL_FIELDS,
        ),
        (
            SqlJoinOnPolicy::Off,
            SqlColumnAddressing::Factored,
            true,
            RAW_SYNTAX_STREAM_CONTRACT_ELIDED_V1_FACTORED_COLUMNS_VISIT_ALL_FIELDS,
        ),
        (
            SqlJoinOnPolicy::Structural,
            SqlColumnAddressing::Factored,
            true,
            RAW_SYNTAX_STREAM_CONTRACT_ELIDED_V1_JOINON_STRUCTURAL_FACTORED_COLUMNS_VISIT_ALL_FIELDS,
        ),
        (
            SqlJoinOnPolicy::FkDeclared,
            SqlColumnAddressing::Flat,
            true,
            RAW_SYNTAX_STREAM_CONTRACT_ELIDED_V1_JOINON_FK_DECLARED_VISIT_ALL_FIELDS,
        ),
        (
            SqlJoinOnPolicy::FkDeclared,
            SqlColumnAddressing::Factored,
            true,
            RAW_SYNTAX_STREAM_CONTRACT_ELIDED_V1_JOINON_FK_DECLARED_FACTORED_COLUMNS_VISIT_ALL_FIELDS,
        ),
    ];
    let mut seen: BTreeSet<&'static str> = BTreeSet::new();
    for (join_on, addressing, visit_all, expected) in cases {
        let rt = SqlSchemaLinkingRuntime::from_catalog_with_sql_policies(
            concert_singer_catalog(),
            join_on,
            addressing,
            visit_all,
        )
        .expect("policy runtime");
        assert_eq!(rt.stream_contract(), expected);
        assert_eq!(rt.column_addressing(), addressing);
        assert_eq!(rt.visit_all_fields(), visit_all);
        assert_eq!(
            rt.capability_report().column_addressing,
            addressing.as_str()
        );
        assert_eq!(rt.capability_report().visit_all_fields, visit_all);
        assert!(seen.insert(expected), "contract tags must be distinct");
    }
    assert_eq!(seen.len(), 12);
}

#[test]
fn cross_policy_elided_streams_fail_closed() {
    // A stream encoded under one addressing policy must not decode under the
    // other: corpora and BPE artifacts cannot silently mix.
    let flat = flat_runtime(concert_singer_catalog());
    let factored = factored_runtime(concert_singer_catalog());
    let sql = SCOPE_STADIUM_SINGER;
    let flat_elided = flat.encode_elided(sql).expect("flat elide");
    let factored_elided = factored.encode_elided(sql).expect("factored elide");
    assert_ne!(
        flat_elided, factored_elided,
        "the policies must produce different canonical streams"
    );

    let flat_render = flat
        .strict_decode_complete_elided(&flat_elided)
        .expect("flat strict");
    match factored.strict_decode_complete_elided(&flat_elided) {
        Err(_) => {}
        Ok(other) => assert_ne!(
            other, flat_render,
            "a flat stream must not decode to the same SQL under factored addressing"
        ),
    }
}

/// Single table `t` with 300 columns `c0..c299`: flat addressing needs a
/// two-digit radix code (width 2 over the 256 band), factored addressing
/// cannot bind at all.
fn wide_single_table_catalog() -> SqlCatalog {
    let mut columns = String::from("[-1, \"*\"]");
    for i in 0..300 {
        columns.push_str(&format!(", [0, \"c{i}\"]"));
    }
    let mut types = String::from("\"text\"");
    for _ in 0..300 {
        types.push_str(", \"number\"");
    }
    let tables_json = format!(
        r#"[{{
            "db_id": "wide_table",
            "table_names_original": ["t"],
            "table_names": ["t"],
            "column_names_original": [{columns}],
            "column_names": [{columns}],
            "column_types": [{types}],
            "primary_keys": [],
            "foreign_keys": []
        }}]"#
    );
    SqlCatalog::parse_tables_json_unpinned(tables_json.as_bytes(), "wide_table")
        .expect("wide-table catalog parses")
}

#[test]
fn factored_binding_fails_closed_when_a_table_exceeds_the_menu_band() {
    // Digit 2 addresses columns inside one table, so a table wider than the
    // 256-slot band is unrepresentable and must be rejected at bind time
    // rather than emitting a dead digit.
    let catalog = wide_single_table_catalog();
    // Flat addressing still binds (two radix digits cover 65_536 ordinals)...
    assert!(SqlSchemaLinkingRuntime::from_catalog(catalog.clone()).is_ok());
    // ...but factored addressing must fail closed.
    let err = SqlSchemaLinkingRuntime::from_catalog_with_policies(
        catalog,
        SqlJoinOnPolicy::Off,
        SqlColumnAddressing::Factored,
    )
    .expect_err("300-column table must not bind under factored addressing");
    let message = format!("{err}");
    assert!(
        message.contains("factored"),
        "expected a factored capacity error, got: {message}"
    );
}

/// Advance along the gold elided stream until the active menu has `want` kind.
///
/// Returns the menu together with the number of gold tokens consumed, so
/// callers can continue from the *actual* gold continuation instead of
/// restarting the stream from index 0.
fn advance_to_menu_kind(
    st: &mut SqlElidedTctDecodeState,
    tokens: &[u32],
    want: &str,
) -> (SqlActiveMenu, usize) {
    let mut consumed = 0usize;
    loop {
        let menu = st.active_menu().expect("menu");
        if menu.kind == want {
            return (menu, consumed);
        }
        if consumed >= tokens.len() || st.is_complete().expect("complete?") {
            panic!("never reached menu kind {want}");
        }
        st.apply(tokens[consumed]).expect("apply gold");
        consumed += 1;
    }
}

#[test]
fn active_menu_names_factored_column_digit_after_elided_singleton_table() {
    // Single-table scope forces the factored table digit (cardinality 1), so
    // the elided stream's first column site is the column-in-table digit and
    // must name singer's columns in catalog order — the same `#0 Singer_ID,
    // #1 Name, …` list the prompt already shows.
    let rt = factored_runtime(concert_singer_catalog());
    let tokens = rt.encode_elided(SCOPE_SINGER).expect("elide");
    let mut st = rt.begin_elided_tct_decode().expect("begin");
    let (menu, _) = advance_to_menu_kind(&mut st, &tokens, "column");
    assert_eq!(menu.addressing, Some("factored"));
    assert_eq!(menu.digits_seen, 1);
    assert_eq!(menu.width, 2);
    assert!(menu.resolved);
    let names: Vec<&str> = menu.referents.iter().map(|r| r.name.as_str()).collect();
    assert_eq!(
        names,
        vec![
            "Singer_ID",
            "Name",
            "Country",
            "Song_Name",
            "Song_release_year",
            "Age",
            "Is_male",
        ]
    );
    assert_eq!(
        menu.referents.iter().map(|r| r.ordinal).collect::<Vec<_>>(),
        vec![0, 1, 2, 3, 4, 5, 6]
    );
}

#[test]
fn active_menu_names_in_scope_tables_then_that_tables_columns() {
    let rt = factored_runtime(concert_singer_catalog());
    let tokens = rt.encode_elided(SCOPE_STADIUM_SINGER).expect("elide");
    let mut st = rt.begin_elided_tct_decode().expect("begin");
    let (table_menu, consumed) = advance_to_menu_kind(&mut st, &tokens, "column_table");
    assert_eq!(table_menu.addressing, Some("factored"));
    assert_eq!(table_menu.width, 2);
    assert!(table_menu.resolved);
    let table_names: Vec<&str> = table_menu
        .referents
        .iter()
        .map(|r| r.name.as_str())
        .collect();
    assert_eq!(table_names, vec!["stadium", "singer"]);
    assert_eq!(
        table_menu
            .referents
            .iter()
            .map(|r| r.ordinal)
            .collect::<Vec<_>>(),
        vec![0, 1]
    );

    // Continue with the *gold* table digit: the first column site is the
    // ON-lhs `singer.Singer_ID`, so gold chooses catalog table 1 (singer).
    // Pinning the chosen table first is what makes the exact column-list
    // assertion below a narrowing proof rather than a membership check an
    // un-narrowed union would also satisfy.
    let (band_start, _) = menu_band(&rt);
    let gold_table_digit = tokens[consumed];
    assert_eq!(
        gold_table_digit - band_start,
        1,
        "gold pair for the ON-lhs must address singer (catalog table 1)"
    );
    st.apply(gold_table_digit).expect("apply gold table digit");
    let col_menu = st.active_menu().expect("column digit");
    assert_eq!(col_menu.kind, "column");
    assert_eq!(col_menu.digits_seen, 1);
    assert!(col_menu.resolved);
    let names: Vec<&str> = col_menu.referents.iter().map(|r| r.name.as_str()).collect();
    assert_eq!(
        names,
        vec![
            "Singer_ID",
            "Name",
            "Country",
            "Song_Name",
            "Song_release_year",
            "Age",
            "Is_male",
        ],
        "column digit must name exactly the chosen table's columns"
    );
    assert_eq!(
        col_menu
            .referents
            .iter()
            .map(|r| r.ordinal)
            .collect::<Vec<_>>(),
        vec![0, 1, 2, 3, 4, 5, 6]
    );
}

#[test]
fn active_menu_names_flat_scope_concatenation_across_tables() {
    // The DEFAULT addressing policy: one radix ordinal over the in-scope
    // concatenation (stadium then singer => flat 0..13). Width is 1 here, so
    // the single digit resolves directly to names.
    let rt = flat_runtime(concert_singer_catalog());
    let tokens = rt.encode_elided(SCOPE_STADIUM_SINGER).expect("elide");
    let mut st = rt.begin_elided_tct_decode().expect("begin");
    let (menu, _) = advance_to_menu_kind(&mut st, &tokens, "column_flat");
    assert_eq!(menu.addressing, Some("flat"));
    assert_eq!(menu.width, 1);
    assert_eq!(menu.digits_seen, 0);
    assert!(menu.resolved);
    let names: Vec<&str> = menu.referents.iter().map(|r| r.name.as_str()).collect();
    assert_eq!(
        names,
        vec![
            "Stadium_ID",
            "Location",
            "Name",
            "Capacity",
            "Highest",
            "Lowest",
            "Average",
            "Singer_ID",
            "Name",
            "Country",
            "Song_Name",
            "Song_release_year",
            "Age",
            "Is_male",
        ],
        "flat referents must walk the scope concatenation in FROM order"
    );
    assert_eq!(
        menu.referents.iter().map(|r| r.ordinal).collect::<Vec<_>>(),
        (0..14).collect::<Vec<u32>>()
    );
}

#[test]
fn active_menu_defers_naming_on_the_leading_wide_radix_digit_then_resolves() {
    // 300 columns force a two-digit radix code under flat addressing. A
    // leading digit spans a 256-ordinal range — naming it as an ordinal would
    // be silently wrong — so referents stay empty until the final digit,
    // which names exactly the ordinals the chosen prefix can complete.
    let rt = flat_runtime(wide_single_table_catalog());
    let tokens = rt.encode_elided("SELECT t.c299 FROM t").expect("elide");
    let mut st = rt.begin_elided_tct_decode().expect("begin");
    let (menu, consumed) = advance_to_menu_kind(&mut st, &tokens, "column_flat");
    assert_eq!(menu.width, 2);
    assert_eq!(menu.digits_seen, 0);
    assert!(
        !menu.resolved,
        "a leading radix digit must not claim named referents"
    );
    assert!(menu.referents.is_empty());

    let (band_start, _) = menu_band(&rt);
    let gold_high_digit = tokens[consumed];
    assert_eq!(gold_high_digit - band_start, 1, "c299 = 1*256 + 43");
    st.apply(gold_high_digit).expect("apply high digit");
    let low_menu = st.active_menu().expect("low digit menu");
    assert_eq!(low_menu.kind, "column_flat");
    assert_eq!(low_menu.digits_seen, 1);
    assert!(low_menu.resolved);
    assert_eq!(
        low_menu
            .referents
            .iter()
            .map(|r| r.ordinal)
            .collect::<Vec<_>>(),
        (256..300).collect::<Vec<u32>>(),
        "low digits after high=1 complete ordinals 256..299"
    );
    assert_eq!(
        low_menu.referents.first().map(|r| r.name.as_str()),
        Some("c256")
    );
    assert_eq!(
        low_menu.referents.last().map(|r| r.name.as_str()),
        Some("c299")
    );
}

#[test]
fn active_menu_reports_unresolved_after_the_factored_pair_completes() {
    // On the expanded walker the cursor can rest between the pair's last
    // digit and payload_end. `menu_partial` is then the flat ordinal, not a
    // table index, so the resolver must report no referents instead of
    // looking up an unrelated table.
    let rt = factored_runtime(concert_singer_catalog());
    let tokens = rt.encode(SCOPE_SINGER).expect("encode");
    let mut st = rt.begin_tct_decode();
    let last_menu_pos = *menu_positions(&rt, &tokens).last().expect("menu tokens");
    for &tok in &tokens[..=last_menu_pos] {
        st.apply(tok).expect("apply");
    }
    let menu = st.active_menu().expect("menu");
    assert_eq!(menu.kind, "column");
    assert_eq!(menu.digits_seen, 2);
    assert_eq!(menu.width, 2);
    assert!(!menu.resolved);
    assert!(menu.referents.is_empty());
}
