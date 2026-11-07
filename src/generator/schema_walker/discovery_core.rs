//! Discovery Core Module
//!
//! High-level orchestration for schema discovery process.
//! Contains the main discovery workflow functions that coordinate type analysis.

use super::type_determination::schemas_structurally_equal;
use crate::TctResult;
use crate::generator::schema_discovery::{
    DefinitionInfo, RootSchemaInfo, SchemaLocation, TypeDiscovery, TypeKind,
};
use crate::schema_utils::{
    escape_json_pointer_segment, local_definition_ref_target, local_ref_segments,
};
use crate::utils::{to_type_name, to_upper_camel_case};
use serde_json::{Map, Value};

/// SchemaWalker methods for high-level discovery orchestration
impl super::SchemaWalker {
    pub(super) fn discover_root(&mut self, schema: &Value, root_name: &str) -> TctResult<()> {
        let sanitized_name = to_upper_camel_case(root_name);
        crate::log_debug!(
            "🌱 Discovering root: '{}' → '{}'",
            root_name,
            sanitized_name
        );

        if let Some(schema_obj) = schema.as_object() {
            let is_composition = self.is_composition_schema(schema_obj);
            crate::log_debug!("🔍 Root schema analysis:");
            crate::log_debug!("   • Is composition (oneOf/anyOf/enum): {}", is_composition);
            crate::log_debug!(
                "   • Schema keys: {:?}",
                schema_obj.keys().collect::<Vec<_>>()
            );

            let root_info = RootSchemaInfo {
                type_name: sanitized_name.clone(),
                schema: schema_obj.clone(),
                is_composition,
            };

            self.discovery.root_info = Some(root_info);

            // Discover the root type structure
            crate::log_debug!(
                "🔍 Discovering root type structure for '{}'",
                sanitized_name
            );
            self.discover_schema_as_type(schema_obj, &sanitized_name, None, None)?;
            crate::log_debug!("✅ Root discovery completed for '{}'", sanitized_name);
        } else {
            crate::log_debug!("⚠️  Root schema is not an object: {:?}", schema);
        }

        Ok(())
    }

    /// Discover definitions section
    pub(super) fn discover_definitions(
        &mut self,
        definitions: &Map<String, Value>,
    ) -> TctResult<()> {
        crate::log_debug!("📚 Processing {} definitions", definitions.len());

        for (def_name, def_schema) in definitions {
            let sanitized_name = to_type_name(def_name);
            crate::log_debug!("📖 Definition: '{}' → '{}'", def_name, sanitized_name);

            if let Some(schema_obj) = def_schema.as_object() {
                crate::log_debug!(
                    "   • Schema keys: {:?}",
                    schema_obj.keys().collect::<Vec<_>>()
                );

                let definition_info = DefinitionInfo {
                    name: def_name.clone(),
                    schema: schema_obj.clone(),
                    type_name: sanitized_name.clone(),
                };

                self.discovery
                    .definitions
                    .insert(def_name.clone(), definition_info);

                // Register reference mapping
                let json_path = format!("#/definitions/{}", escape_json_pointer_segment(def_name));
                self.discovery
                    .ref_mappings
                    .insert(json_path.clone(), sanitized_name.clone());
                crate::log_debug!("🔗 Reference mapping: {} → {}", json_path, sanitized_name);

                // Discover the definition type structure
                crate::log_debug!(
                    "🔍 Discovering definition type structure for '{}'",
                    sanitized_name
                );
                self.discover_schema_as_type(schema_obj, &sanitized_name, None, None)?;

                // Check for nested definitions and discover them recursively
                if let Some(nested_definitions) =
                    schema_obj.get("definitions").and_then(Value::as_object)
                {
                    crate::log_debug!(
                        "🔄 Found {} nested definitions in '{}'",
                        nested_definitions.len(),
                        def_name
                    );
                    self.discover_nested_definitions(nested_definitions, def_name)?;
                }

                self.discovery.stats.from_definitions += 1;
                crate::log_debug!("✅ Definition '{}' processed", def_name);
            } else {
                crate::log_debug!("⚠️  Definition '{}' schema is not an object", def_name);
            }
        }

        crate::log_debug!(
            "✅ All definitions processed ({} total)",
            self.discovery.stats.from_definitions
        );
        Ok(())
    }

