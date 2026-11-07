//! AST ↔ model-conforming [`RawSyntaxDecodedNode`] (T5–T9).
//!
//! Language-specific lower/raise for the generic codec layer (**P2**). Field
//! order on `select_statement` is from-first (**P6**). Optional absent fields
//! are materialised with `missing: true` so encode→decode tree identity holds.

use crate::sql_schema_linking::ast::{
    AggFunc, AggregateArg, BoolOp, CmpOp, ColumnRef, Expression, FromClause, GroupByClause,
    JoinClause, JoinKind, OrderByClause, OrderDirection, OrderItem, SelectItem, SelectItems,
    SelectList, SelectStatement, TableRef,
};
use crate::sql_schema_linking::grammar::{
    AGG_AVG_RULE, AGG_COUNT_RULE, AGG_MAX_RULE, AGG_MIN_RULE, AGG_SUM_RULE, AGGREGATE_CALL_RULE,
    BETWEEN_PREDICATE_RULE, BINARY_BOOL_RULE, COLUMN_MENU_OWNER_FIELD, COLUMN_MENU_OWNER_RULE,
    COLUMN_REF_RULE, COMPARISON_RULE, COUNT_STAR_RULE, DISTINCT_MARKER_RULE, EXPRESSION_RULE,
    FROM_CLAUSE_RULE, GROUP_BY_CLAUSE_RULE, IN_NEGATED_RULE, IN_POSITIVE_RULE, IN_PREDICATE_RULE,
    IN_VALUE_LIST_RULE, INTEGER_LITERAL_RULE, JOIN_CLAUSE_RULE, JOIN_INNER_RULE, JOIN_LEFT_RULE,
    LIMIT_CLAUSE_RULE, LIMITED_SELECT_DISTINCT_ITEMS_RULE, LIMITED_SELECT_ITEMS_RULE,
    LIMITED_STAR_RULE, OP_AND_RULE, OP_EQ_RULE, OP_GE_RULE, OP_GT_RULE, OP_LE_RULE, OP_LIKE_RULE,
    OP_LT_RULE, OP_NE_RULE, OP_OR_RULE, ORDER_ASC_RULE, ORDER_BY_CLAUSE_RULE, ORDER_DESC_RULE,
    ORDER_ITEM_RULE, ORDERED_LIMITED_SELECT_DISTINCT_ITEMS_RULE, ORDERED_LIMITED_SELECT_ITEMS_RULE,
    ORDERED_LIMITED_STAR_RULE, ORDERED_SELECT_DISTINCT_ITEMS_RULE, ORDERED_SELECT_ITEMS_RULE,
    ORDERED_STAR_RULE, SELECT_DISTINCT_ITEMS_RULE, SELECT_ITEM_RULE, SELECT_ITEMS_RULE,
    SELECT_STATEMENT_RULE, STAR_RULE, STRING_LITERAL_RULE, SUBQUERY_RULE, TABLE_MENU_OWNER_FIELD,
    TABLE_MENU_OWNER_RULE, TABLE_REF_RULE,
};
use crate::syntax_state_machine::{
    RawSyntaxDecodedField, RawSyntaxDecodedNode, RawSyntaxDecodedTerminal, RawSyntaxDecodedValue,
};

/// Lower an M0–M5 select AST to a model-conforming decoded-node tree.
pub fn select_to_decoded_node(stmt: &SelectStatement) -> RawSyntaxDecodedNode {
    select_to_decoded_node_with_fold_select_distinct(stmt, false)
}

/// Lower with the rank-1 select_list × distinct fold (3-way select_list alt).
pub fn select_to_decoded_node_with_fold_select_distinct(
    stmt: &SelectStatement,
    fold_select_distinct: bool,
) -> RawSyntaxDecodedNode {
    let mut fields = vec![
        present_field(
            "from",
            vec![RawSyntaxDecodedValue::Node(Box::new(from_to_node(
                &stmt.from,
                fold_select_distinct,
            )))],
        ),
        optional_expression_field("where", stmt.where_clause.as_ref(), fold_select_distinct),
        optional_group_by_field(
            stmt.group_by.as_ref(),
            stmt.having.as_ref(),
            fold_select_distinct,
        ),
    ];
    if !fold_select_distinct {
        fields.push(optional_expression_field(
            "having",
            stmt.having.as_ref(),
            fold_select_distinct,
        ));
    }
    if !fold_select_distinct {
        fields.push(optional_order_by_field(stmt.order_by.as_ref()));
        fields.push(optional_limit_field(stmt.limit));
    }
    fields.push(present_field(
        "select_list",
        vec![RawSyntaxDecodedValue::Node(Box::new(select_list_to_node(
            &stmt.select_list,
            stmt.limit,
            stmt.order_by.as_ref(),
            fold_select_distinct,
        )))],
    ));
    RawSyntaxDecodedNode {
        rule_name: SELECT_STATEMENT_RULE.to_string(),
        node_kind: SELECT_STATEMENT_RULE.to_string(),
        fields,
    }
}

/// Raise a decoded-node tree back to an M0–M5 select AST.
pub fn select_from_decoded_node(node: &RawSyntaxDecodedNode) -> Result<SelectStatement, TreeError> {
    if node.rule_name != SELECT_STATEMENT_RULE || node.node_kind != SELECT_STATEMENT_RULE {
        return Err(TreeError::new(format!(
            "expected root {SELECT_STATEMENT_RULE}, got rule={} kind={}",
            node.rule_name, node.node_kind
        )));
    }
    let from_node = required_child(node, "from")?;
    let from = from_from_node(from_node)?;
    let where_clause = optional_expression_from_field(node, "where")?;
    let (group_by, folded_having) = optional_group_by_from_field(node)?;
    let having = if node.field("having").is_some() {
        optional_expression_from_field(node, "having")?
    } else {
        folded_having
    };
    let select_list_node = required_child(node, "select_list")?;
    let (select_list, folded_limit, folded_order) = select_list_from_node(select_list_node)?;
    let order_by = if node.field("order_by").is_some() {
        optional_order_by_from_field(node)?
    } else {
        folded_order
    };
    let limit = if node.field("limit").is_some() {
        optional_limit_from_field(node)?
    } else {
        folded_limit
    };
    Ok(SelectStatement {
        from,
        where_clause,
        group_by,
        having,
        order_by,
        limit,
        select_list,
    })
}

