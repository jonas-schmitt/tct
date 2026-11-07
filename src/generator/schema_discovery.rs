//! Schema Discovery System for Multi-Pass Compilation
//!
//! This module provides pure discovery data structures that capture schema structure
//! without generating any code. Each pass has its own specialized data store:
//!
//! - SchemaDiscovery: Pure schema analysis (DiscoveryPass)
//! - TypeAnalysis: Dependency graphs and trait analysis (AnalysisPass)
//! - CodeGeneration: Final code generation (GenerationPass)

use rustc_hash::FxHashMap;
use serde_json::{Map, Value};
use std::collections::HashSet;

/// Pure schema discovery results - NO code generation
#[derive(Debug, Clone)]
pub struct SchemaDiscovery {
    /// All discovered type locations and their schema information
    pub discovered_types: FxHashMap<String, TypeDiscovery>,
    /// Root schema information
    pub root_info: Option<RootSchemaInfo>,
    /// Definitions section analysis
    pub definitions: FxHashMap<String, DefinitionInfo>,
    /// Statistics about discovery
    pub stats: DiscoveryStats,
    /// JSON pointer to type name mapping
    pub ref_mappings: FxHashMap<String, String>,
    /// Pattern validator information
    pub pattern_validators: FxHashMap<String, PatternValidatorInfo>,
}

/// Discriminator information for enum types
///
/// For oneOf/anyOf enums where each variant has a unique discriminator field value
/// (like Kubernetes `kind` field), this stores the mapping from discriminator values
/// to variant indices. This enables partial encoding for enum roots.
///
/// Supports both single-field discriminators (e.g., "kind" alone) and compound
/// discriminators (e.g., "kind" + "apiVersion" for Kubernetes).
#[derive(Debug, Clone)]
pub struct DiscriminatorInfo {
    /// Names of the discriminator fields (e.g., ["kind"] or ["apiVersion", "kind"]).
    /// Compound discriminators list their fields in alphabetical order.
    pub field_names: Vec<String>,
    /// Mapping from discriminator value(s) to variant index
    /// For single field: value_to_variant = [("Pod", 32), ("Deployment", 6), ...]
    /// For compound: value_to_variant = [("v1|Pod", 32), ("apps/v1|Deployment", 6), ...]
    /// Values are joined with "|" in `field_names` order for compound discriminators
    pub value_to_variant: Vec<(String, usize)>,
}

/// Information about a discovered type (pure schema analysis)
#[derive(Debug, Clone)]
pub struct TypeDiscovery {
    /// The type name we'll generate
    pub type_name: String,
    /// What kind of type this will be
    pub type_kind: TypeKind,
    /// Where this type comes from in the schema
    pub schema_location: SchemaLocation,
    /// The actual schema object for this type
    pub schema_object: Map<String, Value>,
    /// Properties/variants this type will have
    pub members: Vec<MemberInfo>,
    /// Types this type directly references
    pub direct_references: HashSet<String>,
    /// Whether this type needs Box wrapping (detected later in analysis pass)
    pub needs_boxing: bool,
    /// Discriminator info for oneOf/anyOf enums (enables partial encoding)
    /// Only populated when a common discriminator field with unique values is detected
    pub discriminator_info: Option<DiscriminatorInfo>,
}

/// Root schema analysis
#[derive(Debug, Clone)]
pub struct RootSchemaInfo {
    /// Root type name
    pub type_name: String,
    /// Root schema object
    pub schema: Map<String, Value>,
    /// Whether root is a composition (oneOf/anyOf at top level)
    pub is_composition: bool,
}

/// Definition analysis
#[derive(Debug, Clone)]
pub struct DefinitionInfo {
    /// Definition name (from definitions section)
    pub name: String,
    /// Definition schema object
    pub schema: Map<String, Value>,
    /// Generated type name for this definition
    pub type_name: String,
}

