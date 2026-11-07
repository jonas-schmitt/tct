//! Reference Resolution Module
//!
//! Handles $ref resolution and reference-related analysis for JSON Schema.
//! This module provides utilities for resolving $ref paths and checking
//! null variants in referenced types.

use serde_json::{Map, Value};

/// SchemaWalker methods for reference resolution
impl super::SchemaWalker {
    /// Resolve a $ref path in the root schema
    ///
    /// Handles both object keys and array indices (e.g., `items/0` for tuple schemas)
    pub(super) fn resolve_ref(&self, ref_path: &str) -> Option<Map<String, Value>> {
        crate::schema_utils::resolve_local_ref(&self.root_schema, ref_path)
            .and_then(Value::as_object)
            .cloned()
    }

    /// Check if a referenced type has a null variant in its oneOf/anyOf
    ///
    /// This is used to detect when Option<T> needs to be OptionalField<T>
    /// because serde's Option deserializer won't try to deserialize null as T
    pub(super) fn ref_type_has_null_variant(&self, schema: &Map<String, Value>) -> bool {
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
}
