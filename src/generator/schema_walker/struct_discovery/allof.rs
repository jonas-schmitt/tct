//! `allOf` member-discovery support for struct schemas.
//!
//! This module owns mergeable `allOf` subschema traversal, local `$ref`
//! recursion guards, required-property propagation, and validation-only
//! `allOf` forms that are checked here but emitted later as generated schema
//! validation instead of discovered struct members.

use super::{
    is_allof_annotation_key, is_allof_ref_annotation_key, json_pointer_segment,
    schema_declares_exact_object, unsupported_allof_shape,
};
use crate::TctResult;
use crate::generator::external_schemas::{
    external_schema_resource_uri, get_external_schema_resource,
};
use crate::generator::schema_discovery::TypeDiscovery;
use crate::generator::schema_walker::SchemaWalker;
use serde_json::{Map, Value};
use std::collections::HashSet;

#[derive(Debug, Clone)]
pub(super) struct AllOfRequiredEntry {
    property_name: String,
    schema_path: String,
}

impl SchemaWalker {
    pub(super) fn discover_allof_subschema_members(
        &mut self,
        subschema: &Map<String, Value>,
        type_discovery: &mut TypeDiscovery,
        index: usize,
        schema_path: &str,
        allow_implicit_object: bool,
    ) -> TctResult<()> {
        crate::log_debug!(
            "   📋 Processing allOf subschema {} for type '{}'",
            index,
            type_discovery.type_name
        );

        if let Some(ref_value) = subschema.get("$ref") {
            if let Some(validation_sibling) = subschema
                .keys()
                .find(|key| !is_allof_ref_annotation_key(key))
            {
                return Err(unsupported_allof_shape(
                    format!(
                        "{}/{}",
                        schema_path,
                        json_pointer_segment(validation_sibling)
                    ),
                    "$ref sibling",
                    "Move validation siblings into the referenced schema or implement exact $ref sibling semantics before compiling this allOf subschema.",
                ));
            }

            let Some(ref_path) = ref_value.as_str() else {
                return Err(unsupported_allof_shape(
                    format!("{}/$ref", schema_path),
                    "$ref",
                    "Use a string local $ref in allOf subschemas.",
                ));
            };
            if ref_path.starts_with("http://") || ref_path.starts_with("https://") {
                let resource_uri = external_schema_resource_uri(ref_path);
                if get_external_schema_resource(resource_uri).is_some() {
                    crate::log_debug!(
                        "✅ External allOf $ref '{}' will be emitted as vendored JSON Schema validation",
                        ref_path
                    );
                    return Ok(());
                }
                return Err(unsupported_allof_shape(
                    format!("{}/$ref", schema_path),
                    "external $ref",
                    format!(
                        "Vendor external schema '{}' before compiling this allOf subschema, or pre-resolve the ref into the input schema.",
                        resource_uri
                    ),
                ));
            }
            if self.allof_ref_stack.contains(ref_path) {
                return Err(unsupported_allof_shape(
                    format!("{}/$ref", schema_path),
                    "recursive allOf $ref",
                    format!(
                        "Break the recursive allOf reference cycle involving '{}' or model it without allOf property merging.",
                        ref_path
                    ),
                ));
            }
            self.check_schema_depth(self.allof_ref_stack.len() + 1, schema_path)?;

            self.allof_ref_stack.insert(ref_path.to_string());
            let result = (|| {
                let resolved =
                    self.resolve_allof_ref(ref_path, &format!("{}/$ref", schema_path))?;
                validate_supported_allof_subschema(
                    &resolved,
                    &format!("{}/$ref", schema_path),
                    allow_implicit_object,
                )?;
                self.discover_struct_members_internal(
                    &resolved,
                    type_discovery,
                    true,
                    &format!("{}/$ref", schema_path),
                )
            })();
            self.allof_ref_stack.remove(ref_path);
            return result;
        }

        if subschema.contains_key("oneOf") {
            return Err(unsupported_allof_shape(
                format!("{}/oneOf", schema_path),
                "oneOf",
                "Implement exact oneOf validation for allOf subschemas before compiling this shape.",
            ));
        }

        if subschema.contains_key("not") {
            validate_supported_allof_not_required_subschema(subschema, schema_path)?;
            return Ok(());
        }

        if subschema.contains_key("if")
            || subschema.contains_key("then")
            || subschema.contains_key("else")
        {
            validate_supported_allof_if_then_subschema(subschema, schema_path)?;
            return Ok(());
        }

        if let Some(anyof_value) = subschema.get("anyOf") {
            if !anyof_value.is_array() {
                return Err(unsupported_allof_shape(
                    format!("{}/anyOf", schema_path),
                    "anyOf",
                    "Use a JSON array for anyOf inside allOf subschemas.",
                ));
            }
            validate_supported_allof_anyof_subschema(subschema, schema_path)?;
            crate::log_debug!(
                "✅ allOf anyOf at '{}' will be emitted as generated JSON Schema validation",
                schema_path
            );
            return Ok(());
        }

        validate_supported_allof_subschema(subschema, schema_path, allow_implicit_object)?;

        // Recursively process this subschema's members. Skip additionalProperties
        // for allOf subschemas; only the containing object can add its catch-all map.
        self.discover_struct_members_internal(subschema, type_discovery, true, schema_path)
    }

