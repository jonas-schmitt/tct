//! Pure menu-index radix digit codec.
//!
//! Menu ordinals are fixed-width big-endian base-B digit sequences over the
//! existing menu band (`B` = layout band width, typically 256). Width `W` is
//! derived from live scope cardinality `N` so that `B^W >= N`.
//!
//! Digit mask rule (exact): with `k` digits already consumed, `p` the integer
//! formed so far, and `R = B^(W-1-k)`:
//!
//! ```text
//! end_k = min(B, ceil(N / R) - p * B)
//! digit d admissible  <=>  0 <= d < end_k
//! ```
//!
//! Scope-pushing table sites additionally require at least one completion in
//! the digit's residual range to survive scope exclusion: with
//! `lo = (p*B + d)*R` and `hi = min(lo + R, N)`, admissible iff `lo < hi` and
//! `|scope ∩ [lo,hi)| < (hi - lo)`.
//!
//! At `W = 1` this reduces textually to the historical single-token menu code
//! (mask end = `min(B, N)`; scope excludes exact ordinals). T-A call sites
//! hard-code `W = 1` so behaviour stays bit-identical; later tranches enable
//! `W > 1` behind `max_menu_digits` without forking mask vs apply logic.

/// Minimum digit width `W` such that `base^W >= cardinality`.
///
/// `cardinality == 0` yields `1` (defensive; zero-limit sites fail closed at
/// bind time and never reach the engine). `base < 2` yields `1`.
pub(crate) fn menu_digit_width(cardinality: u32, base: u32) -> u32 {
    if cardinality <= 1 || base < 2 {
        return 1;
    }
    let mut width = 1u32;
    let mut capacity = base as u64;
    let n = cardinality as u64;
    while capacity < n {
        width = width.saturating_add(1);
        // Cap defensive growth: base^width overflow means capacity already
        // exceeds any u32 cardinality.
        match capacity.checked_mul(base as u64) {
            Some(next) => capacity = next,
            None => break,
        }
        // Practical upper bound: two digits over a 256 band already cover 65_536.
        if width >= 8 {
            break;
        }
    }
    width
}

/// Place value `R = base^(width - 1 - digits_seen)` for the next digit.
fn menu_digit_place_value(base: u32, width: u32, digits_seen: u32) -> u64 {
    if base == 0 || width == 0 || digits_seen >= width {
        return 0;
    }
    let exp = width - 1 - digits_seen;
    (base as u64).pow(exp)
}

/// Exclusive upper bound on the next digit (`end_k`).
///
/// Returns `0` when no digit is admissible (including exhausted width, zero
/// base, or a partial already past the live cardinality).
pub(crate) fn menu_digit_end(
    cardinality: u32,
    base: u32,
    width: u32,
    digits_seen: u32,
    partial: u32,
) -> u32 {
    if cardinality == 0 || base == 0 || width == 0 || digits_seen >= width {
        return 0;
    }
    let r = menu_digit_place_value(base, width, digits_seen);
    if r == 0 {
        return 0;
    }
    let n = cardinality as u64;
    // ceil(N / R)
    let ceil_n_over_r = n.div_ceil(r);
    let p_times_b = (partial as u64).saturating_mul(base as u64);
    if ceil_n_over_r <= p_times_b {
        return 0;
    }
    let raw = ceil_n_over_r - p_times_b;
    raw.min(base as u64) as u32
}

/// Whether digit `d` is admissible at the current multi-digit step.
///
/// When `scope` is `Some`, apply the residual-range survival filter used by
/// scope-pushing table menu sites (at least one completion in `[lo, hi)` must
/// not already be in scope). When `scope` is `None`, only the digit-end bound
/// applies (column menus and any non-scope-pushing site).
pub(crate) fn menu_digit_admissible(
    digit: u32,
    cardinality: u32,
    base: u32,
    width: u32,
    digits_seen: u32,
    partial: u32,
    scope: Option<&[u32]>,
) -> bool {
    let end = menu_digit_end(cardinality, base, width, digits_seen, partial);
    if digit >= end {
        return false;
    }
    let Some(scope) = scope else {
        return true;
    };
    let r = menu_digit_place_value(base, width, digits_seen);
    if r == 0 {
        return false;
    }
    let lo = ((partial as u64) * (base as u64) + (digit as u64)).saturating_mul(r);
    let n = cardinality as u64;
    if lo >= n {
        return false;
    }
    let hi = (lo + r).min(n);
    if lo >= hi {
        return false;
    }
    let span = hi - lo;
    let excluded = scope
        .iter()
        .filter(|&&ord| {
            let o = ord as u64;
            o >= lo && o < hi
        })
        .count() as u64;
    excluded < span
}