fn optional_expression_field(
    name: &str,
    expr: Option<&Expression>,
    fold_select_distinct: bool,
) -> RawSyntaxDecodedField {
    match expr {
        Some(e) => present_field(
            name,
            vec![RawSyntaxDecodedValue::Node(Box::new(expression_to_node(
                e,
                fold_select_distinct,
            )))],
        ),
        None => missing_field(name),
    }
}

fn optional_expression_from_field(
    node: &RawSyntaxDecodedNode,
    name: &str,
) -> Result<Option<Expression>, TreeError> {
    let Some(field) = node.field(name) else {
        return Ok(None);
    };
    if field.missing || field.values.is_empty() {
        return Ok(None);
    }
    let child = value_as_node(&field.values[0], name)?;
    Ok(Some(expression_from_node(child)?))
}

fn optional_group_by_field(
    group: Option<&GroupByClause>,
    having: Option<&Expression>,
    fold_having_under_group_by: bool,
) -> RawSyntaxDecodedField {
    match group {
        Some(g) => present_field(
            "group_by",
            vec![RawSyntaxDecodedValue::Node(Box::new(group_by_to_node(
                g,
                having,
                fold_having_under_group_by,
            )))],
        ),
        None => missing_field("group_by"),
    }
}

fn optional_group_by_from_field(
    node: &RawSyntaxDecodedNode,
) -> Result<(Option<GroupByClause>, Option<Expression>), TreeError> {
    let Some(field) = node.field("group_by") else {
        return Ok((None, None));
    };
    if field.missing || field.values.is_empty() {
        return Ok((None, None));
    }
    let child = value_as_node(&field.values[0], "group_by")?;
    let (group, having) = group_by_from_node(child)?;
    Ok((Some(group), having))
}

fn optional_order_by_field(order: Option<&OrderByClause>) -> RawSyntaxDecodedField {
    match order {
        Some(o) => present_field(
            "order_by",
            vec![RawSyntaxDecodedValue::Node(Box::new(order_by_to_node(o)))],
        ),
        None => missing_field("order_by"),
    }
}

fn optional_order_by_from_field(
    node: &RawSyntaxDecodedNode,
) -> Result<Option<OrderByClause>, TreeError> {
    let Some(field) = node.field("order_by") else {
        return Ok(None);
    };
    if field.missing || field.values.is_empty() {
        return Ok(None);
    }
    let child = value_as_node(&field.values[0], "order_by")?;
    Ok(Some(order_by_from_node(child)?))
}

fn optional_limit_field(limit: Option<u32>) -> RawSyntaxDecodedField {
    match limit {
        Some(v) => present_field(
            "limit",
            vec![RawSyntaxDecodedValue::Node(Box::new(limit_to_node(v)))],
        ),
        None => missing_field("limit"),
    }
}

fn optional_limit_from_field(node: &RawSyntaxDecodedNode) -> Result<Option<u32>, TreeError> {
    let Some(field) = node.field("limit") else {
        return Ok(None);
    };
    if field.missing || field.values.is_empty() {
        return Ok(None);
    }
    let child = value_as_node(&field.values[0], "limit")?;
    Ok(Some(limit_from_node(child)?))
}

fn group_by_to_node(
    group: &GroupByClause,
    having: Option<&Expression>,
    fold_having_under_group_by: bool,
) -> RawSyntaxDecodedNode {
    let values: Vec<RawSyntaxDecodedValue> = group
        .columns
        .iter()
        .map(|c| RawSyntaxDecodedValue::Node(Box::new(column_ref_to_node(c))))
        .collect();
    let mut fields = vec![present_field("columns", values)];
    if fold_having_under_group_by {
        fields.push(optional_expression_field(
            "having",
            having,
            fold_having_under_group_by,
        ));
    }
    RawSyntaxDecodedNode {
        rule_name: GROUP_BY_CLAUSE_RULE.to_string(),
        node_kind: GROUP_BY_CLAUSE_RULE.to_string(),
        fields,
    }
}

fn group_by_from_node(
    node: &RawSyntaxDecodedNode,
) -> Result<(GroupByClause, Option<Expression>), TreeError> {
    require_rule(node, GROUP_BY_CLAUSE_RULE)?;
    let columns_field = node
        .field("columns")
        .ok_or_else(|| TreeError::new("group_by_clause missing columns"))?;
    if columns_field.missing || columns_field.values.is_empty() {
        return Err(TreeError::new("group_by_clause.columns must be non-empty"));
    }
    let mut columns = Vec::with_capacity(columns_field.values.len());
    for value in &columns_field.values {
        let child = value_as_node(value, "group_by_clause.columns")?;
        columns.push(column_ref_from_node(child)?);
    }
    let having = optional_expression_from_field(node, "having")?;
    Ok((GroupByClause { columns }, having))
}

fn order_by_to_node(order: &OrderByClause) -> RawSyntaxDecodedNode {
    let values: Vec<RawSyntaxDecodedValue> = order
        .items
        .iter()
        .map(|item| RawSyntaxDecodedValue::Node(Box::new(order_item_to_node(item))))
        .collect();
    RawSyntaxDecodedNode {
        rule_name: ORDER_BY_CLAUSE_RULE.to_string(),
        node_kind: ORDER_BY_CLAUSE_RULE.to_string(),
        fields: vec![present_field("items", values)],
    }
}

fn order_by_from_node(node: &RawSyntaxDecodedNode) -> Result<OrderByClause, TreeError> {
    require_rule(node, ORDER_BY_CLAUSE_RULE)?;
    let items_field = node
        .field("items")
        .ok_or_else(|| TreeError::new("order_by_clause missing items"))?;
    if items_field.missing || items_field.values.is_empty() {
        return Err(TreeError::new("order_by_clause.items must be non-empty"));
    }
    let mut items = Vec::with_capacity(items_field.values.len());
    for value in &items_field.values {
        let child = value_as_node(value, "order_by_clause.items")?;
        items.push(order_item_from_node(child)?);
    }
    Ok(OrderByClause { items })
}

fn order_item_to_node(item: &OrderItem) -> RawSyntaxDecodedNode {
    use crate::sql_schema_linking::ast::OrderKey;

    let dir_rule = match item.direction {
        OrderDirection::Asc => ORDER_ASC_RULE,
        OrderDirection::Desc => ORDER_DESC_RULE,
    };
    let key_node = match &item.key {
        OrderKey::Column(col) => column_ref_to_node(col),
        OrderKey::CountStar => field_less_leaf(COUNT_STAR_RULE),
        OrderKey::Aggregate {
            func,
            arg,
            distinct,
        } => aggregate_to_node(*func, arg, *distinct),
    };
    RawSyntaxDecodedNode {
        rule_name: ORDER_ITEM_RULE.to_string(),
        node_kind: ORDER_ITEM_RULE.to_string(),
        fields: vec![
            present_field(
                "column",
                vec![RawSyntaxDecodedValue::Node(Box::new(key_node))],
            ),
            present_field(
                "direction",
                vec![RawSyntaxDecodedValue::Node(Box::new(field_less_leaf(
                    dir_rule,
                )))],
            ),
        ],
    }
}

