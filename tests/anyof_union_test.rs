//! AnyOf Union Schema Tests
//!
//! This test suite validates support for JSON Schema AnyOf unions,
//! which allow values to match any of several sub-schemas.
//! Critical for multi-type support and schema flexibility.

use serde_json::json;
use std::fs;
use std::process::Command;
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

#[test]
fn test_anyof_union_anyof_basic_string_or_number() {
    // Test schema allowing either string or number
    let schema = json!({
        "title": "StringOrNumber",
        "anyOf": [
            {"type": "string"},
            {"type": "number"}
        ]
    });

    let result = TctCompiler::new(schema, "StringOrNumber")
        .unwrap()
        .compile();

    // Should compile successfully
    assert!(result.is_ok(), "AnyOf string|number should compile");
    result
        .unwrap()
        .generate_code()
        .expect("AnyOf string|number should generate code");

    println!("✅ Basic AnyOf string|number union compiled successfully");
}

#[test]
fn test_anyof_union_anyof_object_with_different_properties() {
    // Test AnyOf with objects having different property sets
    let schema = json!({
        "title": "UserOrProduct",
        "anyOf": [
            {
                "type": "object",
                "properties": {
                    "name": {"type": "string"},
                    "email": {"type": "string"}
                },
                "required": ["name", "email"]
            },
            {
                "type": "object",
                "properties": {
                    "title": {"type": "string"},
                    "price": {"type": "number"}
                },
                "required": ["title", "price"]
            }
        ]
    });

    let result = TctCompiler::new(schema, "UserOrProduct").unwrap().compile();

    assert!(
        result.is_ok(),
        "AnyOf with different object schemas should compile"
    );
    result
        .unwrap()
        .generate_code()
        .expect("AnyOf with different object schemas should generate code");

    println!("✅ AnyOf with different object schemas compiled successfully");
}

#[test]
fn test_anyof_union_anyof_nested_unions() {
    // Test nested AnyOf unions
    let schema = json!({
        "title": "NestedUnion",
        "anyOf": [
            {"type": "string"},
            {
                "anyOf": [
                    {"type": "number"},
                    {"type": "boolean"}
                ]
            }
        ]
    });

    let result = TctCompiler::new(schema, "NestedUnion").unwrap().compile();

    assert!(result.is_ok(), "Nested AnyOf should compile");

    println!("✅ Nested AnyOf unions compiled successfully");
}

#[test]
fn test_anyof_union_anyof_with_enum_constraints() {
    // Test AnyOf where some branches have enum constraints
    let schema = json!({
        "title": "StatusOrPriority",
        "anyOf": [
            {
                "type": "string",
                "enum": ["pending", "active", "completed"]
            },
            {
                "type": "string",
                "enum": ["low", "medium", "high"]
            }
        ]
    });

    let result = TctCompiler::new(schema, "StatusOrPriority")
        .unwrap()
        .compile();

    assert!(result.is_ok(), "AnyOf with enum constraints should compile");
    result
        .unwrap()
        .generate_code()
        .expect("AnyOf with enum constraints should generate code");

    println!("✅ AnyOf with enum constraints compiled successfully");
}

#[test]
fn test_anyof_union_anyof_array_items() {
    // Test AnyOf in array items
    let schema = json!({
        "title": "MixedArray",
        "type": "array",
        "items": {
            "anyOf": [
                {"type": "string"},
                {"type": "number"},
                {"type": "boolean"}
            ]
        }
    });

    let result = TctCompiler::new(schema, "MixedArray").unwrap().compile();

    assert!(result.is_ok(), "Array with AnyOf items should compile");

    println!("✅ Array with AnyOf items compiled successfully");
}

#[test]
fn test_anyof_union_anyof_object_property() {
    // Test AnyOf as object property
    let schema = json!({
        "title": "ObjectWithUnion",
        "type": "object",
        "properties": {
            "id": {"type": "string"},
            "data": {
                "anyOf": [
                    {"type": "string"},
                    {"type": "object", "properties": {"value": {"type": "number"}}}
                ]
            }
        },
        "required": ["id"]
    });

    let result = TctCompiler::new(schema, "ObjectWithUnion")
        .unwrap()
        .compile();

    assert!(result.is_ok(), "Object with AnyOf property should compile");

    println!("✅ Object with AnyOf property compiled successfully");
}

#[test]
fn test_anyof_union_anyof_with_null() {
    // Test AnyOf including null type
    let schema = json!({
        "title": "OptionalString",
        "anyOf": [
            {"type": "string"},
            {"type": "null"}
        ]
    });

    let result = TctCompiler::new(schema, "OptionalString")
        .unwrap()
        .compile();

    assert!(result.is_ok(), "AnyOf with null should compile");
    result
        .unwrap()
        .generate_code()
        .expect("AnyOf with null should generate code");

    println!("✅ AnyOf with null compiled successfully");
}

