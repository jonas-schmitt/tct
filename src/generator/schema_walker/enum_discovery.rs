//! Enum Discovery Module
//!
//! Handles discovery of enum variants from JSON Schema including:
//! - Simple string enums (enum keyword and string const)
//! - Complex enums (oneOf/anyOf)
//! - Type arrays (type: ["string", "number"])

use crate::TctResult;
use crate::generator::schema_discovery::{MemberInfo, MemberType, TypeDiscovery};
use serde_json::{Map, Value};

/// SchemaWalker methods for enum variant discovery
impl super::SchemaWalker {
    /// Discover enum variants (oneOf/anyOf/enum/string const)
    pub(super) fn discover_enum_variants(
        &mut self,
        schema: &Map<String, Value>,
        type_discovery: &mut TypeDiscovery,
    ) -> TctResult<()> {
        crate::log_debug!(
            "🔍 DEBUG ENUM VARIANTS: Starting variant discovery for type '{}'",
            type_discovery.type_name
        );
        crate::log_debug!("   • Schema keys: {:?}", schema.keys().collect::<Vec<_>>());

        // Handle oneOf/anyOf (complex enums)
        if let Some(one_of) = schema.get("oneOf").and_then(Value::as_array) {
            crate::log_debug!("   • Found oneOf with {} variants", one_of.len());
            self.discover_complex_enum_variants(one_of, type_discovery)?;
            // Detect discriminator pattern (e.g., Kubernetes "kind" field)
            self.detect_discriminator_pattern(type_discovery);
        } else if let Some(any_of) = schema.get("anyOf").and_then(Value::as_array) {
            crate::log_debug!("   • Found anyOf with {} variants", any_of.len());
            self.discover_complex_enum_variants(any_of, type_discovery)?;
            // Detect discriminator pattern (e.g., Kubernetes "kind" field)
            self.detect_discriminator_pattern(type_discovery);
        }
        // Handle type arrays: {"type": ["object", "string"]}
        else if let Some(type_array) = schema.get("type").and_then(Value::as_array) {
            if type_array.len() > 1 {
                crate::log_debug!("   • Found type array with {} types", type_array.len());
                self.discover_type_array_variants(type_array, schema, type_discovery)?;
            } else {
                crate::log_debug!("   • Type array has only 1 type, not an enum");
                return Ok(());
            }
        }
        // Handle simple string enums: {"enum": ["value1", "value2", ...]}
        else if let Some(enum_values) = schema.get("enum").and_then(Value::as_array) {
            crate::log_debug!("   • Found simple enum with {} values", enum_values.len());
            self.discover_simple_enum_variants(enum_values, type_discovery)?;
        } else if let Some(const_value) = schema.get("const").and_then(Value::as_str) {
            crate::log_debug!("   • Found string const '{}'", const_value);
            let enum_values = [Value::String(const_value.to_string())];
            self.discover_simple_enum_variants(&enum_values, type_discovery)?;
        } else {
            crate::log_debug!(
                "   • No oneOf/anyOf/enum/string const/type-array found - returning early"
            );
            return Ok(());
        }

        crate::log_debug!(
            "🔍 DEBUG ENUM VARIANTS: Completed variant discovery for '{}' - {} variants found",
            type_discovery.type_name,
            type_discovery.members.len()
        );
        Ok(())
    }

