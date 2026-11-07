//! Expression / literal parsers for the SQL schema-linking frontend.
//!
//! Extracted from the main SELECT/FROM/JOIN walker so clause and expression
//! ownership stay separate (module-size policy). Behavior-preserving.

use super::{Parser, SqlParseError};
use crate::sql_schema_linking::ast::{
    AggFunc, AggregateArg, BoolOp, CmpOp, ColumnRef, Expression, InRhs,
};

impl<'a> Parser<'a> {
    /// Expression: OR < AND < comparison < atom (left-associative).
    pub(super) fn parse_expression(
        &mut self,
        scope: &[usize],
    ) -> Result<Expression, SqlParseError> {
        self.parse_or(scope)
    }

    fn parse_or(&mut self, scope: &[usize]) -> Result<Expression, SqlParseError> {
        let mut left = self.parse_and(scope)?;
        loop {
            self.skip_whitespace();
            if !self.peek_keyword("OR") {
                break;
            }
            self.expect_keyword("OR")?;
            self.require_whitespace("binary_bool")?;
            let right = self.parse_and(scope)?;
            left = Expression::BinaryBool {
                left: Box::new(left),
                op: BoolOp::Or,
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_and(&mut self, scope: &[usize]) -> Result<Expression, SqlParseError> {
        let mut left = self.parse_comparison(scope)?;
        loop {
            self.skip_whitespace();
            if !self.peek_keyword("AND") {
                break;
            }
            self.expect_keyword("AND")?;
            self.require_whitespace("binary_bool")?;
            let right = self.parse_comparison(scope)?;
            left = Expression::BinaryBool {
                left: Box::new(left),
                op: BoolOp::And,
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_comparison(&mut self, scope: &[usize]) -> Result<Expression, SqlParseError> {
        let left = self.parse_atom(scope)?;
        self.skip_whitespace();
        // IN / NOT IN (subquery) at comparison level — residual lift; value lists deferred.
        if self.peek_keyword("NOT") || self.peek_keyword("IN") {
            return self.parse_in_predicate_after_left(left, scope);
        }
        // BETWEEN low AND high (comparison level).
        if self.peek_keyword("BETWEEN") {
            return self.parse_between_after_left(left, scope);
        }
        let Some(op) = self.try_parse_cmp_op()? else {
            return Ok(left);
        };
        self.skip_whitespace();
        let right = self.parse_atom(scope)?;
        Ok(Expression::Comparison {
            left: Box::new(left),
            op,
            right: Box::new(right),
        })
    }

    /// Parse ` [NOT] IN ( SELECT … )` after the left operand is already parsed.
    fn parse_in_predicate_after_left(
        &mut self,
        left: Expression,
        scope: &[usize],
    ) -> Result<Expression, SqlParseError> {
        self.skip_whitespace();
        let negated = if self.peek_keyword("NOT") {
            self.expect_keyword("NOT")?;
            self.skip_whitespace();
            if !self.peek_keyword("IN") {
                return Err(self.error(
                    "in_predicate",
                    "expected IN after NOT (value-list / other NOT forms outside subset)",
                ));
            }
            true
        } else {
            false
        };
        self.expect_keyword("IN")?;
        self.skip_whitespace();
        self.expect_char('(', "in_predicate")?;
        self.skip_whitespace();
        let right = if self.peek_keyword("SELECT") {
            let sub = self.parse_select_body()?;
            self.skip_whitespace();
            self.expect_char(')', "in_predicate")?;
            InRhs::Subquery(Box::new(sub))
        } else {
            // Non-empty value list: IN (atom, atom, …).
            let mut values = Vec::new();
            loop {
                values.push(self.parse_atom(scope)?);
                self.skip_whitespace();
                if self.consume_char(',') {
                    self.skip_whitespace();
                    continue;
                }
                break;
            }
            if values.is_empty() {
                return Err(self.error("in_predicate", "IN value list must be non-empty"));
            }
            self.expect_char(')', "in_predicate")?;
            InRhs::List(values)
        };
        Ok(Expression::In {
            left: Box::new(left),
            negated,
            right,
        })
    }

    /// Parse ` BETWEEN low AND high` after the left operand is already parsed.
    fn parse_between_after_left(
        &mut self,
        left: Expression,
        scope: &[usize],
    ) -> Result<Expression, SqlParseError> {
        self.expect_keyword("BETWEEN")?;
        self.skip_whitespace();
        let low = self.parse_atom(scope)?;
        self.skip_whitespace();
        self.expect_keyword("AND")?;
        self.skip_whitespace();
        let high = self.parse_atom(scope)?;
        Ok(Expression::Between {
            expr: Box::new(left),
            low: Box::new(low),
            high: Box::new(high),
        })
    }

    fn try_parse_cmp_op(&mut self) -> Result<Option<CmpOp>, SqlParseError> {
        if self.peek_keyword("LIKE") {
            self.expect_keyword("LIKE")?;
            return Ok(Some(CmpOp::Like));
        }
        if self.starts_with("<>") {
            self.position += 2;
            return Ok(Some(CmpOp::Ne));
        }
        if self.starts_with("!=") {
            self.position += 2;
            return Ok(Some(CmpOp::Ne));
        }
        if self.starts_with("<=") {
            self.position += 2;
            return Ok(Some(CmpOp::Le));
        }
        if self.starts_with(">=") {
            self.position += 2;
            return Ok(Some(CmpOp::Ge));
        }
        if self.consume_char('=') {
            return Ok(Some(CmpOp::Eq));
        }
        if self.consume_char('<') {
            return Ok(Some(CmpOp::Lt));
        }
        if self.consume_char('>') {
            return Ok(Some(CmpOp::Gt));
        }
        Ok(None)
    }

    fn parse_atom(&mut self, scope: &[usize]) -> Result<Expression, SqlParseError> {
        self.skip_whitespace();
        if self.consume_char('(') {
            self.skip_whitespace();
            // Uncorrelated scalar subquery: `( SELECT … )` (**P9** / M5).
            if self.peek_keyword("SELECT") {
                let sub = self.parse_select_body()?;
                self.skip_whitespace();
                self.expect_char(')', "subquery")?;
                return Ok(Expression::Subquery(Box::new(sub)));
            }
            let inner = self.parse_expression(scope)?;
            self.skip_whitespace();
            self.expect_char(')', "expression")?;
            return Ok(inner);
        }
        if self.peek_char() == Some('\'') {
            let s = self.parse_string_literal()?;
            return Ok(Expression::StringLiteral(s));
        }
        if self.peek_char().is_some_and(|ch| ch.is_ascii_digit()) {
            let n = self.parse_integer_literal("integer_literal")?;
            return Ok(Expression::IntegerLiteral(n));
        }
        // Aggregates in expression position (HAVING SUM/AVG/…/COUNT(col) and COUNT(*)).
        if let Some(func) = self.try_peek_agg_func() {
            return self.parse_aggregate_expression(func, scope);
        }
        // Column ref (possibly qualified). Outer-scope columns are not visible
        // inside a subquery because each SELECT body builds its own scope.
        let col = self.parse_unresolved_column_ref()?;
        let menu_index = self.resolve_column_ordinal(&col, scope)?;
        Ok(Expression::Column(ColumnRef { menu_index }))
    }

    /// Parse `COUNT(*)` / `FUNC([DISTINCT] col)` as an expression atom.
    fn parse_aggregate_expression(
        &mut self,
        func: AggFunc,
        scope: &[usize],
    ) -> Result<Expression, SqlParseError> {
        let keyword = match func {
            AggFunc::Count => "COUNT",
            AggFunc::Sum => "SUM",
            AggFunc::Avg => "AVG",
            AggFunc::Min => "MIN",
            AggFunc::Max => "MAX",
        };
        self.expect_keyword(keyword)?;
        self.skip_whitespace();
        self.expect_char('(', "expression")?;
        self.skip_whitespace();
        if self.consume_char('*') {
            if func != AggFunc::Count {
                return Err(self.error(
                    "expression",
                    format!("{keyword}(*) is not supported; only COUNT(*) is structural"),
                ));
            }
            self.skip_whitespace();
            self.expect_char(')', "expression")?;
            return Ok(Expression::CountStar);
        }
        let distinct = if self.peek_keyword("DISTINCT") {
            self.expect_keyword("DISTINCT")?;
            self.skip_whitespace();
            if self.peek_char() == Some('*') {
                return Err(self.error(
                    "expression",
                    "DISTINCT * is not representable in v0 aggregate expression",
                ));
            }
            true
        } else {
            false
        };
        let col = self.parse_unresolved_column_ref()?;
        let menu_index = self.resolve_column_ordinal(&col, scope)?;
        self.skip_whitespace();
        self.expect_char(')', "expression")?;
        Ok(Expression::Aggregate {
            func,
            arg: AggregateArg::Column(ColumnRef { menu_index }),
            distinct,
        })
    }

    fn parse_string_literal(&mut self) -> Result<String, SqlParseError> {
        self.expect_char('\'', "string_literal")?;
        let mut out = String::new();
        loop {
            let Some(ch) = self.peek_char() else {
                return Err(self.error("string_literal", "unterminated string literal"));
            };
            if ch == '\0' {
                return Err(self.error(
                    "string_literal",
                    "NUL is not representable in SQLite SQL text",
                ));
            }
            if ch == '\'' {
                self.advance_char();
                // SQL escape: doubled single quote.
                if self.peek_char() == Some('\'') {
                    self.advance_char();
                    out.push('\'');
                    continue;
                }
                return Ok(out);
            }
            out.push(ch);
            self.advance_char();
        }
    }

    /// Parse a decimal integer under the shared-band multi-digit capacity
    /// (`0..=16_777_215`). Values above fail closed as `integer_capacity`.
    pub(super) fn parse_integer_literal(
        &mut self,
        rule: &'static str,
    ) -> Result<u32, SqlParseError> {
        let start = self.position;
        if !self.peek_char().is_some_and(|ch| ch.is_ascii_digit()) {
            return Err(self.error(rule, "expected decimal integer literal"));
        }
        while self.peek_char().is_some_and(|ch| ch.is_ascii_digit()) {
            self.advance_char();
        }
        let text = &self.source[start..self.position];
        // Reject leading zeros except a single "0".
        if text.len() > 1 && text.starts_with('0') {
            return Err(self.error(rule, format!("invalid integer literal {text:?}")));
        }
        let value: u64 = text
            .parse()
            .map_err(|_| self.error(rule, format!("invalid integer literal {text:?}")))?;
        let capacity = u64::from(crate::syntax_state_machine::INTEGER_PAYLOAD_CAPACITY_MAX);
        if value > capacity {
            return Err(self.error(
                rule,
                format!(
                    "integer literal {value} exceeds integer capacity ({capacity}); residual bucket integer_capacity"
                ),
            ));
        }
        Ok(value as u32)
    }

    /// Historical name kept as a thin alias for call sites not yet renamed.
    #[allow(dead_code)]
    pub(super) fn parse_u8_integer_literal(
        &mut self,
        rule: &'static str,
    ) -> Result<u32, SqlParseError> {
        self.parse_integer_literal(rule)
    }
}
