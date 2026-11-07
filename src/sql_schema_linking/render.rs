//! Canonical SQL renderer (T5–T9).
//!
//! Emits SQL keyword order (SELECT … FROM … [JOIN … ON …] [WHERE …]
//! [GROUP BY …] [HAVING …] [ORDER BY …] [LIMIT …]) from the from-first
//! AST (**P6**). Round-trip claim is tree identity + render fixpoint, never
//! byte-identity with arbitrary input SQL (**P8**).
//!
//! Canonical form:
//! - Uppercase keywords, single spaces, no trailing semicolon.
//! - Qualify a column iff the scope holds **two or more** tables.
//! - JOIN is emitted as `JOIN` (INNER); LEFT as `LEFT JOIN`.
//! - ORDER BY always emits ASC or DESC (default ASC when omitted on parse).
//! - String literals use single quotes with `''` escape.
//! - Identifiers prefer Spider `name_original` (SQL form) over display `name`
//!   (which may replace `_` with spaces). Always double-quote the chosen
//!   spelling, escaping embedded quotes; never infer dialect keywords from
//!   the subset parser's keyword list. NUL is not representable in SQL text.
//! - Comparison `!=` is canonical for inequality (`<>` accepted on parse).
//! - `COUNT(*)` is structural; aggregates render as `FUNC(arg)`.

use crate::sql_schema_linking::ast::{
    AggFunc, AggregateArg, BoolOp, CmpOp, Expression, JoinKind, OrderDirection, SelectItem,
    SelectList, SelectStatement,
};
use crate::sql_schema_linking::catalog::SqlCatalog;

/// SQL text/admission semantics, separate from token layout and stream spelling.
pub const SQL_TEXT_CONTRACT: &str = "sqlite_quoted_identifiers_no_nul_v1";

/// Render a select statement to canonical SQL against the catalog.
pub fn render_select(stmt: &SelectStatement, catalog: &SqlCatalog) -> Result<String, RenderError> {
    let scope: Vec<usize> = stmt
        .scope_table_indices()
        .into_iter()
        .map(|i| i as usize)
        .collect();
    if scope.is_empty() {
        return Err(RenderError::new("select has empty table scope"));
    }
    for &idx in &scope {
        if idx >= catalog.tables.len() {
            return Err(RenderError::new(format!(
                "scope table index {idx} out of catalog range"
            )));
        }
    }

    let mut out = String::from("SELECT ");
    match &stmt.select_list {
        SelectList::Star => out.push('*'),
        SelectList::Items(items) => {
            if items.items.is_empty() {
                return Err(RenderError::new("select list items must be non-empty"));
            }
            if items.distinct {
                out.push_str("DISTINCT ");
            }
            for (i, item) in items.items.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                out.push_str(&render_select_item(item, &scope, catalog)?);
            }
        }
    }
    out.push_str(" FROM ");
    out.push_str(&render_table_ident(&catalog.tables[scope[0]])?);

    // Progressive scope for ON predicates: head, then each join table.
    let mut on_scope: Vec<usize> = vec![scope[0]];
    for (join_i, join) in stmt.from.joins.iter().enumerate() {
        let table_idx = scope
            .get(join_i + 1)
            .copied()
            .ok_or_else(|| RenderError::new("join table missing from scope"))?;
        on_scope.push(table_idx);
        match join.kind {
            JoinKind::Inner => out.push_str(" JOIN "),
            JoinKind::Left => out.push_str(" LEFT JOIN "),
        }
        out.push_str(&render_table_ident(&catalog.tables[table_idx])?);
        out.push_str(" ON ");
        out.push_str(&render_expression(&join.on, &on_scope, catalog)?);
    }

    if let Some(ref where_expr) = stmt.where_clause {
        out.push_str(" WHERE ");
        out.push_str(&render_expression(where_expr, &scope, catalog)?);
    }

    if let Some(ref group) = stmt.group_by {
        if group.columns.is_empty() {
            return Err(RenderError::new("GROUP BY columns must be non-empty"));
        }
        out.push_str(" GROUP BY ");
        for (i, col) in group.columns.iter().enumerate() {
            if i > 0 {
                out.push_str(", ");
            }
            out.push_str(&render_column(col.menu_index, &scope, catalog)?);
        }
    }

    if let Some(ref having_expr) = stmt.having {
        out.push_str(" HAVING ");
        out.push_str(&render_expression(having_expr, &scope, catalog)?);
    }

    if let Some(ref order) = stmt.order_by {
        if order.items.is_empty() {
            return Err(RenderError::new("ORDER BY items must be non-empty"));
        }
        out.push_str(" ORDER BY ");
        for (i, item) in order.items.iter().enumerate() {
            if i > 0 {
                out.push_str(", ");
            }
            match &item.key {
                crate::sql_schema_linking::ast::OrderKey::Column(col) => {
                    out.push_str(&render_column(col.menu_index, &scope, catalog)?);
                }
                crate::sql_schema_linking::ast::OrderKey::CountStar => {
                    out.push_str("COUNT(*)");
                }
                crate::sql_schema_linking::ast::OrderKey::Aggregate {
                    func,
                    arg,
                    distinct,
                } => {
                    let fname = match func {
                        crate::sql_schema_linking::ast::AggFunc::Count => "COUNT",
                        crate::sql_schema_linking::ast::AggFunc::Sum => "SUM",
                        crate::sql_schema_linking::ast::AggFunc::Avg => "AVG",
                        crate::sql_schema_linking::ast::AggFunc::Min => "MIN",
                        crate::sql_schema_linking::ast::AggFunc::Max => "MAX",
                    };
                    let crate::sql_schema_linking::ast::AggregateArg::Column(col) = arg;
                    out.push_str(fname);
                    out.push('(');
                    if *distinct {
                        out.push_str("DISTINCT ");
                    }
                    out.push_str(&render_column(col.menu_index, &scope, catalog)?);
                    out.push(')');
                }
            }
            match item.direction {
                OrderDirection::Asc => out.push_str(" ASC"),
                OrderDirection::Desc => out.push_str(" DESC"),
            }
        }
    }

    if let Some(limit) = stmt.limit {
        out.push_str(" LIMIT ");
        out.push_str(&limit.to_string());
    }

    Ok(out)
}

