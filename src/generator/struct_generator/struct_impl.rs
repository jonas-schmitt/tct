//! Struct implementation renderer for generated tokenizers.
//!
//! This module emits each generated struct's `JsonTokenEncoder`
//! implementation, covering field encode/decode order, optional-field
//! presence encoding, token-count accounting, bit-packed struct delegation,
//! and optional post-decode validation. It also renders custom serde
//! implementations for structs with pattern validators so regular fields,
//! `OptionalField` state, pattern-key maps, `allOf` checks, and additional
//! properties remain aligned with the generated tokenization contract.

use super::allof_validation::AllOfPatternValidationTokens;
use super::{StructGenerator, StructTokenEncoderGenerationInput};
use crate::TctResult;
use crate::generator::encoding_decision::EncodingDecision;
use crate::generator::schema_discovery::{MemberInfo, MemberType, TypeDiscovery};
use proc_macro2::{Ident, Span, TokenStream};
use quote::{format_ident, quote};
use rustc_hash::FxHashMap;

impl<'a> StructGenerator<'a> {
    /// Check if a struct has pattern validators
    ///
    /// Pattern validators are special fields that validate properties based on regex patterns
    pub(super) fn has_pattern_validators(&self, type_discovery: &TypeDiscovery) -> bool {
        type_discovery.members.iter().any(|member| {
            member.name.starts_with("pattern_")
                && member.name.chars().skip(8).all(|c| c.is_ascii_digit())
        })
    }

    /// Generate JsonTokenEncoder implementation for a struct
    ///
    /// This generates encode(), decode(), and token_count() methods
    pub(super) fn generate_struct_token_encoder_impl(
        &self,
        input: StructTokenEncoderGenerationInput<'_>,
        member_type_fn: &dyn Fn(&MemberType) -> TctResult<TokenStream>,
    ) -> TctResult<TokenStream> {
        let StructTokenEncoderGenerationInput {
            type_ident,
            type_name,
            members,
            field_name_map,
            encoding_decision,
            post_decode_validate,
        } = input;

        // Check for bit-packed struct optimization (O3/Os encoding decision)
        if let Some(EncodingDecision::BitPackedStruct {
            optional_field_count,
            presence_tokens_needed,
        }) = encoding_decision
        {
            crate::log_debug!(
                "→ Generating bit-packed encoder for {} ({} optional fields → {} presence-mask groups)",
                type_name,
                optional_field_count,
                presence_tokens_needed
            );
            return self.generate_bitpacked_encode(
                type_ident,
                type_name,
                members,
                field_name_map,
                *optional_field_count,
                *presence_tokens_needed,
                post_decode_validate,
                member_type_fn,
            );
        }

        // Standard struct encoding (individual presence flags for optional fields)
        // Generate field encoding statements
        let encode_fields = members
            .iter()
            .map(|member| {
                let sanitized_name = field_name_map
                    .get(&member.name)
                    .unwrap_or(&member.name)
                    .clone();
                let field_name = format_ident!("{}", sanitized_name);

                if member.required {
                    // Required field: encode directly (no markers)
                    // Token format is identical for streaming and non-streaming modes
                    quote! {
                        self.#field_name.encode(tokens);
                    }
                } else {
                    // Check if we're using OptionalField (either explicit nullable or Value type)
                    let using_optional_field = member.uses_optional_field_encoding();

                    if using_optional_field {
                        // OptionalField<T>:
                        // OptionalField handles its own encoding (Absent/Null/Present states)
                        quote! {
                            self.#field_name.encode(tokens);
                        }
                    } else {
                        // Regular optional field (Option<T>):
                        // Encode presence flag, then value if present
                        quote! {
                            if let Some(ref value) = self.#field_name {
                                tokens.push(1u32); // Field is present
                                value.encode(tokens);
                            } else {
                                tokens.push(0u32); // Field is None
                            }
                        }
                    }
                }
            })
            .collect::<Vec<_>>();

        self.validate_decode_field_order_matches_encode(type_name, members)?;

