//! Factored column-address digit codec: `(table, column)` menu pairs.
//!
//! Sibling of the radix [`super::menu_codec`], and deliberately *not* a radix
//! code. A factored column address is two positional digits over two
//! **different** alphabets:
//!
//! - digit 0 is a catalog table index, masked to the tables currently in
//!   scope;
//! - digit 1 is a column index inside that table, so its cardinality is a
//!   function of digit 0's value.
//!
//! The radix engine derives every digit's bound from one flat cardinality and
//! cannot express that dependency, which is why this lives in its own module
//! rather than as a wider `menu_digit_*` mode.
//!
//! The decoded *value* is still the flat scope-relative ordinal the AST
//! carries, so decode recomposes what encode decomposed and nothing
//! downstream of the token stream changes. Mask, apply, and encode all route
//! through [`factored_table_slice`], so the three agree by construction.

/// Digit count of a factored column address: `(table, column)`.
///
/// Fixed at two positional digits; unlike [`super::menu_codec`]'s radix code,
/// the two digits have *different* alphabets (catalog tables, then that
/// table's columns), so the second digit's cardinality depends on the first
/// digit's value.
pub(crate) const FACTORED_COLUMN_MENU_DIGITS: u32 = 2;

/// Flat scope-relative slice `[lo, lo + count)` occupied by catalog table
/// `table` in the live scope concatenation, or `None` when `table` is not in
/// scope.
///
/// This is the single place that relates factored `(table, column)` digits to
/// the flat scope-relative ordinal the AST still carries.
/// Duplicate scope entries cannot occur pre-M6 (both encode and decode reject
/// repeated table ordinals); the first occurrence is used either way so the
/// forward and inverse maps stay mutually consistent.
pub(crate) fn factored_table_slice(
    scope: &[u32],
    columns_per_table: &[u32],
    table: u32,
) -> Option<(u32, u32)> {
    let mut lo = 0u32;
    for &in_scope in scope {
        let count = columns_per_table
            .get(in_scope as usize)
            .copied()
            .unwrap_or(0);
        if in_scope == table {
            return Some((lo, count));
        }
        lo = lo.saturating_add(count);
    }
    None
}

/// Whether flat range `[lo, hi)` survives an optional allowed-ordinal range.
///
/// `allowed` is the join-ON structural constraint expressed in flat
/// scope-relative ordinals, so the two SQL policies compose without either one
/// knowing about the other. An empty range admits nothing (fail closed).
fn factored_flat_range_allowed(lo: u32, hi: u32, allowed: Option<(u32, u32)>) -> bool {
    if lo >= hi {
        return false;
    }
    match allowed {
        None => true,
        Some((allowed_lo, allowed_hi)) => lo.max(allowed_lo) < hi.min(allowed_hi),
    }
}

/// Digit 0 admissibility: `digit` is a catalog table index, admissible iff the
/// table is in scope, contributes at least one column, and at least one of its
/// columns survives `allowed`.
pub(crate) fn factored_table_digit_admissible(
    digit: u32,
    scope: &[u32],
    columns_per_table: &[u32],
    allowed: Option<(u32, u32)>,
) -> bool {
    match factored_table_slice(scope, columns_per_table, digit) {
        Some((lo, count)) => factored_flat_range_allowed(lo, lo.saturating_add(count), allowed),
        None => false,
    }
}

/// Digit 1 admissibility: `digit` is a column index inside catalog table
/// `table` (which digit 0 already fixed).
pub(crate) fn factored_column_digit_admissible(
    digit: u32,
    table: u32,
    scope: &[u32],
    columns_per_table: &[u32],
    allowed: Option<(u32, u32)>,
) -> bool {
    let Some((lo, count)) = factored_table_slice(scope, columns_per_table, table) else {
        return false;
    };
    if digit >= count {
        return false;
    }
    let flat = lo.saturating_add(digit);
    factored_flat_range_allowed(flat, flat.saturating_add(1), allowed)
}

/// Flat scope-relative ordinal addressed by a complete `(table, column)` pair.
pub(crate) fn factored_flat_ordinal(
    table: u32,
    column: u32,
    scope: &[u32],
    columns_per_table: &[u32],
) -> Option<u32> {
    let (lo, count) = factored_table_slice(scope, columns_per_table, table)?;
    (column < count).then(|| lo.saturating_add(column))
}

