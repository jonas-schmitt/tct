//! Shared renderers for generated JSON Schema validator setup.
//!
//! Enum variant validation and `allOf` validation both compile embedded schema
//! fragments with optional vendored external resources. Keep the registry
//! construction here so resource parsing, registry preparation, and error
//! policy do not drift between generated validators.

use crate::generator::external_schemas::vendored_external_schema_resources;
use proc_macro2::{Span, TokenStream};
use quote::quote;
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GeneratedValidatorExternalResource {
    pub(crate) uri: String,
    pub(crate) schema_json: String,
}

pub(crate) enum ValidatorBuildErrorPolicy<'a> {
    Option,
    ResultString {
        schema_path: &'a str,
        validation_kind: &'static str,
    },
}

pub(crate) fn generated_validator_external_resources(
    schema: &Value,
) -> Vec<GeneratedValidatorExternalResource> {
    vendored_external_schema_resources(schema)
        .into_iter()
        .map(|resource| GeneratedValidatorExternalResource {
            uri: resource.uri.to_string(),
            schema_json: resource.schema_json.to_string(),
        })
        .collect()
}

pub(crate) fn render_external_resource_entries(
    resources: &[GeneratedValidatorExternalResource],
) -> Vec<TokenStream> {
    resources
        .iter()
        .map(|resource| {
            let uri = syn::LitStr::new(&resource.uri, Span::call_site());
            let schema_json = syn::LitStr::new(&resource.schema_json, Span::call_site());
            quote! {
                (#uri, #schema_json)
            }
        })
        .collect()
}

/// Retriever emitted next to every generated validator build.
///
/// Generated tokenizers never fetch schemas: an external `$ref` resolves only
/// against the vendored resources registered with the validator. Without this
/// the `jsonschema` crate's default retriever would GET `http(s)://` references
/// and read `file://` ones whenever it is compiled with its default features,
/// which would make decoding depend on the network and on the local filesystem.
/// A reference that is not vendored fails the validator build instead, so the
/// branch simply never validates.
fn render_offline_retriever() -> TokenStream {
    quote! {
        struct TctOfflineRetriever;

        impl jsonschema::Retrieve for TctOfflineRetriever {
            fn retrieve(
                &self,
                uri: &jsonschema::Uri<String>,
            ) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
                Err(format!(
                    "external schema resource {} is not vendored; generated tokenizers never fetch schemas",
                    uri
                )
                .into())
            }
        }
    }
}

