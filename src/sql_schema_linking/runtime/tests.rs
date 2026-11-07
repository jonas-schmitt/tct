use super::*;
use crate::sql_schema_linking::catalog::{
    SQL_SCHEMA_LINKING_FIXTURE_DB_ID, fixture_dir, load_fixture_bundle, load_fixture_catalog,
    load_wide_menu_capacity_catalog,
};
use crate::sql_schema_linking::layout::menu_band_range;
use crate::sql_schema_linking::layout::sql_syntax_token_layout;
use serde::Deserialize;
use std::collections::BTreeSet;

#[derive(Debug, Deserialize)]
struct GoldFile {
    queries: Vec<GoldQuery>,
}

#[derive(Debug, Deserialize)]
struct GoldQuery {
    id: String,
    sql: String,
}

fn gold_sql(id: &str) -> String {
    let bundle = load_fixture_bundle().expect("bundle");
    let file: GoldFile = serde_json::from_slice(&bundle.gold_queries_bytes).expect("parse gold");
    file.queries
        .into_iter()
        .find(|q| q.id == id)
        .unwrap_or_else(|| panic!("missing gold query {id}"))
        .sql
}

fn fixture_runtime() -> SqlSchemaLinkingRuntime {
    SqlSchemaLinkingRuntime::fixture().expect("fixture runtime")
}

/// Exact 64-hex SHA-256 used only as a valid tokenizer_hash fixture value.
fn sample_tokenizer_hash() -> String {
    "a".repeat(64)
}

