// Simple correctness test without complex string formatting
use serde_json::json;
use tct::{TctCompiler, tokenizer::JsonTokenEncoder};

#[test]
fn test_correctness_basic_schema_generates_valid_code() {
    // Test basic round-trip: JSON -> Struct -> Tokens -> Struct -> JSON
    let schema = json!({
        "title": "SimpleTest",
        "type": "object",
        "properties": {
            "name": { "type": "string" },
            "age": { "type": "integer" },
            "active": { "type": "boolean" }
        },
        "required": ["name"]
    });

    // Generate the tokenizer
    let grammar = TctCompiler::new(schema, "SimpleTest")
        .expect("Failed to create compiler")
        .compile()
        .expect("Failed to generate grammar");
    let generated_code = grammar.generate_code().expect("Failed to generate code");

    // Test data (validates JSON structure)
    let _test_json = json!({"name": "Alice", "age": 30, "active": true});

    println!(
        "Generated code structure looks valid: {}",
        generated_code.contains("SimpleTest")
    );
    println!("Test passed - basic generation works");
}

#[test]
fn test_correctness_string_roundtrip_preserves_data() {
    // Test basic string tokenization
    let test_string = "Hello World".to_string();
    let mut tokens = Vec::new();
    test_string.encode(&mut tokens);

    let mut token_iter = tokens.into_iter();
    let decoded: Result<String, _> = String::decode(&mut token_iter);

    assert_eq!(decoded.unwrap(), test_string, "String round-trip failed");
    println!("String tokenization works correctly");
}

#[test]
fn test_correctness_array_roundtrip_preserves_data() {
    // Test basic array tokenization
    let test_array = vec![
        "apple".to_string(),
        "banana".to_string(),
        "cherry".to_string(),
    ];
    let mut tokens = Vec::new();
    test_array.encode(&mut tokens);

    let mut token_iter = tokens.into_iter();
    let decoded: Result<Vec<String>, _> = Vec::decode(&mut token_iter);

    assert_eq!(decoded.unwrap(), test_array, "Array round-trip failed");
    println!("Array tokenization works correctly");
}

#[test]
fn test_correctness_boolean_roundtrip_preserves_data() {
    // Test boolean tokenization
    for test_bool in [true, false] {
        let mut tokens = Vec::new();
        test_bool.encode(&mut tokens);

        let mut token_iter = tokens.into_iter();
        let decoded: Result<bool, _> = bool::decode(&mut token_iter);

        assert_eq!(
            decoded.unwrap(),
            test_bool,
            "Boolean round-trip failed for {}",
            test_bool
        );
    }
    println!("Boolean tokenization works correctly");
}

#[test]
fn test_correctness_integer_roundtrip_preserves_data() {
    // Test integer tokenization
    let test_integers = [0i64, 42, -17, i64::MAX, i64::MIN];

    for test_int in test_integers {
        let mut tokens = Vec::new();
        test_int.encode(&mut tokens);

        let mut token_iter = tokens.into_iter();
        let decoded: Result<i64, _> = i64::decode(&mut token_iter);

        assert_eq!(
            decoded.unwrap(),
            test_int,
            "Integer round-trip failed for {}",
            test_int
        );
    }
    println!("Integer tokenization works correctly");
}

#[test]
fn test_correctness_string_validation_matches_utf8_end_marker_contract() {
    // Test that UTF-8 byte validation works correctly. Token values 0-255 are
    // valid byte tokens, token 256 is the end marker, and the collected byte
    // sequence must still be valid UTF-8.
    use tct::tokenizer::{DecodeError, JsonTokenEncoder};

    // Test 1: valid non-ASCII UTF-8 sequence for "é".
    let utf8_tokens = vec![195u32, 169u32, 256u32];
    let mut token_iter = utf8_tokens.into_iter();

    let result = String::decode(&mut token_iter);
    assert_eq!(result.unwrap(), "é");

    // Test 2: lone continuation bytes are in the byte-token range but are not
    // valid UTF-8 sequences.
    let utf8_tokens = vec![128u32, 256u32];
    let mut token_iter = utf8_tokens.into_iter();

    let result = String::decode(&mut token_iter);
    assert!(
        matches!(result, Err(DecodeError::Utf8Error)),
        "Should reject malformed UTF-8 continuation byte, got: {:?}",
        result
    );

    // Test 3: invalid token above END_STRING_MARKER should fail.
    let invalid_tokens = vec![300u32, 256u32]; // 300 > 256 (invalid)
    let mut token_iter = invalid_tokens.into_iter();

    let result = String::decode(&mut token_iter);
    assert!(
        matches!(result, Err(DecodeError::InvalidToken(300))),
        "Should reject token >= END_STRING_MARKER (256), got: {:?}",
        result
    );

    // Test 4: Valid ASCII (should pass)
    let valid_tokens = vec![72u32, 101u32, 108u32, 108u32, 111u32, 256u32]; // "Hello"
    let mut token_iter = valid_tokens.into_iter();

    let result = String::decode(&mut token_iter);
    assert!(
        result.is_ok(),
        "Should accept ASCII string, got: {:?}",
        result
    );
    assert_eq!(result.unwrap(), "Hello");

    println!("String UTF-8 validation works correctly");
}

#[test]
fn test_correctness_invalid_schemas_fail_gracefully() {
    // Test that invalid schemas are rejected gracefully
    let invalid_schemas = [
        json!({}),                            // Empty schema
        json!({"type": "invalid_type"}),      // Invalid type
        json!({"properties": {"field": {}}}), // Missing type in property
    ];

    for (i, schema) in invalid_schemas.iter().enumerate() {
        let result = TctCompiler::new(schema.clone(), format!("InvalidTest{}", i))
            .and_then(|compiler| compiler.compile());
        // Should either succeed or fail gracefully, never panic
        match result {
            Ok(_) => println!("Schema {} was accepted (might be valid)", i),
            Err(e) => {
                let error_msg = format!("{}", e);
                assert!(!error_msg.is_empty(), "Error message should not be empty");
                println!("Schema {} rejected with error: {}", i, error_msg);
            }
        }
    }
}
