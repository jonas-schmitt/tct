//! Type-dispatching `Deserialize` for enums discovered from a `type` array.
//!
//! Each variant of such an enum holds the values of one JSON type. The
//! generated impl reads the JSON type of the value and converts it into the
//! variant of that type, so a value never lands in the variant of another
//! type. A derived untagged enum tries every variant in turn instead, which
//! lets a generated struct variant take a JSON array positionally and an
//! untyped array variant take any value.

use crate::TctResult;
use crate::generator::schema_discovery::{MemberInfo, MemberType, TypeDiscovery};
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use serde_json::Value;

/// The JSON type of each variant, when `type_discovery` is an enum discovered
/// from a `type` array: the variant schema's single `type`, or `object` for
/// the variant that references a definition next to a `type` array.
pub(super) fn type_array_variant_json_types(
    type_discovery: &TypeDiscovery,
) -> Option<Vec<&'static str>> {
    let schema = &type_discovery.schema_object;
    if schema.contains_key("oneOf") || schema.contains_key("anyOf") {
        return None;
    }
    schema.get("type").and_then(Value::as_array)?;
    if type_discovery.members.is_empty() {
        return None;
    }
    type_discovery
        .members
        .iter()
        .map(|member| match member.schema.get("type") {
            Some(Value::String(type_name)) => match type_name.as_str() {
                "null" => Some("null"),
                "boolean" => Some("boolean"),
                "string" => Some("string"),
                "integer" => Some("integer"),
                "number" => Some("number"),
                "array" => Some("array"),
                "object" => Some("object"),
                _ => None,
            },
            Some(_) => None,
            None => member.schema.contains_key("$ref").then_some("object"),
        })
        .collect()
}

/// Generate the `Deserialize` impl: the first variant whose JSON type is the
/// type of the value converts it, and a value of an unlisted type is an error.
pub(super) fn generate_type_array_deserialize_impl(
    type_ident: &Ident,
    members: &[MemberInfo],
    variant_json_types: &[&str],
    member_type_fn: &dyn Fn(&MemberType) -> TctResult<TokenStream>,
) -> TctResult<TokenStream> {
    let branches = members
        .iter()
        .zip(variant_json_types)
        .enumerate()
        .map(|(index, (member, json_type))| {
            let variant_name = format_ident!("Variant{}", index);
            let has_json_type = match *json_type {
                "null" => quote! { value.is_null() },
                "boolean" => quote! { value.is_boolean() },
                "string" => quote! { value.is_string() },
                "integer" => quote! { value.is_i64() || value.is_u64() },
                "number" => quote! { value.is_number() },
                "array" => quote! { value.is_array() },
                _ => quote! { value.is_object() },
            };
            if matches!(&member.member_type, MemberType::Primitive(t) if t == "Null") {
                return Ok(quote! {
                    if #has_json_type {
                        return Ok(Self::#variant_name);
                    }
                });
            }
            let variant_type = member_type_fn(&member.member_type)?;
            Ok(quote! {
                if #has_json_type {
                    return serde_json::from_value::<#variant_type>(value)
                        .map(Self::#variant_name)
                        .map_err(serde::de::Error::custom);
                }
            })
        })
        .collect::<TctResult<Vec<_>>>()?;
    let expected = variant_json_types.join(", ");

    Ok(quote! {
        impl<'de> serde::Deserialize<'de> for #type_ident {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                let value = serde_json::Value::deserialize(deserializer)?;
                #(#branches)*
                Err(serde::de::Error::custom(format!(
                    "Data did not match any variant of {} enum: expected one of the types {}",
                    stringify!(#type_ident),
                    #expected
                )))
            }
        }
    })
}
