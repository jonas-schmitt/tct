//! Prefix-decoding renderer for generated enum types.
//!
//! The generated methods decode discriminators first, then delegate to variant
//! prefix decoders so streaming callers can recover partial enum payloads with
//! field-progress metadata instead of requiring a complete token stream.

use super::EnumGenerator;
use super::support::container_inner_type_tokens;
use crate::TctResult;
use crate::generator::encoding_decision::EncodingDecision;
use crate::generator::schema_discovery::{MemberInfo, MemberType};
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};

impl<'a> EnumGenerator<'a> {
    /// Generate streaming-friendly decode_prefix method for enums
    ///
    /// This implementation enables progressive decoding by:
    /// 1. First decoding just the discriminator (variant index)
    /// 2. Then calling decode_prefix on the inner type (not full decode)
    /// 3. Wrapping the partial result in the correct variant
    ///
    /// This allows partial results even when the inner struct is incomplete.
    pub(super) fn generate_enum_streaming_decode_prefix(
        &self,
        type_name: &str,
        type_ident: &Ident,
        members: &[MemberInfo],
        encoding_decision: Option<&EncodingDecision>,
        member_type_fn: &dyn Fn(&MemberType) -> TctResult<TokenStream>,
    ) -> TctResult<TokenStream> {
        let variant_count = members.len() as u32;

        // Check if this is a simple enum (all unit variants)
        let is_simple_enum = members.iter().all(|member| {
            matches!(
                member.member_type,
                crate::generator::schema_discovery::MemberType::Primitive(ref t) if t == "()"
            )
        });

        // CRITICAL: single unit/null variants are "zero-token" - they encode nothing.
        // Single variants with payloads still encode the payload, so prefix decode must
        // decode that payload without consuming a discriminator.
        if variant_count == 1 {
            let variant_name = if is_simple_enum {
                format_ident!("{}", members[0].name)
            } else {
                format_ident!("Variant0")
            };
            let is_zero_token_unit = matches!(
                &members[0].member_type,
                crate::generator::schema_discovery::MemberType::Primitive(t) if t == "Null" || t == "()"
            );

            if !is_zero_token_unit {
                let decode_result = self.generate_variant_decode_prefix_result(
                    &variant_name,
                    &members[0],
                    member_type_fn,
                )?;

                return Ok(quote! {
                    impl #type_ident {
                        /// Decode from a potentially truncated token stream (streaming mode)
                        ///
                        /// Single payload variant: no discriminator is consumed, but the
                        /// payload is decoded with prefix-tolerant semantics.
                        ///
                        /// # Returns
                        /// - `PrefixDecodeResult` with the decoded instance and metadata
                        pub fn decode_prefix(
                            tokens: &mut impl Iterator<Item = u32>,
                        ) -> PrefixDecodeResult<Self> {
                            #decode_result
                        }
                    }
                });
            }

            return Ok(quote! {
                impl #type_ident {
                    /// Decode from a potentially truncated token stream (streaming mode)
                    ///
                    /// Single-variant enum: no discriminator needed, always returns the sole variant.
                    /// This is a zero-token decode - no tokens are consumed.
                    ///
                    /// # Returns
                    /// - `PrefixDecodeResult` with the decoded instance and metadata
                    pub fn decode_prefix(
                        _tokens: &mut impl Iterator<Item = u32>,
                    ) -> PrefixDecodeResult<Self> {
                        PrefixDecodeResult {
                            instance: Self::#variant_name,
                            fields_decoded: 0,
                            complete: true,
                            partial_field: None,
                        }
                    }
                }
            });
        }

        // Generate discriminator decode based on encoding decision
        let decode_discriminator =
            if let Some(EncodingDecision::EnumCodec { .. }) = encoding_decision {
                // Use codec for multi-token decode
                let codec_mod_name = format_ident!("{}_codec", type_name.to_lowercase());
                quote! {
                    let variant_index = match #codec_mod_name::decode_prefix(tokens) {
                        Some(idx) => idx,
                        None => {
                            // Can't even decode discriminator - return default
                            return PrefixDecodeResult {
                                instance: Self::default(),
                                fields_decoded: 0,
                                complete: false,
                                partial_field: None,
                            };
                        }
                    };
                }
            } else {
                // Standard single-token decode
                quote! {
                    let variant_index = match tokens.next() {
                        Some(idx) => idx,
                        None => {
                            // Can't even decode discriminator - return default
                            return PrefixDecodeResult {
                                instance: Self::default(),
                                fields_decoded: 0,
                                complete: false,
                                partial_field: None,
                            };
                        }
                    };
                }
            };

        // Generate decode_prefix arms for each variant
        let decode_prefix_arms = members
            .iter()
            .enumerate()
            .map(|(i, member)| {
                // Use actual variant name for simple enums, Variant{i} for complex enums
                let variant_name = if is_simple_enum {
                    format_ident!("{}", member.name)
                } else {
                    format_ident!("Variant{}", i)
                };
                let variant_index = i as u32;
                let decode_result = self.generate_variant_decode_prefix_result(
                    &variant_name,
                    member,
                    member_type_fn,
                )?;
                Ok(quote! {
                    #variant_index => {
                        #decode_result
                    }
                })
            })
            .collect::<TctResult<Vec<_>>>()?;

        Ok(quote! {
            impl #type_ident {
                /// Decode from a potentially truncated token stream (streaming mode)
                ///
                /// For enums, this first decodes the discriminator, then calls
                /// decode_prefix on the inner type for progressive decoding.
                /// This allows partial results even when the inner struct is incomplete.
                ///
                /// # Returns
                /// - `PrefixDecodeResult` with the decoded instance and metadata
                pub fn decode_prefix(tokens: &mut impl Iterator<Item = u32>) -> PrefixDecodeResult<Self> {
                    #decode_discriminator

                    // Apply modulo for graceful handling of out-of-range variant indices
                    let variant_index = variant_index % #variant_count;

                    match variant_index {
                        #(#decode_prefix_arms),*,
                        _ => unreachable!("Modulo operation ensures valid variant index")
                    }
                }
            }
        })
    }

    fn generate_variant_decode_prefix_result(
        &self,
        variant_name: &Ident,
        member: &MemberInfo,
        member_type_fn: &dyn Fn(&MemberType) -> TctResult<TokenStream>,
    ) -> TctResult<TokenStream> {
        let is_null = matches!(
            &member.member_type,
            crate::generator::schema_discovery::MemberType::Primitive(t) if t == "Null" || t == "()"
        );

        if is_null {
            return Ok(quote! {
                PrefixDecodeResult {
                    instance: Self::#variant_name,
                    fields_decoded: 1,
                    complete: true,
                    partial_field: None,
                }
            });
        }

        if matches!(
            &member.member_type,
            crate::generator::schema_discovery::MemberType::Primitive(_)
        ) {
            let member_type = member_type_fn(&member.member_type)?;
            return Ok(quote! {
                match <#member_type as crate::tokenizer::JsonTokenEncoder>::decode(tokens) {
                    Ok(value) => PrefixDecodeResult {
                        instance: Self::#variant_name(value),
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
            });
        }

        if let crate::generator::schema_discovery::MemberType::Container { container, inner } =
            &member.member_type
            && container == "Vec"
        {
            let inner_type = container_inner_type_tokens(inner);
            let vec_decode_call = if inner == "String" {
                quote! { crate::tokenizer::decode_string_vec_streaming(tokens) }
            } else {
                quote! { crate::tokenizer::decode_vec_streaming::<#inner_type>(tokens) }
            };
            return Ok(quote! {
                let (items, complete, _expected_length) = #vec_decode_call;
                let _partial_field = if complete {
                    None
                } else {
                    Some(PartialFieldInfo {
                        field_name: "variant",
                        field_type: "Vec",
                        items_decoded: items.len(),
                        expected_length: _expected_length,
                    })
                };
                PrefixDecodeResult {
                    instance: Self::#variant_name(items),
                    fields_decoded: 1,
                    complete,
                    partial_field: _partial_field,
                }
            });
        }

        if let crate::generator::schema_discovery::MemberType::ComplexContainer { container, inner } =
            &member.member_type
            && container == "Vec"
        {
            let inner_ident = format_ident!("{}", inner);
            let member_type = member_type_fn(&member.member_type)?;
            let needs_boxing = Self::token_stream_is_vec_of_boxed_type(&member_type, inner);
            let element_type = if needs_boxing {
                quote! { Box<#inner_ident> }
            } else {
                quote! { #inner_ident }
            };
            let push_item = if needs_boxing {
                quote! { items.push(Box::new(element_result.instance)); }
            } else {
                quote! { items.push(element_result.instance); }
            };

            return Ok(quote! {
                let vec_length: usize = match crate::tokenizer::variable_base::decode_u32_fixed_base(tokens, "Vec<T>") {
                    Ok(len) => len as usize,
                    Err(_) => {
                        return PrefixDecodeResult {
                            instance: Self::default(),
                            fields_decoded: 0,
                            complete: false,
                            partial_field: Some(PartialFieldInfo {
                                field_name: "variant",
                                field_type: "Vec",
                                items_decoded: 0,
                                expected_length: None,
                            }),
                        };
                    }
                };

                let mut items: Vec<#element_type> = Vec::with_capacity(vec_length);
                let mut all_complete = true;
                for _ in 0..vec_length {
                    let element_result = <#inner_ident>::decode_prefix(tokens);
                    #push_item
                    if !element_result.complete {
                        all_complete = false;
                        break;
                    }
                }
                let _partial_field = if all_complete {
                    None
                } else {
                    Some(PartialFieldInfo {
                        field_name: "variant",
                        field_type: "Vec",
                        items_decoded: items.len(),
                        expected_length: Some(vec_length),
                    })
                };
                PrefixDecodeResult {
                    instance: Self::#variant_name(items),
                    fields_decoded: 1,
                    complete: all_complete,
                    partial_field: _partial_field,
                }
            });
        }

        if matches!(
            &member.member_type,
            crate::generator::schema_discovery::MemberType::HashMap { .. }
        ) {
            let member_type = member_type_fn(&member.member_type)?;
            return Ok(quote! {
                match <#member_type as crate::tokenizer::JsonTokenEncoder>::decode(tokens) {
                    Ok(inner) => PrefixDecodeResult {
                        instance: Self::#variant_name(inner),
                        fields_decoded: 1,
                        complete: true,
                        partial_field: None,
                    },
                    Err(_) => PrefixDecodeResult {
                        instance: Self::#variant_name(Default::default()),
                        fields_decoded: 0,
                        complete: false,
                        partial_field: None,
                    },
                }
            });
        }

        if let crate::generator::schema_discovery::MemberType::TypeReference(name) =
            &member.member_type
        {
            let member_type = member_type_fn(&member.member_type)?;
            let inner_ident = format_ident!("{}", name);
            let wrap_instance = if Self::token_stream_is_boxed_type(&member_type, name) {
                quote! { Box::new(inner_result.instance) }
            } else {
                quote! { inner_result.instance }
            };

            return Ok(quote! {
                let inner_result = <#inner_ident>::decode_prefix(tokens);
                PrefixDecodeResult {
                    instance: Self::#variant_name(#wrap_instance),
                    fields_decoded: inner_result.fields_decoded,
                    complete: inner_result.complete,
                    partial_field: inner_result.partial_field,
                }
            });
        }

        let member_type = member_type_fn(&member.member_type)?;
        Ok(quote! {
            let inner_result = <#member_type>::decode_prefix(tokens);
            PrefixDecodeResult {
                instance: Self::#variant_name(inner_result.instance),
                fields_decoded: inner_result.fields_decoded,
                complete: inner_result.complete,
                partial_field: inner_result.partial_field,
            }
        })
    }

    fn token_stream_is_boxed_type(tokens: &TokenStream, type_name: &str) -> bool {
        let compact = tokens.to_string().replace(' ', "");
        compact == format!("Box<{type_name}>")
    }

    fn token_stream_is_vec_of_boxed_type(tokens: &TokenStream, type_name: &str) -> bool {
        let compact = tokens.to_string().replace(' ', "");
        compact == format!("Vec<Box<{type_name}>>")
    }
}
