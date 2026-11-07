//! Rendering for generated `allOf` validation checks.
//!
//! This module turns an [`AllOfValidationPlan`] into generated serde
//! attributes, custom `Deserialize` implementations, pattern-validator hooks,
//! raw-object tracking, conditional checks, and cached JSON Schema validator
//! calls with contextual deserialization errors.

use super::{
    AllOfConditionalConstraint, AllOfDeserializeGenerationInput, AllOfJsonSchemaValidation,
    AllOfPatternValidationTokens, AllOfValidationPlan,
};
use crate::generator::json_schema_validator::{
    ValidatorBuildErrorPolicy, render_external_resource_entries, render_jsonschema_validator_build,
};
use crate::generator::schema_discovery::MemberInfo;
use crate::generator::struct_generator::StructGenerator;
use crate::generator::type_analysis::TraitDerivability;
use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote};

impl<'a> StructGenerator<'a> {
    pub(in crate::generator::struct_generator) fn serde_deserialize_attrs(
        member: &MemberInfo,
        has_pattern_validators: bool,
    ) -> Vec<TokenStream> {
        let mut attrs = Vec::new();
        let original_name = &member.name;
        let is_pattern_field = member.name.starts_with("pattern_")
            && member.name.chars().skip(8).all(|c| c.is_ascii_digit());

        if is_pattern_field || member.name == "additional_properties" {
            if member.name == "additional_properties" && !has_pattern_validators {
                attrs.push(quote! { #[serde(flatten)] });
            }
        } else if !has_pattern_validators {
            if member.required {
                attrs.push(quote! { #[serde(rename = #original_name)] });
            } else if member.uses_optional_field_encoding() {
                attrs.push(quote! { #[serde(rename = #original_name, default)] });
            } else {
                attrs.push(quote! {
                    #[serde(
                        rename = #original_name,
                        default,
                        deserialize_with = "crate::tokenizer::deserialize_option_reject_null"
                    )]
                });
            }
        }

        attrs
    }

    pub(in crate::generator::struct_generator) fn generate_derive_traits_without_deserialize(
        &self,
        trait_derivability: Option<&TraitDerivability>,
    ) -> TokenStream {
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

        if self.streaming_encode {
            traits.push("Default");
        }

        traits.push("serde::Serialize");

        let trait_idents: Vec<_> = traits
            .iter()
            .map(|trait_name| {
                if trait_name.contains("::") {
                    match syn::parse_str::<syn::Path>(trait_name) {
                        Ok(path) => quote! { #path },
                        Err(error) => {
                            crate::log_debug!(
                                "⚠️  Failed to parse trait path '{}': {}",
                                trait_name,
                                error
                            );
                            let ident = format_ident!("{}", trait_name.replace("::", "_"));
                            quote! { #ident }
                        }
                    }
                } else {
                    let ident = format_ident!("{}", trait_name);
                    quote! { #ident }
                }
            })
            .collect();

        quote! {
            #[derive(#(#trait_idents),*)]
        }
    }

    pub(in crate::generator::struct_generator) fn pattern_allof_validation_tokens(
        plan: &AllOfValidationPlan,
    ) -> AllOfPatternValidationTokens {
        let seen_init = if plan.not_required_constraints.is_empty() {
            quote! {}
        } else {
            quote! {
                let mut tct_allof_seen_fields = std::collections::HashSet::new();
            }
        };
        let seen_record = if plan.not_required_constraints.is_empty() {
            quote! {}
        } else {
            quote! {
                tct_allof_seen_fields.insert(key.clone());
            }
        };
        let not_required_checks =
            Self::allof_not_required_seen_set_checks(&plan.not_required_constraints);

        let raw_object_init = if plan.needs_raw_object() {
            quote! {
                let mut tct_allof_raw_object = serde_json::Map::new();
            }
        } else {
            quote! {}
        };
        let raw_object_record = if plan.needs_raw_object() {
            quote! {
                tct_allof_raw_object.insert(key.clone(), value.clone());
            }
        } else {
            quote! {}
        };
        let json_value_init = if plan.json_schema_validations.is_empty() {
            quote! {}
        } else {
            quote! {
                let tct_allof_value =
                    serde_json::Value::Object(tct_allof_raw_object.clone());
            }
        };
        let conditional_checks =
            Self::allof_conditional_raw_object_checks(&plan.conditional_constraints);
        let json_schema_checks = Self::allof_json_schema_value_checks(
            &plan.json_schema_validations,
            quote! { tct_allof_value },
            quote! { de::Error },
        );

        AllOfPatternValidationTokens {
            seen_init,
            seen_record,
            raw_object_init,
            raw_object_record,
            json_value_init,
            not_required_checks,
            conditional_checks,
            json_schema_checks,
        }
    }

    pub(in crate::generator::struct_generator) fn generate_custom_deserialize_for_allof_validation(
        input: AllOfDeserializeGenerationInput<'_>,
    ) -> TokenStream {
        let AllOfDeserializeGenerationInput {
            type_ident,
            validation_plan,
            deserialize_helper_fields,
            field_idents,
        } = input;

        let helper_ident = format_ident!("{}AllOfDeserializeHelper", type_ident);
        let not_required_checks =
            Self::allof_not_required_object_checks(&validation_plan.not_required_constraints);
        let conditional_checks =
            Self::allof_conditional_object_checks(&validation_plan.conditional_constraints);
        let json_schema_checks = Self::allof_json_schema_value_checks(
            &validation_plan.json_schema_validations,
            quote! { value },
            quote! { D::Error },
        );
        let field_assignments = field_idents.iter().map(|field_ident| {
            quote! {
                #field_ident: helper.#field_ident
            }
        });
        let helper_serde_attrs = if validation_plan.deny_unknown_fields {
            quote! { #[serde(deny_unknown_fields)] }
        } else {
            quote! {}
        };

        quote! {
            impl<'de> serde::Deserialize<'de> for #type_ident {
                fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
                where
                    D: serde::Deserializer<'de>,
                {
                    use serde::de::Error as _;

                    let value =
                        <serde_json::Value as serde::Deserialize>::deserialize(deserializer)?;
                    // Only the not|required and conditional checks read the fields; the
                    // binding still has to exist for the "expected object" rejection.
                    #[allow(unused_variables)]
                    let object = value.as_object().ok_or_else(|| {
                        D::Error::custom("expected object for allOf validation")
                    })?;

                    #( #not_required_checks )*
                    #( #conditional_checks )*
                    #( #json_schema_checks )*

                    #[derive(serde::Deserialize)]
                    #helper_serde_attrs
                    struct #helper_ident {
                        #( #deserialize_helper_fields ),*
                    }

                    let helper = serde_json::from_value::<#helper_ident>(value)
                        .map_err(D::Error::custom)?;

                    Ok(Self {
                        #( #field_assignments ),*
                    })
                }
            }
        }
    }

    fn allof_not_required_object_checks(constraints: &[Vec<String>]) -> Vec<TokenStream> {
        constraints
            .iter()
            .map(|required_fields| {
                let fields = required_fields
                    .iter()
                    .map(|field| syn::LitStr::new(field, Span::call_site()))
                    .collect::<Vec<_>>();
                let message = format!(
                    "allOf not|required constraint violated: object must not contain all of [{}]",
                    required_fields.join(", ")
                );
                quote! {
                    if [#(#fields),*].iter().all(|field| object.contains_key(*field)) {
                        return Err(D::Error::custom(#message));
                    }
                }
            })
            .collect()
    }

    fn json_type_check_tokens(value_expr: TokenStream, type_name: &str) -> TokenStream {
        match type_name {
            "string" => quote! { #value_expr.is_string() },
            "boolean" => quote! { #value_expr.is_boolean() },
            "number" => quote! { #value_expr.is_number() },
            "integer" => {
                quote! { (#value_expr.as_i64().is_some() || #value_expr.as_u64().is_some()) }
            }
            "object" => quote! { #value_expr.is_object() },
            "array" => quote! { #value_expr.is_array() },
            "null" => quote! { #value_expr.is_null() },
            _ => quote! { false },
        }
    }

    fn allof_conditional_object_checks(
        constraints: &[AllOfConditionalConstraint],
    ) -> Vec<TokenStream> {
        constraints
            .iter()
            .map(|constraint| {
                let if_property = syn::LitStr::new(&constraint.if_property, Span::call_site());
                let if_const = syn::LitStr::new(&constraint.if_const, Span::call_site());
                let required_checks = constraint.then_required.iter().map(|required_field| {
                    let required_field = syn::LitStr::new(required_field, Span::call_site());
                    let message = format!(
                        "allOf if/then constraint violated: '{}' must be present when '{}' is '{}'",
                        required_field.value(),
                        constraint.if_property,
                        constraint.if_const
                    );
                    quote! {
                        if !object.contains_key(#required_field) {
                            return Err(D::Error::custom(#message));
                        }
                    }
                });
                let property_type_checks = constraint.then_property_types.iter().map(
                    |(property_name, type_name)| {
                        let property_name_lit = syn::LitStr::new(property_name, Span::call_site());
                        let type_check =
                            Self::json_type_check_tokens(quote! { then_value }, type_name);
                        let message = format!(
                            "allOf if/then constraint violated: '{}' must be {} when '{}' is '{}'",
                            property_name, type_name, constraint.if_property, constraint.if_const
                        );
                        quote! {
                            if let Some(then_value) = object.get(#property_name_lit) {
                                if !(#type_check) {
                                    return Err(D::Error::custom(format!(
                                        "{} (found {})",
                                        #message,
                                        then_value
                                    )));
                                }
                            }
                        }
                    },
                );

                quote! {
                    if object
                        .get(#if_property)
                        .and_then(serde_json::Value::as_str)
                        == Some(#if_const)
                    {
                        #( #required_checks )*
                        #( #property_type_checks )*
                    }
                }
            })
            .collect()
    }

    fn allof_json_schema_value_checks(
        validations: &[AllOfJsonSchemaValidation],
        value_expr: TokenStream,
        error_type: TokenStream,
    ) -> Vec<TokenStream> {
        validations
            .iter()
            .enumerate()
            .map(|(index, validation)| {
                let schema_json = &validation.schema_json;
                let schema_path = &validation.schema_path;
                let validator_static = format_ident!("TCT_ALLOF_VALIDATOR_{}", index);
                let external_resources_static =
                    format_ident!("TCT_ALLOF_EXTERNAL_RESOURCES_{}", index);
                let external_resource_entries =
                    render_external_resource_entries(&validation.external_resources);
                let validator_build = render_jsonschema_validator_build(
                    quote! { &schema },
                    quote! { #external_resources_static },
                    ValidatorBuildErrorPolicy::ResultString {
                        schema_path,
                        validation_kind: "allOf",
                    },
                );
                let invalid_schema_message = format!(
                    "invalid embedded allOf JSON Schema validation schema at {}",
                    schema_path
                );
                let validation_message =
                    format!("allOf JSON Schema validation failed at {}", schema_path);
                quote! {
                    {
                        static #validator_static: std::sync::OnceLock<Result<jsonschema::Validator, String>> =
                            std::sync::OnceLock::new();
                        let validator = #validator_static
                            .get_or_init(|| {
                                const #external_resources_static: &'static [(&'static str, &'static str)] = &[
                                    #(#external_resource_entries),*
                                ];
                                let schema: serde_json::Value = serde_json::from_str(#schema_json)
                                    .map_err(|error| {
                                        format!(
                                            "{}: {}",
                                            #invalid_schema_message,
                                            error
                                        )
                                    })?;
                                #validator_build.map_err(|error| {
                                    format!(
                                        "{}: {}",
                                        #invalid_schema_message,
                                        error
                                    )
                                })
                            })
                            .as_ref()
                            .map_err(|error| #error_type::custom(error.clone()))?;
                        if !validator.is_valid(&#value_expr) {
                            return Err(#error_type::custom(#validation_message));
                        }
                    }
                }
            })
            .collect()
    }

    fn allof_not_required_seen_set_checks(constraints: &[Vec<String>]) -> Vec<TokenStream> {
        constraints
            .iter()
            .map(|required_fields| {
                let fields = required_fields
                    .iter()
                    .map(|field| syn::LitStr::new(field, Span::call_site()))
                    .collect::<Vec<_>>();
                let message = format!(
                    "allOf not|required constraint violated: object must not contain all of [{}]",
                    required_fields.join(", ")
                );
                quote! {
                    if [#(#fields),*]
                        .iter()
                        .all(|field| tct_allof_seen_fields.contains(*field))
                    {
                        return Err(de::Error::custom(#message));
                    }
                }
            })
            .collect()
    }

    fn allof_conditional_raw_object_checks(
        constraints: &[AllOfConditionalConstraint],
    ) -> Vec<TokenStream> {
        constraints
            .iter()
            .map(|constraint| {
                let if_property = syn::LitStr::new(&constraint.if_property, Span::call_site());
                let if_const = syn::LitStr::new(&constraint.if_const, Span::call_site());
                let required_checks = constraint.then_required.iter().map(|required_field| {
                    let required_field_lit = syn::LitStr::new(required_field, Span::call_site());
                    let message = format!(
                        "allOf if/then constraint violated: '{}' must be present when '{}' is '{}'",
                        required_field, constraint.if_property, constraint.if_const
                    );
                    quote! {
                        if !tct_allof_raw_object.contains_key(#required_field_lit) {
                            return Err(de::Error::custom(#message));
                        }
                    }
                });
                let property_type_checks = constraint.then_property_types.iter().map(
                    |(property_name, type_name)| {
                        let property_name_lit = syn::LitStr::new(property_name, Span::call_site());
                        let type_check =
                            Self::json_type_check_tokens(quote! { then_value }, type_name);
                        let message = format!(
                            "allOf if/then constraint violated: '{}' must be {} when '{}' is '{}'",
                            property_name, type_name, constraint.if_property, constraint.if_const
                        );
                        quote! {
                            if let Some(then_value) = tct_allof_raw_object.get(#property_name_lit) {
                                if !(#type_check) {
                                    return Err(de::Error::custom(format!(
                                        "{} (found {})",
                                        #message,
                                        then_value
                                    )));
                                }
                            }
                        }
                    },
                );

                quote! {
                    if tct_allof_raw_object
                        .get(#if_property)
                        .and_then(serde_json::Value::as_str)
                        == Some(#if_const)
                    {
                        #( #required_checks )*
                        #( #property_type_checks )*
                    }
                }
            })
            .collect()
    }
}