        let decode_fields = members
            .iter()
            .map(|member| {
                let sanitized_name = field_name_map
                    .get(&member.name)
                    .unwrap_or(&member.name)
                    .clone();
                let field_name = format_ident!("{}", sanitized_name);

                if member.required {
                    // Required field: decode directly (no markers)
                    // Token format is identical for streaming and non-streaming modes
                    let field_name_str = &member.name;
                    let decode_value = Self::decode_member_value_with_context(
                        type_name,
                        field_name_str,
                        quote! { crate::tokenizer::JsonTokenEncoder::decode(tokens) },
                    );
                    quote! {
                        let #field_name = #decode_value;
                    }
                } else {
                    // Check if we're using OptionalField (either explicit nullable or Value type)
                    let using_optional_field = member.uses_optional_field_encoding();
                    let field_name_str = &member.name;
                    let decode_value = Self::decode_member_value_with_context(
                        type_name,
                        field_name_str,
                        quote! { crate::tokenizer::JsonTokenEncoder::decode(tokens) },
                    );

                    if using_optional_field {
                        // OptionalField<T>:
                        // OptionalField handles its own decoding (Absent/Null/Present states)
                        quote! {
                            let #field_name = #decode_value;
                        }
                    } else {
                        // Regular optional field (Option<T>):
                        // Check presence flag first
                        quote! {
                            let #field_name = match tokens.next() {
                                Some(presence_token) => {
                                    if presence_token % 2 == 1 {
                                        Some(#decode_value)
                                    } else {
                                        None
                                    }
                                }
                                None => {
                                    return Err(crate::tokenizer::DecodeError::UnexpectedEndOfStream {
                                        context: crate::tokenizer::EOFContext::new(
                                            #type_name,
                                            Some(#field_name_str),
                                            "presence flag"
                                        )
                                    });
                                }
                            };
                        }
                    }
                }
            })
            .collect::<Vec<_>>();
        let strict_decode_fields = members
            .iter()
            .map(|member| {
                let sanitized_name = field_name_map
                    .get(&member.name)
                    .unwrap_or(&member.name)
                    .clone();
                let field_name = format_ident!("{}", sanitized_name);

                if member.required {
                    let field_name_str = &member.name;
                    let decode_value = Self::decode_member_value_with_context(
                        type_name,
                        field_name_str,
                        quote! { crate::tokenizer::JsonTokenEncoder::decode_strict(tokens) },
                    );
                    quote! {
                        let #field_name = #decode_value;
                    }
                } else {
                    let using_optional_field = member.uses_optional_field_encoding();
                    let field_name_str = &member.name;
                    let decode_value = Self::decode_member_value_with_context(
                        type_name,
                        field_name_str,
                        quote! { crate::tokenizer::JsonTokenEncoder::decode_strict(tokens) },
                    );

                    if using_optional_field {
                        quote! {
                            let #field_name = #decode_value;
                        }
                    } else {
                        quote! {
                            let #field_name = match tokens.next() {
                                Some(1) => Some(#decode_value),
                                Some(0) => None,
                                Some(token) => {
                                    return Err(crate::tokenizer::DecodeError::InvalidTokenInContext {
                                        token,
                                        context: crate::tokenizer::EOFContext::new(
                                            #type_name,
                                            Some(#field_name_str),
                                            "presence flag 0 or 1",
                                        ),
                                    });
                                }
                                None => {
                                    return Err(crate::tokenizer::DecodeError::UnexpectedEndOfStream {
                                        context: crate::tokenizer::EOFContext::new(
                                            #type_name,
                                            Some(#field_name_str),
                                            "presence flag"
                                        )
                                    });
                                }
                            };
                        }
                    }
                }
            })
            .collect::<Vec<_>>();

        // Generate field assignments for struct construction
        let field_assignments = members
            .iter()
            .map(|member| {
                let sanitized_name = field_name_map
                    .get(&member.name)
                    .unwrap_or(&member.name)
                    .clone();
                let field_name = format_ident!("{}", sanitized_name);
                quote! { #field_name }
            })
            .collect::<Vec<_>>();
        let post_decode_validation = Self::post_decode_validation(type_name, post_decode_validate);

        // Generate token count calculation for each field
        let token_count_fields = members
            .iter()
            .map(|member| {
                let sanitized_name = field_name_map
                    .get(&member.name)
                    .unwrap_or(&member.name)
                    .clone();
                let field_name = format_ident!("{}", sanitized_name);

                if member.required {
                    // Required field: just the field's token count
                    quote! {
                        + self.#field_name.token_count()
                    }
                } else {
                    // Check if we're using OptionalField (either explicit nullable or Value type)
                    let using_optional_field = member.uses_optional_field_encoding();

                    if using_optional_field {
                        // OptionalField<T>:
                        // OptionalField handles its own token counting (1 for state + value if Present)
                        quote! {
                            + self.#field_name.token_count()
                        }
                    } else {
                        // Regular optional field (Option<T>):
                        // 1 (presence flag) + field's token count if present
                        quote! {
                            + 1 + self.#field_name.as_ref().map_or(0, |v| v.token_count())
                        }
                    }
                }
            })
            .collect::<Vec<_>>();

        Ok(quote! {
            impl crate::tokenizer::JsonTokenEncoder for #type_ident {
                fn encode(&self, tokens: &mut Vec<u32>) {
                    #( #encode_fields )*
                }

                fn decode(tokens: &mut impl Iterator<Item = u32>) -> Result<Self, crate::tokenizer::DecodeError> {
                    #( #decode_fields )*
                    let decoded = Self {
                        #( #field_assignments ),*
                    };
                    #post_decode_validation
                    Ok(decoded)
                }

                fn decode_strict(tokens: &mut impl Iterator<Item = u32>) -> Result<Self, crate::tokenizer::DecodeError> {
                    #( #strict_decode_fields )*
                    let decoded = Self {
                        #( #field_assignments ),*
                    };
                    #post_decode_validation
                    Ok(decoded)
                }

                fn token_count(&self) -> usize {
                    0 #( #token_count_fields )*
                }
            }
        })
    }

    pub(super) fn decode_member_value_with_context(
        type_name: &str,
        field_name: &str,
        decode_expr: TokenStream,
    ) -> TokenStream {
        // Wrap field value decode so terminal-key introspection can report the
        // active schema object-property path at host-value positions. The path
        // push is active only during `terminal_key_export` replay; ordinary
        // decode/role paths see zero overhead beyond a TLS flag check.
        quote! {
            crate::tokenizer::terminal_key_export::with_object_property(#field_name, || {
                crate::tokenizer::with_schema_field_decode_context(
                    #decode_expr,
                    #type_name,
                    #field_name,
                )
            })?
        }
    }

    fn validate_decode_field_order_matches_encode(
        &self,
        type_name: &str,
        members: &[MemberInfo],
    ) -> TctResult<()> {
        let Some(field_order) = self
            .analysis
            .decode_strategies
            .get(type_name)
            .map(|strategy| &strategy.field_order)
            .filter(|order| !order.is_empty())
        else {
            return Ok(());
        };

        let encoded_order = members
            .iter()
            .map(|member| member.name.as_str())
            .collect::<Vec<_>>();
        let decode_order = field_order.iter().map(String::as_str).collect::<Vec<_>>();

        if decode_order == encoded_order {
            return Ok(());
        }

        Err(crate::error::TctError::CodeGeneration {
            message: format!(
                "decode field_order for {type_name} diverges from generated encode order: encode={encoded_order:?}, decode={decode_order:?}"
            ),
            schema_path: format!("#/types/{type_name}"),
            suggestion: "Keep DecodeStrategy.field_order equal to the emitted member order unless encode order, decode order, and schema-evolution compatibility are updated together."
                .to_string(),
            phase: crate::error::GenerationPhase::TypeGeneration,
        })
    }

    pub(super) fn post_decode_validation(type_name: &str, enabled: bool) -> TokenStream {
        if !enabled {
            return quote! {};
        }

        quote! {
            let tct_decoded_value =
                serde_json::to_value(&decoded).map_err(|error| {
                    crate::tokenizer::DecodeError::InvalidData(format!(
                        "post-decode validation serialization failed for {}: {}",
                        #type_name,
                        error
                    ))
                })?;
            serde_json::from_value::<Self>(tct_decoded_value).map_err(|error| {
                crate::tokenizer::DecodeError::InvalidData(format!(
                    "post-decode validation failed for {}: {}",
                    #type_name,
                    error
                ))
            })?;
        }
    }

    /// Generate custom serde implementation for structs with pattern validators
    ///
    /// Pattern validators require custom field routing that can't be expressed with serde attributes
    pub(super) fn generate_custom_serde_for_pattern_struct(
        &self,
        type_ident: &Ident,
        type_discovery: &TypeDiscovery,
        field_name_map: &FxHashMap<String, String>,
        member_type_fn: &dyn Fn(&MemberType) -> TctResult<TokenStream>,
    ) -> TctResult<TokenStream> {
        let mut serialize_fields = Vec::new();
        let mut pattern_validator_fields = Vec::new();
        let mut pattern_validator_types = Vec::new();
        let mut regular_field_idents = Vec::new();
        let mut regular_field_names = Vec::new();
        let mut regular_field_required = Vec::new();
        let mut regular_field_types = Vec::new(); // NEW: Track field types for correct deserialization
        let mut regular_field_uses_optional_field = Vec::new(); // NEW: Track if field uses OptionalField vs Option
        let mut additional_properties_field = None;

        // Separate pattern validators from regular fields
        for member in &type_discovery.members {
            let field_name = field_name_map.get(&member.name).unwrap_or(&member.name);
            let field_ident = format_ident!("{}", field_name);

            if member.name.starts_with("pattern_")
                && member.name.chars().skip(8).all(|c| c.is_ascii_digit())
            {
                pattern_validator_fields.push(field_ident);
                // Get the actual type name from the member_type
                if let crate::generator::schema_discovery::MemberType::TypeReference(type_name) =
                    &member.member_type
                {
                    let type_ident = format_ident!("{}", type_name);
                    pattern_validator_types.push(type_ident);
                }
            } else if member.name == "additional_properties" {
                additional_properties_field = Some(field_ident);
            } else {
                let original_name = &member.name;
                regular_field_idents.push(field_ident.clone());
                regular_field_names.push(original_name.clone());
                regular_field_required.push(member.required);

                // NEW: Determine the deserialization type (Option vs OptionalField)
                // This MUST match the logic in generate() for field type generation
                let (deser_type, default_value, uses_optional_field) = if !member.required {
                    // Get the base field type
                    let field_type = member_type_fn(&member.member_type)?;

                    if member.uses_optional_field_encoding() {
                        // Nullable and Value types use OptionalField to preserve null vs absent
                        (
                            quote! { crate::tokenizer::OptionalField<#field_type> },
                            quote! { crate::tokenizer::OptionalField::Absent },
                            true,
                        )
                    } else {
                        // Regular optional field uses Option
                        (quote! { Option<#field_type> }, quote! { None }, false)
                    }
                } else {
                    // Required fields don't get wrapped in Option/OptionalField during deserialization
                    // They're stored as Option temporarily and unwrapped later
                    let field_type = member_type_fn(&member.member_type)?;
                    (quote! { Option<#field_type> }, quote! { None }, false)
                };
                regular_field_types.push((deser_type, default_value));
                regular_field_uses_optional_field.push(uses_optional_field);

                // Add regular field serialization with correct pattern matching
                // OptionalField uses Present/Absent, Option uses Some/None
                if member.required {
                    serialize_fields.push(quote! {
                        map.serialize_entry(#original_name, &self.#field_ident)?;
                    });
                } else if uses_optional_field {
                    // OptionalField: serialize Present and Null, skip only Absent.
                    serialize_fields.push(quote! {
                        match &self.#field_ident {
                            crate::tokenizer::OptionalField::Present(value) => {
                                map.serialize_entry(#original_name, value)?;
                            }
                            crate::tokenizer::OptionalField::Null => {
                                map.serialize_entry(#original_name, &serde_json::Value::Null)?;
                            }
                            crate::tokenizer::OptionalField::Absent => {}
                        }
                    });
                } else {
                    // Option: serialize only if Some, skip None
                    serialize_fields.push(quote! {
                        if let Some(ref value) = self.#field_ident {
                            map.serialize_entry(#original_name, value)?;
                        }
                    });
                }
            }
        }

        // Extract default values from the types vector for use in deserialization
        let regular_field_default_values: Vec<_> = regular_field_types
            .iter()
            .map(|(_, default_value)| default_value.clone())
            .collect();

        // Generate assignment patterns for deserialization (Option vs OptionalField)
        let regular_field_assignments: Vec<_> = regular_field_idents.iter()
            .zip(regular_field_names.iter())
            .zip(regular_field_required.iter())
            .zip(regular_field_uses_optional_field.iter())
            .map(|(((field_ident, field_name), &is_required), &uses_opt_field)| {
                let field_name_lit = syn::LitStr::new(field_name, Span::call_site());
                if !is_required && uses_opt_field {
                    // OptionalField: Null → OptionalField::Null, Value → OptionalField::Present(value)
                    quote! {
                        if value.is_null() {
                            #field_ident = crate::tokenizer::OptionalField::Null;
                        } else {
                            #field_ident = crate::tokenizer::OptionalField::Present(serde_json::from_value(value).map_err(de::Error::custom)?);
                        }
                    }
                } else {
                    // Option or required: optional non-nullable fields reject explicit Null.
                    quote! {
                        if !#is_required && value.is_null() {
                            return Err(de::Error::custom(format!(
                                "field '{}' does not allow explicit null",
                                #field_name_lit
                            )));
                        } else {
                            #field_ident = Some(serde_json::from_value(value).map_err(de::Error::custom)?);
                        }
                    }
                }
            })
            .collect();

        // Generate Serialize implementation
        let additional_props_serialize =
            if let Some(ref additional_field) = additional_properties_field {
                quote! {
                    for (key, value) in &self.#additional_field {
                        map.serialize_entry(key, value)?;
                    }
                }
            } else {
                quote! {}
            };

        let serialize_impl = quote! {
            impl serde::Serialize for #type_ident {
                fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
                where
                    S: serde::Serializer,
                {
                    use serde::ser::SerializeMap;
                    let mut map = serializer.serialize_map(None)?;

                    // Serialize regular fields
                    #( #serialize_fields )*

                    // Serialize pattern validator fields (they handle their own key-value pairs)
                    #(
                        for (key, value) in self.#pattern_validator_fields.iter() {
                            map.serialize_entry(key, value)?;
                        }
                    )*

                    // Serialize additional properties if present
                    #additional_props_serialize

                    map.end()
                }
            }
        };

        // Generate Deserialize implementation with pattern routing
        let additional_props_init = if let Some(ref additional_field) = additional_properties_field
        {
            quote! {
                let mut #additional_field = rustc_hash::FxHashMap::default();
            }
        } else {
            quote! {}
        };

        let additional_props_handling =
            if let Some(ref additional_field) = additional_properties_field {
                quote! {
                    // Additional properties can accept any remaining fields
                    match serde_json::from_value(value.clone()) {
                        Ok(typed_value) => {
                            #additional_field.insert(key.clone(), typed_value);
                            handled = true;
                        }
                        Err(_) => {} // Continue to error if type doesn't match
                    }
                }
            } else {
                quote! {}
            };

        let additional_props_field_construction =
            if let Some(ref additional_field) = additional_properties_field {
                quote! { #additional_field, }
            } else {
                quote! {}
            };

        let allof_validation_plan = self.allof_validation_plan(type_discovery);
        let AllOfPatternValidationTokens {
            seen_init: allof_seen_init,
            seen_record: allof_seen_record,
            raw_object_init: allof_raw_object_init,
            raw_object_record: allof_raw_object_record,
            json_value_init: allof_json_value_init,
            not_required_checks: allof_not_required_checks,
            conditional_checks: allof_conditional_checks,
            json_schema_checks: allof_json_schema_checks,
        } = Self::pattern_allof_validation_tokens(&allof_validation_plan);

        // Generate field unwrapping for required fields only
        let required_field_unwrapping: Vec<_> = regular_field_idents
            .iter()
            .zip(regular_field_names.iter())
            .zip(regular_field_required.iter())
            .filter_map(|((ident, name), &is_required)| {
                if is_required {
                    Some(quote! {
                        let #ident = #ident.ok_or_else(|| de::Error::missing_field(#name))?;
                    })
                } else {
                    None
                }
            })
            .collect();

        let deserialize_impl = quote! {
            impl<'de> serde::Deserialize<'de> for #type_ident {
                fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
                where
                    D: serde::Deserializer<'de>,
                {
                    use serde::de::{self, MapAccess, Visitor};
                    use std::fmt;

                    struct FieldVisitor;

                    impl<'de> Visitor<'de> for FieldVisitor {
                        type Value = #type_ident;

                        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                            formatter.write_str("a map")
                        }

                        fn visit_map<V>(self, mut map: V) -> Result<Self::Value, V::Error>
                        where
                            V: MapAccess<'de>,
                        {
                            // Initialize pattern validators
                            #(
                                let mut #pattern_validator_fields = match #pattern_validator_types::new() {
                                    Ok(validator) => validator,
                                    Err(e) => return Err(de::Error::custom(format!("Failed to create pattern validator: {}", e))),
                                };
                            )*

                            // Initialize regular fields with correct types and default values
                            // This prevents Rust never type fallback issues in newer compiler versions
                            // (Option<T> defaults to None, OptionalField<T> defaults to Missing)
                            #(
                                let mut #regular_field_idents = #regular_field_default_values;
                            )*

                            // Initialize additional properties if present
                            #additional_props_init

                            #allof_seen_init
                            #allof_raw_object_init

                            // Process each key-value pair
                            while let Some((key, value)) = map.next_entry::<String, serde_json::Value>()? {
                                #allof_seen_record
                                #allof_raw_object_record

                                // Try regular fields first with type-appropriate assignment patterns
                                #(
                                    if key == #regular_field_names {
                                        #regular_field_assignments
                                        continue;
                                    }
                                )*

                                // Try pattern validators
                                let mut handled = false;
                                #(
                                    if #pattern_validator_fields.validate_key(&key).is_ok() {
                                        match serde_json::from_value(value.clone()) {
                                            Ok(typed_value) => {
                                                #pattern_validator_fields.insert(key.clone(), typed_value).map_err(de::Error::custom)?;
                                                handled = true;
                                            }
                                            Err(e) => return Err(de::Error::custom(format!("Type error for key '{}': {}", key, e))),
                                        }
                                    }
                                )*

                                // If not handled by pattern validators, try additional properties
                                if !handled {
                                    #additional_props_handling
                                }

                                if !handled {
                                    return Err(de::Error::custom(format!("Unknown field: {}", key)));
                                }
                            }

                            #( #allof_not_required_checks )*
                            #allof_json_value_init
                            #( #allof_json_schema_checks )*
                            #( #allof_conditional_checks )*

                            // Unwrap required fields (will error if missing)
                            // Optional fields remain as Option<T>
                            #( #required_field_unwrapping )*

                            Ok(#type_ident {
                                #( #regular_field_idents, )*
                                #( #pattern_validator_fields, )*
                                #additional_props_field_construction
                            })
                        }
                    }

                    deserializer.deserialize_map(FieldVisitor)
                }
            }
        };

        Ok(quote! {
            #serialize_impl
            #deserialize_impl
        })
    }
}
