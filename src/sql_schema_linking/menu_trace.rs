//! Menu-trace extraction from a parsed [`SelectStatement`] + catalog.
//!
//! Ordinals and scope are **exactly** those the TCT parser already resolved
//! into the AST (`TableRef.menu_index`, `ColumnRef.menu_index`). This is not a
//! Python reimplementation of menu policy: callers must obtain the statement
//! via [`crate::sql_schema_linking::codec::SqlSchemaLinkingCodec::parse`] (the
//! same path as A5 encode).

use crate::sql_schema_linking::ast::{
    AggregateArg, ColumnRef, Expression, InRhs, OrderKey, SelectItem, SelectList, SelectStatement,
    TableRef,
};
use crate::sql_schema_linking::catalog::SqlCatalog;

/// One menu site in from-first decision-stream order (A5 tree order).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MenuTraceSite {
    pub clause: &'static str,
    pub kind: &'static str,
    pub ordinal: u32,
    pub name: String,
    pub qualifier_name: Option<String>,
    pub qualifier_ordinal: Option<u32>,
}

impl MenuTraceSite {
    /// JSON-friendly map for Python / handoff sidecars.
    pub fn to_json_map(&self) -> serde_json::Map<String, serde_json::Value> {
        let mut m = serde_json::Map::new();
        m.insert("clause".into(), self.clause.into());
        m.insert("kind".into(), self.kind.into());
        m.insert("ordinal".into(), self.ordinal.into());
        m.insert("name".into(), self.name.clone().into());
        if let Some(ref q) = self.qualifier_name {
            m.insert("qualifier_name".into(), q.clone().into());
        }
        if let Some(qo) = self.qualifier_ordinal {
            m.insert("qualifier_ordinal".into(), qo.into());
        }
        m
    }
}

/// Fail-closed menu-trace error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MenuTraceError {
    pub message: String,
}

impl MenuTraceError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for MenuTraceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "menu_trace: {}", self.message)
    }
}

impl std::error::Error for MenuTraceError {}

/// Extract menu sites from a **parsed** statement using catalog name lookup.
///
/// Clause order matches A5 from-first field order: from/join, where, group_by,
/// having, order_by, select (star/COUNT(*) emit no column menu sites).
pub fn menu_trace_from_select(
    stmt: &SelectStatement,
    catalog: &SqlCatalog,
) -> Result<Vec<MenuTraceSite>, MenuTraceError> {
    let mut out = Vec::new();
    let mut scope: Vec<u32> = Vec::new();

    push_table(&mut out, &mut scope, catalog, &stmt.from.head, "from")?;
    for join in &stmt.from.joins {
        push_table(&mut out, &mut scope, catalog, &join.table, "join")?;
        emit_expr_columns(&mut out, catalog, &scope, &join.on, "join")?;
    }
    if let Some(ref w) = stmt.where_clause {
        emit_expr_columns(&mut out, catalog, &scope, w, "where")?;
    }
    if let Some(ref g) = stmt.group_by {
        for c in &g.columns {
            push_column(&mut out, catalog, &scope, c, "group_by")?;
        }
    }
    if let Some(ref h) = stmt.having {
        emit_expr_columns(&mut out, catalog, &scope, h, "having")?;
    }
    if let Some(ref o) = stmt.order_by {
        for item in &o.items {
            match &item.key {
                OrderKey::Column(c) => push_column(&mut out, catalog, &scope, c, "order_by")?,
                OrderKey::CountStar => {}
                OrderKey::Aggregate { arg, .. } => match arg {
                    AggregateArg::Column(c) => {
                        push_column(&mut out, catalog, &scope, c, "order_by")?
                    }
                },
            }
        }
    }
    match &stmt.select_list {
        SelectList::Star => {}
        SelectList::Items(items) => {
            for item in &items.items {
                match item {
                    SelectItem::Column(c) => push_column(&mut out, catalog, &scope, c, "select")?,
                    SelectItem::CountStar => {}
                    SelectItem::Aggregate { arg, .. } => match arg {
                        AggregateArg::Column(c) => {
                            push_column(&mut out, catalog, &scope, c, "select")?
                        }
                    },
                }
            }
        }
    }
    Ok(out)
}

fn push_table(
    out: &mut Vec<MenuTraceSite>,
    scope: &mut Vec<u32>,
    catalog: &SqlCatalog,
    table: &TableRef,
    clause: &'static str,
) -> Result<(), MenuTraceError> {
    let idx = table.menu_index as usize;
    let name = catalog
        .tables
        .get(idx)
        .map(table_surface_name)
        .ok_or_else(|| {
            MenuTraceError::new(format!(
                "table menu_index {} out of catalog range {}",
                table.menu_index,
                catalog.tables.len()
            ))
        })?;
    out.push(MenuTraceSite {
        clause,
        kind: "table",
        ordinal: table.menu_index,
        name,
        qualifier_name: None,
        qualifier_ordinal: None,
    });
    scope.push(table.menu_index);
    Ok(())
}

