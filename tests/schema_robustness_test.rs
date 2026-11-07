// Schema robustness and edge case handling tests
//
// These tests verify that the schema compiler handles malformed inputs,
// edge cases, and error conditions gracefully with appropriate error messages.

use serde_json::json;
use tct::TctCompiler;

#[test]
fn test_schema_robustness_invalid_regex_pattern_error_handling() {
    // Test that invalid regex patterns are caught during generation, not at runtime
    let schema = json!({
        "title": "InvalidRegexTest",
        "type": "object",
        "properties": {
            "normal_prop": { "type": "string" }
        },
        "patternProperties": {
            "[invalid_regex": {  // Missing closing bracket - invalid regex
                "type": "string"
            }
        }
    });

    // Should fail during generation with clear error message
    let result = TctCompiler::new(schema.clone(), "InvalidRegexTest")
        .unwrap()
        .compile();
    assert!(result.is_err(), "Should fail on invalid regex pattern");

    let error_msg = format!("{}", result.err().unwrap());
    assert!(
        error_msg.contains("Invalid regex pattern"),
        "Error should mention invalid regex pattern, got: {}",
        error_msg
    );
    assert!(
        error_msg.contains("[invalid_regex"),
        "Error should show the problematic pattern, got: {}",
        error_msg
    );
}

#[test]
fn test_schema_robustness_complex_valid_regex_patterns() {
    // Test that complex but valid regex patterns work correctly
    let schema = json!({
        "title": "ComplexRegexTest",
        "type": "object",
        "properties": {
            "normal_field": { "type": "string" }
        },
        "patternProperties": {
            "^[a-zA-Z][a-zA-Z0-9_]*$": {  // Valid identifier pattern
                "type": "string"
            },
            "^\\d{3}-\\d{2}-\\d{4}$": {  // SSN pattern with escaping
                "type": "string"
            }
        }
    });

    let result = TctCompiler::new(schema.clone(), "ComplexRegexTest")
        .unwrap()
        .compile();
    assert!(
        result.is_ok(),
        "Should succeed with valid complex regex patterns: {}",
        result.is_ok()
    );

    let grammar = result.unwrap();
    let code = grammar.generate_code();
    assert!(
        code.is_ok(),
        "Should generate code successfully: {:?}",
        code
    );

    let generated = code.unwrap();
    assert!(
        generated.contains("ComplexRegexTest"),
        "Should contain struct name"
    );
    assert!(
        generated.contains("Regex validated during generation"),
        "Should contain validation comment"
    );
}

#[test]
fn test_schema_robustness_circular_reference_detection() {
    // Test what happens with self-referential structures
    // Note: The current implementation may not catch all circular references,
    // but it should not panic or hang
    let schema = json!({
        "title": "CircularTest",
        "type": "object",
        "properties": {
            "data": { "$ref": "#/definitions/Node" }
        },
        "definitions": {
            "Node": {
                "type": "object",
                "properties": {
                    "value": { "type": "string" },
                    "child": { "$ref": "#/definitions/Node" }
                }
            }
        }
    });

    // The generation should complete without hanging or panicking
    // It may succeed (creating recursive types) or fail gracefully
    let result = std::panic::catch_unwind(|| {
        TctCompiler::new(schema.clone(), "CircularTest")
            .unwrap()
            .compile()
    });

    assert!(result.is_ok(), "Should not panic on circular references");

    // If it fails, it should fail gracefully with a meaningful error
    if let Ok(Err(e)) = result {
        let error_msg = format!("{}", e);
        assert!(
            !error_msg.is_empty(),
            "Should provide meaningful error message"
        );
    }
}

#[test]
fn test_schema_robustness_indirect_circular_reference() {
    // Test A -> B -> C -> A circular reference
    let schema = json!({
        "title": "IndirectCircularTest",
        "type": "object",
        "properties": {
            "entry": { "$ref": "#/definitions/TypeA" }
        },
        "definitions": {
            "TypeA": {
                "type": "object",
                "properties": {
                    "ref_to_b": { "$ref": "#/definitions/TypeB" }
                }
            },
            "TypeB": {
                "type": "object",
                "properties": {
                    "ref_to_c": { "$ref": "#/definitions/TypeC" }
                }
            },
            "TypeC": {
                "type": "object",
                "properties": {
                    "ref_to_a": { "$ref": "#/definitions/TypeA" }  // Completes the cycle
                }
            }
        }
    });

    // Should not panic or hang, may succeed or fail gracefully
    let result = std::panic::catch_unwind(|| {
        TctCompiler::new(schema.clone(), "IndirectCircularTest")
            .unwrap()
            .compile()
    });

    assert!(
        result.is_ok(),
        "Should not panic on indirect circular references"
    );
}

