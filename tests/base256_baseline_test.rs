//! Baseline tests for base-256 encoding across all code paths
//!
//! These tests establish the current behavior of base-256 encoding BEFORE refactoring.
//! After refactoring to use shared `src/base256.rs`, these tests MUST continue to pass
//! with identical token outputs.
//!
//! Code paths tested:
//! 1. Runtime primitives: i32, i64, f64 (src/tokenizer/primitives.rs)
//! 2. Runtime collections: Vec, HashMap lengths (src/tokenizer/primitives.rs)
//! 3. Generated inline code: i32 in generated structs (src/grammar.rs)
//! 4. Generated structural codecs: bounded integers, enums (src/generator/structural_codec.rs)

use tct::tokenizer::{DEFAULT_MAX_COLLECTION_LENGTH, JsonTokenEncoder};

/// Test i32 encoding (runtime primitive)
/// Location: src/tokenizer/primitives.rs:38-82
#[test]
fn test_i32_encode_decode_base256() {
    let test_values = vec![
        0i32,     // 1 digit
        255,      // 1 digit (max single)
        256,      // 2 digits
        -1,       // Large value as u32
        i32::MIN, // Minimum i32
        i32::MAX, // Maximum i32
    ];

    for value in test_values {
        let mut tokens = Vec::new();
        value.encode(&mut tokens);

        let mut iter = tokens.clone().into_iter();
        let decoded = i32::decode(&mut iter).expect("decode should succeed");

        assert_eq!(value, decoded, "i32 round-trip failed for {}", value);

        // Verify all tokens are ≤255 (vocabulary constraint)
        for (i, token) in tokens.iter().enumerate() {
            assert!(
                *token <= 255,
                "Token {} at index {} exceeds 255 for value {}",
                token,
                i,
                value
            );
        }
    }
}

/// Test i32 token count calculation
/// Location: src/tokenizer/primitives.rs:84-98
#[test]
fn test_i32_token_count() {
    let test_cases = vec![
        (0i32, 2),     // width + 1 digit
        (255, 2),      // width + 1 digit
        (256, 3),      // width + 2 digits
        (-1, 5),       // Large value: width + 4 digits
        (i32::MAX, 5), // width + 4 digits
    ];

    for (value, expected_count) in test_cases {
        let count = value.token_count();
        assert_eq!(
            count, expected_count,
            "i32 token count mismatch for {}: expected {}, got {}",
            value, expected_count, count
        );
    }
}

/// Test i64 encoding (runtime primitive)
/// Location: src/tokenizer/primitives.rs:100-158
#[test]
fn test_i64_encode_decode_base256() {
    let test_values = vec![0i64, 255, 256, 65535, 65536, -1i64, i64::MIN, i64::MAX];

    for value in test_values {
        let mut tokens = Vec::new();
        value.encode(&mut tokens);

        let mut iter = tokens.clone().into_iter();
        let decoded = i64::decode(&mut iter).expect("decode should succeed");

        assert_eq!(value, decoded, "i64 round-trip failed for {}", value);

        // Verify all tokens are ≤255 (vocabulary constraint)
        for (i, token) in tokens.iter().enumerate() {
            assert!(
                *token <= 255,
                "Token {} at index {} exceeds 255 for value {}",
                token,
                i,
                value
            );
        }
    }
}

/// Test f64 encoding (runtime primitive)
/// Location: src/tokenizer/primitives.rs:160-240
#[test]
fn test_f64_encode_decode_base256() {
    let test_values = vec![
        0.0f64,
        1.0,
        -1.0,
        std::f64::consts::PI,
        f64::MIN,
        f64::MAX,
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
    ];

    for value in test_values {
        let mut tokens = Vec::new();
        value.encode(&mut tokens);

        let mut iter = tokens.clone().into_iter();
        let decoded = f64::decode(&mut iter).expect("decode should succeed");

        // NaN != NaN, so handle specially
        if value.is_nan() {
            assert!(decoded.is_nan(), "f64 NaN round-trip failed");
        } else {
            assert_eq!(value, decoded, "f64 round-trip failed for {}", value);
        }

        // Verify all tokens are ≤255 (vocabulary constraint)
        for (i, token) in tokens.iter().enumerate() {
            assert!(
                *token <= 255,
                "Token {} at index {} exceeds 255 for value {}",
                token,
                i,
                value
            );
        }
    }
}