    /// Discover root properties as standalone types for #/properties/ references
    /// Only discovers properties that are actually referenced in the schema
    pub(super) fn discover_properties(&mut self, properties: &Map<String, Value>) -> TctResult<()> {
        crate::log_debug!("🏠 Processing {} root properties", properties.len());

        // First, collect all #/properties/ references in the entire schema
        let referenced_properties = self.collect_property_references();
        crate::log_debug!(
            "🔍 Found {} referenced properties: {:?}",
            referenced_properties.len(),
            referenced_properties
        );

        for (prop_name, prop_schema) in properties {
            // Only discover properties that are actually referenced via #/properties/
            if referenced_properties.contains(prop_name) {
                if let Some(schema_obj) = prop_schema.as_object() {
                    let sanitized_name = to_upper_camel_case(&format!(
                        "Properties{}",
                        to_upper_camel_case(prop_name)
                    ));
                    crate::log_debug!(
                        "🏠 Referenced Property: '{}' → '{}'",
                        prop_name,
                        sanitized_name
                    );

                    // Register reference mapping for #/properties/{prop_name}
                    let json_path =
                        format!("#/properties/{}", escape_json_pointer_segment(prop_name));
                    self.discovery
                        .ref_mappings
                        .insert(json_path.clone(), sanitized_name.clone());
                    crate::log_debug!("🔗 Mapped '{}' → '{}'", json_path, sanitized_name);

                    // Discover the property schema as a standalone type
                    crate::log_debug!(
                        "🔍 Discovering property type structure for '{}'",
                        sanitized_name
                    );
                    self.discover_schema_as_type(
                        schema_obj,
                        &sanitized_name,
                        None,
                        Some(prop_name.clone()),
                    )?;

                    self.discovery.stats.from_definitions += 1; // Count as definition-like
                    crate::log_debug!("✅ Referenced property '{}' processed", prop_name);
                } else {
                    crate::log_debug!(
                        "⚠️ Referenced property '{}' schema is not an object",
                        prop_name
                    );
                }
            } else {
                crate::log_debug!("⏭️ Skipping unreferenced property '{}'", prop_name);
            }
        }

        crate::log_debug!("✅ All referenced properties processed");
        Ok(())
    }

    /// Collect all #/properties/ references used throughout the schema
    fn collect_property_references(&self) -> std::collections::HashSet<String> {
        let mut referenced_properties = std::collections::HashSet::new();

        // Convert schema back to JSON for scanning
        if let Some(root_info) = &self.discovery.root_info {
            let schema_value = serde_json::Value::Object(root_info.schema.clone());
            self.scan_for_property_refs(&schema_value, &mut referenced_properties);
        }

        // Also scan discovered definitions
        for def_info in self.discovery.definitions.values() {
            let def_value = serde_json::Value::Object(def_info.schema.clone());
            self.scan_for_property_refs(&def_value, &mut referenced_properties);
        }

        referenced_properties
    }

    /// Recursively scan a JSON value for #/properties/ references
    #[allow(clippy::only_used_in_recursion)]
    fn scan_for_property_refs(
        &self,
        value: &serde_json::Value,
        referenced: &mut std::collections::HashSet<String>,
    ) {
        match value {
            serde_json::Value::Object(obj) => {
                for (key, val) in obj {
                    if key == "$ref"
                        && let Some(ref_str) = val.as_str()
                        && let Some(segments) = local_ref_segments(ref_str)
                        && segments.first().map(String::as_str) == Some("properties")
                        && let Some(prop_name) = segments.get(1)
                    {
                        referenced.insert(prop_name.clone());
                    }
                    self.scan_for_property_refs(val, referenced);
                }
            }
            serde_json::Value::Array(arr) => {
                for item in arr {
                    self.scan_for_property_refs(item, referenced);
                }
            }
            _ => {}
        }
    }

    /// Collect all nested #/definitions/foo/bar references in the schema
    /// These are refs that point to sub-paths within definitions (not nested definitions sections)
    fn collect_nested_definition_refs(&self) -> std::collections::HashSet<String> {
        let mut nested_refs = std::collections::HashSet::new();

        // Scan root schema
        if let Some(root_info) = &self.discovery.root_info {
            let schema_value = serde_json::Value::Object(root_info.schema.clone());
            self.scan_for_nested_definition_refs(&schema_value, &mut nested_refs);
        }

        // Scan discovered definitions
        for def_info in self.discovery.definitions.values() {
            let def_value = serde_json::Value::Object(def_info.schema.clone());
            self.scan_for_nested_definition_refs(&def_value, &mut nested_refs);
        }

        nested_refs
    }

