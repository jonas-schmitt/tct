//! Root Generator - Specialized code generation for root schema handling
//!
//! This module handles code generation for the root schema, which requires special
//! handling when the root is a composition (oneOf/anyOf at the root level).
//!
//! Root compositions need to be generated directly into root_generation rather than
//! type_definitions to prevent duplicate code generation.
//!
//! Extracted from code_generator.rs as part of module size refactoring (2025-10-04).

use crate::TctResult;
use crate::generator::enum_generator::EnumGenerator;
use crate::generator::generation_output::RootCodeGeneration;
use crate::generator::schema_discovery::{MemberType, SchemaDiscovery};
use crate::generator::type_analysis::{TraitDerivability, TypeAnalysis};
use proc_macro2::TokenStream;

/// Specialized generator for root schema handling
///
/// Handles the special case of root compositions (oneOf/anyOf at root level)
/// which need to be generated directly into root_generation rather than
/// type_definitions to prevent duplicates.
///
/// Uses dependency injection for closures to avoid premature extraction of utilities.
#[derive(Default)]
pub struct RootGenerator;

impl RootGenerator {
    /// Create a new root generator
    pub fn new() -> Self {
        Self
    }

    /// Process root schema generation
    ///
    /// # Parameters
    ///
    /// - `discovery`: Schema discovery results containing root_info
    /// - `analysis`: Type analysis results for trait derivability
    /// - `has_type_fn`: Function to check if a type has already been generated
    /// - `generate_composition_fn`: Function to generate root composition directly
    ///
    /// # Returns
    ///
    /// Optional RootCodeGeneration if root is a composition
    pub fn generate_root(
        &self,
        discovery: &SchemaDiscovery,
        analysis: &TypeAnalysis,
        has_type_fn: &dyn Fn(&str) -> bool,
        generate_composition_fn: &dyn Fn(
            &crate::generator::schema_discovery::TypeDiscovery,
            Option<&TraitDerivability>,
        ) -> TctResult<RootCodeGeneration>,
    ) -> TctResult<Option<RootCodeGeneration>> {
        crate::log_debug!("🌱 Processing root schema generation");

        // Clone root_info to avoid borrow conflicts
        if let Some(root_info) = discovery.root_info.clone() {
            let root_type_name = &root_info.type_name;
            crate::log_debug!("   • Root type name: '{}'", root_type_name);
            crate::log_debug!("   • Is composition: {}", root_info.is_composition);

            // Check if root is a composition (oneOf/anyOf at root level)
            if root_info.is_composition {
                crate::log_debug!(
                    "   • Generating root composition enum using multi-pass approach"
                );

                // Check if this type was already generated to prevent duplicates
                if has_type_fn(root_type_name) {
                    crate::log_debug!(
                        "   • Root type '{}' already generated, skipping",
                        root_type_name
                    );
                    return Ok(None);
                }

                // Use multi-pass approach: get discovered type info and generate for ROOT ONLY
                if let Some(root_type_discovery) = discovery.get_type(&root_info.type_name).cloned()
                {
                    let root_trait_derivability = analysis
                        .trait_derivability
                        .get(&root_info.type_name)
                        .cloned();
                    crate::log_debug!(
                        "   • Found discovered type with {} members",
                        root_type_discovery.members.len()
                    );

                    // CRITICAL FIX: Generate root composition directly for root_generation
                    // Do NOT use generate_enum() as it adds to type_definitions causing duplicates
                    let root_generation = generate_composition_fn(
                        &root_type_discovery,
                        root_trait_derivability.as_ref(),
                    )?;
                    crate::log_debug!(
                        "   • Root composition generated directly as root_generation (no type_definitions entry)"
                    );

                    crate::log_debug!(
                        "   • Root composition generated successfully using multi-pass"
                    );
                    return Ok(Some(root_generation));
                } else {
                    return Err(crate::error::TctError::CodeGeneration {
                        message: format!(
                            "Root type {} not found in discovery results",
                            root_info.type_name
                        ),
                        schema_path: "#/".to_string(),
                        suggestion: "Ensure DiscoveryPass discovered the root type correctly"
                            .to_string(),
                        phase: crate::error::GenerationPhase::TypeGeneration,
                    });
                }
            } else {
                crate::log_debug!("   • Root is regular type - verifying existence");
                // Root is a regular struct - should already be generated
                // Just verify it exists
                if !has_type_fn(root_type_name) {
                    crate::log_debug!(
                        "   • ERROR: Root type '{}' not found in generated types",
                        root_type_name
                    );
                    return Err(crate::error::TctError::CodeGeneration {
                        message: format!("Root type {} was not generated", root_type_name),
                        schema_path: "#/".to_string(),
                        suggestion: "Ensure root type is properly discovered and generated"
                            .to_string(),
                        phase: crate::error::GenerationPhase::TypeGeneration,
                    });
                }
                crate::log_debug!("   • Root type '{}' verified successfully", root_type_name);
            }
        } else {
            crate::log_debug!("   • No root info found - using default behavior");
        }

        Ok(None)
    }

