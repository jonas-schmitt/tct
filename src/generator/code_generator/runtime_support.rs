//! Runtime-support renderers shared by generated tokenizer crates.
//!
//! The helpers in this module emit compact codec functions for vocabulary
//! constrained length and integer values. They are generated once per tokenizer
//! crate and keep token IDs within `MAX_VOCAB` while preserving deterministic
//! encode/decode behavior.

use proc_macro2::TokenStream;
use quote::quote;

/// Generate vocabulary-constrained codec helper functions.
///
/// These helpers ensure all token values respect the MAX_VOCAB constraint by using
/// multi-token codec encoding when values exceed the vocabulary budget.
pub(super) fn generate_codec_helpers(_max_vocab: u32) -> TokenStream {
    quote! {
        /// Encode a length value respecting MAX_VOCAB constraint
        /// Uses multi-token codec encoding if length >= MAX_VOCAB-1
        /// Reserves (MAX_VOCAB-1) as escape token for multi-token encoding
        #[inline(always)]
        fn encode_length_constrained(len: usize, tokens: &mut Vec<u32>) {
            const ESCAPE_TOKEN: u32 = MAX_VOCAB - 1;
            let len_u32 = len as u32;

            if len_u32 < ESCAPE_TOKEN {
                // Fast path: single token (0..MAX_VOCAB-2)
                tokens.push(len_u32);
            } else {
                // Multi-token encoding: [ESCAPE_TOKEN, width-1, digit_n, ..., digit_0]
                // All tokens remain < MAX_VOCAB
                let base = MAX_VOCAB;
                let mut token_width = 1u32;
                while base.saturating_pow(token_width) <= len_u32 {
                    token_width += 1;
                    if token_width >= 10 { break; }  // Safety limit
                }

                // Emit escape token to indicate multi-token encoding follows
                tokens.push(ESCAPE_TOKEN);

                // Emit width-1 (so 0 = 1 token, 1 = 2 tokens, etc.)
                tokens.push(token_width - 1);

                // Emit digits in big-endian order
                for i in (0..token_width).rev() {
                    let divisor = base.pow(i);
                    tokens.push((len_u32 / divisor) % base);
                }
            }
        }

        /// Decode a length value respecting MAX_VOCAB constraint
        #[inline(always)]
        fn decode_length_constrained(
            tokens: &mut impl Iterator<Item = u32>
        ) -> Result<usize, crate::tokenizer::DecodeError> {
            const ESCAPE_TOKEN: u32 = MAX_VOCAB - 1;
            let raw_token = tokens
                .next()
                .ok_or(crate::tokenizer::DecodeError::UnexpectedEndOfStream {
                    context: crate::tokenizer::EOFContext::new("ConstrainedLength", None, "length value")
                })?;
            let first_token = raw_token % MAX_VOCAB;

            if first_token < ESCAPE_TOKEN {
                // Fast path: single token length (0..MAX_VOCAB-2)
                Ok(first_token as usize)
            } else if first_token == ESCAPE_TOKEN {
                // Multi-token encoding: read width, then digits
                let width_minus_1 = tokens.next()
                    .ok_or(crate::tokenizer::DecodeError::UnexpectedEndOfStream {
                        context: crate::tokenizer::EOFContext::new("ConstrainedLength", Some("width"), "token width")
                    })?;
                let token_width = (width_minus_1 + 1) as usize;
                let base = MAX_VOCAB;

                // Decode digits in big-endian order
                let mut value = 0u32;
                for _ in 0..token_width {
                    let digit = tokens.next()
                        .ok_or(crate::tokenizer::DecodeError::UnexpectedEndOfStream {
                            context: crate::tokenizer::EOFContext::new("ConstrainedLength", Some("digit"), "length digit")
                        })?;
                    value = value.saturating_mul(base).saturating_add(digit % base);
                }

                Ok(value as usize)
            } else {
                // Should never happen with modulo
                unreachable!("first_token % MAX_VOCAB should always be < MAX_VOCAB")
            }
        }

        /// Encode a u32 value respecting MAX_VOCAB constraint using base-N encoding
        /// Uses fixed-width encoding: calculates token_width from value, emits width, then digits
        #[inline(always)]
        fn encode_u32_constrained(value: u32, tokens: &mut Vec<u32>) {
            let base = MAX_VOCAB;
            assert!(base >= 2, "BUG: MAX_VOCAB must be >= 2, got {}", base);
            if value < base {
                // Single token: emit 0 (width=1) then the value
                tokens.push(0u32);  // Width - 1 (0 means 1 token)
                tokens.push(value);
                return;
            }

            // Calculate token width needed
            let mut token_width = 0u32;
            loop {
                token_width += 1;
                let max_representable = base.saturating_pow(token_width);
                if value < max_representable {
                    break;
                }
                if token_width >= 10 {
                    // Safety limit to prevent infinite loop
                    break;
                }
            }

            // Emit token width - 1 (so 0 = 1 token, 1 = 2 tokens, etc.)
            tokens.push(token_width - 1);

            // Emit digits in big-endian order
            for i in (0..token_width).rev() {
                // Use saturating_pow to prevent overflow
                let divisor = base.saturating_pow(i);
                tokens.push((value / divisor) % base);
            }
        }

        /// Decode a u32 value respecting MAX_VOCAB constraint
        #[inline(always)]
        fn decode_u32_constrained(
            tokens: &mut impl Iterator<Item = u32>
        ) -> Result<u32, crate::tokenizer::DecodeError> {
            let first_token = tokens
                .next()
                .ok_or(crate::tokenizer::DecodeError::UnexpectedEndOfStream {
                    context: crate::tokenizer::EOFContext::new("ConstrainedU32", None, "integer width")
                })?;
            decode_u32_constrained_with_first(first_token, tokens)
        }

        /// Decode a u32 value from multi-token codec encoding (with first token already read)
        ///
        /// Uses saturating arithmetic to match encoder's saturating_pow behavior.
        /// The encoder uses saturating_pow which caps at u32::MAX, so the decoder
        /// must use saturating arithmetic to correctly decode those values.
        #[inline(always)]
        fn decode_u32_constrained_with_first(
            first_token: u32,
            tokens: &mut impl Iterator<Item = u32>
        ) -> Result<u32, crate::tokenizer::DecodeError> {
            let base = MAX_VOCAB;

            // First token is the width - 1
            let token_width = (first_token % base) + 1;

            // Decode digits in big-endian order using saturating arithmetic
            let mut value = 0u32;
            for i in (0..token_width).rev() {
                let digit = tokens
                    .next()
                    .ok_or(crate::tokenizer::DecodeError::UnexpectedEndOfStream {
                        context: crate::tokenizer::EOFContext::new("ConstrainedU32", Some("digit"), "integer digit")
                    })?
                    % base;

                // Use saturating_pow to match encoder behavior
                let power = base.saturating_pow(i);
                value = value.saturating_add(digit.saturating_mul(power));
            }

            Ok(value)
        }
    }
}

