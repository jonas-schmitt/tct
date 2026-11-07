//! Variable-width integer payload codec over the shared 256-wide
//! `u8_integer_payload` band.
//!
//! # Canonical forms (prefix-free **and** unique)
//!
//! | Value | Stream digits (base-256) |
//! |---|---|
//! | `0..=253` | one digit = value (bit-identical to the historical single-token form) |
//! | `254..=65_535` | marker `254` + 2 big-endian digits |
//! | `65_536..=16_777_215` | marker `255` + 3 big-endian digits |
//! | `> 16_777_215` | capacity failure (`integer_capacity`) |
//!
//! The band width stays 256 (SharedAlphabet / Go / C# role). Values 254 and 255
//! as *payloads* never use a single digit — those digit codes are escape markers.
//!
//! # Canonical-form enforcement
//!
//! Without mask discipline the scheme is not unique: value `5` can also be
//! written as marker `254` + `(0, 5)`. Non-canonical streams break
//! decode→re-encode token fixpoint. Continuation digits must exclude
//! short-form-expressible values:
//!
//! - After marker `254`, reconstructed value must be `≥ 254`
//!   (if high digit `d1 == 0` then low digit `d0 ∈ {254, 255}` only).
//! - After marker `255`, high digit `d2 ≥ 1` (value `≥ 65_536`).

/// Largest value encoded as a single digit (historical bit-identical range).
pub(crate) const INTEGER_SINGLE_MAX: u32 = 253;
/// Escape marker for the 2-digit continuation form.
pub(crate) const INTEGER_MARKER_2: u8 = 254;
/// Escape marker for the 3-digit continuation form.
pub(crate) const INTEGER_MARKER_3: u8 = 255;
/// Inclusive max for 2-digit form (`256^2 - 1`).
pub(crate) const INTEGER_TWO_DIGIT_MAX: u32 = 65_535;
/// Inclusive max encodable value (`256^3 - 1`).
pub(crate) const INTEGER_CAPACITY_MAX: u32 = 16_777_215;

/// Encode `value` to the canonical digit sequence (band-relative 0..=255).
pub(crate) fn encode_integer_digits(value: u32) -> Result<Vec<u8>, String> {
    if value <= INTEGER_SINGLE_MAX {
        Ok(vec![value as u8])
    } else if value <= INTEGER_TWO_DIGIT_MAX {
        let hi = ((value >> 8) & 0xff) as u8;
        let lo = (value & 0xff) as u8;
        Ok(vec![INTEGER_MARKER_2, hi, lo])
    } else if value <= INTEGER_CAPACITY_MAX {
        let b2 = ((value >> 16) & 0xff) as u8;
        let b1 = ((value >> 8) & 0xff) as u8;
        let b0 = (value & 0xff) as u8;
        debug_assert!(b2 >= 1, "3-digit form requires high digit ≥ 1");
        Ok(vec![INTEGER_MARKER_3, b2, b1, b0])
    } else {
        Err(format!(
            "integer value {value} exceeds integer capacity {INTEGER_CAPACITY_MAX} (variable-width multi-digit on 256 band)"
        ))
    }
}

/// Expected total digit count once the first digit is known.
pub(crate) fn expected_digit_count(first: u8) -> u32 {
    if first <= INTEGER_SINGLE_MAX as u8 {
        1
    } else if first == INTEGER_MARKER_2 {
        3
    } else {
        // INTEGER_MARKER_3
        4
    }
}

/// Whether the accumulated digits form a complete integer payload.
pub(crate) fn integer_digits_complete(digits: &[u8]) -> bool {
    match digits.first() {
        None => false,
        Some(&first) => digits.len() as u32 == expected_digit_count(first),
    }
}

/// Whether `digit` is admissible as the next digit after `prefix` (canonical).
///
/// `prefix` is the digits already accepted (length = `digits_seen`).
pub(crate) fn integer_digit_admissible(prefix: &[u8], digit: u8) -> bool {
    match prefix.len() {
        0 => true, // any first digit 0..=255 (254/255 are markers)
        _ => {
            let first = prefix[0];
            if first <= INTEGER_SINGLE_MAX as u8 {
                // Single-digit form already complete; no further value digits.
                return false;
            }
            if first == INTEGER_MARKER_2 {
                // Need two continuation digits (indices 1, 2).
                match prefix.len() {
                    1 => true, // high digit free
                    2 => {
                        let d1 = prefix[1];
                        if d1 == 0 {
                            // value = d0 must be ≥ 254
                            digit >= INTEGER_MARKER_2
                        } else {
                            true
                        }
                    }
                    _ => false,
                }
            } else if first == INTEGER_MARKER_3 {
                match prefix.len() {
                    1 => digit >= 1, // d2 ≥ 1 → value ≥ 65_536
                    2 | 3 => true,
                    _ => false,
                }
            } else {
                false
            }
        }
    }
}

