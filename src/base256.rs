//! Multi-token encoding with configurable base
//!
//! Encodes u32 values using 1-4 digits with configurable power-of-2 base.
//! All tokens respect vocabulary constraints based on the chosen base.

use crate::tokenizer::DecodeError;
use crate::tokenizer::EOFContext;

/// Default base for multi-token encoding (256 for backward compatibility)
pub const DEFAULT_BASE: u32 = 256;

/// The width indicator is decoded modulo four, so supported bases must encode
/// every `u32` value in at most four base digits.
const MAX_WIDTH_INDICATOR_DIGITS: u32 = 4;

/// Calculate maximum token width needed for a given base
///
/// Returns the minimum number of tokens needed to encode u32::MAX with the given base.
/// For power-of-2 bases: width = ceil(32 / log2(base))
///
/// # Examples
/// - base 256 (2^8): 4 tokens (32/8 = 4)
/// - base 512 (2^9): 4 tokens (32/9 = 3.56 → 4)
/// - base 65536 (2^16): 2 tokens (32/16 = 2)
#[inline(always)]
fn max_token_width_for_base(base: u32) -> u32 {
    let mut width = 1u32;
    let mut max_value = base as u64;
    while max_value <= u32::MAX as u64 {
        width += 1;
        max_value = max_value.saturating_mul(base as u64);
    }
    debug_assert!(
        width <= MAX_WIDTH_INDICATOR_DIGITS,
        "base {base} requires {width} digits, but the width indicator supports at most {MAX_WIDTH_INDICATOR_DIGITS}",
    );
    width
}

#[inline(always)]
fn checked_base_power(base: u32, exponent: u32) -> u32 {
    base.checked_pow(exponent)
        .expect("variable-base divisor invariant violated: base^(width - 1) must fit in u32")
}

/// Encode u32 value using multi-token encoding with configurable base
///
/// Format: [width_indicator, digit_n-1, ..., digit_1, digit_0]
/// - width_indicator: 0-3 (maps to 1-4 digits)
/// - digits: big-endian base-N digits
///
/// # Arguments
/// * `value` - Value to encode (0 to u32::MAX)
/// * `tokens` - Token vector to append to
/// * `base` - Encoding base (must be power-of-2, typically 256-65536)
///
/// # Example
/// ```
/// let mut tokens = Vec::new();
/// tct::base256::encode_u32(42, &mut tokens, 256);
/// // tokens = [0, 42] (width=1, value=42)
///
/// tct::base256::encode_u32(1000, &mut tokens, 256);
/// // tokens = [0, 42, 1, 3, 232] (width=2, value=1000 = 3*256 + 232)
/// ```
#[inline(always)]
pub fn encode_u32(value: u32, tokens: &mut Vec<u32>, base: u32) {
    let max_width = max_token_width_for_base(base);

    // Calculate number of digits needed
    let mut token_width = 1u32;
    let mut max_value = base;
    while value >= max_value && token_width < max_width {
        token_width += 1;
        max_value = max_value.saturating_mul(base);
    }
    debug_assert!(
        token_width <= MAX_WIDTH_INDICATOR_DIGITS,
        "base {base} selected {token_width} digits, but the width indicator supports at most {MAX_WIDTH_INDICATOR_DIGITS}",
    );

    // Push width indicator (0-3 for 1-4 digits)
    tokens.push(token_width - 1);

    // Push digits in big-endian order (most significant first)
    for i in (0..token_width).rev() {
        // `i` is always below the computed token width. For supported bases,
        // that makes base^i the largest emitted divisor and keeps it inside
        // u32; the checked helper makes the contract fail close to the source.
        let digit = (value / checked_base_power(base, i)) % base;
        tokens.push(digit);
    }
}