/// What kind of Rust type will be generated
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeKind {
    /// Object -> struct
    Struct,
    /// oneOf/anyOf/enum -> enum
    Enum,
    /// Inline object -> nested struct
    NestedStruct,
    /// Array with complex items -> `Vec<ComplexType>`
    ArrayOfComplex,
    /// Reference to definition -> reference to generated type
    Reference {
        /// Name of the target type being referenced
        target: String,
    },
    /// Primitive type in definition -> newtype wrapper (e.g., pub struct SemVer(pub String))
    PrimitiveWrapper {
        /// The primitive Rust type (String, i64, f64, bool)
        primitive_type: String,
    },
    /// Minimal object definition -> generic object wrapper (e.g., pub struct Config(pub serde_json::Value))
    MinimalObjectWrapper,
}

/// Where a type comes from in the schema
#[derive(Debug, Clone)]
pub struct SchemaLocation {
    /// JSON pointer path (e.g., "#/properties/config/properties/enabled")
    pub json_path: String,
    /// Human readable description
    pub description: String,
    /// Parent type that contains this type
    pub parent_type: Option<String>,
    /// Property/field name within parent
    pub property_name: Option<String>,
}

/// Member information (property or enum variant)
#[derive(Debug, Clone)]
pub struct MemberInfo {
    /// Member name (property name or variant name)
    pub name: String,
    /// Member type (what Rust type this will be)
    pub member_type: MemberType,
    /// Whether this member is required
    pub required: bool,
    /// Whether this field explicitly allows null (for optional fields)
    /// When true and !required, use `OptionalField<T>` instead of `Option<T>`
    pub is_nullable: bool,
    /// Schema object for this member
    pub schema: Map<String, Value>,
}

impl MemberInfo {
    /// Whether this member uses the generated OptionalField<T> representation.
    ///
    /// OptionalField<T> is a three-state representation for absent/null/present
    /// values. It is required for nullable optional fields and for
    /// serde_json::Value-backed fields where null and absent must stay distinct.
    pub(crate) fn uses_optional_field_encoding(&self) -> bool {
        self.is_nullable || self.member_type.contains_json_value()
    }

    /// Whether this optional member uses a two-state Option<T> presence flag.
    pub(crate) fn uses_option_presence_flag(&self) -> bool {
        !self.required && !self.uses_optional_field_encoding()
    }
}

/// Type of a struct property or enum variant
#[derive(Debug, Clone, PartialEq)]
pub enum MemberType {
    /// Primitive Rust type
    Primitive(String), // "String", "i32", "bool", etc.
    /// Reference to another discovered type
    TypeReference(String),
    /// Container of primitives
    Container {
        /// Container type (Vec, Option, etc.)
        container: String,
        /// Inner primitive type
        inner: String,
    }, // Vec<String>, Option<i32>
    /// Container of complex types
    ComplexContainer {
        /// Container type (Vec, Option, etc.)
        container: String,
        /// Inner complex type name
        inner: String,
    }, // Vec<MyType>, Option<MyStruct>
    /// HashMap for additionalProperties
    HashMap {
        /// Key type (usually String)
        key: String,
        /// Value type
        value: String,
    }, // std::collections::FxHashMap<String, ValueType>
    /// Pattern properties with validators
    PatternProperties {
        /// List of pattern validator type names
        validators: Vec<String>,
    }, // Pattern validator types
}

impl MemberType {
    /// Whether this metadata type directly contains serde_json::Value.
    ///
    /// This intentionally inspects structured discovery metadata rather than
    /// generated Rust token streams.
    pub(crate) fn contains_json_value(&self) -> bool {
        match self {
            Self::Primitive(type_name) => rust_type_mentions_json_value(type_name),
            Self::Container { inner, .. } => rust_type_mentions_json_value(inner),
            Self::ComplexContainer { inner, .. } => rust_type_mentions_json_value(inner),
            Self::HashMap { value, .. } => rust_type_mentions_json_value(value),
            Self::TypeReference(_) | Self::PatternProperties { .. } => false,
        }
    }
}

fn rust_type_mentions_json_value(type_name: &str) -> bool {
    type_name.replace(' ', "").contains("serde_json::Value")
}

/// Information about a pattern validator
#[derive(Debug, Clone)]
pub struct PatternValidatorInfo {
    /// Name of the validator type
    pub validator_name: String,
    /// Regex pattern string
    pub pattern: String,
    /// Type of values for this pattern
    pub value_type: String,
    /// Original schema for the pattern value
    pub value_schema: Map<String, Value>,
}