fn render_select_item(
    item: &SelectItem,
    scope: &[usize],
    catalog: &SqlCatalog,
) -> Result<String, RenderError> {
    match item {
        SelectItem::Column(col) => render_column(col.menu_index, scope, catalog),
        SelectItem::CountStar => Ok("COUNT(*)".to_string()),
        SelectItem::Aggregate {
            func,
            arg,
            distinct,
        } => {
            let func_s = match func {
                AggFunc::Count => "COUNT",
                AggFunc::Sum => "SUM",
                AggFunc::Avg => "AVG",
                AggFunc::Min => "MIN",
                AggFunc::Max => "MAX",
            };
            let AggregateArg::Column(col) = arg;
            let arg_s = render_column(col.menu_index, scope, catalog)?;
            if *distinct {
                Ok(format!("{func_s}(DISTINCT {arg_s})"))
            } else {
                Ok(format!("{func_s}({arg_s})"))
            }
        }
    }
}

/// Boolean/comparison precedence for canonical parentheses (**P8**).
/// Higher binds tighter. Order: OR < AND < comparison < atom.
///
/// AND/OR are **left-associative**: a same-precedence right child must be
/// parenthesized so `a OR (b OR c)` re-parses as right-nested, not as
/// left-associative `(a OR b) OR c`. Comparisons are **non-associative** in
/// this subset (the parser accepts one comparison per level), so a nested
/// comparison operand is always parenthesized to avoid unfaithful chains
/// such as `a = b = c`.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum ExprPrec {
    Or = 1,
    And = 2,
    Comparison = 3,
    /// Columns, literals, subqueries — atomic relative to comparisons/AND/OR.
    Atom = 4,
}

fn expr_prec(expr: &Expression) -> ExprPrec {
    match expr {
        Expression::BinaryBool { op: BoolOp::Or, .. } => ExprPrec::Or,
        Expression::BinaryBool {
            op: BoolOp::And, ..
        } => ExprPrec::And,
        Expression::Comparison { .. } | Expression::In { .. } | Expression::Between { .. } => {
            ExprPrec::Comparison
        }
        Expression::Column(_)
        | Expression::IntegerLiteral(_)
        | Expression::StringLiteral(_)
        | Expression::CountStar
        | Expression::Subquery(_)
        | Expression::Aggregate { .. } => ExprPrec::Atom,
    }
}

fn render_expression(
    expr: &Expression,
    scope: &[usize],
    catalog: &SqlCatalog,
) -> Result<String, RenderError> {
    // Root context: never wrap solely because of parent precedence.
    render_expression_prec(expr, scope, catalog, ExprPrec::Or, false)
}