#[test]
fn fixture_runtime_encodes_and_strict_decodes_round_trip() {
    let rt = fixture_runtime();
    let sql = gold_sql("select_one_table");
    let tokens = rt.encode(&sql).expect("encode");
    assert!(tokens.iter().all(|&t| t < rt.pad_token()));
    let rendered = rt.decode(&tokens).expect("decode");
    assert_eq!(rendered, r#"SELECT "name", "city" FROM "customers""#);
    let again = rt.encode(&rendered).expect("re-encode");
    assert_eq!(again, tokens);
}

#[test]
fn strict_decode_rejects_pad_and_incomplete_and_extra_token() {
    let rt = fixture_runtime();
    let tokens = rt.encode(&gold_sql("star_structural")).expect("encode");

    let mut with_pad = tokens.clone();
    with_pad.push(rt.pad_token());
    let err = rt.decode(&with_pad).expect_err("PAD must fail");
    assert!(
        err.message.contains("PAD") || err.phase == "tokens" || err.phase == "raw_sm",
        "{err}"
    );

    let incomplete = &tokens[..tokens.len().saturating_sub(1)];
    let err = rt.decode(incomplete).expect_err("incomplete must fail");
    assert!(
        err.phase == "raw_sm" || err.phase == "decode" || err.phase == "decode_tree",
        "unexpected incomplete error: {err}"
    );

    let mut extra = tokens.clone();
    // A structural token that cannot trail a complete stream.
    extra.push(0);
    let err = rt.decode(&extra).expect_err("extra token must fail");
    assert!(
        err.phase == "raw_sm" || err.phase == "decode",
        "unexpected extra error: {err}"
    );
}

#[test]
fn wrong_catalog_sha_fails_closed() {
    let path = fixture_dir().join("tables.json");
    let bytes = std::fs::read(&path).expect("read tables");
    let bad_sha = "0".repeat(64);
    let err = SqlSchemaLinkingRuntime::from_catalog_json_bytes(
        &bytes,
        &bad_sha,
        SQL_SCHEMA_LINKING_FIXTURE_DB_ID,
    )
    .expect_err("wrong sha");
    assert_eq!(err.phase, "catalog");
    assert!(err.message.contains("sha256 mismatch"), "{err}");
}

#[test]
fn constructor_accepts_fixture_sha_and_db_id() {
    let path = fixture_dir().join("tables.json");
    let bytes = std::fs::read(&path).expect("read tables");
    let sha = sha256_hex(&bytes);
    let rt = SqlSchemaLinkingRuntime::from_catalog_json_bytes(
        &bytes,
        &sha,
        SQL_SCHEMA_LINKING_FIXTURE_DB_ID,
    )
    .expect("construct");
    assert_eq!(rt.db_id(), SQL_SCHEMA_LINKING_FIXTURE_DB_ID);
    assert_eq!(rt.catalog_sha256(), sha);
    assert_eq!(rt.vocab_size(), SQL_SYNTAX_LAYOUT_VOCAB_SIZE);
    assert_eq!(rt.pad_token(), SQL_SYNTAX_LAYOUT_PAD_TOKEN);
}

/// Mask membership on the incremental state must equal model transition
/// acceptance (and therefore apply) for every candidate below PAD.
fn assert_mask_equals_transition(
    rt: &SqlSchemaLinkingRuntime,
    state: &SqlTctDecodeState,
    label: &str,
) {
    let allowed: BTreeSet<u32> = state
        .allowed_token_ids()
        .expect("mask")
        .into_iter()
        .collect();
    let prefix = state.tokens();
    let pad = rt.pad_token();
    for candidate in 0..pad {
        let in_mask = allowed.contains(&candidate);
        let accepted = rt.codec().model().transition(prefix, candidate).accepted;
        assert_eq!(
            in_mask,
            accepted,
            "{label} prefix_len={} candidate={candidate} mask={in_mask} transition={accepted}",
            prefix.len()
        );
    }
    // Spot-check: a few mask members and non-members via apply.
    for &candidate in allowed.iter().take(3) {
        let mut probe = state.clone();
        assert!(
            probe.apply(candidate).is_ok(),
            "{label}: mask member {candidate} must apply"
        );
    }
    if let Some(outside) = (0..pad).find(|c| !allowed.contains(c)) {
        let mut probe = state.clone();
        assert!(
            probe.apply(outside).is_err(),
            "{label}: non-member {outside} must fail apply"
        );
    }
}

#[test]
fn incremental_tct_decode_replays_encode_stream_and_renders() {
    let rt = fixture_runtime();
    let sql = gold_sql("join_two_tables");
    let tokens = rt.encode(&sql).expect("encode");
    let mut state = rt.begin_tct_decode();
    assert!(!state.is_complete().expect("empty complete"));
    assert_eq!(state.consumed_tokens(), 0);
    assert_eq!(state.applied_transitions(), 0);

    for (i, &token) in tokens.iter().enumerate() {
        let allowed = state.allowed_token_ids().expect("mask");
        assert!(
            allowed.contains(&token),
            "token[{i}]={token} must be in allowed mask"
        );
        assert_mask_equals_transition(&rt, &state, &format!("join_two_tables@{i}"));
        state.apply(token).expect("apply gold token");
    }
    assert!(state.is_complete().expect("complete"));
    assert!(state.allowed_token_ids().expect("done mask").is_empty());
    let rendered = state.render_sql().expect("render");
    assert_eq!(rendered, rt.decode(&tokens).expect("decode"));

    let err = state.apply(0).expect_err("extra after complete");
    assert!(err.message.contains("complete"), "{err}");
}

/// Prove live O(1) advance: each apply performs exactly one engine transition,
/// so after N successful applies `applied_transitions == N` (not triangular).
#[test]
fn incremental_apply_is_one_transition_per_token_not_prefix_replay() {
    let rt = fixture_runtime();
    let tokens = rt.encode(&gold_sql("join_two_tables")).expect("encode");
    assert!(tokens.len() >= 8, "need a non-trivial stream");

    let mut state = rt.begin_tct_decode();
    assert_eq!(state.applied_transitions(), 0);

    for (i, &token) in tokens.iter().enumerate() {
        let before = state.applied_transitions();
        // Mask must not advance the cursor.
        let _ = state.allowed_token_ids().expect("mask");
        assert_eq!(
            state.applied_transitions(),
            before,
            "allowed_token_ids must not apply transitions"
        );
        state.apply(token).expect("apply");
        let after = state.applied_transitions();
        assert_eq!(
            after,
            before + 1,
            "step {i}: expected exactly one transition per apply (live cursor), got before={before} after={after}"
        );
        // Reject triangular growth that full-prefix replay would produce.
        assert_eq!(
            after,
            (i as u64) + 1,
            "after {} applies, transitions must equal step count (not 1+2+...+k replay)",
            i + 1
        );
    }

    assert_eq!(state.applied_transitions(), tokens.len() as u64);
    assert_eq!(state.consumed_tokens(), tokens.len());
    assert!(state.is_complete().expect("complete"));
    // render_sql uses strict fixpoint; must not require replaying into applied_transitions.
    let before_render = state.applied_transitions();
    let _ = state.render_sql().expect("render");
    assert_eq!(
        state.applied_transitions(),
        before_render,
        "render_sql must not mutate live transition accounting"
    );
}

#[test]
fn multi_digit_menu_mask_equals_apply_on_wide_catalog() {
    // Wide catalog forces multi-digit menu encoding (T-D). Prefix masks on
    // the incremental runtime must equal transition acceptance.
    let catalog = load_wide_menu_capacity_catalog().expect("wide catalog");
    let rt = SqlSchemaLinkingRuntime::from_catalog(catalog).expect("runtime");
    // Wide catalog uses synthetic names (t000, c0, …); build SQL from catalog.
    let table0 = &rt.catalog().tables[0];
    let col0 = &table0.columns[0];
    let sql = format!("SELECT {} FROM {}", col0.name, table0.name);
    let tokens = rt
        .encode(&sql)
        .unwrap_or_else(|e| panic!("wide encode failed for {sql}: {e}"));
    assert!(
        tokens.len() > 2,
        "wide multi-digit streams should not be trivial: {tokens:?}"
    );

    let mut state = rt.begin_tct_decode();
    assert_eq!(state.applied_transitions(), 0);
    for (i, &token) in tokens.iter().enumerate() {
        let allowed: BTreeSet<u32> = state
            .allowed_token_ids()
            .expect("mask")
            .into_iter()
            .collect();
        assert!(
            allowed.contains(&token),
            "wide prefix_len={i}: gold token {token} missing from mask"
        );
        assert_mask_equals_transition(&rt, &state, &format!("wide@{i}"));
        state.apply(token).expect("apply");
        assert_eq!(state.applied_transitions(), (i as u64) + 1);
    }
    assert!(state.is_complete().unwrap());
    let _ = state.render_sql().expect("render wide");
}

#[test]
fn capability_report_and_text_constraint_fail_closed() {
    let rt = fixture_runtime();
    let caps = rt.capability_report();
    assert!(caps.a5_encode_decode);
    assert!(caps.a5_incremental_tct_decode);
    assert!(!caps.a1_text_structural_constraint);
    assert!(!caps.a2_text_dynamic_scope_constraint);
    let blocker = caps.text_constraint_blocker.expect("blocker");
    assert_eq!(blocker.code, SQL_TEXT_CONSTRAINT_BLOCKER_CODE);
    assert!(blocker.reusable);
    assert!(blocker.detail.contains("P6") || blocker.detail.contains("from-first"));

    let pieces = TextTokenPieceMap::new(
        sample_tokenizer_hash(),
        [(1u32, b"SELECT".to_vec()), (2u32, b" ".to_vec())],
    )
    .expect("pieces");
    assert_eq!(pieces.tokenizer_hash(), sample_tokenizer_hash());
    for dynamic in [false, true] {
        let err = rt
            .begin_text_constraint(&pieces, dynamic)
            .expect_err("must fail closed");
        assert_eq!(err.phase, "text_constraint");
        assert_eq!(err.blocker_code, Some(SQL_TEXT_CONSTRAINT_BLOCKER_CODE));
    }
}

#[test]
fn token_piece_map_requires_exact_sha256_and_valid_utf8() {
    let err = TextTokenPieceMap::new("", [(0, b"a".to_vec())]).expect_err("empty hash");
    assert_eq!(err.phase, "token_pieces");
    assert!(
        err.message.contains("64")
            || err.message.contains("SHA-256")
            || err.message.contains("hex"),
        "{err}"
    );

    let err = TextTokenPieceMap::new("not-a-hash", [(0, b"a".to_vec())]).expect_err("short hash");
    assert_eq!(err.phase, "token_pieces");

    let err = TextTokenPieceMap::new("g".repeat(64), [(0, b"a".to_vec())]).expect_err("non-hex");
    assert_eq!(err.phase, "token_pieces");

    let err =
        TextTokenPieceMap::new(sample_tokenizer_hash(), [(0, vec![0xff, 0xfe])]).expect_err("utf8");
    assert!(err.message.contains("UTF-8"), "{err}");

    let err = TextTokenPieceMap::new(
        sample_tokenizer_hash(),
        [(1, b"a".to_vec()), (1, b"b".to_vec())],
    )
    .expect_err("dup");
    assert!(err.message.contains("duplicate"), "{err}");

    // Uppercase hex is accepted and normalized to lowercase.
    let upper = "A".repeat(64);
    let map = TextTokenPieceMap::new(upper, [(0, b"x".to_vec())]).expect("upper hex");
    assert_eq!(map.tokenizer_hash(), "a".repeat(64));
}

#[test]
fn a5_scope_relative_column_mask_excludes_out_of_scope_on_join() {
    // Documents that A5 TCT (not A1/A2 text) already enforces live scope:
    // after customers JOIN orders, products columns are not admissible.
    let rt = fixture_runtime();
    let tokens = rt.encode(&gold_sql("join_two_tables")).expect("encode");
    let menu = menu_band_range(&sql_syntax_token_layout()).expect("menu");
    let mut state = rt.begin_tct_decode();
    let mut column_sites = 0usize;
    for &token in &tokens {
        let allowed: BTreeSet<u32> = state
            .allowed_token_ids()
            .expect("mask")
            .into_iter()
            .collect();
        // Pure column menu of card 7: all admitted ids in [menu.0, menu.0+7).
        if allowed.len() == 7 && allowed.iter().all(|&t| t >= menu.0 && t < menu.0 + 7) {
            column_sites += 1;
            for prod in 7..10 {
                assert!(
                    !allowed.contains(&(menu.0 + prod)),
                    "products ordinal {prod} must not be in A5 live-scope mask"
                );
            }
        }
        state.apply(token).expect("apply");
    }
    assert_eq!(column_sites, 4, "expected four column sites on join gold");
}

#[test]
fn render_sql_fails_on_incomplete_state() {
    let rt = fixture_runtime();
    let tokens = rt.encode(&gold_sql("select_one_table")).expect("encode");
    let mut state = rt.begin_tct_decode();
    state.apply(tokens[0]).expect("one token");
    let err = state.render_sql().expect_err("incomplete render");
    assert_eq!(err.phase, "render");
}

fn all_gold_queries() -> Vec<GoldQuery> {
    let bundle = load_fixture_bundle().expect("bundle");
    let file: GoldFile = serde_json::from_slice(&bundle.gold_queries_bytes).expect("parse gold");
    file.queries
}

/// Property: expand(elide(s)) == s for every gold stream in the fixture corpus.
#[test]
fn elide_expand_round_trip_over_full_gold_corpus() {
    let rt = fixture_runtime();
    let queries = all_gold_queries();
    assert!(!queries.is_empty(), "fixture gold corpus must be non-empty");

    let mut expanded_lens = Vec::new();
    let mut elided_lens = Vec::new();

    for q in &queries {
        let expanded = rt
            .encode(&q.sql)
            .unwrap_or_else(|e| panic!("encode {}: {e}", q.id));
        let elided = rt
            .elide_forced_tokens(&expanded)
            .unwrap_or_else(|e| panic!("elide {}: {e}", q.id));
        let reexpanded = rt
            .expand_elided_tokens(&elided)
            .unwrap_or_else(|e| panic!("expand {}: {e}", q.id));
        assert_eq!(
            reexpanded, expanded,
            "expand(elide(s)) == s failed for gold id={}",
            q.id
        );
        expanded_lens.push(expanded.len());
        elided_lens.push(elided.len());
    }

    // Median elided length should be roughly 1/4 of expanded (forced share ~75%).
    expanded_lens.sort_unstable();
    elided_lens.sort_unstable();
    let mid = expanded_lens.len() / 2;
    let med_exp = expanded_lens[mid] as f64;
    let med_eli = elided_lens[mid] as f64;
    let ratio = med_eli / med_exp;
    eprintln!(
        "sql elided corpus stats: n={} median_expanded={} median_elided={} ratio={:.4} \
         expanded_lens={:?} elided_lens={:?}",
        expanded_lens.len(),
        med_exp as usize,
        med_eli as usize,
        ratio,
        expanded_lens,
        elided_lens
    );
    assert!(
        (0.15..=0.40).contains(&ratio),
        "median elided/expanded length ratio {ratio:.3} (elided={med_eli}, expanded={med_exp}) \
         should be ~0.25; walk is not identifying forced positions like A5 mask decode"
    );
}

#[test]
fn elide_rejects_incomplete_stream() {
    let rt = fixture_runtime();
    let tokens = rt.encode(&gold_sql("select_one_table")).expect("encode");
    assert!(tokens.len() >= 2);
    let incomplete = &tokens[..tokens.len() - 1];
    let err = rt
        .elide_forced_tokens(incomplete)
        .expect_err("incomplete must fail elide");
    assert_eq!(
        err.phase, "premature_eof",
        "elide must require EOF-valid streams: {err}"
    );
}

#[test]
fn expand_rejects_spelled_out_forced_token_with_typed_error() {
    let rt = fixture_runtime();
    let expanded = rt.encode(&gold_sql("select_one_table")).expect("encode");
    let elided = rt.elide_forced_tokens(&expanded).expect("elide");
    assert!(!elided.is_empty(), "need at least one free-choice token");

    // Initial position is always forced (root node_start). Spelling it into
    // an otherwise-canonical elided stream must fail with the typed kind.
    let mut walker = rt.begin_elided_tct_decode().expect("begin elided");
    let initial = walker.initial_forced_run().to_vec();
    assert!(
        !initial.is_empty(),
        "SQL elided construction must drain a non-empty initial forced run"
    );
    let forced0 = initial[0];
    let err = walker
        .apply(forced0)
        .expect_err("spelling a forced token must fail");
    assert_eq!(
        err.phase, "forced_token_in_elided_stream",
        "typed error for forced token in elided stream: {err}"
    );

    // Prefixing the elided stream with a forced token is also rejected.
    let mut bad = vec![forced0];
    bad.extend_from_slice(&elided);
    let err = rt
        .expand_elided_tokens(&bad)
        .expect_err("expand must reject forced token");
    assert_eq!(
        err.phase, "forced_token_in_elided_stream",
        "expand typed error: {err}"
    );
}

#[test]
fn encode_elided_expand_decode_reproduces_sql() {
    let rt = fixture_runtime();
    for q in all_gold_queries() {
        let elided = rt
            .encode_elided(&q.sql)
            .unwrap_or_else(|e| panic!("encode_elided {}: {e}", q.id));
        let expanded = rt
            .expand_elided_tokens(&elided)
            .unwrap_or_else(|e| panic!("expand {}: {e}", q.id));
        let rendered = rt
            .decode(&expanded)
            .unwrap_or_else(|e| panic!("decode {}: {e}", q.id));
        let again = rt
            .encode(&rendered)
            .unwrap_or_else(|e| panic!("re-encode {}: {e}", q.id));
        assert_eq!(again, expanded, "fixpoint for {}", q.id);

        // strict_decode_complete_elided is the one-shot path.
        let via_strict = rt
            .strict_decode_complete_elided(&elided)
            .unwrap_or_else(|e| panic!("strict elided {}: {e}", q.id));
        assert_eq!(via_strict, rendered, "strict elided for {}", q.id);
    }
}

#[test]
fn elided_incremental_decode_replays_elided_stream() {
    let rt = fixture_runtime();
    let sql = gold_sql("join_two_tables");
    let elided = rt.encode_elided(&sql).expect("encode_elided");
    let mut state = rt.begin_elided_tct_decode().expect("begin");
    assert!(!state.is_complete().expect("empty complete"));
    for (i, &token) in elided.iter().enumerate() {
        let allowed = state.allowed_token_ids().expect("mask");
        assert!(
            allowed.len() != 1,
            "elided position {i} must not be a singleton mask (got {allowed:?})"
        );
        assert!(
            allowed.contains(&token),
            "elided token[{i}]={token} must be in free-choice mask"
        );
        state.apply(token).expect("apply elided");
    }
    assert!(state.is_complete().expect("complete"));
    let rendered = state.render_sql().expect("render");
    assert_eq!(
        rendered,
        rt.strict_decode_complete_elided(&elided).expect("strict")
    );
}

fn singleton_join_catalog() -> SqlCatalog {
    // Table `a` has exactly one column, so a structural join-ON right side
    // restricted to earlier tables is a singleton menu.
    let tables_json = br#"[{
        "db_id": "joinon_singleton",
        "table_names_original": ["a", "b"],
        "table_names": ["a", "b"],
        "column_names_original": [[-1, "*"], [0, "aid"], [1, "bid"], [1, "x"]],
        "column_names": [[-1, "*"], [0, "aid"], [1, "bid"], [1, "x"]],
        "column_types": ["text", "number", "number", "number"],
        "primary_keys": [],
        "foreign_keys": []
    }]"#;
    SqlCatalog::parse_tables_json_unpinned(tables_json, "joinon_singleton")
        .expect("singleton join catalog parses")
}

