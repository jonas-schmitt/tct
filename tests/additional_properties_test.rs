//! AdditionalProperties Support Tests
//!
//! Comprehensive test suite for JSON Schema additionalProperties feature supporting:
//! - additionalProperties: true (allow any additional properties)
//! - additionalProperties: false (strict validation, no additional properties)
//! - additionalProperties: {schema} (typed additional properties with schema validation)
//!
//! This test file follows TDD principles - tests are written first to define expected behavior,
//! then implementation follows to make these tests pass.

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

/// Test additionalProperties: true (allow any additional properties)
#[test]
fn test_additional_properties_true_basic_support() {
    let schema = json!({
        "type": "object",
        "properties": {
            "name": {"type": "string"}
        },
        "additionalProperties": true
    });

    // Should successfully compile schema with additionalProperties: true
    let result = TctCompiler::new(schema, "DynamicObject").and_then(|compiler| compiler.compile());

    assert!(result.is_ok(), "Should support additionalProperties: true");

    // Generated code should include HashMap for additional properties
    let grammar = result.unwrap();
    let generated_code = grammar.generate_code().unwrap();
    let code_str = generated_code.to_string();

    // Should generate a HashMap field for additional properties
    assert!(
        code_str.contains("HashMap") || code_str.contains("BTreeMap"),
        "Should generate map field for additional properties"
    );
    assert!(
        code_str.contains("DynamicObject"),
        "Should generate struct with given name"
    );
}

/// Test additionalProperties: false (strict validation)
#[test]
fn test_additional_properties_false_strict_validation() {
    let schema = json!({
        "type": "object",
        "properties": {
            "id": {"type": "integer"},
            "name": {"type": "string"}
        },
        "additionalProperties": false
    });

    // Should successfully compile with strict validation
    let result = TctCompiler::new(schema, "StrictObject").and_then(|compiler| compiler.compile());

    assert!(result.is_ok(), "Should support additionalProperties: false");

    let grammar = result.unwrap();
    let generated_code = grammar.generate_code().unwrap();
    let code_str = generated_code.to_string();

    // Extract the struct definition (between "pub struct StrictObject" and the impl block)
    let struct_start = code_str
        .find("pub struct StrictObject")
        .expect("Should have struct definition");
    let impl_start = code_str[struct_start..]
        .find("impl ")
        .expect("Should have impl block");
    let struct_def = &code_str[struct_start..struct_start + impl_start];

    // Should NOT generate HashMap field in the struct itself
    assert!(
        !struct_def.contains("HashMap")
            && !struct_def.contains("BTreeMap")
            && !struct_def.contains("additional_properties"),
        "Should not generate map field or additional_properties when additionalProperties: false"
    );
    // Should only contain explicitly defined properties
    assert!(
        struct_def.contains("id") && struct_def.contains("name"),
        "Should contain explicitly defined properties"
    );
}

/// Test additionalProperties with schema (typed additional properties)
#[test]
fn test_additional_properties_with_schema_typed_support() {
    let schema = json!({
        "type": "object",
        "properties": {
            "version": {"type": "string"}
        },
        "additionalProperties": {
            "type": "string"
        }
    });

    // Should successfully compile with typed additional properties
    let result =
        TctCompiler::new(schema, "TypedDynamicObject").and_then(|compiler| compiler.compile());

    assert!(
        result.is_ok(),
        "Should support additionalProperties with schema"
    );

    let grammar = result.unwrap();
    let generated_code = grammar.generate_code().unwrap();
    let code_str = generated_code.to_string();

    // Should generate HashMap<String, String> for string additional properties
    assert!(
        code_str.contains("HashMap") || code_str.contains("BTreeMap"),
        "Should generate map field for typed additional properties"
    );
    // Should specify the type constraint for additional properties
    assert!(
        code_str.contains("String"),
        "Should generate String type for additional properties"
    );
}

/// Test additionalProperties with complex schema objects
#[test]
fn test_additional_properties_complex_schema_support() {
    let schema = json!({
        "type": "object",
        "properties": {
            "config": {"type": "string"}
        },
        "additionalProperties": {
            "type": "object",
            "properties": {
                "enabled": {"type": "boolean"},
                "value": {"type": "integer"}
            }
        }
    });

    // Should support complex object schemas in additionalProperties
    let result =
        TctCompiler::new(schema, "ComplexDynamicObject").and_then(|compiler| compiler.compile());

    assert!(
        result.is_ok(),
        "Should support complex additionalProperties schemas"
    );

    let grammar = result.unwrap();
    let generated_code = grammar.generate_code().unwrap();
    let code_str = generated_code.to_string();

    // Should generate nested struct for complex additional properties
    assert!(
        code_str.contains("HashMap") || code_str.contains("BTreeMap"),
        "Should generate map field for complex additional properties"
    );
    assert!(
        code_str.contains("enabled") && code_str.contains("value"),
        "Should generate fields for complex additional property schema"
    );
}