fn order_item_from_node(node: &RawSyntaxDecodedNode) -> Result<OrderItem, TreeError> {
    use crate::sql_schema_linking::ast::OrderKey;

    require_rule(node, ORDER_ITEM_RULE)?;
    let key_node = required_child(node, "column")?;
    let key = match key_node.node_kind.as_str() {
        COLUMN_REF_RULE => OrderKey::Column(column_ref_from_node(key_node)?),
        COUNT_STAR_RULE => OrderKey::CountStar,
        AGGREGATE_CALL_RULE => match aggregate_from_node(key_node)? {
            SelectItem::Aggregate {
                func,
                arg,
                distinct,
            } => OrderKey::Aggregate {
                func,
                arg,
                distinct,
            },
            other => {
                return Err(TreeError::new(format!(
                    "expected aggregate order key, got {other:?}"
                )));
            }
        },
        other => {
            return Err(TreeError::new(format!(
                "unsupported order key kind {other}"
            )));
        }
    };
    let dir_node = required_child(node, "direction")?;
    let direction = match dir_node.node_kind.as_str() {
        ORDER_ASC_RULE => OrderDirection::Asc,
        ORDER_DESC_RULE => OrderDirection::Desc,
        other => {
            return Err(TreeError::new(format!(
                "unsupported order direction {other}"
            )));
        }
    };
    Ok(OrderItem { key, direction })
}

fn limit_to_node(value: u32) -> RawSyntaxDecodedNode {
    RawSyntaxDecodedNode {
        rule_name: LIMIT_CLAUSE_RULE.to_string(),
        node_kind: LIMIT_CLAUSE_RULE.to_string(),
        fields: vec![present_field(
            "value",
            vec![RawSyntaxDecodedValue::Terminal(RawSyntaxDecodedTerminal {
                node_kind: "limit_value".to_string(),
                terminal: "integer".to_string(),
                payload: value.to_string(),
            })],
        )],
    }
}

fn limit_from_node(node: &RawSyntaxDecodedNode) -> Result<u32, TreeError> {
    require_rule(node, LIMIT_CLAUSE_RULE)?;
    let f = node
        .field("value")
        .ok_or_else(|| TreeError::new("limit_clause missing value"))?;
    if f.missing || f.values.is_empty() {
        return Err(TreeError::new("limit_clause.value has no payload"));
    }
    match &f.values[0] {
        RawSyntaxDecodedValue::Terminal(t) => parse_integer_payload(&t.payload, "limit"),
        RawSyntaxDecodedValue::Node(_) => {
            Err(TreeError::new("limit value expected Terminal, got Node"))
        }
    }
}

fn from_to_node(from: &FromClause, fold_select_distinct: bool) -> RawSyntaxDecodedNode {
    let joins_field = if from.joins.is_empty() {
        missing_field("joins")
    } else {
        present_field(
            "joins",
            from.joins
                .iter()
                .map(|j| {
                    RawSyntaxDecodedValue::Node(Box::new(join_to_node(j, fold_select_distinct)))
                })
                .collect(),
        )
    };
    RawSyntaxDecodedNode {
        rule_name: FROM_CLAUSE_RULE.to_string(),
        node_kind: FROM_CLAUSE_RULE.to_string(),
        fields: vec![
            present_field(
                "head",
                vec![RawSyntaxDecodedValue::Node(Box::new(table_ref_to_node(
                    &from.head,
                )))],
            ),
            joins_field,
        ],
    }
}

fn from_from_node(node: &RawSyntaxDecodedNode) -> Result<FromClause, TreeError> {
    require_rule(node, FROM_CLAUSE_RULE)?;
    let head = table_ref_from_node(required_child(node, "head")?)?;
    let mut joins = Vec::new();
    if let Some(joins_field) = node.field("joins")
        && !joins_field.missing
    {
        for value in &joins_field.values {
            let child = value_as_node(value, "from_clause.joins")?;
            joins.push(join_from_node(child)?);
        }
    }
    Ok(FromClause { head, joins })
}

fn join_to_node(join: &JoinClause, fold_select_distinct: bool) -> RawSyntaxDecodedNode {
    let kind_rule = match join.kind {
        JoinKind::Inner => JOIN_INNER_RULE,
        JoinKind::Left => JOIN_LEFT_RULE,
    };
    RawSyntaxDecodedNode {
        rule_name: JOIN_CLAUSE_RULE.to_string(),
        node_kind: JOIN_CLAUSE_RULE.to_string(),
        fields: vec![
            present_field(
                "kind",
                vec![RawSyntaxDecodedValue::Node(Box::new(field_less_leaf(
                    kind_rule,
                )))],
            ),
            present_field(
                "table",
                vec![RawSyntaxDecodedValue::Node(Box::new(table_ref_to_node(
                    &join.table,
                )))],
            ),
            present_field(
                "on",
                vec![RawSyntaxDecodedValue::Node(Box::new(expression_to_node(
                    &join.on,
                    fold_select_distinct,
                )))],
            ),
        ],
    }
}

fn join_from_node(node: &RawSyntaxDecodedNode) -> Result<JoinClause, TreeError> {
    require_rule(node, JOIN_CLAUSE_RULE)?;
    let kind_node = required_child(node, "kind")?;
    let kind = match kind_node.node_kind.as_str() {
        JOIN_INNER_RULE => JoinKind::Inner,
        JOIN_LEFT_RULE => JoinKind::Left,
        other => {
            return Err(TreeError::new(format!(
                "unsupported join kind node_kind {other}"
            )));
        }
    };
    let table = table_ref_from_node(required_child(node, "table")?)?;
    let on = expression_from_node(required_child(node, "on")?)?;
    Ok(JoinClause { kind, table, on })
}

