//! Pattern Validator Generator - Specialized code generation for pattern-validated HashMap types
//!
//! This module handles code generation for pattern validator types, which are HashMap-like
//! types that validate keys against regex patterns. These are used for JSON Schema
//! patternProperties support.
//!
//! Generated types include:
//! - Struct with FxHashMap<String, T> storage and regex::Regex validation
//! - Runtime validation on insertion
//! - JsonTokenEncoder implementation for serialization
//! - Serde implementations with key validation
//!
//! Extracted from code_generator.rs as part of module size refactoring (2025-10-04).

use crate::TctResult;
use crate::generator::generation_output::TypeCodeGeneration;
use crate::generator::schema_discovery::PatternValidatorInfo;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use rustc_hash::FxHashMap;

/// Specialized generator for pattern validator types
///
/// Pattern validators are HashMap-like types that validate keys against a regex pattern.
/// They provide runtime validation while maintaining type safety for values.
///
/// Uses dependency injection for shared utilities (type parsing) to avoid code duplication.
#[derive(Default)]
pub struct PatternValidatorGenerator {
    streaming_encode: bool,
}

impl PatternValidatorGenerator {
    /// Create a new pattern validator generator
    pub fn new() -> Self {
        Self {
            streaming_encode: false,
        }
    }

    /// Enable streaming-only prefix decode helpers.
    pub fn with_streaming_encode(mut self, streaming_encode: bool) -> Self {
        self.streaming_encode = streaming_encode;
        self
    }

    /// Generate all pattern validator types from discovery
    ///
    /// # Parameters
    ///
    /// - `pattern_validators`: Map of validator names to their info
    /// - `parse_type_fn`: Function to parse Rust type strings into TokenStreams
    ///
    /// # Returns
    ///
    /// Vector of type code generations for all validators
    pub fn generate_all(
        &self,
        pattern_validators: &FxHashMap<String, PatternValidatorInfo>,
        parse_type_fn: &dyn Fn(&str) -> TokenStream,
    ) -> TctResult<Vec<TypeCodeGeneration>> {
        let mut results = Vec::new();

        for (validator_name, validator_info) in pattern_validators {
            crate::log_debug!("🎯 Generating pattern validator: '{}'", validator_name);
            let type_gen =
                self.generate_validator(validator_name, validator_info, parse_type_fn)?;
            results.push(type_gen);
        }

        Ok(results)
    }