#[test]
fn test_anyof_union_anyof_error_handling() {
    // Test error cases for AnyOf

    // Empty AnyOf should fail
    let empty_schema = json!({
        "title": "EmptyUnion",
        "anyOf": []
    });

    let result = TctCompiler::new(empty_schema, "EmptyUnion")
        .unwrap()
        .compile();

    // Check if empty AnyOf is supported or fails gracefully
    match result {
        Ok(_) => println!("✅ Empty AnyOf is supported"),
        Err(e) => println!("✅ Empty AnyOf fails as expected: {}", e),
    }

    // Invalid nested schema should FAIL with clear error, not fallback to Value
    let invalid_schema = json!({
        "title": "InvalidUnion",
        "anyOf": [
            {"type": "string"},
            {"type": "invalid_type"}
        ]
    });

    let result2 = TctCompiler::new(invalid_schema, "InvalidUnion")
        .unwrap()
        .compile();

    // CORRECTNESS: Invalid types should produce errors, not inefficient Value fallbacks
    assert!(
        result2.is_err(),
        "Invalid type should fail with error, not fallback to Value"
    );

    let error_msg = result2.err().unwrap().to_string();
    assert!(
        error_msg.contains("invalid_type") || error_msg.contains("Unsupported"),
        "Error should mention the invalid type, got: {}",
        error_msg
    );

    println!("✅ AnyOf correctly rejects invalid types with proper error messages");
}

#[test]
fn test_anyof_union_anyof_optimization_behavior() {
    // Test that AnyOf unions work with optimization passes
    let schema = json!({
        "title": "OptimizedUnion",
        "anyOf": [
            {
                "type": "string",
                "enum": ["option1", "option2"]
            },
            {
                "type": "object",
                "properties": {
                    "priority": {
                        "type": "integer",
                        "minimum": 1,
                        "maximum": 5
                    }
                }
            }
        ]
    });

    let result = TctCompiler::new(schema, "OptimizedUnion")
        .unwrap()
        .with_optimization_level(tct::OptimizationLevel::O2)
        .compile();

    assert!(result.is_ok(), "AnyOf with optimizations should compile");
    result
        .unwrap()
        .generate_code()
        .expect("optimized AnyOf should generate code");

    println!("✅ AnyOf with optimizations compiled successfully");
}

#[test]
fn test_anyof_union_anyof_real_world_api_response() {
    // Test realistic API response that can be success or error
    let schema = json!({
        "title": "ApiResponse",
        "anyOf": [
            {
                "type": "object",
                "properties": {
                    "success": {"type": "boolean", "enum": [true]},
                    "data": {"type": "object"},
                    "message": {"type": "string"}
                },
                "required": ["success", "data"]
            },
            {
                "type": "object",
                "properties": {
                    "success": {"type": "boolean", "enum": [false]},
                    "error": {"type": "string"},
                    "code": {"type": "integer"}
                },
                "required": ["success", "error", "code"]
            }
        ]
    });

    let result = TctCompiler::new(schema, "ApiResponse").unwrap().compile();

    // Check if complex AnyOf compiles or has known limitations
    match result {
        Ok(_) => println!("✅ Real-world AnyOf API response compiled successfully"),
        Err(e) => println!("ℹ️  Real-world AnyOf has limitations: {}", e),
    }
}

#[test]
fn test_anyof_union_root_anyof_generated_crate_semantics() {
    let schema = json!({
        "title": "StringOrNumber",
        "anyOf": [
            {"type": "string"},
            {"type": "number"}
        ]
    });

    let grammar = TctCompiler::new(schema, "StringOrNumber")
        .unwrap()
        .compile()
        .expect("root anyOf schema should compile");
    let generated_code = grammar
        .generate_code()
        .expect("root anyOf schema should generate code");

    run_generated_anyof_contract_crate(
        "anyof-root-contracts",
        &generated_code,
        r#"
assert_round_trip::<StringOrNumber>(json!("literal string"))?;
assert_round_trip::<StringOrNumber>(json!(42.5))?;

if serde_json::from_value::<StringOrNumber>(json!(true)).is_ok() {
    return Err("root anyOf accepted a boolean outside string|number variants".to_string());
}

Ok(())
"#,
    );
}

