//! Bit-packed optional-field presence rendering for generated structs.
//!
//! This module emits the O3/Os struct encoder and decoder variant that packs
//! many two-state `Option<T>` presence flags into generated `u32` mask groups,
//! encodes those masks through the structural codec, and keeps generated
//! `token_count` aligned with the compact presence representation.

use super::StructGenerator;
use crate::TctResult;
use crate::generator::encoding_decision::CodecSpec;
use crate::generator::schema_discovery::{MemberInfo, MemberType};
use crate::generator::structural_codec;
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use rustc_hash::FxHashMap;

impl<'a> StructGenerator<'a> {
    /// Generate bit-packed encoder for structs with many optional fields (O3/Os optimization)
    ///
    /// Packs optional field presence flags into u32 mask groups using bitwise operations:
    /// - For each optional `Option<T>` field: set corresponding bit in presence mask
    /// - Emit ceil(optional_count / 32) u32 masks containing packed presence bits
    /// - Encode each u32 mask through the structural codec, which may use multiple base tokens
    /// - Encode only present field values (skip absent fields)
    ///
    /// Example: 10 optional fields -> 1 presence-mask group
    /// Example: 35 optional fields -> 2 presence-mask groups (32 + 3 bits used)
    ///
    /// This reduces per-field presence overhead for structs with many optional fields.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn generate_bitpacked_encode(
        &self,
        type_ident: &Ident,
        type_name: &str,
        members: &[MemberInfo],
        field_name_map: &FxHashMap<String, String>,
        _optional_field_count: usize,
        presence_tokens_needed: usize,
        post_decode_validate: bool,
        member_type_fn: &dyn Fn(&MemberType) -> TctResult<TokenStream>,
    ) -> TctResult<TokenStream> {
        // Filter to optional Option<T> fields only (not OptionalField<T> which uses 3-state encoding)
        // Also exclude fields with serde_json::Value types which always use OptionalField<T>
        let optional_fields: Vec<_> = members
            .iter()
            .filter(|m| {
                if m.required || m.is_nullable {
                    return false;
                }

                // Check if field type contains serde_json::Value (would use OptionalField<T>)
                m.uses_option_presence_flag()
            })
            .collect();

        // Generate presence mask variable declarations (one u32 mask group per 32 fields).
        let mut mask_decls = Vec::new();
        for token_idx in 0..presence_tokens_needed {
            let var_name = format_ident!("presence_mask_{}", token_idx);
            mask_decls.push(quote! {
                let mut #var_name: u32 = 0;
            });
        }

        // Generate bit-setting logic for each optional field
        let mut bit_setters = Vec::new();
        for (field_idx, member) in optional_fields.iter().enumerate() {
            let sanitized_name = field_name_map.get(&member.name).unwrap_or(&member.name);
            let field_name = format_ident!("{}", sanitized_name);
            let token_idx = field_idx / 32;
            let bit_idx = field_idx % 32;
            let mask_var = format_ident!("presence_mask_{}", token_idx);

            bit_setters.push(quote! {
                if self.#field_name.is_some() {
                    #mask_var |= 1u32 << #bit_idx;
                }
            });
        }

        // Generate token push statements for presence masks using multi-token codec.
        // Presence masks are u32 values (0 to 2^32-1) that need to be encoded as multiple
        // base-256 tokens to stay within vocabulary bounds [0, 255]
        // Note: generate_codec_module appends "_codec" to the module name automatically
        let codec_module_name_ident =
            format_ident!("{}_presence_mask_codec", type_name.to_lowercase());
        let mut token_pushes = Vec::new();
        for token_idx in 0..presence_tokens_needed {
            let mask_var = format_ident!("presence_mask_{}", token_idx);
            token_pushes.push(quote! {
                #codec_module_name_ident::encode(#mask_var, tokens);
            });
        }

        // Generate field encoding statements (encode only present field values)
        let mut field_encoders = Vec::new();
        for member in optional_fields.iter() {
            let sanitized_name = field_name_map.get(&member.name).unwrap_or(&member.name);
            let field_name = format_ident!("{}", sanitized_name);
            field_encoders.push(quote! {
                if let Some(ref value) = self.#field_name {
                    value.encode(tokens);
                }
            });
        }

        // Encode required fields and OptionalField<T> fields using standard encoding
        // (all fields NOT in the bit-packed optional_fields set)
        let bitpacked_field_names: std::collections::HashSet<_> =
            optional_fields.iter().map(|m| &m.name).collect();

        let mut standard_field_encoders = Vec::new();
        for member in members.iter() {
            if !bitpacked_field_names.contains(&member.name) {
                let sanitized_name = field_name_map.get(&member.name).unwrap_or(&member.name);
                let field_name = format_ident!("{}", sanitized_name);
                standard_field_encoders.push(quote! {
                    self.#field_name.encode(tokens);
                });
            }
        }

        // Generate decode logic with bit-packed presence flags
        let decode_body = self.generate_bitpacked_decode(
            type_name,
            members,
            field_name_map,
            presence_tokens_needed,
            post_decode_validate,
            member_type_fn,
        )?;

        // Generate codec module for presence mask encoding (u32 -> base-256 multi-token)
        // This prevents huge tokens (like 1073741824 = 0x40000000) from exceeding vocabulary
        // Base 256 ensures all tokens stay in [0, 255], perfectly aligned with byte encoding
        // Make module name unique to avoid conflicts when multiple structs use bit-packing
        let codec_module_name = format!("{}_presence_mask", type_name.to_lowercase());
        let presence_mask_codec = {
            // u32 max = 2^32 - 1, base = 256 (power of 2 for optimal bit-shift encoding)
            let codec_spec = CodecSpec::calculate(u32::MAX, Some(256));
            structural_codec::generate_codec_module(&codec_module_name, &codec_spec)
        };

        // Combine all code: codec module + JsonTokenEncoder implementation
        Ok(quote! {
            // Codec module for encoding presence masks (OUTSIDE impl block)
            #presence_mask_codec

            impl crate::tokenizer::JsonTokenEncoder for #type_ident {
                fn encode(&self, tokens: &mut Vec<u32>) {
                    // Build presence bitmasks for optional Option<T> fields
                    #(#mask_decls)*
                    #(#bit_setters)*

                    // Emit presence-mask groups (multi-token encoded)
                    #(#token_pushes)*

                    // Encode required fields and OptionalField<T> fields (standard encoding)
                    #(#standard_field_encoders)*

                    // Encode present optional field values (in declaration order)
                    #(#field_encoders)*
                }

                #decode_body
            }
        })
    }

    /// Generate bit-packed decoder for structs with many optional fields (O3/Os optimization)
    ///
    /// Reads presence-mask groups and extracts individual bits to determine field presence:
    /// - Read ceil(optional_count / 32) u32 masks containing packed presence bits
    /// - For each optional field: extract bit at position (field_idx % 32) from the appropriate mask group
    /// - Conditionally decode field value if presence bit is set
    ///
    /// This decoding mirrors the bit-packed encoding strategy
    fn generate_bitpacked_decode(
        &self,
        type_name: &str,
        members: &[MemberInfo],
        field_name_map: &FxHashMap<String, String>,
        presence_tokens_needed: usize,
        post_decode_validate: bool,
        member_type_fn: &dyn Fn(&MemberType) -> TctResult<TokenStream>,
    ) -> TctResult<TokenStream> {
        // Filter to optional Option<T> fields only (not OptionalField<T>)
        // Also exclude fields with serde_json::Value types which always use OptionalField<T>
        let optional_fields: Vec<_> = members
            .iter()
            .filter(|m| {
                if m.required || m.is_nullable {
                    return false;
                }

                // Check if field type contains serde_json::Value (would use OptionalField<T>)
                m.uses_option_presence_flag()
            })
            .collect();

        // Generate presence-mask reads using multi-token codec.
        // Presence masks are encoded as multiple base-256 tokens [0, 255]
        // Note: generate_codec_module appends "_codec" to the module name automatically
        let codec_module_name_ident =
            format_ident!("{}_presence_mask_codec", type_name.to_lowercase());
        let mut token_reads = Vec::new();
        let mut strict_token_reads = Vec::new();
        for token_idx in 0..presence_tokens_needed {
            let mask_var = format_ident!("presence_mask_{}", token_idx);
            let mask_field_name = format!("presence_mask_{token_idx}");
            let used_bits = optional_fields.len().saturating_sub(token_idx * 32).min(32);
            let valid_mask = if used_bits == 32 {
                u32::MAX
            } else {
                (1u32 << used_bits) - 1
            };
            token_reads.push(quote! {
                let #mask_var = #codec_module_name_ident::decode(tokens).map_err(|e| {
                    // Convert DecodeError to provide better context
                    match e {
                        crate::tokenizer::DecodeError::UnexpectedEndOfStream { .. } => {
                            crate::tokenizer::DecodeError::UnexpectedEndOfStream {
                                context: crate::tokenizer::EOFContext::new(
                                    #type_name,
                                    Some("presence_flags"),
                                    "bit-packed presence mask (multi-token)"
                                )
                            }
                        }
                        other => other,
                    }
                })?;
            });
            strict_token_reads.push(quote! {
                let #mask_var = #codec_module_name_ident::decode_strict(tokens).map_err(|e| {
                    match e {
                        crate::tokenizer::DecodeError::UnexpectedEndOfStream { .. } => {
                            crate::tokenizer::DecodeError::UnexpectedEndOfStream {
                                context: crate::tokenizer::EOFContext::new(
                                    #type_name,
                                    Some(#mask_field_name),
                                    "bit-packed presence mask (multi-token)"
                                )
                            }
                        }
                        crate::tokenizer::DecodeError::InvalidToken(token) => {
                            crate::tokenizer::DecodeError::InvalidTokenInContext {
                                token,
                                context: crate::tokenizer::EOFContext::new(
                                    #type_name,
                                    Some(#mask_field_name),
                                    "presence mask codec digit below base"
                                )
                            }
                        }
                        other => other,
                    }
                })?;
                if (#mask_var & !#valid_mask) != 0 {
                    return Err(crate::tokenizer::DecodeError::InvalidTokenInContext {
                        token: #mask_var,
                        context: crate::tokenizer::EOFContext::new(
                            #type_name,
                            Some(#mask_field_name),
                            "presence mask bits for optional fields"
                        ),
                    });
                }
            });
        }

        // Generate field decoding statements for required and OptionalField<T> fields (standard decoding)
        // (all fields NOT in the bit-packed optional_fields set)
        let bitpacked_field_names: std::collections::HashSet<_> =
            optional_fields.iter().map(|m| &m.name).collect();

        let mut standard_field_decoders = Vec::new();
        let mut strict_standard_field_decoders = Vec::new();
        for member in members.iter() {
            if !bitpacked_field_names.contains(&member.name) {
                let sanitized_name = field_name_map.get(&member.name).unwrap_or(&member.name);
                let field_name = format_ident!("{}", sanitized_name);
                let decode_value = Self::decode_member_value_with_context(
                    type_name,
                    &member.name,
                    quote! { crate::tokenizer::JsonTokenEncoder::decode(tokens) },
                );
                standard_field_decoders.push(quote! {
                    let #field_name = #decode_value;
                });
                let strict_decode_value = Self::decode_member_value_with_context(
                    type_name,
                    &member.name,
                    quote! { crate::tokenizer::JsonTokenEncoder::decode_strict(tokens) },
                );
                strict_standard_field_decoders.push(quote! {
                    let #field_name = #strict_decode_value;
                });
            }
        }

        // Generate conditional field decoding for bit-packed optional fields
        let mut field_decoders = Vec::new();
        let mut strict_field_decoders = Vec::new();
        for (field_idx, member) in optional_fields.iter().enumerate() {
            let sanitized_name = field_name_map.get(&member.name).unwrap_or(&member.name);
            let field_name = format_ident!("{}", sanitized_name);
            let field_type = member_type_fn(&member.member_type).unwrap_or_else(|_| quote! { () });
            let token_idx = field_idx / 32;
            let bit_idx = field_idx % 32;
            let mask_var = format_ident!("presence_mask_{}", token_idx);
            let decode_value = Self::decode_member_value_with_context(
                type_name,
                &member.name,
                quote! { crate::tokenizer::JsonTokenEncoder::decode(tokens) },
            );

            field_decoders.push(quote! {
                let #field_name: Option<#field_type> = if (#mask_var & (1u32 << #bit_idx)) != 0 {
                    Some(#decode_value)
                } else {
                    None
                };
            });

            let strict_decode_value = Self::decode_member_value_with_context(
                type_name,
                &member.name,
                quote! { crate::tokenizer::JsonTokenEncoder::decode_strict(tokens) },
            );

            strict_field_decoders.push(quote! {
                let #field_name: Option<#field_type> = if (#mask_var & (1u32 << #bit_idx)) != 0 {
                    Some(#strict_decode_value)
                } else {
                    None
                };
            });
        }

        let standard_token_count_fields = members
            .iter()
            .filter(|member| !bitpacked_field_names.contains(&member.name))
            .map(|member| {
                let sanitized_name = field_name_map.get(&member.name).unwrap_or(&member.name);
                let field_name = format_ident!("{}", sanitized_name);
                quote! {
                    total += self.#field_name.token_count();
                }
            })
            .collect::<Vec<_>>();

        let bitpacked_token_count_fields = optional_fields
            .iter()
            .map(|member| {
                let sanitized_name = field_name_map.get(&member.name).unwrap_or(&member.name);
                let field_name = format_ident!("{}", sanitized_name);
                quote! {
                    if let Some(ref value) = self.#field_name {
                        total += value.token_count();
                    }
                }
            })
            .collect::<Vec<_>>();

        // Build field list for struct construction (order matches declaration order)
        let construct_fields: Vec<_> = members
            .iter()
            .map(|member| {
                let sanitized_name = field_name_map.get(&member.name).unwrap_or(&member.name);
                let field_name = format_ident!("{}", sanitized_name);
                quote! { #field_name }
            })
            .collect();
        let post_decode_validation = Self::post_decode_validation(type_name, post_decode_validate);

        // Combine decode logic
        Ok(quote! {
            fn decode(tokens: &mut impl Iterator<Item = u32>) -> Result<Self, crate::tokenizer::DecodeError> {
                #[allow(unused_imports)]
                use crate::tokenizer::JsonTokenEncoder;

                // Read presence-mask groups
                #(#token_reads)*

                // Decode required fields and OptionalField<T> fields (standard decoding)
                #(#standard_field_decoders)*

                // Decode optional Option<T> fields conditionally based on presence bits
                #(#field_decoders)*

                // Construct instance with all fields
                let decoded = Self {
                    #(#construct_fields),*
                };
                #post_decode_validation
                Ok(decoded)
            }

            fn decode_strict(tokens: &mut impl Iterator<Item = u32>) -> Result<Self, crate::tokenizer::DecodeError> {
                #[allow(unused_imports)]
                use crate::tokenizer::JsonTokenEncoder;

                // Read and validate presence-mask groups without modulo recovery.
                #(#strict_token_reads)*

                // Decode required fields and OptionalField<T> fields (standard decoding).
                #(#strict_standard_field_decoders)*

                // Decode optional Option<T> fields conditionally based on presence bits.
                #(#strict_field_decoders)*

                // Construct instance with all fields.
                let decoded = Self {
                    #(#construct_fields),*
                };
                #post_decode_validation
                Ok(decoded)
            }

            fn token_count(&self) -> usize {
                #[allow(unused_imports)]
                use crate::tokenizer::JsonTokenEncoder;

                // Start with the encoded presence-mask digits.
                let mut total = #presence_tokens_needed * #codec_module_name_ident::TOKEN_WIDTH;

                // Required fields and three-state OptionalField<T> fields use standard encoding.
                #(#standard_token_count_fields)*

                // Bit-packed Option<T> fields do not emit individual presence flags.
                #(#bitpacked_token_count_fields)*

                total
            }
        })
    }
}
