//! SQL menu-site binding construction (T4 table sites; T5/T6 scope columns).
//!
//! Builds [`MenuSiteBindings`] keyed by `(owner_rule, owner_field)` for the
//! IntegerPayload menu leaves declared in the v0 GrammarIR (**P3**/**P4**).
//! Table sites push onto `scope_tables` on the select root; column sites
//! resolve cardinality from that live scope (**P5**).

use crate::sql_schema_linking::catalog::SqlCatalog;
use crate::sql_schema_linking::grammar::{
    COLUMN_MENU_OWNER_FIELD, COLUMN_MENU_OWNER_RULE, COMPARISON_LEFT_FIELD, COMPARISON_OP_FIELD,
    COMPARISON_RIGHT_FIELD, COMPARISON_RULE, EXPRESSION_RULE, JOIN_CLAUSE_RULE, JOIN_ON_FIELD,
    OP_EQ_RULE, SELECT_STATEMENT_RULE, TABLE_MENU_OWNER_FIELD, TABLE_MENU_OWNER_RULE,
};
use crate::sql_schema_linking::layout::SQL_MENU_BAND_WIDTH;
use crate::syntax_state_machine::{JoinOnConstraint, MenuSiteBindings};

/// Join-ON masking policy for SQL menu bindings.
///
/// `Off` keeps historical behavior (both ON column sides admit every in-scope
/// column). `Structural` binds the engine-level [`JoinOnConstraint`]: equality
/// join keys must pair one column of the just-joined table with one column of
/// an earlier in-scope table. Gold-compatible on the campaign corpus at
/// 6,928/6,930 (see `the downstream training repository` `docs/sql_evidence/a5_join_mask_feasibility_v1.json`).
/// `FkDeclared` is structural plus a unique-declared-FK shrink: when the two
/// tables in the ON pair have exactly one catalog FK, the right-hand menu
/// collapses to that FK column. Multiple/no FKs keep the structural slice.
/// Gold that uses a non-FK join key becomes unencodable under this policy
/// (stream-contract break). No type-compatibility mask.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SqlJoinOnPolicy {
    /// No join-ON constraint (historical streams; default).
    #[default]
    Off,
    /// Structural pair constraint on equality join keys.
    Structural,
    /// Structural pair plus unique declared-FK column shrink.
    FkDeclared,
}

impl SqlJoinOnPolicy {
    /// Stable identifier recorded in layout descriptors and capability reports.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Structural => "structural",
            Self::FkDeclared => "fk_declared",
        }
    }

    /// Parse a policy identifier (used by the runtime constructor surface).
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "off" => Some(Self::Off),
            "structural" => Some(Self::Structural),
            "fk_declared" => Some(Self::FkDeclared),
            _ => None,
        }
    }

    /// Whether this policy binds the structural join-ON constraint engine.
    pub fn binds_join_on_constraint(self) -> bool {
        matches!(self, Self::Structural | Self::FkDeclared)
    }
}

/// Column-reference addressing policy for SQL menu bindings (L4).
///
/// `Flat` is the historical encoding: one radix-coded ordinal into the
/// concatenation, in FROM/JOIN order, of every in-scope table's columns. The
/// same column therefore has a different code in different queries, and the
/// model must compute `sum(len(cols) for earlier in-scope tables) +
/// per_table_index` from a schema prompt that shows per-table indices.
///
/// `Factored` emits the same reference as `(catalog table index, column index
/// inside that table)` — two positional menu digits whose values are exactly
/// the two numbers the prompt already shows, so the target is a copy rather
/// than an arithmetic derivation. It also narrows the second digit's mask from
/// "every in-scope column" to "that table's columns".
///
/// Costs one extra token per column reference **whenever the table digit is a
/// free choice**; single-table scopes force the table digit, so it is elided
/// and those queries pay nothing. Changes canonical streams, so callers must
/// treat a policy flip as a stream-contract break (see
/// `RAW_SYNTAX_STREAM_CONTRACT_ELIDED_V1_FACTORED_COLUMNS`).
///
/// Motivation and measurement: `the downstream training repository`
/// `docs/sql_evidence/l4_factored_column_addressing_proposal_v1.md`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SqlColumnAddressing {
    /// Single scope-relative flat ordinal (historical streams; default).
    #[default]
    Flat,
    /// `(catalog table, column in that table)` digit pair.
    Factored,
}

impl SqlColumnAddressing {
    /// Stable identifier recorded in capability reports and corpus metadata.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Flat => "flat",
            Self::Factored => "factored",
        }
    }

    /// Parse a policy identifier (runtime constructor / CLI / PyO3 surface).
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "flat" => Some(Self::Flat),
            "factored" => Some(Self::Factored),
            _ => None,
        }
    }
}

/// SQL menu digit budget after T-D: two base-256 digits → capacity 65_536.
///
/// Keeps the shared alphabet menu band width at 256; wide catalogs emit
/// multi-digit ordinals rather than widening the band.
pub const SQL_MAX_MENU_DIGITS: u32 = 2;

/// Exclusive ordinal capacity for SQL menu sites: `band_width ^ max_digits`.
pub fn sql_menu_ordinal_capacity() -> u64 {
    (SQL_MENU_BAND_WIDTH as u64).pow(SQL_MAX_MENU_DIGITS)
}

fn require_within_sql_menu_capacity(limit: u32, subject: &str) -> Result<(), MenuBindingError> {
    let capacity = sql_menu_ordinal_capacity();
    if (limit as u64) > capacity {
        return Err(MenuBindingError::new(format!(
            "{subject} has limit {limit} which exceeds SQL menu capacity {capacity} \
             (band width {SQL_MENU_BAND_WIDTH}, max_menu_digits {SQL_MAX_MENU_DIGITS})"
        )));
    }
    Ok(())
}

/// Build table-site menu bindings from a loaded catalog (T4 probe helper).
///
/// Cardinality is `catalog.tables.len()` in catalog order. Does **not** mark
/// the site as scope-pushing (table-only probes without a select root).
/// Opts in to [`SQL_MAX_MENU_DIGITS`] so wide catalogs bind cleanly.
pub fn table_menu_bindings_from_catalog(
    catalog: &SqlCatalog,
) -> Result<MenuSiteBindings, MenuBindingError> {
    let cardinality = table_cardinality(catalog)?;
    let mut bindings = MenuSiteBindings::new();
    bindings.set_max_menu_digits(SQL_MAX_MENU_DIGITS);
    bindings.insert(TABLE_MENU_OWNER_RULE, TABLE_MENU_OWNER_FIELD, cardinality);
    Ok(bindings)
}

