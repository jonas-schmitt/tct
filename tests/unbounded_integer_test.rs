use std::fs;
use std::process::Command;

use serde_json::json;
use tct::TctCompiler;
use tct::generated_crate::{
    GeneratedCargoToml, configure_generated_cargo_command, copy_workspace_cargo_lock,
};

fn generated_integer_cargo_command() -> Command {
    let tct_path = std::env::current_dir().expect("failed to resolve repo root");
    let mut command = Command::new("cargo");
    configure_generated_cargo_command(&mut command, &tct_path);
    command
}

#[test]
fn generated_unbounded_integer_field_preserves_large_unsigned_values() {
    let schema = json!({
        "title": "UnboundedIntegerContract",
        "type": "object",
        "properties": {
            "value": {"type": "integer"},
            "maybe": {"type": "integer"}
        },
        "required": ["value"],
        "additionalProperties": false
    });

    let grammar = TctCompiler::new(schema, "UnboundedIntegerContract")
        .unwrap()
        .compile()
        .expect("unbounded integer schema should compile");
    let generated_code = grammar
        .generate_code()
        .expect("unbounded integer schema should generate code");

    run_generated_integer_contract_crate(
        "unbounded-integer-field-contract",
        &generated_code,
        r#"
let original = serde_json::json!({
    "value": u64::MAX,
    "maybe": 9_223_372_036_854_775_808u64
});

let value: UnboundedIntegerContract = serde_json::from_value(original.clone())
    .map_err(|error| format!("generated type rejected valid large unsigned integer: {error}"))?;

let fractional = serde_json::json!({"value": 1.25});
if serde_json::from_value::<UnboundedIntegerContract>(fractional).is_ok() {
    return Err("integer field accepted a fractional JSON number".to_string());
}

let mut tokens = Vec::new();
value.encode(&mut tokens);
if tokens.is_empty() {
    return Err("large unsigned integer fixture encoded to no tokens".to_string());
}
if value.token_count() != tokens.len() {
    return Err(format!(
        "token_count mismatch: counted {}, encoded {}",
        value.token_count(),
        tokens.len()
    ));
}
if let Some(max_token) = tokens.iter().max() {
    if *max_token >= VOCAB_SIZE {
        return Err(format!("token {max_token} outside VOCAB_SIZE {VOCAB_SIZE}"));
    }
}

let mut iter = tokens.into_iter();
let decoded = UnboundedIntegerContract::decode(&mut iter)
    .map_err(|error| error.to_string())?;
if let Some(extra) = iter.next() {
    return Err(format!("decode left an unconsumed token: {extra}"));
}

let decoded_json = serde_json::to_value(decoded).map_err(|error| error.to_string())?;
if decoded_json != original {
    return Err(format!("large unsigned integer round trip mismatch: expected {original}, got {decoded_json}"));
}

Ok(())
"#,
    );
}

#[test]
fn generated_integer_additional_properties_preserve_large_unsigned_values() {
    let schema = json!({
        "title": "TypedIntegerAdditionalProperties",
        "type": "object",
        "properties": {
            "payload": {
                "type": "object",
                "additionalProperties": {"type": "integer"}
            }
        },
        "required": ["payload"],
        "additionalProperties": false
    });

    let grammar = TctCompiler::new(schema, "TypedIntegerAdditionalProperties")
        .unwrap()
        .compile()
        .expect("typed integer additionalProperties schema should compile");
    let generated_code = grammar
        .generate_code()
        .expect("typed integer additionalProperties schema should generate code");

    run_generated_integer_contract_crate(
        "typed-integer-additional-properties-contract",
        &generated_code,
        r#"
let original = serde_json::json!({
    "payload": {
        "max": u64::MAX,
        "negative": -1
    }
});

let value: TypedIntegerAdditionalProperties = serde_json::from_value(original.clone())
    .map_err(|error| format!("generated type rejected valid typed integer additionalProperties: {error}"))?;

let fractional = serde_json::json!({"payload": {"not_integer": 1.25}});
if serde_json::from_value::<TypedIntegerAdditionalProperties>(fractional).is_ok() {
    return Err("typed integer additionalProperties accepted a fractional JSON number".to_string());
}

let mut tokens = Vec::new();
value.encode(&mut tokens);
if tokens.is_empty() {
    return Err("typed integer additionalProperties fixture encoded to no tokens".to_string());
}
if value.token_count() != tokens.len() {
    return Err(format!(
        "token_count mismatch: counted {}, encoded {}",
        value.token_count(),
        tokens.len()
    ));
}
if let Some(max_token) = tokens.iter().max() {
    if *max_token >= VOCAB_SIZE {
        return Err(format!("token {max_token} outside VOCAB_SIZE {VOCAB_SIZE}"));
    }
}

let mut iter = tokens.into_iter();
let decoded = TypedIntegerAdditionalProperties::decode(&mut iter)
    .map_err(|error| error.to_string())?;
if let Some(extra) = iter.next() {
    return Err(format!("decode left an unconsumed token: {extra}"));
}

let decoded_json = serde_json::to_value(decoded).map_err(|error| error.to_string())?;
if decoded_json != original {
    return Err(format!("typed integer additionalProperties round trip mismatch: expected {original}, got {decoded_json}"));
}

Ok(())
"#,
    );
}