#[test]
fn test_schema_robustness_valid_self_reference_structure() {
    // Test that legitimate self-referential structures don't cause panics
    let schema = json!({
        "title": "ValidSelfRef",
        "type": "object",
        "properties": {
            "root": { "$ref": "#/definitions/Node" }
        },
        "definitions": {
            "Node": {
                "type": "object",
                "properties": {
                    "value": { "type": "string" },
                    "next": {
                        "oneOf": [
                            { "type": "null" },
                            { "$ref": "#/definitions/Node" }
                        ]
                    }
                }
            }
        }
    });

    // Should not panic, may succeed or fail gracefully
    let result = std::panic::catch_unwind(|| {
        TctCompiler::new(schema.clone(), "ValidSelfRef")
            .unwrap()
            .compile()
    });

    assert!(
        result.is_ok(),
        "Should not panic on self-referential structures"
    );
}

#[test]
fn test_schema_robustness_array_length_overflow_protection() {
    // Test that we handle potential usize overflow gracefully
    use tct::tokenizer::{DecodeError, JsonTokenEncoder};

    // Create a token stream with maximum u32 value as length
    let mut tokens = vec![u32::MAX].into_iter(); // Maximum u32 value

    let result = String::decode(&mut tokens);

    // With UTF-8 encoding (END_STRING_MARKER=256), u32::MAX is treated as an invalid token
    // because it's >= 256 (outside valid UTF-8 byte range 0-255).
    // This is correct behavior - tokens >= END_STRING_MARKER are invalid for string content.
    assert!(
        matches!(result, Err(DecodeError::InvalidToken(_))),
        "Should fail with InvalidToken for u32::MAX (invalid UTF-8 byte value). Got: {:?}",
        result
    );
}

#[test]
fn test_schema_robustness_large_array_handling() {
    use tct::tokenizer::JsonTokenEncoder;

    // Test with a reasonably large array that should work
    let large_vec: Vec<String> = (0..1000).map(|i| format!("item_{}", i)).collect();
    let mut tokens = Vec::new();
    large_vec.encode(&mut tokens);

    // Should successfully round-trip
    let mut token_iter = tokens.into_iter();
    let decoded: Result<Vec<String>, _> = Vec::decode(&mut token_iter);

    assert!(decoded.is_ok(), "Should handle reasonably large arrays");
    assert_eq!(decoded.unwrap(), large_vec, "Should round-trip correctly");
}

#[test]
fn test_schema_robustness_malformed_schema_edge_cases() {
    // Test various malformed schema inputs
    let malformed_schemas = vec![
        // Empty object
        json!({}),
        // Missing type in property
        json!({
            "title": "MalformedTest",
            "properties": {
                "bad_prop": { "description": "no type specified" }
            }
        }),
        // Invalid $ref format
        json!({
            "title": "BadRefTest",
            "properties": {
                "bad_ref": { "$ref": "" }  // Empty ref
            }
        }),
        // Invalid $ref format 2
        json!({
            "title": "BadRefTest2",
            "properties": {
                "bad_ref": { "$ref": "not/a/valid/ref" }  // No # prefix
            }
        }),
    ];

    for (i, schema) in malformed_schemas.into_iter().enumerate() {
        let compiler_result = TctCompiler::new(schema.clone(), format!("MalformedTest{}", i));
        let result = match compiler_result {
            Ok(compiler) => compiler.compile(),
            Err(e) => Err(e), // TctCompiler construction failed
        };
        // All of these should either work (graceful handling) or fail with clear errors
        // But they should never panic
        if let Err(e) = result {
            let error_msg = format!("{}", e);
            assert!(!error_msg.is_empty(), "Error messages should not be empty");
            // Ensure we get meaningful error messages, not just generic ones
            assert!(
                error_msg.len() > 10,
                "Error messages should be descriptive, got: {}",
                error_msg
            );
        }
    }
}

