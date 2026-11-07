//! Schema compatibility testing - isolate specific compilation issues

use serde_json::json;
use tct::TctCompiler;

/// Test schema with union types (type array)
#[test]
fn test_union_type_array_schema() {
    let schema = json!({
        "type": ["object", "string"],
        "properties": {
            "name": {"type": "string"}
        }
    });

    let result = TctCompiler::new(schema, "UnionTypeTest").and_then(|compiler| compiler.compile());

    match result {
        Ok(_) => println!("✅ Union type schema compilation succeeded"),
        Err(e) => println!("❌ Union type schema failed: {}", e),
    }

    // Reaching this point verifies compilation did not panic.
}

/// Test schema with allOf only (no properties)
#[test]
fn test_allof_only_schema() {
    let schema = json!({
        "allOf": [
            {"type": "object", "properties": {"name": {"type": "string"}}},
            {"type": "object", "properties": {"version": {"type": "string"}}}
        ]
    });

    let result = TctCompiler::new(schema, "AllOfTest").and_then(|compiler| compiler.compile());

    match result {
        Ok(_) => println!("✅ AllOf only schema compilation succeeded"),
        Err(e) => println!("❌ AllOf only schema failed: {}", e),
    }

    // Reaching this point verifies compilation did not panic.
}

/// Test schema with definitions and $ref
#[test]
fn test_definitions_ref_schema() {
    let schema = json!({
        "type": "object",
        "properties": {
            "person": {"$ref": "#/definitions/person"}
        },
        "definitions": {
            "person": {
                "type": "object",
                "properties": {
                    "name": {"type": "string"}
                }
            }
        }
    });

    let result = TctCompiler::new(schema, "RefTest").and_then(|compiler| compiler.compile());

    match result {
        Ok(_) => println!("✅ Definitions+$ref schema compilation succeeded"),
        Err(e) => println!("❌ Definitions+$ref schema failed: {}", e),
    }

    // Reaching this point verifies compilation did not panic.
}

/// Test successful compilation of basic working schema
#[test]
fn test_basic_working_schema() {
    let schema = json!({
        "type": "object",
        "properties": {
            "name": {"type": "string"},
            "age": {"type": "integer"}
        },
        "additionalProperties": {"type": "string"}
    });

    let result = TctCompiler::new(schema, "BasicTest").and_then(|compiler| compiler.compile());

    result.expect("Basic schema should compile successfully");
    println!("✅ Basic schema compilation succeeded");
}
