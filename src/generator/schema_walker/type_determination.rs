//! Type Determination Module
//!
//! Handles type inference and member type resolution from JSON Schema.
//! This is the core logic that maps JSON Schema constructs to Rust types.

mod member_extraction;

use crate::generator::schema_discovery::{MemberType, TypeKind};
use crate::schema_utils::{local_definition_ref_target, local_ref_segments};
use crate::utils::{to_type_name, to_upper_camel_case};
use crate::{TctError, TctResult};
use serde_json::{Map, Value};

fn primitive_type_for_const_value(const_value: &Value) -> String {
    if const_value.is_boolean() {
        "bool".to_string()
    } else if const_value.as_i64().is_some() {
        "i64".to_string()
    } else if const_value.as_u64().is_some() {
        "serde_json::Value".to_string()
    } else if const_value.as_f64().is_some() {
        "f64".to_string()
    } else {
        "serde_json::Value".to_string()
    }
}

fn integer_schema_has_i64_bounds(schema: &Map<String, Value>) -> bool {
    schema.get("minimum").and_then(Value::as_i64).is_some()
        && schema.get("maximum").and_then(Value::as_i64).is_some()
}

fn rust_type_for_integer_schema(schema: &Map<String, Value>) -> String {
    if integer_schema_has_i64_bounds(schema) {
        "i64".to_string()
    } else {
        "JsonInteger".to_string()
    }
}

fn local_ref_type_target(ref_str: &str) -> Option<String> {
    let segments = local_ref_segments(ref_str)?;
    if segments.is_empty() {
        return None;
    }
    Some(segments.join("_"))
}

pub(super) fn member_type_as_rust_value_type(member_type: MemberType) -> String {
    match member_type {
        MemberType::Primitive(inner) | MemberType::TypeReference(inner) => inner,
        MemberType::Container { container, inner }
        | MemberType::ComplexContainer { container, inner } => {
            format!("{container}<{inner}>")
        }
        MemberType::HashMap { key, value } => {
            format!("rustc_hash::FxHashMap<{key}, {value}>")
        }
        MemberType::PatternProperties { .. } => "serde_json::Value".to_string(),
    }
}

