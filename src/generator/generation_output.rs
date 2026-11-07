//! Generation output data structures for multi-pass compilation.
//!
//! This module owns the data model produced by the pure generation pass after
//! discovery, analysis, and optimization have completed:
//!
//! SchemaDiscovery → TypeAnalysis → EncodingStrategy → **CodeGeneration**
//!
//! The `CodeGeneration` output contains:
//! - Generated TokenStreams for each type
//! - Final definitions structure
//! - Root struct generation
//! - Generation statistics

use crate::grammar::Definitions;
use proc_macro2::TokenStream;
use rustc_hash::FxHashMap;

/// Pure code generation results - ONLY code generation
#[derive(Clone)]
pub struct CodeGeneration {
    /// Generated TokenStreams for each type
    pub type_definitions: FxHashMap<String, TypeCodeGeneration>,
    /// Final definitions structure for Grammar
    pub definitions: Definitions,
    /// Root struct generation (if schema has root type)
    pub root_generation: Option<RootCodeGeneration>,
    /// Generation statistics
    pub stats: GenerationStats,
}

/// Generated code for a specific type
#[derive(Clone)]
pub struct TypeCodeGeneration {
    /// The type name
    pub type_name: String,
    /// Generated type definition (struct/enum declaration)
    pub definition_tokens: TokenStream,
    /// Generated implementation code (JsonTokenEncoder impl, etc.)
    pub implementation_tokens: TokenStream,
    /// Types this generated code depends on
    pub dependencies: Vec<String>,
    /// Whether this type was generated with Box wrapping
    pub uses_boxing: bool,
    /// Traits derived for this type
    pub derived_traits: Vec<String>,
    /// Explicit kind of generated item for routing into final definitions
    pub generated_item_kind: GeneratedItemKind,
    /// Whether this type has a generated JsonTokenEncoder implementation
    pub has_token_encoder_impl: bool,
}

/// Explicit generated item category.
///
/// This is code-generation metadata, not schema metadata: several schema kinds
/// are emitted as Rust structs, while simple references are emitted as type
/// aliases.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GeneratedItemKind {
    /// Generated Rust struct or newtype wrapper.
    Struct,
    /// Generated Rust enum.
    Enum,
    /// Generated Rust type alias.
    TypeAlias,
    /// Kind has not been assigned yet.
    #[default]
    Unknown,
}

impl GeneratedItemKind {
    fn label(self) -> &'static str {
        match self {
            Self::Struct => "struct",
            Self::Enum => "enum",
            Self::TypeAlias => "type alias",
            Self::Unknown => "unknown",
        }
    }
}

/// Root code generation (for schemas with root types)
#[derive(Clone)]
pub struct RootCodeGeneration {
    /// Root type name
    pub type_name: String,
    /// Root struct definition
    pub definition_tokens: TokenStream,
    /// Root implementation code
    pub implementation_tokens: TokenStream,
    /// Whether root uses composition (oneOf/anyOf at root level)
    pub is_composition: bool,
}

/// Code generation statistics
#[derive(Debug, Clone, Default)]
pub struct GenerationStats {
    /// Total types generated
    pub total_types: usize,
    /// Generated types by category
    pub structs_generated: usize,
    /// Number of enum types generated
    pub enums_generated: usize,
    /// Number of type aliases generated
    pub type_aliases_generated: usize,
    /// Code size metrics
    pub total_tokens: usize,
    /// Estimated number of lines of code generated
    pub total_lines_estimated: usize,
    /// Trait derivations
    pub eq_derivations: usize,
    /// Number of types with Ord derivation
    pub ord_derivations: usize,
    /// Number of types with Hash derivation
    pub hash_derivations: usize,
    /// Boxing usage
    pub types_using_boxing: usize,
    /// Implementation features
    pub token_encoder_impls: usize,
    /// Number of custom serde/decode implementations generated
    pub custom_implementations: usize,
    /// Final base vocabulary size before BPE and PAD.
    pub base_vocab_size: u32,
}

impl Default for CodeGeneration {
    fn default() -> Self {
        Self::new()
    }
}

impl CodeGeneration {
    /// Create a new code generation
    pub fn new() -> Self {
        Self {
            type_definitions: FxHashMap::default(),
            definitions: Definitions {
                structs: FxHashMap::default(),
                enums: Vec::new(),
                constants: Vec::new(),
                type_aliases: Vec::new(),
            },
            root_generation: None,
            stats: GenerationStats::default(),
        }
    }

