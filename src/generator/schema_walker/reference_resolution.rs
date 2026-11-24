//! Reference Resolution Module
//!
//! Handles $ref resolution and reference-related analysis for JSON Schema.
//! This module provides utilities for resolving $ref paths and for deciding
//! whether a schema, followed through its references, admits null.

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

    /// Whether a schema lists `null` among the instances it accepts.
    ///
    /// True for `"type": "null"`, for a `type` array that contains `"null"`,
    /// for a `oneOf`/`anyOf` with a variant that admits null, and for a
    /// `$ref` to such a schema (followed through chains of references).
    ///
    /// An optional member whose schema admits null must be `OptionalField<T>`:
    /// serde's `Option<T>` deserializer takes null for itself and never
    /// offers it to `T`, so `Option<T>` would reject a null the schema allows.
    pub(super) fn schema_admits_null(&self, schema: &Map<String, Value>) -> bool {
        let mut visited_refs = Vec::new();
        self.schema_admits_null_inner(schema, &mut visited_refs)
    }

    fn schema_admits_null_inner(
        &self,
        schema: &Map<String, Value>,
        visited_refs: &mut Vec<String>,
    ) -> bool {
        // A reference is decided by its target; a reference that cannot be
        // resolved, or that leads back to itself, admits nothing here.
        if let Some(Value::String(ref_path)) = schema.get("$ref") {
            // A sibling `type` array that is honoured next to the reference
            // (see `ref_sibling_type_names`) decides, like any `type` array.
            if let Some(sibling_type_names) = self.ref_sibling_type_names(schema) {
                return sibling_type_names.contains(&"null");
            }
            if visited_refs.iter().any(|visited| visited == ref_path) {
                return false;
            }
            let Some(resolved_schema) = self.resolve_ref(ref_path) else {
                return false;
            };
            visited_refs.push(ref_path.clone());
            let admits_null = self.schema_admits_null_inner(&resolved_schema, visited_refs);
            visited_refs.pop();
            return admits_null;
        }

        match schema.get("type") {
            Some(Value::String(type_name)) if type_name == "null" => return true,
            Some(Value::Array(type_names))
                if type_names
                    .iter()
                    .any(|type_name| type_name.as_str() == Some("null")) =>
            {
                return true;
            }
            _ => {}
        }

        ["oneOf", "anyOf"].into_iter().any(|key| {
            schema
                .get(key)
                .and_then(Value::as_array)
                .is_some_and(|variants| {
                    variants.iter().any(|variant| {
                        variant.as_object().is_some_and(|variant_schema| {
                            self.schema_admits_null_inner(variant_schema, visited_refs)
                        })
                    })
                })
        })
    }
}
