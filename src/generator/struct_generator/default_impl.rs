//! `Default` handling for generated structs.
//!
//! `serde_json::Number` has no `Default` impl, so a struct with a required
//! `{"type": "number"}` leaf can neither `#[derive(Default)]` nor initialise the
//! field with `Default::default()`. The enum generator already handles this by
//! dropping `Default` from the derive list and emitting a manual `impl Default`
//! that constructs the number explicitly; this module lets structs follow the
//! same convention.

use crate::generator::schema_discovery::{MemberInfo, MemberType};
use proc_macro2::{Delimiter, Group, Ident, TokenStream, TokenTree};
use quote::{format_ident, quote};
use rustc_hash::FxHashMap;

/// Whether this member's Rust type lacks a `Default` impl and must be
/// constructed explicitly.
///
/// Only *required* members qualify: an optional number is rendered as
/// `Option<serde_json::Number>` or `OptionalField<serde_json::Number>`, and both
/// of those wrappers implement `Default` regardless of their inner type.
/// Containers (`Vec<serde_json::Number>`) are likewise fine.
pub(super) fn member_needs_explicit_default(member: &MemberInfo) -> bool {
    member.required
        && matches!(
            &member.member_type,
            MemberType::Primitive(rust_type) if rust_type == "serde_json::Number"
        )
}

/// Default-value expression for a single field.
pub(super) fn default_value_expr(member: &MemberInfo) -> TokenStream {
    if member_needs_explicit_default(member) {
        return quote! { serde_json::Number::from(0) };
    }
    quote! { Default::default() }
}

/// Whether any member of this struct forces a manual `Default` impl.
pub(super) fn needs_manual_default(members: &[MemberInfo]) -> bool {
    members.iter().any(member_needs_explicit_default)
}

/// Field identifier for a member, honouring the collision-avoidance map.
pub(super) fn field_ident(
    member: &MemberInfo,
    field_name_map: &FxHashMap<String, String>,
) -> Ident {
    format_ident!(
        "{}",
        field_name_map.get(&member.name).unwrap_or(&member.name)
    )
}