/// Test npm package.json dependencies pattern (real-world use case)
#[test]
fn test_additional_properties_npm_dependencies_pattern() {
    let schema = json!({
        "type": "object",
        "properties": {
            "name": {"type": "string"},
            "version": {"type": "string"},
            "dependencies": {
                "type": "object",
                "additionalProperties": {"type": "string"}
            }
        }
    });

    // Should handle npm-style dependencies with dynamic package names
    let result = TctCompiler::new(schema, "PackageJson").and_then(|compiler| compiler.compile());

    assert!(result.is_ok(), "Should support npm dependencies pattern");

    let grammar = result.unwrap();
    let generated_code = grammar.generate_code().unwrap();
    let code_str = generated_code.to_string();

    // Should generate appropriate structure for package dependencies
    assert!(
        code_str.contains("dependencies"),
        "Should include dependencies field"
    );
    assert!(
        code_str.contains("HashMap") || code_str.contains("BTreeMap"),
        "Should use map for dynamic package dependencies"
    );
}

/// Test edge case: empty object with additionalProperties
#[test]
fn test_additional_properties_empty_object_edge_case() {
    let schema = json!({
        "type": "object",
        "additionalProperties": {"type": "number"}
    });

    // Should handle object with no explicit properties but additionalProperties
    let result =
        TctCompiler::new(schema, "EmptyWithAdditional").and_then(|compiler| compiler.compile());

    assert!(
        result.is_ok(),
        "Should support object with only additionalProperties"
    );

    let grammar = result.unwrap();
    let generated_code = grammar.generate_code().unwrap();
    let code_str = generated_code.to_string();

    // Should generate struct with just the additional properties map
    assert!(
        code_str.contains("HashMap") || code_str.contains("BTreeMap"),
        "Should generate map for additional properties"
    );
    assert!(
        code_str.contains("f64") || code_str.contains("number"),
        "Should use appropriate numeric type"
    );
}

/// Test mixed properties and additionalProperties
#[test]
fn test_additional_properties_mixed_with_explicit_properties() {
    let schema = json!({
        "type": "object",
        "properties": {
            "required_field": {"type": "string"},
            "optional_field": {"type": "integer"}
        },
        "required": ["required_field"],
        "additionalProperties": {"type": "boolean"}
    });

    // Should handle mix of explicit and additional properties
    let result =
        TctCompiler::new(schema, "MixedProperties").and_then(|compiler| compiler.compile());

    assert!(
        result.is_ok(),
        "Should support mixed explicit and additional properties"
    );

    let grammar = result.unwrap();
    let generated_code = grammar.generate_code().unwrap();
    let code_str = generated_code.to_string();

    // Should include both explicit fields and additional properties map
    assert!(
        code_str.contains("required_field") && code_str.contains("optional_field"),
        "Should include explicit properties"
    );
    assert!(
        code_str.contains("HashMap") || code_str.contains("BTreeMap"),
        "Should include map for additional properties"
    );
    assert!(
        code_str.contains("bool"),
        "Should use boolean type for additional properties"
    );
}

/// Test nested additionalProperties
#[test]
fn test_additional_properties_nested_support() {
    let schema = json!({
        "type": "object",
        "properties": {
            "config": {
                "type": "object",
                "properties": {
                    "name": {"type": "string"}
                },
                "additionalProperties": {"type": "string"}
            }
        },
        "additionalProperties": {
            "type": "object",
            "additionalProperties": {"type": "integer"}
        }
    });

    // Should support nested additionalProperties at multiple levels
    let result = TctCompiler::new(schema, "NestedAdditionalProperties")
        .and_then(|compiler| compiler.compile());

    assert!(result.is_ok(), "Should support nested additionalProperties");

    let grammar = result.unwrap();
    let generated_code = grammar.generate_code().unwrap();
    let code_str = generated_code.to_string();

    // Should handle multiple levels of additional properties
    assert!(
        code_str.contains("HashMap") || code_str.contains("BTreeMap"),
        "Should generate maps for nested additional properties"
    );
    assert!(
        code_str.contains("config"),
        "Should include explicit nested property"
    );
}