/// SchemaWalker methods for type determination and inference
impl super::SchemaWalker {
    /// Determine type kind from schema
    ///
    /// This is the core type inference logic that classifies schemas into:
    /// - References ($ref)
    /// - Enums (oneOf/anyOf/enum/type arrays)
    /// - Structs (objects with properties)
    /// - Primitives (string/integer/number/boolean)
    /// - Arrays (with items)
    pub(super) fn determine_type_kind(&self, schema: &Map<String, Value>) -> TctResult<TypeKind> {
        // Check for references first
        if let Some(ref_str) = schema.get("$ref").and_then(Value::as_str) {
            if let Some(target) = local_definition_ref_target(ref_str) {
                // Local definition reference
                return Ok(TypeKind::Reference {
                    target: to_type_name(&target),
                });
            } else if ref_str.starts_with("http://") || ref_str.starts_with("https://") {
                // External URL reference - treat as unknown/any type
                // We can't fetch external schemas, so use a generic struct type
                crate::log_debug!("⚠️ External $ref '{}' treated as generic struct", ref_str);
                return Ok(TypeKind::Struct);
            } else if ref_str == "#" {
                // Root reference - treat as recursive reference
                // This is a valid JSON Schema pattern for recursive structures
                return Ok(TypeKind::Struct);
            } else if let Some(target) = local_ref_type_target(ref_str) {
                // Other local reference (e.g., #/properties/foo)
                return Ok(TypeKind::Reference {
                    target: to_type_name(&target),
                });
            } else {
                // Unknown reference format - this is an error
                return Err(TctError::schema_validation(
                    format!("Invalid $ref format: '{}'", ref_str),
                    "$ref",
                    "JSON Schema $ref must be a valid URI reference (e.g., '#/definitions/MyType' or 'http://example.com/schema.json')",
                ));
            }
        }

        // Check for compositions and simple enums
        if schema.contains_key("oneOf") || schema.contains_key("anyOf") {
            return Ok(TypeKind::Enum);
        }

        // Check for allOf composition
        // allOf schemas should be treated as structs that merge properties from multiple subschemas
        if schema.contains_key("allOf") {
            return Ok(TypeKind::NestedStruct);
        }

        // A string const is equivalent to a single-value string enum.
        // Other JSON const values are represented by zero-token unit wrappers
        // so public construction cannot carry a different payload.
        if let Some(const_value) = schema.get("const") {
            if const_value.is_string() {
                return Ok(TypeKind::Enum);
            }
            return Ok(TypeKind::PrimitiveWrapper {
                primitive_type: primitive_type_for_const_value(const_value),
            });
        }

        // Check for simple string enums: {"enum": ["value1", "value2", ...]}
        if let Some(enum_values) = schema.get("enum").and_then(Value::as_array)
            && enum_values.iter().all(|v| v.is_string())
        {
            // Verify it's an array of strings (simple enum)
            return Ok(TypeKind::Enum);
        }

        // Check for type arrays (e.g., "type": ["object", "string"])
        // Per JSON Schema spec, type can be a string OR an array of strings
        if let Some(type_array) = schema.get("type").and_then(Value::as_array)
            && type_array.len() > 1
        {
            // Type is an array like ["object", "string"].
            // Multiple types → treat as enum (oneOf the different types).
            crate::log_debug!(
                "   ✓ Type array detected with {} types, treating as enum",
                type_array.len()
            );
            return Ok(TypeKind::Enum);
        }

        // Check for objects and primitives
        if let Some(type_str) = schema.get("type").and_then(Value::as_str) {
            match type_str {
                "object" => {
                    // Check if this is a minimal object definition without meaningful properties
                    let has_properties = schema
                        .get("properties")
                        .and_then(Value::as_object)
                        .map(|props| !props.is_empty())
                        .unwrap_or(false);

                    let has_required = schema
                        .get("required")
                        .and_then(Value::as_array)
                        .map(|req| !req.is_empty())
                        .unwrap_or(false);

                    let has_pattern_properties = schema
                        .get("patternProperties")
                        .and_then(Value::as_object)
                        .map(|props| !props.is_empty())
                        .unwrap_or(false);

                    let has_additional_properties_schema = schema
                        .get("additionalProperties")
                        .map(|v| v.is_object())
                        .unwrap_or(false);

                    let allows_additional = schema
                        .get("additionalProperties")
                        .map(|v| v.as_bool().unwrap_or(true) || v.is_object())
                        .unwrap_or(true); // Default to true if not specified

                    // If object has no properties, no required fields, no pattern properties, no additional properties schema,
                    // but allows additional properties, treat it as a minimal object that should use generic JSON handling
                    if !has_properties
                        && !has_required
                        && !has_pattern_properties
                        && !has_additional_properties_schema
                        && allows_additional
                    {
                        return Ok(TypeKind::MinimalObjectWrapper);
                    }

                    return Ok(TypeKind::Struct);
                }
                "array" if schema.contains_key("items") => {
                    // All arrays should be handled as ArrayOfComplex type kind.
                    // This includes both arrays of complex objects and arrays of primitives.
                    return Ok(TypeKind::ArrayOfComplex);
                }
                "string" => {
                    return Ok(TypeKind::PrimitiveWrapper {
                        primitive_type: "String".to_string(),
                    });
                }
                "number" => {
                    // Use serde_json::Number to preserve integer vs float representation
                    return Ok(TypeKind::PrimitiveWrapper {
                        primitive_type: "serde_json::Number".to_string(),
                    });
                }
                "integer" => {
                    return Ok(TypeKind::PrimitiveWrapper {
                        primitive_type: rust_type_for_integer_schema(schema),
                    });
                }
                "boolean" => {
                    return Ok(TypeKind::PrimitiveWrapper {
                        primitive_type: "bool".to_string(),
                    });
                }
                _ => {}
            }
        }

        // Default to nested struct for inline objects
        Ok(TypeKind::NestedStruct)
    }