/// Decode u32 value from multi-token encoding with configurable base
///
/// # Arguments
/// * `tokens` - Token iterator
/// * `context_type` - Type name for error messages (e.g., "i32", "Vec length").
///   Must be a string literal with 'static lifetime
/// * `base` - Encoding base (must match the base used for encoding)
///
/// # Returns
/// * `Ok(value)` - Decoded value
/// * `Err(DecodeError)` - If stream ends unexpectedly
///
/// # Example
/// ```
/// let tokens = vec![1, 3, 232];  // width=2, value=1000
/// let mut iter = tokens.into_iter();
/// let value = tct::base256::decode_u32(&mut iter, "test", 256).unwrap();
/// assert_eq!(value, 1000);
/// ```
#[inline(always)]
pub fn decode_u32(
    tokens: &mut impl Iterator<Item = u32>,
    context_type: &'static str,
    base: u32,
) -> Result<u32, DecodeError> {
    // Read width indicator
    let width_indicator = tokens.next().ok_or(DecodeError::UnexpectedEndOfStream {
        context: EOFContext::new(context_type, Some("width"), "width indicator"),
    })?;
    let token_width = (width_indicator % 4) + 1; // 0-3 maps to 1-4 digits

    // Read digits and reconstruct value
    let mut value = 0u32;
    for _ in 0..token_width {
        let digit = tokens.next().ok_or(DecodeError::UnexpectedEndOfStream {
            context: EOFContext::new(context_type, Some("digit"), "multi-token digit"),
        })?;
        value = value.saturating_mul(base).saturating_add(digit % base);
    }

    Ok(value)
}

#[inline(always)]
fn decode_digit_strict(
    tokens: &mut impl Iterator<Item = u32>,
    context_type: &'static str,
    base: u32,
) -> Result<u32, DecodeError> {
    let digit = tokens.next().ok_or(DecodeError::UnexpectedEndOfStream {
        context: EOFContext::new(context_type, Some("digit"), "multi-token digit"),
    })?;
    if digit < base {
        Ok(digit)
    } else {
        Err(DecodeError::InvalidTokenInContext {
            token: digit,
            context: EOFContext::new(context_type, Some("digit"), "multi-token digit below base"),
        })
    }
}

/// Decode u32 value from multi-token encoding without modulo-normalizing token values.
#[inline(always)]
pub fn decode_u32_strict(
    tokens: &mut impl Iterator<Item = u32>,
    context_type: &'static str,
    base: u32,
) -> Result<u32, DecodeError> {
    let width_indicator = tokens.next().ok_or(DecodeError::UnexpectedEndOfStream {
        context: EOFContext::new(context_type, Some("width"), "width indicator"),
    })?;
    if width_indicator >= MAX_WIDTH_INDICATOR_DIGITS {
        return Err(DecodeError::InvalidTokenInContext {
            token: width_indicator,
            context: EOFContext::new(context_type, Some("width"), "width indicator 0..=3"),
        });
    }
    let token_width = width_indicator + 1;

    let mut value = 0u32;
    for _ in 0..token_width {
        let digit = decode_digit_strict(tokens, context_type, base)?;
        value = value.saturating_mul(base).saturating_add(digit);
    }

    Ok(value)
}

