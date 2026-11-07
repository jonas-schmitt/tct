//! Type Alias Generator - Specialized code generation for type aliases and wrapper types
//!
//! This module handles code generation for various wrapper and alias types:
//! - Array type aliases (`Vec<T>` wrappers)
//! - Primitive wrappers (String, i64, f64, bool wrappers)
//! - Minimal object wrappers (serde_json::Value wrappers)
//! - Type aliases (pub type A = B;)
//!
//! All generated wrappers include:
//! - Transparent serde serialization
//! - JsonTokenEncoder implementation
//! - Helper methods (len, is_empty, value, etc.)
//! - Conditional trait derivation (Eq, Ord, Hash) based on analysis
//!
//! Extracted from code_generator.rs as part of module size refactoring (2025-10-04).

use crate::TctResult;
use crate::generator::generation_output::TypeCodeGeneration;
use crate::generator::schema_discovery::TypeDiscovery;
use crate::generator::type_analysis::TraitDerivability;
use quote::{format_ident, quote};
use serde_json::Value;

mod json_const;
mod primitive_encoding;

/// Specialized generator for type aliases and wrapper types
///
/// Handles generation of:
/// - Array wrappers (newtype around `Vec<T>`)
/// - Primitive wrappers (newtype around String, i64, etc.)
/// - Minimal object wrappers (newtype around serde_json::Value)
/// - Simple type aliases (pub type A = B;)
///
/// Uses dependency injection for shared utilities to avoid code duplication.
pub struct TypeAliasGenerator {
    /// Whether to enable streaming encode support (adds Default derive)
    streaming_encode: bool,
    /// Base for multi-token collection length encoding.
    encoding_base: u32,
    /// Maximum collection length accepted by strict generated decoders.
    max_collection_length: usize,
}

impl Default for TypeAliasGenerator {
    fn default() -> Self {
        Self {
            streaming_encode: false,
            encoding_base: Self::DEFAULT_ENCODING_BASE,
            max_collection_length: Self::DEFAULT_MAX_COLLECTION_LENGTH,
        }
    }
}

impl TypeAliasGenerator {
    const DEFAULT_ENCODING_BASE: u32 = 256;
    const DEFAULT_MAX_COLLECTION_LENGTH: usize = 2048;

    /// Create a new type alias generator
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a new type alias generator with streaming encode support
    pub fn with_streaming_encode(streaming_encode: bool) -> Self {
        Self {
            streaming_encode,
            encoding_base: Self::DEFAULT_ENCODING_BASE,
            max_collection_length: Self::DEFAULT_MAX_COLLECTION_LENGTH,
        }
    }

    /// Create a new type alias generator with streaming support and an encoding base.
    pub fn with_streaming_encode_and_encoding_base(
        streaming_encode: bool,
        encoding_base: u32,
    ) -> Self {
        Self {
            streaming_encode,
            encoding_base,
            max_collection_length: Self::DEFAULT_MAX_COLLECTION_LENGTH,
        }
    }

    /// Create a new type alias generator with streaming support, an encoding base, and a collection cap.
    pub fn with_streaming_encode_encoding_base_and_max_collection_length(
        streaming_encode: bool,
        encoding_base: u32,
        max_collection_length: usize,
    ) -> Self {
        Self {
            streaming_encode,
            encoding_base,
            max_collection_length,
        }
    }

    fn integer_item_type_tokens(items: &Value) -> proc_macro2::TokenStream {
        let has_i64_bounds = items.as_object().is_some_and(|schema| {
            schema.get("minimum").and_then(Value::as_i64).is_some()
                && schema.get("maximum").and_then(Value::as_i64).is_some()
        });

        if has_i64_bounds {
            quote! { i64 }
        } else {
            quote! { crate::tokenizer::JsonInteger }
        }
    }

    /// Generate array type alias (`Vec<T>` wrapper)
    ///
    /// Creates a newtype wrapper around `Vec<T>` with:
    /// - Transparent serde serialization
    /// - JsonTokenEncoder with length-prefixed encoding
    /// - Helper methods (len, is_empty, iter)
    /// - Conditional trait derivation (special handling for serde_json::Value elements)
    pub fn generate_array_type_alias(
        &self,
        type_discovery: &TypeDiscovery,
        trait_derivability: Option<&TraitDerivability>,
    ) -> TctResult<TypeCodeGeneration> {
        let type_name = &type_discovery.type_name;
        let type_ident = format_ident!("{}", type_name);

        crate::log_debug!("🔧 Generating array type alias for '{}'", type_name);

        // Extract element type from schema.items.$ref or schema.items.type
        // Always work with TokenStreams to avoid format_ident! issues with complex types
        let element_type_tokens = if let Some(items) = type_discovery.schema_object.get("items") {
            if let Some(ref_path) = items.get("$ref").and_then(Value::as_str) {
                // Reference to another type - extract type name
                crate::log_debug!("   • Array items reference: {}", ref_path);
                if let Some(type_name) = crate::schema_utils::local_definition_ref_target(ref_path)
                {
                    let sanitized_name = crate::utils::to_upper_camel_case(&type_name);
                    crate::log_debug!("   • Element type from $ref: {}", sanitized_name);
                    let element_ident = format_ident!("{}", sanitized_name);
                    quote! { #element_ident }
                } else {
                    crate::log_debug!(
                        "   • Fallback to serde_json::Value for unresolved reference"
                    );
                    quote! { serde_json::Value }
                }
            } else if let Some(item_type) = items.get("type").and_then(Value::as_str) {
                // Primitive type
                crate::log_debug!("   • Array items primitive type: {}", item_type);
                match item_type {
                    "string" => quote! { String },
                    "number" => quote! { serde_json::Number },
                    "integer" => Self::integer_item_type_tokens(items),
                    "boolean" => quote! { bool },
                    _ => {
                        crate::log_debug!(
                            "   • Unknown primitive type '{}', using serde_json::Value",
                            item_type
                        );
                        quote! { serde_json::Value }
                    }
                }
            } else {
                crate::log_debug!("   • No type info in items, using serde_json::Value");
                quote! { serde_json::Value }
            }
        } else {
            crate::log_debug!("   • No items found, using serde_json::Value");
            quote! { serde_json::Value }
        };

        crate::log_debug!("   • Final element type tokens generated");

        // Check if element type is serde_json::Value (cannot derive Ord/PartialOrd)
        let is_value_type = Self::array_items_use_json_value(type_discovery);

        if is_value_type {
            crate::log_debug!(
                "   ⚠️  Element type is serde_json::Value - disabling Ord/PartialOrd derivation"
            );
        }

        // Generate newtype wrapper instead of type alias to avoid orphan rule violations
        // Using a tuple struct allows JsonTokenEncoder implementation on our own type
        // Conditionally derive Eq/Ord/Hash based on trait analysis
        // IMPORTANT: serde_json::Value doesn't implement Ord, so skip Ord if element is Value
        let (eq_trait, ord_traits, hash_trait) = if let Some(traits) = trait_derivability {
            crate::log_debug!(
                "   • Using trait analysis: Eq={}, Ord={}, Hash={}",
                traits.can_derive_eq,
                traits.can_derive_ord && !is_value_type,
                traits.can_derive_hash
            );
            (
                if traits.can_derive_eq {
                    quote! { Eq, }
                } else {
                    quote! {}
                },
                if traits.can_derive_ord && !is_value_type {
                    quote! { PartialOrd, Ord, }
                } else {
                    quote! {}
                },
                if traits.can_derive_hash {
                    quote! { Hash, }
                } else {
                    quote! {}
                },
            )
        } else {
            crate::log_debug!("   • No trait analysis available, using conservative defaults");
            (quote! {}, quote! {}, quote! {})
        };

        // Add Default derive when streaming encode is enabled
        let default_trait = if self.streaming_encode {
            quote! { Default, }
        } else {
            quote! {}
        };

        let definition_tokens = quote! {
            #[derive(Debug, Clone, PartialEq, #eq_trait #ord_traits #hash_trait #default_trait serde::Serialize, serde::Deserialize)]
            #[serde(transparent)]
            pub struct #type_ident(pub Vec<#element_type_tokens>);
        };

        // Generate helper methods for the array wrapper
        let helper_methods = quote! {
            impl #type_ident {
                /// Get the length of the array
                pub fn len(&self) -> usize {
                    self.0.len()
                }

                /// Check if the array is empty
                pub fn is_empty(&self) -> bool {
                    self.0.is_empty()
                }

                /// Get an iterator over the elements
                pub fn iter(&self) -> impl Iterator<Item = &#element_type_tokens> {
                    self.0.iter()
                }
            }
        };

        let encoding_base = self.encoding_base;
        let max_collection_length = self.max_collection_length;
        let schema_root = Value::Object(type_discovery.schema_object.clone());
        let array_items_can_use_streaming_decode_prefix = type_discovery
            .schema_object
            .get("items")
            .and_then(|items| items.get("$ref"))
            .and_then(Value::as_str)
            .is_some_and(|ref_path| {
                crate::schema_utils::local_definition_ref_target(ref_path).is_some()
                    && crate::schema_utils::resolve_local_ref(&schema_root, ref_path)
                        .is_none_or(|referenced_schema| referenced_schema.get("const").is_none())
            });
        let array_items_are_string = type_discovery
            .schema_object
            .get("items")
            .and_then(|items| items.get("type"))
            .and_then(Value::as_str)
            == Some("string");

