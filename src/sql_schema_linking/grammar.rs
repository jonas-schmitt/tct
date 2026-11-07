//! SQL schema-linking GrammarIR (T3).
//!
//! Complete v0 `GrammarIr` shape for the SQLite/Spider subset ladder (M0–M6).
//! Field order on `select_statement` is **from-first** per **P6**:
//! `from`, `where`, `group_by`, `having`, `order_by`, `limit`, `select_list`.
//! The renderer emits SQL keyword order; the canonical stream is a tree
//! serialization, so scope-before-use is a structural invariant.
//!
//! Menu sites are ordinary `IntegerPayload` leaves keyed later by
//! `(owner_rule, owner_field)` (**P3**/**P4**). No `GrammarTerminalKind`
//! variant is added. The parser, tree, render, and menu binding land in
//! later tranches; designing the IR shape once here avoids reshaping the
//! token stream after the models handoff.

use crate::grammar_ir::{
    GrammarField, GrammarFieldAlternative, GrammarIr, GrammarIrProvenance, GrammarRule,
    GrammarTerminalKind,
};
use crate::grammar_ir_structural_model::{
    GrammarStructuralModelError, GrammarStructuralModelSidecar,
    lower_grammar_ir_to_structural_model,
};
use crate::sql_schema_linking::layout::sql_syntax_token_layout;

/// Grammar id recorded in the StructuralModel sidecar and handoff.
pub const SQL_GRAMMAR_ID: &str = "sql_schema_linking_sqlite_subset_v0";
/// Provenance source_kind for the hand-written SQL frontend grammar.
pub const SQL_GRAMMAR_SOURCE_KIND: &str = "hand_written_sql_schema_linking_grammar";
/// Source path recorded in GrammarIR provenance.
pub const SQL_GRAMMAR_SOURCE_PATH: &str = "src/sql_schema_linking/grammar.rs";

/// Root rule name (must be first in `rules` so ModelFacts root matches).
pub const SELECT_STATEMENT_RULE: &str = "select_statement";

/// P6 field order on [`SELECT_STATEMENT_RULE`].
pub const SELECT_STATEMENT_FIELD_ORDER: &[&str] = &[
    "from",
    "where",
    "group_by",
    "having",
    "order_by",
    "limit",
    "select_list",
];

/// Table menu site: owner rule for the menu binding (**P3**/**P4**).
pub const TABLE_MENU_OWNER_RULE: &str = "table_ref";
/// Table menu site: owner field (IntegerPayload leaf).
pub const TABLE_MENU_OWNER_FIELD: &str = "menu_index";
/// Column menu site: owner rule for the menu binding (**P3**/**P4**).
pub const COLUMN_MENU_OWNER_RULE: &str = "column_ref";
/// Column menu site: owner field (IntegerPayload leaf).
pub const COLUMN_MENU_OWNER_FIELD: &str = "menu_index";

// --- rule / node kind names (name == node_kind throughout) ---

pub const FROM_CLAUSE_RULE: &str = "from_clause";
pub const TABLE_REF_RULE: &str = "table_ref";
pub const ALIAS_MARKER_RULE: &str = "alias_marker";
pub const JOIN_CLAUSE_RULE: &str = "join_clause";
pub const JOIN_INNER_RULE: &str = "join_inner";
pub const JOIN_LEFT_RULE: &str = "join_left";
/// `join_clause` field names used by the structural join-ON constraint.
pub const JOIN_ON_FIELD: &str = "on";
/// `comparison` field names used by the structural join-ON constraint.
pub const COMPARISON_LEFT_FIELD: &str = "left";
pub const COMPARISON_OP_FIELD: &str = "op";
pub const COMPARISON_RIGHT_FIELD: &str = "right";
pub const STAR_RULE: &str = "star";
pub const SELECT_ITEMS_RULE: &str = "select_items";
/// Rank-1 fold: `SELECT DISTINCT …` as its own select_list alternative so
/// `select_items.distinct` is not a second free decision.
pub const SELECT_DISTINCT_ITEMS_RULE: &str = "select_distinct_items";
pub const DISTINCT_MARKER_RULE: &str = "distinct_marker";
pub const SELECT_ITEM_RULE: &str = "select_item";
pub const COLUMN_REF_RULE: &str = "column_ref";
pub const COUNT_STAR_RULE: &str = "count_star";
pub const AGGREGATE_CALL_RULE: &str = "aggregate_call";
pub const AGG_COUNT_RULE: &str = "agg_count";
pub const AGG_SUM_RULE: &str = "agg_sum";
pub const AGG_AVG_RULE: &str = "agg_avg";
pub const AGG_MIN_RULE: &str = "agg_min";
pub const AGG_MAX_RULE: &str = "agg_max";
pub const GROUP_BY_CLAUSE_RULE: &str = "group_by_clause";
pub const ORDER_BY_CLAUSE_RULE: &str = "order_by_clause";
pub const ORDER_ITEM_RULE: &str = "order_item";
pub const ORDER_ASC_RULE: &str = "order_asc";
pub const ORDER_DESC_RULE: &str = "order_desc";
pub const LIMIT_CLAUSE_RULE: &str = "limit_clause";
/// Rank-1 fold: LIMIT presence × select_list kind (6-way). LIMIT value
/// stays a child of the limited_* alternative.
pub const LIMITED_STAR_RULE: &str = "limited_star";
pub const LIMITED_SELECT_ITEMS_RULE: &str = "limited_select_items";
pub const LIMITED_SELECT_DISTINCT_ITEMS_RULE: &str = "limited_select_distinct_items";
/// Rank-1 fold: ORDER BY presence × (LIMIT × select_list kind) (12-way).
/// ORDER BY items stay children of the ordered_* alternative.
pub const ORDERED_STAR_RULE: &str = "ordered_star";
pub const ORDERED_SELECT_ITEMS_RULE: &str = "ordered_select_items";
pub const ORDERED_SELECT_DISTINCT_ITEMS_RULE: &str = "ordered_select_distinct_items";
pub const ORDERED_LIMITED_STAR_RULE: &str = "ordered_limited_star";
pub const ORDERED_LIMITED_SELECT_ITEMS_RULE: &str = "ordered_limited_select_items";
pub const ORDERED_LIMITED_SELECT_DISTINCT_ITEMS_RULE: &str =
    "ordered_limited_select_distinct_items";