fn push_column(
    out: &mut Vec<MenuTraceSite>,
    catalog: &SqlCatalog,
    scope: &[u32],
    col: &ColumnRef,
    clause: &'static str,
) -> Result<(), MenuTraceError> {
    let (name, owner_table_idx) = resolve_column_name(catalog, scope, col.menu_index)?;
    let (qualifier_name, qualifier_ordinal) = if scope.len() >= 2 {
        let t = &catalog.tables[owner_table_idx];
        (Some(table_surface_name(t)), Some(owner_table_idx as u32))
    } else {
        (None, None)
    };
    out.push(MenuTraceSite {
        clause,
        kind: "column",
        ordinal: col.menu_index,
        name,
        qualifier_name,
        qualifier_ordinal,
    });
    Ok(())
}

fn resolve_column_name(
    catalog: &SqlCatalog,
    scope: &[u32],
    ordinal: u32,
) -> Result<(String, usize), MenuTraceError> {
    let mut cursor = 0u32;
    for &t_idx in scope {
        let ti = t_idx as usize;
        let table = catalog.tables.get(ti).ok_or_else(|| {
            MenuTraceError::new(format!("scope table index {t_idx} out of catalog range"))
        })?;
        let n = table.columns.len() as u32;
        if ordinal < cursor + n {
            let local = (ordinal - cursor) as usize;
            let col = &table.columns[local];
            let name = if !col.name_original.is_empty() {
                col.name_original.clone()
            } else {
                col.name.clone()
            };
            return Ok((name, ti));
        }
        cursor += n;
    }
    Err(MenuTraceError::new(format!(
        "column menu_index {ordinal} out of scope (scope={scope:?})"
    )))
}

fn table_surface_name(table: &crate::sql_schema_linking::catalog::SqlTable) -> String {
    if !table.name_original.is_empty() {
        table.name_original.clone()
    } else {
        table.name.clone()
    }
}

fn emit_expr_columns(
    out: &mut Vec<MenuTraceSite>,
    catalog: &SqlCatalog,
    scope: &[u32],
    expr: &Expression,
    clause: &'static str,
) -> Result<(), MenuTraceError> {
    match expr {
        Expression::Column(c) => push_column(out, catalog, scope, c, clause),
        Expression::IntegerLiteral(_) | Expression::StringLiteral(_) | Expression::CountStar => {
            Ok(())
        }
        Expression::Comparison { left, right, .. } => {
            emit_expr_columns(out, catalog, scope, left, clause)?;
            emit_expr_columns(out, catalog, scope, right, clause)
        }
        Expression::BinaryBool { left, right, .. } => {
            emit_expr_columns(out, catalog, scope, left, clause)?;
            emit_expr_columns(out, catalog, scope, right, clause)
        }
        Expression::Subquery(inner) => {
            // Nested uncorrelated scope: collect sites under "select" of inner
            // but label clause as the outer site's clause for IR simplicity.
            let nested = menu_trace_from_select(inner, catalog)?;
            out.extend(nested);
            Ok(())
        }
        Expression::In { left, right, .. } => {
            emit_expr_columns(out, catalog, scope, left, clause)?;
            match right {
                InRhs::Subquery(inner) => {
                    let nested = menu_trace_from_select(inner, catalog)?;
                    out.extend(nested);
                    Ok(())
                }
                InRhs::List(list) => {
                    for e in list {
                        emit_expr_columns(out, catalog, scope, e, clause)?;
                    }
                    Ok(())
                }
            }
        }
        Expression::Aggregate { arg, .. } => match arg {
            AggregateArg::Column(c) => push_column(out, catalog, scope, c, clause),
        },
        Expression::Between { expr: e, low, high } => {
            emit_expr_columns(out, catalog, scope, e, clause)?;
            emit_expr_columns(out, catalog, scope, low, clause)?;
            emit_expr_columns(out, catalog, scope, high, clause)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sql_schema_linking::catalog::load_fixture_catalog;
    use crate::sql_schema_linking::parser::parse_select;

    #[test]
    fn fixture_select_name_emits_table_then_column_menu_sites() {
        let catalog = load_fixture_catalog().expect("catalog");
        let stmt = parse_select("SELECT name FROM customers", &catalog).expect("parse");
        let trace = menu_trace_from_select(&stmt, &catalog).expect("trace");
        assert!(
            trace
                .iter()
                .any(|s| s.kind == "table" && s.name == "customers"),
            "trace={trace:?}"
        );
        assert!(
            trace.iter().any(|s| s.kind == "column" && s.name == "name"),
            "trace={trace:?}"
        );
        // From-first: table site before select column site.
        let t_pos = trace.iter().position(|s| s.kind == "table").unwrap();
        let c_pos = trace.iter().position(|s| s.kind == "column").unwrap();
        assert!(t_pos < c_pos);
    }
}
