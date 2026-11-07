//! Shared helper renderers for generated Rust type definitions.
//!
//! These helpers keep derive selection, member type rendering, and common type
//! predicates centralized so struct, enum, and alias renderers make consistent
//! decisions about serde derives, ordering traits, hashability, and generated
//! Rust field types.

use crate::TctResult;
use crate::generator::schema_discovery::MemberType;
use crate::generator::type_analysis::{TraitDerivability, TypeAnalysis};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

pub(super) fn generate_derive_traits(
    trait_derivability: Option<&TraitDerivability>,
    has_pattern_validators: bool,
    streaming_encode: bool,
) -> TokenStream {
    generate_derive_attribute(
        trait_derivability,
        has_pattern_validators,
        streaming_encode,
        true,
    )
}

pub(super) fn generate_derive_traits_without_default(
    trait_derivability: Option<&TraitDerivability>,
    has_pattern_validators: bool,
) -> TokenStream {
    generate_derive_attribute(trait_derivability, has_pattern_validators, false, true)
}

pub(super) fn generate_derive_traits_without_deserialize(
    trait_derivability: Option<&TraitDerivability>,
    has_pattern_validators: bool,
    streaming_encode: bool,
) -> TokenStream {
    generate_derive_attribute(
        trait_derivability,
        has_pattern_validators,
        streaming_encode,
        false,
    )
}

pub(super) fn generate_derive_traits_without_deserialize_and_default(
    trait_derivability: Option<&TraitDerivability>,
    has_pattern_validators: bool,
) -> TokenStream {
    generate_derive_attribute(trait_derivability, has_pattern_validators, false, false)
}

fn generate_derive_attribute(
    trait_derivability: Option<&TraitDerivability>,
    has_pattern_validators: bool,
    include_default: bool,
    include_deserialize: bool,
) -> TokenStream {
    // Keep the generated derive order stable for snapshot and substring tests.
    let mut traits = vec!["Debug", "PartialEq"];

    if let Some(derivability) = trait_derivability
        && derivability.can_derive_eq
    {
        traits.push("Eq");
    }

    traits.push("Clone");

    if let Some(derivability) = trait_derivability {
        if derivability.can_derive_ord {
            traits.push("PartialOrd");
            traits.push("Ord");
        }
        if derivability.can_derive_hash {
            traits.push("Hash");
        }
    }

    if include_default {
        traits.push("Default");
    }

    if !has_pattern_validators {
        traits.push("serde::Serialize");
        if include_deserialize {
            traits.push("serde::Deserialize");
        }
    }

    let trait_idents: Vec<_> = traits
        .iter()
        .map(|trait_name| parse_trait(trait_name))
        .collect();

    quote! {
        #[derive(#(#trait_idents),*)]
    }
}