/// Render `expr` under a parent of precedence `parent_prec`.
///
/// When `tight` is true, also parenthesize when `expr` has **equal** precedence
/// to the parent (right child of a left-associative operator, or either
/// operand of a non-associative comparison). When false, only strictly looser
/// children are wrapped (left child of left-associative AND/OR, root).
fn render_expression_prec(
    expr: &Expression,
    scope: &[usize],
    catalog: &SqlCatalog,
    parent_prec: ExprPrec,
    tight: bool,
) -> Result<String, RenderError> {
    let prec = expr_prec(expr);
    let rendered = match expr {
        Expression::Column(col) => render_column(col.menu_index, scope, catalog)?,
        Expression::IntegerLiteral(v) => v.to_string(),
        Expression::StringLiteral(s) => render_string_literal(s)?,
        Expression::CountStar => "COUNT(*)".to_string(),
        Expression::Comparison { left, op, right } => {
            // Non-associative: both operands use tight=true so nested
            // comparisons are parenthesized.
            let l = render_expression_prec(left, scope, catalog, ExprPrec::Comparison, true)?;
            let r = render_expression_prec(right, scope, catalog, ExprPrec::Comparison, true)?;
            let op_s = match op {
                CmpOp::Eq => "=",
                CmpOp::Ne => "!=",
                CmpOp::Lt => "<",
                CmpOp::Le => "<=",
                CmpOp::Gt => ">",
                CmpOp::Ge => ">=",
                CmpOp::Like => "LIKE",
            };
            format!("{l} {op_s} {r}")
        }
        Expression::BinaryBool { left, op, right } => {
            // Left-associative: left child may share precedence without parens;
            // right child of equal precedence must be parenthesized.
            let l = render_expression_prec(left, scope, catalog, prec, false)?;
            let r = render_expression_prec(right, scope, catalog, prec, true)?;
            let op_s = match op {
                BoolOp::And => "AND",
                BoolOp::Or => "OR",
            };
            format!("{l} {op_s} {r}")
        }
        Expression::Subquery(stmt) => {
            let inner = render_select(stmt, catalog)?;
            format!("({inner})")
        }
        Expression::In {
            left,
            negated,
            right,
        } => {
            use crate::sql_schema_linking::ast::InRhs;
            // Same tight comparison operands as CmpOp so nested IN stays parenthesized.
            let l = render_expression_prec(left, scope, catalog, ExprPrec::Comparison, true)?;
            let inner = match right {
                InRhs::Subquery(stmt) => render_select(stmt, catalog)?,
                InRhs::List(values) => {
                    let mut parts = Vec::with_capacity(values.len());
                    for v in values {
                        parts.push(render_expression_prec(
                            v,
                            scope,
                            catalog,
                            ExprPrec::Comparison,
                            true,
                        )?);
                    }
                    parts.join(", ")
                }
            };
            if *negated {
                format!("{l} NOT IN ({inner})")
            } else {
                format!("{l} IN ({inner})")
            }
        }
        Expression::Aggregate {
            func,
            arg,
            distinct,
        } => {
            let func_s = match func {
                AggFunc::Count => "COUNT",
                AggFunc::Sum => "SUM",
                AggFunc::Avg => "AVG",
                AggFunc::Min => "MIN",
                AggFunc::Max => "MAX",
            };
            let AggregateArg::Column(col) = arg;
            let arg_s = render_column(col.menu_index, scope, catalog)?;
            if *distinct {
                format!("{func_s}(DISTINCT {arg_s})")
            } else {
                format!("{func_s}({arg_s})")
            }
        }
        Expression::Between { expr, low, high } => {
            let e = render_expression_prec(expr, scope, catalog, ExprPrec::Comparison, true)?;
            let l = render_expression_prec(low, scope, catalog, ExprPrec::Comparison, true)?;
            let h = render_expression_prec(high, scope, catalog, ExprPrec::Comparison, true)?;
            format!("{e} BETWEEN {l} AND {h}")
        }
    };
    let needs_parens = if tight {
        prec <= parent_prec
    } else {
        prec < parent_prec
    };
    if needs_parens {
        Ok(format!("({rendered})"))
    } else {
        Ok(rendered)
    }
}

fn render_string_literal(s: &str) -> Result<String, RenderError> {
    if s.contains('\0') {
        return Err(RenderError::new(
            "string_literal.value contains NUL, which is not representable in SQLite SQL text",
        ));
    }
    let mut out = String::with_capacity(s.len() + 2);
    out.push('\'');
    for ch in s.chars() {
        if ch == '\'' {
            out.push_str("''");
        } else {
            out.push(ch);
        }
    }
    out.push('\'');
    Ok(out)
}

fn render_column(
    menu_index: u32,
    scope: &[usize],
    catalog: &SqlCatalog,
) -> Result<String, RenderError> {
    let (table_idx, column_ident) = resolve_scope_column(menu_index, scope, catalog)?;
    if scope.len() >= 2 {
        Ok(format!(
            "{}.{}",
            render_table_ident(&catalog.tables[table_idx])?,
            column_ident
        ))
    } else {
        Ok(column_ident)
    }
}

/// Prefer SQL original name; fall back to display name; always quote.
fn prefer_sql_ident(display: &str, original: &str) -> Result<String, RenderError> {
    let chosen = if !original.is_empty() {
        original
    } else {
        display
    };
    if chosen.contains('\0') {
        return Err(RenderError::new(
            "catalog identifier contains NUL, which is not representable in SQLite SQL text",
        ));
    }
    Ok(format!("\"{}\"", chosen.replace('"', "\"\"")))
}

fn render_table_ident(
    table: &crate::sql_schema_linking::catalog::SqlTable,
) -> Result<String, RenderError> {
    prefer_sql_ident(&table.name, &table.name_original)
}

