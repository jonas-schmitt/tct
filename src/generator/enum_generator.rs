//! Enum Generator - Specialized code generation for enum types
//!
//! This module handles all code generation for enum types, including:
//! - Simple enums (unit variants with serde rename)
//! - Complex enums (oneOf/anyOf with data variants)
//! - Schema-aware deserialization for anyOf/oneOf compositions
//! - JsonTokenEncoder implementations for enums
//!
//! Extracted from code_generator.rs as part of module size refactoring (2025-10-03).

use crate::TctResult;
use crate::generator::encoding_decision::EncodingDecision;
use crate::generator::generation_output::TypeCodeGeneration;
use crate::generator::schema_discovery::{MemberInfo, MemberType, SchemaDiscovery, TypeDiscovery};
use crate::generator::type_analysis::TraitDerivability;
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};

mod partial_encoding;
mod schema_validation;
mod streaming_decode;
mod support;

use support::{EnumEncodingMode, default_value_for_member_type, determine_encoding_mode};

/// Specialized generator for enum types
///
/// This generator handles all aspects of enum generation, including:
/// - Simple enum variants with serde rename
/// - Complex enum variants with data payloads
/// - Schema-aware deserialization for anyOf/oneOf
/// - JsonTokenEncoder implementation with discriminator-based encoding
///
/// Uses dependency injection for shared utilities (type generation, derive traits, etc.)
/// to avoid premature extraction of code used by multiple generators.
pub struct EnumGenerator<'a> {
    /// Schema discovery results (for accessing root schema)
    discovery: &'a SchemaDiscovery,
    /// Whether streaming-friendly encoding is enabled (for prefix decoding)
    streaming_encode: bool,
}

impl<'a> EnumGenerator<'a> {
    /// Create a new enum generator
    ///
    /// # Parameters
    /// - `discovery`: Schema discovery results
    /// - `streaming_encode`: Whether streaming-friendly encoding is enabled
    pub fn new(discovery: &'a SchemaDiscovery, streaming_encode: bool) -> Self {
        Self {
            discovery,
            streaming_encode,
        }
    }

