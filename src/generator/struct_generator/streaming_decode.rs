//! Streaming `decode_prefix` renderer for generated structs.
//!
//! This module emits truncation-tolerant struct decoders used by prefix and
//! continuation workflows. The ordinary renderer decodes fields in schema
//! order, returns default-filled partial structs when decoding stops early, and
//! records partial-field metadata for token-consuming strings, vectors, nested
//! structs, and nested vector elements. Bit-packed structs are delegated to the
//! bit-packed renderer, while the zero-token-aware entry point routes
//! const-first bit-packed decoding when classifier metadata is available.

use super::StructGenerator;
use crate::TctResult;
use crate::generator::schema_discovery::{MemberInfo, MemberType};
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use rustc_hash::FxHashMap;

mod bitpacked;
mod partial;

impl<'a> StructGenerator<'a> {
    /// Generate streaming-friendly decode_prefix method
    ///
    /// This method enables truncation-tolerant decoding for LLM autocompletion:
    /// - Attempts to decode each field in order
    /// - Returns partial struct with defaults when decoding fails or EOF
    /// - Provides field index where decoding stopped
    ///
    /// Token format is identical to standard encode/decode - no special markers.
    /// Supports both standard encoding and bit-packed encoding (O3/Os optimization).
    fn generate_streaming_decode_prefix(
        &self,
        type_ident: &Ident,
        type_name: &str,
        members: &[MemberInfo],
        field_name_map: &FxHashMap<String, String>,
        encoding_decision: Option<&crate::generator::encoding_decision::EncodingDecision>,
        member_type_fn: &dyn Fn(&MemberType) -> TctResult<TokenStream>,
    ) -> TctResult<TokenStream> {
        // Check for bit-packed encoding
        if let Some(crate::generator::encoding_decision::EncodingDecision::BitPackedStruct {
            presence_tokens_needed,
            ..
        }) = encoding_decision
        {
            return self.generate_bitpacked_streaming_decode_prefix(
                type_ident,
                type_name,
                members,
                field_name_map,
                *presence_tokens_needed,
                member_type_fn,
            );
        }

        // Standard (non-bit-packed) streaming decode_prefix
        let field_count = members.len();

        // Capture encoding_base for use in quote! blocks
        let encoding_base = self.encoding_base;

        // Generate decode attempts for each field
        // Each field decoder returns the partial struct if it fails
        let mut decode_attempts = Vec::new();
        let mut field_idents = Vec::new();

        for (idx, member) in members.iter().enumerate() {
            let sanitized_name = field_name_map.get(&member.name).unwrap_or(&member.name);
            let field_ident = format_ident!("{}", sanitized_name);
            field_idents.push(field_ident.clone());

            // Create partial struct constructor with fields decoded so far
            let partial_struct = partial::before_field(members, field_name_map, &field_idents, idx);

            // Check if this is a required String field - use streaming decode for partial results
            let is_required_string = member.required
                && matches!(&member.member_type, MemberType::Primitive(t) if t == "String");

            // Check if this is a required Vec<T> field - use streaming decode for partial results
            let is_required_vec = member.required
                && matches!(&member.member_type, MemberType::Container { container, .. } if container == "Vec");

            // Check if this is a required Vec<Struct> field (ComplexContainer) - use recursive decode_prefix per element
            let is_required_complex_vec = member.required
                && matches!(&member.member_type, MemberType::ComplexContainer { container, .. } if container == "Vec");

            // Optional Option<Vec<Struct>> fields carry a presence flag before
            // the vector payload. When present, recurse per element so nested
            // truncation metadata is preserved.
            let is_optional_complex_vec = !member.required
                && member.uses_option_presence_flag()
                && matches!(&member.member_type, MemberType::ComplexContainer { container, .. } if container == "Vec");

            // Check if this is a TypeReference field (nested struct/enum) - use recursive decode_prefix
            let is_type_reference =
                member.required && matches!(&member.member_type, MemberType::TypeReference(_));

            // Optional Option<T> references carry a presence flag before the nested
            // value. When present, recurse so nested truncation metadata is preserved.
            let is_optional_type_reference = !member.required
                && member.uses_option_presence_flag()
                && matches!(&member.member_type, MemberType::TypeReference(_));

            // Nullable OptionalField<T> references use a three-state flag
            // (absent/null/present) before the nested value.
            let is_nullable_type_reference = !member.required
                && member.uses_optional_field_encoding()
                && matches!(&member.member_type, MemberType::TypeReference(_));

            let decode_attempt = if is_required_string {
                // For required String fields, use streaming decode to get partial content
                // Build partial struct including this field with partial string value
                let partial_struct_with_field =
                    partial::through_field(members, field_name_map, &field_idents, idx);

                let field_name_lit = sanitized_name.as_str();
                quote! {
                    let (#field_ident, _string_complete, _bytes_decoded) = crate::tokenizer::decode_string_streaming(tokens);
                    if !_string_complete {
                        // String was truncated - return partial result with the partial string
                        return PrefixDecodeResult {
                            instance: #partial_struct_with_field,
                            fields_decoded: #idx + 1, // Count this field as decoded (partially)
                            complete: false,
                            partial_field: Some(PartialFieldInfo {
                                field_name: #field_name_lit,
                                field_type: "String",
                                items_decoded: _bytes_decoded,
                                expected_length: None,
                            }),
                        };
                    }
                }
            } else if is_required_vec {
                // For required Vec fields, use streaming decode to get partial elements
                let partial_struct_with_field =
                    partial::through_field(members, field_name_map, &field_idents, idx);

                // Get the inner type for Vec<T>
                let inner_type = match &member.member_type {
                    MemberType::Container { inner, .. } => Self::container_inner_type_tokens(inner),
                    _ => quote! { () },
                };
                let vec_decode_call = match &member.member_type {
                    MemberType::Container { inner, .. } if inner == "String" => {
                        quote! { crate::tokenizer::decode_string_vec_streaming(tokens) }
                    }
                    _ => quote! { crate::tokenizer::decode_vec_streaming::<#inner_type>(tokens) },
                };

                let field_name_lit = sanitized_name.as_str();
                quote! {
                    let (#field_ident, _vec_complete, _expected_length) = #vec_decode_call;
                    if !_vec_complete {
                        // Vec was truncated - return partial result with the partial elements
                        let _items_decoded = #field_ident.len();
                        return PrefixDecodeResult {
                            instance: #partial_struct_with_field,
                            fields_decoded: #idx + 1, // Count this field as decoded (partially)
                            complete: false,
                            partial_field: Some(PartialFieldInfo {
                                field_name: #field_name_lit,
                                field_type: "Vec",
                                items_decoded: _items_decoded,
                                expected_length: _expected_length,
                            }),
                        };
                    }
                }
            } else if is_required_complex_vec {
                // For Vec<Struct> fields, decode each element with decode_prefix for partial results
                let partial_struct_with_field =
                    partial::through_field(members, field_name_map, &field_idents, idx);

                // Get the inner type for Vec<InnerType> and check if boxing is needed
                let (inner_type, needs_boxing) = match &member.member_type {
                    MemberType::ComplexContainer { inner, .. } => {
                        let inner_ident = format_ident!("{}", inner);
                        (quote! { #inner_ident }, self.analysis.needs_boxing(inner))
                    }
                    _ => (quote! { () }, false),
                };

                let field_name_lit = sanitized_name.as_str();

                // Generate push statement with or without Box wrapping
                let push_stmt = if needs_boxing {
                    quote! { #field_ident.push(Box::new(_element_result.instance)); }
                } else {
                    quote! { #field_ident.push(_element_result.instance); }
                };

                quote! {
                    // Decode Vec length using multi-token variable_base encoding
                    let _vec_length: usize = match crate::tokenizer::variable_base::decode_u32(tokens, "Vec<T>", #encoding_base) {
                        Ok(len) => len as usize,
                        Err(_) => {
                            return PrefixDecodeResult {
                                instance: #partial_struct,
                                fields_decoded: #idx,
                                complete: false,
                                partial_field: Some(PartialFieldInfo {
                                    field_name: #field_name_lit,
                                    field_type: "Vec",
                                    items_decoded: 0,
                                    expected_length: None,
                                }),
                            };
                        }
                    };

                    // Decode each element with decode_prefix for partial results
                    let mut #field_ident = Vec::with_capacity(_vec_length);
                    for _i in 0.._vec_length {
                        let _element_result = <#inner_type>::decode_prefix(tokens);
                        #push_stmt
                        if !_element_result.complete {
                            // Element was truncated - return partial Vec
                            // Capture length before moving field_ident into struct
                            let _items_decoded = #field_ident.len();
                            return PrefixDecodeResult {
                                instance: #partial_struct_with_field,
                                fields_decoded: #idx + 1,
                                complete: false,
                                partial_field: Some(PartialFieldInfo {
                                    field_name: #field_name_lit,
                                    field_type: "Vec",
                                    items_decoded: _items_decoded,
                                    expected_length: Some(_vec_length),
                                }),
                            };
                        }
                    }
                }
            } else if is_optional_complex_vec {
                // For Option<Vec<Struct>> fields, preserve the presence-flag
                // recovery contract while recursing into each present element.
                let field_type = member_type_fn(&member.member_type)?;
                let field_name_lit = sanitized_name.as_str();
                let (inner_type, needs_boxing) = match &member.member_type {
                    MemberType::ComplexContainer { inner, .. } => {
                        let inner_ident = format_ident!("{}", inner);
                        (quote! { #inner_ident }, self.analysis.needs_boxing(inner))
                    }
                    _ => (quote! { () }, false),
                };

                let push_stmt = if needs_boxing {
                    quote! { _items.push(Box::new(_element_result.instance)); }
                } else {
                    quote! { _items.push(_element_result.instance); }
                };

                let element_type = if needs_boxing {
                    quote! { Box<#inner_type> }
                } else {
                    quote! { #inner_type }
                };

                let partial_struct_with_field = partial::with_field_value(
                    members,
                    field_name_map,
                    &field_idents,
                    idx,
                    &field_ident,
                    quote! { Some(_items) },
                );

                quote! {
                    let #field_ident: Option<#field_type> = match tokens.next() {
                        Some(_presence_token) if _presence_token % 2 == 1 => {
                            let _vec_length: usize = match crate::tokenizer::variable_base::decode_u32(tokens, "Vec<T>", #encoding_base) {
                                Ok(len) => len as usize,
                                Err(_) => {
                                    return PrefixDecodeResult {
                                        instance: #partial_struct,
                                        fields_decoded: #idx,
                                        complete: false,
                                        partial_field: Some(PartialFieldInfo {
                                            field_name: #field_name_lit,
                                            field_type: "Vec",
                                            items_decoded: 0,
                                            expected_length: None,
                                        }),
                                    };
                                }
                            };

                            let mut _items: Vec<#element_type> = Vec::with_capacity(_vec_length);
                            for _i in 0.._vec_length {
                                let _element_result = <#inner_type>::decode_prefix(tokens);
                                #push_stmt
                                if !_element_result.complete {
                                    let _items_decoded = _items.len();
                                    return PrefixDecodeResult {
                                        instance: #partial_struct_with_field,
                                        fields_decoded: #idx + 1,
                                        complete: false,
                                        partial_field: Some(PartialFieldInfo {
                                            field_name: #field_name_lit,
                                            field_type: "Vec",
                                            items_decoded: _items_decoded,
                                            expected_length: Some(_vec_length),
                                        }),
                                    };
                                }
                            }
                            Some(_items)
                        }
                        Some(_) => None,
                        None => {
                            return PrefixDecodeResult {
                                instance: #partial_struct,
                                fields_decoded: #idx,
                                complete: false,
                                partial_field: None,
                            };
                        }
                    };
                }
            } else if is_type_reference {
                // For TypeReference fields (nested structs/enums), use recursive decode_prefix
                let partial_struct_with_field =
                    partial::through_field(members, field_name_map, &field_idents, idx);

                let field_name_lit = sanitized_name.as_str();
                let (type_name_str, inner_type, needs_boxing) = match &member.member_type {
                    MemberType::TypeReference(name) => {
                        let inner = format_ident!("{}", name);
                        (
                            name.as_str(),
                            quote! { #inner },
                            self.analysis.needs_boxing(name),
                        )
                    }
                    _ => ("Unknown", quote! { () }, false),
                };

                // If the type needs boxing, decode the inner type and wrap in Box::new()
                let (decode_call, assign_stmt) = if needs_boxing {
                    (
                        quote! { let _nested_result = <#inner_type>::decode_prefix(tokens); },
                        quote! { let #field_ident = Box::new(_nested_result.instance); },
                    )
                } else {
                    (
                        quote! { let _nested_result = <#inner_type>::decode_prefix(tokens); },
                        quote! { let #field_ident = _nested_result.instance; },
                    )
                };

                quote! {
                    #decode_call
                    #assign_stmt
                    if !_nested_result.complete {
                        let _nested_fields_decoded = _nested_result.fields_decoded;
                        let _partial_field = match _nested_result.partial_field {
                            Some(_nested_partial_field) if _nested_partial_field.field_type == "Vec" => {
                                Some(PartialFieldInfo {
                                    field_name: #field_name_lit,
                                    field_type: _nested_partial_field.field_type,
                                    items_decoded: _nested_partial_field.items_decoded,
                                    expected_length: _nested_partial_field.expected_length,
                                })
                            }
                            _ => Some(PartialFieldInfo {
                                field_name: #field_name_lit,
                                field_type: #type_name_str,
                                items_decoded: _nested_fields_decoded,
                                expected_length: None,
                            }),
                        };
                        // Nested type was truncated - return partial result
                        return PrefixDecodeResult {
                            instance: #partial_struct_with_field,
                            fields_decoded: #idx + 1, // Count this field as decoded (partially)
                            complete: false,
                            partial_field: _partial_field,
                        };
                    }
                }
            } else if is_optional_type_reference {
                // For Option<TypeReference> fields, preserve the presence-flag recovery
                // contract while recursing into the nested prefix decoder when present.
                let field_type = member_type_fn(&member.member_type)?;
                let field_name_lit = sanitized_name.as_str();
                let (type_name_str, inner_type, needs_boxing) = match &member.member_type {
                    MemberType::TypeReference(name) => {
                        let inner = format_ident!("{}", name);
                        (
                            name.as_str(),
                            quote! { #inner },
                            self.analysis.needs_boxing(name),
                        )
                    }
                    _ => ("Unknown", quote! { () }, false),
                };

                let (wrap_for_early_return, wrap_for_final) = if needs_boxing {
                    (
                        quote! { Some(Box::new(_nested_result.instance)) },
                        quote! { Some(Box::new(_nested_result.instance)) },
                    )
                } else {
                    (
                        quote! { Some(_nested_result.instance) },
                        quote! { Some(_nested_result.instance) },
                    )
                };

                let partial_struct_with_field = partial::with_field_value(
                    members,
                    field_name_map,
                    &field_idents,
                    idx,
                    &field_ident,
                    wrap_for_early_return,
                );

                quote! {
                    let #field_ident: Option<#field_type> = match tokens.next() {
                        Some(_presence_token) if _presence_token % 2 == 1 => {
                            let _nested_result = <#inner_type>::decode_prefix(tokens);
                            if !_nested_result.complete {
                                let _nested_fields_decoded = _nested_result.fields_decoded;
                                let _partial_field = match _nested_result.partial_field {
                                    Some(_nested_partial_field) if _nested_partial_field.field_type == "Vec" => {
                                        Some(PartialFieldInfo {
                                            field_name: #field_name_lit,
                                            field_type: _nested_partial_field.field_type,
                                            items_decoded: _nested_partial_field.items_decoded,
                                            expected_length: _nested_partial_field.expected_length,
                                        })
                                    }
                                    _ => Some(PartialFieldInfo {
                                        field_name: #field_name_lit,
                                        field_type: #type_name_str,
                                        items_decoded: _nested_fields_decoded,
                                        expected_length: None,
                                    }),
                                };
                                return PrefixDecodeResult {
                                    instance: #partial_struct_with_field,
                                    fields_decoded: #idx + 1,
                                    complete: false,
                                    partial_field: _partial_field,
                                };
                            }
                            #wrap_for_final
                        }
                        Some(_) => None,
                        None => {
                            return PrefixDecodeResult {
                                instance: #partial_struct,
                                fields_decoded: #idx,
                                complete: false,
                                partial_field: None,
                            };
                        }
                    };
                }
            } else if is_nullable_type_reference {
                // For OptionalField<TypeReference> fields, preserve the
                // absent/null/present recovery contract while recursing into the
                // nested prefix decoder for present values.
                let field_type = member_type_fn(&member.member_type)?;
                let field_name_lit = sanitized_name.as_str();
                let (type_name_str, inner_type, needs_boxing) = match &member.member_type {
                    MemberType::TypeReference(name) => {
                        let inner = format_ident!("{}", name);
                        (
                            name.as_str(),
                            quote! { #inner },
                            self.analysis.needs_boxing(name),
                        )
                    }
                    _ => ("Unknown", quote! { () }, false),
                };

                let (wrap_for_early_return, wrap_for_final) = if needs_boxing {
                    (
                        quote! { crate::tokenizer::OptionalField::Present(Box::new(_nested_result.instance)) },
                        quote! { crate::tokenizer::OptionalField::Present(Box::new(_nested_result.instance)) },
                    )
                } else {
                    (
                        quote! { crate::tokenizer::OptionalField::Present(_nested_result.instance) },
                        quote! { crate::tokenizer::OptionalField::Present(_nested_result.instance) },
                    )
                };

                let partial_struct_with_field = partial::with_field_value(
                    members,
                    field_name_map,
                    &field_idents,
                    idx,
                    &field_ident,
                    wrap_for_early_return,
                );

                quote! {
                    let #field_ident: crate::tokenizer::OptionalField<#field_type> = match tokens.next() {
                        Some(_state_flag) => match _state_flag % 3 {
                            0 => crate::tokenizer::OptionalField::Absent,
                            1 => crate::tokenizer::OptionalField::Null,
                            2 => {
                                let _nested_result = <#inner_type>::decode_prefix(tokens);
                                if !_nested_result.complete {
                                    let _nested_fields_decoded = _nested_result.fields_decoded;
                                    let _partial_field = match _nested_result.partial_field {
                                        Some(_nested_partial_field) if _nested_partial_field.field_type == "Vec" => {
                                            Some(PartialFieldInfo {
                                                field_name: #field_name_lit,
                                                field_type: _nested_partial_field.field_type,
                                                items_decoded: _nested_partial_field.items_decoded,
                                                expected_length: _nested_partial_field.expected_length,
                                            })
                                        }
                                        _ => Some(PartialFieldInfo {
                                            field_name: #field_name_lit,
                                            field_type: #type_name_str,
                                            items_decoded: _nested_fields_decoded,
                                            expected_length: None,
                                        }),
                                    };
                                    return PrefixDecodeResult {
                                        instance: #partial_struct_with_field,
                                        fields_decoded: #idx + 1,
                                        complete: false,
                                        partial_field: _partial_field,
                                    };
                                }
                                #wrap_for_final
                            }
                            _ => unreachable!("Modulo 3 ensures valid optional-field state"),
                        },
                        None => {
                            return PrefixDecodeResult {
                                instance: #partial_struct,
                                fields_decoded: #idx,
                                complete: false,
                                partial_field: None,
                            };
                        }
                    };
                }
            } else if member.name == "additional_properties" {
                // For additional_properties, use fallback to empty map instead of returning early
                // This allows streaming decode to continue even if additional_properties decode fails
                // (e.g., when token stream is truncated before additional_properties data)
                quote! {
                    let #field_ident = crate::tokenizer::JsonTokenEncoder::decode(tokens)
                        .unwrap_or_default();
                }
            } else {
                // Standard decode for other types
                quote! {
                    let #field_ident = match crate::tokenizer::JsonTokenEncoder::decode(tokens) {
                        Ok(v) => v,
                        Err(_) => {
                            return PrefixDecodeResult {
                                instance: #partial_struct,
                                fields_decoded: #idx,
                                complete: false,
                                partial_field: None,
                            };
                        }
                    };
                }
            };

            decode_attempts.push(decode_attempt);
        }

        // Final struct construction (all fields decoded successfully)
        let all_fields: Vec<_> = field_idents.iter().map(|f| quote! { #f }).collect();

        Ok(quote! {
            impl #type_ident {
                /// Decode from a potentially truncated token stream (streaming mode)
                ///
                /// This method enables LLM autocompletion by:
                /// - Attempting to decode each field in order
                /// - Returning partial struct with defaults when decoding fails or EOF
                /// - Reporting how many fields were successfully decoded
                ///
                /// Token format is identical to standard encode/decode - no special markers.
                ///
                /// # Returns
                /// - `PrefixDecodeResult` with the decoded instance and metadata
                pub fn decode_prefix(tokens: &mut impl Iterator<Item = u32>) -> PrefixDecodeResult<Self> {
                    #( #decode_attempts )*

                    // All fields decoded successfully
                    PrefixDecodeResult {
                        instance: Self {
                            #( #all_fields ),*
                        },
                        fields_decoded: #field_count,
                        complete: true,
                        partial_field: None,
                    }
                }
            }
        })
    }

    /// Generate streaming decode_prefix with zero-token type information
    ///
    /// This variant enables const-first decoding: zero-token fields are decoded
    /// BEFORE variable-token fields, making them visible even when earlier
    /// variable-length fields are truncated.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn generate_streaming_decode_prefix_with_zero_token_info(
        &self,
        type_ident: &Ident,
        type_name: &str,
        members: &[MemberInfo],
        field_name_map: &FxHashMap<String, String>,
        encoding_decision: Option<&crate::generator::encoding_decision::EncodingDecision>,
        member_type_fn: &dyn Fn(&MemberType) -> TctResult<TokenStream>,
        is_zero_token_type_fn: Option<&dyn Fn(&str) -> bool>,
    ) -> TctResult<TokenStream> {
        // Check for bit-packed encoding
        if let Some(crate::generator::encoding_decision::EncodingDecision::BitPackedStruct {
            presence_tokens_needed,
            ..
        }) = encoding_decision
        {
            return self.generate_bitpacked_streaming_decode_prefix_with_zero_token_info(
                type_ident,
                type_name,
                members,
                field_name_map,
                *presence_tokens_needed,
                member_type_fn,
                is_zero_token_type_fn,
            );
        }

        // For non-bit-packed structs, use standard decode_prefix (no const-first optimization)
        self.generate_streaming_decode_prefix(
            type_ident,
            type_name,
            members,
            field_name_map,
            encoding_decision,
            member_type_fn,
        )
    }

    pub(super) fn container_inner_type_tokens(inner: &str) -> TokenStream {
        match inner {
            "serde_json::Value" => quote! { serde_json::Value },
            "serde_json::Number" => quote! { serde_json::Number },
            "JsonInteger" | "crate::tokenizer::JsonInteger" => {
                quote! { crate::tokenizer::JsonInteger }
            }
            "String" => quote! { String },
            "i32" => quote! { i32 },
            "i64" => quote! { i64 },
            "f64" => quote! { f64 },
            "bool" => quote! { bool },
            other => {
                let inner_ident = format_ident!("{}", other);
                quote! { #inner_ident }
            }
        }
    }
}