#[test]
fn structural_join_on_singleton_right_menu_is_forced_and_elided() {
    // The validity-by-construction payoff: when the allowed pair slice is a
    // single ordinal, the right-side join-key position becomes mask-forced and
    // VANISHES from the canonical elided stream — a wrong join key is not
    // merely rejected, it is unwritable.
    let sql = "SELECT b.x FROM a JOIN b ON b.bid = a.aid";
    let off = SqlSchemaLinkingRuntime::from_catalog(singleton_join_catalog()).expect("off runtime");
    let structural = SqlSchemaLinkingRuntime::from_catalog_with_join_policy(
        singleton_join_catalog(),
        SqlJoinOnPolicy::Structural,
    )
    .expect("structural runtime");

    let expanded_off = off.encode(sql).expect("encode off");
    let expanded_structural = structural.encode(sql).expect("encode structural");
    assert_eq!(
        expanded_off, expanded_structural,
        "expanded gold streams are policy-invariant"
    );

    let elided_off = off.encode_elided(sql).expect("elide off");
    let elided_structural = structural.encode_elided(sql).expect("elide structural");
    assert_eq!(
        elided_structural.len() + 1,
        elided_off.len(),
        "structural policy must elide exactly the forced right-side join key"
    );

    // The elided stream stays losslessly invertible under the same policy.
    let reexpanded = structural
        .expand_elided_tokens(&elided_structural)
        .expect("expand structural elided");
    assert_eq!(reexpanded, expanded_structural);

    // Policy mixing fails closed: the off-policy stream spells out the token
    // the structural walker derives as forced.
    let err = structural
        .expand_elided_tokens(&elided_off)
        .expect_err("off-policy elided stream must be rejected under structural");
    let message = format!("{err:?}");
    assert!(
        message.contains("forced_token_in_elided_stream") || message.contains("forced"),
        "unexpected policy-mixing error: {message}"
    );

    // Capability + contract surfaces record the policy.
    assert_eq!(structural.join_on_policy(), SqlJoinOnPolicy::Structural);
    assert_eq!(
        structural.stream_contract(),
        crate::syntax_token_layout_diff::RAW_SYNTAX_STREAM_CONTRACT_ELIDED_V1_JOINON_STRUCTURAL
    );
    assert_eq!(
        off.stream_contract(),
        crate::syntax_token_layout_diff::RAW_SYNTAX_STREAM_CONTRACT_ELIDED_V1
    );
    assert_eq!(structural.capability_report().join_on_policy, "structural");
}

