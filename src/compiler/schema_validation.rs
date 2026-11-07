//! Schema Validation and Analysis Utilities
//!
//! This module provides functions for validating JSON schemas and analyzing
//! their optimization potential.

use super::schema_processors::{DomainTypesProcessor, OptimizableFieldsProcessor};
use crate::error::{TctError, TctResult};
use crate::schema_utils::traverse_schema;
use serde_json::Value;

/// Validate the provided JSON schema for basic structural correctness
pub fn validate_schema(schema: &Value) -> TctResult<()> {
    // Basic schema validation - must be a valid JSON Schema
    match schema {
        Value::Object(obj) => {
            // A valid schema can have 'type', 'oneOf', 'anyOf', 'allOf', '$ref', etc.
            // We only require that it's not completely empty or has some schema keyword
            let has_schema_keyword = obj.contains_key("type")
                || obj.contains_key("oneOf")
                || obj.contains_key("anyOf")
                || obj.contains_key("allOf")
                || obj.contains_key("$ref")
                || obj.contains_key("enum")
                || obj.contains_key("const")
                || obj.contains_key("properties")
                || obj.contains_key("items");

            if !has_schema_keyword {
                return Err(TctError::SchemaValidation {
                    message: "Schema must have at least one schema keyword".to_string(),
                    path: "root".to_string(),
                    suggestion: "Add a 'type' field or other JSON Schema keyword like 'oneOf', 'properties', etc.".to_string(),
                    context: None,
                });
            }
        }
        _ => {
            return Err(TctError::SchemaValidation {
                message: "Schema must be a JSON object".to_string(),
                path: "root".to_string(),
                suggestion: "Provide a valid JSON Schema object".to_string(),
                context: None,
            });
        }
    }
    Ok(())
}

/// Check if schema has content that can be optimized (enums, bounded integers, etc.)
pub fn schema_has_optimizable_content(schema: &Value) -> bool {
    // Quick heuristic check for optimizable content
    if let Some(obj) = schema.as_object() {
        // Check for enums
        if obj.contains_key("enum") {
            return true;
        }

        // Check for bounded integers
        if obj.get("type").and_then(|t| t.as_str()) == Some("integer")
            && (obj.contains_key("minimum") || obj.contains_key("maximum"))
        {
            return true;
        }

        // Recursively check properties
        if let Some(props) = obj.get("properties").and_then(|p| p.as_object()) {
            for prop_schema in props.values() {
                if schema_has_optimizable_content(prop_schema) {
                    return true;
                }
            }
        }

        // Check array items
        if let Some(items) = obj.get("items")
            && schema_has_optimizable_content(items)
        {
            return true;
        }
    }

    false
}

/// Count the number of optimizable fields in a schema
pub fn count_optimizable_fields(schema: &Value) -> usize {
    let mut processor = OptimizableFieldsProcessor::new();
    traverse_schema(schema, &mut processor, "").0
}

/// Count the total number of constrained values across all optimizable fields
pub fn count_constrained_values(schema: &Value) -> usize {
    let mut processor = OptimizableFieldsProcessor::new();
    traverse_schema(schema, &mut processor, "").1
}

