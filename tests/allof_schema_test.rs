// Tests for allOf schema composition support
// The allOf construct requires that data be valid against ALL of the provided schemas

use std::fs;
use std::process::Command;

use serde_json::json;
use tct::TctCompiler;
use tct::generated_crate::{
    GeneratedCargoToml, configure_generated_cargo_command, copy_workspace_cargo_lock,
};

fn generated_contract_cargo_command() -> Command {
    let tct_path = std::env::current_dir().expect("failed to resolve repo root");
    let mut command = Command::new("cargo");
    configure_generated_cargo_command(&mut command, &tct_path);
    command
}

fn parse_generated_file(code: &str) -> syn::File {
    syn::parse_file(code).unwrap_or_else(|error| panic!("generated Rust should parse: {error}"))
}

fn path_ends_with_ident(path: &syn::Path, name: &str) -> bool {
    path.segments
        .last()
        .is_some_and(|segment| segment.ident == name)
}

fn type_ends_with_ident(ty: &syn::Type, name: &str) -> bool {
    matches!(ty, syn::Type::Path(type_path) if path_ends_with_ident(&type_path.path, name))
}

fn generated_file_has_struct(file: &syn::File, name: &str) -> bool {
    file.items.iter().any(|item| match item {
        syn::Item::Struct(item_struct) => item_struct.ident == name,
        _ => false,
    })
}

fn generated_file_has_trait_impl_for(file: &syn::File, type_name: &str, trait_name: &str) -> bool {
    file.items.iter().any(|item| match item {
        syn::Item::Impl(item_impl) => {
            type_ends_with_ident(&item_impl.self_ty, type_name)
                && item_impl
                    .trait_
                    .as_ref()
                    .is_some_and(|(_, path, _)| path_ends_with_ident(path, trait_name))
        }
        _ => false,
    })
}

/// Test basic allOf composition with simple properties
#[test]
fn test_allof_schema_allof_basic_composition() {
    let schema = json!({
        "title": "Person",
        "type": "object",
        "allOf": [
            {
                "type": "object",
                "properties": {
                    "name": {"type": "string"}
                },
                "required": ["name"]
            },
            {
                "type": "object",
                "properties": {
                    "age": {"type": "integer"}
                },
                "required": ["age"]
            }
        ]
    });

    let grammar = TctCompiler::new(schema, "Person")
        .unwrap()
        .compile()
        .expect("basic allOf composition should compile");
    let generated_code = grammar
        .generate_code()
        .expect("basic allOf composition should generate code");

    run_generated_allof_contract_crate(
        "allof-basic-composition",
        &generated_code,
        r#"
let valid = serde_json::json!({"name": "Ada", "age": 37});
let value: Person = serde_json::from_value(valid.clone()).map_err(|error| error.to_string())?;
let mut tokens = Vec::new();
value.encode(&mut tokens);
let mut iter = tokens.into_iter();
let decoded = Person::decode(&mut iter).map_err(|error| error.to_string())?;
let decoded_json = serde_json::to_value(decoded).map_err(|error| error.to_string())?;
if decoded_json != valid {
    return Err(format!("round trip mismatch: expected {valid}, got {decoded_json}"));
}

let missing_name = serde_json::json!({"age": 37});
if serde_json::from_value::<Person>(missing_name).is_ok() {
    return Err("basic allOf composition accepted a missing required name".to_string());
}

let missing_age = serde_json::json!({"name": "Ada"});
if serde_json::from_value::<Person>(missing_age).is_ok() {
    return Err("basic allOf composition accepted a missing required age".to_string());
}

Ok(())
"#,
    );

    println!("✅ Basic allOf composition test passed");
}

#[test]
fn test_allof_schema_preserves_large_unsigned_integer_property() {
    let schema = json!({
        "title": "AllOfLargeInteger",
        "type": "object",
        "allOf": [
            {
                "type": "object",
                "properties": {
                    "value": {"type": "integer"}
                },
                "required": ["value"]
            }
        ],
        "additionalProperties": false
    });

    let grammar = TctCompiler::new(schema, "AllOfLargeInteger")
        .unwrap()
        .compile()
        .expect("allOf large integer schema should compile");
    let generated_code = grammar
        .generate_code()
        .expect("allOf large integer schema should generate code");

    run_generated_allof_contract_crate(
        "allof-large-unsigned-integer",
        &generated_code,
        r#"
let valid = serde_json::json!({"value": u64::MAX});
let value: AllOfLargeInteger =
    serde_json::from_value(valid.clone()).map_err(|error| error.to_string())?;

let fractional = serde_json::json!({"value": 1.25});
if serde_json::from_value::<AllOfLargeInteger>(fractional).is_ok() {
    return Err("allOf integer property accepted a fractional number".to_string());
}

let mut tokens = Vec::new();
value.encode(&mut tokens);
if value.token_count() != tokens.len() {
    return Err(format!(
        "token_count mismatch: counted {}, encoded {}",
        value.token_count(),
        tokens.len()
    ));
}
let mut iter = tokens.into_iter();
let decoded = AllOfLargeInteger::decode(&mut iter).map_err(|error| error.to_string())?;
let decoded_json = serde_json::to_value(decoded).map_err(|error| error.to_string())?;
if decoded_json != valid {
    return Err(format!("round trip mismatch: expected {valid}, got {decoded_json}"));
}

Ok(())
"#,
    );
}

/// Test allOf with overlapping properties (should merge)
#[test]
fn test_allof_schema_allof_overlapping_properties() {
    let schema = json!({
        "title": "Contact",
        "type": "object",
        "additionalProperties": false,
        "allOf": [
            {
                "type": "object",
                "properties": {
                    "name": {"type": "string"},
                    "email": {"type": "string"}
                },
                "required": ["name"]
            },
            {
                "type": "object",
                "properties": {
                    "name": {"type": "string"},  // Same property in both schemas
                    "phone": {"type": "string"}
                },
                "required": ["phone"]
            }
        ]
    });

    let result = TctCompiler::new(schema.clone(), "Contact")
        .unwrap()
        .compile();
    assert!(result.is_ok(), "AllOf with overlapping properties failed");

    let grammar = result.unwrap();
    let code = grammar.generate_code();
    assert!(code.is_ok(), "Code generation for overlapping allOf failed");

    let generated_code = code.unwrap();
    run_generated_allof_contract_crate(
        "allof-overlapping-properties",
        &generated_code,
        r#"
let valid = serde_json::json!({"name": "Ada", "phone": "555-0100"});
let value: Contact = serde_json::from_value(valid.clone()).map_err(|error| error.to_string())?;
let mut tokens = Vec::new();
value.encode(&mut tokens);
let mut iter = tokens.into_iter();
let decoded = Contact::decode(&mut iter).map_err(|error| error.to_string())?;
let decoded_json = serde_json::to_value(decoded).map_err(|error| error.to_string())?;
if decoded_json != valid {
    return Err(format!("round trip mismatch: expected {valid}, got {decoded_json}"));
}

let with_optional = serde_json::json!({"name": "Ada", "phone": "555-0100", "email": "ada@example.com"});
serde_json::from_value::<Contact>(with_optional).map_err(|error| error.to_string())?;

let missing_name = serde_json::json!({"phone": "555-0100"});
if serde_json::from_value::<Contact>(missing_name).is_ok() {
    return Err("allOf merge accepted missing required duplicate property 'name'".to_string());
}

let missing_phone = serde_json::json!({"name": "Ada"});
if serde_json::from_value::<Contact>(missing_phone).is_ok() {
    return Err("allOf merge accepted missing required property 'phone'".to_string());
}

let unknown_field = serde_json::json!({"name": "Ada", "phone": "555-0100", "extra": true});
if serde_json::from_value::<Contact>(unknown_field).is_ok() {
    return Err("allOf merge ignored additionalProperties: false unknown field".to_string());
}

Ok(())
"#,
    );

    println!("✅ AllOf overlapping properties test passed");
}