fn fixture_runtime_visit_all() -> SqlSchemaLinkingRuntime {
    let catalog = load_fixture_catalog().expect("fixture catalog");
    SqlSchemaLinkingRuntime::from_catalog_with_sql_policies(
        catalog,
        SqlJoinOnPolicy::Off,
        crate::sql_schema_linking::menu_binding::SqlColumnAddressing::Flat,
        true,
    )
    .expect("visit-all fixture runtime")
}

fn elided_free_sites(rt: &SqlSchemaLinkingRuntime, sql: &str) -> Vec<String> {
    let elided = rt.encode_elided(sql).expect("encode_elided");
    let mut state = rt.begin_elided_tct_decode().expect("begin elided");
    let mut sites = Vec::new();
    for token in elided {
        let site = state.active_site().expect("active_site");
        let rule = site.active_rule.unwrap_or_default();
        let field = site.active_field.unwrap_or_else(|| "_".to_string());
        sites.push(format!("{rule}.{field}"));
        state.apply(token).expect("apply elided");
    }
    sites
}

#[test]
fn visit_all_fields_elides_table_ref_skip_vs_visit_and_keeps_alias() {
    // Rank-1 first unit: merge table_ref._ (skip remaining optionals vs
    // field_start) into the subsequent alias decision by forcing visitation.
    // Alias present vs missing stays a free 2-way choice. Stream-contract
    // break; historical L6 streams must not decode under this policy.
    let sql = gold_sql("select_one_table");
    let historical = fixture_runtime();
    let visit_all = fixture_runtime_visit_all();

    let historical_sites = elided_free_sites(&historical, &sql);
    let visit_all_sites = elided_free_sites(&visit_all, &sql);
    assert!(
        historical_sites.iter().any(|s| s == "table_ref._"),
        "historical free stream must include table_ref._ skip-vs-visit: {historical_sites:?}"
    );
    assert!(
        historical_sites.iter().any(|s| s == "table_ref.alias"),
        "historical free stream must include table_ref.alias: {historical_sites:?}"
    );
    assert!(
        !visit_all_sites.iter().any(|s| s == "table_ref._"),
        "visit-all must elide table_ref._: {visit_all_sites:?}"
    );
    assert!(
        !visit_all_sites.iter().any(|s| s == "from_clause._"),
        "visit-all must also elide from_clause._ (trailing optional joins): {visit_all_sites:?}"
    );
    assert!(
        visit_all_sites.iter().any(|s| s == "table_ref.alias"),
        "alias present vs missing must remain a free decision: {visit_all_sites:?}"
    );

    let historical_elided = historical.encode_elided(&sql).expect("hist elide");
    let visit_all_elided = visit_all.encode_elided(&sql).expect("visit-all elide");
    assert!(
        visit_all_elided.len() < historical_elided.len(),
        "visit-all elided stream must be shorter (hist {}, visit-all {})",
        historical_elided.len(),
        visit_all_elided.len()
    );

    let rendered = visit_all
        .strict_decode_complete_elided(&visit_all_elided)
        .expect("visit-all strict decode");
    let again = visit_all.encode_elided(&rendered).expect("re-elide");
    assert_eq!(again, visit_all_elided);

    // Policy mixing fails closed: historical free skip-vs-visit tokens are
    // forced under visit-all.
    let err = visit_all
        .expand_elided_tokens(&historical_elided)
        .expect_err("historical elided stream must be rejected under visit-all");
    let message = format!("{err:?}");
    assert!(
        message.contains("forced_token_in_elided_stream") || message.contains("forced"),
        "unexpected policy-mixing error: {message}"
    );

    assert!(visit_all.visit_all_fields());
    assert!(!historical.visit_all_fields());
    assert_eq!(
        visit_all.stream_contract(),
        crate::syntax_token_layout_diff::RAW_SYNTAX_STREAM_CONTRACT_ELIDED_V1_VISIT_ALL_FIELDS
    );
    assert_eq!(
        historical.stream_contract(),
        crate::syntax_token_layout_diff::RAW_SYNTAX_STREAM_CONTRACT_ELIDED_V1
    );
    assert!(visit_all.capability_report().visit_all_fields);
}