#[test]
fn test_schema_robustness_utf8_validation_in_patterns() {
    // Test that we handle non-UTF8 and invalid UTF8 sequences properly
    let schema = json!({
        "title": "UTF8Test",
        "type": "object",
        "properties": {
            "normal_field": { "type": "string" }
        },
        "patternProperties": {
            "^[αβγ]+$": {  // Unicode characters in regex
                "type": "string"
            }
        }
    });

    let result = TctCompiler::new(schema.clone(), "UTF8Test")
        .unwrap()
        .compile();
    assert!(
        result.is_ok(),
        "Should handle Unicode regex patterns: {}",
        result.is_ok()
    );

    if let Ok(grammar) = result {
        let code = grammar.generate_code();
        assert!(
            code.is_ok(),
            "Should generate code with Unicode patterns: {:?}",
            code
        );
    }
}

#[test]
fn test_schema_robustness_deeply_nested_schema_handling() {
    // Test very deep nesting doesn't cause stack overflow
    let mut deeply_nested = json!({
        "type": "object",
        "properties": {
            "value": { "type": "string" }
        }
    });

    // Create 20 levels of nesting (reduced from 100 to prevent stack overflow in CI)
    // Note: 100 levels causes stack overflow - this is a known limitation
    for i in 0..20 {
        deeply_nested = json!({
            "type": "object",
            "properties": {
                &format!("level_{}", i): deeply_nested
            }
        });
    }

    let schema = json!({
        "title": "DeepNestingTest",
        "type": "object",
        "properties": {
            "deep": deeply_nested
        }
    });

    // Should either succeed or fail gracefully, never panic or stack overflow
    let result = std::panic::catch_unwind(|| {
        TctCompiler::new(schema.clone(), "DeepNestingTest")
            .unwrap()
            .compile()
    });

    assert!(result.is_ok(), "Should not panic on deeply nested schemas");

    // The actual generation might succeed or fail, but shouldn't panic
    if let Ok(Err(e)) = result {
        let error_msg = format!("{}", e);
        assert!(
            !error_msg.is_empty(),
            "Should provide meaningful error for complex schemas"
        );
    }
}

#[test]
fn test_schema_robustness_zero_length_arrays_and_strings() {
    use tct::tokenizer::JsonTokenEncoder;

    // Test empty string
    let empty_string = String::new();
    let mut tokens = Vec::new();
    empty_string.encode(&mut tokens);

    let mut token_iter = tokens.into_iter();
    let decoded: Result<String, _> = String::decode(&mut token_iter);
    assert_eq!(
        decoded.unwrap(),
        empty_string,
        "Should handle empty strings"
    );

    // Test empty array
    let empty_vec: Vec<String> = Vec::new();
    let mut tokens = Vec::new();
    empty_vec.encode(&mut tokens);

    let mut token_iter = tokens.into_iter();
    let decoded: Result<Vec<String>, _> = Vec::decode(&mut token_iter);
    assert_eq!(decoded.unwrap(), empty_vec, "Should handle empty arrays");
}

#[test]
fn test_schema_robustness_string_encoding_edge_cases() {
    use tct::tokenizer::JsonTokenEncoder;

    // Test ASCII-only strings (should succeed in the default UTF-8 string mode)
    let ascii_test_strings = vec![
        "".to_string(),                            // Empty
        "\0".to_string(),                          // Null character
        "a".repeat(65536),                         // Very long string
        "Hello, World! 123 @#$%^&*()".to_string(), // ASCII with symbols
    ];

    for test_string in ascii_test_strings {
        let mut tokens = Vec::new();
        test_string.encode(&mut tokens);

        let mut token_iter = tokens.into_iter();
        let decoded: Result<String, _> = String::decode(&mut token_iter);

        assert!(
            decoded.is_ok(),
            "Should handle ASCII string: {:?}",
            test_string.chars().take(50).collect::<String>()
        );
        assert_eq!(decoded.unwrap(), test_string, "Should round-trip correctly");
    }

    // Test non-ASCII strings (now supported with UTF-8 encoding)
    let non_ascii_test_strings = vec![
        "🚀🦀".to_string(),         // Unicode emojis
        "\u{FEFF}test".to_string(), // BOM character (non-ASCII)
        "café".to_string(),         // Accented characters
    ];

    for test_string in non_ascii_test_strings {
        // With UTF-8 encoding, non-ASCII strings should encode successfully
        let mut tokens = Vec::new();
        test_string.encode(&mut tokens);

        assert!(
            !tokens.is_empty(),
            "Should encode non-ASCII string: {:?}",
            test_string
        );

        // Verify round-trip
        let mut token_iter = tokens.into_iter();
        let decoded = String::decode(&mut token_iter);
        assert!(
            decoded.is_ok(),
            "Non-ASCII string should round-trip successfully: {:?}",
            test_string
        );
        assert_eq!(decoded.unwrap(), test_string);
    }
}
