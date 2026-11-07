//! Generated variable-base integer helpers for standalone tokenizer output.
//!
//! These renderers emit the fixed-base encode, decode, and token-count helpers
//! used by generated primitive, collection, and JSON-value encoders while
//! preserving the recovery normalization behavior of the generic paths.

use proc_macro2::TokenStream;
use quote::quote;

pub(super) fn render_variable_base_helpers(
    encoding_base: u32,
    fixed_base_encode_chain: &TokenStream,
    fixed_base_token_count_chain: &TokenStream,
) -> TokenStream {
    quote! {
        /// Variable-base encoding helpers for multi-token primitives
        pub mod variable_base {
            use super::{DecodeError, EOFContext};

            pub const GENERATED_ENCODING_BASE: u32 = #encoding_base;

            /// Calculate maximum token width needed for a given base
            #[inline(always)]
            fn max_token_width_for_base(base: u32) -> u32 {
                let mut width = 1u32;
                let mut max_value = base as u64;
                while max_value <= u32::MAX as u64 {
                    width += 1;
                    max_value = max_value.saturating_mul(base as u64);
                }
                width
            }

            /// Encode u32 value using multi-token encoding with configurable base
            #[inline(always)]
            pub fn encode_u32(value: u32, tokens: &mut Vec<u32>, base: u32) {
                let max_width = max_token_width_for_base(base);

                // Calculate number of digits needed
                let mut token_width = 1u32;
                let mut max_value = base;
                while value >= max_value && token_width < max_width {
                    token_width += 1;
                    max_value = max_value.saturating_mul(base);
                }

                // Push width indicator (0-3 for 1-4 digits)
                tokens.push(token_width - 1);

                // Push digits in big-endian order (most significant first)
                for i in (0..token_width).rev() {
                    let digit = (value / base.pow(i)) % base;
                    tokens.push(digit);
                }
            }

            /// Decode u32 value from multi-token encoding with configurable base
            #[inline(always)]
            pub fn decode_u32(
                tokens: &mut impl Iterator<Item = u32>,
                context_type: &'static str,
                base: u32,
            ) -> Result<u32, DecodeError> {
                // Read width indicator
                let width_indicator = tokens.next().ok_or(DecodeError::UnexpectedEndOfStream {
                    context: EOFContext::new(context_type, Some("width"), "width indicator"),
                })?;
                let token_width = (width_indicator % 4) + 1; // 0-3 maps to 1-4 digits

                // Read digits and reconstruct value
                let mut value = 0u32;
                for _ in 0..token_width {
                    let digit = tokens.next().ok_or(DecodeError::UnexpectedEndOfStream {
                        context: EOFContext::new(context_type, Some("digit"), "multi-token digit"),
                    })?;
                    value = value.saturating_mul(base).saturating_add(digit % base);
                }

                Ok(value)
            }

            #[inline(always)]
            fn decode_fixed_base_digit(
                tokens: &mut impl Iterator<Item = u32>,
                context_type: &'static str,
            ) -> Result<u32, DecodeError> {
                let digit = tokens.next().ok_or(DecodeError::UnexpectedEndOfStream {
                    context: EOFContext::new(context_type, Some("digit"), "multi-token digit"),
                })?;
                Ok(digit % GENERATED_ENCODING_BASE)
            }

            /// Calculate token count for encoding a u32 value with configurable base
            #[inline(always)]
            pub fn token_count_u32(value: u32, base: u32) -> usize {
                let max_width = max_token_width_for_base(base);

                let mut token_width = 1u32;
                let mut max_value = base;
                while value >= max_value && token_width < max_width {
                    token_width += 1;
                    max_value = max_value.saturating_mul(base);
                }
                (token_width + 1) as usize // +1 for width indicator
            }

            /// Encode u32 with the generated tokenizer's fixed base.
            #[inline(always)]
            pub fn encode_u32_fixed_base(value: u32, tokens: &mut Vec<u32>) {
                #fixed_base_encode_chain
            }

            /// Decode u32 with the generated tokenizer's fixed base.
            ///
            /// This preserves the generic decoder's recovery behavior: the
            /// width indicator is normalized modulo 4, digits are normalized
            /// modulo the generated base, and reconstruction uses saturating
            /// arithmetic.
            #[inline(always)]
            pub fn decode_u32_fixed_base(
                tokens: &mut impl Iterator<Item = u32>,
                context_type: &'static str,
            ) -> Result<u32, DecodeError> {
                let width_indicator = tokens.next().ok_or(DecodeError::UnexpectedEndOfStream {
                    context: EOFContext::new(context_type, Some("width"), "width indicator"),
                })?;
                let token_width = (width_indicator % 4) + 1;
                let mut value = 0u32;

                match token_width {
                    1 => {
                        let digit = decode_fixed_base_digit(tokens, context_type)?;
                        value = value
                            .saturating_mul(GENERATED_ENCODING_BASE)
                            .saturating_add(digit);
                    }
                    2 => {
                        let digit = decode_fixed_base_digit(tokens, context_type)?;
                        value = value
                            .saturating_mul(GENERATED_ENCODING_BASE)
                            .saturating_add(digit);
                        let digit = decode_fixed_base_digit(tokens, context_type)?;
                        value = value
                            .saturating_mul(GENERATED_ENCODING_BASE)
                            .saturating_add(digit);
                    }
                    3 => {
                        let digit = decode_fixed_base_digit(tokens, context_type)?;
                        value = value
                            .saturating_mul(GENERATED_ENCODING_BASE)
                            .saturating_add(digit);
                        let digit = decode_fixed_base_digit(tokens, context_type)?;
                        value = value
                            .saturating_mul(GENERATED_ENCODING_BASE)
                            .saturating_add(digit);
                        let digit = decode_fixed_base_digit(tokens, context_type)?;
                        value = value
                            .saturating_mul(GENERATED_ENCODING_BASE)
                            .saturating_add(digit);
                    }
                    _ => {
                        let digit = decode_fixed_base_digit(tokens, context_type)?;
                        value = value
                            .saturating_mul(GENERATED_ENCODING_BASE)
                            .saturating_add(digit);
                        let digit = decode_fixed_base_digit(tokens, context_type)?;
                        value = value
                            .saturating_mul(GENERATED_ENCODING_BASE)
                            .saturating_add(digit);
                        let digit = decode_fixed_base_digit(tokens, context_type)?;
                        value = value
                            .saturating_mul(GENERATED_ENCODING_BASE)
                            .saturating_add(digit);
                        let digit = decode_fixed_base_digit(tokens, context_type)?;
                        value = value
                            .saturating_mul(GENERATED_ENCODING_BASE)
                            .saturating_add(digit);
                    }
                }

                Ok(value)
            }

            #[inline(always)]
            fn decode_fixed_base_digit_strict(
                tokens: &mut impl Iterator<Item = u32>,
                context_type: &'static str,
            ) -> Result<u32, DecodeError> {
                let digit = tokens.next().ok_or(DecodeError::UnexpectedEndOfStream {
                    context: EOFContext::new(context_type, Some("digit"), "multi-token digit"),
                })?;
                if digit < GENERATED_ENCODING_BASE {
                    Ok(digit)
                } else {
                    Err(DecodeError::InvalidTokenInContext {
                        token: digit,
                        context: EOFContext::new(
                            context_type,
                            Some("digit"),
                            "multi-token digit below GENERATED_ENCODING_BASE",
                        ),
                    })
                }
            }

            /// Decode u32 with the generated tokenizer's fixed base without
            /// modulo-normalizing the width indicator or digits.
            #[inline(always)]
            pub fn decode_u32_fixed_base_strict(
                tokens: &mut impl Iterator<Item = u32>,
                context_type: &'static str,
            ) -> Result<u32, DecodeError> {
                let width_indicator = tokens.next().ok_or(DecodeError::UnexpectedEndOfStream {
                    context: EOFContext::new(context_type, Some("width"), "width indicator"),
                })?;
                if width_indicator >= 4 {
                    return Err(DecodeError::InvalidTokenInContext {
                        token: width_indicator,
                        context: EOFContext::new(
                            context_type,
                            Some("width"),
                            "width indicator 0..=3",
                        ),
                    });
                }
                let token_width = width_indicator + 1;
                let mut value = 0u32;

                match token_width {
                    1 => {
                        let digit = decode_fixed_base_digit_strict(tokens, context_type)?;
                        value = value
                            .saturating_mul(GENERATED_ENCODING_BASE)
                            .saturating_add(digit);
                    }
                    2 => {
                        let digit = decode_fixed_base_digit_strict(tokens, context_type)?;
                        value = value
                            .saturating_mul(GENERATED_ENCODING_BASE)
                            .saturating_add(digit);
                        let digit = decode_fixed_base_digit_strict(tokens, context_type)?;
                        value = value
                            .saturating_mul(GENERATED_ENCODING_BASE)
                            .saturating_add(digit);
                    }
                    3 => {
                        let digit = decode_fixed_base_digit_strict(tokens, context_type)?;
                        value = value
                            .saturating_mul(GENERATED_ENCODING_BASE)
                            .saturating_add(digit);
                        let digit = decode_fixed_base_digit_strict(tokens, context_type)?;
                        value = value
                            .saturating_mul(GENERATED_ENCODING_BASE)
                            .saturating_add(digit);
                        let digit = decode_fixed_base_digit_strict(tokens, context_type)?;
                        value = value
                            .saturating_mul(GENERATED_ENCODING_BASE)
                            .saturating_add(digit);
                    }
                    _ => {
                        let digit = decode_fixed_base_digit_strict(tokens, context_type)?;
                        value = value
                            .saturating_mul(GENERATED_ENCODING_BASE)
                            .saturating_add(digit);
                        let digit = decode_fixed_base_digit_strict(tokens, context_type)?;
                        value = value
                            .saturating_mul(GENERATED_ENCODING_BASE)
                            .saturating_add(digit);
                        let digit = decode_fixed_base_digit_strict(tokens, context_type)?;
                        value = value
                            .saturating_mul(GENERATED_ENCODING_BASE)
                            .saturating_add(digit);
                        let digit = decode_fixed_base_digit_strict(tokens, context_type)?;
                        value = value
                            .saturating_mul(GENERATED_ENCODING_BASE)
                            .saturating_add(digit);
                    }
                }

                Ok(value)
            }

            /// Count u32 tokens with the generated tokenizer's fixed base.
            #[inline(always)]
            pub fn token_count_u32_fixed_base(value: u32) -> usize {
                #fixed_base_token_count_chain
            }

            /// Decode u32 value from multi-token encoding, returning None on EOF
            ///
            /// Used for streaming decode where partial results are acceptable
            #[inline(always)]
            pub fn decode_u32_option(
                tokens: &mut impl Iterator<Item = u32>,
                base: u32,
            ) -> Option<u32> {
                // Read width indicator
                let width_indicator = tokens.next()?;
                let token_width = (width_indicator % 4) + 1; // 0-3 maps to 1-4 digits

                // Read digits and reconstruct value
                let mut value = 0u32;
                for _ in 0..token_width {
                    let digit = tokens.next()?;
                    value = value.saturating_mul(base).saturating_add(digit % base);
                }

                Some(value)
            }
        }
    }
}
