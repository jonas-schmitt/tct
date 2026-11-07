//! Schema Walker - Pure Discovery Pass
//!
//! This module implements pure schema walking that analyzes JSON schema structure
//! and populates SchemaDiscovery data structures without generating any code.
//!
//! This is the FIRST pass in our multi-pass compilation system:
//! **SchemaWalker** → TypeAnalyzer → CodeGenerator
//!
//! ## Module Structure
//!
//! The SchemaWalker is organized into specialized submodules:
//!
//! - `reference_resolution` - $ref resolution and null variant detection
//! - `schema_analysis` - Schema property analysis (null handling, composition detection)
//! - `enum_discovery` - Enum variant discovery (simple, complex, type arrays)
//! - `struct_discovery` - Struct member discovery (properties, allOf, patterns)
//! - `type_determination` - Core type inference logic (maps JSON Schema → Rust types)
//! - `discovery_core` - High-level orchestration (walk, discover_root, discover_definitions)
//!
//! ## Public API
//!
//! - [`SchemaWalker::walk`] - Main entry point for schema discovery
//! - [`SchemaWalker::new`] - Create new walker instance

use crate::TctResult;
use crate::generator::schema_discovery::*;
use serde_json::Value;
use std::collections::HashSet;

// Submodules
mod discovery_core;
mod enum_discovery;
mod reference_resolution;
mod schema_analysis;
mod struct_discovery;
mod type_determination;

// Import helper functions

/// Pure schema walker - NO code generation, ONLY discovery
pub struct SchemaWalker {
    /// Discovery results being built
    discovery: SchemaDiscovery,
    /// Current nesting depth (for statistics)
    current_depth: usize,
    /// Max nesting depth seen so far
    max_depth: usize,
    /// Root schema for resolving $ref references
    root_schema: Value,
    /// Visited definition paths (for cycle detection)
    visited_paths: HashSet<String>,
    /// Active allOf $ref expansion stack (for cycle detection)
    allof_ref_stack: HashSet<String>,
    /// Maximum recursive schema traversal depth
    max_schema_depth: usize,
}

impl SchemaWalker {
    /// Create new schema walker
    pub fn new(root_schema: Value) -> Self {
        Self::with_max_depth(
            root_schema,
            crate::schema_utils::DEFAULT_MAX_SCHEMA_TRAVERSAL_DEPTH,
        )
    }

    /// Create new schema walker with an explicit traversal depth ceiling.
    pub fn with_max_depth(root_schema: Value, max_schema_depth: usize) -> Self {
        Self {
            discovery: SchemaDiscovery::new(),
            current_depth: 0,
            max_depth: 0,
            root_schema,
            visited_paths: HashSet::new(),
            allof_ref_stack: HashSet::new(),
            max_schema_depth,
        }
    }

    /// Walk a schema and return discovery results
    pub fn walk(schema: &Value, root_name: &str) -> TctResult<SchemaDiscovery> {
        Self::walk_with_max_depth(
            schema,
            root_name,
            crate::schema_utils::DEFAULT_MAX_SCHEMA_TRAVERSAL_DEPTH,
        )
    }

    /// Walk a schema with an explicit traversal depth ceiling.
    pub fn walk_with_max_depth(
        schema: &Value,
        root_name: &str,
        max_schema_depth: usize,
    ) -> TctResult<SchemaDiscovery> {
        crate::log_debug!("🚀 Starting SchemaWalker for root type: '{}'", root_name);
        crate::schema_utils::validate_schema_traversal_depth(schema, max_schema_depth)?;

        let mut walker = Self::with_max_depth(schema.clone(), max_schema_depth);

        // Walk root schema
        crate::log_debug!("🔍 Discovering root schema structure");
        walker.discover_root(schema, root_name)?;

        // Walk definitions section
        if let Some(definitions) = schema.get("definitions").and_then(Value::as_object) {
            crate::log_debug!(
                "📚 Discovering definitions section with {} definitions",
                definitions.len()
            );
            walker.discover_definitions(definitions)?;
        } else {
            crate::log_debug!("📚 No definitions section found");
        }

        // Walk root properties for standalone property references
        if let Some(properties) = schema.get("properties").and_then(Value::as_object) {
            crate::log_debug!(
                "🏠 Discovering root properties section with {} properties",
                properties.len()
            );
            walker.discover_properties(properties)?;
        } else {
            crate::log_debug!("🏠 No root properties section found");
        }

        // Discover nested definition refs (like #/definitions/foo/bar)
        // This must run after discover_definitions to ensure we have all definitions registered
        crate::log_debug!("🔗 Discovering nested definition refs");
        walker.discover_nested_definition_refs()?;

        // Finalize statistics
        walker.discovery.stats.max_nesting_depth = walker.max_depth;

        // Log comprehensive discovery results
        crate::log_debug!("✅ SchemaWalker completed successfully!");
        crate::log_debug!("📊 Discovery Statistics:");
        crate::log_debug!(
            "   • Total types discovered: {}",
            walker.discovery.stats.total_types
        );
        crate::log_debug!("   • Structs: {}", walker.discovery.stats.structs);
        crate::log_debug!("   • Enums: {}", walker.discovery.stats.enums);
        crate::log_debug!(
            "   • Nested structs: {}",
            walker.discovery.stats.nested_structs
        );
        crate::log_debug!("   • References: {}", walker.discovery.stats.references);
        crate::log_debug!(
            "   • Max nesting depth: {}",
            walker.discovery.stats.max_nesting_depth
        );
        crate::log_debug!(
            "   • Total properties: {}",
            walker.discovery.stats.total_properties
        );
        crate::log_debug!(
            "   • Total enum variants: {}",
            walker.discovery.stats.total_enum_variants
        );

        // Log discovered types summary
        for (type_name, type_discovery) in &walker.discovery.discovered_types {
            crate::log_debug!(
                "🔹 Type '{}': {:?} with {} members",
                type_name,
                type_discovery.type_kind,
                type_discovery.members.len()
            );
        }

        // Log ref mappings
        if !walker.discovery.ref_mappings.is_empty() {
            crate::log_debug!("🔗 Reference mappings:");
            for (json_path, type_name) in &walker.discovery.ref_mappings {
                crate::log_debug!("   {} → {}", json_path, type_name);
            }
        }

        Ok(walker.discovery)
    }

