//! Struct Generator - Specialized code generation for struct types
//!
//! This module handles all code generation for struct types, including:
//! - Field generation with proper naming and type handling
//! - Pattern validator support
//! - Custom serde implementations for complex field routing
//! - JsonTokenEncoder implementations for structs
//!
//! Extracted from code_generator.rs as part of module size refactoring (2025-10-02).

use crate::TctResult;
use crate::generator::encoding_decision::EncodingDecision;
use crate::generator::generation_output::TypeCodeGeneration;
use crate::generator::schema_discovery::{MemberInfo, MemberType, SchemaDiscovery, TypeDiscovery};
use crate::generator::type_analysis::{TraitDerivability, TypeAnalysis};
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use rustc_hash::FxHashMap;

mod allof_validation;
mod bitpacked;
mod default_impl;
mod partial_encoding;
mod streaming_decode;
mod struct_impl;
use allof_validation::AllOfDeserializeGenerationInput;

/// Specialized generator for struct types
///
/// This generator handles all aspects of struct generation, including:
/// - Field generation with optional/nullable handling
/// - Pattern validator detection and custom serde generation
/// - JsonTokenEncoder implementation
///
/// Uses dependency injection for shared utilities (sanitization, type generation, etc.)
/// to avoid premature extraction of code used by multiple generators.
pub struct StructGenerator<'a> {
    /// Schema discovery results, used when emitted validation snippets need root definitions.
    discovery: Option<&'a SchemaDiscovery>,
    /// Type analysis results (for boxing detection, etc.)
    analysis: &'a TypeAnalysis,
    /// Whether streaming-friendly encoding is enabled (for prefix decoding)
    streaming_encode: bool,
    /// Encoding base for multi-token encoding (default 256, configurable via max_base_vocab)
    encoding_base: u32,
}

struct StructTokenEncoderGenerationInput<'a> {
    type_ident: &'a Ident,
    type_name: &'a str,
    members: &'a [MemberInfo],
    field_name_map: &'a FxHashMap<String, String>,
    encoding_decision: Option<&'a EncodingDecision>,
    post_decode_validate: bool,
}

impl<'a> StructGenerator<'a> {
    /// Default encoding base (for backward compatibility and tests)
    pub const DEFAULT_ENCODING_BASE: u32 = 256;

    /// Create a new struct generator
    ///
    /// # Parameters
    /// - `analysis`: Type analysis results
    /// - `streaming_encode`: Whether streaming-friendly encoding is enabled
    pub fn new(analysis: &'a TypeAnalysis, streaming_encode: bool) -> Self {
        Self::with_encoding_base(analysis, streaming_encode, Self::DEFAULT_ENCODING_BASE)
    }

    /// Create a new struct generator with custom encoding base
    ///
    /// # Parameters
    /// - `analysis`: Type analysis results
    /// - `streaming_encode`: Whether streaming-friendly encoding is enabled
    /// - `encoding_base`: Base for multi-token encoding (default 256)
    pub fn with_encoding_base(
        analysis: &'a TypeAnalysis,
        streaming_encode: bool,
        encoding_base: u32,
    ) -> Self {
        Self {
            discovery: None,
            analysis,
            streaming_encode,
            encoding_base,
        }
    }

    /// Create a new struct generator with access to root schema discovery data.
    ///
    /// This is used for validation-only composition shapes that need embedded schemas
    /// with root `definitions` for local `$ref` resolution in generated crates.
    pub fn with_discovery_and_encoding_base(
        analysis: &'a TypeAnalysis,
        discovery: &'a SchemaDiscovery,
        streaming_encode: bool,
        encoding_base: u32,
    ) -> Self {
        Self {
            discovery: Some(discovery),
            analysis,
            streaming_encode,
            encoding_base,
        }
    }

    /// Generate a complete struct type with all implementations
    ///
    /// # Parameters
    ///
    /// - `type_discovery`: Schema discovery results for this type
    /// - `trait_derivability`: Trait analysis results (Eq, Ord, Hash)
    /// - `encoding_decision`: Optional encoding decision from optimization pass
    /// - `sanitize_fn`: Function to sanitize field names for Rust identifiers
    /// - `member_type_fn`: Function to generate Rust types from schema member types
    /// - `derive_traits_fn`: Function to generate derive trait attributes
    ///
    /// # Returns
    ///
    /// Complete type code generation including definition and implementations
    pub fn generate(
        &self,
        type_discovery: &TypeDiscovery,
        trait_derivability: Option<&TraitDerivability>,
        encoding_decision: Option<&crate::generator::encoding_decision::EncodingDecision>,
        sanitize_fn: &dyn Fn(&str) -> String,
        member_type_fn: &dyn Fn(&MemberType) -> TctResult<TokenStream>,
        derive_traits_fn: &dyn Fn(Option<&TraitDerivability>, bool) -> TokenStream,
    ) -> TctResult<TypeCodeGeneration> {
        self.generate_internal(
            type_discovery,
            trait_derivability,
            encoding_decision,
            sanitize_fn,
            member_type_fn,
            derive_traits_fn,
            None,
        )
    }

    /// Generate a complete struct type with zero-token type information
    ///
    /// This variant allows specifying which member types are zero-token (const enums),
    /// enabling const-first decode ordering in streaming mode.
    ///
    /// Zero-token fields are decoded FIRST in decode_prefix, making them visible
    /// even when variable-length fields are truncated.
    #[allow(clippy::too_many_arguments)]
    pub fn generate_with_zero_token_info(
        &self,
        type_discovery: &TypeDiscovery,
        trait_derivability: Option<&TraitDerivability>,
        encoding_decision: Option<&crate::generator::encoding_decision::EncodingDecision>,
        sanitize_fn: &dyn Fn(&str) -> String,
        member_type_fn: &dyn Fn(&MemberType) -> TctResult<TokenStream>,
        derive_traits_fn: &dyn Fn(Option<&TraitDerivability>, bool) -> TokenStream,
        is_zero_token_type_fn: Option<&dyn Fn(&str) -> bool>,
    ) -> TctResult<TypeCodeGeneration> {
        self.generate_internal(
            type_discovery,
            trait_derivability,
            encoding_decision,
            sanitize_fn,
            member_type_fn,
            derive_traits_fn,
            is_zero_token_type_fn,
        )
    }