    /// Add generated code for a type
    pub fn add_type_generation(&mut self, generation: TypeCodeGeneration) {
        let type_name = generation.type_name.clone();

        // Update statistics
        self.stats.total_types += 1;
        self.stats.total_tokens += self.estimate_tokens(&generation.definition_tokens)
            + self.estimate_tokens(&generation.implementation_tokens);

        if generation.uses_boxing {
            self.stats.types_using_boxing += 1;
        }

        // Count trait derivations
        for trait_name in &generation.derived_traits {
            match trait_name.as_str() {
                "Eq" => self.stats.eq_derivations += 1,
                "Ord" => self.stats.ord_derivations += 1,
                "Hash" => self.stats.hash_derivations += 1,
                _ => {}
            }
        }

        match generation.generated_item_kind {
            GeneratedItemKind::Struct => self.stats.structs_generated += 1,
            GeneratedItemKind::Enum => self.stats.enums_generated += 1,
            GeneratedItemKind::TypeAlias => self.stats.type_aliases_generated += 1,
            GeneratedItemKind::Unknown => {}
        }

        if generation.has_token_encoder_impl {
            self.stats.token_encoder_impls += 1;
        }

        self.type_definitions.insert(type_name, generation);
    }

    /// Set root generation
    pub fn set_root_generation(&mut self, root: RootCodeGeneration) {
        self.root_generation = Some(root);
    }

    /// Get generated code for a type
    pub fn get_type_generation(&self, type_name: &str) -> Option<&TypeCodeGeneration> {
        self.type_definitions.get(type_name)
    }

    /// Get all type names that were generated
    pub fn get_generated_type_names(&self) -> Vec<&String> {
        self.type_definitions.keys().collect()
    }

    /// Check if a type was generated
    pub fn has_type(&self, type_name: &str) -> bool {
        self.type_definitions.contains_key(type_name)
    }

    /// Build final Definitions structure for Grammar
    pub fn build_definitions(&mut self) {
        use crate::grammar::{GeneratedEnum, GeneratedStruct};

        crate::log_debug!(
            "🏗️  Building definitions from {} type generations",
            self.type_definitions.len()
        );

        // Clear existing definitions before rebuilding
        self.definitions.structs.clear();
        self.definitions.enums.clear();
        self.definitions.type_aliases.clear();
        self.definitions.constants.clear();

        // Convert type_definitions to definitions structure
        for (type_name, type_gen) in &self.type_definitions {
            crate::log_debug!(
                "   • Processing type '{}' (generated item kind: {})",
                type_name,
                type_gen.generated_item_kind.label()
            );

            match type_gen.generated_item_kind {
                GeneratedItemKind::Enum => {
                    let generated_enum = GeneratedEnum {
                        type_name: type_name.clone(),
                        definition: type_gen.definition_tokens.clone(),
                        implementation: type_gen.implementation_tokens.clone(),
                    };
                    crate::log_debug!("     → Added enum to definitions");
                    self.definitions.enums.push(generated_enum);
                }
                GeneratedItemKind::Struct => {
                    // Check if struct already exists to prevent duplicates
                    if self.definitions.structs.contains_key(type_name) {
                        crate::log_debug!(
                            "     → WARNING: Struct '{}' already exists in definitions",
                            type_name
                        );
                    } else {
                        let generated_struct = GeneratedStruct {
                            definition: type_gen.definition_tokens.clone(),
                            implementation: Some(type_gen.implementation_tokens.clone()),
                        };
                        self.definitions
                            .structs
                            .insert(type_name.clone(), generated_struct);
                        crate::log_debug!("     → Added struct to definitions");
                    }
                }
                GeneratedItemKind::TypeAlias => {
                    // Combine definition and implementation tokens properly
                    let mut combined = type_gen.definition_tokens.clone();
                    combined.extend(type_gen.implementation_tokens.clone());
                    self.definitions.type_aliases.push(combined);
                    crate::log_debug!("     → Added type alias to definitions");
                }
                GeneratedItemKind::Unknown => {
                    crate::log_debug!(
                        "     → WARNING: Unknown generated item kind for '{}'",
                        type_name
                    );
                }
            }
        }

        crate::log_debug!(
            "🏁 Final definitions: {} structs, {} enums, {} type aliases",
            self.definitions.structs.len(),
            self.definitions.enums.len(),
            self.definitions.type_aliases.len()
        );
    }

    /// Estimate number of tokens in a TokenStream (rough approximation)
    fn estimate_tokens(&self, tokens: &TokenStream) -> usize {
        // Simple approximation: count whitespace-separated tokens
        tokens.to_string().split_whitespace().count()
    }
}

impl TypeCodeGeneration {
    /// Create new type code generation
    pub fn new(
        type_name: String,
        definition_tokens: TokenStream,
        implementation_tokens: TokenStream,
    ) -> Self {
        Self {
            type_name,
            definition_tokens,
            implementation_tokens,
            dependencies: Vec::new(),
            uses_boxing: false,
            derived_traits: Vec::new(),
            generated_item_kind: GeneratedItemKind::Unknown,
            has_token_encoder_impl: false,
        }
    }

    /// Create generated code for a Rust struct or newtype wrapper.
    pub fn new_struct(
        type_name: String,
        definition_tokens: TokenStream,
        implementation_tokens: TokenStream,
    ) -> Self {
        Self::new(type_name, definition_tokens, implementation_tokens)
            .with_generated_item_kind(GeneratedItemKind::Struct)
    }