/// Duplicate allOf properties with identical schemas should be merged into one field.
#[test]
fn test_allof_schema_allof_identical_duplicate_properties_merge_requiredness() {
    let schema = json!({
        "title": "Contact",
        "type": "object",
        "additionalProperties": false,
        "allOf": [
            {
                "type": "object",
                "properties": {
                    "name": {"type": "string"},
                    "email": {"type": "string"}
                }
            },
            {
                "type": "object",
                "properties": {
                    "name": {"type": "string"},
                    "phone": {"type": "string"}
                },
                "required": ["name", "phone"]
            }
        ]
    });

    let grammar = TctCompiler::new(schema, "Contact")
        .unwrap()
        .compile()
        .expect("identical duplicate allOf properties should compile");
    let generated_code = grammar
        .generate_code()
        .expect("merged allOf properties should generate code");

    run_generated_allof_contract_crate(
        "allof-identical-duplicate-properties",
        &generated_code,
        r#"
let valid = serde_json::json!({"name": "Ada", "phone": "555-0100", "email": "ada@example.com"});
let value: Contact = serde_json::from_value(valid.clone()).map_err(|error| error.to_string())?;
let mut tokens = Vec::new();
value.encode(&mut tokens);
let mut iter = tokens.into_iter();
let decoded = Contact::decode(&mut iter).map_err(|error| error.to_string())?;
let decoded_json = serde_json::to_value(decoded).map_err(|error| error.to_string())?;
if decoded_json != valid {
    return Err(format!("round trip mismatch: expected {valid}, got {decoded_json}"));
}

let without_optional = serde_json::json!({"name": "Ada", "phone": "555-0100"});
serde_json::from_value::<Contact>(without_optional).map_err(|error| error.to_string())?;

let missing_name = serde_json::json!({"phone": "555-0100"});
if serde_json::from_value::<Contact>(missing_name).is_ok() {
    return Err("duplicate allOf merge accepted a missing required name".to_string());
}

let missing_phone = serde_json::json!({"name": "Ada"});
if serde_json::from_value::<Contact>(missing_phone).is_ok() {
    return Err("duplicate allOf merge accepted a missing required phone".to_string());
}

let unknown = serde_json::json!({"name": "Ada", "phone": "555-0100", "extra": true});
if serde_json::from_value::<Contact>(unknown).is_ok() {
    return Err("duplicate allOf merge ignored additionalProperties: false".to_string());
}

Ok(())
"#,
    );

    println!("✅ AllOf identical duplicate properties merge requiredness");
}

/// Duplicate allOf properties with different schemas are currently unsupported.
#[test]
fn test_allof_schema_allof_conflicting_duplicate_properties_fail_with_path() {
    let schema = json!({
        "title": "ConflictingContact",
        "type": "object",
        "allOf": [
            {
                "type": "object",
                "properties": {
                    "id": {"type": "string"}
                }
            },
            {
                "type": "object",
                "properties": {
                    "id": {"type": "integer"}
                }
            }
        ]
    });

    let error = match TctCompiler::new(schema, "ConflictingContact")
        .unwrap()
        .compile()
    {
        Ok(_) => panic!("conflicting duplicate allOf properties should fail"),
        Err(error) => error,
    };
    let error_text = error.to_string();

    assert!(
        error_text.contains("Conflicting allOf property definitions for 'id'"),
        "expected conflicting-property message, got: {}",
        error_text
    );
    assert!(
        error_text.contains("#/allOf/1/properties/id"),
        "expected allOf schema path, got: {}",
        error_text
    );

    println!("✅ AllOf conflicting duplicate properties fail with path");
}

#[test]
fn test_allof_schema_allof_conflicting_duplicate_scalar_constraints_fail_with_path() {
    let schema = json!({
        "title": "ConflictingConstraints",
        "type": "object",
        "allOf": [
            {
                "type": "object",
                "properties": {
                    "count": {"type": "integer", "minimum": 0}
                }
            },
            {
                "type": "object",
                "properties": {
                    "count": {"type": "integer", "minimum": 1}
                }
            }
        ]
    });

    let error = match TctCompiler::new(schema, "ConflictingConstraints")
        .unwrap()
        .compile()
    {
        Ok(_) => panic!("duplicate allOf properties with different constraints should fail"),
        Err(error) => error,
    };
    let error_text = error.to_string();

    assert!(
        error_text.contains("Conflicting allOf property definitions for 'count'"),
        "expected conflicting-property message, got: {}",
        error_text
    );
    assert!(
        error_text.contains("#/allOf/1/properties/count"),
        "expected allOf schema path, got: {}",
        error_text
    );
}

#[test]
fn test_allof_schema_rejects_non_object_subschema_type() {
    let schema = json!({
        "title": "ImpossibleAllOf",
        "type": "object",
        "allOf": [
            {"type": "string"}
        ]
    });

    let error = match TctCompiler::new(schema, "ImpossibleAllOf")
        .unwrap()
        .compile()
    {
        Ok(_) => panic!("non-object allOf subschema should fail"),
        Err(error) => error,
    };
    let error_text = error.to_string();
    assert!(
        error_text.contains("Unsupported allOf subschema shape")
            && error_text.contains("#/allOf/0/type"),
        "expected allOf type error with path, got: {}",
        error_text
    );
}

#[test]
fn test_allof_schema_rejects_invalid_additional_properties_value() {
    let schema = json!({
        "title": "InvalidAdditionalPropertiesAllOf",
        "type": "object",
        "allOf": [
            {
                "type": "object",
                "properties": {
                    "name": {"type": "string"}
                },
                "additionalProperties": ["not", "a", "schema"]
            }
        ]
    });

    let error = match TctCompiler::new(schema, "InvalidAdditionalPropertiesAllOf")
        .unwrap()
        .compile()
    {
        Ok(_) => panic!("invalid allOf additionalProperties value should fail"),
        Err(error) => error,
    };
    let error_text = error.to_string();
    assert!(
        error_text.contains("additionalProperties")
            && error_text.contains("#/allOf/0/additionalProperties"),
        "expected allOf additionalProperties error with path, got: {}",
        error_text
    );
}