    /// Generate root composition directly for root_generation (no type_definitions entry)
    ///
    /// This prevents duplicates by bypassing the type_definitions -> build_definitions path.
    /// Delegates to EnumGenerator for the actual enum generation.
    ///
    /// # Parameters
    ///
    /// - `discovery`: Schema discovery results for enum generator
    /// - `type_discovery`: Type discovery info for the root composition
    /// - `trait_derivability`: Optional trait analysis for the root type
    /// - `member_type_fn`: Function to generate member type tokens
    /// - `derive_traits_fn`: Function to generate derive traits
    /// - `derive_traits_without_deserialize_fn`: Function to generate derive traits without Deserialize
    /// - `derive_traits_without_default_fn`: Function to generate derive traits without Default
    /// - `derive_traits_without_deserialize_and_default_fn`: Function excluding both
    ///
    /// # Returns
    ///
    /// RootCodeGeneration with the generated root composition
    #[allow(clippy::too_many_arguments)]
    pub fn generate_root_composition_direct(
        &self,
        discovery: &SchemaDiscovery,
        type_discovery: &crate::generator::schema_discovery::TypeDiscovery,
        trait_derivability: Option<&TraitDerivability>,
        member_type_fn: &dyn Fn(&MemberType) -> TctResult<TokenStream>,
        derive_traits_fn: &dyn Fn(Option<&TraitDerivability>, bool) -> TokenStream,
        derive_traits_without_deserialize_fn: &dyn Fn(
            Option<&TraitDerivability>,
            bool,
        ) -> TokenStream,
        derive_traits_without_default_fn: &dyn Fn(Option<&TraitDerivability>, bool) -> TokenStream,
        derive_traits_without_deserialize_and_default_fn: &dyn Fn(
            Option<&TraitDerivability>,
            bool,
        ) -> TokenStream,
        streaming_encode: bool,
    ) -> TctResult<RootCodeGeneration> {
        crate::log_debug!(
            "🎯 Generating root composition directly: '{}'",
            type_discovery.type_name
        );

        // Delegate to EnumGenerator for root composition generation
        let enum_generator = EnumGenerator::new(discovery, streaming_encode);

        let type_generation = enum_generator.generate(
            type_discovery,
            trait_derivability,
            None, // Root compositions use standard encoding for now
            member_type_fn,
            derive_traits_fn,
            derive_traits_without_deserialize_fn,
            derive_traits_without_default_fn,
            derive_traits_without_deserialize_and_default_fn,
        )?;

        // Convert TypeCodeGeneration to RootCodeGeneration
        Ok(RootCodeGeneration::new(
            type_generation.type_name,
            type_generation.definition_tokens,
            type_generation.implementation_tokens,
            true, // is_composition
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generator::schema_discovery::{
        MemberInfo, RootSchemaInfo, SchemaDiscovery, SchemaLocation, TypeDiscovery, TypeKind,
    };
    use crate::generator::type_analysis::TypeAnalysis;
    use quote::quote;

    fn create_test_location() -> SchemaLocation {
        SchemaLocation {
            json_path: "#/".to_string(),
            description: "Test location".to_string(),
            parent_type: None,
            property_name: None,
        }
    }

    #[test]
    fn test_root_generator_new() {
        let generator = RootGenerator::new();
        // Verify it's created (simple constructor test)
        let _ = generator;
    }

    #[test]
    fn test_root_generator_default() {
        let generator = RootGenerator;
        let _ = generator;
    }

    #[test]
    fn test_generate_root_no_root_info() {
        // Test: No root_info in discovery → Ok(None)
        let generator = RootGenerator::new();
        let discovery = SchemaDiscovery::new(); // No root_info
        let analysis = TypeAnalysis::default();

        let has_type_fn = |_: &str| false;
        let generate_composition_fn: &dyn Fn(
            &TypeDiscovery,
            Option<&TraitDerivability>,
        ) -> TctResult<RootCodeGeneration> =
            &|_, _| panic!("Should not be called when no root_info");

        let result =
            generator.generate_root(&discovery, &analysis, &has_type_fn, generate_composition_fn);

        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn test_generate_root_composition_already_generated() {
        // Test: Root is composition but already generated → Ok(None)
        let generator = RootGenerator::new();
        let mut discovery = SchemaDiscovery::new();

        // Set root_info with is_composition = true
        discovery.root_info = Some(RootSchemaInfo {
            type_name: "RootEnum".to_string(),
            schema: serde_json::Map::new(),
            is_composition: true,
        });

        // Register the type in discovery
        discovery.register_type(TypeDiscovery::new(
            "RootEnum".to_string(),
            TypeKind::Enum,
            create_test_location(),
            serde_json::Map::new(),
        ));

        let analysis = TypeAnalysis::default();

        // has_type_fn returns true (type already generated)
        let has_type_fn = |name: &str| name == "RootEnum";
        let generate_composition_fn: &dyn Fn(
            &TypeDiscovery,
            Option<&TraitDerivability>,
        ) -> TctResult<RootCodeGeneration> =
            &|_, _| panic!("Should not be called when type already generated");

        let result =
            generator.generate_root(&discovery, &analysis, &has_type_fn, generate_composition_fn);

        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn test_generate_root_composition_success() {
        // Test: Root is composition, not yet generated → Ok(Some(...))
        let generator = RootGenerator::new();
        let mut discovery = SchemaDiscovery::new();

        discovery.root_info = Some(RootSchemaInfo {
            type_name: "RootEnum".to_string(),
            schema: serde_json::Map::new(),
            is_composition: true,
        });

        // Register the type
        discovery.register_type(TypeDiscovery::new(
            "RootEnum".to_string(),
            TypeKind::Enum,
            create_test_location(),
            serde_json::Map::new(),
        ));

        let analysis = TypeAnalysis::default();

        // has_type_fn returns false (type not yet generated)
        let has_type_fn = |_: &str| false;
        let generate_composition_fn: &dyn Fn(
            &TypeDiscovery,
            Option<&TraitDerivability>,
        ) -> TctResult<RootCodeGeneration> = &|type_disc: &TypeDiscovery, _| {
            // Mock generation
            Ok(RootCodeGeneration::new(
                type_disc.type_name.clone(),
                quote! { pub enum RootEnum {} },
                quote! { impl RootEnum {} },
                true,
            ))
        };

        let result =
            generator.generate_root(&discovery, &analysis, &has_type_fn, generate_composition_fn);

        assert!(result.is_ok());
        assert!(result.unwrap().is_some());
    }

    #[test]
    fn test_generate_root_composition_not_in_discovery() {
        // Test: Root is composition but not found in discovery → Err
        let generator = RootGenerator::new();
        let mut discovery = SchemaDiscovery::new();

        discovery.root_info = Some(RootSchemaInfo {
            type_name: "MissingType".to_string(),
            schema: serde_json::Map::new(),
            is_composition: true,
        });

        // Do NOT register the type (simulate discovery failure)

        let analysis = TypeAnalysis::default();

        let has_type_fn = |_: &str| false;
        let generate_composition_fn: &dyn Fn(
            &TypeDiscovery,
            Option<&TraitDerivability>,
        ) -> TctResult<RootCodeGeneration> =
            &|_, _| panic!("Should not be called when type not in discovery");

        let result =
            generator.generate_root(&discovery, &analysis, &has_type_fn, generate_composition_fn);

        assert!(result.is_err());
        match result {
            Err(crate::error::TctError::CodeGeneration { message, .. }) => {
                assert!(message.contains("MissingType"));
                assert!(message.contains("not found in discovery"));
            }
            _ => panic!("Expected CodeGeneration error"),
        }
    }

    #[test]
    fn test_generate_root_regular_not_generated() {
        // Test: Root is NOT composition but not found in generated types → Err
        let generator = RootGenerator::new();
        let mut discovery = SchemaDiscovery::new();

        discovery.root_info = Some(RootSchemaInfo {
            type_name: "RegularRoot".to_string(),
            schema: serde_json::Map::new(),
            is_composition: false, // Regular struct
        });

        let analysis = TypeAnalysis::default();

        // has_type_fn returns false (type not found)
        let has_type_fn = |_: &str| false;
        let generate_composition_fn: &dyn Fn(
            &TypeDiscovery,
            Option<&TraitDerivability>,
        ) -> TctResult<RootCodeGeneration> =
            &|_, _| panic!("Should not be called for regular root");

        let result =
            generator.generate_root(&discovery, &analysis, &has_type_fn, generate_composition_fn);

        assert!(result.is_err());
        match result {
            Err(crate::error::TctError::CodeGeneration { message, .. }) => {
                assert!(message.contains("RegularRoot"));
                assert!(message.contains("not generated"));
            }
            _ => panic!("Expected CodeGeneration error"),
        }
    }

    #[test]
    fn test_generate_root_regular_already_exists() {
        // Test: Root is NOT composition and exists in generated types → Ok(None)
        let generator = RootGenerator::new();
        let mut discovery = SchemaDiscovery::new();

        discovery.root_info = Some(RootSchemaInfo {
            type_name: "RegularRoot".to_string(),
            schema: serde_json::Map::new(),
            is_composition: false,
        });

        let analysis = TypeAnalysis::default();

        // has_type_fn returns true (type exists)
        let has_type_fn = |name: &str| name == "RegularRoot";
        let generate_composition_fn: &dyn Fn(
            &TypeDiscovery,
            Option<&TraitDerivability>,
        ) -> TctResult<RootCodeGeneration> =
            &|_, _| panic!("Should not be called for regular root");

        let result =
            generator.generate_root(&discovery, &analysis, &has_type_fn, generate_composition_fn);

        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn test_generate_root_composition_direct() {
        // Test: generate_root_composition_direct delegates to EnumGenerator
        let generator = RootGenerator::new();
        let discovery = SchemaDiscovery::new();

        // Create a mock type discovery for root enum
        let mut type_discovery = TypeDiscovery::new(
            "RootEnum".to_string(),
            TypeKind::Enum,
            create_test_location(),
            serde_json::Map::new(),
        );

        // Add a member to the enum
        type_discovery.members.push(MemberInfo {
            name: "VariantA".to_string(),
            member_type: MemberType::TypeReference("VariantA".to_string()),
            required: true,
            is_nullable: false,
            schema: serde_json::Map::new(),
        });

        // Mock closures for enum generation
        let member_type_fn: &dyn Fn(&MemberType) -> TctResult<TokenStream> = &|member| match member
        {
            MemberType::TypeReference(type_name) => {
                let ident = quote::format_ident!("{}", type_name);
                Ok(quote! { #ident })
            }
            MemberType::Primitive(prim) => {
                let ident = quote::format_ident!("{}", prim);
                Ok(quote! { #ident })
            }
            _ => Ok(quote! { Unknown }),
        };

        let derive_traits_fn: &dyn Fn(Option<&TraitDerivability>, bool) -> TokenStream =
            &|_, _| quote! { #[derive(Debug, Clone)] };

        let derive_traits_without_deserialize_fn: &dyn Fn(
            Option<&TraitDerivability>,
            bool,
        ) -> TokenStream = &|_, _| quote! { #[derive(Debug, Clone)] };

        let derive_traits_without_default_fn: &dyn Fn(
            Option<&TraitDerivability>,
            bool,
        ) -> TokenStream = &|_, _| quote! { #[derive(Debug, Clone)] };

        let derive_traits_without_deserialize_and_default_fn: &dyn Fn(
            Option<&TraitDerivability>,
            bool,
        ) -> TokenStream = &|_, _| quote! { #[derive(Debug, Clone)] };

        // Call generate_root_composition_direct
        let result = generator.generate_root_composition_direct(
            &discovery,
            &type_discovery,
            None,
            member_type_fn,
            derive_traits_fn,
            derive_traits_without_deserialize_fn,
            derive_traits_without_default_fn,
            derive_traits_without_deserialize_and_default_fn,
            false, // streaming_encode disabled for test
        );

        // Verify success
        assert!(result.is_ok());
        let root_generation = result.unwrap();
        assert_eq!(root_generation.type_name, "RootEnum");
        assert!(root_generation.is_composition);
    }
}
