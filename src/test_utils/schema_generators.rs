//! Schema generation utilities for comprehensive edge case testing
//!
//! This module provides utilities for creating systematic test schemas that
//! cover boundary conditions, composition patterns, nesting structures,
//! and problematic edge cases for thorough testing coverage.

use rustc_hash::FxHashMap;
use serde_json::{Value, json};

/// Type alias for test category function to reduce complexity
type TestCategoryFn = fn() -> Vec<(Value, &'static str, &'static str)>;

/// Schema generation utilities for systematic testing
pub struct SchemaTestUtils;

impl SchemaTestUtils {
    /// Generate schemas that test boundary conditions
    pub fn boundary_condition_schemas() -> Vec<(Value, &'static str, &'static str)> {
        vec![
            // Minimum viable schemas
            (
                json!({"type": "string"}),
                "MinimalString",
                "Simplest possible string schema",
            ),
            (
                json!({"type": "integer"}),
                "MinimalInteger",
                "Simplest possible integer schema",
            ),
            (
                json!({"type": "boolean"}),
                "MinimalBoolean",
                "Simplest possible boolean schema",
            ),
            (
                json!({"type": "null"}),
                "MinimalNull",
                "Simplest possible null schema",
            ),
            // Empty collections
            (
                json!({"type": "object", "properties": {}}),
                "EmptyObject",
                "Object with no properties",
            ),
            (
                json!({"type": "array", "items": {"type": "string"}}),
                "ArraySchema",
                "Simple array schema",
            ),
            // Single-item schemas
            (
                json!({
                    "type": "object",
                    "properties": {
                        "single_field": {"type": "string"}
                    }
                }),
                "SingleFieldObject",
                "Object with exactly one property",
            ),
            // Edge case values
            (
                json!({"type": "number", "minimum": 0, "maximum": 0}),
                "ZeroRange",
                "Number with zero range (min=max=0)",
            ),
            (
                json!({"type": "string", "minLength": 0, "maxLength": 0}),
                "EmptyStringConstraint",
                "String that must be empty",
            ),
            (
                json!({"type": "string", "minLength": 1, "maxLength": 1}),
                "SingleCharString",
                "String that must be exactly one character",
            ),
            // Large constraint values
            (
                json!({"type": "integer", "minimum": 1000000, "maximum": 1000001}),
                "LargeRangeConstraint",
                "Integer with large minimum values",
            ),
            (
                json!({"type": "string", "minLength": 100}),
                "LongStringConstraint",
                "String with large minimum length",
            ),
        ]
    }

    /// Generate schemas that test composition features (oneOf, allOf, anyOf)
    pub fn composition_schemas() -> Vec<(Value, &'static str, &'static str)> {
        vec![
            // Simple oneOf
            (
                json!({
                    "oneOf": [
                        {"type": "string"},
                        {"type": "integer"}
                    ]
                }),
                "SimpleOneOf",
                "Simple oneOf composition",
            ),
            (
                json!({
                    "oneOf": [
                        {"type": "string", "enum": ["a", "b"]},
                        {"type": "integer", "enum": [1, 2]},
                        {"type": "boolean"}
                    ]
                }),
                "EnumOneOf",
                "OneOf with enum constraints",
            ),
            // Complex oneOf
            (
                json!({
                    "oneOf": [
                        {
                            "type": "object",
                            "properties": {
                                "type": {"type": "string", "const": "user"},
                                "name": {"type": "string"}
                            },
                            "required": ["type", "name"]
                        },
                        {
                            "type": "object",
                            "properties": {
                                "type": {"type": "string", "const": "admin"},
                                "permissions": {"type": "array", "items": {"type": "string"}}
                            },
                            "required": ["type", "permissions"]
                        }
                    ]
                }),
                "DiscriminatedUnion",
                "Discriminated union with type field",
            ),
            // AllOf composition
            (
                json!({
                    "allOf": [
                        {"type": "object", "properties": {"name": {"type": "string"}}},
                        {"type": "object", "properties": {"age": {"type": "integer"}}}
                    ]
                }),
                "SimpleAllOf",
                "Simple allOf composition",
            ),
            (
                json!({
                    "allOf": [
                        {"type": "object", "properties": {"base": {"type": "string"}}},
                        {
                            "oneOf": [
                                {"properties": {"variant_a": {"type": "boolean"}}},
                                {"properties": {"variant_b": {"type": "integer"}}}
                            ]
                        }
                    ]
                }),
                "AllOfWithOneOf",
                "AllOf containing oneOf composition",
            ),
        ]
    }

