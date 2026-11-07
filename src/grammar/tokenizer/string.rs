//! Rendering for the generated standalone `String` tokenizer implementation.

use proc_macro2::TokenStream;
use quote::quote;

use crate::generator::encoding_decision::StringEncoding;

pub(super) fn render_string_impl(
    string_encoding: StringEncoding,
    has_vocab_constraint: bool,
) -> TokenStream {
    if has_vocab_constraint {
        return quote! {};
    }

    let end_string_marker = string_encoding.end_marker();
    let max_byte_value = string_encoding.max_byte_value();
    let mode_description = match string_encoding {
        StringEncoding::Ascii => "ASCII byte or END_STRING_MARKER (128)",
        StringEncoding::Utf8 => "UTF-8 byte or END_STRING_MARKER (256)",
    };
    let ascii_encode_guard = match string_encoding {
        StringEncoding::Ascii => quote! {
            assert!(
                self.is_ascii(),
                "StringEncoding::Ascii cannot encode non-ASCII strings; use StringEncoding::Utf8 for Unicode input"
            );
        },
        StringEncoding::Utf8 => quote! {},
    };

    quote! {
        impl JsonTokenEncoder for String {
            fn encode(&self, tokens: &mut Vec<u32>) {
                const END_STRING_MARKER: u32 = #end_string_marker;
                #ascii_encode_guard
                tokens.reserve(self.len() + 1);
                tokens.extend(self.as_bytes().iter().map(|&b| b as u32));
                tokens.push(END_STRING_MARKER);
            }

            fn decode(tokens: &mut impl Iterator<Item = u32>) -> Result<Self, DecodeError> {
                const END_STRING_MARKER: u32 = #end_string_marker;
                const MAX_BYTE_VALUE: u32 = #max_byte_value;
                let mut bytes = Vec::new();
                terminal_key_export::begin_host_value();

                loop {
                    let token = match tokens.next() {
                        Some(token) => token,
                        None => {
                            terminal_key_export::capture_host_value_eof();
                            return Err(DecodeError::UnexpectedEndOfStream {
                                context: EOFContext::new(
                                    "String",
                                    Some("byte or end marker"),
                                    #mode_description,
                                ),
                            });
                        }
                    };

                    if token == END_STRING_MARKER {
                        role_export::mark_value_end();
                        break;
                    }

                    if token > MAX_BYTE_VALUE {
                        terminal_key_export::end_host_value();
                        return Err(DecodeError::InvalidToken(token));
                    }

                    role_export::mark_value();
                    bytes.push(token as u8);
                }

                terminal_key_export::end_host_value();
                String::from_utf8(bytes).map_err(|e| DecodeError::InvalidData(
                    format!("Invalid UTF-8 in string decode: {}", e)
                ))
            }

            fn token_count(&self) -> usize { self.len() + 1 }
        }
    }
}