fn parse_trait(trait_name: &str) -> TokenStream {
    if trait_name.contains("::") {
        match syn::parse_str::<syn::Path>(trait_name) {
            Ok(path) => quote! { #path },
            Err(error) => {
                crate::log_debug!("⚠️  Failed to parse trait path '{}': {}", trait_name, error);
                let ident = format_ident!("{}", trait_name.replace("::", "_"));
                quote! { #ident }
            }
        }
    } else {
        let ident = format_ident!("{}", trait_name);
        quote! { #ident }
    }
}

pub(super) fn generate_member_type(
    member_type: &MemberType,
    analysis: &TypeAnalysis,
) -> TctResult<TokenStream> {
    match member_type {
        MemberType::Primitive(rust_type) => Ok(parse_rust_type(rust_type)),

        MemberType::TypeReference(type_name) => {
            let type_ident = format_ident!("{}", type_name);
            if analysis.needs_boxing(type_name) {
                Ok(quote! { Box<#type_ident> })
            } else {
                Ok(quote! { #type_ident })
            }
        }

        MemberType::Container { container, inner } => {
            let container_ident = format_ident!("{}", container);
            let inner_type = parse_rust_type(inner);
            Ok(quote! { #container_ident<#inner_type> })
        }

        MemberType::ComplexContainer { container, inner } => {
            let container_ident = format_ident!("{}", container);
            let inner_ident = format_ident!("{}", inner);
            if analysis.needs_boxing(inner) {
                Ok(quote! { #container_ident<Box<#inner_ident>> })
            } else {
                Ok(quote! { #container_ident<#inner_ident> })
            }
        }

        MemberType::HashMap { key, value } => {
            let key_type = parse_rust_type(key);
            let value_type = parse_rust_type(value);
            Ok(quote! { rustc_hash::FxHashMap<#key_type, #value_type> })
        }

        MemberType::PatternProperties { .. } => unreachable!(
            "PatternProperties should be converted to individual TypeReference fields during schema walking"
        ),
    }
}

pub(super) fn parse_rust_type(rust_type: &str) -> TokenStream {
    match rust_type {
        "serde_json::Value" => quote! { serde_json::Value },
        "serde_json::Number" => quote! { serde_json::Number },
        "JsonInteger" => quote! { crate::tokenizer::JsonInteger },
        "crate::tokenizer::JsonInteger" => quote! { crate::tokenizer::JsonInteger },
        "String" => quote! { String },
        "i32" => quote! { i32 },
        "i64" => quote! { i64 },
        "f64" => quote! { f64 },
        "bool" => quote! { bool },
        _ => {
            if rust_type.contains('<') {
                let rust_type = qualify_generated_helper_types(rust_type);
                rust_type
                    .parse()
                    .unwrap_or_else(|_| quote! { serde_json::Value })
            } else {
                let type_ident = format_ident!("{}", rust_type);
                quote! { #type_ident }
            }
        }
    }
}

fn qualify_generated_helper_types(rust_type: &str) -> String {
    const JSON_INTEGER: &str = "JsonInteger";
    const QUALIFIED_JSON_INTEGER: &str = "crate::tokenizer::JsonInteger";

    let mut output = String::with_capacity(rust_type.len());
    let mut remaining = rust_type;

    while let Some(index) = remaining.find(JSON_INTEGER) {
        let (before, after_start) = remaining.split_at(index);
        output.push_str(before);

        let before_char = before.chars().last().or_else(|| output.chars().last());
        let after = &after_start[JSON_INTEGER.len()..];
        let after_char = after.chars().next();
        let is_already_qualified = output.ends_with("crate::tokenizer::");
        let has_identifier_boundary_before =
            before_char.is_none_or(|ch| !(ch == '_' || ch == ':' || ch.is_alphanumeric()));
        let has_identifier_boundary_after =
            after_char.is_none_or(|ch| !(ch == '_' || ch.is_alphanumeric()));

        if is_already_qualified || !has_identifier_boundary_before || !has_identifier_boundary_after
        {
            output.push_str(JSON_INTEGER);
        } else {
            output.push_str(QUALIFIED_JSON_INTEGER);
        }
        remaining = after;
    }

    output.push_str(remaining);
    output
}

pub(super) fn sanitize_field_name(name: &str) -> String {
    const RESERVED_IDENTIFIERS: &[&str] = &[
        "as", "break", "const", "continue", "crate", "else", "enum", "extern", "false", "fn",
        "for", "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub", "ref",
        "return", "self", "Self", "static", "struct", "super", "trait", "true", "type", "unsafe",
        "use", "where", "while", "async", "await", "dyn", "abstract", "become", "box", "do",
        "final", "macro", "override", "priv", "typeof", "unsized", "virtual", "yield", "try",
        // Generated JsonTokenEncoder methods use `tokens` for their iterator
        // or output buffer. A schema field with this wire name must not shadow
        // that parameter inside decode implementations.
        "tokens",
    ];

    match name {
        "." => return "dot".to_string(),
        "*" => return "star".to_string(),
        "^" => return "caret".to_string(),
        "^_" => return "caret_underscore".to_string(),
        _ => {}
    }

    let mut result = if let Some(stripped) = name.strip_prefix("$") {
        format!("dollar_{}", stripped)
    } else if let Some(stripped) = name.strip_prefix("+") {
        format!("plus_{}", stripped)
    } else if let Some(stripped) = name.strip_prefix("//") {
        format!("double_slash_{}", stripped)
    } else if let Some(stripped) = name.strip_prefix("/") {
        format!("slash_{}", stripped)
    } else if let Some(stripped) = name.strip_prefix("@") {
        format!("at_{}", stripped)
    } else if let Some(stripped) = name.strip_prefix("#") {
        format!("hash_{}", stripped)
    } else {
        name.to_string()
    };

    result = result
        .replace("-", "_")
        .replace(".", "_")
        .replace(" ", "_")
        .replace(":", "_")
        .replace(";", "_")
        .replace("!", "_")
        .replace("?", "_")
        .replace("(", "_")
        .replace(")", "_")
        .replace("[", "_")
        .replace("]", "_")
        .replace("{", "_")
        .replace("}", "_")
        .replace(",", "_")
        .replace("'", "_")
        .replace("\"", "_")
        .replace("/", "_")
        .replace("\\", "_")
        .replace("|", "_")
        .replace("*", "_")
        .replace("&", "_")
        .replace("^", "_")
        .replace("%", "_")
        .replace("#", "_")
        .replace("@", "_")
        .replace("$", "_")
        .replace("+", "plus")
        .replace("=", "eq")
        .replace("<", "lt")
        .replace(">", "gt")
        .replace("~", "_");

    if !result.is_empty()
        && let Some(first_char) = result.chars().next()
        && !first_char.is_alphabetic()
        && first_char != '_'
    {
        result = format!("field_{}", result);
    }

    if result.is_empty() {
        result = "field".to_string();
    }

    result = force_valid_identifier(&result.to_lowercase());

    if RESERVED_IDENTIFIERS.contains(&result.as_str()) {
        format!("{}_", result)
    } else {
        result
    }
}

/// Whether `text` is, in full, one Rust identifier.
///
/// The parser skips Unicode whitespace around a token, so the parsed identifier
/// has to be compared with the whole input; otherwise `"a\u{a0}"` would pass.
fn is_identifier_text(text: &str) -> bool {
    syn::parse_str::<syn::Ident>(text).is_ok_and(|ident| ident == text)
}

/// Whether `c` may start a Rust identifier (`XID_Start`, or `_`).
fn is_identifier_start(c: char) -> bool {
    c == '_' || c.is_ascii_alphabetic() || (!c.is_ascii() && is_identifier_text(&format!("{c}a")))
}

/// Whether `c` may continue a Rust identifier (`XID_Continue`).
fn is_identifier_continue(c: char) -> bool {
    c == '_' || c.is_ascii_alphanumeric() || (!c.is_ascii() && is_identifier_text(&format!("a{c}")))
}

/// Turn a sanitized property name into one that `format_ident!` accepts.
///
/// Property names are arbitrary JSON strings, and the punctuation replacements
/// above do not cover control characters, whitespace such as tab or newline,
/// symbols, or emoji; those reached `format_ident!` unchanged and panicked the
/// compiler. Characters that cannot appear in an identifier become `_`, a name
/// that cannot start one gets a `field_` prefix, and a lone `_`, which is not an
/// identifier, becomes `underscore`. Names that were already valid identifiers
/// are returned unchanged.
fn force_valid_identifier(name: &str) -> String {
    let mut cleaned: String = name
        .chars()
        .map(|c| if is_identifier_continue(c) { c } else { '_' })
        .collect();
    if !cleaned.chars().next().is_some_and(is_identifier_start) {
        cleaned.insert_str(0, "field_");
    }
    if cleaned == "_" {
        cleaned = "underscore".to_string();
    }
    cleaned
}
