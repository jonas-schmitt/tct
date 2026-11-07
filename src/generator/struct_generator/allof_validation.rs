//! Shared generated `allOf` validation contract for struct generation.
//!
//! This module defines the plan and constraint data passed from schema
//! inspection in [`plan`] to token rendering in [`render`], including custom
//! deserialization inputs and pattern-validator token fragments.

mod plan;
mod render;

use proc_macro2::{Ident, TokenStream};

use crate::generator::json_schema_validator::GeneratedValidatorExternalResource;

#[derive(Debug, Clone, Default)]
pub(super) struct AllOfValidationPlan {
    pub(super) not_required_constraints: Vec<Vec<String>>,
    pub(super) conditional_constraints: Vec<AllOfConditionalConstraint>,
    pub(super) json_schema_validations: Vec<AllOfJsonSchemaValidation>,
    pub(super) deny_unknown_fields: bool,
}

impl AllOfValidationPlan {
    pub(super) fn has_constraints(&self) -> bool {
        !self.not_required_constraints.is_empty()
            || !self.conditional_constraints.is_empty()
            || !self.json_schema_validations.is_empty()
    }

    fn needs_raw_object(&self) -> bool {
        !self.conditional_constraints.is_empty() || !self.json_schema_validations.is_empty()
    }
}

#[derive(Debug, Clone)]
pub(super) struct AllOfConditionalConstraint {
    if_property: String,
    if_const: String,
    then_required: Vec<String>,
    then_property_types: Vec<(String, String)>,
}

#[derive(Debug, Clone)]
pub(super) struct AllOfJsonSchemaValidation {
    schema_json: String,
    schema_path: String,
    external_resources: Vec<GeneratedValidatorExternalResource>,
}

pub(super) struct AllOfDeserializeGenerationInput<'a> {
    pub(super) type_ident: &'a Ident,
    pub(super) validation_plan: &'a AllOfValidationPlan,
    pub(super) deserialize_helper_fields: &'a [TokenStream],
    pub(super) field_idents: &'a [Ident],
}

pub(super) struct AllOfPatternValidationTokens {
    pub(super) seen_init: TokenStream,
    pub(super) seen_record: TokenStream,
    pub(super) raw_object_init: TokenStream,
    pub(super) raw_object_record: TokenStream,
    pub(super) json_value_init: TokenStream,
    pub(super) not_required_checks: Vec<TokenStream>,
    pub(super) conditional_checks: Vec<TokenStream>,
    pub(super) json_schema_checks: Vec<TokenStream>,
}

#[cfg(test)]
mod tests {
    use super::super::StructGenerator;
    use super::*;
    use crate::generator::schema_discovery::{
        RootSchemaInfo, SchemaDiscovery, SchemaLocation, TypeDiscovery, TypeKind,
    };
    use crate::generator::type_analysis::TypeAnalysis;
    use serde_json::Value;
    use serde_json::json;

    fn type_discovery(schema: Value) -> TypeDiscovery {
        TypeDiscovery::new(
            "Root".to_string(),
            TypeKind::Struct,
            SchemaLocation {
                json_path: "#".to_string(),
                description: "test root".to_string(),
                parent_type: None,
                property_name: None,
            },
            schema.as_object().expect("schema object").clone(),
        )
    }

