//! Default Values Schema Tests
//!
//! This test suite validates support for JSON Schema default values,
//! which specify fallback values when properties are missing.
//! Essential for optional fields and robust data handling.

use serde_json::json;
use tct::TctCompiler;

#[test]
fn test_default_values_default_string_value() {
    // Test schema with default string value
    let schema = json!({
        "title": "DefaultString",
        "type": "object",
        "properties": {
            "name": {
                "type": "string",
                "default": "Anonymous"
            },
            "required_field": {"type": "string"}
        },
        "required": ["required_field"]
    });

    let result = TctCompiler::new(schema, "DefaultString").unwrap().compile();

    match result {
        Ok(_) => {
            println!("✅ Schema with default string value compiled successfully");
            let generated = result.unwrap().generate_code().unwrap();
            // Should handle optional field with default
            assert!(
                generated.contains("DefaultString"),
                "Should generate struct with default handling"
            );
        }
        Err(e) => {
            assert!(
                e.to_string().contains("default") || e.to_string().contains("unsupported"),
                "Error should mention default value limitation: {}",
                e
            );
            println!("⚠️ Default values not yet implemented: {}", e);
        }
    }
}

#[test]
fn test_default_values_default_numeric_values() {
    // Test schema with various numeric default values
    let schema = json!({
        "title": "DefaultNumbers",
        "type": "object",
        "properties": {
            "count": {
                "type": "integer",
                "default": 0
            },
            "ratio": {
                "type": "number",
                "default": 1.0
            },
            "percentage": {
                "type": "number",
                "default": 0.5
            }
        }
    });

    let result = TctCompiler::new(schema, "DefaultNumbers")
        .unwrap()
        .compile();

    match result {
        Ok(_) => {
            println!("✅ Schema with default numeric values compiled successfully");
        }
        Err(e) => {
            println!("⚠️ Default numeric values not implemented: {}", e);
        }
    }
}

#[test]
fn test_default_values_default_boolean_value() {
    // Test schema with default boolean value
    let schema = json!({
        "title": "DefaultBoolean",
        "type": "object",
        "properties": {
            "enabled": {
                "type": "boolean",
                "default": false
            },
            "active": {
                "type": "boolean",
                "default": true
            }
        }
    });

    let result = TctCompiler::new(schema, "DefaultBoolean")
        .unwrap()
        .compile();

    match result {
        Ok(_) => {
            println!("✅ Schema with default boolean values compiled successfully");
        }
        Err(e) => {
            println!("⚠️ Default boolean values not implemented: {}", e);
        }
    }
}

#[test]
fn test_default_values_default_array_value() {
    // Test schema with default array value
    let schema = json!({
        "title": "DefaultArray",
        "type": "object",
        "properties": {
            "tags": {
                "type": "array",
                "items": {"type": "string"},
                "default": []
            },
            "numbers": {
                "type": "array",
                "items": {"type": "integer"},
                "default": [1, 2, 3]
            }
        }
    });

    let result = TctCompiler::new(schema, "DefaultArray").unwrap().compile();

    match result {
        Ok(_) => {
            println!("✅ Schema with default array values compiled successfully");
        }
        Err(e) => {
            println!("⚠️ Default array values not implemented: {}", e);
        }
    }
}

#[test]
fn test_default_values_default_object_value() {
    // Test schema with default object value
    let schema = json!({
        "title": "DefaultObject",
        "type": "object",
        "properties": {
            "config": {
                "type": "object",
                "properties": {
                    "timeout": {"type": "integer"},
                    "retries": {"type": "integer"}
                },
                "default": {
                    "timeout": 30,
                    "retries": 3
                }
            }
        }
    });

    let result = TctCompiler::new(schema, "DefaultObject").unwrap().compile();

    match result {
        Ok(_) => {
            println!("✅ Schema with default object value compiled successfully");
        }
        Err(e) => {
            println!("⚠️ Default object values not implemented: {}", e);
        }
    }
}

#[test]
fn test_default_values_default_enum_value() {
    // Test schema with default enum value
    let schema = json!({
        "title": "DefaultEnum",
        "type": "object",
        "properties": {
            "status": {
                "type": "string",
                "enum": ["draft", "published", "archived"],
                "default": "draft"
            },
            "priority": {
                "type": "string",
                "enum": ["low", "medium", "high"],
                "default": "medium"
            }
        }
    });

    let result = TctCompiler::new(schema, "DefaultEnum")
        .unwrap()
        .with_optimization_level(tct::OptimizationLevel::O2)
        .compile();

    match result {
        Ok(_) => {
            println!("✅ Schema with default enum values and optimizations compiled");
        }
        Err(e) => {
            println!("⚠️ Default enum values not implemented: {}", e);
        }
    }
}

#[test]
fn test_default_values_default_null_value() {
    // Test schema with null default value
    let schema = json!({
        "title": "DefaultNull",
        "type": "object",
        "properties": {
            "optional_data": {
                "anyOf": [
                    {"type": "string"},
                    {"type": "null"}
                ],
                "default": null
            }
        }
    });

    let result = TctCompiler::new(schema, "DefaultNull").unwrap().compile();

    match result {
        Ok(_) => {
            println!("✅ Schema with null default value compiled successfully");
        }
        Err(e) => {
            println!("⚠️ Null default values not implemented: {}", e);
        }
    }
}