fn expression_to_node(expr: &Expression, fold_select_distinct: bool) -> RawSyntaxDecodedNode {
    let inner = match expr {
        Expression::Column(col) => column_ref_to_node(col),
        Expression::IntegerLiteral(v) => integer_literal_to_node(*v),
        Expression::StringLiteral(s) => string_literal_to_node(s),
        Expression::CountStar => field_less_leaf(COUNT_STAR_RULE),
        Expression::Comparison { left, op, right } => {
            comparison_to_node(left, *op, right, fold_select_distinct)
        }
        Expression::BinaryBool { left, op, right } => {
            binary_bool_to_node(left, *op, right, fold_select_distinct)
        }
        Expression::Subquery(stmt) => subquery_to_node(stmt, fold_select_distinct),
        Expression::In {
            left,
            negated,
            right,
        } => in_predicate_to_node(left, *negated, right, fold_select_distinct),
        Expression::Aggregate {
            func,
            arg,
            distinct,
        } => aggregate_to_node(*func, arg, *distinct),
        Expression::Between { expr, low, high } => {
            between_to_node(expr, low, high, fold_select_distinct)
        }
    };
    RawSyntaxDecodedNode {
        rule_name: EXPRESSION_RULE.to_string(),
        node_kind: EXPRESSION_RULE.to_string(),
        fields: vec![present_field(
            "kind",
            vec![RawSyntaxDecodedValue::Node(Box::new(inner))],
        )],
    }
}

fn expression_from_node(node: &RawSyntaxDecodedNode) -> Result<Expression, TreeError> {
    require_rule(node, EXPRESSION_RULE)?;
    let kind = required_child(node, "kind")?;
    match kind.node_kind.as_str() {
        COLUMN_REF_RULE => Ok(Expression::Column(column_ref_from_node(kind)?)),
        INTEGER_LITERAL_RULE => Ok(Expression::IntegerLiteral(integer_literal_from_node(kind)?)),
        STRING_LITERAL_RULE => Ok(Expression::StringLiteral(string_literal_from_node(kind)?)),
        COUNT_STAR_RULE => Ok(Expression::CountStar),
        COMPARISON_RULE => comparison_from_node(kind),
        BINARY_BOOL_RULE => binary_bool_from_node(kind),
        SUBQUERY_RULE => Ok(Expression::Subquery(Box::new(subquery_from_node(kind)?))),
        IN_PREDICATE_RULE => in_predicate_from_node(kind),
        AGGREGATE_CALL_RULE => match aggregate_from_node(kind)? {
            SelectItem::Aggregate {
                func,
                arg,
                distinct,
            } => Ok(Expression::Aggregate {
                func,
                arg,
                distinct,
            }),
            other => Err(TreeError::new(format!(
                "aggregate_call under expression must be Aggregate, got {other:?}"
            ))),
        },
        BETWEEN_PREDICATE_RULE => between_from_node(kind),
        other => Err(TreeError::new(format!(
            "unsupported expression kind {other}"
        ))),
    }
}

fn between_to_node(
    expr: &Expression,
    low: &Expression,
    high: &Expression,
    fold_select_distinct: bool,
) -> RawSyntaxDecodedNode {
    RawSyntaxDecodedNode {
        rule_name: BETWEEN_PREDICATE_RULE.to_string(),
        node_kind: BETWEEN_PREDICATE_RULE.to_string(),
        fields: vec![
            present_field(
                "expr",
                vec![RawSyntaxDecodedValue::Node(Box::new(expression_to_node(
                    expr,
                    fold_select_distinct,
                )))],
            ),
            present_field(
                "low",
                vec![RawSyntaxDecodedValue::Node(Box::new(expression_to_node(
                    low,
                    fold_select_distinct,
                )))],
            ),
            present_field(
                "high",
                vec![RawSyntaxDecodedValue::Node(Box::new(expression_to_node(
                    high,
                    fold_select_distinct,
                )))],
            ),
        ],
    }
}

fn between_from_node(node: &RawSyntaxDecodedNode) -> Result<Expression, TreeError> {
    require_rule(node, BETWEEN_PREDICATE_RULE)?;
    Ok(Expression::Between {
        expr: Box::new(expression_from_node(required_child(node, "expr")?)?),
        low: Box::new(expression_from_node(required_child(node, "low")?)?),
        high: Box::new(expression_from_node(required_child(node, "high")?)?),
    })
}

fn in_predicate_to_node(
    left: &Expression,
    negated: bool,
    right: &crate::sql_schema_linking::ast::InRhs,
    fold_select_distinct: bool,
) -> RawSyntaxDecodedNode {
    use crate::sql_schema_linking::ast::InRhs;
    let polarity = if negated {
        field_less_leaf(IN_NEGATED_RULE)
    } else {
        field_less_leaf(IN_POSITIVE_RULE)
    };
    let right_node = match right {
        InRhs::Subquery(stmt) => subquery_to_node(stmt, fold_select_distinct),
        InRhs::List(values) => {
            let kids: Vec<RawSyntaxDecodedValue> = values
                .iter()
                .map(|v| {
                    RawSyntaxDecodedValue::Node(Box::new(expression_to_node(
                        v,
                        fold_select_distinct,
                    )))
                })
                .collect();
            RawSyntaxDecodedNode {
                rule_name: IN_VALUE_LIST_RULE.to_string(),
                node_kind: IN_VALUE_LIST_RULE.to_string(),
                fields: vec![present_field("values", kids)],
            }
        }
    };
    RawSyntaxDecodedNode {
        rule_name: IN_PREDICATE_RULE.to_string(),
        node_kind: IN_PREDICATE_RULE.to_string(),
        fields: vec![
            present_field(
                "left",
                vec![RawSyntaxDecodedValue::Node(Box::new(expression_to_node(
                    left,
                    fold_select_distinct,
                )))],
            ),
            present_field(
                "polarity",
                vec![RawSyntaxDecodedValue::Node(Box::new(polarity))],
            ),
            present_field(
                "right",
                vec![RawSyntaxDecodedValue::Node(Box::new(right_node))],
            ),
        ],
    }
}

