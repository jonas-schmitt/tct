//! Shared enum generation helpers.
//!
//! This module owns enum encoding-mode selection, primitive/container type
//! token rendering, and default-value snippets that are reused by enum encode,
//! decode, streaming, and partial-encoding renderers.

use crate::TctResult;
use crate::generator::encoding_decision::EncodingDecision;
use crate::generator::schema_discovery::MemberType;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

/// Enum encoding mode - determines how enum variants are encoded to tokens.
#[derive(Debug, Clone)]
pub(super) enum EnumEncodingMode {
    Standard,
    Codec(crate::generator::encoding_decision::CodecSpec),
    Constant,
}

pub(super) fn container_inner_type_tokens(inner: &str) -> TokenStream {
    match inner {
        "serde_json::Value" => quote! { serde_json::Value },
        "serde_json::Number" => quote! { serde_json::Number },
        "JsonInteger" | "crate::tokenizer::JsonInteger" => {
            quote! { crate::tokenizer::JsonInteger }
        }
        "String" => quote! { String },
        "i32" => quote! { i32 },
        "i64" => quote! { i64 },
        "f64" => quote! { f64 },
        "bool" => quote! { bool },
        other => {
            let inner_ident = format_ident!("{}", other);
            quote! { #inner_ident }
        }
    }
}

pub(super) fn default_value_for_member_type(
    member_type: &MemberType,
    member_type_fn: &dyn Fn(&MemberType) -> TctResult<TokenStream>,
) -> TctResult<TokenStream> {
    if let MemberType::Primitive(primitive_type) = member_type {
        return Ok(match primitive_type.as_str() {
            "serde_json::Number" => quote! { serde_json::Number::from(0) },
            "serde_json::Value" => quote! { serde_json::Value::Null },
            "JsonInteger" | "crate::tokenizer::JsonInteger" => {
                quote! { crate::tokenizer::JsonInteger::default() }
            }
            _ => {
                let member_type = member_type_fn(member_type)?;
                quote! { <#member_type as Default>::default() }
            }
        });
    }

    let member_type = member_type_fn(member_type)?;
    Ok(quote! { <#member_type as Default>::default() })
}

/// Helper function to determine encoding mode from encoding decision.
pub(super) fn determine_encoding_mode(
    type_name: &str,
    encoding_decision: Option<&EncodingDecision>,
) -> EnumEncodingMode {
    if let Some(decision) = encoding_decision {
        match decision {
            EncodingDecision::EnumCodec { codec_spec, .. } => {
                crate::log_debug!(
                    "🚀 Using codec-based encoding for enum '{}' with {}-token codec (base={}, LOSSLESS)",
                    type_name,
                    codec_spec.token_width,
                    codec_spec.base
                );
                EnumEncodingMode::Codec(codec_spec.clone())
            }
            EncodingDecision::EnumIndex { .. } => {
                crate::log_debug!("⚡ Using index-based encoding for enum '{}'", type_name);
                EnumEncodingMode::Standard
            }
            EncodingDecision::SingleVariantEnum { variant } => {
                crate::log_debug!(
                    "✨ O3: Single-variant enum '{}' - constant value '{}' (0 tokens)",
                    type_name,
                    variant
                );
                EnumEncodingMode::Constant
            }
            EncodingDecision::Standard => {
                crate::log_debug!("📝 Using standard encoding for enum '{}'", type_name);
                EnumEncodingMode::Standard
            }
            _ => EnumEncodingMode::Standard,
        }
    } else {
        crate::log_debug!(
            "📝 No encoding decision for enum '{}', using standard encoding",
            type_name
        );
        EnumEncodingMode::Standard
    }
}