#[test]
fn test_allof_schema_additional_properties_constraints_are_enforced_by_deserialize() {
    let schema = json!({
        "title": "AdditionalPropertiesAllOf",
        "type": "object",
        "additionalProperties": true,
        "allOf": [
            {
                "type": "object",
                "properties": {
                    "name": {"type": "string"}
                },
                "additionalProperties": false
            },
            {
                "type": "object",
                "properties": {
                    "age": {"type": "integer"}
                }
            }
        ]
    });

    let grammar = TctCompiler::new(schema, "AdditionalPropertiesAllOf")
        .unwrap()
        .compile()
        .expect("allOf additionalProperties constraints should compile");
    let generated_code = grammar
        .generate_code()
        .expect("allOf additionalProperties constraints should generate code");
    assert_generated_allof_uses_cached_jsonschema_validator(&generated_code);

    run_generated_allof_contract_crate(
        "allof-additional-properties-false",
        &generated_code,
        r#"
let valid = serde_json::json!({"name": "Ada"});
let value: AdditionalPropertiesAllOf = serde_json::from_value(valid.clone()).map_err(|error| error.to_string())?;
let mut tokens = Vec::new();
value.encode(&mut tokens);
let mut iter = tokens.into_iter();
let decoded = AdditionalPropertiesAllOf::decode(&mut iter).map_err(|error| error.to_string())?;
let decoded_json = serde_json::to_value(decoded).map_err(|error| error.to_string())?;
if decoded_json != valid {
    return Err(format!("round trip mismatch: expected {valid}, got {decoded_json}"));
}

let sibling_property = serde_json::json!({"name": "Ada", "age": 42});
if serde_json::from_value::<AdditionalPropertiesAllOf>(sibling_property).is_ok() {
    return Err("allOf additionalProperties:false accepted a sibling allOf property".to_string());
}

let unknown_property = serde_json::json!({"name": "Ada", "extra": true});
if serde_json::from_value::<AdditionalPropertiesAllOf>(unknown_property).is_ok() {
    return Err("allOf additionalProperties:false accepted an unknown field".to_string());
}

Ok(())
"#,
    );

    let typed_schema = json!({
        "title": "TypedAdditionalPropertiesAllOf",
        "type": "object",
        "allOf": [
            {
                "type": "object",
                "properties": {
                    "name": {"type": "string"}
                },
                "additionalProperties": {"type": "string"}
            }
        ]
    });

    let grammar = TctCompiler::new(typed_schema, "TypedAdditionalPropertiesAllOf")
        .unwrap()
        .compile()
        .expect("allOf typed additionalProperties constraints should compile");
    let generated_code = grammar
        .generate_code()
        .expect("allOf typed additionalProperties constraints should generate code");
    assert_generated_allof_uses_cached_jsonschema_validator(&generated_code);

    run_generated_allof_contract_crate(
        "allof-additional-properties-typed",
        &generated_code,
        r#"
let valid = serde_json::json!({"name": "Ada", "extra": "ok"});
let value: TypedAdditionalPropertiesAllOf = serde_json::from_value(valid.clone()).map_err(|error| error.to_string())?;
let mut tokens = Vec::new();
value.encode(&mut tokens);
let mut iter = tokens.into_iter();
let decoded = TypedAdditionalPropertiesAllOf::decode(&mut iter).map_err(|error| error.to_string())?;
let decoded_json = serde_json::to_value(decoded).map_err(|error| error.to_string())?;
if decoded_json != valid {
    return Err(format!("round trip mismatch: expected {valid}, got {decoded_json}"));
}

let invalid = serde_json::json!({"name": "Ada", "extra": 42});
if serde_json::from_value::<TypedAdditionalPropertiesAllOf>(invalid).is_ok() {
    return Err("allOf typed additionalProperties accepted a non-string extra field".to_string());
}

Ok(())
"#,
    );
}

#[test]
fn test_allof_schema_cross_subschema_required_merges_existing_property() {
    let schema = json!({
        "title": "CrossRequiredAllOf",
        "type": "object",
        "additionalProperties": false,
        "allOf": [
            {
                "type": "object",
                "properties": {
                    "name": {"type": "string"}
                }
            },
            {
                "type": "object",
                "required": ["name"]
            }
        ]
    });

    let grammar = TctCompiler::new(schema, "CrossRequiredAllOf")
        .unwrap()
        .compile()
        .expect("cross-subschema allOf required should compile when property is generated");
    let generated_code = grammar
        .generate_code()
        .expect("cross-subschema allOf required should generate code");

    run_generated_allof_contract_crate(
        "allof-cross-required",
        &generated_code,
        r#"
let valid = serde_json::json!({"name": "Ada"});
let value: CrossRequiredAllOf = serde_json::from_value(valid.clone()).map_err(|error| error.to_string())?;
let mut tokens = Vec::new();
value.encode(&mut tokens);
let mut iter = tokens.into_iter();
let decoded = CrossRequiredAllOf::decode(&mut iter).map_err(|error| error.to_string())?;
let decoded_json = serde_json::to_value(decoded).map_err(|error| error.to_string())?;
if decoded_json != valid {
    return Err(format!("round trip mismatch: expected {valid}, got {decoded_json}"));
}

let missing_name = serde_json::json!({});
if serde_json::from_value::<CrossRequiredAllOf>(missing_name).is_ok() {
    return Err("cross-subschema allOf required accepted a missing property".to_string());
}

Ok(())
"#,
    );
}

#[test]
fn test_allof_schema_rejects_required_without_generated_member() {
    let schema = json!({
        "title": "MissingRequiredAllOf",
        "type": "object",
        "allOf": [
            {
                "type": "object",
                "required": ["missing"]
            }
        ]
    });

    let error = match TctCompiler::new(schema, "MissingRequiredAllOf")
        .unwrap()
        .compile()
    {
        Ok(_) => panic!("allOf required without a generated member should fail"),
        Err(error) => error,
    };
    let error_text = error.to_string();
    assert!(
        error_text.contains("required") && error_text.contains("#/allOf/0/required/0"),
        "expected allOf missing required-member error with path, got: {}",
        error_text
    );
}

#[test]
fn test_allof_schema_ref_subschema_merges_referenced_properties() {
    let schema = json!({
        "title": "MergedRef",
        "type": "object",
        "additionalProperties": false,
        "allOf": [
            { "$ref": "#/definitions/base" },
            {
                "type": "object",
                "properties": {
                    "enabled": { "type": "boolean" }
                },
                "required": ["enabled"]
            }
        ],
        "definitions": {
            "base": {
                "type": "object",
                "properties": {
                    "name": { "type": "string" }
                },
                "required": ["name"]
            }
        }
    });

    let grammar = TctCompiler::new(schema, "MergedRef")
        .unwrap()
        .compile()
        .expect("allOf $ref subschema should compile");
    let generated_code = grammar
        .generate_code()
        .expect("allOf $ref subschema should generate code");

    run_generated_allof_contract_crate(
        "allof-ref-round-trip",
        &generated_code,
        r#"
let input = serde_json::json!({"name": "alpha", "enabled": true});
let value: MergedRef = serde_json::from_value(input.clone()).map_err(|error| error.to_string())?;
let mut tokens = Vec::new();
value.encode(&mut tokens);
let mut iter = tokens.into_iter();
let decoded = MergedRef::decode(&mut iter).map_err(|error| error.to_string())?;
let decoded_json = serde_json::to_value(decoded).map_err(|error| error.to_string())?;
if decoded_json != input {
    return Err(format!("round trip mismatch: expected {input}, got {decoded_json}"));
}

let missing_name = serde_json::json!({"enabled": true});
if serde_json::from_value::<MergedRef>(missing_name).is_ok() {
    return Err("allOf ref merge accepted missing referenced required name".to_string());
}

let missing_enabled = serde_json::json!({"name": "alpha"});
if serde_json::from_value::<MergedRef>(missing_enabled).is_ok() {
    return Err("allOf ref merge accepted missing sibling required enabled".to_string());
}

Ok(())
"#,
    );
}