    /// Create generated code for a Rust enum.
    pub fn new_enum(
        type_name: String,
        definition_tokens: TokenStream,
        implementation_tokens: TokenStream,
    ) -> Self {
        Self::new(type_name, definition_tokens, implementation_tokens)
            .with_generated_item_kind(GeneratedItemKind::Enum)
    }

    /// Create generated code for a Rust type alias.
    pub fn new_type_alias(
        type_name: String,
        definition_tokens: TokenStream,
        implementation_tokens: TokenStream,
    ) -> Self {
        Self::new(type_name, definition_tokens, implementation_tokens)
            .with_generated_item_kind(GeneratedItemKind::TypeAlias)
    }

    /// Set the explicit generated item kind.
    pub fn with_generated_item_kind(mut self, item_kind: GeneratedItemKind) -> Self {
        self.generated_item_kind = item_kind;
        self
    }

    /// Set whether the generated type includes a JsonTokenEncoder implementation.
    pub fn with_token_encoder_impl(mut self, has_token_encoder_impl: bool) -> Self {
        self.has_token_encoder_impl = has_token_encoder_impl;
        self
    }

    /// Add a dependency
    pub fn add_dependency(&mut self, dependency: String) {
        if !self.dependencies.contains(&dependency) {
            self.dependencies.push(dependency);
        }
    }

    /// Mark as using Box wrapping
    pub fn set_uses_boxing(&mut self, uses_boxing: bool) {
        self.uses_boxing = uses_boxing;
    }

    /// Mark whether this type has a generated JsonTokenEncoder implementation.
    pub fn set_has_token_encoder_impl(&mut self, has_token_encoder_impl: bool) {
        self.has_token_encoder_impl = has_token_encoder_impl;
    }

    /// Add a derived trait
    pub fn add_derived_trait(&mut self, trait_name: String) {
        if !self.derived_traits.contains(&trait_name) {
            self.derived_traits.push(trait_name);
        }
    }

    /// Get total estimated size of generated code
    pub fn estimated_size(&self) -> usize {
        self.definition_tokens.to_string().len() + self.implementation_tokens.to_string().len()
    }
}

impl RootCodeGeneration {
    /// Create new root code generation
    pub fn new(
        type_name: String,
        definition_tokens: TokenStream,
        implementation_tokens: TokenStream,
        is_composition: bool,
    ) -> Self {
        Self {
            type_name,
            definition_tokens,
            implementation_tokens,
            is_composition,
        }
    }
}

// Manual Debug implementations since TokenStream doesn't implement Debug
impl std::fmt::Debug for CodeGeneration {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CodeGeneration")
            .field(
                "type_definitions",
                &format!("{} types", self.type_definitions.len()),
            )
            .field(
                "root_generation",
                &self.root_generation.as_ref().map(|r| &r.type_name),
            )
            .field("stats", &self.stats)
            .finish()
    }
}

impl std::fmt::Debug for TypeCodeGeneration {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TypeCodeGeneration")
            .field("type_name", &self.type_name)
            .field("dependencies", &self.dependencies)
            .field("uses_boxing", &self.uses_boxing)
            .field("derived_traits", &self.derived_traits)
            .field("generated_item_kind", &self.generated_item_kind)
            .field("has_token_encoder_impl", &self.has_token_encoder_impl)
            .field("definition_size", &self.definition_tokens.to_string().len())
            .field(
                "implementation_size",
                &self.implementation_tokens.to_string().len(),
            )
            .finish()
    }
}

impl std::fmt::Debug for RootCodeGeneration {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RootCodeGeneration")
            .field("type_name", &self.type_name)
            .field("is_composition", &self.is_composition)
            .field("definition_size", &self.definition_tokens.to_string().len())
            .field(
                "implementation_size",
                &self.implementation_tokens.to_string().len(),
            )
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quote::quote;

    // === Phase 4.5: CodeGeneration Coverage Tests ===
    // Batch 1: CodeGeneration Core Methods