    /// Generate a complete enum type with all implementations
    ///
    /// # Parameters
    ///
    /// - `type_discovery`: Schema discovery results for this type
    /// - `trait_derivability`: Trait analysis results (Eq, Ord, Hash)
    /// - `encoding_decision`: Encoding strategy decision (None = standard encoding)
    /// - `member_type_fn`: Function to generate Rust types from schema member types
    /// - `derive_traits_fn`: Function to generate derive trait attributes
    /// - `derive_traits_without_deserialize_fn`: Function to generate derive traits excluding Deserialize
    /// - `derive_traits_without_default_fn`: Function to generate derive traits excluding Default
    /// - `derive_traits_without_deserialize_and_default_fn`: Function excluding both
    ///
    /// # Returns
    ///
    /// Complete type code generation including definition and implementations
    #[allow(clippy::too_many_arguments)]
    pub fn generate(
        &self,
        type_discovery: &TypeDiscovery,
        trait_derivability: Option<&TraitDerivability>,
        encoding_decision: Option<&EncodingDecision>,
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
    ) -> TctResult<TypeCodeGeneration> {
        let type_name = &type_discovery.type_name;
        let type_ident = format_ident!("{}", type_name);

        // Generate enum variants
        crate::log_debug!(
            "🔍 DEBUG ENUM GEN: Generating enum for '{}' with {} members",
            type_name,
            type_discovery.members.len()
        );
        let mut variants = Vec::new();

        // Check if this is a simple enum (all variants are unit types)
        let is_simple_enum = type_discovery.members.iter().all(|member| {
            matches!(member.member_type, crate::generator::schema_discovery::MemberType::Primitive(ref t) if t == "()")
        });

        crate::log_debug!("🔍 DEBUG ENUM GEN: Is simple enum: {}", is_simple_enum);

        // Check if first variant is a unit variant (for #[default] attribute applicability)
        // The #[default] attribute can only be used on unit variants (no data)
        let first_variant_is_unit = if let Some(first_member) = type_discovery.members.first() {
            // Unit variants: simple enum units OR complex enum with null type first
            is_simple_enum
                || matches!(
                    &first_member.member_type,
                    crate::generator::schema_discovery::MemberType::Primitive(t) if t == "Null" || t == "()"
                )
        } else {
            false
        };

        // Track info needed for manual Default impl (for complex enums with data variants)
        let mut first_variant_default: Option<proc_macro2::TokenStream> = None;
        let needs_manual_default = self.streaming_encode && !first_variant_is_unit;

        for (index, member) in type_discovery.members.iter().enumerate() {
            crate::log_debug!(
                "   • Member {}: name='{}', type={:?}",
                index,
                member.name,
                member.member_type
            );

            // Add #[default] attribute ONLY for unit variants when streaming_encode is enabled
            // For variants with data, we generate manual impl Default instead
            let default_attr = if index == 0 && self.streaming_encode && first_variant_is_unit {
                quote! { #[default] }
            } else {
                quote! {}
            };

            if is_simple_enum {
                // For simple enums, use the actual variant name and generate unit variants
                let variant_name = format_ident!("{}", member.name);

                // Get the original JSON value for serde rename from the schema
                let original_value = member
                    .schema
                    .get("__original_enum_value")
                    .and_then(|v| v.as_str())
                    .unwrap_or(&member.name); // Fallback to variant name if not found

                variants.push(quote! {
                    #[serde(rename = #original_value)]
                    #default_attr
                    #variant_name
                });
                crate::log_debug!(
                    "     → Simple variant: {} (rename: {})",
                    variant_name,
                    original_value
                );
            } else {
                // For complex enums (oneOf/anyOf), use the existing logic
                let variant_name = format_ident!("Variant{}", index);

                // Check if this is a null type - generate unit variant
                let is_null = matches!(
                    &member.member_type,
                    crate::generator::schema_discovery::MemberType::Primitive(t) if t == "Null"
                );

                if is_null {
                    // Null type becomes a unit variant
                    variants.push(quote! {
                        #default_attr
                        #variant_name
                    });
                    crate::log_debug!(
                        "     → Null variant: {} (unit variant for JSON null)",
                        variant_name
                    );
                } else {
                    let variant_type = member_type_fn(&member.member_type)?;

                    // Capture first variant default for manual Default impl
                    if index == 0 && needs_manual_default {
                        first_variant_default = Some(default_value_for_member_type(
                            &member.member_type,
                            member_type_fn,
                        )?);
                    }

                    variants.push(quote! {
                        #default_attr
                        #variant_name(#variant_type)
                    });
                    crate::log_debug!(
                        "     → Complex variant: {} with type {}",
                        variant_name,
                        variant_type
                    );
                }
            }
        }

        crate::log_debug!(
            "🔍 DEBUG ENUM GEN: Generated {} variants for enum '{}'",
            variants.len(),
            type_name
        );

        // Generate derive traits (enums don't have pattern validators)
        // For complex enums with data first variants, use derive without Default
        // (we'll generate a manual impl Default instead)
        let derive_traits = if needs_manual_default {
            derive_traits_without_default_fn(trait_derivability, false)
        } else {
            derive_traits_fn(trait_derivability, false)
        };

        // Check if this is an anyOf/oneOf enum that needs schema-aware deserialization
        let anyof_oneof_type = if !is_simple_enum {
            self.check_anyof_oneof_enum(type_discovery)
        } else {
            None
        };
        let is_anyof_oneof = anyof_oneof_type.is_some();

        // Generate enum definition
        let definition_tokens = if is_simple_enum {
            // Simple enums don't need untagged serialization
            quote! {
                #derive_traits
                pub enum #type_ident {
                    #(#variants),*
                }
            }
        } else if is_anyof_oneof {
            // anyOf/oneOf enums: Use schema-aware deserialization with untagged serialization
            // Don't derive Deserialize - we'll generate a custom impl
            // Keep #[serde(untagged)] for Serialize to match our custom Deserialize expectations
            // Also exclude Default if we need manual impl
            let derive_traits_without_deserialize = if needs_manual_default {
                // Need to exclude both Deserialize AND Default
                derive_traits_without_deserialize_and_default_fn(trait_derivability, false)
            } else {
                derive_traits_without_deserialize_fn(trait_derivability, false)
            };

            quote! {
                #derive_traits_without_deserialize
                #[serde(untagged)]
                pub enum #type_ident {
                    #(#variants),*
                }
            }
        } else {
            // Complex enums (fallback) - keep untagged serialization
            quote! {
                #derive_traits
                #[serde(untagged)]
                pub enum #type_ident {
                    #(#variants),*
                }
            }
        };

        // Generate JsonTokenEncoder implementation
        let mut implementation_tokens = self.generate_enum_token_encoder_impl(
            type_name,
            &type_ident,
            &type_discovery.members,
            encoding_decision,
            member_type_fn,
        )?;

        // For anyOf/oneOf enums, add schema-aware deserialization
        if is_anyof_oneof {
            crate::log_debug!(
                "🔍 Generating schema-aware deserialization for anyOf/oneOf enum '{}'",
                type_name
            );

            // Generate schema constants
            let schema_constants =
                self.generate_variant_schema_constants(type_name, &type_discovery.members);

            // Generate validation functions
            let validation_functions =
                self.generate_variant_validation_functions(type_name, &type_discovery.members);

            // Generate custom Deserialize impl
            let is_anyof = anyof_oneof_type.ok_or_else(|| {
                crate::error::TctError::code_generation(
                    format!(
                        "anyof_oneof_type should be Some for anyOf/oneOf enum '{}'",
                        type_name
                    ),
                    format!("#/types/{}", type_name),
                    "This is an internal consistency error - check anyOf/oneOf detection logic"
                        .to_string(),
                    crate::error::GenerationPhase::TypeGeneration,
                )
            })?;
            let custom_deserialize = self.generate_schema_aware_deserialize_impl(
                &type_ident,
                &type_discovery.members,
                is_anyof,
                member_type_fn,
            )?;

            // Wrap everything in an impl block
            let schema_aware_impl = quote! {
                impl #type_ident {
                    #(#schema_constants)*

                    #(#validation_functions)*
                }

                #custom_deserialize
            };

            // Combine with token encoder impl
            implementation_tokens = quote! {
                #implementation_tokens
                #schema_aware_impl
            };
        }

        // Generate partial encoding methods if discriminator info is available
        if let Some(ref discriminator_info) = type_discovery.discriminator_info {
            crate::log_debug!(
                "🔄 Generating partial encoding methods for enum '{}' with discriminator '{}'",
                type_name,
                discriminator_info.field_names.join("+")
            );
            let partial_encoding_methods = self.generate_enum_partial_encoding_methods(
                &type_ident,
                &type_discovery.members,
                discriminator_info,
                encoding_decision,
                member_type_fn,
            )?;
            implementation_tokens = quote! {
                #implementation_tokens
                #partial_encoding_methods
            };
        }

        // Generate streaming decode_prefix method (only when streaming mode enabled)
        if self.streaming_encode {
            let streaming_methods = self.generate_enum_streaming_decode_prefix(
                type_name,
                &type_ident,
                &type_discovery.members,
                encoding_decision,
                member_type_fn,
            )?;
            implementation_tokens = quote! {
                #implementation_tokens
                #streaming_methods
            };
        }

        // Generate manual Default impl for complex enums with data variants
        // (can't use #[default] attribute on non-unit variants)
        if needs_manual_default && let Some(ref first_variant_default) = first_variant_default {
            let manual_default_impl = quote! {
                impl Default for #type_ident {
                    fn default() -> Self {
                        Self::Variant0(#first_variant_default)
                    }
                }
            };
            implementation_tokens = quote! {
                #implementation_tokens
                #manual_default_impl
            };
            crate::log_debug!(
                "🔍 Generated manual Default impl for complex enum '{}' (first variant has data type)",
                type_name
            );
        }

        // Create type code generation
        let mut type_generation = TypeCodeGeneration::new_enum(
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

        Ok(type_generation)
    }

    /// Generate JsonTokenEncoder implementation for enum
    fn generate_enum_token_encoder_impl(
        &self,
        type_name: &str,
        type_ident: &Ident,
        members: &[MemberInfo],
        encoding_decision: Option<&EncodingDecision>,
        member_type_fn: &dyn Fn(&MemberType) -> TctResult<TokenStream>,
    ) -> TctResult<TokenStream> {
        // Check if this is a simple enum (all variants are unit types)
        let is_simple_enum = members.iter().all(|member| {
            matches!(member.member_type, crate::generator::schema_discovery::MemberType::Primitive(ref t) if t == "()")
        });

        // Determine encoding strategy from decision
        let encoding_mode = determine_encoding_mode(type_name, encoding_decision);

        let (encode_arms, token_count_arms) = if is_simple_enum {
            // Simple enum: encode the variant index
            let encode_arms = members
                .iter()
                .enumerate()
                .map(|(i, member)| {
                    let variant_name = format_ident!("{}", member.name);
                    let variant_index = i as u32;

                    match &encoding_mode {
                        EnumEncodingMode::Codec(_codec_spec) => {
                            // Use codec for multi-token LOSSLESS encoding
                            let codec_mod_name = format_ident!("{}_codec", type_name.to_lowercase());
                            quote! {
                                Self::#variant_name => #codec_mod_name::encode(#variant_index, tokens)
                            }
                        }
                        EnumEncodingMode::Standard => {
                            // Standard index encoding (1 token per variant)
                            quote! {
                                Self::#variant_name => tokens.push(#variant_index)
                            }
                        }
                        EnumEncodingMode::Constant => {
                            // Constant: no encoding needed (0 tokens)
                            quote! {
                                Self::#variant_name => { /* constant - no tokens */ }
                            }
                        }
                    }
                })
                .collect::<Vec<_>>();

            // Token count: depends on encoding mode
            let token_count = match &encoding_mode {
                EnumEncodingMode::Codec(codec_spec) => codec_spec.token_width,
                EnumEncodingMode::Standard => 1,
                EnumEncodingMode::Constant => 0, // Constant: 0 tokens
            };

            let token_count_arms = members
                .iter()
                .map(|member| {
                    let variant_name = format_ident!("{}", member.name);
                    let count_lit = proc_macro2::Literal::usize_unsuffixed(token_count);
                    quote! {
                        Self::#variant_name => #count_lit
                    }
                })
                .collect::<Vec<_>>();

            (encode_arms, token_count_arms)
        } else {
            // Complex enum: use LENGTH-PREFIXED UNTAGGED encoding
            // For untagged enums, serde tries each variant in order until one succeeds
            // We must NOT use discriminants as they break serde's untagged deserialization

            let encode_arms = members
                .iter()
                .enumerate()
                .map(|(i, member)| {
                    let variant_name = format_ident!("Variant{}", i);
                    let variant_index = i as u32;

                    // Check if this is a null type - generate unit variant encode
                    let is_null = matches!(
                        &member.member_type,
                        crate::generator::schema_discovery::MemberType::Primitive(t) if t == "Null"
                    );

                    // Generate discriminator encoding based on encoding mode
                    let encode_discriminator = match &encoding_mode {
                        EnumEncodingMode::Codec(_codec_spec) => {
                            // Use codec for discriminator to respect vocabulary constraints
                            let codec_mod_name =
                                format_ident!("{}_codec", type_name.to_lowercase());
                            quote! {
                                #codec_mod_name::encode(#variant_index, tokens);
                            }
                        }
                        EnumEncodingMode::Standard => {
                            // Standard: push discriminator directly
                            quote! {
                                tokens.push(#variant_index);
                            }
                        }
                        EnumEncodingMode::Constant => {
                            // Constant: no discriminator needed
                            quote! {
                                /* constant - no discriminator */
                            }
                        }
                    };

                    if is_null {
                        // Null type: just encode discriminator (unit variant)
                        quote! {
                            Self::#variant_name => {
                                #encode_discriminator
                            }
                        }
                    } else {
                        // DISCRIMINATOR-PREFIXED ENCODING: Write variant index first, then the value
                        quote! {
                            Self::#variant_name(value) => {
                                // Write discriminator (variant index) first
                                #encode_discriminator
                                // Then write the actual value
                                value.encode(tokens);
                            }
                        }
                    }
                })
                .collect::<Vec<_>>();

            let token_count_arms = members
                .iter()
                .enumerate()
                .map(|(i, member)| {
                    let variant_name = format_ident!("Variant{}", i);

                    // Check if this is a null type
                    let is_null = matches!(
                        &member.member_type,
                        crate::generator::schema_discovery::MemberType::Primitive(t) if t == "Null"
                    );

                    // Calculate discriminator token count based on encoding mode
                    let discriminator_token_count = match &encoding_mode {
                        EnumEncodingMode::Codec(codec_spec) => codec_spec.token_width,
                        EnumEncodingMode::Standard => 1,
                        EnumEncodingMode::Constant => 0,
                    };

                    if is_null {
                        // Null type: just discriminator tokens
                        quote! {
                            Self::#variant_name => #discriminator_token_count
                        }
                    } else {
                        // DISCRIMINATOR-PREFIXED TOKEN COUNT: value's token count + discriminator tokens
                        quote! {
                            Self::#variant_name(value) => #discriminator_token_count + value.token_count()
                        }
                    }
                })
                .collect::<Vec<_>>();

            (encode_arms, token_count_arms)
        };

        // Generate decode implementation based on semantic analysis decode strategy
        let decode_impl = self.generate_enum_decode_impl(
            type_name,
            type_ident,
            members,
            member_type_fn,
            encoding_decision,
        )?;

        // Generate codec module if using codec-based encoding
        let codec_module = if let Some(EncodingDecision::EnumCodec { codec_spec, .. }) =
            encoding_decision
        {
            let codec_mod =
                crate::generator::structural_codec::generate_codec_module(type_name, codec_spec);
            quote! { #codec_mod }
        } else {
            quote! {}
        };

        // A constant (zero-token) enum encodes nothing, so its arms never touch the
        // output; naming the parameter `_tokens` keeps the generated crate free of
        // unused-variable warnings, which users see because they build without the
        // lint caps that this repository's own generated builds apply.
        let encode_uses_tokens = encode_arms
            .iter()
            .any(|arm| arm.to_string().contains("tokens"));
        let tokens_param = format_ident!(
            "{}",
            if encode_uses_tokens {
                "tokens"
            } else {
                "_tokens"
            }
        );

        // Generate the implementation with untagged decode strategy
        Ok(quote! {
            #codec_module

            impl crate::tokenizer::JsonTokenEncoder for #type_ident {
                fn encode(&self, #tokens_param: &mut Vec<u32>) {
                    match self {
                        #(#encode_arms),*
                    }
                }

                #decode_impl

                fn token_count(&self) -> usize {
                    match self {
                        #(#token_count_arms),*
                    }
                }
            }
        })
    }

    /// Generate decode implementation for untagged enum based on semantic analysis
    fn generate_enum_decode_impl(
        &self,
        type_name: &str,
        _type_ident: &Ident,
        members: &[MemberInfo],
        member_type_fn: &dyn Fn(&MemberType) -> TctResult<TokenStream>,
        encoding_decision: Option<&EncodingDecision>,
    ) -> TctResult<TokenStream> {
        // Check if this is a simple enum (all variants are unit types)
        let is_simple_enum = members.iter().all(|member| {
            matches!(member.member_type, crate::generator::schema_discovery::MemberType::Primitive(ref t) if t == "()")
        });

        if is_simple_enum {
            // Simple enum: decode the variant index (single or multi-token) and match it
            let decode_arms = members
                .iter()
                .enumerate()
                .map(|(i, member)| {
                    let variant_name = format_ident!("{}", member.name);
                    let variant_index = i as u32;
                    quote! {
                        #variant_index => Ok(Self::#variant_name)
                    }
                })
                .collect::<Vec<_>>();

            // Determine how to decode the variant index based on encoding decision
            let encoding_mode = determine_encoding_mode(type_name, encoding_decision);
            let is_constant = matches!(&encoding_mode, EnumEncodingMode::Constant);
            let decode_impl = if is_constant {
                // Constant: return the single variant directly (no tokens consumed)
                let first_variant = &members[0];
                let variant_name = format_ident!("{}", first_variant.name);
                quote! {
                    fn decode(_tokens: &mut impl Iterator<Item = u32>) -> Result<Self, crate::tokenizer::DecodeError> {
                        // Constant enum: always return the single variant
                        Ok(Self::#variant_name)
                    }

                    fn decode_strict(_tokens: &mut impl Iterator<Item = u32>) -> Result<Self, crate::tokenizer::DecodeError> {
                        Ok(Self::#variant_name)
                    }
                }
            } else {
                let decode_variant_index =
                    if let Some(EncodingDecision::EnumCodec { codec_spec: _, .. }) =
                        encoding_decision
                    {
                        // Use codec for multi-token decode
                        let codec_mod_name = format_ident!("{}_codec", type_name.to_lowercase());
                        quote! {
                            let variant_index = #codec_mod_name::decode(tokens)?;
                        }
                    } else {
                        // Standard single-token decode
                        quote! {
                            let variant_index = tokens.next()
                                .ok_or(crate::tokenizer::DecodeError::UnexpectedEndOfStream {
                                    context: crate::tokenizer::EOFContext::new(
                                        #type_name,
                                        Some("variant_index"),
                                        "enum variant index"
                                    )
                                })?;
                        }
                    };
                let strict_decode_variant_index =
                    if let Some(EncodingDecision::EnumCodec { codec_spec: _, .. }) =
                        encoding_decision
                    {
                        // Use strict codec decode to reject out-of-range discriminator digits.
                        let codec_mod_name = format_ident!("{}_codec", type_name.to_lowercase());
                        quote! {
                            let variant_index = #codec_mod_name::decode_strict(tokens)?;
                        }
                    } else {
                        quote! {
                            let variant_index = tokens.next()
                                .ok_or(crate::tokenizer::DecodeError::UnexpectedEndOfStream {
                                    context: crate::tokenizer::EOFContext::new(
                                        #type_name,
                                        Some("variant_index"),
                                        "enum variant index"
                                    )
                                })?;
                        }
                    };

                let variant_count = members.len() as u32;

                quote! {
                    fn decode(tokens: &mut impl Iterator<Item = u32>) -> Result<Self, crate::tokenizer::DecodeError> {
                        #decode_variant_index

                        // Tag the just-consumed selector token as the discriminator
                        // (role export; no-op outside token_roles replay).
                        crate::tokenizer::role_export::mark_discriminator();

                        // Apply modulo for graceful handling of out-of-range variant indices
                        let variant_index = variant_index % #variant_count;

                        match variant_index {
                            #(#decode_arms),*,
                            _ => unreachable!("Modulo operation ensures valid token range")
                        }
                    }

                    fn decode_strict(tokens: &mut impl Iterator<Item = u32>) -> Result<Self, crate::tokenizer::DecodeError> {
                        #strict_decode_variant_index

                        match variant_index {
                            #(#decode_arms),*,
                            _ => Err(crate::tokenizer::DecodeError::InvalidTokenInContext {
                                token: variant_index,
                                context: crate::tokenizer::EOFContext::new(
                                    #type_name,
                                    Some("variant_index"),
                                    "enum variant index in range",
                                ),
                            }),
                        }
                    }
                }
            };

            return Ok(decode_impl);
        }

        // For data variant enums, use deterministic discriminator-based decode
        // This is truly schema-aware: uses compile-time analysis, NO runtime token analysis
        crate::log_debug!(
            "🎯 Using deterministic discriminator-based decode for data variant enum {}",
            type_name
        );
        self.generate_deterministic_enum_decode(
            type_name,
            members,
            member_type_fn,
            encoding_decision,
        )
    }

    /// Generate deterministic variant-index-based decode for data variant enums
    /// This is truly schema-aware: uses compile-time analysis to generate optimal decode code
    /// with NO runtime token structure analysis - everything figured out at compile time
    fn generate_deterministic_enum_decode(
        &self,
        type_name: &str,
        members: &[MemberInfo],
        member_type_fn: &dyn Fn(&MemberType) -> TctResult<TokenStream>,
        encoding_decision: Option<&EncodingDecision>,
    ) -> TctResult<TokenStream> {
        // Generate decode arms for each variant
        let decode_arms = members
            .iter()
            .enumerate()
            .map(|(i, member)| {
                let variant_name = format_ident!("Variant{}", i);
                let variant_index = i as u32;

                // Check if this is a null type - generate unit variant decode
                let is_null = matches!(
                    &member.member_type,
                    crate::generator::schema_discovery::MemberType::Primitive(t) if t == "Null"
                );

                if is_null {
                    // Null type: no payload to decode, just return unit variant
                    Ok(quote! {
                        #variant_index => {
                            Ok(Self::#variant_name)
                        }
                    })
                } else {
                    let member_type = member_type_fn(&member.member_type)?;

                    Ok(quote! {
                        #variant_index => {
                            let payload = <#member_type>::decode(tokens)?;
                            Ok(Self::#variant_name(payload))
                        }
                    })
                }
            })
            .collect::<TctResult<Vec<_>>>()?;
        let strict_decode_arms = members
            .iter()
            .enumerate()
            .map(|(i, member)| {
                let variant_name = format_ident!("Variant{}", i);
                let variant_index = i as u32;

                let is_null = matches!(
                    &member.member_type,
                    crate::generator::schema_discovery::MemberType::Primitive(t) if t == "Null"
                );

                if is_null {
                    Ok(quote! {
                        #variant_index => {
                            Ok(Self::#variant_name)
                        }
                    })
                } else {
                    let member_type = member_type_fn(&member.member_type)?;

                    Ok(quote! {
                        #variant_index => {
                            let payload = <#member_type>::decode_strict(tokens)?;
                            Ok(Self::#variant_name(payload))
                        }
                    })
                }
            })
            .collect::<TctResult<Vec<_>>>()?;

        if matches!(
            determine_encoding_mode(type_name, encoding_decision),
            EnumEncodingMode::Constant
        ) {
            let member = members
                .first()
                .ok_or_else(|| crate::error::TctError::CodeGeneration {
                    message: format!(
                        "Cannot generate zero-token decode for empty enum '{}'",
                        type_name
                    ),
                    schema_path: "#/oneOf".to_string(),
                    suggestion: "Ensure oneOf/anyOf schemas have at least one variant.".to_string(),
                    phase: crate::error::GenerationPhase::TypeGeneration,
                })?;

            let is_null = matches!(
                &member.member_type,
                crate::generator::schema_discovery::MemberType::Primitive(t) if t == "Null"
            );

            if is_null {
                return Ok(quote! {
                    fn decode(_tokens: &mut impl Iterator<Item = u32>) -> Result<Self, crate::tokenizer::DecodeError> {
                        Ok(Self::Variant0)
                    }

                    fn decode_strict(_tokens: &mut impl Iterator<Item = u32>) -> Result<Self, crate::tokenizer::DecodeError> {
                        Ok(Self::Variant0)
                    }
                });
            }

            let member_type = member_type_fn(&member.member_type)?;
            return Ok(quote! {
                fn decode(tokens: &mut impl Iterator<Item = u32>) -> Result<Self, crate::tokenizer::DecodeError> {
                    let payload = <#member_type>::decode(tokens)?;
                    Ok(Self::Variant0(payload))
                }

                fn decode_strict(tokens: &mut impl Iterator<Item = u32>) -> Result<Self, crate::tokenizer::DecodeError> {
                    let payload = <#member_type>::decode_strict(tokens)?;
                    Ok(Self::Variant0(payload))
                }
            });
        }

        // Generate discriminator decode based on encoding decision
        let decode_discriminator =
            if let Some(EncodingDecision::EnumCodec { .. }) = encoding_decision {
                // Use codec for discriminator to respect vocabulary constraints
                let codec_mod_name = format_ident!("{}_codec", type_name.to_lowercase());
                quote! {
                    let variant_index = #codec_mod_name::decode(tokens)?;
                }
            } else {
                // Standard: read single token
                quote! {
                    let variant_index = tokens.next()
                        .ok_or(crate::tokenizer::DecodeError::UnexpectedEndOfStream {
                            context: crate::tokenizer::EOFContext::new(
                                #type_name,
                                Some("discriminator"),
                                "enum discriminator"
                            )
                        })?;
                }
            };
        let strict_decode_discriminator =
            if let Some(EncodingDecision::EnumCodec { .. }) = encoding_decision {
                let codec_mod_name = format_ident!("{}_codec", type_name.to_lowercase());
                quote! {
                    let variant_index = #codec_mod_name::decode_strict(tokens)?;
                }
            } else {
                quote! {
                    let variant_index = tokens.next()
                        .ok_or(crate::tokenizer::DecodeError::UnexpectedEndOfStream {
                            context: crate::tokenizer::EOFContext::new(
                                #type_name,
                                Some("discriminator"),
                                "enum discriminator"
                            )
                        })?;
                }
            };

        let variant_count = members.len() as u32;

        Ok(quote! {
            fn decode(tokens: &mut impl Iterator<Item = u32>) -> Result<Self, crate::tokenizer::DecodeError> {
                // Read variant index (information-theoretic minimum for oneOf/anyOf)
                // Schema analysis at compile-time determined this encoding strategy
                #decode_discriminator

                // Tag the just-consumed selector token as the discriminator
                // (role export; no-op outside token_roles replay).
                crate::tokenizer::role_export::mark_discriminator();

                // Apply modulo for graceful handling of out-of-range variant indices
                let variant_index = variant_index % #variant_count;

                // Deterministic routing - NO runtime token structure analysis required
                // All routing decisions computed at compile-time from schema
                match variant_index {
                    #(#decode_arms),*
                    _ => unreachable!("Modulo operation ensures valid token range")
                }
            }

            fn decode_strict(tokens: &mut impl Iterator<Item = u32>) -> Result<Self, crate::tokenizer::DecodeError> {
                #strict_decode_discriminator

                match variant_index {
                    #(#strict_decode_arms),*
                    _ => Err(crate::tokenizer::DecodeError::InvalidTokenInContext {
                        token: variant_index,
                        context: crate::tokenizer::EOFContext::new(
                            #type_name,
                            Some("discriminator"),
                            "enum discriminator in range",
                        ),
                    }),
                }
            }
        })
    }

    /// Check if this enum is an anyOf/oneOf composition that needs schema-aware deserialization
    /// Returns Some(is_anyof) if it's a composition, None otherwise
    fn check_anyof_oneof_enum(&self, type_discovery: &TypeDiscovery) -> Option<bool> {
        schema_validation::check_anyof_oneof_enum(type_discovery)
    }

    /// Generate schema storage constants for anyOf/oneOf variants
    fn generate_variant_schema_constants(
        &self,
        _type_name: &str,
        members: &[MemberInfo],
    ) -> Vec<TokenStream> {
        schema_validation::generate_variant_schema_constants(self.discovery, members)
    }

    /// Generate validation functions for each anyOf/oneOf variant
    fn generate_variant_validation_functions(
        &self,
        _type_name: &str,
        members: &[MemberInfo],
    ) -> Vec<TokenStream> {
        schema_validation::generate_variant_validation_functions(members)
    }

    /// Generate custom Deserialize impl for schema-aware anyOf/oneOf variant selection
    /// `is_anyof`: true for anyOf (use first match), false for oneOf (require unique match)
    fn generate_schema_aware_deserialize_impl(
        &self,
        type_ident: &Ident,
        members: &[MemberInfo],
        is_anyof: bool,
        member_type_fn: &dyn Fn(&MemberType) -> TctResult<TokenStream>,
    ) -> TctResult<TokenStream> {
        schema_validation::generate_schema_aware_deserialize_impl(
            type_ident,
            members,
            is_anyof,
            member_type_fn,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::support::container_inner_type_tokens;
    use super::*;
    use crate::generator::schema_discovery::{
        MemberInfo, MemberType, SchemaDiscovery, SchemaLocation, TypeDiscovery, TypeKind,
    };
    use quote::ToTokens;
    use serde_json::json;

    fn parse_generated_file(tokens: TokenStream) -> syn::File {
        match syn::parse2(tokens) {
            Ok(file) => file,
            Err(error) => panic!("generated enum tokens should parse as Rust items: {error}"),
        }
    }

    fn impl_self_type_matches(item_impl: &syn::ItemImpl, type_name: &str) -> bool {
        item_impl.self_ty.to_token_stream().to_string() == type_name
    }

    fn impl_method<'a>(
        item_impl: &'a syn::ItemImpl,
        method_name: &str,
    ) -> Option<&'a syn::ImplItemFn> {
        item_impl.items.iter().find_map(|item| match item {
            syn::ImplItem::Fn(method) if method.sig.ident == method_name => Some(method),
            _ => None,
        })
    }

    fn generated_inherent_method<'a>(
        file: &'a syn::File,
        type_name: &str,
        method_name: &str,
    ) -> &'a syn::ImplItemFn {
        file.items
            .iter()
            .find_map(|item| match item {
                syn::Item::Impl(item_impl)
                    if item_impl.trait_.is_none()
                        && impl_self_type_matches(item_impl, type_name) =>
                {
                    impl_method(item_impl, method_name)
                }
                _ => None,
            })
            .unwrap_or_else(|| panic!("expected generated method {type_name}::{method_name}"))
    }

    fn generated_trait_method<'a>(
        file: &'a syn::File,
        type_name: &str,
        trait_name: &str,
        method_name: &str,
    ) -> &'a syn::ImplItemFn {
        file.items
            .iter()
            .find_map(|item| match item {
                syn::Item::Impl(item_impl)
                    if impl_self_type_matches(item_impl, type_name)
                        && item_impl.trait_.as_ref().is_some_and(|(_, path, _)| {
                            path.segments
                                .last()
                                .is_some_and(|segment| segment.ident == trait_name)
                        }) =>
                {
                    impl_method(item_impl, method_name)
                }
                _ => None,
            })
            .unwrap_or_else(|| {
                panic!("expected generated {trait_name}::{method_name} impl for {type_name}")
            })
    }

    fn generated_method_body_contains(method: &syn::ImplItemFn, expected: &str) -> bool {
        method
            .block
            .to_token_stream()
            .to_string()
            .contains(expected)
    }

    fn generated_method_signature_contains(method: &syn::ImplItemFn, expected: &str) -> bool {
        method.sig.to_token_stream().to_string().contains(expected)
    }

    fn assert_method_body_contains(method: &syn::ImplItemFn, expected: &str, context: &str) {
        let body = method.block.to_token_stream().to_string();
        assert!(body.contains(expected), "{context}: {body}");
    }

    fn assert_method_body_not_contains(method: &syn::ImplItemFn, expected: &str, context: &str) {
        let body = method.block.to_token_stream().to_string();
        assert!(!body.contains(expected), "{context}: {body}");
    }

    // Helper: Create test SchemaDiscovery with root schema
    fn create_test_discovery_with_root() -> SchemaDiscovery {
        let mut discovery = SchemaDiscovery::new();

        // Add root info with definitions
        let root_schema = json!({
            "$schema": "http://json-schema.org/draft-07/schema#",
            "definitions": {
                "TestType": {
                    "type": "object",
                    "properties": {
                        "value": {"type": "string"}
                    }
                }
            }
        });

        discovery.root_info = Some(crate::generator::schema_discovery::RootSchemaInfo {
            type_name: "Root".to_string(),
            schema: root_schema.as_object().unwrap().clone(),
            is_composition: false,
        });

        discovery
    }

    // Helper: Create a simple enum TypeDiscovery (unit variants)
    fn create_simple_enum_type() -> TypeDiscovery {
        let schema_location = SchemaLocation {
            json_path: "#/definitions/Status".to_string(),
            description: "Status enum".to_string(),
            parent_type: None,
            property_name: None,
        };

        let mut schema_obj = serde_json::Map::new();
        schema_obj.insert("enum".to_string(), json!(["active", "inactive", "pending"]));

        let mut type_discovery = TypeDiscovery::new(
            "Status".to_string(),
            TypeKind::Enum,
            schema_location,
            schema_obj.clone(),
        );

        // Add unit variant members
        for (idx, name) in ["Active", "Inactive", "Pending"].iter().enumerate() {
            let mut member_schema = serde_json::Map::new();
            member_schema.insert(
                "__original_enum_value".to_string(),
                json!(["active", "inactive", "pending"][idx]),
            );

            let member = MemberInfo {
                name: name.to_string(),
                member_type: MemberType::Primitive("()".to_string()),
                required: true,
                is_nullable: false,
                schema: member_schema,
            };
            type_discovery.add_member(member);
        }

        type_discovery
    }

    // Helper: Create a complex enum TypeDiscovery (data variants)
    fn create_complex_enum_type() -> TypeDiscovery {
        let schema_location = SchemaLocation {
            json_path: "#/definitions/Response".to_string(),
            description: "Response oneOf enum".to_string(),
            parent_type: None,
            property_name: None,
        };

        let mut schema_obj = serde_json::Map::new();
        schema_obj.insert(
            "oneOf".to_string(),
            json!([
                {"type": "object", "properties": {"success": {"type": "boolean"}}},
                {"type": "object", "properties": {"error": {"type": "string"}}}
            ]),
        );

        let mut type_discovery = TypeDiscovery::new(
            "Response".to_string(),
            TypeKind::Enum,
            schema_location,
            schema_obj.clone(),
        );

        // Add data variant members
        let member1 = MemberInfo {
            name: "Variant0".to_string(),
            member_type: MemberType::TypeReference("SuccessPayload".to_string()),
            required: true,
            is_nullable: false,
            schema: json!({"type": "object", "properties": {"success": {"type": "boolean"}}})
                .as_object()
                .unwrap()
                .clone(),
        };

        let member2 = MemberInfo {
            name: "Variant1".to_string(),
            member_type: MemberType::TypeReference("ErrorPayload".to_string()),
            required: true,
            is_nullable: false,
            schema: json!({"type": "object", "properties": {"error": {"type": "string"}}})
                .as_object()
                .unwrap()
                .clone(),
        };

        type_discovery.add_member(member1);
        type_discovery.add_member(member2);

        type_discovery
    }

    // ========================================================================================
    // Test Group 1: anyOf/oneOf Detection (2 tests)
    // ========================================================================================

    #[test]
    fn test_check_anyof_oneof_enum_with_oneof() {
        let discovery = create_test_discovery_with_root();
        let generator = EnumGenerator::new(&discovery, false);

        let type_discovery = create_complex_enum_type();
        let result = generator.check_anyof_oneof_enum(&type_discovery);

        assert!(result.is_some(), "Should detect oneOf composition");
        assert!(
            !result.unwrap(),
            "Should return false for oneOf (not anyOf)"
        );
    }

    #[test]
    fn test_check_anyof_oneof_enum_with_anyof() {
        let discovery = create_test_discovery_with_root();
        let generator = EnumGenerator::new(&discovery, false);

        let schema_location = SchemaLocation {
            json_path: "#/definitions/AnyOfEnum".to_string(),
            description: "anyOf enum".to_string(),
            parent_type: None,
            property_name: None,
        };

        let mut schema_obj = serde_json::Map::new();
        schema_obj.insert(
            "anyOf".to_string(),
            json!([
                {"type": "string"},
                {"type": "number"}
            ]),
        );

        let type_discovery = TypeDiscovery::new(
            "AnyOfEnum".to_string(),
            TypeKind::Enum,
            schema_location,
            schema_obj,
        );

        let result = generator.check_anyof_oneof_enum(&type_discovery);

        assert!(result.is_some(), "Should detect anyOf composition");
        assert!(result.unwrap(), "Should return true for anyOf");
    }

    #[test]
    fn test_check_anyof_oneof_enum_without_composition() {
        let discovery = create_test_discovery_with_root();
        let generator = EnumGenerator::new(&discovery, false);

        let simple_enum = create_simple_enum_type();
        let result = generator.check_anyof_oneof_enum(&simple_enum);

        assert!(
            result.is_none(),
            "Simple enum should not be detected as anyOf/oneOf"
        );
    }

    // ========================================================================================
    // Test Group 2: Schema Constants Generation (2 tests)
    // ========================================================================================

    #[test]
    fn test_generate_variant_schema_constants_simple() {
        let discovery = create_test_discovery_with_root();
        let generator = EnumGenerator::new(&discovery, false);

        let type_discovery = create_complex_enum_type();
        let constants =
            generator.generate_variant_schema_constants("TestEnum", &type_discovery.members);

        assert_eq!(constants.len(), 2, "Should generate 2 schema constants");

        // Check that constants contain VARIANT_0_SCHEMA and VARIANT_1_SCHEMA
        let tokens_str = constants
            .iter()
            .map(|t| t.to_string())
            .collect::<Vec<_>>()
            .join(" ");
        assert!(
            tokens_str.contains("VARIANT_0_SCHEMA"),
            "Should contain VARIANT_0_SCHEMA"
        );
        assert!(
            tokens_str.contains("VARIANT_1_SCHEMA"),
            "Should contain VARIANT_1_SCHEMA"
        );
    }

    #[test]
    fn test_generate_variant_schema_constants_with_refs() {
        let discovery = create_test_discovery_with_root();
        let generator = EnumGenerator::new(&discovery, false);

        // Create a variant with $ref
        let mut member_schema = serde_json::Map::new();
        member_schema.insert("$ref".to_string(), json!("#/definitions/TestType"));

        let member = MemberInfo {
            name: "Variant0".to_string(),
            member_type: MemberType::TypeReference("TestType".to_string()),
            required: true,
            is_nullable: false,
            schema: member_schema,
        };

        let constants = generator.generate_variant_schema_constants("TestEnum", &[member]);

        assert_eq!(constants.len(), 1);
        let tokens_str = constants[0].to_string();

        // Should include definitions from root schema when $ref is present
        assert!(
            tokens_str.contains("definitions"),
            "Should include definitions for $ref resolution"
        );
        assert!(
            tokens_str.contains("$schema"),
            "Should preserve the root schema dialect for validator fragments"
        );
    }

    #[test]
    fn test_generate_variant_schema_constants_with_external_refs() {
        let discovery = create_test_discovery_with_root();
        let generator = EnumGenerator::new(&discovery, false);

        let member = MemberInfo {
            name: "Variant0".to_string(),
            member_type: MemberType::TypeReference("ExternalPayload".to_string()),
            required: true,
            is_nullable: false,
            schema: json!({
                "type": "object",
                "properties": {
                    "eslintConfig": {
                        "$ref": "https://json.schemastore.org/eslintrc.json"
                    }
                }
            })
            .as_object()
            .unwrap()
            .clone(),
        };

        let constants = generator.generate_variant_schema_constants("TestEnum", &[member]);

        assert_eq!(constants.len(), 1);
        let tokens = constants[0].to_string();

        assert!(
            tokens.contains("VARIANT_0_EXTERNAL_RESOURCES"),
            "variant constants should expose a per-variant external resource registry"
        );
        assert!(
            tokens.contains("https://json.schemastore.org/eslintrc.json"),
            "direct vendored external refs should be embedded for generated validation"
        );
        assert!(
            tokens.contains("https://json.schemastore.org/partial-eslint-plugins.json"),
            "transitive vendored external refs should be embedded for generated validation"
        );
    }

    #[test]
    fn test_generate_variant_schema_constants_preserve_root_schema_dialect() {
        let discovery = create_test_discovery_with_root();
        let generator = EnumGenerator::new(&discovery, false);

        let member = MemberInfo {
            name: "Variant0".to_string(),
            member_type: MemberType::ComplexContainer {
                container: "Vec".to_string(),
                inner: "serde_json::Value".to_string(),
            },
            required: true,
            is_nullable: false,
            schema: json!({
                "type": "array",
                "items": [
                    {"type": "number", "enum": [0, 1, 2]},
                    {"type": "string", "enum": ["always", "never"]},
                    {}
                ],
                "additionalItems": false
            })
            .as_object()
            .unwrap()
            .clone(),
        };

        let constants = generator.generate_variant_schema_constants("Rule", &[member]);
        let tokens = constants[0].to_string();

        assert!(
            tokens.contains("$schema"),
            "variant constants should preserve root dialect for draft-07 tuple arrays"
        );
        assert!(
            tokens.contains("draft-07"),
            "variant constants should include the draft-07 URI"
        );
    }

    // ========================================================================================
    // Test Group 3: Validation Functions Generation (1 test)
    // ========================================================================================

    #[test]
    fn test_generate_variant_validation_functions() {
        let discovery = create_test_discovery_with_root();
        let generator = EnumGenerator::new(&discovery, false);

        let type_discovery = create_complex_enum_type();
        let validation_fns =
            generator.generate_variant_validation_functions("TestEnum", &type_discovery.members);

        assert_eq!(
            validation_fns.len(),
            2,
            "Should generate 2 validation functions"
        );

        let tokens_str = validation_fns
            .iter()
            .map(|t| t.to_string())
            .collect::<Vec<_>>()
            .join(" ");
        assert!(
            tokens_str.contains("validate_variant_0"),
            "Should contain validate_variant_0"
        );
        assert!(
            tokens_str.contains("validate_variant_1"),
            "Should contain validate_variant_1"
        );
        assert!(
            tokens_str.contains("jsonschema :: options"),
            "Should use jsonschema crate"
        );
        assert!(
            tokens_str.contains("with_retriever (TctOfflineRetriever)")
                && !tokens_str.contains("validator_for"),
            "Validators must be built with the offline retriever, not the default one"
        );
        assert!(
            tokens_str.contains("std :: sync :: OnceLock"),
            "Should cache compiled validators"
        );
        assert!(
            tokens_str.contains("jsonschema :: Validator"),
            "Should store compiled jsonschema validators"
        );
    }

    // ========================================================================================
    // Test Group 4: Deserialization Implementation Generation (2 tests)
    // ========================================================================================

    #[test]
    fn test_generate_schema_aware_deserialize_impl_anyof() {
        let discovery = create_test_discovery_with_root();
        let generator = EnumGenerator::new(&discovery, false);

        let type_discovery = create_complex_enum_type();
        let type_ident = format_ident!("TestEnum");

        let member_type_fn = |mt: &MemberType| -> TctResult<TokenStream> {
            match mt {
                MemberType::TypeReference(name) => Ok(quote::quote! { #name }),
                _ => Ok(quote::quote! { String }),
            }
        };

        let result = generator.generate_schema_aware_deserialize_impl(
            &type_ident,
            &type_discovery.members,
            true, // is_anyof = true
            &member_type_fn,
        );

        assert!(
            result.is_ok(),
            "Should generate deserialize impl successfully"
        );
        let tokens = result.unwrap().to_string();

        assert!(
            tokens.contains("Deserialize"),
            "Should contain Deserialize trait"
        );
        assert!(
            tokens.contains("matched_variants"),
            "Should track matched variants"
        );
    }

    #[test]
    fn test_generate_schema_aware_deserialize_impl_oneof() {
        let discovery = create_test_discovery_with_root();
        let generator = EnumGenerator::new(&discovery, false);

        let type_discovery = create_complex_enum_type();
        let type_ident = format_ident!("TestEnum");

        let member_type_fn = |mt: &MemberType| -> TctResult<TokenStream> {
            match mt {
                MemberType::TypeReference(name) => {
                    let ident = format_ident!("{}", name);
                    Ok(quote::quote! { #ident })
                }
                _ => Ok(quote::quote! { String }),
            }
        };

        let result = generator.generate_schema_aware_deserialize_impl(
            &type_ident,
            &type_discovery.members,
            false, // is_anyof = false (oneOf)
            &member_type_fn,
        );

        assert!(
            result.is_ok(),
            "Should generate deserialize impl successfully"
        );
        let tokens = result.unwrap().to_string();

        assert!(
            tokens.contains("Deserialize"),
            "Should contain Deserialize trait"
        );
        assert!(
            tokens.contains("Ambiguous schema"),
            "Should have error for multiple matches in oneOf"
        );
    }

    #[test]
    fn streaming_manual_default_uses_constructible_number_zero() {
        let discovery = create_test_discovery_with_root();
        let generator = EnumGenerator::new(&discovery, true);

        let schema_location = SchemaLocation {
            json_path: "#/definitions/NumberFirstEnum".to_string(),
            description: "number-first enum".to_string(),
            parent_type: None,
            property_name: None,
        };
        let mut type_discovery = TypeDiscovery::new(
            "NumberFirstEnum".to_string(),
            TypeKind::Enum,
            schema_location,
            serde_json::Map::new(),
        );
        type_discovery.add_member(MemberInfo {
            name: "Variant0".to_string(),
            member_type: MemberType::Primitive("serde_json::Number".to_string()),
            required: true,
            is_nullable: false,
            schema: json!({"type": "number"}).as_object().unwrap().clone(),
        });
        type_discovery.add_member(MemberInfo {
            name: "Variant1".to_string(),
            member_type: MemberType::Primitive("String".to_string()),
            required: true,
            is_nullable: false,
            schema: json!({"type": "string"}).as_object().unwrap().clone(),
        });

        let member_type_fn = |member_type: &MemberType| -> TctResult<TokenStream> {
            match member_type {
                MemberType::Primitive(value) if value == "serde_json::Number" => {
                    Ok(quote! { serde_json::Number })
                }
                MemberType::Primitive(value) if value == "String" => Ok(quote! { String }),
                other => panic!("unexpected test member type: {other:?}"),
            }
        };
        let derive_traits = |_traits: Option<&TraitDerivability>, _has_validators: bool| {
            quote! { #[derive(Debug, PartialEq, Clone, serde::Serialize, serde::Deserialize, Default)] }
        };
        let derive_traits_without_deserialize =
            |_traits: Option<&TraitDerivability>, _has_validators: bool| {
                quote! { #[derive(Debug, PartialEq, Clone, serde::Serialize, Default)] }
            };
        let derive_traits_without_default =
            |_traits: Option<&TraitDerivability>, _has_validators: bool| {
                quote! { #[derive(Debug, PartialEq, Clone, serde::Serialize, serde::Deserialize)] }
            };
        let derive_traits_without_deserialize_and_default =
            |_traits: Option<&TraitDerivability>, _has_validators: bool| {
                quote! { #[derive(Debug, PartialEq, Clone, serde::Serialize)] }
            };

        let generated = generator
            .generate(
                &type_discovery,
                None,
                None,
                &member_type_fn,
                &derive_traits,
                &derive_traits_without_deserialize,
                &derive_traits_without_default,
                &derive_traits_without_deserialize_and_default,
            )
            .expect("number-first streaming enum should generate");
        let implementation_file = parse_generated_file(generated.implementation_tokens);
        let default_method = generated_trait_method(
            &implementation_file,
            "NumberFirstEnum",
            "Default",
            "default",
        );

        assert_method_body_contains(
            default_method,
            "Self :: Variant0 (serde_json :: Number :: from (0))",
            "manual Default should construct a serde_json::Number explicitly",
        );
        assert_method_body_not_contains(
            default_method,
            "< serde_json :: Number as Default >",
            "serde_json::Number does not implement Default",
        );
    }

    // ========================================================================================
    // Test Group 5: Null Variant Handling (1 test)
    // ========================================================================================

    #[test]
    fn test_deserialize_impl_with_null_variant() {
        let discovery = create_test_discovery_with_root();
        let generator = EnumGenerator::new(&discovery, false);

        let schema_location = SchemaLocation {
            json_path: "#/definitions/NullableEnum".to_string(),
            description: "Enum with null variant".to_string(),
            parent_type: None,
            property_name: None,
        };

        let mut schema_obj = serde_json::Map::new();
        schema_obj.insert(
            "oneOf".to_string(),
            json!([
                {"type": "string"},
                {"type": "null"}
            ]),
        );

        let mut type_discovery = TypeDiscovery::new(
            "NullableEnum".to_string(),
            TypeKind::Enum,
            schema_location,
            schema_obj,
        );

        // Add string variant
        let member1 = MemberInfo {
            name: "Variant0".to_string(),
            member_type: MemberType::Primitive("String".to_string()),
            required: true,
            is_nullable: false,
            schema: json!({"type": "string"}).as_object().unwrap().clone(),
        };

        // Add null variant
        let member2 = MemberInfo {
            name: "Variant1".to_string(),
            member_type: MemberType::Primitive("Null".to_string()),
            required: true,
            is_nullable: true,
            schema: json!({"type": "null"}).as_object().unwrap().clone(),
        };

        type_discovery.add_member(member1);
        type_discovery.add_member(member2);

        let type_ident = format_ident!("NullableEnum");
        let member_type_fn = |mt: &MemberType| -> TctResult<TokenStream> {
            match mt {
                MemberType::Primitive(t) if t == "Null" => Ok(quote::quote! { () }),
                MemberType::Primitive(t) => {
                    let ident = format_ident!("{}", t);
                    Ok(quote::quote! { #ident })
                }
                _ => Ok(quote::quote! { String }),
            }
        };

        let result = generator.generate_schema_aware_deserialize_impl(
            &type_ident,
            &type_discovery.members,
            false,
            &member_type_fn,
        );

        assert!(result.is_ok(), "Should handle null variant successfully");
        let tokens = result.unwrap().to_string();

        // Null variant should have special handling (no deserialization)
        assert!(
            tokens.contains("matched_variants"),
            "Should validate variants"
        );
    }

    #[test]
    fn test_streaming_decode_prefix_with_container_variants() {
        // Test that Container (Vec<primitive>) and ComplexContainer (Vec<struct>) variants
        // are properly handled in decode_prefix generation
        let discovery = create_test_discovery_with_root();
        let generator = EnumGenerator::new(&discovery, true); // streaming enabled

        // Create members with Container and ComplexContainer variants
        let members = vec![
            MemberInfo {
                name: "StringVariant".to_string(),
                member_type: MemberType::Primitive("String".to_string()),
                required: true,
                is_nullable: false,
                schema: json!({"type": "string"}).as_object().unwrap().clone(),
            },
            MemberInfo {
                name: "VecStringVariant".to_string(),
                member_type: MemberType::Container {
                    container: "Vec".to_string(),
                    inner: "String".to_string(),
                },
                required: true,
                is_nullable: false,
                schema: json!({"type": "array", "items": {"type": "string"}})
                    .as_object()
                    .unwrap()
                    .clone(),
            },
            MemberInfo {
                name: "VecStructVariant".to_string(),
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
            },
        ];

        let type_ident = format_ident!("TestContainerEnum");

        let member_type_fn = |mt: &MemberType| -> TctResult<TokenStream> {
            match mt {
                MemberType::Primitive(t) => {
                    let ident = format_ident!("{}", t);
                    Ok(quote::quote! { #ident })
                }
                MemberType::Container { container, inner } => {
                    let container_ident = format_ident!("{}", container);
                    let inner_ident = format_ident!("{}", inner);
                    Ok(quote::quote! { #container_ident<#inner_ident> })
                }
                MemberType::ComplexContainer { container, inner } => {
                    let container_ident = format_ident!("{}", container);
                    let inner_ident = format_ident!("{}", inner);
                    Ok(quote::quote! { #container_ident<#inner_ident> })
                }
                MemberType::TypeReference(name) => {
                    let ident = format_ident!("{}", name);
                    Ok(quote::quote! { #ident })
                }
                _ => Ok(quote::quote! { () }),
            }
        };

        // Generate the streaming decode_prefix
        let result = generator.generate_enum_streaming_decode_prefix(
            "TestContainerEnum",
            &type_ident,
            &members,
            None,
            &member_type_fn,
        );

        assert!(
            result.is_ok(),
            "Should generate streaming decode_prefix for Container variants"
        );

        let generated_file = parse_generated_file(result.unwrap());
        let decode_prefix =
            generated_inherent_method(&generated_file, "TestContainerEnum", "decode_prefix");

        // Verify Vec<String> container variant uses the string-preserving helper.
        assert_method_body_contains(
            decode_prefix,
            "decode_string_vec_streaming",
            "Vec<String> container variant should preserve truncated in-progress strings",
        );

        // Verify ComplexContainer variant uses per-element decode_prefix
        // Note: Uses turbofish syntax <InnerStruct>::decode_prefix for trait disambiguation
        assert_method_body_contains(
            decode_prefix,
            "< InnerStruct > :: decode_prefix",
            "ComplexContainer variant should use per-element decode_prefix with turbofish syntax",
        );

        // Verify it doesn't try to call Vec::decode_prefix (which doesn't exist)
        assert_method_body_not_contains(
            decode_prefix,
            "Vec < String > :: decode_prefix",
            "Should NOT try to call Vec::decode_prefix",
        );
    }

    #[test]
    fn test_streaming_decode_prefix_vec_json_integer_variant_uses_qualified_inner_type() {
        let discovery = create_test_discovery_with_root();
        let generator = EnumGenerator::new(&discovery, true);
        let members = vec![
            MemberInfo {
                name: "IntSlice".to_string(),
                member_type: MemberType::Container {
                    container: "Vec".to_string(),
                    inner: "JsonInteger".to_string(),
                },
                required: true,
                is_nullable: false,
                schema: json!({"type": "array", "items": {"type": "integer"}})
                    .as_object()
                    .unwrap()
                    .clone(),
            },
            MemberInfo {
                name: "Label".to_string(),
                member_type: MemberType::Primitive("String".to_string()),
                required: true,
                is_nullable: false,
                schema: json!({"type": "string"}).as_object().unwrap().clone(),
            },
        ];
        let type_ident = format_ident!("JsonIntegerEnum");
        let member_type_fn = |member_type: &MemberType| -> TctResult<TokenStream> {
            Ok(match member_type {
                MemberType::Container { container, inner } => {
                    let container_ident = format_ident!("{}", container);
                    let inner_tokens = container_inner_type_tokens(inner);
                    quote! { #container_ident<#inner_tokens> }
                }
                _ => quote! { () },
            })
        };

        let result = generator
            .generate_enum_streaming_decode_prefix(
                "JsonIntegerEnum",
                &type_ident,
                &members,
                None,
                &member_type_fn,
            )
            .expect("streaming enum decode_prefix generation should succeed");

        let code = result.to_string();
        assert!(
            code.contains("decode_vec_streaming :: < crate :: tokenizer :: JsonInteger >"),
            "streaming enum Vec decode should qualify JsonInteger: {code}"
        );
        assert!(
            !code.contains("decode_vec_streaming :: < JsonInteger >"),
            "streaming enum Vec decode must not use unqualified JsonInteger: {code}"
        );
    }

    #[test]
    fn test_streaming_decode_prefix_with_hashmap_container() {
        // Test that HashMap container variants are properly handled
        let discovery = create_test_discovery_with_root();
        let generator = EnumGenerator::new(&discovery, true); // streaming enabled

        let members = vec![
            MemberInfo {
                name: "StringVariant".to_string(),
                member_type: MemberType::Primitive("String".to_string()),
                required: true,
                is_nullable: false,
                schema: json!({"type": "string"}).as_object().unwrap().clone(),
            },
            MemberInfo {
                name: "HashMapVariant".to_string(),
                member_type: MemberType::HashMap {
                    key: "String".to_string(),
                    value: "i64".to_string(),
                },
                required: true,
                is_nullable: false,
                schema: json!({"type": "object", "additionalProperties": {"type": "integer"}})
                    .as_object()
                    .unwrap()
                    .clone(),
            },
        ];

        let type_ident = format_ident!("TestHashMapEnum");

        let member_type_fn = |mt: &MemberType| -> TctResult<TokenStream> {
            match mt {
                MemberType::Primitive(t) => {
                    let ident = format_ident!("{}", t);
                    Ok(quote::quote! { #ident })
                }
                MemberType::HashMap { key, value } => {
                    let key_ident = format_ident!("{}", key);
                    let value_ident = format_ident!("{}", value);
                    Ok(quote::quote! { std::collections::HashMap<#key_ident, #value_ident> })
                }
                _ => Ok(quote::quote! { () }),
            }
        };

        let result = generator.generate_enum_streaming_decode_prefix(
            "TestHashMapEnum",
            &type_ident,
            &members,
            None,
            &member_type_fn,
        );

        assert!(
            result.is_ok(),
            "Should generate streaming decode_prefix for HashMap variants"
        );

        let tokens = result.unwrap().to_string();

        // HashMap should use full decode (no streaming support for HashMap)
        assert!(
            tokens.contains("decode"),
            "HashMap variant should use decode"
        );
    }

    #[test]
    fn test_streaming_decode_prefix_with_type_reference() {
        // Test that TypeReference variants use decode_prefix for progressive decoding
        let discovery = create_test_discovery_with_root();
        let generator = EnumGenerator::new(&discovery, true); // streaming enabled

        let members = vec![
            MemberInfo {
                name: "PrimitiveVariant".to_string(),
                member_type: MemberType::Primitive("String".to_string()),
                required: true,
                is_nullable: false,
                schema: json!({"type": "string"}).as_object().unwrap().clone(),
            },
            MemberInfo {
                name: "StructVariant".to_string(),
                member_type: MemberType::TypeReference("MyStruct".to_string()),
                required: true,
                is_nullable: false,
                schema: json!({"$ref": "#/definitions/MyStruct"})
                    .as_object()
                    .unwrap()
                    .clone(),
            },
        ];

        let type_ident = format_ident!("TestTypeRefEnum");

        let member_type_fn = |mt: &MemberType| -> TctResult<TokenStream> {
            match mt {
                MemberType::Primitive(t) => {
                    let ident = format_ident!("{}", t);
                    Ok(quote::quote! { #ident })
                }
                MemberType::TypeReference(name) => {
                    let ident = format_ident!("{}", name);
                    Ok(quote::quote! { #ident })
                }
                _ => Ok(quote::quote! { () }),
            }
        };

        let result = generator.generate_enum_streaming_decode_prefix(
            "TestTypeRefEnum",
            &type_ident,
            &members,
            None,
            &member_type_fn,
        );

        assert!(
            result.is_ok(),
            "Should generate streaming decode_prefix for TypeReference variants"
        );

        let generated_file = parse_generated_file(result.unwrap());
        let decode_prefix =
            generated_inherent_method(&generated_file, "TestTypeRefEnum", "decode_prefix");

        // TypeReference should call decode_prefix on the inner type
        assert!(
            generated_method_body_contains(decode_prefix, "decode_prefix")
                && generated_method_body_contains(decode_prefix, "MyStruct"),
            "TypeReference variant should use inner type's decode_prefix"
        );
    }

    #[test]
    fn test_streaming_decode_prefix_boxed_type_reference_decodes_inner_type() {
        let discovery = create_test_discovery_with_root();
        let generator = EnumGenerator::new(&discovery, true);

        let members = vec![
            MemberInfo {
                name: "PrimitiveVariant".to_string(),
                member_type: MemberType::Primitive("String".to_string()),
                required: true,
                is_nullable: false,
                schema: json!({"type": "string"}).as_object().unwrap().clone(),
            },
            MemberInfo {
                name: "BoxedStructVariant".to_string(),
                member_type: MemberType::TypeReference("RecursiveStruct".to_string()),
                required: true,
                is_nullable: false,
                schema: json!({"$ref": "#/definitions/RecursiveStruct"})
                    .as_object()
                    .unwrap()
                    .clone(),
            },
        ];
        let type_ident = format_ident!("BoxedTypeRefEnum");
        let member_type_fn = |member_type: &MemberType| -> TctResult<TokenStream> {
            Ok(match member_type {
                MemberType::Primitive(t) => {
                    let ident = format_ident!("{}", t);
                    quote! { #ident }
                }
                MemberType::TypeReference(name) => {
                    let ident = format_ident!("{}", name);
                    quote! { Box<#ident> }
                }
                _ => quote! { () },
            })
        };

        let generated_file = parse_generated_file(
            generator
                .generate_enum_streaming_decode_prefix(
                    "BoxedTypeRefEnum",
                    &type_ident,
                    &members,
                    None,
                    &member_type_fn,
                )
                .expect("streaming enum decode_prefix generation should succeed"),
        );
        let decode_prefix =
            generated_inherent_method(&generated_file, "BoxedTypeRefEnum", "decode_prefix");

        assert_method_body_contains(
            decode_prefix,
            "< RecursiveStruct > :: decode_prefix",
            "boxed TypeReference variants should decode the inner type",
        );
        assert_method_body_contains(
            decode_prefix,
            "Box :: new (inner_result . instance)",
            "boxed TypeReference variants should wrap prefix results in Box",
        );
        assert_method_body_not_contains(
            decode_prefix,
            "< Box < RecursiveStruct > > :: decode_prefix",
            "Box<T> has no generated decode_prefix method",
        );
    }

    #[test]
    fn test_streaming_decode_prefix_disabled() {
        // Test that streaming decode_prefix is NOT generated when streaming is disabled
        let discovery = create_test_discovery_with_root();
        let generator = EnumGenerator::new(&discovery, false); // streaming DISABLED

        let members = vec![MemberInfo {
            name: "StringVariant".to_string(),
            member_type: MemberType::Primitive("String".to_string()),
            required: true,
            is_nullable: false,
            schema: json!({"type": "string"}).as_object().unwrap().clone(),
        }];

        let type_ident = format_ident!("TestEnum");

        let member_type_fn = |mt: &MemberType| -> TctResult<TokenStream> {
            match mt {
                MemberType::Primitive(t) => {
                    let ident = format_ident!("{}", t);
                    Ok(quote::quote! { #ident })
                }
                _ => Ok(quote::quote! { () }),
            }
        };

        let result = generator.generate_enum_streaming_decode_prefix(
            "TestEnum",
            &type_ident,
            &members,
            None,
            &member_type_fn,
        );

        // Should still succeed but produce empty tokens when streaming is disabled
        // (the caller checks streaming_encode flag before calling this)
        assert!(
            result.is_ok(),
            "Should not fail even when streaming disabled"
        );
    }

    #[test]
    fn test_streaming_decode_prefix_all_variant_types() {
        // Comprehensive test with ALL supported variant types
        let discovery = create_test_discovery_with_root();
        let generator = EnumGenerator::new(&discovery, true);

        let members = vec![
            // Primitive variant
            MemberInfo {
                name: "StringVariant".to_string(),
                member_type: MemberType::Primitive("String".to_string()),
                required: true,
                is_nullable: false,
                schema: json!({"type": "string"}).as_object().unwrap().clone(),
            },
            // Container (Vec<primitive>)
            MemberInfo {
                name: "VecI64Variant".to_string(),
                member_type: MemberType::Container {
                    container: "Vec".to_string(),
                    inner: "i64".to_string(),
                },
                required: true,
                is_nullable: false,
                schema: json!({"type": "array", "items": {"type": "integer"}})
                    .as_object()
                    .unwrap()
                    .clone(),
            },
            // ComplexContainer (Vec<struct>)
            MemberInfo {
                name: "VecStructVariant".to_string(),
                member_type: MemberType::ComplexContainer {
                    container: "Vec".to_string(),
                    inner: "InnerType".to_string(),
                },
                required: true,
                is_nullable: false,
                schema: json!({"type": "array", "items": {"$ref": "#/definitions/InnerType"}})
                    .as_object()
                    .unwrap()
                    .clone(),
            },
            // TypeReference (single struct)
            MemberInfo {
                name: "StructVariant".to_string(),
                member_type: MemberType::TypeReference("OuterType".to_string()),
                required: true,
                is_nullable: false,
                schema: json!({"$ref": "#/definitions/OuterType"})
                    .as_object()
                    .unwrap()
                    .clone(),
            },
        ];

        let type_ident = format_ident!("ComprehensiveEnum");

        let member_type_fn = |mt: &MemberType| -> TctResult<TokenStream> {
            match mt {
                MemberType::Primitive(t) => {
                    let ident = format_ident!("{}", t);
                    Ok(quote::quote! { #ident })
                }
                MemberType::Container { container, inner } => {
                    let c = format_ident!("{}", container);
                    let i = format_ident!("{}", inner);
                    Ok(quote::quote! { #c<#i> })
                }
                MemberType::ComplexContainer { container, inner } => {
                    let c = format_ident!("{}", container);
                    let i = format_ident!("{}", inner);
                    Ok(quote::quote! { #c<#i> })
                }
                MemberType::TypeReference(name) => {
                    let ident = format_ident!("{}", name);
                    Ok(quote::quote! { #ident })
                }
                _ => Ok(quote::quote! { () }),
            }
        };

        let result = generator.generate_enum_streaming_decode_prefix(
            "ComprehensiveEnum",
            &type_ident,
            &members,
            None,
            &member_type_fn,
        );

        assert!(result.is_ok(), "Should handle all variant types");

        let generated_file = parse_generated_file(result.unwrap());
        let decode_prefix =
            generated_inherent_method(&generated_file, "ComprehensiveEnum", "decode_prefix");

        // Verify each variant type is handled correctly:
        // 1. Primitive uses regular decode (check for decode call with String)
        assert!(
            generated_method_body_contains(decode_prefix, "decode")
                && generated_method_body_contains(decode_prefix, "String"),
            "Primitive should use decode"
        );

        // 2. Container uses decode_vec_streaming
        assert_method_body_contains(
            decode_prefix,
            "decode_vec_streaming",
            "Container should use decode_vec_streaming",
        );

        // 3. ComplexContainer uses per-element decode_prefix
        assert!(
            generated_method_body_contains(decode_prefix, "decode_prefix")
                && generated_method_body_contains(decode_prefix, "InnerType"),
            "ComplexContainer should use inner type's decode_prefix"
        );

        // 4. TypeReference uses decode_prefix
        assert!(
            generated_method_body_contains(decode_prefix, "decode_prefix")
                && generated_method_body_contains(decode_prefix, "OuterType"),
            "TypeReference should use decode_prefix"
        );

        // Verify the function signature
        assert!(
            matches!(decode_prefix.vis, syn::Visibility::Public(_)),
            "Should have public decode_prefix function"
        );
        assert!(
            generated_method_signature_contains(decode_prefix, "PrefixDecodeResult"),
            "Should return PrefixDecodeResult<Self>"
        );
    }

    #[test]
    fn test_streaming_decode_prefix_with_codec() {
        // Test streaming decode_prefix with codec-based encoding
        let discovery = create_test_discovery_with_root();
        let generator = EnumGenerator::new(&discovery, true);

        let members = vec![
            MemberInfo {
                name: "Variant0".to_string(),
                member_type: MemberType::Primitive("String".to_string()),
                required: true,
                is_nullable: false,
                schema: json!({"type": "string"}).as_object().unwrap().clone(),
            },
            MemberInfo {
                name: "Variant1".to_string(),
                member_type: MemberType::Container {
                    container: "Vec".to_string(),
                    inner: "bool".to_string(),
                },
                required: true,
                is_nullable: false,
                schema: json!({"type": "array", "items": {"type": "boolean"}})
                    .as_object()
                    .unwrap()
                    .clone(),
            },
        ];

        let type_ident = format_ident!("CodecEnum");

        // Create a codec spec using the calculate method and wrap in EncodingDecision
        let codec_spec = crate::generator::encoding_decision::CodecSpec::calculate(255, Some(256));
        let encoding_decision = EncodingDecision::EnumCodec {
            variants: vec!["Variant0".to_string(), "Variant1".to_string()],
            codec_spec,
        };

        let member_type_fn = |mt: &MemberType| -> TctResult<TokenStream> {
            match mt {
                MemberType::Primitive(t) => {
                    let ident = format_ident!("{}", t);
                    Ok(quote::quote! { #ident })
                }
                MemberType::Container { container, inner } => {
                    let c = format_ident!("{}", container);
                    let i = format_ident!("{}", inner);
                    Ok(quote::quote! { #c<#i> })
                }
                _ => Ok(quote::quote! { () }),
            }
        };

        let result = generator.generate_enum_streaming_decode_prefix(
            "CodecEnum",
            &type_ident,
            &members,
            Some(&encoding_decision),
            &member_type_fn,
        );

        assert!(result.is_ok(), "Should handle codec-based encoding");

        let generated_file = parse_generated_file(result.unwrap());
        let decode_prefix =
            generated_inherent_method(&generated_file, "CodecEnum", "decode_prefix");

        // With codec, should use codec module for variant index decoding
        // The codec module name is generated from the type name (codecenum_codec)
        assert!(
            generated_method_body_contains(decode_prefix, "codecenum_codec")
                && generated_method_body_contains(decode_prefix, "decode_prefix"),
            "Should use codec module for variant decoding"
        );
    }

    #[test]
    fn test_streaming_decode_prefix_single_variant_zero_token() {
        // Test that single-variant enums (zero-token enums) do NOT consume any tokens
        // in decode_prefix. This is critical because:
        // 1. Single-variant enums encode as 0 tokens (no discriminator needed)
        // 2. Standard decode() does not consume any tokens
        // 3. decode_prefix() must also not consume tokens to maintain alignment
        //
        // Bug discovered: Previously decode_prefix always tried to read a discriminator
        // token, causing token stream misalignment for nested structs.
        let discovery = create_test_discovery_with_root();
        let generator = EnumGenerator::new(&discovery, true); // streaming enabled

        // Create a single-variant enum (like NamespaceKind with only "Namespace")
        let members = vec![MemberInfo {
            name: "OnlyVariant".to_string(),
            member_type: MemberType::Primitive("()".to_string()),
            required: true,
            is_nullable: false,
            schema: json!({"type": "null"}).as_object().unwrap().clone(),
        }];

        let type_ident = format_ident!("SingleVariantEnum");

        let member_type_fn =
            |_mt: &MemberType| -> TctResult<TokenStream> { Ok(quote::quote! { () }) };

        let result = generator.generate_enum_streaming_decode_prefix(
            "SingleVariantEnum",
            &type_ident,
            &members,
            None,
            &member_type_fn,
        );

        assert!(
            result.is_ok(),
            "Should generate streaming decode_prefix for single-variant enum"
        );

        let tokens = result.unwrap().to_string();

        // Should NOT consume any tokens - the parameter should be _tokens (unused)
        assert!(
            tokens.contains("_tokens"),
            "Single-variant decode_prefix should NOT use tokens parameter (use _tokens). \
             This is a zero-token decode. Got code:\n{}",
            &tokens[..tokens.len().min(1500)]
        );

        // Should immediately return the single variant
        assert!(
            tokens.contains("Self :: OnlyVariant"),
            "Should return the single variant immediately"
        );

        // Should NOT have tokens.next() call
        assert!(
            !tokens.contains("tokens . next"),
            "Single-variant decode_prefix should NOT call tokens.next(). \
             Zero-token enums must not consume any tokens."
        );

        // Should have fields_decoded: 0 (no tokens consumed)
        assert!(
            tokens.contains("fields_decoded : 0"),
            "Single-variant decode_prefix should report 0 fields decoded (zero tokens consumed)"
        );
    }

    #[test]
    fn test_constant_enum_encode_does_not_name_an_unused_tokens_parameter() {
        // A single-variant enum encodes zero tokens. Naming the parameter `tokens` would
        // make every generated crate warn about an unused variable, and users build
        // generated code without the lint caps that this repository's own builds apply.
        let discovery = create_test_discovery_with_root();
        let generator = EnumGenerator::new(&discovery, false);
        let members = vec![MemberInfo {
            name: "OnlyVariant".to_string(),
            member_type: MemberType::Primitive("()".to_string()),
            required: true,
            is_nullable: false,
            schema: json!({"type": "null"}).as_object().unwrap().clone(),
        }];
        let type_ident = format_ident!("ConstantEnum");
        let member_type_fn =
            |_mt: &MemberType| -> TctResult<TokenStream> { Ok(quote::quote! { () }) };
        let decision = EncodingDecision::SingleVariantEnum {
            variant: "OnlyVariant".to_string(),
        };

        let generated = generator
            .generate_enum_token_encoder_impl(
                "ConstantEnum",
                &type_ident,
                &members,
                Some(&decision),
                &member_type_fn,
            )
            .expect("constant enum should generate a JsonTokenEncoder impl");
        let file = parse_generated_file(generated);
        let encode = generated_trait_method(&file, "ConstantEnum", "JsonTokenEncoder", "encode");

        assert!(
            generated_method_signature_contains(encode, "_tokens : & mut Vec < u32 >"),
            "constant enum encode must name its unused parameter _tokens"
        );
        assert_method_body_not_contains(encode, "tokens .", "constant enum encode writes nothing");
    }

    #[test]
    fn test_indexed_enum_encode_keeps_the_tokens_parameter_it_writes_to() {
        let discovery = create_test_discovery_with_root();
        let generator = EnumGenerator::new(&discovery, false);
        let type_discovery = create_simple_enum_type();
        let type_ident = format_ident!("Status");
        let member_type_fn =
            |_mt: &MemberType| -> TctResult<TokenStream> { Ok(quote::quote! { () }) };

        let generated = generator
            .generate_enum_token_encoder_impl(
                "Status",
                &type_ident,
                &type_discovery.members,
                Some(&EncodingDecision::Standard),
                &member_type_fn,
            )
            .expect("indexed enum should generate a JsonTokenEncoder impl");
        let file = parse_generated_file(generated);
        let encode = generated_trait_method(&file, "Status", "JsonTokenEncoder", "encode");

        assert!(
            generated_method_signature_contains(encode, "tokens : & mut Vec < u32 >")
                && !generated_method_signature_contains(encode, "_tokens"),
            "an enum whose encode pushes the variant index must keep the tokens parameter"
        );
        assert_method_body_contains(
            encode,
            "tokens . push",
            "indexed enum encode pushes an index",
        );
    }

    #[test]
    fn test_streaming_decode_prefix_two_variant_enum_consumes_token() {
        // Verify that multi-variant enums (2+ variants) DO consume a discriminator token.
        // This is the counterpart to the single-variant test - we need to ensure
        // multi-variant enums still work correctly after the single-variant fix.
        let discovery = create_test_discovery_with_root();
        let generator = EnumGenerator::new(&discovery, true);

        // Create a two-variant simple enum
        let members = vec![
            MemberInfo {
                name: "VariantA".to_string(),
                member_type: MemberType::Primitive("()".to_string()),
                required: true,
                is_nullable: false,
                schema: json!({"type": "null"}).as_object().unwrap().clone(),
            },
            MemberInfo {
                name: "VariantB".to_string(),
                member_type: MemberType::Primitive("()".to_string()),
                required: true,
                is_nullable: false,
                schema: json!({"type": "null"}).as_object().unwrap().clone(),
            },
        ];

        let type_ident = format_ident!("TwoVariantEnum");

        let member_type_fn =
            |_mt: &MemberType| -> TctResult<TokenStream> { Ok(quote::quote! { () }) };

        let result = generator.generate_enum_streaming_decode_prefix(
            "TwoVariantEnum",
            &type_ident,
            &members,
            None,
            &member_type_fn,
        );

        assert!(result.is_ok());
        let tokens = result.unwrap().to_string();

        // Multi-variant enums SHOULD consume a token for the discriminator
        assert!(
            tokens.contains("tokens . next"),
            "Two-variant enum decode_prefix MUST call tokens.next() for discriminator"
        );

        // Should have match arms for both variants
        assert!(
            tokens.contains("0u32 =>") && tokens.contains("1u32 =>"),
            "Should have match arms for variant indices 0 and 1"
        );

        // Parameter should be `tokens` (used), not `_tokens` (unused)
        assert!(
            !tokens.contains("_tokens : & mut"),
            "Multi-variant enum should use tokens parameter, not _tokens"
        );
    }

    #[test]
    fn test_streaming_decode_prefix_complex_single_variant_with_payload() {
        // Test single-variant enum WITH a payload (not unit variant).
        // Even with a payload, if there's only one variant, no discriminator is needed.
        // But the payload itself needs to be decoded.
        let discovery = create_test_discovery_with_root();
        let generator = EnumGenerator::new(&discovery, true);

        // Single variant with String payload
        let members = vec![MemberInfo {
            name: "OnlyVariant".to_string(),
            member_type: MemberType::Primitive("String".to_string()),
            required: true,
            is_nullable: false,
            schema: json!({"type": "string"}).as_object().unwrap().clone(),
        }];

        let type_ident = format_ident!("SinglePayloadEnum");

        let member_type_fn = |mt: &MemberType| -> TctResult<TokenStream> {
            match mt {
                MemberType::Primitive(t) => {
                    let ident = format_ident!("{}", t);
                    Ok(quote::quote! { #ident })
                }
                _ => Ok(quote::quote! { () }),
            }
        };

        let result = generator.generate_enum_streaming_decode_prefix(
            "SinglePayloadEnum",
            &type_ident,
            &members,
            None,
            &member_type_fn,
        );

        assert!(result.is_ok());
        let generated_file = parse_generated_file(result.unwrap());
        let decode_prefix =
            generated_inherent_method(&generated_file, "SinglePayloadEnum", "decode_prefix");

        // Single-variant with payload still shouldn't read discriminator
        assert_method_body_not_contains(
            decode_prefix,
            "instance : Self :: Variant0 ,",
            "tuple single-variant enums must construct the payload, not use the variant constructor as a value",
        );
        assert!(
            generated_method_body_contains(
                decode_prefix,
                "String as crate :: tokenizer :: JsonTokenEncoder"
            ) || generated_method_body_contains(decode_prefix, "String > :: decode"),
            "single-variant payload decode_prefix should decode the payload without a discriminator"
        );
        assert_method_body_not_contains(
            decode_prefix,
            "tokens . next",
            "single-variant payload decode_prefix should not consume a discriminator token",
        );
    }

    #[test]
    fn test_streaming_decode_prefix_complex_container_uses_concrete_length_decode() {
        let discovery = create_test_discovery_with_root();
        let generator = EnumGenerator::new(&discovery, true);
        let members = vec![
            MemberInfo {
                name: "StructItems".to_string(),
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
            },
            MemberInfo {
                name: "Label".to_string(),
                member_type: MemberType::Primitive("String".to_string()),
                required: true,
                is_nullable: false,
                schema: json!({"type": "string"}).as_object().unwrap().clone(),
            },
        ];
        let type_ident = format_ident!("ComplexVecEnum");
        let member_type_fn = |member_type: &MemberType| -> TctResult<TokenStream> {
            Ok(match member_type {
                MemberType::ComplexContainer { container, inner } => {
                    let container_ident = format_ident!("{}", container);
                    let inner_ident = format_ident!("{}", inner);
                    quote! { #container_ident<#inner_ident> }
                }
                MemberType::Primitive(t) => {
                    let ident = format_ident!("{}", t);
                    quote! { #ident }
                }
                _ => quote! { () },
            })
        };

        let generated_file = parse_generated_file(
            generator
                .generate_enum_streaming_decode_prefix(
                    "ComplexVecEnum",
                    &type_ident,
                    &members,
                    None,
                    &member_type_fn,
                )
                .expect("streaming enum decode_prefix generation should succeed"),
        );
        let decode_prefix =
            generated_inherent_method(&generated_file, "ComplexVecEnum", "decode_prefix");

        assert_method_body_contains(
            decode_prefix,
            "variable_base :: decode_u32_fixed_base",
            "ComplexContainer prefix decode should use the generated Vec length codec",
        );
        assert_method_body_not_contains(
            decode_prefix,
            "JsonTokenEncoder :: decode (tokens)",
            "ComplexContainer prefix decode should not rely on an ambiguous generic length decode",
        );
    }

    #[test]
    fn test_streaming_decode_prefix_boxed_complex_container_wraps_items() {
        let discovery = create_test_discovery_with_root();
        let generator = EnumGenerator::new(&discovery, true);
        let members = vec![
            MemberInfo {
                name: "StructItems".to_string(),
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
            },
            MemberInfo {
                name: "Label".to_string(),
                member_type: MemberType::Primitive("String".to_string()),
                required: true,
                is_nullable: false,
                schema: json!({"type": "string"}).as_object().unwrap().clone(),
            },
        ];
        let type_ident = format_ident!("BoxedComplexVecEnum");
        let member_type_fn = |member_type: &MemberType| -> TctResult<TokenStream> {
            Ok(match member_type {
                MemberType::ComplexContainer { container, inner } => {
                    let container_ident = format_ident!("{}", container);
                    let inner_ident = format_ident!("{}", inner);
                    quote! { #container_ident<Box<#inner_ident>> }
                }
                MemberType::Primitive(t) => {
                    let ident = format_ident!("{}", t);
                    quote! { #ident }
                }
                _ => quote! { () },
            })
        };

        let generated_file = parse_generated_file(
            generator
                .generate_enum_streaming_decode_prefix(
                    "BoxedComplexVecEnum",
                    &type_ident,
                    &members,
                    None,
                    &member_type_fn,
                )
                .expect("streaming enum decode_prefix generation should succeed"),
        );
        let decode_prefix =
            generated_inherent_method(&generated_file, "BoxedComplexVecEnum", "decode_prefix");

        assert_method_body_contains(
            decode_prefix,
            "< InnerStruct > :: decode_prefix",
            "boxed ComplexContainer variants should decode the inner element type",
        );
        assert_method_body_contains(
            decode_prefix,
            "items . push (Box :: new (element_result . instance))",
            "boxed ComplexContainer variants should wrap decoded items in Box",
        );
    }

    #[test]
    fn test_streaming_decode_prefix_variant_count_boundary() {
        // Test the boundary between single-variant (0 tokens) and multi-variant (1+ tokens)
        let discovery = create_test_discovery_with_root();
        let generator = EnumGenerator::new(&discovery, true);

        let member_type_fn =
            |_mt: &MemberType| -> TctResult<TokenStream> { Ok(quote::quote! { () }) };

        // Test with exactly 1 variant
        let one_variant = vec![MemberInfo {
            name: "Only".to_string(),
            member_type: MemberType::Primitive("()".to_string()),
            required: true,
            is_nullable: false,
            schema: json!({"type": "null"}).as_object().unwrap().clone(),
        }];

        let result1 = generator.generate_enum_streaming_decode_prefix(
            "OneVariant",
            &format_ident!("OneVariant"),
            &one_variant,
            None,
            &member_type_fn,
        );
        let code1 = result1.unwrap().to_string();
        assert!(
            code1.contains("_tokens"),
            "1 variant: should NOT consume tokens"
        );

        // Test with exactly 2 variants
        let two_variants = vec![
            MemberInfo {
                name: "First".to_string(),
                member_type: MemberType::Primitive("()".to_string()),
                required: true,
                is_nullable: false,
                schema: json!({"type": "null"}).as_object().unwrap().clone(),
            },
            MemberInfo {
                name: "Second".to_string(),
                member_type: MemberType::Primitive("()".to_string()),
                required: true,
                is_nullable: false,
                schema: json!({"type": "null"}).as_object().unwrap().clone(),
            },
        ];

        let result2 = generator.generate_enum_streaming_decode_prefix(
            "TwoVariants",
            &format_ident!("TwoVariants"),
            &two_variants,
            None,
            &member_type_fn,
        );
        let code2 = result2.unwrap().to_string();
        assert!(
            code2.contains("tokens . next"),
            "2 variants: MUST consume discriminator token"
        );

        // Test with 3 variants
        let three_variants = vec![
            MemberInfo {
                name: "A".to_string(),
                member_type: MemberType::Primitive("()".to_string()),
                required: true,
                is_nullable: false,
                schema: json!({"type": "null"}).as_object().unwrap().clone(),
            },
            MemberInfo {
                name: "B".to_string(),
                member_type: MemberType::Primitive("()".to_string()),
                required: true,
                is_nullable: false,
                schema: json!({"type": "null"}).as_object().unwrap().clone(),
            },
            MemberInfo {
                name: "C".to_string(),
                member_type: MemberType::Primitive("()".to_string()),
                required: true,
                is_nullable: false,
                schema: json!({"type": "null"}).as_object().unwrap().clone(),
            },
        ];

        let result3 = generator.generate_enum_streaming_decode_prefix(
            "ThreeVariants",
            &format_ident!("ThreeVariants"),
            &three_variants,
            None,
            &member_type_fn,
        );
        let code3 = result3.unwrap().to_string();
        assert!(
            code3.contains("tokens . next"),
            "3 variants: MUST consume discriminator token"
        );
        assert!(
            code3.contains("% 3u32"),
            "3 variants: should use modulo 3 for variant index"
        );
    }
}