    /// Recursively scan a JSON value for nested #/definitions/foo/bar references
    #[allow(clippy::only_used_in_recursion)]
    fn scan_for_nested_definition_refs(
        &self,
        value: &serde_json::Value,
        nested_refs: &mut std::collections::HashSet<String>,
    ) {
        match value {
            serde_json::Value::Object(obj) => {
                for (key, val) in obj {
                    if key == "$ref"
                        && let Some(ref_str) = val.as_str()
                        && let Some(segments) = local_ref_segments(ref_str)
                        // Include paths that traverse into definitions, properties, or items.
                        // Examples:
                        //   - notificationObject/pushover -> nested property reference
                        //   - tsRules/properties/foo/definitions/bar/items -> deeply nested
                        && segments.first().map(String::as_str) == Some("definitions")
                        && segments.len() > 2
                    {
                        nested_refs.insert(ref_str.to_string());
                    }
                    self.scan_for_nested_definition_refs(val, nested_refs);
                }
            }
            serde_json::Value::Array(arr) => {
                for item in arr {
                    self.scan_for_nested_definition_refs(item, nested_refs);
                }
            }
            _ => {}
        }
    }

    /// Discover nested definition refs (like #/definitions/notificationObject/pushover)
    /// These point to sub-schemas within definitions that need to be discovered as types
    pub(super) fn discover_nested_definition_refs(&mut self) -> TctResult<()> {
        let nested_refs = self.collect_nested_definition_refs();

        if nested_refs.is_empty() {
            return Ok(());
        }

        crate::log_debug!(
            "🔗 Found {} nested definition refs: {:?}",
            nested_refs.len(),
            nested_refs
        );

        for ref_path in nested_refs {
            // Skip if already registered
            if self.discovery.ref_mappings.contains_key(&ref_path) {
                crate::log_debug!("⏭️ Skipping already registered ref: {}", ref_path);
                continue;
            }

            // Generate type name from the path (same logic as type_determination.rs)
            let Some(target) = local_definition_ref_target(&ref_path) else {
                crate::log_debug!("⚠️ Could not decode nested definition ref: {}", ref_path);
                continue;
            };
            let type_name = to_type_name(&target);

            crate::log_debug!("📖 Nested Definition Ref: '{}' → '{}'", ref_path, type_name);

            // Resolve the reference to get the schema
            if let Some(schema_obj) = self.resolve_ref(&ref_path) {
                // Register reference mapping
                self.discovery
                    .ref_mappings
                    .insert(ref_path.clone(), type_name.clone());
                crate::log_debug!("🔗 Reference mapping: {} → {}", ref_path, type_name);

                // Discover the schema as a type
                crate::log_debug!(
                    "🔍 Discovering nested definition ref type structure for '{}'",
                    type_name
                );
                self.discover_schema_as_type(&schema_obj, &type_name, None, None)?;

                self.discovery.stats.from_definitions += 1;
                crate::log_debug!("✅ Nested definition ref '{}' processed", ref_path);
            } else {
                crate::log_debug!("⚠️ Could not resolve nested definition ref: {}", ref_path);
            }
        }

        Ok(())
    }