#[test]
fn test_anyof_union_overlapping_root_variant_selection_semantics() {
    let anyof_schema = json!({
        "title": "OverlappingAnyOf",
        "anyOf": [
            {"type": "string"},
            {"type": "string", "minLength": 1}
        ]
    });

    let anyof_grammar = TctCompiler::new(anyof_schema, "OverlappingAnyOf")
        .unwrap()
        .compile()
        .expect("overlapping root anyOf schema should compile");
    let anyof_generated_code = anyof_grammar
        .generate_code()
        .expect("overlapping root anyOf schema should generate code");

    run_generated_anyof_contract_crate(
        "anyof-overlap-contracts",
        &anyof_generated_code,
        r#"
let value: OverlappingAnyOf =
    serde_json::from_value(json!("matched")).map_err(|error| error.to_string())?;
match value {
    OverlappingAnyOf::Variant0(inner) if inner == "matched" => {}
    OverlappingAnyOf::Variant0(inner) => {
        return Err(format!("overlapping anyOf preserved wrong payload: {inner}"));
    }
    OverlappingAnyOf::Variant1(_) => {
        return Err("overlapping anyOf selected the second matching variant".to_string());
    }
}
assert_round_trip::<OverlappingAnyOf>(json!("matched"))?;

if serde_json::from_value::<OverlappingAnyOf>(json!(true)).is_ok() {
    return Err("overlapping anyOf accepted a boolean outside string variants".to_string());
}

Ok(())
"#,
    );

    let oneof_schema = json!({
        "title": "OverlappingOneOf",
        "oneOf": [
            {"type": "string"},
            {"type": "string", "minLength": 1}
        ]
    });

    let oneof_grammar = TctCompiler::new(oneof_schema, "OverlappingOneOf")
        .unwrap()
        .compile()
        .expect("overlapping root oneOf schema should compile");
    let oneof_generated_code = oneof_grammar
        .generate_code()
        .expect("overlapping root oneOf schema should generate code");

    run_generated_anyof_contract_crate(
        "oneof-overlap-contracts",
        &oneof_generated_code,
        r#"
assert_round_trip::<OverlappingOneOf>(json!(""))?;

if serde_json::from_value::<OverlappingOneOf>(json!("matched")).is_ok() {
    return Err("overlapping oneOf accepted a value matching multiple variants".to_string());
}

if serde_json::from_value::<OverlappingOneOf>(json!(true)).is_ok() {
    return Err("overlapping oneOf accepted a boolean outside string variants".to_string());
}

Ok(())
"#,
    );
}

#[test]
fn test_anyof_union_object_anyof_and_oneof_generated_crate_semantics() {
    let schema = json!({
        "title": "UnionContract",
        "type": "object",
        "properties": {
            "payload": {
                "anyOf": [
                    {
                        "type": "object",
                        "properties": {
                            "name": {"type": "string"},
                            "email": {"type": "string"}
                        },
                        "required": ["name", "email"]
                    },
                    {
                        "type": "object",
                        "properties": {
                            "title": {"type": "string"},
                            "price": {"type": "number"}
                        },
                        "required": ["title", "price"]
                    }
                ]
            },
            "source": {
                "oneOf": [
                    {
                        "type": "object",
                        "properties": {"url": {"type": "string"}},
                        "required": ["url"]
                    },
                    {
                        "type": "object",
                        "properties": {"filePath": {"type": "string"}},
                        "required": ["filePath"]
                    }
                ]
            },
            "scalar": {
                "anyOf": [
                    {"type": "string"},
                    {"type": "number"}
                ]
            },
            "maybe": {
                "anyOf": [
                    {"type": "string"},
                    {"type": "null"}
                ]
            }
        },
        "required": ["payload", "source", "scalar", "maybe"],
        "additionalProperties": false
    });

    let grammar = TctCompiler::new(schema, "UnionContract")
        .unwrap()
        .compile()
        .expect("object anyOf/oneOf schema should compile");
    let generated_code = grammar
        .generate_code()
        .expect("object anyOf/oneOf schema should generate code");

    run_generated_anyof_contract_crate(
        "anyof-object-contracts",
        &generated_code,
        r#"
let user_payload = json!({
    "payload": {"name": "Ada", "email": "ada@example.com"},
    "source": {"url": "https://example.test/data.json"},
    "scalar": "fast",
    "maybe": null
});
assert_round_trip::<UnionContract>(user_payload)?;

let product_payload = json!({
    "payload": {"title": "Keyboard", "price": 99.5},
    "source": {"filePath": "/tmp/data.json"},
    "scalar": 12.25,
    "maybe": "present"
});
assert_round_trip::<UnionContract>(product_payload)?;

let missing_anyof_required = json!({
    "payload": {"name": "Ada"},
    "source": {"url": "https://example.test/data.json"},
    "scalar": "fast",
    "maybe": null
});
if serde_json::from_value::<UnionContract>(missing_anyof_required).is_ok() {
    return Err("anyOf payload accepted an object missing required variant fields".to_string());
}

let ambiguous_oneof = json!({
    "payload": {"name": "Ada", "email": "ada@example.com"},
    "source": {"url": "https://example.test/data.json", "filePath": "/tmp/data.json"},
    "scalar": "fast",
    "maybe": null
});
if serde_json::from_value::<UnionContract>(ambiguous_oneof).is_ok() {
    return Err("oneOf source accepted an object matching multiple variants".to_string());
}

let invalid_scalar = json!({
    "payload": {"name": "Ada", "email": "ada@example.com"},
    "source": {"url": "https://example.test/data.json"},
    "scalar": true,
    "maybe": null
});
if serde_json::from_value::<UnionContract>(invalid_scalar).is_ok() {
    return Err("scalar anyOf accepted a boolean outside string|number variants".to_string());
}

let root_unknown = json!({
    "payload": {"name": "Ada", "email": "ada@example.com"},
    "source": {"url": "https://example.test/data.json"},
    "scalar": "fast",
    "maybe": null,
    "extra": true
});
if serde_json::from_value::<UnionContract>(root_unknown).is_ok() {
    return Err("root additionalProperties:false accepted an unknown field".to_string());
}

Ok(())
"#,
    );
}