#[test]
fn generated_root_unbounded_integer_is_not_constant_optimized() {
    let schema = json!({
        "title": "RootUnboundedInteger",
        "type": "integer"
    });

    let grammar = TctCompiler::new(schema, "RootUnboundedInteger")
        .unwrap()
        .compile()
        .expect("root unbounded integer schema should compile");
    let generated_code = grammar
        .generate_code()
        .expect("root unbounded integer schema should generate code");

    run_generated_integer_contract_crate(
        "root-unbounded-integer-contract",
        &generated_code,
        r#"
let original = serde_json::json!(u64::MAX);
let value: RootUnboundedInteger = serde_json::from_value(original.clone())
    .map_err(|error| format!("generated root integer rejected u64::MAX: {error}"))?;

let mut tokens = Vec::new();
value.encode(&mut tokens);
if tokens.is_empty() {
    return Err("root unbounded integer was encoded as zero tokens".to_string());
}
if value.token_count() != tokens.len() {
    return Err(format!(
        "token_count mismatch: counted {}, encoded {}",
        value.token_count(),
        tokens.len()
    ));
}

let mut iter = tokens.into_iter();
let decoded = RootUnboundedInteger::decode(&mut iter).map_err(|error| error.to_string())?;
if let Some(extra) = iter.next() {
    return Err(format!("decode left an unconsumed token: {extra}"));
}
let decoded_json = serde_json::to_value(decoded).map_err(|error| error.to_string())?;
if decoded_json != original {
    return Err(format!("root integer round trip mismatch: expected {original}, got {decoded_json}"));
}

Ok(())
"#,
    );
}

fn run_generated_integer_contract_crate(crate_name: &str, generated_code: &str, body: &str) {
    let crate_dir = tempfile::tempdir().expect("failed to create generated integer temp dir");
    let src_dir = crate_dir.path().join("src");
    fs::create_dir(&src_dir).expect("failed to create generated integer src dir");

    let tct_path = std::env::current_dir().expect("failed to resolve repo root");
    let cargo_toml = GeneratedCargoToml::new(crate_name)
        .with_preserve_order()
        .with_rustc_hash()
        .with_regex()
        .with_tct_path(tct_path.display().to_string())
        .with_bin(crate_name, "src/main.rs")
        .render();
    fs::write(crate_dir.path().join("Cargo.toml"), cargo_toml)
        .expect("failed to write generated integer Cargo.toml");
    copy_workspace_cargo_lock(&tct_path, crate_dir.path())
        .expect("failed to copy workspace Cargo.lock into generated integer crate");

    let main_rs = format!(
        r#"
#![allow(warnings)]

use crate::tokenizer::JsonTokenEncoder;

{generated_code}

fn main() -> Result<(), String> {{
{body}
}}
"#,
    );
    fs::write(src_dir.join("main.rs"), main_rs).expect("failed to write generated integer main.rs");

    let output = generated_integer_cargo_command()
        .arg("run")
        .arg("--quiet")
        .env(
            "CARGO_TARGET_DIR",
            std::env::current_dir()
                .unwrap()
                .join("target/tct-unbounded-integer-contracts"),
        )
        .current_dir(crate_dir.path())
        .output()
        .expect("failed to run generated integer contract crate");

    assert!(
        output.status.success(),
        "generated integer contract failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