    /// Generate schemas that test nested object structures
    pub fn nesting_schemas() -> Vec<(Value, &'static str, &'static str)> {
        vec![
            // Simple nesting
            (
                json!({
                    "type": "object",
                    "properties": {
                        "user": {
                            "type": "object",
                            "properties": {
                                "name": {"type": "string"},
                                "email": {"type": "string"}
                            }
                        }
                    }
                }),
                "SimpleNesting",
                "Object containing nested object",
            ),
            // Deep nesting
            (
                json!({
                    "type": "object",
                    "properties": {
                        "level1": {
                            "type": "object",
                            "properties": {
                                "level2": {
                                    "type": "object",
                                    "properties": {
                                        "level3": {"type": "string"}
                                    }
                                }
                            }
                        }
                    }
                }),
                "DeepNesting",
                "Three levels of object nesting",
            ),
            // Array of objects
            (
                json!({
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "id": {"type": "integer"},
                            "data": {"type": "string"}
                        }
                    }
                }),
                "ArrayOfObjects",
                "Array containing object items",
            ),
        ]
    }

    /// Generate schemas that test pattern properties and additional properties
    pub fn pattern_property_schemas() -> Vec<(Value, &'static str, &'static str)> {
        vec![
            // Simple pattern
            (
                json!({
                    "type": "object",
                    "patternProperties": {
                        "^str_": {"type": "string"},
                        "^num_": {"type": "number"}
                    },
                    "additionalProperties": false
                }),
                "SimplePattern",
                "Simple pattern property",
            ),
            // Complex pattern with additional properties
            (
                json!({
                    "type": "object",
                    "properties": {
                        "fixed_field": {"type": "string"}
                    },
                    "patternProperties": {
                        "^dynamic_": {
                            "type": "object",
                            "properties": {
                                "value": {"type": "string"},
                                "metadata": {"type": "object"}
                            }
                        }
                    },
                    "additionalProperties": {"type": "boolean"}
                }),
                "ComplexPattern",
                "Pattern with mixed property types",
            ),
            // Pattern with enum constraint
            (
                json!({
                    "type": "object",
                    "patternProperties": {
                        "^config_": {
                            "type": "string",
                            "enum": ["enabled", "disabled", "auto"]
                        }
                    }
                }),
                "PatternEnum",
                "Pattern property with enum constraint",
            ),
        ]
    }

    /// Generate schemas that test various constraint types
    pub fn constraint_schemas() -> Vec<(Value, &'static str, &'static str)> {
        vec![
            // String constraints
            (
                json!({
                    "type": "string",
                    "minLength": 5,
                    "maxLength": 10,
                    "pattern": "^[a-z]+$"
                }),
                "StringConstraints",
                "String with multiple constraints",
            ),
            // Number constraints
            (
                json!({
                    "type": "number",
                    "minimum": 0,
                    "maximum": 100,
                    "multipleOf": 5
                }),
                "NumberConstraints",
                "Number with range and multiple constraints",
            ),
            // Array constraints
            (
                json!({
                    "type": "array",
                    "minItems": 1,
                    "maxItems": 5,
                    "uniqueItems": true,
                    "items": {"type": "string", "enum": ["a", "b", "c", "d", "e"]}
                }),
                "ArrayConstraints",
                "Array with size and uniqueness constraints",
            ),
            // Object constraints
            (
                json!({
                    "type": "object",
                    "properties": {
                        "required_field": {"type": "string"},
                        "optional_field": {"type": "integer"}
                    },
                    "required": ["required_field"],
                    "additionalProperties": false,
                    "minProperties": 1,
                    "maxProperties": 2
                }),
                "ObjectConstraints",
                "Object with property count constraints",
            ),
            // Enum constraints
            (
                json!({
                    "type": "string",
                    "enum": ["option1", "option2", "option3"]
                }),
                "StringEnum",
                "String with enum constraint",
            ),
            (
                json!({
                    "type": "integer",
                    "enum": [1, 5, 10, 25, 50]
                }),
                "IntegerEnum",
                "Integer with enum constraint",
            ),
        ]
    }

    /// Generate schemas that are known to be problematic or edge cases
    pub fn problematic_schemas() -> Vec<(Value, &'static str, &'static str)> {
        vec![
            // Empty schema
            (json!({}), "EmptySchema", "Completely empty schema object"),
            // Schema with only description
            (
                json!({"description": "A schema with only metadata"}),
                "MetadataOnly",
                "Schema with only description field",
            ),
            // Contradictory constraints
            (
                json!({
                    "type": "string",
                    "minLength": 10,
                    "maxLength": 5
                }),
                "ContradictoryString",
                "String with impossible length constraints",
            ),
            (
                json!({
                    "type": "number",
                    "minimum": 100,
                    "maximum": 50
                }),
                "ContradictoryNumber",
                "Number with impossible range constraints",
            ),
            // Complex contradictory schema
            (
                json!({
                    "allOf": [
                        {"type": "string"},
                        {"type": "integer"}
                    ]
                }),
                "ImpossibleAllOf",
                "AllOf requiring impossible type combination",
            ),
            // Large enum
            (
                json!({
                    "type": "string",
                    "enum": (0..1000).map(|i| format!("value_{}", i)).collect::<Vec<_>>()
                }),
                "LargeEnum",
                "String enum with 1000 values",
            ),
            // Complex nesting with potential circular reference patterns
            (
                json!({
                    "type": "object",
                    "properties": {
                        "self_ref": {
                            "type": "object",
                            "properties": {
                                "nested": {
                                    "type": "object",
                                    "properties": {
                                        "deeply_nested": {"type": "string"}
                                    }
                                }
                            }
                        }
                    }
                }),
                "DeepSelfReference",
                "Deeply nested structure that could reference itself",
            ),
        ]
    }

    /// Generate a comprehensive test suite combining all schema types
    pub fn comprehensive_test_suite() -> Vec<(Value, &'static str, &'static str)> {
        let mut suite = Vec::new();

        suite.extend(Self::boundary_condition_schemas());
        suite.extend(Self::composition_schemas());
        suite.extend(Self::nesting_schemas());
        suite.extend(Self::pattern_property_schemas());
        suite.extend(Self::constraint_schemas());
        suite.extend(Self::problematic_schemas());

        suite
    }

    /// Get test categories for organized testing
    pub fn test_categories() -> FxHashMap<&'static str, TestCategoryFn> {
        let mut categories = FxHashMap::default();

        categories.insert(
            "boundary_conditions",
            Self::boundary_condition_schemas as fn() -> _,
        );
        categories.insert("composition", Self::composition_schemas as fn() -> _);
        categories.insert("nesting", Self::nesting_schemas as fn() -> _);
        categories.insert("patterns", Self::pattern_property_schemas as fn() -> _);
        categories.insert("constraints", Self::constraint_schemas as fn() -> _);
        categories.insert("problematic", Self::problematic_schemas as fn() -> _);

        categories
    }

    /// Validate a test schema result and categorize any errors
    pub fn categorize_test_result(
        result: &Result<crate::grammar::Grammar, crate::TctError>,
        _schema_name: &str,
    ) -> (&'static str, String) {
        match result {
            Ok(grammar) => match grammar.generate_code() {
                Ok(_) => ("Success", "Schema processed successfully".to_string()),
                Err(e) => {
                    let error_msg = e.to_string();
                    let category = if error_msg.contains("empty") {
                        "Empty Identifier"
                    } else if error_msg.contains("invalid") {
                        "Invalid Syntax"
                    } else if error_msg.contains("not a valid Ident") {
                        "Invalid Identifier"
                    } else {
                        "Code Generation Error"
                    };
                    (category, error_msg)
                }
            },
            Err(e) => {
                let error_msg = e.to_string();
                let category = if error_msg.contains("missing") {
                    "Missing Properties"
                } else if error_msg.contains("malformed") {
                    "Malformed Schema"
                } else if error_msg.contains("Unsupported") {
                    "Unsupported Feature"
                } else if error_msg.contains("Circular reference") {
                    "Circular Reference"
                } else {
                    "Schema Processing Error"
                };
                (category, error_msg)
            }
        }
    }
}