    /// Discover complex enum variants (oneOf/anyOf with object schemas)
    fn discover_complex_enum_variants(
        &mut self,
        variants: &[Value],
        type_discovery: &mut TypeDiscovery,
    ) -> TctResult<()> {
        // Extract root-level properties to merge with variants
        // This is needed because JSON Schema semantics say that when a schema has both
        // properties and anyOf/oneOf, ALL variants must satisfy the root properties
        let root_properties = type_discovery.schema_object.get("properties").cloned();
        let root_required = type_discovery.schema_object.get("required").cloned();
        let root_pattern_properties = type_discovery
            .schema_object
            .get("patternProperties")
            .cloned();
        let root_additional_properties = type_discovery
            .schema_object
            .get("additionalProperties")
            .cloned();

        if root_properties.is_some() {
            crate::log_debug!("   • Root schema has properties - will merge with each variant");
        }

        for (index, variant_value) in variants.iter().enumerate() {
            crate::log_debug!(
                "   • Processing complex variant {}: {:?}",
                index,
                variant_value
            );
            if let Some(variant_schema) = variant_value.as_object() {
                crate::log_debug!(
                    "     ✅ Variant {} is object with keys: {:?}",
                    index,
                    variant_schema.keys().collect::<Vec<_>>()
                );

                // Merge root properties into variant schema
                let mut merged_schema = variant_schema.clone();
                if let Some(ref root_props) = root_properties {
                    merged_schema.insert("properties".to_string(), root_props.clone());
                    crate::log_debug!("       • Merged root properties into variant {}", index);
                }
                if let Some(ref root_req) = root_required {
                    // Merge required arrays (variant required + root required)
                    if let Some(variant_req) =
                        merged_schema.get("required").and_then(Value::as_array)
                    {
                        let mut combined_required = variant_req.clone();
                        if let Some(root_req_array) = root_req.as_array() {
                            for req in root_req_array {
                                if !combined_required.contains(req) {
                                    combined_required.push(req.clone());
                                }
                            }
                        }
                        merged_schema
                            .insert("required".to_string(), Value::Array(combined_required));
                    } else {
                        merged_schema.insert("required".to_string(), root_req.clone());
                    }
                    crate::log_debug!("       • Merged root required into variant {}", index);
                }
                if let Some(ref root_pattern) = root_pattern_properties
                    && !merged_schema.contains_key("patternProperties")
                {
                    merged_schema.insert("patternProperties".to_string(), root_pattern.clone());
                    crate::log_debug!(
                        "       • Merged root patternProperties into variant {}",
                        index
                    );
                }
                if let Some(ref root_additional) = root_additional_properties
                    && !merged_schema.contains_key("additionalProperties")
                {
                    merged_schema
                        .insert("additionalProperties".to_string(), root_additional.clone());
                    crate::log_debug!(
                        "       • Merged root additionalProperties into variant {}",
                        index
                    );
                }

                let variant_name = format!("Variant{}", index);
                // Use variant-specific parent name to generate unique nested types
                // This prevents different variants from overwriting each other
                let variant_parent_name = format!("{}{}", type_discovery.type_name, variant_name);
                let member_type =
                    self.determine_member_type(&merged_schema, &variant_parent_name, None)?;
                crate::log_debug!("     ✅ Variant {} member type: {:?}", index, member_type);

                let member_info = MemberInfo {
                    name: variant_name,
                    member_type,
                    required: true,     // Enum variants are always "required"
                    is_nullable: false, // Enum variants are never nullable
                    schema: merged_schema,
                };

                type_discovery.add_member(member_info);
                self.discovery.stats.total_enum_variants += 1;
                crate::log_debug!("     ✅ Variant {} added successfully", index);
            } else {
                crate::log_debug!(
                    "     ❌ Variant {} is not an object: {:?}",
                    index,
                    variant_value
                );
            }
        }
        Ok(())
    }

