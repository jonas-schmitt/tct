//! Qualified candidate identities use the live cursor, never ambiguous names.

use super::*;

const CATALOGS: &[u8] =
    include_bytes!("../../../tests/fixtures/sql_schema_linking/qualified_menu/tables.json");

fn runtime(db_id: &str, addressing: SqlColumnAddressing) -> SqlSchemaLinkingRuntime {
    SqlSchemaLinkingRuntime::from_catalog_json_bytes_with_policies(
        CATALOGS,
        &sha256_hex(CATALOGS),
        db_id,
        SqlJoinOnPolicy::Off,
        addressing,
    )
    .expect("qualified-menu fixture")
}

fn assert_bindings(rt: &SqlSchemaLinkingRuntime, menu: &SqlActiveMenuBindings, allowed: &[u32]) {
    assert_eq!(menu.bindings.len(), menu.menu.referents.len());
    if menu.menu.resolved {
        // The historical menu shares the resolver, so referent parity alone
        // cannot detect a candidate dropped from both APIs. Compare directly
        // with the complete native payload-digit mask at a resolved site.
        let expected: BTreeSet<_> = allowed
            .iter()
            .copied()
            .filter(|token| {
                (SQL_SHARED_PAYLOAD_START..SQL_SHARED_PAYLOAD_END_EXCLUSIVE).contains(token)
            })
            .collect();
        let actual: BTreeSet<_> = menu
            .bindings
            .iter()
            .map(|binding| binding.token_id)
            .collect();
        assert_eq!(actual, expected, "all admissible candidates need bindings");
        assert_eq!(actual.len(), menu.bindings.len(), "one binding per token");
    } else {
        assert!(menu.bindings.is_empty());
    }
    for (binding, referent) in menu.bindings.iter().zip(&menu.menu.referents) {
        assert_eq!(binding.ordinal, referent.ordinal);
        assert!(allowed.contains(&binding.token_id));
        assert!(binding.token_id < rt.pad_token());
        let table = &rt.codec().catalog().tables[binding.table_ordinal as usize];
        assert_eq!(binding.table_name, table.name_original);
        if menu.menu.kind != "table" {
            assert_ne!(binding.table_name, "Unused");
        }
        match binding.column_ordinal {
            Some(column) => {
                assert_eq!(
                    binding.column_name.as_deref(),
                    Some(table.columns[column as usize].name_original.as_str())
                );
                assert_eq!(binding.column_name.as_ref(), Some(&referent.name));
            }
            None => {
                assert!(binding.column_name.is_none());
                assert_eq!(binding.table_name, referent.name);
            }
        }
    }
}

#[test]
fn active_menu_bindings_duplicate_names_follow_catalog_and_join_order() {
    for db_id in ["qualified_menu", "qualified_menu_permuted"] {
        for addressing in [SqlColumnAddressing::Flat, SqlColumnAddressing::Factored] {
            let rt = runtime(db_id, addressing);
            for from in [
                "Alpha JOIN Beta ON Alpha.Id = Beta.Id",
                "Beta JOIN Alpha ON Alpha.Id = Beta.Id",
            ] {
                let sql = format!("SELECT Alpha.Name, Beta.Name FROM {from}");
                let tokens = rt.encode_elided(&sql).expect("encode");
                let mut state = rt.begin_elided_tct_decode().expect("begin");
                let mut chosen_columns = Vec::new();
                let mut kinds = BTreeSet::new();
                for token in tokens {
                    let allowed = state.allowed_token_ids().expect("mask");
                    let before = state.expanded_tokens().to_vec();
                    let menu = state.active_menu_bindings().expect("bindings");
                    assert_eq!(menu.menu, state.active_menu().expect("historical menu"));
                    assert_eq!(menu, state.active_menu_bindings().expect("repeat"));
                    assert_eq!(before, state.expanded_tokens());
                    assert_eq!(allowed, state.allowed_token_ids().expect("same mask"));
                    assert_bindings(&rt, &menu, &allowed);
                    kinds.insert(menu.menu.kind);
                    if let Some(chosen) = menu.bindings.iter().find(|b| b.token_id == token)
                        && let Some(column) = &chosen.column_name
                    {
                        // Identity is chosen using the current token only; the
                        // snapshot contains no oracle for a future selection.
                        chosen_columns.push((chosen.table_name.clone(), column.clone()));
                    }
                    state.apply(token).expect("apply");
                }
                assert_eq!(
                    chosen_columns,
                    [
                        ("Alpha", "Id"),
                        ("Beta", "Id"),
                        ("Alpha", "Name"),
                        ("Beta", "Name")
                    ]
                    .map(|(table, column)| (table.to_owned(), column.to_owned()))
                );
                assert!(kinds.contains("table"));
                if addressing == SqlColumnAddressing::Factored {
                    assert!(kinds.contains("column_table"));
                    assert!(kinds.contains("column"));
                } else {
                    assert!(kinds.contains("column_flat"));
                }
                assert!(state.is_complete().expect("complete"));
                assert!(
                    state
                        .active_menu_bindings()
                        .expect("EOF")
                        .bindings
                        .is_empty()
                );
            }
        }
    }
}

