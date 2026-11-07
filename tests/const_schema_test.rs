//! Regression tests for JSON Schema `const` handling.

use serde_json::json;
use std::fs;
use std::path::Path;
use std::process::Command;
use tct::TctCompiler;
use tct::generated_crate::{
    GeneratedCargoToml, configure_generated_cargo_command, copy_workspace_cargo_lock,
};

fn generated_const_cargo_command() -> Command {
    let tct_path = std::env::current_dir().expect("failed to resolve repo root");
    let mut command = Command::new("cargo");
    configure_generated_cargo_command(&mut command, &tct_path);
    command
}

fn write_generated_const_cargo_toml(crate_dir: &Path, cargo_toml: GeneratedCargoToml) {
    let tct_path = std::env::current_dir().expect("failed to resolve repo root");
    let cargo_toml = cargo_toml
        .with_tct_path(tct_path.display().to_string())
        .render();
    fs::write(crate_dir.join("Cargo.toml"), cargo_toml)
        .expect("failed to write generated crate Cargo.toml");
    copy_workspace_cargo_lock(&tct_path, crate_dir)
        .expect("failed to copy workspace Cargo.lock into generated const crate");
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

fn generated_struct_has_public_field_type(
    file: &syn::File,
    struct_name: &str,
    field_name: &str,
    type_name: &str,
) -> bool {
    file.items.iter().any(|item| match item {
        syn::Item::Struct(item_struct) if item_struct.ident == struct_name => {
            let syn::Fields::Named(fields) = &item_struct.fields else {
                return false;
            };
            fields.named.iter().any(|field| {
                field
                    .ident
                    .as_ref()
                    .is_some_and(|ident| ident == field_name)
                    && matches!(field.vis, syn::Visibility::Public(_))
                    && type_ends_with_ident(&field.ty, type_name)
            })
        }
        _ => false,
    })
}

fn attr_is_serde_rename(attr: &syn::Attribute, expected_rename: &str) -> bool {
    if !attr.path().is_ident("serde") {
        return false;
    }

    let mut found = false;
    let parsed = attr.parse_nested_meta(|meta| {
        if meta.path.is_ident("rename") {
            let value = meta.value()?;
            let rename: syn::LitStr = value.parse()?;
            found = rename.value() == expected_rename;
        }
        Ok(())
    });

    parsed.is_ok() && found
}

fn generated_enum_has_serde_rename(
    file: &syn::File,
    enum_name: &str,
    expected_rename: &str,
) -> bool {
    file.items.iter().any(|item| match item {
        syn::Item::Enum(item_enum) if item_enum.ident == enum_name => {
            item_enum.variants.iter().any(|variant| {
                variant
                    .attrs
                    .iter()
                    .any(|attr| attr_is_serde_rename(attr, expected_rename))
            })
        }
        _ => false,
    })
}

#[test]
fn test_string_const_property_enforced_by_generated_serde_and_round_trip() {
    let schema = json!({
        "type": "object",
        "properties": {
            "mode": {"type": "string", "const": "premium"}
        },
        "required": ["mode"],
        "additionalProperties": false
    });

    let grammar = TctCompiler::new(schema, "ConstConfig")
        .unwrap()
        .compile()
        .unwrap();
    let generated_code = grammar.generate_code().unwrap();
    let generated_file = parse_generated_file(&generated_code);

    assert!(
        generated_struct_has_public_field_type(
            &generated_file,
            "ConstConfig",
            "mode",
            "ConstConfigMode"
        ),
        "string const should become a generated enum field:\n{}",
        generated_code
    );
    assert!(
        generated_enum_has_serde_rename(&generated_file, "ConstConfigMode", "premium"),
        "string const enum must preserve the original JSON literal:\n{}",
        generated_code
    );

    let crate_dir = tempfile::tempdir().expect("failed to create generated crate temp dir");
    let src_dir = crate_dir.path().join("src");
    fs::create_dir(&src_dir).expect("failed to create generated crate src dir");

    write_generated_const_cargo_toml(
        crate_dir.path(),
        GeneratedCargoToml::new("string-const-round-trip")
            .with_preserve_order()
            .with_rustc_hash()
            .with_bin("string-const-round-trip", "src/main.rs"),
    );

    let main_rs = format!(
        r#"
#![allow(warnings)]

use serde_json::{{json, Value}};
use crate::tokenizer::JsonTokenEncoder;

{generated_code}

fn main() -> Result<(), String> {{
    let good_json = json!({{"mode": "premium"}});
    let typed: ConstConfig = serde_json::from_value(good_json.clone())
        .map_err(|error| format!("generated type rejected valid const JSON: {{}}", error))?;

    let bad = serde_json::from_value::<ConstConfig>(json!({{"mode": "basic"}}));
    if bad.is_ok() {{
        return Err("generated type accepted JSON that violates string const".to_string());
    }}

    let mut tokens = Vec::new();
    typed.encode(&mut tokens);
    if let Some(max_token) = tokens.iter().max() {{
        if *max_token >= VOCAB_SIZE {{
            return Err(format!("token {{}} outside VOCAB_SIZE {{}}", max_token, VOCAB_SIZE));
        }}
    }}

    let mut token_iter = tokens.into_iter();
    let decoded = ConstConfig::decode(&mut token_iter)
        .map_err(|error| format!("decode failed: {{}}", error))?;
    if let Some(remaining_token) = token_iter.next() {{
        return Err(format!("Unconsumed token remains: {{}}", remaining_token));
    }}

    let decoded_json = serde_json::to_value(decoded)
        .map_err(|error| format!("decoded JSON serialization failed: {{}}", error))?;
    if decoded_json != good_json {{
        return Err(format!("decoded {{}} but expected {{}}", decoded_json, good_json));
    }}

    Ok(())
}}
"#
    );
    fs::write(src_dir.join("main.rs"), main_rs).expect("failed to write generated crate main.rs");

    let output = generated_const_cargo_command()
        .arg("run")
        .arg("--quiet")
        .current_dir(crate_dir.path())
        .output()
        .expect("failed to run generated string const crate");

    assert!(
        output.status.success(),
        "generated string const round trip failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn test_root_string_const_generated_crate_semantics() {
    let grammar = TctCompiler::new(json!({"const": "premium"}), "Plan")
        .unwrap()
        .compile()
        .unwrap();
    let generated_code = grammar
        .generate_code()
        .expect("root string const should generate code");

    run_generated_const_contract_crate(
        "root-string-const-round-trip",
        &generated_code,
        r#"
let expected = json!("premium");
let typed: Plan = serde_json::from_value(expected.clone())
    .map_err(|error| format!("generated type rejected valid root string const JSON: {}", error))?;

for invalid in [json!("basic"), json!(null), json!(7), json!({"value": "premium"})] {
    if serde_json::from_value::<Plan>(invalid).is_ok() {
        return Err("generated root string const accepted an invalid JSON value".to_string());
    }
}

let mut tokens = Vec::new();
typed.encode(&mut tokens);
if let Some(max_token) = tokens.iter().max() {
    if *max_token >= VOCAB_SIZE {
        return Err(format!("token {} outside VOCAB_SIZE {}", max_token, VOCAB_SIZE));
    }
}

let mut token_iter = tokens.into_iter();
let decoded = Plan::decode(&mut token_iter)
    .map_err(|error| format!("decode failed: {}", error))?;
if let Some(remaining_token) = token_iter.next() {
    return Err(format!("Unconsumed token remains: {}", remaining_token));
}
let decoded_json = serde_json::to_value(decoded)
    .map_err(|error| format!("decoded JSON serialization failed: {}", error))?;
if decoded_json != expected {
    return Err(format!("decoded {} but expected {}", decoded_json, expected));
}

Ok(())
"#,
    );
}

#[test]
fn test_root_null_const_generated_crate_semantics() {
    let grammar = TctCompiler::new(json!({"const": null}), "Nothing")
        .unwrap()
        .compile()
        .expect("root null const should compile");
    let generated_code = grammar
        .generate_code()
        .expect("root null const should generate code");

    run_generated_const_contract_crate(
        "root-null-const-round-trip",
        &generated_code,
        r#"
let expected = json!(null);
let typed: Nothing = serde_json::from_value(expected.clone())
    .map_err(|error| format!("generated type rejected valid root null const JSON: {}", error))?;

for invalid in [json!("null"), json!(false), json!(0), json!({})] {
    if serde_json::from_value::<Nothing>(invalid).is_ok() {
        return Err("generated root null const accepted an invalid JSON value".to_string());
    }
}

let mut tokens = Vec::new();
typed.encode(&mut tokens);
if !tokens.is_empty() {
    return Err(format!("root null const should be zero-token, got {tokens:?}"));
}

let mut token_iter = tokens.into_iter();
let decoded = Nothing::decode(&mut token_iter)
    .map_err(|error| format!("decode failed: {}", error))?;
if let Some(remaining_token) = token_iter.next() {
    return Err(format!("Unconsumed token remains: {}", remaining_token));
}
let decoded_json = serde_json::to_value(decoded)
    .map_err(|error| format!("decoded JSON serialization failed: {}", error))?;
if decoded_json != expected {
    return Err(format!("decoded {} but expected {}", decoded_json, expected));
}

Ok(())
"#,
    );
}

#[test]
fn test_non_string_const_properties_are_exact_zero_token_types() {
    let schema = json!({
        "type": "object",
        "properties": {
            "version": {"type": "integer", "const": 1},
            "max_u64": {"type": "integer", "const": 18446744073709551615u64},
            "ratio": {"type": "number", "const": 1.25},
            "flag": {"type": "boolean", "const": false},
            "nothing": {"const": null},
            "settings": {
                "const": {
                    "enabled": true,
                    "levels": [1, 2]
                }
            },
            "items": {
                "const": [1, "two", null]
            }
        },
        "required": ["version", "max_u64", "ratio", "flag", "nothing", "settings", "items"],
        "additionalProperties": false
    });

    let grammar = TctCompiler::new(schema, "ConstValues")
        .unwrap()
        .compile()
        .expect("non-string const values should compile as exact wrappers");
    let generated_code = grammar
        .generate_code()
        .expect("non-string const wrappers should generate code");

    let crate_dir = tempfile::tempdir().expect("failed to create generated crate temp dir");
    let src_dir = crate_dir.path().join("src");
    fs::create_dir(&src_dir).expect("failed to create generated crate src dir");

    write_generated_const_cargo_toml(
        crate_dir.path(),
        GeneratedCargoToml::new("non-string-const-round-trip")
            .with_preserve_order()
            .with_rustc_hash()
            .with_bin("non-string-const-round-trip", "src/main.rs"),
    );

    let main_rs = format!(
        r#"
#![allow(warnings)]

use serde_json::json;
use crate::tokenizer::JsonTokenEncoder;

{generated_code}

fn main() -> Result<(), String> {{
    let expected = json!({{
        "version": 1,
        "max_u64": 18446744073709551615u64,
        "ratio": 1.25,
        "flag": false,
        "nothing": null,
        "settings": {{"enabled": true, "levels": [1, 2]}},
        "items": [1, "two", null]
    }});

    let typed: ConstValues = serde_json::from_value(expected.clone())
        .map_err(|error| format!("generated type rejected valid const JSON: {{}}", error))?;

    for invalid in [
        json!({{"version": 2, "max_u64": 18446744073709551615u64, "ratio": 1.25, "flag": false, "nothing": null, "settings": {{"enabled": true, "levels": [1, 2]}}, "items": [1, "two", null]}}),
        json!({{"version": 1, "max_u64": 18446744073709551614u64, "ratio": 1.25, "flag": false, "nothing": null, "settings": {{"enabled": true, "levels": [1, 2]}}, "items": [1, "two", null]}}),
        json!({{"version": 1, "max_u64": 18446744073709551615u64, "ratio": 1.5, "flag": false, "nothing": null, "settings": {{"enabled": true, "levels": [1, 2]}}, "items": [1, "two", null]}}),
        json!({{"version": 1, "max_u64": 18446744073709551615u64, "ratio": 1.25, "flag": true, "nothing": null, "settings": {{"enabled": true, "levels": [1, 2]}}, "items": [1, "two", null]}}),
        json!({{"version": 1, "max_u64": 18446744073709551615u64, "ratio": 1.25, "flag": false, "nothing": "null", "settings": {{"enabled": true, "levels": [1, 2]}}, "items": [1, "two", null]}}),
        json!({{"version": 1, "max_u64": 18446744073709551615u64, "ratio": 1.25, "flag": false, "nothing": null, "settings": {{"enabled": false, "levels": [1, 2]}}, "items": [1, "two", null]}}),
        json!({{"version": 1, "max_u64": 18446744073709551615u64, "ratio": 1.25, "flag": false, "nothing": null, "settings": {{"enabled": true, "levels": [1, 2]}}, "items": [1, "two"]}}),
    ] {{
        if serde_json::from_value::<ConstValues>(invalid).is_ok() {{
            return Err("generated type accepted JSON that violates a non-string const".to_string());
        }}
    }}

    let direct = ConstValues {{
        version: ConstValuesVersionConst::new(),
        max_u64: ConstValuesMaxU64Const::new(),
        ratio: ConstValuesRatioConst::new(),
        flag: ConstValuesFlagConst::new(),
        nothing: ConstValuesNothingConst::new(),
        settings: ConstValuesSettingsConst::new(),
        items: ConstValuesItemsConst::new(),
    }};
    let direct_json = serde_json::to_value(direct)
        .map_err(|error| format!("direct const serialization failed: {{}}", error))?;
    if direct_json != expected {{
        return Err(format!("direct const construction serialized as {{direct_json}}, expected {{expected}}"));
    }}

    let mut tokens = Vec::new();
    typed.encode(&mut tokens);
    if !tokens.is_empty() {{
        return Err(format!("non-string const fields should be zero-token, got {{tokens:?}}"));
    }}

    let mut token_iter = tokens.into_iter();
    let decoded = ConstValues::decode(&mut token_iter)
        .map_err(|error| format!("decode failed: {{}}", error))?;
    if let Some(remaining_token) = token_iter.next() {{
        return Err(format!("Unconsumed token remains: {{}}", remaining_token));
    }}
    let decoded_json = serde_json::to_value(decoded)
        .map_err(|error| format!("decoded JSON serialization failed: {{}}", error))?;
    if decoded_json != expected {{
        return Err(format!("decoded {{}} but expected {{}}", decoded_json, expected));
    }}

    Ok(())
}}
"#
    );
    fs::write(src_dir.join("main.rs"), main_rs).expect("failed to write generated crate main.rs");

    let output = generated_const_cargo_command()
        .arg("run")
        .arg("--quiet")
        .current_dir(crate_dir.path())
        .output()
        .expect("failed to run generated non-string const crate");

    assert!(
        output.status.success(),
        "generated non-string const round trip failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn test_boolean_const_oneof_variant_is_exact_zero_token_type() {
    let schema = json!({
        "type": "object",
        "properties": {
            "comment": {
                "oneOf": [
                    {
                        "type": "object",
                        "properties": {
                            "require_changes": {"type": "boolean"}
                        },
                        "additionalProperties": false
                    },
                    {
                        "const": false
                    }
                ]
            }
        },
        "required": ["comment"],
        "additionalProperties": false
    });

    let grammar = TctCompiler::new(schema, "ConstConfig")
        .unwrap()
        .compile()
        .expect("boolean const oneOf variant should compile");
    let generated_code = grammar
        .generate_code()
        .expect("boolean const oneOf variant should generate code");

    let crate_dir = tempfile::tempdir().expect("failed to create generated crate temp dir");
    let src_dir = crate_dir.path().join("src");
    fs::create_dir(&src_dir).expect("failed to create generated crate src dir");

    write_generated_const_cargo_toml(
        crate_dir.path(),
        GeneratedCargoToml::new("boolean-const-round-trip")
            .with_preserve_order()
            .with_rustc_hash()
            .with_jsonschema()
            .with_bin("boolean-const-round-trip", "src/main.rs"),
    );

    let main_rs = format!(
        r#"
#![allow(warnings)]

use serde_json::json;
use crate::tokenizer::JsonTokenEncoder;

{generated_code}

fn main() -> Result<(), String> {{
    let false_json = json!({{"comment": false}});
    let typed: ConstConfig = serde_json::from_value(false_json.clone())
        .map_err(|error| format!("generated type rejected valid boolean const JSON: {{}}", error))?;

    if serde_json::from_value::<ConstConfig>(json!({{"comment": true}})).is_ok() {{
        return Err("generated type accepted true for const false variant".to_string());
    }}

    let direct = ConstConfig {{
        comment: ConstConfigCommentEnum::Variant1(ConstConfigCommentEnumVariant1Const::new()),
    }};
    let direct_json = serde_json::to_value(direct)
        .map_err(|error| format!("direct const serialization failed: {{}}", error))?;
    if direct_json != false_json {{
        return Err(format!("direct const construction serialized as {{direct_json}}, expected {{false_json}}"));
    }}

    let mut tokens = Vec::new();
    typed.encode(&mut tokens);
    if let Some(max_token) = tokens.iter().max() {{
        if *max_token >= VOCAB_SIZE {{
            return Err(format!("token {{}} outside VOCAB_SIZE {{}}", max_token, VOCAB_SIZE));
        }}
    }}

    let mut token_iter = tokens.into_iter();
    let decoded = ConstConfig::decode(&mut token_iter)
        .map_err(|error| format!("decode failed: {{}}", error))?;
    if let Some(remaining_token) = token_iter.next() {{
        return Err(format!("Unconsumed token remains: {{}}", remaining_token));
    }}
    let decoded_json = serde_json::to_value(decoded)
        .map_err(|error| format!("decoded JSON serialization failed: {{}}", error))?;
    if decoded_json != false_json {{
        return Err(format!("decoded {{}} but expected {{}}", decoded_json, false_json));
    }}

    Ok(())
}}
"#
    );
    fs::write(src_dir.join("main.rs"), main_rs).expect("failed to write generated crate main.rs");

    let output = generated_const_cargo_command()
        .arg("run")
        .arg("--quiet")
        .current_dir(crate_dir.path())
        .output()
        .expect("failed to run generated boolean const crate");

    assert!(
        output.status.success(),
        "generated boolean const round trip failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn run_generated_const_contract_crate(crate_name: &str, generated_code: &str, body: &str) {
    let crate_dir = tempfile::tempdir().expect("failed to create generated const temp dir");
    let src_dir = crate_dir.path().join("src");
    fs::create_dir(&src_dir).expect("failed to create generated const src dir");
    let tct_path = std::env::current_dir().expect("failed to resolve repo root");
    let shared_target_dir = tct_path.join("target").join("tct-const-contracts");
    fs::create_dir_all(&shared_target_dir).expect("failed to create shared const target dir");

    let mut cargo_toml = GeneratedCargoToml::new(crate_name)
        .with_preserve_order()
        .with_rustc_hash()
        .with_tct_path(tct_path.display().to_string())
        .with_bin(crate_name, "src/main.rs");
    if generated_code.contains("jsonschema ::") || generated_code.contains("jsonschema::") {
        cargo_toml = cargo_toml.with_jsonschema();
    }
    let cargo_toml = cargo_toml.render();
    fs::write(crate_dir.path().join("Cargo.toml"), cargo_toml)
        .expect("failed to write generated const Cargo.toml");
    copy_workspace_cargo_lock(&tct_path, crate_dir.path())
        .expect("failed to copy workspace Cargo.lock into generated const crate");

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
    fs::write(src_dir.join("main.rs"), main_rs).expect("failed to write generated const main.rs");

    let output = generated_const_cargo_command()
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