#[test]
fn test_allof_schema_rejects_ref_sibling_constraints() {
    let schema = json!({
        "title": "RefSiblingAllOf",
        "type": "object",
        "allOf": [
            {
                "$ref": "#/definitions/base",
                "properties": {
                    "enabled": { "type": "boolean" }
                }
            }
        ],
        "definitions": {
            "base": {
                "type": "object",
                "properties": {
                    "name": { "type": "string" }
                }
            }
        }
    });

    let error = match TctCompiler::new(schema, "RefSiblingAllOf")
        .unwrap()
        .compile()
    {
        Ok(_) => panic!("allOf $ref validation siblings should fail until exact semantics exist"),
        Err(error) => error,
    };
    let error_text = error.to_string();
    assert!(
        error_text.contains("$ref sibling") && error_text.contains("#/allOf/0/properties"),
        "expected allOf $ref sibling error with path, got: {}",
        error_text
    );
}

#[test]
fn test_allof_schema_rejects_recursive_ref_cycle() {
    let schema = json!({
        "title": "RecursiveAllOf",
        "type": "object",
        "properties": {
            "entry": { "$ref": "#/definitions/LoopA" }
        },
        "definitions": {
            "LoopA": {
                "allOf": [
                    { "$ref": "#/definitions/LoopB" },
                    {
                        "type": "object",
                        "properties": {
                            "a": { "type": "string" }
                        }
                    }
                ]
            },
            "LoopB": {
                "allOf": [
                    { "$ref": "#/definitions/LoopA" },
                    {
                        "type": "object",
                        "properties": {
                            "b": { "type": "number" }
                        }
                    }
                ]
            }
        }
    });

    let error = match TctCompiler::new(schema, "RecursiveAllOf")
        .unwrap()
        .compile()
    {
        Ok(_) => panic!("recursive allOf $ref cycle should fail without stack overflow"),
        Err(error) => error,
    };
    let error_text = error.to_string();
    assert!(
        error_text.contains("recursive allOf $ref") && error_text.contains("#/allOf/0/$ref"),
        "expected recursive allOf $ref error, got: {}",
        error_text
    );
}

#[test]
fn test_allof_schema_rejects_required_choice_subschema() {
    let schema = json!({
        "title": "RequiredChoiceAllOf",
        "type": "object",
        "allOf": [
            {
                "oneOf": [
                    {
                        "type": "object",
                        "properties": {
                            "mode": { "type": "string" }
                        },
                        "required": ["mode"]
                    },
                    {
                        "type": "object",
                        "properties": {
                            "count": { "type": "integer" }
                        }
                    }
                ]
            }
        ]
    });

    let error = match TctCompiler::new(schema, "RequiredChoiceAllOf")
        .unwrap()
        .compile()
    {
        Ok(_) => panic!("allOf oneOf with required branch should fail until exact semantics exist"),
        Err(error) => error,
    };
    let error_text = error.to_string();
    assert!(
        error_text.contains("oneOf") && error_text.contains("#/allOf/0/oneOf"),
        "expected allOf oneOf error with path, got: {}",
        error_text
    );
}

#[test]
fn test_allof_schema_rejects_unvendored_external_ref() {
    let schema = json!({
        "title": "UnvendoredExternalAllOf",
        "type": "object",
        "allOf": [
            {
                "$ref": "https://example.com/not-vendored-for-tct.json"
            }
        ]
    });

    let error = match TctCompiler::new(schema, "UnvendoredExternalAllOf")
        .unwrap()
        .compile()
    {
        Ok(_) => panic!("unvendored external allOf $ref should fail"),
        Err(error) => error,
    };
    let error_text = error.to_string();
    assert!(
        error_text.contains("external $ref") && error_text.contains("#/allOf/0/$ref"),
        "expected unvendored external allOf $ref error with path, got: {}",
        error_text
    );
}

#[test]
fn test_allof_schema_vendored_external_ref_validation_is_enforced_by_deserialize() {
    let schema = json!({
        "title": "PluginRules",
        "type": "object",
        "additionalProperties": true,
        "allOf": [
            {
                "$ref": "https://json.schemastore.org/partial-eslint-plugins.json"
            }
        ]
    });

    let grammar = TctCompiler::new(schema, "PluginRules")
        .unwrap()
        .compile()
        .expect("vendored external allOf $ref should compile");
    let generated_code = grammar
        .generate_code()
        .expect("vendored external allOf validation should generate code");
    assert_generated_allof_uses_cached_jsonschema_validator(&generated_code);

    run_generated_allof_contract_crate(
        "allof-vendored-external-ref-validation",
        &generated_code,
        r#"
let valid = serde_json::json!({"@typescript-eslint/no-unused-vars": "warn"});
let value: PluginRules = serde_json::from_value(valid.clone()).map_err(|error| error.to_string())?;
let mut tokens = Vec::new();
value.encode(&mut tokens);
let mut iter = tokens.into_iter();
let decoded = PluginRules::decode(&mut iter).map_err(|error| error.to_string())?;
let decoded_json = serde_json::to_value(decoded).map_err(|error| error.to_string())?;
if decoded_json != valid {
    return Err(format!("round trip mismatch: expected {valid}, got {decoded_json}"));
}

let invalid = serde_json::json!({"@typescript-eslint/no-unused-vars": "not-a-rule-level"});
if serde_json::from_value::<PluginRules>(invalid).is_ok() {
    return Err("vendored external allOf validation accepted an invalid plugin rule value".to_string());
}

Ok(())
"#,
    );
}

#[test]
fn test_allof_schema_anyof_validation_subschema_is_enforced_by_deserialize() {
    let schema = json!({
        "title": "AnyOfAllOf",
        "type": "object",
        "additionalProperties": false,
        "properties": {
            "a": {},
            "b": {}
        },
        "allOf": [
            {
                "anyOf": [
                    { "$ref": "#/definitions/aBranch" },
                    { "$ref": "#/definitions/bBranch" }
                ]
            }
        ],
        "definitions": {
            "aBranch": {
                "type": "object",
                "properties": {
                    "a": { "type": "string" }
                },
                "required": ["a"]
            },
            "bBranch": {
                "type": "object",
                "properties": {
                    "b": { "type": "number" }
                },
                "required": ["b"]
            }
        }
    });

    let grammar = TctCompiler::new(schema, "AnyOfAllOf")
        .unwrap()
        .compile()
        .expect("allOf anyOf validation subschema should compile");
    let generated_code = grammar
        .generate_code()
        .expect("allOf anyOf validation subschema should generate code");

    run_generated_allof_contract_crate(
        "allof-anyof-validation",
        &generated_code,
        r#"
let valid_a = serde_json::json!({"a": "ok"});
let value: AnyOfAllOf = serde_json::from_value(valid_a.clone()).map_err(|error| error.to_string())?;
let mut tokens = Vec::new();
value.encode(&mut tokens);
let mut iter = tokens.into_iter();
let decoded = AnyOfAllOf::decode(&mut iter).map_err(|error| error.to_string())?;
let decoded_json = serde_json::to_value(decoded).map_err(|error| error.to_string())?;
if decoded_json != valid_a {
    return Err(format!("round trip mismatch: expected {valid_a}, got {decoded_json}"));
}

let valid_b = serde_json::json!({"b": 1.25});
serde_json::from_value::<AnyOfAllOf>(valid_b).map_err(|error| error.to_string())?;

let invalid_a = serde_json::json!({"a": 7});
if serde_json::from_value::<AnyOfAllOf>(invalid_a).is_ok() {
    return Err("allOf anyOf validation accepted non-string a without valid b".to_string());
}

let invalid_b = serde_json::json!({"b": "not a number"});
if serde_json::from_value::<AnyOfAllOf>(invalid_b).is_ok() {
    return Err("allOf anyOf validation accepted non-number b without valid a".to_string());
}

let unknown_field = serde_json::json!({"a": "ok", "extra": true});
if serde_json::from_value::<AnyOfAllOf>(unknown_field).is_ok() {
    return Err("allOf anyOf validation ignored additionalProperties: false unknown field".to_string());
}

Ok(())
"#,
    );
}

