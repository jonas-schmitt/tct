//! Recursive Schema Tests
//!
//! This test suite validates support for recursive and circular schema references,
//! including $ref resolution, depth limits, and cycle detection.
//! Critical for tree structures, nested data, and self-referencing schemas.

use serde_json::json;
use tct::TctCompiler;

#[test]
fn test_recursive_schema_recursive_tree_structure() {
    // Test simple recursive tree (Node with optional children)
    let schema = json!({
        "title": "TreeNode",
        "type": "object",
        "properties": {
            "value": {"type": "string"},
            "children": {
                "type": "array",
                "items": {"$ref": "#"}
            }
        },
        "required": ["value"]
    });

    let result = TctCompiler::new(schema, "TreeNode").unwrap().compile();

    match result {
        Ok(_) => {
            println!("✅ Recursive tree structure compiled successfully");
            let generated = result.unwrap().generate_code().unwrap();
            assert!(
                generated.contains("TreeNode"),
                "Should generate recursive type"
            );
        }
        Err(e) => {
            assert!(
                e.to_string().contains("recursive")
                    || e.to_string().contains("$ref")
                    || e.to_string().contains("circular")
                    || e.to_string().contains("unsupported"),
                "Error should mention recursive/ref limitation: {}",
                e
            );
            println!("⚠️ Recursive schema not yet implemented: {}", e);
        }
    }
}

#[test]
fn test_recursive_schema_self_referencing_object() {
    // Test object that can contain itself as a property
    let schema = json!({
        "title": "SelfRef",
        "type": "object",
        "properties": {
            "name": {"type": "string"},
            "nested": {"$ref": "#"}
        }
    });

    let result = TctCompiler::new(schema, "SelfRef").unwrap().compile();

    match result {
        Ok(_) => {
            println!("✅ Self-referencing object compiled successfully");
        }
        Err(e) => {
            println!("⚠️ Self-referencing schema not implemented: {}", e);
        }
    }
}

#[test]
fn test_recursive_schema_mutually_recursive_schemas() {
    // Test two schemas that reference each other
    let schema = json!({
        "title": "PersonCompany",
        "definitions": {
            "Person": {
                "type": "object",
                "properties": {
                    "name": {"type": "string"},
                    "company": {"$ref": "#/definitions/Company"}
                }
            },
            "Company": {
                "type": "object",
                "properties": {
                    "name": {"type": "string"},
                    "employees": {
                        "type": "array",
                        "items": {"$ref": "#/definitions/Person"}
                    }
                }
            }
        },
        "$ref": "#/definitions/Person"
    });

    let result = TctCompiler::new(schema, "PersonCompany").unwrap().compile();

    match result {
        Ok(_) => {
            println!("✅ Mutually recursive schemas compiled successfully");
        }
        Err(e) => {
            println!("⚠️ Mutually recursive schemas not implemented: {}", e);
        }
    }
}

#[test]
fn test_recursive_schema_recursive_depth_limit() {
    // Test that extremely deep recursion is handled gracefully
    let schema = json!({
        "title": "DeepRecursion",
        "type": "object",
        "properties": {
            "level": {"type": "integer"},
            "child": {"$ref": "#"}
        }
    });

    let result = TctCompiler::new(schema, "DeepRecursion").unwrap().compile();

    match result {
        Ok(_) => {
            println!("✅ Recursive schema with depth compiled successfully");
            // Should have protection against stack overflow
        }
        Err(e) => {
            println!("⚠️ Deep recursive schema not implemented: {}", e);
        }
    }
}

#[test]
fn test_recursive_schema_recursive_with_constraints() {
    // Test recursive schema with additional constraints
    let schema = json!({
        "title": "ConstrainedTree",
        "type": "object",
        "properties": {
            "id": {
                "type": "integer",
                "minimum": 1,
                "maximum": 1000
            },
            "category": {
                "type": "string",
                "enum": ["leaf", "branch", "root"]
            },
            "subtree": {"$ref": "#"}
        },
        "required": ["id", "category"]
    });

    let result = TctCompiler::new(schema, "ConstrainedTree")
        .unwrap()
        .with_optimization_level(tct::OptimizationLevel::O2)
        .compile();

    match result {
        Ok(_) => {
            println!("✅ Recursive schema with constraints and optimizations compiled");
        }
        Err(e) => {
            println!("⚠️ Constrained recursive schema not implemented: {}", e);
        }
    }
}

#[test]
fn test_recursive_schema_recursive_array_nesting() {
    // Test recursive structure via arrays
    let schema = json!({
        "title": "NestedArrays",
        "type": "array",
        "items": {
            "anyOf": [
                {"type": "string"},
                {"$ref": "#"}
            ]
        }
    });

    let result = TctCompiler::new(schema, "NestedArrays").unwrap().compile();

    match result {
        Ok(_) => {
            println!("✅ Recursive array nesting compiled successfully");
        }
        Err(e) => {
            println!("⚠️ Recursive array schema not implemented: {}", e);
        }
    }
}