pub const EXPRESSION_RULE: &str = "expression";
pub const COMPARISON_RULE: &str = "comparison";
pub const BINARY_BOOL_RULE: &str = "binary_bool";
pub const INTEGER_LITERAL_RULE: &str = "integer_literal";
pub const STRING_LITERAL_RULE: &str = "string_literal";
pub const SUBQUERY_RULE: &str = "subquery";
/// `expr [NOT] IN (SELECT …)` predicate (appended expression alt; gold streams stable).
pub const IN_PREDICATE_RULE: &str = "in_predicate";
pub const IN_POSITIVE_RULE: &str = "in_positive";
pub const IN_NEGATED_RULE: &str = "in_negated";
pub const BETWEEN_PREDICATE_RULE: &str = "between_predicate";
pub const IN_VALUE_LIST_RULE: &str = "in_value_list";
pub const OP_EQ_RULE: &str = "op_eq";
pub const OP_NE_RULE: &str = "op_ne";
pub const OP_LT_RULE: &str = "op_lt";
pub const OP_LE_RULE: &str = "op_le";
pub const OP_GT_RULE: &str = "op_gt";
pub const OP_GE_RULE: &str = "op_ge";
pub const OP_LIKE_RULE: &str = "op_like";
pub const OP_AND_RULE: &str = "op_and";
pub const OP_OR_RULE: &str = "op_or";

/// Build the complete v0 SQL schema-linking `GrammarIr`.
///
/// The shape covers the full M0–M6 ladder even though the parser lands
/// incrementally: adding fields later would reshape the token stream.
pub fn sql_schema_linking_grammar_ir() -> GrammarIr {
    sql_schema_linking_grammar_ir_with_fold_select_distinct(false)
}