    #[test]
    fn test_code_generation_add_type_generation() {
        // Test add_type_generation() with various TypeCodeGeneration instances
        let mut codegen = CodeGeneration::new();

        // Create a type with Eq/Ord/Hash traits
        let mut type_gen1 = TypeCodeGeneration::new_struct(
            "User".to_string(),
            quote! { pub struct User { name: String } },
            quote! { impl JsonTokenEncoder for User {} },
        )
        .with_token_encoder_impl(true);
        type_gen1.add_derived_trait("Eq".to_string());
        type_gen1.add_derived_trait("Ord".to_string());
        type_gen1.add_derived_trait("Hash".to_string());

        codegen.add_type_generation(type_gen1);

        // Verify stats updated
        assert_eq!(codegen.stats.total_types, 1);
        assert_eq!(codegen.stats.eq_derivations, 1);
        assert_eq!(codegen.stats.ord_derivations, 1);
        assert_eq!(codegen.stats.hash_derivations, 1);
        assert_eq!(codegen.stats.token_encoder_impls, 1);
        assert!(codegen.stats.total_tokens > 0);

        // Create a type with boxing
        let mut type_gen2 = TypeCodeGeneration::new_struct(
            "Node".to_string(),
            quote! { pub struct Node { value: Box<Node> } },
            quote! { impl JsonTokenEncoder for Node {} },
        )
        .with_token_encoder_impl(true);
        type_gen2.set_uses_boxing(true);

        codegen.add_type_generation(type_gen2);

        // Verify boxing counted
        assert_eq!(codegen.stats.total_types, 2);
        assert_eq!(codegen.stats.types_using_boxing, 1);
        assert_eq!(codegen.stats.token_encoder_impls, 2);

        // Create a type without JsonTokenEncoder
        let type_gen3 = TypeCodeGeneration::new_type_alias(
            "SimpleType".to_string(),
            quote! { pub type SimpleType = String; },
            quote! {},
        );

        codegen.add_type_generation(type_gen3);

        // Verify no additional JsonTokenEncoder counted
        assert_eq!(codegen.stats.total_types, 3);
        assert_eq!(codegen.stats.token_encoder_impls, 2);
        assert_eq!(codegen.stats.structs_generated, 2);
        assert_eq!(codegen.stats.type_aliases_generated, 1);
    }

    #[test]
    fn test_token_encoder_count_uses_metadata_not_implementation_text() {
        let mut codegen = CodeGeneration::new();

        let misleading_impl = TypeCodeGeneration::new_struct(
            "Misleading".to_string(),
            quote! { pub struct Misleading; },
            quote! { const LABEL: &str = "JsonTokenEncoder"; },
        );
        codegen.add_type_generation(misleading_impl);

        assert_eq!(
            codegen.stats.token_encoder_impls, 0,
            "implementation token text must not imply encoder metadata"
        );

        let empty_marked_impl = TypeCodeGeneration::new_struct(
            "Marked".to_string(),
            quote! { pub struct Marked; },
            quote! {},
        )
        .with_token_encoder_impl(true);
        codegen.add_type_generation(empty_marked_impl);

        assert_eq!(codegen.stats.token_encoder_impls, 1);
    }

    #[test]
    fn test_generated_item_stats_use_metadata_not_definition_text() {
        let mut codegen = CodeGeneration::new();

        let enum_text_marked_struct = TypeCodeGeneration::new(
            "EnumTextMarkedStruct".to_string(),
            quote! { pub enum EnumTextMarkedStruct { A } },
            quote! {},
        )
        .with_generated_item_kind(GeneratedItemKind::Struct);
        codegen.add_type_generation(enum_text_marked_struct);

        let type_alias_text_without_kind = TypeCodeGeneration::new(
            "TypeAliasTextWithoutKind".to_string(),
            quote! { pub type TypeAliasTextWithoutKind = String; },
            quote! {},
        );
        codegen.add_type_generation(type_alias_text_without_kind);

        assert_eq!(codegen.stats.structs_generated, 1);
        assert_eq!(
            codegen.stats.enums_generated, 0,
            "definition token text must not imply enum metadata"
        );
        assert_eq!(
            codegen.stats.type_aliases_generated, 0,
            "definition token text must not imply type-alias metadata"
        );
    }

    #[test]
    fn test_code_generation_type_lookup_methods() {
        // Test get_type_generation(), get_generated_type_names(), has_type()
        let mut codegen = CodeGeneration::new();

        // Add some types
        let type1 = TypeCodeGeneration::new_struct(
            "Foo".to_string(),
            quote! { pub struct Foo {} },
            quote! {},
        );
        let type2 = TypeCodeGeneration::new_struct(
            "Bar".to_string(),
            quote! { pub struct Bar {} },
            quote! {},
        );
        let type3 =
            TypeCodeGeneration::new_enum("Baz".to_string(), quote! { pub enum Baz {} }, quote! {});

        codegen.add_type_generation(type1);
        codegen.add_type_generation(type2);
        codegen.add_type_generation(type3);

        // Test get_type_generation()
        let foo = codegen.get_type_generation("Foo");
        assert!(foo.is_some());
        assert_eq!(foo.unwrap().type_name, "Foo");

        let bar = codegen.get_type_generation("Bar");
        assert!(bar.is_some());
        assert_eq!(bar.unwrap().type_name, "Bar");

        // Test non-existent type
        assert!(codegen.get_type_generation("NonExistent").is_none());

        // Test get_generated_type_names()
        let type_names = codegen.get_generated_type_names();
        assert_eq!(type_names.len(), 3);
        assert!(type_names.contains(&&"Foo".to_string()));
        assert!(type_names.contains(&&"Bar".to_string()));
        assert!(type_names.contains(&&"Baz".to_string()));

        // Test has_type()
        assert!(codegen.has_type("Foo"));
        assert!(codegen.has_type("Bar"));
        assert!(codegen.has_type("Baz"));
        assert!(!codegen.has_type("NonExistent"));
        assert!(!codegen.has_type("Missing"));
    }