/// Superset of the keyword literals the subset lexer can consume at an
/// identifier position (the parser's `peek_keyword` / `expect_keyword` call
/// sites, plus `AS`, which v0 never lexes but real SQL reserves).
///
/// Used by the parser-hazard column census in
/// [`crate::sql_schema_linking::spider_coverage`]. This is not a SQLite keyword
/// inventory or a quoting policy: the renderer quotes every identifier.
/// Parser-hazard coverage is checked by
/// `subset_keyword_list_covers_every_word_that_breaks_a_bare_column_reference`.
pub const SQL_SUBSET_KEYWORDS: &[&str] = &[
    "SELECT", "FROM", "JOIN", "LEFT", "INNER", "ON", "WHERE", "GROUP", "BY", "HAVING", "ORDER",
    "LIMIT", "DISTINCT", "ASC", "DESC", "AND", "OR", "NOT", "IN", "BETWEEN", "LIKE", "COUNT",
    "SUM", "AVG", "MIN", "MAX", "AS",
];

/// Identifiers the subset parser would otherwise consume as keywords.
pub fn is_sql_subset_keyword(s: &str) -> bool {
    SQL_SUBSET_KEYWORDS
        .iter()
        .any(|keyword| s.eq_ignore_ascii_case(keyword))
}

/// Map a scope-relative column ordinal to `(catalog_table_index, rendered_ident)`.
fn resolve_scope_column(
    menu_index: u32,
    scope: &[usize],
    catalog: &SqlCatalog,
) -> Result<(usize, String), RenderError> {
    let mut remaining = menu_index;
    for &table_idx in scope {
        let table = &catalog.tables[table_idx];
        let n = u32::try_from(table.columns.len())
            .map_err(|_| RenderError::new(format!("table {} column count overflow", table.name)))?;
        if remaining < n {
            let col = &table.columns[remaining as usize];
            return Ok((table_idx, prefer_sql_ident(&col.name, &col.name_original)?));
        }
        remaining -= n;
    }
    Err(RenderError::new(format!(
        "column menu index {menu_index} is out of scope (scope tables: {scope:?})"
    )))
}

/// Render error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderError {
    pub message: String,
}

impl RenderError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for RenderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SQL render error: {}", self.message)
    }
}

