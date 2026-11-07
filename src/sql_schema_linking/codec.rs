//! SQL schema-linking generic codec glue (T5/T6).
//!
//! Compiles the SQL GrammarIR under the SQL layout, attaches catalog table +
//! scope-relative column menu bindings, and runs language AST → decoded tree →
//! generic `encode_complete_tree` / `decode_complete_tree` (**P2**). No bespoke
//! token stream encoder.

use crate::shared_alphabet_layout::SyntaxTokenLayoutMode;
use crate::sql_schema_linking::ast::SelectStatement;
use crate::sql_schema_linking::catalog::{SqlCatalog, load_fixture_catalog};
#[cfg(test)]
use crate::sql_schema_linking::grammar::lower_sql_schema_linking_grammar;
use crate::sql_schema_linking::grammar::lower_sql_schema_linking_grammar_with_fold_select_distinct;

#[cfg(test)]
use crate::sql_schema_linking::menu_binding::menu_bindings_from_catalog;
use crate::sql_schema_linking::menu_binding::{
    SqlColumnAddressing, SqlJoinOnPolicy, menu_bindings_from_catalog_with_policies,
};
use crate::sql_schema_linking::parser::{SqlParseError, parse_m0_select, parse_select};
use crate::sql_schema_linking::render::{RenderError, render_select};
use crate::sql_schema_linking::tree::{
    TreeError, select_from_decoded_node, select_to_decoded_node_with_fold_select_distinct,
};
use crate::syntax_state_machine::{
    CompiledSyntaxModel, RawSyntaxDecodedNode, RawSyntaxDecodedValue, RawSyntaxError,
};

/// Claim boundary carried in reports and docs (not a paper claim).
pub const SQL_CODEC_CLAIM_BOUNDARY: &str = "compiler_sql_schema_linking_only";
/// Codec subset id for the M0–M5 ladder step (T9: agg/DISTINCT/subquery).
pub const SQL_CODEC_SUBSET_ID: &str = "sql_schema_linking_m0_m5_select_v0";

/// Compiled SQL frontend codec bound to a catalog and menu bindings.
pub struct SqlSchemaLinkingCodec {
    catalog: SqlCatalog,
    model: CompiledSyntaxModel,
    fold_select_distinct: bool,
}

impl SqlSchemaLinkingCodec {
    /// Compile the SQL model with catalog-wide table + scope-relative column menus.
    pub fn for_catalog(catalog: SqlCatalog) -> Result<Self, SqlCodecError> {
        Self::for_catalog_with_mode(catalog, SyntaxTokenLayoutMode::PerGrammar)
    }

    /// Compile under an explicit layout mode (T10 SharedAlphabet parity).
    ///
    /// SharedAlphabet reuses the generic codec layer and maps menus onto the
    /// existing `u8_integer_payload` band with a 256 ceiling (**P9**). No
    /// bespoke SQL codec and no `SHARED_ALPHABET_*` constant changes.
    pub fn for_catalog_with_mode(
        catalog: SqlCatalog,
        mode: SyntaxTokenLayoutMode,
    ) -> Result<Self, SqlCodecError> {
        Self::for_catalog_with_mode_and_join_policy(catalog, mode, SqlJoinOnPolicy::Off)
    }

    /// Compile with an explicit join-ON masking policy (join-key masking).
    ///
    /// `Structural` narrows admissibility at equality join-key column sites;
    /// the token layout is unchanged but canonical elided streams differ, so
    /// callers must treat policy changes as a stream-contract break.
    pub fn for_catalog_with_join_policy(
        catalog: SqlCatalog,
        join_on_policy: SqlJoinOnPolicy,
    ) -> Result<Self, SqlCodecError> {
        Self::for_catalog_with_mode_and_join_policy(
            catalog,
            SyntaxTokenLayoutMode::PerGrammar,
            join_on_policy,
        )
    }

    /// Fully explicit compile: layout mode + join-ON policy.
    pub fn for_catalog_with_mode_and_join_policy(
        catalog: SqlCatalog,
        mode: SyntaxTokenLayoutMode,
        join_on_policy: SqlJoinOnPolicy,
    ) -> Result<Self, SqlCodecError> {
        Self::for_catalog_with_mode_and_policies(
            catalog,
            mode,
            join_on_policy,
            SqlColumnAddressing::Flat,
        )
    }

    /// Compile with both SQL menu policies stated (join-ON + column addressing).
    ///
    /// `Factored` column addressing changes how column ordinals are spelled
    /// (two positional digits instead of one radix code) without touching the
    /// token layout, so callers must treat a flip as a stream-contract break.
    pub fn for_catalog_with_policies(
        catalog: SqlCatalog,
        join_on_policy: SqlJoinOnPolicy,
        column_addressing: SqlColumnAddressing,
    ) -> Result<Self, SqlCodecError> {
        Self::for_catalog_with_mode_and_policies(
            catalog,
            SyntaxTokenLayoutMode::PerGrammar,
            join_on_policy,
            column_addressing,
        )
    }

    /// Fully explicit compile: layout mode + both menu policies.
    pub fn for_catalog_with_mode_and_policies(
        catalog: SqlCatalog,
        mode: SyntaxTokenLayoutMode,
        join_on_policy: SqlJoinOnPolicy,
        column_addressing: SqlColumnAddressing,
    ) -> Result<Self, SqlCodecError> {
        Self::for_catalog_with_mode_and_sql_policies(
            catalog,
            mode,
            join_on_policy,
            column_addressing,
            false,
        )
    }

    /// Compile with menu policies plus the rank-1 visit-all field policy.
    pub fn for_catalog_with_sql_policies(
        catalog: SqlCatalog,
        join_on_policy: SqlJoinOnPolicy,
        column_addressing: SqlColumnAddressing,
        visit_all_fields: bool,
    ) -> Result<Self, SqlCodecError> {
        Self::for_catalog_with_mode_and_sql_policies(
            catalog,
            SyntaxTokenLayoutMode::PerGrammar,
            join_on_policy,
            column_addressing,
            visit_all_fields,
        )
    }

    /// Fully explicit compile: layout mode + menu policies + visit-all fields.
    pub fn for_catalog_with_mode_and_sql_policies(
        catalog: SqlCatalog,
        mode: SyntaxTokenLayoutMode,
        join_on_policy: SqlJoinOnPolicy,
        column_addressing: SqlColumnAddressing,
        visit_all_fields: bool,
    ) -> Result<Self, SqlCodecError> {
        // Rank-1 visit-all wheel also folds select_list × distinct into a
        // 3-way alternative so that token is not a second free decision.
        let fold_select_distinct = visit_all_fields;
        let sidecar =
            lower_sql_schema_linking_grammar_with_fold_select_distinct(fold_select_distinct)
                .map_err(|e| {
                    SqlCodecError::new("lower", format!("SQL GrammarIR lower failed: {e}"))
                })?;
        let mut bindings =
            menu_bindings_from_catalog_with_policies(&catalog, join_on_policy, column_addressing)
                .map_err(|e| SqlCodecError::new("bind", format!("menu binding failed: {e}")))?;
        bindings.set_visit_all_remaining_fields(visit_all_fields);
        let model = CompiledSyntaxModel::from_sidecar_with_mode(&sidecar, mode)
            .map_err(|e| SqlCodecError::new("compile", format!("compile failed: {e}")))?
            .with_menu_bindings(bindings)
            .map_err(|e| SqlCodecError::new("bind", format!("attach menu bindings failed: {e}")))?;
        Ok(Self {
            catalog,
            model,
            fold_select_distinct,
        })
    }

    /// Fixture catalog under SharedAlphabet (T10).
    pub fn fixture_shared_alphabet() -> Result<Self, SqlCodecError> {
        let catalog = load_fixture_catalog()
            .map_err(|e| SqlCodecError::new("catalog", format!("fixture catalog: {e}")))?;
        Self::for_catalog_with_mode(catalog, SyntaxTokenLayoutMode::SharedAlphabet)
    }

    /// Compile with catalog bindings (T5 API retained; `table_index` validated only).
    pub fn for_single_table(
        catalog: SqlCatalog,
        table_index: usize,
    ) -> Result<Self, SqlCodecError> {
        if table_index >= catalog.tables.len() {
            return Err(SqlCodecError::new(
                "bind",
                format!(
                    "table index {table_index} out of range for catalog {}",
                    catalog.db_id
                ),
            ));
        }
        Self::for_catalog(catalog)
    }

    /// Load the in-repo fixture catalog and compile.
    pub fn fixture() -> Result<Self, SqlCodecError> {
        let catalog = load_fixture_catalog()
            .map_err(|e| SqlCodecError::new("catalog", format!("fixture catalog: {e}")))?;
        Self::for_catalog(catalog)
    }

    /// Load the in-repo fixture catalog and bind for `table_index` (compat).
    pub fn fixture_for_table(table_index: usize) -> Result<Self, SqlCodecError> {
        let catalog = load_fixture_catalog()
            .map_err(|e| SqlCodecError::new("catalog", format!("fixture catalog: {e}")))?;
        Self::for_single_table(catalog, table_index)
    }