    #[test]
    fn test_code_generation_root_management() {
        // Test set_root_generation() storage
        let mut codegen = CodeGeneration::new();

        // Initially no root
        assert!(codegen.root_generation.is_none());

        // Add root generation (non-composition)
        let root1 = RootCodeGeneration::new(
            "Root".to_string(),
            quote! { pub struct Root {} },
            quote! { impl JsonTokenEncoder for Root {} },
            false,
        );

        codegen.set_root_generation(root1);

        // Verify root set
        assert!(codegen.root_generation.is_some());
        let root = codegen.root_generation.as_ref().unwrap();
        assert_eq!(root.type_name, "Root");
        assert!(!root.is_composition);

        // Replace with composition root
        let root2 = RootCodeGeneration::new(
            "RootEnum".to_string(),
            quote! { pub enum RootEnum { A, B } },
            quote! { impl JsonTokenEncoder for RootEnum {} },
            true,
        );

        codegen.set_root_generation(root2);

        // Verify root replaced
        let root = codegen.root_generation.as_ref().unwrap();
        assert_eq!(root.type_name, "RootEnum");
        assert!(root.is_composition);
    }

    // Batch 2: build_definitions() Integration

    #[test]
    fn test_build_definitions_struct_detection() {
        // Test struct detection and addition to definitions.structs
        let mut codegen = CodeGeneration::new();

        // Add structs
        let struct1 = TypeCodeGeneration::new_struct(
            "User".to_string(),
            quote! { pub struct User { name: String } },
            quote! { impl JsonTokenEncoder for User {} },
        );

        let struct2 = TypeCodeGeneration::new_struct(
            "Post".to_string(),
            quote! { pub struct Post { title: String, author: User } },
            quote! { impl JsonTokenEncoder for Post {} },
        );

        codegen.add_type_generation(struct1);
        codegen.add_type_generation(struct2);

        // Build definitions
        codegen.build_definitions();

        // Verify structs added
        assert_eq!(codegen.definitions.structs.len(), 2);
        assert!(codegen.definitions.structs.contains_key("User"));
        assert!(codegen.definitions.structs.contains_key("Post"));
        assert_eq!(codegen.definitions.enums.len(), 0);
        assert_eq!(codegen.definitions.type_aliases.len(), 0);

        // Test duplicate prevention - add same struct again
        let duplicate_struct = TypeCodeGeneration::new_struct(
            "User".to_string(),
            quote! { pub struct User { email: String } },
            quote! {},
        );

        codegen.add_type_generation(duplicate_struct);
        codegen.build_definitions();

        // Should still have 2 unique structs (duplicate not added)
        assert_eq!(codegen.definitions.structs.len(), 2);
    }

    #[test]
    fn test_build_definitions_enum_detection() {
        // Test enum detection and addition to definitions.enums
        let mut codegen = CodeGeneration::new();

        // Add enums
        let enum1 = TypeCodeGeneration::new_enum(
            "Status".to_string(),
            quote! { pub enum Status { Active, Inactive, Pending } },
            quote! { impl JsonTokenEncoder for Status {} },
        );

        let enum2 = TypeCodeGeneration::new_enum(
            "Color".to_string(),
            quote! { pub enum Color { Red, Green, Blue } },
            quote! { impl JsonTokenEncoder for Color {} },
        );

        codegen.add_type_generation(enum1);
        codegen.add_type_generation(enum2);

        // Build definitions
        codegen.build_definitions();

        // Verify enums added
        assert_eq!(codegen.definitions.enums.len(), 2);
        assert_eq!(codegen.definitions.structs.len(), 0);
        assert_eq!(codegen.definitions.type_aliases.len(), 0);

        // Add more enums and rebuild
        let enum3 = TypeCodeGeneration::new_enum(
            "Priority".to_string(),
            quote! { pub enum Priority { Low, Medium, High } },
            quote! {},
        );

        codegen.add_type_generation(enum3);
        codegen.build_definitions();

        // Should have all 3 enums
        assert_eq!(codegen.definitions.enums.len(), 3);
    }