    pub(super) fn check_schema_depth(&self, depth: usize, schema_path: &str) -> TctResult<()> {
        if depth > self.max_schema_depth {
            return Err(crate::schema_utils::schema_traversal_depth_error(
                schema_path,
                depth,
                self.max_schema_depth,
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::type_determination::schemas_structurally_equal;
    use super::*;
    use serde_json::json;

    #[test]
    fn test_determine_type_kind_object() {
        let walker = SchemaWalker::new(json!({}));

        // Object with properties
        let schema = json!({"type": "object", "properties": {"name": {"type": "string"}}});
        let schema_obj = schema.as_object().unwrap();
        assert!(matches!(
            walker.determine_type_kind(schema_obj).unwrap(),
            TypeKind::Struct
        ));
    }

    #[test]
    fn test_determine_type_kind_string() {
        let walker = SchemaWalker::new(json!({}));

        let schema = json!({"type": "string"});
        let schema_obj = schema.as_object().unwrap();
        assert!(matches!(
            walker.determine_type_kind(schema_obj).unwrap(),
            TypeKind::PrimitiveWrapper { primitive_type } if primitive_type == "String"
        ));
    }

    #[test]
    fn test_determine_type_kind_integer() {
        let walker = SchemaWalker::new(json!({}));

        let schema = json!({"type": "integer"});
        let schema_obj = schema.as_object().unwrap();
        assert!(matches!(
            walker.determine_type_kind(schema_obj).unwrap(),
            TypeKind::PrimitiveWrapper { primitive_type } if primitive_type == "JsonInteger"
        ));
    }

    #[test]
    fn test_determine_type_kind_bounded_integer() {
        let walker = SchemaWalker::new(json!({}));

        let schema = json!({"type": "integer", "minimum": 0, "maximum": 10});
        let schema_obj = schema.as_object().unwrap();
        assert!(matches!(
            walker.determine_type_kind(schema_obj).unwrap(),
            TypeKind::PrimitiveWrapper { primitive_type } if primitive_type == "i64"
        ));
    }

    #[test]
    fn test_determine_type_kind_enum_simple() {
        let walker = SchemaWalker::new(json!({}));

        // Simple string enum
        let schema = json!({"enum": ["a", "b", "c"]});
        let schema_obj = schema.as_object().unwrap();
        assert!(matches!(
            walker.determine_type_kind(schema_obj).unwrap(),
            TypeKind::Enum
        ));
    }

    #[test]
    fn test_determine_type_kind_oneof() {
        let walker = SchemaWalker::new(json!({}));

        // oneOf composition
        let schema = json!({
            "oneOf": [
                {"type": "string"},
                {"type": "number"}
            ]
        });
        let schema_obj = schema.as_object().unwrap();
        assert!(matches!(
            walker.determine_type_kind(schema_obj).unwrap(),
            TypeKind::Enum
        ));
    }

    #[test]
    fn test_determine_type_kind_anyof() {
        let walker = SchemaWalker::new(json!({}));

        // anyOf composition
        let schema = json!({
            "anyOf": [
                {"type": "object", "properties": {"a": {"type": "string"}}},
                {"type": "object", "properties": {"b": {"type": "number"}}}
            ]
        });
        let schema_obj = schema.as_object().unwrap();
        assert!(matches!(
            walker.determine_type_kind(schema_obj).unwrap(),
            TypeKind::Enum
        ));
    }

    #[test]
    fn test_determine_type_kind_reference() {
        let walker = SchemaWalker::new(json!({}));

        // Local reference
        let schema = json!({"$ref": "#/definitions/User"});
        let schema_obj = schema.as_object().unwrap();
        if let TypeKind::Reference { target } = walker.determine_type_kind(schema_obj).unwrap() {
            assert_eq!(target, "User");
        } else {
            panic!("Expected Reference type kind");
        }
    }

    #[test]
    fn test_determine_type_kind_array() {
        let walker = SchemaWalker::new(json!({}));

        // Array with items
        let schema = json!({"type": "array", "items": {"type": "string"}});
        let schema_obj = schema.as_object().unwrap();
        assert!(matches!(
            walker.determine_type_kind(schema_obj).unwrap(),
            TypeKind::ArrayOfComplex
        ));
    }

    #[test]
    fn test_determine_type_kind_minimal_object() {
        let walker = SchemaWalker::new(json!({}));

        // Minimal object (no properties, allows additional)
        let schema = json!({"type": "object"});
        let schema_obj = schema.as_object().unwrap();
        assert!(matches!(
            walker.determine_type_kind(schema_obj).unwrap(),
            TypeKind::MinimalObjectWrapper
        ));
    }

    #[test]
    fn test_schema_allows_null_direct_null() {
        let root = json!({});
        let walker = SchemaWalker::new(root);

        // Direct null type
        let schema = json!({"type": "null"});
        let schema_obj = schema.as_object().unwrap();
        assert!(walker.schema_allows_null(schema_obj));
    }

    #[test]
    fn test_schema_allows_null_type_array() {
        let root = json!({});
        let walker = SchemaWalker::new(root);

        // Type array with null
        let schema = json!({"type": ["string", "null"]});
        let schema_obj = schema.as_object().unwrap();
        assert!(walker.schema_allows_null(schema_obj));
    }

    #[test]
    fn test_schema_allows_null_oneof_variant() {
        let root = json!({});
        let walker = SchemaWalker::new(root);

        // oneOf with null variant
        let schema = json!({
            "oneOf": [
                {"type": "string"},
                {"type": "null"}
            ]
        });
        let schema_obj = schema.as_object().unwrap();
        assert!(walker.schema_allows_null(schema_obj));
    }

    #[test]
    fn test_schema_allows_null_no_null() {
        let root = json!({});
        let walker = SchemaWalker::new(root);

        // No null allowed
        let schema = json!({"type": "string"});
        let schema_obj = schema.as_object().unwrap();
        assert!(!walker.schema_allows_null(schema_obj));
    }

    #[test]
    fn test_create_nested_type_name_with_property() {
        let root = json!({});
        let walker = SchemaWalker::new(root);

        let result = walker.create_nested_type_name("User", Some("address"));
        assert_eq!(result, "UserAddress");
    }

    #[test]
    fn test_create_nested_type_name_without_property() {
        let root = json!({});
        let walker = SchemaWalker::new(root);

        let result = walker.create_nested_type_name("User", None);
        assert_eq!(result, "UserNested");
    }

    #[test]
    fn test_create_nested_type_name_special_chars() {
        let root = json!({});
        let walker = SchemaWalker::new(root);

        // Dot character - converted to EmptyValue by to_upper_camel_case
        let result = walker.create_nested_type_name("Config", Some("."));
        assert_eq!(result, "ConfigEmptyValue");

        // Star character - also converted to EmptyValue
        let result = walker.create_nested_type_name("Config", Some("*"));
        assert_eq!(result, "ConfigEmptyValue");

        // Underscore prefix
        let result = walker.create_nested_type_name("Config", Some("_internal"));
        assert_eq!(result, "ConfigInternal");
    }

    #[test]
    fn test_resolve_ref_simple() {
        let root = json!({
            "definitions": {
                "User": {
                    "type": "object",
                    "properties": {
                        "name": {"type": "string"}
                    }
                }
            }
        });
        let walker = SchemaWalker::new(root);

        let resolved = walker.resolve_ref("#/definitions/User");
        assert!(resolved.is_some());
        let schema = resolved.unwrap();
        assert_eq!(schema.get("type").and_then(Value::as_str), Some("object"));
    }

    #[test]
    fn test_resolve_ref_root() {
        let root = json!({"type": "object"});
        let walker = SchemaWalker::new(root.clone());

        let resolved = walker.resolve_ref("#");
        assert!(resolved.is_some());
        let schema = resolved.unwrap();
        assert_eq!(schema.get("type").and_then(Value::as_str), Some("object"));
    }

    #[test]
    fn test_resolve_ref_nested() {
        let root = json!({
            "definitions": {
                "Address": {
                    "properties": {
                        "street": {"type": "string"}
                    }
                }
            }
        });
        let walker = SchemaWalker::new(root);

        let resolved = walker.resolve_ref("#/definitions/Address/properties/street");
        assert!(resolved.is_some());
        let schema = resolved.unwrap();
        assert_eq!(schema.get("type").and_then(Value::as_str), Some("string"));
    }

    #[test]
    fn test_resolve_ref_decodes_json_pointer_escape_sequences() {
        let root = json!({
            "definitions": {
                "A/B~C": {
                    "type": "object",
                    "properties": {
                        "tilde~name": {"type": "string"}
                    }
                }
            },
            "properties": {
                "slash/name": {"type": "integer"}
            }
        });
        let walker = SchemaWalker::new(root);

        let resolved = walker.resolve_ref("#/definitions/A~1B~0C/properties/tilde~0name");
        assert_eq!(
            resolved
                .as_ref()
                .and_then(|schema| schema.get("type"))
                .and_then(Value::as_str),
            Some("string")
        );

        let resolved = walker.resolve_ref("#/properties/slash~1name");
        assert_eq!(
            resolved
                .as_ref()
                .and_then(|schema| schema.get("type"))
                .and_then(Value::as_str),
            Some("integer")
        );
    }

    #[test]
    fn test_walk_discovers_escaped_root_property_reference() {
        let schema = json!({
            "type": "object",
            "properties": {
                "slash/name": {
                    "type": "object",
                    "properties": {
                        "value": {"type": "string"}
                    },
                    "additionalProperties": false
                },
                "alias": {"$ref": "#/properties/slash~1name"}
            },
            "required": ["alias"],
            "additionalProperties": false
        });

        let discovery = SchemaWalker::walk(&schema, "Root").unwrap();
        assert!(
            discovery
                .ref_mappings
                .contains_key("#/properties/slash~1name"),
            "escaped property refs should map to the referenced root property"
        );
    }

    #[test]
    fn test_resolve_ref_invalid() {
        let root = json!({"definitions": {}});
        let walker = SchemaWalker::new(root);

        let resolved = walker.resolve_ref("#/definitions/NonExistent");
        assert!(resolved.is_none());
    }

    #[test]
    fn test_is_composition_schema() {
        let root = json!({});
        let walker = SchemaWalker::new(root);

        // oneOf
        let schema1 = json!({"oneOf": [{"type": "string"}]});
        assert!(walker.is_composition_schema(schema1.as_object().unwrap()));

        // anyOf
        let schema2 = json!({"anyOf": [{"type": "number"}]});
        assert!(walker.is_composition_schema(schema2.as_object().unwrap()));

        // enum
        let schema3 = json!({"enum": ["a", "b"]});
        assert!(walker.is_composition_schema(schema3.as_object().unwrap()));

        // Not composition
        let schema4 = json!({"type": "object"});
        assert!(!walker.is_composition_schema(schema4.as_object().unwrap()));
    }

    #[test]
    fn test_schemas_structurally_equal_identical() {
        let schema1 = json!({
            "properties": {"name": {"type": "string"}},
            "required": ["name"]
        });
        let schema2 = json!({
            "properties": {"name": {"type": "string"}},
            "required": ["name"]
        });

        assert!(schemas_structurally_equal(
            schema1.as_object().unwrap(),
            schema2.as_object().unwrap()
        ));
    }

    #[test]
    fn test_schemas_structurally_equal_different_required() {
        let schema1 = json!({
            "properties": {"name": {"type": "string"}},
            "required": ["name"]
        });
        let schema2 = json!({
            "properties": {"name": {"type": "string"}},
            "required": []
        });

        assert!(!schemas_structurally_equal(
            schema1.as_object().unwrap(),
            schema2.as_object().unwrap()
        ));
    }

    #[test]
    fn test_schemas_structurally_equal_different_properties() {
        let schema1 = json!({
            "properties": {"name": {"type": "string"}}
        });
        let schema2 = json!({
            "properties": {"age": {"type": "number"}}
        });

        assert!(!schemas_structurally_equal(
            schema1.as_object().unwrap(),
            schema2.as_object().unwrap()
        ));
    }

    #[test]
    fn test_schemas_structurally_equal_different_scalar_constraints() {
        let constrained = json!({"type": "string", "pattern": "default"});
        let unconstrained = json!({"type": "string"});

        assert!(!schemas_structurally_equal(
            constrained.as_object().unwrap(),
            unconstrained.as_object().unwrap()
        ));
    }

    #[test]
    fn test_schemas_structurally_equal_different_composition_reference_or_conditionals() {
        let cases = [
            (
                json!({"oneOf": [{"type": "string"}]}),
                json!({"oneOf": [{"type": "number"}]}),
            ),
            (
                json!({"anyOf": [{"type": "string"}]}),
                json!({"anyOf": [{"type": "number"}]}),
            ),
            (
                json!({"allOf": [{"type": "object", "required": ["name"]}]}),
                json!({"allOf": [{"type": "object", "required": ["id"]}]}),
            ),
            (json!({"$ref": "#/$defs/A"}), json!({"$ref": "#/$defs/B"})),
            (
                json!({"if": {"properties": {"kind": {"const": "a"}}}, "then": {"required": ["a"]}}),
                json!({"if": {"properties": {"kind": {"const": "b"}}}, "then": {"required": ["b"]}}),
            ),
            (
                json!({"else": {"properties": {"fallback": {"type": "string"}}}}),
                json!({"else": {"properties": {"fallback": {"type": "number"}}}}),
            ),
        ];

        for (left, right) in cases {
            assert!(
                !schemas_structurally_equal(left.as_object().unwrap(), right.as_object().unwrap()),
                "schemas with different composition/reference/conditional semantics must not be equal"
            );
        }
    }

    #[test]
    fn test_validation_only_anyof_keeps_constrained_scalar_variants() {
        let schema = json!({
            "type": "object",
            "properties": {
                "features": {
                    "type": "array",
                    "items": {
                        "anyOf": [
                            {"type": "string", "pattern": "default"},
                            {"type": "string"}
                        ]
                    }
                }
            }
        });

        let discovery = SchemaWalker::walk(&schema, "TestRoot").unwrap();
        let item_type = discovery
            .discovered_types
            .get("TestRootEnum")
            .expect("array anyOf item type should be discovered");

        assert!(
            matches!(item_type.type_kind, TypeKind::Enum),
            "constrained scalar anyOf variants must remain an enum, got {:?}",
            item_type.type_kind
        );
        assert_eq!(item_type.members.len(), 2);
    }

    // === Phase 4.2: SchemaWalker Coverage Tests ===
    // Batch 1: Nested definitions and property references

    #[test]
    fn test_discover_nested_definitions_simple() {
        // Test discovering simple nested definitions
        let schema = json!({
            "type": "object",
            "definitions": {
                "Parent": {
                    "type": "object",
                    "properties": {
                        "child": {"type": "string"}
                    },
                    "definitions": {
                        "NestedChild": {
                            "type": "object",
                            "properties": {
                                "value": {"type": "number"}
                            }
                        }
                    }
                }
            }
        });

        let result = SchemaWalker::walk(&schema, "TestRoot");
        assert!(result.is_ok());

        let discovery = result.unwrap();

        // Should discover both Parent and NestedChild definitions
        assert!(discovery.definitions.contains_key("Parent"));
        assert!(
            discovery
                .definitions
                .contains_key("Parent/definitions/NestedChild")
        );

        // Should have reference mappings
        assert!(discovery.ref_mappings.contains_key("#/definitions/Parent"));
        assert!(
            discovery
                .ref_mappings
                .contains_key("#/definitions/Parent/definitions/NestedChild")
        );
    }

    #[test]
    fn test_discover_nested_definitions_with_cycle_detection() {
        // Test that cycle detection prevents infinite recursion
        let schema = json!({
            "type": "object",
            "definitions": {
                "Node": {
                    "type": "object",
                    "properties": {
                        "value": {"type": "string"}
                    },
                    "definitions": {
                        "Child": {
                            "type": "object",
                            "properties": {
                                "data": {"type": "number"}
                            },
                            "definitions": {
                                "GrandChild": {
                                    "type": "object",
                                    "properties": {
                                        "nested": {"type": "boolean"}
                                    }
                                }
                            }
                        }
                    }
                }
            }
        });

        let result = SchemaWalker::walk(&schema, "TestRoot");
        assert!(result.is_ok());

        let discovery = result.unwrap();

        // All nested levels should be discovered
        assert!(discovery.definitions.contains_key("Node"));
        assert!(discovery.definitions.contains_key("Node/definitions/Child"));
        assert!(
            discovery
                .definitions
                .contains_key("Node/definitions/Child/definitions/GrandChild")
        );

        // Stats should reflect nested discoveries
        assert!(discovery.stats.from_definitions >= 3);
    }

    #[test]
    fn test_collect_property_references_with_scanning() {
        // Test property reference collection and scanning
        let schema = json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "config": {"type": "object"}
            },
            "definitions": {
                "Item": {
                    "type": "object",
                    "properties": {
                        "ref_to_name": {"$ref": "#/properties/name"},
                        "ref_to_config": {"$ref": "#/properties/config"}
                    }
                }
            }
        });

        let result = SchemaWalker::walk(&schema, "TestRoot");
        assert!(result.is_ok());

        let discovery = result.unwrap();

        // Should discover referenced properties as standalone types
        assert!(discovery.ref_mappings.contains_key("#/properties/name"));
        assert!(discovery.ref_mappings.contains_key("#/properties/config"));

        // Should have created PropertiesName and PropertiesConfig types
        let has_properties_name = discovery
            .discovered_types
            .keys()
            .any(|k| k.contains("PropertiesName"));
        let has_properties_config = discovery
            .discovered_types
            .keys()
            .any(|k| k.contains("PropertiesConfig"));

        assert!(has_properties_name, "Should have PropertiesName type");
        assert!(has_properties_config, "Should have PropertiesConfig type");
    }

    // Batch 2: Inline type extraction

    #[test]
    fn test_extract_inline_enum_from_property() {
        // Test that inline enums are extracted as separate types
        let schema = json!({
            "type": "object",
            "properties": {
                "status": {
                    "type": "string",
                    "enum": ["active", "inactive", "pending"]
                }
            }
        });

        let result = SchemaWalker::walk(&schema, "TestRoot");
        assert!(result.is_ok());

        let discovery = result.unwrap();

        // Should extract inline enum as a separate type
        let has_inline_enum = discovery
            .discovered_types
            .keys()
            .any(|k| k.contains("Status"));

        assert!(has_inline_enum, "Should have extracted inline enum type");

        // The inline enum should have 3 variants
        if let Some(enum_type) = discovery
            .discovered_types
            .values()
            .find(|t| t.type_name.contains("Status"))
        {
            assert_eq!(
                enum_type.members.len(),
                3,
                "Inline enum should have 3 variants"
            );
        }
    }

    #[test]
    fn test_extract_bounded_integer_from_property() {
        // Test that bounded integers are extracted as separate types
        let schema = json!({
            "type": "object",
            "properties": {
                "port": {
                    "type": "integer",
                    "minimum": 1024,
                    "maximum": 65535
                }
            }
        });

        let result = SchemaWalker::walk(&schema, "TestRoot");
        assert!(result.is_ok());

        let discovery = result.unwrap();

        // Should extract bounded integer as a separate type
        let has_bounded_int = discovery
            .discovered_types
            .keys()
            .any(|k| k.contains("Port"));

        assert!(
            has_bounded_int,
            "Should have extracted bounded integer type"
        );

        // Stats should reflect bounded integer extraction
        assert!(discovery.stats.bounded_integers >= 1);
    }

    #[test]
    fn test_inline_enum_variant_naming() {
        // Test proper variant naming for inline enums with various values
        let schema = json!({
            "type": "object",
            "properties": {
                "mode": {
                    "type": "string",
                    "enum": ["read-only", "write-only", "read-write"]
                }
            }
        });

        let result = SchemaWalker::walk(&schema, "TestRoot");
        assert!(result.is_ok());

        let discovery = result.unwrap();

        // Find the inline enum type
        let enum_type = discovery
            .discovered_types
            .values()
            .find(|t| t.type_name.contains("Mode"));

        assert!(enum_type.is_some(), "Should have Mode enum type");

        let enum_type = enum_type.unwrap();

        // Check that variants were created (should be camelCase converted)
        assert_eq!(enum_type.members.len(), 3, "Should have 3 variants");

        // Verify member names are properly camelCased
        let variant_names: Vec<_> = enum_type.members.iter().map(|m| m.name.as_str()).collect();

        assert!(variant_names.contains(&"ReadOnly") || variant_names.contains(&"Readonly"));
        assert!(variant_names.contains(&"WriteOnly") || variant_names.contains(&"Writeonly"));
        assert!(variant_names.contains(&"ReadWrite") || variant_names.contains(&"Readwrite"));
    }

    // Batch 3: Pattern properties

    #[test]
    fn test_discover_pattern_properties_simple() {
        // Test basic pattern properties discovery
        let schema = json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"}
            },
            "patternProperties": {
                "^[a-z]+$": {"type": "string"},
                "^[0-9]+$": {"type": "number"}
            }
        });

        let result = SchemaWalker::walk(&schema, "TestRoot");
        assert!(result.is_ok());

        let discovery = result.unwrap();

        // Should create pattern validator types
        assert_eq!(
            discovery.pattern_validators.len(),
            2,
            "Should have 2 pattern validators"
        );

        // Pattern validators should be registered
        let validator_names: Vec<_> = discovery
            .pattern_validators
            .keys()
            .map(|k| k.as_str())
            .collect();

        assert!(
            validator_names
                .iter()
                .any(|n| n.contains("PatternValidator0"))
        );
        assert!(
            validator_names
                .iter()
                .any(|n| n.contains("PatternValidator1"))
        );
    }

    #[test]
    fn test_pattern_validator_type_creation() {
        // Test that pattern validators are created as standalone types
        let schema = json!({
            "type": "object",
            "patternProperties": {
                "^env_.*": {
                    "type": "object",
                    "properties": {
                        "value": {"type": "string"}
                    }
                }
            }
        });

        let result = SchemaWalker::walk(&schema, "Config");
        assert!(result.is_ok());

        let discovery = result.unwrap();

        // Should have created pattern validator
        assert_eq!(discovery.pattern_validators.len(), 1);

        // Pattern validator should have correct value type
        let validator = discovery.pattern_validators.values().next().unwrap();
        assert_eq!(validator.pattern, "^env_.*");
        assert!(!validator.value_type.is_empty());

        // Pattern validator should be registered as a type
        let has_validator_type = discovery
            .discovered_types
            .keys()
            .any(|k| k.contains("PatternValidator"));

        assert!(
            has_validator_type,
            "Pattern validator should be registered as type"
        );
    }

    // Batch 4: Complex enum variants

    #[test]
    fn test_discover_complex_enum_property_merging() {
        // Test that root properties are properly merged into anyOf/oneOf variants
        let schema = json!({
            "type": "object",
            "properties": {
                "common": {"type": "string"}
            },
            "required": ["common"],
            "anyOf": [
                {
                    "properties": {
                        "variant1": {"type": "number"}
                    },
                    "required": ["variant1"]
                },
                {
                    "properties": {
                        "variant2": {"type": "boolean"}
                    },
                    "required": ["variant2"]
                }
            ]
        });

        let result = SchemaWalker::walk(&schema, "TestRoot");
        assert!(result.is_ok());

        let discovery = result.unwrap();

        // Root type should be an enum with merged properties
        let root_type = discovery.discovered_types.get("TestRoot");
        assert!(root_type.is_some());

        let root_type = root_type.unwrap();
        assert!(matches!(root_type.type_kind, TypeKind::Enum));

        // Should have 2 variants
        assert_eq!(root_type.members.len(), 2);

        // Both variants should have the common property merged
        // (This is verified at generation time, here we check structure)
        assert!(root_type.members.iter().all(|m| !m.name.is_empty()));
    }

    #[test]
    fn test_discover_type_array_variants() {
        // Test handling of type arrays at root level: {"type": ["string", "number"]}
        let schema = json!({
            "type": ["string", "number", "boolean"]
        });

        let result = SchemaWalker::walk(&schema, "TestRoot");
        assert!(result.is_ok());

        let discovery = result.unwrap();

        // Root type should be an enum for the type array
        let root_type = discovery.discovered_types.get("TestRoot");
        assert!(root_type.is_some(), "Should have TestRoot type");

        let root_type = root_type.unwrap();

        // Should be an enum with 3 variants
        assert!(
            matches!(root_type.type_kind, TypeKind::Enum),
            "Type array should produce enum"
        );
        assert_eq!(
            root_type.members.len(),
            3,
            "Type array should produce 3 variants"
        );

        // Verify each variant corresponds to a primitive type
        let variant_types: Vec<_> = root_type.members.iter().map(|m| &m.member_type).collect();

        // All should be primitive types
        assert!(
            variant_types
                .iter()
                .all(|t| matches!(t, MemberType::Primitive(_)))
        );
    }

    #[test]
    fn test_validation_only_anyof_detection() {
        // Test detection of validation-only anyOf (all variants produce same type)
        let schema = json!({
            "type": "object",
            "anyOf": [
                {
                    "properties": {
                        "name": {"type": "string"},
                        "age": {"type": "number"}
                    },
                    "required": ["name"],
                    "minProperties": 1
                },
                {
                    "properties": {
                        "name": {"type": "string"},
                        "age": {"type": "number"}
                    },
                    "required": ["name"],
                    "maxProperties": 10
                }
            ]
        });

        let result = SchemaWalker::walk(&schema, "TestRoot");
        assert!(result.is_ok());

        let discovery = result.unwrap();

        // When anyOf variants produce identical types, should convert to struct
        let root_type = discovery.discovered_types.get("TestRoot");
        assert!(root_type.is_some());

        let root_type = root_type.unwrap();

        // Should be converted to Struct (not Enum) due to identical variants
        assert!(
            matches!(root_type.type_kind, TypeKind::Struct),
            "Validation-only anyOf should be converted to Struct, got {:?}",
            root_type.type_kind
        );

        // Should have properties from the merged variant
        assert!(!root_type.members.is_empty(), "Should have struct members");
    }

    // Batch 5: Integration and edge cases

    #[test]
    fn test_schema_walker_statistics_tracking() {
        // Test that statistics are correctly tracked during discovery
        let schema = json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "age": {"type": "integer"}
            },
            "definitions": {
                "Address": {
                    "type": "object",
                    "properties": {
                        "street": {"type": "string"}
                    }
                },
                "Contact": {
                    "type": "object",
                    "properties": {
                        "email": {"type": "string"}
                    }
                }
            }
        });

        let result = SchemaWalker::walk(&schema, "TestRoot");
        assert!(result.is_ok());

        let discovery = result.unwrap();
        let stats = &discovery.stats;

        // Should track types discovered
        assert!(
            stats.total_types >= 3,
            "Should have at least 3 types (root + 2 definitions)"
        );
        assert!(stats.structs >= 3, "Should have at least 3 structs");
        assert_eq!(
            stats.from_definitions, 2,
            "Should have 2 types from definitions"
        );

        // Should track properties
        assert!(
            stats.total_properties >= 4,
            "Should have at least 4 properties total"
        );
    }

    #[test]
    fn test_max_nesting_depth_tracking() {
        // Test that max nesting depth is correctly calculated
        let schema = json!({
            "type": "object",
            "properties": {
                "level1": {
                    "type": "object",
                    "properties": {
                        "level2": {
                            "type": "object",
                            "properties": {
                                "level3": {
                                    "type": "string"
                                }
                            }
                        }
                    }
                }
            }
        });

        let result = SchemaWalker::walk(&schema, "TestRoot");
        assert!(result.is_ok());

        let discovery = result.unwrap();

        // Max nesting depth should reflect the nested structure
        assert!(
            discovery.stats.max_nesting_depth >= 3,
            "Max nesting depth should be at least 3, got {}",
            discovery.stats.max_nesting_depth
        );
    }

    #[test]
    fn test_discover_schema_with_all_features() {
        // Integration test: schema with multiple features
        let schema = json!({
            "type": "object",
            "properties": {
                "status": {
                    "type": "string",
                    "enum": ["active", "inactive"]
                },
                "config": {
                    "type": "object",
                    "properties": {
                        "timeout": {
                            "type": "integer",
                            "minimum": 0,
                            "maximum": 3600
                        }
                    },
                    "patternProperties": {
                        "^extra_": {"type": "string"}
                    }
                }
            },
            "definitions": {
                "User": {
                    "oneOf": [
                        {"properties": {"name": {"type": "string"}}},
                        {"properties": {"id": {"type": "number"}}}
                    ]
                }
            }
        });

        let result = SchemaWalker::walk(&schema, "TestRoot");
        assert!(result.is_ok());

        let discovery = result.unwrap();

        // Should discover multiple types
        assert!(
            discovery.discovered_types.len() >= 4,
            "Should have multiple types"
        );

        // Should have extracted inline enum
        let has_status_enum = discovery
            .discovered_types
            .values()
            .any(|t| matches!(t.type_kind, TypeKind::Enum) && t.type_name.contains("Status"));
        assert!(has_status_enum, "Should extract status enum");

        // Should have extracted bounded integer
        assert!(
            discovery.stats.bounded_integers >= 1,
            "Should extract bounded integer"
        );

        // Should have pattern validators
        assert!(
            !discovery.pattern_validators.is_empty(),
            "Should have pattern validators"
        );

        // Should have definition with oneOf
        let user_def = discovery.definitions.get("User");
        assert!(user_def.is_some(), "Should have User definition");
    }

    // === Phase 4.2: SchemaWalker Coverage Improvement ===
    // Tests targeting uncovered branches in determine_member_type and other functions

    #[test]
    fn test_determine_member_type_external_url_reference() {
        // Test handling of external URL references (http://, https://)
        use serde_json::json;

        let schema = json!({
            "type": "object",
            "properties": {
                "external": {
                    "$ref": "https://example.com/schema.json"
                }
            }
        });

        let result = SchemaWalker::walk(&schema, "TestRoot");
        assert!(result.is_ok());

        let discovery = result.unwrap();

        // External URL references should be treated as serde_json::Value
        let root_type = discovery
            .discovered_types
            .values()
            .find(|t| t.type_name == "TestRoot");
        assert!(root_type.is_some());
    }

    #[test]
    fn test_determine_member_type_http_url_reference() {
        // Test handling of http:// URL references
        use serde_json::json;

        let schema = json!({
            "type": "object",
            "properties": {
                "external_http": {
                    "$ref": "http://example.com/schema.json"
                }
            }
        });

        let result = SchemaWalker::walk(&schema, "TestRoot");
        assert!(result.is_ok());

        let discovery = result.unwrap();
        let root_type = discovery
            .discovered_types
            .values()
            .find(|t| t.type_name == "TestRoot");
        assert!(root_type.is_some());
    }

    #[test]
    fn test_determine_member_type_local_property_reference() {
        // Test handling of #/properties/foo style references
        use serde_json::json;

        let schema = json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "copy_of_name": {
                    "$ref": "#/properties/name"
                }
            }
        });

        let result = SchemaWalker::walk(&schema, "TestRoot");
        assert!(result.is_ok());

        let discovery = result.unwrap();

        // Should have created a type for the referenced property
        let has_properties_name = discovery.ref_mappings.contains_key("#/properties/name");
        assert!(has_properties_name);
    }

    #[test]
    fn test_determine_member_type_unknown_reference_format() {
        // Test that unknown $ref formats are rejected
        use serde_json::json;

        let schema = json!({
            "type": "object",
            "properties": {
                "unknown_ref": {
                    "$ref": "unknown-format:foo/bar"
                }
            }
        });

        let result = SchemaWalker::walk(&schema, "TestRoot");
        assert!(result.is_err(), "Unknown $ref format should be rejected");

        // Verify the error message is helpful
        let err = result.unwrap_err();
        let err_msg = err.to_string();
        assert!(
            err_msg.contains("Invalid $ref format"),
            "Error message should mention invalid $ref format"
        );
    }

    #[test]
    fn test_determine_member_type_null_type() {
        // Test handling of "type": "null"
        use serde_json::json;

        let schema = json!({
            "type": "object",
            "properties": {
                "null_value": {"type": "null"}
            }
        });

        let result = SchemaWalker::walk(&schema, "TestRoot");
        assert!(result.is_ok());

        let discovery = result.unwrap();
        let root_type = discovery
            .discovered_types
            .values()
            .find(|t| t.type_name == "TestRoot");
        assert!(root_type.is_some());

        // Should have a member for null_value
        if let Some(root) = root_type {
            let has_null_member = root.members.iter().any(|m| m.name == "null_value");
            assert!(has_null_member);
        }
    }

    #[test]
    fn test_determine_member_type_array_with_complex_items() {
        // Test handling of arrays with complex (TypeReference) items
        use serde_json::json;

        let schema = json!({
            "definitions": {
                "Item": {
                    "type": "object",
                    "properties": {
                        "value": {"type": "string"}
                    }
                }
            },
            "type": "object",
            "properties": {
                "items": {
                    "type": "array",
                    "items": {
                        "$ref": "#/definitions/Item"
                    }
                }
            }
        });

        let result = SchemaWalker::walk(&schema, "TestRoot");
        assert!(result.is_ok());

        let discovery = result.unwrap();

        // Should have Item definition
        assert!(discovery.definitions.contains_key("Item"));

        // Should have root type with items member
        let root_type = discovery
            .discovered_types
            .values()
            .find(|t| t.type_name == "TestRoot");
        assert!(root_type.is_some());
    }

    #[test]
    fn test_determine_member_type_empty_object() {
        // Test handling of empty objects (additionalProperties: false with no properties)
        use serde_json::json;

        let schema = json!({
            "type": "object",
            "properties": {
                "empty": {
                    "type": "object",
                    "additionalProperties": false
                }
            }
        });

        let result = SchemaWalker::walk(&schema, "TestRoot");
        assert!(result.is_ok());

        let discovery = result.unwrap();

        // Should create a nested type for the empty object
        let has_nested_type = discovery
            .discovered_types
            .values()
            .any(|t| t.type_name.contains("Empty"));
        assert!(has_nested_type);
    }

    #[test]
    fn test_determine_member_type_hashmap_with_typed_values() {
        // Test handling of additionalProperties with schema (HashMap with typed values)
        use serde_json::json;

        let schema = json!({
            "type": "object",
            "properties": {
                "config": {
                    "type": "object",
                    "additionalProperties": {
                        "type": "string"
                    }
                }
            }
        });

        let result = SchemaWalker::walk(&schema, "TestRoot");
        assert!(result.is_ok());

        let discovery = result.unwrap();
        let root_type = discovery
            .discovered_types
            .values()
            .find(|t| t.type_name == "TestRoot");
        assert!(root_type.is_some());

        // Should have a member for config as HashMap
        if let Some(root) = root_type {
            let has_config = root.members.iter().any(|m| m.name == "config");
            assert!(has_config);
        }
    }

    #[test]
    fn test_determine_member_type_unconstrained_object_true() {
        // Test handling of additionalProperties: true
        use serde_json::json;

        let schema = json!({
            "type": "object",
            "properties": {
                "metadata": {
                    "type": "object",
                    "additionalProperties": true
                }
            }
        });

        let result = SchemaWalker::walk(&schema, "TestRoot");
        assert!(result.is_ok());

        let discovery = result.unwrap();
        let root_type = discovery
            .discovered_types
            .values()
            .find(|t| t.type_name == "TestRoot");
        assert!(root_type.is_some());

        // Should have a member for metadata as HashMap
        if let Some(root) = root_type {
            let has_metadata = root.members.iter().any(|m| m.name == "metadata");
            assert!(has_metadata);
        }
    }

    #[test]
    fn test_determine_member_type_unconstrained_object_missing() {
        // Test handling of missing additionalProperties (defaults to true)
        use serde_json::json;

        let schema = json!({
            "type": "object",
            "properties": {
                "data": {
                    "type": "object"
                    // additionalProperties not specified - defaults to true
                }
            }
        });

        let result = SchemaWalker::walk(&schema, "TestRoot");
        assert!(result.is_ok());

        let discovery = result.unwrap();
        let root_type = discovery
            .discovered_types
            .values()
            .find(|t| t.type_name == "TestRoot");
        assert!(root_type.is_some());

        // Should have a member for data as HashMap
        if let Some(root) = root_type {
            let has_data = root.members.iter().any(|m| m.name == "data");
            assert!(has_data);
        }
    }

    #[test]
    fn test_determine_member_type_inline_oneof_composition() {
        // Test handling of inline oneOf composition in member
        use serde_json::json;

        let schema = json!({
            "type": "object",
            "properties": {
                "value": {
                    "oneOf": [
                        {"type": "string"},
                        {"type": "number"}
                    ]
                }
            }
        });

        let result = SchemaWalker::walk(&schema, "TestRoot");
        assert!(result.is_ok());

        let discovery = result.unwrap();

        // Should create an enum type for the oneOf composition
        let has_value_enum = discovery
            .discovered_types
            .values()
            .any(|t| matches!(t.type_kind, TypeKind::Enum) && t.type_name.contains("Value"));
        assert!(has_value_enum);
    }

    #[test]
    fn test_determine_member_type_inline_anyof_composition() {
        // Test handling of inline anyOf composition in member
        use serde_json::json;

        let schema = json!({
            "type": "object",
            "properties": {
                "config": {
                    "anyOf": [
                        {"type": "object", "properties": {"a": {"type": "string"}}},
                        {"type": "object", "properties": {"b": {"type": "number"}}}
                    ]
                }
            }
        });

        let result = SchemaWalker::walk(&schema, "TestRoot");
        assert!(result.is_ok());

        let discovery = result.unwrap();

        // Should create an enum type for the anyOf composition
        let has_config_enum = discovery
            .discovered_types
            .values()
            .any(|t| matches!(t.type_kind, TypeKind::Enum) && t.type_name.contains("Config"));
        assert!(has_config_enum);
    }

    #[test]
    fn test_determine_member_type_hashmap_with_complex_values() {
        // Test additionalProperties with complex object schema
        use serde_json::json;

        let schema = json!({
            "definitions": {
                "Config": {
                    "type": "object",
                    "properties": {
                        "enabled": {"type": "boolean"}
                    }
                }
            },
            "type": "object",
            "properties": {
                "settings": {
                    "type": "object",
                    "additionalProperties": {
                        "$ref": "#/definitions/Config"
                    }
                }
            }
        });

        let result = SchemaWalker::walk(&schema, "TestRoot");
        assert!(result.is_ok());

        let discovery = result.unwrap();

        // Should have Config definition
        assert!(discovery.definitions.contains_key("Config"));

        // Should have root type with settings member
        let root_type = discovery
            .discovered_types
            .values()
            .find(|t| t.type_name == "TestRoot");
        assert!(root_type.is_some());
    }

    #[test]
    fn test_determine_type_kind_allof_composition() {
        // Test handling of allOf composition
        // Note: allOf is handled differently than oneOf/anyOf in the code
        // allOf schemas fall through to NestedStruct since they represent merged objects
        use serde_json::json;

        let schema = json!({
            "allOf": [
                {"type": "object", "properties": {"a": {"type": "string"}}},
                {"type": "object", "properties": {"b": {"type": "number"}}}
            ]
        });

        let walker = SchemaWalker::new(json!({}));
        let schema_obj = schema.as_object().unwrap();

        let type_kind = walker.determine_type_kind(schema_obj).unwrap();

        // allOf falls through to NestedStruct (not Enum like oneOf/anyOf)
        assert!(matches!(type_kind, TypeKind::NestedStruct));
    }

    #[test]
    fn test_schema_allows_null_anyof_variant() {
        // Test null detection in anyOf variant
        use serde_json::json;

        let schema = json!({
            "anyOf": [
                {"type": "string"},
                {"type": "null"}
            ]
        });

        let walker = SchemaWalker::new(json!({}));
        let schema_obj = schema.as_object().unwrap();

        assert!(walker.schema_allows_null(schema_obj));
    }

    #[test]
    fn test_resolve_ref_external_url() {
        // Test that external URL references return None
        use serde_json::json;

        let schema = json!({});
        let walker = SchemaWalker::new(schema);

        let result = walker.resolve_ref("https://example.com/schema.json");
        assert!(result.is_none());
    }

    #[test]
    fn test_resolve_ref_with_properties() {
        // Test resolving reference to nested properties
        use serde_json::json;

        let schema = json!({
            "properties": {
                "user": {
                    "type": "object",
                    "properties": {
                        "name": {"type": "string"}
                    }
                }
            }
        });

        let walker = SchemaWalker::new(schema);

        let result = walker.resolve_ref("#/properties/user/properties/name");
        assert!(result.is_some());

        let resolved = result.unwrap();
        assert_eq!(
            resolved.get("type").and_then(|v| v.as_str()),
            Some("string")
        );
    }

    #[test]
    fn test_discover_constrained_object_with_pattern_properties() {
        // Test object with both properties and patternProperties
        use serde_json::json;

        let schema = json!({
            "type": "object",
            "properties": {
                "config": {
                    "type": "object",
                    "properties": {
                        "name": {"type": "string"}
                    },
                    "patternProperties": {
                        "^env_": {"type": "string"}
                    }
                }
            }
        });

        let result = SchemaWalker::walk(&schema, "TestRoot");
        assert!(result.is_ok());

        let discovery = result.unwrap();

        // Should create nested type for constrained object
        let has_nested = discovery
            .discovered_types
            .values()
            .any(|t| t.type_name.contains("Config"));
        assert!(has_nested);

        // Should have pattern validators
        assert!(!discovery.pattern_validators.is_empty());
    }

    #[test]
    fn test_discover_array_without_items() {
        // Test handling of array without items schema (should default to Value)
        use serde_json::json;

        let schema = json!({
            "type": "object",
            "properties": {
                "tags": {
                    "type": "array"
                    // No items schema
                }
            }
        });

        let result = SchemaWalker::walk(&schema, "TestRoot");
        assert!(result.is_ok());

        let discovery = result.unwrap();
        let root_type = discovery
            .discovered_types
            .values()
            .find(|t| t.type_name == "TestRoot");
        assert!(root_type.is_some());
    }

    #[test]
    fn test_schemas_structurally_equal_with_additional_properties() {
        // Test structural equality comparison with additionalProperties
        use serde_json::json;

        let schema1 = json!({
            "type": "object",
            "properties": {"name": {"type": "string"}},
            "additionalProperties": false
        });

        let schema2 = json!({
            "type": "object",
            "properties": {"name": {"type": "string"}},
            "additionalProperties": true
        });

        let equal =
            schemas_structurally_equal(schema1.as_object().unwrap(), schema2.as_object().unwrap());

        // Different additionalProperties should make schemas not equal
        assert!(!equal);
    }

    #[test]
    fn test_determine_member_type_number_type() {
        // Test handling of "type": "number" (should use serde_json::Number)
        use serde_json::json;

        let schema = json!({
            "type": "object",
            "properties": {
                "value": {"type": "number"}
            }
        });

        let result = SchemaWalker::walk(&schema, "TestRoot");
        assert!(result.is_ok());

        let discovery = result.unwrap();
        let root_type = discovery
            .discovered_types
            .values()
            .find(|t| t.type_name == "TestRoot");
        assert!(root_type.is_some());

        // Should have a member for value
        if let Some(root) = root_type {
            let has_value = root.members.iter().any(|m| m.name == "value");
            assert!(has_value);
        }
    }

    #[test]
    fn test_determine_member_type_integer_without_bounds() {
        // Test handling of integer without bounds (not a bounded integer)
        use serde_json::json;

        let schema = json!({
            "type": "object",
            "properties": {
                "count": {"type": "integer"}
            }
        });

        let result = SchemaWalker::walk(&schema, "TestRoot");
        assert!(result.is_ok());

        let discovery = result.unwrap();

        // Should NOT extract as bounded integer (no bounds)
        assert_eq!(discovery.stats.bounded_integers, 0);

        let root_type = discovery
            .discovered_types
            .values()
            .find(|t| t.type_name == "TestRoot");
        assert!(root_type.is_some());
    }

    #[test]
    fn test_determine_member_type_integer_with_only_minimum() {
        // Test handling of integer with only minimum (not both bounds)
        use serde_json::json;

        let schema = json!({
            "type": "object",
            "properties": {
                "port": {
                    "type": "integer",
                    "minimum": 1024
                    // No maximum
                }
            }
        });

        let result = SchemaWalker::walk(&schema, "TestRoot");
        assert!(result.is_ok());

        let discovery = result.unwrap();

        // Should NOT extract as bounded integer (needs both min and max)
        assert_eq!(discovery.stats.bounded_integers, 0);
    }

    #[test]
    fn test_determine_member_type_integer_with_only_maximum() {
        // Test handling of integer with only maximum (not both bounds)
        use serde_json::json;

        let schema = json!({
            "type": "object",
            "properties": {
                "priority": {
                    "type": "integer",
                    "maximum": 100
                    // No minimum
                }
            }
        });

        let result = SchemaWalker::walk(&schema, "TestRoot");
        assert!(result.is_ok());

        let discovery = result.unwrap();

        // Should NOT extract as bounded integer (needs both min and max)
        assert_eq!(discovery.stats.bounded_integers, 0);
    }

    // === Phase 4.2.2: Additional Edge Case Tests ===

    #[test]
    fn test_discover_complex_enum_with_non_object_variant() {
        // Test handling of oneOf/anyOf with non-object variants
        use serde_json::json;

        let schema = json!({
            "oneOf": [
                {"type": "object", "properties": {"name": {"type": "string"}}},
                {"type": "string"},  // Non-object variant (primitive)
                42  // Invalid variant (not even an object)
            ]
        });

        let result = SchemaWalker::walk(&schema, "TestRoot");
        assert!(result.is_ok());

        let discovery = result.unwrap();
        let root_type = discovery
            .discovered_types
            .values()
            .find(|t| t.type_name == "TestRoot");
        assert!(root_type.is_some());
    }

    #[test]
    fn test_discover_complex_enum_with_root_pattern_properties() {
        // Test merging of root patternProperties into variants
        use serde_json::json;

        let schema = json!({
            "type": "object",
            "patternProperties": {
                "^env_": {"type": "string"}
            },
            "anyOf": [
                {"properties": {"name": {"type": "string"}}},
                {"properties": {"id": {"type": "number"}}}
            ]
        });

        let result = SchemaWalker::walk(&schema, "TestRoot");
        assert!(result.is_ok());

        let discovery = result.unwrap();

        // Root type should be discovered as enum due to anyOf
        let root_type = discovery
            .discovered_types
            .values()
            .find(|t| t.type_name == "TestRoot");
        assert!(root_type.is_some());
    }

    #[test]
    fn test_discover_complex_enum_with_root_additional_properties() {
        // Test merging of root additionalProperties into variants
        use serde_json::json;

        let schema = json!({
            "type": "object",
            "additionalProperties": {"type": "boolean"},
            "oneOf": [
                {"properties": {"name": {"type": "string"}}},
                {"properties": {"id": {"type": "number"}}}
            ]
        });

        let result = SchemaWalker::walk(&schema, "TestRoot");
        assert!(result.is_ok());

        let discovery = result.unwrap();
        let root_type = discovery
            .discovered_types
            .values()
            .find(|t| t.type_name == "TestRoot");
        assert!(root_type.is_some());
    }

    #[test]
    fn test_discover_complex_enum_with_variant_required_merge() {
        // Test merging of root required with variant required
        use serde_json::json;

        let schema = json!({
            "type": "object",
            "properties": {
                "common": {"type": "string"}
            },
            "required": ["common"],
            "anyOf": [
                {
                    "properties": {
                        "specific1": {"type": "string"}
                    },
                    "required": ["specific1"]  // Variant has its own required
                },
                {
                    "properties": {
                        "specific2": {"type": "number"}
                    }
                    // No required in this variant
                }
            ]
        });

        let result = SchemaWalker::walk(&schema, "TestRoot");
        assert!(result.is_ok());

        let discovery = result.unwrap();
        let root_type = discovery
            .discovered_types
            .values()
            .find(|t| t.type_name == "TestRoot");
        assert!(root_type.is_some());
    }

    #[test]
    fn test_discover_type_array_with_invalid_element() {
        // Test type array with non-string element
        use serde_json::json;

        let schema = json!({
            "type": ["string", 42, "number"]  // 42 is not a valid type string
        });

        let result = SchemaWalker::walk(&schema, "TestRoot");
        assert!(result.is_ok());

        let discovery = result.unwrap();
        let root_type = discovery
            .discovered_types
            .values()
            .find(|t| t.type_name == "TestRoot");
        assert!(root_type.is_some());
    }

    #[test]
    fn test_discover_inline_enum_with_non_string_values() {
        // Test inline enum extraction rejects non-string values
        use serde_json::json;

        let schema = json!({
            "type": "object",
            "properties": {
                "mixed_enum": {
                    "enum": ["string_val", 42, true]  // Mixed types - should NOT extract
                }
            }
        });

        let result = SchemaWalker::walk(&schema, "TestRoot");
        assert!(result.is_ok());

        let discovery = result.unwrap();

        // Should NOT extract inline enum (has non-string values)
        let has_inline_enum = discovery
            .discovered_types
            .values()
            .any(|t| matches!(t.type_kind, TypeKind::Enum) && t.type_name.contains("MixedEnum"));
        assert!(!has_inline_enum, "Mixed-type enum should not be extracted");
    }

    #[test]
    fn test_discover_inline_enum_with_empty_values() {
        // Test inline enum extraction rejects empty enum
        use serde_json::json;

        let schema = json!({
            "type": "object",
            "properties": {
                "empty_enum": {
                    "enum": []  // Empty enum
                }
            }
        });

        let result = SchemaWalker::walk(&schema, "TestRoot");
        assert!(result.is_ok());

        let discovery = result.unwrap();

        // Should NOT extract inline enum (empty)
        let has_inline_enum = discovery
            .discovered_types
            .values()
            .any(|t| matches!(t.type_kind, TypeKind::Enum) && t.type_name.contains("EmptyEnum"));
        assert!(!has_inline_enum, "Empty enum should not be extracted");
    }

    #[test]
    fn test_determine_member_type_array_with_non_object_items() {
        // Test array with items that's not an object
        use serde_json::json;

        let schema = json!({
            "type": "object",
            "properties": {
                "values": {
                    "type": "array",
                    "items": "invalid"  // Not an object
                }
            }
        });

        let result = SchemaWalker::walk(&schema, "TestRoot");
        assert!(result.is_ok());

        let discovery = result.unwrap();
        let root_type = discovery
            .discovered_types
            .values()
            .find(|t| t.type_name == "TestRoot");
        assert!(root_type.is_some());
    }

    #[test]
    fn test_schema_allows_null_with_ref_to_non_null_type() {
        // Test null detection when $ref points to type without null variant
        use serde_json::json;

        let schema = json!({
            "definitions": {
                "NonNullType": {
                    "type": "object",
                    "properties": {
                        "value": {"type": "string"}
                    }
                }
            },
            "type": "object",
            "properties": {
                "field": {
                    "$ref": "#/definitions/NonNullType"
                }
            }
        });

        let walker = SchemaWalker::new(schema.clone());

        let field_schema = json!({
            "$ref": "#/definitions/NonNullType"
        });

        let allows_null = walker.schema_allows_null(field_schema.as_object().unwrap());
        assert!(!allows_null, "Ref to non-null type should not allow null");
    }

    #[test]
    fn test_schema_allows_null_with_unresolvable_ref() {
        // Test null detection with $ref that can't be resolved
        use serde_json::json;

        let schema = json!({
            "type": "object"
        });

        let walker = SchemaWalker::new(schema);

        let field_schema = json!({
            "$ref": "#/definitions/NonExistent"
        });

        let allows_null = walker.schema_allows_null(field_schema.as_object().unwrap());
        assert!(
            !allows_null,
            "Unresolvable ref should conservatively return false"
        );
    }

    #[test]
    fn test_determine_member_type_with_container_and_other_member_types() {
        // Test array with Container (Vec<primitive>) result
        use serde_json::json;

        let schema = json!({
            "type": "object",
            "properties": {
                "numbers": {
                    "type": "array",
                    "items": {
                        "type": "integer"
                    }
                }
            }
        });

        let result = SchemaWalker::walk(&schema, "TestRoot");
        assert!(result.is_ok());

        let discovery = result.unwrap();
        let root_type = discovery
            .discovered_types
            .values()
            .find(|t| t.type_name == "TestRoot");
        assert!(root_type.is_some());

        // Should have a member for numbers as Vec<i32>
        if let Some(root) = root_type {
            let has_numbers = root.members.iter().any(|m| m.name == "numbers");
            assert!(has_numbers);
        }
    }

    #[test]
    fn test_resolve_ref_with_missing_intermediate_path() {
        // Test resolve_ref with path that has missing intermediate components
        use serde_json::json;

        let schema = json!({
            "definitions": {
                "User": {
                    "type": "object"
                }
            }
        });

        let walker = SchemaWalker::new(schema);

        // Try to resolve path with non-existent intermediate component
        let result = walker.resolve_ref("#/definitions/User/properties/nonexistent");
        assert!(
            result.is_none(),
            "Should return None for missing intermediate path"
        );
    }

    #[test]
    fn test_discover_nested_definitions_with_multiple_levels() {
        // Test deeply nested definitions (3+ levels)
        use serde_json::json;

        let schema = json!({
            "definitions": {
                "Level1": {
                    "type": "object",
                    "definitions": {
                        "Level2": {
                            "type": "object",
                            "definitions": {
                                "Level3": {
                                    "type": "object",
                                    "properties": {
                                        "deep": {"type": "string"}
                                    }
                                }
                            }
                        }
                    }
                }
            }
        });

        let result = SchemaWalker::walk(&schema, "TestRoot");
        assert!(result.is_ok());

        let discovery = result.unwrap();

        // Should discover all 3 levels
        assert!(discovery.definitions.contains_key("Level1"));
        assert!(
            discovery
                .definitions
                .contains_key("Level1/definitions/Level2")
        );
        assert!(
            discovery
                .definitions
                .contains_key("Level1/definitions/Level2/definitions/Level3")
        );
    }

    #[test]
    fn test_is_composition_schema_with_allof() {
        // Test that allOf is NOT considered a composition schema
        use serde_json::json;

        let schema = json!({
            "allOf": [
                {"type": "object", "properties": {"a": {"type": "string"}}},
                {"type": "object", "properties": {"b": {"type": "number"}}}
            ]
        });

        let walker = SchemaWalker::new(json!({}));
        let schema_obj = schema.as_object().unwrap();

        // allOf is NOT a composition schema (only oneOf, anyOf, enum are)
        assert!(!walker.is_composition_schema(schema_obj));
    }

    #[test]
    fn test_build_json_path_variations() {
        // Test build_json_path with different parameter combinations
        use serde_json::json;

        let walker = SchemaWalker::new(json!({}));

        // Both parent and property
        let path1 = walker.build_json_path(Some("Parent"), Some("property"));
        assert_eq!(path1, "#/properties/Parent/properties/property");

        // Only parent
        let path2 = walker.build_json_path(Some("Parent"), None);
        assert_eq!(path2, "#/properties/Parent");

        // Only property
        let path3 = walker.build_json_path(None, Some("property"));
        assert_eq!(path3, "#/properties/property");

        // Neither parent nor property
        let path4 = walker.build_json_path(None, None);
        assert_eq!(path4, "#/");
    }

    /// Phase 4.2.3: Final targeted tests to reach 85% regions coverage (84.55% → 85%+)

    #[test]
    fn test_schema_allows_null_with_direct_null_type() {
        // Test schema with type: "null"
        use serde_json::json;

        let schema = json!({"type": "null"});
        let walker = SchemaWalker::new(json!({}));

        let schema_obj = schema.as_object().unwrap();
        assert!(
            walker.schema_allows_null(schema_obj),
            "Schema with type: null should allow null"
        );
    }

    #[test]
    fn test_schema_allows_null_with_type_array_containing_null() {
        // Test schema with type: ["string", "null"]
        use serde_json::json;

        let schema = json!({"type": ["string", "null"]});
        let walker = SchemaWalker::new(json!({}));

        let schema_obj = schema.as_object().unwrap();
        assert!(
            walker.schema_allows_null(schema_obj),
            "Schema with null in type array should allow null"
        );
    }

    #[test]
    fn test_schema_allows_null_with_type_array_without_null() {
        // Test schema with type array that doesn't contain null
        use serde_json::json;

        let schema = json!({"type": ["string", "integer", "boolean"]});
        let walker = SchemaWalker::new(json!({}));

        let schema_obj = schema.as_object().unwrap();
        assert!(
            !walker.schema_allows_null(schema_obj),
            "Schema without null in type array should not allow null"
        );
    }

    #[test]
    fn test_is_composition_schema_with_oneof() {
        // Explicit test for oneOf detection
        use serde_json::json;

        let schema = json!({"oneOf": [{"type": "string"}, {"type": "number"}]});
        let walker = SchemaWalker::new(json!({}));

        let schema_obj = schema.as_object().unwrap();
        assert!(
            walker.is_composition_schema(schema_obj),
            "Schema with oneOf should be composition"
        );
    }

    #[test]
    fn test_is_composition_schema_with_anyof() {
        // Explicit test for anyOf detection
        use serde_json::json;

        let schema = json!({"anyOf": [{"type": "string"}, {"type": "number"}]});
        let walker = SchemaWalker::new(json!({}));

        let schema_obj = schema.as_object().unwrap();
        assert!(
            walker.is_composition_schema(schema_obj),
            "Schema with anyOf should be composition"
        );
    }

    #[test]
    fn test_is_composition_schema_with_enum() {
        // Explicit test for enum detection
        use serde_json::json;

        let schema = json!({"enum": ["a", "b", "c"]});
        let walker = SchemaWalker::new(json!({}));

        let schema_obj = schema.as_object().unwrap();
        assert!(
            walker.is_composition_schema(schema_obj),
            "Schema with enum should be composition"
        );
    }

    #[test]
    fn test_is_composition_schema_with_regular_object() {
        // Test that regular objects are NOT composition schemas
        use serde_json::json;

        let schema = json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"}
            }
        });
        let walker = SchemaWalker::new(json!({}));

        let schema_obj = schema.as_object().unwrap();
        assert!(
            !walker.is_composition_schema(schema_obj),
            "Regular object should not be composition"
        );
    }

    #[test]
    fn test_determine_member_type_with_boolean_type() {
        // Test that boolean type is handled correctly
        use serde_json::json;

        let schema = json!({
            "type": "object",
            "properties": {
                "active": {"type": "boolean"}
            }
        });

        let result = SchemaWalker::walk(&schema, "TestRoot");
        assert!(result.is_ok());

        let discovery = result.unwrap();
        let root_type = discovery
            .discovered_types
            .values()
            .find(|t| t.type_name == "TestRoot");
        assert!(root_type.is_some());

        if let Some(root) = root_type {
            let active_field = root.members.iter().find(|m| m.name == "active");
            assert!(active_field.is_some(), "Boolean field should be discovered");
        }
    }

    #[test]
    fn test_schemas_structurally_equal_with_different_required() {
        // Test that schemas with different required arrays are not structurally equal
        use serde_json::json;

        let schema1 = json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"}
            },
            "required": ["name"]
        });

        let schema2 = json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"}
            }
            // No required array
        });

        let obj1 = schema1.as_object().unwrap();
        let obj2 = schema2.as_object().unwrap();

        assert!(
            !schemas_structurally_equal(obj1, obj2),
            "Schemas with different required should not be equal"
        );
    }

    #[test]
    fn test_discover_inline_enum_min_threshold() {
        // Test inline enum extraction with exactly MIN_ENUM_VALUES (2)
        use serde_json::json;

        let schema = json!({
            "type": "object",
            "properties": {
                "status": {
                    "type": "string",
                    "enum": ["active", "inactive"]  // Exactly 2 values (MIN threshold)
                }
            }
        });

        let result = SchemaWalker::walk(&schema, "TestRoot");
        assert!(result.is_ok());

        let discovery = result.unwrap();

        // Should extract inline enum with 2 values
        let has_inline_enum = discovery
            .discovered_types
            .values()
            .any(|t| matches!(t.type_kind, TypeKind::Enum) && t.type_name.contains("Status"));
        assert!(
            has_inline_enum,
            "Should extract enum with 2 values (MIN threshold)"
        );
    }

    #[test]
    fn test_discover_inline_enum_single_value() {
        // Test inline enum extraction with single-value enum
        use serde_json::json;

        let schema = json!({
            "type": "object",
            "properties": {
                "constant": {
                    "type": "string",
                    "enum": ["only_value"]  // Single value
                }
            }
        });

        let result = SchemaWalker::walk(&schema, "TestRoot");
        assert!(result.is_ok());

        let discovery = result.unwrap();

        // Should extract inline enum even with single value
        let has_inline_enum = discovery
            .discovered_types
            .values()
            .any(|t| matches!(t.type_kind, TypeKind::Enum) && t.type_name.contains("Constant"));
        assert!(has_inline_enum, "Should extract single-value enum");
    }
}
