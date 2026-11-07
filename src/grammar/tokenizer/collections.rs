//! Rendering for generated collection and number tokenizer implementations.

use proc_macro2::TokenStream;
use quote::quote;

pub(super) fn render_collections_and_numbers(
    has_vocab_constraint: bool,
    max_collection_length: usize,
    decimal_number_encoding: bool,
    end_string_marker: u32,
) -> TokenStream {
    if has_vocab_constraint {
        return quote! {};
    }

    let number_impls = if decimal_number_encoding {
        render_decimal_number_impls(end_string_marker)
    } else {
        render_default_number_impls()
    };

    quote! {
        #number_impls

        impl<T: JsonTokenEncoder> JsonTokenEncoder for Vec<T> {
            fn encode(&self, tokens: &mut Vec<u32>) {
                // Multi-token encoding for length to stay within vocabulary bounds
                variable_base::encode_u32_fixed_base(self.len() as u32, tokens);
                for item in self {
                    item.encode(tokens);
                }
            }
            fn decode(tokens: &mut impl Iterator<Item = u32>) -> Result<Self, DecodeError> {
                let len = variable_base::decode_u32_fixed_base(tokens, "Vec<T>")? as usize;
                // Prevent OOM from model-generated garbage (max matches context size)
                if len > #max_collection_length {
                    return Err(DecodeError::InvalidData(
                        format!("Vec length {} exceeds maximum {}", len, #max_collection_length)
                    ));
                }
                let mut vec = Vec::with_capacity(len);
                for index in 0..len {
                    let item = terminal_key_export::with_array_index(index, || T::decode(tokens))?;
                    vec.push(item);
                }
                Ok(vec)
            }
            fn decode_strict(tokens: &mut impl Iterator<Item = u32>) -> Result<Self, DecodeError> {
                let len = variable_base::decode_u32_fixed_base_strict(tokens, "Vec<T>")? as usize;
                if len > #max_collection_length {
                    return Err(DecodeError::InvalidData(
                        format!("Vec length {} exceeds maximum {}", len, #max_collection_length)
                    ));
                }
                let mut vec = Vec::with_capacity(len);
                for index in 0..len {
                    let item =
                        terminal_key_export::with_array_index(index, || T::decode_strict(tokens))?;
                    vec.push(item);
                }
                Ok(vec)
            }
            fn token_count(&self) -> usize {
                variable_base::token_count_u32_fixed_base(self.len() as u32)
                    + self.iter().map(|item| item.token_count()).sum::<usize>()
            }
        }

        impl<K, V> JsonTokenEncoder for rustc_hash::FxHashMap<K, V>
        where
            K: JsonTokenEncoder + std::cmp::Eq + std::hash::Hash,
            V: JsonTokenEncoder,
        {
            fn encode(&self, tokens: &mut Vec<u32>) {
                // Multi-token encoding for length to stay within vocabulary bounds
                variable_base::encode_u32_fixed_base(self.len() as u32, tokens);
                for (key, value) in self.iter() {
                    key.encode(tokens);
                    value.encode(tokens);
                }
            }
            fn decode(tokens: &mut impl Iterator<Item = u32>) -> Result<Self, DecodeError> {
                let len = variable_base::decode_u32_fixed_base(tokens, "FxHashMap<K,V>")? as usize;
                // Prevent OOM from model-generated garbage (max matches context size)
                if len > #max_collection_length {
                    return Err(DecodeError::InvalidData(
                        format!("HashMap length {} exceeds maximum {}", len, #max_collection_length)
                    ));
                }
                let mut map = rustc_hash::FxHashMap::default();
                for _ in 0..len {
                    let key = K::decode(tokens)?;
                    let value = V::decode(tokens)?;
                    map.insert(key, value);
                }
                Ok(map)
            }
            fn decode_strict(tokens: &mut impl Iterator<Item = u32>) -> Result<Self, DecodeError> {
                let len = variable_base::decode_u32_fixed_base_strict(tokens, "FxHashMap<K,V>")? as usize;
                if len > #max_collection_length {
                    return Err(DecodeError::InvalidData(
                        format!("HashMap length {} exceeds maximum {}", len, #max_collection_length)
                    ));
                }
                let mut map = rustc_hash::FxHashMap::default();
                for _ in 0..len {
                    let key = K::decode_strict(tokens)?;
                    let value = V::decode_strict(tokens)?;
                    map.insert(key, value);
                }
                Ok(map)
            }
            fn token_count(&self) -> usize {
                variable_base::token_count_u32_fixed_base(self.len() as u32)
                    + self.iter().map(|(k, v)| k.token_count() + v.token_count()).sum::<usize>()
            }
        }

        impl JsonTokenEncoder for i32 {
            fn encode(&self, tokens: &mut Vec<u32>) {
                // Multi-token encoding with configurable base to respect vocabulary bounds
                variable_base::encode_u32_fixed_base(*self as u32, tokens);
            }

            fn decode(tokens: &mut impl Iterator<Item = u32>) -> Result<Self, DecodeError> {
                // Multi-token decoding with configurable base
                Ok(variable_base::decode_u32_fixed_base(tokens, "i32")? as i32)
            }

            fn decode_strict(tokens: &mut impl Iterator<Item = u32>) -> Result<Self, DecodeError> {
                Ok(variable_base::decode_u32_fixed_base_strict(tokens, "i32")? as i32)
            }

            fn token_count(&self) -> usize {
                // Multi-token token count with configurable base
                variable_base::token_count_u32_fixed_base(*self as u32)
            }
        }

    }
}