    #[test]
    fn test_build_definitions_type_alias_detection() {
        // Test type alias detection and addition to definitions.type_aliases
        let mut codegen = CodeGeneration::new();

        // Add type aliases
        let alias1 = TypeCodeGeneration::new_type_alias(
            "UserId".to_string(),
            quote! { pub type UserId = String; },
            quote! {},
        );

        let alias2 = TypeCodeGeneration::new_type_alias(
            "Timestamp".to_string(),
            quote! { pub type Timestamp = i64; },
            quote! {},
        );

        codegen.add_type_generation(alias1);
        codegen.add_type_generation(alias2);

        // Build definitions
        codegen.build_definitions();

        // Verify type aliases added
        assert_eq!(codegen.definitions.type_aliases.len(), 2);
        assert_eq!(codegen.definitions.structs.len(), 0);
        assert_eq!(codegen.definitions.enums.len(), 0);

        // Test mixed types
        let mut codegen2 = CodeGeneration::new();

        codegen2.add_type_generation(TypeCodeGeneration::new_struct(
            "User".to_string(),
            quote! { pub struct User {} },
            quote! {},
        ));
        codegen2.add_type_generation(TypeCodeGeneration::new_enum(
            "Status".to_string(),
            quote! { pub enum Status { A, B } },
            quote! {},
        ));
        codegen2.add_type_generation(TypeCodeGeneration::new_type_alias(
            "UserId".to_string(),
            quote! { pub type UserId = String; },
            quote! {},
        ));

        codegen2.build_definitions();

        // Verify all types added to correct categories
        assert_eq!(codegen2.definitions.structs.len(), 1);
        assert_eq!(codegen2.definitions.enums.len(), 1);
        assert_eq!(codegen2.definitions.type_aliases.len(), 1);
    }

    #[test]
    fn test_build_definitions_uses_item_kind_not_definition_text() {
        let mut codegen = CodeGeneration::new();

        codegen.add_type_generation(TypeCodeGeneration::new_struct(
            "MisleadingStruct".to_string(),
            quote! {
                #[doc = "mentions pub enum but is still a struct"]
                pub struct MisleadingStruct {}
            },
            quote! {},
        ));

        codegen.add_type_generation(TypeCodeGeneration::new_enum(
            "MisleadingEnum".to_string(),
            quote! {
                #[doc = "mentions pub struct but is still an enum"]
                pub enum MisleadingEnum { A }
            },
            quote! {},
        ));

        codegen.build_definitions();

        assert!(codegen.definitions.structs.contains_key("MisleadingStruct"));
        assert_eq!(codegen.definitions.enums.len(), 1);
        assert_eq!(codegen.definitions.type_aliases.len(), 0);
    }

    // Batch 3: TypeCodeGeneration Methods

    #[test]
    fn test_type_code_generation_dependencies() {
        // Test add_dependency() adds dependencies with duplicate prevention
        let mut type_gen =
            TypeCodeGeneration::new("Post".to_string(), quote! { pub struct Post {} }, quote! {});

        // Initially no dependencies
        assert_eq!(type_gen.dependencies.len(), 0);

        // Add dependencies
        type_gen.add_dependency("User".to_string());
        type_gen.add_dependency("Comment".to_string());
        type_gen.add_dependency("Tag".to_string());

        assert_eq!(type_gen.dependencies.len(), 3);
        assert!(type_gen.dependencies.contains(&"User".to_string()));
        assert!(type_gen.dependencies.contains(&"Comment".to_string()));
        assert!(type_gen.dependencies.contains(&"Tag".to_string()));

        // Test duplicate prevention
        type_gen.add_dependency("User".to_string());
        type_gen.add_dependency("Comment".to_string());

        // Should still have 3 unique dependencies
        assert_eq!(type_gen.dependencies.len(), 3);

        // Add new dependency
        type_gen.add_dependency("Category".to_string());
        assert_eq!(type_gen.dependencies.len(), 4);
        assert!(type_gen.dependencies.contains(&"Category".to_string()));
    }

    #[test]
    fn test_type_code_generation_traits_and_boxing() {
        // Test add_derived_trait() and set_uses_boxing()
        let mut type_gen =
            TypeCodeGeneration::new("Node".to_string(), quote! { pub struct Node {} }, quote! {});

        // Initially no traits and no boxing
        assert_eq!(type_gen.derived_traits.len(), 0);
        assert!(!type_gen.uses_boxing);

        // Add traits
        type_gen.add_derived_trait("Eq".to_string());
        type_gen.add_derived_trait("Ord".to_string());
        type_gen.add_derived_trait("Hash".to_string());

        assert_eq!(type_gen.derived_traits.len(), 3);
        assert!(type_gen.derived_traits.contains(&"Eq".to_string()));
        assert!(type_gen.derived_traits.contains(&"Ord".to_string()));
        assert!(type_gen.derived_traits.contains(&"Hash".to_string()));

        // Test duplicate trait prevention
        type_gen.add_derived_trait("Eq".to_string());
        type_gen.add_derived_trait("Ord".to_string());

        // Should still have 3 unique traits
        assert_eq!(type_gen.derived_traits.len(), 3);

        // Set boxing
        type_gen.set_uses_boxing(true);
        assert!(type_gen.uses_boxing);

        // Unset boxing
        type_gen.set_uses_boxing(false);
        assert!(!type_gen.uses_boxing);

        // Test adding more traits
        type_gen.add_derived_trait("Clone".to_string());
        type_gen.add_derived_trait("Debug".to_string());

        assert_eq!(type_gen.derived_traits.len(), 5);
        assert!(type_gen.derived_traits.contains(&"Clone".to_string()));
        assert!(type_gen.derived_traits.contains(&"Debug".to_string()));
    }