#[test]
fn test_allof_schema_not_required_constraints_are_enforced_by_deserialize() {
    let schema = json!({
        "title": "RefFilters",
        "type": "object",
        "additionalProperties": false,
        "properties": {
            "branches": {
                "type": "array",
                "items": { "type": "string" }
            },
            "branches-ignore": {
                "type": "array",
                "items": { "type": "string" }
            },
            "tags": {
                "type": "array",
                "items": { "type": "string" }
            },
            "tags-ignore": {
                "type": "array",
                "items": { "type": "string" }
            }
        },
        "allOf": [
            {
                "not": {
                    "required": ["branches", "branches-ignore"]
                }
            },
            {
                "not": {
                    "required": ["tags", "tags-ignore"]
                }
            }
        ]
    });

    let grammar = TctCompiler::new(schema, "RefFilters")
        .unwrap()
        .compile()
        .expect("allOf not|required constraints should compile");
    let generated_code = grammar
        .generate_code()
        .expect("allOf not|required constraints should generate code");

    run_generated_allof_contract_crate(
        "allof-not-required-validation",
        &generated_code,
        r#"
let valid = serde_json::json!({"branches": ["main"], "tags-ignore": ["v*"]});
let value: RefFilters = serde_json::from_value(valid.clone()).map_err(|error| error.to_string())?;
let mut tokens = Vec::new();
value.encode(&mut tokens);
let mut iter = tokens.into_iter();
let decoded = RefFilters::decode(&mut iter).map_err(|error| error.to_string())?;
let decoded_json = serde_json::to_value(decoded).map_err(|error| error.to_string())?;
if decoded_json != valid {
    return Err(format!("round trip mismatch: expected {valid}, got {decoded_json}"));
}

let invalid = serde_json::json!({"branches": ["main"], "branches-ignore": ["release"]});
if serde_json::from_value::<RefFilters>(invalid).is_ok() {
    return Err("allOf not|required constraint accepted mutually exclusive branch fields".to_string());
}

let invalid_tags = serde_json::json!({"tags": ["v1"], "tags-ignore": ["v2"]});
if serde_json::from_value::<RefFilters>(invalid_tags).is_ok() {
    return Err("allOf not|required constraint accepted mutually exclusive tag fields".to_string());
}

let invalid_from_tokens = RefFilters {
    branches: Some(vec!["main".to_string()]),
    branches_ignore: Some(vec!["release".to_string()]),
    tags: None,
    tags_ignore: None,
};
let mut invalid_tokens = Vec::new();
invalid_from_tokens.encode(&mut invalid_tokens);
invalid_tokens.push(9001);
let mut iter = invalid_tokens.into_iter();
let decoded_invalid = RefFilters::decode(&mut iter);
if let Ok(decoded) = decoded_invalid {
    let decoded_json = serde_json::to_value(decoded).map_err(|error| error.to_string())?;
    return Err(format!("decode bypassed allOf not|required validation: {decoded_json}"));
}
if iter.next() != Some(9001) {
    return Err("decode consumed tokens beyond the invalid allOf object".to_string());
}

Ok(())
"#,
    );
}

#[test]
fn test_allof_schema_decode_revalidates_minimal_not_required_constraint() {
    let schema = json!({
        "title": "NoBoth",
        "type": "object",
        "additionalProperties": false,
        "properties": {
            "a": {},
            "b": {}
        },
        "allOf": [
            {
                "not": {
                    "required": ["a", "b"]
                }
            }
        ]
    });

    let grammar = TctCompiler::new(schema, "NoBoth")
        .unwrap()
        .compile()
        .expect("minimal allOf not|required constraints should compile");
    let generated_code = grammar
        .generate_code()
        .expect("minimal allOf not|required constraints should generate code");

    run_generated_allof_contract_crate(
        "allof-minimal-decode-validation",
        &generated_code,
        r#"
let invalid_json = serde_json::json!({"a": null, "b": null});
if serde_json::from_value::<NoBoth>(invalid_json).is_ok() {
    return Err("serde accepted invalid allOf not|required object".to_string());
}

let tokens = vec![4_u32, 4, 9001];
let total = tokens.len();
let mut iter = tokens.into_iter();

let decoded = NoBoth::decode(&mut iter);
let consumed = total - iter.len();

if let Ok(decoded) = decoded {
    let decoded_json = serde_json::to_value(decoded).map_err(|error| error.to_string())?;
    return Err(format!("decode bypassed allOf not|required validation: {decoded_json}"));
}
if consumed != 2 {
    return Err(format!("expected 2 consumed object tokens before validation failure, got {consumed}"));
}
if iter.next() != Some(9001) {
    return Err("decode consumed sentinel after invalid object".to_string());
}

Ok(())
"#,
    );
}

#[test]
fn test_allof_schema_if_then_constraints_are_enforced_by_deserialize() {
    let schema = json!({
        "title": "DispatchInput",
        "type": "object",
        "additionalProperties": false,
        "properties": {
            "type": {
                "type": "string",
                "enum": ["string", "choice", "boolean", "number", "environment"]
            },
            "default": {},
            "options": {
                "type": "array",
                "items": { "type": "string" }
            }
        },
        "allOf": [
            {
                "if": {
                    "properties": {
                        "type": { "const": "boolean" }
                    },
                    "required": ["type"]
                },
                "then": {
                    "properties": {
                        "default": { "type": "boolean" }
                    }
                }
            },
            {
                "if": {
                    "properties": {
                        "type": { "const": "choice" }
                    },
                    "required": ["type"]
                },
                "then": {
                    "required": ["options"]
                }
            }
        ]
    });

    let grammar = TctCompiler::new(schema, "DispatchInput")
        .unwrap()
        .compile()
        .expect("allOf if/then constraints should compile");
    let generated_code = grammar
        .generate_code()
        .expect("allOf if/then constraints should generate code");

    run_generated_allof_contract_crate(
        "allof-if-then-validation",
        &generated_code,
        r#"
let valid = serde_json::json!({"type": "boolean", "default": true});
let value: DispatchInput = serde_json::from_value(valid.clone()).map_err(|error| error.to_string())?;
let mut tokens = Vec::new();
value.encode(&mut tokens);
let mut iter = tokens.into_iter();
let decoded = DispatchInput::decode(&mut iter).map_err(|error| error.to_string())?;
let decoded_json = serde_json::to_value(decoded).map_err(|error| error.to_string())?;
if decoded_json != valid {
    return Err(format!("round trip mismatch: expected {valid}, got {decoded_json}"));
}

let invalid_default = serde_json::json!({"type": "boolean", "default": "true"});
if serde_json::from_value::<DispatchInput>(invalid_default).is_ok() {
    return Err("allOf if/then constraint accepted boolean input with string default".to_string());
}

let invalid_choice = serde_json::json!({"type": "choice"});
if serde_json::from_value::<DispatchInput>(invalid_choice).is_ok() {
    return Err("allOf if/then constraint accepted choice input without options".to_string());
}

Ok(())
"#,
    );
}