/// Generate vocabulary-constrained collection encoder implementations.
///
/// These implementations override the default Vec/HashMap/String encoders
/// to use vocabulary-constrained length encoding.
pub(super) fn generate_collection_encoders(max_collection_length: usize) -> TokenStream {
    // Note: max_string_length removed - end-marker encoding eliminates length constraints
    quote! {
        /// Vocabulary-constrained String encoder
        ///
        /// ALWAYS uses END_STRING_MARKER encoding (never switches to length-prefix).
        /// When MAX_BASE_VOCAB is set, END_STRING_MARKER = MAX_BASE_VOCAB - 1,
        /// and bytes are encoded using base = END_STRING_MARKER.
        impl crate::tokenizer::JsonTokenEncoder for String {
            fn encode(&self, tokens: &mut Vec<u32>) {
                // END_STRING_MARKER encoding (ALWAYS used)
                // Reserve highest token for END_STRING_MARKER
                const END_STRING_MARKER: u32 = crate::tokenizer::MAX_BASE_VOCAB - 1;
                const ENCODING_BASE: u32 = END_STRING_MARKER;  // Bytes encoded in this base

                // Encode each byte using multi-token codec if necessary
                for &byte in self.as_bytes() {
                    let byte_u32 = byte as u32;
                    if byte_u32 < ENCODING_BASE {
                        // Single token: emit directly
                        tokens.push(byte_u32);
                    } else {
                        // Multi-token encoding in base=ENCODING_BASE
                        // Calculate how many tokens needed
                        let mut value = byte_u32;
                        let mut digits = Vec::new();
                        while value > 0 || digits.is_empty() {
                            digits.push(value % ENCODING_BASE);
                            value /= ENCODING_BASE;
                        }
                        // Emit width, then digits (big-endian)
                        tokens.push((digits.len() - 1) as u32);  // Width - 1
                        for digit in digits.iter().rev() {
                            tokens.push(*digit);
                        }
                    }
                }

                // ALWAYS emit END_STRING_MARKER to signal end
                tokens.push(END_STRING_MARKER);
            }

            fn decode(
                tokens: &mut impl Iterator<Item = u32>
            ) -> Result<Self, crate::tokenizer::DecodeError> {
                const END_STRING_MARKER: u32 = crate::tokenizer::MAX_BASE_VOCAB - 1;
                const ENCODING_BASE: u32 = END_STRING_MARKER;

                // Get first token.
                let first_token = tokens.next().ok_or(crate::tokenizer::DecodeError::UnexpectedEndOfStream {
                    context: crate::tokenizer::EOFContext::new("String", None, "byte or END_STRING_MARKER")
                })?;

                // END_STRING_MARKER decoding: read bytes until END_STRING_MARKER
                let mut bytes = Vec::new();
                let mut current_token = first_token;

                loop {
                    // Check for END_STRING_MARKER
                    if current_token == END_STRING_MARKER {
                        break;  // End of string
                    }

                    // Decode byte (may be multi-token encoded)
                    let byte_val = if current_token < ENCODING_BASE {
                        // Single token: direct value
                        current_token
                    } else {
                        // This is a width indicator for multi-token encoding
                        let width = (current_token + 1) as usize;  // width = token + 1
                        let mut value = 0u32;
                        for _ in 0..width {
                            let digit = tokens.next().ok_or(crate::tokenizer::DecodeError::UnexpectedEndOfStream {
                                context: crate::tokenizer::EOFContext::new("String", Some("byte_digit"), "multi-token byte digit")
                            })?;
                            value = value * ENCODING_BASE + (digit % ENCODING_BASE);
                        }
                        value
                    };

                    // Convert to byte with modulo for robustness
                    let byte = (byte_val % 256) as u8;
                    bytes.push(byte);

                    // Get next token
                    current_token = tokens.next().ok_or(crate::tokenizer::DecodeError::UnexpectedEndOfStream {
                        context: crate::tokenizer::EOFContext::new("String", Some("byte or end marker"), "byte or END_STRING_MARKER")
                    })?;
                }

                String::from_utf8(bytes).map_err(|_| crate::tokenizer::DecodeError::Utf8Error)
            }

            fn token_count(&self) -> usize {
                // END_STRING_MARKER encoding (ALWAYS used)
                // Reserve highest token for END_STRING_MARKER
                const END_STRING_MARKER: u32 = crate::tokenizer::MAX_BASE_VOCAB - 1;
                const ENCODING_BASE: u32 = END_STRING_MARKER;

                // Calculate tokens for byte content
                let mut byte_tokens = 0usize;
                for &byte in self.as_bytes() {
                    let byte_u32 = byte as u32;
                    if byte_u32 < ENCODING_BASE {
                        // Single token: emit directly
                        byte_tokens += 1;
                    } else {
                        // Multi-token encoding: width indicator + digits
                        let mut value = byte_u32;
                        let mut width = 0usize;
                        while value > 0 {
                            width += 1;
                            value /= ENCODING_BASE;
                        }
                        // Emit width, then digits
                        byte_tokens += 1 + width;
                    }
                }

                // Add 1 for END_STRING_MARKER
                byte_tokens + 1
            }
        }

        /// Vocabulary-constrained Vec encoder
        impl<T: crate::tokenizer::JsonTokenEncoder> crate::tokenizer::JsonTokenEncoder for Vec<T> {
            fn encode(&self, tokens: &mut Vec<u32>) {
                // Use constrained length encoding
                encode_length_constrained(self.len(), tokens);
                // Encode elements
                for item in self {
                    item.encode(tokens);
                }
            }

            fn decode(
                tokens: &mut impl Iterator<Item = u32>
            ) -> Result<Self, crate::tokenizer::DecodeError> {
                // Decode constrained length
                let len = decode_length_constrained(tokens)?;
                if len > #max_collection_length {
                    return Err(crate::tokenizer::DecodeError::InvalidData(
                        format!("Vec length {} exceeds maximum {}", len, #max_collection_length)
                    ));
                }
                // Decode elements
                let mut vec = Vec::with_capacity(len);
                for _ in 0..len {
                    vec.push(T::decode(tokens)?);
                }
                Ok(vec)
            }

            fn token_count(&self) -> usize {
                const ESCAPE_TOKEN: u32 = MAX_VOCAB - 1;
                let len_u32 = self.len() as u32;

                let len_tokens = if len_u32 < ESCAPE_TOKEN {
                    1  // Direct encoding
                } else {
                    // Escape token + width + width digits
                    let base = MAX_VOCAB;
                    let mut token_width = 1u32;
                    while base.saturating_pow(token_width) <= len_u32 {
                        token_width += 1;
                        if token_width >= 10 { break; }
                    }
                    2 + token_width as usize
                };

                len_tokens + self.iter().map(|item| item.token_count()).sum::<usize>()
            }
        }

        /// Vocabulary-constrained HashMap encoder
        impl<K, V> crate::tokenizer::JsonTokenEncoder for rustc_hash::FxHashMap<K, V>
        where
            K: crate::tokenizer::JsonTokenEncoder + std::hash::Hash + Eq,
            V: crate::tokenizer::JsonTokenEncoder,
        {
            fn encode(&self, tokens: &mut Vec<u32>) {
                // Use constrained length encoding
                encode_length_constrained(self.len(), tokens);
                // Encode key-value pairs
                for (key, value) in self.iter() {
                    key.encode(tokens);
                    value.encode(tokens);
                }
            }

            fn decode(
                tokens: &mut impl Iterator<Item = u32>
            ) -> Result<Self, crate::tokenizer::DecodeError> {
                // Decode constrained length
                let len = decode_length_constrained(tokens)?;
                if len > #max_collection_length {
                    return Err(crate::tokenizer::DecodeError::InvalidData(
                        format!("HashMap length {} exceeds maximum {}", len, #max_collection_length)
                    ));
                }
                // Decode key-value pairs
                let mut map = rustc_hash::FxHashMap::default();
                for _ in 0..len {
                    let key = K::decode(tokens)?;
                    let value = V::decode(tokens)?;
                    map.insert(key, value);
                }
                Ok(map)
            }

            fn token_count(&self) -> usize {
                const ESCAPE_TOKEN: u32 = MAX_VOCAB - 1;
                let len_u32 = self.len() as u32;

                let len_tokens = if len_u32 < ESCAPE_TOKEN {
                    1  // Direct encoding
                } else {
                    // Escape token + width + width digits
                    let base = MAX_VOCAB;
                    let mut token_width = 1u32;
                    while base.saturating_pow(token_width) <= len_u32 {
                        token_width += 1;
                        if token_width >= 10 { break; }
                    }
                    2 + token_width as usize
                };

                len_tokens + self.iter().map(|(k, v)| k.token_count() + v.token_count()).sum::<usize>()
            }
        }
    }
}