/// Test integration with existing optimization passes
#[test]
fn test_additional_properties_optimization_integration() {
    let schema = json!({
        "type": "object",
        "properties": {
            "status": {"enum": ["active", "inactive"]},
            "priority": {"type": "integer", "minimum": 1, "maximum": 10}
        },
        "additionalProperties": {"type": "string"}
    });

    // Should work with existing enum and integer optimizations
    let result = TctCompiler::new(schema, "OptimizedWithAdditional").and_then(|compiler| {
        compiler
            .with_optimization_level(tct::OptimizationLevel::O2)
            .compile()
    });

    assert!(
        result.is_ok(),
        "Should integrate with existing optimizations"
    );

    let grammar = result.unwrap();
    let generated_code = grammar.generate_code().unwrap();
    let code_str = generated_code.to_string();

    // Should apply optimizations to explicit properties while preserving additionalProperties
    assert!(
        code_str.contains("HashMap") || code_str.contains("BTreeMap"),
        "Should preserve additional properties with optimizations"
    );
    // Optimized enums and integers should still work
    assert!(
        code_str.contains("status") && code_str.contains("priority"),
        "Should preserve optimized explicit properties"
    );
}

/// Test error handling for invalid additionalProperties schemas
#[test]
fn test_additional_properties_invalid_schema_error_handling() {
    let schema = json!({
        "type": "object",
        "properties": {
            "name": {"type": "string"}
        },
        "additionalProperties": {
            "type": "invalid_type"
        }
    });

    // Should handle invalid additionalProperties schema gracefully
    let result = TctCompiler::new(schema, "InvalidAdditionalProperties")
        .and_then(|compiler| compiler.compile());

    // Should either succeed with fallback or fail with meaningful error
    match result {
        Ok(_) => {
            // If it succeeds, it should fall back to a safe default
            println!("Successfully handled invalid additionalProperties with fallback");
        }
        Err(e) => {
            // If it fails, error should be descriptive
            let error_msg = format!("{}", e);
            assert!(
                error_msg.contains("additional") || error_msg.contains("invalid"),
                "Error should mention additionalProperties or invalid type"
            );
        }
    }
}