    /// Create unique nested type name using property name when available
    pub(super) fn create_nested_type_name(
        &self,
        parent_type: &str,
        property_name: Option<&str>,
    ) -> String {
        if let Some(prop_name) = property_name {
            let sanitized = to_upper_camel_case(prop_name);

            // Check if sanitization produced a valid, non-reserved type name component
            if !sanitized.is_empty()
                && sanitized != "_"
                && !sanitized.starts_with('_')
                && sanitized.chars().next().is_some_and(|c| c.is_alphabetic())
            {
                // Normal case: use sanitized property name for readability
                return format!("{}{}", parent_type, sanitized);
            }

            // Edge case: sanitization failed (special characters, reserved identifiers)
            // Map common special characters to readable names
            let special_name = match prop_name {
                "." => "Dot".to_string(),
                "*" => "Star".to_string(),
                "^" => "Caret".to_string(),
                "$" => "Dollar".to_string(),
                "^_" => "CaretUnderscore".to_string(),
                _ if prop_name.starts_with('_') => {
                    // Starts with underscore - prefix with "Field"
                    let trimmed = sanitized.trim_start_matches('_');
                    if trimmed.is_empty() {
                        // All underscores - use length as disambiguator
                        format!("Field{}", prop_name.len())
                    } else {
                        format!("Field{}", trimmed)
                    }
                }
                _ => {
                    // Last resort: prefix with "Prop" and use first char hex
                    format!("Prop{:02x}", prop_name.chars().next().unwrap_or('_') as u32)
                }
            };
            format!("{}{}", parent_type, special_name)
        } else {
            // No property name context - fallback to generic naming
            format!("{}Nested", parent_type)
        }
    }