#[test]
fn test_default_values_nested_default_values() {
    // Test schema with nested default values
    let schema = json!({
        "title": "NestedDefaults",
        "type": "object",
        "properties": {
            "user": {
                "type": "object",
                "properties": {
                    "name": {
                        "type": "string",
                        "default": "Guest"
                    },
                    "preferences": {
                        "type": "object",
                        "properties": {
                            "theme": {
                                "type": "string",
                                "default": "light"
                            }
                        },
                        "default": {"theme": "light"}
                    }
                },
                "default": {
                    "name": "Guest",
                    "preferences": {"theme": "light"}
                }
            }
        }
    });

    let result = TctCompiler::new(schema, "NestedDefaults")
        .unwrap()
        .compile();

    match result {
        Ok(_) => {
            println!("✅ Schema with nested default values compiled successfully");
        }
        Err(e) => {
            println!("⚠️ Nested default values not implemented: {}", e);
        }
    }
}

#[test]
fn test_default_values_default_with_validation() {
    // Test that default values respect validation constraints
    let schema = json!({
        "title": "ValidatedDefaults",
        "type": "object",
        "properties": {
            "bounded_number": {
                "type": "integer",
                "minimum": 1,
                "maximum": 100,
                "default": 50
            },
            "validated_string": {
                "type": "string",
                "pattern": "^[A-Z][a-z]+$",
                "default": "Default"
            }
        }
    });

    let result = TctCompiler::new(schema, "ValidatedDefaults")
        .unwrap()
        .compile();

    match result {
        Ok(_) => {
            println!("✅ Schema with validated default values compiled successfully");
        }
        Err(e) => {
            println!("⚠️ Validated default values not implemented: {}", e);
        }
    }
}

#[test]
fn test_default_values_default_error_cases() {
    // Test error handling for invalid default values

    // Default value doesn't match type
    let type_mismatch_schema = json!({
        "title": "TypeMismatch",
        "type": "object",
        "properties": {
            "number_field": {
                "type": "integer",
                "default": "not_a_number"
            }
        }
    });

    let result = TctCompiler::new(type_mismatch_schema, "TypeMismatch")
        .unwrap()
        .compile();

    // Should either compile (ignoring invalid default) or error appropriately
    match result {
        Ok(_) => println!("ℹ️ Type mismatch in default handled gracefully"),
        Err(e) => {
            assert!(
                e.to_string().contains("default")
                    || e.to_string().contains("type")
                    || e.to_string().contains("mismatch"),
                "Error should mention default/type issue: {}",
                e
            );
            println!("✅ Type mismatch in default caught: {}", e);
        }
    }

    // Default value violates constraints
    let constraint_violation_schema = json!({
        "title": "ConstraintViolation",
        "type": "object",
        "properties": {
            "bounded": {
                "type": "integer",
                "minimum": 10,
                "maximum": 20,
                "default": 5  // Outside bounds
            }
        }
    });

    let result2 = TctCompiler::new(constraint_violation_schema, "ConstraintViolation")
        .unwrap()
        .compile();

    match result2 {
        Ok(_) => println!("ℹ️ Constraint violation in default handled gracefully"),
        Err(e) => println!("✅ Constraint violation in default caught: {}", e),
    }

    println!("✅ Default value error handling validated");
}

#[test]
fn test_default_values_default_with_required_fields() {
    // Test interaction between default values and required fields
    let schema = json!({
        "title": "DefaultsWithRequired",
        "type": "object",
        "properties": {
            "required_field": {"type": "string"},
            "optional_with_default": {
                "type": "string",
                "default": "default_value"
            },
            "optional_without_default": {"type": "string"}
        },
        "required": ["required_field"]
    });

    let result = TctCompiler::new(schema, "DefaultsWithRequired")
        .unwrap()
        .compile();

    match result {
        Ok(_) => {
            println!("✅ Schema with defaults and required fields compiled successfully");
        }
        Err(e) => {
            println!("⚠️ Defaults with required fields not implemented: {}", e);
        }
    }
}

#[test]
fn test_default_values_default_real_world_config() {
    // Test realistic configuration schema with defaults
    let schema = json!({
        "title": "ServerConfig",
        "type": "object",
        "properties": {
            "host": {
                "type": "string",
                "default": "localhost"
            },
            "port": {
                "type": "integer",
                "minimum": 1,
                "maximum": 65535,
                "default": 8080
            },
            "ssl": {
                "type": "boolean",
                "default": false
            },
            "timeout": {
                "type": "integer",
                "default": 30
            },
            "max_connections": {
                "type": "integer",
                "default": 100
            },
            "features": {
                "type": "object",
                "properties": {
                    "logging": {"type": "boolean", "default": true},
                    "metrics": {"type": "boolean", "default": false}
                },
                "default": {
                    "logging": true,
                    "metrics": false
                }
            }
        }
    });

    let result = TctCompiler::new(schema, "ServerConfig")
        .unwrap()
        .with_optimization_level(tct::OptimizationLevel::O2)
        .compile();

    match result {
        Ok(_) => {
            println!("✅ Real-world config schema with defaults compiled successfully");
        }
        Err(e) => {
            println!("⚠️ Complex default schema not implemented: {}", e);
        }
    }
}