fn in_predicate_from_node(node: &RawSyntaxDecodedNode) -> Result<Expression, TreeError> {
    use crate::sql_schema_linking::ast::InRhs;
    require_rule(node, IN_PREDICATE_RULE)?;
    let left = expression_from_node(required_child(node, "left")?)?;
    let polarity = required_child(node, "polarity")?;
    let negated = match polarity.node_kind.as_str() {
        IN_POSITIVE_RULE => false,
        IN_NEGATED_RULE => true,
        other => {
            return Err(TreeError::new(format!(
                "unsupported in_predicate polarity {other}"
            )));
        }
    };
    let right_node = required_child(node, "right")?;
    let right = match right_node.node_kind.as_str() {
        SUBQUERY_RULE => InRhs::Subquery(Box::new(subquery_from_node(right_node)?)),
        IN_VALUE_LIST_RULE => {
            let f = right_node
                .field("values")
                .ok_or_else(|| TreeError::new("in_value_list missing values"))?;
            if f.missing || f.values.is_empty() {
                return Err(TreeError::new("in_value_list.values must be non-empty"));
            }
            let mut values = Vec::with_capacity(f.values.len());
            for v in &f.values {
                let child = value_as_node(v, "in_value_list.values")?;
                values.push(expression_from_node(child)?);
            }
            InRhs::List(values)
        }
        other => {
            return Err(TreeError::new(format!(
                "unsupported in_predicate right {other}"
            )));
        }
    };
    Ok(Expression::In {
        left: Box::new(left),
        negated,
        right,
    })
}

fn subquery_to_node(stmt: &SelectStatement, fold_select_distinct: bool) -> RawSyntaxDecodedNode {
    RawSyntaxDecodedNode {
        rule_name: SUBQUERY_RULE.to_string(),
        node_kind: SUBQUERY_RULE.to_string(),
        fields: vec![present_field(
            "query",
            vec![RawSyntaxDecodedValue::Node(Box::new(
                select_to_decoded_node_with_fold_select_distinct(stmt, fold_select_distinct),
            ))],
        )],
    }
}

fn subquery_from_node(node: &RawSyntaxDecodedNode) -> Result<SelectStatement, TreeError> {
    require_rule(node, SUBQUERY_RULE)?;
    let query = required_child(node, "query")?;
    select_from_decoded_node(query)
}

fn integer_literal_to_node(value: u32) -> RawSyntaxDecodedNode {
    RawSyntaxDecodedNode {
        rule_name: INTEGER_LITERAL_RULE.to_string(),
        node_kind: INTEGER_LITERAL_RULE.to_string(),
        fields: vec![present_field(
            "value",
            vec![RawSyntaxDecodedValue::Terminal(RawSyntaxDecodedTerminal {
                node_kind: "integer_literal_value".to_string(),
                terminal: "integer".to_string(),
                payload: value.to_string(),
            })],
        )],
    }
}

fn integer_literal_from_node(node: &RawSyntaxDecodedNode) -> Result<u32, TreeError> {
    require_rule(node, INTEGER_LITERAL_RULE)?;
    let f = node
        .field("value")
        .ok_or_else(|| TreeError::new("integer_literal missing value"))?;
    if f.missing || f.values.is_empty() {
        return Err(TreeError::new("integer_literal.value has no payload"));
    }
    match &f.values[0] {
        RawSyntaxDecodedValue::Terminal(t) => parse_integer_payload(&t.payload, "integer_literal"),
        RawSyntaxDecodedValue::Node(_) => Err(TreeError::new(
            "integer_literal value expected Terminal, got Node",
        )),
    }
}

fn parse_integer_payload(payload: &str, subject: &str) -> Result<u32, TreeError> {
    let value: u64 = payload.parse().map_err(|_| {
        TreeError::new(format!(
            "{subject} payload {payload:?} is not a decimal integer"
        ))
    })?;
    let capacity = u64::from(crate::syntax_state_machine::INTEGER_PAYLOAD_CAPACITY_MAX);
    if value > capacity {
        return Err(TreeError::new(format!(
            "{subject} payload {value} exceeds integer capacity ({capacity})"
        )));
    }
    Ok(value as u32)
}

fn string_literal_to_node(value: &str) -> RawSyntaxDecodedNode {
    RawSyntaxDecodedNode {
        rule_name: STRING_LITERAL_RULE.to_string(),
        node_kind: STRING_LITERAL_RULE.to_string(),
        fields: vec![present_field(
            "value",
            vec![RawSyntaxDecodedValue::Terminal(RawSyntaxDecodedTerminal {
                node_kind: "string_literal_value".to_string(),
                terminal: "string".to_string(),
                payload: value.to_string(),
            })],
        )],
    }
}

fn string_literal_from_node(node: &RawSyntaxDecodedNode) -> Result<String, TreeError> {
    require_rule(node, STRING_LITERAL_RULE)?;
    let f = node
        .field("value")
        .ok_or_else(|| TreeError::new("string_literal missing value"))?;
    if f.missing || f.values.is_empty() {
        return Err(TreeError::new("string_literal.value has no payload"));
    }
    match &f.values[0] {
        RawSyntaxDecodedValue::Terminal(t) => {
            if t.payload.contains('\0') {
                return Err(TreeError::new(
                    "string_literal.value contains NUL, which is not representable in SQLite SQL text",
                ));
            }
            Ok(t.payload.clone())
        }
        RawSyntaxDecodedValue::Node(_) => Err(TreeError::new(
            "string_literal value expected Terminal, got Node",
        )),
    }
}

fn comparison_to_node(
    left: &Expression,
    op: CmpOp,
    right: &Expression,
    fold_select_distinct: bool,
) -> RawSyntaxDecodedNode {
    let op_rule = match op {
        CmpOp::Eq => OP_EQ_RULE,
        CmpOp::Ne => OP_NE_RULE,
        CmpOp::Lt => OP_LT_RULE,
        CmpOp::Le => OP_LE_RULE,
        CmpOp::Gt => OP_GT_RULE,
        CmpOp::Ge => OP_GE_RULE,
        CmpOp::Like => OP_LIKE_RULE,
    };
    RawSyntaxDecodedNode {
        rule_name: COMPARISON_RULE.to_string(),
        node_kind: COMPARISON_RULE.to_string(),
        fields: vec![
            present_field(
                "left",
                vec![RawSyntaxDecodedValue::Node(Box::new(expression_to_node(
                    left,
                    fold_select_distinct,
                )))],
            ),
            present_field(
                "op",
                vec![RawSyntaxDecodedValue::Node(Box::new(field_less_leaf(
                    op_rule,
                )))],
            ),
            present_field(
                "right",
                vec![RawSyntaxDecodedValue::Node(Box::new(expression_to_node(
                    right,
                    fold_select_distinct,
                )))],
            ),
        ],
    }
}