    #[test]
    fn test_type_code_generation_size_estimation() {
        // Test estimated_size() calculates correctly
        let type_gen1 = TypeCodeGeneration::new(
            "Small".to_string(),
            quote! { pub struct Small {} },
            quote! {},
        );

        let size1 = type_gen1.estimated_size();
        assert!(size1 > 0);

        // Larger type with more code
        let type_gen2 = TypeCodeGeneration::new(
            "Large".to_string(),
            quote! {
                pub struct Large {
                    field1: String,
                    field2: i64,
                    field3: bool,
                    field4: Vec<String>,
                }
            },
            quote! {
                impl JsonTokenEncoder for Large {
                    fn encode(&self, tokens: &mut Vec<u32>) {
                        // encoding logic
                    }
                    fn decode(tokens: &mut impl Iterator<Item = u32>) -> Result<Self, DecodeError> {
                        // decoding logic
                    }
                }
            },
        );

        let size2 = type_gen2.estimated_size();
        assert!(
            size2 > size1,
            "Larger type should have larger estimated size"
        );

        // Test with only definition, no implementation
        let type_gen3 = TypeCodeGeneration::new(
            "TypeAlias".to_string(),
            quote! { pub type TypeAlias = String; },
            quote! {},
        );

        let size3 = type_gen3.estimated_size();
        assert!(size3 > 0);
        assert!(size3 < size2);
    }

    // Batch 4: Factory Methods and Edge Cases

    #[test]
    fn test_code_generation_factory_methods_and_stats() {
        // Test CodeGeneration::new() initializes correctly
        let codegen = CodeGeneration::new();
        assert_eq!(codegen.type_definitions.len(), 0);
        assert_eq!(codegen.definitions.structs.len(), 0);
        assert_eq!(codegen.definitions.enums.len(), 0);
        assert_eq!(codegen.definitions.type_aliases.len(), 0);
        assert!(codegen.root_generation.is_none());
        assert_eq!(codegen.stats.total_types, 0);
        assert_eq!(codegen.stats.eq_derivations, 0);
        assert_eq!(codegen.stats.types_using_boxing, 0);

        // Test TypeCodeGeneration::new() initializes correctly
        let type_gen = TypeCodeGeneration::new(
            "Test".to_string(),
            quote! { pub struct Test {} },
            quote! { impl JsonTokenEncoder for Test {} },
        );
        assert_eq!(type_gen.type_name, "Test");
        assert_eq!(type_gen.dependencies.len(), 0);
        assert!(!type_gen.uses_boxing);
        assert_eq!(type_gen.derived_traits.len(), 0);
        assert_eq!(type_gen.generated_item_kind, GeneratedItemKind::Unknown);
        assert!(!type_gen.has_token_encoder_impl);

        // Test RootCodeGeneration::new() initializes correctly
        let root = RootCodeGeneration::new(
            "Root".to_string(),
            quote! { pub struct Root {} },
            quote! {},
            true,
        );
        assert_eq!(root.type_name, "Root");
        assert!(root.is_composition);

        // Test estimate_tokens() helper
        let codegen2 = CodeGeneration::new();
        let tokens = quote! { pub struct User { name: String, age: i64 } };
        let token_count = codegen2.estimate_tokens(&tokens);
        assert!(token_count > 0);

        // More complex tokens should have more token count
        let complex_tokens = quote! {
            pub struct ComplexType {
                field1: String,
                field2: Vec<i64>,
                field3: HashMap<String, Value>,
            }
            impl JsonTokenEncoder for ComplexType {
                fn encode(&self, tokens: &mut Vec<u32>) {}
            }
        };
        let complex_count = codegen2.estimate_tokens(&complex_tokens);
        assert!(complex_count > token_count);

        // Test GenerationStats default values
        let stats = GenerationStats::default();
        assert_eq!(stats.total_types, 0);
        assert_eq!(stats.structs_generated, 0);
        assert_eq!(stats.enums_generated, 0);
        assert_eq!(stats.type_aliases_generated, 0);
        assert_eq!(stats.total_tokens, 0);
        assert_eq!(stats.eq_derivations, 0);
        assert_eq!(stats.ord_derivations, 0);
        assert_eq!(stats.hash_derivations, 0);
        assert_eq!(stats.types_using_boxing, 0);
        assert_eq!(stats.token_encoder_impls, 0);
    }

    // === Additional Coverage Tests ===

    #[test]
    fn test_code_generation_default_trait() {
        // Test Default trait implementation for CodeGeneration
        let codegen = CodeGeneration::default();
        assert_eq!(codegen.type_definitions.len(), 0);
        assert!(codegen.root_generation.is_none());
        assert_eq!(codegen.stats.total_types, 0);

        // Verify Default produces same result as new()
        let codegen_new = CodeGeneration::new();
        assert_eq!(
            codegen.type_definitions.len(),
            codegen_new.type_definitions.len()
        );
        assert_eq!(codegen.stats.total_types, codegen_new.stats.total_types);
    }