    fn resolve_allof_ref(
        &self,
        ref_path: &str,
        schema_path: &str,
    ) -> TctResult<Map<String, Value>> {
        self.resolve_ref(ref_path).ok_or_else(|| {
            unsupported_allof_shape(
                schema_path,
                "$ref",
                format!(
                    "Resolve local allOf references before compiling; unsupported or missing reference '{}'",
                    ref_path
                ),
            )
        })
    }

    pub(super) fn allof_array_declares_object(&self, allof_array: &[Value]) -> bool {
        allof_array.iter().any(|subschema| {
            let Some(subschema) = subschema.as_object() else {
                return false;
            };

            if schema_declares_exact_object(subschema) {
                return true;
            }

            let Some(ref_path) = subschema.get("$ref").and_then(Value::as_str) else {
                return false;
            };

            if ref_path.starts_with("http://") || ref_path.starts_with("https://") {
                return false;
            }

            self.resolve_ref(ref_path)
                .as_ref()
                .map(schema_declares_exact_object)
                .unwrap_or(false)
        })
    }

    pub(super) fn collect_allof_required_entries(
        &self,
        allof_array: &[Value],
        schema_path: &str,
    ) -> TctResult<Vec<AllOfRequiredEntry>> {
        let mut entries = Vec::new();
        let mut visited_refs = HashSet::new();

        for (index, subschema) in allof_array.iter().enumerate() {
            let Some(subschema) = subschema.as_object() else {
                continue;
            };

            self.collect_required_entries_from_allof_subschema(
                subschema,
                &format!("{}/allOf/{}", schema_path, index),
                &mut entries,
                &mut visited_refs,
            )?;
        }

        Ok(entries)
    }

    fn collect_required_entries_from_allof_subschema(
        &self,
        subschema: &Map<String, Value>,
        schema_path: &str,
        entries: &mut Vec<AllOfRequiredEntry>,
        visited_refs: &mut HashSet<String>,
    ) -> TctResult<()> {
        if let Some(ref_path) = subschema.get("$ref").and_then(Value::as_str) {
            if ref_path.starts_with("http://") || ref_path.starts_with("https://") {
                return Ok(());
            }

            self.check_schema_depth(visited_refs.len() + 1, schema_path)?;

            if !visited_refs.insert(ref_path.to_string()) {
                return Ok(());
            }

            let resolved = self.resolve_allof_ref(ref_path, &format!("{}/$ref", schema_path))?;
            self.collect_required_entries_from_allof_subschema(
                &resolved,
                &format!("{}/$ref", schema_path),
                entries,
                visited_refs,
            )?;
            visited_refs.remove(ref_path);
            return Ok(());
        }

        // These allOf forms have separate validation semantics; their nested
        // required arrays must not become unconditional required fields.
        if subschema.contains_key("not")
            || subschema.contains_key("if")
            || subschema.contains_key("then")
            || subschema.contains_key("else")
            || subschema.contains_key("anyOf")
            || subschema.contains_key("oneOf")
        {
            return Ok(());
        }

        if let Some(required) = subschema.get("required") {
            let Some(required) = required.as_array() else {
                return Err(unsupported_allof_shape(
                    format!("{}/required", schema_path),
                    "required",
                    "Use a JSON array of property names for required in allOf subschemas.",
                ));
            };

            for (index, required_name) in required.iter().enumerate() {
                let Some(required_name) = required_name.as_str() else {
                    return Err(unsupported_allof_shape(
                        format!("{}/required/{}", schema_path, index),
                        "required",
                        "Use only string property names in required arrays.",
                    ));
                };

                entries.push(AllOfRequiredEntry {
                    property_name: required_name.to_string(),
                    schema_path: format!("{}/required/{}", schema_path, index),
                });
            }
        }

        if let Some(nested_allof) = subschema.get("allOf").and_then(Value::as_array) {
            for (index, nested_subschema) in nested_allof.iter().enumerate() {
                if let Some(nested_subschema) = nested_subschema.as_object() {
                    self.collect_required_entries_from_allof_subschema(
                        nested_subschema,
                        &format!("{}/allOf/{}", schema_path, index),
                        entries,
                        visited_refs,
                    )?;
                }
            }
        }

        Ok(())
    }