fn comparison_from_node(node: &RawSyntaxDecodedNode) -> Result<Expression, TreeError> {
    require_rule(node, COMPARISON_RULE)?;
    let left = expression_from_node(required_child(node, "left")?)?;
    let op_node = required_child(node, "op")?;
    let op = match op_node.node_kind.as_str() {
        OP_EQ_RULE => CmpOp::Eq,
        OP_NE_RULE => CmpOp::Ne,
        OP_LT_RULE => CmpOp::Lt,
        OP_LE_RULE => CmpOp::Le,
        OP_GT_RULE => CmpOp::Gt,
        OP_GE_RULE => CmpOp::Ge,
        OP_LIKE_RULE => CmpOp::Like,
        other => {
            return Err(TreeError::new(format!("unsupported comparison op {other}")));
        }
    };
    let right = expression_from_node(required_child(node, "right")?)?;
    Ok(Expression::Comparison {
        left: Box::new(left),
        op,
        right: Box::new(right),
    })
}

fn binary_bool_to_node(
    left: &Expression,
    op: BoolOp,
    right: &Expression,
    fold_select_distinct: bool,
) -> RawSyntaxDecodedNode {
    let op_rule = match op {
        BoolOp::And => OP_AND_RULE,
        BoolOp::Or => OP_OR_RULE,
    };
    RawSyntaxDecodedNode {
        rule_name: BINARY_BOOL_RULE.to_string(),
        node_kind: BINARY_BOOL_RULE.to_string(),
        fields: vec![
            present_field(
                "left",
                vec![RawSyntaxDecodedValue::Node(Box::new(expression_to_node(
                    left,
                    fold_select_distinct,
                )))],
            ),
            present_field(
                "op",
                vec![RawSyntaxDecodedValue::Node(Box::new(field_less_leaf(
                    op_rule,
                )))],
            ),
            present_field(
                "right",
                vec![RawSyntaxDecodedValue::Node(Box::new(expression_to_node(
                    right,
                    fold_select_distinct,
                )))],
            ),
        ],
    }
}

fn binary_bool_from_node(node: &RawSyntaxDecodedNode) -> Result<Expression, TreeError> {
    require_rule(node, BINARY_BOOL_RULE)?;
    let left = expression_from_node(required_child(node, "left")?)?;
    let op_node = required_child(node, "op")?;
    let op = match op_node.node_kind.as_str() {
        OP_AND_RULE => BoolOp::And,
        OP_OR_RULE => BoolOp::Or,
        other => {
            return Err(TreeError::new(format!("unsupported boolean op {other}")));
        }
    };
    let right = expression_from_node(required_child(node, "right")?)?;
    Ok(Expression::BinaryBool {
        left: Box::new(left),
        op,
        right: Box::new(right),
    })
}

fn table_ref_to_node(table: &TableRef) -> RawSyntaxDecodedNode {
    RawSyntaxDecodedNode {
        rule_name: TABLE_REF_RULE.to_string(),
        node_kind: TABLE_REF_RULE.to_string(),
        fields: vec![
            present_field(
                TABLE_MENU_OWNER_FIELD,
                vec![RawSyntaxDecodedValue::Terminal(RawSyntaxDecodedTerminal {
                    node_kind: "table_menu_index".to_string(),
                    terminal: "integer".to_string(),
                    payload: table.menu_index.to_string(),
                })],
            ),
            missing_field("alias"),
        ],
    }
}

fn table_ref_from_node(node: &RawSyntaxDecodedNode) -> Result<TableRef, TreeError> {
    require_rule(node, TABLE_REF_RULE)?;
    let menu_index = menu_payload(node, TABLE_MENU_OWNER_FIELD, TABLE_MENU_OWNER_RULE)?;
    Ok(TableRef { menu_index })
}

fn folded_select_list_rule(
    star: bool,
    distinct: bool,
    has_order: bool,
    has_limit: bool,
) -> &'static str {
    match (star, distinct, has_order, has_limit) {
        (true, _, false, false) => STAR_RULE,
        (true, _, false, true) => LIMITED_STAR_RULE,
        (true, _, true, false) => ORDERED_STAR_RULE,
        (true, _, true, true) => ORDERED_LIMITED_STAR_RULE,
        (false, false, false, false) => SELECT_ITEMS_RULE,
        (false, false, false, true) => LIMITED_SELECT_ITEMS_RULE,
        (false, false, true, false) => ORDERED_SELECT_ITEMS_RULE,
        (false, false, true, true) => ORDERED_LIMITED_SELECT_ITEMS_RULE,
        (false, true, false, false) => SELECT_DISTINCT_ITEMS_RULE,
        (false, true, false, true) => LIMITED_SELECT_DISTINCT_ITEMS_RULE,
        (false, true, true, false) => ORDERED_SELECT_DISTINCT_ITEMS_RULE,
        (false, true, true, true) => ORDERED_LIMITED_SELECT_DISTINCT_ITEMS_RULE,
    }
}

fn select_list_to_node(
    list: &SelectList,
    limit: Option<u32>,
    order_by: Option<&OrderByClause>,
    fold_select_distinct: bool,
) -> RawSyntaxDecodedNode {
    if fold_select_distinct {
        let has_order = order_by.is_some();
        let has_limit = limit.is_some();
        let mut fields = Vec::new();
        if let Some(order) = order_by {
            fields.push(present_field(
                "order_by",
                vec![RawSyntaxDecodedValue::Node(Box::new(order_by_to_node(
                    order,
                )))],
            ));
        }
        if let Some(value) = limit {
            fields.push(present_field(
                "limit",
                vec![RawSyntaxDecodedValue::Node(Box::new(limit_to_node(value)))],
            ));
        }
        let (star, distinct) = match list {
            SelectList::Star => (true, false),
            SelectList::Items(items) => {
                let values: Vec<RawSyntaxDecodedValue> = items
                    .items
                    .iter()
                    .map(|item| RawSyntaxDecodedValue::Node(Box::new(select_item_to_node(item))))
                    .collect();
                fields.push(present_field("items", values));
                (false, items.distinct)
            }
        };
        if star && !has_order && !has_limit {
            return field_less_leaf(STAR_RULE);
        }
        let rule = folded_select_list_rule(star, distinct, has_order, has_limit);
        return RawSyntaxDecodedNode {
            rule_name: rule.to_string(),
            node_kind: rule.to_string(),
            fields,
        };
    }
    match list {
        SelectList::Star => field_less_leaf(STAR_RULE),
        SelectList::Items(items) => {
            let values: Vec<RawSyntaxDecodedValue> = items
                .items
                .iter()
                .map(|item| RawSyntaxDecodedValue::Node(Box::new(select_item_to_node(item))))
                .collect();
            let distinct_field = if items.distinct {
                present_field(
                    "distinct",
                    vec![RawSyntaxDecodedValue::Node(Box::new(field_less_leaf(
                        DISTINCT_MARKER_RULE,
                    )))],
                )
            } else {
                missing_field("distinct")
            };
            RawSyntaxDecodedNode {
                rule_name: SELECT_ITEMS_RULE.to_string(),
                node_kind: SELECT_ITEMS_RULE.to_string(),
                fields: vec![distinct_field, present_field("items", values)],
            }
        }
    }
}

