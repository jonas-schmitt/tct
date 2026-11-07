//! Emits tokenizer implementations for primitive type aliases.
//!
//! This module owns the generated `JsonTokenEncoder` impls for schema-derived
//! newtype wrappers around primitive Rust values. It keeps O3 format shortcuts
//! lossless by tagging optimized representations and preserving a fallback to
//! ordinary string encoding when a value is not canonical for the selected
//! format.

use crate::generator::encoding_decision::{EncodingDecision, StringFormat};
use proc_macro2::{Ident, TokenStream};
use quote::quote;

pub(super) fn generate_encoder_impl(
    type_ident: &Ident,
    rust_type_tokens: &TokenStream,
    encoding_decision: Option<&EncodingDecision>,
) -> TokenStream {
    if let Some(decision) = encoding_decision {
        match decision {
            EncodingDecision::ConstantInteger { value } => {
                crate::log_debug!("   • O3: Constant integer optimization (value={})", value);
                quote! {
                    impl crate::tokenizer::JsonTokenEncoder for #type_ident {
                        fn encode(&self, _tokens: &mut Vec<u32>) {
                            // Constant integer: no encoding needed (0 tokens)
                            // Value is always #value
                        }

                        fn decode(_tokens: &mut impl Iterator<Item = u32>) -> Result<Self, crate::tokenizer::DecodeError> {
                            // Constant integer: always return the constant value
                            Ok(Self(#value))
                        }

                        fn decode_strict(_tokens: &mut impl Iterator<Item = u32>) -> Result<Self, crate::tokenizer::DecodeError> {
                            Ok(Self(#value))
                        }

                        fn token_count(&self) -> usize {
                            0  // Constant: 0 tokens
                        }
                    }
                }
            }
            EncodingDecision::FormatOptimized { format, .. } => {
                crate::log_debug!("   • O3: Format-optimized string ({:?})", format);
                generate_format_optimized_encoder(type_ident, rust_type_tokens, format)
            }
            _ => generate_standard_encoder(type_ident, rust_type_tokens),
        }
    } else {
        generate_standard_encoder(type_ident, rust_type_tokens)
    }
}

fn generate_format_optimized_encoder(
    type_ident: &Ident,
    rust_type_tokens: &TokenStream,
    format: &StringFormat,
) -> TokenStream {
    match format {
        StringFormat::Int32 => {
            // Tagged lossless fast path for canonical int32 strings.
            quote! {
                impl crate::tokenizer::JsonTokenEncoder for #type_ident {
                    #[inline(always)]
                    fn encode(&self, tokens: &mut Vec<u32>) {
                        if let Ok(parsed) = self.0.parse::<i32>() {
                            if parsed.to_string() == self.0 {
                                tokens.push(0);
                                parsed.encode(tokens);
                                return;
                            }
                        }
                        tokens.push(1);
                        self.0.encode(tokens);
                    }

                    fn decode(tokens: &mut impl Iterator<Item = u32>) -> Result<Self, crate::tokenizer::DecodeError> {
                        let mode = tokens.next().ok_or(crate::tokenizer::DecodeError::UnexpectedEndOfStream {
                            context: crate::tokenizer::EOFContext::new(stringify!(#type_ident), None, "format encoding tag")
                        })?;
                        match mode {
                            0 => {
                                let value = i32::decode(tokens)?;
                                Ok(Self(value.to_string()))
                            }
                            1 => {
                                let value = String::decode(tokens)?;
                                Ok(Self(value))
                            }
                            other => Err(crate::tokenizer::DecodeError::InvalidToken(other)),
                        }
                    }

                    fn decode_strict(tokens: &mut impl Iterator<Item = u32>) -> Result<Self, crate::tokenizer::DecodeError> {
                        let mode = tokens.next().ok_or(crate::tokenizer::DecodeError::UnexpectedEndOfStream {
                            context: crate::tokenizer::EOFContext::new(stringify!(#type_ident), None, "format encoding tag")
                        })?;
                        match mode {
                            0 => {
                                let value = i32::decode_strict(tokens)?;
                                Ok(Self(value.to_string()))
                            }
                            1 => {
                                let value = String::decode_strict(tokens)?;
                                Ok(Self(value))
                            }
                            other => Err(crate::tokenizer::DecodeError::InvalidToken(other)),
                        }
                    }

                    fn token_count(&self) -> usize {
                        if let Ok(parsed) = self.0.parse::<i32>() {
                            if parsed.to_string() == self.0 {
                                return 1 + parsed.token_count();
                            }
                        }
                        1 + self.0.token_count()
                    }
                }
            }
        }
        StringFormat::Int64 => {
            // Tagged lossless fast path for canonical int64 strings.
            quote! {
                impl crate::tokenizer::JsonTokenEncoder for #type_ident {
                    #[inline(always)]
                    fn encode(&self, tokens: &mut Vec<u32>) {
                        if let Ok(parsed) = self.0.parse::<i64>() {
                            if parsed.to_string() == self.0 {
                                tokens.push(0);
                                parsed.encode(tokens);
                                return;
                            }
                        }
                        tokens.push(1);
                        self.0.encode(tokens);
                    }

                    fn decode(tokens: &mut impl Iterator<Item = u32>) -> Result<Self, crate::tokenizer::DecodeError> {
                        let mode = tokens.next().ok_or(crate::tokenizer::DecodeError::UnexpectedEndOfStream {
                            context: crate::tokenizer::EOFContext::new(stringify!(#type_ident), None, "format encoding tag")
                        })?;
                        match mode {
                            0 => {
                                let value = i64::decode(tokens)?;
                                Ok(Self(value.to_string()))
                            }
                            1 => {
                                let value = String::decode(tokens)?;
                                Ok(Self(value))
                            }
                            other => Err(crate::tokenizer::DecodeError::InvalidToken(other)),
                        }
                    }

                    fn decode_strict(tokens: &mut impl Iterator<Item = u32>) -> Result<Self, crate::tokenizer::DecodeError> {
                        let mode = tokens.next().ok_or(crate::tokenizer::DecodeError::UnexpectedEndOfStream {
                            context: crate::tokenizer::EOFContext::new(stringify!(#type_ident), None, "format encoding tag")
                        })?;
                        match mode {
                            0 => {
                                let value = i64::decode_strict(tokens)?;
                                Ok(Self(value.to_string()))
                            }
                            1 => {
                                let value = String::decode_strict(tokens)?;
                                Ok(Self(value))
                            }
                            other => Err(crate::tokenizer::DecodeError::InvalidToken(other)),
                        }
                    }

                    fn token_count(&self) -> usize {
                        if let Ok(parsed) = self.0.parse::<i64>() {
                            if parsed.to_string() == self.0 {
                                return 1 + parsed.token_count();
                            }
                        }
                        1 + self.0.token_count()
                    }
                }
            }
        }
        StringFormat::Double | StringFormat::Float => {
            // Tagged lossless fast path for canonical f64 strings.
            quote! {
                impl crate::tokenizer::JsonTokenEncoder for #type_ident {
                    #[inline(always)]
                    fn encode(&self, tokens: &mut Vec<u32>) {
                        if let Ok(parsed) = self.0.parse::<f64>() {
                            if parsed.to_string() == self.0 {
                                tokens.push(0);
                                let bits = parsed.to_bits() as i64;
                                bits.encode(tokens);
                                return;
                            }
                        }
                        tokens.push(1);
                        self.0.encode(tokens);
                    }

                    fn decode(tokens: &mut impl Iterator<Item = u32>) -> Result<Self, crate::tokenizer::DecodeError> {
                        let mode = tokens.next().ok_or(crate::tokenizer::DecodeError::UnexpectedEndOfStream {
                            context: crate::tokenizer::EOFContext::new(stringify!(#type_ident), None, "format encoding tag")
                        })?;
                        match mode {
                            0 => {
                                let bits = i64::decode(tokens)?;
                                let value = f64::from_bits(bits as u64);
                                Ok(Self(value.to_string()))
                            }
                            1 => {
                                let value = String::decode(tokens)?;
                                Ok(Self(value))
                            }
                            other => Err(crate::tokenizer::DecodeError::InvalidToken(other)),
                        }
                    }

                    fn decode_strict(tokens: &mut impl Iterator<Item = u32>) -> Result<Self, crate::tokenizer::DecodeError> {
                        let mode = tokens.next().ok_or(crate::tokenizer::DecodeError::UnexpectedEndOfStream {
                            context: crate::tokenizer::EOFContext::new(stringify!(#type_ident), None, "format encoding tag")
                        })?;
                        match mode {
                            0 => {
                                let bits = i64::decode_strict(tokens)?;
                                let value = f64::from_bits(bits as u64);
                                Ok(Self(value.to_string()))
                            }
                            1 => {
                                let value = String::decode_strict(tokens)?;
                                Ok(Self(value))
                            }
                            other => Err(crate::tokenizer::DecodeError::InvalidToken(other)),
                        }
                    }

                    fn token_count(&self) -> usize {
                        if let Ok(parsed) = self.0.parse::<f64>() {
                            if parsed.to_string() == self.0 {
                                let bits = parsed.to_bits() as i64;
                                return 1 + bits.token_count();
                            }
                        }
                        1 + self.0.token_count()
                    }
                }
            }
        }
        StringFormat::Byte => {
            // Treat base64 strings as regular strings so vocabulary constraints and BPE still apply.
            quote! {
                impl crate::tokenizer::JsonTokenEncoder for #type_ident {
                    #[inline(always)]
                    fn encode(&self, tokens: &mut Vec<u32>) {
                        self.0.encode(tokens);
                    }

                    fn decode(tokens: &mut impl Iterator<Item = u32>) -> Result<Self, crate::tokenizer::DecodeError> {
                        let s = String::decode(tokens)?;
                        Ok(Self(s))
                    }

                    fn decode_strict(tokens: &mut impl Iterator<Item = u32>) -> Result<Self, crate::tokenizer::DecodeError> {
                        let s = String::decode_strict(tokens)?;
                        Ok(Self(s))
                    }

                    fn token_count(&self) -> usize {
                        self.0.token_count()
                    }
                }
            }
        }
        StringFormat::DateTime | StringFormat::Date | StringFormat::Time => {
            // Tagged lossless fast path for canonical RFC 3339 date-times. The
            // fast path stores a whole-second Unix timestamp, so it applies only
            // when the value has no sub-second part (fractional seconds, leap
            // seconds); every other value takes the string fallback.
            quote! {
                impl crate::tokenizer::JsonTokenEncoder for #type_ident {
                    #[inline(always)]
                    fn encode(&self, tokens: &mut Vec<u32>) {
                        use chrono::{DateTime, Utc};
                        if let Ok(parsed) = self.0.parse::<DateTime<Utc>>() {
                            let canonical = parsed.to_rfc3339();
                            if canonical == self.0 && parsed.timestamp_subsec_nanos() == 0 {
                                tokens.push(0);
                                let timestamp = parsed.timestamp();
                                timestamp.encode(tokens);
                                return;
                            }
                        }
                        tokens.push(1);
                        self.0.encode(tokens);
                    }

                    fn decode(tokens: &mut impl Iterator<Item = u32>) -> Result<Self, crate::tokenizer::DecodeError> {
                        use chrono::{TimeZone, Utc};
                        let mode = tokens.next().ok_or(crate::tokenizer::DecodeError::UnexpectedEndOfStream {
                            context: crate::tokenizer::EOFContext::new(stringify!(#type_ident), None, "format encoding tag")
                        })?;
                        match mode {
                            0 => {
                                let timestamp = i64::decode(tokens)?;
                                let dt = Utc.timestamp_opt(timestamp, 0)
                                    .single()
                                    .ok_or(crate::tokenizer::DecodeError::InvalidData(
                                        "Invalid Unix timestamp for date-time".to_string()
                                    ))?;
                                Ok(Self(dt.to_rfc3339()))
                            }
                            1 => {
                                let value = String::decode(tokens)?;
                                Ok(Self(value))
                            }
                            other => Err(crate::tokenizer::DecodeError::InvalidToken(other)),
                        }
                    }

                    fn decode_strict(tokens: &mut impl Iterator<Item = u32>) -> Result<Self, crate::tokenizer::DecodeError> {
                        use chrono::{TimeZone, Utc};
                        let mode = tokens.next().ok_or(crate::tokenizer::DecodeError::UnexpectedEndOfStream {
                            context: crate::tokenizer::EOFContext::new(stringify!(#type_ident), None, "format encoding tag")
                        })?;
                        match mode {
                            0 => {
                                let timestamp = i64::decode_strict(tokens)?;
                                let dt = Utc.timestamp_opt(timestamp, 0)
                                    .single()
                                    .ok_or(crate::tokenizer::DecodeError::InvalidData(
                                        "Invalid Unix timestamp for date-time".to_string()
                                    ))?;
                                Ok(Self(dt.to_rfc3339()))
                            }
                            1 => {
                                let value = String::decode_strict(tokens)?;
                                Ok(Self(value))
                            }
                            other => Err(crate::tokenizer::DecodeError::InvalidToken(other)),
                        }
                    }

                    fn token_count(&self) -> usize {
                        use chrono::{DateTime, Utc};
                        if let Ok(parsed) = self.0.parse::<DateTime<Utc>>() {
                            let canonical = parsed.to_rfc3339();
                            if canonical == self.0 && parsed.timestamp_subsec_nanos() == 0 {
                                let timestamp = parsed.timestamp();
                                return 1 + timestamp.token_count();
                            }
                        }
                        1 + self.0.token_count()
                    }
                }
            }
        }
        _ => {
            crate::log_debug!(
                "   • Format {:?} not yet optimized, using standard string encoding",
                format
            );
            generate_standard_encoder(type_ident, rust_type_tokens)
        }
    }
}

fn generate_standard_encoder(type_ident: &Ident, rust_type_tokens: &TokenStream) -> TokenStream {
    quote! {
        impl crate::tokenizer::JsonTokenEncoder for #type_ident {
            fn encode(&self, tokens: &mut Vec<u32>) {
                self.0.encode(tokens);
            }

            fn decode(tokens: &mut impl Iterator<Item = u32>) -> Result<Self, crate::tokenizer::DecodeError> {
                let value = <#rust_type_tokens>::decode(tokens)?;
                Ok(Self(value))
            }

            fn decode_strict(tokens: &mut impl Iterator<Item = u32>) -> Result<Self, crate::tokenizer::DecodeError> {
                let value = <#rust_type_tokens>::decode_strict(tokens)?;
                Ok(Self(value))
            }

            fn token_count(&self) -> usize {
                self.0.token_count()
            }
        }
    }
}