/// Inverse of [`factored_flat_ordinal`]: the `(table, column)` digits that
/// address flat scope-relative ordinal `flat`.
///
/// `None` when `flat` lies outside the live scope concatenation; encode turns
/// that into a typed fail-closed error rather than emitting a wrong pair.
pub(crate) fn factored_digits_for_flat_ordinal(
    flat: u32,
    scope: &[u32],
    columns_per_table: &[u32],
) -> Option<(u32, u32)> {
    let mut lo = 0u32;
    for &table in scope {
        let count = columns_per_table.get(table as usize).copied().unwrap_or(0);
        let hi = lo.saturating_add(count);
        if flat >= lo && flat < hi {
            return Some((table, flat - lo));
        }
        lo = hi;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    /// concert_singer column counts by catalog index (the L4 measurement shape).
    const CONCERT_SINGER: [u32; 4] = [7, 7, 5, 2];

    #[test]
    fn factored_pair_is_scope_independent_where_the_flat_ordinal_is_not() {
        // singer = catalog table 1, `Name` = column 1. The flat ordinal moves
        // with FROM order (1 / 8 / 6); the factored pair does not.
        for (scope, flat) in [(vec![1u32], 1u32), (vec![0, 1], 8), (vec![2, 1], 6)] {
            assert_eq!(
                factored_flat_ordinal(1, 1, &scope, &CONCERT_SINGER),
                Some(flat),
                "scope {scope:?}"
            );
            assert_eq!(
                factored_digits_for_flat_ordinal(flat, &scope, &CONCERT_SINGER),
                Some((1, 1)),
                "scope {scope:?}"
            );
        }
    }

    #[test]
    fn factored_digits_round_trip_over_every_live_ordinal() {
        let scope = vec![2u32, 1, 0];
        let total: u32 = scope.iter().map(|&t| CONCERT_SINGER[t as usize]).sum();
        for flat in 0..total {
            let (table, column) = factored_digits_for_flat_ordinal(flat, &scope, &CONCERT_SINGER)
                .expect("live ordinal decomposes");
            assert!(factored_table_digit_admissible(
                table,
                &scope,
                &CONCERT_SINGER,
                None
            ));
            assert!(factored_column_digit_admissible(
                column,
                table,
                &scope,
                &CONCERT_SINGER,
                None
            ));
            assert_eq!(
                factored_flat_ordinal(table, column, &scope, &CONCERT_SINGER),
                Some(flat)
            );
        }
        assert_eq!(
            factored_digits_for_flat_ordinal(total, &scope, &CONCERT_SINGER),
            None,
            "an ordinal past the live scope must fail closed"
        );
    }

    #[test]
    fn out_of_scope_tables_are_inadmissible_at_the_table_digit() {
        let scope = vec![0u32, 1];
        assert!(factored_table_digit_admissible(
            0,
            &scope,
            &CONCERT_SINGER,
            None
        ));
        assert!(factored_table_digit_admissible(
            1,
            &scope,
            &CONCERT_SINGER,
            None
        ));
        for out in [2u32, 3, 4, 255] {
            assert!(
                !factored_table_digit_admissible(out, &scope, &CONCERT_SINGER, None),
                "table {out} is not in scope"
            );
        }
        assert_eq!(factored_flat_ordinal(2, 0, &scope, &CONCERT_SINGER), None);
    }

    #[test]
    fn column_digit_admits_exactly_the_chosen_tables_columns() {
        let scope = vec![0u32, 1, 2];
        for (table, count) in [(0u32, 7u32), (1, 7), (2, 5)] {
            for column in 0..count {
                assert!(factored_column_digit_admissible(
                    column,
                    table,
                    &scope,
                    &CONCERT_SINGER,
                    None
                ));
            }
            assert!(
                !factored_column_digit_admissible(count, table, &scope, &CONCERT_SINGER, None),
                "table {table} has only {count} columns"
            );
        }
    }

    #[test]
    fn join_on_allowed_range_composes_with_factored_digits() {
        // scope [stadium(7), singer(7)]; the structural join-ON right side is
        // restricted to the just-joined table's slice [7, 14).
        let scope = vec![0u32, 1];
        let allowed = Some((7u32, 14u32));
        assert!(!factored_table_digit_admissible(
            0,
            &scope,
            &CONCERT_SINGER,
            allowed
        ));
        assert!(factored_table_digit_admissible(
            1,
            &scope,
            &CONCERT_SINGER,
            allowed
        ));
        for column in 0..7 {
            assert!(factored_column_digit_admissible(
                column,
                1,
                &scope,
                &CONCERT_SINGER,
                allowed
            ));
            assert!(!factored_column_digit_admissible(
                column,
                0,
                &scope,
                &CONCERT_SINGER,
                allowed
            ));
        }
        // An impossible constraint admits nothing.
        assert!(!factored_table_digit_admissible(
            1,
            &scope,
            &CONCERT_SINGER,
            Some((0, 0))
        ));
    }

    #[test]
    fn empty_scope_admits_no_factored_digit() {
        for table in 0..4u32 {
            assert!(!factored_table_digit_admissible(
                table,
                &[],
                &CONCERT_SINGER,
                None
            ));
        }
        assert_eq!(
            factored_digits_for_flat_ordinal(0, &[], &CONCERT_SINGER),
            None
        );
    }
}