fn select_list_from_node(
    node: &RawSyntaxDecodedNode,
) -> Result<(SelectList, Option<u32>, Option<OrderByClause>), TreeError> {
    let folded_limit = match node.field("limit") {
        Some(f) if !f.missing && !f.values.is_empty() => {
            Some(limit_from_node(value_as_node(&f.values[0], "limit")?)?)
        }
        _ => None,
    };
    let folded_order = match node.field("order_by") {
        Some(f) if !f.missing && !f.values.is_empty() => Some(order_by_from_node(value_as_node(
            &f.values[0],
            "order_by",
        )?)?),
        _ => None,
    };
    let star = matches!(
        node.node_kind.as_str(),
        STAR_RULE | LIMITED_STAR_RULE | ORDERED_STAR_RULE | ORDERED_LIMITED_STAR_RULE
    );
    if star {
        return Ok((SelectList::Star, folded_limit, folded_order));
    }
    let distinct = matches!(
        node.node_kind.as_str(),
        SELECT_DISTINCT_ITEMS_RULE
            | LIMITED_SELECT_DISTINCT_ITEMS_RULE
            | ORDERED_SELECT_DISTINCT_ITEMS_RULE
            | ORDERED_LIMITED_SELECT_DISTINCT_ITEMS_RULE
    ) || matches!(
        node.field("distinct"),
        Some(f) if !f.missing && !f.values.is_empty()
    );
    let items_kind = matches!(
        node.node_kind.as_str(),
        SELECT_ITEMS_RULE
            | SELECT_DISTINCT_ITEMS_RULE
            | LIMITED_SELECT_ITEMS_RULE
            | LIMITED_SELECT_DISTINCT_ITEMS_RULE
            | ORDERED_SELECT_ITEMS_RULE
            | ORDERED_SELECT_DISTINCT_ITEMS_RULE
            | ORDERED_LIMITED_SELECT_ITEMS_RULE
            | ORDERED_LIMITED_SELECT_DISTINCT_ITEMS_RULE
    );
    if !items_kind {
        return Err(TreeError::new(format!(
            "select_list child must be star/items/distinct or limited_*/ordered_*, got {}",
            node.node_kind
        )));
    }
    let items_field = node
        .field("items")
        .ok_or_else(|| TreeError::new("select_items is missing required field items"))?;
    if items_field.missing || items_field.values.is_empty() {
        return Err(TreeError::new("select_items.items must be non-empty"));
    }
    let mut items = Vec::with_capacity(items_field.values.len());
    for value in &items_field.values {
        let child = value_as_node(value, "select_items.items")?;
        items.push(select_item_from_node(child)?);
    }
    Ok((
        SelectList::Items(SelectItems { distinct, items }),
        folded_limit,
        folded_order,
    ))
}

fn select_item_to_node(item: &SelectItem) -> RawSyntaxDecodedNode {
    let kind_node = match item {
        SelectItem::Column(col) => column_ref_to_node(col),
        SelectItem::CountStar => field_less_leaf(COUNT_STAR_RULE),
        SelectItem::Aggregate {
            func,
            arg,
            distinct,
        } => aggregate_to_node(*func, arg, *distinct),
    };
    RawSyntaxDecodedNode {
        rule_name: SELECT_ITEM_RULE.to_string(),
        node_kind: SELECT_ITEM_RULE.to_string(),
        fields: vec![present_field(
            "kind",
            vec![RawSyntaxDecodedValue::Node(Box::new(kind_node))],
        )],
    }
}

fn select_item_from_node(node: &RawSyntaxDecodedNode) -> Result<SelectItem, TreeError> {
    require_rule(node, SELECT_ITEM_RULE)?;
    let kind = required_child(node, "kind")?;
    match kind.node_kind.as_str() {
        COLUMN_REF_RULE => Ok(SelectItem::Column(column_ref_from_node(kind)?)),
        COUNT_STAR_RULE => Ok(SelectItem::CountStar),
        AGGREGATE_CALL_RULE => aggregate_from_node(kind),
        other => Err(TreeError::new(format!(
            "select_item unsupported kind {other}"
        ))),
    }
}

fn aggregate_to_node(func: AggFunc, arg: &AggregateArg, distinct: bool) -> RawSyntaxDecodedNode {
    let func_rule = match func {
        AggFunc::Count => AGG_COUNT_RULE,
        AggFunc::Sum => AGG_SUM_RULE,
        AggFunc::Avg => AGG_AVG_RULE,
        AggFunc::Min => AGG_MIN_RULE,
        AggFunc::Max => AGG_MAX_RULE,
    };
    let AggregateArg::Column(col) = arg;
    let arg_node = column_ref_to_node(col);
    let distinct_field = if distinct {
        present_field(
            "distinct",
            vec![RawSyntaxDecodedValue::Node(Box::new(field_less_leaf(
                DISTINCT_MARKER_RULE,
            )))],
        )
    } else {
        missing_field("distinct")
    };
    RawSyntaxDecodedNode {
        rule_name: AGGREGATE_CALL_RULE.to_string(),
        node_kind: AGGREGATE_CALL_RULE.to_string(),
        fields: vec![
            present_field(
                "func",
                vec![RawSyntaxDecodedValue::Node(Box::new(field_less_leaf(
                    func_rule,
                )))],
            ),
            present_field("arg", vec![RawSyntaxDecodedValue::Node(Box::new(arg_node))]),
            distinct_field,
        ],
    }
}