/// Test Vec<T> length encoding (runtime collection)
/// Location: src/tokenizer/primitives.rs:363-410
#[test]
fn test_vec_length_encode_decode_base256() {
    let test_lengths = vec![
        0,                             // Empty vec
        1,                             // Single element
        255,                           // 1 digit max
        256,                           // 2 digits
        1000,                          // 2 digits
        DEFAULT_MAX_COLLECTION_LENGTH, // maximum fallback collection decode length
    ];

    for length in test_lengths {
        // Create vec with specified length
        let vec: Vec<i32> = (0..length as i32).collect();

        let mut tokens = Vec::new();
        vec.encode(&mut tokens);

        let mut iter = tokens.clone().into_iter();
        let decoded = Vec::<i32>::decode(&mut iter).expect("decode should succeed");

        assert_eq!(vec, decoded, "Vec round-trip failed for length {}", length);

        // Verify all tokens are ≤255 (vocabulary constraint)
        for (i, token) in tokens.iter().enumerate() {
            assert!(
                *token <= 255,
                "Token {} at index {} exceeds 255 for vec length {}",
                token,
                i,
                length
            );
        }
    }

    for length in [65535u32, 65536] {
        let mut tokens = Vec::new();
        tct::base256::encode_u32(length, &mut tokens, tct::base256::DEFAULT_BASE);
        let mut iter = tokens.clone().into_iter();
        let decoded = tct::base256::decode_u32(&mut iter, "Vec<T>", tct::base256::DEFAULT_BASE)
            .expect("base256 length decode should succeed");

        assert_eq!(
            length, decoded,
            "base256 length round-trip failed for {}",
            length
        );

        for (i, token) in tokens.iter().enumerate() {
            assert!(
                *token <= 255,
                "Token {} at index {} exceeds 255 for vec length {}",
                token,
                i,
                length
            );
        }
    }
}

/// Test FxHashMap<K,V> length encoding (runtime collection)
/// Location: src/tokenizer/primitives.rs:438-486
/// Note: FxHashMap encoding includes length prefix which uses base-256
#[test]
fn test_hashmap_length_encode_decode_base256() {
    use rustc_hash::FxHashMap;
    use tct::tokenizer::JsonTokenEncoder;

    // Test different FxHashMap lengths to verify base-256 length encoding
    let test_cases = vec![
        (0usize, vec![0, 0]), // Empty: length=0 → [width=0, value=0]
        (1, vec![0, 1]),      // Single: length=1 → [width=0, value=1]
        (255, vec![0, 255]),  // Max 1-digit: [width=0, value=255]
        (256, vec![1, 1, 0]), // 2-digit boundary: [width=1, byte1=1, byte0=0]
    ];

    for (length, expected_length_tokens) in test_cases {
        // Create hashmap with specified length
        let mut map = FxHashMap::default();
        for i in 0..length {
            map.insert(format!("key{}", i), i as i32);
        }

        let mut tokens = Vec::new();
        map.encode(&mut tokens);

        // Verify length prefix matches expected base-256 encoding
        assert!(
            tokens.len() >= expected_length_tokens.len(),
            "Tokens should start with length prefix"
        );
        for (i, expected_token) in expected_length_tokens.iter().enumerate() {
            assert_eq!(
                tokens[i], *expected_token,
                "Length prefix token {} mismatch for length {}",
                i, length
            );
        }

        // Verify length prefix tokens are ≤255 (vocabulary constraint for base structural tokens)
        // Note: String keys and values may use BPE tokens >256, which is intentional
        for (i, token) in tokens[..expected_length_tokens.len()].iter().enumerate() {
            assert!(
                *token <= 255,
                "Length prefix token {} at index {} exceeds 255 for hashmap length {}",
                token,
                i,
                length
            );
        }
    }
}

// NOTE: Tests for generated code (grammar.rs inline encoding and structural_codec.rs)
// are covered by integration tests. This baseline test focuses on runtime primitives
// that will be refactored to use the shared base256 module.

