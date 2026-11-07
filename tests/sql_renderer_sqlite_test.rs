//! External dialect witnesses: the SQL parser's own fixpoint cannot prove
//! SQLite syntax or denotation. These tests require Python's stdlib sqlite3.

use serde_json::json;
use std::io::Write;
use std::process::{Command, Stdio};
use tct::bpe::{BpeMerge, BpeMergeTable, BpeMetadata};
use tct::sql_schema_linking::elided_bpe_mask::{
    bpe_codec_for_merges, bpe_token_is_admissible, expand_bpe_token_to_elided_base,
    try_apply_bpe_token,
};
use tct::sql_schema_linking::layout::SQL_SHARED_PAYLOAD_START;
use tct::sql_schema_linking::runtime::{SqlActiveMenuBindings, SqlElidedTctDecodeState};
use tct::sql_schema_linking::{
    Expression, SqlCatalog, SqlColumnAddressing, SqlJoinOnPolicy, SqlSchemaLinkingCodec,
    SqlSchemaLinkingRuntime, render_select,
};

fn catalog(table: &str, column: &str) -> SqlCatalog {
    let bytes = serde_json::to_vec(&json!([{
        "db_id": "sqlite_witness",
        "table_names": [table], "table_names_original": [table],
        "column_names": [[-1, "*"], [0, column]],
        "column_names_original": [[-1, "*"], [0, column]],
        "column_types": ["text", "text"], "primary_keys": [], "foreign_keys": []
    }]))
    .unwrap();
    SqlCatalog::parse_tables_json_unpinned(&bytes, "sqlite_witness").unwrap()
}

fn quoted(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}