fn aggregate_from_node(node: &RawSyntaxDecodedNode) -> Result<SelectItem, TreeError> {
    require_rule(node, AGGREGATE_CALL_RULE)?;
    let func_node = required_child(node, "func")?;
    let func = match func_node.node_kind.as_str() {
        AGG_COUNT_RULE => AggFunc::Count,
        AGG_SUM_RULE => AggFunc::Sum,
        AGG_AVG_RULE => AggFunc::Avg,
        AGG_MIN_RULE => AggFunc::Min,
        AGG_MAX_RULE => AggFunc::Max,
        other => {
            return Err(TreeError::new(format!(
                "unsupported aggregate func {other}"
            )));
        }
    };
    let arg_node = required_child(node, "arg")?;
    let distinct = matches!(
        node.field("distinct"),
        Some(f) if !f.missing && !f.values.is_empty()
    );
    // Fail closed: star under aggregate_call is outside the declared subset.
    // Structural COUNT(*) must use the count_star select_item path (**P7**).
    match arg_node.node_kind.as_str() {
        COLUMN_REF_RULE => Ok(SelectItem::Aggregate {
            func,
            arg: AggregateArg::Column(column_ref_from_node(arg_node)?),
            distinct,
        }),
        STAR_RULE => Err(TreeError::new(format!(
            "aggregate_call rejects star arg for {func:?}; only COUNT(*) via count_star is structural"
        ))),
        other => Err(TreeError::new(format!(
            "unsupported aggregate arg kind {other}"
        ))),
    }
}

fn column_ref_to_node(col: &ColumnRef) -> RawSyntaxDecodedNode {
    RawSyntaxDecodedNode {
        rule_name: COLUMN_REF_RULE.to_string(),
        node_kind: COLUMN_REF_RULE.to_string(),
        fields: vec![present_field(
            COLUMN_MENU_OWNER_FIELD,
            vec![RawSyntaxDecodedValue::Terminal(RawSyntaxDecodedTerminal {
                node_kind: "column_menu_index".to_string(),
                terminal: "integer".to_string(),
                payload: col.menu_index.to_string(),
            })],
        )],
    }
}

fn column_ref_from_node(node: &RawSyntaxDecodedNode) -> Result<ColumnRef, TreeError> {
    require_rule(node, COLUMN_REF_RULE)?;
    let menu_index = menu_payload(node, COLUMN_MENU_OWNER_FIELD, COLUMN_MENU_OWNER_RULE)?;
    Ok(ColumnRef { menu_index })
}

fn menu_payload(
    node: &RawSyntaxDecodedNode,
    field: &str,
    _owner_rule: &str,
) -> Result<u32, TreeError> {
    let f = node
        .field(field)
        .ok_or_else(|| TreeError::new(format!("missing menu field {field}")))?;
    if f.missing || f.values.is_empty() {
        return Err(TreeError::new(format!("menu field {field} has no value")));
    }
    match &f.values[0] {
        RawSyntaxDecodedValue::Terminal(t) => t.payload.parse::<u32>().map_err(|_| {
            TreeError::new(format!(
                "menu payload {:?} is not a decimal ordinal",
                t.payload
            ))
        }),
        RawSyntaxDecodedValue::Node(_) => Err(TreeError::new(format!(
            "menu field {field} expected Terminal, got Node"
        ))),
    }
}

fn required_child<'a>(
    node: &'a RawSyntaxDecodedNode,
    field: &str,
) -> Result<&'a RawSyntaxDecodedNode, TreeError> {
    let f = node
        .field(field)
        .ok_or_else(|| TreeError::new(format!("missing field {field}")))?;
    if f.missing || f.values.is_empty() {
        return Err(TreeError::new(format!(
            "required field {field} is missing or empty"
        )));
    }
    value_as_node(&f.values[0], field)
}

fn value_as_node<'a>(
    value: &'a RawSyntaxDecodedValue,
    field: &str,
) -> Result<&'a RawSyntaxDecodedNode, TreeError> {
    match value {
        RawSyntaxDecodedValue::Node(n) => Ok(n),
        RawSyntaxDecodedValue::Terminal(_) => Err(TreeError::new(format!(
            "field {field} expected Node child, got Terminal"
        ))),
    }
}

fn require_rule(node: &RawSyntaxDecodedNode, rule: &str) -> Result<(), TreeError> {
    if node.rule_name != rule || node.node_kind != rule {
        return Err(TreeError::new(format!(
            "expected rule/kind {rule}, got {}/{}",
            node.rule_name, node.node_kind
        )));
    }
    Ok(())
}

fn field_less_leaf(rule: &str) -> RawSyntaxDecodedNode {
    RawSyntaxDecodedNode {
        rule_name: rule.to_string(),
        node_kind: rule.to_string(),
        fields: vec![],
    }
}

fn present_field(name: &str, values: Vec<RawSyntaxDecodedValue>) -> RawSyntaxDecodedField {
    RawSyntaxDecodedField {
        name: name.to_string(),
        values,
        missing: false,
        repeated_empty: false,
    }
}

fn missing_field(name: &str) -> RawSyntaxDecodedField {
    RawSyntaxDecodedField {
        name: name.to_string(),
        values: vec![],
        missing: true,
        repeated_empty: false,
    }
}

/// Tree lower/raise error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreeError {
    pub message: String,
}

impl TreeError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for TreeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SQL tree error: {}", self.message)
    }
}

impl std::error::Error for TreeError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sql_schema_linking::grammar::{AGG_SUM_RULE, AGGREGATE_CALL_RULE, STAR_RULE};
    use crate::syntax_state_machine::{
        RawSyntaxDecodedField, RawSyntaxDecodedNode, RawSyntaxDecodedValue,
    };

    #[test]
    fn aggregate_call_with_star_arg_fails_closed() {
        // GrammarIR no longer admits star under aggregate_call; a hand-built
        // decoded tree with SUM(*) must still fail closed at the tree boundary
        // rather than producing AggregateArg::Star outside the subset.
        let node = RawSyntaxDecodedNode {
            rule_name: AGGREGATE_CALL_RULE.to_string(),
            node_kind: AGGREGATE_CALL_RULE.to_string(),
            fields: vec![
                RawSyntaxDecodedField {
                    name: "func".to_string(),
                    values: vec![RawSyntaxDecodedValue::Node(Box::new(field_less_leaf(
                        AGG_SUM_RULE,
                    )))],
                    missing: false,
                    repeated_empty: false,
                },
                RawSyntaxDecodedField {
                    name: "arg".to_string(),
                    values: vec![RawSyntaxDecodedValue::Node(Box::new(field_less_leaf(
                        STAR_RULE,
                    )))],
                    missing: false,
                    repeated_empty: false,
                },
            ],
        };
        let err = aggregate_from_node(&node).expect_err("SUM(*) must fail closed");
        assert!(
            err.message.contains("star") || err.message.contains("count_star"),
            "unexpected error: {err}"
        );
    }
}