/// `field: <default>` initialiser for a partially decoded struct literal.
///
/// Every streaming decode path that fills not-yet-decoded fields must go through
/// here rather than hand-writing `Default::default()`, or number leaves reach the
/// generated crate as uncompilable code.
pub(super) fn default_field_init(
    member: &MemberInfo,
    field_name_map: &FxHashMap<String, String>,
) -> TokenStream {
    let ident = field_ident(member, field_name_map);
    let value = default_value_expr(member);
    quote! { #ident: #value }
}

/// Remove `Default` from a `#[derive(..)]` attribute.
///
/// Returns the rewritten attribute and whether `Default` was actually present.
/// The caller uses that flag to decide whether a manual impl is owed: derives
/// are injected by the caller, and `Default` is only included when streaming
/// encode is on, so its presence cannot be assumed.
pub(super) fn strip_default_from_derive(tokens: &TokenStream) -> (TokenStream, bool) {
    let mut found = false;
    let rewritten = strip_in_stream(tokens.clone(), false, &mut found);
    (rewritten, found)
}

fn strip_in_stream(stream: TokenStream, inside_derive: bool, found: &mut bool) -> TokenStream {
    let mut out: Vec<TokenTree> = Vec::new();
    let mut saw_derive_ident = false;

    for tree in stream {
        match tree {
            TokenTree::Ident(ref ident) if inside_derive && ident == "Default" => {
                *found = true;
                // Drop the trailing separator too, or the leading one if this was
                // the final entry in the list.
                continue;
            }
            TokenTree::Group(group) => {
                let enter = inside_derive
                    || (saw_derive_ident && group.delimiter() == Delimiter::Parenthesis);
                let inner = strip_in_stream(group.stream(), enter, found);
                let mut rebuilt = Group::new(group.delimiter(), inner);
                rebuilt.set_span(group.span());
                out.push(TokenTree::Group(rebuilt));
            }
            other => {
                if let TokenTree::Ident(ref ident) = other {
                    saw_derive_ident = ident == "derive";
                }
                out.push(other);
            }
        }
    }

    if inside_derive {
        normalize_commas(out)
    } else {
        out.into_iter().collect()
    }
}

/// Collapse the doubled / leading / trailing commas left behind by a removal.
fn normalize_commas(trees: Vec<TokenTree>) -> TokenStream {
    let is_comma = |t: &TokenTree| matches!(t, TokenTree::Punct(p) if p.as_char() == ',');
    let mut out: Vec<TokenTree> = Vec::new();
    for tree in trees {
        if is_comma(&tree) && out.last().is_none_or(is_comma) {
            continue;
        }
        out.push(tree);
    }
    while out.last().is_some_and(is_comma) {
        out.pop();
    }
    out.into_iter().collect()
}

/// Manual `impl Default` constructing every field explicitly.
pub(super) fn manual_default_impl(
    type_ident: &Ident,
    members: &[MemberInfo],
    field_name_map: &FxHashMap<String, String>,
) -> TokenStream {
    let fields: Vec<TokenStream> = members
        .iter()
        .map(|member| {
            let ident = field_ident(member, field_name_map);
            let value = default_value_expr(member);
            quote! { #ident: #value }
        })
        .collect();

    quote! {
        impl Default for #type_ident {
            fn default() -> Self {
                Self {
                    #( #fields ),*
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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

    fn derive_attr() -> TokenStream {
        quote! {
            #[derive(Debug, PartialEq, Clone, Default, serde::Serialize, serde::Deserialize)]
        }
    }

    #[test]
    fn required_number_member_needs_manual_default() {
        assert!(needs_manual_default(&[member(
            "x",
            "serde_json::Number",
            true
        )]));
    }

    #[test]
    fn optional_number_and_ordinary_members_do_not() {
        assert!(!needs_manual_default(&[member(
            "x",
            "serde_json::Number",
            false
        )]));
        assert!(!needs_manual_default(&[
            member("a", "i64", true),
            member("b", "String", true),
        ]));
    }

    /// Regression: `#[derive(.., Default, ..)]` on a struct holding a
    /// `serde_json::Number` produced E0277 on real tool-call schemas.
    #[test]
    fn default_is_removed_from_the_middle_of_a_derive_list() {
        let (stripped, found) = strip_default_from_derive(&derive_attr());
        let rendered = stripped.to_string();

        assert!(found, "Default was present and should be reported");
        assert!(
            !rendered.contains("Default"),
            "Default must be gone, got: {rendered}"
        );
        for kept in ["Debug", "PartialEq", "Clone", "serde :: Serialize"] {
            assert!(rendered.contains(kept), "{kept} must survive: {rendered}");
        }
        assert!(
            !rendered.contains(", ,") && !rendered.contains("( ,"),
            "no dangling separators, got: {rendered}"
        );
        syn::parse_file(&format!("{rendered} struct S;"))
            .expect("stripped derive must still parse as Rust");
    }

    #[test]
    fn stripping_a_trailing_default_leaves_no_trailing_comma() {
        let (stripped, found) = strip_default_from_derive(&quote! {
            #[derive(Debug, Default)]
        });
        let rendered = stripped.to_string();

        assert!(found);
        assert!(
            !rendered.contains(",") || !rendered.trim_end().ends_with(", )"),
            "no trailing comma, got: {rendered}"
        );
        syn::parse_file(&format!("{rendered} struct S;"))
            .expect("stripped derive must still parse as Rust");
    }

    #[test]
    fn derive_without_default_is_untouched() {
        let original = quote! { #[derive(Debug, Clone, serde::Serialize)] };
        let (stripped, found) = strip_default_from_derive(&original);

        assert!(!found, "no Default present, so nothing is owed");
        assert_eq!(stripped.to_string(), original.to_string());
    }

    /// `Default` must only be stripped from the derive list — a field or type
    /// literally named `Default` elsewhere in the attribute is not a derive.
    #[test]
    fn only_the_derive_list_is_rewritten() {
        let (stripped, found) = strip_default_from_derive(&quote! {
            #[serde(default)]
        });
        assert!(!found);
        assert_eq!(
            stripped.to_string(),
            quote! { #[serde(default)] }.to_string()
        );
    }

    #[test]
    fn manual_impl_constructs_number_but_defaults_the_rest() {
        let members = vec![
            member("x", "serde_json::Number", true),
            member("count", "i64", true),
            member("maybe", "serde_json::Number", false),
        ];
        let rendered =
            manual_default_impl(&format_ident!("Widget"), &members, &FxHashMap::default())
                .to_string();

        assert!(rendered.contains("impl Default for Widget"));
        assert!(
            rendered.contains("x : serde_json :: Number :: from (0)"),
            "required number constructed explicitly, got: {rendered}"
        );
        assert!(
            rendered.contains("count : Default :: default ()"),
            "ordinary field keeps Default::default(), got: {rendered}"
        );
        assert!(
            rendered.contains("maybe : Default :: default ()"),
            "Option<Number> keeps Default::default(), got: {rendered}"
        );
        syn::parse_file(&rendered).expect("manual impl must parse as Rust");
    }

    /// Regression: the bit-packed streaming decode paths render their own
    /// partial-struct literals and hand-wrote `Default::default()`, so number
    /// leaves still failed to compile there after the derive and the standard
    /// `decode_prefix` renderer were fixed. All such sites share this helper.
    #[test]
    fn default_field_init_matches_the_member_rule() {
        let map = FxHashMap::default();

        let required_number =
            default_field_init(&member("iteration", "serde_json::Number", true), &map).to_string();
        assert!(
            required_number.contains("iteration : serde_json :: Number :: from (0)"),
            "got: {required_number}"
        );

        let optional_number =
            default_field_init(&member("iteration", "serde_json::Number", false), &map).to_string();
        assert!(
            optional_number.contains("iteration : Default :: default ()"),
            "got: {optional_number}"
        );

        let ordinary = default_field_init(&member("count", "i64", true), &map).to_string();
        assert!(
            ordinary.contains("count : Default :: default ()"),
            "got: {ordinary}"
        );
    }

    #[test]
    fn manual_impl_honours_the_field_name_collision_map() {
        let mut map = FxHashMap::default();
        map.insert("type".to_string(), "type_".to_string());
        let rendered = manual_default_impl(
            &format_ident!("Widget"),
            &[member("type", "serde_json::Number", true)],
            &map,
        )
        .to_string();

        assert!(
            rendered.contains("type_ : serde_json :: Number :: from (0)"),
            "sanitised field name must be used, got: {rendered}"
        );
    }
}