    #[test]
    fn test_trait_counting_with_other_traits() {
        // Test trait counting with traits other than Eq/Ord/Hash (hits _ arm)
        let mut codegen = CodeGeneration::new();

        // Add type with Clone and Debug traits (not Eq/Ord/Hash)
        let mut type_gen = TypeCodeGeneration::new(
            "MyType".to_string(),
            quote! { pub struct MyType {} },
            quote! {},
        );
        type_gen.add_derived_trait("Clone".to_string());
        type_gen.add_derived_trait("Debug".to_string());
        type_gen.add_derived_trait("PartialEq".to_string());

        codegen.add_type_generation(type_gen);

        // Verify these traits don't increment Eq/Ord/Hash counters
        assert_eq!(codegen.stats.eq_derivations, 0);
        assert_eq!(codegen.stats.ord_derivations, 0);
        assert_eq!(codegen.stats.hash_derivations, 0);
        assert_eq!(codegen.stats.total_types, 1);
    }

    #[test]
    fn test_debug_implementations() {
        // Test Debug trait implementations for all types
        let mut codegen = CodeGeneration::new();

        // Add a type
        let type_gen = TypeCodeGeneration::new(
            "User".to_string(),
            quote! { pub struct User { name: String } },
            quote! { impl JsonTokenEncoder for User {} },
        );
        codegen.add_type_generation(type_gen);

        // Add root generation
        let root = RootCodeGeneration::new(
            "Root".to_string(),
            quote! { pub struct Root {} },
            quote! {},
            true,
        );
        codegen.set_root_generation(root);

        // Test CodeGeneration Debug
        let debug_str = format!("{:?}", codegen);
        assert!(debug_str.contains("CodeGeneration"));
        assert!(debug_str.contains("1 types"));
        assert!(debug_str.contains("Root"));

        // Test TypeCodeGeneration Debug
        let type_gen2 = TypeCodeGeneration::new(
            "TestType".to_string(),
            quote! { pub struct TestType {} },
            quote! { impl JsonTokenEncoder for TestType {} },
        );
        let debug_str2 = format!("{:?}", type_gen2);
        assert!(debug_str2.contains("TypeCodeGeneration"));
        assert!(debug_str2.contains("TestType"));
        assert!(debug_str2.contains("definition_size"));
        assert!(debug_str2.contains("implementation_size"));

        // Test RootCodeGeneration Debug
        let root2 = RootCodeGeneration::new(
            "MyRoot".to_string(),
            quote! { pub enum MyRoot { A, B } },
            quote! {},
            false,
        );
        let debug_str3 = format!("{:?}", root2);
        assert!(debug_str3.contains("RootCodeGeneration"));
        assert!(debug_str3.contains("MyRoot"));
        assert!(debug_str3.contains("is_composition"));
        assert!(debug_str3.contains("false"));
    }

    #[test]
    fn test_build_definitions_unknown_type() {
        // Test build_definitions with unknown type definition (not struct/enum/type alias)
        let mut codegen = CodeGeneration::new();

        // Add a type with unknown definition (doesn't contain "pub struct", "pub enum", or "pub type")
        let unknown_type = TypeCodeGeneration::new(
            "UnknownType".to_string(),
            quote! { const VALUE: u32 = 42; }, // const, not a struct/enum/type
            quote! {},
        );

        codegen.add_type_generation(unknown_type);

        // Build definitions (should log warning for unknown type)
        codegen.build_definitions();

        // Verify unknown type not added to any category
        assert_eq!(codegen.definitions.structs.len(), 0);
        assert_eq!(codegen.definitions.enums.len(), 0);
        assert_eq!(codegen.definitions.type_aliases.len(), 0);
    }

    #[test]
    fn test_build_definitions_duplicate_struct_warning() {
        // Test that duplicate struct generates warning but doesn't add duplicate
        let mut codegen = CodeGeneration::new();

        // Add first struct
        let struct1 = TypeCodeGeneration::new_struct(
            "User".to_string(),
            quote! { pub struct User { name: String } },
            quote! {},
        );
        codegen.add_type_generation(struct1);
        codegen.build_definitions();

        // Verify first struct added
        assert_eq!(codegen.definitions.structs.len(), 1);
        assert!(codegen.definitions.structs.contains_key("User"));

        // Add duplicate struct with same name
        let duplicate_struct = TypeCodeGeneration::new_struct(
            "User".to_string(),
            quote! { pub struct User { email: String } },
            quote! {},
        );
        codegen.add_type_generation(duplicate_struct);
        codegen.build_definitions();

        // Should still have only 1 struct (duplicate not added, warning logged)
        assert_eq!(codegen.definitions.structs.len(), 1);
        assert!(codegen.definitions.structs.contains_key("User"));
    }
}