/// Generate vocabulary-constrained number encoder implementations.
///
/// These implementations override the default i32/i64/f64 encoders from primitives.rs
/// to use vocabulary-constrained encoding for all numeric values.
///
/// With `decimal_number_encoding`, unbounded i64/u64/f64 values are encoded as
/// their shortest round-trip decimal text — one byte token per ASCII character
/// (identical token ids to the same characters in string payloads) terminated
/// by END_STRING_MARKER — instead of fixed-width bit splits. Motivation:
/// digit-aligned tokens let a language model copy numeric values positionally from prompt text, and they cost
/// len(text)+1 tokens instead of a flat 6-8. Bounded integers are unaffected
/// (separate optimization path). Opt-in: token streams are incompatible with
/// the default encoding, so existing corpora/wheels keep the old layout.
pub(super) fn generate_number_encoders(decimal_number_encoding: bool) -> TokenStream {
    if decimal_number_encoding {
        return generate_decimal_number_encoders();
    }
    quote! {
        /// Vocabulary-constrained i32 encoder
        /// Uses zigzag encoding to map signed integers to unsigned: 0, -1, 1, -2, 2, ...
        impl crate::tokenizer::JsonTokenEncoder for i32 {
            fn encode(&self, tokens: &mut Vec<u32>) {
                // Zigzag encoding: maps negatives to odd numbers, positives to even
                // 0 -> 0, -1 -> 1, 1 -> 2, -2 -> 3, 2 -> 4, ...
                let zigzag = if *self >= 0 {
                    (*self as u32) * 2
                } else {
                    let abs_val = self.wrapping_neg() as u32;
                    (abs_val * 2) - 1
                };
                encode_u32_constrained(zigzag, tokens);
            }

            fn decode(
                tokens: &mut impl Iterator<Item = u32>
            ) -> Result<Self, crate::tokenizer::DecodeError> {
                let first_token = tokens
                    .next()
                    .ok_or(crate::tokenizer::DecodeError::UnexpectedEndOfStream {
                        context: crate::tokenizer::EOFContext::new("i32", None, "zigzag-encoded integer")
                    })?;
                let zigzag = decode_u32_constrained_with_first(first_token, tokens)?;
                // Decode zigzag: even -> positive, odd -> negative
                let value = if zigzag % 2 == 0 {
                    (zigzag / 2) as i32
                } else {
                    -(((zigzag + 1) / 2) as i32)
                };
                Ok(value)
            }

            fn token_count(&self) -> usize {
                let zigzag = if *self >= 0 {
                    (*self as u32) * 2
                } else {
                    let abs_val = self.wrapping_neg() as u32;
                    (abs_val * 2) - 1
                };
                if zigzag < MAX_VOCAB { 2 } else { 3 }  // width + 1-2 digits for typical values
            }
        }

        /// Vocabulary-constrained i64 encoder
        impl crate::tokenizer::JsonTokenEncoder for i64 {
            fn encode(&self, tokens: &mut Vec<u32>) {
                // Split i64 into bytes
                let bytes = self.to_le_bytes();
                let low = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
                let high = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);

                // Encode each u32 with vocabulary constraint
                encode_u32_constrained(low, tokens);
                encode_u32_constrained(high, tokens);
            }

            fn decode(
                tokens: &mut impl Iterator<Item = u32>
            ) -> Result<Self, crate::tokenizer::DecodeError> {
                // Decode low u32
                let first_token_low = tokens
                    .next()
                    .ok_or(crate::tokenizer::DecodeError::UnexpectedEndOfStream {
                        context: crate::tokenizer::EOFContext::new("i64", Some("low_bits"), "low 32 bits")
                    })?;
                let low = decode_u32_constrained_with_first(first_token_low, tokens)?;

                // Decode high u32
                let first_token_high = tokens
                    .next()
                    .ok_or(crate::tokenizer::DecodeError::UnexpectedEndOfStream {
                        context: crate::tokenizer::EOFContext::new("i64", Some("high_bits"), "high 32 bits")
                    })?;
                let high = decode_u32_constrained_with_first(first_token_high, tokens)?;

                // Reconstruct i64 from bytes
                let mut bytes = [0u8; 8];
                bytes[0..4].copy_from_slice(&low.to_le_bytes());
                bytes[4..8].copy_from_slice(&high.to_le_bytes());
                Ok(i64::from_le_bytes(bytes))
            }

            fn token_count(&self) -> usize {
                // Two u32 values, each encoded separately (conservative estimate)
                6  // 2 * 3 tokens for typical u32 values
            }
        }

        /// Vocabulary-constrained u64 encoder
        impl crate::tokenizer::JsonTokenEncoder for u64 {
            fn encode(&self, tokens: &mut Vec<u32>) {
                let bytes = self.to_le_bytes();
                let low = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
                let high = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);

                encode_u32_constrained(low, tokens);
                encode_u32_constrained(high, tokens);
            }

            fn decode(
                tokens: &mut impl Iterator<Item = u32>
            ) -> Result<Self, crate::tokenizer::DecodeError> {
                let first_token_low = tokens
                    .next()
                    .ok_or(crate::tokenizer::DecodeError::UnexpectedEndOfStream {
                        context: crate::tokenizer::EOFContext::new("u64", Some("low_bits"), "low 32 bits")
                    })?;
                let low = decode_u32_constrained_with_first(first_token_low, tokens)?;

                let first_token_high = tokens
                    .next()
                    .ok_or(crate::tokenizer::DecodeError::UnexpectedEndOfStream {
                        context: crate::tokenizer::EOFContext::new("u64", Some("high_bits"), "high 32 bits")
                    })?;
                let high = decode_u32_constrained_with_first(first_token_high, tokens)?;

                let bits = (u64::from(high) << 32) | u64::from(low);
                Ok(bits)
            }

            fn token_count(&self) -> usize {
                6
            }
        }

        /// Vocabulary-constrained f64 encoder
        impl crate::tokenizer::JsonTokenEncoder for f64 {
            fn encode(&self, tokens: &mut Vec<u32>) {
                // Split f64 into two u32 values
                let bits = self.to_bits();
                let low = bits as u32;
                let high = (bits >> 32) as u32;

                // Encode each u32 with vocabulary constraint
                encode_u32_constrained(low, tokens);
                encode_u32_constrained(high, tokens);
            }

            fn decode(
                tokens: &mut impl Iterator<Item = u32>
            ) -> Result<Self, crate::tokenizer::DecodeError> {
                // Decode low u32
                let first_token_low = tokens
                    .next()
                    .ok_or(crate::tokenizer::DecodeError::UnexpectedEndOfStream {
                        context: crate::tokenizer::EOFContext::new("f64", Some("low_bits"), "low 32 bits")
                    })?;
                let low = decode_u32_constrained_with_first(first_token_low, tokens)?;

                // Decode high u32
                let first_token_high = tokens
                    .next()
                    .ok_or(crate::tokenizer::DecodeError::UnexpectedEndOfStream {
                        context: crate::tokenizer::EOFContext::new("f64", Some("high_bits"), "high 32 bits")
                    })?;
                let high = decode_u32_constrained_with_first(first_token_high, tokens)?;

                // Reconstruct f64 from bits
                let bits = (high as u64) << 32 | (low as u64);
                Ok(f64::from_bits(bits))
            }

            fn token_count(&self) -> usize {
                // Two u32 values, each encoded separately (conservative estimate)
                6  // 2 * 3 tokens for typical u32 values
            }
        }
    }
}