    /// Determine member type from property schema
    ///
    /// This is the most complex type inference function that handles:
    /// - References ($ref)
    /// - Inline enums, string consts, and bounded integers (extraction to separate types)
    /// - Primitives (string/integer/number/boolean/null)
    /// - Arrays (Vec<T>)
    /// - Objects (structs or HashMap)
    /// - Compositions (allOf/oneOf/anyOf)
    pub(super) fn determine_member_type(
        &mut self,
        schema: &Map<String, Value>,
        parent_type: &str,
        property_name: Option<&str>,
    ) -> TctResult<MemberType> {
        // Handle references
        if let Some(ref_str) = schema.get("$ref").and_then(Value::as_str) {
            if let Some(target) = local_definition_ref_target(ref_str) {
                // Local definition reference
                return Ok(MemberType::TypeReference(to_type_name(&target)));
            } else if ref_str.starts_with("http://") || ref_str.starts_with("https://") {
                // External URL reference - treat as serde_json::Value
                crate::log_debug!(
                    "⚠️ External $ref '{}' in member, using serde_json::Value",
                    ref_str
                );
                return Ok(MemberType::Primitive("serde_json::Value".to_string()));
            } else if ref_str == "#" {
                // Root reference - treat as recursive reference
                // This is a valid JSON Schema pattern for recursive structures
                return Ok(MemberType::Primitive("serde_json::Value".to_string()));
            } else if let Some(target) = local_ref_type_target(ref_str) {
                // Other local reference (e.g., #/properties/foo)
                return Ok(MemberType::TypeReference(to_type_name(&target)));
            } else {
                // Unknown reference format - this is an error
                return Err(TctError::schema_validation(
                    format!("Invalid $ref format: '{}'", ref_str),
                    "$ref",
                    "JSON Schema $ref must be a valid URI reference (e.g., '#/definitions/MyType' or 'http://example.com/schema.json')",
                ));
            }
        }

        // Check for string const BEFORE handling as primitives. A string const
        // is a single-value string enum and must not fall back to Value/String.
        if let Some(const_value) = schema.get("const") {
            if let Some(const_str) = const_value.as_str() {
                let enum_values = [Value::String(const_str.to_string())];
                return self.extract_inline_string_enum_type(
                    schema,
                    &enum_values,
                    parent_type,
                    property_name,
                    "String const",
                );
            }
            return self.extract_inline_json_const_type(schema, parent_type, property_name);
        }

        // Check for inline enums BEFORE handling as primitives
        // Inline enums like {"type": "string", "enum": ["a", "b", "c"]} should be extracted as separate types
        if let Some(enum_values) = schema.get("enum").and_then(Value::as_array) {
            // Verify it's a valid inline enum (all values are strings)
            if enum_values.iter().all(|v| v.is_string()) && !enum_values.is_empty() {
                return self.extract_inline_string_enum_type(
                    schema,
                    enum_values,
                    parent_type,
                    property_name,
                    "Inline enum",
                );
            }
        }

        // Check for bounded integers BEFORE handling as primitives
        // Bounded integers like {"type": "integer", "minimum": 1, "maximum": 1000} should be extracted for optimization
        if let Some(member_type) =
            self.try_extract_bounded_integer_type(schema, parent_type, property_name)
        {
            return Ok(member_type);
        }

        // Check for format-constrained strings BEFORE handling as primitives
        // Format-constrained strings like {"type": "string", "format": "date-time"} should be extracted for O3 optimization
        if let Some(member_type) =
            self.try_extract_format_string_type(schema, parent_type, property_name)
        {
            return Ok(member_type);
        }

        // Handle primitive types
        if let Some(type_str) = schema.get("type").and_then(Value::as_str) {
            match type_str {
                "string" => return Ok(MemberType::Primitive("String".to_string())),
                "integer" => {
                    return Ok(MemberType::Primitive(rust_type_for_integer_schema(schema)));
                }
                "number" => {
                    // Use serde_json::Number to preserve integer vs float representation
                    return Ok(MemberType::Primitive("serde_json::Number".to_string()));
                }
                "boolean" => return Ok(MemberType::Primitive("bool".to_string())),
                "null" => {
                    // Represent JSON null with special marker for unit variant generation
                    // The enum generator will create a unit variant for this
                    return Ok(MemberType::Primitive("Null".to_string()));
                }
                "array" => {
                    if let Some(items) = schema.get("items").and_then(Value::as_object) {
                        let inner_type = self.determine_member_type(items, parent_type, None)?;
                        match inner_type {
                            MemberType::Primitive(inner) => {
                                return Ok(MemberType::Container {
                                    container: "Vec".to_string(),
                                    inner,
                                });
                            }
                            MemberType::TypeReference(inner) => {
                                return Ok(MemberType::ComplexContainer {
                                    container: "Vec".to_string(),
                                    inner,
                                });
                            }
                            _ => {}
                        }
                    }
                }
                "object" => {
                    // Determine if this is a constrained or unconstrained object
                    let has_properties = schema.contains_key("properties");
                    let has_pattern_properties = schema.contains_key("patternProperties");

                    if has_properties || has_pattern_properties {
                        // Constrained object with explicit structure - needs struct type
                        let nested_type_name =
                            self.create_nested_type_name(parent_type, property_name);

                        crate::log_debug!(
                            "🏗️  Constrained object property '{}' in '{}' → Struct '{}'",
                            property_name.unwrap_or("unnamed"),
                            parent_type,
                            nested_type_name
                        );

                        self.discover_schema_as_type(
                            schema,
                            &nested_type_name,
                            Some(parent_type.to_string()),
                            None,
                        )?;
                        return Ok(MemberType::TypeReference(nested_type_name));
                    }

                    // Unconstrained object - check additionalProperties behavior
                    // Per JSON Schema spec, if additionalProperties is not present, it defaults to true
                    match schema.get("additionalProperties") {
                        Some(Value::Bool(false)) => {
                            // additionalProperties: false with no properties = empty object
                            // Needs an empty struct type
                            let nested_type_name =
                                self.create_nested_type_name(parent_type, property_name);

                            crate::log_debug!(
                                "📦 Empty object property '{}' in '{}' → Empty Struct '{}'",
                                property_name.unwrap_or("unnamed"),
                                parent_type,
                                nested_type_name
                            );

                            self.discover_schema_as_type(
                                schema,
                                &nested_type_name,
                                Some(parent_type.to_string()),
                                None,
                            )?;
                            return Ok(MemberType::TypeReference(nested_type_name));
                        }
                        Some(additional_props) if additional_props.is_object() => {
                            // additionalProperties with schema - HashMap with typed values
                            let value_type = if let Some(add_schema) = additional_props.as_object()
                            {
                                member_type_as_rust_value_type(self.determine_member_type(
                                    add_schema,
                                    parent_type,
                                    None,
                                )?)
                            } else {
                                "serde_json::Value".to_string()
                            };

                            crate::log_debug!(
                                "🗺️  Unconstrained object property '{}' in '{}' → HashMap<String, {}>",
                                property_name.unwrap_or("unnamed"),
                                parent_type,
                                value_type
                            );

                            return Ok(MemberType::HashMap {
                                key: "String".to_string(),
                                value: value_type,
                            });
                        }
                        Some(Value::Bool(true)) | None => {
                            // additionalProperties: true OR missing (implicit default is true)
                            // Unconstrained object - use HashMap<String, Value>
                            crate::log_debug!(
                                "🎯 Unconstrained object property '{}' in '{}' → HashMap<String, serde_json::Value>",
                                property_name.unwrap_or("unnamed"),
                                parent_type
                            );

                            return Ok(MemberType::HashMap {
                                key: "String".to_string(),
                                value: "serde_json::Value".to_string(),
                            });
                        }
                        _ => {
                            // Shouldn't happen, but be defensive
                            crate::log_debug!(
                                "⚠️  Unknown additionalProperties value for '{}', defaulting to HashMap",
                                property_name.unwrap_or("unnamed")
                            );

                            return Ok(MemberType::HashMap {
                                key: "String".to_string(),
                                value: "serde_json::Value".to_string(),
                            });
                        }
                    }
                }
                _ => {
                    // Invalid or unsupported type - reject with clear error instead of a Value fallback.
                    return Err(format!(
                        "Unsupported type '{}' in schema. Valid JSON Schema types are: string, integer, number, boolean, null, array, object",
                        type_str
                    ).into());
                }
            }
        }

        // Handle allOf composition
        if schema.contains_key("allOf") {
            if let Some(member_type) =
                self.try_transparent_allof_ref_member_type(schema, parent_type, property_name)?
            {
                return Ok(member_type);
            }

            let nested_type_name = self.create_nested_type_name(parent_type, property_name);

            crate::log_debug!(
                "🔄 AllOf property '{}' in '{}' → Struct '{}'",
                property_name.unwrap_or("unnamed"),
                parent_type,
                nested_type_name
            );

            self.discover_schema_as_type(
                schema,
                &nested_type_name,
                Some(parent_type.to_string()),
                property_name.map(String::from),
            )?;
            return Ok(MemberType::TypeReference(nested_type_name));
        }

        // Handle compositions
        if schema.contains_key("oneOf") || schema.contains_key("anyOf") {
            let enum_type_name = if let Some(prop_name) = property_name {
                // Include property name to make enum type unique per property
                format!(
                    "{}{}Enum",
                    parent_type,
                    crate::utils::to_upper_camel_case(prop_name)
                )
            } else {
                // Fallback to old naming for cases without property context
                format!("{}Enum", parent_type)
            };
            self.discover_schema_as_type(
                schema,
                &enum_type_name,
                Some(parent_type.to_string()),
                None,
            )?;
            return Ok(MemberType::TypeReference(enum_type_name));
        }

        // Default to serde_json::Value for unknown types
        Ok(MemberType::Primitive("serde_json::Value".to_string()))
    }