#[test]
fn test_allof_schema_rejects_implicit_object_without_object_enforcer() {
    let schema = json!({
        "title": "ImplicitObjectOnly",
        "allOf": [
            {
                "properties": {
                    "name": { "type": "string" }
                }
            }
        ]
    });

    let error = match TctCompiler::new(schema, "ImplicitObjectOnly")
        .unwrap()
        .compile()
    {
        Ok(_) => panic!("implicit allOf object subschema should fail without object enforcer"),
        Err(error) => error,
    };
    let error_text = error.to_string();
    assert!(
        error_text.contains("implicit object") && error_text.contains("#/allOf/0"),
        "expected implicit object allOf error with path, got: {}",
        error_text
    );
}

/// Test allOf with pattern properties
#[test]
fn test_allof_schema_allof_with_pattern_properties() {
    let schema = json!({
        "title": "Configuration",
        "type": "object",
        "allOf": [
            {
                "type": "object",
                "properties": {
                    "version": {"type": "string"}
                },
                "required": ["version"]
            },
            {
                "type": "object",
                "patternProperties": {
                    "^env_": {"type": "string"}
                }
            }
        ]
    });

    let grammar = TctCompiler::new(schema, "Configuration")
        .unwrap()
        .compile()
        .expect("allOf with pattern properties should compile");
    let generated_code = grammar
        .generate_code()
        .expect("allOf with pattern properties should generate code");

    run_generated_allof_contract_crate(
        "allof-pattern-properties",
        &generated_code,
        r#"
let valid = serde_json::json!({"version": "1.0", "env_MODE": "prod"});
let value: Configuration = serde_json::from_value(valid.clone()).map_err(|error| error.to_string())?;
let mut tokens = Vec::new();
value.encode(&mut tokens);
let mut iter = tokens.into_iter();
let decoded = Configuration::decode(&mut iter).map_err(|error| error.to_string())?;
let decoded_json = serde_json::to_value(decoded).map_err(|error| error.to_string())?;
if decoded_json != valid {
    return Err(format!("round trip mismatch: expected {valid}, got {decoded_json}"));
}

let missing_version = serde_json::json!({"env_MODE": "prod"});
if serde_json::from_value::<Configuration>(missing_version).is_ok() {
    return Err("allOf patternProperties accepted a missing required version".to_string());
}

let invalid_env_value = serde_json::json!({"version": "1.0", "env_MODE": 7});
if serde_json::from_value::<Configuration>(invalid_env_value).is_ok() {
    return Err("allOf patternProperties accepted a non-string env value".to_string());
}

Ok(())
"#,
    );

    println!("✅ AllOf with pattern properties test passed");
}

/// Test allOf with nested objects
#[test]
fn test_allof_schema_allof_nested_objects() {
    let schema = json!({
        "title": "ComplexObject",
        "type": "object",
        "allOf": [
            {
                "type": "object",
                "properties": {
                    "basic": {"type": "string"},
                    "metadata": {
                        "type": "object",
                        "properties": {
                            "created": {"type": "string"}
                        }
                    }
                }
            },
            {
                "type": "object",
                "properties": {
                    "status": {"type": "string"},
                    "config": {
                        "type": "object",
                        "properties": {
                            "enabled": {"type": "boolean"}
                        }
                    }
                }
            }
        ]
    });

    let grammar = TctCompiler::new(schema, "ComplexObject")
        .unwrap()
        .compile()
        .expect("allOf with nested objects should compile");
    let generated_code = grammar
        .generate_code()
        .expect("allOf with nested objects should generate code");

    run_generated_allof_contract_crate(
        "allof-nested-objects",
        &generated_code,
        r#"
let valid = serde_json::json!({
    "basic": "base",
    "metadata": {"created": "today"},
    "status": "ready",
    "config": {"enabled": true}
});
let value: ComplexObject = serde_json::from_value(valid.clone()).map_err(|error| error.to_string())?;
let mut tokens = Vec::new();
value.encode(&mut tokens);
let mut iter = tokens.into_iter();
let decoded = ComplexObject::decode(&mut iter).map_err(|error| error.to_string())?;
let decoded_json = serde_json::to_value(decoded).map_err(|error| error.to_string())?;
if decoded_json != valid {
    return Err(format!("round trip mismatch: expected {valid}, got {decoded_json}"));
}

serde_json::from_value::<ComplexObject>(serde_json::json!({})).map_err(|error| error.to_string())?;

let invalid_nested_value = serde_json::json!({"config": {"enabled": "yes"}});
if serde_json::from_value::<ComplexObject>(invalid_nested_value).is_ok() {
    return Err("allOf nested object accepted a non-boolean config.enabled".to_string());
}

Ok(())
"#,
    );

    println!("✅ AllOf with nested objects test passed");
}

/// Test allOf with three or more schemas
#[test]
fn test_allof_schema_allof_multiple_schemas() {
    let schema = json!({
        "title": "MultiMerged",
        "type": "object",
        "allOf": [
            {
                "type": "object",
                "properties": {"field1": {"type": "string"}},
                "required": ["field1"]
            },
            {
                "type": "object",
                "properties": {"field2": {"type": "integer"}},
                "required": ["field2"]
            },
            {
                "type": "object",
                "properties": {"field3": {"type": "boolean"}},
                "required": ["field3"]
            }
        ]
    });

    let grammar = TctCompiler::new(schema, "MultiMerged")
        .unwrap()
        .compile()
        .expect("allOf with multiple schemas should compile");
    let generated_code = grammar
        .generate_code()
        .expect("allOf with multiple schemas should generate code");

    run_generated_allof_contract_crate(
        "allof-multiple-schemas",
        &generated_code,
        r#"
let valid = serde_json::json!({"field1": "alpha", "field2": 42, "field3": true});
let value: MultiMerged = serde_json::from_value(valid.clone()).map_err(|error| error.to_string())?;
let mut tokens = Vec::new();
value.encode(&mut tokens);
let mut iter = tokens.into_iter();
let decoded = MultiMerged::decode(&mut iter).map_err(|error| error.to_string())?;
let decoded_json = serde_json::to_value(decoded).map_err(|error| error.to_string())?;
if decoded_json != valid {
    return Err(format!("round trip mismatch: expected {valid}, got {decoded_json}"));
}

for (field, invalid) in [
    ("field1", serde_json::json!({"field2": 42, "field3": true})),
    ("field2", serde_json::json!({"field1": "alpha", "field3": true})),
    ("field3", serde_json::json!({"field1": "alpha", "field2": 42})),
] {
    if serde_json::from_value::<MultiMerged>(invalid).is_ok() {
        return Err(format!("three-way allOf merge accepted a missing required {field}"));
    }
}

Ok(())
"#,
    );

    println!("✅ AllOf with multiple schemas test passed");
}