#[test]
fn test_additional_properties_generated_crate_semantic_contracts() {
    let schema = json!({
        "title": "AdditionalPropertiesContract",
        "type": "object",
        "properties": {
            "dynamic": {
                "type": "object",
                "additionalProperties": true
            },
            "strict": {
                "type": "object",
                "properties": {
                    "id": {"type": "integer"},
                    "name": {"type": "string"}
                },
                "required": ["id", "name"],
                "additionalProperties": false
            },
            "typed": {
                "type": "object",
                "additionalProperties": {"type": "string"}
            },
            "nested": {
                "type": "object",
                "properties": {
                    "config": {
                        "type": "object",
                        "properties": {
                            "name": {"type": "string"}
                        },
                        "required": ["name"],
                        "additionalProperties": {"type": "string"}
                    }
                },
                "required": ["config"],
                "additionalProperties": {
                    "type": "object",
                    "additionalProperties": {"type": "integer"}
                }
            }
        },
        "required": ["dynamic", "strict", "typed", "nested"],
        "additionalProperties": false
    });

    let grammar = TctCompiler::new(schema, "AdditionalPropertiesContract")
        .unwrap()
        .compile()
        .expect("additionalProperties contract schema should compile");
    let generated_code = grammar
        .generate_code()
        .expect("additionalProperties contract schema should generate code");
    let generated_code_string = generated_code.to_string();
    assert!(
        !generated_code_string.contains("k . clone () , v . clone ()"),
        "generated serde_json::Value object encode/token_count should not clone key-value pairs"
    );
    assert!(
        generated_code_string.contains("encode_json_object_map")
            && generated_code_string.contains("json_object_map_token_count"),
        "generated serde_json::Value object encode/token_count should use borrowed object helpers"
    );

    run_generated_additional_properties_contract_crate(
        "additional-properties-contracts",
        &generated_code,
        r#"
let valid = serde_json::json!({
    "dynamic": {
        "extra_string": "ok",
        "extra_bool": true,
        "extra_number": 42,
        "extra_array": [1, "two"],
        "extra_object": {"nested": true}
    },
    "strict": {"id": 1, "name": "Ada"},
    "typed": {"package": "1.0.0", "serde": "1.0"},
    "nested": {
        "config": {"name": "cfg", "mode": "fast"},
        "limits": {"cpu": 2, "memory": 512}
    }
});

let value: AdditionalPropertiesContract =
    serde_json::from_value(valid.clone()).map_err(|error| error.to_string())?;
let mut tokens = Vec::new();
value.encode(&mut tokens);
if tokens.is_empty() {
    return Err("valid additionalProperties fixture encoded to no tokens".to_string());
}
if let Some(max_token) = tokens.iter().max() {
    if *max_token >= VOCAB_SIZE {
        return Err(format!("token {max_token} outside VOCAB_SIZE {VOCAB_SIZE}"));
    }
}
let estimated_tokens = value.token_count();
if estimated_tokens != tokens.len() {
    return Err(format!(
        "token_count mismatch before decode: counted {estimated_tokens}, encoded {}",
        tokens.len()
    ));
}

let mut iter = tokens.clone().into_iter();
let decoded = AdditionalPropertiesContract::decode(&mut iter).map_err(|error| error.to_string())?;
if let Some(extra) = iter.next() {
    return Err(format!("decode left an unconsumed token: {extra}"));
}
let decoded_tokens = {
    let mut decoded_tokens = Vec::new();
    decoded.encode(&mut decoded_tokens);
    decoded_tokens
};
if decoded_tokens != tokens {
    return Err(format!(
        "decoded re-encode changed token order: expected {:?}, got {:?}",
        tokens,
        decoded_tokens
    ));
}
let decoded_token_count = decoded.token_count();
if decoded_token_count != tokens.len() {
    return Err(format!(
        "decoded token_count mismatch: counted {decoded_token_count}, encoded {}",
        tokens.len()
    ));
}
let decoded_json = serde_json::to_value(decoded).map_err(|error| error.to_string())?;
if decoded_json != valid {
    return Err(format!("round trip mismatch: expected {valid}, got {decoded_json}"));
}

let mut recovered_empty_object = vec![11u32, 0u32, 0u32].into_iter();
let recovered = serde_json::Value::decode(&mut recovered_empty_object)
    .map_err(|error| format!("modulo object tag recovery failed: {error}"))?;
if recovered != serde_json::json!({}) {
    return Err(format!("expected empty object from modulo recovered object tag, got {recovered}"));
}
if let Some(extra) = recovered_empty_object.next() {
    return Err(format!("modulo recovery left an unconsumed token: {extra}"));
}

let mut recovered_uint_number_tokens = vec![2u32, 5u32];
42u64.encode(&mut recovered_uint_number_tokens);
let mut recovered_uint_number_iter = recovered_uint_number_tokens.into_iter();
let recovered_uint_number = serde_json::Value::decode(&mut recovered_uint_number_iter)
    .map_err(|error| format!("modulo number subtype recovery failed: {error}"))?;
if recovered_uint_number != serde_json::json!(42u64) {
    return Err(format!(
        "expected unsigned number from modulo recovered number subtype, got {recovered_uint_number}"
    ));
}
if let Some(extra) = recovered_uint_number_iter.next() {
    return Err(format!("modulo number subtype recovery left an unconsumed token: {extra}"));
}

let strict_unknown = serde_json::json!({
    "dynamic": {},
    "strict": {"id": 1, "name": "Ada", "extra": true},
    "typed": {},
    "nested": {"config": {"name": "cfg"}}
});
if serde_json::from_value::<AdditionalPropertiesContract>(strict_unknown).is_ok() {
    return Err("strict additionalProperties:false object accepted an unknown field".to_string());
}

let typed_non_string = serde_json::json!({
    "dynamic": {},
    "strict": {"id": 1, "name": "Ada"},
    "typed": {"package": 7},
    "nested": {"config": {"name": "cfg"}}
});
if serde_json::from_value::<AdditionalPropertiesContract>(typed_non_string).is_ok() {
    return Err("typed additionalProperties object accepted a non-string extra value".to_string());
}

let nested_non_integer = serde_json::json!({
    "dynamic": {},
    "strict": {"id": 1, "name": "Ada"},
    "typed": {},
    "nested": {
        "config": {"name": "cfg"},
        "limits": {"cpu": "two"}
    }
});
if serde_json::from_value::<AdditionalPropertiesContract>(nested_non_integer).is_ok() {
    return Err("nested additionalProperties object accepted a non-integer extra value".to_string());
}

let root_unknown = serde_json::json!({
    "dynamic": {},
    "strict": {"id": 1, "name": "Ada"},
    "typed": {},
    "nested": {"config": {"name": "cfg"}},
    "unknown": true
});
if serde_json::from_value::<AdditionalPropertiesContract>(root_unknown).is_ok() {
    return Err("root additionalProperties:false accepted an unknown field".to_string());
}

Ok(())
"#,
    );
}