    fn try_transparent_allof_ref_member_type(
        &mut self,
        schema: &Map<String, Value>,
        parent_type: &str,
        property_name: Option<&str>,
    ) -> TctResult<Option<MemberType>> {
        let Some(allof) = schema.get("allOf").and_then(Value::as_array) else {
            return Ok(None);
        };
        if allof.len() != 1 {
            return Ok(None);
        }
        if schema.keys().any(|key| !is_transparent_allof_ref_key(key)) {
            return Ok(None);
        }

        let Some(ref_path) = allof[0]
            .as_object()
            .and_then(|subschema| subschema.get("$ref"))
            .and_then(Value::as_str)
        else {
            return Ok(None);
        };

        if ref_path.starts_with("http://") || ref_path.starts_with("https://") {
            return Ok(Some(MemberType::Primitive("serde_json::Value".to_string())));
        }

        let ref_schema =
            Map::from_iter([("$ref".to_string(), Value::String(ref_path.to_string()))]);
        Ok(Some(self.determine_member_type(
            &ref_schema,
            parent_type,
            property_name,
        )?))
    }

    /// Build JSON pointer path for schema location
    pub(super) fn build_json_path(
        &self,
        parent_type: Option<&str>,
        property_name: Option<&str>,
    ) -> String {
        match (parent_type, property_name) {
            (Some(parent), Some(prop)) => format!("#/properties/{}/properties/{}", parent, prop),
            (Some(parent), None) => format!("#/properties/{}", parent),
            (None, Some(prop)) => format!("#/properties/{}", prop),
            (None, None) => "#/".to_string(),
        }
    }
}

