//! Partial-encoding renderer for generated enum roots.
//!
//! This module emits enum methods that preserve discriminator handling while
//! delegating partial JSON parsing and gap-aware payload encoding to variant
//! types. It keeps enum-specific gap offsets and discriminator accounting near
//! the enum generator instead of spreading that logic across struct renderers.

use super::EnumGenerator;
use crate::TctResult;
use crate::generator::encoding_decision::EncodingDecision;
use crate::generator::schema_discovery::{DiscriminatorInfo, MemberInfo, MemberType};
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};

impl<'a> EnumGenerator<'a> {
    /// Generate partial encoding methods for enum types with discriminator
    ///
    /// Generates two methods that enable partial instance encoding for enum roots:
    /// - from_json_partial(): Deserialize partial JSON with presence tracking
    /// - encode_partial(): Encode with presence tracking (returns PartialEncoding)
    ///
    /// These methods delegate to the variant type's partial encoding methods.
    pub(super) fn generate_enum_partial_encoding_methods(
        &self,
        type_ident: &Ident,
        members: &[MemberInfo],
        discriminator_info: &DiscriminatorInfo,
        _encoding_decision: Option<&EncodingDecision>,
        member_type_fn: &dyn Fn(&MemberType) -> TctResult<TokenStream>,
    ) -> TctResult<TokenStream> {
        // For compound discriminators, we combine field values with "|"
        let is_compound = discriminator_info.field_names.len() > 1;
        let disc_fields: Vec<_> = discriminator_info
            .field_names
            .iter()
            .map(|f| f.as_str())
            .collect();

        // Generate match arms for from_json_partial
        // Maps discriminator value to variant construction with delegated partial parsing
        let from_partial_arms: Vec<_> = discriminator_info
            .value_to_variant
            .iter()
            .filter_map(|(disc_value, variant_idx)| {
                let member = members.get(*variant_idx)?;
                let variant_name = format_ident!("Variant{}", variant_idx);

                // Get the variant type for delegation
                let variant_type = match &member.member_type {
                    MemberType::TypeReference(type_name) => {
                        let type_ident = format_ident!("{}", type_name);
                        quote! { #type_ident }
                    }
                    _ => return None, // Skip non-reference types
                };

                Some(quote! {
                    #disc_value => {
                        // Delegate to variant's from_json_partial
                        let (variant_instance, variant_presence) = #variant_type::from_json_partial(json_str)?;
                        Ok((Self::#variant_name(variant_instance), variant_presence))
                    }
                })
            })
            .collect();

        // Generate match arms for encode_partial
        // Each variant encodes: discriminator token(s) + variant's partial encoding
        let encode_partial_arms: Vec<_> = members
            .iter()
            .enumerate()
            .filter_map(|(idx, member)| {
                let variant_name = format_ident!("Variant{}", idx);
                let variant_idx = idx as u32;

                // Get the variant type for delegation
                match &member.member_type {
                    MemberType::TypeReference(type_name) => {
                        let _type_ident = format_ident!("{}", type_name);

                        Some(quote! {
                            Self::#variant_name(inner) => {
                                let mut tokens = Vec::new();

                                // 1. Encode discriminator
                                tokens.push(#variant_idx);
                                let discriminator_token_count = tokens.len();

                                // 2. Get variant's partial encoding
                                let variant_encoding = inner.encode_partial(presence);

                                // 3. Append variant tokens
                                tokens.extend(variant_encoding.tokens);

                                // 4. Adjust gap ranges by discriminator offset
                                let adjusted_gaps: Vec<GapInfo> = variant_encoding.gaps.into_iter().map(|gap| {
                                    GapInfo {
                                        field_index: gap.field_index,
                                        field_name: gap.field_name,
                                        token_range: (gap.token_range.start + discriminator_token_count)
                                            ..(gap.token_range.end + discriminator_token_count),
                                    }
                                }).collect();

                                PartialEncoding { tokens, gaps: adjusted_gaps }
                            }
                        })
                    }
                    _ => None, // Skip non-reference types
                }
            })
            .collect();

        // Generate match arms for decode_partial
        // Each variant decodes: discriminator token + variant's decode_partial
        let decode_partial_arms: Vec<_> = members
            .iter()
            .enumerate()
            .filter_map(|(idx, member)| {
                let variant_name = format_ident!("Variant{}", idx);
                let variant_idx = idx as u32;

                // Get the variant type for delegation
                match &member.member_type {
                    MemberType::TypeReference(type_name) => {
                        let type_ident = format_ident!("{}", type_name);

                        Some(quote! {
                            #variant_idx => {
                                let variant = #type_ident::decode_partial(remaining)?;
                                Ok(Self::#variant_name(variant))
                            }
                        })
                    }
                    _ => None, // Skip non-reference types
                }
            })
            .collect();

        let _variant_count = members.len() as u32;

        // Build variant type for member_type_fn call (for documentation)
        let _variant_types: Vec<_> = members
            .iter()
            .filter_map(|m| member_type_fn(&m.member_type).ok())
            .collect();

        // Generate discriminator value extraction code
        let disc_value_extraction = if is_compound {
            // Compound: extract multiple fields and join with "|"
            let field_extractions: Vec<_> = disc_fields.iter().map(|field| {
                quote! {
                    obj.get(#field)
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| format!("Missing or invalid discriminator field '{}'", #field))?
                }
            }).collect();

            quote! {
                let parts: Vec<&str> = vec![#(#field_extractions),*];
                let disc_value = parts.join("|");
            }
        } else {
            // Single field: extract directly
            let field = disc_fields.first().unwrap_or(&"");
            quote! {
                let disc_value = obj.get(#field)
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| format!("Missing or invalid discriminator field '{}'", #field))?
                    .to_string();
            }
        };

        // Error message for unknown discriminator
        let disc_field_names = disc_fields.join("+");

        Ok(quote! {
            impl #type_ident {
                /// Deserialize partial JSON with presence tracking (enum with discriminator)
                ///
                /// Identifies the variant using the discriminator field(s), then delegates
                /// to that variant's from_json_partial for presence tracking.
                ///
                /// # Parameters
                /// - `json_str`: JSON string (may be incomplete/partial)
                ///
                /// # Returns
                /// Tuple of (deserialized instance, presence array from variant)
                pub fn from_json_partial(json_str: &str) -> Result<(Self, Vec<bool>), Box<dyn std::error::Error>> {
                    // Parse JSON to extract discriminator field(s)
                    let json_value: serde_json::Value = serde_json::from_str(json_str)?;
                    let obj = json_value.as_object()
                        .ok_or_else(|| format!("Expected JSON object for enum with discriminator"))?;

                    // Get discriminator value(s)
                    #disc_value_extraction

                    // Match discriminator to variant and delegate
                    match disc_value.as_str() {
                        #(#from_partial_arms)*
                        other => Err(format!("Unknown discriminator value '{}' for field(s) '{}'", other, #disc_field_names).into())
                    }
                }

                /// Encode a partial enum instance with presence tracking
                ///
                /// Encodes the discriminator first, then delegates to the variant's encode_partial.
                /// Gap ranges are adjusted to account for discriminator token(s).
                ///
                /// # Parameters
                /// - `presence`: Boolean array indicating which fields were present (from variant)
                ///
                /// # Returns
                /// Complete token sequence with gap metadata (ranges offset by discriminator)
                pub fn encode_partial(&self, presence: &[bool]) -> PartialEncoding {
                    match self {
                        #(#encode_partial_arms)*
                    }
                }

                /// Decode partial encoding (field-by-field format)
                ///
                /// Decodes a token sequence produced by encode_partial().
                /// First reads the variant discriminator, then delegates to variant's decode_partial.
                ///
                /// # Parameters
                /// - `tokens`: Token slice from encode_partial()
                ///
                /// # Returns
                /// Decoded enum instance
                pub fn decode_partial(tokens: &[u32]) -> Result<Self, crate::tokenizer::DecodeError> {
                    if tokens.is_empty() {
                        return Err(crate::tokenizer::DecodeError::UnexpectedEndOfStream {
                            context: crate::tokenizer::EOFContext::new(stringify!(#type_ident), None, "variant discriminator"),
                        });
                    }

                    let discriminator = tokens[0];
                    let remaining = &tokens[1..];

                    match discriminator {
                        #(#decode_partial_arms)*
                        other => Err(crate::tokenizer::DecodeError::InvalidToken(other))
                    }
                }
            }
        })
    }
}