/// Test allOf property within another object
#[test]
fn test_allof_schema_allof_as_property() {
    let schema = json!({
        "title": "Container",
        "type": "object",
        "properties": {
            "id": {"type": "string"},
            "merged_data": {
                "allOf": [
                    {
                        "type": "object",
                        "properties": {"name": {"type": "string"}}
                    },
                    {
                        "type": "object",
                        "properties": {"value": {"type": "integer"}}
                    }
                ]
            }
        },
        "required": ["id"]
    });

    let grammar = TctCompiler::new(schema, "Container")
        .unwrap()
        .compile()
        .expect("allOf as property should compile");
    let generated_code = grammar
        .generate_code()
        .expect("allOf as property should generate code");

    run_generated_allof_contract_crate(
        "allof-as-property",
        &generated_code,
        r#"
let valid = serde_json::json!({"id": "item-1", "merged_data": {"name": "widget", "value": 9}});
let value: Container = serde_json::from_value(valid.clone()).map_err(|error| error.to_string())?;
let mut tokens = Vec::new();
value.encode(&mut tokens);
let mut iter = tokens.into_iter();
let decoded = Container::decode(&mut iter).map_err(|error| error.to_string())?;
let decoded_json = serde_json::to_value(decoded).map_err(|error| error.to_string())?;
if decoded_json != valid {
    return Err(format!("round trip mismatch: expected {valid}, got {decoded_json}"));
}

serde_json::from_value::<Container>(serde_json::json!({"id": "item-1"})).map_err(|error| error.to_string())?;

let invalid_nested_value = serde_json::json!({"id": "item-1", "merged_data": {"name": "widget", "value": "nine"}});
if serde_json::from_value::<Container>(invalid_nested_value).is_ok() {
    return Err("nested allOf property accepted a non-integer value".to_string());
}

Ok(())
"#,
    );

    println!("✅ AllOf as property test passed");
}

/// Test allOf with enum properties
#[test]
fn test_allof_schema_allof_with_enums() {
    let schema = json!({
        "title": "EnumMerged",
        "type": "object",
        "allOf": [
            {
                "type": "object",
                "properties": {
                    "status": {"enum": ["active", "inactive"]}
                }
            },
            {
                "type": "object",
                "properties": {
                    "priority": {"enum": ["low", "medium", "high"]}
                }
            }
        ]
    });

    let grammar = TctCompiler::new(schema, "EnumMerged")
        .unwrap()
        .compile()
        .expect("allOf with enums should compile");
    let generated_code = grammar
        .generate_code()
        .expect("allOf with enums should generate code");

    run_generated_allof_contract_crate(
        "allof-with-enums",
        &generated_code,
        r#"
let valid = serde_json::json!({"status": "active", "priority": "high"});
let value: EnumMerged = serde_json::from_value(valid.clone()).map_err(|error| error.to_string())?;
let mut tokens = Vec::new();
value.encode(&mut tokens);
let mut iter = tokens.into_iter();
let decoded = EnumMerged::decode(&mut iter).map_err(|error| error.to_string())?;
let decoded_json = serde_json::to_value(decoded).map_err(|error| error.to_string())?;
if decoded_json != valid {
    return Err(format!("round trip mismatch: expected {valid}, got {decoded_json}"));
}

serde_json::from_value::<EnumMerged>(serde_json::json!({})).map_err(|error| error.to_string())?;

let invalid_status = serde_json::json!({"status": "paused", "priority": "high"});
if serde_json::from_value::<EnumMerged>(invalid_status).is_ok() {
    return Err("allOf enum property accepted an unknown status".to_string());
}

let invalid_priority = serde_json::json!({"status": "active", "priority": "urgent"});
if serde_json::from_value::<EnumMerged>(invalid_priority).is_ok() {
    return Err("allOf enum property accepted an unknown priority".to_string());
}

Ok(())
"#,
    );

    println!("✅ AllOf with enums test passed");
}

/// Test performance of allOf with complex merging
#[test]
fn test_allof_schema_allof_performance() {
    let schema = json!({
        "title": "LargeAllOf",
        "type": "object",
        "allOf": [
            {
                "type": "object",
                "properties": {
                    "prop1": {"type": "string"}, "prop2": {"type": "integer"},
                    "prop3": {"type": "boolean"}, "prop4": {"type": "string"}
                },
                "required": ["prop1", "prop2"]
            },
            {
                "type": "object",
                "properties": {
                    "prop5": {"type": "string"}, "prop6": {"type": "integer"},
                    "prop7": {"type": "boolean"}, "prop8": {"type": "string"}
                },
                "required": ["prop5", "prop6"]
            },
            {
                "type": "object",
                "properties": {
                    "prop9": {"type": "string"}, "prop10": {"type": "integer"}
                },
                "patternProperties": {
                    "^extra_": {"type": "string"}
                }
            }
        ]
    });

    let start = std::time::Instant::now();
    let result = TctCompiler::new(schema.clone(), "LargeAllOf")
        .unwrap()
        .compile();
    let generation_time = start.elapsed();

    assert!(result.is_ok(), "Large allOf performance test failed");

    let grammar = result.unwrap();
    let start = std::time::Instant::now();
    let code = grammar.generate_code();
    let code_gen_time = start.elapsed();

    assert!(code.is_ok(), "Large allOf code generation failed");
    let generated_code = code.unwrap();
    let generated_file = parse_generated_file(&generated_code);
    assert!(generated_file_has_struct(&generated_file, "LargeAllOf"));
    assert!(generated_file_has_trait_impl_for(
        &generated_file,
        "LargeAllOf",
        "JsonTokenEncoder"
    ));

    println!("✅ AllOf performance test passed");
    println!("   Schema generation: {:?}", generation_time);
    println!("   Code generation: {:?}", code_gen_time);
}

/// Test allOf integration with oneOf and enums
#[test]
fn test_allof_schema_allof_integration_with_other_features() {
    let schema = json!({
        "title": "IntegratedSchema",
        "type": "object",
        "allOf": [
            {
                "type": "object",
                "properties": {
                    "name": {"type": "string"},
                    "status": {"enum": ["active", "inactive"]}
                },
                "required": ["name"]
            },
            {
                "type": "object",
                "properties": {
                    "metadata": {
                        "oneOf": [
                            {"type": "string"},
                            {
                                "type": "object",
                                "properties": {"version": {"type": "string"}}
                            }
                        ]
                    }
                },
                "patternProperties": {
                    "^config_": {"type": "string"}
                }
            }
        ]
    });

    let grammar = TctCompiler::new(schema, "IntegratedSchema")
        .unwrap()
        .compile()
        .expect("allOf integration with other features should compile");
    let generated_code = grammar
        .generate_code()
        .expect("allOf integration with other features should generate code");

    run_generated_allof_contract_crate(
        "allof-integrated-features",
        &generated_code,
        r#"
let valid_string_metadata = serde_json::json!({
    "name": "service",
    "status": "active",
    "metadata": "v1",
    "config_mode": "prod"
});
let value: IntegratedSchema = serde_json::from_value(valid_string_metadata.clone()).map_err(|error| error.to_string())?;
let mut tokens = Vec::new();
value.encode(&mut tokens);
let mut iter = tokens.into_iter();
let decoded = IntegratedSchema::decode(&mut iter).map_err(|error| error.to_string())?;
let decoded_json = serde_json::to_value(decoded).map_err(|error| error.to_string())?;
if decoded_json != valid_string_metadata {
    return Err(format!("round trip mismatch: expected {valid_string_metadata}, got {decoded_json}"));
}

let valid_object_metadata = serde_json::json!({"name": "service", "metadata": {"version": "2"}});
serde_json::from_value::<IntegratedSchema>(valid_object_metadata).map_err(|error| error.to_string())?;

let missing_name = serde_json::json!({"status": "active"});
if serde_json::from_value::<IntegratedSchema>(missing_name).is_ok() {
    return Err("integrated allOf accepted missing required name".to_string());
}

let invalid_status = serde_json::json!({"name": "service", "status": "paused"});
if serde_json::from_value::<IntegratedSchema>(invalid_status).is_ok() {
    return Err("integrated allOf accepted unknown status".to_string());
}

let invalid_metadata = serde_json::json!({"name": "service", "metadata": 7});
if serde_json::from_value::<IntegratedSchema>(invalid_metadata).is_ok() {
    return Err("integrated allOf accepted non-string/non-object metadata".to_string());
}

let invalid_pattern_value = serde_json::json!({"name": "service", "config_mode": 7});
if serde_json::from_value::<IntegratedSchema>(invalid_pattern_value).is_ok() {
    return Err("integrated allOf accepted a non-string config value".to_string());
}

Ok(())
"#,
    );

    println!("✅ AllOf integration with other features test passed");
}