fn is_transparent_allof_ref_key(key: &str) -> bool {
    matches!(
        key,
        "allOf"
            | "$schema"
            | "$id"
            | "id"
            | "$comment"
            | "title"
            | "description"
            | "default"
            | "examples"
            | "deprecated"
            | "readOnly"
            | "writeOnly"
    )
}

/// Check if two schemas are structurally equal (produce identical Rust types)
///
/// KEY FIX: Also compares "required" arrays because different required fields
/// produce different type signatures (Option<T> vs T)
pub(super) fn schemas_structurally_equal(
    schema1: &Map<String, Value>,
    schema2: &Map<String, Value>,
) -> bool {
    let shape_keys = [
        "type",
        "enum",
        "const",
        "pattern",
        "format",
        "items",
        "minItems",
        "maxItems",
        "uniqueItems",
        "minimum",
        "maximum",
        "exclusiveMinimum",
        "exclusiveMaximum",
        "multipleOf",
        "minLength",
        "maxLength",
        "$ref",
        "oneOf",
        "anyOf",
        "allOf",
        "not",
        "if",
        "then",
        "else",
    ];

    for key in shape_keys {
        if schema1.get(key) != schema2.get(key) {
            return false;
        }
    }

    // Compare properties
    let props1 = schema1.get("properties").and_then(Value::as_object);
    let props2 = schema2.get("properties").and_then(Value::as_object);
    if props1 != props2 {
        return false;
    }

    // Compare required arrays - CRITICAL for detecting different type signatures
    // Different required = different Optional vs non-Optional fields
    let required1 = schema1.get("required").and_then(Value::as_array);
    let required2 = schema2.get("required").and_then(Value::as_array);
    if required1 != required2 {
        return false;
    }

    // Compare pattern properties
    let pattern1 = schema1.get("patternProperties").and_then(Value::as_object);
    let pattern2 = schema2.get("patternProperties").and_then(Value::as_object);
    if pattern1 != pattern2 {
        return false;
    }

    // Compare additionalProperties
    let additional1 = schema1.get("additionalProperties");
    let additional2 = schema2.get("additionalProperties");
    if additional1 != additional2 {
        return false;
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn member_type_as_rust_value_type_preserves_nested_maps() {
        assert_eq!(
            member_type_as_rust_value_type(MemberType::Primitive("String".to_string())),
            "String"
        );
        assert_eq!(
            member_type_as_rust_value_type(MemberType::HashMap {
                key: "String".to_string(),
                value: "i64".to_string(),
            }),
            "rustc_hash::FxHashMap<String, i64>"
        );
    }
}