/// Full catalog menu bindings: table sites push scope; column sites are
/// scope-relative; `select_statement` is the scope root (**P5**/**T6**).
///
/// Sets `max_menu_digits = 2` so catalogs with more than 256 tables/columns
/// bind under the radix-digit engine (T-D). Capacity is
/// [`sql_menu_ordinal_capacity`] (65_536), not the single-token band width.
pub fn menu_bindings_from_catalog(
    catalog: &SqlCatalog,
) -> Result<MenuSiteBindings, MenuBindingError> {
    menu_bindings_from_catalog_with_join_policy(catalog, SqlJoinOnPolicy::Off)
}

/// [`menu_bindings_from_catalog`] with an explicit join-ON masking policy.
///
/// `Structural` additionally binds the engine [`JoinOnConstraint`] over the
/// v0 grammar's `join_clause.on` / `comparison` / `column_ref` names. The
/// policy changes admissibility (and therefore canonical elided streams), not
/// the token layout; callers own the stream-contract version they record.
pub fn menu_bindings_from_catalog_with_join_policy(
    catalog: &SqlCatalog,
    join_on_policy: SqlJoinOnPolicy,
) -> Result<MenuSiteBindings, MenuBindingError> {
    menu_bindings_from_catalog_with_policies(catalog, join_on_policy, SqlColumnAddressing::Flat)
}

/// [`menu_bindings_from_catalog`] with both SQL menu policies stated.
///
/// The policies are independent and compose: `join_on_policy` narrows
/// admissibility at equality join keys, `column_addressing` changes how every
/// column ordinal is spelled. Neither changes the token layout; callers own
/// the combined stream-contract tag they record.
pub fn menu_bindings_from_catalog_with_policies(
    catalog: &SqlCatalog,
    join_on_policy: SqlJoinOnPolicy,
    column_addressing: SqlColumnAddressing,
) -> Result<MenuSiteBindings, MenuBindingError> {
    let table_card = table_cardinality(catalog)?;
    let columns_per_table = columns_per_table(catalog)?;
    let full_column_sum = columns_per_table
        .iter()
        .copied()
        .fold(0u32, u32::saturating_add);
    if full_column_sum == 0 {
        return Err(MenuBindingError::new(format!(
            "catalog {} has no real columns for column-site menu binding",
            catalog.db_id
        )));
    }
    require_within_sql_menu_capacity(
        full_column_sum,
        &format!("catalog {} real-column sum", catalog.db_id),
    )?;
    let mut bindings = MenuSiteBindings::new();
    bindings.set_max_menu_digits(SQL_MAX_MENU_DIGITS);
    bindings.insert_table_site(TABLE_MENU_OWNER_RULE, TABLE_MENU_OWNER_FIELD, table_card);
    match column_addressing {
        SqlColumnAddressing::Flat => bindings.insert_scope_column_site(
            COLUMN_MENU_OWNER_RULE,
            COLUMN_MENU_OWNER_FIELD,
            columns_per_table,
        ),
        SqlColumnAddressing::Factored => bindings.insert_factored_column_site(
            COLUMN_MENU_OWNER_RULE,
            COLUMN_MENU_OWNER_FIELD,
            columns_per_table,
        ),
    }
    bindings.mark_scope_root(SELECT_STATEMENT_RULE);
    if join_on_policy.binds_join_on_constraint() {
        bindings.bind_join_on_structural(JoinOnConstraint {
            join_rule: JOIN_CLAUSE_RULE.to_string(),
            on_field: JOIN_ON_FIELD.to_string(),
            comparison_rule: COMPARISON_RULE.to_string(),
            comparison_left_field: COMPARISON_LEFT_FIELD.to_string(),
            comparison_op_field: COMPARISON_OP_FIELD.to_string(),
            comparison_right_field: COMPARISON_RIGHT_FIELD.to_string(),
            eq_op_rule: OP_EQ_RULE.to_string(),
            column_rule: COLUMN_MENU_OWNER_RULE.to_string(),
            column_field: COLUMN_MENU_OWNER_FIELD.to_string(),
            expression_wrapper_rule: EXPRESSION_RULE.to_string(),
        });
    }
    if join_on_policy == SqlJoinOnPolicy::FkDeclared {
        bindings.bind_join_on_fk_pairs(fk_pairs_from_catalog(catalog)?);
    }
    Ok(bindings)
}

/// Catalog FK edges as menu-ordinal pairs (table index, per-table column index).
fn fk_pairs_from_catalog(
    catalog: &SqlCatalog,
) -> Result<Vec<crate::syntax_state_machine::JoinOnFkPair>, MenuBindingError> {
    use crate::syntax_state_machine::JoinOnFkPair;
    let mut pairs = Vec::with_capacity(catalog.foreign_keys.len());
    for fk in &catalog.foreign_keys {
        let from_table = catalog.table(&fk.from_table).ok_or_else(|| {
            MenuBindingError::new(format!(
                "FK from_table {} missing in catalog {}",
                fk.from_table, catalog.db_id
            ))
        })?;
        let to_table = catalog.table(&fk.to_table).ok_or_else(|| {
            MenuBindingError::new(format!(
                "FK to_table {} missing in catalog {}",
                fk.to_table, catalog.db_id
            ))
        })?;
        let from_column = from_table
            .columns
            .iter()
            .position(|column| column.name == fk.from_column)
            .ok_or_else(|| {
                MenuBindingError::new(format!(
                    "FK from_column {}.{} missing in catalog {}",
                    fk.from_table, fk.from_column, catalog.db_id
                ))
            })?;
        let to_column = to_table
            .columns
            .iter()
            .position(|column| column.name == fk.to_column)
            .ok_or_else(|| {
                MenuBindingError::new(format!(
                    "FK to_column {}.{} missing in catalog {}",
                    fk.to_table, fk.to_column, catalog.db_id
                ))
            })?;
        pairs.push(JoinOnFkPair {
            from_table: u32::try_from(from_table.table_index)
                .map_err(|_| MenuBindingError::new("FK from table_index exceeds u32"))?,
            from_column: u32::try_from(from_column)
                .map_err(|_| MenuBindingError::new("FK from column index exceeds u32"))?,
            to_table: u32::try_from(to_table.table_index)
                .map_err(|_| MenuBindingError::new("FK to table_index exceeds u32"))?,
            to_column: u32::try_from(to_column)
                .map_err(|_| MenuBindingError::new("FK to column index exceeds u32"))?,
        });
    }
    Ok(pairs)
}

/// Column-site cardinality for a single-table scope (diagnostic / T5 helper).
pub fn column_menu_cardinality_for_table(
    catalog: &SqlCatalog,
    table_index: usize,
) -> Result<u32, MenuBindingError> {
    let table = catalog.tables.get(table_index).ok_or_else(|| {
        MenuBindingError::new(format!(
            "table index {table_index} out of range for catalog {}",
            catalog.db_id
        ))
    })?;
    let cardinality = u32::try_from(table.columns.len()).map_err(|_| {
        MenuBindingError::new(format!(
            "table {} has more columns than u32::MAX",
            table.name
        ))
    })?;
    if cardinality == 0 {
        return Err(MenuBindingError::new(format!(
            "table {} has no columns for column-site menu binding",
            table.name
        )));
    }
    require_within_sql_menu_capacity(cardinality, &format!("table {} column count", table.name))?;
    Ok(cardinality)
}