#[test]
fn visit_all_folds_select_list_distinct_into_one_alt() {
    // Rank-1 second unit: SELECT DISTINCT is a select_list alternative, not
    // a second free decision after choosing select_items.
    let sql = gold_sql("select_distinct");
    let historical = fixture_runtime();
    let visit_all = fixture_runtime_visit_all();
    let hist_sites = elided_free_sites(&historical, &sql);
    let fold_sites = elided_free_sites(&visit_all, &sql);
    assert!(
        hist_sites.iter().any(|s| s == "select_items.distinct"),
        "historical must keep select_items.distinct: {hist_sites:?}"
    );
    assert!(
        !fold_sites.iter().any(|s| s == "select_items.distinct"),
        "visit-all fold must elide select_items.distinct: {fold_sites:?}"
    );
    let hist = historical.encode_elided(&sql).expect("hist");
    let folded = visit_all.encode_elided(&sql).expect("fold");
    assert!(
        folded.len() < hist.len(),
        "folded DISTINCT stream must be shorter (hist {}, fold {})",
        hist.len(),
        folded.len()
    );
    let rendered = visit_all
        .strict_decode_complete_elided(&folded)
        .expect("decode fold");
    assert!(
        rendered.to_ascii_uppercase().contains("DISTINCT"),
        "DISTINCT must survive fold render: {rendered}"
    );
    assert_eq!(visit_all.encode_elided(&rendered).expect("re"), folded);
}

