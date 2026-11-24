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
    /// 2. A oneOf/anyOf variant that admits null: {"type": "null"}, a `type`
    ///    array that lists "null", or a `$ref` to a schema that admits null
    /// 3. Reference to a schema that admits null: {"$ref": "#/definitions/X"}
    ///    where X is one of the above, directly or through further references
    ///
    /// Cases 2 and 3 are critical: when a field's type has a null variant,
    /// we must use OptionalField<T> because serde's Option<T> deserializer treats
    /// null specially and won't try to deserialize it as T.
    pub(super) fn schema_allows_null(&self, schema: &Map<String, Value>) -> bool {
        self.schema_admits_null(schema)
    }

    /// Check if schema is a composition (oneOf/anyOf) or simple enum
    ///
    /// Returns true if the schema uses oneOf, anyOf, or enum keywords.
    pub(super) fn is_composition_schema(&self, schema: &Map<String, Value>) -> bool {
        schema.contains_key("oneOf") || schema.contains_key("anyOf") || schema.contains_key("enum")
    }
}