/// Table + scope-relative column menu bindings (T5 single-table / T6 multi-table).
///
/// `table_index` is retained for API stability; scope is driven by encoded
/// table sites, not this argument.
pub fn menu_bindings_for_single_table_scope(
    catalog: &SqlCatalog,
    _table_index: usize,
) -> Result<MenuSiteBindings, MenuBindingError> {
    menu_bindings_from_catalog(catalog)
}

fn table_cardinality(catalog: &SqlCatalog) -> Result<u32, MenuBindingError> {
    let cardinality = u32::try_from(catalog.tables.len()).map_err(|_| {
        MenuBindingError::new(format!(
            "catalog {} has more tables than u32::MAX",
            catalog.db_id
        ))
    })?;
    if cardinality == 0 {
        return Err(MenuBindingError::new(format!(
            "catalog {} has no tables for table-site menu binding",
            catalog.db_id
        )));
    }
    require_within_sql_menu_capacity(
        cardinality,
        &format!("catalog {} table count", catalog.db_id),
    )?;
    Ok(cardinality)
}

fn columns_per_table(catalog: &SqlCatalog) -> Result<Vec<u32>, MenuBindingError> {
    catalog
        .tables
        .iter()
        .map(|table| {
            u32::try_from(table.columns.len()).map_err(|_| {
                MenuBindingError::new(format!(
                    "table {} has more columns than u32::MAX",
                    table.name
                ))
            })
        })
        .collect()
}

/// Typed error for menu binding construction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MenuBindingError {
    pub message: String,
}

impl MenuBindingError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for MenuBindingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SQL menu binding error: {}", self.message)
    }
}