/// Discovery statistics (no code generation stats)
#[derive(Debug, Clone, Default)]
pub struct DiscoveryStats {
    /// Total types that will need to be generated
    pub total_types: usize,
    /// Types by kind
    pub structs: usize,
    /// Number of enum types discovered
    pub enums: usize,
    /// Number of nested struct types discovered
    pub nested_structs: usize,
    /// Number of type references discovered
    pub references: usize,
    /// Discovery sources
    pub from_definitions: usize,
    /// Types discovered from object properties
    pub from_properties: usize,
    /// Types discovered from oneOf/anyOf/allOf compositions
    pub from_compositions: usize,
    /// Complexity indicators
    pub max_nesting_depth: usize,
    /// Total number of properties across all types
    pub total_properties: usize,
    /// Total number of enum variants across all enums
    pub total_enum_variants: usize,
    /// Number of bounded integers discovered
    pub bounded_integers: usize,
    /// Number of format-constrained strings discovered
    pub format_constrained_strings: usize,
}

impl Default for SchemaDiscovery {
    fn default() -> Self {
        Self::new()
    }
}

impl SchemaDiscovery {
    /// Creates a new empty schema discovery
    pub fn new() -> Self {
        Self {
            discovered_types: FxHashMap::default(),
            root_info: None,
            definitions: FxHashMap::default(),
            stats: DiscoveryStats::default(),
            ref_mappings: FxHashMap::default(),
            pattern_validators: FxHashMap::default(),
        }
    }

    /// Check if a type has been discovered
    pub fn has_type(&self, type_name: &str) -> bool {
        self.discovered_types.contains_key(type_name)
    }

    /// Register a discovered type
    pub fn register_type(&mut self, discovery: TypeDiscovery) {
        let type_name = discovery.type_name.clone();

        // Update statistics
        match discovery.type_kind {
            TypeKind::Struct => self.stats.structs += 1,
            TypeKind::Enum => self.stats.enums += 1,
            TypeKind::NestedStruct => self.stats.nested_structs += 1,
            TypeKind::Reference { .. } => self.stats.references += 1,
            _ => {}
        }

        self.stats.total_types += 1;
        self.stats.total_properties += discovery.members.len();

        self.discovered_types.insert(type_name, discovery);
    }

    /// Get all types that need code generation (excludes references)
    pub fn types_needing_generation(&self) -> Vec<&TypeDiscovery> {
        self.discovered_types
            .values()
            .filter(|t| !matches!(t.type_kind, TypeKind::Reference { .. }))
            .collect()
    }

    /// Get type discovery by name
    pub fn get_type(&self, type_name: &str) -> Option<&TypeDiscovery> {
        self.discovered_types.get(type_name)
    }
}

impl TypeDiscovery {
    /// Create a new type discovery
    pub fn new(
        type_name: String,
        type_kind: TypeKind,
        schema_location: SchemaLocation,
        schema_object: Map<String, Value>,
    ) -> Self {
        Self {
            type_name,
            type_kind,
            schema_location,
            schema_object,
            members: Vec::new(),
            direct_references: HashSet::new(),
            needs_boxing: false,
            discriminator_info: None,
        }
    }

    /// Add a member (property or variant) to this type
    pub fn add_member(&mut self, member: MemberInfo) {
        self.members.push(member);
    }