    pub(super) fn apply_allof_required_entries(
        type_discovery: &mut TypeDiscovery,
        entries: &[AllOfRequiredEntry],
    ) -> TctResult<()> {
        for entry in entries {
            let Some(member) = type_discovery
                .members
                .iter_mut()
                .find(|member| member.name == entry.property_name)
            else {
                return Err(unsupported_allof_shape(
                    entry.schema_path.clone(),
                    "required",
                    format!(
                        "Required property '{}' must be defined by the containing object or a mergeable allOf subschema before TCT can encode it exactly.",
                        entry.property_name
                    ),
                ));
            };

            let was_required = member.required;
            member.required = true;
            if !was_required {
                member.is_nullable = false;
            }
        }

        Ok(())
    }
}

fn validate_supported_allof_subschema(
    subschema: &Map<String, Value>,
    schema_path: &str,
    allow_implicit_object: bool,
) -> TctResult<()> {
    if let Some(type_value) = subschema.get("type") {
        if type_value.as_str() != Some("object") {
            return Err(unsupported_allof_shape(
                format!("{}/type", schema_path),
                "type",
                "TCT currently supports allOf subschemas only when they are object schemas with mergeable object members.",
            ));
        }
    } else if (subschema.contains_key("properties")
        || subschema.contains_key("patternProperties")
        || subschema.contains_key("required"))
        && !allow_implicit_object
    {
        return Err(unsupported_allof_shape(
            schema_path,
            "implicit object subschema",
            "Add \"type\": \"object\" to allOf subschemas, or combine them with an allOf subschema that explicitly constrains the containing value to an object.",
        ));
    }

    if let Some(additional_properties) = subschema.get("additionalProperties")
        && !(additional_properties.is_boolean() || additional_properties.is_object())
    {
        return Err(unsupported_allof_shape(
            format!("{}/additionalProperties", schema_path),
            "additionalProperties",
            "Use a boolean or schema object for additionalProperties inside allOf subschemas.",
        ));
    }

    for keyword in [
        "minProperties",
        "maxProperties",
        "propertyNames",
        "dependentRequired",
        "dependentSchemas",
        "dependencies",
        "unevaluatedProperties",
        "if",
        "then",
        "else",
        "enum",
        "const",
    ] {
        if subschema.contains_key(keyword) {
            return Err(unsupported_allof_shape(
                format!("{}/{}", schema_path, keyword),
                keyword,
                "Use object member schemas that TCT can merge exactly, or split this schema before compiling.",
            ));
        }
    }

    if let Some(required_value) = subschema.get("required") {
        let Some(required) = required_value.as_array() else {
            return Err(unsupported_allof_shape(
                format!("{}/required", schema_path),
                "required",
                "Use a JSON array of property names for required in allOf subschemas.",
            ));
        };

        for required_name in required {
            if required_name.as_str().is_none() {
                return Err(unsupported_allof_shape(
                    format!("{}/required", schema_path),
                    "required",
                    "Use only string property names in required arrays.",
                ));
            }
        }
    }

    Ok(())
}

fn validate_supported_allof_not_required_subschema(
    subschema: &Map<String, Value>,
    schema_path: &str,
) -> TctResult<()> {
    for key in subschema.keys() {
        if key != "not" && !is_allof_annotation_key(key) {
            return Err(unsupported_allof_shape(
                format!("{}/{}", schema_path, json_pointer_segment(key)),
                "not sibling",
                "TCT only supports allOf not subschemas when the allOf entry contains no validation keywords besides not.",
            ));
        }
    }

    let Some(not_schema) = subschema.get("not").and_then(Value::as_object) else {
        return Err(unsupported_allof_shape(
            format!("{}/not", schema_path),
            "not",
            "Use an object schema for not inside allOf.",
        ));
    };

    for key in not_schema.keys() {
        if key != "required" && !is_allof_annotation_key(key) {
            return Err(unsupported_allof_shape(
                format!("{}/not/{}", schema_path, json_pointer_segment(key)),
                "not",
                "TCT currently supports allOf not subschemas only for not schemas of the form {\"required\": [...]}."
                    .to_string(),
            ));
        }
    }

    let Some(required) = not_schema.get("required").and_then(Value::as_array) else {
        return Err(unsupported_allof_shape(
            format!("{}/not/required", schema_path),
            "not",
            "Use not with a required-property array inside allOf, for example {\"not\": {\"required\": [\"a\", \"b\"]}}.",
        ));
    };

    for required_name in required {
        if required_name.as_str().is_none() {
            return Err(unsupported_allof_shape(
                format!("{}/not/required", schema_path),
                "not",
                "Use only string property names in allOf not|required arrays.",
            ));
        }
    }

    Ok(())
}

