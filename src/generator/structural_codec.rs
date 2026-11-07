//! Structural value codec generation
//!
//! Generates brutally efficient encode/decode functions for structural values
//! (enum variants, bounded integers, pattern indices) with compile-time
//! specialization and zero runtime overhead.
//!
//! # Core Principle
//!
//! Resolve EVERYTHING at compile time:
//! - No runtime branches (all decisions in generated code)
//! - No runtime loops (completely unrolled)
//! - Const propagation (base, divisors, shifts all const)
//! - Aggressive inlining (#[inline(always)])
//!
//! # Generated Code Quality
//!
//! For enum with 200 variants, base=10 (token_width=3):
//! ```rust,ignore
//! mod my_enum_codec {
//!     const BASE: u32 = 10;
//!     const TOKEN_WIDTH: usize = 3;
//!     const MAX_VALUE: u32 = 199;
//!
//!     #[inline(always)]
//!     pub fn encode(value: u32, tokens: &mut Vec<u32>) {
//!         tokens.push((value / 100) % 10);  // Digit 2
//!         tokens.push((value / 10) % 10);   // Digit 1
//!         tokens.push(value % 10);          // Digit 0
//!     }
//!
//!     #[inline(always)]
//!     pub fn decode(tokens: &mut impl Iterator<Item = u32>) -> Result<u32, DecodeError> {
//!         let d2 = tokens.next()? % 10;  // Defensive modulo
//!         let d1 = tokens.next()? % 10;
//!         let d0 = tokens.next()? % 10;
//!         Ok((d2 * 100 + d1 * 10 + d0) % 200)  // Final bounds check
//!     }
//! }
//! ```
//!
//! Assembly output: ~6 instructions per encode, no branches, no loops.

use crate::generator::encoding_decision::CodecSpec;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

/// Generate a complete codec module for a specific type
///
/// Creates a module with const values and #[inline(always)] encode/decode functions
/// specialized for the given codec specification.
///
/// # Arguments
/// * `type_name` - Name of the type (e.g., "MyEnum", "BoundedU32")
/// * `spec` - Codec specification with precomputed values
///
/// # Returns
/// TokenStream for complete codec module
///
/// # Example
/// ```rust,ignore
/// let spec = CodecSpec::calculate(199, Some(10)); // 200 variants, base-10
/// let codec = generate_codec_module("MyEnum", &spec);
/// // Generates: mod my_enum_codec { ... }
/// ```
pub fn generate_codec_module(type_name: &str, spec: &CodecSpec) -> TokenStream {
    let codec_mod_name = format_ident!("{}_codec", type_name.to_lowercase());

    // Generate const declarations
    let base = spec.base;
    let token_width = spec.token_width;
    let max_value = spec.max_value;

    let consts = quote! {
        #[allow(dead_code)]
        pub const BASE: u32 = #base;
        #[allow(dead_code)]
        pub const TOKEN_WIDTH: usize = #token_width;
        #[allow(dead_code)]
        pub const MAX_VALUE: u32 = #max_value;
    };

    // Generate encode function (specialized based on token_width and power-of-2)
    let encode_fn = generate_encode_function(spec);

    // Generate decode function (specialized, defensive modulo)
    let decode_fn = generate_decode_function(spec);

    // Generate strict decode function (same layout, no modulo recovery)
    let decode_strict_fn = generate_decode_strict_function(spec);

    // Generate decode_prefix function for streaming mode (returns Option<u32>)
    let decode_prefix_fn = generate_decode_prefix_function(spec);

    quote! {
        mod #codec_mod_name {
            use crate::tokenizer::DecodeError;

            #consts

            #encode_fn

            #decode_fn

            #decode_strict_fn

            #decode_prefix_fn
        }
    }
}

/// Generate specialized encode function
///
/// Generates completely unrolled encode code with no loops or branches.
/// Uses bit shifts for power-of-2 bases, division for others.
fn generate_encode_function(spec: &CodecSpec) -> TokenStream {
    match spec.token_width {
        1 => generate_encode_single_token(spec),
        2..=4 => generate_encode_multi_token_unrolled(spec),
        _ => generate_encode_multi_token_loop(spec), // Fallback for very large widths
    }
}

