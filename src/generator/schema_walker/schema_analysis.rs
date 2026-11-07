//! Schema Analysis Module
//!
//! Provides utilities for analyzing JSON Schema properties such as null handling,
//! composition detection, and type characteristics.

use serde_json::{Map, Value};

/// SchemaWalker methods for schema property analysis
impl super::SchemaWalker {
    /// Check if a schema explicitly allows null values at the FIELD level
    ///
    /// Returns true if:
    /// 1. Direct null: "type": "null" or "type": [..., "null", ...]
    /// 2. Direct null in oneOf/anyOf: {"oneOf": [... {"type": "null"} ...]}
    /// 3. Reference to type with null variant: {"$ref": "#/definitions/X"} where X has null variant
    ///
    /// Case 3 is critical: when a field has $ref to a type with null variant,
    /// we must use OptionalField<T> because serde's Option<T> deserializer treats
    /// null specially and won't try to deserialize it as T.
    pub(super) fn schema_allows_null(&self, schema: &Map<String, Value>) -> bool {
        // Check if schema has a $ref - if so, check if the referenced type has null variant
        if let Some(Value::String(ref_path)) = schema.get("$ref") {
            if let Some(resolved_schema) = self.resolve_ref(ref_path) {
                // Check if the referenced type has a null variant
                return self.ref_type_has_null_variant(&resolved_schema);
            }
            // If $ref couldn't be resolved, conservatively return false
            return false;
        }

        // Check direct type field
        if let Some(type_value) = schema.get("type") {
            match type_value {
                Value::String(s) if s == "null" => return true,
                Value::Array(arr) if arr.iter().any(|v| v.as_str() == Some("null")) => {
                    return true;
                }
                _ => {}
            }
        }

        // Check oneOf/anyOf for null variant
        for key in ["oneOf", "anyOf"] {
            if let Some(Value::Array(variants)) = schema.get(key) {
                for variant in variants {
                    if let Some(variant_obj) = variant.as_object() {
                        // Check if this variant is {"type": "null"}
                        if let Some(Value::String(s)) = variant_obj.get("type")
                            && s == "null"
                        {
                            return true;
                        }
                    }
                }
            }
        }

        false
    }

    /// Check if schema is a composition (oneOf/anyOf) or simple enum
    ///
    /// Returns true if the schema uses oneOf, anyOf, or enum keywords.
    pub(super) fn is_composition_schema(&self, schema: &Map<String, Value>) -> bool {
        schema.contains_key("oneOf") || schema.contains_key("anyOf") || schema.contains_key("enum")
    }
}
