//! Tests for input validation and robustness
//!
//! This test suite defines expected behavior for handling invalid inputs
//! and ensures graceful error handling instead of panics.

use serde_json::json;
use tct::*;

#[test]
fn test_input_validation_empty_enum_fails_gracefully() {
    let schema = json!({
        "title": "EmptyEnumTest",
        "type": "object",
        "properties": {
            "status": {
                "enum": ["", "valid"]  // Empty string should be handled gracefully
            }
        }
    });

    let result = TctCompiler::new(schema.clone(), "EmptyEnumTest")
        .unwrap()
        .compile();

    // Should either succeed with sanitized identifier or fail gracefully (not panic)
    match result {
        Ok(_) => println!("✅ Empty enum values handled gracefully"),
        Err(e) => {
            println!("📋 Expected graceful failure: {}", e);
            // Should be a proper error, not a panic
            assert!(!format!("{}", e).contains("panic"));
        }
    }
}

#[test]
fn test_input_validation_invalid_identifiers_fail_gracefully() {
    let schema = json!({
        "title": "InvalidIdentifierTest",
        "type": "object",
        "properties": {
            "option": {
                "enum": [
                    "123invalid",     // Starts with number
                    "spaces not allowed",  // Contains spaces
                    "special@chars",  // Contains special chars
                    "",               // Empty string
                    "valid_option"    // Should work
                ]
            }
        }
    });

    let result = TctCompiler::new(schema.clone(), "InvalidIdentifierTest")
        .unwrap()
        .compile();

    match result {
        Ok(grammar) => {
            println!("✅ Invalid identifiers sanitized successfully");
            let code = grammar
                .generate_code()
                .expect("Code generation should work");
            // Should contain sanitized valid identifiers
            assert!(code.contains("enum"));
        }
        Err(e) => {
            println!("📋 Graceful failure on invalid identifiers: {}", e);
            // Should be proper error, not panic
            assert!(!format!("{}", e).contains("panic"));
            assert!(!format!("{}", e).contains("not a valid Ident"));
        }
    }
}

#[test]
fn test_input_validation_extreme_enum_values_handled() {
    use std::panic;

    let schema = json!({
        "title": "ExtremeTest",
        "type": "object",
        "properties": {
            "extreme": {
                "enum": [
                    "\0",           // Null character
                    "\n\t\r",       // Whitespace only
                    "🦀🔥",         // Unicode emojis
                    "very_long_enum_value_that_exceeds_reasonable_length_limits_and_might_cause_issues",
                    "ALLCAPS",      // All caps
                    "123",          // Pure number
                    "valid"         // Should work
                ]
            }
        }
    });

    // This should NOT panic - that's the whole point of robust input handling
    let result = panic::catch_unwind(|| {
        TctCompiler::new(schema.clone(), "ExtremeTest")
            .unwrap()
            .compile()
    });

    assert!(
        result.is_ok(),
        "Code generation should not panic on extreme inputs"
    );

    let generation_result = result.unwrap();
    match generation_result {
        Ok(_) => println!("✅ Extreme enum values handled robustly"),
        Err(e) => {
            println!("📋 Graceful failure on extreme values: {}", e);
            // Should be clean error message, not panic details
            assert!(!format!("{}", e).contains("panic"));
        }
    }
}

// NOTE: Utility function testing moved to unit tests in utils.rs
// This integration test focuses on end-to-end behavior

#[test]
fn test_input_validation_property_names_sanitized() {
    let schema = json!({
        "title": "PropertyTest",
        "type": "object",
        "properties": {
            "": "invalid empty property name",
            "123number": {"type": "string"},
            "special@property": {"type": "boolean"},
            "valid_property": {"type": "integer"}
        }
    });

    let result = TctCompiler::new(schema.clone(), "PropertyTest")
        .unwrap()
        .compile();

    match result {
        Ok(grammar) => {
            println!("✅ Invalid property names handled");
            let code = grammar
                .generate_code()
                .expect("Code generation should work");
            // Generated code should have valid field names
            assert!(code.contains("struct"));
        }
        Err(e) => {
            println!("📋 Graceful failure on invalid property names: {}", e);
            assert!(!format!("{}", e).contains("panic"));
        }
    }
}