/// Generate single-token encode (most common case)
fn generate_encode_single_token(_spec: &CodecSpec) -> TokenStream {
    quote! {
        #[inline(always)]
        pub fn encode(value: u32, tokens: &mut Vec<u32>) {
            tokens.push(value);
        }
    }
}

/// Generate multi-token encode with complete unrolling (token_width 2-4)
fn generate_encode_multi_token_unrolled(spec: &CodecSpec) -> TokenStream {
    if spec.is_power_of_2 {
        generate_encode_power_of_2(spec)
    } else {
        generate_encode_arithmetic(spec)
    }
}

/// Generate power-of-2 optimized encode (bit shifts)
fn generate_encode_power_of_2(spec: &CodecSpec) -> TokenStream {
    let shift = spec.shift.expect("Power of 2 must have shift");
    let mask = spec.mask.expect("Power of 2 must have mask");

    // Generate unrolled bit-shift operations
    let push_ops = (0..spec.token_width).rev().map(|i| {
        let shift_amount = shift * i as u32;
        if shift_amount > 0 {
            quote! {
                tokens.push((value >> #shift_amount) & #mask);
            }
        } else {
            quote! {
                tokens.push(value & #mask);
            }
        }
    });

    quote! {
        #[inline(always)]
        pub fn encode(value: u32, tokens: &mut Vec<u32>) {
            #(#push_ops)*
        }
    }
}

/// Generate arithmetic encode (division and modulo)
fn generate_encode_arithmetic(spec: &CodecSpec) -> TokenStream {
    let base = spec.base;
    let divisors = spec.divisors.clone();

    // Generate unrolled division operations
    let push_ops = divisors.iter().map(|divisor| {
        if *divisor == 1 {
            quote! {
                tokens.push(value % #base);
            }
        } else {
            quote! {
                tokens.push((value / #divisor) % #base);
            }
        }
    });

    quote! {
        #[inline(always)]
        pub fn encode(value: u32, tokens: &mut Vec<u32>) {
            #(#push_ops)*
        }
    }
}

/// Generate multi-token encode with loop (fallback for large token_width)
fn generate_encode_multi_token_loop(spec: &CodecSpec) -> TokenStream {
    let base = spec.base;
    let token_width = spec.token_width;

    quote! {
        #[inline(always)]
        pub fn encode(value: u32, tokens: &mut Vec<u32>) {
            const BASE: u32 = #base;
            const WIDTH: usize = #token_width;

            let mut temp = value;
            let mut digits = [0u32; WIDTH];

            // Extract digits
            for i in 0..WIDTH {
                digits[WIDTH - 1 - i] = temp % BASE;
                temp /= BASE;
            }

            // Push all digits
            for digit in digits {
                tokens.push(digit);
            }
        }
    }
}

/// Generate specialized decode function with defensive modulo
///
/// Always applies modulo to handle LLM-generated invalid tokens gracefully.
fn generate_decode_function(spec: &CodecSpec) -> TokenStream {
    match spec.token_width {
        1 => generate_decode_single_token(spec),
        2..=4 => generate_decode_multi_token_unrolled(spec),
        _ => generate_decode_multi_token_loop(spec),
    }
}

/// Generate specialized strict decode function without modulo recovery.
fn generate_decode_strict_function(spec: &CodecSpec) -> TokenStream {
    match spec.token_width {
        1 => generate_decode_strict_single_token(spec),
        2..=4 => generate_decode_strict_multi_token_unrolled(spec),
        _ => generate_decode_strict_multi_token_loop(spec),
    }
}

/// Generate final bounds check for decode
///
/// Always use u64 arithmetic to avoid any overflow concerns.
/// When max_value = u32::MAX, LLVM optimizes `value % 2^32` to a no-op.
fn generate_final_bounds_check(max_value: u32) -> TokenStream {
    let modulo = max_value as u64 + 1;
    quote! { Ok((value as u64 % #modulo) as u32) }
}

/// Generate single-token decode
fn generate_decode_single_token(spec: &CodecSpec) -> TokenStream {
    let max_value = spec.max_value;

    let final_check = generate_final_bounds_check(max_value);
    quote! {
        #[inline(always)]
        pub fn decode(tokens: &mut impl Iterator<Item = u32>) -> Result<u32, DecodeError> {
            let value = tokens.next()
                .ok_or(DecodeError::UnexpectedEndOfStream {
                    context: crate::tokenizer::EOFContext::new("StructuralValue", None, "codec value")
                })?;
            #final_check
        }
    }
}

/// Generate single-token strict decode.
fn generate_decode_strict_single_token(spec: &CodecSpec) -> TokenStream {
    let max_value = spec.max_value;

    quote! {
        #[inline(always)]
        pub fn decode_strict(tokens: &mut impl Iterator<Item = u32>) -> Result<u32, DecodeError> {
            let value = tokens.next()
                .ok_or(DecodeError::UnexpectedEndOfStream {
                    context: crate::tokenizer::EOFContext::new("StructuralValue", None, "codec value")
                })?;
            if value <= #max_value {
                Ok(value)
            } else {
                Err(DecodeError::InvalidToken(value))
            }
        }
    }
}

/// Generate multi-token decode with complete unrolling
fn generate_decode_multi_token_unrolled(spec: &CodecSpec) -> TokenStream {
    // Always use arithmetic decode with modulo for consistency
    // Power-of-2 optimization (bitwise AND) is disabled by default
    generate_decode_arithmetic(spec)
}

/// Generate multi-token strict decode with complete unrolling.
fn generate_decode_strict_multi_token_unrolled(spec: &CodecSpec) -> TokenStream {
    generate_decode_strict_arithmetic(spec)
}

/// Generate arithmetic decode (multiplication and addition)
fn generate_decode_arithmetic(spec: &CodecSpec) -> TokenStream {
    let base = spec.base;
    let max_value = spec.max_value;
    let final_check = generate_final_bounds_check(max_value);

    let divisors = spec.divisors.clone();

    // Generate unrolled token reads with defensive modulo
    let token_reads: Vec<_> = (0..spec.token_width)
        .map(|i| {
            let digit_name = format_ident!("d{}", i);
            quote! {
                let #digit_name = tokens.next()
                    .ok_or(DecodeError::UnexpectedEndOfStream {
                        context: crate::tokenizer::EOFContext::new("StructuralValue", Some("digit"), "codec digit")
                    })? % #base;
            }
        })
        .collect();

    // Generate reconstruction with multiplication
    let reconstruction = divisors.iter().enumerate().map(|(i, divisor)| {
        let digit_name = format_ident!("d{}", i);
        quote! { ((#digit_name as u64) * (#divisor as u64)) }
    });

    quote! {
        #[inline(always)]
        pub fn decode(tokens: &mut impl Iterator<Item = u32>) -> Result<u32, DecodeError> {
            #(#token_reads)*

            let value = #(#reconstruction)+*;
            #final_check
        }
    }
}

/// Generate arithmetic strict decode (multiplication and addition).
fn generate_decode_strict_arithmetic(spec: &CodecSpec) -> TokenStream {
    let base = spec.base;
    let max_value = spec.max_value;

    let divisors = spec.divisors.clone();

    let token_reads: Vec<_> = (0..spec.token_width)
        .map(|i| {
            let digit_name = format_ident!("d{}", i);
            quote! {
                let #digit_name = tokens.next()
                    .ok_or(DecodeError::UnexpectedEndOfStream {
                        context: crate::tokenizer::EOFContext::new("StructuralValue", Some("digit"), "codec digit")
                    })?;
                if #digit_name >= #base {
                    return Err(DecodeError::InvalidToken(#digit_name));
                }
            }
        })
        .collect();

    let reconstruction = divisors.iter().enumerate().map(|(i, divisor)| {
        let digit_name = format_ident!("d{}", i);
        quote! { ((#digit_name as u64) * (#divisor as u64)) }
    });

    quote! {
        #[inline(always)]
        pub fn decode_strict(tokens: &mut impl Iterator<Item = u32>) -> Result<u32, DecodeError> {
            #(#token_reads)*

            let value = #(#reconstruction)+*;
            if value <= #max_value as u64 {
                Ok(value as u32)
            } else {
                Err(DecodeError::InvalidData(format!(
                    "codec value {} exceeds maximum {}",
                    value,
                    #max_value
                )))
            }
        }
    }
}

/// Generate multi-token decode with loop (fallback for large token_width)
fn generate_decode_multi_token_loop(spec: &CodecSpec) -> TokenStream {
    let base = spec.base;
    let token_width = spec.token_width;
    let max_value = spec.max_value;
    let final_check = generate_final_bounds_check(max_value);

    quote! {
        #[inline(always)]
        pub fn decode(tokens: &mut impl Iterator<Item = u32>) -> Result<u32, DecodeError> {
            const BASE: u32 = #base;
            const WIDTH: usize = #token_width;

            let mut value = 0u64;
            for _ in 0..WIDTH {
                let digit = tokens.next()
                    .ok_or(DecodeError::UnexpectedEndOfStream {
                        context: crate::tokenizer::EOFContext::new("StructuralValue", Some("digit"), "codec digit")
                    })? % BASE;
                value = value.saturating_mul(BASE as u64).saturating_add(digit as u64);
            }

            #final_check
        }
    }
}

/// Generate multi-token strict decode with loop fallback.
fn generate_decode_strict_multi_token_loop(spec: &CodecSpec) -> TokenStream {
    let base = spec.base;
    let token_width = spec.token_width;
    let max_value = spec.max_value;

    quote! {
        #[inline(always)]
        pub fn decode_strict(tokens: &mut impl Iterator<Item = u32>) -> Result<u32, DecodeError> {
            const BASE: u32 = #base;
            const WIDTH: usize = #token_width;

            let mut value = 0u64;
            for _ in 0..WIDTH {
                let digit = tokens.next()
                    .ok_or(DecodeError::UnexpectedEndOfStream {
                        context: crate::tokenizer::EOFContext::new("StructuralValue", Some("digit"), "codec digit")
                    })?;
                if digit >= BASE {
                    return Err(DecodeError::InvalidToken(digit));
                }
                value = value.saturating_mul(BASE as u64).saturating_add(digit as u64);
            }

            if value <= #max_value as u64 {
                Ok(value as u32)
            } else {
                Err(DecodeError::InvalidData(format!(
                    "codec value {} exceeds maximum {}",
                    value,
                    #max_value
                )))
            }
        }
    }
}

/// Generate decode_prefix function for streaming mode
///
/// Returns Option<u32> instead of Result - None if not enough tokens,
/// Some(value) if successful. Used by enum decode_prefix to handle
/// truncated token streams gracefully.
fn generate_decode_prefix_function(spec: &CodecSpec) -> TokenStream {
    match spec.token_width {
        1 => generate_decode_prefix_single_token(spec),
        2..=4 => generate_decode_prefix_multi_token(spec),
        _ => generate_decode_prefix_multi_token_loop(spec),
    }
}

/// Generate single-token decode_prefix
fn generate_decode_prefix_single_token(spec: &CodecSpec) -> TokenStream {
    let max_value = spec.max_value;

    if max_value > 255 {
        // Use multi-token for large values
        generate_decode_prefix_multi_token(spec)
    } else {
        // Simple single-token: just return the modulo'd value
        let modulo = max_value as u64 + 1;
        quote! {
            /// Streaming decode that returns None on EOF instead of error
            #[inline(always)]
            #[allow(dead_code)]
            pub fn decode_prefix(tokens: &mut impl Iterator<Item = u32>) -> Option<u32> {
                let value = tokens.next()?;
                Some((value as u64 % #modulo) as u32)
            }
        }
    }
}

/// Generate multi-token decode_prefix with unrolling
fn generate_decode_prefix_multi_token(spec: &CodecSpec) -> TokenStream {
    let base = spec.base;
    let max_value = spec.max_value;
    let modulo = max_value as u64 + 1;

    let divisors = spec.divisors.clone();

    // Generate unrolled token reads with Option chaining
    let token_reads: Vec<_> = (0..spec.token_width)
        .map(|i| {
            let digit_name = format_ident!("d{}", i);
            quote! {
                let #digit_name = tokens.next()? % #base;
            }
        })
        .collect();

    // Generate reconstruction with multiplication
    let reconstruction = divisors.iter().enumerate().map(|(i, divisor)| {
        let digit_name = format_ident!("d{}", i);
        quote! { ((#digit_name as u64) * (#divisor as u64)) }
    });

    quote! {
        /// Streaming decode that returns None on EOF instead of error
        #[inline(always)]
        #[allow(dead_code)]
        pub fn decode_prefix(tokens: &mut impl Iterator<Item = u32>) -> Option<u32> {
            #(#token_reads)*

            let value = #(#reconstruction)+*;
            Some((value as u64 % #modulo) as u32)
        }
    }
}

/// Generate multi-token decode_prefix with loop (fallback)
fn generate_decode_prefix_multi_token_loop(spec: &CodecSpec) -> TokenStream {
    let base = spec.base;
    let token_width = spec.token_width;
    let max_value = spec.max_value;
    let modulo = max_value as u64 + 1;

    quote! {
        /// Streaming decode that returns None on EOF instead of error
        #[inline(always)]
        #[allow(dead_code)]
        pub fn decode_prefix(tokens: &mut impl Iterator<Item = u32>) -> Option<u32> {
            const BASE: u32 = #base;
            const WIDTH: usize = #token_width;

            let mut value = 0u64;
            for _ in 0..WIDTH {
                let digit = tokens.next()? % BASE;
                value = value.saturating_mul(BASE as u64).saturating_add(digit as u64);
            }

            Some((value as u64 % #modulo) as u32)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_generated_file(code: &str) -> syn::File {
        syn::parse_file(code).unwrap_or_else(|error| panic!("generated Rust should parse: {error}"))
    }

    fn generated_module_has_public_function(
        file: &syn::File,
        module_name: &str,
        name: &str,
    ) -> bool {
        file.items.iter().any(|item| match item {
            syn::Item::Mod(item_mod) if item_mod.ident == module_name => {
                item_mod.content.as_ref().is_some_and(|(_, items)| {
                    items.iter().any(|item| {
                        matches!(
                            item,
                            syn::Item::Fn(function)
                                if function.sig.ident == name
                                    && matches!(function.vis, syn::Visibility::Public(_))
                        )
                    })
                })
            }
            _ => false,
        })
    }

    fn generated_public_function_has_top_level_for_loop(
        file: &syn::File,
        module_name: &str,
        name: &str,
    ) -> bool {
        file.items.iter().any(|item| match item {
            syn::Item::Mod(item_mod) if item_mod.ident == module_name => {
                item_mod.content.as_ref().is_some_and(|(_, items)| {
                    items.iter().any(|item| match item {
                        syn::Item::Fn(function) if function.sig.ident == name => {
                            function.block.stmts.iter().any(|stmt| {
                                matches!(stmt, syn::Stmt::Expr(syn::Expr::ForLoop(_), _))
                            })
                        }
                        _ => false,
                    })
                })
            }
            _ => false,
        })
    }

    #[test]
    fn test_generate_single_token_codec() {
        let spec = CodecSpec::calculate(5, None);
        let codec = generate_codec_module("TestEnum", &spec);
        let code = codec.to_string();

        // Should generate single-token encode
        assert!(code.contains("push (value)") || code.contains("push(value)"));
        assert!(code.contains("TOKEN_WIDTH") && code.contains("1usize"));
    }

    #[test]
    fn test_generate_multi_token_codec() {
        let spec = CodecSpec::calculate(199, Some(10)); // 200 variants, base-10, width=3
        let codec = generate_codec_module("LargeEnum", &spec);
        let code = codec.to_string();

        // Should generate unrolled arithmetic
        assert!(code.contains("TOKEN_WIDTH") && code.contains("3usize"));
        assert!(code.contains("BASE") && code.contains("10u32"));
        assert!(code.contains("/ 100"));
        assert!(code.contains("/ 10"));
        assert!(code.contains("% 10"));
    }

    #[test]
    fn test_generate_power_of_2_codec() {
        let spec = CodecSpec::calculate(999, Some(256)); // base-256, width=2
        let codec = generate_codec_module("PowerEnum", &spec);
        let code = codec.to_string();

        // Should use bit shifts, not division
        assert!(code.contains(">>") || code.contains("<<"));
        assert!(code.contains("&"));
        assert!(!code.contains("/ 256")); // Should NOT use division
    }

    #[test]
    fn test_defensive_decode() {
        let spec = CodecSpec::calculate(99, Some(10));
        let codec = generate_codec_module("DefensiveEnum", &spec);
        let code = codec.to_string();

        // Should have defensive modulo in decode
        assert!(code.contains("% 10")); // Per-digit modulo
        assert!(code.contains("% (99") || code.contains("% 100")); // Final bounds check
    }

    // ========== Loop-based Encode/Decode Tests (token_width > 4) ==========

    #[test]
    fn test_generate_encode_multi_token_loop_width_5() {
        // Token width = 5 triggers loop-based encode
        let spec = CodecSpec::calculate(99999, Some(10)); // width=5
        let codec = generate_codec_module("LargeWidth", &spec);
        let code = codec.to_string();

        // Should use loop-based encoding
        assert!(code.contains("for i in 0 .. WIDTH") || code.contains("for _ in 0 .. WIDTH"));
        assert!(code.contains("digits"));
        assert!(code.contains("temp"));
    }

    #[test]
    fn test_generate_decode_multi_token_loop_width_5() {
        // Token width = 5 triggers loop-based decode
        let spec = CodecSpec::calculate(99999, Some(10)); // width=5
        let codec = generate_codec_module("LargeWidth", &spec);
        let code = codec.to_string();

        // Should use loop-based decoding with saturating arithmetic
        assert!(code.contains("saturating_mul") || code.contains("saturating_add"));
        assert!(code.contains("let mut value"));
    }

    #[test]
    fn test_generate_encode_multi_token_loop_width_6() {
        // Even larger width to ensure loop path
        let spec = CodecSpec::calculate(999999, Some(10)); // width=6
        let codec = generate_codec_module("VeryLargeWidth", &spec);
        let code = codec.to_string();

        assert!(code.contains("WIDTH") && code.contains("6usize"));
        assert!(code.contains("digits"));
    }

    // ========== Token Width Variations ==========

    #[test]
    fn test_generate_codec_token_width_2() {
        let spec = CodecSpec::calculate(99, Some(10)); // width=2
        let codec = generate_codec_module("Width2", &spec);
        let code = codec.to_string();

        assert!(code.contains("TOKEN_WIDTH") && code.contains("2usize"));
        assert!(code.contains("/ 10"));
        assert!(code.contains("% 10"));
    }

    #[test]
    fn test_generate_codec_token_width_3() {
        let spec = CodecSpec::calculate(999, Some(10)); // width=3
        let codec = generate_codec_module("Width3", &spec);
        let code = codec.to_string();

        assert!(code.contains("TOKEN_WIDTH") && code.contains("3usize"));
        assert!(code.contains("/ 100"));
        assert!(code.contains("/ 10"));
        assert!(code.contains("% 10"));
    }

    #[test]
    fn test_generate_codec_token_width_4() {
        let spec = CodecSpec::calculate(9999, Some(10)); // width=4 (max unrolled)
        let codec = generate_codec_module("Width4", &spec);
        let code = codec.to_string();

        assert!(code.contains("TOKEN_WIDTH") && code.contains("4usize"));
        assert!(code.contains("/ 1000"));
        assert!(code.contains("/ 100"));
        assert!(code.contains("/ 10"));
        let file = parse_generated_file(&code);
        for function_name in ["encode", "decode", "decode_strict", "decode_prefix"] {
            assert!(
                !generated_public_function_has_top_level_for_loop(
                    &file,
                    "width4_codec",
                    function_name,
                ),
                "{function_name} should stay unrolled for width-4 codecs"
            );
        }
    }

    // ========== Power-of-2 Variations ==========

    #[test]
    fn test_generate_power_of_2_codec_width_2() {
        let spec = CodecSpec::calculate(255, Some(16)); // base-16, width=2
        let codec = generate_codec_module("Pow2Width2", &spec);
        let code = codec.to_string();

        // Should use bit shifts
        assert!(code.contains(">>") || code.contains("<<"));
        assert!(code.contains("&"));
        assert!(code.contains("TOKEN_WIDTH") && code.contains("2usize"));
    }

    #[test]
    fn test_generate_power_of_2_codec_width_3() {
        let spec = CodecSpec::calculate(4095, Some(16)); // base-16, width=3
        let codec = generate_codec_module("Pow2Width3", &spec);
        let code = codec.to_string();

        assert!(code.contains(">>") || code.contains("<<"));
        assert!(code.contains("TOKEN_WIDTH") && code.contains("3usize"));
    }

    #[test]
    fn test_generate_power_of_2_codec_base_256() {
        let spec = CodecSpec::calculate(65535, Some(256)); // base-256, width=2
        let codec = generate_codec_module("Base256", &spec);
        let code = codec.to_string();

        // Verify shift and mask values are correct
        assert!(code.contains(">>") || code.contains("<<"));
        assert!(code.contains("&"));
        assert!(code.contains("255")); // mask for base-256
    }

    #[test]
    fn test_generate_power_of_2_codec_base_512_preserves_effective_base() {
        let spec = CodecSpec::calculate(70000, Some(512)); // base-512, width=2
        let codec = generate_codec_module("Base512", &spec);
        let code = codec.to_string();

        assert!(code.contains("BASE") && code.contains("512u32"));
        assert!(code.contains("TOKEN_WIDTH") && code.contains("2usize"));
        assert!(code.contains(">> 9u32") || code.contains(">> 9"));
        assert!(code.contains("& 511u32") || code.contains("& 511"));
        assert!(
            !code.contains("255u32"),
            "structural codec must not silently cap a base-512 spec to base 256"
        );
    }

    // ========== Edge Cases ==========

    #[test]
    fn test_generate_codec_module_name_formatting() {
        let spec = CodecSpec::calculate(10, None);
        let codec = generate_codec_module("MySpecialEnum", &spec);
        let code = codec.to_string();

        // Module name should be lowercase
        assert!(code.contains("mod myspecialenum_codec"));
    }

    #[test]
    fn test_generate_single_token_codec_max_value() {
        let spec = CodecSpec::calculate(10, None);
        let codec = generate_codec_module("SmallEnum", &spec);
        let code = codec.to_string();

        // Verify max value constant
        assert!(code.contains("MAX_VALUE") && code.contains("10u32"));
        // Verify defensive modulo in decode
        assert!(code.contains("% (10") || code.contains("% 11"));
    }

    #[test]
    fn test_generate_encode_single_token_direct() {
        let spec = CodecSpec::calculate(3, None);
        let encode_fn = generate_encode_single_token(&spec);
        let code = encode_fn.to_string();

        assert!(code.contains("inline"));
        assert!(code.contains("push (value)") || code.contains("push(value)"));
    }

    #[test]
    fn test_generate_decode_single_token_direct() {
        let spec = CodecSpec::calculate(7, None);
        let decode_fn = generate_decode_single_token(&spec);
        let code = decode_fn.to_string();

        assert!(code.contains("inline"));
        assert!(code.contains("next"));
        assert!(code.contains("% (7") || code.contains("% 8"));
    }

    #[test]
    fn test_generate_encode_arithmetic_divisor_1() {
        // Test arithmetic encoding where last divisor is 1
        let spec = CodecSpec::calculate(99, Some(10)); // divisors: [10, 1]
        let encode_fn = generate_encode_arithmetic(&spec);
        let code = encode_fn.to_string();

        // Should have both division and modulo-only cases
        assert!(code.contains("/ 10"));
        assert!(code.contains("% 10"));
    }

    #[test]
    fn test_generate_decode_arithmetic_reconstruction() {
        let spec = CodecSpec::calculate(99, Some(10));
        let decode_fn = generate_decode_arithmetic(&spec);
        let code = decode_fn.to_string();

        // Should reconstruct with multiplication
        assert!(code.contains("d0"));
        assert!(code.contains("d1"));
        assert!(code.contains("*"));
        assert!(code.contains("% 10"));
    }

    #[test]
    fn test_generate_encode_power_of_2_shift_zero() {
        // Single-width power-of-2 (shift=0 for first token)
        let spec = CodecSpec::calculate(15, Some(16)); // width=1
        let codec = generate_codec_module("SinglePow2", &spec);
        let code = codec.to_string();

        // Single token encode should just push value
        assert!(code.contains("push (value)") || code.contains("push(value)"));
    }

    #[test]
    fn test_generate_encode_multi_token_loop_const_declarations() {
        let spec = CodecSpec::calculate(999999, Some(10)); // width=6
        let encode_fn = generate_encode_multi_token_loop(&spec);
        let code = encode_fn.to_string();

        // Should declare const BASE and WIDTH
        assert!(code.contains("const BASE"));
        assert!(code.contains("const WIDTH"));
        assert!(code.contains("10u32"));
        assert!(code.contains("6usize"));
    }

    #[test]
    fn test_generate_decode_multi_token_loop_saturating_ops() {
        let spec = CodecSpec::calculate(999999, Some(10)); // width=6
        let decode_fn = generate_decode_multi_token_loop(&spec);
        let code = decode_fn.to_string();

        // Should use saturating operations
        assert!(code.contains("saturating_mul"));
        assert!(code.contains("saturating_add"));
        assert!(code.contains("% (999999") || code.contains("% 1000000"));
    }

    #[test]
    fn test_generate_codec_module_complete_structure() {
        let spec = CodecSpec::calculate(199, Some(10));
        let codec = generate_codec_module("CompleteTest", &spec);
        let code = codec.to_string();
        let generated_file = parse_generated_file(&code);

        // Verify module structure
        assert!(code.contains("mod completetest_codec"));
        assert!(code.contains("use crate :: tokenizer :: DecodeError"));
        assert!(code.contains("pub const BASE"));
        assert!(code.contains("pub const TOKEN_WIDTH"));
        assert!(code.contains("pub const MAX_VALUE"));
        assert!(generated_module_has_public_function(
            &generated_file,
            "completetest_codec",
            "encode"
        ));
        assert!(generated_module_has_public_function(
            &generated_file,
            "completetest_codec",
            "decode"
        ));
        assert!(generated_module_has_public_function(
            &generated_file,
            "completetest_codec",
            "decode_strict"
        ));
    }

    #[test]
    fn test_generate_encode_function_dispatches_correctly() {
        // Width 1 -> single token
        let spec1 = CodecSpec::calculate(5, None);
        let fn1 = generate_encode_function(&spec1);
        assert!(
            fn1.to_string().contains("push (value)") || fn1.to_string().contains("push(value)")
        );

        // Width 3 -> multi-token unrolled
        let spec3 = CodecSpec::calculate(999, Some(10));
        let fn3 = generate_encode_function(&spec3);
        assert!(fn3.to_string().contains("/ 100"));

        // Width 5 -> multi-token loop
        let spec5 = CodecSpec::calculate(99999, Some(10));
        let fn5 = generate_encode_function(&spec5);
        assert!(fn5.to_string().contains("for"));
    }

    #[test]
    fn test_generate_decode_function_dispatches_correctly() {
        // Width 1 -> single token
        let spec1 = CodecSpec::calculate(5, None);
        let fn1 = generate_decode_function(&spec1);
        assert!(fn1.to_string().contains("next"));

        // Width 3 -> multi-token unrolled
        let spec3 = CodecSpec::calculate(999, Some(10));
        let fn3 = generate_decode_function(&spec3);
        assert!(fn3.to_string().contains("d0"));

        // Width 5 -> multi-token loop
        let spec5 = CodecSpec::calculate(99999, Some(10));
        let fn5 = generate_decode_function(&spec5);
        assert!(fn5.to_string().contains("saturating"));
    }
}