#[test]
fn visit_all_fold_encodes_nested_select_without_distinct_field() {
    // Nested selects must inherit the fold. Historical subquery lowering
    // still emits select_items.distinct, which the folded grammar rejects
    // (`encode_unknown_field`). This is the rematerialize-540 hole.
    let visit_all = fixture_runtime_visit_all();
    let cases = [
        gold_sql("scalar_subquery"),
        "SELECT name FROM customers WHERE customer_id IN (SELECT DISTINCT customer_id FROM orders)"
            .to_string(),
    ];
    for sql in cases {
        let folded = visit_all
            .encode_elided(&sql)
            .unwrap_or_else(|e| panic!("visit-all nested encode failed for {sql}: {e}"));
        let rendered = visit_all
            .strict_decode_complete_elided(&folded)
            .unwrap_or_else(|e| panic!("decode nested fold for {sql}: {e}"));
        assert_eq!(
            visit_all.encode_elided(&rendered).expect("re"),
            folded,
            "nested fold fixpoint miss for {sql} rendered={rendered}"
        );
        let sites = elided_free_sites(&visit_all, &sql);
        assert!(
            !sites.iter().any(|s| s == "select_items.distinct"),
            "nested fold must not expose select_items.distinct: {sites:?} sql={sql}"
        );
    }
}