impl std::error::Error for RenderError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sql_schema_linking::catalog::load_fixture_catalog;
    use crate::sql_schema_linking::parser::{parse_m0_select, parse_select};

    #[test]
    fn render_select_one_table_is_canonical_unqualified() {
        let catalog = load_fixture_catalog().expect("catalog");
        let stmt = parse_m0_select("SELECT name, city FROM customers", &catalog).expect("parse");
        let sql = render_select(&stmt, &catalog).expect("render");
        assert_eq!(sql, r#"SELECT "name", "city" FROM "customers""#);
    }

    #[test]
    fn render_star_is_structural() {
        let catalog = load_fixture_catalog().expect("catalog");
        let stmt = parse_m0_select("SELECT * FROM products", &catalog).expect("parse");
        let sql = render_select(&stmt, &catalog).expect("render");
        assert_eq!(sql, r#"SELECT * FROM "products""#);
    }

    #[test]
    fn render_join_qualifies_columns() {
        let catalog = load_fixture_catalog().expect("catalog");
        let sql = "SELECT customers.name, orders.amount FROM customers JOIN orders ON customers.customer_id = orders.customer_id";
        let stmt = parse_select(sql, &catalog).expect("parse");
        let rendered = render_select(&stmt, &catalog).expect("render");
        assert_eq!(
            rendered,
            r#"SELECT "customers"."name", "orders"."amount" FROM "customers" JOIN "orders" ON "customers"."customer_id" = "orders"."customer_id""#
        );
    }

    #[test]
    fn render_where_group_order_limit_is_canonical() {
        let catalog = load_fixture_catalog().expect("catalog");
        let sql = "SELECT city FROM customers WHERE customer_id > 0 GROUP BY city HAVING city = 'Springfield' ORDER BY city DESC LIMIT 10";
        let stmt = parse_select(sql, &catalog).expect("parse");
        let rendered = render_select(&stmt, &catalog).expect("render");
        assert_eq!(
            rendered,
            r#"SELECT "city" FROM "customers" WHERE "customer_id" > 0 GROUP BY "city" HAVING "city" = 'Springfield' ORDER BY "city" DESC LIMIT 10"#
        );
    }

    #[test]
    fn render_order_by_defaults_asc() {
        let catalog = load_fixture_catalog().expect("catalog");
        let stmt =
            parse_select("SELECT name FROM customers ORDER BY name", &catalog).expect("parse");
        let rendered = render_select(&stmt, &catalog).expect("render");
        assert_eq!(
            rendered,
            r#"SELECT "name" FROM "customers" ORDER BY "name" ASC"#
        );
    }

    #[test]
    fn render_prefers_original_ident_with_underscore() {
        // Spider display names often replace '_' with spaces; SQL originals keep '_'.
        use crate::sql_schema_linking::catalog::{SqlCatalog, SqlColumn, SqlTable};
        let catalog = SqlCatalog {
            db_id: "t".into(),
            tables: vec![SqlTable {
                table_index: 0,
                name: "pets".into(),
                name_original: "Pets".into(),
                columns: vec![
                    SqlColumn {
                        raw_column_index: 1,
                        name: "pet age".into(),
                        name_original: "pet_age".into(),
                        column_type: "number".into(),
                        is_primary_key: false,
                    },
                    SqlColumn {
                        raw_column_index: 2,
                        name: "weight".into(),
                        name_original: "weight".into(),
                        column_type: "number".into(),
                        is_primary_key: false,
                    },
                ],
            }],
            foreign_keys: vec![],
            tables_json_sha256: "test".into(),
            tables_json_path: "test".into(),
        };
        let stmt = parse_select("SELECT weight FROM pets ORDER BY pet_age LIMIT 1", &catalog)
            .expect("parse");
        let rendered = render_select(&stmt, &catalog).expect("render");
        assert!(
            rendered.contains("pet_age"),
            "must emit original underscore form, got {rendered}"
        );
        assert!(
            !rendered.contains("pet age"),
            "must not emit display-space form, got {rendered}"
        );
        // Round-trip reparse of rendered form.
        parse_select(&rendered, &catalog).expect("reparse rendered");
    }

    #[test]
    fn render_quotes_keyword_column_ident_so_reparse_is_not_an_aggregate() {
        use crate::sql_schema_linking::catalog::{SqlCatalog, SqlColumn, SqlTable};
        let catalog = SqlCatalog {
            db_id: "yelp".into(),
            tables: vec![SqlTable {
                table_index: 0,
                name: "checkin".into(),
                name_original: "checkin".into(),
                columns: vec![SqlColumn {
                    raw_column_index: 1,
                    name: "count".into(),
                    name_original: "count".into(),
                    column_type: "number".into(),
                    is_primary_key: false,
                }],
            }],
            foreign_keys: vec![],
            tables_json_sha256: "test".into(),
            tables_json_path: "test".into(),
        };
        let sql = r#"SELECT "count" FROM checkin"#;
        let stmt = parse_select(sql, &catalog).expect("parse quoted count");
        let rendered = render_select(&stmt, &catalog).expect("render");
        assert!(
            rendered.contains("\"count\""),
            "keyword column must stay quoted, got {rendered}"
        );
        parse_select(&rendered, &catalog).expect("reparse quoted count");
        let codec =
            crate::sql_schema_linking::codec::SqlSchemaLinkingCodec::for_catalog_with_sql_policies(
                catalog,
                crate::sql_schema_linking::menu_binding::SqlJoinOnPolicy::Off,
                crate::sql_schema_linking::menu_binding::SqlColumnAddressing::Flat,
                true,
            )
            .expect("visit-all codec");
        codec
            .round_trip_source(sql)
            .expect("visit-all quoted count round-trip");
    }

    /// Drift guard: keep the subset keyword census aware of every bare-name
    /// parser hazard. Quoting is unconditional and does not use this list.
    ///
    /// Sweeps the SQLite reserved-word list (a superset of the subset grammar's
    /// keywords) so adding a `peek_keyword`/`expect_keyword` call site without
    /// updating the constant fails here instead of silently dropping a column.
    #[test]
    fn subset_keyword_list_covers_every_word_that_breaks_a_bare_column_reference() {
        use crate::sql_schema_linking::catalog::SqlCatalog;
        use crate::sql_schema_linking::codec::SqlSchemaLinkingCodec;

        // SQLite reserved words (superset of the M0–M5 subset grammar).
        const RESERVED: &str = "ABORT ACTION ADD AFTER ALL ALTER ALWAYS ANALYZE AND AS ASC ATTACH \
             AUTOINCREMENT BEFORE BEGIN BETWEEN BY CASCADE CASE CAST CHECK COLLATE COLUMN COMMIT \
             CONFLICT CONSTRAINT CREATE CROSS CURRENT DATABASE DEFAULT DEFERRABLE DEFERRED DELETE \
             DESC DETACH DISTINCT DO DROP EACH ELSE END ESCAPE EXCEPT EXCLUDE EXCLUSIVE EXISTS \
             EXPLAIN FAIL FILTER FIRST FOLLOWING FOR FOREIGN FROM FULL GENERATED GLOB GROUP GROUPS \
             HAVING IF IGNORE IMMEDIATE IN INDEX INDEXED INITIALLY INNER INSERT INSTEAD INTERSECT \
             INTO IS ISNULL JOIN KEY LAST LEFT LIKE LIMIT MATCH MATERIALIZED NATURAL NO NOT NOTHING \
             NOTNULL NULL NULLS OF OFFSET ON OR ORDER OTHERS OUTER OVER PARTITION PLAN PRAGMA \
             PRECEDING PRIMARY QUERY RAISE RANGE RECURSIVE REFERENCES REGEXP REINDEX RELEASE RENAME \
             REPLACE RESTRICT RETURNING RIGHT ROLLBACK ROW ROWS SAVEPOINT SELECT SET TABLE TEMP \
             TEMPORARY THEN TIES TO TRANSACTION TRIGGER UNBOUNDED UNION UNIQUE UPDATE USING VACUUM \
             VALUES VIEW VIRTUAL WHEN WHERE WINDOW WITH WITHOUT \
             COUNT SUM AVG MIN MAX YEAR";

        let words: Vec<&str> = RESERVED.split_whitespace().collect();
        let mut column_names = vec![serde_json::json!([-1, "*"])];
        let mut column_names_original = vec![serde_json::json!([-1, "*"])];
        let mut column_types = vec![serde_json::json!("text")];
        for word in &words {
            column_names.push(serde_json::json!([0, word.to_ascii_lowercase()]));
            column_names_original.push(serde_json::json!([0, word]));
            column_types.push(serde_json::json!("number"));
        }
        let raw = serde_json::json!([{
            "db_id": "reserved_probe",
            "table_names": ["t"],
            "table_names_original": ["t"],
            "column_names": column_names,
            "column_names_original": column_names_original,
            "column_types": column_types,
            "primary_keys": [1],
            "foreign_keys": [],
        }]);
        let bytes = serde_json::to_vec(&raw).expect("serialize");
        let catalog =
            SqlCatalog::parse_tables_json_unpinned(&bytes, "reserved_probe").expect("catalog");
        let codec = SqlSchemaLinkingCodec::for_catalog(catalog).expect("codec");

        let mut unlisted_hazards = Vec::new();
        let mut bare_hazards = Vec::new();
        for word in &words {
            // The quoted spelling is the escape hatch and must always work.
            codec
                .round_trip_source(&format!("SELECT \"{word}\" FROM t"))
                .unwrap_or_else(|e| panic!("quoted {word} must round-trip: {e}"));
            if codec
                .round_trip_source(&format!("SELECT {word} FROM t"))
                .is_err()
            {
                bare_hazards.push(*word);
                if !is_sql_subset_keyword(word) {
                    unlisted_hazards.push(*word);
                }
            }
        }
        assert!(
            unlisted_hazards.is_empty(),
            "these words break a bare column reference but are absent from \
             SQL_SUBSET_KEYWORDS (the parser-hazard census will miss them): {unlisted_hazards:?}"
        );
        // Canary: the guard must not be vacuous. `DISTINCT` is the one word the
        // select-list lexer still consumes bare (documented residual in
        // `docs/sql-schema-linking-frontend-contract.md`, P7).
        assert_eq!(
            bare_hazards,
            ["DISTINCT"],
            "expected exactly the documented bare-reference residual"
        );
    }

    #[test]
    fn render_pets_1_original_ident_host_catalog_if_present() {
        // Full codec identity when unpinned host pets_1 catalog is present.
        let tables = std::path::PathBuf::from(std::env::var_os("HOME").unwrap_or_default())
            .join("data/spider-census-reconstructed/tables.json");
        if tables.is_file() {
            use crate::sql_schema_linking::catalog::SqlCatalog;
            use crate::sql_schema_linking::codec::SqlSchemaLinkingCodec;
            let cat = SqlCatalog::load_unpinned_tables_json(&tables, "pets_1").expect("cat");
            let codec = SqlSchemaLinkingCodec::for_catalog(cat).expect("codec");
            let sql = "SELECT weight FROM pets ORDER BY pet_age LIMIT 1";
            codec
                .round_trip_source(sql)
                .expect("pets_1 pet_age original-ident round-trip");
        }
    }

    #[test]
    fn render_fixpoint_and_or_parentheses() {
        // P8: parenthesized AND-over-OR must re-render with parens so reparse
        // preserves tree identity. Without precedence-aware parens,
        // `a AND (b OR c)` becomes `a AND b OR c` → OR(AND(...), ...).
        use crate::sql_schema_linking::ast::{
            BoolOp, CmpOp, ColumnRef, Expression, FromClause, SelectItem, SelectList,
            SelectStatement, TableRef,
        };

        let catalog = load_fixture_catalog().expect("catalog");
        // customers: customer_id=0, name=1, city=2
        let col = |i| Expression::Column(ColumnRef { menu_index: i });
        let cmp = |i, op, lit: Expression| Expression::Comparison {
            left: Box::new(col(i)),
            op,
            right: Box::new(lit),
        };

        // AND (OR): customer_id > 0 AND (city = 'A' OR name = 'B')
        let and_or = Expression::BinaryBool {
            left: Box::new(cmp(0, CmpOp::Gt, Expression::IntegerLiteral(0))),
            op: BoolOp::And,
            right: Box::new(Expression::BinaryBool {
                left: Box::new(cmp(2, CmpOp::Eq, Expression::StringLiteral("A".into()))),
                op: BoolOp::Or,
                right: Box::new(cmp(1, CmpOp::Eq, Expression::StringLiteral("B".into()))),
            }),
        };
        // (OR) AND: (city = 'A' OR name = 'B') AND customer_id > 0
        let or_and = Expression::BinaryBool {
            left: Box::new(Expression::BinaryBool {
                left: Box::new(cmp(2, CmpOp::Eq, Expression::StringLiteral("A".into()))),
                op: BoolOp::Or,
                right: Box::new(cmp(1, CmpOp::Eq, Expression::StringLiteral("B".into()))),
            }),
            op: BoolOp::And,
            right: Box::new(cmp(0, CmpOp::Gt, Expression::IntegerLiteral(0))),
        };

        for (label, where_expr, expected_fragment) in [
            (
                "AND(OR)",
                and_or,
                r#""customer_id" > 0 AND ("city" = 'A' OR "name" = 'B')"#,
            ),
            (
                "(OR)AND",
                or_and,
                r#"("city" = 'A' OR "name" = 'B') AND "customer_id" > 0"#,
            ),
        ] {
            let stmt = SelectStatement {
                from: FromClause {
                    head: TableRef { menu_index: 1 },
                    joins: vec![],
                },
                where_clause: Some(where_expr.clone()),
                group_by: None,
                having: None,
                order_by: None,
                limit: None,
                select_list: SelectList::Items(crate::sql_schema_linking::ast::SelectItems {
                    distinct: false,
                    items: vec![SelectItem::Column(ColumnRef { menu_index: 1 })],
                }),
            };
            let rendered = render_select(&stmt, &catalog).expect("render");
            assert!(
                rendered.contains(expected_fragment),
                "{label}: expected fragment {expected_fragment:?} in {rendered:?}"
            );
            // Render fixpoint: reparse(render(tree)) == tree (where clause).
            let reparsed = parse_select(&rendered, &catalog).expect("reparse");
            assert_eq!(
                reparsed.where_clause, stmt.where_clause,
                "{label}: where tree must survive render fixpoint"
            );
            let re_rendered = render_select(&reparsed, &catalog).expect("re-render");
            assert_eq!(re_rendered, rendered, "{label}: render must be idempotent");
        }
    }

    #[test]
    fn render_fixpoint_nested_decoded_and_or_tree() {
        // Nested decoded-tree expression: AND of (OR of comparisons) with an
        // outer OR — forces multi-level paren decisions.
        use crate::sql_schema_linking::ast::{
            BoolOp, CmpOp, ColumnRef, Expression, FromClause, SelectItem, SelectList,
            SelectStatement, TableRef,
        };

        let catalog = load_fixture_catalog().expect("catalog");
        let col = |i| Expression::Column(ColumnRef { menu_index: i });
        let tree = Expression::BinaryBool {
            left: Box::new(Expression::BinaryBool {
                left: Box::new(Expression::Comparison {
                    left: Box::new(col(0)),
                    op: CmpOp::Gt,
                    right: Box::new(Expression::IntegerLiteral(0)),
                }),
                op: BoolOp::And,
                right: Box::new(Expression::BinaryBool {
                    left: Box::new(Expression::Comparison {
                        left: Box::new(col(2)),
                        op: CmpOp::Eq,
                        right: Box::new(Expression::StringLiteral("A".into())),
                    }),
                    op: BoolOp::Or,
                    right: Box::new(Expression::Comparison {
                        left: Box::new(col(1)),
                        op: CmpOp::Eq,
                        right: Box::new(Expression::StringLiteral("B".into())),
                    }),
                }),
            }),
            op: BoolOp::Or,
            right: Box::new(Expression::Comparison {
                left: Box::new(col(2)),
                op: CmpOp::Eq,
                right: Box::new(Expression::StringLiteral("C".into())),
            }),
        };
        // Expected: (customer_id > 0 AND (city = 'A' OR name = 'B')) OR city = 'C'
        // Outer OR does not need parens at root; inner OR under AND does.
        let stmt = SelectStatement {
            from: FromClause {
                head: TableRef { menu_index: 1 },
                joins: vec![],
            },
            where_clause: Some(tree.clone()),
            group_by: None,
            having: None,
            order_by: None,
            limit: None,
            select_list: SelectList::Items(crate::sql_schema_linking::ast::SelectItems {
                distinct: false,
                items: vec![SelectItem::Column(ColumnRef { menu_index: 1 })],
            }),
        };
        let rendered = render_select(&stmt, &catalog).expect("render");
        assert_eq!(
            rendered,
            r#"SELECT "name" FROM "customers" WHERE "customer_id" > 0 AND ("city" = 'A' OR "name" = 'B') OR "city" = 'C'"#
        );
        let reparsed = parse_select(&rendered, &catalog).expect("reparse");
        assert_eq!(reparsed.where_clause, Some(tree));
    }

    /// Helper: one-table SELECT name FROM customers with a synthetic WHERE tree.
    fn stmt_with_where(where_expr: Expression) -> SelectStatement {
        use crate::sql_schema_linking::ast::{
            ColumnRef, FromClause, SelectItem, SelectList, SelectStatement, TableRef,
        };
        SelectStatement {
            from: FromClause {
                head: TableRef { menu_index: 1 },
                joins: vec![],
            },
            where_clause: Some(where_expr),
            group_by: None,
            having: None,
            order_by: None,
            limit: None,
            select_list: SelectList::Items(crate::sql_schema_linking::ast::SelectItems {
                distinct: false,
                items: vec![SelectItem::Column(ColumnRef { menu_index: 1 })],
            }),
        }
    }

    #[test]
    fn render_fixpoint_right_nested_or_preserves_tree() {
        // Left-associative OR: a OR (b OR c) must emit parens on the right child
        // or reparse yields (a OR b) OR c.
        use crate::sql_schema_linking::ast::{BoolOp, CmpOp, ColumnRef, Expression};

        let catalog = load_fixture_catalog().expect("catalog");
        let col = |i| Expression::Column(ColumnRef { menu_index: i });
        let cmp = |i, lit: &str| Expression::Comparison {
            left: Box::new(col(i)),
            op: CmpOp::Eq,
            right: Box::new(Expression::StringLiteral(lit.into())),
        };
        // city = 'A' OR (name = 'B' OR city = 'C')
        let tree = Expression::BinaryBool {
            left: Box::new(cmp(2, "A")),
            op: BoolOp::Or,
            right: Box::new(Expression::BinaryBool {
                left: Box::new(cmp(1, "B")),
                op: BoolOp::Or,
                right: Box::new(cmp(2, "C")),
            }),
        };
        let stmt = stmt_with_where(tree.clone());
        let rendered = render_select(&stmt, &catalog).expect("render");
        assert!(
            rendered.contains(r#""city" = 'A' OR ("name" = 'B' OR "city" = 'C')"#),
            "right-nested OR must parenthesize right child; got {rendered:?}"
        );
        let reparsed = parse_select(&rendered, &catalog).expect("reparse");
        assert_eq!(
            reparsed.where_clause,
            Some(tree),
            "right-nested OR tree must survive render→parse"
        );
        let re_rendered = render_select(&reparsed, &catalog).expect("re-render");
        assert_eq!(re_rendered, rendered, "render must be idempotent");
    }

    #[test]
    fn render_fixpoint_right_nested_and_preserves_tree() {
        use crate::sql_schema_linking::ast::{BoolOp, CmpOp, ColumnRef, Expression};

        let catalog = load_fixture_catalog().expect("catalog");
        let col = |i| Expression::Column(ColumnRef { menu_index: i });
        let cmp = |i, lit: &str| Expression::Comparison {
            left: Box::new(col(i)),
            op: CmpOp::Eq,
            right: Box::new(Expression::StringLiteral(lit.into())),
        };
        // city = 'A' AND (name = 'B' AND city = 'C')
        let tree = Expression::BinaryBool {
            left: Box::new(cmp(2, "A")),
            op: BoolOp::And,
            right: Box::new(Expression::BinaryBool {
                left: Box::new(cmp(1, "B")),
                op: BoolOp::And,
                right: Box::new(cmp(2, "C")),
            }),
        };
        let stmt = stmt_with_where(tree.clone());
        let rendered = render_select(&stmt, &catalog).expect("render");
        assert!(
            rendered.contains(r#""city" = 'A' AND ("name" = 'B' AND "city" = 'C')"#),
            "right-nested AND must parenthesize right child; got {rendered:?}"
        );
        let reparsed = parse_select(&rendered, &catalog).expect("reparse");
        assert_eq!(reparsed.where_clause, Some(tree));
    }

    #[test]
    fn render_fixpoint_nested_comparison_operands() {
        // GrammarIR may decode Comparison(Comparison(...), op, ...) trees.
        // Without parens this becomes the invalid/unfaithful chain `a = b = c`.
        use crate::sql_schema_linking::ast::{CmpOp, ColumnRef, Expression};

        let catalog = load_fixture_catalog().expect("catalog");
        let col = |i| Expression::Column(ColumnRef { menu_index: i });
        // (customer_id = name) = city  — nested comparison as left operand
        let tree = Expression::Comparison {
            left: Box::new(Expression::Comparison {
                left: Box::new(col(0)),
                op: CmpOp::Eq,
                right: Box::new(col(1)),
            }),
            op: CmpOp::Eq,
            right: Box::new(col(2)),
        };
        let stmt = stmt_with_where(tree.clone());
        let rendered = render_select(&stmt, &catalog).expect("render");
        assert!(
            rendered.contains(r#"("customer_id" = "name") = "city""#),
            "nested comparison left operand must be parenthesized; got {rendered:?}"
        );
        assert!(
            !rendered.contains(r#""customer_id" = "name" = "city""#),
            "must not emit unparenthesized comparison chain; got {rendered:?}"
        );
        let reparsed = parse_select(&rendered, &catalog).expect("reparse");
        assert_eq!(
            reparsed.where_clause,
            Some(tree),
            "nested comparison tree must survive render→parse"
        );
    }
}