    /// Discover nested definitions recursively with proper path accumulation
    fn discover_nested_definitions(
        &mut self,
        nested_definitions: &Map<String, Value>,
        parent_path: &str,
    ) -> TctResult<()> {
        crate::log_debug!(
            "🔄 Processing {} nested definitions under '{}'",
            nested_definitions.len(),
            parent_path
        );

        for (nested_name, nested_schema) in nested_definitions {
            if let Some(schema_obj) = nested_schema.as_object() {
                // Build definition path for cycle detection
                let definition_path = format!("{}/definitions/{}", parent_path, nested_name);

                // Cycle detection: Check if we've already visited this path
                if self.visited_paths.contains(&definition_path) {
                    crate::log_debug!(
                        "⚠️  Skipping already visited definition path: '{}'",
                        definition_path
                    );
                    continue; // Skip this definition to prevent infinite recursion
                }

                // Mark path as visited before processing
                self.visited_paths.insert(definition_path.clone());

                // Create nested type name with path accumulation
                let nested_type_name = to_upper_camel_case(&format!(
                    "{}Definitions{}",
                    to_upper_camel_case(parent_path),
                    to_upper_camel_case(nested_name)
                ));

                crate::log_debug!(
                    "📖 Nested Definition: '{}' in '{}' → '{}'",
                    nested_name,
                    parent_path,
                    nested_type_name
                );

                // Register nested definition info
                let definition_info = DefinitionInfo {
                    name: format!("{}/definitions/{}", parent_path, nested_name),
                    schema: schema_obj.clone(),
                    type_name: nested_type_name.clone(),
                };

                self.discovery.definitions.insert(
                    format!("{}/definitions/{}", parent_path, nested_name),
                    definition_info,
                );

                // Register reference mapping for nested path
                let escaped_parent_path = parent_path
                    .split('/')
                    .map(escape_json_pointer_segment)
                    .collect::<Vec<_>>()
                    .join("/");
                let json_path = format!(
                    "#/definitions/{}/definitions/{}",
                    escaped_parent_path,
                    escape_json_pointer_segment(nested_name)
                );
                self.discovery
                    .ref_mappings
                    .insert(json_path.clone(), nested_type_name.clone());
                crate::log_debug!(
                    "🔗 Nested reference mapping: {} → {}",
                    json_path,
                    nested_type_name
                );

                // Discover the nested definition type structure
                crate::log_debug!(
                    "🔍 Discovering nested definition type structure for '{}'",
                    nested_type_name
                );
                self.discover_schema_as_type(schema_obj, &nested_type_name, None, None)?;

                // Check for further nested definitions (recursive)
                if let Some(deeply_nested) =
                    schema_obj.get("definitions").and_then(Value::as_object)
                {
                    crate::log_debug!(
                        "🔄 Found {} deeply nested definitions in '{}/{}'",
                        deeply_nested.len(),
                        parent_path,
                        nested_name
                    );
                    let nested_path = format!("{}/definitions/{}", parent_path, nested_name);
                    self.discover_nested_definitions(deeply_nested, &nested_path)?;
                }

                self.discovery.stats.from_definitions += 1;
                crate::log_debug!(
                    "✅ Nested definition '{}/{}' processed",
                    parent_path,
                    nested_name
                );
            } else {
                crate::log_debug!(
                    "⚠️ Nested definition '{}/{}' schema is not an object",
                    parent_path,
                    nested_name
                );
            }
        }

        Ok(())
    }

