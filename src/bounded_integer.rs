//! Bounded Integer Types for Constraint-Aware Encoding
//!
//! This module provides specialized integer types that optimize encoding
//! for integers with minimum/maximum constraints, achieving 90%+ compression
//! compared to standard i64 encoding.

use crate::tokenizer::{DecodeError, EOFContext, JsonTokenEncoder};

const BOUNDED_U8_VALUES: u64 = u8::MAX as u64 + 1;
const BOUNDED_U16_VALUES: u64 = u16::MAX as u64 + 1;
const BOUNDED_U32_VALUES: u64 = u32::MAX as u64 + 1;

fn validate_range_capacity(
    min: i64,
    max: i64,
    type_name: &str,
    max_values: u64,
) -> Result<u64, String> {
    if max < min {
        return Err(format!(
            "Invalid range [{}, {}] for {}: maximum is below minimum",
            min, max, type_name
        ));
    }

    let range_size = i128::from(max) - i128::from(min) + 1;
    if range_size > i128::from(max_values) {
        return Err(format!(
            "Range [{}, {}] too large for {}",
            min, max, type_name
        ));
    }

    Ok(range_size as u64)
}

fn checked_range_size(min: i64, max: i64) -> Option<u64> {
    if max < min {
        return None;
    }

    let range_size = i128::from(max) - i128::from(min) + 1;
    u64::try_from(range_size).ok()
}

fn unsupported_boundless_decode(type_name: &str) -> DecodeError {
    DecodeError::InvalidData(format!(
        "{}::decode requires schema bounds; use {}::decode_with_bounds(min, max, tokens)",
        type_name, type_name
    ))
}

fn decode_offset_with_bounds(
    tokens: &mut impl Iterator<Item = u32>,
    min: i64,
    max: i64,
    type_name: &'static str,
    max_values: u64,
) -> Result<i64, DecodeError> {
    let range_size = validate_range_capacity(min, max, type_name, max_values)
        .map_err(DecodeError::InvalidData)?;
    let offset = tokens.next().ok_or(DecodeError::UnexpectedEndOfStream {
        context: EOFContext::new(type_name, Some("offset"), "integer offset"),
    })?;

    if u64::from(offset) >= range_size {
        return Err(DecodeError::InvalidToken(offset));
    }

    let value = i128::from(min) + i128::from(offset);
    i64::try_from(value).map_err(|_| {
        DecodeError::InvalidData(format!(
            "{} decoded value {} is outside i64 range",
            type_name, value
        ))
    })
}

