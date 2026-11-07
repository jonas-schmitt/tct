//! Advisory schema warning collection for compiler diagnostics.
//!
//! These checks do not change schema compilation or token layout. They surface
//! schema shapes that are likely to reduce tokenizer usefulness, such as
//! unconstrained `additionalProperties`, while keeping the warning traversal
//! separate from discovery, optimization, and generation.

use super::TctCompiler;
use crate::warnings::{SchemaWarning, SchemaWarnings};
use serde_json::Value;

impl TctCompiler {
    /// Detect schema quality warnings.
    pub(super) fn detect_warnings(&self) -> Option<SchemaWarnings> {
        let mut warnings = SchemaWarnings::new();

        // Recursively check for unconstrained additionalProperties
        self.check_additional_properties(&self.schema, "#", &mut warnings);

        if warnings.is_empty() {
            None
        } else {
            Some(warnings)
        }
    }

    /// Recursively check schema for unconstrained additionalProperties.
    fn check_additional_properties(
        &self,
        schema: &Value,
        path: &str,
        warnings: &mut SchemaWarnings,
    ) {
        if let Some(obj) = schema.as_object() {
            // Check if this object has additionalProperties: true
            if let Some(additional_props) = obj.get("additionalProperties") {
                // Check if it's exactly true (boolean true, not a schema object)
                if additional_props.as_bool() == Some(true) {
                    // This is unconstrained - add warning
                    warnings.add(SchemaWarning::unconstrained_additional_properties(
                        path.to_string(),
                        self.optimization_level,
                    ));
                }
            }

            // Recursively check properties
            if let Some(properties) = obj.get("properties").and_then(|v| v.as_object()) {
                for (prop_name, prop_schema) in properties {
                    let prop_path = format!("{}/properties/{}", path, prop_name);
                    self.check_additional_properties(prop_schema, &prop_path, warnings);
                }
            }

            // Recursively check definitions
            if let Some(definitions) = obj.get("definitions").and_then(|v| v.as_object()) {
                for (def_name, def_schema) in definitions {
                    let def_path = format!("{}/definitions/{}", path, def_name);
                    self.check_additional_properties(def_schema, &def_path, warnings);
                }
            }

            // Recursively check items (for arrays)
            if let Some(items) = obj.get("items") {
                let items_path = format!("{}/items", path);
                self.check_additional_properties(items, &items_path, warnings);
            }

            // Recursively check allOf/anyOf/oneOf
            for key in &["allOf", "anyOf", "oneOf"] {
                if let Some(schemas) = obj.get(*key).and_then(|v| v.as_array()) {
                    for (i, schema) in schemas.iter().enumerate() {
                        let sub_path = format!("{}/{}/{}", path, key, i);
                        self.check_additional_properties(schema, &sub_path, warnings);
                    }
                }
            }
        }
    }
}