/// Decimal digit-aligned encoders for unbounded i64/u64/f64 (opt-in).
///
/// Values are rendered with Rust's `Display` (shortest representation that
/// round-trips exactly for f64) and emitted one byte token per ASCII
/// character, terminated by END_STRING_MARKER — the same token ids the same
/// characters get inside string payloads. i32 keeps the zigzag encoding
/// (interior/index use), and bounded integers keep their dedicated compact
/// path.
fn generate_decimal_number_encoders() -> TokenStream {
    quote! {
        /// Shared helpers for decimal digit-aligned number encoding.
        pub mod decimal_number_codec {
            const END_STRING_MARKER: u32 = crate::tokenizer::MAX_BASE_VOCAB - 1;
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
            ) -> Result<String, crate::tokenizer::DecodeError> {
                let mut bytes: Vec<u8> = Vec::new();
                loop {
                    let token = tokens.next().ok_or(
                        crate::tokenizer::DecodeError::UnexpectedEndOfStream {
                            context: crate::tokenizer::EOFContext::new(
                                type_name,
                                Some("decimal_text"),
                                "decimal digit byte or END_STRING_MARKER",
                            ),
                        },
                    )?;
                    if token == END_STRING_MARKER {
                        break;
                    }
                    let byte = (token % 256) as u8;
                    if strict && (token > 255 || !is_decimal_byte(byte)) {
                        return Err(crate::tokenizer::DecodeError::InvalidTokenInContext {
                            token,
                            context: crate::tokenizer::EOFContext::new(
                                type_name,
                                Some("decimal_text"),
                                "ASCII digit, sign, '.', 'e', or END_STRING_MARKER",
                            ),
                        });
                    }
                    bytes.push(byte);
                    if bytes.len() > MAX_DECIMAL_TEXT_LEN {
                        return Err(crate::tokenizer::DecodeError::InvalidData(format!(
                            "{} decimal text exceeds {} bytes",
                            type_name, MAX_DECIMAL_TEXT_LEN
                        )));
                    }
                }
                String::from_utf8(bytes)
                    .map_err(|_| crate::tokenizer::DecodeError::Utf8Error)
            }
        }

        /// Vocabulary-constrained i32 encoder (zigzag; unchanged in decimal mode)
        impl crate::tokenizer::JsonTokenEncoder for i32 {
            fn encode(&self, tokens: &mut Vec<u32>) {
                let zigzag = if *self >= 0 {
                    (*self as u32) * 2
                } else {
                    let abs_val = self.wrapping_neg() as u32;
                    (abs_val * 2) - 1
                };
                encode_u32_constrained(zigzag, tokens);
            }

            fn decode(
                tokens: &mut impl Iterator<Item = u32>
            ) -> Result<Self, crate::tokenizer::DecodeError> {
                let first_token = tokens
                    .next()
                    .ok_or(crate::tokenizer::DecodeError::UnexpectedEndOfStream {
                        context: crate::tokenizer::EOFContext::new("i32", None, "zigzag-encoded integer")
                    })?;
                let zigzag = decode_u32_constrained_with_first(first_token, tokens)?;
                let value = if zigzag % 2 == 0 {
                    (zigzag / 2) as i32
                } else {
                    -(((zigzag + 1) / 2) as i32)
                };
                Ok(value)
            }

            fn token_count(&self) -> usize {
                let zigzag = if *self >= 0 {
                    (*self as u32) * 2
                } else {
                    let abs_val = self.wrapping_neg() as u32;
                    (abs_val * 2) - 1
                };
                if zigzag < MAX_VOCAB { 2 } else { 3 }
            }
        }

        /// Decimal digit-aligned i64 encoder
        impl crate::tokenizer::JsonTokenEncoder for i64 {
            fn encode(&self, tokens: &mut Vec<u32>) {
                decimal_number_codec::encode_text(&self.to_string(), tokens);
            }

            fn decode(
                tokens: &mut impl Iterator<Item = u32>
            ) -> Result<Self, crate::tokenizer::DecodeError> {
                let text = decimal_number_codec::decode_text(tokens, "i64", false)?;
                text.parse::<i64>().map_err(|_| {
                    crate::tokenizer::DecodeError::InvalidData(format!(
                        "invalid decimal i64 text {:?}", text
                    ))
                })
            }

            fn decode_strict(
                tokens: &mut impl Iterator<Item = u32>
            ) -> Result<Self, crate::tokenizer::DecodeError> {
                let text = decimal_number_codec::decode_text(tokens, "i64", true)?;
                text.parse::<i64>().map_err(|_| {
                    crate::tokenizer::DecodeError::InvalidData(format!(
                        "invalid decimal i64 text {:?}", text
                    ))
                })
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
        impl crate::tokenizer::JsonTokenEncoder for u64 {
            fn encode(&self, tokens: &mut Vec<u32>) {
                decimal_number_codec::encode_text(&self.to_string(), tokens);
            }

            fn decode(
                tokens: &mut impl Iterator<Item = u32>
            ) -> Result<Self, crate::tokenizer::DecodeError> {
                let text = decimal_number_codec::decode_text(tokens, "u64", false)?;
                text.parse::<u64>().map_err(|_| {
                    crate::tokenizer::DecodeError::InvalidData(format!(
                        "invalid decimal u64 text {:?}", text
                    ))
                })
            }

            fn decode_strict(
                tokens: &mut impl Iterator<Item = u32>
            ) -> Result<Self, crate::tokenizer::DecodeError> {
                let text = decimal_number_codec::decode_text(tokens, "u64", true)?;
                text.parse::<u64>().map_err(|_| {
                    crate::tokenizer::DecodeError::InvalidData(format!(
                        "invalid decimal u64 text {:?}", text
                    ))
                })
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
        /// parses back to the identical bits, so encode(text)+parse is an
        /// exact round trip for every finite value. NaN/Inf are not valid
        /// JSON numbers and are rejected on decode by the parse.
        impl crate::tokenizer::JsonTokenEncoder for f64 {
            fn encode(&self, tokens: &mut Vec<u32>) {
                decimal_number_codec::encode_text(&self.to_string(), tokens);
            }

            fn decode(
                tokens: &mut impl Iterator<Item = u32>
            ) -> Result<Self, crate::tokenizer::DecodeError> {
                let text = decimal_number_codec::decode_text(tokens, "f64", false)?;
                let value = text.parse::<f64>().map_err(|_| {
                    crate::tokenizer::DecodeError::InvalidData(format!(
                        "invalid decimal f64 text {:?}", text
                    ))
                })?;
                if !value.is_finite() {
                    return Err(crate::tokenizer::DecodeError::InvalidData(
                        "non-finite f64 is not a valid JSON number".to_string(),
                    ));
                }
                Ok(value)
            }

            fn decode_strict(
                tokens: &mut impl Iterator<Item = u32>
            ) -> Result<Self, crate::tokenizer::DecodeError> {
                let text = decimal_number_codec::decode_text(tokens, "f64", true)?;
                let value = text.parse::<f64>().map_err(|_| {
                    crate::tokenizer::DecodeError::InvalidData(format!(
                        "invalid decimal f64 text {:?}", text
                    ))
                })?;
                if !value.is_finite() {
                    return Err(crate::tokenizer::DecodeError::InvalidData(
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

/// Generate partial encoding support data structures.
///
/// These structures enable two encoding modes:
/// - Mode 1: Complete instances (existing encode/decode, zero overhead)
/// - Mode 2: Decode to first gap (decoder-only LM continuation)
///
/// Partial encoding produces a complete default-filled token sequence plus
/// gap metadata. It is not a live parser state for suffix-constrained infill.
pub(super) fn generate_partial_encoding_types() -> TokenStream {
    quote! {
        /// Information about a gap (field filled with default value)
        ///
        /// Tracks which tokens in a sequence correspond to fields that were
        /// missing from the original partial instance and filled with defaults.
        /// This is metadata over a complete default-filled sequence, not the
        /// parser state needed for suffix-constrained infill.
        #[derive(Debug, Clone, PartialEq, Eq)]
        pub struct GapInfo {
            /// Index of the field in the struct
            pub field_index: usize,
            /// Name of the field
            pub field_name: &'static str,
            /// Token range for this field's encoding
            pub token_range: std::ops::Range<usize>,
        }

        /// Partial encoding with gap tracking
        ///
        /// Contains a complete token sequence (with defaults filled in for missing fields)
        /// and metadata about which token ranges correspond to default values.
        /// It does not carry owner type, generated-order cursor, suffix
        /// constraints, or BPE boundary state.
        #[derive(Debug, Clone)]
        pub struct PartialEncoding {
            /// Complete token sequence (defaults filled)
            pub tokens: Vec<u32>,
            /// Information about which token ranges are defaults (gaps)
            pub gaps: Vec<GapInfo>,
        }

        /// Result of decoding to first gap (Mode 2)
        ///
        /// Reports the first default-filled field after decoding the complete
        /// PartialEncoding token stream. This is not suffix-constrained
        /// parser state and does not prove that a generated middle span can
        /// satisfy an arbitrary right suffix.
        #[derive(Debug, Clone)]
        pub struct PrefixResult<T> {
            /// Decoded instance (with defaults for fields after the gap)
            pub instance: T,
            /// Index of the first field that was a default
            pub stopped_at_field: usize,
            /// Number of tokens consumed during decoding (for surplus detection)
            pub tokens_consumed: usize,
            /// Total tokens available in the encoding
            pub total_tokens: usize,
        }

        /// Information about a partially decoded field
        ///
        /// When streaming decode encounters a truncated String or Vec field,
        /// this struct captures what was successfully decoded.
        #[derive(Debug, Clone, PartialEq)]
        pub struct PartialFieldInfo {
            /// Name of the field that was partially decoded
            pub field_name: &'static str,
            /// Type of the partial field ("String" or "Vec")
            pub field_type: &'static str,
            /// For strings: number of bytes decoded. For vecs: number of elements decoded.
            pub items_decoded: usize,
            /// For vecs: expected total length (if known)
            pub expected_length: Option<usize>,
        }

        /// Result of prefix decoding from truncated stream (streaming mode)
        ///
        /// Used for LLM autocompletion: decode as many fields as possible from
        /// a potentially truncated token stream, return partial struct with
        /// defaults for remaining fields.
        #[derive(Debug, Clone)]
        pub struct PrefixDecodeResult<T> {
            /// Decoded instance (with defaults for fields that couldn't be decoded)
            pub instance: T,
            /// Number of fields successfully decoded
            pub fields_decoded: usize,
            /// Whether all fields were successfully decoded
            pub complete: bool,
            /// Information about a partially decoded field (if any)
            ///
            /// This is Some when the last decoded field was truncated (e.g., String
            /// without END_MARKER, or Vec with fewer elements than expected).
            pub partial_field: Option<PartialFieldInfo>,
        }

        /// Variant presence tracking for enums
        ///
        /// Tracks which enum variant was chosen and which fields within that
        /// variant were present in the original partial JSON.
        #[derive(Debug, Clone)]
        pub struct VariantPresence {
            /// Index of the variant that was matched
            pub variant_index: usize,
            /// Presence tracking for fields within the variant
            /// For simple enums (unit variants), this is empty.
            /// For complex enums (data variants), this tracks field presence.
            pub field_presence: Vec<bool>,
        }
    }
}