/// Grammar with optional rank-1 visit-all folds: `select_list` ×
/// `select_items.distinct` into a 3-way alternative, HAVING moved
/// under `group_by_clause` (SQL: HAVING requires GROUP BY), and LIMIT
/// plus ORDER BY presence folded into a 12-way `select_list` (sequential
/// P6 fields).
pub fn sql_schema_linking_grammar_ir_with_fold_select_distinct(fold: bool) -> GrammarIr {
    let mut grammar = GrammarIr::new(
        SQL_GRAMMAR_ID,
        GrammarIrProvenance {
            source_kind: SQL_GRAMMAR_SOURCE_KIND.to_string(),
            source_path: Some(SQL_GRAMMAR_SOURCE_PATH.to_string()),
            source_sha256: None,
            metadata_path: None,
            metadata_sha256: None,
            parser: None,
        },
    );

    // Root first: ModelFacts::root_rule is the first structural record.
    // HAVING requires GROUP BY (SQL). Visit-all moves it under
    // group_by_clause so queries without GROUP BY do not pay a having skip.
    let mut select_fields = vec![
        nt_field("from", true, false, &[FROM_CLAUSE_RULE]),
        nt_field("where", false, false, &[EXPRESSION_RULE]),
        nt_field("group_by", false, false, &[GROUP_BY_CLAUSE_RULE]),
    ];
    if !fold {
        select_fields.push(nt_field("having", false, false, &[EXPRESSION_RULE]));
        select_fields.push(nt_field("order_by", false, false, &[ORDER_BY_CLAUSE_RULE]));
        select_fields.push(nt_field("limit", false, false, &[LIMIT_CLAUSE_RULE]));
    }
    select_fields.push(if fold {
        nt_field(
            "select_list",
            true,
            false,
            &[
                STAR_RULE,
                SELECT_ITEMS_RULE,
                SELECT_DISTINCT_ITEMS_RULE,
                LIMITED_STAR_RULE,
                LIMITED_SELECT_ITEMS_RULE,
                LIMITED_SELECT_DISTINCT_ITEMS_RULE,
                ORDERED_STAR_RULE,
                ORDERED_SELECT_ITEMS_RULE,
                ORDERED_SELECT_DISTINCT_ITEMS_RULE,
                ORDERED_LIMITED_STAR_RULE,
                ORDERED_LIMITED_SELECT_ITEMS_RULE,
                ORDERED_LIMITED_SELECT_DISTINCT_ITEMS_RULE,
            ],
        )
    } else {
        nt_field("select_list", true, false, &[STAR_RULE, SELECT_ITEMS_RULE])
    });
    grammar.rules.push(GrammarRule {
        name: SELECT_STATEMENT_RULE.to_string(),
        node_kind: SELECT_STATEMENT_RULE.to_string(),
        fields: select_fields,
    });

    grammar.rules.push(GrammarRule {
        name: FROM_CLAUSE_RULE.to_string(),
        node_kind: FROM_CLAUSE_RULE.to_string(),
        fields: vec![
            nt_field("head", true, false, &[TABLE_REF_RULE]),
            nt_field("joins", false, true, &[JOIN_CLAUSE_RULE]),
        ],
    });

    grammar.rules.push(GrammarRule {
        name: TABLE_REF_RULE.to_string(),
        node_kind: TABLE_REF_RULE.to_string(),
        fields: vec![
            // Table menu site: ordinary IntegerPayload; binding later (**P3**).
            int_field(TABLE_MENU_OWNER_FIELD, true, "table_menu_index"),
            // Alias *definition* is a structural marker; spelling not preserved (**P7**).
            nt_field("alias", false, false, &[ALIAS_MARKER_RULE]),
        ],
    });

    grammar.rules.push(fieldless(ALIAS_MARKER_RULE));

    grammar.rules.push(GrammarRule {
        name: JOIN_CLAUSE_RULE.to_string(),
        node_kind: JOIN_CLAUSE_RULE.to_string(),
        fields: vec![
            nt_field("kind", true, false, &[JOIN_INNER_RULE, JOIN_LEFT_RULE]),
            nt_field("table", true, false, &[TABLE_REF_RULE]),
            nt_field("on", true, false, &[EXPRESSION_RULE]),
        ],
    });
    grammar.rules.push(fieldless(JOIN_INNER_RULE));
    grammar.rules.push(fieldless(JOIN_LEFT_RULE));

    // Star is field-less: one structural choice token, zero menu tokens (**P7**).
    grammar.rules.push(fieldless(STAR_RULE));

    grammar.rules.push(GrammarRule {
        name: SELECT_ITEMS_RULE.to_string(),
        node_kind: SELECT_ITEMS_RULE.to_string(),
        fields: if fold {
            vec![nt_field("items", true, true, &[SELECT_ITEM_RULE])]
        } else {
            vec![
                nt_field("distinct", false, false, &[DISTINCT_MARKER_RULE]),
                nt_field("items", true, true, &[SELECT_ITEM_RULE]),
            ]
        },
    });
    if fold {
        grammar.rules.push(GrammarRule {
            name: SELECT_DISTINCT_ITEMS_RULE.to_string(),
            node_kind: SELECT_DISTINCT_ITEMS_RULE.to_string(),
            fields: vec![nt_field("items", true, true, &[SELECT_ITEM_RULE])],
        });
    }
    grammar.rules.push(fieldless(DISTINCT_MARKER_RULE));

    grammar.rules.push(GrammarRule {
        name: SELECT_ITEM_RULE.to_string(),
        node_kind: SELECT_ITEM_RULE.to_string(),
        fields: vec![nt_field(
            "kind",
            true,
            false,
            &[COLUMN_REF_RULE, COUNT_STAR_RULE, AGGREGATE_CALL_RULE],
        )],
    });

    grammar.rules.push(GrammarRule {
        name: COLUMN_REF_RULE.to_string(),
        node_kind: COLUMN_REF_RULE.to_string(),
        fields: vec![
            // Column menu site: scope-relative ordinal later (**P4**/**P7**).
            // Qualifier uses need no token; render derives them from scope.
            int_field(COLUMN_MENU_OWNER_FIELD, true, "column_menu_index"),
        ],
    });

    // COUNT(*) is structural; no menu token (**P7**).
    grammar.rules.push(fieldless(COUNT_STAR_RULE));

    grammar.rules.push(GrammarRule {
        name: AGGREGATE_CALL_RULE.to_string(),
        node_kind: AGGREGATE_CALL_RULE.to_string(),
        fields: vec![
            nt_field(
                "func",
                true,
                false,
                &[
                    AGG_COUNT_RULE,
                    AGG_SUM_RULE,
                    AGG_AVG_RULE,
                    AGG_MIN_RULE,
                    AGG_MAX_RULE,
                ],
            ),
            // Arg is column_ref only. Structural COUNT(*) uses the dedicated
            // count_star select_item path (**P7**); SUM(*)/AVG(*) are not in
            // the declared subset and must not be decodable via aggregate_call.
            nt_field("arg", true, false, &[COLUMN_REF_RULE]),
            // Optional DISTINCT marker **appended** so existing non-DISTINCT
            // aggregate streams keep the same field order until the marker.
            nt_field("distinct", false, false, &[DISTINCT_MARKER_RULE]),
        ],
    });
    grammar.rules.push(fieldless(AGG_COUNT_RULE));
    grammar.rules.push(fieldless(AGG_SUM_RULE));
    grammar.rules.push(fieldless(AGG_AVG_RULE));
    grammar.rules.push(fieldless(AGG_MIN_RULE));
    grammar.rules.push(fieldless(AGG_MAX_RULE));

    grammar.rules.push(GrammarRule {
        name: GROUP_BY_CLAUSE_RULE.to_string(),
        node_kind: GROUP_BY_CLAUSE_RULE.to_string(),
        fields: if fold {
            vec![
                nt_field("columns", true, true, &[COLUMN_REF_RULE]),
                nt_field("having", false, false, &[EXPRESSION_RULE]),
            ]
        } else {
            vec![nt_field("columns", true, true, &[COLUMN_REF_RULE])]
        },
    });

    grammar.rules.push(GrammarRule {
        name: ORDER_BY_CLAUSE_RULE.to_string(),
        node_kind: ORDER_BY_CLAUSE_RULE.to_string(),
        fields: vec![nt_field("items", true, true, &[ORDER_ITEM_RULE])],
    });

    grammar.rules.push(GrammarRule {
        name: ORDER_ITEM_RULE.to_string(),
        node_kind: ORDER_ITEM_RULE.to_string(),
        fields: vec![
            // Field name stays `column` for stream compatibility; multi-alt
            // admits structural COUNT(*) and column aggregates alongside refs.
            nt_field(
                "column",
                true,
                false,
                &[COLUMN_REF_RULE, COUNT_STAR_RULE, AGGREGATE_CALL_RULE],
            ),
            nt_field("direction", true, false, &[ORDER_ASC_RULE, ORDER_DESC_RULE]),
        ],
    });
    grammar.rules.push(fieldless(ORDER_ASC_RULE));
    grammar.rules.push(fieldless(ORDER_DESC_RULE));

    grammar.rules.push(GrammarRule {
        name: LIMIT_CLAUSE_RULE.to_string(),
        node_kind: LIMIT_CLAUSE_RULE.to_string(),
        fields: vec![
            // Literal integer (u8 band), not a menu site — no binding later.
            int_field("value", true, "limit_value"),
        ],
    });
    if fold {
        grammar.rules.push(GrammarRule {
            name: LIMITED_STAR_RULE.to_string(),
            node_kind: LIMITED_STAR_RULE.to_string(),
            fields: vec![nt_field("limit", true, false, &[LIMIT_CLAUSE_RULE])],
        });
        grammar.rules.push(GrammarRule {
            name: LIMITED_SELECT_ITEMS_RULE.to_string(),
            node_kind: LIMITED_SELECT_ITEMS_RULE.to_string(),
            fields: vec![
                nt_field("limit", true, false, &[LIMIT_CLAUSE_RULE]),
                nt_field("items", true, true, &[SELECT_ITEM_RULE]),
            ],
        });
        grammar.rules.push(GrammarRule {
            name: LIMITED_SELECT_DISTINCT_ITEMS_RULE.to_string(),
            node_kind: LIMITED_SELECT_DISTINCT_ITEMS_RULE.to_string(),
            fields: vec![
                nt_field("limit", true, false, &[LIMIT_CLAUSE_RULE]),
                nt_field("items", true, true, &[SELECT_ITEM_RULE]),
            ],
        });
        grammar.rules.push(GrammarRule {
            name: ORDERED_STAR_RULE.to_string(),
            node_kind: ORDERED_STAR_RULE.to_string(),
            fields: vec![nt_field("order_by", true, false, &[ORDER_BY_CLAUSE_RULE])],
        });
        grammar.rules.push(GrammarRule {
            name: ORDERED_SELECT_ITEMS_RULE.to_string(),
            node_kind: ORDERED_SELECT_ITEMS_RULE.to_string(),
            fields: vec![
                nt_field("order_by", true, false, &[ORDER_BY_CLAUSE_RULE]),
                nt_field("items", true, true, &[SELECT_ITEM_RULE]),
            ],
        });
        grammar.rules.push(GrammarRule {
            name: ORDERED_SELECT_DISTINCT_ITEMS_RULE.to_string(),
            node_kind: ORDERED_SELECT_DISTINCT_ITEMS_RULE.to_string(),
            fields: vec![
                nt_field("order_by", true, false, &[ORDER_BY_CLAUSE_RULE]),
                nt_field("items", true, true, &[SELECT_ITEM_RULE]),
            ],
        });
        grammar.rules.push(GrammarRule {
            name: ORDERED_LIMITED_STAR_RULE.to_string(),
            node_kind: ORDERED_LIMITED_STAR_RULE.to_string(),
            fields: vec![
                nt_field("order_by", true, false, &[ORDER_BY_CLAUSE_RULE]),
                nt_field("limit", true, false, &[LIMIT_CLAUSE_RULE]),
            ],
        });
        grammar.rules.push(GrammarRule {
            name: ORDERED_LIMITED_SELECT_ITEMS_RULE.to_string(),
            node_kind: ORDERED_LIMITED_SELECT_ITEMS_RULE.to_string(),
            fields: vec![
                nt_field("order_by", true, false, &[ORDER_BY_CLAUSE_RULE]),
                nt_field("limit", true, false, &[LIMIT_CLAUSE_RULE]),
                nt_field("items", true, true, &[SELECT_ITEM_RULE]),
            ],
        });
        grammar.rules.push(GrammarRule {
            name: ORDERED_LIMITED_SELECT_DISTINCT_ITEMS_RULE.to_string(),
            node_kind: ORDERED_LIMITED_SELECT_DISTINCT_ITEMS_RULE.to_string(),
            fields: vec![
                nt_field("order_by", true, false, &[ORDER_BY_CLAUSE_RULE]),
                nt_field("limit", true, false, &[LIMIT_CLAUSE_RULE]),
                nt_field("items", true, true, &[SELECT_ITEM_RULE]),
            ],
        });
    }

    // Expression multi-alt covers comparison, boolean connectives, atoms,
    // subquery, and IN/NOT IN. New alts are **appended** so existing gold
    // choice indices stay byte-stable.
    grammar.rules.push(GrammarRule {
        name: EXPRESSION_RULE.to_string(),
        node_kind: EXPRESSION_RULE.to_string(),
        fields: vec![nt_field(
            "kind",
            true,
            false,
            &[
                COMPARISON_RULE,
                BINARY_BOOL_RULE,
                COLUMN_REF_RULE,
                INTEGER_LITERAL_RULE,
                STRING_LITERAL_RULE,
                COUNT_STAR_RULE,
                SUBQUERY_RULE,
                IN_PREDICATE_RULE,
                // HAVING / expression-position aggregates (appended; gold indices stable).
                AGGREGATE_CALL_RULE,
                BETWEEN_PREDICATE_RULE,
            ],
        )],
    });

    grammar.rules.push(GrammarRule {
        name: COMPARISON_RULE.to_string(),
        node_kind: COMPARISON_RULE.to_string(),
        fields: vec![
            nt_field("left", true, false, &[EXPRESSION_RULE]),
            nt_field(
                "op",
                true,
                false,
                &[
                    OP_EQ_RULE,
                    OP_NE_RULE,
                    OP_LT_RULE,
                    OP_LE_RULE,
                    OP_GT_RULE,
                    OP_GE_RULE,
                    OP_LIKE_RULE,
                ],
            ),
            nt_field("right", true, false, &[EXPRESSION_RULE]),
        ],
    });
    grammar.rules.push(fieldless(OP_EQ_RULE));
    grammar.rules.push(fieldless(OP_NE_RULE));
    grammar.rules.push(fieldless(OP_LT_RULE));
    grammar.rules.push(fieldless(OP_LE_RULE));
    grammar.rules.push(fieldless(OP_GT_RULE));
    grammar.rules.push(fieldless(OP_GE_RULE));
    grammar.rules.push(fieldless(OP_LIKE_RULE));

    grammar.rules.push(GrammarRule {
        name: BINARY_BOOL_RULE.to_string(),
        node_kind: BINARY_BOOL_RULE.to_string(),
        fields: vec![
            nt_field("left", true, false, &[EXPRESSION_RULE]),
            nt_field("op", true, false, &[OP_AND_RULE, OP_OR_RULE]),
            nt_field("right", true, false, &[EXPRESSION_RULE]),
        ],
    });
    grammar.rules.push(fieldless(OP_AND_RULE));
    grammar.rules.push(fieldless(OP_OR_RULE));

    grammar.rules.push(GrammarRule {
        name: INTEGER_LITERAL_RULE.to_string(),
        node_kind: INTEGER_LITERAL_RULE.to_string(),
        fields: vec![int_field("value", true, "integer_literal_value")],
    });

    grammar.rules.push(GrammarRule {
        name: STRING_LITERAL_RULE.to_string(),
        node_kind: STRING_LITERAL_RULE.to_string(),
        fields: vec![str_field("value", true, "string_literal_value")],
    });

    // Uncorrelated subquery only in v0 (**P9**); outer scope is not reachable.
    grammar.rules.push(GrammarRule {
        name: SUBQUERY_RULE.to_string(),
        node_kind: SUBQUERY_RULE.to_string(),
        fields: vec![nt_field("query", true, false, &[SELECT_STATEMENT_RULE])],
    });

    // IN / NOT IN (subquery or non-empty value list).
    grammar.rules.push(GrammarRule {
        name: IN_PREDICATE_RULE.to_string(),
        node_kind: IN_PREDICATE_RULE.to_string(),
        fields: vec![
            nt_field("left", true, false, &[EXPRESSION_RULE]),
            nt_field(
                "polarity",
                true,
                false,
                &[IN_POSITIVE_RULE, IN_NEGATED_RULE],
            ),
            // Subquery first so existing IN streams keep choice index 0.
            nt_field("right", true, false, &[SUBQUERY_RULE, IN_VALUE_LIST_RULE]),
        ],
    });
    grammar.rules.push(fieldless(IN_POSITIVE_RULE));
    grammar.rules.push(fieldless(IN_NEGATED_RULE));
    grammar.rules.push(GrammarRule {
        name: IN_VALUE_LIST_RULE.to_string(),
        node_kind: IN_VALUE_LIST_RULE.to_string(),
        fields: vec![nt_field("values", true, true, &[EXPRESSION_RULE])],
    });

    // BETWEEN low AND high (comparison-level residual lift).
    grammar.rules.push(GrammarRule {
        name: BETWEEN_PREDICATE_RULE.to_string(),
        node_kind: BETWEEN_PREDICATE_RULE.to_string(),
        fields: vec![
            nt_field("expr", true, false, &[EXPRESSION_RULE]),
            nt_field("low", true, false, &[EXPRESSION_RULE]),
            nt_field("high", true, false, &[EXPRESSION_RULE]),
        ],
    });

    grammar
}