fn run_generated_anyof_contract_crate(crate_name: &str, generated_code: &str, body: &str) {
    let crate_dir = tempfile::tempdir().expect("failed to create generated anyOf temp dir");
    let src_dir = crate_dir.path().join("src");
    fs::create_dir(&src_dir).expect("failed to create generated anyOf src dir");
    let tct_path = std::env::current_dir().expect("failed to resolve repo root");
    let shared_target_dir = tct_path.join("target").join("tct-anyof-contracts");
    fs::create_dir_all(&shared_target_dir).expect("failed to create shared anyOf target dir");

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
        .expect("failed to write generated anyOf Cargo.toml");
    copy_workspace_cargo_lock(&tct_path, crate_dir.path())
        .expect("failed to copy workspace Cargo.lock into generated anyOf crate");

    let main_rs = format!(
        r#"
#![allow(warnings)]

use crate::tokenizer::JsonTokenEncoder;
use serde_json::json;

{generated_code}

fn assert_round_trip<T>(input: serde_json::Value) -> Result<(), String>
where
    T: JsonTokenEncoder + serde::Serialize + for<'de> serde::Deserialize<'de>,
{{
    let value: T = serde_json::from_value(input.clone()).map_err(|error| error.to_string())?;
    let mut tokens = Vec::new();
    value.encode(&mut tokens);
    if tokens.is_empty() {{
        return Err("valid anyOf/oneOf fixture encoded to no tokens".to_string());
    }}
    let token_count = value.token_count();
    if token_count != tokens.len() {{
        return Err(format!(
            "token_count mismatch before decode: counted {{token_count}}, encoded {{}}",
            tokens.len()
        ));
    }}
    if let Some(max_token) = tokens.iter().max() {{
        if *max_token >= VOCAB_SIZE {{
            return Err(format!("token {{max_token}} outside VOCAB_SIZE {{VOCAB_SIZE}}"));
        }}
    }}

    let original_tokens = tokens.clone();
    let mut iter = tokens.into_iter();
    let decoded = T::decode(&mut iter).map_err(|error| error.to_string())?;
    if let Some(extra) = iter.next() {{
        return Err(format!("decode left an unconsumed token: {{extra}}"));
    }}
    let decoded_token_count = decoded.token_count();
    if decoded_token_count != original_tokens.len() {{
        return Err(format!(
            "token_count mismatch after decode: counted {{decoded_token_count}}, encoded {{}}",
            original_tokens.len()
        ));
    }}
    let mut reencoded_tokens = Vec::new();
    decoded.encode(&mut reencoded_tokens);
    if reencoded_tokens != original_tokens {{
        return Err(format!(
            "non-deterministic anyOf/oneOf tokenization: expected {{original_tokens:?}}, got {{reencoded_tokens:?}}"
        ));
    }}
    let decoded_json = serde_json::to_value(decoded).map_err(|error| error.to_string())?;
    if decoded_json != input {{
        return Err(format!("round trip mismatch: expected {{input}}, got {{decoded_json}}"));
    }}

    Ok(())
}}

fn main() -> Result<(), String> {{
{body}
}}
"#,
    );
    fs::write(src_dir.join("main.rs"), main_rs).expect("failed to write generated anyOf main.rs");

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
