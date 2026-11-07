//! Partial-struct helpers for const-first bit-packed streaming decode rendering.

use crate::generator::schema_discovery::MemberInfo;
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use rustc_hash::FxHashMap;

pub(super) fn optional_field_partial_struct(
    optional_fields: &[&MemberInfo],
    field_idx: usize,
    field_name_map: &FxHashMap<String, String>,
    decoded_zero_token_fields: &[Ident],
    decoded_variable_token_fields: &[Ident],
    field_ident: &Ident,
    current_field_value: TokenStream,
) -> TokenStream {
    let optional_before_current: Vec<_> = optional_fields
        .iter()
        .take(field_idx)
        .map(|m| {
            let f = format_ident!("{}", field_name_map.get(&m.name).unwrap_or(&m.name));
            quote! { #f }
        })
        .collect();

    let optional_remaining: Vec<_> = optional_fields
        .iter()
        .skip(field_idx + 1)
        .map(|m| {
            crate::generator::struct_generator::default_impl::default_field_init(m, field_name_map)
        })
        .collect();

    let standard_assigns: Vec<_> = decoded_zero_token_fields
        .iter()
        .chain(decoded_variable_token_fields.iter())
        .map(|f| quote! { #f })
        .collect();

    quote! {
        Self {
            #( #standard_assigns, )*
            #( #optional_before_current, )*
            #field_ident: #current_field_value,
            #( #optional_remaining ),*
        }
    }
}
