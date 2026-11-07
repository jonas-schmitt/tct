//! End-Marker String Encoding Tests
//!
//! Tests the new end-marker string encoding that replaces length-prefix encoding.
//! This encoding is more LLM-friendly as the model can generate strings of any length
//! and naturally learn when to emit the END_STRING_MARKER token.
//!
//! **Old encoding (length-prefix):**
//! ```
//! [length, byte1, byte2, ..., byteN]
//! ```
//!
//! **New encoding (end-marker):**
//! ```
//! [byte1, byte2, ..., byteN, END_STRING_MARKER]
//! ```

use tct::tokenizer::JsonTokenEncoder;

/// Test basic end-marker string encoding for short strings
#[test]
fn test_end_marker_short_string() {
    let s = String::from("hello");
    let mut tokens = Vec::new();
    s.encode(&mut tokens);

    // Expected: ['h', 'e', 'l', 'l', 'o', END_MARKER]
    // ASCII: h=104, e=101, l=108, l=108, o=111
    // END_MARKER = 256 (first token after UTF-8 byte range 0-255)
    assert_eq!(tokens.len(), 6, "Should have 5 bytes + 1 end marker");
    assert_eq!(tokens[0], 104); // 'h'
    assert_eq!(tokens[1], 101); // 'e'
    assert_eq!(tokens[2], 108); // 'l'
    assert_eq!(tokens[3], 108); // 'l'
    assert_eq!(tokens[4], 111); // 'o'
    assert_eq!(tokens[5], 256); // END_STRING_MARKER (token 256)

    // Test round-trip
    let mut token_iter = tokens.into_iter();
    let decoded = String::decode(&mut token_iter).expect("Should decode successfully");
    assert_eq!(decoded, "hello");
}

/// Test end-marker encoding for empty string
#[test]
fn test_end_marker_empty_string() {
    let s = String::from("");
    let mut tokens = Vec::new();
    s.encode(&mut tokens);

    // Expected: [END_MARKER] only
    assert_eq!(tokens.len(), 1, "Empty string should just be END_MARKER");
    assert_eq!(tokens[0], 256); // END_STRING_MARKER

    // Test round-trip
    let mut token_iter = tokens.into_iter();
    let decoded = String::decode(&mut token_iter).expect("Should decode successfully");
    assert_eq!(decoded, "");
}

/// Test end-marker encoding for large strings (>512 chars)
#[test]
fn test_end_marker_large_string() {
    // Create a 10KB string
    let s = "A".repeat(10000);
    let mut tokens = Vec::new();
    s.encode(&mut tokens);

    // Expected: 10000 bytes + 1 end marker = 10001 tokens
    assert_eq!(
        tokens.len(),
        10001,
        "Should have 10000 bytes + 1 end marker"
    );

    // Verify all bytes are 'A' (ASCII 65) except last token
    for (i, token) in tokens.iter().take(10000).enumerate() {
        assert_eq!(*token, 65, "Byte {} should be 'A'", i);
    }
    assert_eq!(tokens[10000], 256); // END_STRING_MARKER

    // Test round-trip
    let mut token_iter = tokens.into_iter();
    let decoded = String::decode(&mut token_iter).expect("Should decode successfully");
    assert_eq!(decoded, s);
}

/// Test that decoder stops at END_MARKER even if more tokens available
#[test]
fn test_end_marker_stops_correctly() {
    // Tokens: "hi" + END_MARKER + extra tokens
    let tokens = vec![
        104, // 'h'
        105, // 'i'
        256, // END_STRING_MARKER
        999, // Extra token (should not be consumed)
        888, // Extra token (should not be consumed)
    ];

    let mut token_iter = tokens.into_iter();
    let decoded = String::decode(&mut token_iter).expect("Should decode successfully");
    assert_eq!(decoded, "hi");

    // Verify extra tokens remain in iterator
    assert_eq!(token_iter.next(), Some(999));
    assert_eq!(token_iter.next(), Some(888));
}

/// Test that decoder handles missing END_MARKER gracefully
#[test]
fn test_end_marker_missing_error() {
    // Tokens without END_MARKER
    let tokens = vec![104, 105]; // Just "hi" without terminator

    let mut token_iter = tokens.into_iter();
    let result = String::decode(&mut token_iter);

    // Should get UnexpectedEndOfStream error
    assert!(result.is_err(), "Should error when END_MARKER missing");
}

/// Test end-marker with special characters
#[test]
fn test_end_marker_special_chars() {
    let s = String::from("Hello, World! 123");
    let mut tokens = Vec::new();
    s.encode(&mut tokens);

    // Should have all ASCII chars + END_MARKER
    assert_eq!(tokens.len(), 18, "Should have 17 chars + 1 end marker");
    assert_eq!(tokens.last(), Some(&256)); // END_STRING_MARKER

    // Test round-trip
    let mut token_iter = tokens.into_iter();
    let decoded = String::decode(&mut token_iter).expect("Should decode successfully");
    assert_eq!(decoded, "Hello, World! 123");
}