/// Default bit-split encoders for unbounded i64/u64/f64 (8 byte tokens each).
fn render_default_number_impls() -> TokenStream {
    quote! {
        impl JsonTokenEncoder for i64 {
            fn encode(&self, tokens: &mut Vec<u32>) {
                // Encode as 8 bytes directly (each byte 0-255)
                let bytes = self.to_le_bytes();
                for byte in bytes {
                    tokens.push(byte as u32);
                }
            }

            fn decode(tokens: &mut impl Iterator<Item = u32>) -> Result<Self, DecodeError> {
                let mut bytes = [0u8; 8];
                for i in 0..8 {
                    let byte_token = tokens.next().ok_or(DecodeError::UnexpectedEndOfStream {
                        context: EOFContext::new("i64", Some("byte"), "byte value")
                    })?;
                    bytes[i] = (byte_token % 256) as u8;
                }
                Ok(i64::from_le_bytes(bytes))
            }

            fn decode_strict(tokens: &mut impl Iterator<Item = u32>) -> Result<Self, DecodeError> {
                let mut bytes = [0u8; 8];
                for i in 0..8 {
                    bytes[i] = decode_strict_byte_token(tokens, "i64")?;
                }
                Ok(i64::from_le_bytes(bytes))
            }

            fn token_count(&self) -> usize { 8 }
        }

        impl JsonTokenEncoder for u64 {
            fn encode(&self, tokens: &mut Vec<u32>) {
                // Encode as 8 bytes directly (each byte 0-255)
                let bytes = self.to_le_bytes();
                for byte in bytes {
                    tokens.push(byte as u32);
                }
            }

            fn decode(tokens: &mut impl Iterator<Item = u32>) -> Result<Self, DecodeError> {
                let mut bytes = [0u8; 8];
                for i in 0..8 {
                    let byte_token = tokens.next().ok_or(DecodeError::UnexpectedEndOfStream {
                        context: EOFContext::new("u64", Some("byte"), "byte value")
                    })?;
                    bytes[i] = (byte_token % 256) as u8;
                }
                Ok(u64::from_le_bytes(bytes))
            }

            fn decode_strict(tokens: &mut impl Iterator<Item = u32>) -> Result<Self, DecodeError> {
                let mut bytes = [0u8; 8];
                for i in 0..8 {
                    bytes[i] = decode_strict_byte_token(tokens, "u64")?;
                }
                Ok(u64::from_le_bytes(bytes))
            }

            fn token_count(&self) -> usize { 8 }
        }

        impl JsonTokenEncoder for f64 {
            fn encode(&self, tokens: &mut Vec<u32>) {
                // Encode as 8 bytes directly (each byte 0-255)
                let bytes = self.to_bits().to_le_bytes();
                for byte in bytes {
                    tokens.push(byte as u32);
                }
            }

            fn decode(tokens: &mut impl Iterator<Item = u32>) -> Result<Self, DecodeError> {
                let mut bytes = [0u8; 8];
                for i in 0..8 {
                    let byte_token = tokens.next().ok_or(DecodeError::UnexpectedEndOfStream {
                        context: EOFContext::new("f64", Some("byte"), "byte value")
                    })?;
                    bytes[i] = (byte_token % 256) as u8;
                }
                Ok(f64::from_bits(u64::from_le_bytes(bytes)))
            }

            fn decode_strict(tokens: &mut impl Iterator<Item = u32>) -> Result<Self, DecodeError> {
                let mut bytes = [0u8; 8];
                for i in 0..8 {
                    bytes[i] = decode_strict_byte_token(tokens, "f64")?;
                }
                Ok(f64::from_bits(u64::from_le_bytes(bytes)))
            }

            fn token_count(&self) -> usize { 8 }
        }
    }
}