/// Decode a complete canonical digit sequence to a value.
pub(crate) fn decode_integer_digits(digits: &[u8]) -> Result<u32, String> {
    if digits.is_empty() {
        return Err("integer payload has no digits".to_string());
    }
    let first = digits[0];
    let expected = expected_digit_count(first);
    if digits.len() as u32 != expected {
        return Err(format!(
            "integer payload expected {expected} digits after first={first}, got {}",
            digits.len()
        ));
    }
    // Reject non-canonical continuations even if length matches.
    for i in 1..digits.len() {
        if !integer_digit_admissible(&digits[..i], digits[i]) {
            return Err(format!(
                "non-canonical integer digit sequence {digits:?} (short-form-expressible continuation)"
            ));
        }
    }
    let value = match first {
        d if d <= INTEGER_SINGLE_MAX as u8 => u32::from(d),
        INTEGER_MARKER_2 => {
            let hi = u32::from(digits[1]);
            let lo = u32::from(digits[2]);
            hi * 256 + lo
        }
        INTEGER_MARKER_3 => {
            let b2 = u32::from(digits[1]);
            let b1 = u32::from(digits[2]);
            let b0 = u32::from(digits[3]);
            b2 * 65_536 + b1 * 256 + b0
        }
        _ => unreachable!("first digit classified by expected_digit_count"),
    };
    if value > INTEGER_CAPACITY_MAX {
        return Err(format!(
            "decoded integer {value} exceeds capacity {INTEGER_CAPACITY_MAX}"
        ));
    }
    // Double-check encode fixpoint.
    let re = encode_integer_digits(value)?;
    if re != digits {
        return Err(format!(
            "non-canonical integer digits {digits:?} decode to {value} but re-encode as {re:?}"
        ));
    }
    Ok(value)
}

/// Parse a decimal integer payload string under capacity.
pub(crate) fn parse_integer_payload_text(payload_text: &str) -> Result<u32, String> {
    let value: u64 = payload_text
        .parse()
        .map_err(|_| format!("integer payload {payload_text:?} is not a decimal integer"))?;
    if value > u64::from(INTEGER_CAPACITY_MAX) {
        return Err(format!(
            "integer value {value} exceeds integer capacity {INTEGER_CAPACITY_MAX}"
        ));
    }
    Ok(value as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_single_digit_bit_identical_range() {
        for v in [0u32, 1, 42, 253] {
            assert_eq!(encode_integer_digits(v).unwrap(), vec![v as u8]);
        }
    }

    #[test]
    fn encode_two_and_three_digit_forms() {
        assert_eq!(encode_integer_digits(254).unwrap(), vec![254, 0, 254]);
        assert_eq!(encode_integer_digits(255).unwrap(), vec![254, 0, 255]);
        assert_eq!(encode_integer_digits(256).unwrap(), vec![254, 1, 0]);
        assert_eq!(encode_integer_digits(300).unwrap(), vec![254, 1, 44]);
        assert_eq!(encode_integer_digits(2014).unwrap(), vec![254, 7, 222]);
        assert_eq!(encode_integer_digits(65_535).unwrap(), vec![254, 255, 255]);
        assert_eq!(encode_integer_digits(65_536).unwrap(), vec![255, 1, 0, 0]);
        assert_eq!(
            encode_integer_digits(16_777_215).unwrap(),
            vec![255, 255, 255, 255]
        );
    }

    #[test]
    fn capacity_rejects_over_max() {
        assert!(encode_integer_digits(16_777_216).is_err());
        assert!(encode_integer_digits(242_518_965).is_err());
    }

    #[test]
    fn decode_round_trip_and_fixpoint() {
        for v in [
            0u32, 1, 253, 254, 255, 256, 300, 2014, 65_535, 65_536, 1_000_000, 16_777_215,
        ] {
            let digits = encode_integer_digits(v).unwrap();
            assert_eq!(decode_integer_digits(&digits).unwrap(), v);
            // fixpoint: re-encode matches
            assert_eq!(encode_integer_digits(v).unwrap(), digits);
        }
    }

    #[test]
    fn non_canonical_two_digit_form_for_small_value_rejected() {
        // value 5 as marker 254 + (0, 5) — short-form expressible
        let bad = [254u8, 0, 5];
        assert!(decode_integer_digits(&bad).is_err());
        assert!(!integer_digit_admissible(&[254, 0], 5));
        assert!(integer_digit_admissible(&[254, 0], 254));
        assert!(integer_digit_admissible(&[254, 0], 255));
    }

    #[test]
    fn non_canonical_three_digit_form_high_zero_rejected() {
        // marker 255 + d2==0 would encode value < 65536
        assert!(!integer_digit_admissible(&[255], 0));
        assert!(integer_digit_admissible(&[255], 1));
        let bad = [255u8, 0, 1, 0];
        assert!(decode_integer_digits(&bad).is_err());
    }

    #[test]
    fn mask_after_marker_254_high_nonzero_allows_all_low() {
        assert!(integer_digit_admissible(&[254, 1], 0));
        assert!(integer_digit_admissible(&[254, 1], 255));
    }
}