#[test]
fn visit_all_moves_having_under_group_by() {
    // Rank-1 third unit: HAVING requires GROUP BY (SQL grammar fact), so
    // visit-all drops select_statement.having. Queries without GROUP BY
    // no longer pay a having skip; HAVING remains a 2-way on
    // group_by_clause when GROUP BY is present.
    let historical = fixture_runtime();
    let visit_all = fixture_runtime_visit_all();

    let no_group = gold_sql("select_one_table");
    let hist_no = elided_free_sites(&historical, &no_group);
    let fold_no = elided_free_sites(&visit_all, &no_group);
    assert!(
        hist_no.iter().any(|s| s == "select_statement.having"),
        "historical must keep select_statement.having skip: {hist_no:?}"
    );
    assert!(
        !fold_no.iter().any(|s| s == "select_statement.having"),
        "visit-all must not expose select_statement.having without GROUP BY: {fold_no:?}"
    );
    assert!(
        !fold_no.iter().any(|s| s == "group_by_clause.having"),
        "no GROUP BY means no group_by_clause.having: {fold_no:?}"
    );

    let group_only = gold_sql("group_order_limit");
    let fold_g = elided_free_sites(&visit_all, &group_only);
    assert!(
        !fold_g.iter().any(|s| s == "select_statement.having"),
        "GROUP BY without HAVING must not use select_statement.having: {fold_g:?}"
    );
    assert!(
        fold_g.iter().any(|s| s == "group_by_clause.having"),
        "GROUP BY without HAVING still decides group_by_clause.having: {fold_g:?}"
    );

    let with_having = gold_sql("where_group_having_order_limit");
    let hist_h = elided_free_sites(&historical, &with_having);
    let fold_h = elided_free_sites(&visit_all, &with_having);
    assert!(
        hist_h.iter().any(|s| s == "select_statement.having"),
        "historical HAVING site is select_statement.having: {hist_h:?}"
    );
    assert!(
        !fold_h.iter().any(|s| s == "select_statement.having"),
        "visit-all HAVING must not stay on select_statement: {fold_h:?}"
    );
    assert!(
        fold_h.iter().any(|s| s == "group_by_clause.having"),
        "visit-all HAVING lives on group_by_clause: {fold_h:?}"
    );
    let fold_tok = visit_all.encode_elided(&with_having).expect("fold");
    let rendered = visit_all
        .strict_decode_complete_elided(&fold_tok)
        .expect("decode having fold");
    assert!(
        rendered.to_ascii_uppercase().contains("HAVING"),
        "HAVING must survive fold render: {rendered}"
    );
    assert_eq!(visit_all.encode_elided(&rendered).expect("re"), fold_tok);
    let no_group_hist = historical.encode_elided(&no_group).expect("hist ng");
    let no_group_fold = visit_all.encode_elided(&no_group).expect("fold ng");
    assert!(
        no_group_fold.len() < no_group_hist.len(),
        "dropping having-skip must shorten no-GROUP-BY streams (hist {}, fold {})",
        no_group_hist.len(),
        no_group_fold.len()
    );
}