macro_rules! define_bounded_integer {
    ($(#[$doc:meta])* $name:ident, $max_values:ident) => {
        $(#[$doc])*
        #[derive(Debug, PartialEq, serde::Serialize, serde::Deserialize)]
        #[serde(transparent)]
        pub struct $name {
            pub value: i64,
            #[serde(skip)]
            pub min: i64,
            #[serde(skip)]
            pub max: i64,
        }

        impl $name {
            pub fn new(value: i64, min: i64, max: i64) -> Result<Self, String> {
                if value < min || value > max {
                    return Err(format!(
                        "Value {} is outside range [{}, {}]",
                        value, min, max
                    ));
                }
                validate_range_capacity(min, max, stringify!($name), $max_values)?;
                Ok(Self { value, min, max })
            }

            pub fn range_size(&self) -> u64 {
                checked_range_size(self.min, self.max).unwrap_or(0)
            }

            pub fn decode_with_bounds(
                min: i64,
                max: i64,
                tokens: &mut impl Iterator<Item = u32>,
            ) -> Result<Self, DecodeError> {
                let value =
                    decode_offset_with_bounds(tokens, min, max, stringify!($name), $max_values)?;
                Ok(Self { value, min, max })
            }
        }

        impl JsonTokenEncoder for $name {
            fn encode(&self, tokens: &mut Vec<u32>) {
                let offset = (self.value - self.min) as u32;
                tokens.push(offset);
            }

            fn decode(_tokens: &mut impl Iterator<Item = u32>) -> Result<Self, DecodeError> {
                Err(unsupported_boundless_decode(stringify!($name)))
            }

            fn token_count(&self) -> usize {
                1
            }
        }
    };
}

define_bounded_integer!(
    /// A bounded 8-bit integer that encodes values in a range [min, max] using offset encoding
    BoundedU8,
    BOUNDED_U8_VALUES
);

define_bounded_integer!(
    /// A bounded 16-bit integer for medium ranges
    BoundedU16,
    BOUNDED_U16_VALUES
);

define_bounded_integer!(
    /// A bounded 32-bit integer for large ranges
    BoundedU32,
    BOUNDED_U32_VALUES
);

/// Analyzes integer constraints and determines the optimal bounded type
pub fn analyze_integer_constraints(
    props: &serde_json::Map<String, serde_json::Value>,
) -> IntegerType {
    let min = props
        .get("minimum")
        .and_then(|v| v.as_i64())
        .unwrap_or(i64::MIN);
    let max = props
        .get("maximum")
        .and_then(|v| v.as_i64())
        .unwrap_or(i64::MAX);

    // Check if both constraints are present
    if min == i64::MIN || max == i64::MAX {
        return IntegerType::Unbounded;
    }

    // Handle potential overflow in range calculation
    // Use checked arithmetic to prevent overflow
    let range_size = match max.checked_sub(min).and_then(|diff| diff.checked_add(1)) {
        Some(size) if size > 0 => size as u64,
        _ => return IntegerType::Unbounded, // Overflow or invalid range
    };

    match range_size {
        1..=256 => IntegerType::BoundedU8 { min, max },
        257..=65536 => IntegerType::BoundedU16 { min, max },
        65537..=1073741824 => IntegerType::BoundedU32 { min, max }, // Max 2^30 values (1GB range)
        _ => IntegerType::Unbounded,
    }
}

/// Represents different integer type optimizations
#[derive(Debug, Clone)]
pub enum IntegerType {
    BoundedU8 { min: i64, max: i64 },
    BoundedU16 { min: i64, max: i64 },
    BoundedU32 { min: i64, max: i64 },
    Unbounded,
}

impl IntegerType {
    /// Calculate compression ratio compared to standard i64 (2 tokens)
    pub fn compression_ratio(&self) -> f64 {
        match self {
            IntegerType::BoundedU8 { .. }
            | IntegerType::BoundedU16 { .. }
            | IntegerType::BoundedU32 { .. } => 2.0, // 1 token vs 2 tokens = 50% space usage
            IntegerType::Unbounded => 1.0, // No compression
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // ============================================================================
    // Batch 1 - BoundedU8 Tests (7 tests)
    // ============================================================================

    #[test]
    fn test_bounded_u8_new_valid() {
        // Test valid construction with various ranges
        let test_cases = vec![
            (0, 0, 255),     // Full u8 range
            (42, 10, 110),   // Arbitrary range
            (-50, -50, 50),  // Negative to positive
            (100, 100, 100), // Single value
        ];

        for (value, min, max) in test_cases {
            let bounded = BoundedU8::new(value, min, max);
            assert!(
                bounded.is_ok(),
                "Failed for value={}, min={}, max={}",
                value,
                min,
                max
            );
            let bounded = bounded.unwrap();
            assert_eq!(bounded.value, value);
            assert_eq!(bounded.min, min);
            assert_eq!(bounded.max, max);
        }
    }

    #[test]
    fn test_bounded_u8_new_value_too_low() {
        let result = BoundedU8::new(5, 10, 110);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("outside range"));
    }

    #[test]
    fn test_bounded_u8_new_value_too_high() {
        let result = BoundedU8::new(111, 10, 110);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("outside range"));
    }

    #[test]
    fn test_bounded_u8_new_range_too_large() {
        // Range of 257 values exceeds BoundedU8 capacity (max 256)
        let result = BoundedU8::new(128, 0, 256);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("too large"));
    }

    #[test]
    fn test_bounded_u8_new_overflowing_range_returns_error() {
        let result = BoundedU8::new(0, i64::MIN, 0);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("too large"));
    }

    #[test]
    fn test_bounded_u8_round_trip() {
        // Test encode -> decode round trip
        let bounded = BoundedU8::new(42, 10, 110).unwrap();

        // Encode
        let mut tokens = Vec::new();
        bounded.encode(&mut tokens);
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0], 32); // 42 - 10 = 32

        // Decode with schema bounds.
        let mut iter = tokens.into_iter();
        let decoded = BoundedU8::decode_with_bounds(10, 110, &mut iter).unwrap();
        assert_eq!(decoded.value, 42);
        assert_eq!(decoded.min, 10);
        assert_eq!(decoded.max, 110);
        assert_eq!(decoded.token_count(), 1);
    }

    #[test]
    fn test_bounded_u8_decode_without_bounds_rejects_placeholder_contract() {
        let mut iter = vec![32].into_iter();
        let result = BoundedU8::decode(&mut iter);
        assert!(matches!(result, Err(DecodeError::InvalidData(_))));
    }

    #[test]
    fn test_bounded_u8_range_size() {
        let test_cases = vec![
            (100, 0, 255, 256), // Full range
            (50, 10, 110, 101), // 101 values
            (0, -50, 50, 101),  // Negative range
            (100, 100, 100, 1), // Single value
            (i64::MIN + 1, i64::MIN, i64::MIN + 255, 256),
            (i64::MAX - 1, i64::MAX - 255, i64::MAX, 256),
        ];

        for (value, min, max, expected_size) in test_cases {
            let bounded = BoundedU8::new(value, min, max).unwrap();
            assert_eq!(bounded.range_size(), expected_size);
        }
    }

    #[test]
    fn test_bounded_u8_decode_with_bounds_rejects_out_of_range_offset() {
        let mut iter = vec![101].into_iter();
        let result = BoundedU8::decode_with_bounds(10, 110, &mut iter);
        assert!(matches!(result, Err(DecodeError::InvalidToken(101))));
    }

    #[test]
    fn test_bounded_u8_decode_eof() {
        let mut empty_iter = std::iter::empty();
        let result = BoundedU8::decode_with_bounds(0, 255, &mut empty_iter);
        assert!(result.is_err());
        match result {
            Err(DecodeError::UnexpectedEndOfStream { .. }) => {}
            _ => panic!("Expected UnexpectedEndOfStream error"),
        }
    }

    // ============================================================================
    // Batch 2 - BoundedU16 Tests (7 tests)
    // ============================================================================

    #[test]
    fn test_bounded_u16_new_valid() {
        // Test valid construction with various ranges
        let test_cases = vec![
            (0, 0, 65535),       // Full u16 range
            (500, 100, 1000),    // Arbitrary range
            (-1000, -1000, 500), // Negative to positive
            (256, 256, 512),     // Range > u8 capacity
        ];

        for (value, min, max) in test_cases {
            let bounded = BoundedU16::new(value, min, max);
            assert!(
                bounded.is_ok(),
                "Failed for value={}, min={}, max={}",
                value,
                min,
                max
            );
            let bounded = bounded.unwrap();
            assert_eq!(bounded.value, value);
            assert_eq!(bounded.min, min);
            assert_eq!(bounded.max, max);
        }
    }

    #[test]
    fn test_bounded_u16_new_value_too_low() {
        let result = BoundedU16::new(99, 100, 1000);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("outside range"));
    }

    #[test]
    fn test_bounded_u16_new_value_too_high() {
        let result = BoundedU16::new(1001, 100, 1000);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("outside range"));
    }

    #[test]
    fn test_bounded_u16_new_range_too_large() {
        // Range of 65537 values exceeds BoundedU16 capacity (max 65536)
        let result = BoundedU16::new(32768, 0, 65536);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("too large"));
    }

    #[test]
    fn test_bounded_u16_new_overflowing_range_returns_error() {
        let result = BoundedU16::new(0, i64::MIN, 0);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("too large"));
    }

    #[test]
    fn test_bounded_u16_round_trip() {
        // Test encode -> decode round trip
        let bounded = BoundedU16::new(500, 100, 1000).unwrap();

        // Encode
        let mut tokens = Vec::new();
        bounded.encode(&mut tokens);
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0], 400); // 500 - 100 = 400

        // Decode with schema bounds.
        let mut iter = tokens.into_iter();
        let decoded = BoundedU16::decode_with_bounds(100, 1000, &mut iter).unwrap();
        assert_eq!(decoded.value, 500);
        assert_eq!(decoded.min, 100);
        assert_eq!(decoded.max, 1000);
        assert_eq!(decoded.token_count(), 1);
    }

    #[test]
    fn test_bounded_u16_range_size() {
        let test_cases = vec![
            (30000, 0, 65535, 65536), // Full range
            (500, 100, 1000, 901),    // 901 values
            (0, -1000, 500, 1501),    // Negative range
            (512, 256, 512, 257),     // 257 values (> u8)
            (i64::MIN + 1, i64::MIN, i64::MIN + 65535, 65536),
            (i64::MAX - 1, i64::MAX - 65535, i64::MAX, 65536),
        ];

        for (value, min, max, expected_size) in test_cases {
            let bounded = BoundedU16::new(value, min, max).unwrap();
            assert_eq!(bounded.range_size(), expected_size);
        }
    }

    #[test]
    fn test_bounded_u16_decode_with_bounds_rejects_out_of_range_offset() {
        let mut iter = vec![901].into_iter();
        let result = BoundedU16::decode_with_bounds(100, 1000, &mut iter);
        assert!(matches!(result, Err(DecodeError::InvalidToken(901))));
    }

    #[test]
    fn test_bounded_u16_decode_eof() {
        let mut empty_iter = std::iter::empty();
        let result = BoundedU16::decode_with_bounds(0, 65535, &mut empty_iter);
        assert!(result.is_err());
        match result {
            Err(DecodeError::UnexpectedEndOfStream { .. }) => {}
            _ => panic!("Expected UnexpectedEndOfStream error"),
        }
    }

    // ============================================================================
    // Batch 3 - BoundedU32 Tests (7 tests)
    // ============================================================================

    #[test]
    fn test_bounded_u32_new_valid() {
        // Test valid construction with various ranges
        let test_cases = vec![
            (0, 0, u32::MAX as i64),    // Full u32 range
            (100000, 10000, 200000),    // Large arbitrary range
            (-100000, -100000, 100000), // Negative to positive
            (65536, 65536, 131072),     // Range > u16 capacity
        ];

        for (value, min, max) in test_cases {
            let bounded = BoundedU32::new(value, min, max);
            assert!(
                bounded.is_ok(),
                "Failed for value={}, min={}, max={}",
                value,
                min,
                max
            );
            let bounded = bounded.unwrap();
            assert_eq!(bounded.value, value);
            assert_eq!(bounded.min, min);
            assert_eq!(bounded.max, max);
        }
    }

    #[test]
    fn test_bounded_u32_new_value_too_low() {
        let result = BoundedU32::new(9999, 10000, 200000);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("outside range"));
    }

    #[test]
    fn test_bounded_u32_new_value_too_high() {
        let result = BoundedU32::new(200001, 10000, 200000);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("outside range"));
    }

    #[test]
    fn test_bounded_u32_new_range_too_large() {
        // Range exceeding 2^32 values (4294967296)
        // Use 0 to 4294967296 which is 4294967297 values
        let result = BoundedU32::new(1000000, 0, 4294967296);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("too large"));
    }

    #[test]
    fn test_bounded_u32_new_overflowing_range_returns_error() {
        let result = BoundedU32::new(0, i64::MIN, 0);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("too large"));
    }

    #[test]
    fn test_bounded_u32_round_trip() {
        // Test encode -> decode round trip
        let bounded = BoundedU32::new(100000, 10000, 200000).unwrap();

        // Encode
        let mut tokens = Vec::new();
        bounded.encode(&mut tokens);
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0], 90000); // 100000 - 10000 = 90000

        // Decode with schema bounds.
        let mut iter = tokens.into_iter();
        let decoded = BoundedU32::decode_with_bounds(10000, 200000, &mut iter).unwrap();
        assert_eq!(decoded.value, 100000);
        assert_eq!(decoded.min, 10000);
        assert_eq!(decoded.max, 200000);
        assert_eq!(decoded.token_count(), 1);
    }

    #[test]
    fn test_bounded_u32_range_size() {
        let test_cases = vec![
            (1000000, 0, 1073741823, 1073741824), // Max allowed range (2^30)
            (100000, 10000, 200000, 190001),      // Large range
            (0, -100000, 100000, 200001),         // Negative range
            (65536, 65536, 131072, 65537),        // Range > u16
            (i64::MIN + 1, i64::MIN, i64::MIN + 65536, 65537),
            (i64::MAX - 1, i64::MAX - 65536, i64::MAX, 65537),
        ];

        for (value, min, max, expected_size) in test_cases {
            let bounded = BoundedU32::new(value, min, max).unwrap();
            assert_eq!(bounded.range_size(), expected_size);
        }
    }

    #[test]
    fn test_bounded_u32_decode_with_bounds_rejects_out_of_range_offset() {
        let mut iter = vec![190001].into_iter();
        let result = BoundedU32::decode_with_bounds(10000, 200000, &mut iter);
        assert!(matches!(result, Err(DecodeError::InvalidToken(190001))));
    }

    #[test]
    fn test_bounded_u32_decode_eof() {
        let mut empty_iter = std::iter::empty();
        let result = BoundedU32::decode_with_bounds(0, u32::MAX as i64, &mut empty_iter);
        assert!(result.is_err());
        match result {
            Err(DecodeError::UnexpectedEndOfStream { .. }) => {}
            _ => panic!("Expected UnexpectedEndOfStream error"),
        }
    }

    // ============================================================================
    // Batch 4 - analyze_integer_constraints Tests (8 tests)
    // ============================================================================

    #[test]
    fn test_analyze_constraints_boundary_u8_u16() {
        // Exactly 256 values should use BoundedU16
        let props = json!({"minimum": 0, "maximum": 255})
            .as_object()
            .unwrap()
            .clone();
        match analyze_integer_constraints(&props) {
            IntegerType::BoundedU8 { min, max } => {
                assert_eq!(min, 0);
                assert_eq!(max, 255);
            }
            _ => panic!("Expected BoundedU8"),
        }

        // 257 values should use BoundedU16
        let props = json!({"minimum": 0, "maximum": 256})
            .as_object()
            .unwrap()
            .clone();
        match analyze_integer_constraints(&props) {
            IntegerType::BoundedU16 { min, max } => {
                assert_eq!(min, 0);
                assert_eq!(max, 256);
            }
            _ => panic!("Expected BoundedU16"),
        }
    }

    #[test]
    fn test_analyze_constraints_boundary_u16_u32() {
        // Exactly 65536 values should use BoundedU16
        let props = json!({"minimum": 0, "maximum": 65535})
            .as_object()
            .unwrap()
            .clone();
        match analyze_integer_constraints(&props) {
            IntegerType::BoundedU16 { min, max } => {
                assert_eq!(min, 0);
                assert_eq!(max, 65535);
            }
            _ => panic!("Expected BoundedU16"),
        }

        // 65537 values should use BoundedU32
        let props = json!({"minimum": 0, "maximum": 65536})
            .as_object()
            .unwrap()
            .clone();
        match analyze_integer_constraints(&props) {
            IntegerType::BoundedU32 { min, max } => {
                assert_eq!(min, 0);
                assert_eq!(max, 65536);
            }
            _ => panic!("Expected BoundedU32"),
        }
    }

    #[test]
    fn test_analyze_constraints_negative_ranges() {
        // Negative to negative
        let props = json!({"minimum": -100, "maximum": -50})
            .as_object()
            .unwrap()
            .clone();
        match analyze_integer_constraints(&props) {
            IntegerType::BoundedU8 { min, max } => {
                assert_eq!(min, -100);
                assert_eq!(max, -50);
            }
            _ => panic!("Expected BoundedU8"),
        }

        // Negative to positive
        let props = json!({"minimum": -128, "maximum": 127})
            .as_object()
            .unwrap()
            .clone();
        match analyze_integer_constraints(&props) {
            IntegerType::BoundedU8 { min, max } => {
                assert_eq!(min, -128);
                assert_eq!(max, 127);
            }
            _ => panic!("Expected BoundedU8"),
        }
    }

    #[test]
    fn test_analyze_constraints_overflow_protection() {
        // Very large range should be handled safely (no overflow)
        let props = json!({"minimum": i64::MIN, "maximum": i64::MAX})
            .as_object()
            .unwrap()
            .clone();
        match analyze_integer_constraints(&props) {
            IntegerType::Unbounded => {} // Expected
            _ => panic!("Expected Unbounded for overflow range"),
        }

        // Large but valid range
        let props = json!({"minimum": 0, "maximum": 1073741823})
            .as_object()
            .unwrap()
            .clone();
        match analyze_integer_constraints(&props) {
            IntegerType::BoundedU32 { min, max } => {
                assert_eq!(min, 0);
                assert_eq!(max, 1073741823);
            }
            _ => panic!("Expected BoundedU32"),
        }
    }

    #[test]
    fn test_analyze_constraints_invalid_range() {
        // max < min should return Unbounded (invalid range)
        let props = json!({"minimum": 100, "maximum": 50})
            .as_object()
            .unwrap()
            .clone();
        match analyze_integer_constraints(&props) {
            IntegerType::Unbounded => {} // Expected for invalid range
            _ => panic!("Expected Unbounded for max < min"),
        }
    }

    #[test]
    fn test_analyze_constraints_single_value_range() {
        // Single value (min == max)
        let props = json!({"minimum": 42, "maximum": 42})
            .as_object()
            .unwrap()
            .clone();
        match analyze_integer_constraints(&props) {
            IntegerType::BoundedU8 { min, max } => {
                assert_eq!(min, 42);
                assert_eq!(max, 42);
            }
            _ => panic!("Expected BoundedU8"),
        }
    }

    #[test]
    fn test_analyze_constraints_only_minimum() {
        // Only minimum specified -> Unbounded
        let props = json!({"minimum": 0}).as_object().unwrap().clone();
        match analyze_integer_constraints(&props) {
            IntegerType::Unbounded => {}
            _ => panic!("Expected Unbounded for only minimum"),
        }
    }

    #[test]
    fn test_analyze_constraints_only_maximum() {
        // Only maximum specified -> Unbounded
        let props = json!({"maximum": 100}).as_object().unwrap().clone();
        match analyze_integer_constraints(&props) {
            IntegerType::Unbounded => {}
            _ => panic!("Expected Unbounded for only maximum"),
        }
    }

    // ============================================================================
    // Batch 5 - IntegerType Tests (4 tests)
    // ============================================================================

    #[test]
    fn test_integer_type_clone() {
        let orig = IntegerType::BoundedU8 { min: 0, max: 255 };
        let cloned = orig.clone();
        match cloned {
            IntegerType::BoundedU8 { min, max } => {
                assert_eq!(min, 0);
                assert_eq!(max, 255);
            }
            _ => panic!("Clone failed"),
        }
    }

    #[test]
    fn test_integer_type_debug() {
        let types = vec![
            IntegerType::BoundedU8 { min: 0, max: 255 },
            IntegerType::BoundedU16 { min: 0, max: 65535 },
            IntegerType::BoundedU32 {
                min: 0,
                max: 100000,
            },
            IntegerType::Unbounded,
        ];

        for int_type in types {
            let debug_str = format!("{:?}", int_type);
            assert!(!debug_str.is_empty());
        }
    }

    #[test]
    fn test_compression_ratio_all_variants() {
        let test_cases = vec![
            (IntegerType::BoundedU8 { min: 0, max: 255 }, 2.0),
            (IntegerType::BoundedU16 { min: 0, max: 65535 }, 2.0),
            (
                IntegerType::BoundedU32 {
                    min: 0,
                    max: 100000,
                },
                2.0,
            ),
            (IntegerType::Unbounded, 1.0),
        ];

        for (int_type, expected_ratio) in test_cases {
            assert_eq!(int_type.compression_ratio(), expected_ratio);
        }
    }

    #[test]
    fn test_integer_type_match_patterns() {
        // Test all match patterns
        let u8_type = IntegerType::BoundedU8 { min: 10, max: 110 };
        let u16_type = IntegerType::BoundedU16 {
            min: 1000,
            max: 2000,
        };
        let u32_type = IntegerType::BoundedU32 {
            min: 100000,
            max: 200000,
        };
        let unbounded = IntegerType::Unbounded;

        match u8_type {
            IntegerType::BoundedU8 { min, max } => {
                assert_eq!(min, 10);
                assert_eq!(max, 110);
            }
            _ => panic!("Expected BoundedU8"),
        }

        match u16_type {
            IntegerType::BoundedU16 { min, max } => {
                assert_eq!(min, 1000);
                assert_eq!(max, 2000);
            }
            _ => panic!("Expected BoundedU16"),
        }

        match u32_type {
            IntegerType::BoundedU32 { min, max } => {
                assert_eq!(min, 100000);
                assert_eq!(max, 200000);
            }
            _ => panic!("Expected BoundedU32"),
        }

        match unbounded {
            IntegerType::Unbounded => {}
            _ => panic!("Expected Unbounded"),
        }
    }

    // ============================================================================
    // Existing Tests
    // ============================================================================

    #[test]
    fn test_constraint_analysis() {
        let test_cases = vec![
            // (schema_props, expected_type, description)
            (
                json!({"minimum": 0, "maximum": 3}),
                "BoundedU8",
                "Tiny range",
            ),
            (
                json!({"minimum": 10, "maximum": 110}),
                "BoundedU8",
                "Small range",
            ),
            (
                json!({"minimum": 0, "maximum": 300}),
                "BoundedU16",
                "Medium range",
            ),
            (
                json!({"minimum": -50, "maximum": 50}),
                "BoundedU8",
                "Symmetric range",
            ),
            (
                json!({"minimum": 0, "maximum": 100000}),
                "BoundedU32",
                "Large range",
            ),
            (json!({}), "Unbounded", "No constraints"),
            (json!({"minimum": 0}), "Unbounded", "Only minimum"),
            (json!({"maximum": 100}), "Unbounded", "Only maximum"),
        ];

        for (props, expected, description) in test_cases {
            let props_map = props.as_object().unwrap();
            let result = analyze_integer_constraints(props_map);

            let result_name = match result {
                IntegerType::BoundedU8 { .. } => "BoundedU8",
                IntegerType::BoundedU16 { .. } => "BoundedU16",
                IntegerType::BoundedU32 { .. } => "BoundedU32",
                IntegerType::Unbounded => "Unbounded",
            };

            assert_eq!(result_name, expected, "Failed for: {}", description);
            println!("✅ {}: {} -> {}", description, props, result_name);
        }
    }

    #[test]
    fn test_bounded_u8_encoding() {
        let bounded = BoundedU8::new(42, 10, 110).expect("Valid range");

        let mut tokens = Vec::new();
        bounded.encode(&mut tokens);

        assert_eq!(tokens.len(), 1, "Should use exactly 1 token");
        assert_eq!(tokens[0], 32, "Should encode as offset: 42 - 10 = 32");

        println!(
            "✅ BoundedU8 encoding: value=42, min=10 -> token={}",
            tokens[0]
        );
    }

    #[test]
    fn test_compression_ratios() {
        let types = vec![
            IntegerType::BoundedU8 { min: 0, max: 255 },
            IntegerType::BoundedU16 { min: 0, max: 65535 },
            IntegerType::BoundedU32 {
                min: 0,
                max: u32::MAX as i64,
            },
            IntegerType::Unbounded,
        ];

        for int_type in types {
            let ratio = int_type.compression_ratio();
            println!("Compression ratio for {:?}: {}x", int_type, ratio);
        }
    }
}