/// Test allOf with a real-world-style API specification
#[test]
fn test_allof_schema_allof_real_world_api_spec() {
    let schema = json!({
        "title": "APIEndpoint",
        "type": "object",
        "allOf": [
            {
                "type": "object",
                "properties": {
                    "method": {"enum": ["GET", "POST", "PUT", "DELETE"]},
                    "path": {"type": "string"}
                },
                "required": ["method", "path"]
            },
            {
                "type": "object",
                "properties": {
                    "headers": {
                        "type": "object",
                        "patternProperties": {
                            "^[a-zA-Z-]+$": {"type": "string"}
                        }
                    },
                    "query": {
                        "type": "object",
                        "patternProperties": {
                            "^[a-zA-Z_]+$": {"type": "string"}
                        }
                    }
                }
            },
            {
                "type": "object",
                "properties": {
                    "response": {
                        "type": "object",
                        "properties": {
                            "status": {"type": "integer", "minimum": 100, "maximum": 599},
                            "content_type": {"enum": ["application/json", "text/plain", "text/html"]}
                        },
                        "required": ["status"]
                    }
                }
            }
        ]
    });

    let grammar = TctCompiler::new(schema, "APIEndpoint")
        .unwrap()
        .compile()
        .expect("real-world allOf API spec should compile");
    let generated_code = grammar
        .generate_code()
        .expect("real-world allOf API spec should generate code");

    run_generated_allof_contract_crate(
        "allof-api-spec",
        &generated_code,
        r#"
let valid = serde_json::json!({
    "method": "GET",
    "path": "/v1/items",
    "headers": {"Accept": "application/json"},
    "query": {"page": "1"},
    "response": {"status": 200, "content_type": "application/json"}
});
let value: ApiEndpoint = serde_json::from_value(valid.clone()).map_err(|error| error.to_string())?;
let mut tokens = Vec::new();
value.encode(&mut tokens);
let mut iter = tokens.into_iter();
let decoded = ApiEndpoint::decode(&mut iter).map_err(|error| error.to_string())?;
let decoded_json = serde_json::to_value(decoded).map_err(|error| error.to_string())?;
if decoded_json != valid {
    return Err(format!("round trip mismatch: expected {valid}, got {decoded_json}"));
}

let missing_method = serde_json::json!({"path": "/v1/items"});
if serde_json::from_value::<ApiEndpoint>(missing_method).is_ok() {
    return Err("API allOf accepted missing required method".to_string());
}

let invalid_method = serde_json::json!({"method": "PATCH", "path": "/v1/items"});
if serde_json::from_value::<ApiEndpoint>(invalid_method).is_ok() {
    return Err("API allOf accepted unknown method".to_string());
}

let invalid_header_value = serde_json::json!({"method": "GET", "path": "/v1/items", "headers": {"Accept": 7}});
if serde_json::from_value::<ApiEndpoint>(invalid_header_value).is_ok() {
    return Err("API allOf accepted non-string header value".to_string());
}

let invalid_response = serde_json::json!({"method": "GET", "path": "/v1/items", "response": {"content_type": "application/json"}});
if serde_json::from_value::<ApiEndpoint>(invalid_response).is_ok() {
    return Err("API allOf accepted response without required status".to_string());
}

let invalid_content_type = serde_json::json!({"method": "GET", "path": "/v1/items", "response": {"status": 200, "content_type": "application/xml"}});
if serde_json::from_value::<ApiEndpoint>(invalid_content_type).is_ok() {
    return Err("API allOf accepted unknown response content_type".to_string());
}

Ok(())
"#,
    );

    println!("✅ AllOf real-world API spec test passed");
}

fn run_generated_allof_contract_crate(crate_name: &str, generated_code: &str, body: &str) {
    let crate_dir = tempfile::tempdir().expect("failed to create generated crate temp dir");
    let src_dir = crate_dir.path().join("src");
    fs::create_dir(&src_dir).expect("failed to create generated crate src dir");
    let tct_path = std::env::current_dir().expect("failed to resolve repo root");
    let shared_target_dir = tct_path.join("target").join("tct-allof-contracts");
    fs::create_dir_all(&shared_target_dir)
        .expect("failed to create shared allOf contract target dir");

    let mut cargo_toml = GeneratedCargoToml::new(crate_name)
        .with_preserve_order()
        .with_rustc_hash()
        .with_regex()
        .with_tct_path(tct_path.display().to_string())
        .with_bin(crate_name, "src/main.rs");
    if generated_code.contains("jsonschema ::") || generated_code.contains("jsonschema::") {
        cargo_toml = cargo_toml.with_jsonschema();
    }
    let cargo_toml = cargo_toml.render();
    fs::write(crate_dir.path().join("Cargo.toml"), cargo_toml)
        .expect("failed to write generated crate Cargo.toml");
    copy_workspace_cargo_lock(&tct_path, crate_dir.path())
        .expect("failed to copy workspace Cargo.lock into generated allOf crate");

    let main_rs = format!(
        r#"
#![allow(warnings)]

use crate::tokenizer::JsonTokenEncoder;

{generated_code}

fn main() -> Result<(), String> {{
{body}
}}
"#
    );
    fs::write(src_dir.join("main.rs"), main_rs).expect("failed to write generated crate main.rs");

    let output = generated_contract_cargo_command()
        .arg("run")
        .arg("--quiet")
        .current_dir(crate_dir.path())
        .env("CARGO_TARGET_DIR", &shared_target_dir)
        .output()
        .unwrap_or_else(|error| panic!("failed to run generated crate {}: {}", crate_name, error));

    assert!(
        output.status.success(),
        "generated crate {} failed\nstdout:\n{}\nstderr:\n{}",
        crate_name,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn assert_generated_allof_uses_cached_jsonschema_validator(generated_code: &str) {
    assert!(
        generated_code.contains("TCT_ALLOF_VALIDATOR"),
        "generated allOf JSON Schema validation should cache compiled validators"
    );
    assert!(
        generated_code.contains("OnceLock"),
        "generated allOf JSON Schema validation should use OnceLock"
    );
}
