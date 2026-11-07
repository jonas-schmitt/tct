//! Vendored external JSON Schema resources used during deterministic code generation.
//!
//! TCT does not fetch schemas while compiling or while generated tokenizers run.
//! External `$ref` support is limited to resources explicitly vendored here.

#[derive(Debug, Clone, Copy)]
pub(crate) struct ExternalSchemaResource {
    pub uri: &'static str,
    pub schema_json: &'static str,
}

use serde_json::Value;
use std::collections::BTreeMap;

const PARTIAL_ESLINT_PLUGINS_URI: &str = "https://json.schemastore.org/partial-eslint-plugins.json";
const PARTIAL_ESLINT_PLUGINS_SCHEMA: &str =
    include_str!("../../schemas/external/partial-eslint-plugins.json");
const ESLINTRC_URI: &str = "https://json.schemastore.org/eslintrc.json";
const ESLINTRC_SCHEMA: &str = include_str!("../../schemas/popular/eslintrc.json");
const PRETTIERRC_URI: &str = "https://json.schemastore.org/prettierrc.json";
const PRETTIERRC_SCHEMA: &str = include_str!("../../schemas/popular/prettierrc.json");
const STYLELINTRC_URI: &str = "https://json.schemastore.org/stylelintrc.json";
const STYLELINTRC_SCHEMA: &str = include_str!("../../schemas/popular/stylelintrc.json");
const AVA_URI: &str = "https://json.schemastore.org/ava.json";
const AVA_SCHEMA: &str = include_str!("../../schemas/popular/ava.json");
const SEMANTIC_RELEASE_URI: &str = "https://json.schemastore.org/semantic-release.json";
const SEMANTIC_RELEASE_SCHEMA: &str = include_str!("../../schemas/common/semantic-release.json");
const JSCPD_URI: &str = "https://json.schemastore.org/jscpd.json";
const JSCPD_SCHEMA: &str = include_str!("../../schemas/external/jscpd.json");

pub(crate) fn external_schema_resource_uri(ref_path: &str) -> &str {
    ref_path
        .split('#')
        .next()
        .filter(|uri| !uri.is_empty())
        .unwrap_or(ref_path)
}

pub(crate) fn get_external_schema_resource(ref_path: &str) -> Option<ExternalSchemaResource> {
    match external_schema_resource_uri(ref_path) {
        PARTIAL_ESLINT_PLUGINS_URI => Some(ExternalSchemaResource {
            uri: PARTIAL_ESLINT_PLUGINS_URI,
            schema_json: PARTIAL_ESLINT_PLUGINS_SCHEMA,
        }),
        ESLINTRC_URI => Some(ExternalSchemaResource {
            uri: ESLINTRC_URI,
            schema_json: ESLINTRC_SCHEMA,
        }),
        PRETTIERRC_URI => Some(ExternalSchemaResource {
            uri: PRETTIERRC_URI,
            schema_json: PRETTIERRC_SCHEMA,
        }),
        STYLELINTRC_URI => Some(ExternalSchemaResource {
            uri: STYLELINTRC_URI,
            schema_json: STYLELINTRC_SCHEMA,
        }),
        AVA_URI => Some(ExternalSchemaResource {
            uri: AVA_URI,
            schema_json: AVA_SCHEMA,
        }),
        SEMANTIC_RELEASE_URI => Some(ExternalSchemaResource {
            uri: SEMANTIC_RELEASE_URI,
            schema_json: SEMANTIC_RELEASE_SCHEMA,
        }),
        JSCPD_URI => Some(ExternalSchemaResource {
            uri: JSCPD_URI,
            schema_json: JSCPD_SCHEMA,
        }),
        _ => None,
    }
}

pub(crate) fn vendored_external_schema_resources(schema: &Value) -> Vec<ExternalSchemaResource> {
    let mut resources = BTreeMap::<String, ExternalSchemaResource>::new();
    collect_vendored_external_schema_resources(schema, &mut resources);
    resources.into_values().collect()
}

fn collect_vendored_external_schema_resources(
    schema: &Value,
    resources: &mut BTreeMap<String, ExternalSchemaResource>,
) {
    match schema {
        Value::Object(object) => {
            if let Some(ref_path) = object.get("$ref").and_then(Value::as_str)
                && let Some(resource) = get_external_schema_resource(ref_path)
            {
                let is_new = resources
                    .insert(resource.uri.to_string(), resource)
                    .is_none();
                if is_new
                    && let Ok(resource_schema) = serde_json::from_str::<Value>(resource.schema_json)
                {
                    collect_vendored_external_schema_resources(&resource_schema, resources);
                }
            }

            for value in object.values() {
                collect_vendored_external_schema_resources(value, resources);
            }
        }
        Value::Array(items) => {
            for item in items {
                collect_vendored_external_schema_resources(item, resources);
            }
        }
        _ => {}
    }
}