/// Decimal digit-aligned encoders for unbounded i64/u64/f64 (opt-in).
///
/// Values are rendered with Rust's `Display` (shortest text that round-trips
/// exactly for f64) and emitted one byte token per ASCII character, terminated
/// by the same end marker string payloads use. Token streams are incompatible
/// with the default bit-split encoding, hence opt-in.
fn render_decimal_number_impls(end_string_marker: u32) -> TokenStream {
    quote! {
        /// Shared helpers for decimal digit-aligned number encoding.
        pub mod decimal_number_codec {
            use super::{DecodeError, EOFContext};

            const END_STRING_MARKER: u32 = #end_string_marker;
            /// Longest accepted decimal text; f64 Display never exceeds this.
            const MAX_DECIMAL_TEXT_LEN: usize = 512;

            pub fn encode_text(text: &str, tokens: &mut Vec<u32>) {
                for &byte in text.as_bytes() {
                    tokens.push(byte as u32);
                }
                tokens.push(END_STRING_MARKER);
            }

            fn is_decimal_byte(byte: u8) -> bool {
                byte.is_ascii_digit() || matches!(byte, b'-' | b'+' | b'.' | b'e' | b'E')
            }

            pub fn decode_text(
                tokens: &mut impl Iterator<Item = u32>,
                type_name: &'static str,
                strict: bool,
            ) -> Result<String, DecodeError> {
                let mut bytes: Vec<u8> = Vec::new();
                super::terminal_key_export::begin_host_value();
                loop {
                    let token = match tokens.next() {
                        Some(token) => token,
                        None => {
                            super::terminal_key_export::capture_host_value_eof();
                            return Err(DecodeError::UnexpectedEndOfStream {
                                context: EOFContext::new(
                                    type_name,
                                    Some("decimal_text"),
                                    "decimal digit byte or END_STRING_MARKER",
                                ),
                            });
                        }
                    };
                    if token == END_STRING_MARKER {
                        super::role_export::mark_value_end();
                        break;
                    }
                    let byte = (token % 256) as u8;
                    super::role_export::mark_value();
                    if strict && (token > 255 || !is_decimal_byte(byte)) {
                        super::terminal_key_export::end_host_value();
                        return Err(DecodeError::InvalidTokenInContext {
                            token,
                            context: EOFContext::new(
                                type_name,
                                Some("decimal_text"),
                                "ASCII digit, sign, '.', 'e', or END_STRING_MARKER",
                            ),
                        });
                    }
                    bytes.push(byte);
                    if bytes.len() > MAX_DECIMAL_TEXT_LEN {
                        super::terminal_key_export::end_host_value();
                        return Err(DecodeError::InvalidData(format!(
                            "{} decimal text exceeds {} bytes",
                            type_name, MAX_DECIMAL_TEXT_LEN
                        )));
                    }
                }
                super::terminal_key_export::end_host_value();
                String::from_utf8(bytes).map_err(|_| DecodeError::Utf8Error)
            }
        }

        /// Decimal digit-aligned i64 encoder
        impl JsonTokenEncoder for i64 {
            fn encode(&self, tokens: &mut Vec<u32>) {
                decimal_number_codec::encode_text(&self.to_string(), tokens);
            }

            fn decode(tokens: &mut impl Iterator<Item = u32>) -> Result<Self, DecodeError> {
                let text = decimal_number_codec::decode_text(tokens, "i64", false)?;
                text.parse::<i64>().map_err(|_| DecodeError::InvalidData(
                    format!("invalid decimal i64 text {:?}", text)
                ))
            }

            fn decode_strict(tokens: &mut impl Iterator<Item = u32>) -> Result<Self, DecodeError> {
                let text = decimal_number_codec::decode_text(tokens, "i64", true)?;
                text.parse::<i64>().map_err(|_| DecodeError::InvalidData(
                    format!("invalid decimal i64 text {:?}", text)
                ))
            }

            fn token_count(&self) -> usize {
                // digits (+ sign) + END_STRING_MARKER
                let mut len = 1usize; // marker
                let mut value = *self;
                if value < 0 { len += 1; }
                loop {
                    len += 1;
                    value /= 10;
                    if value == 0 { break; }
                }
                len
            }
        }

        /// Decimal digit-aligned u64 encoder
        impl JsonTokenEncoder for u64 {
            fn encode(&self, tokens: &mut Vec<u32>) {
                decimal_number_codec::encode_text(&self.to_string(), tokens);
            }

            fn decode(tokens: &mut impl Iterator<Item = u32>) -> Result<Self, DecodeError> {
                let text = decimal_number_codec::decode_text(tokens, "u64", false)?;
                text.parse::<u64>().map_err(|_| DecodeError::InvalidData(
                    format!("invalid decimal u64 text {:?}", text)
                ))
            }

            fn decode_strict(tokens: &mut impl Iterator<Item = u32>) -> Result<Self, DecodeError> {
                let text = decimal_number_codec::decode_text(tokens, "u64", true)?;
                text.parse::<u64>().map_err(|_| DecodeError::InvalidData(
                    format!("invalid decimal u64 text {:?}", text)
                ))
            }

            fn token_count(&self) -> usize {
                let mut len = 1usize; // marker
                let mut value = *self;
                loop {
                    len += 1;
                    value /= 10;
                    if value == 0 { break; }
                }
                len
            }
        }

        /// Decimal digit-aligned f64 encoder.
        ///
        /// Rust's `Display` for f64 emits the shortest decimal text that
        /// parses back to the identical bits, so encode+parse is an exact
        /// round trip for every finite value. NaN/Inf are not valid JSON
        /// numbers and are rejected on decode.
        impl JsonTokenEncoder for f64 {
            fn encode(&self, tokens: &mut Vec<u32>) {
                decimal_number_codec::encode_text(&self.to_string(), tokens);
            }

            fn decode(tokens: &mut impl Iterator<Item = u32>) -> Result<Self, DecodeError> {
                let text = decimal_number_codec::decode_text(tokens, "f64", false)?;
                let value = text.parse::<f64>().map_err(|_| DecodeError::InvalidData(
                    format!("invalid decimal f64 text {:?}", text)
                ))?;
                if !value.is_finite() {
                    return Err(DecodeError::InvalidData(
                        "non-finite f64 is not a valid JSON number".to_string(),
                    ));
                }
                Ok(value)
            }

            fn decode_strict(tokens: &mut impl Iterator<Item = u32>) -> Result<Self, DecodeError> {
                let text = decimal_number_codec::decode_text(tokens, "f64", true)?;
                let value = text.parse::<f64>().map_err(|_| DecodeError::InvalidData(
                    format!("invalid decimal f64 text {:?}", text)
                ))?;
                if !value.is_finite() {
                    return Err(DecodeError::InvalidData(
                        "non-finite f64 is not a valid JSON number".to_string(),
                    ));
                }
                Ok(value)
            }

            fn token_count(&self) -> usize {
                self.to_string().len() + 1
            }
        }
    }
}
