//! Performance Memory Limits Test Suite
//!
//! This test suite validates TCT behavior under memory pressure and with large schemas.
//! Tests are designed to stress memory usage patterns and ensure graceful handling
//! of resource constraints without panics or memory leaks.

use serde_json::{Value, json};
use std::collections::HashMap;
use tct::{OptimizationLevel, TctCompiler};

fn parse_generated_code(code: &str) -> syn::File {
    syn::parse_file(code).unwrap_or_else(|error| panic!("generated Rust should parse: {error}"))
}

fn assert_generated_struct(code: &str, name: &str) {
    let file = parse_generated_code(code);
    assert!(
        file.items
            .iter()
            .any(|item| matches!(item, syn::Item::Struct(structure) if structure.ident == name)),
        "generated struct `{name}` should exist"
    );
}

fn assert_generated_enum_present(code: &str) {
    let file = parse_generated_code(code);
    assert!(
        file.items
            .iter()
            .any(|item| matches!(item, syn::Item::Enum(_))),
        "generated code should contain an enum"
    );
}

/// Test processing very large schemas with many properties
#[test]
fn test_memory_limits_large_schema_many_properties() {
    let mut properties = HashMap::new();

    // Create schema with 1000 properties to stress memory allocation
    for i in 0..1000 {
        properties.insert(
            format!("property_{}", i),
            json!({
                "type": "string",
                "description": format!("Property number {}", i)
            }),
        );
    }

    let schema = json!({
        "title": "LargeSchema",
        "type": "object",
        "properties": properties,
        "required": ["property_0", "property_1", "property_2"]
    });

    // Should handle large schema without memory issues
    let result = TctCompiler::new(schema, "LargeSchema")
        .and_then(|compiler| compiler.compile())
        .and_then(|grammar| grammar.generate_code());
    assert!(
        result.is_ok(),
        "Should handle large schema with many properties"
    );

    let code = result.unwrap();
    assert!(
        code.len() > 1000,
        "Generated code should be substantial for large schema"
    );
    assert_generated_struct(&code, "LargeSchema");
}

/// Test deeply nested object schemas to stress stack and heap usage
#[test]
fn test_memory_limits_deeply_nested_objects() {
    // Create 50-level deep nesting to stress recursion handling
    let mut current_schema = json!({
        "type": "string",
        "description": "Leaf value at maximum depth"
    });

    for depth in (0..50).rev() {
        current_schema = json!({
            "type": "object",
            "properties": {
                format!("level_{}", depth): current_schema
            },
            "required": [format!("level_{}", depth)]
        });
    }

    let schema = json!({
        "title": "DeeplyNested",
        "type": "object",
        "properties": {
            "root": current_schema
        },
        "required": ["root"]
    });

    // Should handle deep nesting without stack overflow
    let result = TctCompiler::new(schema, "DeeplyNested")
        .and_then(|compiler| compiler.compile())
        .and_then(|grammar| grammar.generate_code());
    assert!(
        result.is_ok(),
        "Should handle deeply nested schemas without stack overflow"
    );

    let code = result.unwrap();
    assert_generated_struct(&code, "DeeplyNested");
    // Verify nested structures are generated
    assert!(code.contains("Level0"), "Should generate nested structures");
}

/// Test large array schemas with complex item types
#[test]
fn test_memory_limits_large_array_schemas() {
    let complex_item = json!({
        "type": "object",
        "properties": {
            "id": { "type": "string" },
            "data": {
                "type": "object",
                "properties": {
                    "values": {
                        "type": "array",
                        "items": { "type": "number" }
                    },
                    "metadata": {
                        "type": "object",
                        "properties": {
                            "created": { "type": "string" },
                            "modified": { "type": "string" },
                            "tags": {
                                "type": "array",
                                "items": { "type": "string" }
                            }
                        }
                    }
                }
            }
        },
        "required": ["id", "data"]
    });

    let schema = json!({
        "title": "LargeArraySchema",
        "type": "object",
        "properties": {
            "items": {
                "type": "array",
                "items": complex_item,
                "description": "Large array of complex nested objects"
            },
            "backup_items": {
                "type": "array",
                "items": complex_item,
                "description": "Backup array with same complex structure"
            }
        },
        "required": ["items"]
    });

    // Should handle large array schemas with complex nested items
    let result = TctCompiler::new(schema, "LargeArraySchema")
        .and_then(|compiler| compiler.compile())
        .and_then(|grammar| grammar.generate_code());
    assert!(result.is_ok(), "Should handle large array schemas");

    let code = result.unwrap();
    assert_generated_struct(&code, "LargeArraySchema");
    assert!(
        code.contains("Vec<"),
        "Should generate Vec types for arrays"
    );
}

/// Test schemas with many enum variants to stress vocabulary generation
#[test]
fn test_memory_limits_many_enum_variants() {
    let mut enum_values = Vec::new();

    // Create enum with 500 variants to stress enum processing
    for i in 0..500 {
        enum_values.push(format!("variant_{:03}", i));
    }

    let schema = json!({
        "title": "ManyVariants",
        "type": "object",
        "properties": {
            "status": {
                "enum": enum_values,
                "description": "Enum with many variants"
            },
            "backup_status": {
                "enum": enum_values,
                "description": "Backup enum with same variants"
            }
        },
        "required": ["status"]
    });

    // Should handle enums with many variants
    let result = TctCompiler::new(schema, "ManyVariants")
        .and_then(|compiler| compiler.compile())
        .and_then(|grammar| grammar.generate_code());
    assert!(result.is_ok(), "Should handle enums with many variants");

    let code = result.unwrap();
    assert_generated_struct(&code, "ManyVariants");
    assert_generated_enum_present(&code);
    assert!(
        code.contains("Variant000"),
        "Should generate enum variants with correct naming"
    );
}