    /// Discover simple enum variants (string literals)
    pub(super) fn discover_simple_enum_variants(
        &mut self,
        enum_values: &[Value],
        type_discovery: &mut TypeDiscovery,
    ) -> TctResult<()> {
        // Distinct wire strings can sanitize to the same PascalCase ident
        // (e.g. YYYY-MM-DD vs YYYY/MM/DD). Disambiguate stably in enum order.
        let mut used_variant_names = std::collections::HashSet::new();

        for (index, enum_value) in enum_values.iter().enumerate() {
            if let Some(value_str) = enum_value.as_str() {
                crate::log_debug!(
                    "   • Processing simple enum value {}: '{}'",
                    index,
                    value_str
                );

                // Convert enum value to a unique Rust variant name (PascalCase)
                let variant_name = crate::utils::allocate_unique_upper_camel_ident(
                    value_str,
                    &mut used_variant_names,
                );
                let variant_name_copy = variant_name.clone(); // Clone before moving

                // Simple enums are just unit variants (no associated data)
                let member_type = MemberType::Primitive("()".to_string()); // Unit type

                // Store the original enum value for serde rename
                let mut variant_schema = serde_json::Map::new();
                variant_schema.insert(
                    "__original_enum_value".to_string(),
                    serde_json::Value::String(value_str.to_string()),
                );

                let member_info = MemberInfo {
                    name: variant_name,
                    member_type,
                    required: true,     // Enum variants are always "required"
                    is_nullable: false, // Enum variants are never nullable
                    schema: variant_schema,
                };
                type_discovery.add_member(member_info);
                self.discovery.stats.total_enum_variants += 1;
                crate::log_debug!(
                    "     ✅ Simple enum variant '{}' -> '{}' added successfully",
                    value_str,
                    variant_name_copy
                );
            } else {
                crate::log_debug!(
                    "     ❌ Enum value {} is not a string: {:?}",
                    index,
                    enum_value
                );
            }
        }
        Ok(())
    }

    /// Discover type array variants: {"type": ["object", "string"]}
    /// Per JSON Schema spec, type can be an array of type strings
    fn discover_type_array_variants(
        &mut self,
        type_array: &[Value],
        schema: &Map<String, Value>,
        type_discovery: &mut TypeDiscovery,
    ) -> TctResult<()> {
        crate::log_debug!(
            "   🔍 Processing type array with {} types for '{}'",
            type_array.len(),
            type_discovery.type_name
        );

        for (index, type_value) in type_array.iter().enumerate() {
            if let Some(type_str) = type_value.as_str() {
                crate::log_debug!("     • Type array element {}: '{}'", index, type_str);

                // Create a schema for this specific type
                // Inherit all properties from parent schema except "type"
                let mut variant_schema = schema.clone();
                variant_schema.insert("type".to_string(), Value::String(type_str.to_string()));

                let variant_name = format!("Variant{}", index);

                // Use variant-specific parent name to generate unique nested types
                // This prevents different variants from overwriting each other
                let variant_parent_name = format!("{}{}", type_discovery.type_name, variant_name);

                // Use determine_member_type to properly discover nested types
                // This handles objects, arrays, and all other types consistently
                let member_type =
                    self.determine_member_type(&variant_schema, &variant_parent_name, None)?;
                crate::log_debug!("     ✅ Variant {} member type: {:?}", index, member_type);

                let member_info = MemberInfo {
                    name: variant_name.clone(),
                    member_type,
                    required: true,
                    is_nullable: false, // Enum variants are never nullable
                    schema: variant_schema,
                };

                type_discovery.add_member(member_info);
                self.discovery.stats.total_enum_variants += 1;
                crate::log_debug!(
                    "     ✅ Type array variant '{}' added successfully",
                    variant_name
                );
            } else {
                crate::log_debug!(
                    "       ❌ Type array element {} is not a string: {:?}",
                    index,
                    type_value
                );
            }
        }
        Ok(())
    }

