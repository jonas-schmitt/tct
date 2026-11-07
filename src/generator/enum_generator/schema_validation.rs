//! Schema-aware validation renderers for `anyOf` and `oneOf` enum variants.
//!
//! These helpers preserve the source schema fragments needed for generated
//! variant validation, including inherited draft and definition context, then
//! emit cached validator functions used by schema-aware deserialization paths.

use crate::TctResult;
use crate::generator::json_schema_validator::{
    ValidatorBuildErrorPolicy, generated_validator_external_resources,
    render_external_resource_entries, render_jsonschema_validator_build,
};
use crate::generator::schema_discovery::{MemberInfo, MemberType, SchemaDiscovery, TypeDiscovery};
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use serde_json::Value;

/// Check if this enum is an anyOf/oneOf composition that needs schema-aware deserialization.
/// Returns Some(is_anyof) if it's a composition, None otherwise.
pub(super) fn check_anyof_oneof_enum(type_discovery: &TypeDiscovery) -> Option<bool> {
    let has_anyof = type_discovery.schema_object.contains_key("anyOf");
    let has_oneof = type_discovery.schema_object.contains_key("oneOf");

    if has_anyof || has_oneof {
        crate::log_debug!(
            "SCHEMA-AWARE: Type '{}' has anyOf={} oneOf={}",
            type_discovery.type_name,
            has_anyof,
            has_oneof
        );
        Some(has_anyof)
    } else {
        crate::log_debug!(
            "SCHEMA-AWARE: Type '{}' has NO anyOf/oneOf. Keys: {:?}",
            type_discovery.type_name,
            type_discovery.schema_object.keys().collect::<Vec<_>>()
        );
        None
    }
}

/// Generate schema storage constants for anyOf/oneOf variants.
pub(super) fn generate_variant_schema_constants(
    discovery: &SchemaDiscovery,
    members: &[MemberInfo],
) -> Vec<TokenStream> {
    members
        .iter()
        .enumerate()
        .map(|(idx, member)| {
            let mut complete_schema = member.schema.clone();
            let schema_str =
                serde_json::to_string(&member.schema).expect("Failed to serialize variant schema");
            let needs_definitions = schema_str.contains("$ref");

            if let Some(root_info) = &discovery.root_info {
                if let Some(root_schema_draft) = root_info.schema.get("$schema")
                    && !complete_schema.contains_key("$schema")
                {
                    complete_schema.insert("$schema".to_string(), root_schema_draft.clone());
                }

                if needs_definitions
                    && let Some(root_defs) = root_info.schema.get("definitions")
                    && !complete_schema.contains_key("definitions")
                {
                    complete_schema.insert("definitions".to_string(), root_defs.clone());
                }
            }

            let complete_schema_value = Value::Object(complete_schema);
            let external_resources = generated_validator_external_resources(&complete_schema_value);
            let external_resource_entries = render_external_resource_entries(&external_resources);

            let schema_json = serde_json::to_string(&complete_schema_value)
                .expect("Failed to serialize variant schema");
            let const_name = format_ident!("VARIANT_{}_SCHEMA", idx);
            let resources_const_name = format_ident!("VARIANT_{}_EXTERNAL_RESOURCES", idx);

            quote! {
                const #const_name: &'static str = #schema_json;
                const #resources_const_name: &'static [(&'static str, &'static str)] = &[
                    #(#external_resource_entries),*
                ];
            }
        })
        .collect()
}

/// Generate validation functions for each anyOf/oneOf variant.
pub(super) fn generate_variant_validation_functions(members: &[MemberInfo]) -> Vec<TokenStream> {
    members
        .iter()
        .enumerate()
        .map(|(idx, _member)| {
            let fn_name = format_ident!("validate_variant_{}", idx);
            let const_name = format_ident!("VARIANT_{}_SCHEMA", idx);
            let resources_const_name = format_ident!("VARIANT_{}_EXTERNAL_RESOURCES", idx);
            let validator_build = render_jsonschema_validator_build(
                quote! { &schema },
                quote! { Self::#resources_const_name },
                ValidatorBuildErrorPolicy::Option,
            );

            quote! {
                fn #fn_name(value: &serde_json::Value) -> bool {
                    static VALIDATOR: std::sync::OnceLock<Option<jsonschema::Validator>> =
                        std::sync::OnceLock::new();

                    VALIDATOR
                        .get_or_init(|| {
                            let schema: serde_json::Value = serde_json::from_str(Self::#const_name)
                                .expect("Failed to parse variant schema");
                            #validator_build
                        })
                        .as_ref()
                        .is_some_and(|validator| validator.is_valid(value))
                }
            }
        })
        .collect()
}

/// Generate custom Deserialize impl for schema-aware anyOf/oneOf variant selection.
/// `is_anyof`: true for anyOf (use first match), false for oneOf (require unique match).
pub(super) fn generate_schema_aware_deserialize_impl(
    type_ident: &Ident,
    members: &[MemberInfo],
    is_anyof: bool,
    member_type_fn: &dyn Fn(&MemberType) -> TctResult<TokenStream>,
) -> TctResult<TokenStream> {
    let variant_count = members.len();

    let validation_checks: Vec<_> = (0..variant_count)
        .map(|idx| {
            let fn_name = format_ident!("validate_variant_{}", idx);
            quote! {
                if Self::#fn_name(&value) {
                    matched_variants.push(#idx);
                }
            }
        })
        .collect();

    let deserialization_branches: Vec<_> = members
        .iter()
        .enumerate()
        .map(|(idx, member)| {
            let variant_name = format_ident!("Variant{}", idx);

            let is_null = matches!(
                &member.member_type,
                crate::generator::schema_discovery::MemberType::Primitive(t) if t == "Null"
            );

            if is_null {
                Ok(quote! {
                    #idx => {
                        // Null variant: validated by schema, just return unit variant
                        Ok(Self::#variant_name)
                    }
                })
            } else {
                let variant_type = member_type_fn(&member.member_type)?;

                Ok(quote! {
                    #idx => {
                        let typed = serde_json::from_value::<#variant_type>(value)
                            .map_err(serde::de::Error::custom)?;
                        Ok(Self::#variant_name(typed))
                    }
                })
            }
        })
        .collect::<TctResult<Vec<_>>>()?;

    Ok(quote! {
        impl<'de> serde::Deserialize<'de> for #type_ident {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                // Step 1: Deserialize to Value (preserves original structure)
                let value = serde_json::Value::deserialize(deserializer)?;

                // Step 2: Validate against each variant's schema
                let mut matched_variants = Vec::new();

                #(#validation_checks)*

                // Step 3: Handle match results
                match matched_variants.len() {
                    0 => Err(serde::de::Error::custom(
                        format!("Data did not match any variant of {} enum", stringify!(#type_ident))
                    )),
                    1 => {
                        // Exactly one variant matched - deserialize into typed struct
                        match matched_variants[0] {
                            #(#deserialization_branches,)*
                            _ => unreachable!(),
                        }
                    }
                    _ => {
                        if #is_anyof {
                            // For anyOf: multiple matches are valid, use the first matching variant
                            // This follows JSON Schema semantics where anyOf means "valid if ANY match"
                            match matched_variants[0] {
                                #(#deserialization_branches,)*
                                _ => unreachable!(),
                            }
                        } else {
                            // For oneOf: multiple matches is an error (schema violation)
                            Err(serde::de::Error::custom(format!(
                                "Ambiguous schema: data matched multiple variants: {:?}",
                                matched_variants
                            )))
                        }
                    }
                }
            }
        }
    })
}