    fn generator_without_discovery(analysis: &TypeAnalysis) -> StructGenerator<'_> {
        StructGenerator::new(analysis, false)
    }

    #[test]
    fn allof_validation_plan_extracts_not_required_if_then_and_deny_unknown_fields() {
        let schema = json!({
            "type": "object",
            "additionalProperties": false,
            "allOf": [
                {
                    "not": {
                        "required": ["first", "second"]
                    }
                },
                {
                    "if": {
                        "properties": {
                            "kind": { "const": "special" }
                        },
                        "required": ["kind"]
                    },
                    "then": {
                        "required": ["payload"],
                        "properties": {
                            "count": { "type": "integer" }
                        }
                    }
                }
            ]
        });
        let discovery = type_discovery(schema);
        let analysis = TypeAnalysis::default();
        let generator = generator_without_discovery(&analysis);

        let plan = generator.allof_validation_plan(&discovery);

        assert!(plan.has_constraints());
        assert!(plan.deny_unknown_fields);
        assert_eq!(plan.not_required_constraints, vec![vec!["first", "second"]]);
        assert_eq!(plan.conditional_constraints.len(), 1);
        let constraint = &plan.conditional_constraints[0];
        assert_eq!(constraint.if_property, "kind");
        assert_eq!(constraint.if_const, "special");
        assert_eq!(constraint.then_required, vec!["payload"]);
        assert_eq!(
            constraint.then_property_types,
            vec![("count".to_string(), "integer".to_string())]
        );
    }

    #[test]
    fn allof_validation_plan_keeps_root_defs_and_vendored_external_resources() {
        let schema = json!({
            "type": "object",
            "allOf": [
                {
                    "type": "object",
                    "anyOf": [
                        { "required": ["plugins"] },
                        { "required": ["rules"] }
                    ],
                    "properties": {
                        "plugins": {
                            "$ref": "https://json.schemastore.org/partial-eslint-plugins.json"
                        },
                        "rules": {
                            "$ref": "#/definitions/Rules"
                        }
                    }
                }
            ]
        });
        let root_schema = json!({
            "$schema": "http://json-schema.org/draft-07/schema#",
            "definitions": {
                "Rules": {
                    "type": "object"
                }
            }
        });
        let mut schema_discovery = SchemaDiscovery::new();
        schema_discovery.root_info = Some(RootSchemaInfo {
            type_name: "Root".to_string(),
            schema: root_schema.as_object().expect("root schema object").clone(),
            is_composition: false,
        });
        let type_discovery = type_discovery(schema);
        let analysis = TypeAnalysis::default();
        let generator = StructGenerator::with_discovery_and_encoding_base(
            &analysis,
            &schema_discovery,
            false,
            StructGenerator::DEFAULT_ENCODING_BASE,
        );

        let plan = generator.allof_validation_plan(&type_discovery);

        assert_eq!(plan.json_schema_validations.len(), 1);
        let validation = &plan.json_schema_validations[0];
        assert_eq!(validation.schema_path, "#/allOf/0");
        assert_eq!(validation.external_resources.len(), 1);
        assert_eq!(
            validation.external_resources[0].uri,
            "https://json.schemastore.org/partial-eslint-plugins.json"
        );

        let embedded_schema: Value =
            serde_json::from_str(&validation.schema_json).expect("embedded schema JSON");
        assert_eq!(
            embedded_schema.get("$schema").and_then(Value::as_str),
            Some("http://json-schema.org/draft-07/schema#")
        );
        assert!(embedded_schema.get("definitions").is_some());
        assert!(embedded_schema.get("anyOf").is_some());
    }

    #[test]
    fn allof_validation_plan_is_empty_without_allof() {
        let schema = json!({
            "type": "object",
            "properties": {
                "name": { "type": "string" }
            }
        });
        let discovery = type_discovery(schema);
        let analysis = TypeAnalysis::default();
        let generator = generator_without_discovery(&analysis);

        let plan = generator.allof_validation_plan(&discovery);

        assert!(!plan.has_constraints());
        assert!(!plan.deny_unknown_fields);
        assert!(plan.not_required_constraints.is_empty());
        assert!(plan.conditional_constraints.is_empty());
        assert!(plan.json_schema_validations.is_empty());
    }

    #[test]
    fn pattern_allof_validation_tokens_emit_only_needed_state() {
        let empty = AllOfValidationPlan::default();
        let tokens = StructGenerator::pattern_allof_validation_tokens(&empty);
        assert!(tokens.seen_init.is_empty());
        assert!(tokens.raw_object_init.is_empty());
        assert!(tokens.not_required_checks.is_empty());
        assert!(tokens.conditional_checks.is_empty());
        assert!(tokens.json_schema_checks.is_empty());

        let plan = AllOfValidationPlan {
            not_required_constraints: vec![vec!["a".to_string(), "b".to_string()]],
            conditional_constraints: vec![AllOfConditionalConstraint {
                if_property: "kind".to_string(),
                if_const: "special".to_string(),
                then_required: vec!["payload".to_string()],
                then_property_types: Vec::new(),
            }],
            json_schema_validations: Vec::new(),
            deny_unknown_fields: false,
        };
        let tokens = StructGenerator::pattern_allof_validation_tokens(&plan);
        assert!(!tokens.seen_init.is_empty());
        assert!(!tokens.raw_object_init.is_empty());
        assert_eq!(tokens.not_required_checks.len(), 1);
        assert_eq!(tokens.conditional_checks.len(), 1);
        assert!(tokens.json_schema_checks.is_empty());
    }
}