#[test]
fn test_generated_additional_properties_preserves_large_unsigned_json_number() {
    let schema = json!({
        "title": "LargeUnsignedAdditionalProperties",
        "type": "object",
        "properties": {
            "payload": {
                "type": "object",
                "additionalProperties": true
            }
        },
        "required": ["payload"],
        "additionalProperties": false
    });

    let grammar = TctCompiler::new(schema, "LargeUnsignedAdditionalProperties")
        .unwrap()
        .compile()
        .expect("large unsigned additionalProperties schema should compile");
    let generated_code = grammar
        .generate_code()
        .expect("large unsigned additionalProperties schema should generate code");

    run_generated_additional_properties_contract_crate(
        "large-unsigned-additional-properties-contracts",
        &generated_code,
        r#"
let original = serde_json::json!({
    "payload": {
        "max": u64::MAX
    }
});

let value: LargeUnsignedAdditionalProperties =
    serde_json::from_value(original.clone()).map_err(|error| error.to_string())?;
let mut tokens = Vec::new();
value.encode(&mut tokens);
if tokens.is_empty() {
    return Err("large unsigned fixture encoded to no tokens".to_string());
}
if let Some(max_token) = tokens.iter().max() {
    if *max_token >= VOCAB_SIZE {
        return Err(format!("token {max_token} outside VOCAB_SIZE {VOCAB_SIZE}"));
    }
}

let mut iter = tokens.into_iter();
let decoded = LargeUnsignedAdditionalProperties::decode(&mut iter)
    .map_err(|error| error.to_string())?;
if let Some(extra) = iter.next() {
    return Err(format!("decode left an unconsumed token: {extra}"));
}

let decoded_json = serde_json::to_value(decoded).map_err(|error| error.to_string())?;
if decoded_json != original {
    return Err(format!("large unsigned round trip mismatch: expected {original}, got {decoded_json}"));
}
let decoded_number = decoded_json
    .get("payload")
    .and_then(|payload| payload.get("max"))
    .and_then(serde_json::Value::as_u64);
if decoded_number != Some(u64::MAX) {
    return Err(format!("decoded large number was not preserved as u64::MAX: {decoded_number:?}"));
}

Ok(())
"#,
    );
}

#[test]
fn test_generated_pattern_properties_preserves_nullable_optional_null() {
    let schema = json!({
        "type": "object",
        "properties": {
            "maybe": {"type": ["string", "null"]},
            "title": {"type": "string"}
        },
        "patternProperties": {
            "^x_": {"type": "string"}
        },
        "additionalProperties": false
    });

    let grammar = TctCompiler::new(schema, "PatternNullableNullContract")
        .unwrap()
        .compile()
        .expect("patternProperties nullable schema should compile");
    let generated_code = grammar
        .generate_code()
        .expect("patternProperties nullable schema should generate code");

    run_generated_additional_properties_contract_crate(
        "pattern-null-contracts",
        &generated_code,
        r#"
let valid = serde_json::json!({
    "maybe": null,
    "title": "Doc",
    "x_name": "ok"
});

let value: PatternNullableNullContract =
    serde_json::from_value(valid.clone()).map_err(|error| error.to_string())?;
let explicit_null = serde_json::json!({
    "maybe": "value",
    "title": null,
    "x_name": "ok"
});
if serde_json::from_value::<PatternNullableNullContract>(explicit_null).is_ok() {
    return Err("custom patternProperties serde accepted explicit null for optional non-nullable field".to_string());
}

let mut tokens = Vec::new();
value.encode(&mut tokens);
if tokens.is_empty() {
    return Err("nullable optional patternProperties fixture encoded to no tokens".to_string());
}
let token_count = value.token_count();
if token_count != tokens.len() {
    return Err(format!(
        "token_count mismatch before decode: counted {token_count}, encoded {}",
        tokens.len()
    ));
}
if let Some(max_token) = tokens.iter().max() {
    if *max_token >= VOCAB_SIZE {
        return Err(format!("token {max_token} outside VOCAB_SIZE {VOCAB_SIZE}"));
    }
}

let original_tokens = tokens.clone();
let mut iter = tokens.into_iter();
let decoded = PatternNullableNullContract::decode(&mut iter).map_err(|error| error.to_string())?;
if let Some(extra) = iter.next() {
    return Err(format!("decode left an unconsumed token: {extra}"));
}
let decoded_token_count = decoded.token_count();
if decoded_token_count != original_tokens.len() {
    return Err(format!(
        "token_count mismatch after decode: counted {decoded_token_count}, encoded {}",
        original_tokens.len()
    ));
}
let mut reencoded_tokens = Vec::new();
decoded.encode(&mut reencoded_tokens);
if reencoded_tokens != original_tokens {
    return Err(format!(
        "non-deterministic patternProperties tokenization: expected {original_tokens:?}, got {reencoded_tokens:?}"
    ));
}

let decoded_json = serde_json::to_value(decoded).map_err(|error| error.to_string())?;
if decoded_json != valid {
    return Err(format!("nullable optional null round trip mismatch: expected {valid}, got {decoded_json}"));
}

Ok(())
"#,
    );
}