        // Generate JsonTokenEncoder implementation for the newtype wrapper
        let implementation_tokens = quote! {
            #helper_methods

            impl crate::tokenizer::JsonTokenEncoder for #type_ident {
                fn encode(&self, tokens: &mut Vec<u32>) {
                    // Encode array length first
                    crate::tokenizer::variable_base::encode_u32_fixed_base(
                        self.0.len() as u32,
                        tokens
                    );
                    // Then encode each element
                    for item in &self.0 {
                        item.encode(tokens);
                    }
                }

                fn decode(tokens: &mut impl Iterator<Item = u32>) -> Result<Self, crate::tokenizer::DecodeError> {
                    let length = crate::tokenizer::variable_base::decode_u32_fixed_base(
                        tokens,
                        #type_name
                    )? as usize;
                    if length > #max_collection_length {
                        return Err(crate::tokenizer::DecodeError::InvalidData(
                            format!("Array length {} exceeds maximum {}", length, #max_collection_length)
                        ));
                    }
                    let mut result = Vec::with_capacity(length);

                    for _ in 0..length {
                        let item = <#element_type_tokens>::decode(tokens)?;
                        result.push(item);
                    }

                    Ok(Self(result))
                }

                fn decode_strict(tokens: &mut impl Iterator<Item = u32>) -> Result<Self, crate::tokenizer::DecodeError> {
                    let length = crate::tokenizer::variable_base::decode_u32_fixed_base_strict(
                        tokens,
                        #type_name
                    )? as usize;
                    if length > #max_collection_length {
                        return Err(crate::tokenizer::DecodeError::InvalidData(
                            format!("Array length {} exceeds maximum {}", length, #max_collection_length)
                        ));
                    }
                    let mut result = Vec::with_capacity(length);

                    for _ in 0..length {
                        let item = <#element_type_tokens>::decode_strict(tokens)?;
                        result.push(item);
                    }

                    Ok(Self(result))
                }

                fn token_count(&self) -> usize {
                    crate::tokenizer::variable_base::token_count_u32_fixed_base(
                        self.0.len() as u32
                    ) + self.0.iter().map(|item| item.token_count()).sum::<usize>()
                }
            }
        };

        // Generate streaming decode_prefix if streaming mode is enabled
        let streaming_element_decode = if array_items_can_use_streaming_decode_prefix {
            quote! {
                let _element_result = <#element_type_tokens>::decode_prefix(tokens);
                result.push(_element_result.instance);
                decoded_count += 1;
                if !_element_result.complete {
                    return PrefixDecodeResult {
                        instance: Self(result),
                        fields_decoded: decoded_count,
                        complete: false,
                        partial_field: Some(PartialFieldInfo {
                            field_name: "elements",
                            field_type: "Vec",
                            items_decoded: decoded_count,
                            expected_length: Some(length),
                        }),
                    };
                }
            }
        } else {
            quote! {
                match <#element_type_tokens as crate::tokenizer::JsonTokenEncoder>::decode(tokens) {
                    Ok(item) => {
                        result.push(item);
                        decoded_count += 1;
                    }
                    Err(_) => {
                        // Partial decode - return what we have so far
                        return PrefixDecodeResult {
                            instance: Self(result),
                            fields_decoded: decoded_count,
                            complete: false,
                            partial_field: Some(PartialFieldInfo {
                                field_name: "elements",
                                field_type: "Vec",
                                items_decoded: decoded_count,
                                expected_length: Some(length),
                            }),
                        };
                    }
                }
            }
        };

        let decode_prefix_impl = if self.streaming_encode && array_items_are_string {
            quote! {
                impl #type_ident {
                    /// Streaming decode that handles truncated token sequences
                    pub fn decode_prefix(tokens: &mut impl Iterator<Item = u32>) -> PrefixDecodeResult<Self> {
                        let (result, complete, expected_length) =
                            crate::tokenizer::decode_string_vec_streaming(tokens);
                        let decoded_count = result.len();
                        if !complete {
                            return PrefixDecodeResult {
                                instance: Self(result),
                                fields_decoded: decoded_count,
                                complete: false,
                                partial_field: Some(PartialFieldInfo {
                                    field_name: "elements",
                                    field_type: "Vec",
                                    items_decoded: decoded_count,
                                    expected_length,
                                }),
                            };
                        }

                        PrefixDecodeResult {
                            instance: Self(result),
                            fields_decoded: decoded_count,
                            complete: true,
                            partial_field: None,
                        }
                    }
                }
            }
        } else if self.streaming_encode {
            quote! {
                impl #type_ident {
                    /// Streaming decode that handles truncated token sequences
                    pub fn decode_prefix(tokens: &mut impl Iterator<Item = u32>) -> PrefixDecodeResult<Self> {
                        // Try to decode the length prefix.
                        let length = match crate::tokenizer::variable_base::decode_u32_option(
                            tokens,
                            #encoding_base
                        ) {
                            Some(len) => len as usize,
                            None => {
                                // No tokens at all - return empty array
                                return PrefixDecodeResult {
                                    instance: Self::default(),
                                    fields_decoded: 0,
                                    complete: false,
                                    partial_field: None,
                                };
                            }
                        };

                        let mut result = Vec::with_capacity(length.min(#max_collection_length)); // Cap allocation
                        let mut decoded_count = 0;

                        for _ in 0..length {
                            #streaming_element_decode
                        }

                        PrefixDecodeResult {
                            instance: Self(result),
                            fields_decoded: decoded_count,
                            complete: true,
                            partial_field: None,
                        }
                    }
                }
            }
        } else {
            quote! {}
        };

        // Combine implementation tokens
        let full_implementation = quote! {
            #implementation_tokens
            #decode_prefix_impl
        };

        // Create type code generation
        let type_generation = TypeCodeGeneration::new_struct(
            type_name.clone(),
            definition_tokens,
            full_implementation,
        )
        .with_token_encoder_impl(true);

        crate::log_debug!("✅ Array type alias '{}' generated", type_name);
        Ok(type_generation)
    }

    fn array_items_use_json_value(type_discovery: &TypeDiscovery) -> bool {
        let Some(items) = type_discovery.schema_object.get("items") else {
            return true;
        };

        if let Some(ref_path) = items.get("$ref").and_then(Value::as_str) {
            return crate::schema_utils::local_definition_ref_target(ref_path).is_none();
        }

        match items.get("type").and_then(Value::as_str) {
            Some("string" | "number" | "integer" | "boolean") => false,
            Some(_) | None => true,
        }
    }

    /// Generate primitive wrapper (String, i64, f64, bool wrapper)
    ///
    /// Creates a newtype wrapper around primitive types with:
    /// - Transparent serde serialization
    /// - JsonTokenEncoder delegating to inner type
    /// - Helper methods (value, into_value, new)
    pub fn generate_primitive_wrapper(
        &self,
        type_discovery: &TypeDiscovery,
        primitive_type: &str,
        trait_derivability: Option<&TraitDerivability>,
    ) -> TctResult<TypeCodeGeneration> {
        if let Some(const_value) = type_discovery.schema_object.get("const") {
            return json_const::generate_json_const_wrapper(type_discovery, const_value);
        }

        let type_name = &type_discovery.type_name;
        let type_ident = format_ident!("{}", type_name);

        crate::log_debug!(
            "🔧 Generating primitive wrapper for '{}' ({})",
            type_name,
            primitive_type
        );

        // Convert primitive type string to Rust type
        let rust_type_tokens = match primitive_type {
            "String" => quote! { String },
            "f64" => quote! { f64 },
            "i64" => quote! { i64 },
            "JsonInteger" => quote! { crate::tokenizer::JsonInteger },
            "bool" => quote! { bool },
            "serde_json::Number" => quote! { serde_json::Number },
            _ => {
                crate::log_debug!(
                    "   • Unknown primitive type '{}', using String",
                    primitive_type
                );
                quote! { String }
            }
        };

        // Conditionally derive Eq/Ord/Hash based on trait analysis
        let (eq_trait, ord_traits, hash_trait) = if let Some(traits) = trait_derivability {
            crate::log_debug!(
                "   • Using trait analysis: Eq={}, Ord={}, Hash={}",
                traits.can_derive_eq,
                traits.can_derive_ord,
                traits.can_derive_hash
            );
            (
                if traits.can_derive_eq {
                    quote! { Eq, }
                } else {
                    quote! {}
                },
                if traits.can_derive_ord {
                    quote! { PartialOrd, Ord, }
                } else {
                    quote! {}
                },
                if traits.can_derive_hash {
                    quote! { Hash, }
                } else {
                    quote! {}
                },
            )
        } else {
            crate::log_debug!("   • No trait analysis available, using conservative defaults");
            (quote! {}, quote! {}, quote! {})
        };

        // Add Default derive when streaming encode is enabled
        let default_trait = if self.streaming_encode {
            quote! { Default, }
        } else {
            quote! {}
        };

        let definition_tokens = quote! {
            #[derive(Debug, Clone, PartialEq, #eq_trait #ord_traits #hash_trait #default_trait serde::Serialize, serde::Deserialize)]
            #[serde(transparent)]
            pub struct #type_ident(pub #rust_type_tokens);
        };

        // Generate helper methods for the primitive wrapper
        let helper_methods = quote! {
            impl #type_ident {
                /// Get the inner value
                pub fn value(&self) -> &#rust_type_tokens {
                    &self.0
                }

                /// Get the inner value (consuming)
                pub fn into_value(self) -> #rust_type_tokens {
                    self.0
                }

                /// Create from inner value
                pub fn new(value: #rust_type_tokens) -> Self {
                    Self(value)
                }
            }
        };

        // Generate streaming decode_prefix if streaming mode is enabled
        let decode_prefix_impl = if self.streaming_encode {
            quote! {
                impl #type_ident {
                    /// Streaming decode that handles truncated token sequences
                    pub fn decode_prefix(tokens: &mut impl Iterator<Item = u32>) -> PrefixDecodeResult<Self> {
                        // Use Self::decode() to ensure we use the type's own decoder
                        // (important for FormatOptimized types like DateTime which have custom encode/decode)
                        match <Self as crate::tokenizer::JsonTokenEncoder>::decode(tokens) {
                            Ok(instance) => PrefixDecodeResult {
                                instance,
                                fields_decoded: 1,
                                complete: true,
                                partial_field: None,
                            },
                            Err(_) => PrefixDecodeResult {
                                instance: Self::default(),
                                fields_decoded: 0,
                                complete: false,
                                partial_field: None,
                            },
                        }
                    }
                }
            }
        } else {
            quote! {}
        };

        // Generate JsonTokenEncoder implementation for the primitive wrapper
        let implementation_tokens = quote! {
            #helper_methods

            impl crate::tokenizer::JsonTokenEncoder for #type_ident {
                fn encode(&self, tokens: &mut Vec<u32>) {
                    self.0.encode(tokens);
                }

                fn decode(tokens: &mut impl Iterator<Item = u32>) -> Result<Self, crate::tokenizer::DecodeError> {
                    let value = <#rust_type_tokens>::decode(tokens)?;
                    Ok(Self(value))
                }

                fn decode_strict(tokens: &mut impl Iterator<Item = u32>) -> Result<Self, crate::tokenizer::DecodeError> {
                    let value = <#rust_type_tokens>::decode_strict(tokens)?;
                    Ok(Self(value))
                }

                fn token_count(&self) -> usize {
                    self.0.token_count()
                }
            }

            #decode_prefix_impl
        };

        // Create type code generation
        let type_generation = TypeCodeGeneration::new_struct(
            type_name.clone(),
            definition_tokens,
            implementation_tokens,
        )
        .with_token_encoder_impl(true);

        crate::log_debug!("✅ Primitive wrapper '{}' generated", type_name);
        Ok(type_generation)
    }

    /// Generate primitive wrapper with encoding decision (O3 optimizations)
    ///
    /// This is similar to `generate_primitive_wrapper` but handles O3 optimizations:
    /// - ConstantInteger: Encode/decode 0 tokens (value is constant)
    /// - FormatOptimized: Use format-specific encoding for strings
    pub fn generate_primitive_wrapper_with_decision(
        &self,
        type_discovery: &TypeDiscovery,
        primitive_type: &str,
        trait_derivability: Option<&TraitDerivability>,
        encoding_decision: Option<&crate::generator::encoding_decision::EncodingDecision>,
    ) -> TctResult<TypeCodeGeneration> {
        if let Some(const_value) = type_discovery.schema_object.get("const") {
            return json_const::generate_json_const_wrapper(type_discovery, const_value);
        }

        let type_name = &type_discovery.type_name;
        let type_ident = format_ident!("{}", type_name);

        crate::log_debug!(
            "🔧 Generating primitive wrapper with O3 optimization for '{}' ({})",
            type_name,
            primitive_type
        );

        // Convert primitive type string to Rust type
        let rust_type_tokens = match primitive_type {
            "String" => quote! { String },
            "f64" => quote! { f64 },
            "i64" => quote! { i64 },
            "JsonInteger" => quote! { crate::tokenizer::JsonInteger },
            "bool" => quote! { bool },
            "serde_json::Number" => quote! { serde_json::Number },
            _ => quote! { String },
        };

        // Conditionally derive Eq/Ord/Hash based on trait analysis
        let (eq_trait, ord_traits, hash_trait) = if let Some(traits) = trait_derivability {
            (
                if traits.can_derive_eq {
                    quote! { Eq, }
                } else {
                    quote! {}
                },
                if traits.can_derive_ord {
                    quote! { PartialOrd, Ord, }
                } else {
                    quote! {}
                },
                if traits.can_derive_hash {
                    quote! { Hash, }
                } else {
                    quote! {}
                },
            )
        } else {
            (quote! {}, quote! {}, quote! {})
        };

        // Add Default derive when streaming encode is enabled
        let default_trait = if self.streaming_encode {
            quote! { Default, }
        } else {
            quote! {}
        };

        let definition_tokens = quote! {
            #[derive(Debug, Clone, PartialEq, #eq_trait #ord_traits #hash_trait #default_trait serde::Serialize, serde::Deserialize)]
            #[serde(transparent)]
            pub struct #type_ident(pub #rust_type_tokens);
        };

        // Generate helper methods for the primitive wrapper
        let helper_methods = quote! {
            impl #type_ident {
                /// Get the inner value
                pub fn value(&self) -> &#rust_type_tokens {
                    &self.0
                }

                /// Get the inner value (consuming)
                pub fn into_value(self) -> #rust_type_tokens {
                    self.0
                }

                /// Create from inner value
                pub fn new(value: #rust_type_tokens) -> Self {
                    Self(value)
                }
            }
        };

        let encoder_impl = primitive_encoding::generate_encoder_impl(
            &type_ident,
            &rust_type_tokens,
            encoding_decision,
        );

        // Generate streaming decode_prefix if streaming mode is enabled
        let decode_prefix_impl = if self.streaming_encode {
            quote! {
                impl #type_ident {
                    /// Streaming decode that handles truncated token sequences
                    pub fn decode_prefix(tokens: &mut impl Iterator<Item = u32>) -> PrefixDecodeResult<Self> {
                        // Use Self::decode() to ensure we use the type's own decoder
                        // (important for FormatOptimized types like DateTime which have custom encode/decode)
                        match <Self as crate::tokenizer::JsonTokenEncoder>::decode(tokens) {
                            Ok(instance) => PrefixDecodeResult {
                                instance,
                                fields_decoded: 1,
                                complete: true,
                                partial_field: None,
                            },
                            Err(_) => PrefixDecodeResult {
                                instance: Self::default(),
                                fields_decoded: 0,
                                complete: false,
                                partial_field: None,
                            },
                        }
                    }
                }
            }
        } else {
            quote! {}
        };

        let implementation_tokens = quote! {
            #helper_methods
            #encoder_impl
            #decode_prefix_impl
        };

        // Create type code generation
        let type_generation = TypeCodeGeneration::new_struct(
            type_name.clone(),
            definition_tokens,
            implementation_tokens,
        )
        .with_token_encoder_impl(true);

        crate::log_debug!(
            "✅ Primitive wrapper with O3 optimization '{}' generated",
            type_name
        );
        Ok(type_generation)
    }

    /// Generate minimal object wrapper (serde_json::Value wrapper)
    ///
    /// Creates a newtype wrapper around serde_json::Value for generic objects.
    /// Note: serde_json::Value has limited trait support (no Eq, Ord, Hash).
    pub fn generate_minimal_object_wrapper(
        &self,
        type_discovery: &TypeDiscovery,
        trait_derivability: Option<&TraitDerivability>,
    ) -> TctResult<TypeCodeGeneration> {
        let type_name = &type_discovery.type_name;
        let type_ident = format_ident!("{}", type_name);

        crate::log_debug!("🔧 Generating minimal object wrapper for '{}'", type_name);

        // Minimal objects use serde_json::Value for generic JSON handling
        let rust_type_tokens = quote! { serde_json::Value };

        // serde_json::Value has limited trait support (no Eq, Ord, Hash)
        // but it does implement Default (defaults to Value::Null)
        let (eq_trait, ord_traits, hash_trait) = if let Some(traits) = trait_derivability {
            crate::log_debug!(
                "   • Using trait analysis: Eq={}, Ord={}, Hash={}",
                traits.can_derive_eq,
                traits.can_derive_ord,
                traits.can_derive_hash
            );
            // serde_json::Value doesn't support Eq/Ord/Hash due to floating point values
            (quote! {}, quote! {}, quote! {})
        } else {
            crate::log_debug!("   • No trait analysis available, using conservative defaults");
            (quote! {}, quote! {}, quote! {})
        };

        // Add Default derive when streaming encode is enabled
        // serde_json::Value implements Default (defaults to Value::Null)
        let default_trait = if self.streaming_encode {
            quote! { Default, }
        } else {
            quote! {}
        };

        let definition_tokens = quote! {
            #[derive(Debug, Clone, PartialEq, #eq_trait #ord_traits #hash_trait #default_trait serde::Serialize, serde::Deserialize)]
            #[serde(transparent)]
            pub struct #type_ident(pub #rust_type_tokens);
        };

        // Generate helper methods for the minimal object wrapper
        let helper_methods = quote! {
            impl #type_ident {
                /// Get the inner value
                pub fn value(&self) -> &#rust_type_tokens {
                    &self.0
                }

                /// Get the inner value (consuming)
                pub fn into_value(self) -> #rust_type_tokens {
                    self.0
                }

                /// Create from inner value
                pub fn new(value: #rust_type_tokens) -> Self {
                    Self(value)
                }
            }
        };

        // Generate streaming decode_prefix if streaming mode is enabled
        let decode_prefix_impl = if self.streaming_encode {
            quote! {
                impl #type_ident {
                    /// Streaming decode that handles truncated token sequences
                    pub fn decode_prefix(tokens: &mut impl Iterator<Item = u32>) -> PrefixDecodeResult<Self> {
                        // Use Self::decode() to ensure we use the type's own decoder
                        match <Self as crate::tokenizer::JsonTokenEncoder>::decode(tokens) {
                            Ok(instance) => PrefixDecodeResult {
                                instance,
                                fields_decoded: 1,
                                complete: true,
                                partial_field: None,
                            },
                            Err(_) => PrefixDecodeResult {
                                instance: Self::default(),
                                fields_decoded: 0,
                                complete: false,
                                partial_field: None,
                            },
                        }
                    }
                }
            }
        } else {
            quote! {}
        };

        // Generate JsonTokenEncoder implementation for the minimal object wrapper
        let implementation_tokens = quote! {
            #helper_methods

            impl crate::tokenizer::JsonTokenEncoder for #type_ident {
                fn encode(&self, tokens: &mut Vec<u32>) {
                    self.0.encode(tokens);
                }

                fn decode(tokens: &mut impl Iterator<Item = u32>) -> Result<Self, crate::tokenizer::DecodeError> {
                    let value = <#rust_type_tokens>::decode(tokens)?;
                    Ok(Self(value))
                }

                fn decode_strict(tokens: &mut impl Iterator<Item = u32>) -> Result<Self, crate::tokenizer::DecodeError> {
                    let value = <#rust_type_tokens>::decode_strict(tokens)?;
                    Ok(Self(value))
                }

                fn token_count(&self) -> usize {
                    self.0.token_count()
                }
            }

            #decode_prefix_impl
        };

        // Create type code generation
        let type_generation = TypeCodeGeneration::new_struct(
            type_name.clone(),
            definition_tokens,
            implementation_tokens,
        )
        .with_token_encoder_impl(true);

        crate::log_debug!("✅ Minimal object wrapper '{}' generated", type_name);
        Ok(type_generation)
    }

    /// Generate a simple type alias
    ///
    /// Creates: pub type A = B;
    ///
    /// Type aliases don't need JsonTokenEncoder implementation as they inherit
    /// the implementation from the target type.
    pub fn generate_type_alias(
        &self,
        type_name: &str,
        target_type: &str,
    ) -> TctResult<TypeCodeGeneration> {
        crate::log_debug!("🔗 Generating type alias: {} = {}", type_name, target_type);

        let type_ident = format_ident!("{}", type_name);
        let target_ident = format_ident!("{}", target_type);

        // Generate the type alias
        let definition_tokens = quote! {
            pub type #type_ident = #target_ident;
        };

        // Type aliases don't need JsonTokenEncoder implementation
        // They inherit it from the target type
        let implementation_tokens = quote! {};

        // Create type code generation
        let type_generation = TypeCodeGeneration::new_type_alias(
            type_name.to_string(),
            definition_tokens,
            implementation_tokens,
        );

        crate::log_debug!("✅ Type alias '{}' generated", type_name);
        Ok(type_generation)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generator::schema_discovery::{SchemaLocation, TypeKind};
    use crate::generator::type_analysis::TraitDerivability;
    use quote::ToTokens;
    use serde_json::{Map, json};
    use syn::punctuated::Punctuated;

    // Helper to create TypeDiscovery for testing
    fn create_test_type_discovery(
        name: &str,
        type_kind: TypeKind,
        schema: Map<String, Value>,
    ) -> TypeDiscovery {
        TypeDiscovery::new(
            name.to_string(),
            type_kind,
            SchemaLocation {
                json_path: format!("#/{}", name),
                description: format!("Test type {}", name),
                parent_type: None,
                property_name: None,
            },
            schema,
        )
    }

    fn parse_generated_file(tokens: proc_macro2::TokenStream) -> syn::File {
        match syn::parse2(tokens) {
            Ok(file) => file,
            Err(error) => panic!("generated tokens should parse as Rust items: {error}"),
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

    fn generated_type_alias<'a>(file: &'a syn::File, name: &str) -> &'a syn::ItemType {
        file.items
            .iter()
            .find_map(|item| match item {
                syn::Item::Type(item_type) if item_type.ident == name => Some(item_type),
                _ => None,
            })
            .unwrap_or_else(|| panic!("expected generated type alias {name}"))
    }

    fn tuple_struct_field_type(item_struct: &syn::ItemStruct) -> String {
        match &item_struct.fields {
            syn::Fields::Unnamed(fields) if fields.unnamed.len() == 1 => {
                fields.unnamed[0].ty.to_token_stream().to_string()
            }
            _ => panic!("expected single-field tuple struct {}", item_struct.ident),
        }
    }

    fn type_alias_target(item_type: &syn::ItemType) -> String {
        item_type.ty.to_token_stream().to_string()
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

    fn has_serde_transparent(attrs: &[syn::Attribute]) -> bool {
        attrs.iter().any(|attr| {
            if !attr.path().is_ident("serde") {
                return false;
            }

            let mut found = false;
            let _ = attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("transparent") {
                    found = true;
                }
                Ok(())
            });
            found
        })
    }

    fn impl_type_name(item_impl: &syn::ItemImpl) -> Option<&syn::Ident> {
        match item_impl.self_ty.as_ref() {
            syn::Type::Path(type_path) => {
                type_path.path.segments.last().map(|segment| &segment.ident)
            }
            _ => None,
        }
    }

    fn impl_has_method(item_impl: &syn::ItemImpl, method_name: &str) -> bool {
        item_impl.items.iter().any(|item| match item {
            syn::ImplItem::Fn(method) => method.sig.ident == method_name,
            _ => false,
        })
    }

    fn has_trait_impl_for_type(file: &syn::File, trait_name: &str, type_name: &str) -> bool {
        file.items.iter().any(|item| {
            let syn::Item::Impl(item_impl) = item else {
                return false;
            };
            let Some((_, trait_path, _)) = &item_impl.trait_ else {
                return false;
            };
            let trait_matches = trait_path
                .segments
                .last()
                .is_some_and(|segment| segment.ident == trait_name);
            let type_matches = impl_type_name(item_impl).is_some_and(|ident| ident == type_name);
            trait_matches && type_matches
        })
    }

    fn trait_impl_has_method(
        file: &syn::File,
        trait_name: &str,
        type_name: &str,
        method_name: &str,
    ) -> bool {
        file.items.iter().any(|item| {
            let syn::Item::Impl(item_impl) = item else {
                return false;
            };
            let Some((_, trait_path, _)) = &item_impl.trait_ else {
                return false;
            };
            let trait_matches = trait_path
                .segments
                .last()
                .is_some_and(|segment| segment.ident == trait_name);
            let type_matches = impl_type_name(item_impl).is_some_and(|ident| ident == type_name);
            trait_matches && type_matches && impl_has_method(item_impl, method_name)
        })
    }

    fn inherent_impl_has_method(file: &syn::File, type_name: &str, method_name: &str) -> bool {
        file.items.iter().any(|item| {
            let syn::Item::Impl(item_impl) = item else {
                return false;
            };
            if item_impl.trait_.is_some() {
                return false;
            }
            let type_matches = impl_type_name(item_impl).is_some_and(|ident| ident == type_name);
            type_matches && impl_has_method(item_impl, method_name)
        })
    }

    // === Phase 4.3: TypeAliasGenerator Coverage Tests ===
    // Batch 1: Array type alias tests

    #[test]
    fn test_generate_array_with_primitive_elements() {
        // Test array generation with various primitive element types
        let generator = TypeAliasGenerator::new();

        // Test String array
        let string_schema = json!({
            "type": "array",
            "items": {"type": "string"}
        });
        let type_discovery = create_test_type_discovery(
            "StringArray",
            TypeKind::ArrayOfComplex,
            string_schema.as_object().unwrap().clone(),
        );

        let result = generator.generate_array_type_alias(&type_discovery, None);
        assert!(result.is_ok(), "String array generation should succeed");

        let generated = result.unwrap();
        assert_eq!(generated.type_name, "StringArray");

        let definition_file = parse_generated_file(generated.definition_tokens);
        let string_array = generated_struct(&definition_file, "StringArray");
        assert_eq!(tuple_struct_field_type(string_array), "Vec < String >");
        assert!(has_serde_transparent(&string_array.attrs));

        let implementation_file = parse_generated_file(generated.implementation_tokens);
        assert!(has_trait_impl_for_type(
            &implementation_file,
            "JsonTokenEncoder",
            "StringArray"
        ));
        assert!(inherent_impl_has_method(
            &implementation_file,
            "StringArray",
            "len"
        ));
        assert!(inherent_impl_has_method(
            &implementation_file,
            "StringArray",
            "is_empty"
        ));
        assert!(inherent_impl_has_method(
            &implementation_file,
            "StringArray",
            "iter"
        ));

        // Test integer array
        let int_schema = json!({
            "type": "array",
            "items": {"type": "integer"}
        });
        let type_discovery = create_test_type_discovery(
            "IntArray",
            TypeKind::ArrayOfComplex,
            int_schema.as_object().unwrap().clone(),
        );

        let result = generator.generate_array_type_alias(&type_discovery, None);
        assert!(result.is_ok(), "Integer array generation should succeed");

        let generated = result.unwrap();
        let definition_file = parse_generated_file(generated.definition_tokens);
        let int_array = generated_struct(&definition_file, "IntArray");
        assert_eq!(
            tuple_struct_field_type(int_array),
            "Vec < crate :: tokenizer :: JsonInteger >"
        );

        // Fully i64-bounded integer arrays can keep the compact signed type.
        let bounded_int_schema = json!({
            "type": "array",
            "items": {"type": "integer", "minimum": -10, "maximum": 10}
        });
        let type_discovery = create_test_type_discovery(
            "BoundedIntArray",
            TypeKind::ArrayOfComplex,
            bounded_int_schema.as_object().unwrap().clone(),
        );

        let result = generator.generate_array_type_alias(&type_discovery, None);
        assert!(
            result.is_ok(),
            "Bounded integer array generation should succeed"
        );

        let generated = result.unwrap();
        let definition_file = parse_generated_file(generated.definition_tokens);
        let bounded_int_array = generated_struct(&definition_file, "BoundedIntArray");
        assert_eq!(tuple_struct_field_type(bounded_int_array), "Vec < i64 >");

        // Test boolean array
        let bool_schema = json!({
            "type": "array",
            "items": {"type": "boolean"}
        });
        let type_discovery = create_test_type_discovery(
            "BoolArray",
            TypeKind::ArrayOfComplex,
            bool_schema.as_object().unwrap().clone(),
        );

        let result = generator.generate_array_type_alias(&type_discovery, None);
        assert!(result.is_ok(), "Boolean array generation should succeed");

        let generated = result.unwrap();
        let definition_file = parse_generated_file(generated.definition_tokens);
        let bool_array = generated_struct(&definition_file, "BoolArray");
        assert_eq!(tuple_struct_field_type(bool_array), "Vec < bool >");
    }

    #[test]
    fn test_generate_array_with_reference_elements() {
        // Test array generation with $ref element types
        let generator = TypeAliasGenerator::new();

        let schema = json!({
            "type": "array",
            "items": {"$ref": "#/definitions/User"}
        });

        let type_discovery = create_test_type_discovery(
            "UserArray",
            TypeKind::ArrayOfComplex,
            schema.as_object().unwrap().clone(),
        );

        let result = generator.generate_array_type_alias(&type_discovery, None);
        assert!(result.is_ok(), "Reference array generation should succeed");

        let generated = result.unwrap();
        assert_eq!(generated.type_name, "UserArray");

        let definition_file = parse_generated_file(generated.definition_tokens);
        let user_array = generated_struct(&definition_file, "UserArray");
        assert_eq!(tuple_struct_field_type(user_array), "Vec < User >");

        let implementation_file = parse_generated_file(generated.implementation_tokens);
        assert!(has_trait_impl_for_type(
            &implementation_file,
            "JsonTokenEncoder",
            "UserArray"
        ));
        assert!(trait_impl_has_method(
            &implementation_file,
            "JsonTokenEncoder",
            "UserArray",
            "encode"
        ));
        assert!(trait_impl_has_method(
            &implementation_file,
            "JsonTokenEncoder",
            "UserArray",
            "decode"
        ));
        assert!(trait_impl_has_method(
            &implementation_file,
            "JsonTokenEncoder",
            "UserArray",
            "token_count"
        ));
    }

    #[test]
    fn test_generate_array_with_escaped_definition_reference_elements() {
        let generator = TypeAliasGenerator::new();

        let schema = json!({
            "type": "array",
            "items": {"$ref": "#/definitions/User~1Record"}
        });

        let type_discovery = create_test_type_discovery(
            "UserRecordArray",
            TypeKind::ArrayOfComplex,
            schema.as_object().unwrap().clone(),
        );

        let generated = generator
            .generate_array_type_alias(&type_discovery, None)
            .unwrap();
        let definition_file = parse_generated_file(generated.definition_tokens);
        let user_record_array = generated_struct(&definition_file, "UserRecordArray");
        assert_eq!(
            tuple_struct_field_type(user_record_array),
            "Vec < UserRecord >",
            "escaped definition refs should derive the decoded type name"
        );
    }

    #[test]
    fn test_generate_array_with_value_type_no_ord() {
        // Test that serde_json::Value arrays don't derive Ord
        let generator = TypeAliasGenerator::new();

        // Array with no items info - defaults to serde_json::Value
        let schema = json!({
            "type": "array"
        });

        let type_discovery = create_test_type_discovery(
            "GenericArray",
            TypeKind::ArrayOfComplex,
            schema.as_object().unwrap().clone(),
        );

        // With trait derivability that allows Ord (but should be disabled for Value)
        let traits = TraitDerivability {
            can_derive_eq: true,
            can_derive_ord: true,
            can_derive_hash: true,
            reasoning: "Test trait derivability".to_string(),
            confidence: 1.0,
            blocking_types: std::collections::HashSet::new(),
        };

        let result = generator.generate_array_type_alias(&type_discovery, Some(&traits));
        assert!(result.is_ok());

        let generated = result.unwrap();
        let definition_file = parse_generated_file(generated.definition_tokens);
        let generic_array = generated_struct(&definition_file, "GenericArray");
        let traits = derive_traits(&generic_array.attrs);

        // Should use serde_json::Value as element type
        assert_eq!(
            tuple_struct_field_type(generic_array),
            "Vec < serde_json :: Value >"
        );

        // Should derive Eq and Hash, but NOT Ord (Value doesn't support Ord)
        assert!(traits.iter().any(|trait_name| trait_name == "Eq"));
        assert!(traits.iter().any(|trait_name| trait_name == "Hash"));
        assert!(!traits.iter().any(|trait_name| trait_name == "Ord"));

        // Without trait derivability - should use conservative defaults (no extra traits)
        let result = generator.generate_array_type_alias(&type_discovery, None);
        assert!(result.is_ok());

        let generated = result.unwrap();
        let definition_file = parse_generated_file(generated.definition_tokens);
        let generic_array = generated_struct(&definition_file, "GenericArray");
        assert!(
            derive_traits(&generic_array.attrs)
                .iter()
                .any(|trait_name| trait_name == "PartialEq"),
            "Should always have PartialEq"
        );
    }

    #[test]
    fn test_array_value_detection_uses_schema_metadata() {
        let string_schema = json!({
            "type": "array",
            "items": {"type": "string"}
        });
        let string_array = create_test_type_discovery(
            "StringArray",
            TypeKind::ArrayOfComplex,
            string_schema.as_object().unwrap().clone(),
        );
        assert!(!TypeAliasGenerator::array_items_use_json_value(
            &string_array
        ));

        let local_ref_schema = json!({
            "type": "array",
            "items": {"$ref": "#/definitions/User"}
        });
        let local_ref_array = create_test_type_discovery(
            "UserArray",
            TypeKind::ArrayOfComplex,
            local_ref_schema.as_object().unwrap().clone(),
        );
        assert!(!TypeAliasGenerator::array_items_use_json_value(
            &local_ref_array
        ));

        let unknown_schema = json!({
            "type": "array",
            "items": {"type": "object"}
        });
        let unknown_array = create_test_type_discovery(
            "UnknownArray",
            TypeKind::ArrayOfComplex,
            unknown_schema.as_object().unwrap().clone(),
        );
        assert!(TypeAliasGenerator::array_items_use_json_value(
            &unknown_array
        ));

        let external_ref_schema = json!({
            "type": "array",
            "items": {"$ref": "https://example.test/schema.json#/User"}
        });
        let external_ref_array = create_test_type_discovery(
            "ExternalRefArray",
            TypeKind::ArrayOfComplex,
            external_ref_schema.as_object().unwrap().clone(),
        );
        assert!(TypeAliasGenerator::array_items_use_json_value(
            &external_ref_array
        ));
    }

    // Batch 2: Primitive wrapper tests

    #[test]
    fn test_generate_primitive_wrappers_all_types() {
        // Test primitive wrapper generation for all supported types
        let generator = TypeAliasGenerator::new();

        // Test String wrapper
        let string_schema = json!({"type": "string"});
        let type_discovery = create_test_type_discovery(
            "StringWrapper",
            TypeKind::PrimitiveWrapper {
                primitive_type: "String".to_string(),
            },
            string_schema.as_object().unwrap().clone(),
        );

        let result = generator.generate_primitive_wrapper(&type_discovery, "String", None);
        assert!(result.is_ok());

        let generated = result.unwrap();
        let definition_file = parse_generated_file(generated.definition_tokens);
        let string_wrapper = generated_struct(&definition_file, "StringWrapper");
        assert_eq!(tuple_struct_field_type(string_wrapper), "String");

        let implementation_file = parse_generated_file(generated.implementation_tokens);
        assert!(inherent_impl_has_method(
            &implementation_file,
            "StringWrapper",
            "value"
        ));
        assert!(inherent_impl_has_method(
            &implementation_file,
            "StringWrapper",
            "into_value"
        ));
        assert!(inherent_impl_has_method(
            &implementation_file,
            "StringWrapper",
            "new"
        ));

        // Test i64 wrapper
        let int_schema = json!({"type": "integer"});
        let type_discovery = create_test_type_discovery(
            "IntWrapper",
            TypeKind::PrimitiveWrapper {
                primitive_type: "i64".to_string(),
            },
            int_schema.as_object().unwrap().clone(),
        );

        let result = generator.generate_primitive_wrapper(&type_discovery, "i64", None);
        assert!(result.is_ok());

        let generated = result.unwrap();
        let definition_file = parse_generated_file(generated.definition_tokens);
        let int_wrapper = generated_struct(&definition_file, "IntWrapper");
        assert_eq!(tuple_struct_field_type(int_wrapper), "i64");

        // Test f64 wrapper
        let result = generator.generate_primitive_wrapper(&type_discovery, "f64", None);
        assert!(result.is_ok());

        let number_schema = json!({"type": "number"});
        let type_discovery = create_test_type_discovery(
            "NumberWrapper",
            TypeKind::PrimitiveWrapper {
                primitive_type: "serde_json::Number".to_string(),
            },
            number_schema.as_object().unwrap().clone(),
        );

        let result =
            generator.generate_primitive_wrapper(&type_discovery, "serde_json::Number", None);
        assert!(result.is_ok());

        let generated = result.unwrap();
        let definition_file = parse_generated_file(generated.definition_tokens);
        let number_wrapper = generated_struct(&definition_file, "NumberWrapper");
        assert_eq!(
            tuple_struct_field_type(number_wrapper),
            "serde_json :: Number",
            "number wrappers must not fall back to String"
        );

        // Test bool wrapper
        let bool_schema = json!({"type": "boolean"});
        let type_discovery = create_test_type_discovery(
            "BoolWrapper",
            TypeKind::PrimitiveWrapper {
                primitive_type: "bool".to_string(),
            },
            bool_schema.as_object().unwrap().clone(),
        );

        let result = generator.generate_primitive_wrapper(&type_discovery, "bool", None);
        assert!(result.is_ok());

        let generated = result.unwrap();
        let definition_file = parse_generated_file(generated.definition_tokens);
        let bool_wrapper = generated_struct(&definition_file, "BoolWrapper");
        assert_eq!(tuple_struct_field_type(bool_wrapper), "bool");
    }

    #[test]
    fn test_primitive_wrapper_trait_derivability() {
        // Test conditional trait derivation for primitive wrappers
        let generator = TypeAliasGenerator::new();

        let schema = json!({"type": "string"});
        let type_discovery = create_test_type_discovery(
            "StringWrapper",
            TypeKind::PrimitiveWrapper {
                primitive_type: "String".to_string(),
            },
            schema.as_object().unwrap().clone(),
        );

        // With full trait derivability
        let traits = TraitDerivability {
            can_derive_eq: true,
            can_derive_ord: true,
            can_derive_hash: true,
            reasoning: "String supports all traits".to_string(),
            confidence: 1.0,
            blocking_types: std::collections::HashSet::new(),
        };

        let result = generator.generate_primitive_wrapper(&type_discovery, "String", Some(&traits));
        assert!(result.is_ok());

        let generated = result.unwrap();
        let definition_file = parse_generated_file(generated.definition_tokens);
        let string_wrapper = generated_struct(&definition_file, "StringWrapper");
        let traits = derive_traits(&string_wrapper.attrs);

        // Should derive all traits
        assert!(traits.iter().any(|trait_name| trait_name == "Eq"));
        assert!(traits.iter().any(|trait_name| trait_name == "Ord"));
        assert!(traits.iter().any(|trait_name| trait_name == "Hash"));
        assert!(traits.iter().any(|trait_name| trait_name == "PartialEq"));

        // Without trait derivability - conservative defaults
        let result = generator.generate_primitive_wrapper(&type_discovery, "String", None);
        assert!(result.is_ok());

        let generated = result.unwrap();
        let definition_file = parse_generated_file(generated.definition_tokens);
        let string_wrapper = generated_struct(&definition_file, "StringWrapper");
        let traits = derive_traits(&string_wrapper.attrs);

        // Should only have PartialEq
        assert!(traits.iter().any(|trait_name| trait_name == "PartialEq"));
    }

    // Batch 3: Minimal object & type alias tests

    #[test]
    fn test_generate_minimal_object_wrapper() {
        // Test minimal object wrapper (serde_json::Value wrapper)
        let generator = TypeAliasGenerator::new();

        let schema = json!({"type": "object"});
        let type_discovery = create_test_type_discovery(
            "GenericObject",
            TypeKind::MinimalObjectWrapper,
            schema.as_object().unwrap().clone(),
        );

        // With traits (but serde_json::Value doesn't support Eq/Ord/Hash)
        let traits = TraitDerivability {
            can_derive_eq: true,
            can_derive_ord: true,
            can_derive_hash: true,
            reasoning: "Test traits".to_string(),
            confidence: 1.0,
            blocking_types: std::collections::HashSet::new(),
        };

        let result = generator.generate_minimal_object_wrapper(&type_discovery, Some(&traits));
        assert!(result.is_ok());

        let generated = result.unwrap();
        let definition_file = parse_generated_file(generated.definition_tokens);
        let generic_object = generated_struct(&definition_file, "GenericObject");
        let traits = derive_traits(&generic_object.attrs);

        // Should use serde_json::Value
        assert_eq!(
            tuple_struct_field_type(generic_object),
            "serde_json :: Value"
        );

        // Should have PartialEq (always present)
        assert!(traits.iter().any(|trait_name| trait_name == "PartialEq"));

        // Should NOT derive Eq/Ord/Hash (Value doesn't support them)
        assert!(
            !traits.iter().any(|trait_name| trait_name == "Eq"),
            "Should not derive Eq trait for serde_json::Value wrapper"
        );
        assert!(!traits.iter().any(|trait_name| trait_name == "Ord"));
        assert!(!traits.iter().any(|trait_name| trait_name == "Hash"));

        // Should have helper methods
        let implementation_file = parse_generated_file(generated.implementation_tokens);
        assert!(inherent_impl_has_method(
            &implementation_file,
            "GenericObject",
            "value"
        ));
        assert!(inherent_impl_has_method(
            &implementation_file,
            "GenericObject",
            "into_value"
        ));
        assert!(inherent_impl_has_method(
            &implementation_file,
            "GenericObject",
            "new"
        ));
        assert!(has_trait_impl_for_type(
            &implementation_file,
            "JsonTokenEncoder",
            "GenericObject"
        ));
    }

    #[test]
    fn test_generate_simple_type_alias() {
        // Test simple type alias generation
        let generator = TypeAliasGenerator::new();

        let result = generator.generate_type_alias("UserId", "String");
        assert!(result.is_ok());

        let generated = result.unwrap();
        assert_eq!(generated.type_name, "UserId");

        let definition_file = parse_generated_file(generated.definition_tokens);
        let user_id = generated_type_alias(&definition_file, "UserId");
        assert!(matches!(user_id.vis, syn::Visibility::Public(_)));
        assert_eq!(type_alias_target(user_id), "String");

        // Type aliases don't have implementation (inherit from target type)
        let impl_str = generated.implementation_tokens.to_string();
        assert!(impl_str.is_empty() || impl_str.trim().is_empty());

        // Test another type alias
        let result = generator.generate_type_alias("Count", "I32");
        assert!(result.is_ok());

        let generated = result.unwrap();
        let definition_file = parse_generated_file(generated.definition_tokens);
        let count = generated_type_alias(&definition_file, "Count");
        assert_eq!(type_alias_target(count), "I32");
    }

    // Batch 4: Integration test

    #[test]
    fn test_type_alias_generator_code_quality() {
        // Integration test: verify generated code quality and structure
        let generator = TypeAliasGenerator::new();

        // Test array generation code quality
        let array_schema = json!({
            "type": "array",
            "items": {"type": "string"}
        });
        let type_discovery = create_test_type_discovery(
            "TestArray",
            TypeKind::ArrayOfComplex,
            array_schema.as_object().unwrap().clone(),
        );

        let result = generator.generate_array_type_alias(&type_discovery, None);
        assert!(result.is_ok());

        let generated = result.unwrap();

        // Verify definition has proper structure
        let definition_file = parse_generated_file(generated.definition_tokens);
        let test_array = generated_struct(&definition_file, "TestArray");
        let traits = derive_traits(&test_array.attrs);
        assert!(traits.iter().any(|trait_name| trait_name == "Debug"));
        assert!(traits.iter().any(|trait_name| trait_name == "Clone"));
        assert!(has_serde_transparent(&test_array.attrs));

        // Verify implementation has proper structure
        let implementation_file = parse_generated_file(generated.implementation_tokens);
        assert!(has_trait_impl_for_type(
            &implementation_file,
            "JsonTokenEncoder",
            "TestArray"
        ));
        assert!(trait_impl_has_method(
            &implementation_file,
            "JsonTokenEncoder",
            "TestArray",
            "encode"
        ));
        assert!(trait_impl_has_method(
            &implementation_file,
            "JsonTokenEncoder",
            "TestArray",
            "decode"
        ));
        assert!(trait_impl_has_method(
            &implementation_file,
            "JsonTokenEncoder",
            "TestArray",
            "token_count"
        ));

        // Test primitive wrapper code quality
        let prim_schema = json!({"type": "string"});
        let type_discovery = create_test_type_discovery(
            "TestWrapper",
            TypeKind::PrimitiveWrapper {
                primitive_type: "String".to_string(),
            },
            prim_schema.as_object().unwrap().clone(),
        );

        let result = generator.generate_primitive_wrapper(&type_discovery, "String", None);
        assert!(result.is_ok());

        let generated = result.unwrap();
        let implementation_file = parse_generated_file(generated.implementation_tokens);

        // Should have both helper methods and JsonTokenEncoder
        assert!(inherent_impl_has_method(
            &implementation_file,
            "TestWrapper",
            "value"
        ));
        assert!(inherent_impl_has_method(
            &implementation_file,
            "TestWrapper",
            "into_value"
        ));
        assert!(inherent_impl_has_method(
            &implementation_file,
            "TestWrapper",
            "new"
        ));
    }

    // === Phase 1: O3 Primitive Wrapper Tests ===
    // Testing generate_primitive_wrapper_with_decision (lines 339-703)

    #[test]
    fn test_o3_constant_integer_optimization() {
        // Test ConstantInteger encoding decision - should generate 0-token encode/decode
        use crate::generator::encoding_decision::EncodingDecision;

        let generator = TypeAliasGenerator::new();
        let schema = json!({"type": "integer", "const": 42});
        let type_discovery = create_test_type_discovery(
            "ConstantInt",
            TypeKind::PrimitiveWrapper {
                primitive_type: "i64".to_string(),
            },
            schema.as_object().unwrap().clone(),
        );

        let decision = EncodingDecision::ConstantInteger { value: 42 };

        let result = generator.generate_primitive_wrapper_with_decision(
            &type_discovery,
            "i64",
            None,
            Some(&decision),
        );
        assert!(
            result.is_ok(),
            "ConstantInteger optimization should succeed"
        );

        let generated = result.unwrap();
        let impl_str = generated.implementation_tokens.to_string();

        // Should have 0-token encoding (constant optimization)
        assert!(
            impl_str.contains("0"),
            "Should indicate 0 tokens for constant"
        );
        assert!(impl_str.contains("encode"), "Should have encode method");
        assert!(impl_str.contains("decode"), "Should have decode method");
        assert!(
            impl_str.contains("token_count"),
            "Should have token_count method"
        );
    }

    #[test]
    fn test_o3_format_optimized_int32() {
        // Test FormatOptimized Int32 string encoding
        use crate::generator::encoding_decision::{EncodingDecision, StringFormat};

        let generator = TypeAliasGenerator::new();
        let schema = json!({"type": "string", "format": "int32"});
        let type_discovery = create_test_type_discovery(
            "Int32String",
            TypeKind::PrimitiveWrapper {
                primitive_type: "String".to_string(),
            },
            schema.as_object().unwrap().clone(),
        );

        let decision = EncodingDecision::FormatOptimized {
            format: StringFormat::Int32,
            fallback_to_string: false,
        };

        let result = generator.generate_primitive_wrapper_with_decision(
            &type_discovery,
            "String",
            None,
            Some(&decision),
        );
        assert!(result.is_ok(), "Int32 format optimization should succeed");

        let generated = result.unwrap();
        let impl_str = generated.implementation_tokens.to_string();

        // Should parse canonical strings but preserve malformed or non-canonical strings.
        assert!(impl_str.contains("i32"), "Should reference i32 type");
        assert!(impl_str.contains("parse"), "Should parse string to i32");
        assert!(
            impl_str.contains("String :: decode"),
            "Should fall back to string decode"
        );
        assert!(
            !impl_str.contains(". expect"),
            "Format encoding must not panic on invalid wrapper values"
        );
    }

    #[test]
    fn test_o3_format_optimized_int64() {
        // Test FormatOptimized Int64 string encoding
        use crate::generator::encoding_decision::{EncodingDecision, StringFormat};

        let generator = TypeAliasGenerator::new();
        let schema = json!({"type": "string", "format": "int64"});
        let type_discovery = create_test_type_discovery(
            "Int64String",
            TypeKind::PrimitiveWrapper {
                primitive_type: "String".to_string(),
            },
            schema.as_object().unwrap().clone(),
        );

        let decision = EncodingDecision::FormatOptimized {
            format: StringFormat::Int64,
            fallback_to_string: false,
        };

        let result = generator.generate_primitive_wrapper_with_decision(
            &type_discovery,
            "String",
            None,
            Some(&decision),
        );
        assert!(result.is_ok(), "Int64 format optimization should succeed");

        let generated = result.unwrap();
        let impl_str = generated.implementation_tokens.to_string();

        // Should parse canonical strings but preserve malformed or non-canonical strings.
        assert!(impl_str.contains("i64"), "Should reference i64 type");
        assert!(impl_str.contains("parse"), "Should parse string to i64");
        assert!(
            impl_str.contains("String :: decode"),
            "Should fall back to string decode"
        );
        assert!(
            !impl_str.contains(". expect"),
            "Format encoding must not panic on invalid wrapper values"
        );
    }

    #[test]
    fn test_o3_format_optimized_double() {
        // Test FormatOptimized Double string encoding
        use crate::generator::encoding_decision::{EncodingDecision, StringFormat};

        let generator = TypeAliasGenerator::new();
        let schema = json!({"type": "string", "format": "double"});
        let type_discovery = create_test_type_discovery(
            "DoubleString",
            TypeKind::PrimitiveWrapper {
                primitive_type: "String".to_string(),
            },
            schema.as_object().unwrap().clone(),
        );

        let decision = EncodingDecision::FormatOptimized {
            format: StringFormat::Double,
            fallback_to_string: false,
        };

        let result = generator.generate_primitive_wrapper_with_decision(
            &type_discovery,
            "String",
            None,
            Some(&decision),
        );
        assert!(result.is_ok(), "Double format optimization should succeed");

        let generated = result.unwrap();
        let impl_str = generated.implementation_tokens.to_string();

        // Should parse canonical strings but preserve malformed or non-canonical strings.
        assert!(impl_str.contains("f64"), "Should reference f64 type");
        assert!(impl_str.contains("parse"), "Should parse string to f64");
        assert!(impl_str.contains("to_bits"), "Should convert f64 to bits");
        assert!(impl_str.contains("from_bits"), "Should convert bits back");
        assert!(
            impl_str.contains("String :: decode"),
            "Should fall back to string decode"
        );
        assert!(
            !impl_str.contains(". expect"),
            "Format encoding must not panic on invalid wrapper values"
        );
    }

    #[test]
    fn test_o3_format_optimized_byte() {
        // Test FormatOptimized Byte (base64) string encoding
        // FIXED: Now uses regular string encoding to respect vocabulary constraint
        use crate::generator::encoding_decision::{EncodingDecision, StringFormat};

        let generator = TypeAliasGenerator::new();
        let schema = json!({"type": "string", "format": "byte"});
        let type_discovery = create_test_type_discovery(
            "ByteString",
            TypeKind::PrimitiveWrapper {
                primitive_type: "String".to_string(),
            },
            schema.as_object().unwrap().clone(),
        );

        let decision = EncodingDecision::FormatOptimized {
            format: StringFormat::Byte,
            fallback_to_string: false,
        };

        let result = generator.generate_primitive_wrapper_with_decision(
            &type_discovery,
            "String",
            None,
            Some(&decision),
        );
        assert!(result.is_ok(), "Byte format should use string encoding");

        let generated = result.unwrap();
        let impl_str = generated.implementation_tokens.to_string();

        // Should use regular string encoding (delegates to String::encode)
        assert!(
            impl_str.contains("self . 0 . encode"),
            "Should delegate to String::encode"
        );
        assert!(
            impl_str.contains("String :: decode"),
            "Should delegate to String::decode"
        );
        assert!(
            impl_str.contains("self . 0 . token_count"),
            "Should delegate to String::token_count"
        );
    }

    #[test]
    fn test_o3_format_optimized_datetime() {
        // Test FormatOptimized DateTime string encoding
        use crate::generator::encoding_decision::{EncodingDecision, StringFormat};

        let generator = TypeAliasGenerator::new();
        let schema = json!({"type": "string", "format": "date-time"});
        let type_discovery = create_test_type_discovery(
            "DateTimeString",
            TypeKind::PrimitiveWrapper {
                primitive_type: "String".to_string(),
            },
            schema.as_object().unwrap().clone(),
        );

        let decision = EncodingDecision::FormatOptimized {
            format: StringFormat::DateTime,
            fallback_to_string: false,
        };

        let result = generator.generate_primitive_wrapper_with_decision(
            &type_discovery,
            "String",
            None,
            Some(&decision),
        );
        assert!(
            result.is_ok(),
            "DateTime format optimization should succeed"
        );

        let generated = result.unwrap();
        let impl_str = generated.implementation_tokens.to_string();

        // Should fast-path canonical RFC 3339 timestamps but preserve raw strings.
        assert!(impl_str.contains("chrono"), "Should use chrono library");
        assert!(impl_str.contains("DateTime"), "Should parse DateTime");
        assert!(impl_str.contains("timestamp"), "Should use Unix timestamp");
        assert!(
            impl_str.contains("to_rfc3339"),
            "Should convert back to RFC 3339"
        );
        assert!(
            impl_str.contains("String :: decode"),
            "Should fall back to string decode"
        );
        assert!(
            !impl_str.contains(". expect"),
            "Format encoding must not panic on invalid wrapper values"
        );
    }

    #[test]
    fn test_o3_format_optimized_unknown_format_fallback() {
        // Test FormatOptimized with unknown format - should fall back to standard encoding
        use crate::generator::encoding_decision::{EncodingDecision, StringFormat};

        let generator = TypeAliasGenerator::new();
        let schema = json!({"type": "string", "format": "uuid"});
        let type_discovery = create_test_type_discovery(
            "UuidString",
            TypeKind::PrimitiveWrapper {
                primitive_type: "String".to_string(),
            },
            schema.as_object().unwrap().clone(),
        );

        // UUID is not yet optimized, should fall back
        let decision = EncodingDecision::FormatOptimized {
            format: StringFormat::Uuid,
            fallback_to_string: false,
        };

        let result = generator.generate_primitive_wrapper_with_decision(
            &type_discovery,
            "String",
            None,
            Some(&decision),
        );
        assert!(
            result.is_ok(),
            "Unknown format should fall back to standard encoding"
        );

        let generated = result.unwrap();
        let implementation_file = parse_generated_file(generated.implementation_tokens);

        // Should use standard string encoding (self.0.encode)
        assert!(trait_impl_has_method(
            &implementation_file,
            "JsonTokenEncoder",
            "UuidString",
            "encode"
        ));
        assert!(trait_impl_has_method(
            &implementation_file,
            "JsonTokenEncoder",
            "UuidString",
            "decode"
        ));
        assert!(
            has_trait_impl_for_type(&implementation_file, "JsonTokenEncoder", "UuidString"),
            "Should implement JsonTokenEncoder"
        );
    }

    #[test]
    fn test_o3_wrapper_no_decision_standard_encoding() {
        // Test primitive wrapper with O3 function but no encoding decision
        let generator = TypeAliasGenerator::new();
        let schema = json!({"type": "string"});
        let type_discovery = create_test_type_discovery(
            "StandardString",
            TypeKind::PrimitiveWrapper {
                primitive_type: "String".to_string(),
            },
            schema.as_object().unwrap().clone(),
        );

        // No encoding decision - should use standard encoding
        let result = generator.generate_primitive_wrapper_with_decision(
            &type_discovery,
            "String",
            None,
            None,
        );
        assert!(result.is_ok(), "Standard encoding should succeed");

        let generated = result.unwrap();
        let impl_str = generated.implementation_tokens.to_string();

        // Should delegate to self.0.encode (standard encoding)
        assert!(impl_str.contains("self . 0 . encode"));
        assert!(impl_str.contains("self . 0 . token_count"));
    }

    #[test]
    fn test_o3_wrapper_trait_derivability() {
        // Test that O3 wrappers respect trait derivability
        use crate::generator::encoding_decision::{EncodingDecision, StringFormat};

        let generator = TypeAliasGenerator::new();
        let schema = json!({"type": "string", "format": "int32"});
        let type_discovery = create_test_type_discovery(
            "Int32Wrapper",
            TypeKind::PrimitiveWrapper {
                primitive_type: "String".to_string(),
            },
            schema.as_object().unwrap().clone(),
        );

        let traits = TraitDerivability {
            can_derive_eq: true,
            can_derive_ord: true,
            can_derive_hash: true,
            reasoning: "Int32 string supports all traits".to_string(),
            confidence: 1.0,
            blocking_types: std::collections::HashSet::new(),
        };

        let decision = EncodingDecision::FormatOptimized {
            format: StringFormat::Int32,
            fallback_to_string: false,
        };

        let result = generator.generate_primitive_wrapper_with_decision(
            &type_discovery,
            "String",
            Some(&traits),
            Some(&decision),
        );
        assert!(result.is_ok());

        let generated = result.unwrap();
        let def_str = generated.definition_tokens.to_string();

        // Should derive all traits
        assert!(def_str.contains("Eq"), "Should derive Eq");
        assert!(def_str.contains("Ord"), "Should derive Ord");
        assert!(def_str.contains("Hash"), "Should derive Hash");
    }

    // === Phase 2: Edge Case Tests ===

    #[test]
    fn test_array_with_unknown_primitive_type() {
        // Test array with unknown primitive item type - should default to serde_json::Value
        let generator = TypeAliasGenerator::new();

        let schema = json!({
            "type": "array",
            "items": {"type": "unknown_type"}
        });

        let type_discovery = create_test_type_discovery(
            "UnknownArray",
            TypeKind::ArrayOfComplex,
            schema.as_object().unwrap().clone(),
        );

        let result = generator.generate_array_type_alias(&type_discovery, None);
        assert!(
            result.is_ok(),
            "Should handle unknown primitive type gracefully"
        );

        let generated = result.unwrap();
        let definition_file = parse_generated_file(generated.definition_tokens);
        let unknown_array = generated_struct(&definition_file, "UnknownArray");

        // Should default to serde_json::Value for unknown types
        assert_eq!(
            tuple_struct_field_type(unknown_array),
            "Vec < serde_json :: Value >",
            "Should fall back to serde_json::Value for unknown item types"
        );
    }

    #[test]
    fn test_array_with_unresolved_reference() {
        // Test array with malformed $ref - should fall back to serde_json::Value
        let generator = TypeAliasGenerator::new();

        let schema = json!({
            "type": "array",
            "items": {"$ref": "malformed/reference/without/hash"}
        });

        let type_discovery = create_test_type_discovery(
            "UnresolvedRefArray",
            TypeKind::ArrayOfComplex,
            schema.as_object().unwrap().clone(),
        );

        let result = generator.generate_array_type_alias(&type_discovery, None);
        assert!(
            result.is_ok(),
            "Should handle unresolved references gracefully"
        );

        let generated = result.unwrap();
        let definition_file = parse_generated_file(generated.definition_tokens);
        let unresolved_ref_array = generated_struct(&definition_file, "UnresolvedRefArray");

        // Should fall back to serde_json::Value for unresolved references
        assert_eq!(
            tuple_struct_field_type(unresolved_ref_array),
            "Vec < serde_json :: Value >",
            "Should fall back to serde_json::Value for unresolved $ref"
        );
    }

    #[test]
    fn test_primitive_wrapper_unknown_type_fallback() {
        // Test primitive wrapper with unknown type - should default to String
        let generator = TypeAliasGenerator::new();

        let schema = json!({"type": "unknown"});
        let type_discovery = create_test_type_discovery(
            "UnknownWrapper",
            TypeKind::PrimitiveWrapper {
                primitive_type: "UnknownType".to_string(),
            },
            schema.as_object().unwrap().clone(),
        );

        let result = generator.generate_primitive_wrapper(&type_discovery, "UnknownType", None);
        assert!(
            result.is_ok(),
            "Should handle unknown primitive type gracefully"
        );

        let generated = result.unwrap();
        let def_str = generated.definition_tokens.to_string();

        // Should fall back to String for unknown primitive types
        assert!(
            def_str.contains("String"),
            "Should fall back to String for unknown primitive types"
        );
    }

    #[test]
    fn test_streaming_array_generates_decode_prefix() {
        // Streaming mode should generate decode_prefix for array type aliases
        let generator = TypeAliasGenerator::with_streaming_encode(true);

        let schema = json!({
            "type": "array",
            "items": {"type": "string"}
        });
        let type_discovery = create_test_type_discovery(
            "StreamingStringArray",
            TypeKind::ArrayOfComplex,
            schema.as_object().unwrap().clone(),
        );

        let result = generator.generate_array_type_alias(&type_discovery, None);
        assert!(result.is_ok());

        let generated = result.unwrap();
        let implementation_file = parse_generated_file(generated.implementation_tokens);

        // Verify decode_prefix is generated
        assert!(
            inherent_impl_has_method(
                &implementation_file,
                "StreamingStringArray",
                "decode_prefix"
            ),
            "Should generate decode_prefix function for streaming array"
        );
    }

    #[test]
    fn test_streaming_array_handles_partial_decode() {
        // Verify streaming array decode_prefix implementation handles partial sequences
        let generator = TypeAliasGenerator::with_streaming_encode(true);

        let schema = json!({
            "type": "array",
            "items": {"type": "integer"}
        });
        let type_discovery = create_test_type_discovery(
            "StreamingIntArray",
            TypeKind::ArrayOfComplex,
            schema.as_object().unwrap().clone(),
        );

        let result = generator.generate_array_type_alias(&type_discovery, None);
        assert!(result.is_ok());

        let generated = result.unwrap();
        let impl_str = generated.implementation_tokens.to_string();

        // Verify partial decode handling
        assert!(
            impl_str.contains("complete: false") || impl_str.contains("complete : false"),
            "Should set complete: false when decode fails partway"
        );
        assert!(
            impl_str.contains("partial_field") || impl_str.contains("partial_field"),
            "Should track partial_field for array elements"
        );
    }

    #[test]
    fn test_non_streaming_array_no_decode_prefix() {
        // Non-streaming mode should NOT generate decode_prefix for arrays
        let generator = TypeAliasGenerator::new();

        let schema = json!({
            "type": "array",
            "items": {"type": "string"}
        });
        let type_discovery = create_test_type_discovery(
            "NonStreamingArray",
            TypeKind::ArrayOfComplex,
            schema.as_object().unwrap().clone(),
        );

        let result = generator.generate_array_type_alias(&type_discovery, None);
        assert!(result.is_ok());

        let generated = result.unwrap();
        let implementation_file = parse_generated_file(generated.implementation_tokens);

        // Verify decode_prefix is NOT generated
        assert!(
            !inherent_impl_has_method(&implementation_file, "NonStreamingArray", "decode_prefix"),
            "Should NOT generate decode_prefix when streaming is disabled"
        );
    }

    #[test]
    fn test_streaming_array_with_complex_elements() {
        // Streaming array with complex (struct) elements should call element's decode_prefix
        let generator = TypeAliasGenerator::with_streaming_encode(true);

        let schema = json!({
            "type": "array",
            "items": {"$ref": "#/definitions/ComplexItem"}
        });
        let type_discovery = create_test_type_discovery(
            "StreamingComplexArray",
            TypeKind::ArrayOfComplex,
            schema.as_object().unwrap().clone(),
        );

        let result = generator.generate_array_type_alias(&type_discovery, None);
        assert!(result.is_ok());

        let generated = result.unwrap();
        let implementation_file = parse_generated_file(generated.implementation_tokens);

        // Verify decode_prefix is generated
        assert!(
            inherent_impl_has_method(
                &implementation_file,
                "StreamingComplexArray",
                "decode_prefix"
            ),
            "Should generate decode_prefix for complex element arrays"
        );
    }
}