pub(crate) fn render_jsonschema_validator_build(
    schema_expr: TokenStream,
    resources_expr: TokenStream,
    error_policy: ValidatorBuildErrorPolicy<'_>,
) -> TokenStream {
    let offline_retriever = render_offline_retriever();
    match error_policy {
        ValidatorBuildErrorPolicy::Option => {
            quote! {
                {
                    #offline_retriever

                    if #resources_expr.is_empty() {
                        jsonschema::options()
                            .with_retriever(TctOfflineRetriever)
                            .build(#schema_expr)
                            .ok()
                    } else {
                        let tct_external_resources = #resources_expr
                            .iter()
                            .map(|(uri, schema_json)| {
                                let resource_schema: serde_json::Value =
                                    serde_json::from_str(schema_json).ok()?;
                                Some((*uri, resource_schema))
                            })
                            .collect::<Option<Vec<_>>>()?;

                        let mut registry_builder = jsonschema::Registry::new();
                        for (uri, resource_schema) in &tct_external_resources {
                            registry_builder = registry_builder.add(*uri, resource_schema).ok()?;
                        }
                        let registry = registry_builder.prepare().ok()?;

                        jsonschema::options()
                            .with_registry(&registry)
                            .with_retriever(TctOfflineRetriever)
                            .build(#schema_expr)
                            .ok()
                    }
                }
            }
        }
        ValidatorBuildErrorPolicy::ResultString {
            schema_path,
            validation_kind,
        } => {
            let schema_path = syn::LitStr::new(schema_path, Span::call_site());
            let resource_error = format!(
                "invalid embedded external {} validation resource",
                validation_kind
            );
            let registry_error = format!(
                "invalid embedded external {} validation registry",
                validation_kind
            );

            quote! {
                {
                    #offline_retriever

                    if #resources_expr.is_empty() {
                        jsonschema::options()
                            .with_retriever(TctOfflineRetriever)
                            .build(#schema_expr)
                            .map_err(|error| error.to_string())
                    } else {
                        let mut tct_external_resources: Vec<(&'static str, serde_json::Value)> =
                            Vec::new();
                        for (uri, schema_json) in #resources_expr {
                            let resource_schema: serde_json::Value =
                                serde_json::from_str(schema_json).map_err(|error| {
                                    format!(
                                        "{} {} at {}: {}",
                                        #resource_error,
                                        uri,
                                        #schema_path,
                                        error
                                    )
                                })?;
                            tct_external_resources.push((*uri, resource_schema));
                        }

                        let mut registry_builder = jsonschema::Registry::new();
                        for (uri, resource_schema) in &tct_external_resources {
                            registry_builder = registry_builder
                                .add(*uri, resource_schema)
                                .map_err(|error| {
                                    format!("{} at {}: {}", #registry_error, #schema_path, error)
                                })?;
                        }
                        let registry = registry_builder.prepare().map_err(|error| {
                            format!("{} at {}: {}", #registry_error, #schema_path, error)
                        })?;

                        jsonschema::options()
                            .with_registry(&registry)
                            .with_retriever(TctOfflineRetriever)
                            .build(#schema_expr)
                            .map_err(|error| error.to_string())
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quote::quote;
    use serde_json::json;

    #[test]
    fn external_resources_include_transitive_vendored_refs() {
        let schema = json!({
            "type": "object",
            "properties": {
                "eslintConfig": {
                    "$ref": "https://json.schemastore.org/eslintrc.json"
                }
            }
        });

        let resources = generated_validator_external_resources(&schema);
        let uris = resources
            .iter()
            .map(|resource| resource.uri.as_str())
            .collect::<Vec<_>>();

        assert_eq!(
            uris.first(),
            Some(&"https://json.schemastore.org/eslintrc.json")
        );
        assert!(uris.contains(&"https://json.schemastore.org/partial-eslint-plugins.json"));
    }

    #[test]
    fn validator_builds_always_use_the_offline_retriever() {
        let renderings = [
            render_jsonschema_validator_build(
                quote! { &schema },
                quote! { Self::VARIANT_0_EXTERNAL_RESOURCES },
                ValidatorBuildErrorPolicy::Option,
            ),
            render_jsonschema_validator_build(
                quote! { &schema },
                quote! { TCT_ALLOF_EXTERNAL_RESOURCES_0 },
                ValidatorBuildErrorPolicy::ResultString {
                    schema_path: "#/allOf/0",
                    validation_kind: "allOf",
                },
            ),
        ];

        for tokens in renderings {
            let text = tokens.to_string();
            // `validator_for` builds with the crate's default retriever, which
            // fetches http(s) and file references.
            assert!(
                !text.contains("validator_for"),
                "validator builds must not use the default retriever: {text}"
            );
            let builds = text.matches("jsonschema :: options ()").count();
            assert_eq!(builds, 2, "one build per registry branch: {text}");
            assert_eq!(
                text.matches("with_retriever (TctOfflineRetriever)").count(),
                builds,
                "every jsonschema build must set the offline retriever: {text}"
            );
        }
    }

    #[test]
    fn validator_build_renderers_are_valid_expressions() {
        let option_tokens = render_jsonschema_validator_build(
            quote! { &schema },
            quote! { Self::VARIANT_0_EXTERNAL_RESOURCES },
            ValidatorBuildErrorPolicy::Option,
        );
        syn::parse2::<syn::Expr>(option_tokens).expect("option renderer should be an expression");

        let result_tokens = render_jsonschema_validator_build(
            quote! { &schema },
            quote! { TCT_ALLOF_EXTERNAL_RESOURCES_0 },
            ValidatorBuildErrorPolicy::ResultString {
                schema_path: "#/allOf/0",
                validation_kind: "allOf",
            },
        );
        syn::parse2::<syn::Expr>(result_tokens).expect("result renderer should be an expression");
    }
}