    /// Detect discriminator pattern in complex enum variants
    ///
    /// Looks for common fields where each variant has a unique single-value enum.
    /// Supports both single-field discriminators (like "kind") and compound
    /// discriminators (like "kind" + "apiVersion" for Kubernetes).
    ///
    /// For Kubernetes, each variant has:
    /// ```json
    /// "kind": { "type": "string", "enum": ["Pod"] },
    /// "apiVersion": { "type": "string", "enum": ["v1"] }
    /// ```
    fn detect_discriminator_pattern(&self, type_discovery: &mut TypeDiscovery) {
        use crate::generator::schema_discovery::DiscriminatorInfo;
        use std::collections::BTreeMap;

        // Need at least 2 variants to have a discriminator pattern
        if type_discovery.members.len() < 2 {
            return;
        }

        // Candidate discriminator fields: field_name -> (discriminator_value, variant_index)
        //
        // Ordered by field name: which field or pair is chosen, and the order of the
        // fields in a compound key, end up in the generated code, so they must not
        // depend on a hash map's iteration order.
        let mut candidates: BTreeMap<String, Vec<(String, usize)>> = BTreeMap::new();

        // Analyze each variant's schema for potential discriminator fields
        for (variant_idx, member) in type_discovery.members.iter().enumerate() {
            let resolved_schema =
                if let Some(ref_path) = member.schema.get("$ref").and_then(|v| v.as_str()) {
                    self.resolve_ref(ref_path)
                } else {
                    Some(member.schema.clone())
                };

            let properties = resolved_schema
                .as_ref()
                .and_then(|s| s.get("properties"))
                .and_then(|p| p.as_object());

            if let Some(props) = properties {
                for (field_name, field_schema) in props {
                    // Check if this field has a single-value enum (discriminator pattern)
                    // Handle both "enum": ["value"] and "const": "value" patterns
                    let single_value = if let Some(enum_values) =
                        field_schema.get("enum").and_then(|v| v.as_array())
                    {
                        if enum_values.len() == 1 {
                            enum_values[0].as_str().map(|s| s.to_string())
                        } else {
                            None
                        }
                    } else if let Some(const_value) = field_schema.get("const") {
                        // "const" is equivalent to a single-value enum
                        const_value.as_str().map(|s| s.to_string())
                    } else {
                        None
                    };

                    if let Some(value_str) = single_value {
                        candidates
                            .entry(field_name.clone())
                            .or_default()
                            .push((value_str, variant_idx));
                    }
                }
            }
        }

        let variant_count = type_discovery.members.len();

        // Debug: Show candidate field coverage
        for (field_name, mappings) in &candidates {
            let values: Vec<&String> = mappings.iter().map(|(v, _)| v).collect();
            let unique_values: std::collections::HashSet<&String> =
                values.iter().copied().collect();
            crate::log_debug!(
                "   📊 Candidate '{}': {}/{} variants, {}/{} unique values",
                field_name,
                mappings.len(),
                variant_count,
                unique_values.len(),
                mappings.len()
            );
        }

        // First, try single-field discriminators
        for (field_name, mappings) in &candidates {
            if mappings.len() == variant_count {
                let values: Vec<&String> = mappings.iter().map(|(v, _)| v).collect();
                let unique_values: std::collections::HashSet<&String> =
                    values.iter().copied().collect();

                if unique_values.len() == variant_count {
                    crate::log_debug!(
                        "🎯 Detected single-field discriminator '{}' for enum '{}' with {} unique values",
                        field_name,
                        type_discovery.type_name,
                        variant_count
                    );

                    type_discovery.discriminator_info = Some(DiscriminatorInfo {
                        field_names: vec![field_name.clone()],
                        value_to_variant: mappings.clone(),
                    });
                    return;
                }
            }
        }

        // Try compound discriminators (e.g., "kind" + "apiVersion" for Kubernetes)
        // Look for common field pairs that together provide unique identification
        let field_names: Vec<_> = candidates.keys().cloned().collect();
        for i in 0..field_names.len() {
            for j in (i + 1)..field_names.len() {
                let field1 = &field_names[i];
                let field2 = &field_names[j];

                // Both fields must be present in all variants
                let mappings1 = candidates.get(field1);
                let mappings2 = candidates.get(field2);

                if let (Some(m1), Some(m2)) = (mappings1, mappings2)
                    && m1.len() == variant_count
                    && m2.len() == variant_count
                {
                    // Create compound values by joining field values with "|"
                    let compound_mappings: Vec<(String, usize)> = m1
                        .iter()
                        .filter_map(|(v1, idx1)| {
                            m2.iter()
                                .find(|(_, idx2)| idx1 == idx2)
                                .map(|(v2, _)| (format!("{}|{}", v1, v2), *idx1))
                        })
                        .collect();

                    if compound_mappings.len() == variant_count {
                        let compound_values: std::collections::HashSet<_> =
                            compound_mappings.iter().map(|(v, _)| v).collect();

                        if compound_values.len() == variant_count {
                            crate::log_debug!(
                                "🎯 Detected compound discriminator '{}' + '{}' for enum '{}' with {} unique values",
                                field1,
                                field2,
                                type_discovery.type_name,
                                variant_count
                            );

                            type_discovery.discriminator_info = Some(DiscriminatorInfo {
                                field_names: vec![field1.clone(), field2.clone()],
                                value_to_variant: compound_mappings,
                            });
                            return;
                        }
                    }
                }
            }
        }

        crate::log_debug!(
            "   • No discriminator pattern found for enum '{}'",
            type_discovery.type_name
        );
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    #[test]
    fn test_discriminator_detection_with_kind_field() {
        // Create a mock schema similar to Kubernetes with kind discriminator
        let root_schema = json!({
            "oneOf": [
                {"$ref": "#/definitions/Pod"},
                {"$ref": "#/definitions/Deployment"},
                {"$ref": "#/definitions/Service"}
            ],
            "definitions": {
                "Pod": {
                    "type": "object",
                    "properties": {
                        "kind": { "type": "string", "enum": ["Pod"] },
                        "name": { "type": "string" }
                    }
                },
                "Deployment": {
                    "type": "object",
                    "properties": {
                        "kind": { "type": "string", "enum": ["Deployment"] },
                        "replicas": { "type": "integer" }
                    }
                },
                "Service": {
                    "type": "object",
                    "properties": {
                        "kind": { "type": "string", "enum": ["Service"] },
                        "port": { "type": "integer" }
                    }
                }
            }
        });

        use crate::generator::schema_walker::SchemaWalker;
        let result = SchemaWalker::walk(&root_schema, "TestEnum");
        assert!(result.is_ok(), "Schema walk should succeed");

        let discovery = result.unwrap();

        // Find the root type
        let root_type = discovery.discovered_types.get("TestEnum");
        assert!(root_type.is_some(), "Root type should be discovered");

        let root = root_type.unwrap();
        assert!(
            root.discriminator_info.is_some(),
            "Should detect discriminator"
        );

        let disc_info = root.discriminator_info.as_ref().unwrap();
        assert_eq!(
            disc_info.field_names,
            vec!["kind"],
            "Discriminator field should be 'kind'"
        );
        assert_eq!(
            disc_info.value_to_variant.len(),
            3,
            "Should have 3 discriminator mappings"
        );

        // Verify mappings
        let mut mapping: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
        for (value, idx) in &disc_info.value_to_variant {
            mapping.insert(value.as_str(), *idx);
        }

        assert!(mapping.contains_key("Pod"), "Should have Pod mapping");
        assert!(
            mapping.contains_key("Deployment"),
            "Should have Deployment mapping"
        );
        assert!(
            mapping.contains_key("Service"),
            "Should have Service mapping"
        );
    }

    #[test]
    fn test_no_discriminator_without_unique_values() {
        // Create schema where variants don't have unique discriminator values
        let root_schema = json!({
            "oneOf": [
                {
                    "type": "object",
                    "properties": {
                        "type": { "type": "string" }  // No enum, just string
                    }
                },
                {
                    "type": "object",
                    "properties": {
                        "type": { "type": "string" }
                    }
                }
            ]
        });

        use crate::generator::schema_walker::SchemaWalker;
        let result = SchemaWalker::walk(&root_schema, "TestEnum");
        assert!(result.is_ok(), "Schema walk should succeed");

        let discovery = result.unwrap();
        let root_type = discovery.discovered_types.get("TestEnum");
        assert!(root_type.is_some(), "Root type should be discovered");

        let root = root_type.unwrap();
        assert!(
            root.discriminator_info.is_none(),
            "Should NOT detect discriminator without enum values"
        );
    }
}
