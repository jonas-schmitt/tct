//! Generated prefix-decoding helpers for streaming tokenizer APIs.
//!
//! These snippets tolerate truncated strings and vectors so generated
//! `decode_prefix` implementations can return partial values without changing
//! the strict `JsonTokenEncoder::decode` contract.

use proc_macro2::TokenStream;
use quote::quote;

pub(super) fn render_streaming_helpers(
    end_string_marker: u32,
    max_byte_value: u32,
    encoding_base: u32,
) -> TokenStream {
    quote! {
        /// Decode a string with fallback to partial decode for streaming mode.
        ///
        /// Unlike `String::decode()`, this function doesn't return an error if the
        /// stream is truncated before the END_STRING_MARKER is encountered - instead,
        /// it returns what could be decoded as a partial string.
        ///
        /// # Returns
        /// - `(value, complete, bytes_decoded)` tuple
        #[inline]
        pub fn decode_string_streaming(tokens: &mut impl Iterator<Item = u32>) -> (String, bool, usize) {
            const END_STRING_MARKER: u32 = #end_string_marker;
            const MAX_BYTE_VALUE: u32 = #max_byte_value;

            let mut bytes = Vec::new();

            loop {
                match tokens.next() {
                    Some(token) if token == END_STRING_MARKER => {
                        // Complete string found
                        let value = String::from_utf8(bytes.clone())
                            .unwrap_or_else(|_| String::from_utf8_lossy(&bytes).to_string());
                        return (value, true, bytes.len());
                    }
                    Some(token) if token <= MAX_BYTE_VALUE => {
                        // Valid string byte for the configured string encoding
                        bytes.push(token as u8);
                    }
                    Some(_) | None => {
                        // Stream ended or invalid token - return partial result
                        let value = String::from_utf8_lossy(&bytes).to_string();
                        return (value, false, bytes.len());
                    }
                }
            }
        }

        /// Decode a Vec<T> with fallback to partial decode for streaming mode.
        ///
        /// This function returns elements that could be decoded, even if the expected
        /// count wasn't reached. This is used by generated `decode_prefix` methods
        /// for Vec fields.
        ///
        /// # Returns
        /// - `(items, complete, expected_length)` tuple
        #[inline]
        pub fn decode_vec_streaming<T: JsonTokenEncoder>(
            tokens: &mut impl Iterator<Item = u32>,
        ) -> (Vec<T>, bool, Option<usize>) {
            // Try to decode length using variable_base (matches Vec::encode)
            // Note: This uses the same encoding_base as Vec encode/decode
            let length = match variable_base::decode_u32_option(tokens, #encoding_base) {
                Some(len) => len as usize,
                None => return (Vec::new(), false, None),
            };

            let mut items = Vec::with_capacity(length.min(1024)); // Cap allocation for safety

            for _ in 0..length {
                match T::decode(tokens) {
                    Ok(item) => items.push(item),
                    Err(_) => {
                        return (items, false, Some(length));
                    }
                }
            }

            let items_len = items.len();
            (items, items_len == length, Some(length))
        }

        /// Decode a Vec<String> while preserving a truncated in-progress
        /// string element as part of the partial prefix result.
        #[inline]
        pub fn decode_string_vec_streaming(
            tokens: &mut impl Iterator<Item = u32>,
        ) -> (Vec<String>, bool, Option<usize>) {
            let length = match variable_base::decode_u32_option(tokens, #encoding_base) {
                Some(len) => len as usize,
                None => return (Vec::new(), false, None),
            };

            let mut items = Vec::with_capacity(length.min(1024));

            for _ in 0..length {
                let (value, complete, _bytes_decoded) = decode_string_streaming(tokens);
                items.push(value);
                if !complete {
                    return (items, false, Some(length));
                }
            }

            (items, true, Some(length))
        }
    }
}