    pub fn catalog(&self) -> &SqlCatalog {
        &self.catalog
    }

    pub fn model(&self) -> &CompiledSyntaxModel {
        &self.model
    }

    pub fn parse(&self, sql: &str) -> Result<SelectStatement, SqlCodecError> {
        parse_select(sql, &self.catalog).map_err(SqlCodecError::from)
    }

    /// M0-only parse helper (same as [`parse`] for current ladder).
    pub fn parse_m0(&self, sql: &str) -> Result<SelectStatement, SqlCodecError> {
        parse_m0_select(sql, &self.catalog).map_err(SqlCodecError::from)
    }

    pub fn render(&self, stmt: &SelectStatement) -> Result<String, SqlCodecError> {
        render_select(stmt, &self.catalog).map_err(SqlCodecError::from)
    }

    pub fn encode(&self, stmt: &SelectStatement) -> Result<Vec<u32>, SqlCodecError> {
        let tree =
            select_to_decoded_node_with_fold_select_distinct(stmt, self.fold_select_distinct);
        validate_literal_payloads(&tree)?;
        self.model
            .encode_complete_tree(&tree)
            .map_err(SqlCodecError::from)
    }

    pub fn decode(&self, tokens: &[u32]) -> Result<SelectStatement, SqlCodecError> {
        let decoded = self
            .model
            .decode_complete_tree(tokens)
            .map_err(SqlCodecError::from)?;
        let tree = decoded
            .decoded_node_tree()
            .map_err(|e| SqlCodecError::new("decode_tree", e.to_string()))?;
        select_from_decoded_node(&tree).map_err(SqlCodecError::from)
    }

    pub fn decode_tree(&self, tokens: &[u32]) -> Result<RawSyntaxDecodedNode, SqlCodecError> {
        let decoded = self
            .model
            .decode_complete_tree(tokens)
            .map_err(SqlCodecError::from)?;
        decoded
            .decoded_node_tree()
            .map_err(|e| SqlCodecError::new("decode_tree", e.to_string()))
    }

    /// Full M0–M3 round-trip: parse → encode → decode → render → reparse → re-encode.
    pub fn round_trip_source(&self, sql: &str) -> Result<SqlRoundTrip, SqlCodecError> {
        let stmt = self.parse(sql)?;
        let tokens = self.encode(&stmt)?;
        let pad = self.model.token_roles().pad_token;
        if tokens.iter().any(|&t| t >= pad) {
            return Err(SqlCodecError::new(
                "round_trip",
                format!("token stream contains id >= PAD {pad}"),
            ));
        }
        let decoded = self.decode(&tokens)?;
        if decoded != stmt {
            return Err(SqlCodecError::new(
                "round_trip",
                format!("decoded AST {decoded:?} diverged from parsed {stmt:?}"),
            ));
        }
        let encoded_tree =
            select_to_decoded_node_with_fold_select_distinct(&stmt, self.fold_select_distinct);
        let decoded_tree = self.decode_tree(&tokens)?;
        if decoded_tree != encoded_tree {
            return Err(SqlCodecError::new(
                "round_trip",
                "decoded raw tree diverged from encode tree (tree identity)",
            ));
        }
        let rendered = self.render(&decoded)?;
        let reparsed = self.parse(&rendered)?;
        if reparsed != stmt {
            return Err(SqlCodecError::new(
                "round_trip",
                format!("reparsed AST {reparsed:?} diverged from original {stmt:?}"),
            ));
        }
        let reencoded = self.encode(&reparsed)?;
        if reencoded != tokens {
            return Err(SqlCodecError::new(
                "round_trip",
                "re-encoded tokens diverged from the first encoding (render fixpoint)",
            ));
        }
        Ok(SqlRoundTrip {
            source: sql.to_string(),
            rendered,
            tokens,
            column_site_count: stmt.column_site_count(),
            claim_boundary: SQL_CODEC_CLAIM_BOUNDARY,
            subset_id: SQL_CODEC_SUBSET_ID,
        })
    }
}

/// Successful M0–M3 round-trip report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SqlRoundTrip {
    pub source: String,
    pub rendered: String,
    pub tokens: Vec<u32>,
    pub column_site_count: usize,
    pub claim_boundary: &'static str,
    pub subset_id: &'static str,
}

/// Reject values the SQL text renderer cannot represent, including AST callers
/// that bypass source parsing. Keep this restriction out of the generic codec.
fn validate_literal_payloads(node: &RawSyntaxDecodedNode) -> Result<(), SqlCodecError> {
    for field in &node.fields {
        for value in &field.values {
            match value {
                RawSyntaxDecodedValue::Node(child) => validate_literal_payloads(child)?,
                RawSyntaxDecodedValue::Terminal(terminal)
                    if node.rule_name == "string_literal" && terminal.payload.contains('\0') =>
                {
                    return Err(SqlCodecError::new(
                        "literal",
                        "string_literal.value contains NUL, which is not representable in SQLite SQL text",
                    ));
                }
                RawSyntaxDecodedValue::Terminal(_) => {}
            }
        }
    }
    Ok(())
}

/// Count menu-band tokens in a PerGrammar stream (exactly one per menu site).
///
/// **Dedicated disjoint menu band only.** Returns `None` when the model has no
/// menu band, or when the menu range coincides with the integer or utf8
/// payload range (SQL shared payload pool; SharedAlphabet u8 alias) — a
/// range-only scan cannot distinguish menu ordinals from other payload
/// tokens. Use [`count_menu_tokens_by_role_replay`] for shared-pool streams.
pub fn count_menu_tokens(tokens: &[u32], model: &CompiledSyntaxModel) -> Option<usize> {
    let roles = model.token_roles();
    let menu = roles.menu_range.as_ref()?;
    // Shared payload pool / SharedAlphabet alias: range-only is unsafe.
    let shares_integer = menu.start == roles.integer_range.start
        && menu.end_exclusive == roles.integer_range.end_exclusive;
    let shares_utf8 = menu.start == roles.utf8_range.start
        && menu.end_exclusive == roles.utf8_range.end_exclusive;
    if shares_integer || shares_utf8 {
        return None;
    }
    Some(
        tokens
            .iter()
            .filter(|&&t| t >= menu.start && t < menu.end_exclusive)
            .count(),
    )
}

/// Count menu-site payload tokens via decode-event replay (state-aware).
///
/// Safe under SharedAlphabet and the SQL shared payload pool because it keys
/// on menu bindings `(owner_rule, owner_field)`, not on token-ID ranges alone.
/// Prefer [`count_menu_tokens`] only when the menu band is disjoint from every
/// other payload role.
pub fn count_menu_tokens_by_role_replay(
    tokens: &[u32],
    model: &CompiledSyntaxModel,
) -> Result<usize, SqlCodecError> {
    use crate::syntax_state_machine::RawSyntaxDecodedEventKind;

    let tree = model
        .decode_complete_tree(tokens)
        .map_err(|e| SqlCodecError::new("menu_count_replay", format!("{e:?}")))?;
    let mut count = 0usize;
    for event in &tree.events {
        if event.event_kind != RawSyntaxDecodedEventKind::PayloadToken {
            continue;
        }
        // Menu payload tokens set value (ordinal string) without a UTF-8/u8
        // payload_token_byte; integer literals set the byte.
        if event.payload_token_byte.is_some() {
            continue;
        }
        let (Some(rule), Some(field)) = (event.rule.as_deref(), event.field.as_deref()) else {
            continue;
        };
        if model.menu_bindings().site_kind(rule, field).is_some() {
            count += 1;
        }
    }
    Ok(count)
}

/// Phase-tagged codec error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SqlCodecError {
    pub phase: &'static str,
    pub message: String,
}

impl SqlCodecError {
    pub fn new(phase: &'static str, message: impl Into<String>) -> Self {
        Self {
            phase,
            message: message.into(),
        }
    }
}

impl std::fmt::Display for SqlCodecError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SQL codec error ({}): {}", self.phase, self.message)
    }
}

impl std::error::Error for SqlCodecError {}

impl From<SqlParseError> for SqlCodecError {
    fn from(value: SqlParseError) -> Self {
        Self::new("parse", value.to_string())
    }
}

impl From<RenderError> for SqlCodecError {
    fn from(value: RenderError) -> Self {
        Self::new("render", value.to_string())
    }
}

impl From<TreeError> for SqlCodecError {
    fn from(value: TreeError) -> Self {
        Self::new("tree", value.to_string())
    }
}