/// Test oneOf/anyOf schemas with many alternatives to stress branching logic
#[test]
fn test_memory_limits_many_schema_alternatives() {
    let mut alternatives = Vec::new();

    // Create 100 different object alternatives
    for i in 0..100 {
        alternatives.push(json!({
            "type": "object",
            "properties": {
                format!("type_{}", i): { "type": "string" },
                format!("value_{}", i): { "type": "number" },
                format!("active_{}", i): { "type": "boolean" }
            },
            "required": [format!("type_{}", i)]
        }));
    }

    let schema = json!({
        "title": "ManyAlternatives",
        "type": "object",
        "properties": {
            "variant": {
                "oneOf": alternatives,
                "description": "OneOf with many alternatives"
            }
        },
        "required": ["variant"]
    });

    // Should handle oneOf with many alternatives
    let result = TctCompiler::new(schema, "ManyAlternatives")
        .and_then(|compiler| compiler.compile())
        .and_then(|grammar| grammar.generate_code());
    assert!(result.is_ok(), "Should handle oneOf with many alternatives");

    let code = result.unwrap();
    assert_generated_struct(&code, "ManyAlternatives");
}

/// Test processing multiple large schemas simultaneously to stress global memory usage
#[test]
fn test_memory_limits_multiple_large_schemas() {
    let schemas = vec![
        ("Schema1", create_large_test_schema("Schema1", 200)),
        ("Schema2", create_large_test_schema("Schema2", 200)),
        ("Schema3", create_large_test_schema("Schema3", 200)),
        ("Schema4", create_large_test_schema("Schema4", 200)),
        ("Schema5", create_large_test_schema("Schema5", 200)),
    ];

    // Process all schemas - should not accumulate excessive memory
    let mut results: Vec<String> = Vec::new();
    for (name, schema) in schemas {
        let result = TctCompiler::new(schema, name)
            .and_then(|compiler| compiler.compile())
            .and_then(|grammar| grammar.generate_code());
        assert!(
            result.is_ok(),
            "Should process schema {} successfully",
            name
        );
        results.push(result.unwrap());
    }

    // Verify all schemas were processed successfully
    assert_eq!(results.len(), 5, "Should process all 5 large schemas");
    for (i, code) in results.iter().enumerate() {
        let expected_struct = format!("pub struct Schema{}", i + 1);
        assert!(
            code.contains(&expected_struct),
            "Should generate struct for Schema{}",
            i + 1
        );
    }
}

/// Test schema compilation with optimization to verify memory efficiency
#[test]
fn test_memory_limits_optimized_compilation() {
    let schema = create_large_test_schema("OptimizedLarge", 500);

    // Test with different optimization levels
    let result = TctCompiler::new(schema, "OptimizedLarge")
        .and_then(|compiler| {
            compiler
                .with_optimization_level(OptimizationLevel::O2)
                .compile()
        })
        .and_then(|grammar| grammar.generate_code());
    assert!(
        result.is_ok(),
        "Should handle optimized compilation of large schema"
    );

    let code = result.unwrap();
    assert_generated_struct(&code, "OptimizedLarge");
    assert!(
        code.len() > 1000,
        "Should generate substantial optimized code"
    );
}

/// Test memory usage with string-heavy schemas to stress string allocation
#[test]
fn test_memory_limits_string_heavy_schemas() {
    let mut properties = HashMap::new();

    // Create schema with many string properties with long descriptions
    for i in 0..200 {
        let long_description = format!(
            "This is a very long description for property number {} that contains \
            extensive documentation about the purpose, usage, constraints, and \
            validation requirements for this particular string field in the schema. \
            It includes multiple sentences with detailed information about the expected \
            format, length limitations, character encoding requirements, and business \
            logic constraints that apply to this specific property.",
            i
        );

        properties.insert(
            format!("string_property_{:03}", i),
            json!({
                "type": "string",
                "description": long_description,
                "minLength": 1,
                "maxLength": 1000,
                "pattern": "^[a-zA-Z0-9_-]+$"
            }),
        );
    }

    let schema = json!({
        "title": "StringHeavy",
        "type": "object",
        "properties": properties,
        "description": "Schema with many string properties and extensive documentation"
    });

    // Should handle string-heavy schemas efficiently
    let result = TctCompiler::new(schema, "StringHeavy")
        .and_then(|compiler| compiler.compile())
        .and_then(|grammar| grammar.generate_code());
    assert!(result.is_ok(), "Should handle string-heavy schemas");

    let code = result.unwrap();
    assert_generated_struct(&code, "StringHeavy");
}

/// Helper function to create large test schemas
fn create_large_test_schema(name: &str, property_count: usize) -> Value {
    let mut properties = HashMap::new();

    for i in 0..property_count {
        let property_type = match i % 4 {
            0 => json!({ "type": "string" }),
            1 => json!({ "type": "number" }),
            2 => json!({ "type": "boolean" }),
            _ => json!({
                "type": "object",
                "properties": {
                    "nested_value": { "type": "string" },
                    "nested_count": { "type": "integer" }
                }
            }),
        };

        properties.insert(format!("prop_{:03}", i), property_type);
    }

    json!({
        "title": name,
        "type": "object",
        "properties": properties,
        "required": [format!("prop_000"), format!("prop_001")]
    })
}