/// Count domain types (enums vs bounded integers) in the schema
/// Returns (enum_count, integer_count)
pub fn count_domain_types(schema: &Value) -> (usize, usize) {
    let mut processor = DomainTypesProcessor::new();
    traverse_schema(schema, &mut processor, "")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // ========== Batch 1: validate_schema tests ==========

    #[test]
    fn test_validate_schema_with_type() {
        let schema = json!({ "type": "string" });
        assert!(validate_schema(&schema).is_ok());
    }

    #[test]
    fn test_validate_schema_with_oneof() {
        let schema = json!({ "oneOf": [{"type": "string"}, {"type": "number"}] });
        assert!(validate_schema(&schema).is_ok());
    }

    #[test]
    fn test_validate_schema_with_anyof() {
        let schema = json!({ "anyOf": [{"type": "string"}, {"type": "number"}] });
        assert!(validate_schema(&schema).is_ok());
    }

    #[test]
    fn test_validate_schema_with_allof() {
        let schema = json!({ "allOf": [{"type": "object"}, {"properties": {}}] });
        assert!(validate_schema(&schema).is_ok());
    }

    #[test]
    fn test_validate_schema_with_ref() {
        let schema = json!({ "$ref": "#/definitions/MyType" });
        assert!(validate_schema(&schema).is_ok());
    }

    #[test]
    fn test_validate_schema_with_enum() {
        let schema = json!({ "enum": ["a", "b", "c"] });
        assert!(validate_schema(&schema).is_ok());
    }

    #[test]
    fn test_validate_schema_with_const() {
        let schema = json!({ "const": "fixed-value" });
        assert!(validate_schema(&schema).is_ok());
    }

    #[test]
    fn test_validate_schema_with_properties() {
        let schema = json!({ "properties": { "name": { "type": "string" } } });
        assert!(validate_schema(&schema).is_ok());
    }

    #[test]
    fn test_validate_schema_with_items() {
        let schema = json!({ "items": { "type": "string" } });
        assert!(validate_schema(&schema).is_ok());
    }

    #[test]
    fn test_validate_schema_not_object() {
        // Schema must be an object
        let schema = json!("string-schema");
        let result = validate_schema(&schema);
        assert!(result.is_err());
        match result {
            Err(TctError::SchemaValidation { message, .. }) => {
                assert!(message.contains("must be a JSON object"));
            }
            _ => panic!("Expected SchemaValidation error"),
        }
    }

    #[test]
    fn test_validate_schema_empty_object() {
        // Empty object without any schema keywords
        let schema = json!({});
        let result = validate_schema(&schema);
        assert!(result.is_err());
        match result {
            Err(TctError::SchemaValidation { message, .. }) => {
                assert!(message.contains("at least one schema keyword"));
            }
            _ => panic!("Expected SchemaValidation error"),
        }
    }

    #[test]
    fn test_validate_schema_object_without_keywords() {
        // Object with non-schema properties
        let schema = json!({ "description": "A schema without keywords" });
        let result = validate_schema(&schema);
        assert!(result.is_err());
    }

    // ========== Batch 2: schema_has_optimizable_content tests ==========

    #[test]
    fn test_schema_has_optimizable_content_with_enum() {
        let schema = json!({ "enum": ["red", "green", "blue"] });
        assert!(schema_has_optimizable_content(&schema));
    }

    #[test]
    fn test_schema_has_optimizable_content_with_minimum() {
        let schema = json!({ "type": "integer", "minimum": 0 });
        assert!(schema_has_optimizable_content(&schema));
    }

    #[test]
    fn test_schema_has_optimizable_content_with_maximum() {
        let schema = json!({ "type": "integer", "maximum": 100 });
        assert!(schema_has_optimizable_content(&schema));
    }

    #[test]
    fn test_schema_has_optimizable_content_with_min_and_max() {
        let schema = json!({ "type": "integer", "minimum": 0, "maximum": 100 });
        assert!(schema_has_optimizable_content(&schema));
    }

    #[test]
    fn test_schema_has_optimizable_content_in_properties() {
        let schema = json!({
            "type": "object",
            "properties": {
                "status": { "enum": ["pending", "active", "done"] }
            }
        });
        assert!(schema_has_optimizable_content(&schema));
    }

    #[test]
    fn test_schema_has_optimizable_content_in_nested_properties() {
        let schema = json!({
            "type": "object",
            "properties": {
                "outer": {
                    "type": "object",
                    "properties": {
                        "inner": { "type": "integer", "minimum": 1, "maximum": 10 }
                    }
                }
            }
        });
        assert!(schema_has_optimizable_content(&schema));
    }

    #[test]
    fn test_schema_has_optimizable_content_in_items() {
        let schema = json!({
            "type": "array",
            "items": { "enum": ["a", "b", "c"] }
        });
        assert!(schema_has_optimizable_content(&schema));
    }

    #[test]
    fn test_schema_has_optimizable_content_no_optimization() {
        let schema = json!({ "type": "string" });
        assert!(!schema_has_optimizable_content(&schema));
    }

    #[test]
    fn test_schema_has_optimizable_content_integer_without_bounds() {
        // Integer without minimum/maximum is not optimizable
        let schema = json!({ "type": "integer" });
        assert!(!schema_has_optimizable_content(&schema));
    }

    #[test]
    fn test_schema_has_optimizable_content_non_object() {
        // Non-object schemas are not optimizable
        let schema = json!(null);
        assert!(!schema_has_optimizable_content(&schema));

        let schema = json!("string");
        assert!(!schema_has_optimizable_content(&schema));

        let schema = json!(42);
        assert!(!schema_has_optimizable_content(&schema));
    }

    // ========== Batch 3: count_optimizable_fields tests ==========

    #[test]
    fn test_count_optimizable_fields_empty() {
        let schema = json!({ "type": "string" });
        let count = count_optimizable_fields(&schema);
        // String has no optimizable fields
        assert_eq!(count, 0);
    }

    #[test]
    fn test_count_optimizable_fields_single_enum() {
        let schema = json!({ "enum": ["a", "b", "c"] });
        let count = count_optimizable_fields(&schema);
        // One enum field
        assert_eq!(count, 1);
    }

    #[test]
    fn test_count_optimizable_fields_with_properties() {
        let schema = json!({
            "type": "object",
            "properties": {
                "status": { "enum": ["pending", "active", "done"] },
                "priority": { "type": "integer", "minimum": 1, "maximum": 5 }
            }
        });
        let count = count_optimizable_fields(&schema);
        // Two optimizable fields
        assert_eq!(count, 2);
    }

    #[test]
    fn test_count_optimizable_fields_nested() {
        let schema = json!({
            "type": "object",
            "properties": {
                "outer": {
                    "type": "object",
                    "properties": {
                        "inner": { "enum": ["x", "y", "z"] }
                    }
                }
            }
        });
        let count = count_optimizable_fields(&schema);
        // One nested enum field
        assert_eq!(count, 1);
    }

    // ========== Batch 4: count_constrained_values tests ==========

    #[test]
    fn test_count_constrained_values_empty() {
        let schema = json!({ "type": "string" });
        let count = count_constrained_values(&schema);
        assert_eq!(count, 0);
    }

    #[test]
    fn test_count_constrained_values_single_enum() {
        let schema = json!({ "enum": ["a", "b", "c"] });
        let count = count_constrained_values(&schema);
        // Three enum values
        assert_eq!(count, 3);
    }

    #[test]
    fn test_count_constrained_values_bounded_integer() {
        let schema = json!({ "type": "integer", "minimum": 0, "maximum": 10 });
        let count = count_constrained_values(&schema);
        // 11 values (0 through 10 inclusive)
        assert_eq!(count, 11);
    }

    #[test]
    fn test_count_constrained_values_multiple_fields() {
        let schema = json!({
            "type": "object",
            "properties": {
                "status": { "enum": ["a", "b"] },
                "count": { "type": "integer", "minimum": 0, "maximum": 4 }
            }
        });
        let count = count_constrained_values(&schema);
        // 2 (enum) + 5 (integer 0-4) = 7 values
        assert_eq!(count, 7);
    }

    // ========== Batch 5: count_domain_types tests ==========

    #[test]
    fn test_count_domain_types_empty() {
        let schema = json!({ "type": "string" });
        let (enum_count, int_count) = count_domain_types(&schema);
        assert_eq!(enum_count, 0);
        assert_eq!(int_count, 0);
    }

    #[test]
    fn test_count_domain_types_single_enum() {
        let schema = json!({ "enum": ["a", "b", "c"] });
        let (enum_count, int_count) = count_domain_types(&schema);
        assert_eq!(enum_count, 1);
        assert_eq!(int_count, 0);
    }

    #[test]
    fn test_count_domain_types_single_integer() {
        let schema = json!({ "type": "integer", "minimum": 0, "maximum": 100 });
        let (enum_count, int_count) = count_domain_types(&schema);
        assert_eq!(enum_count, 0);
        assert_eq!(int_count, 1);
    }

    #[test]
    fn test_count_domain_types_both() {
        let schema = json!({
            "type": "object",
            "properties": {
                "status": { "enum": ["pending", "active"] },
                "priority": { "type": "integer", "minimum": 1, "maximum": 5 }
            }
        });
        let (enum_count, int_count) = count_domain_types(&schema);
        assert_eq!(enum_count, 1);
        assert_eq!(int_count, 1);
    }

    #[test]
    fn test_count_domain_types_multiple_enums() {
        let schema = json!({
            "type": "object",
            "properties": {
                "status": { "enum": ["a", "b"] },
                "color": { "enum": ["red", "green", "blue"] }
            }
        });
        let (enum_count, int_count) = count_domain_types(&schema);
        assert_eq!(enum_count, 2);
        assert_eq!(int_count, 0);
    }

    #[test]
    fn test_count_domain_types_multiple_integers() {
        let schema = json!({
            "type": "object",
            "properties": {
                "width": { "type": "integer", "minimum": 0, "maximum": 1920 },
                "height": { "type": "integer", "minimum": 0, "maximum": 1080 }
            }
        });
        let (enum_count, int_count) = count_domain_types(&schema);
        assert_eq!(enum_count, 0);
        assert_eq!(int_count, 2);
    }

    #[test]
    fn test_count_domain_types_nested() {
        let schema = json!({
            "type": "object",
            "properties": {
                "outer": {
                    "type": "object",
                    "properties": {
                        "inner_enum": { "enum": ["x", "y"] },
                        "inner_int": { "type": "integer", "minimum": 0, "maximum": 10 }
                    }
                }
            }
        });
        let (enum_count, int_count) = count_domain_types(&schema);
        assert_eq!(enum_count, 1);
        assert_eq!(int_count, 1);
    }
}