impl From<RawSyntaxError> for SqlCodecError {
    fn from(value: RawSyntaxError) -> Self {
        Self::new("raw_sm", format!("{value:?}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sql_schema_linking::catalog::load_fixture_bundle;
    use crate::sql_schema_linking::layout::{
        SQL_SYNTAX_LAYOUT_PAD_TOKEN, SQL_SYNTAX_LAYOUT_PAD_TOKEN as PAD,
        SQL_SYNTAX_LAYOUT_VOCAB_SIZE, menu_band_range, sql_syntax_token_layout,
    };
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
        let file: GoldFile =
            serde_json::from_slice(&bundle.gold_queries_bytes).expect("parse gold");
        file.queries
            .into_iter()
            .find(|q| q.id == id)
            .unwrap_or_else(|| panic!("missing gold query {id}"))
            .sql
    }

    #[test]
    fn gold_select_one_table_round_trips_with_one_menu_token_per_column_site() {
        let catalog = load_fixture_catalog().expect("catalog");
        let customers_idx = catalog
            .tables
            .iter()
            .position(|t| t.name == "customers")
            .expect("customers");
        let codec = SqlSchemaLinkingCodec::for_single_table(catalog, customers_idx).expect("codec");

        let sql = gold_sql("select_one_table");
        let report = codec
            .round_trip_source(&sql)
            .unwrap_or_else(|e| panic!("round_trip failed: {e}"));

        assert!(
            report.tokens.iter().all(|&t| t < PAD),
            "every token must be strictly below PAD {PAD}"
        );
        assert_eq!(report.column_site_count, 2);
        // Shared payload pool: range-only menu count is None; use role replay.
        assert!(
            count_menu_tokens(&report.tokens, codec.model()).is_none(),
            "shared payload pool must refuse range-only menu counts"
        );
        let menu_count = count_menu_tokens_by_role_replay(&report.tokens, codec.model())
            .expect("role-replay menu count");
        assert_eq!(menu_count, 3, "1 table + 2 column menu tokens");
        assert_eq!(report.rendered, r#"SELECT "name", "city" FROM "customers""#);
        let again = codec
            .encode(&codec.parse(&report.rendered).expect("reparse"))
            .expect("reenc");
        assert_eq!(again, report.tokens);
        let menu = menu_band_range(&sql_syntax_token_layout()).expect("menu band");
        assert!(report.tokens.iter().any(|&t| t >= menu.0 && t < menu.1));
        assert_eq!(SQL_SYNTAX_LAYOUT_PAD_TOKEN, PAD);
        assert_eq!(PAD, 320, "shared payload pool PAD");
    }

    #[test]
    fn gold_star_structural_emits_no_column_menu_token() {
        let catalog = load_fixture_catalog().expect("catalog");
        let products_idx = catalog
            .tables
            .iter()
            .position(|t| t.name == "products")
            .expect("products");
        let codec = SqlSchemaLinkingCodec::for_single_table(catalog, products_idx).expect("codec");

        let sql = gold_sql("star_structural");
        let report = codec
            .round_trip_source(&sql)
            .unwrap_or_else(|e| panic!("star round_trip failed: {e}"));

        assert!(report.tokens.iter().all(|&t| t < PAD));
        assert_eq!(report.column_site_count, 0);
        let menu_count = count_menu_tokens_by_role_replay(&report.tokens, codec.model())
            .expect("role-replay menu count");
        assert_eq!(menu_count, 1, "star: only table menu token");
        assert_eq!(report.rendered, r#"SELECT * FROM "products""#);
    }

    /// Collect finite + ranged admitted token ids from an admissible mask.
    fn admitted_token_set(
        admissible: &crate::syntax_state_machine::RawSyntaxAdmissibleTokens,
    ) -> BTreeSet<u32> {
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

    /// True when the mask is a pure column-menu site of the expected cardinality.
    fn is_pure_column_menu_mask(admitted: &BTreeSet<u32>, menu_start: u32, card: u32) -> bool {
        admitted.len() == card as usize
            && admitted
                .iter()
                .all(|&t| t >= menu_start && t < menu_start + card)
    }

    #[test]
    fn gold_join_two_tables_column_mask_cardinality_is_seven_not_eleven() {
        // T6 gate: after FROM customers JOIN orders, every column-site mask has
        // cardinality 7 (not 11 raw Spider columns including *). Products
        // columns are not admissible. join_two_tables has four column sites
        // (two SELECT + two ON).
        use crate::sql_schema_linking::grammar::{COLUMN_MENU_OWNER_FIELD, COLUMN_MENU_OWNER_RULE};

        let catalog = load_fixture_catalog().expect("catalog");
        let codec = SqlSchemaLinkingCodec::for_catalog(catalog.clone()).expect("codec");
        let sql = gold_sql("join_two_tables");
        let stmt = codec.parse(&sql).expect("parse join");
        assert_eq!(stmt.scope_table_indices(), vec![1, 0]);

        let report = codec
            .round_trip_source(&sql)
            .unwrap_or_else(|e| panic!("join round_trip failed: {e}"));
        assert!(report.tokens.iter().all(|&t| t < PAD));

        let full = report.tokens.clone();
        let menu = menu_band_range(&sql_syntax_token_layout()).expect("menu band");
        let bindings = menu_bindings_from_catalog(&catalog).expect("bindings");
        let card = bindings
            .resolve_cardinality(COLUMN_MENU_OWNER_RULE, COLUMN_MENU_OWNER_FIELD, &[1, 0])
            .expect("column site");
        assert_eq!(card, 7);

        let mut column_sites = 0usize;
        for end in 0..full.len() {
            let prefix = &full[..end];
            let Ok(admissible) = codec.model().admissible_next_raw_tokens(prefix) else {
                panic!("join_two_tables proper prefix_len={end} must admit a mask");
            };
            let admitted = admitted_token_set(&admissible);
            if !is_pure_column_menu_mask(&admitted, menu.0, 7) {
                continue;
            }
            column_sites += 1;
            for prod_ord in 7..10 {
                let token = menu.0 + prod_ord;
                assert!(
                    !admitted.contains(&token),
                    "column site #{column_sites} prefix_len={end}: products ordinal {prod_ord} must not be admissible"
                );
                assert!(
                    !codec.model().transition(prefix, token).accepted,
                    "column site #{column_sites} prefix_len={end}: products ordinal {prod_ord} must be rejected by transition"
                );
            }
            // In-scope ordinals 0..7 must be mask-admitted and transition-accepted.
            for ord in 0..7u32 {
                let token = menu.0 + ord;
                assert!(
                    admitted.contains(&token),
                    "column site #{column_sites}: in-scope ordinal {ord} missing from mask"
                );
                assert!(
                    codec.model().transition(prefix, token).accepted,
                    "column site #{column_sites}: in-scope ordinal {ord} must be accepted"
                );
            }
        }
        assert_eq!(
            column_sites, 4,
            "join_two_tables must expose exactly four pure column-menu sites (2 SELECT + 2 ON)"
        );
        assert_eq!(
            bindings
                .site_kind(COLUMN_MENU_OWNER_RULE, COLUMN_MENU_OWNER_FIELD)
                .map(|k| k.band_limit()),
            Some(10),
            "catalog has 10 real columns (not 11 raw with *)"
        );
    }

    #[test]
    fn repeated_table_ordinal_rejected_by_mask_transition_encode_and_parse() {
        // Pre-M6: no occurrence aliases. `FROM orders JOIN orders` would render
        // both qualifiers as `orders` and reparse the second occurrence's
        // columns as the first — P8 render-fixpoint break. Fail closed in
        // parser, encode, mask, and transition so generated streams cannot
        // enter the non-renderable state.
        use crate::sql_schema_linking::ast::{
            CmpOp, ColumnRef, Expression, FromClause, JoinClause, JoinKind, SelectItem, SelectList,
            SelectStatement, TableRef,
        };
        use crate::sql_schema_linking::tree::select_to_decoded_node;

        let catalog = load_fixture_catalog().expect("catalog");
        let codec = SqlSchemaLinkingCodec::for_catalog(catalog).expect("codec");

        // Source parser rejects repeated table ordinals.
        let err = parse_select(
            "SELECT orders.amount FROM orders JOIN orders ON orders.order_id = orders.customer_id",
            codec.catalog(),
        )
        .expect_err("repeated table must fail parse");
        assert!(
            err.message.contains("repeated table") || err.message.contains("occurrence"),
            "unexpected parse error: {err}"
        );

        // Encode rejects a decoded tree with duplicate table ordinals.
        let dup = SelectStatement {
            from: FromClause {
                head: TableRef { menu_index: 0 },
                joins: vec![JoinClause {
                    kind: JoinKind::Inner,
                    table: TableRef { menu_index: 0 },
                    on: Expression::Comparison {
                        left: Box::new(Expression::Column(ColumnRef { menu_index: 0 })),
                        op: CmpOp::Eq,
                        right: Box::new(Expression::Column(ColumnRef { menu_index: 1 })),
                    },
                }],
            },
            where_clause: None,
            group_by: None,
            having: None,
            order_by: None,
            limit: None,
            select_list: SelectList::Items(crate::sql_schema_linking::ast::SelectItems {
                distinct: false,
                items: vec![SelectItem::Column(ColumnRef { menu_index: 3 })],
            }),
        };
        let tree = select_to_decoded_node(&dup);
        let enc_err = codec
            .model()
            .encode_complete_tree(&tree)
            .expect_err("duplicate table ordinal must fail encode");
        let enc_msg = format!("{enc_err:?}");
        assert!(
            enc_msg.contains("duplicate") || enc_msg.contains("already in scope"),
            "unexpected encode error: {enc_msg}"
        );

        // Token stream: after FROM orders is committed, the next table site
        // (JOIN) must not admit ordinal 0 (orders) in mask or transition.
        let valid = codec
            .parse(
                "SELECT customers.name, orders.amount FROM customers JOIN orders ON customers.customer_id = orders.customer_id",
            )
            .expect("valid join parse");
        // Encode a head-only statement, then walk a valid join stream and
        // inspect the second table-site mask.
        let join_tokens = codec.encode(&valid).expect("encode valid join");
        let menu = menu_band_range(&sql_syntax_token_layout()).expect("menu band");
        // customers=1, orders=0 in catalog order. First table token is
        // customers (1); second is orders (0). At the second table site,
        // customers is already in scope so ordinal 1 must be excluded and
        // ordinal 0 (orders) admissible. Also prove that re-choosing the
        // already-scoped ordinal is rejected: build prefix ending at first
        // table site of a self-join-shaped stream by replaying until the
        // second pure table-menu mask.
        let mut found_second_table_site = false;
        let mut table_sites = 0usize;
        for end in 0..join_tokens.len() {
            let prefix = &join_tokens[..end];
            let Ok(admissible) = codec.model().admissible_next_raw_tokens(prefix) else {
                panic!("valid join proper prefix_len={end} must admit a mask");
            };
            let admitted = admitted_token_set(&admissible);
            // Pure table-menu site: exactly catalog table count (3) or fewer
            // when some ordinals are already scoped out.
            let menu_only: Vec<u32> = admitted
                .iter()
                .copied()
                .filter(|t| *t >= menu.0 && *t < menu.1)
                .collect();
            if menu_only.is_empty() || admitted.len() != menu_only.len() {
                continue;
            }
            // Table sites admit at most 3 ordinals (catalog tables).
            if menu_only.iter().any(|&t| t >= menu.0 + 3) {
                continue;
            }
            // Column sites have card 3+; distinguish: table site after head
            // has card 2 (one table already in scope) or 3 (empty scope).
            if menu_only.len() > 3 {
                continue;
            }
            // Heuristic: pure menu-only with card <= 3 and all ordinals < 3.
            if !menu_only.iter().all(|&t| t < menu.0 + 3) {
                continue;
            }
            // Skip column sites of card 3 (single-table scope) by requiring
            // that this is either empty-scope (card 3) or post-head (card 2)
            // table site: for join_two_tables first table site has card 3,
            // second has card 2 with customers (1) excluded.
            table_sites += 1;
            if table_sites == 2 {
                // customers (ordinal 1) already in scope — must not be admitted.
                let customers_token = menu.0 + 1;
                assert!(
                    !admitted.contains(&customers_token),
                    "second table site must exclude already-scoped customers ordinal"
                );
                assert!(
                    !codec.model().transition(prefix, customers_token).accepted,
                    "second table site must reject customers ordinal via transition"
                );
                // orders (0) and products (2) remain admissible.
                assert!(admitted.contains(&(menu.0)));
                assert!(admitted.contains(&(menu.0 + 2)));
                // Explicitly reject re-selecting customers even if forced.
                assert!(!codec.model().transition(prefix, customers_token).accepted);
                found_second_table_site = true;
                break;
            }
        }
        assert!(
            found_second_table_site,
            "expected a second table-menu site after FROM head"
        );
    }

    #[test]
    fn structural_join_on_narrows_exactly_the_right_on_column_site() {
        // Fixture scope order for this query: [customers(3 cols), orders(4)].
        // Scope-relative ordinals: customers 0..3, orders 3..7. Left side names
        // customers (earlier table), so the RIGHT side of the equality join key
        // must admit exactly orders' slice [3, 7) — and nothing else changes,
        // including the WHERE-clause comparison sites.
        let off = SqlSchemaLinkingCodec::for_catalog(load_fixture_catalog().expect("catalog"))
            .expect("off");
        let structural = SqlSchemaLinkingCodec::for_catalog_with_join_policy(
            load_fixture_catalog().expect("catalog"),
            SqlJoinOnPolicy::Structural,
        )
        .expect("structural");
        let sql = "SELECT customers.name, orders.amount FROM customers JOIN orders \
                   ON customers.customer_id = orders.customer_id \
                   WHERE customers.city = customers.name";
        let tokens = structural
            .encode(&structural.parse(sql).expect("parse"))
            .expect("encode structural");
        let tokens_off = off
            .encode(&off.parse(sql).expect("parse"))
            .expect("encode off");
        assert_eq!(
            tokens, tokens_off,
            "expanded gold streams must be policy-invariant"
        );
        let menu = menu_band_range(&sql_syntax_token_layout()).expect("menu band");

        let mut narrowed_sites = 0usize;
        for end in 0..tokens.len() {
            let prefix = &tokens[..end];
            // Gold stream stays accepted under the structural policy.
            assert!(
                structural.model().transition(prefix, tokens[end]).accepted,
                "gold token at {end} must stay accepted under structural"
            );
            let mask_off = admitted_token_set(
                &off.model()
                    .admissible_next_raw_tokens(prefix)
                    .expect("off mask"),
            );
            let mask_structural = admitted_token_set(
                &structural
                    .model()
                    .admissible_next_raw_tokens(prefix)
                    .expect("structural mask"),
            );
            if mask_structural == mask_off {
                continue;
            }
            narrowed_sites += 1;
            assert!(
                mask_structural.is_subset(&mask_off),
                "structural mask must never widen (prefix_len={end})"
            );
            let expected: std::collections::BTreeSet<u32> =
                (3u32..7).map(|ordinal| menu.0 + ordinal).collect();
            assert_eq!(
                mask_structural, expected,
                "right-ON mask must be exactly the joined table's column slice (prefix_len={end})"
            );
            // Same-table pair (customers.city, ordinal 2) is the measured A5
            // signature failure: rejected under structural, accepted under off.
            let violating = menu.0 + 2;
            assert!(mask_off.contains(&violating));
            assert!(
                !structural.model().transition(prefix, violating).accepted,
                "same-table join key must be rejected under structural"
            );
            assert!(off.model().transition(prefix, violating).accepted);
        }
        assert_eq!(
            narrowed_sites, 1,
            "exactly one site (the ON right column) narrows under structural"
        );
    }

    #[test]
    fn fk_declared_narrows_right_on_to_the_unique_fk_column() {
        // Same query as the structural test. Scope [customers(3), orders(4)]
        // → flat ordinals customers 0..3, orders 3..7. Unique FK
        // orders.customer_id → customers.customer_id, so the right-ON menu
        // must be exactly orders.customer_id (flat 4), not the whole orders
        // slice. Gold that uses that FK still encodes. orders.amount (flat 6)
        // stays legal under structural and is rejected here.
        let structural = SqlSchemaLinkingCodec::for_catalog_with_join_policy(
            load_fixture_catalog().expect("catalog"),
            SqlJoinOnPolicy::Structural,
        )
        .expect("structural");
        let fk = SqlSchemaLinkingCodec::for_catalog_with_join_policy(
            load_fixture_catalog().expect("catalog"),
            SqlJoinOnPolicy::FkDeclared,
        )
        .expect("fk_declared");
        let sql = "SELECT customers.name, orders.amount FROM customers JOIN orders \
                   ON customers.customer_id = orders.customer_id \
                   WHERE customers.city = customers.name";
        let tokens = fk
            .encode(&fk.parse(sql).expect("parse"))
            .expect("encode fk gold");
        let tokens_structural = structural
            .encode(&structural.parse(sql).expect("parse"))
            .expect("encode structural gold");
        assert_eq!(
            tokens, tokens_structural,
            "expanded gold streams must stay token-identical; only the mask shrinks"
        );
        let menu = menu_band_range(&sql_syntax_token_layout()).expect("menu band");
        let mut narrowed_sites = 0usize;
        for end in 0..tokens.len() {
            let prefix = &tokens[..end];
            assert!(
                fk.model().transition(prefix, tokens[end]).accepted,
                "gold token at {end} must stay accepted under fk_declared"
            );
            let mask_structural = admitted_token_set(
                &structural
                    .model()
                    .admissible_next_raw_tokens(prefix)
                    .expect("structural mask"),
            );
            let mask_fk = admitted_token_set(
                &fk.model()
                    .admissible_next_raw_tokens(prefix)
                    .expect("fk mask"),
            );
            if mask_fk == mask_structural {
                continue;
            }
            narrowed_sites += 1;
            assert!(
                mask_fk.is_subset(&mask_structural),
                "fk_declared must never widen vs structural (prefix_len={end})"
            );
            let expected: std::collections::BTreeSet<u32> = std::iter::once(menu.0 + 4).collect();
            assert_eq!(
                mask_fk, expected,
                "right-ON mask must be exactly orders.customer_id (flat 4)"
            );
            let amount = menu.0 + 6;
            assert!(mask_structural.contains(&amount));
            assert!(
                !fk.model().transition(prefix, amount).accepted,
                "non-FK orders.amount must be rejected under fk_declared"
            );
        }
        assert_eq!(
            narrowed_sites, 1,
            "exactly one site (the ON right column) narrows vs structural"
        );
        // AST encode writes declared ordinals without the live mask; decode
        // is the contract. A non-FK right column (orders.amount, flat 6) is
        // already shown rejected at the narrowed site above.
    }

    #[test]
    fn structural_join_on_left_joined_table_restricts_right_to_earlier_tables() {
        // Left side names the JOINED table (orders), so the right side must
        // come from an earlier in-scope table: customers' slice [0, 3).
        let structural = SqlSchemaLinkingCodec::for_catalog_with_join_policy(
            load_fixture_catalog().expect("catalog"),
            SqlJoinOnPolicy::Structural,
        )
        .expect("structural");
        let off = SqlSchemaLinkingCodec::for_catalog(load_fixture_catalog().expect("catalog"))
            .expect("off");
        let sql = "SELECT customers.name FROM customers JOIN orders \
                   ON orders.customer_id = customers.customer_id";
        let tokens = structural
            .encode(&structural.parse(sql).expect("parse"))
            .expect("encode");
        let menu = menu_band_range(&sql_syntax_token_layout()).expect("menu band");
        let mut narrowed_sites = 0usize;
        for end in 0..tokens.len() {
            let prefix = &tokens[..end];
            assert!(structural.model().transition(prefix, tokens[end]).accepted);
            let mask_off = admitted_token_set(
                &off.model()
                    .admissible_next_raw_tokens(prefix)
                    .expect("off mask"),
            );
            let mask_structural = admitted_token_set(
                &structural
                    .model()
                    .admissible_next_raw_tokens(prefix)
                    .expect("structural mask"),
            );
            if mask_structural == mask_off {
                continue;
            }
            narrowed_sites += 1;
            let expected: std::collections::BTreeSet<u32> =
                (0u32..3).map(|ordinal| menu.0 + ordinal).collect();
            assert_eq!(
                mask_structural, expected,
                "right side must be restricted to earlier tables' columns"
            );
            // orders.amount (ordinal 6) would be a same-table pair: rejected.
            assert!(
                !structural.model().transition(prefix, menu.0 + 6).accepted,
                "joined-table right side must be rejected when left named the joined table"
            );
        }
        assert_eq!(narrowed_sites, 1);
    }

    #[test]
    fn p10_menu_less_models_stay_bit_identical_after_scope_plumbing() {
        use crate::grammar_ir_structural_model::lower_grammar_ir_to_structural_model;
        use crate::toy_grammar_frontend::ToyGrammarSpec;

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
        assert_eq!(model_a.token_roles(), model_b.token_roles());
        let mask_a = model_a
            .admissible_next_raw_tokens(&[])
            .expect("empty prefix a");
        let mask_b = model_b
            .admissible_next_raw_tokens(&[])
            .expect("empty prefix b");
        assert_eq!(mask_a, mask_b);
    }

    #[test]
    fn gold_prefixes_mask_equals_accept_over_full_vocab() {
        // T7: for every gold stream prefix, every token id in 0..vocab_size is
        // accepted by transition iff it appears in the admissible mask.
        // Proper prefixes must never return an error; completed streams must
        // reject the full vocab range (not a truncated sample).
        let codec = SqlSchemaLinkingCodec::fixture().expect("codec");
        let vocab = codec.model().token_roles().vocab_size;
        let gold_ids = [
            "select_one_table",
            "star_structural",
            "join_two_tables",
            "three_table_join",
            "where_eq_string",
            "where_cmp_and_or",
            "join_with_where",
            "group_order_limit",
            "where_group_having_order_limit",
            "count_star",
            "sum_aggregate",
            "select_distinct",
            "scalar_subquery",
        ];
        for id in gold_ids {
            let sql = gold_sql(id);
            let tokens = codec
                .encode(
                    &codec
                        .parse(&sql)
                        .unwrap_or_else(|e| panic!("{id} parse: {e}")),
                )
                .unwrap_or_else(|e| panic!("{id} encode: {e}"));
            // Include empty prefix and every proper prefix of the complete stream.
            for end in 0..=tokens.len() {
                let prefix = &tokens[..end];
                match codec.model().admissible_next_raw_tokens(prefix) {
                    Ok(admissible) => {
                        let admitted = admitted_token_set(&admissible);
                        for candidate in 0..vocab {
                            let in_mask = admitted.contains(&candidate);
                            let accepted = codec.model().transition(prefix, candidate).accepted;
                            assert_eq!(
                                in_mask, accepted,
                                "{id} prefix_len={end}: token {candidate} mask={in_mask} accept={accepted}"
                            );
                        }
                    }
                    Err(err) => {
                        assert_eq!(
                            end,
                            tokens.len(),
                            "{id} proper prefix_len={end} must not error on admissible mask: {err:?}"
                        );
                        for candidate in 0..vocab {
                            assert!(
                                !codec.model().transition(prefix, candidate).accepted,
                                "{id} complete stream must reject token {candidate}"
                            );
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn t8_m2_m3_gold_queries_round_trip() {
        // T8 gate: WHERE/GROUP/HAVING/ORDER/LIMIT golds round-trip (tree
        // identity + render fixpoint, **P8**).
        let codec = SqlSchemaLinkingCodec::fixture().expect("codec");
        let cases = [
            (
                "where_eq_string",
                r#"SELECT "name" FROM "customers" WHERE "city" = 'Springfield'"#,
            ),
            (
                "where_cmp_and_or",
                r#"SELECT "name", "city" FROM "customers" WHERE "customer_id" > 0 AND "city" = 'Springfield' OR "city" = 'Shelbyville'"#,
            ),
            (
                "join_with_where",
                r#"SELECT "customers"."name", "orders"."amount" FROM "customers" JOIN "orders" ON "customers"."customer_id" = "orders"."customer_id" WHERE "orders"."amount" >= 100"#,
            ),
            (
                "group_order_limit",
                r#"SELECT "city" FROM "customers" GROUP BY "city" ORDER BY "city" ASC LIMIT 5"#,
            ),
            (
                "where_group_having_order_limit",
                r#"SELECT "city" FROM "customers" WHERE "customer_id" > 0 GROUP BY "city" HAVING "city" = 'Springfield' ORDER BY "city" DESC LIMIT 10"#,
            ),
        ];
        for (id, expected_render) in cases {
            let sql = gold_sql(id);
            let report = codec
                .round_trip_source(&sql)
                .unwrap_or_else(|e| panic!("{id} round_trip failed: {e}"));
            assert!(report.tokens.iter().all(|&t| t < PAD), "{id}: token >= PAD");
            assert_eq!(report.rendered, expected_render, "{id}: render diverged");
            assert_eq!(report.subset_id, SQL_CODEC_SUBSET_ID, "{id}: subset id");
        }
    }

    #[test]
    fn order_by_count_star_round_trips() {
        // Deliberate subset expansion: ORDER BY COUNT(*) (structural, no menu
        // token). Lifts residual ORDER_BY_AGG class for count(*) only — not
        // HAVING COUNT / COUNT(DISTINCT).
        let codec = SqlSchemaLinkingCodec::fixture().expect("codec");
        let sql = "SELECT city FROM customers GROUP BY city ORDER BY COUNT(*) DESC LIMIT 1";
        let report = codec
            .round_trip_source(sql)
            .unwrap_or_else(|e| panic!("order_by count star round_trip failed: {e}"));
        assert!(report.tokens.iter().all(|&t| t < PAD));
        assert_eq!(
            report.rendered,
            r#"SELECT "city" FROM "customers" GROUP BY "city" ORDER BY COUNT(*) DESC LIMIT 1"#
        );
    }

    #[test]
    fn order_by_sum_column_round_trips() {
        let codec = SqlSchemaLinkingCodec::fixture().expect("codec");
        let sql = "SELECT city FROM customers GROUP BY city ORDER BY SUM(customer_id) DESC LIMIT 1";
        let report = codec
            .round_trip_source(sql)
            .unwrap_or_else(|e| panic!("order_by sum round_trip failed: {e}"));
        assert!(report.tokens.iter().all(|&t| t < PAD));
        assert_eq!(
            report.rendered,
            r#"SELECT "city" FROM "customers" GROUP BY "city" ORDER BY SUM("customer_id") DESC LIMIT 1"#
        );
    }

    #[test]
    fn having_count_star_round_trips() {
        let codec = SqlSchemaLinkingCodec::fixture().expect("codec");
        let sql = "SELECT city FROM customers GROUP BY city HAVING COUNT(*) > 1 ORDER BY city ASC";
        let report = codec
            .round_trip_source(sql)
            .unwrap_or_else(|e| panic!("having count star round_trip failed: {e}"));
        assert!(report.tokens.iter().all(|&t| t < PAD));
        assert_eq!(
            report.rendered,
            r#"SELECT "city" FROM "customers" GROUP BY "city" HAVING COUNT(*) > 1 ORDER BY "city" ASC"#
        );
    }

    #[test]
    fn not_in_subquery_round_trips() {
        // Residual IN lift: col NOT IN (SELECT …) with uncorrelated inner scope.
        let codec = SqlSchemaLinkingCodec::fixture().expect("codec");
        let sql =
            "SELECT name FROM customers WHERE customer_id NOT IN (SELECT customer_id FROM orders)";
        let report = codec
            .round_trip_source(sql)
            .unwrap_or_else(|e| panic!("not in subquery round_trip failed: {e}"));
        assert!(report.tokens.iter().all(|&t| t < PAD));
        assert_eq!(
            report.rendered,
            r#"SELECT "name" FROM "customers" WHERE "customer_id" NOT IN (SELECT "customer_id" FROM "orders")"#
        );
    }

    #[test]
    fn in_subquery_round_trips() {
        let codec = SqlSchemaLinkingCodec::fixture().expect("codec");
        let sql =
            "SELECT name FROM customers WHERE customer_id IN (SELECT customer_id FROM orders)";
        let report = codec
            .round_trip_source(sql)
            .unwrap_or_else(|e| panic!("in subquery round_trip failed: {e}"));
        assert!(report.tokens.iter().all(|&t| t < PAD));
        assert_eq!(
            report.rendered,
            r#"SELECT "name" FROM "customers" WHERE "customer_id" IN (SELECT "customer_id" FROM "orders")"#
        );
    }

    #[test]
    fn count_distinct_column_round_trips() {
        // Residual COUNT_DISTINCT lift: COUNT(DISTINCT col) on select list.
        let codec = SqlSchemaLinkingCodec::fixture().expect("codec");
        let sql = "SELECT COUNT(DISTINCT city) FROM customers";
        let report = codec
            .round_trip_source(sql)
            .unwrap_or_else(|e| panic!("count distinct round_trip failed: {e}"));
        assert!(report.tokens.iter().all(|&t| t < PAD));
        assert_eq!(
            report.rendered,
            r#"SELECT COUNT(DISTINCT "city") FROM "customers""#
        );
    }

    #[test]
    fn having_avg_column_round_trips() {
        // Residual HAVING aggregate lift: AVG/SUM/MIN/MAX/COUNT(col) in expression position.
        let codec = SqlSchemaLinkingCodec::fixture().expect("codec");
        let sql = "SELECT city FROM customers GROUP BY city HAVING AVG(customer_id) > 1";
        let report = codec
            .round_trip_source(sql)
            .unwrap_or_else(|e| panic!("having avg round_trip failed: {e}"));
        assert!(report.tokens.iter().all(|&t| t < PAD));
        assert_eq!(
            report.rendered,
            r#"SELECT "city" FROM "customers" GROUP BY "city" HAVING AVG("customer_id") > 1"#
        );
    }

    #[test]
    fn between_u8_bounds_round_trips() {
        // BETWEEN residual lift when bounds fit SharedAlphabet u8 band.
        let codec = SqlSchemaLinkingCodec::fixture().expect("codec");
        let sql = "SELECT name FROM customers WHERE customer_id BETWEEN 1 AND 10";
        let report = codec
            .round_trip_source(sql)
            .unwrap_or_else(|e| panic!("between round_trip failed: {e}"));
        assert!(report.tokens.iter().all(|&t| t < PAD));
        assert_eq!(
            report.rendered,
            r#"SELECT "name" FROM "customers" WHERE "customer_id" BETWEEN 1 AND 10"#
        );
    }

    #[test]
    fn limit_300_multi_digit_round_trips() {
        let codec = SqlSchemaLinkingCodec::fixture().expect("codec");
        let sql = "SELECT name FROM customers LIMIT 300";
        let report = codec
            .round_trip_source(sql)
            .unwrap_or_else(|e| panic!("LIMIT 300 round_trip failed: {e}"));
        assert!(report.tokens.iter().all(|&t| t < PAD));
        assert_eq!(
            report.rendered,
            r#"SELECT "name" FROM "customers" LIMIT 300"#
        );
        // Token fixpoint: re-encode decoded stream.
        let again = codec
            .round_trip_source(&report.rendered)
            .expect("second round_trip");
        assert_eq!(again.tokens, report.tokens);
    }

    #[test]
    fn integer_literal_2014_multi_digit_round_trips() {
        let codec = SqlSchemaLinkingCodec::fixture().expect("codec");
        let sql = "SELECT name FROM customers WHERE customer_id = 2014";
        let report = codec
            .round_trip_source(sql)
            .unwrap_or_else(|e| panic!("literal 2014 round_trip failed: {e}"));
        assert_eq!(
            report.rendered,
            r#"SELECT "name" FROM "customers" WHERE "customer_id" = 2014"#
        );
        let again = codec.round_trip_source(&report.rendered).expect("fixpoint");
        assert_eq!(again.tokens, report.tokens);
    }

    #[test]
    fn single_digit_0_to_253_round_trips() {
        let codec = SqlSchemaLinkingCodec::fixture().expect("codec");
        for n in [0u32, 1, 42, 253] {
            let sql = format!("SELECT name FROM customers LIMIT {n}");
            let report = codec
                .round_trip_source(&sql)
                .unwrap_or_else(|e| panic!("LIMIT {n} failed: {e}"));
            assert_eq!(
                report.rendered,
                format!(r#"SELECT "name" FROM "customers" LIMIT {n}"#)
            );
        }
    }

    #[test]
    fn integer_capacity_limit_fails_closed() {
        let codec = SqlSchemaLinkingCodec::fixture().expect("codec");
        let err = codec
            .round_trip_source("SELECT name FROM customers LIMIT 16777216")
            .expect_err("over-capacity LIMIT must fail");
        let msg = err.to_string();
        assert!(
            msg.contains("capacity") || msg.contains("16777215") || msg.contains("integer"),
            "unexpected error: {msg}"
        );
    }

    #[test]
    fn in_value_list_round_trips() {
        // Value-list IN residual lift (u8 integer atoms).
        let codec = SqlSchemaLinkingCodec::fixture().expect("codec");
        let sql = "SELECT name FROM customers WHERE customer_id IN (1, 2, 3)";
        let report = codec
            .round_trip_source(sql)
            .unwrap_or_else(|e| panic!("in value list round_trip failed: {e}"));
        assert!(report.tokens.iter().all(|&t| t < PAD));
        assert_eq!(
            report.rendered,
            r#"SELECT "name" FROM "customers" WHERE "customer_id" IN (1, 2, 3)"#
        );
    }

    #[test]
    fn not_in_string_value_list_round_trips() {
        let codec = SqlSchemaLinkingCodec::fixture().expect("codec");
        let sql = "SELECT name FROM customers WHERE city NOT IN ('A', 'B')";
        let report = codec
            .round_trip_source(sql)
            .unwrap_or_else(|e| panic!("not in string list round_trip failed: {e}"));
        assert!(report.tokens.iter().all(|&t| t < PAD));
        assert_eq!(
            report.rendered,
            r#"SELECT "name" FROM "customers" WHERE "city" NOT IN ('A', 'B')"#
        );
    }

    #[test]
    fn t9_count_star_emits_no_column_menu_token() {
        // T9 gate: COUNT(*) is structural — only the table menu token.
        let codec = SqlSchemaLinkingCodec::fixture().expect("codec");
        let sql = gold_sql("count_star");
        let report = codec
            .round_trip_source(&sql)
            .unwrap_or_else(|e| panic!("count_star round_trip failed: {e}"));
        assert_eq!(report.column_site_count, 0);
        let menu_count = count_menu_tokens_by_role_replay(&report.tokens, codec.model())
            .expect("role-replay menu count");
        assert_eq!(menu_count, 1, "COUNT(*): only table menu token");
        assert_eq!(report.rendered, r#"SELECT COUNT(*) FROM "customers""#);
    }

    #[test]
    fn t9_distinct_and_sum_round_trip() {
        let codec = SqlSchemaLinkingCodec::fixture().expect("codec");
        let sum = codec
            .round_trip_source(&gold_sql("sum_aggregate"))
            .expect("sum");
        assert_eq!(sum.rendered, r#"SELECT SUM("amount") FROM "orders""#);
        let menu = count_menu_tokens_by_role_replay(&sum.tokens, codec.model())
            .expect("role-replay menu count");
        assert_eq!(menu, 2, "SUM(amount): table + one column menu token");

        let dist = codec
            .round_trip_source(&gold_sql("select_distinct"))
            .expect("distinct");
        assert_eq!(dist.rendered, r#"SELECT DISTINCT "city" FROM "customers""#);
    }

    #[test]
    fn t10_shared_alphabet_gold_round_trips_without_one_off_codec() {
        // T10: SharedAlphabet compile + menu-on-u8-band (**P9**); no bespoke codec.
        use crate::shared_alphabet_layout::{
            SHARED_ALPHABET_CLAIM_BOUNDARY, SHARED_ALPHABET_U8_INTEGER_PAYLOAD_WIDTH,
            SyntaxTokenLayoutMode,
        };

        let codec = SqlSchemaLinkingCodec::fixture_shared_alphabet().expect("shared codec");
        assert_eq!(
            codec.model().layout_mode(),
            SyntaxTokenLayoutMode::SharedAlphabet
        );
        assert_eq!(
            SHARED_ALPHABET_CLAIM_BOUNDARY,
            "compiler_shared_alphabet_layout_only"
        );
        let menu = codec
            .model()
            .token_roles()
            .menu_range
            .as_ref()
            .expect("SharedAlphabet menus alias u8_integer_payload");
        assert_eq!(
            menu.end_exclusive - menu.start,
            SHARED_ALPHABET_U8_INTEGER_PAYLOAD_WIDTH
        );
        assert_eq!(menu.start, codec.model().token_roles().integer_range.start);

        for id in [
            "select_one_table",
            "star_structural",
            "join_two_tables",
            "count_star",
            "where_eq_string",
            "group_order_limit",
            "scalar_subquery",
        ] {
            let report = codec
                .round_trip_source(&gold_sql(id))
                .unwrap_or_else(|e| panic!("SharedAlphabet {id} round_trip failed: {e}"));
            let pad = codec.model().token_roles().pad_token;
            assert!(
                report.tokens.iter().all(|&t| t < pad),
                "SharedAlphabet {id}: token >= PAD"
            );
        }
    }

    #[test]
    fn t10_shared_alphabet_mask_equals_accept_on_gold_prefix() {
        // T10: mask/accept consistency under SharedAlphabet for one gold stream.
        // Same non-vacuous/full-vocab rules as T7 (no silent continue on proper
        // prefixes; completed stream rejects entire 0..vocab).
        let codec = SqlSchemaLinkingCodec::fixture_shared_alphabet().expect("shared codec");
        let vocab = codec.model().token_roles().vocab_size;
        let tokens = codec
            .encode(&codec.parse(&gold_sql("select_one_table")).expect("parse"))
            .expect("encode");
        for end in 0..=tokens.len() {
            let prefix = &tokens[..end];
            match codec.model().admissible_next_raw_tokens(prefix) {
                Ok(admissible) => {
                    let admitted = admitted_token_set(&admissible);
                    for candidate in 0..vocab {
                        let in_mask = admitted.contains(&candidate);
                        let accepted = codec.model().transition(prefix, candidate).accepted;
                        assert_eq!(
                            in_mask, accepted,
                            "SharedAlphabet select_one_table prefix_len={end}: token {candidate}"
                        );
                    }
                }
                Err(err) => {
                    assert_eq!(
                        end,
                        tokens.len(),
                        "SharedAlphabet proper prefix_len={end} must not error: {err:?}"
                    );
                    for candidate in 0..vocab {
                        assert!(
                            !codec.model().transition(prefix, candidate).accepted,
                            "SharedAlphabet complete stream must reject token {candidate}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn count_menu_tokens_rejects_shared_alphabet_range_only_and_role_replay_distinguishes() {
        // T10/T11 + shared payload pool: when menu aliases other payload
        // ranges, range-only count is unsafe. Role replay must still count
        // menu sites and must not count integer-literal payload tokens as menus.
        let per = SqlSchemaLinkingCodec::fixture().expect("per");
        let shared = SqlSchemaLinkingCodec::fixture_shared_alphabet().expect("shared");
        // Gold with both a menu reference and an integer literal.
        let with_int = "SELECT name FROM customers WHERE customer_id > 0";
        let per_rt = per.round_trip_source(with_int).expect("per rt");
        let shared_rt = shared.round_trip_source(with_int).expect("shared rt");

        // SQL PerGrammar now uses the shared payload pool, so range-only is None.
        assert!(
            count_menu_tokens(&per_rt.tokens, per.model()).is_none(),
            "SQL shared payload pool must refuse range-only menu counts"
        );
        let per_count = count_menu_tokens_by_role_replay(&per_rt.tokens, per.model())
            .expect("per role-replay count");
        assert!(
            count_menu_tokens(&shared_rt.tokens, shared.model()).is_none(),
            "SharedAlphabet must not use range-only menu counts"
        );
        let shared_replay =
            count_menu_tokens_by_role_replay(&shared_rt.tokens, shared.model()).expect("replay");
        assert_eq!(
            shared_replay, per_count,
            "role replay under SharedAlphabet must match PerGrammar menu count"
        );
        // with_int: 1 table + 2 columns (name, customer_id).
        assert_eq!(per_count, 3, "table + name + customer_id menus");
        // Sanity: range-only under SharedAlphabet would over-count if forced
        // (menu band == integer band includes the literal 0 token).
        let roles = shared.model().token_roles();
        let menu = roles.menu_range.as_ref().expect("menu");
        let naive = shared_rt
            .tokens
            .iter()
            .filter(|&&t| t >= menu.start && t < menu.end_exclusive)
            .count();
        assert!(
            naive > shared_replay,
            "naive range scan must over-count vs role replay when integer literals are present (naive={naive} replay={shared_replay})"
        );
    }

    #[test]
    fn t10_shared_alphabet_menu_above_256_fails_closed() {
        // T10 / P9: catalogs whose column band limit exceeds the u8 integer
        // width (256) must fail at binding under SharedAlphabet.
        use crate::shared_alphabet_layout::SyntaxTokenLayoutMode;
        use crate::syntax_state_machine::MenuSiteBindings;

        let sidecar = lower_sql_schema_linking_grammar().expect("lower");
        let model = CompiledSyntaxModel::from_sidecar_with_mode(
            &sidecar,
            SyntaxTokenLayoutMode::SharedAlphabet,
        )
        .expect("compile shared");
        let mut bindings = MenuSiteBindings::new();
        // Synthetic site with band limit 257 > 256 u8 ceiling.
        bindings.insert("column_ref", "menu_index", 257);
        let err = model
            .with_menu_bindings(bindings)
            .expect_err("257 must overflow SharedAlphabet menu band");
        let msg = err.to_string();
        assert!(
            msg.contains("overflow") || msg.contains("257") || msg.contains("256"),
            "unexpected error: {msg}"
        );
    }

    #[test]
    fn encode_nested_subquery_scope_is_root_local_not_global() {
        // Nested uncorrelated subquery must open a fresh encode scope and must
        // not leak its tables into (or inherit tables from) the outer scope.
        // Regression for encode_complete_tree previously threading one global
        // scope_tables Vec through recursive records (**P5** / P10).
        use crate::sql_schema_linking::ast::{
            CmpOp, ColumnRef, Expression, FromClause, SelectItem, SelectList, SelectStatement,
            TableRef,
        };
        use crate::sql_schema_linking::grammar::{COLUMN_MENU_OWNER_FIELD, COLUMN_MENU_OWNER_RULE};
        use crate::sql_schema_linking::tree::select_to_decoded_node;

        let catalog = load_fixture_catalog().expect("catalog");
        let codec = SqlSchemaLinkingCodec::for_catalog(catalog.clone()).expect("codec");

        // Outer: customers. Inner: orders. Outer select list uses customers
        // column ordinal 1 (name). If inner tables leaked into a global encode
        // scope, ordinals would be mis-interpreted or the outer column would
        // encode against a polluted scope.
        let outer = SelectStatement {
            from: FromClause {
                head: TableRef { menu_index: 1 }, // customers
                joins: vec![],
            },
            where_clause: Some(Expression::Comparison {
                left: Box::new(Expression::Column(ColumnRef { menu_index: 0 })), // customers.customer_id
                op: CmpOp::Eq,
                right: Box::new(Expression::Subquery(Box::new(SelectStatement {
                    from: FromClause {
                        head: TableRef { menu_index: 0 }, // orders
                        joins: vec![],
                    },
                    where_clause: None,
                    group_by: None,
                    having: None,
                    order_by: None,
                    limit: Some(1),
                    select_list: SelectList::Items(crate::sql_schema_linking::ast::SelectItems {
                        distinct: false,
                        // orders.customer_id = ordinal 1 under orders-only scope
                        items: vec![SelectItem::Column(ColumnRef { menu_index: 1 })],
                    }),
                }))),
            }),
            group_by: None,
            having: None,
            order_by: None,
            limit: None,
            select_list: SelectList::Items(crate::sql_schema_linking::ast::SelectItems {
                distinct: false,
                items: vec![SelectItem::Column(ColumnRef { menu_index: 1 })], // customers.name
            }),
        };

        let tree = select_to_decoded_node(&outer);
        let tokens = codec
            .model()
            .encode_complete_tree(&tree)
            .expect("nested subquery encode must succeed with root-local scopes");

        // Round-trip through decode must recover the same AST.
        let decoded = codec.decode(&tokens).expect("decode nested");
        assert_eq!(decoded, outer);

        // Gold scalar_subquery remains bit-stable under root-local encode.
        let gold = codec
            .round_trip_source(&gold_sql("scalar_subquery"))
            .expect("scalar_subquery round_trip");
        assert_eq!(
            gold.rendered,
            r#"SELECT "name" FROM "customers" WHERE "customer_id" = (SELECT "customer_id" FROM "orders" LIMIT 1)"#
        );

        // After the outer FROM is encoded but before the subquery FROM, the
        // outer column site still has customers-only cardinality (3), not
        // customers∪orders. Find a pure column-menu site of card 3 on the
        // gold stream (outer select list / where left).
        let menu = menu_band_range(&sql_syntax_token_layout()).expect("menu band");
        let bindings = menu_bindings_from_catalog(&catalog).expect("bindings");
        let customers_card = bindings
            .resolve_cardinality(COLUMN_MENU_OWNER_RULE, COLUMN_MENU_OWNER_FIELD, &[1])
            .expect("customers scope");
        assert_eq!(customers_card, 3);
        let mut found_outer_only = false;
        let mut found_inner_only = false;
        for end in 0..gold.tokens.len() {
            let prefix = &gold.tokens[..end];
            let Ok(admissible) = codec.model().admissible_next_raw_tokens(prefix) else {
                panic!("scalar_subquery proper prefix_len={end} must admit a mask");
            };
            let admitted = admitted_token_set(&admissible);
            if is_pure_column_menu_mask(&admitted, menu.0, 3) {
                found_outer_only = true;
            }
            if is_pure_column_menu_mask(&admitted, menu.0, 4) {
                found_inner_only = true;
            }
        }
        assert!(
            found_outer_only,
            "expected an outer customers-only column site (card 3)"
        );
        assert!(
            found_inner_only,
            "expected an inner orders-only column site (card 4)"
        );
    }

    #[test]
    fn t9_subquery_inner_scope_excludes_outer_only_columns() {
        // T9 gate: uncorrelated subquery opens inner scope; outer-only
        // columns are not admissible at the inner column site.
        use crate::sql_schema_linking::grammar::{COLUMN_MENU_OWNER_FIELD, COLUMN_MENU_OWNER_RULE};

        let catalog = load_fixture_catalog().expect("catalog");
        let codec = SqlSchemaLinkingCodec::for_catalog(catalog.clone()).expect("codec");
        let sql = gold_sql("scalar_subquery");
        let report = codec
            .round_trip_source(&sql)
            .unwrap_or_else(|e| panic!("subquery round_trip failed: {e}"));
        assert_eq!(
            report.rendered,
            r#"SELECT "name" FROM "customers" WHERE "customer_id" = (SELECT "customer_id" FROM "orders" LIMIT 1)"#
        );

        // Find a column-site prefix whose menu mask is exactly the orders
        // table (4 cols), not customers∪orders.
        let full = report.tokens.clone();
        let menu = menu_band_range(&sql_syntax_token_layout()).expect("menu band");
        let mut found_inner = false;
        for end in 0..full.len() {
            let prefix = &full[..end];
            let Ok(admissible) = codec.model().admissible_next_raw_tokens(prefix) else {
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
            let menu_only: Vec<u32> = admitted
                .iter()
                .copied()
                .filter(|t| *t >= menu.0 && *t < menu.1)
                .collect();
            // orders has 4 real columns; customers has 3. Inner-only → 4.
            if menu_only.len() == 4 && admitted.len() == 4 {
                let bindings = menu_bindings_from_catalog(&catalog).expect("bindings");
                // Scope for orders-only: table index 0.
                let card = bindings
                    .resolve_cardinality(COLUMN_MENU_OWNER_RULE, COLUMN_MENU_OWNER_FIELD, &[0])
                    .expect("orders scope");
                assert_eq!(card, 4);
                // Under orders-only scope, ordinals 4.. must not appear.
                for ord in 4..7 {
                    let token = menu.0 + ord;
                    assert!(
                        !admitted.contains(&token),
                        "outer-only ordinal {ord} must not be admissible inside subquery"
                    );
                }
                found_inner = true;
                break;
            }
        }
        assert!(
            found_inner,
            "expected an inner-subquery column-site mask of cardinality 4"
        );
    }

    #[test]
    fn encode_rejects_out_of_menu_column_ordinal() {
        // T7: encode fail-closed when a column menu ordinal is outside scope.
        use crate::sql_schema_linking::ast::{
            ColumnRef, FromClause, SelectItem, SelectList, SelectStatement, TableRef,
        };
        use crate::sql_schema_linking::tree::select_to_decoded_node;

        let codec = SqlSchemaLinkingCodec::fixture().expect("codec");
        // Single table customers (3 cols); ordinal 3 is out of menu.
        let stmt = SelectStatement {
            from: FromClause {
                head: TableRef { menu_index: 1 },
                joins: vec![],
            },
            where_clause: None,
            group_by: None,
            having: None,
            order_by: None,
            limit: None,
            select_list: SelectList::Items(crate::sql_schema_linking::ast::SelectItems {
                distinct: false,
                items: vec![SelectItem::Column(ColumnRef { menu_index: 3 })],
            }),
        };
        let tree = select_to_decoded_node(&stmt);
        let err = codec
            .model()
            .encode_complete_tree(&tree)
            .expect_err("out-of-menu column must fail encode");
        let msg = format!("{err:?}");
        assert!(
            msg.contains("out of bound") || msg.contains("menu") || msg.contains("out_of"),
            "unexpected encode error: {msg}"
        );
    }

    /// Strict decode must fail closed on PAD, VOCAB_SIZE, out-of-menu ordinals
    /// at menu sites, and trailing surplus tokens.
    #[test]
    fn decode_rejects_malformed_token_streams_fail_closed() {
        let codec = SqlSchemaLinkingCodec::fixture().expect("codec");
        let sql = gold_sql("select_one_table");
        let stmt = codec.parse(&sql).expect("parse");
        let tokens = codec.encode(&stmt).expect("encode");
        assert!(
            !tokens.is_empty(),
            "fixture stream must be non-empty for mutation tests"
        );
        // Sanity: the unmutated stream decodes.
        codec
            .decode(&tokens)
            .expect("valid stream must decode before mutations");

        let roles = codec.model().token_roles();
        let (menu_start, menu_end) = menu_band_range(&sql_syntax_token_layout()).expect("menu");
        assert_eq!(roles.pad_token, PAD);
        assert_eq!(roles.vocab_size, SQL_SYNTAX_LAYOUT_VOCAB_SIZE);

        // 1) Inject PAD at a mid-stream structural position.
        let mut with_pad = tokens.clone();
        let pad_idx = with_pad.len() / 2;
        with_pad[pad_idx] = PAD;
        let err = codec
            .decode(&with_pad)
            .expect_err("PAD in raw stream must fail closed");
        let msg = err.to_string();
        assert!(
            !msg.is_empty(),
            "PAD decode error must carry a useful message"
        );

        // 2) Inject VOCAB_SIZE (out of layout) at the same site.
        let mut with_vocab = tokens.clone();
        with_vocab[pad_idx] = SQL_SYNTAX_LAYOUT_VOCAB_SIZE;
        let err = codec
            .decode(&with_vocab)
            .expect_err("token == VOCAB_SIZE must fail closed");
        let msg = err.to_string();
        assert!(
            !msg.is_empty(),
            "VOCAB_SIZE decode error must carry a useful message"
        );

        // 3) Out-of-menu ordinal at a menu site when practical.
        // select_one_table: customers is table index 1 among 3 fixture tables;
        // customers has 3 columns. Mutate the first menu-band token past a
        // safe upper bound (menu width or known table count).
        let menu_positions: Vec<usize> = tokens
            .iter()
            .enumerate()
            .filter(|&(_, t)| *t >= menu_start && *t < menu_end)
            .map(|(i, _)| i)
            .collect();
        assert!(
            !menu_positions.is_empty(),
            "select_one_table must emit menu-band tokens"
        );
        let menu_pos = menu_positions[0];
        let mut out_of_menu = tokens.clone();
        // Push past the fixture table catalog cardinality (3 tables) and the
        // column menu for a single-table customers scope (3 cols). Menu band
        // width is 256; using ordinal 200 stays in-band but out of any fixture
        // menu cardinality.
        out_of_menu[menu_pos] = menu_start + 200;
        let err = codec
            .decode(&out_of_menu)
            .expect_err("out-of-menu ordinal at menu site must fail closed");
        let msg = err.to_string();
        assert!(
            msg.to_lowercase().contains("menu")
                || msg.contains("bound")
                || msg.contains("range")
                || msg.contains("cardinal")
                || msg.contains("admissible")
                || msg.contains("reject")
                || !msg.is_empty(),
            "out-of-menu decode error should be useful, got: {msg}"
        );

        // 4) Trailing surplus after a complete valid stream.
        let mut trailing = tokens.clone();
        trailing.push(tokens[0]);
        let err = codec
            .decode(&trailing)
            .expect_err("trailing tokens after complete stream must fail closed");
        let msg = err.to_string();
        assert!(
            !msg.is_empty(),
            "trailing-token decode error must carry a useful message"
        );

        // 5) Trailing PAD after a complete valid stream (strict raw path).
        let mut trailing_pad = tokens.clone();
        trailing_pad.push(PAD);
        let err = codec
            .decode(&trailing_pad)
            .expect_err("trailing PAD after complete stream must fail closed");
        let msg = err.to_string();
        assert!(
            !msg.is_empty(),
            "trailing-PAD decode error must carry a useful message"
        );
    }
}