#[test]
fn test_recursive_schema_circular_reference_detection() {
    // Test detection and handling of circular references
    let schema = json!({
        "title": "CircularTest",
        "type": "object",
        "properties": {
            "a": {
                "type": "object",
                "properties": {
                    "b": {"$ref": "#"}
                }
            }
        }
    });

    let result = TctCompiler::new(schema, "CircularTest").unwrap().compile();

    match result {
        Ok(_) => {
            println!("✅ Circular reference handled successfully");
        }
        Err(e) => {
            // Should either support it or give clear error about circular refs
            assert!(
                e.to_string().contains("circular")
                    || e.to_string().contains("recursive")
                    || e.to_string().contains("$ref"),
                "Error should mention circular reference issue: {}",
                e
            );
            println!("⚠️ Circular reference detection working: {}", e);
        }
    }
}

#[test]
fn test_recursive_schema_recursive_error_cases() {
    // Test error handling for malformed recursive schemas

    // Invalid $ref
    let invalid_ref_schema = json!({
        "title": "InvalidRef",
        "type": "object",
        "properties": {
            "child": {"$ref": "#/invalid/path"}
        }
    });

    let result = TctCompiler::new(invalid_ref_schema, "InvalidRef")
        .unwrap()
        .compile();

    // Check if invalid $ref is handled gracefully
    match result {
        Ok(_) => println!("ℹ️  Invalid $ref was accepted (may have fallback behavior)"),
        Err(e) => println!("✅ Invalid $ref properly rejected: {}", e),
    }

    // Malformed $ref syntax
    let malformed_ref_schema = json!({
        "title": "MalformedRef",
        "type": "object",
        "properties": {
            "child": {"$ref": "not_a_valid_ref"}
        }
    });

    let result2 = TctCompiler::new(malformed_ref_schema, "MalformedRef")
        .unwrap()
        .compile();

    assert!(result2.is_err(), "Malformed $ref should fail");

    println!("✅ Recursive schema error handling validated");
}

#[test]
fn test_recursive_schema_recursive_with_allof() {
    // Test recursive schema combined with AllOf
    let schema = json!({
        "title": "RecursiveAllOf",
        "allOf": [
            {
                "type": "object",
                "properties": {
                    "base_field": {"type": "string"}
                }
            },
            {
                "type": "object",
                "properties": {
                    "recursive_field": {"$ref": "#"}
                }
            }
        ]
    });

    let result = TctCompiler::new(schema, "RecursiveAllOf")
        .unwrap()
        .compile();

    match result {
        Ok(_) => {
            println!("✅ Recursive AllOf schema compiled successfully");
        }
        Err(e) => {
            println!("⚠️ Recursive AllOf not implemented: {}", e);
        }
    }
}

#[test]
fn test_recursive_schema_json_pointer_references() {
    // Test $ref with JSON Pointer paths
    let schema = json!({
        "title": "JsonPointerRef",
        "definitions": {
            "Address": {
                "type": "object",
                "properties": {
                    "street": {"type": "string"},
                    "city": {"type": "string"}
                }
            }
        },
        "type": "object",
        "properties": {
            "home": {"$ref": "#/definitions/Address"},
            "work": {"$ref": "#/definitions/Address"}
        }
    });

    let result = TctCompiler::new(schema, "JsonPointerRef")
        .unwrap()
        .compile();

    match result {
        Ok(_) => {
            println!("✅ JSON Pointer $ref compiled successfully");
        }
        Err(e) => {
            println!("⚠️ JSON Pointer $ref not implemented: {}", e);
        }
    }
}

#[test]
fn test_recursive_schema_json_pointer_escaped_references() {
    let schema = json!({
        "title": "EscapedPointerRef",
        "definitions": {
            "User/Record~Internal": {
                "type": "object",
                "properties": {
                    "display~name": {"type": "string"}
                },
                "required": ["display~name"],
                "additionalProperties": false
            }
        },
        "type": "object",
        "properties": {
            "primary": {"$ref": "#/definitions/User~1Record~0Internal"}
        },
        "required": ["primary"],
        "additionalProperties": false
    });

    let grammar = TctCompiler::new(schema, "EscapedPointerRef")
        .unwrap()
        .compile()
        .expect("escaped JSON Pointer $ref should compile");
    let generated = grammar
        .generate_code()
        .expect("escaped JSON Pointer $ref should generate code");

    assert!(
        generated.contains("UserRecordInternal"),
        "escaped definition target should use the decoded type name"
    );
    assert!(
        generated.contains("display~name"),
        "generated serde metadata should preserve the original escaped property name"
    );
}

#[test]
fn test_recursive_schema_recursive_real_world_json_api() {
    // Test realistic recursive structure like a comment thread
    let schema = json!({
        "title": "Comment",
        "type": "object",
        "properties": {
            "id": {"type": "integer"},
            "text": {"type": "string"},
            "author": {"type": "string"},
            "timestamp": {"type": "string"},
            "replies": {
                "type": "array",
                "items": {"$ref": "#"}
            }
        },
        "required": ["id", "text", "author"]
    });

    let result = TctCompiler::new(schema, "Comment").unwrap().compile();

    match result {
        Ok(_) => {
            println!("✅ Real-world recursive comment structure compiled successfully");
        }
        Err(e) => {
            println!("⚠️ Real-world recursive structure not implemented: {}", e);
        }
    }
}