/// Whether digit `d` is admissible when completions are additionally
/// restricted to the contiguous ordinal range `[allowed_lo, allowed_hi)`.
///
/// Used by join-ON constrained column sites: the digit survives iff its
/// residual completion range `[lo, hi)` intersects the allowed range. With
/// `allowed = None` this is exactly [`menu_digit_admissible`] without a scope
/// filter (column menus never scope-exclude). Empty allowed ranges
/// (`allowed_lo >= allowed_hi`) admit nothing, so an impossible constraint
/// fails closed at the digit level rather than emitting a dead menu band.
pub(crate) fn menu_digit_admissible_in_range(
    digit: u32,
    cardinality: u32,
    base: u32,
    width: u32,
    digits_seen: u32,
    partial: u32,
    allowed: Option<(u32, u32)>,
) -> bool {
    let end = menu_digit_end(cardinality, base, width, digits_seen, partial);
    if digit >= end {
        return false;
    }
    let Some((allowed_lo, allowed_hi)) = allowed else {
        return true;
    };
    if allowed_lo >= allowed_hi {
        return false;
    }
    let r = menu_digit_place_value(base, width, digits_seen);
    if r == 0 {
        return false;
    }
    let lo = ((partial as u64) * (base as u64) + (digit as u64)).saturating_mul(r);
    let n = cardinality as u64;
    if lo >= n {
        return false;
    }
    let hi = (lo + r).min(n);
    if lo >= hi {
        return false;
    }
    // Intersect residual [lo, hi) with allowed [allowed_lo, allowed_hi).
    let a_lo = allowed_lo as u64;
    let a_hi = (allowed_hi as u64).min(n);
    lo.max(a_lo) < hi.min(a_hi)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn width_is_one_at_or_below_base() {
        assert_eq!(menu_digit_width(0, 256), 1);
        assert_eq!(menu_digit_width(1, 256), 1);
        assert_eq!(menu_digit_width(256, 256), 1);
        assert_eq!(menu_digit_width(257, 256), 2);
        assert_eq!(menu_digit_width(65_536, 256), 2);
        assert_eq!(menu_digit_width(65_537, 256), 3);
    }

    #[test]
    fn w1_end_matches_historical_single_token_bound() {
        // At W=1, k=0, p=0: end = min(B, N).
        assert_eq!(menu_digit_end(1, 256, 1, 0, 0), 1);
        assert_eq!(menu_digit_end(2, 256, 1, 0, 0), 2);
        assert_eq!(menu_digit_end(255, 256, 1, 0, 0), 255);
        assert_eq!(menu_digit_end(256, 256, 1, 0, 0), 256);
        // N cannot exceed B at W=1 in production (bind fails closed), but the
        // pure end still clamps to base.
        assert_eq!(menu_digit_end(300, 256, 1, 0, 0), 256);
    }

    #[test]
    fn n257_two_digit_mask_cardinalities() {
        let b = 256u32;
        let n = 257u32;
        let w = menu_digit_width(n, b);
        assert_eq!(w, 2);
        // High digit: {0,1}
        assert_eq!(menu_digit_end(n, b, w, 0, 0), 2);
        // Low after high=0: 256 options
        assert_eq!(menu_digit_end(n, b, w, 1, 0), 256);
        // Low after high=1: 1 option
        assert_eq!(menu_digit_end(n, b, w, 1, 1), 1);
        // Total completions: 256 + 1 = 257
        let total: u32 = (0..menu_digit_end(n, b, w, 0, 0))
            .map(|high| menu_digit_end(n, b, w, 1, high))
            .sum();
        assert_eq!(total, 257);
    }

    #[test]
    fn w1_scope_excludes_exact_ordinals() {
        let scope = [1u32, 3];
        assert!(menu_digit_admissible(0, 4, 256, 1, 0, 0, Some(&scope)));
        assert!(!menu_digit_admissible(1, 4, 256, 1, 0, 0, Some(&scope)));
        assert!(menu_digit_admissible(2, 4, 256, 1, 0, 0, Some(&scope)));
        assert!(!menu_digit_admissible(3, 4, 256, 1, 0, 0, Some(&scope)));
        assert!(!menu_digit_admissible(4, 4, 256, 1, 0, 0, None));
    }

    #[test]
    fn w1_allowed_range_restricts_to_contiguous_ordinals() {
        // Scope-relative concatenation of 7 columns; allowed = [3, 7) (the
        // just-joined table's slice). W=1: digits are exact ordinals.
        for d in 0..3 {
            assert!(!menu_digit_admissible_in_range(
                d,
                7,
                256,
                1,
                0,
                0,
                Some((3, 7))
            ));
        }
        for d in 3..7 {
            assert!(menu_digit_admissible_in_range(
                d,
                7,
                256,
                1,
                0,
                0,
                Some((3, 7))
            ));
        }
        // Out of live cardinality stays inadmissible regardless of allowance.
        assert!(!menu_digit_admissible_in_range(
            7,
            7,
            256,
            1,
            0,
            0,
            Some((3, 7))
        ));
        // None behaves exactly like the unfiltered digit-end bound.
        assert!(menu_digit_admissible_in_range(0, 7, 256, 1, 0, 0, None));
    }

    #[test]
    fn empty_allowed_range_admits_nothing() {
        for d in 0..7 {
            assert!(!menu_digit_admissible_in_range(
                d,
                7,
                256,
                1,
                0,
                0,
                Some((0, 0))
            ));
        }
    }

    #[test]
    fn w2_high_digit_survives_only_when_residual_intersects_allowed() {
        // N=300 over base 256 → W=2. Allowed = [256, 300): high digit 0 has
        // residual [0,256) (disjoint → dead), high digit 1 residual [256,300)
        // (intersects → alive).
        let (b, n, w) = (256u32, 300u32, 2u32);
        assert!(!menu_digit_admissible_in_range(
            0,
            n,
            b,
            w,
            0,
            0,
            Some((256, 300))
        ));
        assert!(menu_digit_admissible_in_range(
            1,
            n,
            b,
            w,
            0,
            0,
            Some((256, 300))
        ));
        // After high=1, low digits map to ordinals 256+d: allowed low digits
        // are exactly 0..44.
        assert!(menu_digit_admissible_in_range(
            0,
            n,
            b,
            w,
            1,
            1,
            Some((256, 300))
        ));
        assert!(menu_digit_admissible_in_range(
            43,
            n,
            b,
            w,
            1,
            1,
            Some((256, 300))
        ));
        assert!(!menu_digit_admissible_in_range(
            44,
            n,
            b,
            w,
            1,
            1,
            Some((256, 300))
        ));
        // Allowed slice inside the low band: [0, 3) restricts high digit to 0.
        assert!(menu_digit_admissible_in_range(
            0,
            n,
            b,
            w,
            0,
            0,
            Some((0, 3))
        ));
        assert!(!menu_digit_admissible_in_range(
            1,
            n,
            b,
            w,
            0,
            0,
            Some((0, 3))
        ));
    }

    #[test]
    fn dead_high_digit_when_only_residual_is_in_scope() {
        // N=257, ordinal 256 already in scope → high digit 1 has residual
        // [256, 257) fully excluded, so high=1 must be inadmissible.
        let b = 256u32;
        let n = 257u32;
        let w = 2u32;
        let scope = [256u32];
        assert!(menu_digit_admissible(0, n, b, w, 0, 0, Some(&scope)));
        assert!(!menu_digit_admissible(1, n, b, w, 0, 0, Some(&scope)));
        // Without scope, high=1 remains open (low digit 0 completes ordinal 256).
        assert!(menu_digit_admissible(1, n, b, w, 0, 0, None));
    }
}