fn sqlite_rows(table: &str, column: &str, literal: &str, queries: &[String]) {
    let input = json!({"table": table, "column": column, "literal": literal, "queries": queries});
    let mut child = Command::new("python3")
        .args([
            "-c",
            r#"
import json, sqlite3, sys
case = json.load(sys.stdin)
quote = lambda s: '"' + s.replace('"', '""') + '"'
db = sqlite3.connect(':memory:')
db.execute('CREATE TABLE ' + quote(case['table']) + ' (' + quote(case['column']) + ' TEXT)')
db.execute('INSERT INTO ' + quote(case['table']) + ' VALUES (?)', (case['literal'],))
for query in case['queries']:
    try:
        rows = db.execute(query).fetchall()
    except Exception as exc:
        raise AssertionError((sqlite3.sqlite_version, query, str(exc))) from exc
    assert rows == [(case['literal'],)], (sqlite3.sqlite_version, query, rows)
"#,
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("python3 with stdlib sqlite3 is required for SQL dialect tests");
    child
        .stdin
        .take()
        .unwrap()
        .write_all(input.to_string().as_bytes())
        .unwrap();
    let result = child.wait_with_output().unwrap();
    assert!(
        result.status.success(),
        "SQLite witness failed: {}",
        String::from_utf8_lossy(&result.stderr)
    );
}

fn identifier_witness(table: &str, column: &str) {
    let literal = "O'Brien \"quoted\" λ\nline";
    let sql = format!(
        "SELECT {} FROM {} WHERE {} = '{}'",
        quoted(column),
        quoted(table),
        quoted(column),
        literal.replace('\'', "''")
    );
    let mut queries = vec![sql.clone()];
    for visit_all in [false, true] {
        for addressing in [SqlColumnAddressing::Flat, SqlColumnAddressing::Factored] {
            let rt = SqlSchemaLinkingRuntime::from_catalog_with_sql_policies(
                catalog(table, column),
                SqlJoinOnPolicy::Off,
                addressing,
                visit_all,
            )
            .unwrap();
            let raw = rt.encode(&sql).unwrap();
            let rendered = rt.decode(&raw).unwrap();
            assert_eq!(rt.encode(&rendered).unwrap(), raw);
            queries.push(rendered);
            let mut state = rt.begin_tct_decode();
            for token in &raw {
                assert!(state.allowed_token_ids().unwrap().contains(token));
                state.apply(*token).unwrap();
            }
            queries.push(state.render_sql().unwrap());
            let elided = rt.encode_elided(&sql).unwrap();
            assert_eq!(rt.expand_elided_tokens(&elided).unwrap(), raw);
            let mut state = rt.begin_elided_tct_decode().unwrap();
            for token in elided {
                assert!(state.allowed_token_ids().unwrap().contains(&token));
                state.apply(token).unwrap();
            }
            queries.push(state.render_sql().unwrap());
        }
    }
    sqlite_rows(table, column, literal, &queries);
}

#[test]
fn sqlite_null_column_preserves_denotation() {
    identifier_witness("t", "NULL");
}

#[test]
fn sqlite_keyword_identifiers_preserve_syntax() {
    for word in ["CASE", "INDEX", "CHECK", "CURRENT_DATE", "TRUE"] {
        identifier_witness("t", word);
        identifier_witness(word, "value");
    }
}

#[test]
fn sqlite_quoted_identifiers_and_literals_preserve_bytes() {
    for name in ["a\"b", "two words", "λ", "a.b", "semi;--comment"] {
        identifier_witness(name, name);
    }
}

fn selected_qualified_column(menu: SqlActiveMenuBindings, token: u32) -> Option<serde_json::Value> {
    let selected = menu
        .bindings
        .iter()
        .find(|binding| binding.token_id == token)?;
    let column = selected.column_ordinal?;
    Some(json!([
        selected.table_ordinal,
        column,
        selected.table_name,
        selected.column_name
    ]))
}

/// The qualified menu and renderer must agree even when catalog order, join
/// order, duplicate keyword names, and escaped SQL spellings all differ.
#[test]
fn sqlite_qualified_bindings_and_quoted_join_preserve_selected_columns() {
    let bytes = serde_json::to_vec(&json!([{
        "db_id": "qualified_quoted_join",
        "table_names": ["bravo display", "unused", "alpha display"],
        "table_names_original": ["B\"R", "Unused", "A\"L"],
        "column_names": [[-1, "*"], [0, "bravo value"], [0, "bravo id"],
                         [1, "unused"], [2, "alpha id"], [2, "alpha value"]],
        "column_names_original": [[-1, "*"], [0, "NULL"], [0, "Id"],
                                  [1, "unused"], [2, "Id"], [2, "NULL"]],
        "column_types": ["text", "text", "number", "text", "number", "text"],
        "primary_keys": [], "foreign_keys": []
    }]))
    .unwrap();
    let source = r#"SELECT "A""L"."NULL", "B""R"."NULL" FROM "A""L" JOIN "B""R" ON "A""L"."Id" = "B""R"."Id""#;
    // From-first TCT emits the ON operands before the SELECT operands. These
    // addresses come from the explicit catalog above, not either resolver.
    // The two NULL columns have different catalog-local column ordinals.
    let expected = vec![
        json!([2, 0, "A\"L", "Id"]),
        json!([0, 1, "B\"R", "Id"]),
        json!([2, 1, "A\"L", "NULL"]),
        json!([0, 0, "B\"R", "NULL"]),
    ];
    let mut queries = vec![source.to_owned()];
    for (join_policy, addressing) in [
        (SqlJoinOnPolicy::Off, SqlColumnAddressing::Flat),
        (SqlJoinOnPolicy::Off, SqlColumnAddressing::Factored),
        (SqlJoinOnPolicy::Structural, SqlColumnAddressing::Flat),
        (SqlJoinOnPolicy::Structural, SqlColumnAddressing::Factored),
    ] {
        let rt = SqlSchemaLinkingRuntime::from_catalog_with_policies(
            SqlCatalog::parse_tables_json_unpinned(&bytes, "qualified_quoted_join").unwrap(),
            join_policy,
            addressing,
        )
        .unwrap();
        let raw = rt.encode(source).unwrap();
        let mut state = rt.begin_tct_decode();
        let mut selected = Vec::new();
        for &token in &raw {
            // Snapshot BEFORE apply: after apply the API describes a new site.
            if let Some(binding) =
                selected_qualified_column(state.active_menu_bindings().unwrap(), token)
            {
                selected.push(binding);
            }
            state.apply(token).unwrap();
        }
        assert_eq!(selected, expected, "raw {join_policy:?} {addressing:?}");
        assert!(state.is_complete().unwrap());
        let rendered = state.render_sql().unwrap();
        assert_eq!(rt.encode(&rendered).unwrap(), raw);
        queries.push(rendered);
        queries.push(rt.decode(&raw).unwrap());

        let elided = rt.encode_elided(source).unwrap();
        let mut state = rt.begin_elided_tct_decode().unwrap();
        let mut selected = Vec::new();
        for &token in &elided {
            if let Some(binding) =
                selected_qualified_column(state.active_menu_bindings().unwrap(), token)
            {
                selected.push(binding);
            }
            state.apply(token).unwrap();
        }
        assert_eq!(selected, expected, "elided {join_policy:?} {addressing:?}");
        assert!(state.is_complete().unwrap());
        assert_eq!(state.expanded_tokens(), raw);
        let rendered = state.render_sql().unwrap();
        assert_eq!(rt.encode_elided(&rendered).unwrap(), elided);
        queries.push(rendered);
    }

    // Independent external oracle. Different values identify both duplicate
    // NULL columns; unmatched rows catch a lost or altered ON predicate.
    let mut child = Command::new("python3")
        .args([
            "-c",
            r#"
import json, sqlite3, sys
queries = json.load(sys.stdin)
db = sqlite3.connect(':memory:')
db.execute('CREATE TABLE "A""L" ("Id" INTEGER, "NULL" TEXT)')
db.execute('CREATE TABLE "B""R" ("NULL" TEXT, "Id" INTEGER)')
db.executemany('INSERT INTO "A""L" VALUES (?, ?)',
               [(1, 'alpha-one'), (2, 'alpha-two'), (7, 'unmatched-alpha')])
db.executemany('INSERT INTO "B""R" VALUES (?, ?)',
               [('bravo-two', 2), ('bravo-one', 1), ('unmatched-bravo', 9)])
expected = [('alpha-one', 'bravo-one'), ('alpha-two', 'bravo-two')]
for query in queries:
    try:
        rows = sorted(db.execute(query).fetchall())
    except Exception as exc:
        raise AssertionError((sqlite3.sqlite_version, query, str(exc))) from exc
    assert rows == expected, (sqlite3.sqlite_version, query, rows)
"#,
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("python3 with stdlib sqlite3 is required for the combined runtime witness");
    child
        .stdin
        .take()
        .unwrap()
        .write_all(serde_json::to_string(&queries).unwrap().as_bytes())
        .unwrap();
    let result = child.wait_with_output().unwrap();
    assert!(
        result.status.success(),
        "combined qualified-menu/SQLite witness failed: {}",
        String::from_utf8_lossy(&result.stderr)
    );
}

#[test]
fn nul_literal_source_and_ast_are_rejected() {
    let rt = SqlSchemaLinkingRuntime::from_catalog(catalog("t", "value")).unwrap();
    let sql = "SELECT value FROM t WHERE value = '\0'";
    assert!(
        rt.encode(sql).is_err(),
        "raw encoder admitted a NUL literal"
    );
    assert!(
        rt.encode_elided(sql).is_err(),
        "elided encoder admitted a NUL literal"
    );
    let codec = SqlSchemaLinkingCodec::for_catalog(catalog("t", "value")).unwrap();
    let mut ast = codec
        .parse("SELECT value FROM t WHERE value = 'x'")
        .unwrap();
    if let Some(Expression::Comparison { right, .. }) = ast.where_clause.as_mut() {
        **right = Expression::StringLiteral("\0".into());
    } else {
        panic!("expected comparison witness");
    }
    assert!(
        codec.encode(&ast).is_err(),
        "AST encoder admitted a NUL literal"
    );
    assert!(
        render_select(&ast, codec.catalog()).is_err(),
        "renderer admitted a NUL literal"
    );
}

#[test]
fn nul_literal_raw_mask_apply_and_batch_decode_reject_consistently() {
    let rt = SqlSchemaLinkingRuntime::from_catalog(catalog("t", "value")).unwrap();
    let raw = rt.encode("SELECT value FROM t WHERE value = 'x'").unwrap();
    let mut state = rt.begin_tct_decode();
    for (pos, token) in raw.iter().enumerate() {
        let site = state.active_site().unwrap();
        if site.active_rule.as_deref() == Some("string_literal")
            && site.active_field.as_deref() == Some("value")
            && *token == SQL_SHARED_PAYLOAD_START + u32::from(b'x')
        {
            let mut bad = raw.clone();
            bad[pos] = SQL_SHARED_PAYLOAD_START;
            assert!(rt.decode(&bad).is_err(), "batch decode admitted NUL");
            assert!(
                !state
                    .allowed_token_ids()
                    .unwrap()
                    .contains(&SQL_SHARED_PAYLOAD_START),
                "mask admitted NUL"
            );
            let before = state.tokens().to_vec();
            assert!(state.apply(SQL_SHARED_PAYLOAD_START).is_err());
            assert_eq!(state.tokens(), before);
            state.apply(*token).unwrap();
            return;
        }
        state.apply(*token).unwrap();
    }
    panic!("did not reach string literal");
}

#[test]
fn nul_literal_elided_mask_apply_and_expansion_reject_consistently() {
    let rt = SqlSchemaLinkingRuntime::from_catalog(catalog("t", "value")).unwrap();
    let elided = rt
        .encode_elided("SELECT value FROM t WHERE value = 'x'")
        .unwrap();
    let mut state = rt.begin_elided_tct_decode().unwrap();
    for (pos, token) in elided.iter().enumerate() {
        let site = state.active_site().unwrap();
        if site.active_rule.as_deref() == Some("string_literal")
            && site.active_field.as_deref() == Some("value")
            && *token == SQL_SHARED_PAYLOAD_START + u32::from(b'x')
        {
            let mut bad = elided.clone();
            bad[pos] = SQL_SHARED_PAYLOAD_START;
            assert!(
                rt.expand_elided_tokens(&bad).is_err(),
                "elided expansion admitted NUL"
            );
            assert!(
                !state
                    .allowed_token_ids()
                    .unwrap()
                    .contains(&SQL_SHARED_PAYLOAD_START)
            );
            let before = state.consumed_elided_tokens();
            assert!(state.apply(SQL_SHARED_PAYLOAD_START).is_err());
            assert_eq!(state.consumed_elided_tokens(), before);
            state.apply(*token).unwrap();
            return;
        }
        state.apply(*token).unwrap();
    }
    panic!("did not reach string literal");
}

fn assert_same_elided_state(actual: &SqlElidedTctDecodeState, expected: &SqlElidedTctDecodeState) {
    assert_eq!(actual.expanded_tokens(), expected.expanded_tokens());
    assert_eq!(
        actual.consumed_elided_tokens(),
        expected.consumed_elided_tokens()
    );
    assert_eq!(actual.initial_forced_run(), expected.initial_forced_run());
    assert_eq!(actual.last_forced_run(), expected.last_forced_run());
    assert_eq!(
        actual.active_site().unwrap(),
        expected.active_site().unwrap()
    );
    assert_eq!(
        actual.active_menu().unwrap(),
        expected.active_menu().unwrap()
    );
    assert_eq!(
        actual.allowed_token_ids().unwrap(),
        expected.allowed_token_ids().unwrap()
    );
    assert_eq!(
        actual.is_complete().unwrap(),
        expected.is_complete().unwrap()
    );
}

#[test]
fn nul_after_nonempty_literal_prefix_raw_rejects_without_advancing() {
    let rt = SqlSchemaLinkingRuntime::from_catalog(catalog("t", "value")).unwrap();
    let raw = rt.encode("SELECT value FROM t WHERE value = 'xy'").unwrap();
    let mut state = rt.begin_tct_decode();
    let mut literal_bytes = 0;
    let mut checked = false;
    for (pos, &token) in raw.iter().enumerate() {
        let site = state.active_site().unwrap();
        if site.active_rule.as_deref() == Some("string_literal")
            && site.active_field.as_deref() == Some("value")
        {
            if token == SQL_SHARED_PAYLOAD_START + u32::from(b'x') {
                literal_bytes += 1;
            }
            if token == SQL_SHARED_PAYLOAD_START + u32::from(b'y') {
                assert_eq!(
                    literal_bytes, 1,
                    "must already have accepted a literal byte"
                );
                let before = state.clone();
                assert!(
                    !state
                        .allowed_token_ids()
                        .unwrap()
                        .contains(&SQL_SHARED_PAYLOAD_START)
                );
                assert!(state.apply(SQL_SHARED_PAYLOAD_START).is_err());
                assert_eq!(state.tokens(), before.tokens());
                assert_eq!(state.consumed_tokens(), before.consumed_tokens());
                assert_eq!(state.applied_transitions(), before.applied_transitions());
                assert_eq!(state.active_site().unwrap(), before.active_site().unwrap());
                assert_eq!(state.active_menu().unwrap(), before.active_menu().unwrap());
                assert_eq!(
                    state.allowed_token_ids().unwrap(),
                    before.allowed_token_ids().unwrap()
                );
                assert_eq!(state.is_complete().unwrap(), before.is_complete().unwrap());
                let mut bad = raw.clone();
                bad[pos] = SQL_SHARED_PAYLOAD_START;
                assert!(rt.decode(&bad).is_err(), "full raw decode admitted x + NUL");
                checked = true;
            }
        }
        assert!(state.allowed_token_ids().unwrap().contains(&token));
        state.apply(token).unwrap();
    }
    assert!(checked, "did not probe after the accepted nonempty prefix");
    assert!(state.is_complete().unwrap());
    assert_eq!(state.tokens(), raw);
    assert_eq!(state.render_sql().unwrap(), rt.decode(&raw).unwrap());
}

#[test]
fn nul_after_nonempty_literal_prefix_elided_rejects_without_advancing() {
    let rt = SqlSchemaLinkingRuntime::from_catalog(catalog("t", "value")).unwrap();
    let elided = rt
        .encode_elided("SELECT value FROM t WHERE value = 'xy'")
        .unwrap();
    let mut state = rt.begin_elided_tct_decode().unwrap();
    let mut literal_bytes = 0;
    let mut checked = false;
    for (pos, &token) in elided.iter().enumerate() {
        let site = state.active_site().unwrap();
        if site.active_rule.as_deref() == Some("string_literal")
            && site.active_field.as_deref() == Some("value")
        {
            if token == SQL_SHARED_PAYLOAD_START + u32::from(b'x') {
                literal_bytes += 1;
            }
            if token == SQL_SHARED_PAYLOAD_START + u32::from(b'y') {
                assert_eq!(
                    literal_bytes, 1,
                    "must already have accepted a literal byte"
                );
                let before = state.clone();
                assert!(
                    !state
                        .allowed_token_ids()
                        .unwrap()
                        .contains(&SQL_SHARED_PAYLOAD_START)
                );
                assert!(state.apply(SQL_SHARED_PAYLOAD_START).is_err());
                assert_same_elided_state(&state, &before);
                let mut bad = elided.clone();
                bad[pos] = SQL_SHARED_PAYLOAD_START;
                assert!(
                    rt.expand_elided_tokens(&bad).is_err(),
                    "expansion admitted x + NUL"
                );
                assert!(
                    rt.strict_decode_complete_elided(&bad).is_err(),
                    "full elided decode admitted x + NUL"
                );
                checked = true;
            }
        }
        assert!(state.allowed_token_ids().unwrap().contains(&token));
        state.apply(token).unwrap();
    }
    assert!(checked, "did not probe after the accepted nonempty prefix");
    assert!(state.is_complete().unwrap());
    assert_eq!(
        state.expanded_tokens(),
        rt.expand_elided_tokens(&elided).unwrap()
    );
    assert_eq!(
        state.render_sql().unwrap(),
        rt.strict_decode_complete_elided(&elided).unwrap()
    );
}

#[test]
fn nul_inside_bpe_expansion_rejects_after_legal_byte_transactionally() {
    let rt = SqlSchemaLinkingRuntime::from_catalog(catalog("t", "value")).unwrap();
    let elided = rt
        .encode_elided("SELECT value FROM t WHERE value = 'xyz'")
        .unwrap();
    let y = SQL_SHARED_PAYLOAD_START + u32::from(b'y');
    let z = SQL_SHARED_PAYLOAD_START + u32::from(b'z');
    let bad_merge = rt.vocab_size();
    let good_merge = bad_merge + 1;
    // Hand-specified two-pair fixture, not a trained or installed merge artifact.
    let table = BpeMergeTable {
        schema: "sqlite-nul-literal-fixture".into(),
        base_vocab_size: rt.vocab_size(),
        merges: vec![
            BpeMerge {
                pair: [y, SQL_SHARED_PAYLOAD_START],
                new_token: bad_merge,
                frequency: 1,
            },
            BpeMerge {
                pair: [y, z],
                new_token: good_merge,
                frequency: 1,
            },
        ],
        metadata: BpeMetadata {
            trained_on_instances: 0,
            training_date: "synthetic-untrained-fixture".into(),
            max_merges: 2,
            dict_size: None,
        },
    };
    table.validate().unwrap();
    let (_, decoder, _) = bpe_codec_for_merges(&table, 2).unwrap();
    assert_eq!(
        expand_bpe_token_to_elided_base(&decoder, bad_merge).unwrap(),
        vec![y, SQL_SHARED_PAYLOAD_START]
    );
    assert_eq!(
        expand_bpe_token_to_elided_base(&decoder, good_merge).unwrap(),
        vec![y, z]
    );
    let mut state = rt.begin_elided_tct_decode().unwrap();
    for (pos, &token) in elided.iter().enumerate() {
        let site = state.active_site().unwrap();
        if site.active_rule.as_deref() == Some("string_literal")
            && site.active_field.as_deref() == Some("value")
            && token == y
        {
            assert_eq!(
                state.expanded_tokens().last(),
                Some(&(SQL_SHARED_PAYLOAD_START + u32::from(b'x')))
            );
            assert_eq!(elided.get(pos + 1), Some(&z));
            let before = state.clone();
            let mut base_probe = state.clone();
            assert!(base_probe.allowed_token_ids().unwrap().contains(&y));
            base_probe.apply(y).unwrap();
            assert!(
                !base_probe
                    .allowed_token_ids()
                    .unwrap()
                    .contains(&SQL_SHARED_PAYLOAD_START)
            );
            assert!(base_probe.apply(SQL_SHARED_PAYLOAD_START).is_err());
            assert!(!bpe_token_is_admissible(&state, bad_merge, &decoder).unwrap());
            assert_same_elided_state(&state, &before);
            assert!(
                try_apply_bpe_token(&state, bad_merge, &decoder)
                    .unwrap()
                    .is_none()
            );
            assert_same_elided_state(&state, &before);
            // The same first byte succeeds in a legal merge. Its returned state
            // must equal two base applies, and the original remains untouched.
            assert!(bpe_token_is_admissible(&state, good_merge, &decoder).unwrap());
            base_probe.apply(z).unwrap();
            let mut next = try_apply_bpe_token(&state, good_merge, &decoder)
                .unwrap()
                .unwrap();
            assert_same_elided_state(&next, &base_probe);
            assert_same_elided_state(&state, &before);
            for &suffix in &elided[pos + 2..] {
                next.apply(suffix).unwrap();
            }
            assert!(next.is_complete().unwrap());
            assert_eq!(
                next.render_sql().unwrap(),
                rt.strict_decode_complete_elided(&elided).unwrap()
            );
            let mut bad = elided.clone();
            bad.splice(
                pos..pos + 2,
                expand_bpe_token_to_elided_base(&decoder, bad_merge).unwrap(),
            );
            assert!(rt.strict_decode_complete_elided(&bad).is_err());
            return;
        }
        state.apply(token).unwrap();
    }
    panic!("did not reach a nonempty literal prefix for the BPE probe");
}

#[test]
fn shared_zero_numeric_and_menu_roles_remain_admissible_raw_and_elided() {
    // Two tables and two columns keep zero-valued menu decisions non-singleton
    // in elided streams. A one-table/one-column fixture would elide them away.
    let bytes = serde_json::to_vec(&json!([{
        "db_id": "zero_roles",
        "table_names": ["t", "u"], "table_names_original": ["t", "u"],
        "column_names": [[-1, "*"], [0, "value"], [0, "other"], [1, "value"], [1, "other"]],
        "column_names_original": [
            [-1, "*"], [0, "value"], [0, "other"], [1, "value"], [1, "other"]
        ],
        "column_types": ["text", "number", "number", "number", "number"],
        "primary_keys": [], "foreign_keys": []
    }]))
    .unwrap();
    let sql = "SELECT value FROM t WHERE value = 0 LIMIT 0";
    for addressing in [SqlColumnAddressing::Flat, SqlColumnAddressing::Factored] {
        let rt = SqlSchemaLinkingRuntime::from_catalog_with_sql_policies(
            SqlCatalog::parse_tables_json_unpinned(&bytes, "zero_roles").unwrap(),
            SqlJoinOnPolicy::Off,
            addressing,
            false,
        )
        .unwrap();
        let raw = rt.encode(sql).unwrap();
        let elided = rt.encode_elided(sql).unwrap();
        let mut state = rt.begin_tct_decode();
        let mut raw_roles = std::collections::BTreeSet::new();
        for &token in &raw {
            let site = state.active_site().unwrap();
            if token == SQL_SHARED_PAYLOAD_START {
                let menu = state.active_menu().unwrap();
                if menu.kind != "none" {
                    raw_roles.insert(menu.kind.to_string());
                } else {
                    assert_eq!(site.active_field.as_deref(), Some("value"));
                    let rule = site.active_rule.as_deref().unwrap();
                    assert!(matches!(rule, "integer_literal" | "limit_clause"));
                    raw_roles.insert(rule.to_string());
                }
            }
            assert!(
                state.allowed_token_ids().unwrap().contains(&token),
                "raw mask rejected {token} at {site:?}"
            );
            state.apply(token).unwrap();
        }
        assert!(raw_roles.contains("table"));
        assert!(raw_roles.contains("integer_literal"));
        assert!(raw_roles.contains("limit_clause"));
        let column_role = if addressing == SqlColumnAddressing::Flat {
            "column_flat"
        } else {
            "column"
        };
        assert!(raw_roles.contains(column_role));
        assert!(state.is_complete().unwrap());
        assert_eq!(state.tokens(), raw);
        let expected = rt.decode(&raw).unwrap();
        assert_eq!(state.render_sql().unwrap(), expected);
        assert_eq!(rt.encode(&expected).unwrap(), raw);

        let mut state = rt.begin_elided_tct_decode().unwrap();
        let mut elided_roles = std::collections::BTreeSet::new();
        for &token in &elided {
            let site = state.active_site().unwrap();
            if token == SQL_SHARED_PAYLOAD_START {
                let menu = state.active_menu().unwrap();
                if menu.kind != "none" {
                    elided_roles.insert(menu.kind.to_string());
                } else {
                    assert_eq!(site.active_field.as_deref(), Some("value"));
                    let rule = site.active_rule.as_deref().unwrap();
                    assert!(matches!(rule, "integer_literal" | "limit_clause"));
                    elided_roles.insert(rule.to_string());
                }
            }
            assert!(
                state.allowed_token_ids().unwrap().contains(&token),
                "elided mask rejected {token} at {site:?}"
            );
            state.apply(token).unwrap();
        }
        for role in ["table", "integer_literal", "limit_clause"] {
            assert!(
                elided_roles.contains(role),
                "missing free zero-valued {role}"
            );
        }
        assert!(elided_roles.contains(column_role));
        assert!(state.is_complete().unwrap());
        assert_eq!(state.expanded_tokens(), raw);
        assert_eq!(state.render_sql().unwrap(), expected);
        assert_eq!(rt.strict_decode_complete_elided(&elided).unwrap(), expected);
        assert_eq!(rt.encode_elided(&expected).unwrap(), elided);
    }
}