/// Lower the SQL GrammarIR with the SQL token layout to a StructuralModel sidecar.
pub fn lower_sql_schema_linking_grammar()
-> Result<GrammarStructuralModelSidecar, GrammarStructuralModelError> {
    lower_sql_schema_linking_grammar_with_fold_select_distinct(false)
}

/// Lower with the rank-1 select_list × distinct fold.
pub fn lower_sql_schema_linking_grammar_with_fold_select_distinct(
    fold: bool,
) -> Result<GrammarStructuralModelSidecar, GrammarStructuralModelError> {
    lower_grammar_ir_to_structural_model(
        &sql_schema_linking_grammar_ir_with_fold_select_distinct(fold),
        sql_syntax_token_layout(),
    )
}

/// Field names of `select_statement` in declared (stream) order.
pub fn select_statement_field_names(grammar: &GrammarIr) -> Option<Vec<&str>> {
    grammar
        .rule(SELECT_STATEMENT_RULE)
        .map(|rule| rule.fields.iter().map(|f| f.name.as_str()).collect())
}

/// Index of a named field on `select_statement`, if present.
pub fn select_statement_field_index(grammar: &GrammarIr, field: &str) -> Option<usize> {
    grammar
        .rule(SELECT_STATEMENT_RULE)
        .and_then(|rule| rule.fields.iter().position(|f| f.name == field))
}