#[test]
fn active_menu_bindings_factored_parent_is_retained_after_forced_elision() {
    let rt = runtime("qualified_menu_permuted", SqlColumnAddressing::Factored);
    let sql = "SELECT Name FROM Alpha";
    let tokens = rt.encode_elided(sql).expect("elided encode");
    let mut state = rt.begin_elided_tct_decode().expect("begin");
    let mut saw_column = false;
    for token in tokens {
        let menu = state.active_menu_bindings().expect("bindings");
        assert_bindings(&rt, &menu, &state.allowed_token_ids().expect("mask"));
        assert_ne!(
            menu.menu.kind, "column_table",
            "parent digit must be forced"
        );
        if menu.menu.kind == "column" {
            saw_column = true;
            assert_eq!(menu.menu.digits_seen, 1);
            assert_eq!(menu.bindings.len(), 2);
            assert!(
                menu.bindings
                    .iter()
                    .all(|binding| { binding.table_ordinal == 2 && binding.table_name == "Alpha" })
            );
        }
        state.apply(token).expect("apply");
    }
    assert!(saw_column);

    // Expanded cursor can rest after a completed pair. Its partial value is
    // then a flat ordinal, so treating it as a parent table would be wrong.
    let mut expanded = rt.begin_tct_decode();
    let mut saw_completed_pair = false;
    for token in rt.encode(sql).expect("expanded encode") {
        let menu = expanded.active_menu_bindings().expect("expanded bindings");
        assert_bindings(&rt, &menu, &expanded.allowed_token_ids().expect("mask"));
        assert_eq!(menu.menu, expanded.active_menu().expect("historical menu"));
        if menu.menu.kind == "column" && menu.menu.digits_seen == 2 {
            saw_completed_pair = true;
            assert!(!menu.menu.resolved);
            assert!(menu.bindings.is_empty());
        }
        expanded.apply(token).expect("expanded apply");
    }
    assert!(saw_completed_pair);
    assert!(
        expanded
            .active_menu_bindings()
            .expect("EOF")
            .bindings
            .is_empty()
    );
}

/// Two tables each fit the factored 256-slot alphabet, while their joined
/// flat menu spans 300 columns and therefore needs two radix digits.
fn wide_catalog() -> SqlCatalog {
    let mut columns = vec![serde_json::json!([-1, "*"])];
    for table in 0..2 {
        for column in 0..150 {
            columns.push(serde_json::json!([table, format!("c{column}")]));
        }
    }
    let bytes = serde_json::to_vec(&serde_json::json!([{
        "db_id": "wide_qualified", "table_names": ["alpha", "beta"],
        "table_names_original": ["Alpha", "Beta"],
        "column_names": columns, "column_names_original": columns,
        "column_types": vec!["number"; 301], "primary_keys": [], "foreign_keys": []
    }]))
    .expect("fixture JSON");
    SqlCatalog::parse_tables_json_unpinned(&bytes, "wide_qualified").expect("wide catalog")
}

#[test]
fn active_menu_bindings_wide_flat_final_digit_uses_qualified_full_ordinal() {
    let rt = SqlSchemaLinkingRuntime::from_catalog(wide_catalog()).expect("flat runtime");
    let sql = "SELECT Beta.c149 FROM Alpha JOIN Beta ON Alpha.c0 = Beta.c0";
    let tokens = rt.encode(sql).expect("encode");
    let mut state = rt.begin_tct_decode();
    let mut leading = false;
    let mut completed = false;
    let mut selected = false;
    for token in tokens {
        let menu = state.active_menu_bindings().expect("bindings");
        if menu.menu.kind == "column_flat" {
            assert_eq!(menu.menu.width, 2);
            match menu.menu.digits_seen {
                0 => {
                    leading = true;
                    assert!(!menu.menu.resolved);
                    assert!(menu.bindings.is_empty());
                }
                1 => {
                    assert_bindings(&rt, &menu, &state.allowed_token_ids().expect("mask"));
                    if let Some(binding) = menu.bindings.iter().find(|b| b.ordinal == 299) {
                        assert_eq!(binding.token_id, SQL_SHARED_PAYLOAD_START + 43);
                        assert_eq!(binding.table_ordinal, 1);
                        assert_eq!(binding.column_ordinal, Some(149));
                        assert_eq!(binding.table_name, "Beta");
                        assert_eq!(binding.column_name.as_deref(), Some("c149"));
                        selected |= binding.token_id == token;
                    }
                }
                2 => {
                    completed = true;
                    assert!(!menu.menu.resolved);
                    assert!(menu.bindings.is_empty());
                }
                count => panic!("unexpected width {count}"),
            }
        }
        state.apply(token).expect("apply");
    }
    assert!(leading && completed && selected);
}

