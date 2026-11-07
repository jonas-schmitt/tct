//! Const-first streaming decode renderer for bit-packed generated structs.
//!
//! This module emits the `decode_prefix` variant that decodes bit-packed
//! presence masks, then zero-token standard fields before variable-token
//! fields, so constants and single-variant values remain visible even when a
//! later token-consuming field is truncated. It falls back to the ordinary
//! bit-packed streaming renderer when no zero-token classifier is available.

use super::StructGenerator;
use crate::TctResult;
use crate::generator::schema_discovery::{MemberInfo, MemberType};
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use rustc_hash::FxHashMap;

mod partial;

use partial::optional_field_partial_struct;

impl<'a> StructGenerator<'a> {
    /// Generate bit-packed streaming decode_prefix with const-first ordering
    ///
    /// This variant reorders field decoding to decode zero-token (const) fields BEFORE
    /// variable-token fields. This makes const fields visible earlier in streaming mode,
    /// even when variable-length fields are truncated.
    ///
    /// Decode order:
    /// 1. Presence masks (with fallback to 0)
    /// 2. Zero-token standard fields (const enums, SingleVariantEnum types)
    /// 3. Variable-token standard fields (strings, objects, etc.)
    /// 4. Optional fields based on presence bits
    #[allow(clippy::too_many_arguments)]
    pub(in crate::generator::struct_generator::streaming_decode) fn generate_bitpacked_streaming_decode_prefix_with_zero_token_info(
        &self,
        type_ident: &Ident,
        type_name: &str,
        members: &[MemberInfo],
        field_name_map: &FxHashMap<String, String>,
        presence_tokens_needed: usize,
        member_type_fn: &dyn Fn(&MemberType) -> TctResult<TokenStream>,
        is_zero_token_type_fn: Option<&dyn Fn(&str) -> bool>,
    ) -> TctResult<TokenStream> {
        // If no zero-token info provided, fall back to standard decode
        let Some(is_zero_token) = is_zero_token_type_fn else {
            return self.generate_bitpacked_streaming_decode_prefix(
                type_ident,
                type_name,
                members,
                field_name_map,
                presence_tokens_needed,
                member_type_fn,
            );
        };

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

        // Partition standard fields into zero-token and variable-token
        let mut zero_token_fields: Vec<(usize, &MemberInfo)> = Vec::new();
        let mut variable_token_fields: Vec<(usize, &MemberInfo)> = Vec::new();

        for (idx, member) in standard_fields.iter().enumerate() {
            // Get the type name for this member
            let type_name_str = match &member.member_type {
                MemberType::TypeReference(name) => name.clone(),
                MemberType::Primitive(name) => name.clone(),
                MemberType::ComplexContainer { inner, .. } => inner.clone(),
                MemberType::Container { inner, .. } => inner.clone(),
                MemberType::HashMap { value, .. } => value.clone(),
                MemberType::PatternProperties { .. } => String::new(),
            };

            if !type_name_str.is_empty() && is_zero_token(&type_name_str) {
                zero_token_fields.push((idx, member));
            } else {
                variable_token_fields.push((idx, member));
            }
        }

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
        let mut presence_decodes = Vec::new();
        for token_idx in 0..presence_tokens_needed {
            let mask_var = format_ident!("presence_mask_{}", token_idx);
            presence_decodes.push(quote! {
                let #mask_var = #codec_module_name_ident::decode(tokens).unwrap_or(0u32);
            });
        }

        // Helper to generate a partial struct for early returns
        let generate_partial_struct = |decoded_fields: &[Ident]| -> TokenStream {
            // `all_field_idents` is built from `members` in order, so zipping
            // recovers the MemberInfo needed to pick a compilable default (a
            // required serde_json::Number has no Default impl).
            let field_values: Vec<_> = members
                .iter()
                .zip(all_field_idents.iter())
                .map(|(member, ident)| {
                    if decoded_fields.contains(ident) {
                        quote! { #ident }
                    } else {
                        crate::generator::struct_generator::default_impl::default_value_expr(member)
                    }
                })
                .collect();

            quote! {
                Self {
                    #( #all_field_idents: #field_values ),*
                }
            }
        };

        // Phase 2: Decode ZERO-TOKEN standard fields FIRST
        // These require 0 tokens and are always decodable
        let mut zero_token_decodes = Vec::new();
        let mut decoded_zero_token_fields = Vec::new();

        for (idx, member) in &zero_token_fields {
            let sanitized = field_name_map.get(&member.name).unwrap_or(&member.name);
            let field_ident = format_ident!("{}", sanitized);
            decoded_zero_token_fields.push(field_ident.clone());

            let field_type = member_type_fn(&member.member_type)?;
            let outer_type = if !member.required {
                if member.uses_optional_field_encoding() {
                    quote! { crate::tokenizer::OptionalField<#field_type> }
                } else {
                    quote! { Option<#field_type> }
                }
            } else {
                field_type.clone()
            };

            // Zero-token fields should never fail, but handle gracefully
            let decode_count = *idx;
            let partial_struct = generate_partial_struct(
                &decoded_zero_token_fields[..decoded_zero_token_fields.len() - 1],
            );

            zero_token_decodes.push(quote! {
                let #field_ident: #outer_type = match <#outer_type as crate::tokenizer::JsonTokenEncoder>::decode(tokens) {
                    Ok(v) => v,
                    Err(_) => {
                        return PrefixDecodeResult {
                            instance: #partial_struct,
                            fields_decoded: #decode_count,
                            complete: false,
                            partial_field: None,
                        };
                    }
                };
            });
        }

        // Phase 3: Decode VARIABLE-TOKEN standard fields
        let mut variable_token_decodes = Vec::new();
        let mut decoded_variable_token_fields = Vec::new();

        for (idx, member) in &variable_token_fields {
            let sanitized = field_name_map.get(&member.name).unwrap_or(&member.name);
            let field_ident = format_ident!("{}", sanitized);
            decoded_variable_token_fields.push(field_ident.clone());

            let field_type = member_type_fn(&member.member_type)?;
            let outer_type = if !member.required {
                if member.uses_optional_field_encoding() {
                    quote! { crate::tokenizer::OptionalField<#field_type> }
                } else {
                    quote! { Option<#field_type> }
                }
            } else {
                field_type.clone()
            };

            // Total fields decoded so far (zero-token + variable-token)
            let zero_token_count = zero_token_fields.len();
            let decode_count = zero_token_count + *idx;

            // Build partial struct with all zero-token fields + decoded variable-token fields
            let mut all_decoded = decoded_zero_token_fields.clone();
            all_decoded.extend(
                decoded_variable_token_fields[..decoded_variable_token_fields.len() - 1]
                    .iter()
                    .cloned(),
            );
            let partial_struct = generate_partial_struct(&all_decoded);

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

            let decode_code = if is_required_string {
                // For required String fields, use streaming decode to get partial content
                // Build partial struct including this field with partial string value
                let mut all_decoded_with_current = decoded_zero_token_fields.clone();
                all_decoded_with_current.extend(decoded_variable_token_fields.iter().cloned());
                let partial_struct_with_field = generate_partial_struct(&all_decoded_with_current);

                let field_name_lit = sanitized.as_str();
                quote! {
                    let (#field_ident, _string_complete, _bytes_decoded) = crate::tokenizer::decode_string_streaming(tokens);
                    if !_string_complete {
                        // String was truncated - return partial result with the partial string
                        return PrefixDecodeResult {
                            instance: #partial_struct_with_field,
                            fields_decoded: #decode_count + 1, // Count this field as decoded (partially)
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
                let mut all_decoded_with_current = decoded_zero_token_fields.clone();
                all_decoded_with_current.extend(decoded_variable_token_fields.iter().cloned());
                let partial_struct_with_field = generate_partial_struct(&all_decoded_with_current);

                // Get the inner type for Vec<T>
                let inner_type = match &member.member_type {
                    MemberType::Container { inner, .. } => Self::container_inner_type_tokens(inner),
                    _ => quote! { () }, // Shouldn't happen
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
                            fields_decoded: #decode_count + 1, // Count this field as decoded (partially)
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
                let mut all_decoded_with_current = decoded_zero_token_fields.clone();
                all_decoded_with_current.extend(decoded_variable_token_fields.iter().cloned());
                let partial_struct_with_field = generate_partial_struct(&all_decoded_with_current);

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
                    // Decode Vec length using multi-token base256 encoding
                    let _vec_length: usize = match crate::tokenizer::variable_base::decode_u32(tokens, "Vec<T>", #encoding_base) {
                        Ok(len) => len as usize,
                        Err(_) => {
                            return PrefixDecodeResult {
                                instance: #partial_struct,
                                fields_decoded: #decode_count,
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
                                fields_decoded: #decode_count + 1,
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
                let mut all_decoded_with_current = decoded_zero_token_fields.clone();
                all_decoded_with_current.extend(decoded_variable_token_fields.iter().cloned());
                let partial_struct_with_field = generate_partial_struct(&all_decoded_with_current);

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
                            fields_decoded: #decode_count + 1, // Count this field as decoded (partially)
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
                    let #field_ident: #outer_type = <#outer_type as crate::tokenizer::JsonTokenEncoder>::decode(tokens)
                        .unwrap_or_default();
                }
            } else {
                // Standard decode for other types
                quote! {
                    let #field_ident: #outer_type = match <#outer_type as crate::tokenizer::JsonTokenEncoder>::decode(tokens) {
                        Ok(v) => v,
                        Err(_) => {
                            return PrefixDecodeResult {
                                instance: #partial_struct,
                                fields_decoded: #decode_count,
                                complete: false,
                                partial_field: None,
                            };
                        }
                    };
                }
            };

            variable_token_decodes.push(decode_code);
        }

        // Phase 4: Decode optional fields based on presence bits
        let mut optional_field_decodes = Vec::new();
        for (field_idx, member) in optional_fields.iter().enumerate() {
            let sanitized = field_name_map.get(&member.name).unwrap_or(&member.name);
            let field_ident = format_ident!("{}", sanitized);
            let token_idx = field_idx / 32;
            let bit_idx = field_idx % 32;
            let mask_var = format_ident!("presence_mask_{}", token_idx);

            let field_type = member_type_fn(&member.member_type)?;

            let total_standard = zero_token_fields.len() + variable_token_fields.len();
            let decode_count = total_standard + field_idx;

            // Build partial struct for early return
            let mut all_decoded = decoded_zero_token_fields.clone();
            all_decoded.extend(decoded_variable_token_fields.iter().cloned());
            let partial_struct = generate_partial_struct(&all_decoded);

            // Check if this is an optional String field - use streaming decode for partial results
            let is_optional_string =
                matches!(&member.member_type, MemberType::Primitive(t) if t == "String");

            // Check if this is an optional Vec<T> field (Container)
            let is_optional_vec = matches!(&member.member_type, MemberType::Container { container, .. } if container == "Vec");

            // Check if this is an optional Vec<Struct> field (ComplexContainer)
            let is_optional_complex_vec = matches!(&member.member_type, MemberType::ComplexContainer { container, .. } if container == "Vec");

            // Check if this is an optional TypeReference field (nested struct/enum)
            let is_optional_type_reference =
                matches!(&member.member_type, MemberType::TypeReference(_));

            let decode_code = if is_optional_string {
                // For optional String fields, use streaming decode
                // Build partial struct for early return (excludes current field, assigns it with partial value)
                let partial_struct_for_early_return = optional_field_partial_struct(
                    &optional_fields,
                    field_idx,
                    field_name_map,
                    &decoded_zero_token_fields,
                    &decoded_variable_token_fields,
                    &field_ident,
                    quote! { Some(_str_value) },
                );

                let field_name_lit = sanitized.as_str();
                quote! {
                    let #field_ident: Option<#field_type> = if (#mask_var & (1u32 << #bit_idx)) != 0 {
                        let (_str_value, _str_complete, _str_bytes_decoded) = crate::tokenizer::decode_string_streaming(tokens);
                        if !_str_complete {
                            // String was truncated - return partial result
                            return PrefixDecodeResult {
                                instance: #partial_struct_for_early_return,
                                fields_decoded: #decode_count + 1,
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
                // Build partial struct for early return (excludes current field, assigns it with partial value)
                let partial_struct_for_early_return = optional_field_partial_struct(
                    &optional_fields,
                    field_idx,
                    field_name_map,
                    &decoded_zero_token_fields,
                    &decoded_variable_token_fields,
                    &field_ident,
                    quote! { Some(_vec_items) },
                );

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
                    let #field_ident: Option<Vec<#inner_type>> = if (#mask_var & (1u32 << #bit_idx)) != 0 {
                        let (_vec_items, _vec_complete, _vec_expected_length) = #vec_decode_call;
                        if !_vec_complete {
                            // Vec was truncated - return partial result
                            let _items_decoded = _vec_items.len();
                            return PrefixDecodeResult {
                                instance: #partial_struct_for_early_return,
                                fields_decoded: #decode_count + 1,
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
                // Build partial struct for early return (excludes current field, assigns it with partial value)
                let partial_struct_for_early_return = optional_field_partial_struct(
                    &optional_fields,
                    field_idx,
                    field_name_map,
                    &decoded_zero_token_fields,
                    &decoded_variable_token_fields,
                    &field_ident,
                    quote! { Some(_items) },
                );

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

                let field_name_lit = sanitized.as_str();
                quote! {
                    let #field_ident: Option<Vec<#element_type>> = if (#mask_var & (1u32 << #bit_idx)) != 0 {
                        // Decode Vec length using multi-token base256 encoding
                        let _vec_length: usize = match crate::tokenizer::variable_base::decode_u32(tokens, "Vec<T>", #encoding_base) {
                            Ok(len) => len as usize,
                            Err(_) => {
                                return PrefixDecodeResult {
                                    instance: #partial_struct,
                                    fields_decoded: #decode_count,
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
                                    fields_decoded: #decode_count + 1,
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
                // Build partial struct for early return (excludes current field, assigns it with partial value)
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

                let partial_struct_for_early_return = optional_field_partial_struct(
                    &optional_fields,
                    field_idx,
                    field_name_map,
                    &decoded_zero_token_fields,
                    &decoded_variable_token_fields,
                    &field_ident,
                    wrap_for_early_return,
                );

                quote! {
                    let #field_ident: Option<#field_type> = if (#mask_var & (1u32 << #bit_idx)) != 0 {
                        let _nested_result = <#inner_type>::decode_prefix(tokens);
                        if !_nested_result.complete {
                            // Nested type was truncated - return partial result
                            return PrefixDecodeResult {
                                instance: #partial_struct_for_early_return,
                                fields_decoded: #decode_count + 1,
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
                        match <#field_type as crate::tokenizer::JsonTokenEncoder>::decode(tokens) {
                            Ok(v) => Some(v),
                            Err(_) => {
                                return PrefixDecodeResult {
                                    instance: #partial_struct,
                                    fields_decoded: #decode_count,
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
                /// This method uses const-first ordering:
                /// 1. Decodes presence masks (with fallback)
                /// 2. Decodes zero-token fields FIRST (always visible)
                /// 3. Then decodes variable-token fields
                /// 4. Finally decodes optional fields based on presence bits
                ///
                /// # Returns
                /// - `PrefixDecodeResult` with the decoded instance and metadata
                pub fn decode_prefix(tokens: &mut impl Iterator<Item = u32>) -> PrefixDecodeResult<Self> {
                    // Phase 1: Decode presence masks
                    #( #presence_decodes )*

                    // Phase 2: Decode zero-token standard fields (const enums)
                    #( #zero_token_decodes )*

                    // Phase 3: Decode variable-token standard fields
                    #( #variable_token_decodes )*

                    // Phase 4: Decode optional fields based on presence bits
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