fn validate_supported_allof_if_then_subschema(
    subschema: &Map<String, Value>,
    schema_path: &str,
) -> TctResult<()> {
    for key in subschema.keys() {
        if !matches!(key.as_str(), "if" | "then") && !is_allof_annotation_key(key) {
            return Err(unsupported_allof_shape(
                format!("{}/{}", schema_path, json_pointer_segment(key)),
                "if/then sibling",
                "TCT only supports allOf conditionals when the allOf entry contains if and optional then, with no else or validation siblings.",
            ));
        }
    }

    let Some(if_schema) = subschema.get("if").and_then(Value::as_object) else {
        return Err(unsupported_allof_shape(
            format!("{}/if", schema_path),
            "if",
            "Use an object schema for if inside allOf.",
        ));
    };
    validate_supported_allof_if_schema(if_schema, &format!("{}/if", schema_path))?;

    if let Some(then_schema) = subschema.get("then") {
        let Some(then_schema) = then_schema.as_object() else {
            return Err(unsupported_allof_shape(
                format!("{}/then", schema_path),
                "then",
                "Use an object schema for then inside allOf.",
            ));
        };
        validate_supported_allof_then_schema(then_schema, &format!("{}/then", schema_path))?;
    }

    Ok(())
}

fn validate_supported_allof_anyof_subschema(
    subschema: &Map<String, Value>,
    schema_path: &str,
) -> TctResult<()> {
    for key in subschema.keys() {
        if !matches!(key.as_str(), "anyOf" | "type") && !is_allof_annotation_key(key) {
            return Err(unsupported_allof_shape(
                format!("{}/{}", schema_path, json_pointer_segment(key)),
                "anyOf sibling",
                "TCT currently supports validation-only anyOf inside allOf when the allOf entry contains no validation siblings besides anyOf and optional object type.",
            ));
        }
    }

    if let Some(type_value) = subschema.get("type")
        && type_value.as_str() != Some("object")
    {
        return Err(unsupported_allof_shape(
            format!("{}/type", schema_path),
            "type",
            "Use object validation schemas for anyOf inside allOf.",
        ));
    }

    let Some(anyof_array) = subschema.get("anyOf").and_then(Value::as_array) else {
        return Err(unsupported_allof_shape(
            format!("{}/anyOf", schema_path),
            "anyOf",
            "Use a JSON array for anyOf inside allOf subschemas.",
        ));
    };

    for (index, branch_schema) in anyof_array.iter().enumerate() {
        validate_no_external_refs_in_allof_validation_schema(
            branch_schema,
            &format!("{}/anyOf/{}", schema_path, index),
        )?;
    }

    Ok(())
}

fn validate_no_external_refs_in_allof_validation_schema(
    schema: &Value,
    schema_path: &str,
) -> TctResult<()> {
    match schema {
        Value::Object(schema_obj) => {
            if let Some(ref_value) = schema_obj.get("$ref") {
                let Some(ref_path) = ref_value.as_str() else {
                    return Err(unsupported_allof_shape(
                        format!("{}/$ref", schema_path),
                        "$ref",
                        "Use string refs inside allOf anyOf validation schemas.",
                    ));
                };
                if ref_path.starts_with("http://") || ref_path.starts_with("https://") {
                    return Err(unsupported_allof_shape(
                        format!("{}/$ref", schema_path),
                        "$ref",
                        "Resolve or vendor external refs before compiling anyOf validation inside allOf.",
                    ));
                }
            }

            for (key, value) in schema_obj {
                validate_no_external_refs_in_allof_validation_schema(
                    value,
                    &format!("{}/{}", schema_path, json_pointer_segment(key)),
                )?;
            }
        }
        Value::Array(items) => {
            for (index, item) in items.iter().enumerate() {
                validate_no_external_refs_in_allof_validation_schema(
                    item,
                    &format!("{}/{}", schema_path, index),
                )?;
            }
        }
        _ => {}
    }

    Ok(())
}

