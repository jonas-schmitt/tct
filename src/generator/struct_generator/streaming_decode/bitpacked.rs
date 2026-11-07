//! Streaming decode renderer for bit-packed generated structs.
//!
//! This module emits the ordinary `decode_prefix` implementation used when
//! optional `Option<T>` fields are encoded behind compact presence masks. The
//! generated decoder reads presence masks with a safe absent-field fallback,
//! decodes standard fields in schema order, and then decodes optional fields
//! only when their presence bits are set. It preserves partial-prefix metadata
//! for token-consuming strings, vectors, nested types, and nested vector
//! elements while falling back to defaults where doing so keeps prefix decode
//! useful for truncated streams.

use super::StructGenerator;
use crate::TctResult;
use crate::generator::schema_discovery::{MemberInfo, MemberType};
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use rustc_hash::FxHashMap;

mod zero_token;

impl<'a> StructGenerator<'a> {
    /// Generate streaming decode_prefix for bit-packed structs
    ///
    /// Handles the bit-packed encoding format:
    /// 1. Presence masks (multi-token encoded)
    /// 2. Standard fields (required + OptionalField<T>)
    /// 3. Optional fields (based on presence bits)
    pub(super) fn generate_bitpacked_streaming_decode_prefix(
        &self,
        type_ident: &Ident,
        type_name: &str,
        members: &[MemberInfo],
        field_name_map: &FxHashMap<String, String>,
        presence_tokens_needed: usize,
        member_type_fn: &dyn Fn(&MemberType) -> TctResult<TokenStream>,
    ) -> TctResult<TokenStream> {
        // Filter to optional Option<T> fields (bit-packed)
        let optional_fields: Vec<_> = members
            .iter()
            .filter(|m| {
                if m.required || m.is_nullable {
                    return false;
                }
                m.uses_option_presence_flag()
            })
            .collect();

        // Standard fields (required + OptionalField<T>) - NOT in bit-packed set
        let bitpacked_field_names: std::collections::HashSet<_> =
            optional_fields.iter().map(|m| &m.name).collect();

        let standard_fields: Vec<_> = members
            .iter()
            .filter(|m| !bitpacked_field_names.contains(&m.name))
            .collect();

        // Codec module name for presence masks
        let codec_module_name_ident =
            format_ident!("{}_presence_mask_codec", type_name.to_lowercase());

        // Capture encoding_base for use in quote! blocks
        let encoding_base = self.encoding_base;

        // All field identifiers for struct construction
        let all_field_idents: Vec<_> = members
            .iter()
            .map(|m| {
                let sanitized = field_name_map.get(&m.name).unwrap_or(&m.name);
                format_ident!("{}", sanitized)
            })
            .collect();

        // Phase 1: Decode presence masks with fallback
        // Use unwrap_or(0u32) instead of early return - this allows standard fields
        // (apiVersion, kind, etc.) to be decoded even when presence mask is truncated.
        // Fallback to 0 means "assume no optional fields present" which is safe.
        let mut presence_decodes = Vec::new();
        for token_idx in 0..presence_tokens_needed {
            let mask_var = format_ident!("presence_mask_{}", token_idx);
            presence_decodes.push(quote! {
                // Fallback: if presence mask decode fails, assume no optional fields present
                let #mask_var = #codec_module_name_ident::decode(tokens).unwrap_or(0u32);
            });
        }

        // Phase 2: Decode standard fields (track fields decoded)
        let mut standard_field_decodes = Vec::new();
        let mut decoded_standard_fields = Vec::new();

        for (idx, member) in standard_fields.iter().enumerate() {
            let sanitized = field_name_map.get(&member.name).unwrap_or(&member.name);
            let field_ident = format_ident!("{}", sanitized);
            decoded_standard_fields.push(field_ident.clone());

            // Partial struct with decoded standard fields + defaults for rest
            let partial_standard: Vec<_> = decoded_standard_fields
                .iter()
                .take(idx)
                .map(|f| quote! { #f })
                .collect();

            let remaining_standard: Vec<_> = standard_fields
                .iter()
                .skip(idx)
                .map(|m| {
                    crate::generator::struct_generator::default_impl::default_field_init(
                        m,
                        field_name_map,
                    )
                })
                .collect();

            let optional_defaults: Vec<_> = optional_fields
                .iter()
                .map(|m| {
                    crate::generator::struct_generator::default_impl::default_field_init(
                        m,
                        field_name_map,
                    )
                })
                .collect();

            let partial_struct = if partial_standard.is_empty() {
                quote! {
                    Self {
                        #( #remaining_standard, )*
                        #( #optional_defaults ),*
                    }
                }
            } else {
                quote! {
                    Self {
                        #( #partial_standard, )*
                        #( #remaining_standard, )*
                        #( #optional_defaults ),*
                    }
                }
            };

            let fields_decoded = idx; // 0-indexed count of successfully decoded fields

            // Check if this is a required String field - use streaming decode for partial results
            let is_required_string = member.required
                && matches!(&member.member_type, MemberType::Primitive(t) if t == "String");

            // Check if this is a required Vec<T> field - use streaming decode for partial results
            let is_required_vec = member.required
                && matches!(&member.member_type, MemberType::Container { container, .. } if container == "Vec");

            // Check if this is a required Vec<Struct> field (ComplexContainer) - use recursive decode_prefix per element
            let is_required_complex_vec = member.required
                && matches!(&member.member_type, MemberType::ComplexContainer { container, .. } if container == "Vec");

            // Check if this is a TypeReference field (nested struct/enum) - use recursive decode_prefix
            let is_type_reference =
                member.required && matches!(&member.member_type, MemberType::TypeReference(_));

            // Helper to build partial struct with current field
            let build_partial_struct_with_current = || {
                let partial_with_current: Vec<_> = decoded_standard_fields
                    .iter()
                    .map(|f| quote! { #f })
                    .collect();

                let remaining_after: Vec<_> = standard_fields
                    .iter()
                    .skip(idx + 1)
                    .map(|m| {
                        crate::generator::struct_generator::default_impl::default_field_init(
                            m,
                            field_name_map,
                        )
                    })
                    .collect();

                quote! {
                    Self {
                        #( #partial_with_current, )*
                        #( #remaining_after, )*
                        #( #optional_defaults ),*
                    }
                }
            };

            let decode_code = if is_required_string {
                // For required String fields, use streaming decode to get partial content
                let partial_struct_with_field = build_partial_struct_with_current();

                let field_name_lit = sanitized.as_str();
                quote! {
                    let (#field_ident, _string_complete, _bytes_decoded) = crate::tokenizer::decode_string_streaming(tokens);
                    if !_string_complete {
                        // String was truncated - return partial result with the partial string
                        return PrefixDecodeResult {
                            instance: #partial_struct_with_field,
                            fields_decoded: #fields_decoded + 1, // Count this field as decoded (partially)
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
                let partial_struct_with_field = build_partial_struct_with_current();

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

                let field_name_lit = sanitized.as_str();
                quote! {
                    let (#field_ident, _vec_complete, _expected_length) = #vec_decode_call;
                    if !_vec_complete {
                        // Vec was truncated - return partial result with the partial elements
                        let _items_decoded = #field_ident.len();
                        return PrefixDecodeResult {
                            instance: #partial_struct_with_field,
                            fields_decoded: #fields_decoded + 1, // Count this field as decoded (partially)
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
                let partial_struct_with_field = build_partial_struct_with_current();

                // Get the inner type for Vec<InnerType> and check if boxing is needed
                let (inner_type, needs_boxing) = match &member.member_type {
                    MemberType::ComplexContainer { inner, .. } => {
                        let inner_ident = format_ident!("{}", inner);
                        (quote! { #inner_ident }, self.analysis.needs_boxing(inner))
                    }
                    _ => (quote! { () }, false),
                };

                let field_name_lit = sanitized.as_str();

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
                                fields_decoded: #fields_decoded,
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
                                fields_decoded: #fields_decoded + 1,
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
            } else if is_type_reference {
                // For TypeReference fields (nested structs/enums), use recursive decode_prefix
                let partial_struct_with_field = build_partial_struct_with_current();

                let field_name_lit = sanitized.as_str();
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
                        // Nested type was truncated - return partial result
                        return PrefixDecodeResult {
                            instance: #partial_struct_with_field,
                            fields_decoded: #fields_decoded + 1, // Count this field as decoded (partially)
                            complete: false,
                            partial_field: Some(PartialFieldInfo {
                                field_name: #field_name_lit,
                                field_type: #type_name_str,
                                items_decoded: _nested_result.fields_decoded,
                                expected_length: None,
                            }),
                        };
                    }
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
                                fields_decoded: #fields_decoded,
                                complete: false,
                                partial_field: None,
                            };
                        }
                    };
                }
            };

            standard_field_decodes.push(decode_code);
        }

        // Phase 3: Decode optional fields based on presence bits
        let mut optional_field_decodes = Vec::new();
        let standard_field_count = standard_fields.len();

        for (field_idx, member) in optional_fields.iter().enumerate() {
            let sanitized = field_name_map.get(&member.name).unwrap_or(&member.name);
            let field_ident = format_ident!("{}", sanitized);
            let field_type = member_type_fn(&member.member_type).unwrap_or_else(|_| quote! { () });

            let token_idx = field_idx / 32;
            let bit_idx = field_idx % 32;
            let mask_var = format_ident!("presence_mask_{}", token_idx);

            // Calculate fields decoded at this point (standard + optional so far)
            let fields_decoded = standard_field_count + field_idx;

            // Build partial struct with standard fields + optional fields decoded so far
            let standard_assigns: Vec<_> = decoded_standard_fields
                .iter()
                .map(|f| quote! { #f })
                .collect();

            let optional_so_far: Vec<_> = optional_fields
                .iter()
                .take(field_idx)
                .map(|m| {
                    let f = format_ident!("{}", field_name_map.get(&m.name).unwrap_or(&m.name));
                    quote! { #f }
                })
                .collect();

            let optional_remaining: Vec<_> = optional_fields
                .iter()
                .skip(field_idx)
                .map(|m| {
                    crate::generator::struct_generator::default_impl::default_field_init(
                        m,
                        field_name_map,
                    )
                })
                .collect();

            let partial_struct = quote! {
                Self {
                    #( #standard_assigns, )*
                    #( #optional_so_far, )*
                    #( #optional_remaining ),*
                }
            };

            // Check if this is an optional String field - use streaming decode for partial results
            let is_optional_string =
                matches!(&member.member_type, MemberType::Primitive(t) if t == "String");

            // Check if this is an optional Vec<T> field - use streaming decode for partial results
            let is_optional_vec = matches!(&member.member_type, MemberType::Container { container, .. } if container == "Vec");

            // Check if this is an optional Vec<Struct> field (ComplexContainer)
            let is_optional_complex_vec = matches!(&member.member_type, MemberType::ComplexContainer { container, .. } if container == "Vec");

            // Check if this is an optional TypeReference field (nested struct/enum)
            let is_optional_type_reference =
                matches!(&member.member_type, MemberType::TypeReference(_));

            // Helper to build partial struct with optional fields
            // This version includes the current field (used after successful decode)
            let _build_partial_struct_with_optional = || {
                let optional_with_current: Vec<_> = optional_fields
                    .iter()
                    .take(field_idx + 1)
                    .map(|m| {
                        let f = format_ident!("{}", field_name_map.get(&m.name).unwrap_or(&m.name));
                        quote! { #f }
                    })
                    .collect();

                let optional_after: Vec<_> = optional_fields
                    .iter()
                    .skip(field_idx + 1)
                    .map(|m| {
                        crate::generator::struct_generator::default_impl::default_field_init(
                            m,
                            field_name_map,
                        )
                    })
                    .collect();

                quote! {
                    Self {
                        #( #standard_assigns, )*
                        #( #optional_with_current, )*
                        #( #optional_after ),*
                    }
                }
            };

            // Helper to build partial struct WITHOUT the current field (used during early return in decode)
            // The current field is assigned explicitly by the caller using the partial value
            let build_partial_struct_for_early_return =
                |current_value: proc_macro2::TokenStream| {
                    let optional_before_current: Vec<_> = optional_fields
                        .iter()
                        .take(field_idx)
                        .map(|m| {
                            let f =
                                format_ident!("{}", field_name_map.get(&m.name).unwrap_or(&m.name));
                            quote! { #f }
                        })
                        .collect();

                    let optional_after: Vec<_> = optional_fields
                        .iter()
                        .skip(field_idx + 1)
                        .map(|m| {
                            crate::generator::struct_generator::default_impl::default_field_init(
                                m,
                                field_name_map,
                            )
                        })
                        .collect();

                    quote! {
                        Self {
                            #( #standard_assigns, )*
                            #( #optional_before_current, )*
                            #field_ident: #current_value,
                            #( #optional_after ),*
                        }
                    }
                };

            let decode_code = if is_optional_string {
                // For optional String fields, use streaming decode
                // Use build_partial_struct_for_early_return for early return (assigns partial value)
                // Note: Use _str_value to avoid conflicts with fields named "value"
                let partial_struct_for_early_return =
                    build_partial_struct_for_early_return(quote! { Some(_str_value) });
                let field_name_lit = sanitized.as_str();

                quote! {
                    let #field_ident: Option<#field_type> = if (#mask_var & (1u32 << #bit_idx)) != 0 {
                        let (_str_value, _str_complete, _str_bytes_decoded) = crate::tokenizer::decode_string_streaming(tokens);
                        if !_str_complete {
                            // String was truncated - return partial result
                            return PrefixDecodeResult {
                                instance: #partial_struct_for_early_return,
                                fields_decoded: #fields_decoded + 1,
                                complete: false,
                                partial_field: Some(PartialFieldInfo {
                                    field_name: #field_name_lit,
                                    field_type: "String",
                                    items_decoded: _str_bytes_decoded,
                                    expected_length: None,
                                }),
                            };
                        }
                        Some(_str_value)
                    } else {
                        None
                    };
                }
            } else if is_optional_vec {
                // For optional Vec fields, use streaming decode
                // Use build_partial_struct_for_early_return for early return (assigns partial vec)
                // Note: Use _vec_items to avoid conflicts with fields named "items"
                let partial_struct_for_early_return =
                    build_partial_struct_for_early_return(quote! { Some(_vec_items) });
                let field_name_lit = sanitized.as_str();

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

                quote! {
                    let #field_ident: Option<Vec<#inner_type>> = if (#mask_var & (1u32 << #bit_idx)) != 0 {
                        let (_vec_items, _vec_complete, _vec_expected_length) = #vec_decode_call;
                        if !_vec_complete {
                            // Vec was truncated - return partial result
                            let _items_decoded = _vec_items.len();
                            return PrefixDecodeResult {
                                instance: #partial_struct_for_early_return,
                                fields_decoded: #fields_decoded + 1,
                                complete: false,
                                partial_field: Some(PartialFieldInfo {
                                    field_name: #field_name_lit,
                                    field_type: "Vec",
                                    items_decoded: _items_decoded,
                                    expected_length: _vec_expected_length,
                                }),
                            };
                        }
                        Some(_vec_items)
                    } else {
                        None
                    };
                }
            } else if is_optional_complex_vec {
                // For optional Vec<Struct> fields, decode each element with decode_prefix
                // Use build_partial_struct_for_early_return for early return (assigns partial vec)
                let partial_struct_for_early_return =
                    build_partial_struct_for_early_return(quote! { Some(_items) });
                let field_name_lit = sanitized.as_str();

                // Get the inner type for Vec<InnerType> and check if boxing is needed
                let (inner_type, needs_boxing) = match &member.member_type {
                    MemberType::ComplexContainer { inner, .. } => {
                        let inner_ident = format_ident!("{}", inner);
                        (quote! { #inner_ident }, self.analysis.needs_boxing(inner))
                    }
                    _ => (quote! { () }, false),
                };

                // Generate push statement with or without Box wrapping
                let push_stmt = if needs_boxing {
                    quote! { _items.push(Box::new(_element_result.instance)); }
                } else {
                    quote! { _items.push(_element_result.instance); }
                };

                // Generate the element type (with or without Box)
                let element_type = if needs_boxing {
                    quote! { Box<#inner_type> }
                } else {
                    quote! { #inner_type }
                };

                quote! {
                    let #field_ident: Option<Vec<#element_type>> = if (#mask_var & (1u32 << #bit_idx)) != 0 {
                        // Decode Vec length using multi-token base256 encoding
                        let _vec_length: usize = match crate::tokenizer::variable_base::decode_u32(tokens, "Vec<T>", #encoding_base) {
                            Ok(len) => len as usize,
                            Err(_) => {
                                return PrefixDecodeResult {
                                    instance: #partial_struct,
                                    fields_decoded: #fields_decoded,
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
                        let mut _items = Vec::with_capacity(_vec_length);
                        for _i in 0.._vec_length {
                            let _element_result = <#inner_type>::decode_prefix(tokens);
                            #push_stmt
                            if !_element_result.complete {
                                // Element was truncated - return partial Vec
                                let _items_decoded = _items.len();
                                return PrefixDecodeResult {
                                    instance: #partial_struct_for_early_return,
                                    fields_decoded: #fields_decoded + 1,
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
                    } else {
                        None
                    };
                }
            } else if is_optional_type_reference {
                // For optional TypeReference fields, use recursive decode_prefix
                let field_name_lit = sanitized.as_str();
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

                // Build the wrap expression for early return and final assignment
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

                // Use build_partial_struct_for_early_return for early return (assigns partial nested struct)
                let partial_struct_for_early_return =
                    build_partial_struct_for_early_return(wrap_for_early_return.clone());

                quote! {
                    let #field_ident: Option<#field_type> = if (#mask_var & (1u32 << #bit_idx)) != 0 {
                        let _nested_result = <#inner_type>::decode_prefix(tokens);
                        if !_nested_result.complete {
                            // Nested type was truncated - return partial result
                            return PrefixDecodeResult {
                                instance: #partial_struct_for_early_return,
                                fields_decoded: #fields_decoded + 1,
                                complete: false,
                                partial_field: Some(PartialFieldInfo {
                                    field_name: #field_name_lit,
                                    field_type: #type_name_str,
                                    items_decoded: _nested_result.fields_decoded,
                                    expected_length: None,
                                }),
                            };
                        }
                        #wrap_for_final
                    } else {
                        None
                    };
                }
            } else {
                // Standard decode for other types
                quote! {
                    let #field_ident: Option<#field_type> = if (#mask_var & (1u32 << #bit_idx)) != 0 {
                        match crate::tokenizer::JsonTokenEncoder::decode(tokens) {
                            Ok(v) => Some(v),
                            Err(_) => {
                                return PrefixDecodeResult {
                                    instance: #partial_struct,
                                    fields_decoded: #fields_decoded,
                                    complete: false,
                                    partial_field: None,
                                };
                            }
                        }
                    } else {
                        None
                    };
                }
            };

            optional_field_decodes.push(decode_code);
        }

        let total_fields = members.len();

        Ok(quote! {
            impl #type_ident {
                /// Decode from a potentially truncated token stream (streaming mode)
                ///
                /// This method handles bit-packed encoding:
                /// 1. First decodes presence masks
                /// 2. Then decodes standard fields (required + OptionalField<T>)
                /// 3. Then decodes optional fields based on presence bits
                ///
                /// # Returns
                /// - `PrefixDecodeResult` with the decoded instance and metadata
                pub fn decode_prefix(tokens: &mut impl Iterator<Item = u32>) -> PrefixDecodeResult<Self> {
                    // Phase 1: Decode presence masks
                    #( #presence_decodes )*

                    // Phase 2: Decode standard fields
                    #( #standard_field_decodes )*

                    // Phase 3: Decode optional fields based on presence bits
                    #( #optional_field_decodes )*

                    // All fields decoded successfully
                    PrefixDecodeResult {
                        instance: Self {
                            #( #all_field_idents ),*
                        },
                        fields_decoded: #total_fields,
                        complete: true,
                        partial_field: None,
                    }
                }
            }
        })
    }
}