#[test]
fn visit_all_folds_limit_into_select_list() {
    // Rank-1 fourth unit: LIMIT presence is sequential with select_list (P6).
    // Visit-all folds it into a 6-way select_list alt. LIMIT value stays a
    // child of limited_*. Absent-LIMIT queries no longer pay a skip.
    let historical = fixture_runtime();
    let visit_all = fixture_runtime_visit_all();

    let no_limit = gold_sql("select_one_table");
    let hist_no = elided_free_sites(&historical, &no_limit);
    let fold_no = elided_free_sites(&visit_all, &no_limit);
    assert!(
        hist_no.iter().any(|s| s == "select_statement.limit"),
        "historical must keep select_statement.limit: {hist_no:?}"
    );
    assert!(
        !fold_no.iter().any(|s| s == "select_statement.limit"),
        "visit-all must not expose select_statement.limit: {fold_no:?}"
    );
    let hist_tok = historical.encode_elided(&no_limit).expect("hist");
    let fold_tok = visit_all.encode_elided(&no_limit).expect("fold");
    assert!(
        fold_tok.len() < hist_tok.len(),
        "dropping limit-skip must shorten no-LIMIT streams (hist {}, fold {})",
        hist_tok.len(),
        fold_tok.len()
    );

    let with_limit = gold_sql("group_order_limit");
    let fold_sites = elided_free_sites(&visit_all, &with_limit);
    assert!(
        !fold_sites.iter().any(|s| s == "select_statement.limit"),
        "LIMIT present must not use select_statement.limit: {fold_sites:?}"
    );
    assert!(
        fold_sites.iter().any(|s| s == "limit_clause.value"),
        "LIMIT value must remain a free decision: {fold_sites:?}"
    );
    let folded = visit_all.encode_elided(&with_limit).expect("fold limit");
    let rendered = visit_all
        .strict_decode_complete_elided(&folded)
        .expect("decode limit fold");
    assert!(
        rendered.to_ascii_uppercase().contains("LIMIT"),
        "LIMIT must survive fold render: {rendered}"
    );
    assert_eq!(visit_all.encode_elided(&rendered).expect("re"), folded);
}

#[test]
fn visit_all_folds_order_by_into_select_list() {
    // Rank-1 fifth unit: ORDER BY is sequential with select_list (P6)
    // after LIMIT is folded in. Presence becomes ordered_* alts; items
    // stay children. Absent-ORDER-BY queries no longer pay a skip.
    let historical = fixture_runtime();
    let visit_all = fixture_runtime_visit_all();

    let no_order = gold_sql("select_one_table");
    let hist_no = elided_free_sites(&historical, &no_order);
    let fold_no = elided_free_sites(&visit_all, &no_order);
    assert!(
        hist_no.iter().any(|s| s == "select_statement.order_by"),
        "historical must keep select_statement.order_by: {hist_no:?}"
    );
    assert!(
        !fold_no.iter().any(|s| s == "select_statement.order_by"),
        "visit-all must not expose select_statement.order_by: {fold_no:?}"
    );
    let hist_tok = historical.encode_elided(&no_order).expect("hist");
    let fold_tok = visit_all.encode_elided(&no_order).expect("fold");
    assert!(
        fold_tok.len() < hist_tok.len(),
        "dropping order_by-skip must shorten no-ORDER-BY streams (hist {}, fold {})",
        hist_tok.len(),
        fold_tok.len()
    );

    let with_order = gold_sql("group_order_limit");
    let fold_sites = elided_free_sites(&visit_all, &with_order);
    assert!(
        !fold_sites.iter().any(|s| s == "select_statement.order_by"),
        "ORDER BY present must not use select_statement.order_by: {fold_sites:?}"
    );
    assert!(
        fold_sites
            .iter()
            .any(|s| s == "order_by_clause.items" || s == "order_item.column"),
        "ORDER BY payload must remain: {fold_sites:?}"
    );
    let folded = visit_all.encode_elided(&with_order).expect("fold order");
    let rendered = visit_all
        .strict_decode_complete_elided(&folded)
        .expect("decode order fold");
    assert!(
        rendered.to_ascii_uppercase().contains("ORDER BY"),
        "ORDER BY must survive fold render: {rendered}"
    );
    assert!(
        rendered.to_ascii_uppercase().contains("LIMIT"),
        "LIMIT must survive combined fold render: {rendered}"
    );
    assert_eq!(visit_all.encode_elided(&rendered).expect("re"), folded);
}