impl std::error::Error for MenuBindingError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grammar_ir::{
        GrammarField, GrammarFieldAlternative, GrammarIr, GrammarIrProvenance, GrammarRule,
        GrammarTerminalKind,
    };
    use crate::grammar_ir_structural_model::lower_grammar_ir_to_structural_model;
    use crate::sql_schema_linking::catalog::load_fixture_catalog;
    use crate::sql_schema_linking::grammar::{
        COLUMN_MENU_OWNER_FIELD, COLUMN_MENU_OWNER_RULE, SELECT_STATEMENT_RULE,
        TABLE_MENU_OWNER_FIELD, TABLE_MENU_OWNER_RULE, lower_sql_schema_linking_grammar,
    };
    use crate::sql_schema_linking::layout::{menu_band_range, sql_syntax_token_layout};
    use crate::syntax_state_machine::{
        CompiledSyntaxModel, RawSyntaxDecodedField, RawSyntaxDecodedNode, RawSyntaxDecodedTerminal,
        RawSyntaxDecodedValue,
    };
    use crate::toy_grammar_frontend::ToyGrammarSpec;
    use std::collections::BTreeSet;

    #[test]
    fn table_menu_bindings_match_fixture_catalog_cardinality() {
        let catalog = load_fixture_catalog().expect("fixture catalog");
        let bindings = table_menu_bindings_from_catalog(&catalog).expect("bindings");
        assert_eq!(
            bindings.cardinality(TABLE_MENU_OWNER_RULE, TABLE_MENU_OWNER_FIELD),
            Some(3),
            "fixture has three tables"
        );
        assert!(
            bindings
                .cardinality(COLUMN_MENU_OWNER_RULE, COLUMN_MENU_OWNER_FIELD)
                .is_none(),
            "T4 binds table sites only"
        );
    }

    #[test]
    fn table_site_mask_is_exactly_catalog_table_menu_and_transition_accepts_exactly_those() {
        // Minimal probe: root rule named table_ref with menu_index IntegerPayload
        // so we can open the table menu site without a full select_statement tree.
        let mut grammar = GrammarIr::new(
            "sql_table_menu_probe_v0",
            GrammarIrProvenance {
                source_kind: "sql_menu_probe".to_string(),
                source_path: Some("src/sql_schema_linking/menu_binding.rs".to_string()),
                source_sha256: None,
                metadata_path: None,
                metadata_sha256: None,
                parser: None,
            },
        );
        grammar.rules.push(GrammarRule {
            name: TABLE_MENU_OWNER_RULE.to_string(),
            node_kind: TABLE_MENU_OWNER_RULE.to_string(),
            fields: vec![GrammarField {
                name: TABLE_MENU_OWNER_FIELD.to_string(),
                required: true,
                multiple: false,
                alternatives: vec![GrammarFieldAlternative {
                    node_kind: "table_menu_index".to_string(),
                    terminal: GrammarTerminalKind::IntegerPayload,
                }],
            }],
        });
        let sidecar = lower_grammar_ir_to_structural_model(&grammar, sql_syntax_token_layout())
            .expect("lower probe");
        let catalog = load_fixture_catalog().expect("catalog");
        let bindings = table_menu_bindings_from_catalog(&catalog).expect("bindings");
        let model = CompiledSyntaxModel::from_sidecar(&sidecar)
            .expect("compile")
            .with_menu_bindings(bindings)
            .expect("attach table menu bindings");

        let menu = menu_band_range(&sql_syntax_token_layout()).expect("menu band on SQL layout");
        let cardinality = 3u32;
        let expected_menu: BTreeSet<u32> = (0..cardinality).map(|i| menu.0 + i).collect();

        // Prefix through terminal opening of the menu payload.
        let roles = model.token_roles();
        let prefix = vec![roles.node_start, roles.field_start, roles.terminal];
        let admissible = model
            .admissible_next_raw_tokens(&prefix)
            .expect("prefix must be legal");
        let mut admitted = BTreeSet::new();
        for &id in &admissible.finite_raw_token_ids {
            admitted.insert(id);
        }
        for range in &admissible.raw_token_ranges {
            for token in range.start..range.end_exclusive {
                admitted.insert(token);
            }
        }
        assert_eq!(
            admitted, expected_menu,
            "table-site mask must be exactly the catalog table menu"
        );

        for &token in &expected_menu {
            let t = model.transition(&prefix, token);
            assert!(
                t.accepted,
                "transition must accept table menu token {token}"
            );
        }
        // Probe every id outside the menu for rejection (spot-check band edges).
        for token in [
            menu.0.saturating_sub(1),
            menu.0 + cardinality,
            roles.integer_range.start,
            roles.pad_token,
        ] {
            if expected_menu.contains(&token) {
                continue;
            }
            let t = model.transition(&prefix, token);
            assert!(
                !t.accepted,
                "transition must reject non-menu token {token} at table site"
            );
        }

        // Full encode/decode of a menu ordinal round-trips on the menu band.
        let tree = RawSyntaxDecodedNode {
            rule_name: TABLE_MENU_OWNER_RULE.to_string(),
            node_kind: TABLE_MENU_OWNER_RULE.to_string(),
            fields: vec![RawSyntaxDecodedField {
                name: TABLE_MENU_OWNER_FIELD.to_string(),
                values: vec![RawSyntaxDecodedValue::Terminal(RawSyntaxDecodedTerminal {
                    node_kind: "table_menu_index".to_string(),
                    terminal: "integer".to_string(),
                    payload: "1".to_string(),
                })],
                missing: false,
                repeated_empty: false,
            }],
        };
        let tokens = model.encode_complete_tree(&tree).expect("encode");
        assert!(tokens.iter().all(|&t| t < roles.pad_token));
        assert!(
            tokens.contains(&(menu.0 + 1)),
            "encoded stream must include menu ordinal 1"
        );
        let decoded = model
            .decode_complete_tree(&tokens)
            .expect("decode")
            .decoded_node_tree()
            .expect("tree");
        assert_eq!(decoded, tree);
    }

    #[test]
    fn binding_rejects_table_cardinality_above_menu_band_width() {
        // T7: catalogs wider than SQL_MENU_BAND_WIDTH fail closed at bind time.
        let mut bindings = MenuSiteBindings::new();
        bindings.insert_table_site(
            TABLE_MENU_OWNER_RULE,
            TABLE_MENU_OWNER_FIELD,
            SQL_MENU_BAND_WIDTH + 1,
        );
        let sidecar = lower_sql_schema_linking_grammar().expect("lower");
        let err = CompiledSyntaxModel::from_sidecar(&sidecar)
            .expect("compile")
            .with_menu_bindings(bindings)
            .expect_err("over-wide table menu must fail closed");
        let msg = format!("{err:?}");
        assert!(
            msg.contains("exceeds") || msg.contains("overflow") || msg.contains("band"),
            "unexpected error: {msg}"
        );
    }

    #[test]
    fn binding_rejects_column_band_limit_above_menu_band_width() {
        let mut bindings = MenuSiteBindings::new();
        bindings.insert_scope_column_site(
            COLUMN_MENU_OWNER_RULE,
            COLUMN_MENU_OWNER_FIELD,
            vec![SQL_MENU_BAND_WIDTH + 1],
        );
        let sidecar = lower_sql_schema_linking_grammar().expect("lower");
        let err = CompiledSyntaxModel::from_sidecar(&sidecar)
            .expect("compile")
            .with_menu_bindings(bindings)
            .expect_err("over-wide column menu must fail closed");
        let msg = format!("{err:?}");
        assert!(
            msg.contains("exceeds") || msg.contains("overflow") || msg.contains("band"),
            "unexpected error: {msg}"
        );
    }

    #[test]
    fn out_of_menu_table_index_rejected_by_mask_and_transition() {
        // T7: at a table site, ordinals outside [0, cardinality) are rejected
        // by both the admissible mask and transition.
        let mut grammar = GrammarIr::new(
            "sql_table_menu_out_of_range_v0",
            GrammarIrProvenance {
                source_kind: "sql_menu_probe".to_string(),
                source_path: Some("src/sql_schema_linking/menu_binding.rs".to_string()),
                source_sha256: None,
                metadata_path: None,
                metadata_sha256: None,
                parser: None,
            },
        );
        grammar.rules.push(GrammarRule {
            name: TABLE_MENU_OWNER_RULE.to_string(),
            node_kind: TABLE_MENU_OWNER_RULE.to_string(),
            fields: vec![GrammarField {
                name: TABLE_MENU_OWNER_FIELD.to_string(),
                required: true,
                multiple: false,
                alternatives: vec![GrammarFieldAlternative {
                    node_kind: "table_menu_index".to_string(),
                    terminal: GrammarTerminalKind::IntegerPayload,
                }],
            }],
        });
        let sidecar = lower_grammar_ir_to_structural_model(&grammar, sql_syntax_token_layout())
            .expect("lower probe");
        let catalog = load_fixture_catalog().expect("catalog");
        let bindings = table_menu_bindings_from_catalog(&catalog).expect("bindings");
        let model = CompiledSyntaxModel::from_sidecar(&sidecar)
            .expect("compile")
            .with_menu_bindings(bindings)
            .expect("attach");
        let roles = model.token_roles();
        let menu = menu_band_range(&sql_syntax_token_layout()).expect("menu band");
        let cardinality = 3u32;
        let prefix = vec![roles.node_start, roles.field_start, roles.terminal];
        let admissible = model
            .admissible_next_raw_tokens(&prefix)
            .expect("prefix legal");
        let mut admitted = BTreeSet::new();
        for &id in &admissible.finite_raw_token_ids {
            admitted.insert(id);
        }
        for range in &admissible.raw_token_ranges {
            for token in range.start..range.end_exclusive {
                admitted.insert(token);
            }
        }
        // In-menu accepted.
        for i in 0..cardinality {
            let token = menu.0 + i;
            assert!(
                admitted.contains(&token),
                "menu ordinal {i} must be in mask"
            );
            assert!(
                model.transition(&prefix, token).accepted,
                "transition must accept menu ordinal {i}"
            );
        }
        // Out-of-menu rejected by both mask and transition.
        for i in [cardinality, cardinality + 1, SQL_MENU_BAND_WIDTH - 1] {
            let token = menu.0 + i;
            assert!(
                !admitted.contains(&token),
                "out-of-menu ordinal {i} must not be in mask"
            );
            assert!(
                !model.transition(&prefix, token).accepted,
                "transition must reject out-of-menu ordinal {i}"
            );
        }
    }

    #[test]
    fn p10_menu_less_models_keep_bit_identical_roles_and_masks() {
        // Toy layout has no menu_index_payload; empty bindings leave menu paths unreachable.
        let spec = ToyGrammarSpec::assignment_language_v1();
        let grammar = spec.to_grammar_ir();
        let layout =
            crate::grammar_ir_structural_model::GrammarStructuralTokenLayout::from_toy_assignment(
                &spec.layout,
            );
        let sidecar = lower_grammar_ir_to_structural_model(&grammar, layout).expect("toy lower");
        let model_a = CompiledSyntaxModel::from_sidecar(&sidecar).expect("compile a");
        let model_b = CompiledSyntaxModel::from_sidecar(&sidecar).expect("compile b");
        assert!(model_a.menu_bindings().is_empty());
        assert!(model_b.menu_bindings().is_empty());
        assert!(
            model_a.token_roles().menu_range.is_none(),
            "toy layout must not declare menu_index_payload"
        );
        assert_eq!(
            model_a.token_roles(),
            model_b.token_roles(),
            "P10: menu-less compile is deterministic on roles"
        );

        // Empty-prefix admissible masks must match (menu code unreachable).
        let mask_a = model_a
            .admissible_next_raw_tokens(&[])
            .expect("empty prefix a");
        let mask_b = model_b
            .admissible_next_raw_tokens(&[])
            .expect("empty prefix b");
        assert_eq!(mask_a, mask_b, "P10: empty-prefix masks bit-identical");

        // Strengthen: replay every prefix of a menu-less toy stream and assert
        // the menu layout role stays absent (no menu_cardinality path can arm).
        let leaf = RawSyntaxDecodedNode {
            rule_name: "program".to_string(),
            node_kind: "program".to_string(),
            fields: vec![],
        };
        // Prefer a field-less root if present; otherwise just walk empty→partial
        // structural prefixes built from roles.
        let roles = model_a.token_roles();
        let toy_stream = [roles.node_start, roles.node_end];
        for end in 0..=toy_stream.len() {
            let prefix = &toy_stream[..end];
            assert!(
                model_a.token_roles().menu_range.is_none(),
                "P10: menu_range must stay absent on menu-less model"
            );
            assert!(model_a.menu_bindings().is_empty());
            if end < toy_stream.len() {
                let _ = model_a.admissible_next_raw_tokens(prefix);
            }
        }
        let _ = leaf; // structural witness that decoded nodes stay available

        // SQL grammar without bindings: integer path still uses u8 band, not menu.
        let sql_sidecar = lower_sql_schema_linking_grammar().expect("sql lower");
        let sql_model = CompiledSyntaxModel::from_sidecar(&sql_sidecar).expect("sql compile");
        assert!(sql_model.menu_bindings().is_empty());
        let menu_range = sql_model
            .token_roles()
            .menu_range
            .as_ref()
            .expect("SQL layout declares menu band even when unbound");
        // Unbound SQL model must not admit menu band tokens at empty prefix
        // *or* after opening the root node (every proper structural prefix
        // before a payload site).
        let sql_roles = sql_model.token_roles();
        let sql_prefixes: Vec<Vec<u32>> = vec![
            vec![],
            vec![sql_roles.node_start],
            vec![sql_roles.node_start, sql_roles.field_start],
        ];
        for prefix in &sql_prefixes {
            let mask = sql_model
                .admissible_next_raw_tokens(prefix)
                .expect("unbound SQL structural prefix");
            for token in menu_range.start..menu_range.end_exclusive.min(menu_range.start + 8) {
                let in_finite = mask.finite_raw_token_ids.contains(&token);
                let in_range = mask
                    .raw_token_ranges
                    .iter()
                    .any(|r| token >= r.start && token < r.end_exclusive);
                assert!(
                    !in_finite && !in_range,
                    "unbound SQL model must not admit menu token {token} at prefix {prefix:?}"
                );
            }
        }
    }

    // --- T-C: multi-digit engine (SQL still opts out at max_menu_digits=1) ---

    fn wide_table_menu_probe_model(cardinality: u32) -> CompiledSyntaxModel {
        let mut grammar = GrammarIr::new(
            "sql_wide_table_menu_probe_v0",
            GrammarIrProvenance {
                source_kind: "sql_menu_probe".to_string(),
                source_path: Some("src/sql_schema_linking/menu_binding.rs".to_string()),
                source_sha256: None,
                metadata_path: None,
                metadata_sha256: None,
                parser: None,
            },
        );
        grammar.rules.push(GrammarRule {
            name: TABLE_MENU_OWNER_RULE.to_string(),
            node_kind: TABLE_MENU_OWNER_RULE.to_string(),
            fields: vec![GrammarField {
                name: TABLE_MENU_OWNER_FIELD.to_string(),
                required: true,
                multiple: false,
                alternatives: vec![GrammarFieldAlternative {
                    node_kind: "table_menu_index".to_string(),
                    terminal: GrammarTerminalKind::IntegerPayload,
                }],
            }],
        });
        let sidecar = lower_grammar_ir_to_structural_model(&grammar, sql_syntax_token_layout())
            .expect("lower wide probe");
        let mut bindings = MenuSiteBindings::new();
        bindings.set_max_menu_digits(2);
        bindings.insert(TABLE_MENU_OWNER_RULE, TABLE_MENU_OWNER_FIELD, cardinality);
        CompiledSyntaxModel::from_sidecar(&sidecar)
            .expect("compile")
            .with_menu_bindings(bindings)
            .expect("attach wide table menu")
    }

    fn admitted_set(model: &CompiledSyntaxModel, prefix: &[u32]) -> BTreeSet<u32> {
        let admissible = model
            .admissible_next_raw_tokens(prefix)
            .expect("prefix legal");
        let mut admitted = BTreeSet::new();
        for &id in &admissible.finite_raw_token_ids {
            admitted.insert(id);
        }
        for range in &admissible.raw_token_ranges {
            for token in range.start..range.end_exclusive {
                admitted.insert(token);
            }
        }
        admitted
    }

    fn menu_tree(ordinal: u32) -> RawSyntaxDecodedNode {
        RawSyntaxDecodedNode {
            rule_name: TABLE_MENU_OWNER_RULE.to_string(),
            node_kind: TABLE_MENU_OWNER_RULE.to_string(),
            fields: vec![RawSyntaxDecodedField {
                name: TABLE_MENU_OWNER_FIELD.to_string(),
                values: vec![RawSyntaxDecodedValue::Terminal(RawSyntaxDecodedTerminal {
                    node_kind: "table_menu_index".to_string(),
                    terminal: "integer".to_string(),
                    payload: ordinal.to_string(),
                })],
                missing: false,
                repeated_empty: false,
            }],
        }
    }

    #[test]
    fn tc_wide_catalog_round_trips_boundary_ordinals() {
        // (1) Synthetic 300-table / 3,000-column band limits round-trip
        // ordinals 0 / 255 / 256 / 257 / N-1 under max_menu_digits=2.
        for &(n_tables, n_columns) in &[(300u32, 3_000u32)] {
            let table_model = wide_table_menu_probe_model(n_tables);
            for &ord in &[0u32, 255, 256, 257, n_tables - 1] {
                let tree = menu_tree(ord);
                let tokens = table_model
                    .encode_complete_tree(&tree)
                    .unwrap_or_else(|e| panic!("encode table ord={ord}: {e:?}"));
                // W=2 for N=300: two digit tokens + structural framing.
                assert!(
                    tokens.len() >= 5,
                    "wide ordinal {ord} must emit multi-digit stream, got {tokens:?}"
                );
                let decoded = table_model
                    .decode_complete_tree(&tokens)
                    .unwrap_or_else(|e| panic!("decode table ord={ord}: {e:?}"))
                    .decoded_node_tree()
                    .expect("tree");
                assert_eq!(decoded, tree, "table ord={ord}");
            }

            // Column band as fixed (non-scope) site with same digit path.
            let mut grammar = GrammarIr::new(
                "sql_wide_column_menu_probe_v0",
                GrammarIrProvenance {
                    source_kind: "sql_menu_probe".to_string(),
                    source_path: Some("src/sql_schema_linking/menu_binding.rs".to_string()),
                    source_sha256: None,
                    metadata_path: None,
                    metadata_sha256: None,
                    parser: None,
                },
            );
            grammar.rules.push(GrammarRule {
                name: COLUMN_MENU_OWNER_RULE.to_string(),
                node_kind: COLUMN_MENU_OWNER_RULE.to_string(),
                fields: vec![GrammarField {
                    name: COLUMN_MENU_OWNER_FIELD.to_string(),
                    required: true,
                    multiple: false,
                    alternatives: vec![GrammarFieldAlternative {
                        node_kind: "column_menu_index".to_string(),
                        terminal: GrammarTerminalKind::IntegerPayload,
                    }],
                }],
            });
            let sidecar = lower_grammar_ir_to_structural_model(&grammar, sql_syntax_token_layout())
                .expect("lower column probe");
            let mut bindings = MenuSiteBindings::new();
            bindings.set_max_menu_digits(2);
            bindings.insert(COLUMN_MENU_OWNER_RULE, COLUMN_MENU_OWNER_FIELD, n_columns);
            let col_model = CompiledSyntaxModel::from_sidecar(&sidecar)
                .expect("compile")
                .with_menu_bindings(bindings)
                .expect("attach wide column menu");
            for &ord in &[0u32, 255, 256, 257, n_columns - 1] {
                let tree = RawSyntaxDecodedNode {
                    rule_name: COLUMN_MENU_OWNER_RULE.to_string(),
                    node_kind: COLUMN_MENU_OWNER_RULE.to_string(),
                    fields: vec![RawSyntaxDecodedField {
                        name: COLUMN_MENU_OWNER_FIELD.to_string(),
                        values: vec![RawSyntaxDecodedValue::Terminal(RawSyntaxDecodedTerminal {
                            node_kind: "column_menu_index".to_string(),
                            terminal: "integer".to_string(),
                            payload: ord.to_string(),
                        })],
                        missing: false,
                        repeated_empty: false,
                    }],
                };
                let tokens = col_model
                    .encode_complete_tree(&tree)
                    .unwrap_or_else(|e| panic!("encode column ord={ord}: {e:?}"));
                let decoded = col_model
                    .decode_complete_tree(&tokens)
                    .unwrap_or_else(|e| panic!("decode column ord={ord}: {e:?}"))
                    .decoded_node_tree()
                    .expect("tree");
                assert_eq!(decoded, tree, "column ord={ord}");
            }
        }
    }

    #[test]
    fn tc_mask_cardinality_equals_n_across_digit_boundary() {
        // (2) Exact first-digit-completion mask cardinality == N for the
        // named boundary values (count distinct complete ordinals admitted
        // by expanding the digit trie).
        let cases: &[u32] = &[1, 2, 255, 256, 257, 258, 511, 512, 513, 65_535, 65_536];
        for &n in cases {
            let model = wide_table_menu_probe_model(n);
            let roles = model.token_roles();
            let menu = roles.menu_range.as_ref().expect("menu band");
            let base = menu.end_exclusive.saturating_sub(menu.start);
            let prefix = vec![roles.node_start, roles.field_start, roles.terminal];
            let admitted = admitted_set(&model, &prefix);
            // Expand full completions from the digit mask.
            let mut completions = 0u32;
            let high_digits: Vec<u32> = admitted
                .iter()
                .filter(|&&t| t >= menu.start && t < menu.end_exclusive)
                .map(|&t| t - menu.start)
                .collect();
            if n <= base {
                // W=1: mask size is exactly N.
                assert_eq!(
                    high_digits.len() as u32,
                    n,
                    "W=1 mask size must equal N={n}"
                );
                completions = high_digits.len() as u32;
            } else {
                // W=2: for each high digit, open low-digit mask and sum.
                for &high in &high_digits {
                    let mut mid = prefix.clone();
                    mid.push(menu.start + high);
                    let low_admitted = admitted_set(&model, &mid);
                    let lows = low_admitted
                        .iter()
                        .filter(|&&t| t >= menu.start && t < menu.end_exclusive)
                        .count() as u32;
                    completions = completions.saturating_add(lows);
                }
            }
            assert_eq!(
                completions, n,
                "completion cardinality must equal N={n}, got {completions}"
            );
        }
    }

    #[test]
    fn tc_wide_streams_mask_equals_accept_over_full_vocab() {
        // (3) Full-vocab mask==accept sweep over wide multi-digit streams.
        let n = 300u32;
        let model = wide_table_menu_probe_model(n);
        let vocab = model.token_roles().vocab_size;
        for &ord in &[0u32, 255, 256, 257, n - 1] {
            let tokens = model
                .encode_complete_tree(&menu_tree(ord))
                .unwrap_or_else(|e| panic!("encode ord={ord}: {e:?}"));
            for end in 0..=tokens.len() {
                let prefix = &tokens[..end];
                match model.admissible_next_raw_tokens(prefix) {
                    Ok(admissible) => {
                        let mut admitted = BTreeSet::new();
                        for &id in &admissible.finite_raw_token_ids {
                            admitted.insert(id);
                        }
                        for range in &admissible.raw_token_ranges {
                            for token in range.start..range.end_exclusive {
                                admitted.insert(token);
                            }
                        }
                        for candidate in 0..vocab {
                            let in_mask = admitted.contains(&candidate);
                            let accepted = model.transition(prefix, candidate).accepted;
                            assert_eq!(
                                in_mask, accepted,
                                "ord={ord} prefix_len={end}: token {candidate} mask={in_mask} accept={accepted}"
                            );
                        }
                    }
                    Err(err) => {
                        assert_eq!(
                            end,
                            tokens.len(),
                            "proper prefix_len={end} must not error: {err:?}"
                        );
                        for candidate in 0..vocab {
                            assert!(
                                !model.transition(prefix, candidate).accepted,
                                "complete stream must reject token {candidate}"
                            );
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn tc_dead_high_digit_excluded_when_only_residual_in_scope() {
        // (4) N=257 with ordinal 256 already in scope: high digit 1 must be
        // absent from the mask and rejected by transition.
        let mut grammar = GrammarIr::new(
            "sql_dead_high_digit_probe_v0",
            GrammarIrProvenance {
                source_kind: "sql_menu_probe".to_string(),
                source_path: Some("src/sql_schema_linking/menu_binding.rs".to_string()),
                source_sha256: None,
                metadata_path: None,
                metadata_sha256: None,
                parser: None,
            },
        );
        // Scope root with two successive table_ref fields so the first push
        // populates scope before the second menu site opens.
        grammar.rules.push(GrammarRule {
            name: SELECT_STATEMENT_RULE.to_string(),
            node_kind: SELECT_STATEMENT_RULE.to_string(),
            fields: vec![
                GrammarField {
                    name: "t1".to_string(),
                    required: true,
                    multiple: false,
                    alternatives: vec![GrammarFieldAlternative {
                        node_kind: TABLE_MENU_OWNER_RULE.to_string(),
                        terminal: GrammarTerminalKind::Nonterminal,
                    }],
                },
                GrammarField {
                    name: "t2".to_string(),
                    required: true,
                    multiple: false,
                    alternatives: vec![GrammarFieldAlternative {
                        node_kind: TABLE_MENU_OWNER_RULE.to_string(),
                        terminal: GrammarTerminalKind::Nonterminal,
                    }],
                },
            ],
        });
        grammar.rules.push(GrammarRule {
            name: TABLE_MENU_OWNER_RULE.to_string(),
            node_kind: TABLE_MENU_OWNER_RULE.to_string(),
            fields: vec![GrammarField {
                name: TABLE_MENU_OWNER_FIELD.to_string(),
                required: true,
                multiple: false,
                alternatives: vec![GrammarFieldAlternative {
                    node_kind: "table_menu_index".to_string(),
                    terminal: GrammarTerminalKind::IntegerPayload,
                }],
            }],
        });
        let sidecar = lower_grammar_ir_to_structural_model(&grammar, sql_syntax_token_layout())
            .expect("lower dead-high probe");
        let mut bindings = MenuSiteBindings::new();
        bindings.set_max_menu_digits(2);
        bindings.insert_table_site(TABLE_MENU_OWNER_RULE, TABLE_MENU_OWNER_FIELD, 257);
        bindings.mark_scope_root(SELECT_STATEMENT_RULE);
        let model = CompiledSyntaxModel::from_sidecar(&sidecar)
            .expect("compile")
            .with_menu_bindings(bindings)
            .expect("attach");

        let table_node = |ord: u32| RawSyntaxDecodedNode {
            rule_name: TABLE_MENU_OWNER_RULE.to_string(),
            node_kind: TABLE_MENU_OWNER_RULE.to_string(),
            fields: vec![RawSyntaxDecodedField {
                name: TABLE_MENU_OWNER_FIELD.to_string(),
                values: vec![RawSyntaxDecodedValue::Terminal(RawSyntaxDecodedTerminal {
                    node_kind: "table_menu_index".to_string(),
                    terminal: "integer".to_string(),
                    payload: ord.to_string(),
                })],
                missing: false,
                repeated_empty: false,
            }],
        };
        let tree = RawSyntaxDecodedNode {
            rule_name: SELECT_STATEMENT_RULE.to_string(),
            node_kind: SELECT_STATEMENT_RULE.to_string(),
            fields: vec![
                RawSyntaxDecodedField {
                    name: "t1".to_string(),
                    values: vec![RawSyntaxDecodedValue::Node(Box::new(table_node(256)))],
                    missing: false,
                    repeated_empty: false,
                },
                RawSyntaxDecodedField {
                    name: "t2".to_string(),
                    values: vec![RawSyntaxDecodedValue::Node(Box::new(table_node(0)))],
                    missing: false,
                    repeated_empty: false,
                },
            ],
        };
        let tokens = model
            .encode_complete_tree(&tree)
            .expect("encode two-table stream with 256 then 0");

        // Walk to the second table menu payload open (after t1 complete).
        // Find the second occurrence of terminal that opens a menu payload
        // with high-digit choice under scope containing 256.
        let roles = model.token_roles();
        let menu = roles.menu_range.as_ref().expect("menu");
        let high1 = menu.start + 1; // high digit 1 → residual [256, 257)

        // Scan prefixes: at any menu digit position where high=1 is the
        // only residual for ordinal 256 and 256 is in scope, high1 must die.
        let mut saw_dead_high = false;
        for end in 0..tokens.len() {
            let prefix = &tokens[..end];
            let Ok(admissible) = model.admissible_next_raw_tokens(prefix) else {
                continue;
            };
            let mut admitted = BTreeSet::new();
            for &id in &admissible.finite_raw_token_ids {
                admitted.insert(id);
            }
            for range in &admissible.raw_token_ranges {
                for token in range.start..range.end_exclusive {
                    admitted.insert(token);
                }
            }
            // Heuristic: mid-stream menu site where high digit 0 is admitted
            // (W=2 start) but we already consumed ordinal 256 earlier.
            let admits_high0 = admitted.contains(&(menu.start));
            let admits_any_menu = admitted
                .iter()
                .any(|&t| t >= menu.start && t < menu.end_exclusive);
            if admits_high0 && admits_any_menu && end > 4 {
                // After t1 (ordinal 256) completed, second site first digit:
                if !admitted.contains(&high1) && !model.transition(prefix, high1).accepted {
                    // Confirm high0 still open (so this is a first digit, not low).
                    assert!(
                        model.transition(prefix, menu.start).accepted,
                        "high0 must remain admissible at dead-high site"
                    );
                    saw_dead_high = true;
                    break;
                }
            }
        }
        assert!(
            saw_dead_high,
            "expected a second-table first-digit site where high digit 1 is dead under scope={{256}}"
        );

        // Direct reject: complete ordinal 256 twice must fail encode.
        let dup = RawSyntaxDecodedNode {
            rule_name: SELECT_STATEMENT_RULE.to_string(),
            node_kind: SELECT_STATEMENT_RULE.to_string(),
            fields: vec![
                RawSyntaxDecodedField {
                    name: "t1".to_string(),
                    values: vec![RawSyntaxDecodedValue::Node(Box::new(table_node(256)))],
                    missing: false,
                    repeated_empty: false,
                },
                RawSyntaxDecodedField {
                    name: "t2".to_string(),
                    values: vec![RawSyntaxDecodedValue::Node(Box::new(table_node(256)))],
                    missing: false,
                    repeated_empty: false,
                },
            ],
        };
        let err = model
            .encode_complete_tree(&dup)
            .expect_err("duplicate scope table 256 must fail");
        let msg = format!("{err:?}");
        assert!(
            msg.contains("scope") || msg.contains("duplicate") || msg.contains("256"),
            "unexpected duplicate error: {msg}"
        );
    }

    #[test]
    fn tc_engine_default_max_menu_digits_one_still_fails_closed_at_257() {
        // Engine default remains max_menu_digits=1 (SQL opt-in is only via
        // menu_bindings_from_catalog / table_menu_bindings_from_catalog).
        let mut bindings = MenuSiteBindings::new();
        assert_eq!(bindings.max_menu_digits, 1);
        bindings.insert_table_site(
            TABLE_MENU_OWNER_RULE,
            TABLE_MENU_OWNER_FIELD,
            SQL_MENU_BAND_WIDTH + 1,
        );
        let sidecar = lower_sql_schema_linking_grammar().expect("lower");
        let err = CompiledSyntaxModel::from_sidecar(&sidecar)
            .expect("compile")
            .with_menu_bindings(bindings)
            .expect_err("default max_menu_digits=1 must reject N=257");
        let msg = format!("{err:?}");
        assert!(
            msg.contains("exceeds") || msg.contains("capacity") || msg.contains("257"),
            "unexpected: {msg}"
        );
    }

    #[test]
    fn td_wide_fixture_binds_with_max_menu_digits_two() {
        use crate::sql_schema_linking::catalog::{
            SQL_SCHEMA_LINKING_FIXTURE_DB_ID, SQL_SCHEMA_LINKING_WIDE_FIXTURE_DB_ID,
            load_fixture_bundle, load_wide_menu_capacity_catalog,
        };

        assert_eq!(sql_menu_ordinal_capacity(), 65_536);
        assert_eq!(SQL_MAX_MENU_DIGITS, 2);

        let catalog = load_wide_menu_capacity_catalog().expect("wide catalog");
        assert_eq!(catalog.db_id, SQL_SCHEMA_LINKING_WIDE_FIXTURE_DB_ID);
        assert_eq!(catalog.tables.len(), 300);
        let total_cols: usize = catalog.tables.iter().map(|t| t.columns.len()).sum();
        assert_eq!(total_cols, 3_000);

        let bindings = menu_bindings_from_catalog(&catalog).expect("wide bindings");
        assert_eq!(bindings.max_menu_digits, SQL_MAX_MENU_DIGITS);
        assert_eq!(
            bindings.cardinality(TABLE_MENU_OWNER_RULE, TABLE_MENU_OWNER_FIELD),
            Some(300)
        );

        let sidecar = lower_sql_schema_linking_grammar().expect("lower");
        let model = CompiledSyntaxModel::from_sidecar(&sidecar)
            .expect("compile")
            .with_menu_bindings(bindings)
            .expect("wide catalog must attach under max_menu_digits=2");
        assert_eq!(model.menu_bindings().max_menu_digits, 2);

        // Bundle path remains the small join fixture only (T11/T12 stable).
        let bundle = load_fixture_bundle().expect("join bundle");
        assert_eq!(bundle.catalog.db_id, SQL_SCHEMA_LINKING_FIXTURE_DB_ID);
        assert_ne!(bundle.catalog.db_id, SQL_SCHEMA_LINKING_WIDE_FIXTURE_DB_ID);
        assert_eq!(bundle.catalog.tables.len(), 3);
    }

    #[test]
    fn td_capacity_bound_rejects_above_65536() {
        let mut bindings = MenuSiteBindings::new();
        bindings.set_max_menu_digits(SQL_MAX_MENU_DIGITS);
        bindings.insert_table_site(TABLE_MENU_OWNER_RULE, TABLE_MENU_OWNER_FIELD, 65_537);
        let sidecar = lower_sql_schema_linking_grammar().expect("lower");
        let err = CompiledSyntaxModel::from_sidecar(&sidecar)
            .expect("compile")
            .with_menu_bindings(bindings)
            .expect_err("N=65537 must exceed base^2 capacity");
        let msg = format!("{err:?}");
        assert!(
            msg.contains("65537") || msg.contains("capacity") || msg.contains("exceeds"),
            "unexpected: {msg}"
        );
    }

    /// Host-mounted / reconstructed Spider catalog smoke (skipped when absent).
    ///
    /// `baseball_1` has 352 columns (>256) so multi-digit menu capacity is required.
    /// Evidence class when exercised: fixture/dry-run compiler smoke — not claim-bearing.
    #[test]
    fn unpinned_spider_baseball_1_binds_under_two_digit_capacity() {
        use crate::sql_schema_linking::catalog::SqlCatalog;
        use crate::sql_schema_linking::codec::SqlSchemaLinkingCodec;
        use std::path::PathBuf;

        let path = std::env::var_os("TCT_SPIDER_TABLES_JSON")
            .map(PathBuf::from)
            .or_else(|| {
                let default = PathBuf::from("data/spider-reconstructed/tables.json");
                default.is_file().then_some(default)
            })
            .or_else(|| {
                let home = PathBuf::from(std::env::var_os("HOME")?);
                let p = home.join("data/spider-reconstructed/tables.json");
                p.is_file().then_some(p)
            });
        let Some(path) = path else {
            eprintln!(
                "skip unpinned_spider_baseball_1: set TCT_SPIDER_TABLES_JSON or place \
                 reconstructed tables at data/spider-reconstructed/tables.json"
            );
            return;
        };
        if !path.is_file() {
            eprintln!(
                "skip unpinned_spider_baseball_1: missing {}",
                path.display()
            );
            return;
        }

        let ids = SqlCatalog::list_db_ids(&path).expect("list db ids");
        assert!(
            ids.iter().any(|id| id == "baseball_1"),
            "expected baseball_1 in {}",
            path.display()
        );

        let catalog = SqlCatalog::load_unpinned_tables_json(&path, "baseball_1")
            .expect("load baseball_1 unpinned");
        assert_eq!(catalog.db_id, "baseball_1");
        let total_cols: usize = catalog.tables.iter().map(|t| t.columns.len()).sum();
        assert!(
            total_cols > SQL_MENU_BAND_WIDTH as usize,
            "baseball_1 should exceed single-token band ({total_cols} cols)"
        );
        assert!((total_cols as u64) <= sql_menu_ordinal_capacity());

        let bindings = menu_bindings_from_catalog(&catalog).expect("bindings under max_digits=2");
        assert_eq!(bindings.max_menu_digits, SQL_MAX_MENU_DIGITS);
        assert_eq!(
            bindings.cardinality(TABLE_MENU_OWNER_RULE, TABLE_MENU_OWNER_FIELD),
            Some(catalog.tables.len() as u32)
        );

        let codec = SqlSchemaLinkingCodec::for_catalog(catalog).expect("codec");
        // Parser matches SqlTable::name (display). Prefer a space-free display name
        // so unquoted SELECT * FROM <table> works without identifier quoting.
        let table = codec
            .catalog()
            .tables
            .iter()
            .find(|t| !t.name.contains(' '))
            .map(|t| t.name.as_str())
            .expect("baseball_1 should have at least one unquoted-safe table name");
        let sql = format!("SELECT * FROM {table}");
        let trip = codec
            .round_trip_source(&sql)
            .unwrap_or_else(|e| panic!("round-trip {sql}: {e}"));
        assert!(
            !trip.tokens.is_empty(),
            "expected non-empty token stream for {sql}"
        );
    }
}