    /// Generate a single pattern validator struct
    ///
    /// Creates a complete type with:
    /// - HashMap storage with regex validation
    /// - Insert/get/iter methods with validation
    /// - JsonTokenEncoder implementation
    /// - Serde implementations with key validation
    /// - PartialEq implementation (regex compared by pattern string)
    fn generate_validator(
        &self,
        validator_name: &str,
        validator_info: &PatternValidatorInfo,
        parse_type_fn: &dyn Fn(&str) -> TokenStream,
    ) -> TctResult<TypeCodeGeneration> {
        let validator_ident = format_ident!("{}", validator_name);
        let pattern_literal = &validator_info.pattern;
        let value_type = parse_type_fn(&validator_info.value_type);
        let streaming_prefix_impl = if self.streaming_encode {
            quote! {
                impl #validator_ident {
                    pub fn decode_prefix(
                        tokens: &mut impl Iterator<Item = u32>,
                    ) -> PrefixDecodeResult<Self> {
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

        // Generate the validator struct with typed data storage and validation
        let validator_struct = quote! {
            /// Regex validated during generation
            #[derive(Debug, Clone)]
            pub struct #validator_ident {
                inner: rustc_hash::FxHashMap<String, #value_type>,
                pattern: regex::Regex,
            }

            impl #validator_ident {
                pub fn new() -> Result<Self, regex::Error> {
                    Ok(Self {
                        inner: rustc_hash::FxHashMap::default(),
                        pattern: regex::Regex::new(#pattern_literal)?,
                    })
                }

                pub fn insert(&mut self, key: String, value: #value_type) -> Result<Option<#value_type>, String> {
                    if !self.pattern.is_match(&key) {
                        return Err(format!("Key '{}' does not match pattern '{}'", key, #pattern_literal));
                    }
                    Ok(self.inner.insert(key, value))
                }

                pub fn get(&self, key: &str) -> Option<&#value_type> {
                    self.inner.get(key)
                }

                pub fn iter(&self) -> impl Iterator<Item = (&String, &#value_type)> {
                    self.inner.iter()
                }

                pub fn len(&self) -> usize {
                    self.inner.len()
                }

                pub fn is_empty(&self) -> bool {
                    self.inner.is_empty()
                }

                pub fn validate_key(&self, key: &str) -> Result<(), String> {
                    if !self.pattern.is_match(&key) {
                        return Err(format!("Key '{}' does not match pattern '{}'", key, #pattern_literal));
                    }
                    Ok(())
                }

                pub fn pattern_str(&self) -> &'static str {
                    #pattern_literal
                }
            }

            impl PartialEq for #validator_ident {
                fn eq(&self, other: &Self) -> bool {
                    // Compare pattern string and inner data
                    // regex::Regex doesn't implement PartialEq, so we compare the pattern strings
                    self.pattern.as_str() == other.pattern.as_str() && self.inner == other.inner
                }
            }

            impl Default for #validator_ident {
                fn default() -> Self {
                    Self::new().expect("pattern validator regex was validated during schema discovery")
                }
            }

            impl crate::tokenizer::JsonTokenEncoder for #validator_ident {
                fn encode(&self, tokens: &mut Vec<u32>) {
                    // Use typed HashMap encoder for optimal tokenization
                    self.inner.encode(tokens);
                }

                fn decode(tokens: &mut impl Iterator<Item = u32>) -> Result<Self, crate::tokenizer::DecodeError> {
                    // Decode raw HashMap first
                    let inner = rustc_hash::FxHashMap::<String, #value_type>::decode(tokens)?;

                    // Create validator and validate all keys during construction
                    let mut validator = Self::new()
                        .map_err(|e| crate::tokenizer::DecodeError::InvalidData(e.to_string()))?;

                    // Insert each key-value pair with validation
                    for (key, value) in inner {
                        validator.insert(key, value)
                            .map_err(|e| crate::tokenizer::DecodeError::InvalidData(e))?;
                    }

                    Ok(validator)
                }

                fn decode_strict(tokens: &mut impl Iterator<Item = u32>) -> Result<Self, crate::tokenizer::DecodeError> {
                    // Decode raw HashMap first with strict token-value semantics
                    let inner = rustc_hash::FxHashMap::<String, #value_type>::decode_strict(tokens)?;

                    // Create validator and validate all keys during construction
                    let mut validator = Self::new()
                        .map_err(|e| crate::tokenizer::DecodeError::InvalidData(e.to_string()))?;

                    // Insert each key-value pair with validation
                    for (key, value) in inner {
                        validator.insert(key, value)
                            .map_err(|e| crate::tokenizer::DecodeError::InvalidData(e))?;
                    }

                    Ok(validator)
                }

                fn token_count(&self) -> usize {
                    // Delegate to typed HashMap for accurate count
                    self.inner.token_count()
                }
            }

            impl serde::Serialize for #validator_ident {
                fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
                where
                    S: serde::Serializer,
                {
                    // Serialize as plain HashMap for transparent JSON representation
                    self.inner.serialize(serializer)
                }
            }

            impl<'de> serde::Deserialize<'de> for #validator_ident {
                fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
                where
                    D: serde::Deserializer<'de>,
                {
                    // Deserialize as HashMap first
                    let inner: rustc_hash::FxHashMap<String, #value_type> =
                        rustc_hash::FxHashMap::deserialize(deserializer)?;

                    // Create pattern regex
                    let pattern = regex::Regex::new(#pattern_literal)
                        .map_err(|e| serde::de::Error::custom(format!("Invalid pattern '{}': {}", #pattern_literal, e)))?;

                    // Validate all keys during deserialization
                    for key in inner.keys() {
                        if !pattern.is_match(key) {
                            return Err(serde::de::Error::custom(
                                format!("Key '{}' does not match pattern '{}'", key, #pattern_literal)
                            ));
                        }
                    }

                    Ok(Self { inner, pattern })
                }
            }

            #streaming_prefix_impl
        };

        // Create type code generation
        let type_generation = TypeCodeGeneration::new_struct(
            validator_name.to_string(),
            validator_struct,
            quote! {}, // No additional implementations needed
        );

        crate::log_debug!("   ✅ Generated validator struct '{}'", validator_name);
        Ok(type_generation)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn parse_generated_file(code: &str) -> syn::File {
        syn::parse_file(code).unwrap_or_else(|error| panic!("generated Rust should parse: {error}"))
    }

    fn path_ends_with_ident(path: &syn::Path, name: &str) -> bool {
        path.segments
            .last()
            .is_some_and(|segment| segment.ident == name)
    }

    fn type_ends_with_ident(ty: &syn::Type, name: &str) -> bool {
        matches!(ty, syn::Type::Path(type_path) if path_ends_with_ident(&type_path.path, name))
    }

    fn generated_file_has_trait_impl_for(
        file: &syn::File,
        type_name: &str,
        trait_name: &str,
    ) -> bool {
        file.items.iter().any(|item| match item {
            syn::Item::Impl(item_impl) => {
                type_ends_with_ident(&item_impl.self_ty, type_name)
                    && item_impl
                        .trait_
                        .as_ref()
                        .is_some_and(|(_, path, _)| path_ends_with_ident(path, trait_name))
            }
            _ => false,
        })
    }

    fn generated_file_has_public_impl_method(
        file: &syn::File,
        type_name: &str,
        method_name: &str,
    ) -> bool {
        file.items.iter().any(|item| match item {
            syn::Item::Impl(item_impl) if type_ends_with_ident(&item_impl.self_ty, type_name) => {
                item_impl.items.iter().any(|impl_item| {
                    matches!(
                        impl_item,
                        syn::ImplItem::Fn(function)
                            if function.sig.ident == method_name
                                && matches!(function.vis, syn::Visibility::Public(_))
                    )
                })
            }
            _ => false,
        })
    }

    fn pattern_info() -> PatternValidatorInfo {
        PatternValidatorInfo {
            validator_name: "PackageExportsEntryObjectPatternValidator0".to_string(),
            pattern: "^\\\\.".to_string(),
            value_type: "String".to_string(),
            value_schema: json!({"type": "string"}).as_object().unwrap().clone(),
        }
    }

    #[test]
    fn streaming_pattern_validator_generates_default_and_decode_prefix() {
        let generator = PatternValidatorGenerator::new().with_streaming_encode(true);
        let type_generation = generator
            .generate_validator(
                "PackageExportsEntryObjectPatternValidator0",
                &pattern_info(),
                &|rust_type| {
                    let ident = format_ident!("{}", rust_type);
                    quote! { #ident }
                },
            )
            .expect("pattern validator generation should succeed");
        let code = type_generation.definition_tokens.to_string();
        let generated_file = parse_generated_file(&code);

        assert!(
            generated_file_has_trait_impl_for(
                &generated_file,
                "PackageExportsEntryObjectPatternValidator0",
                "Default"
            ),
            "streaming parent structs derive Default, so pattern validators must provide it: {code}"
        );
        assert!(
            generated_file_has_public_impl_method(
                &generated_file,
                "PackageExportsEntryObjectPatternValidator0",
                "decode_prefix"
            ),
            "streaming parent structs call decode_prefix on pattern validator fields: {code}"
        );
    }
}
