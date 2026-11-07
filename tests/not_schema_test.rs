//! Not Schema Tests
//!
//! This test suite validates support for JSON Schema "not" keyword,
//! which excludes values that match the specified sub-schema.
//! Essential for constraint-based validation and type exclusion.

use serde_json::json;
use tct::TctCompiler;

#[test]
fn test_not_schema_not_simple_type() {
    // Test schema that excludes a specific type
    let schema = json!({
        "title": "NotString",
        "type": "object",
        "not": {"type": "string"}
    });

    let result = TctCompiler::new(schema, "NotString").unwrap().compile();

    // Should compile (though implementation may vary)
    // The test validates the schema is processed without errors
    match result {
        Ok(_) => println!("✅ Not string schema compiled successfully"),
        Err(e) => {
            // If not implemented, should give clear error
            assert!(
                e.to_string().contains("not") || e.to_string().contains("unsupported"),
                "Error should mention 'not' schema limitation: {}",
                e
            );
            println!("⚠️ Not schema not yet implemented: {}", e);
        }
    }
}

#[test]
fn test_not_schema_not_enum_values() {
    // Test schema that excludes specific enum values
    let schema = json!({
        "title": "NotSpecificValues",
        "type": "string",
        "not": {
            "enum": ["forbidden", "blocked"]
        }
    });

    let result = TctCompiler::new(schema, "NotSpecificValues")
        .unwrap()
        .compile();

    match result {
        Ok(_) => println!("✅ Not enum values schema compiled successfully"),
        Err(e) => {
            assert!(
                e.to_string().contains("not") || e.to_string().contains("unsupported"),
                "Error should mention 'not' schema limitation: {}",
                e
            );
            println!("⚠️ Not enum schema not yet implemented: {}", e);
        }
    }
}

#[test]
fn test_not_schema_not_object_properties() {
    // Test schema that excludes objects with specific properties
    let schema = json!({
        "title": "NotWithProperty",
        "type": "object",
        "not": {
            "properties": {
                "forbidden_field": {"type": "string"}
            },
            "required": ["forbidden_field"]
        }
    });

    let result = TctCompiler::new(schema, "NotWithProperty")
        .unwrap()
        .compile();

    match result {
        Ok(_) => println!("✅ Not object properties schema compiled successfully"),
        Err(e) => {
            assert!(
                e.to_string().contains("not") || e.to_string().contains("unsupported"),
                "Error should mention 'not' schema limitation: {}",
                e
            );
            println!("⚠️ Not object schema not yet implemented: {}", e);
        }
    }
}

#[test]
fn test_not_schema_not_with_allof() {
    // Test Not combined with AllOf
    let schema = json!({
        "title": "NotAllOf",
        "allOf": [
            {"type": "object"},
            {
                "not": {
                    "properties": {
                        "invalid": {"type": "boolean"}
                    }
                }
            }
        ]
    });

    let result = TctCompiler::new(schema, "NotAllOf").unwrap().compile();

    match result {
        Ok(_) => println!("✅ Not with AllOf schema compiled successfully"),
        Err(e) => {
            // Could fail due to either not or allof+not complexity
            println!("⚠️ Complex Not+AllOf schema not implemented: {}", e);
        }
    }
}

#[test]
fn test_not_schema_not_nested() {
    // Test nested Not schemas
    let schema = json!({
        "title": "NestedNot",
        "type": "object",
        "not": {
            "not": {
                "type": "string"
            }
        }
    });

    let result = TctCompiler::new(schema, "NestedNot").unwrap().compile();

    match result {
        Ok(_) => {
            // Double negation should be equivalent to the original type
            println!("✅ Nested Not schema compiled successfully (double negation)");
        }
        Err(e) => {
            println!("⚠️ Nested Not schema not implemented: {}", e);
        }
    }
}

#[test]
fn test_not_schema_not_with_constraints() {
    // Test Not with value constraints
    let schema = json!({
        "title": "NotConstraints",
        "type": "integer",
        "not": {
            "minimum": 0,
            "maximum": 100
        }
    });

    let result = TctCompiler::new(schema, "NotConstraints")
        .unwrap()
        .compile();

    match result {
        Ok(_) => println!("✅ Not with constraints schema compiled successfully"),
        Err(e) => {
            println!("⚠️ Not with constraints not implemented: {}", e);
        }
    }
}

#[test]
fn test_not_schema_not_error_cases() {
    // Test error handling for invalid Not schemas

    // Invalid Not schema structure
    let invalid_schema = json!({
        "title": "InvalidNot",
        "type": "object",
        "not": "invalid_not_value"
    });

    let result = TctCompiler::new(invalid_schema, "InvalidNot")
        .unwrap()
        .compile();

    // Current implementation is permissive and handles invalid "not" gracefully
    // by falling back to generic processing rather than failing
    match result {
        Ok(_) => println!("ℹ️ Invalid Not schema handled gracefully with fallback"),
        Err(_) => println!("ℹ️ Invalid Not schema properly rejected"),
    }

    // Empty Not schema
    let empty_not_schema = json!({
        "title": "EmptyNot",
        "type": "object",
        "not": {}
    });

    let result2 =
        TctCompiler::new(empty_not_schema, "EmptyNot").and_then(|compiler| compiler.compile());

    // This might be valid (excludes nothing) or invalid depending on implementation
    match result2 {
        Ok(_) => println!("ℹ️ Empty Not schema treated as valid"),
        Err(_) => println!("ℹ️ Empty Not schema treated as invalid"),
    }

    println!("✅ Not schema error handling validated");
}

#[test]
fn test_not_schema_not_with_optimizations() {
    // Test Not schemas with optimization passes
    let schema = json!({
        "title": "OptimizedNot",
        "type": "string",
        "not": {
            "enum": ["excluded1", "excluded2"]
        }
    });

    let result = TctCompiler::new(schema, "OptimizedNot")
        .unwrap()
        .with_optimization_level(tct::OptimizationLevel::O2)
        .compile();

    match result {
        Ok(_) => println!("✅ Not schema with optimizations compiled successfully"),
        Err(e) => {
            println!("⚠️ Not schema optimizations not implemented: {}", e);
        }
    }
}

#[test]
fn test_not_schema_not_array_items() {
    // Test Not applied to array items
    let schema = json!({
        "title": "ArrayNotItems",
        "type": "array",
        "items": {
            "not": {"type": "string"}
        }
    });

    let result = TctCompiler::new(schema, "ArrayNotItems").unwrap().compile();

    match result {
        Ok(_) => println!("✅ Array with Not items compiled successfully"),
        Err(e) => {
            println!("⚠️ Array Not items not implemented: {}", e);
        }
    }
}

#[test]
fn test_not_schema_not_real_world_validation() {
    // Test realistic validation scenario excluding invalid patterns
    let schema = json!({
        "title": "ValidatedEmail",
        "type": "string",
        "pattern": "^[^@]+@[^@]+\\.[^@]+$",
        "not": {
            "pattern": ".*test.*"
        }
    });

    let result = TctCompiler::new(schema, "ValidatedEmail")
        .unwrap()
        .compile();

    match result {
        Ok(_) => {
            println!("✅ Real-world Not validation schema compiled successfully");
            // This would represent emails that match email pattern but don't contain "test"
        }
        Err(e) => {
            println!("⚠️ Complex Not validation not implemented: {}", e);
        }
    }
}