    /// Add a direct reference to another type
    pub fn add_reference(&mut self, type_name: String) {
        self.direct_references.insert(type_name);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ============================================================================
    // Batch 1 - SchemaDiscovery Tests (8 tests)
    // ============================================================================

    #[test]
    fn test_schema_discovery_new() {
        let discovery = SchemaDiscovery::new();
        assert!(discovery.discovered_types.is_empty());
        assert!(discovery.root_info.is_none());
        assert!(discovery.definitions.is_empty());
        assert_eq!(discovery.stats.total_types, 0);
        assert!(discovery.ref_mappings.is_empty());
        assert!(discovery.pattern_validators.is_empty());
    }

    #[test]
    fn test_schema_discovery_default() {
        let discovery1 = SchemaDiscovery::default();
        let discovery2 = SchemaDiscovery::new();
        assert_eq!(
            discovery1.discovered_types.len(),
            discovery2.discovered_types.len()
        );
        assert_eq!(discovery1.stats.total_types, discovery2.stats.total_types);
    }

    #[test]
    fn test_schema_discovery_has_type() {
        let mut discovery = SchemaDiscovery::new();
        assert!(!discovery.has_type("User"));

        let type_disc = TypeDiscovery::new(
            "User".to_string(),
            TypeKind::Struct,
            SchemaLocation {
                json_path: "#/properties/user".to_string(),
                description: "User object".to_string(),
                parent_type: None,
                property_name: Some("user".to_string()),
            },
            serde_json::Map::new(),
        );
        discovery.register_type(type_disc);

        assert!(discovery.has_type("User"));
        assert!(!discovery.has_type("NonExistent"));
    }

    #[test]
    fn test_schema_discovery_register_type_struct() {
        let mut discovery = SchemaDiscovery::new();

        let type_disc = TypeDiscovery::new(
            "User".to_string(),
            TypeKind::Struct,
            SchemaLocation {
                json_path: "#/properties/user".to_string(),
                description: "User struct".to_string(),
                parent_type: None,
                property_name: None,
            },
            serde_json::Map::new(),
        );

        discovery.register_type(type_disc);

        assert_eq!(discovery.stats.total_types, 1);
        assert_eq!(discovery.stats.structs, 1);
        assert_eq!(discovery.stats.enums, 0);
        assert!(discovery.has_type("User"));
    }

    #[test]
    fn test_schema_discovery_register_type_enum() {
        let mut discovery = SchemaDiscovery::new();

        let type_disc = TypeDiscovery::new(
            "Status".to_string(),
            TypeKind::Enum,
            SchemaLocation {
                json_path: "#/properties/status".to_string(),
                description: "Status enum".to_string(),
                parent_type: None,
                property_name: None,
            },
            serde_json::Map::new(),
        );

        discovery.register_type(type_disc);

        assert_eq!(discovery.stats.total_types, 1);
        assert_eq!(discovery.stats.structs, 0);
        assert_eq!(discovery.stats.enums, 1);
    }

    #[test]
    fn test_schema_discovery_register_type_nested_struct() {
        let mut discovery = SchemaDiscovery::new();

        let type_disc = TypeDiscovery::new(
            "Config".to_string(),
            TypeKind::NestedStruct,
            SchemaLocation {
                json_path: "#/properties/config".to_string(),
                description: "Nested config".to_string(),
                parent_type: Some("Root".to_string()),
                property_name: Some("config".to_string()),
            },
            serde_json::Map::new(),
        );

        discovery.register_type(type_disc);

        assert_eq!(discovery.stats.nested_structs, 1);
        assert_eq!(discovery.stats.total_types, 1);
    }

    #[test]
    fn test_schema_discovery_register_type_reference() {
        let mut discovery = SchemaDiscovery::new();

        let type_disc = TypeDiscovery::new(
            "UserRef".to_string(),
            TypeKind::Reference {
                target: "User".to_string(),
            },
            SchemaLocation {
                json_path: "#/$ref".to_string(),
                description: "Reference to User".to_string(),
                parent_type: None,
                property_name: None,
            },
            serde_json::Map::new(),
        );

        discovery.register_type(type_disc);

        assert_eq!(discovery.stats.references, 1);
        assert_eq!(discovery.stats.total_types, 1);
    }

    #[test]
    fn test_schema_discovery_types_needing_generation() {
        let mut discovery = SchemaDiscovery::new();

        // Add struct (needs generation)
        discovery.register_type(TypeDiscovery::new(
            "User".to_string(),
            TypeKind::Struct,
            SchemaLocation {
                json_path: "#/properties/user".to_string(),
                description: "User".to_string(),
                parent_type: None,
                property_name: None,
            },
            serde_json::Map::new(),
        ));

        // Add reference (doesn't need generation)
        discovery.register_type(TypeDiscovery::new(
            "UserRef".to_string(),
            TypeKind::Reference {
                target: "User".to_string(),
            },
            SchemaLocation {
                json_path: "#/$ref".to_string(),
                description: "Ref".to_string(),
                parent_type: None,
                property_name: None,
            },
            serde_json::Map::new(),
        ));

        // Add enum (needs generation)
        discovery.register_type(TypeDiscovery::new(
            "Status".to_string(),
            TypeKind::Enum,
            SchemaLocation {
                json_path: "#/properties/status".to_string(),
                description: "Status".to_string(),
                parent_type: None,
                property_name: None,
            },
            serde_json::Map::new(),
        ));

        let types = discovery.types_needing_generation();
        assert_eq!(types.len(), 2); // User and Status, not UserRef
    }

    #[test]
    fn test_schema_discovery_get_type() {
        let mut discovery = SchemaDiscovery::new();

        let type_disc = TypeDiscovery::new(
            "User".to_string(),
            TypeKind::Struct,
            SchemaLocation {
                json_path: "#/properties/user".to_string(),
                description: "User".to_string(),
                parent_type: None,
                property_name: None,
            },
            serde_json::Map::new(),
        );

        discovery.register_type(type_disc);

        assert!(discovery.get_type("User").is_some());
        assert!(discovery.get_type("NonExistent").is_none());

        let retrieved = discovery.get_type("User").unwrap();
        assert_eq!(retrieved.type_name, "User");
    }

    // ============================================================================
    // Batch 2 - TypeDiscovery Tests (7 tests)
    // ============================================================================

    #[test]
    fn test_type_discovery_new() {
        let schema = serde_json::Map::new();
        let location = SchemaLocation {
            json_path: "#/properties/test".to_string(),
            description: "Test type".to_string(),
            parent_type: None,
            property_name: Some("test".to_string()),
        };

        let discovery = TypeDiscovery::new(
            "TestType".to_string(),
            TypeKind::Struct,
            location.clone(),
            schema.clone(),
        );

        assert_eq!(discovery.type_name, "TestType");
        assert_eq!(discovery.type_kind, TypeKind::Struct);
        assert_eq!(discovery.schema_location.json_path, "#/properties/test");
        assert!(discovery.members.is_empty());
        assert!(discovery.direct_references.is_empty());
        assert!(!discovery.needs_boxing);
    }

    #[test]
    fn test_type_discovery_add_member() {
        let mut discovery = TypeDiscovery::new(
            "User".to_string(),
            TypeKind::Struct,
            SchemaLocation {
                json_path: "#/properties/user".to_string(),
                description: "User".to_string(),
                parent_type: None,
                property_name: None,
            },
            serde_json::Map::new(),
        );

        let member = MemberInfo {
            name: "name".to_string(),
            member_type: MemberType::Primitive("String".to_string()),
            required: true,
            is_nullable: false,
            schema: serde_json::Map::new(),
        };

        discovery.add_member(member);
        assert_eq!(discovery.members.len(), 1);
        assert_eq!(discovery.members[0].name, "name");
    }

    #[test]
    fn test_type_discovery_add_multiple_members() {
        let mut discovery = TypeDiscovery::new(
            "User".to_string(),
            TypeKind::Struct,
            SchemaLocation {
                json_path: "#".to_string(),
                description: "Root".to_string(),
                parent_type: None,
                property_name: None,
            },
            serde_json::Map::new(),
        );

        discovery.add_member(MemberInfo {
            name: "name".to_string(),
            member_type: MemberType::Primitive("String".to_string()),
            required: true,
            is_nullable: false,
            schema: serde_json::Map::new(),
        });

        discovery.add_member(MemberInfo {
            name: "age".to_string(),
            member_type: MemberType::Primitive("i32".to_string()),
            required: false,
            is_nullable: true,
            schema: serde_json::Map::new(),
        });

        assert_eq!(discovery.members.len(), 2);
    }

    #[test]
    fn test_type_discovery_add_reference() {
        let mut discovery = TypeDiscovery::new(
            "Product".to_string(),
            TypeKind::Struct,
            SchemaLocation {
                json_path: "#".to_string(),
                description: "Product".to_string(),
                parent_type: None,
                property_name: None,
            },
            serde_json::Map::new(),
        );

        discovery.add_reference("User".to_string());
        discovery.add_reference("Category".to_string());

        assert_eq!(discovery.direct_references.len(), 2);
        assert!(discovery.direct_references.contains("User"));
        assert!(discovery.direct_references.contains("Category"));
    }

    #[test]
    fn test_type_discovery_add_duplicate_reference() {
        let mut discovery = TypeDiscovery::new(
            "Test".to_string(),
            TypeKind::Struct,
            SchemaLocation {
                json_path: "#".to_string(),
                description: "Test".to_string(),
                parent_type: None,
                property_name: None,
            },
            serde_json::Map::new(),
        );

        discovery.add_reference("User".to_string());
        discovery.add_reference("User".to_string()); // Duplicate

        assert_eq!(discovery.direct_references.len(), 1); // Set deduplicates
    }

    #[test]
    fn test_type_discovery_all_type_kinds() {
        // Test each TypeKind variant
        let kinds = vec![
            TypeKind::Struct,
            TypeKind::Enum,
            TypeKind::NestedStruct,
            TypeKind::ArrayOfComplex,
            TypeKind::Reference {
                target: "SomeType".to_string(),
            },
            TypeKind::PrimitiveWrapper {
                primitive_type: "String".to_string(),
            },
            TypeKind::MinimalObjectWrapper,
        ];

        for kind in kinds {
            let discovery = TypeDiscovery::new(
                "Test".to_string(),
                kind.clone(),
                SchemaLocation {
                    json_path: "#".to_string(),
                    description: "Test".to_string(),
                    parent_type: None,
                    property_name: None,
                },
                serde_json::Map::new(),
            );
            assert_eq!(discovery.type_kind, kind);
        }
    }

    #[test]
    fn test_type_discovery_needs_boxing() {
        let mut discovery = TypeDiscovery::new(
            "RecursiveType".to_string(),
            TypeKind::Struct,
            SchemaLocation {
                json_path: "#".to_string(),
                description: "Recursive".to_string(),
                parent_type: None,
                property_name: None,
            },
            serde_json::Map::new(),
        );

        assert!(!discovery.needs_boxing);
        discovery.needs_boxing = true;
        assert!(discovery.needs_boxing);
    }

    // ============================================================================
    // Batch 3 - DiscoveryStats Tests (3 tests)
    // ============================================================================

    #[test]
    fn test_discovery_stats_default() {
        let stats = DiscoveryStats::default();
        assert_eq!(stats.total_types, 0);
        assert_eq!(stats.structs, 0);
        assert_eq!(stats.enums, 0);
        assert_eq!(stats.nested_structs, 0);
        assert_eq!(stats.references, 0);
        assert_eq!(stats.from_definitions, 0);
        assert_eq!(stats.from_properties, 0);
        assert_eq!(stats.from_compositions, 0);
        assert_eq!(stats.max_nesting_depth, 0);
        assert_eq!(stats.total_properties, 0);
        assert_eq!(stats.total_enum_variants, 0);
        assert_eq!(stats.bounded_integers, 0);
    }

    #[test]
    fn test_discovery_stats_accumulation() {
        let mut discovery = SchemaDiscovery::new();

        // Register multiple types
        discovery.register_type(TypeDiscovery::new(
            "User".to_string(),
            TypeKind::Struct,
            SchemaLocation {
                json_path: "#".to_string(),
                description: "User".to_string(),
                parent_type: None,
                property_name: None,
            },
            serde_json::Map::new(),
        ));

        discovery.register_type(TypeDiscovery::new(
            "Status".to_string(),
            TypeKind::Enum,
            SchemaLocation {
                json_path: "#".to_string(),
                description: "Status".to_string(),
                parent_type: None,
                property_name: None,
            },
            serde_json::Map::new(),
        ));

        assert_eq!(discovery.stats.total_types, 2);
        assert_eq!(discovery.stats.structs, 1);
        assert_eq!(discovery.stats.enums, 1);
    }

    #[test]
    fn test_discovery_stats_with_members() {
        let mut discovery = SchemaDiscovery::new();
        let mut type_disc = TypeDiscovery::new(
            "User".to_string(),
            TypeKind::Struct,
            SchemaLocation {
                json_path: "#".to_string(),
                description: "User".to_string(),
                parent_type: None,
                property_name: None,
            },
            serde_json::Map::new(),
        );

        // Add 3 members
        for i in 0..3 {
            type_disc.add_member(MemberInfo {
                name: format!("field{}", i),
                member_type: MemberType::Primitive("String".to_string()),
                required: true,
                is_nullable: false,
                schema: serde_json::Map::new(),
            });
        }

        discovery.register_type(type_disc);

        assert_eq!(discovery.stats.total_properties, 3);
    }

    // ============================================================================
    // Batch 4 - MemberType Tests
    // ============================================================================

    #[test]
    fn test_member_type_primitive() {
        let member_type = MemberType::Primitive("String".to_string());
        match member_type {
            MemberType::Primitive(ref t) => assert_eq!(t, "String"),
            _ => panic!("Expected Primitive"),
        }
    }

    #[test]
    fn test_member_type_type_reference() {
        let member_type = MemberType::TypeReference("User".to_string());
        match member_type {
            MemberType::TypeReference(ref t) => assert_eq!(t, "User"),
            _ => panic!("Expected TypeReference"),
        }
    }

    #[test]
    fn test_member_type_container() {
        let member_type = MemberType::Container {
            container: "Vec".to_string(),
            inner: "String".to_string(),
        };
        match member_type {
            MemberType::Container {
                ref container,
                ref inner,
            } => {
                assert_eq!(container, "Vec");
                assert_eq!(inner, "String");
            }
            _ => panic!("Expected Container"),
        }
    }

    #[test]
    fn test_member_type_complex_container() {
        let member_type = MemberType::ComplexContainer {
            container: "Vec".to_string(),
            inner: "User".to_string(),
        };
        match member_type {
            MemberType::ComplexContainer {
                ref container,
                ref inner,
            } => {
                assert_eq!(container, "Vec");
                assert_eq!(inner, "User");
            }
            _ => panic!("Expected ComplexContainer"),
        }
    }

    #[test]
    fn test_member_type_hashmap() {
        let member_type = MemberType::HashMap {
            key: "String".to_string(),
            value: "User".to_string(),
        };
        match member_type {
            MemberType::HashMap { ref key, ref value } => {
                assert_eq!(key, "String");
                assert_eq!(value, "User");
            }
            _ => panic!("Expected HashMap"),
        }
    }

    #[test]
    fn test_member_type_contains_json_value_metadata() {
        assert!(MemberType::Primitive("serde_json::Value".to_string()).contains_json_value());
        assert!(
            MemberType::Container {
                container: "Vec".to_string(),
                inner: "serde_json :: Value".to_string(),
            }
            .contains_json_value()
        );
        assert!(
            MemberType::HashMap {
                key: "String".to_string(),
                value: "Vec<serde_json::Value>".to_string(),
            }
            .contains_json_value()
        );
        assert!(!MemberType::TypeReference("JsonValueWrapper".to_string()).contains_json_value());
    }

    #[test]
    fn test_member_info_optional_field_encoding_metadata() {
        let optional_string = MemberInfo {
            name: "name".to_string(),
            member_type: MemberType::Primitive("String".to_string()),
            required: false,
            is_nullable: false,
            schema: serde_json::Map::new(),
        };
        assert!(!optional_string.uses_optional_field_encoding());
        assert!(optional_string.uses_option_presence_flag());

        let optional_value = MemberInfo {
            name: "payload".to_string(),
            member_type: MemberType::Primitive("serde_json::Value".to_string()),
            required: false,
            is_nullable: false,
            schema: serde_json::Map::new(),
        };
        assert!(optional_value.uses_optional_field_encoding());
        assert!(!optional_value.uses_option_presence_flag());

        let required_string = MemberInfo {
            required: true,
            ..optional_string
        };
        assert!(!required_string.uses_option_presence_flag());
    }

    #[test]
    fn test_member_type_pattern_properties() {
        let member_type = MemberType::PatternProperties {
            validators: vec![
                "PatternValidator1".to_string(),
                "PatternValidator2".to_string(),
            ],
        };
        match member_type {
            MemberType::PatternProperties { ref validators } => {
                assert_eq!(validators.len(), 2);
                assert_eq!(validators[0], "PatternValidator1");
            }
            _ => panic!("Expected PatternProperties"),
        }
    }
}