#[test]
fn active_menu_bindings_factored_wide_scope_uses_local_column_digit() {
    let rt = SqlSchemaLinkingRuntime::from_catalog_with_policies(
        wide_catalog(),
        SqlJoinOnPolicy::Off,
        SqlColumnAddressing::Factored,
    )
    .expect("factored runtime");
    let sql = "SELECT Beta.c149 FROM Alpha JOIN Beta ON Alpha.c0 = Beta.c0";
    let tokens = rt.encode_elided(sql).expect("encode");
    let mut state = rt.begin_elided_tct_decode().expect("begin");
    let mut selected = false;
    for token in tokens {
        let menu = state.active_menu_bindings().expect("bindings");
        assert_bindings(&rt, &menu, &state.allowed_token_ids().expect("mask"));
        if menu.menu.kind == "column" {
            assert_eq!(menu.menu.width, 2);
            assert_eq!(menu.menu.digits_seen, 1);
            let binding = &menu.bindings[149];
            assert_eq!(binding.ordinal, 149);
            assert_eq!(binding.column_ordinal, Some(149));
            assert_eq!(binding.token_id, SQL_SHARED_PAYLOAD_START + 149);
            selected |= binding.table_ordinal == 1 && binding.token_id == token;
        }
        state.apply(token).expect("apply");
    }
    assert!(selected);
}

#[test]
fn active_menu_bindings_wide_table_prefix_and_completed_site_are_unresolved() {
    // A synthetic cursor isolates the metadata contract without compiling a
    // 300-table schema merely to check the final-digit identity mapping.
    let mut catalog =
        SqlCatalog::parse_tables_json_unpinned(CATALOGS, "qualified_menu").expect("catalog");
    let template = catalog.tables[0].clone();
    catalog.tables = (0..300)
        .map(|ordinal| {
            let mut table = template.clone();
            table.table_index = ordinal;
            table.name_original = format!("T{ordinal}");
            table
        })
        .collect();
    let mut cursor = LiveMenuCursor {
        owner_rule: "table_ref".to_owned(),
        owner_field: "table".to_owned(),
        site_kind: MenuSiteKind::Fixed {
            cardinality: 300,
            pushes_scope: true,
        },
        scope_tables: Vec::new(),
        digits_seen: 0,
        menu_partial: 0,
        menu_cardinality: Some(300),
        menu_width: Some(2),
    };
    let leading = resolve_active_menu_bindings(
        &catalog,
        Some(&cursor),
        &[SQL_SHARED_PAYLOAD_START, SQL_SHARED_PAYLOAD_START + 1],
    );
    assert!(!leading.menu.resolved);
    assert!(leading.bindings.is_empty());
    cursor.digits_seen = 1;
    cursor.menu_partial = 1;
    let allowed: Vec<u32> = (0..44)
        .map(|digit| SQL_SHARED_PAYLOAD_START + digit)
        .collect();
    let final_digit = resolve_active_menu_bindings(&catalog, Some(&cursor), &allowed);
    assert!(final_digit.menu.resolved);
    assert_eq!(final_digit.bindings.len(), 44);
    for (digit, binding) in final_digit.bindings.iter().enumerate() {
        assert_eq!(binding.ordinal, 256 + digit as u32);
        assert_eq!(binding.table_ordinal, binding.ordinal);
        assert_eq!(binding.table_name, format!("T{}", binding.ordinal));
        assert_eq!(binding.token_id, SQL_SHARED_PAYLOAD_START + digit as u32);
        assert!(binding.column_ordinal.is_none());
        assert!(binding.column_name.is_none());
    }
    cursor.digits_seen = 2;
    cursor.menu_partial = 299;
    let completed = resolve_active_menu_bindings(&catalog, Some(&cursor), &[]);
    assert!(!completed.menu.resolved);
    assert!(completed.bindings.is_empty());
    assert!(
        resolve_active_menu_bindings(&catalog, None, &allowed)
            .bindings
            .is_empty()
    );
}