fn fieldless(name: &str) -> GrammarRule {
    GrammarRule {
        name: name.to_string(),
        node_kind: name.to_string(),
        fields: vec![],
    }
}

fn nt_field(name: &str, required: bool, multiple: bool, kinds: &[&str]) -> GrammarField {
    GrammarField {
        name: name.to_string(),
        required,
        multiple,
        alternatives: kinds
            .iter()
            .map(|kind| GrammarFieldAlternative {
                node_kind: (*kind).to_string(),
                terminal: GrammarTerminalKind::Nonterminal,
            })
            .collect(),
    }
}

fn int_field(name: &str, required: bool, node_kind: &str) -> GrammarField {
    GrammarField {
        name: name.to_string(),
        required,
        multiple: false,
        alternatives: vec![GrammarFieldAlternative {
            node_kind: node_kind.to_string(),
            terminal: GrammarTerminalKind::IntegerPayload,
        }],
    }
}

fn str_field(name: &str, required: bool, node_kind: &str) -> GrammarField {
    GrammarField {
        name: name.to_string(),
        required,
        multiple: false,
        alternatives: vec![GrammarFieldAlternative {
            node_kind: node_kind.to_string(),
            terminal: GrammarTerminalKind::StringPayload,
        }],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grammar_ir::GrammarTerminalKind;
    use crate::syntax_state_machine::CompiledSyntaxModel;

    #[test]
    fn sql_grammar_lowers_and_select_statement_orders_from_before_select_list() {
        // T3 gate: lowering succeeds and `from` is strictly before `select_list` (**P6**).
        let grammar = sql_schema_linking_grammar_ir();
        let sidecar = lower_sql_schema_linking_grammar().expect("SQL GrammarIR must lower");

        assert_eq!(sidecar.input.grammar_id, SQL_GRAMMAR_ID);
        assert_eq!(
            sidecar.structural_model.frontend_id, SQL_GRAMMAR_ID,
            "structural model frontend id must match grammar id"
        );
        assert!(
            sidecar.proof.tokenizable_decisions_match,
            "proof must report tokenizable decisions match"
        );

        let from_idx = select_statement_field_index(&grammar, "from")
            .expect("select_statement must have from");
        let select_list_idx = select_statement_field_index(&grammar, "select_list")
            .expect("select_statement must have select_list");
        assert!(
            from_idx < select_list_idx,
            "P6: from (index {from_idx}) must be strictly before select_list (index {select_list_idx})"
        );
    }

    #[test]
    fn sql_grammar_select_statement_field_order_is_exactly_p6() {
        let grammar = sql_schema_linking_grammar_ir();
        let names =
            select_statement_field_names(&grammar).expect("select_statement rule must exist");
        assert_eq!(
            names, SELECT_STATEMENT_FIELD_ORDER,
            "select_statement field order must match P6 exactly"
        );
        // First rule is the root for ModelFacts.
        assert_eq!(grammar.rules[0].name, SELECT_STATEMENT_RULE);
    }

    #[test]
    fn sql_grammar_menu_sites_are_integer_payload_not_new_terminal_kind() {
        let grammar = sql_schema_linking_grammar_ir();

        let table = grammar.rule(TABLE_MENU_OWNER_RULE).expect("table_ref rule");
        let table_menu = table
            .fields
            .iter()
            .find(|f| f.name == TABLE_MENU_OWNER_FIELD)
            .expect("table_ref.menu_index");
        assert_eq!(table_menu.alternatives.len(), 1);
        assert_eq!(
            table_menu.alternatives[0].terminal,
            GrammarTerminalKind::IntegerPayload,
            "P3: menu sites use IntegerPayload, not a new GrammarTerminalKind"
        );

        let column = grammar
            .rule(COLUMN_MENU_OWNER_RULE)
            .expect("column_ref rule");
        let column_menu = column
            .fields
            .iter()
            .find(|f| f.name == COLUMN_MENU_OWNER_FIELD)
            .expect("column_ref.menu_index");
        assert_eq!(column_menu.alternatives.len(), 1);
        assert_eq!(
            column_menu.alternatives[0].terminal,
            GrammarTerminalKind::IntegerPayload
        );
    }

    #[test]
    fn sql_grammar_star_is_fieldless_under_select_list_multi_alt() {
        let grammar = sql_schema_linking_grammar_ir();
        let star = grammar.rule(STAR_RULE).expect("star rule");
        assert!(
            star.fields.is_empty(),
            "P7: star must be field-less (zero menu tokens)"
        );

        let select = grammar
            .rule(SELECT_STATEMENT_RULE)
            .expect("select_statement");
        let select_list = select
            .fields
            .iter()
            .find(|f| f.name == "select_list")
            .expect("select_list field");
        assert!(
            select_list.alternatives.len() > 1,
            "select_list must be multi-alternative"
        );
        assert!(
            select_list
                .alternatives
                .iter()
                .any(|a| a.node_kind == STAR_RULE
                    && a.terminal == GrammarTerminalKind::Nonterminal),
            "select_list must include Nonterminal star"
        );
        assert!(
            select_list
                .alternatives
                .iter()
                .any(|a| a.node_kind == SELECT_ITEMS_RULE
                    && a.terminal == GrammarTerminalKind::Nonterminal),
            "select_list must include Nonterminal select_items"
        );

        let count_star = grammar.rule(COUNT_STAR_RULE).expect("count_star");
        assert!(
            count_star.fields.is_empty(),
            "COUNT(*) is structural with no menu token"
        );
    }

    #[test]
    fn sql_grammar_covers_v0_ladder_rules_and_compiles() {
        // Shape is designed once for M0–M6; compile early so structural
        // alternative capacity is checked before T4 menu binding.
        let grammar = sql_schema_linking_grammar_ir();
        for required in [
            SELECT_STATEMENT_RULE,
            FROM_CLAUSE_RULE,
            TABLE_REF_RULE,
            JOIN_CLAUSE_RULE,
            STAR_RULE,
            SELECT_ITEMS_RULE,
            COLUMN_REF_RULE,
            COUNT_STAR_RULE,
            AGGREGATE_CALL_RULE,
            GROUP_BY_CLAUSE_RULE,
            ORDER_BY_CLAUSE_RULE,
            LIMIT_CLAUSE_RULE,
            EXPRESSION_RULE,
            COMPARISON_RULE,
            BINARY_BOOL_RULE,
            SUBQUERY_RULE,
            IN_PREDICATE_RULE,
            IN_POSITIVE_RULE,
            IN_NEGATED_RULE,
            BETWEEN_PREDICATE_RULE,
            DISTINCT_MARKER_RULE,
            ALIAS_MARKER_RULE,
        ] {
            assert!(
                grammar.rule(required).is_some(),
                "v0 grammar must include rule {required}"
            );
        }

        let sidecar = lower_sql_schema_linking_grammar().expect("lower");
        let _model = CompiledSyntaxModel::from_sidecar(&sidecar)
            .expect("complete v0 SQL grammar must compile under SQL layout");
        assert_eq!(
            sidecar
                .structural_model
                .records
                .first()
                .map(|r| r.grammar_rule.as_str()),
            Some(SELECT_STATEMENT_RULE),
            "first structural record must be the select_statement root"
        );
    }

    #[test]
    fn sql_grammar_select_list_required_fields_are_required() {
        let grammar = sql_schema_linking_grammar_ir();
        let select = grammar.rule(SELECT_STATEMENT_RULE).unwrap();
        let from = select.fields.iter().find(|f| f.name == "from").unwrap();
        let select_list = select
            .fields
            .iter()
            .find(|f| f.name == "select_list")
            .unwrap();
        assert!(from.required && !from.multiple);
        assert!(select_list.required && !select_list.multiple);
        for opt in ["where", "group_by", "having", "order_by", "limit"] {
            let field = select.fields.iter().find(|f| f.name == opt).unwrap();
            assert!(!field.required, "{opt} must be optional");
        }
    }

    #[test]
    fn visit_all_grammar_moves_having_under_group_by() {
        let grammar = sql_schema_linking_grammar_ir_with_fold_select_distinct(true);
        let select = grammar.rule(SELECT_STATEMENT_RULE).unwrap();
        assert!(
            select.fields.iter().all(|f| f.name != "having"),
            "visit-all select_statement must not have having: {:?}",
            select
                .fields
                .iter()
                .map(|f| f.name.as_str())
                .collect::<Vec<_>>()
        );
        let group_by = grammar.rule(GROUP_BY_CLAUSE_RULE).unwrap();
        let having = group_by
            .fields
            .iter()
            .find(|f| f.name == "having")
            .expect("group_by_clause.having");
        assert!(!having.required);
        assert!(!having.multiple);
    }

    #[test]
    fn visit_all_grammar_folds_limit_into_select_list() {
        let grammar = sql_schema_linking_grammar_ir_with_fold_select_distinct(true);
        let select = grammar.rule(SELECT_STATEMENT_RULE).unwrap();
        assert!(
            select.fields.iter().all(|f| f.name != "limit"),
            "visit-all select_statement must not have limit: {:?}",
            select
                .fields
                .iter()
                .map(|f| f.name.as_str())
                .collect::<Vec<_>>()
        );
        let select_list = select
            .fields
            .iter()
            .find(|f| f.name == "select_list")
            .expect("select_list");
        let kinds: Vec<&str> = select_list
            .alternatives
            .iter()
            .map(|a| a.node_kind.as_str())
            .collect();
        for need in [
            STAR_RULE,
            SELECT_ITEMS_RULE,
            SELECT_DISTINCT_ITEMS_RULE,
            LIMITED_STAR_RULE,
            LIMITED_SELECT_ITEMS_RULE,
            LIMITED_SELECT_DISTINCT_ITEMS_RULE,
            ORDERED_STAR_RULE,
            ORDERED_SELECT_ITEMS_RULE,
            ORDERED_SELECT_DISTINCT_ITEMS_RULE,
            ORDERED_LIMITED_STAR_RULE,
            ORDERED_LIMITED_SELECT_ITEMS_RULE,
            ORDERED_LIMITED_SELECT_DISTINCT_ITEMS_RULE,
        ] {
            assert!(
                kinds.contains(&need),
                "select_list missing {need}: {kinds:?}"
            );
        }
        assert!(
            select.fields.iter().all(|f| f.name != "order_by"),
            "visit-all select_statement must not have order_by"
        );
    }
}
