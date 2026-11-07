//! Struct Discovery Module
//!
//! Handles discovery of struct members from JSON Schema including:
//! - Regular properties
//! - allOf composition
//! - Pattern properties
//! - Additional properties

mod allof;

use crate::generator::schema_discovery::{
    MemberInfo, MemberType, PatternValidatorInfo, SchemaLocation, TypeDiscovery, TypeKind,
};
use crate::{TctError, TctResult};
use serde_json::{Map, Value};
use std::collections::HashSet;

use super::type_determination::{member_type_as_rust_value_type, schemas_structurally_equal};

fn json_pointer_segment(segment: &str) -> String {
    crate::schema_utils::escape_json_pointer_segment(segment)
}

fn unsupported_allof_shape(
    path: impl Into<String>,
    keyword: &str,
    suggestion: impl Into<String>,
) -> TctError {
    TctError::SchemaValidation {
        message: format!(
            "Unsupported allOf subschema shape: '{}' is not merged by the current allOf implementation",
            keyword
        ),
        path: path.into(),
        suggestion: suggestion.into(),
        context: None,
    }
}

fn schema_declares_exact_object(schema: &Map<String, Value>) -> bool {
    schema.get("type").and_then(Value::as_str) == Some("object")
}

fn is_allof_annotation_key(key: &str) -> bool {
    matches!(
        key,
        "$schema"
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

fn is_allof_ref_annotation_key(key: &str) -> bool {
    key == "$ref" || is_allof_annotation_key(key)
}

/// SchemaWalker methods for struct member discovery
impl super::SchemaWalker {
    /// Discover struct members from a schema
    pub(super) fn discover_struct_members(
        &mut self,
        schema: &Map<String, Value>,
        type_discovery: &mut TypeDiscovery,
    ) -> TctResult<()> {
        self.discover_struct_members_internal(schema, type_discovery, false, "#")
    }

    /// Internal implementation of discover_struct_members with control over additionalProperties processing
    pub(super) fn discover_struct_members_internal(
        &mut self,
        schema: &Map<String, Value>,
        type_discovery: &mut TypeDiscovery,
        skip_additional_properties: bool,
        schema_path: &str,
    ) -> TctResult<()> {
        let mut allof_required_entries = Vec::new();

        // Process allOf composition - merge properties from all subschemas
        if let Some(allof_array) = schema.get("allOf").and_then(Value::as_array) {
            crate::log_debug!(
                "🔄 Processing allOf with {} subschemas for type '{}'",
                allof_array.len(),
                type_discovery.type_name
            );

            let allow_implicit_object_subschemas = schema_declares_exact_object(schema)
                || self.allof_array_declares_object(allof_array);

            for (index, subschema) in allof_array.iter().enumerate() {
                if let Some(subschema_obj) = subschema.as_object() {
                    let subschema_path = format!("{}/allOf/{}", schema_path, index);
                    self.discover_allof_subschema_members(
                        subschema_obj,
                        type_discovery,
                        index,
                        &subschema_path,
                        allow_implicit_object_subschemas,
                    )?;
                } else {
                    return Err(TctError::SchemaValidation {
                        message: "Unsupported allOf subschema shape: expected a schema object"
                            .to_string(),
                        path: format!("{}/allOf/{}", schema_path, index),
                        suggestion:
                            "Use object subschemas in allOf or split unsupported composition shapes before compiling."
                                .to_string(),
                        context: None,
                    });
                }
            }

            allof_required_entries =
                self.collect_allof_required_entries(allof_array, schema_path)?;
        }

        // Process regular properties
        if let Some(properties) = schema.get("properties").and_then(Value::as_object) {
            let required_props = schema
                .get("required")
                .and_then(Value::as_array)
                .map(|arr| arr.iter().filter_map(Value::as_str).collect::<HashSet<_>>())
                .unwrap_or_default();

            for (prop_name, prop_schema) in properties {
                let prop_required = required_props.contains(prop_name.as_str());
                let prop_schema_obj = prop_schema.as_object().unwrap_or(&Map::new()).clone();
                let prop_schema_path = format!(
                    "{}/properties/{}",
                    schema_path,
                    json_pointer_segment(prop_name)
                );

                if let Some(existing_member) = type_discovery
                    .members
                    .iter_mut()
                    .find(|member| member.name == *prop_name)
                {
                    if !schemas_structurally_equal(&existing_member.schema, &prop_schema_obj) {
                        return Err(TctError::SchemaValidation {
                            message: format!(
                                "Conflicting allOf property definitions for '{}'",
                                prop_name
                            ),
                            path: prop_schema_path,
                            suggestion: "TCT currently supports duplicate allOf properties only when their validation-relevant schemas are structurally identical. Merge the constraints explicitly or avoid overlapping property definitions.".to_string(),
                            context: None,
                        });
                    }

                    let was_required = existing_member.required;
                    existing_member.required |= prop_required;
                    if existing_member.required && !was_required {
                        existing_member.is_nullable = false;
                    }
                    crate::log_debug!(
                        "🔁 Merged duplicate allOf property '{}' for type '{}' (required={})",
                        prop_name,
                        type_discovery.type_name,
                        existing_member.required
                    );
                    continue;
                }

                let member_info = self.discover_property_member(
                    prop_name,
                    prop_schema,
                    prop_required,
                    &type_discovery.type_name,
                )?;

                type_discovery.add_member(member_info);
                self.discovery.stats.total_properties += 1;
            }
        }

        Self::apply_allof_required_entries(type_discovery, &allof_required_entries)?;

        // Process pattern properties
        if let Some(pattern_props) = schema.get("patternProperties").and_then(Value::as_object) {
            crate::log_debug!(
                "🎯 Found patternProperties for type '{}' with {} patterns",
                type_discovery.type_name,
                pattern_props.len()
            );

            let pattern_validators =
                self.discover_pattern_properties(pattern_props, &type_discovery.type_name)?;

            // Add individual typed fields for each pattern validator
            for (index, validator_name) in pattern_validators.iter().enumerate() {
                let field_name = format!("pattern_{}", index);
                let pattern_member = MemberInfo {
                    name: field_name.clone(),
                    member_type: MemberType::TypeReference(validator_name.clone()),
                    required: true,     // Always required (initializes as empty)
                    is_nullable: false, // Pattern validators are never nullable
                    schema: Map::new(),
                };

                type_discovery.add_member(pattern_member);
                crate::log_debug!(
                    "✅ Added pattern field '{}' -> '{}' to type '{}'",
                    field_name,
                    validator_name,
                    type_discovery.type_name
                );
            }
        }

        // Process additional properties
        // Per JSON Schema spec: if additionalProperties is not specified, default is true (allow any)
        // However, when processing allOf subschemas, skip additionalProperties to avoid duplicates
        let additional_props_value = schema.get("additionalProperties");
        let should_add_additional_properties = if skip_additional_properties {
            // Skip additionalProperties when processing allOf subschemas
            false
        } else if let Some(additional_props) = additional_props_value {
            crate::log_debug!(
                "🗂️ Found additionalProperties for type '{}': {:?}",
                type_discovery.type_name,
                additional_props
            );
            // If explicitly set to false, don't add HashMap
            additional_props != &Value::Bool(false)
        } else {
            // Not specified → default to true per JSON Schema spec
            crate::log_debug!(
                "🗂️ additionalProperties not specified for type '{}', defaulting to true per JSON Schema spec",
                type_discovery.type_name
            );
            true
        };

        if should_add_additional_properties {
            let additional_props = additional_props_value.unwrap_or(&Value::Bool(true));

            // Use determine_member_type to get the appropriate HashMap type
            let additional_member_type = if additional_props.is_object() {
                // additionalProperties with schema - determine value type and wrap in HashMap
                let value_type = if let Some(add_schema) = additional_props.as_object() {
                    member_type_as_rust_value_type(self.determine_member_type(
                        add_schema,
                        &type_discovery.type_name,
                        Some("additional_properties"),
                    )?)
                } else {
                    "serde_json::Value".to_string()
                };
                Some(MemberType::HashMap {
                    key: "String".to_string(),
                    value: value_type,
                })
            } else if additional_props == &Value::Bool(true) {
                // additionalProperties: true - use generic HashMap
                Some(MemberType::HashMap {
                    key: "String".to_string(),
                    value: "serde_json::Value".to_string(),
                })
            } else {
                // additionalProperties: false or invalid - skip adding member
                crate::log_debug!(
                    "⏭️ Skipping additionalProperties for type '{}' (false or invalid)",
                    type_discovery.type_name
                );
                None
            };

            // Only add if we have a HashMap (additionalProperties: true or schema)
            if let Some(MemberType::HashMap { key, value }) = additional_member_type {
                // Clone for logging before moving
                let key_for_log = key.clone();
                let value_for_log = value.clone();

                let additional_member = MemberInfo {
                    name: "additional_properties".to_string(),
                    member_type: MemberType::HashMap { key, value },
                    required: true, // Always required (initializes as empty HashMap)
                    is_nullable: false, // Additional properties is never nullable
                    schema: Map::new(),
                };

                type_discovery.add_member(additional_member);
                crate::log_debug!(
                    "✅ Added additional_properties HashMap<{}, {}> to type '{}'",
                    key_for_log,
                    value_for_log,
                    type_discovery.type_name
                );
            }
        }

        Ok(())
    }

    /// Discover pattern properties and create validator types
    pub(super) fn discover_pattern_properties(
        &mut self,
        pattern_props: &Map<String, Value>,
        parent_type_name: &str,
    ) -> TctResult<Vec<String>> {
        let mut validator_names = Vec::new();

        for (index, (pattern, value_schema)) in pattern_props.iter().enumerate() {
            let validator_index = index; // 0-based indexing (programming standard)
            let validator_name = format!("{}PatternValidator{}", parent_type_name, validator_index);

            crate::log_debug!(
                "   📋 Creating pattern validator '{}' for pattern '{}'",
                validator_name,
                pattern
            );

            // Determine value type from schema with full type support
            let value_type = if let Some(value_obj) = value_schema.as_object() {
                match self.determine_member_type(value_obj, parent_type_name, None)? {
                    MemberType::Primitive(t) => t,
                    MemberType::TypeReference(t) => t,
                    MemberType::Container { container, inner } => {
                        format!("{}<{}>", container, inner)
                    }
                    MemberType::ComplexContainer { container, inner } => {
                        format!("{}<{}>", container, inner)
                    }
                    MemberType::HashMap { key, value } => {
                        format!("rustc_hash::FxHashMap<{}, {}>", key, value)
                    }
                    MemberType::PatternProperties { .. } => {
                        "serde_json::Value".to_string() // Nested pattern properties fallback
                    }
                }
            } else {
                "serde_json::Value".to_string()
            };

            // Validate regex pattern at compile time
            if let Err(regex_error) = regex::Regex::new(pattern) {
                return Err(crate::error::TctError::SchemaValidation {
                    message: format!("Invalid regex pattern '{}': {}", pattern, regex_error),
                    path: format!("#/patternProperties/{}", pattern),
                    suggestion: "Fix the regex syntax. Common issues: unmatched brackets [], parentheses (), or invalid escape sequences. Use a valid regex pattern or remove patternProperties.".to_string(),
                    context: None,
                });
            }

            // Create pattern validator info
            let validator_info = PatternValidatorInfo {
                validator_name: validator_name.clone(),
                pattern: pattern.clone(),
                value_type,
                value_schema: value_schema.as_object().cloned().unwrap_or_default(),
            };

            // Store the validator info in the discovery
            self.discovery
                .pattern_validators
                .insert(validator_name.clone(), validator_info);

            // Register the pattern validator as a standalone type
            let validator_schema_location = SchemaLocation {
                json_path: format!("#/patternProperties/{}", pattern),
                description: format!("Pattern validator for '{}'", pattern),
                parent_type: Some(parent_type_name.to_string()),
                property_name: Some(format!("pattern_{}", validator_index)),
            };
            let validator_type_discovery = TypeDiscovery::new(
                validator_name.clone(),
                TypeKind::Struct,
                validator_schema_location,
                Map::new(), // Pattern validators are synthetic types with no schema object
            );
            self.discovery.register_type(validator_type_discovery);

            crate::log_debug!(
                "   ✅ Pattern validator created: '{}' -> '{}'",
                pattern,
                validator_name
            );

            validator_names.push(validator_name);
        }

        crate::log_debug!(
            "🎯 Created {} pattern validators for type '{}'",
            validator_names.len(),
            parent_type_name
        );

        Ok(validator_names)
    }

    /// Discover a property member
    pub(super) fn discover_property_member(
        &mut self,
        prop_name: &str,
        prop_schema: &Value,
        required: bool,
        parent_type_name: &str,
    ) -> TctResult<MemberInfo> {
        let prop_schema_obj = prop_schema.as_object().unwrap_or(&Map::new()).clone();
        let member_type =
            self.determine_member_type(&prop_schema_obj, parent_type_name, Some(prop_name))?;

        // Check if this field explicitly allows null
        // For optional fields, this determines whether to use OptionalField<T> or Option<T>
        let is_nullable = !required && self.schema_allows_null(&prop_schema_obj);

        Ok(MemberInfo {
            name: prop_name.to_string(),
            member_type,
            required,
            is_nullable,
            schema: prop_schema_obj,
        })
    }
}