fn validate_supported_allof_if_schema(
    if_schema: &Map<String, Value>,
    schema_path: &str,
) -> TctResult<()> {
    for key in if_schema.keys() {
        if !matches!(key.as_str(), "properties" | "required") && !is_allof_annotation_key(key) {
            return Err(unsupported_allof_shape(
                format!("{}/{}", schema_path, json_pointer_segment(key)),
                "if",
                "TCT currently supports allOf if schemas only for one required property with a const string condition.",
            ));
        }
    }

    let properties = if_schema
        .get("properties")
        .and_then(Value::as_object)
        .ok_or_else(|| {
            unsupported_allof_shape(
                format!("{}/properties", schema_path),
                "if",
                "Use an if schema with a properties object containing one const string property.",
            )
        })?;

    if properties.len() != 1 {
        return Err(unsupported_allof_shape(
            format!("{}/properties", schema_path),
            "if",
            "Use exactly one const string property in allOf if schemas.",
        ));
    }

    let (condition_property, condition_schema) = properties.iter().next().expect("len checked");
    let Some(condition_schema) = condition_schema.as_object() else {
        return Err(unsupported_allof_shape(
            format!(
                "{}/properties/{}",
                schema_path,
                json_pointer_segment(condition_property)
            ),
            "if",
            "Use an object schema for the conditional property.",
        ));
    };

    if !condition_schema
        .get("const")
        .is_some_and(|const_value| const_value.is_string())
    {
        return Err(unsupported_allof_shape(
            format!(
                "{}/properties/{}/const",
                schema_path,
                json_pointer_segment(condition_property)
            ),
            "if",
            "Use a string const value for allOf conditional property checks.",
        ));
    }

    let required = if_schema
        .get("required")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            unsupported_allof_shape(
                format!("{}/required", schema_path),
                "if",
                "Require the same property that carries the conditional const.",
            )
        })?;

    let condition_is_required = required
        .iter()
        .filter_map(Value::as_str)
        .any(|required_name| required_name == condition_property);
    if !condition_is_required {
        return Err(unsupported_allof_shape(
            format!("{}/required", schema_path),
            "if",
            format!(
                "The allOf conditional property '{}' must be listed in the if required array.",
                condition_property
            ),
        ));
    }

    Ok(())
}

fn validate_supported_allof_then_schema(
    then_schema: &Map<String, Value>,
    schema_path: &str,
) -> TctResult<()> {
    for key in then_schema.keys() {
        if !matches!(key.as_str(), "properties" | "required") && !is_allof_annotation_key(key) {
            return Err(unsupported_allof_shape(
                format!("{}/{}", schema_path, json_pointer_segment(key)),
                "then",
                "TCT currently supports allOf then schemas only for required arrays and property type constraints.",
            ));
        }
    }

    if let Some(required) = then_schema.get("required") {
        let Some(required) = required.as_array() else {
            return Err(unsupported_allof_shape(
                format!("{}/required", schema_path),
                "then",
                "Use a JSON array of property names for then.required.",
            ));
        };
        for required_name in required {
            if required_name.as_str().is_none() {
                return Err(unsupported_allof_shape(
                    format!("{}/required", schema_path),
                    "then",
                    "Use only string property names in then.required arrays.",
                ));
            }
        }
    }

    if let Some(properties) = then_schema.get("properties") {
        let Some(properties) = properties.as_object() else {
            return Err(unsupported_allof_shape(
                format!("{}/properties", schema_path),
                "then",
                "Use a JSON object for then.properties.",
            ));
        };

        for (property_name, property_schema) in properties {
            let Some(property_schema) = property_schema.as_object() else {
                return Err(unsupported_allof_shape(
                    format!(
                        "{}/properties/{}",
                        schema_path,
                        json_pointer_segment(property_name)
                    ),
                    "then",
                    "Use object schemas for then property constraints.",
                ));
            };
            let Some(type_name) = property_schema.get("type").and_then(Value::as_str) else {
                return Err(unsupported_allof_shape(
                    format!(
                        "{}/properties/{}/type",
                        schema_path,
                        json_pointer_segment(property_name)
                    ),
                    "then",
                    "Use simple JSON type constraints in then.properties.",
                ));
            };
            if !matches!(
                type_name,
                "string" | "boolean" | "number" | "integer" | "object" | "array" | "null"
            ) {
                return Err(unsupported_allof_shape(
                    format!(
                        "{}/properties/{}/type",
                        schema_path,
                        json_pointer_segment(property_name)
                    ),
                    "then",
                    "Use a supported JSON type constraint in then.properties.",
                ));
            }
        }
    }

    Ok(())
}
