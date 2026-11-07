//! SQL schema-linking AST (T5–T9).
//!
//! Language values for the SQLite Spider subset.
//! - M0: single-table `SELECT <columns|*> FROM <table>`
//! - M1: INNER/LEFT JOIN with ON predicates
//! - M2: WHERE with comparisons, AND/OR, integer/string literals
//! - M3: GROUP BY / HAVING / ORDER BY / LIMIT
//! - M4: aggregation + `COUNT(*)` + DISTINCT
//! - M5: uncorrelated subquery (inner scope only; **P9**)
//!
//! Menu ordinals are catalog-resolved scope-relative indices (**P4**).

/// Resolved table reference (menu ordinal into `catalog.tables`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableRef {
    /// Catalog-order table menu ordinal.
    pub menu_index: u32,
}

/// Column reference: scope-relative menu ordinal (**P4**/**P7**).
///
/// Qualifier uses are not stored; render derives them from scope when
/// two or more tables are present.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColumnRef {
    /// Scope-relative column menu ordinal.
    pub menu_index: u32,
}

/// Aggregate function for `AGGREGATE_CALL` (M4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AggFunc {
    Count,
    Sum,
    Avg,
    Min,
    Max,
}

/// Aggregate argument: column only.
///
/// Structural `COUNT(*)` is [`SelectItem::CountStar`], not
/// `Aggregate { func: Count, arg: Star }`. Star-args on SUM/AVG/MIN/MAX are
/// outside the declared subset and must fail closed at the tree boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AggregateArg {
    Column(ColumnRef),
}

/// One select-list item (M0 column; M4 COUNT(*) / aggregate).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SelectItem {
    Column(ColumnRef),
    /// Structural `COUNT(*)` — zero menu tokens (**P7**).
    CountStar,
    Aggregate {
        func: AggFunc,
        arg: AggregateArg,
        /// When true, emit `FUNC(DISTINCT col)` (COUNT_DISTINCT residual lift).
        distinct: bool,
    },
}

/// Explicit select-item list with optional DISTINCT marker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectItems {
    pub distinct: bool,
    pub items: Vec<SelectItem>,
}

/// Select list: star (structural, zero menu tokens) or explicit items (**P7**).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SelectList {
    Star,
    Items(SelectItems),
}

/// JOIN kind (v0: INNER default keyword `JOIN`; LEFT for `LEFT JOIN`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JoinKind {
    Inner,
    Left,
}

/// Comparison operator for ON / WHERE / HAVING (M2+).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CmpOp {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    Like,
}

/// Boolean connective (AND / OR).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoolOp {
    And,
    Or,
}

/// Expression subset used by JOIN ON, WHERE, HAVING, and scalar subqueries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expression {
    Column(ColumnRef),
    /// Decimal integer on the shared 256-wide integer band (variable-width
    /// multi-digit; capacity `0..=16_777_215`).
    IntegerLiteral(u32),
    /// UTF-8 string literal payload (SQL single-quoted form on render).
    StringLiteral(String),
    /// Structural COUNT(*) for HAVING predicates (no menu token).
    CountStar,
    Comparison {
        left: Box<Expression>,
        op: CmpOp,
        right: Box<Expression>,
    },
    BinaryBool {
        left: Box<Expression>,
        op: BoolOp,
        right: Box<Expression>,
    },
    /// Uncorrelated subquery only (**P9**); opens its own scope root.
    Subquery(Box<SelectStatement>),
    /// `expr [NOT] IN (SELECT … | value, …)` — residual IN lift.
    ///
    /// Subquery RHS is uncorrelated only (**P9**). Value lists are non-empty
    /// expression sequences (typically u8 integer / string atoms).
    /// Polarity is structural (`in_positive` / `in_negated`); no menu token.
    In {
        left: Box<Expression>,
        negated: bool,
        right: InRhs,
    },
    /// Column aggregate in expression position (HAVING SUM/AVG/MIN/MAX/COUNT(col)).
    ///
    /// Structural `COUNT(*)` remains [`Expression::CountStar`]. Shares the
    /// `aggregate_call` GrammarIR rule with select-list aggregates.
    Aggregate {
        func: AggFunc,
        arg: AggregateArg,
        distinct: bool,
    },
    /// `expr BETWEEN low AND high` (comparison level; NOT BETWEEN deferred).
    ///
    /// Bounds are ordinary expressions (typically u8 integer literals). Values
    /// outside the SharedAlphabet u8 band still fail closed at the literal parser.
    Between {
        expr: Box<Expression>,
        low: Box<Expression>,
        high: Box<Expression>,
    },
}

/// RHS of an IN / NOT IN predicate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InRhs {
    /// Uncorrelated subquery only (**P9**).
    Subquery(Box<SelectStatement>),
    /// Non-empty value list: `IN (1, 2, 'a')`.
    List(Vec<Expression>),
}

/// One JOIN clause with required ON predicate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JoinClause {
    pub kind: JoinKind,
    pub table: TableRef,
    pub on: Expression,
}

/// FROM clause: head table plus zero or more JOINs (T6).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FromClause {
    pub head: TableRef,
    pub joins: Vec<JoinClause>,
}

/// GROUP BY column list (M3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupByClause {
    pub columns: Vec<ColumnRef>,
}

/// ORDER BY direction (always explicit in the canonical stream).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderDirection {
    Asc,
    Desc,
}

/// ORDER BY key: column menu site, structural COUNT(*), or column aggregate.
///
/// COUNT(*) / SUM/AVG/MIN/MAX(col) lift residual ORDER_BY_AGG without
/// expanding HAVING aggregates or COUNT(DISTINCT). SharedAlphabet integer
/// band is unchanged.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OrderKey {
    Column(ColumnRef),
    CountStar,
    Aggregate {
        func: AggFunc,
        arg: AggregateArg,
        /// When true, emit `FUNC(DISTINCT col)` (matches select-list aggregates).
        distinct: bool,
    },
}

/// One ORDER BY item: key + direction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrderItem {
    pub key: OrderKey,
    pub direction: OrderDirection,
}

impl OrderItem {
    /// Convenience for column-key items (existing call sites / tests).
    pub fn column(column: ColumnRef, direction: OrderDirection) -> Self {
        Self {
            key: OrderKey::Column(column),
            direction,
        }
    }
}

/// ORDER BY clause (M3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrderByClause {
    pub items: Vec<OrderItem>,
}

/// Root select statement (from-first logical order; surface render uses SQL order).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectStatement {
    pub from: FromClause,
    pub where_clause: Option<Expression>,
    pub group_by: Option<GroupByClause>,
    pub having: Option<Expression>,
    pub order_by: Option<OrderByClause>,
    /// LIMIT value on the shared integer band (variable-width; capacity
    /// `0..=16_777_215`), if present.
    pub limit: Option<u32>,
    pub select_list: SelectList,
}

impl SelectStatement {
    /// Table menu indices currently in scope (FROM head then JOIN tables in order).
    pub fn scope_table_indices(&self) -> Vec<u32> {
        let mut scope = vec![self.from.head.menu_index];
        for join in &self.from.joins {
            scope.push(join.table.menu_index);
        }
        scope
    }

    /// Number of column-ref sites in the select list (star / COUNT(*) → 0).
    pub fn column_site_count(&self) -> usize {
        match &self.select_list {
            SelectList::Star => 0,
            SelectList::Items(items) => items
                .items
                .iter()
                .filter(|item| matches!(item, SelectItem::Column(_)))
                .count(),
        }
    }
}
