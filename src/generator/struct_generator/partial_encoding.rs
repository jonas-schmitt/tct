//! Partial-instance encoding helpers for generated struct types.
//!
//! This module renders inherent methods that deserialize partial JSON with
//! field-presence tracking, encode default-filled instances while recording
//! gap token ranges, and decode those encodings back to the first generated
//! default field for continuation-oriented workflows.

use super::StructGenerator;
use crate::generator::schema_discovery::MemberInfo;
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use rustc_hash::FxHashMap;

impl<'a> StructGenerator<'a> {
    /// Generate partial encoding methods for structs
    ///
    /// Generates three methods that enable partial instance encoding:
    /// - from_json_partial(): Deserialize partial JSON with presence tracking
    /// - encode_partial(): Encode with presence tracking (returns PartialEncoding)
    /// - decode_to_first_gap(): Decode to first default field (Mode 2)
    ///
    /// These methods enable decoder-only LM continuation.
    pub(super) fn generate_partial_encoding_methods(
        &self,
        type_ident: &Ident,
        members: &[MemberInfo],
        field_name_map: &FxHashMap<String, String>,
    ) -> TokenStream {
        let field_count = members.len();

        // Generate field name literals for presence tracking
        let field_names: Vec<_> = members.iter().map(|m| &m.name).collect();

        // Generate default values for each field type
        let default_values: Vec<_> = members
            .iter()
            .map(|member| self.generate_default_value_for_field(member))
            .collect();

        // Generate field-by-field encoding with gap tracking for encode_partial()
        let encode_partial_fields: Vec<_> = members
            .iter()
            .enumerate()
            .map(|(idx, member)| {
                let sanitized_name = field_name_map.get(&member.name).unwrap_or(&member.name);
                let field_ident = format_ident!("{}", sanitized_name);
                let field_name = &member.name;

                // Generate encoding logic for this field (similar to regular encode, but with tracking)
                let encode_logic = if member.required {
                    // Required field: encode directly
                    quote! {
                        let start = tokens.len();
                        self.#field_ident.encode(&mut tokens);
                        let end = tokens.len();
                    }
                } else {
                    // Optional field: check type and encode accordingly
                    // This matches the logic in generate_struct_token_encoder_impl
                    quote! {
                        let start = tokens.len();
                        // Note: We use the JsonTokenEncoder trait which handles Option vs OptionalField
                        self.#field_ident.encode(&mut tokens);
                        let end = tokens.len();
                    }
                };

                quote! {
                    // Field #idx: #field_name
                    #encode_logic
                    if !presence[#idx] {
                        gaps.push(GapInfo {
                            field_index: #idx,
                            field_name: #field_name,
                            token_range: start..end,
                        });
                    }
                }
            })
            .collect();

        // Generate field-by-field decoding for decode_partial()
        let decode_partial_fields: Vec<_> = members
            .iter()
            .map(|member| {
                let sanitized_name = field_name_map.get(&member.name).unwrap_or(&member.name);
                let field_ident = format_ident!("{}", sanitized_name);

                quote! {
                    let #field_ident = crate::tokenizer::JsonTokenEncoder::decode(&mut iter)?;
                }
            })
            .collect();

        // Generate field assignments for struct construction
        let field_assignments: Vec<_> = members
            .iter()
            .map(|member| {
                let sanitized_name = field_name_map.get(&member.name).unwrap_or(&member.name);
                let field_ident = format_ident!("{}", sanitized_name);
                quote! { #field_ident }
            })
            .collect();

        quote! {
            impl #type_ident {
                /// Deserialize partial JSON with presence tracking
                ///
                /// Parses JSON that may have missing required fields, filling them with defaults
                /// while tracking which fields were actually present in the JSON.
                ///
                /// # Parameters
                /// - `json_str`: JSON string (may be incomplete/partial)
                ///
                /// # Returns
                /// Tuple of (deserialized instance, presence array)
                /// - Instance has all fields filled (with defaults if missing)
                /// - Presence array indicates which fields were in the JSON (true) vs defaults (false)
                ///
                /// # Example
                /// ```ignore
                /// let json = r#"{"name": "Alice"}"#;  // Missing optional fields
                /// let (instance, presence) = MyStruct::from_json_partial(json)?;
                /// // presence[0] = true (name was present)
                /// // presence[1..] = false (other fields got defaults)
                /// ```
                pub fn from_json_partial(json_str: &str) -> Result<(Self, Vec<bool>), Box<dyn std::error::Error>> {
                    // Parse JSON as a generic Value to check field presence
                    let mut json_value: serde_json::Value = serde_json::from_str(json_str)?;

                    // Extract the object (or return error if not an object)
                    let obj = json_value.as_object_mut()
                        .ok_or_else(|| "Expected JSON object, got non-object value")?;

                    // Track which fields are present in the JSON
                    let field_names: Vec<&str> = vec![#(#field_names),*];
                    let default_values: Vec<serde_json::Value> = vec![#(#default_values),*];
                    let mut presence = vec![false; #field_count];

                    // Add defaults for missing fields
                    for (i, field_name) in field_names.iter().enumerate() {
                        if obj.contains_key(*field_name) {
                            presence[i] = true;
                        } else {
                            // Field is missing - add default value
                            obj.insert(field_name.to_string(), default_values[i].clone());
                            presence[i] = false;
                        }
                    }

                    // Deserialize the complete JSON (all fields now present with defaults filled)
                    let instance: Self = serde_json::from_value(json_value)?;

                    Ok((instance, presence))
                }

                /// Encode a partial instance with presence tracking
                ///
                /// # Parameters
                /// - `presence`: Boolean array indicating which fields were present (not defaults)
                ///
                /// # Returns
                /// Complete token sequence with gap metadata indicating which tokens are defaults
                pub fn encode_partial(&self, presence: &[bool]) -> PartialEncoding {
                    // Import JsonTokenEncoder trait for .encode() calls
                    #[allow(unused_imports)]
                    use crate::tokenizer::JsonTokenEncoder;

                    // Validate presence array length matches field count
                    assert!(
                        presence.len() == #field_count,
                        "Presence array length mismatch: expected {} fields, got {}. \
                         Ensure presence array has one boolean per struct field.",
                        #field_count, presence.len()
                    );

                    let mut tokens = Vec::new();
                    let mut gaps = Vec::new();

                    // Encode each field individually, tracking gaps
                    #( #encode_partial_fields )*

                    PartialEncoding { tokens, gaps }
                }

                /// Decode partial encoding (field-by-field format)
                ///
                /// Decodes a token sequence produced by encode_partial().
                /// This uses field-by-field decoding (no presence mask) to match
                /// the encoding format.
                ///
                /// # Parameters
                /// - `tokens`: Token slice from encode_partial()
                ///
                /// # Returns
                /// Decoded instance with all fields (including defaults)
                pub fn decode_partial(tokens: &[u32]) -> Result<Self, crate::tokenizer::DecodeError> {
                    let mut iter = crate::tokenizer::BufferedTokenIterator::new(tokens.iter().copied());

                    // Decode each field in sequence (matching encode_partial order)
                    #( #decode_partial_fields )*

                    Ok(Self {
                        #( #field_assignments ),*
                    })
                }

                /// Decode a default-filled partial encoding and report the first gap
                ///
                /// Decodes the complete token sequence produced by encode_partial().
                /// The returned gap metadata identifies the first default-filled
                /// field; it is not suffix-constrained parser state.
                ///
                /// Returns instance, stopped field index, and consumption tracking.
                pub fn decode_to_first_gap(encoding: &PartialEncoding) -> Result<PrefixResult<Self>, crate::tokenizer::DecodeError> {
                    let total = encoding.tokens.len();
                    let instance = Self::decode_partial(&encoding.tokens)?;
                    let stopped_at_field = encoding.gaps.first().map(|g| g.field_index).unwrap_or(#field_count);

                    Ok(PrefixResult {
                        instance,
                        stopped_at_field,
                        tokens_consumed: total,
                        total_tokens: total,
                    })
                }
            }
        }
    }
}