#[test]
fn test_generated_optional_non_nullable_rejects_explicit_null() {
    let schema = json!({
        "type": "object",
        "properties": {
            "name": {"type": "string"}
        },
        "additionalProperties": false
    });

    let grammar = TctCompiler::new(schema, "OptionalNonNullableNullContract")
        .unwrap()
        .compile()
        .expect("optional non-nullable schema should compile");
    let generated_code = grammar
        .generate_code()
        .expect("optional non-nullable schema should generate code");

    run_generated_additional_properties_contract_crate(
        "optional-non-null-contracts",
        &generated_code,
        r#"
let missing = serde_json::json!({});
serde_json::from_value::<OptionalNonNullableNullContract>(missing)
    .map_err(|error| format!("missing optional field should deserialize: {error}"))?;

let present = serde_json::json!({"name": "Ada"});
serde_json::from_value::<OptionalNonNullableNullContract>(present)
    .map_err(|error| format!("present optional field should deserialize: {error}"))?;

let explicit_null = serde_json::json!({"name": null});
if serde_json::from_value::<OptionalNonNullableNullContract>(explicit_null).is_ok() {
    return Err("explicit null for optional non-nullable field was accepted".to_string());
}

Ok(())
"#,
    );
}

fn run_generated_additional_properties_contract_crate(
    crate_name: &str,
    generated_code: &str,
    body: &str,
) {
    let crate_dir =
        tempfile::tempdir().expect("failed to create generated additionalProperties temp dir");
    let src_dir = crate_dir.path().join("src");
    fs::create_dir(&src_dir).expect("failed to create generated additionalProperties src dir");

    let tct_path = std::env::current_dir().expect("failed to resolve repo root");
    let cargo_toml = GeneratedCargoToml::new(crate_name)
        .with_preserve_order()
        .with_rustc_hash()
        .with_regex()
        .with_tct_path(tct_path.display().to_string())
        .with_bin(crate_name, "src/main.rs")
        .render();
    fs::write(crate_dir.path().join("Cargo.toml"), cargo_toml)
        .expect("failed to write generated additionalProperties Cargo.toml");
    copy_workspace_cargo_lock(&tct_path, crate_dir.path())
        .expect("failed to copy workspace Cargo.lock into generated additionalProperties crate");

    let main_rs = format!(
        r#"
#![allow(warnings)]

use serde_json::json;
use crate::tokenizer::JsonTokenEncoder;

{generated_code}

fn main() -> Result<(), String> {{
{body}
}}
"#,
    );
    fs::write(src_dir.join("main.rs"), main_rs)
        .expect("failed to write generated additionalProperties main.rs");

    let output = generated_contract_cargo_command()
        .arg("run")
        .arg("--quiet")
        .env(
            "CARGO_TARGET_DIR",
            std::env::current_dir()
                .unwrap()
                .join("target/tct-additional-properties-contracts"),
        )
        .current_dir(crate_dir.path())
        .output()
        .expect("failed to run generated additionalProperties contract crate");

    assert!(
        output.status.success(),
        "generated additionalProperties contract failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