    /// Internal generate implementation with optional zero-token info
    #[allow(clippy::too_many_arguments)]
    fn generate_internal(
        &self,
        type_discovery: &TypeDiscovery,
        trait_derivability: Option<&TraitDerivability>,
        encoding_decision: Option<&crate::generator::encoding_decision::EncodingDecision>,
        sanitize_fn: &dyn Fn(&str) -> String,
        member_type_fn: &dyn Fn(&MemberType) -> TctResult<TokenStream>,
        derive_traits_fn: &dyn Fn(Option<&TraitDerivability>, bool) -> TokenStream,
        is_zero_token_type_fn: Option<&dyn Fn(&str) -> bool>,
    ) -> TctResult<TypeCodeGeneration> {
        let type_name = &type_discovery.type_name;
        let type_ident = format_ident!("{}", type_name);

        // Check if this struct has pattern validators (affects serde generation)
        let has_pattern_validators = self.has_pattern_validators(type_discovery);
        let allof_validation_plan = self.allof_validation_plan(type_discovery);
        let has_allof_validation_constraints = allof_validation_plan.has_constraints();
        let deny_unknown_fields = allof_validation_plan.deny_unknown_fields;

        // Generate struct fields
        let mut fields = Vec::new();
        let mut field_types = Vec::new();
        let mut deserialize_helper_fields = Vec::new();
        let mut field_idents = Vec::new();

        // Track used field names to prevent collisions
        let mut used_field_names = std::collections::HashSet::new();
        // Map from original field name to sanitized field name
        let mut field_name_map = FxHashMap::default();

        for member in &type_discovery.members {
            let base_sanitized = sanitize_fn(&member.name);

            // Find a unique field name
            let mut final_name = base_sanitized.clone();
            let mut suffix = 2;
            while used_field_names.contains(&final_name) {
                final_name = format!("{}_{}", base_sanitized, suffix);
                suffix += 1;
            }

            used_field_names.insert(final_name.clone());
            field_name_map.insert(member.name.clone(), final_name.clone());

            let field_name = format_ident!("{}", final_name);
            let field_type = member_type_fn(&member.member_type)?;

            // Handle optional fields
            let final_field_type = if !member.required {
                if member.uses_optional_field_encoding() {
                    quote! { crate::tokenizer::OptionalField<#field_type> }
                } else {
                    quote! { Option<#field_type> }
                }
            } else {
                field_type.clone()
            };

            // Generate serde attributes (simplified for internal use)
            let mut serde_attrs = Vec::new();
            let original_name = &member.name;

            // Check if this is a pattern field (pattern_0, pattern_1, etc.) or additional_properties
            if (member.name.starts_with("pattern_")
                && member.name.chars().skip(8).all(|c| c.is_ascii_digit()))
                || member.name == "additional_properties"
            {
                // Pattern fields use custom serde implementations for proper field routing
                if member.name == "additional_properties" && !has_pattern_validators {
                    serde_attrs.push(quote! { #[serde(flatten)] });
                }
                // Pattern fields and additionalProperties (when custom serde) get no serde attributes
            } else if !has_pattern_validators {
                // Only add serde attributes if not using custom serde implementations
                if !member.required {
                    // Check if we're using OptionalField (either explicit nullable or Value type)
                    let using_optional_field = member.uses_optional_field_encoding();

                    if using_optional_field {
                        // OptionalField<T>: skip when Absent (not when Null or Present)
                        serde_attrs.push(quote! {
                            #[serde(rename = #original_name, default, skip_serializing_if = "crate::tokenizer::OptionalField::is_absent")]
                        });
                    } else {
                        // Regular optional field (Option<T>): skip when None, reject explicit null.
                        serde_attrs.push(quote! {
                            #[serde(
                                rename = #original_name,
                                default,
                                skip_serializing_if = "Option::is_none",
                                deserialize_with = "crate::tokenizer::deserialize_option_reject_null"
                            )]
                        });
                    }
                } else {
                    // Required field: always add rename for consistency
                    serde_attrs.push(quote! {
                        #[serde(rename = #original_name)]
                    });
                }
            }

            fields.push(quote! {
                #(#serde_attrs)*
                pub #field_name: #final_field_type
            });
            let deserialize_attrs = Self::serde_deserialize_attrs(member, has_pattern_validators);
            deserialize_helper_fields.push(quote! {
                #(#deserialize_attrs)*
                #field_name: #final_field_type
            });
            field_idents.push(field_name);
            field_types.push(final_field_type);
        }

        // Generate derives
        let derive_attrs = if has_allof_validation_constraints && !has_pattern_validators {
            self.generate_derive_traits_without_deserialize(trait_derivability)
        } else {
            derive_traits_fn(trait_derivability, has_pattern_validators)
        };

        // A required `{"type": "number"}` leaf becomes serde_json::Number, which has
        // no Default impl, so `#[derive(Default)]` would not compile. Drop Default
        // from the derive list and hand-write the impl, mirroring what the enum
        // generator already does for data-carrying first variants.
        let (derive_attrs, owes_manual_default) =
            if default_impl::needs_manual_default(&type_discovery.members) {
                default_impl::strip_default_from_derive(&derive_attrs)
            } else {
                (derive_attrs, false)
            };

        // Definition tokens
        let serde_type_attrs = if deny_unknown_fields
            && !has_pattern_validators
            && !has_allof_validation_constraints
        {
            quote! { #[serde(deny_unknown_fields)] }
        } else {
            quote! {}
        };

        let definition_tokens = quote! {
            #derive_attrs
            #serde_type_attrs
            pub struct #type_ident {
                #(#fields),*
            }
        };

        // Generate JsonTokenEncoder implementation
        let mut implementation_tokens = self.generate_struct_token_encoder_impl(
            StructTokenEncoderGenerationInput {
                type_ident: &type_ident,
                type_name,
                members: &type_discovery.members,
                field_name_map: &field_name_map,
                encoding_decision,
                post_decode_validate: has_allof_validation_constraints,
            },
            member_type_fn,
        )?;

        // Generate custom serde implementations for structs with pattern validators
        if has_pattern_validators {
            let custom_serde = self.generate_custom_serde_for_pattern_struct(
                &type_ident,
                type_discovery,
                &field_name_map,
                member_type_fn,
            )?;
            implementation_tokens = quote! {
                #implementation_tokens
                #custom_serde
            };
        } else if has_allof_validation_constraints {
            let custom_deserialize = Self::generate_custom_deserialize_for_allof_validation(
                AllOfDeserializeGenerationInput {
                    type_ident: &type_ident,
                    validation_plan: &allof_validation_plan,
                    deserialize_helper_fields: &deserialize_helper_fields,
                    field_idents: &field_idents,
                },
            );
            implementation_tokens = quote! {
                #implementation_tokens
                #custom_deserialize
            };
        }

        // Generate partial encoding methods
        let partial_encoding_methods = self.generate_partial_encoding_methods(
            &type_ident,
            &type_discovery.members,
            &field_name_map,
        );
        implementation_tokens = quote! {
            #implementation_tokens
            #partial_encoding_methods
        };

        // Generate streaming decode_prefix method (only when streaming mode enabled)
        if self.streaming_encode {
            let streaming_methods = self.generate_streaming_decode_prefix_with_zero_token_info(
                &type_ident,
                type_name,
                &type_discovery.members,
                &field_name_map,
                encoding_decision,
                member_type_fn,
                is_zero_token_type_fn,
            )?;
            implementation_tokens = quote! {
                #implementation_tokens
                #streaming_methods
            };
        }

        // Replace the derive we stripped above.
        if owes_manual_default {
            let manual_default = default_impl::manual_default_impl(
                &type_ident,
                &type_discovery.members,
                &field_name_map,
            );
            implementation_tokens = quote! {
                #implementation_tokens
                #manual_default
            };
        }

        // Create type code generation
        let mut type_generation = TypeCodeGeneration::new_struct(
            type_name.clone(),
            definition_tokens,
            implementation_tokens,
        )
        .with_token_encoder_impl(true);

        // Add derived traits info
        if let Some(derivability) = trait_derivability {
            if derivability.can_derive_eq {
                type_generation.add_derived_trait("Eq".to_string());
            }
            if derivability.can_derive_ord {
                type_generation.add_derived_trait("Ord".to_string());
            }
            if derivability.can_derive_hash {
                type_generation.add_derived_trait("Hash".to_string());
            }
        }

        // Check if this type uses boxing
        if self.analysis.needs_boxing(type_name) {
            type_generation.set_uses_boxing(true);
        }

        Ok(type_generation)
    }

    /// Generate default JSON value for a field based on its type
    ///
    /// Returns a TokenStream that produces a serde_json::Value default
    fn generate_default_value_for_field(&self, member: &MemberInfo) -> TokenStream {
        // Handle optional fields - they default to null
        if !member.required {
            return quote! { serde_json::Value::Null };
        }

        // For required fields, generate type-specific defaults
        match &member.member_type {
            MemberType::Primitive(rust_type) => match rust_type.as_str() {
                "String" => quote! { serde_json::Value::String(String::new()) },
                "i32" | "i64" => quote! { serde_json::Value::Number(serde_json::Number::from(0)) },
                "JsonInteger" => {
                    quote! { serde_json::Value::Number(serde_json::Number::from(0)) }
                }
                "f64" => quote! {
                    serde_json::Value::Number(
                        serde_json::Number::from_f64(0.0).unwrap_or(serde_json::Number::from(0))
                    )
                },
                "bool" => quote! { serde_json::Value::Bool(false) },
                "serde_json::Value" => quote! { serde_json::Value::Null },
                "serde_json::Number" => {
                    quote! { serde_json::Value::Number(serde_json::Number::from(0)) }
                }
                _ => quote! { serde_json::Value::Null }, // Fallback
            },
            MemberType::Container { container, .. } => {
                if container == "Vec" {
                    quote! { serde_json::Value::Array(Vec::new()) }
                } else {
                    quote! { serde_json::Value::Null }
                }
            }
            MemberType::ComplexContainer { container, .. } => {
                if container == "Vec" {
                    quote! { serde_json::Value::Array(Vec::new()) }
                } else {
                    quote! { serde_json::Value::Null }
                }
            }
            MemberType::HashMap { .. } => {
                quote! { serde_json::Value::Object(serde_json::Map::new()) }
            }
            MemberType::TypeReference(_) => {
                // For complex types, use an empty object as default
                quote! { serde_json::Value::Object(serde_json::Map::new()) }
            }
            MemberType::PatternProperties { .. } => {
                quote! { serde_json::Value::Object(serde_json::Map::new()) }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generator::schema_discovery::{
        MemberInfo, MemberType, SchemaLocation, TypeDiscovery, TypeKind,
    };
    use crate::generator::type_analysis::{DecodeStrategy, TypeAnalysis};
    use quote::ToTokens;
    use serde_json::json;
    use syn::punctuated::Punctuated;

    fn parse_generated_file(tokens: TokenStream) -> syn::File {
        match syn::parse2(tokens) {
            Ok(file) => file,
            Err(error) => panic!("generated struct tokens should parse as Rust: {error}"),
        }
    }

    fn parse_generated_expr(tokens: TokenStream) -> syn::Expr {
        match syn::parse2(tokens) {
            Ok(expr) => expr,
            Err(error) => panic!("generated expression should parse as Rust: {error}"),
        }
    }

    fn generated_struct<'a>(file: &'a syn::File, name: &str) -> &'a syn::ItemStruct {
        file.items
            .iter()
            .find_map(|item| match item {
                syn::Item::Struct(item_struct) if item_struct.ident == name => Some(item_struct),
                _ => None,
            })
            .unwrap_or_else(|| panic!("expected generated struct {name}"))
    }

    fn impl_self_type_matches(item_impl: &syn::ItemImpl, type_name: &str) -> bool {
        item_impl.self_ty.to_token_stream().to_string() == type_name
    }

    fn trait_impl_for_type<'a>(
        file: &'a syn::File,
        type_name: &str,
        trait_name: &str,
    ) -> Option<&'a syn::ItemImpl> {
        file.items.iter().find_map(|item| match item {
            syn::Item::Impl(item_impl)
                if impl_self_type_matches(item_impl, type_name)
                    && item_impl.trait_.as_ref().is_some_and(|(_, path, _)| {
                        path.segments
                            .last()
                            .is_some_and(|segment| segment.ident == trait_name)
                    }) =>
            {
                Some(item_impl)
            }
            _ => None,
        })
    }

    fn generated_trait_method<'a>(
        file: &'a syn::File,
        type_name: &str,
        trait_name: &str,
        method_name: &str,
    ) -> &'a syn::ImplItemFn {
        trait_impl_for_type(file, type_name, trait_name)
            .and_then(|item_impl| {
                item_impl.items.iter().find_map(|item| match item {
                    syn::ImplItem::Fn(method) if method.sig.ident == method_name => Some(method),
                    _ => None,
                })
            })
            .unwrap_or_else(|| {
                panic!("expected generated {trait_name}::{method_name} impl for {type_name}")
            })
    }

    fn generated_inherent_method<'a>(
        file: &'a syn::File,
        type_name: &str,
        method_name: &str,
    ) -> Option<&'a syn::ImplItemFn> {
        file.items.iter().find_map(|item| match item {
            syn::Item::Impl(item_impl)
                if item_impl.trait_.is_none() && impl_self_type_matches(item_impl, type_name) =>
            {
                item_impl.items.iter().find_map(|item| match item {
                    syn::ImplItem::Fn(method) if method.sig.ident == method_name => Some(method),
                    _ => None,
                })
            }
            _ => None,
        })
    }

    fn derive_traits(attrs: &[syn::Attribute]) -> Vec<String> {
        attrs
            .iter()
            .filter(|attr| attr.path().is_ident("derive"))
            .filter_map(|attr| {
                attr.parse_args_with(Punctuated::<syn::Path, syn::Token![,]>::parse_terminated)
                    .ok()
            })
            .flat_map(|paths| paths.into_iter())
            .map(|path| {
                path.segments
                    .iter()
                    .map(|segment| segment.ident.to_string())
                    .collect::<Vec<_>>()
                    .join("::")
            })
            .collect()
    }

    fn has_derive_trait(derives: &[String], expected: &str) -> bool {
        derives.iter().any(|derive| derive == expected)
    }

    fn generated_expr_tokens_contain(expr: &syn::Expr, expected: &str) -> bool {
        expr.to_token_stream().to_string().contains(expected)
    }

    // Helper: Create test TypeAnalysis
    fn create_test_analysis() -> TypeAnalysis {
        TypeAnalysis::default()
    }

    // Helper: Create a simple struct TypeDiscovery
    fn create_simple_struct() -> TypeDiscovery {
        let schema_location = SchemaLocation {
            json_path: "#/definitions/User".to_string(),
            description: "User struct".to_string(),
            parent_type: None,
            property_name: None,
        };

        let mut schema_obj = serde_json::Map::new();
        schema_obj.insert("type".to_string(), json!("object"));
        schema_obj.insert(
            "properties".to_string(),
            json!({
                "name": {"type": "string"},
                "age": {"type": "integer"}
            }),
        );
        schema_obj.insert("required".to_string(), json!(["name"]));

        let mut type_discovery = TypeDiscovery::new(
            "User".to_string(),
            TypeKind::Struct,
            schema_location,
            schema_obj,
        );

        // Add required field
        let member1 = MemberInfo {
            name: "name".to_string(),
            member_type: MemberType::Primitive("String".to_string()),
            required: true,
            is_nullable: false,
            schema: json!({"type": "string"}).as_object().unwrap().clone(),
        };

        // Add optional field
        let member2 = MemberInfo {
            name: "age".to_string(),
            member_type: MemberType::Primitive("i32".to_string()),
            required: false,
            is_nullable: false,
            schema: json!({"type": "integer"}).as_object().unwrap().clone(),
        };

        type_discovery.add_member(member1);
        type_discovery.add_member(member2);

        type_discovery
    }

    // Helper: Create a struct with pattern validators
    fn create_pattern_struct() -> TypeDiscovery {
        let schema_location = SchemaLocation {
            json_path: "#/definitions/Config".to_string(),
            description: "Config with patterns".to_string(),
            parent_type: None,
            property_name: None,
        };

        let mut schema_obj = serde_json::Map::new();
        schema_obj.insert("type".to_string(), json!("object"));
        schema_obj.insert(
            "patternProperties".to_string(),
            json!({
                "^env_": {"type": "string"}
            }),
        );

        let mut type_discovery = TypeDiscovery::new(
            "Config".to_string(),
            TypeKind::Struct,
            schema_location,
            schema_obj,
        );

        // Add pattern field (pattern_0)
        let pattern_member = MemberInfo {
            name: "pattern_0".to_string(),
            member_type: MemberType::TypeReference("PatternValidator0".to_string()),
            required: false,
            is_nullable: false,
            schema: json!({"type": "string"}).as_object().unwrap().clone(),
        };

        type_discovery.add_member(pattern_member);

        type_discovery
    }

    // ========================================================================================
    // Test Group 1: Pattern Validator Detection (2 tests)
    // ========================================================================================

    #[test]
    fn test_has_pattern_validators_true() {
        let analysis = create_test_analysis();
        let generator = StructGenerator::new(&analysis, false);

        let pattern_struct = create_pattern_struct();
        let result = generator.has_pattern_validators(&pattern_struct);

        assert!(result, "Should detect pattern validators");
    }

    #[test]
    fn test_has_pattern_validators_false() {
        let analysis = create_test_analysis();
        let generator = StructGenerator::new(&analysis, false);

        let simple_struct = create_simple_struct();
        let result = generator.has_pattern_validators(&simple_struct);

        assert!(
            !result,
            "Should not detect pattern validators in simple struct"
        );
    }

    #[test]
    fn test_streaming_optional_vec_json_integer_uses_qualified_inner_type() {
        let analysis = create_test_analysis();
        let generator = StructGenerator::new(&analysis, true);

        let mut type_discovery = create_simple_struct();
        type_discovery.members.clear();
        type_discovery.add_member(MemberInfo {
            name: "values".to_string(),
            member_type: MemberType::Container {
                container: "Vec".to_string(),
                inner: "JsonInteger".to_string(),
            },
            required: false,
            is_nullable: false,
            schema: json!({"type": "array", "items": {"type": "integer"}})
                .as_object()
                .unwrap()
                .clone(),
        });

        let sanitize_fn = |name: &str| name.to_string();
        let member_type_fn = |member_type: &MemberType| -> TctResult<TokenStream> {
            Ok(match member_type {
                MemberType::Container { container, inner } => {
                    let container_ident = format_ident!("{}", container);
                    let inner_tokens = StructGenerator::container_inner_type_tokens(inner);
                    quote! { #container_ident<#inner_tokens> }
                }
                _ => quote! { () },
            })
        };
        let derive_traits_fn = |_: Option<&TraitDerivability>, _: bool| {
            quote! { #[derive(Debug, Default)] }
        };

        let encoding_decision =
            crate::generator::encoding_decision::EncodingDecision::BitPackedStruct {
                optional_field_count: 1,
                presence_tokens_needed: 1,
            };
        let result = generator
            .generate(
                &type_discovery,
                None,
                Some(&encoding_decision),
                &sanitize_fn,
                &member_type_fn,
                &derive_traits_fn,
            )
            .expect("streaming struct generation should succeed");

        let definition = result.definition_tokens.to_string();
        let code = result.implementation_tokens.to_string();
        assert!(
            definition.contains("Option < Vec < crate :: tokenizer :: JsonInteger > >"),
            "field type should use the tokenizer-qualified JsonInteger: {definition}"
        );
        assert!(
            code.contains("decode_vec_streaming :: < crate :: tokenizer :: JsonInteger >"),
            "streaming Vec decode should use the tokenizer-qualified JsonInteger: {code}"
        );
        assert!(
            !code.contains("decode_vec_streaming :: < JsonInteger >"),
            "streaming Vec decode must not rely on an unqualified JsonInteger: {code}"
        );
    }

    /// Regression: a required `{"type": "number"}` leaf is rendered as
    /// `serde_json::Number`, which has no `Default` impl. Deriving `Default` on the
    /// containing struct produced E0277 for every such struct. The struct must instead drop `Default`
    /// from the derive list and carry a hand-written impl, as enums already do.
    #[test]
    fn test_required_number_member_gets_manual_default_impl() {
        let analysis = create_test_analysis();
        let generator = StructGenerator::new(&analysis, true);

        let mut type_discovery = create_simple_struct();
        type_discovery.members.clear();
        type_discovery.add_member(MemberInfo {
            name: "amount".to_string(),
            member_type: MemberType::Primitive("serde_json::Number".to_string()),
            required: true,
            is_nullable: false,
            schema: json!({"type": "number"}).as_object().unwrap().clone(),
        });

        let sanitize_fn = |name: &str| name.to_string();
        let member_type_fn = |member_type: &MemberType| -> TctResult<TokenStream> {
            Ok(match member_type {
                MemberType::Primitive(rust_type) => syn::parse_str::<syn::Type>(rust_type)
                    .map(|t| quote! { #t })
                    .unwrap(),
                _ => quote! { () },
            })
        };
        let derive_traits_fn = |_: Option<&TraitDerivability>, _: bool| {
            quote! { #[derive(Debug, PartialEq, Clone, Default, serde::Serialize)] }
        };

        let result = generator
            .generate(
                &type_discovery,
                None,
                None,
                &sanitize_fn,
                &member_type_fn,
                &derive_traits_fn,
            )
            .expect("number-member struct generation should succeed");

        let definition = result.definition_tokens.to_string();
        let implementation = result.implementation_tokens.to_string();

        assert!(
            !definition.contains("Default"),
            "serde_json::Number does not implement Default, so it must not be derived: {definition}"
        );
        assert!(
            definition.contains("Debug") && definition.contains("serde :: Serialize"),
            "the other derives must survive the rewrite: {definition}"
        );
        assert!(
            implementation.contains("impl Default for User"),
            "a manual Default impl must replace the derive: {implementation}"
        );
        assert!(
            implementation.contains("amount : serde_json :: Number :: from (0)"),
            "the manual impl must construct the Number explicitly: {implementation}"
        );
    }

    /// The rewrite is conditional: structs without a bare number leaf keep the
    /// derive and must NOT gain a conflicting hand-written impl.
    #[test]
    fn test_struct_without_number_member_keeps_derived_default() {
        let analysis = create_test_analysis();
        let generator = StructGenerator::new(&analysis, true);

        let type_discovery = create_simple_struct();
        let sanitize_fn = |name: &str| name.to_string();
        let member_type_fn = |_: &MemberType| -> TctResult<TokenStream> { Ok(quote! { String }) };
        let derive_traits_fn = |_: Option<&TraitDerivability>, _: bool| {
            quote! { #[derive(Debug, Default, serde::Serialize)] }
        };

        let result = generator
            .generate(
                &type_discovery,
                None,
                None,
                &sanitize_fn,
                &member_type_fn,
                &derive_traits_fn,
            )
            .expect("plain struct generation should succeed");

        assert!(
            result.definition_tokens.to_string().contains("Default"),
            "ordinary structs keep #[derive(Default)]"
        );
        assert!(
            !result
                .implementation_tokens
                .to_string()
                .contains("impl Default for"),
            "a derived Default must not be duplicated by a manual impl"
        );
    }

    // ========================================================================================
    // Test Group 2: Field Name Collision Handling (2 tests)
    // ========================================================================================

    #[test]
    fn test_field_name_collision_resolution() {
        let analysis = create_test_analysis();
        let generator = StructGenerator::new(&analysis, false);

        // Create struct with potential name collision
        let schema_location = SchemaLocation {
            json_path: "#/definitions/Collision".to_string(),
            description: "Test collision".to_string(),
            parent_type: None,
            property_name: None,
        };

        let mut type_discovery = TypeDiscovery::new(
            "Collision".to_string(),
            TypeKind::Struct,
            schema_location,
            serde_json::Map::new(),
        );

        // Add fields that would collide after sanitization
        let member1 = MemberInfo {
            name: "type".to_string(), // Becomes "type_" (keyword)
            member_type: MemberType::Primitive("String".to_string()),
            required: true,
            is_nullable: false,
            schema: json!({"type": "string"}).as_object().unwrap().clone(),
        };

        let member2 = MemberInfo {
            name: "Type".to_string(), // Also becomes "type_" after lowercase
            member_type: MemberType::Primitive("String".to_string()),
            required: true,
            is_nullable: false,
            schema: json!({"type": "string"}).as_object().unwrap().clone(),
        };

        type_discovery.add_member(member1);
        type_discovery.add_member(member2);

        // Mock functions
        let sanitize_fn = |name: &str| {
            if name == "type" || name == "Type" {
                "type_".to_string()
            } else {
                name.to_lowercase()
            }
        };

        let member_type_fn = |mt: &MemberType| -> TctResult<TokenStream> {
            match mt {
                MemberType::Primitive(t) => {
                    let ident = format_ident!("{}", t);
                    Ok(quote::quote! { #ident })
                }
                _ => Ok(quote::quote! { String }),
            }
        };

        let derive_traits_fn = |_: Option<&TraitDerivability>, _: bool| {
            quote::quote! { #[derive(Debug)] }
        };

        let result = generator.generate(
            &type_discovery,
            None,
            None,
            &sanitize_fn,
            &member_type_fn,
            &derive_traits_fn,
        );

        assert!(result.is_ok(), "Should handle field name collisions");
        // Verify no duplicate field names in generated code
        let tokens = result.unwrap().definition_tokens.to_string();
        // Second field should get suffix: type__2
        assert!(
            tokens.contains("type_"),
            "Should contain sanitized base field"
        );
    }

    #[test]
    fn test_pattern_field_naming() {
        let analysis = create_test_analysis();
        let generator = StructGenerator::new(&analysis, false);

        let schema_location = SchemaLocation {
            json_path: "#/definitions/PatternTest".to_string(),
            description: "Pattern naming test".to_string(),
            parent_type: None,
            property_name: None,
        };

        let mut type_discovery = TypeDiscovery::new(
            "PatternTest".to_string(),
            TypeKind::Struct,
            schema_location,
            serde_json::Map::new(),
        );

        // Add multiple pattern fields
        for i in 0..3 {
            let member = MemberInfo {
                name: format!("pattern_{}", i),
                member_type: MemberType::TypeReference(format!("PatternValidator{}", i)),
                required: false,
                is_nullable: false,
                schema: json!({"type": "string"}).as_object().unwrap().clone(),
            };
            type_discovery.add_member(member);
        }

        let has_patterns = generator.has_pattern_validators(&type_discovery);
        assert!(has_patterns, "Should detect multiple pattern validators");
    }

    // ========================================================================================
    // Test Group 3: Optional Field Type Handling (3 tests)
    // ========================================================================================

    #[test]
    fn test_optional_field_with_nullable() {
        let analysis = create_test_analysis();
        let generator = StructGenerator::new(&analysis, false);

        let schema_location = SchemaLocation {
            json_path: "#/definitions/NullableTest".to_string(),
            description: "Nullable field test".to_string(),
            parent_type: None,
            property_name: None,
        };

        let mut type_discovery = TypeDiscovery::new(
            "NullableTest".to_string(),
            TypeKind::Struct,
            schema_location,
            serde_json::Map::new(),
        );

        // Add nullable optional field (should use OptionalField)
        let member = MemberInfo {
            name: "value".to_string(),
            member_type: MemberType::Primitive("String".to_string()),
            required: false,
            is_nullable: true, // Explicitly allows null
            schema: json!({"type": ["string", "null"]})
                .as_object()
                .unwrap()
                .clone(),
        };

        type_discovery.add_member(member);

        let sanitize_fn = |name: &str| name.to_string();
        let member_type_fn = |mt: &MemberType| -> TctResult<TokenStream> {
            match mt {
                MemberType::Primitive(t) => {
                    let ident = format_ident!("{}", t);
                    Ok(quote::quote! { #ident })
                }
                _ => Ok(quote::quote! { String }),
            }
        };
        let derive_traits_fn = |_: Option<&TraitDerivability>, _: bool| {
            quote::quote! { #[derive(Debug)] }
        };

        let result = generator.generate(
            &type_discovery,
            None,
            None,
            &sanitize_fn,
            &member_type_fn,
            &derive_traits_fn,
        );

        assert!(result.is_ok());
        let tokens = result.unwrap().definition_tokens.to_string();
        // Should use OptionalField for nullable optional field
        assert!(
            tokens.contains("OptionalField"),
            "Nullable optional should use OptionalField<T>"
        );
    }

    #[test]
    fn test_optional_field_without_nullable() {
        let analysis = create_test_analysis();
        let generator = StructGenerator::new(&analysis, false);

        let simple_struct = create_simple_struct();

        let sanitize_fn = |name: &str| name.to_string();
        let member_type_fn = |mt: &MemberType| -> TctResult<TokenStream> {
            match mt {
                MemberType::Primitive(t) => {
                    let ident = format_ident!("{}", t);
                    Ok(quote::quote! { #ident })
                }
                _ => Ok(quote::quote! { String }),
            }
        };
        let derive_traits_fn = |_: Option<&TraitDerivability>, _: bool| {
            quote::quote! { #[derive(Debug)] }
        };

        let result = generator.generate(
            &simple_struct,
            None,
            None,
            &sanitize_fn,
            &member_type_fn,
            &derive_traits_fn,
        );

        assert!(result.is_ok());
        let tokens = result.unwrap().definition_tokens.to_string();
        // Optional non-nullable field should use Option<T>
        assert!(
            tokens.contains("Option"),
            "Non-nullable optional should use Option<T>"
        );
    }

    #[test]
    fn test_optional_value_field() {
        let analysis = create_test_analysis();
        let generator = StructGenerator::new(&analysis, false);

        let schema_location = SchemaLocation {
            json_path: "#/definitions/ValueTest".to_string(),
            description: "Value field test".to_string(),
            parent_type: None,
            property_name: None,
        };

        let mut type_discovery = TypeDiscovery::new(
            "ValueTest".to_string(),
            TypeKind::Struct,
            schema_location,
            serde_json::Map::new(),
        );

        // Add optional serde_json::Value field (should use OptionalField to preserve null vs absent)
        let member = MemberInfo {
            name: "data".to_string(),
            member_type: MemberType::Primitive("serde_json::Value".to_string()),
            required: false,
            is_nullable: false,
            schema: serde_json::Map::new(),
        };

        type_discovery.add_member(member);

        let sanitize_fn = |name: &str| name.to_string();
        let member_type_fn = |mt: &MemberType| -> TctResult<TokenStream> {
            match mt {
                MemberType::Primitive(t) if t == "serde_json::Value" => {
                    Ok(quote::quote! { serde_json::Value })
                }
                MemberType::Primitive(t) => {
                    let ident = format_ident!("{}", t);
                    Ok(quote::quote! { #ident })
                }
                _ => Ok(quote::quote! { String }),
            }
        };
        let derive_traits_fn = |_: Option<&TraitDerivability>, _: bool| {
            quote::quote! { #[derive(Debug)] }
        };

        let result = generator.generate(
            &type_discovery,
            None,
            None,
            &sanitize_fn,
            &member_type_fn,
            &derive_traits_fn,
        );

        assert!(result.is_ok());
        let tokens = result.unwrap().definition_tokens.to_string();
        // Optional Value field should use OptionalField to preserve null vs absent distinction
        assert!(
            tokens.contains("OptionalField"),
            "Optional Value field should use OptionalField<T>"
        );
    }

    #[test]
    fn test_optional_value_field_uses_member_metadata_not_rust_token_text() {
        let analysis = create_test_analysis();
        let generator = StructGenerator::new(&analysis, false);

        let schema_location = SchemaLocation {
            json_path: "#/definitions/ValueAliasTest".to_string(),
            description: "Value alias field test".to_string(),
            parent_type: None,
            property_name: None,
        };

        let mut type_discovery = TypeDiscovery::new(
            "ValueAliasTest".to_string(),
            TypeKind::Struct,
            schema_location,
            serde_json::Map::new(),
        );

        type_discovery.add_member(MemberInfo {
            name: "data".to_string(),
            member_type: MemberType::Primitive("serde_json::Value".to_string()),
            required: false,
            is_nullable: false,
            schema: serde_json::Map::new(),
        });

        let sanitize_fn = |name: &str| name.to_string();
        let member_type_fn = |mt: &MemberType| -> TctResult<TokenStream> {
            match mt {
                MemberType::Primitive(t) if t == "serde_json::Value" => {
                    Ok(quote::quote! { JsonValueAlias })
                }
                _ => Ok(quote::quote! { String }),
            }
        };
        let derive_traits_fn = |_: Option<&TraitDerivability>, _: bool| {
            quote::quote! { #[derive(Debug)] }
        };

        let result = generator.generate(
            &type_discovery,
            None,
            None,
            &sanitize_fn,
            &member_type_fn,
            &derive_traits_fn,
        );

        assert!(result.is_ok());
        let tokens = result.unwrap().definition_tokens.to_string();
        assert!(
            tokens.contains("OptionalField < JsonValueAlias >"),
            "MemberType metadata should select OptionalField even when emitted tokens use an alias"
        );
    }

    // ========================================================================================
    // Test Group 4: Custom Serde Generation (2 tests)
    // ========================================================================================

    #[test]
    fn test_custom_serde_for_pattern_struct() {
        let analysis = create_test_analysis();
        let generator = StructGenerator::new(&analysis, false);

        let pattern_struct = create_pattern_struct();

        let sanitize_fn = |name: &str| name.to_string();
        let member_type_fn = |mt: &MemberType| -> TctResult<TokenStream> {
            match mt {
                MemberType::TypeReference(name) => {
                    let ident = format_ident!("{}", name);
                    Ok(quote::quote! { #ident })
                }
                MemberType::Primitive(t) => {
                    let ident = format_ident!("{}", t);
                    Ok(quote::quote! { #ident })
                }
                _ => Ok(quote::quote! { String }),
            }
        };
        let derive_traits_fn = |_: Option<&TraitDerivability>, has_pattern: bool| {
            if has_pattern {
                // Pattern structs don't derive Serialize/Deserialize (custom impls)
                quote::quote! { #[derive(Debug, Clone)] }
            } else {
                quote::quote! { #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)] }
            }
        };

        let result = generator.generate(
            &pattern_struct,
            None,
            None,
            &sanitize_fn,
            &member_type_fn,
            &derive_traits_fn,
        );

        assert!(result.is_ok());
        let implementation_file = parse_generated_file(result.unwrap().implementation_tokens);

        // Should contain custom Serialize and Deserialize implementations
        assert!(
            trait_impl_for_type(&implementation_file, "Config", "Serialize").is_some(),
            "Should have custom Serialize impl"
        );
        assert!(
            trait_impl_for_type(&implementation_file, "Config", "Deserialize").is_some(),
            "Should have custom Deserialize impl"
        );
    }

    #[test]
    fn test_no_custom_serde_for_simple_struct() {
        let analysis = create_test_analysis();
        let generator = StructGenerator::new(&analysis, false);

        let simple_struct = create_simple_struct();

        let sanitize_fn = |name: &str| name.to_string();
        let member_type_fn = |mt: &MemberType| -> TctResult<TokenStream> {
            match mt {
                MemberType::Primitive(t) => {
                    let ident = format_ident!("{}", t);
                    Ok(quote::quote! { #ident })
                }
                _ => Ok(quote::quote! { String }),
            }
        };
        let derive_traits_fn = |_: Option<&TraitDerivability>, has_pattern: bool| {
            if has_pattern {
                quote::quote! { #[derive(Debug, Clone)] }
            } else {
                quote::quote! { #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)] }
            }
        };

        let result = generator.generate(
            &simple_struct,
            None,
            None,
            &sanitize_fn,
            &member_type_fn,
            &derive_traits_fn,
        );

        assert!(result.is_ok());
        let definition_file = parse_generated_file(result.unwrap().definition_tokens);
        let user_struct = generated_struct(&definition_file, "User");
        let derives = derive_traits(&user_struct.attrs);

        // Should use derived Serialize/Deserialize (not custom)
        assert!(
            has_derive_trait(&derives, "serde::Serialize"),
            "Should derive Serialize"
        );
        assert!(
            has_derive_trait(&derives, "serde::Deserialize"),
            "Should derive Deserialize"
        );
    }

    // ========================================================================================
    // Test Group 5: Token Encoder Implementation (1 test)
    // ========================================================================================

    #[test]
    fn test_token_encoder_impl_structure() {
        let analysis = create_test_analysis();
        let generator = StructGenerator::new(&analysis, false);

        let simple_struct = create_simple_struct();

        let sanitize_fn = |name: &str| name.to_string();
        let member_type_fn = |mt: &MemberType| -> TctResult<TokenStream> {
            match mt {
                MemberType::Primitive(t) => {
                    let ident = format_ident!("{}", t);
                    Ok(quote::quote! { #ident })
                }
                _ => Ok(quote::quote! { String }),
            }
        };
        let derive_traits_fn = |_: Option<&TraitDerivability>, _: bool| {
            quote::quote! { #[derive(Debug)] }
        };

        let result = generator.generate(
            &simple_struct,
            None,
            None,
            &sanitize_fn,
            &member_type_fn,
            &derive_traits_fn,
        );

        assert!(result.is_ok());
        let implementation_file = parse_generated_file(result.unwrap().implementation_tokens);

        // Should contain JsonTokenEncoder implementation with all required methods
        generated_trait_method(&implementation_file, "User", "JsonTokenEncoder", "encode");
        generated_trait_method(&implementation_file, "User", "JsonTokenEncoder", "decode");
        generated_trait_method(
            &implementation_file,
            "User",
            "JsonTokenEncoder",
            "token_count",
        );
    }

    #[test]
    fn test_token_encoder_rejects_decode_field_order_that_differs_from_encode_order() {
        let mut analysis = create_test_analysis();
        analysis.decode_strategies.insert(
            "User".to_string(),
            DecodeStrategy::object().with_field_order(vec!["age".to_string(), "name".to_string()]),
        );
        let generator = StructGenerator::new(&analysis, false);

        let simple_struct = create_simple_struct();

        let sanitize_fn = |name: &str| name.to_string();
        let member_type_fn = |mt: &MemberType| -> TctResult<TokenStream> {
            match mt {
                MemberType::Primitive(t) => {
                    let ident = format_ident!("{}", t);
                    Ok(quote::quote! { #ident })
                }
                _ => Ok(quote::quote! { String }),
            }
        };
        let derive_traits_fn = |_: Option<&TraitDerivability>, _: bool| {
            quote::quote! { #[derive(Debug)] }
        };

        let result = generator.generate(
            &simple_struct,
            None,
            None,
            &sanitize_fn,
            &member_type_fn,
            &derive_traits_fn,
        );

        let Err(error) = result else {
            panic!("expected divergent field_order to fail code generation");
        };

        match error {
            crate::error::TctError::CodeGeneration {
                message,
                suggestion,
                phase,
                ..
            } => {
                assert!(matches!(
                    phase,
                    crate::error::GenerationPhase::TypeGeneration
                ));
                assert!(message.contains("decode field_order for User diverges"));
                assert!(message.contains("encode=[\"name\", \"age\"]"));
                assert!(message.contains("decode=[\"age\", \"name\"]"));
                assert!(suggestion.contains("schema-evolution compatibility"));
            }
            other => panic!("expected CodeGeneration error, got {other:?}"),
        }
    }

    // ========================================================================================
    // Test Group 6: Mode 2 Partial Encoding Methods (5 tests)
    // ========================================================================================

    #[test]
    fn test_partial_encoding_methods_exist() {
        // Test: All three Mode 2 methods should be generated
        let analysis = create_test_analysis();
        let generator = StructGenerator::new(&analysis, false);

        let simple_struct = create_simple_struct();

        let sanitize_fn = |name: &str| name.to_string();
        let member_type_fn = |mt: &MemberType| -> TctResult<TokenStream> {
            match mt {
                MemberType::Primitive(t) => {
                    let ident = format_ident!("{}", t);
                    Ok(quote::quote! { #ident })
                }
                _ => Ok(quote::quote! { String }),
            }
        };
        let derive_traits_fn = |_: Option<&TraitDerivability>, _: bool| {
            quote::quote! { #[derive(Debug)] }
        };

        let result = generator.generate(
            &simple_struct,
            None,
            None,
            &sanitize_fn,
            &member_type_fn,
            &derive_traits_fn,
        );

        assert!(result.is_ok());
        let impl_tokens = result.unwrap().implementation_tokens.to_string();

        // All three Mode 2 methods must exist
        assert!(
            impl_tokens.contains("fn from_json_partial"),
            "Should have from_json_partial method"
        );
        assert!(
            impl_tokens.contains("fn encode_partial"),
            "Should have encode_partial method"
        );
        assert!(
            impl_tokens.contains("fn decode_to_first_gap"),
            "Should have decode_to_first_gap method"
        );
    }

    #[test]
    fn test_from_json_partial_signature() {
        // Test: from_json_partial returns (Self, Vec<bool>)
        let analysis = create_test_analysis();
        let generator = StructGenerator::new(&analysis, false);

        let simple_struct = create_simple_struct();
        let field_name_map = FxHashMap::default();

        let partial_methods = generator.generate_partial_encoding_methods(
            &format_ident!("User"),
            &simple_struct.members,
            &field_name_map,
        );
        let code_str = partial_methods.to_string();

        // Check signature
        assert!(
            code_str.contains(
                "fn from_json_partial (json_str : & str) -> Result < (Self , Vec < bool >) ,"
            ),
            "from_json_partial should return Result<(Self, Vec<bool>), Error>"
        );

        // Check that it tracks field presence
        assert!(
            code_str.contains("let mut presence = vec ! [false"),
            "Should initialize presence vector"
        );
        assert!(
            code_str.contains("presence [i] = true"),
            "Should track present fields"
        );
    }

    #[test]
    fn empty_struct_partial_helpers_type_empty_vectors_explicitly() {
        let analysis = create_test_analysis();
        let generator = StructGenerator::new(&analysis, false);
        let field_name_map = FxHashMap::default();
        let partial_methods = generator.generate_partial_encoding_methods(
            &format_ident!("NoArguments"),
            &[],
            &field_name_map,
        );
        let code = partial_methods.to_string();

        assert!(code.contains("let field_names : Vec < & str > = vec ! []"));
        assert!(code.contains("let default_values : Vec < serde_json :: Value > = vec ! []"));
    }

    #[test]
    fn test_encode_partial_generates_gap_tracking() {
        // Test: encode_partial tracks gaps for missing fields
        let analysis = create_test_analysis();
        let generator = StructGenerator::new(&analysis, false);

        let simple_struct = create_simple_struct();
        let field_name_map = FxHashMap::default();

        let partial_methods = generator.generate_partial_encoding_methods(
            &format_ident!("User"),
            &simple_struct.members,
            &field_name_map,
        );
        let code_str = partial_methods.to_string();

        // Check gap tracking logic
        assert!(
            code_str.contains("let start = tokens . len ()"),
            "Should track token start positions"
        );
        assert!(
            code_str.contains("let end = tokens . len ()"),
            "Should track token end positions"
        );
        assert!(
            code_str.contains("gaps . push (GapInfo {"),
            "Should push gap information"
        );
        assert!(
            code_str.contains("if ! presence ["),
            "Should check presence array"
        );
        assert!(
            code_str.contains("field_index :"),
            "GapInfo should have field_index"
        );
        assert!(
            code_str.contains("field_name :"),
            "GapInfo should have field_name"
        );
        assert!(
            code_str.contains("token_range : start .. end"),
            "GapInfo should have token_range"
        );
    }

    #[test]
    fn test_default_value_generation_for_primitives() {
        // Test: Default values are generated correctly for all primitive types
        let analysis = create_test_analysis();
        let generator = StructGenerator::new(&analysis, false);

        // Test String → ""
        let string_member = MemberInfo {
            name: "name".to_string(),
            member_type: MemberType::Primitive("String".to_string()),
            required: true,
            is_nullable: false,
            schema: json!({"type": "string"}).as_object().unwrap().clone(),
        };
        let default_str =
            parse_generated_expr(generator.generate_default_value_for_field(&string_member));
        assert!(
            generated_expr_tokens_contain(&default_str, "serde_json :: Value :: String"),
            "String should default to empty string"
        );

        // Test i32 → 0
        let int_member = MemberInfo {
            name: "age".to_string(),
            member_type: MemberType::Primitive("i32".to_string()),
            required: true,
            is_nullable: false,
            schema: json!({"type": "integer"}).as_object().unwrap().clone(),
        };
        let default_int =
            parse_generated_expr(generator.generate_default_value_for_field(&int_member));
        assert!(
            generated_expr_tokens_contain(&default_int, "serde_json :: Value :: Number"),
            "Integer should default to 0"
        );

        // Test bool → false
        let bool_member = MemberInfo {
            name: "active".to_string(),
            member_type: MemberType::Primitive("bool".to_string()),
            required: true,
            is_nullable: false,
            schema: json!({"type": "boolean"}).as_object().unwrap().clone(),
        };
        let default_bool =
            parse_generated_expr(generator.generate_default_value_for_field(&bool_member));
        assert!(
            generated_expr_tokens_contain(&default_bool, "serde_json :: Value :: Bool (false)"),
            "Boolean should default to false"
        );
    }

    #[test]
    fn test_default_value_generation_for_collections() {
        // Test: Collections get appropriate default values
        let analysis = create_test_analysis();
        let generator = StructGenerator::new(&analysis, false);

        // Note: We cannot easily test Vec and HashMap member types in unit tests because
        // we don't have the exact field structure. The generate_default_value_for_field()
        // implementation is tested through integration tests with real schemas.

        // Test optional field → null
        let optional_member = MemberInfo {
            name: "optional".to_string(),
            member_type: MemberType::Primitive("String".to_string()),
            required: false,
            is_nullable: false,
            schema: json!({"type": "string"}).as_object().unwrap().clone(),
        };
        let default_optional =
            parse_generated_expr(generator.generate_default_value_for_field(&optional_member));
        assert!(
            generated_expr_tokens_contain(&default_optional, "serde_json :: Value :: Null"),
            "Optional fields should default to null"
        );
    }

    // ========================================================================================
    // Test Group 2: Bit-Packed Encoding Generation (5 tests)
    // ========================================================================================

    #[test]
    fn test_bitpacked_encode_basic_structure() {
        // Test basic bit-packed encoder generation with 4 optional fields
        let analysis = create_test_analysis();
        let generator = StructGenerator::new(&analysis, false);

        let mut type_disc = create_simple_struct();
        type_disc.members.clear();

        // Add 4 optional non-nullable fields (triggers bit-packing)
        for i in 0..4 {
            type_disc.add_member(MemberInfo {
                name: format!("field_{}", i),
                member_type: MemberType::Primitive("String".to_string()),
                required: false,
                is_nullable: false,
                schema: json!({"type": "string"}).as_object().unwrap().clone(),
            });
        }

        let sanitize_fn = |s: &str| s.to_string();
        let member_type_fn = |mt: &MemberType| -> TctResult<TokenStream> {
            match mt {
                MemberType::Primitive(p) => Ok(quote::quote! { #p }),
                _ => Ok(quote::quote! { String }),
            }
        };
        let derive_traits_fn = |_: Option<&TraitDerivability>, _: bool| quote::quote! {};

        // Create encoding decision for bit-packed struct (4 fields, 1 presence-mask group)
        let encoding_decision =
            crate::generator::encoding_decision::EncodingDecision::BitPackedStruct {
                optional_field_count: 4,
                presence_tokens_needed: 1,
            };

        let result = generator.generate(
            &type_disc,
            None,
            Some(&encoding_decision),
            &sanitize_fn,
            &member_type_fn,
            &derive_traits_fn,
        );

        assert!(result.is_ok(), "Should generate bit-packed encoder");
        let code = result.unwrap().implementation_tokens.to_string();

        // Verify presence mask variable is created
        assert!(
            code.contains("presence_mask_0"),
            "Should generate presence mask variable"
        );

        // Verify bit-setting logic exists
        assert!(
            code.contains("1u32 <<"),
            "Should generate bit-shifting for presence flags"
        );
    }

    #[test]
    fn test_bitpacked_encode_multiple_presence_mask_groups() {
        // Test bit-packed encoder with >32 optional fields (requires 2 presence-mask groups)
        let analysis = create_test_analysis();
        let generator = StructGenerator::new(&analysis, false);

        let mut type_disc = create_simple_struct();
        type_disc.members.clear();

        // Add 40 optional fields (requires ceil(40/32) = 2 presence-mask groups)
        for i in 0..40 {
            type_disc.add_member(MemberInfo {
                name: format!("field_{}", i),
                member_type: MemberType::Primitive("i32".to_string()),
                required: false,
                is_nullable: false,
                schema: json!({"type": "integer"}).as_object().unwrap().clone(),
            });
        }

        let sanitize_fn = |s: &str| s.to_string();
        let member_type_fn = |mt: &MemberType| -> TctResult<TokenStream> {
            match mt {
                MemberType::Primitive(p) => {
                    let ident = quote::format_ident!("{}", p);
                    Ok(quote::quote! { #ident })
                }
                _ => Ok(quote::quote! { String }),
            }
        };
        let derive_traits_fn = |_: Option<&TraitDerivability>, _: bool| quote::quote! {};

        // Create encoding decision for bit-packed struct (40 fields, 2 presence-mask groups)
        let encoding_decision =
            crate::generator::encoding_decision::EncodingDecision::BitPackedStruct {
                optional_field_count: 40,
                presence_tokens_needed: 2,
            };

        let result = generator.generate(
            &type_disc,
            None,
            Some(&encoding_decision),
            &sanitize_fn,
            &member_type_fn,
            &derive_traits_fn,
        );

        assert!(result.is_ok(), "Should generate bit-packed encoder");
        let code = result.unwrap().implementation_tokens.to_string();

        // Verify multiple presence masks
        assert!(
            code.contains("presence_mask_0"),
            "Should generate first presence mask"
        );
        assert!(
            code.contains("presence_mask_1"),
            "Should generate second presence mask for fields 32-39"
        );

        // Verify encoded presence-mask emission for both mask groups
        assert!(
            code.matches("tokens . push").count() >= 2,
            "Should emit at least 2 presence-mask groups"
        );
    }

    #[test]
    fn test_bitpacked_encode_mixed_field_types() {
        // Test bit-packing with mix of required, nullable, and optional fields
        let analysis = create_test_analysis();
        let generator = StructGenerator::new(&analysis, false);

        let mut type_disc = create_simple_struct();
        type_disc.members.clear();

        // Add required field (standard encoding)
        type_disc.add_member(MemberInfo {
            name: "required_field".to_string(),
            member_type: MemberType::Primitive("String".to_string()),
            required: true,
            is_nullable: false,
            schema: json!({"type": "string"}).as_object().unwrap().clone(),
        });

        // Add nullable optional field (uses OptionalField<T>, standard encoding)
        type_disc.add_member(MemberInfo {
            name: "nullable_field".to_string(),
            member_type: MemberType::Primitive("String".to_string()),
            required: false,
            is_nullable: true,
            schema: json!({"type": "string"}).as_object().unwrap().clone(),
        });

        // Add 4 optional non-nullable fields (bit-packed)
        for i in 0..4 {
            type_disc.add_member(MemberInfo {
                name: format!("optional_{}", i),
                member_type: MemberType::Primitive("i32".to_string()),
                required: false,
                is_nullable: false,
                schema: json!({"type": "integer"}).as_object().unwrap().clone(),
            });
        }

        let sanitize_fn = |s: &str| s.to_string();
        let member_type_fn = |mt: &MemberType| -> TctResult<TokenStream> {
            match mt {
                MemberType::Primitive(p) => {
                    let ident = quote::format_ident!("{}", p);
                    Ok(quote::quote! { #ident })
                }
                _ => Ok(quote::quote! { String }),
            }
        };
        let derive_traits_fn = |_: Option<&TraitDerivability>, _: bool| quote::quote! {};

        // Create encoding decision for bit-packed struct (4 optional fields, 1 presence-mask group)
        let encoding_decision =
            crate::generator::encoding_decision::EncodingDecision::BitPackedStruct {
                optional_field_count: 4,
                presence_tokens_needed: 1,
            };

        let result = generator.generate(
            &type_disc,
            None,
            Some(&encoding_decision),
            &sanitize_fn,
            &member_type_fn,
            &derive_traits_fn,
        );

        assert!(result.is_ok(), "Should handle mixed field types");
        let code = result.unwrap().implementation_tokens.to_string();

        // Verify bit-packing is used
        assert!(
            code.contains("presence_mask"),
            "Should use bit-packing for optional fields"
        );

        // Verify standard encoding for required/nullable fields
        assert!(
            code.contains("required_field") && code.contains(" . encode"),
            "Should use standard encoding for required field"
        );
        assert!(
            code.contains("nullable_field") && code.contains(" . encode"),
            "Should use standard encoding for nullable field"
        );
    }

    #[test]
    fn test_bitpacked_decode_structure() {
        // Test bit-packed decoder generation
        let analysis = create_test_analysis();
        let generator = StructGenerator::new(&analysis, false);

        let mut type_disc = create_simple_struct();
        type_disc.members.clear();

        // Add 4 optional fields
        for i in 0..4 {
            type_disc.add_member(MemberInfo {
                name: format!("field_{}", i),
                member_type: MemberType::Primitive("String".to_string()),
                required: false,
                is_nullable: false,
                schema: json!({"type": "string"}).as_object().unwrap().clone(),
            });
        }

        let sanitize_fn = |s: &str| s.to_string();
        let member_type_fn = |mt: &MemberType| -> TctResult<TokenStream> {
            match mt {
                MemberType::Primitive(p) => Ok(quote::quote! { #p }),
                _ => Ok(quote::quote! { String }),
            }
        };
        let derive_traits_fn = |_: Option<&TraitDerivability>, _: bool| quote::quote! {};

        // Create encoding decision for bit-packed struct (4 fields, 1 presence-mask group)
        let encoding_decision =
            crate::generator::encoding_decision::EncodingDecision::BitPackedStruct {
                optional_field_count: 4,
                presence_tokens_needed: 1,
            };

        let result = generator.generate(
            &type_disc,
            None,
            Some(&encoding_decision),
            &sanitize_fn,
            &member_type_fn,
            &derive_traits_fn,
        );

        assert!(result.is_ok(), "Should generate bit-packed decoder");
        let code = result.unwrap().implementation_tokens.to_string();

        // Verify decoder reads presence-mask data
        assert!(
            code.contains("tokens") && code.contains(" . next ()"),
            "Decoder should read presence-mask data"
        );

        // Verify bit-testing logic
        assert!(
            code.contains("1u32") && code.contains(" << "),
            "Decoder should test presence bits"
        );

        // Verify conditional decoding
        assert!(
            code.contains("!= 0"),
            "Decoder should conditionally decode based on presence bit"
        );

        // Verify error handling for EOF
        assert!(
            code.contains("UnexpectedEndOfStream"),
            "Decoder should handle EOF errors"
        );
    }

    #[test]
    fn test_bitpacked_token_count() {
        // Test token_count includes encoded presence-mask digits
        let analysis = create_test_analysis();
        let generator = StructGenerator::new(&analysis, false);

        let mut type_disc = create_simple_struct();
        type_disc.members.clear();

        // Add 5 optional fields (requires 1 presence-mask group)
        for i in 0..5 {
            type_disc.add_member(MemberInfo {
                name: format!("field_{}", i),
                member_type: MemberType::Primitive("i32".to_string()),
                required: false,
                is_nullable: false,
                schema: json!({"type": "integer"}).as_object().unwrap().clone(),
            });
        }

        let sanitize_fn = |s: &str| s.to_string();
        let member_type_fn = |mt: &MemberType| -> TctResult<TokenStream> {
            match mt {
                MemberType::Primitive(p) => {
                    let ident = quote::format_ident!("{}", p);
                    Ok(quote::quote! { #ident })
                }
                _ => Ok(quote::quote! { String }),
            }
        };
        let derive_traits_fn = |_: Option<&TraitDerivability>, _: bool| quote::quote! {};

        // Create encoding decision for bit-packed struct (5 fields, 1 presence-mask group)
        let encoding_decision =
            crate::generator::encoding_decision::EncodingDecision::BitPackedStruct {
                optional_field_count: 5,
                presence_tokens_needed: 1,
            };

        let result = generator.generate(
            &type_disc,
            None,
            Some(&encoding_decision),
            &sanitize_fn,
            &member_type_fn,
            &derive_traits_fn,
        );

        assert!(result.is_ok(), "Should generate token_count method");
        let implementation_tokens = result.unwrap().implementation_tokens;
        let implementation_file = parse_generated_file(implementation_tokens.clone());
        let code = implementation_tokens.to_string();

        // Verify token_count includes encoded presence-mask digits
        generated_trait_method(
            &implementation_file,
            "User",
            "JsonTokenEncoder",
            "token_count",
        );

        // For bit-packing with 5 fields: 1 presence-mask group encoded through the codec.
        // The token count should account for its emitted base-token width.
        assert!(
            code.contains("let mut total =") || code.contains("total +="),
            "token_count should accumulate tokens"
        );
    }

    // ========================================================================================
    // Test Group 3: Streaming decode_prefix Phase 1 - Presence Mask Fallback
    // ========================================================================================

    #[test]
    fn test_streaming_decode_prefix_presence_mask_fallback() {
        // Test Phase 1: decode_prefix should continue past presence mask failures
        // by using unwrap_or(0u32) instead of returning early.
        //
        // This enables progressive decoding where standard fields (apiVersion, kind)
        // become visible even when presence mask tokens are truncated.
        let analysis = create_test_analysis();
        // Enable streaming mode - this is the key difference
        let generator = StructGenerator::new(&analysis, true);

        let mut type_disc = create_simple_struct();
        type_disc.members.clear();

        // Add 2 standard (required) fields - these should be decodable even if presence mask fails
        type_disc.add_member(MemberInfo {
            name: "api_version".to_string(),
            member_type: MemberType::Primitive("String".to_string()),
            required: true,
            is_nullable: false,
            schema: json!({"type": "string"}).as_object().unwrap().clone(),
        });
        type_disc.add_member(MemberInfo {
            name: "kind".to_string(),
            member_type: MemberType::Primitive("String".to_string()),
            required: true,
            is_nullable: false,
            schema: json!({"type": "string"}).as_object().unwrap().clone(),
        });

        // Add 4 optional fields (triggers bit-packing with presence mask)
        for i in 0..4 {
            type_disc.add_member(MemberInfo {
                name: format!("optional_field_{}", i),
                member_type: MemberType::Primitive("String".to_string()),
                required: false,
                is_nullable: false,
                schema: json!({"type": "string"}).as_object().unwrap().clone(),
            });
        }

        let sanitize_fn = |s: &str| s.to_string();
        let member_type_fn = |mt: &MemberType| -> TctResult<TokenStream> {
            match mt {
                MemberType::Primitive(p) => Ok(quote::quote! { #p }),
                _ => Ok(quote::quote! { String }),
            }
        };
        let derive_traits_fn = |_: Option<&TraitDerivability>, _: bool| quote::quote! {};

        // Create encoding decision for bit-packed struct
        let encoding_decision =
            crate::generator::encoding_decision::EncodingDecision::BitPackedStruct {
                optional_field_count: 4,
                presence_tokens_needed: 1,
            };

        let result = generator.generate(
            &type_disc,
            None,
            Some(&encoding_decision),
            &sanitize_fn,
            &member_type_fn,
            &derive_traits_fn,
        );

        assert!(result.is_ok(), "Should generate streaming decode_prefix");
        let implementation_tokens = result.unwrap().implementation_tokens;
        let implementation_file = parse_generated_file(implementation_tokens.clone());
        let code = implementation_tokens.to_string();

        // Verify decode_prefix is generated
        assert!(
            generated_inherent_method(&implementation_file, "User", "decode_prefix").is_some(),
            "Should generate decode_prefix method"
        );

        // Phase 1 requirement: presence mask should use unwrap_or fallback
        // instead of early return on failure
        assert!(
            code.contains("unwrap_or"),
            "decode_prefix should use unwrap_or for presence mask fallback. \
             Current code returns early on presence mask failure, preventing \
             standard fields from being decoded. Got code:\n{}",
            &code[..code.len().min(2000)]
        );

        // Should NOT have the old early-return pattern in presence mask section
        // The old pattern was: match codec::decode(tokens) { Ok(v) => v, Err(_) => return ... }
        // We specifically check that presence_mask decode does NOT use the match/Err pattern

        // Find the decode_prefix function section
        let decode_prefix_start = code
            .find("fn decode_prefix")
            .expect("decode_prefix should exist");
        let decode_prefix_code = &code[decode_prefix_start..];

        // Check for the new unwrap_or pattern (should be present)
        assert!(
            decode_prefix_code.contains("unwrap_or (0u32)")
                || decode_prefix_code.contains("unwrap_or(0u32)"),
            "decode_prefix should use unwrap_or(0u32) for presence mask fallback"
        );

        // Check that there's no early return with fields_decoded: 0 BEFORE standard field decode
        // The presence_mask section comes before field decoding
        // This is a heuristic: if unwrap_or is used, there shouldn't be early return in that section
        let presence_section = if let Some(idx) = decode_prefix_code.find("presence_mask") {
            &decode_prefix_code[..idx + 200.min(decode_prefix_code.len() - idx)]
        } else {
            decode_prefix_code
        };

        // Old pattern would have "Err ( _ ) => { return" near presence_mask
        let has_err_return =
            presence_section.contains("Err ( _ )") && presence_section.contains("return");
        assert!(
            !has_err_return,
            "decode_prefix presence mask section should NOT have Err => return pattern"
        );
    }

    #[test]
    fn test_streaming_decode_prefix_const_first_ordering() {
        // Test: Zero-token (const) fields should be decoded BEFORE variable-token fields
        // in decode_prefix. This allows const fields to be visible even when earlier
        // variable-length fields are truncated.
        //
        // Scenario: Struct with fields [var_field, const_field, var_field2]
        // Expected decode order: const_field first, then var_field, var_field2
        let analysis = create_test_analysis();
        let generator = StructGenerator::new(&analysis, true);

        let mut type_disc = create_simple_struct();
        type_disc.members.clear();

        // Add fields in this order: variable, const (0-token), variable
        // The const field should be decoded FIRST despite being second in declaration
        type_disc.add_member(MemberInfo {
            name: "metadata".to_string(), // Variable-length field
            member_type: MemberType::TypeReference("Metadata".to_string()),
            required: true,
            is_nullable: false,
            schema: json!({"$ref": "#/definitions/Metadata"})
                .as_object()
                .unwrap()
                .clone(),
        });
        type_disc.add_member(MemberInfo {
            name: "kind".to_string(), // Will be marked as 0-token
            member_type: MemberType::TypeReference("DeploymentKind".to_string()),
            required: true,
            is_nullable: false,
            schema: json!({"$ref": "#/definitions/DeploymentKind"})
                .as_object()
                .unwrap()
                .clone(),
        });
        type_disc.add_member(MemberInfo {
            name: "spec".to_string(), // Variable-length field
            member_type: MemberType::TypeReference("Spec".to_string()),
            required: true,
            is_nullable: false,
            schema: json!({"$ref": "#/definitions/Spec"})
                .as_object()
                .unwrap()
                .clone(),
        });

        // Add optional fields to trigger bit-packing
        for i in 0..4 {
            type_disc.add_member(MemberInfo {
                name: format!("optional_{}", i),
                member_type: MemberType::Primitive("String".to_string()),
                required: false,
                is_nullable: false,
                schema: json!({"type": "string"}).as_object().unwrap().clone(),
            });
        }

        let sanitize_fn = |s: &str| s.to_string();
        let member_type_fn = |mt: &MemberType| -> TctResult<TokenStream> {
            match mt {
                MemberType::Primitive(p) => Ok(quote::quote! { #p }),
                MemberType::TypeReference(t) => {
                    let ident = quote::format_ident!("{}", t);
                    Ok(quote::quote! { #ident })
                }
                _ => Ok(quote::quote! { String }),
            }
        };
        let derive_traits_fn = |_: Option<&TraitDerivability>, _: bool| quote::quote! {};

        // Mark "DeploymentKind" as a 0-token type
        let is_zero_token_fn = |type_name: &str| -> bool { type_name == "DeploymentKind" };

        let encoding_decision =
            crate::generator::encoding_decision::EncodingDecision::BitPackedStruct {
                optional_field_count: 4,
                presence_tokens_needed: 1,
            };

        let result = generator.generate_with_zero_token_info(
            &type_disc,
            None,
            Some(&encoding_decision),
            &sanitize_fn,
            &member_type_fn,
            &derive_traits_fn,
            Some(&is_zero_token_fn),
        );

        assert!(
            result.is_ok(),
            "Should generate decode_prefix with const-first ordering"
        );
        let code = result.unwrap().implementation_tokens.to_string();

        // Find decode_prefix function (look for struct's version with PrefixDecodeResult return type)
        // This distinguishes it from codec's decode_prefix which returns Option<u32>
        let decode_prefix_start = code
            .find("-> PrefixDecodeResult < Self >")
            .or_else(|| code.find("-> PrefixDecodeResult<Self>"))
            .expect("decode_prefix should exist");
        let decode_prefix_code = &code[decode_prefix_start..];

        // Find positions of field decodes in the generated code
        let kind_pos = decode_prefix_code.find("let kind").unwrap_or(usize::MAX);
        let metadata_pos = decode_prefix_code
            .find("let metadata")
            .unwrap_or(usize::MAX);
        let spec_pos = decode_prefix_code.find("let spec").unwrap_or(usize::MAX);

        // Const field (kind) should be decoded BEFORE variable fields (metadata, spec)
        assert!(
            kind_pos < metadata_pos,
            "Zero-token field 'kind' should be decoded BEFORE 'metadata'. \
             kind_pos={}, metadata_pos={}",
            kind_pos,
            metadata_pos
        );
        assert!(
            kind_pos < spec_pos,
            "Zero-token field 'kind' should be decoded BEFORE 'spec'. \
             kind_pos={}, spec_pos={}",
            kind_pos,
            spec_pos
        );
    }

    #[test]
    fn test_streaming_decode_prefix_additional_properties_fallback() {
        // Test that additional_properties field uses unwrap_or_default() fallback
        // instead of returning early when decode fails.
        //
        // This is critical for Kubernetes schemas where additional_properties
        // (serde(flatten) HashMap) is decoded before optional fields. If the
        // additional_properties decode fails (e.g., truncated stream), we should
        // continue decoding with an empty map rather than returning early with
        // all fields defaulted.
        //
        // Bug discovered: ObjectMeta decode_prefix was returning complete=false
        // with empty metadata because additional_properties decode was returning
        // early on any decode error.
        let analysis = create_test_analysis();
        let generator = StructGenerator::new(&analysis, true);

        let mut type_disc = create_simple_struct();
        type_disc.members.clear();

        // Add additional_properties field (simulating #[serde(flatten)])
        type_disc.add_member(MemberInfo {
            name: "additional_properties".to_string(),
            member_type: MemberType::Container {
                container: "FxHashMap".to_string(),
                inner: "String".to_string(),
            },
            required: true,
            is_nullable: false,
            schema: json!({"type": "object", "additionalProperties": {"type": "string"}})
                .as_object()
                .unwrap()
                .clone(),
        });

        // Add a regular optional field
        type_disc.add_member(MemberInfo {
            name: "name".to_string(),
            member_type: MemberType::Primitive("String".to_string()),
            required: false,
            is_nullable: false,
            schema: json!({"type": "string"}).as_object().unwrap().clone(),
        });

        let sanitize_fn = |s: &str| s.to_string();
        let member_type_fn = |mt: &MemberType| -> TctResult<TokenStream> {
            match mt {
                MemberType::Primitive(p) => {
                    let ident = format_ident!("{}", p);
                    Ok(quote::quote! { #ident })
                }
                MemberType::Container { inner, .. } => {
                    let inner_ident = format_ident!("{}", inner);
                    Ok(quote::quote! { rustc_hash::FxHashMap<String, #inner_ident> })
                }
                _ => Ok(quote::quote! { String }),
            }
        };
        let derive_traits_fn = |_: Option<&TraitDerivability>, _: bool| quote::quote! {};

        // Create encoding decision for bit-packed struct
        let encoding_decision =
            crate::generator::encoding_decision::EncodingDecision::BitPackedStruct {
                optional_field_count: 1,
                presence_tokens_needed: 1,
            };

        let result = generator.generate(
            &type_disc,
            None,
            Some(&encoding_decision),
            &sanitize_fn,
            &member_type_fn,
            &derive_traits_fn,
        );

        assert!(result.is_ok(), "Should generate streaming decode_prefix");
        let code = result.unwrap().implementation_tokens.to_string();

        // Find decode_prefix function (look for struct's version with PrefixDecodeResult return type)
        // This distinguishes it from codec's decode_prefix which returns Option<u32>
        let decode_prefix_start = code
            .find("-> PrefixDecodeResult < Self >")
            .or_else(|| code.find("-> PrefixDecodeResult<Self>"))
            .expect("decode_prefix should exist");
        let decode_prefix_code = &code[decode_prefix_start..];

        // additional_properties should use unwrap_or_default() fallback
        // instead of match { Ok(v) => v, Err(_) => return ... } pattern
        assert!(
            decode_prefix_code.contains("additional_properties")
                && decode_prefix_code.contains("unwrap_or_default"),
            "additional_properties should use unwrap_or_default() fallback in streaming decode. \
             This prevents early return when additional_properties decode fails, allowing \
             subsequent optional fields (like metadata.name) to be decoded. Got code:\n{}",
            &decode_prefix_code[..decode_prefix_code.len().min(3000)]
        );

        // Should NOT have the match/Err early return pattern for additional_properties
        // Look for the pattern: "let additional_properties" followed by "Err(_) => { return"
        let additional_props_section = decode_prefix_code
            .find("let additional_properties")
            .map(|start| {
                &decode_prefix_code[start..start + 500.min(decode_prefix_code.len() - start)]
            })
            .unwrap_or("");

        assert!(
            !additional_props_section.contains("Err (_) => { return")
                && !additional_props_section.contains("Err(_) => { return"),
            "additional_properties decode should NOT return early on Err. \
             Use unwrap_or_default() instead. Found problematic pattern in:\n{}",
            additional_props_section
        );
    }

    // ========================================================================================
    // Test Group 4: Vec<Struct> decode_prefix - Multi-Token Length Encoding (BUG FIX)
    // ========================================================================================

    #[test]
    fn test_vec_struct_decode_prefix_uses_base256_length_encoding() {
        // TDD Test for bug: decode_prefix reads Vec length as single token,
        // but encode uses multi-token base256 encoding.
        //
        // Bug reproduction:
        // - Vec with 1 element encodes length as [0, 1] (width=1, value=1)
        // - decode_prefix reads first token (0) as length -> empty Vec!
        //
        // Fix: decode_prefix must use base256::decode_u32 for Vec length
        let analysis = create_test_analysis();
        let generator = StructGenerator::new(&analysis, true); // streaming enabled

        let mut type_disc = create_simple_struct();
        type_disc.members.clear();

        // Add a required Vec<Struct> field (ComplexContainer)
        type_disc.add_member(MemberInfo {
            name: "items".to_string(),
            member_type: MemberType::ComplexContainer {
                container: "Vec".to_string(),
                inner: "InnerStruct".to_string(),
            },
            required: true,
            is_nullable: false,
            schema: json!({"type": "array", "items": {"$ref": "#/definitions/InnerStruct"}})
                .as_object()
                .unwrap()
                .clone(),
        });

        let sanitize_fn = |s: &str| s.to_string();
        let member_type_fn = |member_type: &MemberType| -> TctResult<TokenStream> {
            Ok(match member_type {
                MemberType::ComplexContainer { inner, .. } => {
                    let inner_ident = format_ident!("{}", inner);
                    quote! { Vec<#inner_ident> }
                }
                MemberType::Primitive(t) => {
                    let type_ident = format_ident!("{}", t);
                    quote! { #type_ident }
                }
                _ => quote! { () },
            })
        };
        let derive_traits_fn = |_: Option<&TraitDerivability>, _: bool| {
            quote! { #[derive(Debug)] }
        };

        let result = generator.generate(
            &type_disc,
            None,
            None,
            &sanitize_fn,
            &member_type_fn,
            &derive_traits_fn,
        );

        assert!(
            result.is_ok(),
            "Should generate struct with Vec<Struct> field"
        );
        let code = result.unwrap().implementation_tokens.to_string();

        // Find decode_prefix function (in implementation tokens, not definition)
        let decode_prefix_start = code
            .find("fn decode_prefix")
            .expect("decode_prefix should exist in implementation tokens");
        let decode_prefix_code = &code[decode_prefix_start..];

        // BUG CHECK: The buggy code uses tokens.next() for Vec length
        // This reads the width indicator as length, causing empty Vec!
        let has_single_token_length = decode_prefix_code.contains("tokens . next ()")
            && decode_prefix_code.contains("len as usize");

        // FIX CHECK: The fixed code should use multi-token decoding
        let has_multi_token_length = decode_prefix_code.contains("base256 :: decode_u32")
            || decode_prefix_code.contains("variable_base :: decode_u32")
            || decode_prefix_code.contains("decode_vec_streaming");

        assert!(
            !has_single_token_length,
            "BUG DETECTED: decode_prefix uses single-token Vec length which is incompatible \
             with multi-token base256 encoding. Vec with 1 element encodes as [0, 1] but \
             decode_prefix reads 0 as length -> empty Vec!\n\
             Found problematic pattern in decode_prefix:\n{}",
            &decode_prefix_code[..decode_prefix_code.len().min(2000)]
        );

        assert!(
            has_multi_token_length,
            "FIX REQUIRED: decode_prefix must use multi-token decoding (variable_base::decode_u32 or decode_vec_streaming) \
             for Vec length to match the multi-token encoding format.\n\
             Current decode_prefix code:\n{}",
            &decode_prefix_code[..decode_prefix_code.len().min(2000)]
        );
    }

    #[test]
    fn test_optional_vec_struct_decode_prefix_uses_base256_length_encoding() {
        // Same bug but for optional Vec<Struct> fields
        let analysis = create_test_analysis();
        let generator = StructGenerator::new(&analysis, true);

        let mut type_disc = create_simple_struct();
        type_disc.members.clear();

        // Add a required field first (to trigger bit-packing)
        type_disc.add_member(MemberInfo {
            name: "name".to_string(),
            member_type: MemberType::Primitive("String".to_string()),
            required: true,
            is_nullable: false,
            schema: json!({"type": "string"}).as_object().unwrap().clone(),
        });

        // Add an optional Vec<Struct> field
        type_disc.add_member(MemberInfo {
            name: "subjects".to_string(),
            member_type: MemberType::ComplexContainer {
                container: "Vec".to_string(),
                inner: "Subject".to_string(),
            },
            required: false,
            is_nullable: false,
            schema: json!({"type": "array", "items": {"$ref": "#/definitions/Subject"}})
                .as_object()
                .unwrap()
                .clone(),
        });

        // Add more optional fields to trigger bit-packing
        for i in 0..3 {
            type_disc.add_member(MemberInfo {
                name: format!("optional_{}", i),
                member_type: MemberType::Primitive("String".to_string()),
                required: false,
                is_nullable: false,
                schema: json!({"type": "string"}).as_object().unwrap().clone(),
            });
        }

        let sanitize_fn = |s: &str| s.to_string();
        let member_type_fn = |member_type: &MemberType| -> TctResult<TokenStream> {
            Ok(match member_type {
                MemberType::ComplexContainer { inner, .. } => {
                    let inner_ident = format_ident!("{}", inner);
                    quote! { Vec<#inner_ident> }
                }
                MemberType::Primitive(t) => {
                    let type_ident = format_ident!("{}", t);
                    quote! { #type_ident }
                }
                _ => quote! { () },
            })
        };
        let derive_traits_fn = |_: Option<&TraitDerivability>, _: bool| {
            quote! { #[derive(Debug)] }
        };

        let result = generator.generate(
            &type_disc,
            None,
            None,
            &sanitize_fn,
            &member_type_fn,
            &derive_traits_fn,
        );
        assert!(
            result.is_ok(),
            "Should generate struct with optional Vec<Struct>"
        );
        let code = result.unwrap().implementation_tokens.to_string();

        // Find decode_prefix function (in implementation tokens, not definition)
        let decode_prefix_start = code
            .find("fn decode_prefix")
            .expect("decode_prefix should exist in implementation tokens");
        let decode_prefix_code = &code[decode_prefix_start..];

        // Find the subjects decoding section specifically
        let subjects_section = decode_prefix_code
            .find("subjects")
            .map(|start| {
                &decode_prefix_code[start..start.saturating_add(800).min(decode_prefix_code.len())]
            })
            .unwrap_or("");

        // The optional field's presence flag still uses tokens.next(); the Vec
        // payload length itself must use the same multi-token decoder as
        // generated encode().
        let has_multi_token_length = subjects_section.contains("variable_base :: decode_u32")
            || subjects_section.contains("base256 :: decode_u32")
            || subjects_section.contains("decode_vec_streaming");

        assert!(
            has_multi_token_length,
            "FIX REQUIRED in optional Vec<Struct> (subjects): Vec length must use multi-token decoding. \
             The optional presence flag is a separate token and must not be mistaken for Vec length.\n\
             Found in subjects section:\n{}",
            subjects_section
        );
    }
}