/// Test edge case: Maximum u32 value
#[test]
fn test_u32_max_encoding() {
    // u32::MAX requires 4 digits in base-256
    let value = u32::MAX as i32; // -1 when interpreted as i32

    let mut tokens = Vec::new();
    value.encode(&mut tokens);

    // Should be: [width_indicator=3, digit3, digit2, digit1, digit0]
    assert_eq!(
        tokens.len(),
        5,
        "u32::MAX should encode to 5 tokens (width + 4 digits)"
    );
    assert_eq!(tokens[0], 3, "Width indicator should be 3 (for 4 digits)");

    // Verify all tokens are ≤255
    for token in &tokens {
        assert!(*token <= 255, "Token {} exceeds 255", token);
    }

    // Verify round-trip
    let mut iter = tokens.into_iter();
    let decoded = i32::decode(&mut iter).expect("decode should succeed");
    assert_eq!(value, decoded);
}

/// Test edge case: Boundary between 1 and 2 digits (255 vs 256)
#[test]
fn test_boundary_255_256() {
    let values = vec![254i32, 255, 256, 257];

    for value in values {
        let mut tokens = Vec::new();
        value.encode(&mut tokens);

        // 254, 255 → 2 tokens (width=0, digit)
        // 256, 257 → 3 tokens (width=1, digit1, digit0)
        let expected_len = if value <= 255 { 2 } else { 3 };
        assert_eq!(
            tokens.len(),
            expected_len,
            "Value {} should encode to {} tokens, got {}",
            value,
            expected_len,
            tokens.len()
        );

        // Verify round-trip
        let mut iter = tokens.into_iter();
        let decoded = i32::decode(&mut iter).expect("decode should succeed");
        assert_eq!(value, decoded);
    }
}

/// Test edge case: Boundary between 2 and 3 digits (65535 vs 65536)
#[test]
fn test_boundary_65535_65536() {
    let values = vec![65534i32, 65535, 65536, 65537];

    for value in values {
        let mut tokens = Vec::new();
        value.encode(&mut tokens);

        // 65534, 65535 → 3 tokens (width=1, digit1, digit0)
        // 65536, 65537 → 4 tokens (width=2, digit2, digit1, digit0)
        let expected_len = if value <= 65535 { 3 } else { 4 };
        assert_eq!(
            tokens.len(),
            expected_len,
            "Value {} should encode to {} tokens, got {}",
            value,
            expected_len,
            tokens.len()
        );

        // Verify round-trip
        let mut iter = tokens.into_iter();
        let decoded = i32::decode(&mut iter).expect("decode should succeed");
        assert_eq!(value, decoded);
    }
}

/// Test all tokens are within vocabulary constraint
#[test]
fn test_vocabulary_constraint_enforcement() {
    // Test a wide range of values to ensure NO token exceeds 255
    let test_values: Vec<i32> = vec![
        0,
        1,
        127,
        128,
        254,
        255,
        256,
        257,
        1000,
        10000,
        65535,
        65536,
        100000,
        i32::MAX,
        i32::MIN,
        -1,
        -1000,
    ];

    for value in test_values {
        let mut tokens = Vec::new();
        value.encode(&mut tokens);

        for (i, token) in tokens.iter().enumerate() {
            assert!(
                *token <= 255,
                "VOCABULARY CONSTRAINT VIOLATED: Token {} at index {} exceeds 255 for value {}",
                token,
                i,
                value
            );
        }
    }
}

/// Baseline performance benchmark (for regression detection)
#[test]
fn test_encoding_performance_baseline() {
    use std::time::Instant;

    let iterations = 10_000;
    let test_value = 1_000_000i32;

    let start = Instant::now();
    for _ in 0..iterations {
        let mut tokens = Vec::new();
        test_value.encode(&mut tokens);
        let mut iter = tokens.into_iter();
        let _ = i32::decode(&mut iter).expect("decode should succeed");
    }
    let elapsed = start.elapsed();

    println!(
        "Baseline: {} encode/decode cycles in {:?} ({:.2} ns/cycle)",
        iterations,
        elapsed,
        elapsed.as_nanos() as f64 / iterations as f64
    );

    // No assertion - just establishing baseline for future comparison
}
