//! Planning for generated `allOf` validation checks.
//!
//! This module reads discovered struct schemas and builds the validation plan
//! that rendering consumes: `not|required` field-set constraints, `if`/`then`
//! property constraints, validation-only JSON Schema fragments with vendored
//! external resources, and `additionalProperties: false` handling.

use super::{AllOfConditionalConstraint, AllOfJsonSchemaValidation, AllOfValidationPlan};
use crate::generator::json_schema_validator::{
    GeneratedValidatorExternalResource, generated_validator_external_resources,
};
use crate::generator::schema_discovery::TypeDiscovery;
use crate::generator::struct_generator::StructGenerator;
use serde_json::Value;

impl<'a> StructGenerator<'a> {
    pub(in crate::generator::struct_generator) fn allof_validation_plan(
        &self,
        type_discovery: &TypeDiscovery,
    ) -> AllOfValidationPlan {
        AllOfValidationPlan {
            not_required_constraints: self.allof_not_required_constraints(type_discovery),
            conditional_constraints: self.allof_conditional_constraints(type_discovery),
            json_schema_validations: self.allof_json_schema_validations(type_discovery),
            deny_unknown_fields: Self::deny_unknown_fields(type_discovery),
        }
    }

    fn allof_not_required_constraints(&self, type_discovery: &TypeDiscovery) -> Vec<Vec<String>> {
        let Some(allof_array) = type_discovery
            .schema_object
            .get("allOf")
            .and_then(Value::as_array)
        else {
            return Vec::new();
        };

        allof_array
            .iter()
            .filter_map(|subschema| {
                let required = subschema
                    .as_object()?
                    .get("not")?
                    .as_object()?
                    .get("required")?
                    .as_array()?;
                Some(
                    required
                        .iter()
                        .filter_map(Value::as_str)
                        .map(str::to_string)
                        .collect::<Vec<_>>(),
                )
            })
            .collect()
    }

    fn allof_conditional_constraints(
        &self,
        type_discovery: &TypeDiscovery,
    ) -> Vec<AllOfConditionalConstraint> {
        let Some(allof_array) = type_discovery
            .schema_object
            .get("allOf")
            .and_then(Value::as_array)
        else {
            return Vec::new();
        };

        allof_array
            .iter()
            .filter_map(|subschema| {
                let subschema = subschema.as_object()?;
                let if_schema = subschema.get("if")?.as_object()?;
                let if_properties = if_schema.get("properties")?.as_object()?;
                let (if_property, if_property_schema) = if_properties.iter().next()?;
                let if_const = if_property_schema
                    .as_object()?
                    .get("const")?
                    .as_str()?
                    .to_string();

                let Some(then_schema) = subschema.get("then").and_then(Value::as_object) else {
                    return Some(AllOfConditionalConstraint {
                        if_property: if_property.clone(),
                        if_const,
                        then_required: Vec::new(),
                        then_property_types: Vec::new(),
                    });
                };

                let then_required = then_schema
                    .get("required")
                    .and_then(Value::as_array)
                    .map(|required| {
                        required
                            .iter()
                            .filter_map(Value::as_str)
                            .map(str::to_string)
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();

                let then_property_types = then_schema
                    .get("properties")
                    .and_then(Value::as_object)
                    .map(|properties| {
                        properties
                            .iter()
                            .filter_map(|(name, schema)| {
                                let type_name = schema.as_object()?.get("type")?.as_str()?;
                                Some((name.clone(), type_name.to_string()))
                            })
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();

                Some(AllOfConditionalConstraint {
                    if_property: if_property.clone(),
                    if_const,
                    then_required,
                    then_property_types,
                })
            })
            .collect()
    }

    fn allof_json_schema_validations(
        &self,
        type_discovery: &TypeDiscovery,
    ) -> Vec<AllOfJsonSchemaValidation> {
        let Some(allof_array) = type_discovery
            .schema_object
            .get("allOf")
            .and_then(Value::as_array)
        else {
            return Vec::new();
        };

        allof_array
            .iter()
            .enumerate()
            .filter_map(|(index, subschema)| {
                let subschema = subschema.as_object()?;
                if !Self::requires_json_schema_allof_validation(subschema) {
                    return None;
                }

                let complete_schema = self.complete_validation_schema(subschema);
                let external_resources = self.external_resources_for_validation_schema(
                    &Value::Object(complete_schema.clone()),
                );
                let schema_json = serde_json::to_string(&Value::Object(complete_schema))
                    .expect("failed to serialize allOf validation schema");
                let schema_path =
                    Self::join_schema_path(&type_discovery.schema_location.json_path, "allOf");
                let schema_path = Self::join_schema_path(&schema_path, &index.to_string());

                Some(AllOfJsonSchemaValidation {
                    schema_json,
                    schema_path,
                    external_resources,
                })
            })
            .collect()
    }

    fn deny_unknown_fields(type_discovery: &TypeDiscovery) -> bool {
        type_discovery
            .schema_object
            .get("additionalProperties")
            .is_some_and(|additional_properties| additional_properties == &Value::Bool(false))
    }

    fn requires_json_schema_allof_validation(schema: &serde_json::Map<String, Value>) -> bool {
        schema.contains_key("anyOf")
            || schema
                .get("additionalProperties")
                .is_some_and(|additional_properties| additional_properties != &Value::Bool(true))
            || schema
                .get("$ref")
                .and_then(Value::as_str)
                .is_some_and(|ref_path| {
                    ref_path.starts_with("http://") || ref_path.starts_with("https://")
                })
    }

    fn external_resources_for_validation_schema(
        &self,
        schema: &Value,
    ) -> Vec<GeneratedValidatorExternalResource> {
        generated_validator_external_resources(schema)
    }

    fn complete_validation_schema(
        &self,
        schema: &serde_json::Map<String, Value>,
    ) -> serde_json::Map<String, Value> {
        let mut complete_schema = schema.clone();
        let schema_str =
            serde_json::to_string(schema).expect("failed to serialize validation schema");
        let needs_definitions = schema_str.contains("$ref");

        let Some(root_info) = self
            .discovery
            .and_then(|discovery| discovery.root_info.as_ref())
        else {
            return complete_schema;
        };

        if let Some(root_schema_draft) = root_info.schema.get("$schema") {
            complete_schema
                .entry("$schema".to_string())
                .or_insert_with(|| root_schema_draft.clone());
        }

        if needs_definitions {
            if let Some(root_defs) = root_info.schema.get("definitions") {
                complete_schema
                    .entry("definitions".to_string())
                    .or_insert_with(|| root_defs.clone());
            }
            if let Some(root_defs) = root_info.schema.get("$defs") {
                complete_schema
                    .entry("$defs".to_string())
                    .or_insert_with(|| root_defs.clone());
            }
        }

        complete_schema
    }

    fn join_schema_path(base: &str, segment: &str) -> String {
        let escaped_segment = crate::schema_utils::escape_json_pointer_segment(segment);
        if base == "#" {
            format!("#/{}", escaped_segment)
        } else {
            format!("{}/{}", base, escaped_segment)
        }
    }
}