    /// Discover a schema object as a type
    pub(super) fn discover_schema_as_type(
        &mut self,
        schema: &Map<String, Value>,
        type_name: &str,
        parent_type: Option<String>,
        property_name: Option<String>,
    ) -> TctResult<()> {
        self.current_depth += 1;
        self.max_depth = self.max_depth.max(self.current_depth);
        let json_path = self.build_json_path(parent_type.as_deref(), property_name.as_deref());
        if let Err(err) = self.check_schema_depth(self.current_depth, &json_path) {
            self.current_depth -= 1;
            return Err(err);
        }

        crate::log_debug!(
            "🔍 [Depth {}] Analyzing schema for type '{}'",
            self.current_depth,
            type_name
        );
        if let Some(ref parent) = parent_type {
            if let Some(ref prop) = property_name {
                crate::log_debug!("   • Context: Property '{}' of parent '{}'", prop, parent);
            } else {
                crate::log_debug!("   • Context: Child of parent '{}'", parent);
            }
        }

        let schema_location = SchemaLocation {
            json_path,
            description: format!("Type {} from schema", type_name),
            parent_type: parent_type.clone(),
            property_name: property_name.clone(),
        };

        // Determine type kind
        let type_kind = self.determine_type_kind(schema)?;
        crate::log_debug!("🎯 Type '{}' determined as: {:?}", type_name, type_kind);

        // Log schema analysis details
        if let Some(reference) = schema.get("$ref") {
            crate::log_debug!("   • Contains $ref: {}", reference);
        }
        if schema.contains_key("oneOf") || schema.contains_key("anyOf") {
            crate::log_debug!("   • Contains composition (oneOf/anyOf)");
        }
        if let Some(type_val) = schema.get("type") {
            crate::log_debug!("   • JSON Schema type: {}", type_val);
        }
        if let Some(properties) = schema.get("properties")
            && let Some(props_obj) = properties.as_object()
        {
            crate::log_debug!("   • Properties count: {}", props_obj.len());
        }

        // Create type discovery
        let mut type_discovery = TypeDiscovery::new(
            type_name.to_string(),
            type_kind.clone(),
            schema_location,
            schema.clone(),
        );

        // Discover members based on type kind
        crate::log_debug!(
            "🔍 Discovering members for type '{}' (kind: {:?})",
            type_name,
            type_kind
        );
        match &type_kind {
            TypeKind::Struct => {
                crate::log_debug!("   • Processing as struct");
                self.discover_struct_members(schema, &mut type_discovery)?;
            }
            TypeKind::Enum => {
                crate::log_debug!("   • Processing as enum");
                self.discover_enum_variants(schema, &mut type_discovery)?;

                // Post-processing: Detect validation-only anyOf/oneOf
                // When all variants produce identical types after property merging,
                // generate a struct instead of an ambiguous untagged enum
                let is_complex_enum = schema.contains_key("anyOf") || schema.contains_key("oneOf");

                // Check if any variant is an empty schema {} - these are true catch-alls
                let has_empty_schema_variant = if is_complex_enum {
                    let variants = schema
                        .get("anyOf")
                        .or_else(|| schema.get("oneOf"))
                        .and_then(Value::as_array)
                        .map(|arr| {
                            arr.iter()
                                .any(|v| v.as_object().map(|o| o.is_empty()).unwrap_or(false))
                        })
                        .unwrap_or(false);
                    if variants {
                        crate::log_debug!(
                            "   ✓ anyOf/oneOf contains empty schema variant {{}} - will generate enum with catch-all"
                        );
                    }
                    variants
                } else {
                    false
                };

                crate::log_debug!(
                    "is_complex_enum={}, members={}, len={}, has_empty_schema={}",
                    is_complex_enum,
                    type_discovery.members.len(),
                    type_discovery.members.len(),
                    has_empty_schema_variant
                );
                if is_complex_enum
                    && !type_discovery.members.is_empty()
                    && type_discovery.members.len() > 1
                    && !has_empty_schema_variant
                // Skip validation-only conversion if empty schema present
                {
                    crate::log_debug!(
                        "   🔍 Checking {} enum variants for structural equality",
                        type_discovery.members.len()
                    );
                    let first_member = &type_discovery.members[0];
                    crate::log_debug!("     First variant type: {:?}", first_member.member_type);

                    let all_identical =
                        type_discovery.members.iter().enumerate().all(|(idx, m)| {
                            let type_match = m.member_type == first_member.member_type;
                            let schema_match =
                                schemas_structurally_equal(&m.schema, &first_member.schema);
                            crate::log_debug!(
                                "  Variant {}: type={:?}, type_match={}, schema_match={}",
                                idx,
                                m.member_type,
                                type_match,
                                schema_match
                            );
                            type_match && schema_match
                        });

                    if all_identical {
                        crate::log_debug!(
                            "   ✓ Validation-only anyOf/oneOf detected: {} variants produce identical type",
                            type_discovery.members.len()
                        );
                        crate::log_debug!(
                            "   → Converting Enum to Struct (avoids ambiguous untagged enum)"
                        );

                        type_discovery.type_kind = TypeKind::Struct;
                        let first_variant_schema = type_discovery.members[0].schema.clone();
                        type_discovery.members.clear();
                        self.discover_struct_members(&first_variant_schema, &mut type_discovery)?;
                    } else {
                        crate::log_debug!(
                            "   ✗ Variants are structurally different - keeping as Enum"
                        );
                    }
                }
            }
            TypeKind::NestedStruct => {
                crate::log_debug!("   • Processing as nested struct");
                self.discover_struct_members(schema, &mut type_discovery)?;
            }
            TypeKind::ArrayOfComplex => {
                crate::log_debug!("   • Processing as array of complex types");
                // Array items will be discovered separately
            }
            TypeKind::Reference { target } => {
                crate::log_debug!("   • Processing as reference to '{}'", target);
                type_discovery.add_reference(target.clone());
            }
            TypeKind::PrimitiveWrapper { primitive_type } => {
                crate::log_debug!(
                    "   • Processing as primitive wrapper for '{}'",
                    primitive_type
                );
                // Primitive wrappers don't need member discovery - they wrap a single primitive value
            }
            TypeKind::MinimalObjectWrapper => {
                crate::log_debug!("   • Processing as minimal object wrapper");
                // Minimal object wrappers don't need member discovery - they wrap serde_json::Value
            }
        }

        crate::log_debug!(
            "📝 Type '{}' discovered with {} members, {} references",
            type_name,
            type_discovery.members.len(),
            type_discovery.direct_references.len()
        );

        // Register the discovered type
        self.discovery.register_type(type_discovery);

        self.current_depth -= 1;
        crate::log_debug!(
            "✅ [Depth {}] Type '{}' analysis completed",
            self.current_depth + 1,
            type_name
        );
        Ok(())
    }
}