/// Calculate token count for encoding a u32 value with configurable base
///
/// Returns: 1 (width indicator) + number of digits (1-4)
///
/// # Arguments
/// * `value` - Value to encode
/// * `base` - Encoding base (must be power-of-2)
#[inline(always)]
pub fn token_count_u32(value: u32, base: u32) -> usize {
    let max_width = max_token_width_for_base(base);

    let mut token_width = 1u32;
    let mut max_value = base;
    while value >= max_value && token_width < max_width {
        token_width += 1;
        max_value = max_value.saturating_mul(base);
    }
    (token_width + 1) as usize // +1 for width indicator
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_decode_round_trip() {
        let test_values = vec![
            0u32,     // 1 digit
            255,      // 1 digit (max single)
            256,      // 2 digits
            65535,    // 2 digits (max)
            65536,    // 3 digits
            16777215, // 3 digits (max)
            16777216, // 4 digits
            u32::MAX, // 4 digits (max)
        ];

        for value in test_values {
            let mut tokens = Vec::new();
            encode_u32(value, &mut tokens, DEFAULT_BASE);
            let mut iter = tokens.into_iter();
            let decoded = decode_u32(&mut iter, "test", DEFAULT_BASE).unwrap();
            assert_eq!(value, decoded, "Round-trip failed for {}", value);
        }
    }

    #[test]
    fn test_token_count() {
        assert_eq!(token_count_u32(0, DEFAULT_BASE), 2); // width + 1 digit
        assert_eq!(token_count_u32(255, DEFAULT_BASE), 2); // width + 1 digit
        assert_eq!(token_count_u32(256, DEFAULT_BASE), 3); // width + 2 digits
        assert_eq!(token_count_u32(65535, DEFAULT_BASE), 3); // width + 2 digits
        assert_eq!(token_count_u32(65536, DEFAULT_BASE), 4); // width + 3 digits
        assert_eq!(token_count_u32(u32::MAX, DEFAULT_BASE), 5); // width + 4 digits
    }

    #[test]
    fn test_decode_error_on_empty_stream() {
        let mut iter = std::iter::empty();
        let result = decode_u32(&mut iter, "test", DEFAULT_BASE);
        assert!(result.is_err());
    }

    #[test]
    fn test_decode_error_on_incomplete_stream() {
        let tokens = vec![1]; // width=2, but no digits
        let mut iter = tokens.into_iter();
        let result = decode_u32(&mut iter, "test", DEFAULT_BASE);
        assert!(result.is_err());
    }

    #[test]
    fn test_configurable_base_512() {
        let base = 512u32;
        let test_values = vec![0u32, 511, 512, 65535, 65536, u32::MAX];

        for value in test_values {
            let mut tokens = Vec::new();
            encode_u32(value, &mut tokens, base);
            let mut iter = tokens.into_iter();
            let decoded = decode_u32(&mut iter, "test", base).unwrap();
            assert_eq!(
                value, decoded,
                "Round-trip failed for {} with base {}",
                value, base
            );
        }
    }

    #[test]
    fn test_configurable_base_1024() {
        let base = 1024u32;
        let test_values = vec![0u32, 1023, 1024, 65535, 65536, u32::MAX];

        for value in test_values {
            let mut tokens = Vec::new();
            encode_u32(value, &mut tokens, base);
            let mut iter = tokens.into_iter();
            let decoded = decode_u32(&mut iter, "test", base).unwrap();
            assert_eq!(
                value, decoded,
                "Round-trip failed for {} with base {}",
                value, base
            );
        }
    }

    #[test]
    fn test_configurable_base_65536() {
        let base = 65536u32;
        let test_values = vec![0u32, 65535, 65536, 16777215, u32::MAX];

        for value in test_values {
            let mut tokens = Vec::new();
            encode_u32(value, &mut tokens, base);
            let mut iter = tokens.into_iter();
            let decoded = decode_u32(&mut iter, "test", base).unwrap();
            assert_eq!(
                value, decoded,
                "Round-trip failed for {} with base {}",
                value, base
            );
        }
    }

    #[test]
    fn test_max_token_width_calculation() {
        assert_eq!(max_token_width_for_base(256), 4); // 256^4 = 2^32
        assert_eq!(max_token_width_for_base(512), 4); // 512^4 > 2^32 (need 4)
        assert_eq!(max_token_width_for_base(1024), 4); // 1024^4 > 2^32 (need 4)
        assert_eq!(max_token_width_for_base(2048), 3); // 2048^3 > 2^32 (need 3)
        assert_eq!(max_token_width_for_base(65536), 2); // 65536^2 = 2^32
    }

    #[test]
    fn test_variable_base_exponent_width_invariant() {
        for base in [
            256u32,
            512,
            1024,
            2048,
            4096,
            8192,
            16384,
            32768,
            65536,
            1 << 20,
            1 << 31,
        ] {
            let width = max_token_width_for_base(base);

            assert!(
                width <= MAX_WIDTH_INDICATOR_DIGITS,
                "base {base} must fit the width indicator"
            );
            assert!(
                base.checked_pow(width - 1).is_some(),
                "largest emitted divisor must fit in u32 for base {base}"
            );
            assert!(
                u64::from(base).checked_pow(width).unwrap() > u64::from(u32::MAX),
                "base {base} and width {width} must cover all u32 values"
            );
        }
    }

    #[test]
    fn test_token_count_varies_with_base() {
        let value = 1000u32;

        // Base 256: 1000 needs 2 digits (3*256 + 232)
        assert_eq!(token_count_u32(value, 256), 3); // width + 2 digits

        // Base 512: 1000 needs 2 digits (1*512 + 488)
        assert_eq!(token_count_u32(value, 512), 3); // width + 2 digits

        // Base 1024: 1000 needs 1 digit (just 1000)
        assert_eq!(token_count_u32(value, 1024), 2); // width + 1 digit

        // Base 65536: 1000 needs 1 digit (just 1000)
        assert_eq!(token_count_u32(value, 65536), 2); // width + 1 digit
    }
}
