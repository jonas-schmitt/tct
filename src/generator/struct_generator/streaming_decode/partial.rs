//! Shared partial-struct rendering helpers for standard struct `decode_prefix`.

use crate::generator::schema_discovery::MemberInfo;
use crate::generator::struct_generator::default_impl::{default_value_expr, field_ident};
use proc_macro2::{Ident, TokenStream};
use quote::quote;
use rustc_hash::FxHashMap;

pub(in crate::generator::struct_generator::streaming_decode) fn before_field(
    members: &[MemberInfo],
    field_name_map: &FxHashMap<String, String>,
    field_idents: &[Ident],
    idx: usize,
) -> TokenStream {
    let decoded_fields: Vec<_> = field_idents
        .iter()
        .take(idx)
        .map(|field| quote! { #field })
        .collect();
    let default_fields = default_fields_from(members, field_name_map, idx);

    struct_literal(decoded_fields, default_fields)
}

pub(in crate::generator::struct_generator::streaming_decode) fn through_field(
    members: &[MemberInfo],
    field_name_map: &FxHashMap<String, String>,
    field_idents: &[Ident],
    idx: usize,
) -> TokenStream {
    let decoded_fields: Vec<_> = field_idents.iter().map(|field| quote! { #field }).collect();
    let default_fields = default_fields_from(members, field_name_map, idx + 1);

    struct_literal(decoded_fields, default_fields)
}

pub(in crate::generator::struct_generator::streaming_decode) fn with_field_value(
    members: &[MemberInfo],
    field_name_map: &FxHashMap<String, String>,
    field_idents: &[Ident],
    idx: usize,
    field_ident: &Ident,
    field_value: TokenStream,
) -> TokenStream {
    let decoded_fields: Vec<_> = field_idents
        .iter()
        .take(idx)
        .map(|field| quote! { #field })
        .collect();
    let default_fields = default_fields_from(members, field_name_map, idx + 1);

    if decoded_fields.is_empty() {
        quote! {
            Self {
                #field_ident: #field_value,
                #( #default_fields ),*
            }
        }
    } else {
        quote! {
            Self {
                #( #decoded_fields, )*
                #field_ident: #field_value,
                #( #default_fields ),*
            }
        }
    }
}

fn default_fields_from(
    members: &[MemberInfo],
    field_name_map: &FxHashMap<String, String>,
    start: usize,
) -> Vec<TokenStream> {
    members
        .iter()
        .skip(start)
        .map(|member| {
            let ident = field_ident(member, field_name_map);
            let default_expr = default_value_expr(member);
            quote! { #ident: #default_expr }
        })
        .collect()
}

fn struct_literal(
    decoded_fields: Vec<TokenStream>,
    default_fields: Vec<TokenStream>,
) -> TokenStream {
    if decoded_fields.is_empty() {
        quote! {
            Self {
                #( #default_fields ),*
            }
        }
    } else {
        quote! {
            Self {
                #( #decoded_fields, )*
                #( #default_fields ),*
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generator::schema_discovery::MemberType;
    use serde_json::json;

    fn member(name: &str, rust_type: &str, required: bool) -> MemberInfo {
        MemberInfo {
            name: name.to_string(),
            member_type: MemberType::Primitive(rust_type.to_string()),
            required,
            is_nullable: false,
            schema: json!({}).as_object().unwrap().clone(),
        }
    }

    /// Regression: `{"type": "number"}` leaves become `serde_json::Number`, which
    /// does NOT implement `Default`. A bare `Default::default()` in `decode_prefix`
    /// made every generated crate containing a number leaf fail to compile with
    /// E0277 on real tool-call schemas.
    ///
    /// Per-expression rules are covered in `default_impl`; this pins the wiring
    /// through the partial-struct renderer that `decode_prefix` actually emits.
    #[test]
    fn partial_struct_literal_uses_explicit_number_default() {
        let members = vec![
            member("x", "serde_json::Number", true),
            member("count", "i64", true),
            member("maybe", "serde_json::Number", false),
        ];
        let fields = default_fields_from(&members, &FxHashMap::default(), 0);
        let rendered: Vec<String> = fields.iter().map(|t| t.to_string()).collect();

        assert!(
            rendered[0].contains("serde_json :: Number :: from (0)"),
            "required number must be constructed explicitly, got: {:?}",
            rendered[0]
        );
        assert!(
            rendered[1].contains("Default :: default"),
            "i64 field should keep Default::default(), got: {:?}",
            rendered[1]
        );
        assert!(
            rendered[2].contains("Default :: default"),
            "Option<Number> implements Default and must use it, got: {:?}",
            rendered[2]
        );
    }
}
