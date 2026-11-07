//! Integration tests for streaming decode functionality.
//!
//! These tests verify that decode_prefix correctly handles nested structs,
//! single-variant enums, and token alignment across complex types.

use quote::ToTokens;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tct::generated_crate::{
    GeneratedCargoToml, configure_generated_cargo_command, copy_workspace_cargo_lock,
};

/// Test helper: Build a minimal test schema, generate code, and return it
fn build_and_test_schema(schema_json: &str, test_name: &str) -> Result<String, String> {
    build_and_test_schema_with_options(schema_json, test_name, None)
}

fn build_and_test_schema_with_optimization(
    schema_json: &str,
    test_name: &str,
    optimization_level: tct::OptimizationLevel,
) -> Result<String, String> {
    build_and_test_schema_with_options(schema_json, test_name, Some(optimization_level))
}

fn build_and_test_schema_with_options(
    schema_json: &str,
    test_name: &str,
    optimization_level: Option<tct::OptimizationLevel>,
) -> Result<String, String> {
    let output_dir = format!("/tmp/streaming-decode-test-{}", test_name);
    let _ = fs::remove_dir_all(&output_dir);
    fs::create_dir_all(&output_dir).map_err(|e| e.to_string())?;

    // Parse schema
    let schema: serde_json::Value = serde_json::from_str(schema_json).map_err(|e| e.to_string())?;

    // Use TctCompiler with streaming enabled
    let compiler = tct::TctCompiler::new(schema, "TestRoot")
        .map_err(|e| e.to_string())?
        .with_streaming_encode();
    let compiler = if let Some(optimization_level) = optimization_level {
        compiler.with_optimization_level(optimization_level)
    } else {
        compiler
    };
    let grammar = compiler.compile().map_err(|e| e.to_string())?;

    let code = grammar.generate_code().map_err(|e| e.to_string())?;

    // Write generated code for debugging
    let generated_path = format!("{}/generated.rs", output_dir);
    fs::write(&generated_path, &code).map_err(|e| e.to_string())?;

    Ok(code)
}

fn generated_contract_cargo_command() -> Command {
    let tct_path = std::env::current_dir().expect("failed to resolve repo root");
    let mut command = Command::new("cargo");
    configure_generated_cargo_command(&mut command, &tct_path);
    command.env("CARGO_TARGET_DIR", generated_contract_target_dir(&tct_path));
    command
}

fn generated_contract_target_dir(repo_root: &Path) -> PathBuf {
    let target_dir = std::env::var_os("TCT_GENERATED_CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            repo_root
                .join("target")
                .join("tct-generated-cargo")
                .join("streaming-decode")
        });
    fs::create_dir_all(&target_dir).unwrap_or_else(|error| {
        panic!(
            "failed to create generated streaming-decode Cargo target dir {}: {}",
            target_dir.display(),
            error
        )
    });
    target_dir
}

fn run_generated_streaming_contract(crate_name: &str, generated_code: &str, body: &str) {
    let crate_dir = tempfile::tempdir().expect("failed to create generated crate temp dir");
    let src_dir = crate_dir.path().join("src");
    fs::create_dir(&src_dir).expect("failed to create generated crate src dir");

    let tct_path = std::env::current_dir().expect("failed to resolve repo root");
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
        .expect("failed to write generated crate Cargo.toml");
    copy_workspace_cargo_lock(&tct_path, crate_dir.path())
        .expect("failed to copy workspace Cargo.lock into generated streaming decode crate");

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
        .output()
        .unwrap_or_else(|error| panic!("failed to run generated crate {crate_name}: {error}"));

    assert!(
        output.status.success(),
        "generated crate {} failed\nstdout:\n{}\nstderr:\n{}",
        crate_name,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn parse_generated_file(code: &str) -> syn::File {
    syn::parse_file(code).unwrap_or_else(|error| panic!("generated Rust should parse: {error}"))
}

fn impl_self_type_matches(item_impl: &syn::ItemImpl, type_name: &str) -> bool {
    match item_impl.self_ty.as_ref() {
        syn::Type::Path(type_path) => type_path
            .path
            .segments
            .last()
            .is_some_and(|segment| segment.ident == type_name),
        _ => false,
    }
}

fn generated_inherent_method<'a>(
    file: &'a syn::File,
    type_name: &str,
    method_name: &str,
) -> &'a syn::ImplItemFn {
    generated_inherent_method_opt(file, type_name, method_name)
        .unwrap_or_else(|| panic!("expected generated inherent method {type_name}::{method_name}"))
}

fn generated_inherent_method_opt<'a>(
    file: &'a syn::File,
    type_name: &str,
    method_name: &str,
) -> Option<&'a syn::ImplItemFn> {
    file.items.iter().find_map(|item| match item {
        syn::Item::Impl(item_impl)
            if item_impl.trait_.is_none() && impl_self_type_matches(item_impl, type_name) =>
        {
            item_impl
                .items
                .iter()
                .find_map(|impl_item| match impl_item {
                    syn::ImplItem::Fn(method) if method.sig.ident == method_name => Some(method),
                    _ => None,
                })
        }
        _ => None,
    })
}

fn generated_method_count(file: &syn::File, method_name: &str) -> usize {
    file.items
        .iter()
        .filter_map(|item| match item {
            syn::Item::Impl(item_impl) if item_impl.trait_.is_none() => Some(item_impl),
            _ => None,
        })
        .flat_map(|item_impl| item_impl.items.iter())
        .filter(|impl_item| match impl_item {
            syn::ImplItem::Fn(method) => method.sig.ident == method_name,
            _ => false,
        })
        .count()
}

fn method_first_arg_name(method: &syn::ImplItemFn) -> Option<String> {
    method.sig.inputs.iter().find_map(|arg| match arg {
        syn::FnArg::Typed(pat_type) => match pat_type.pat.as_ref() {
            syn::Pat::Ident(pat_ident) => Some(pat_ident.ident.to_string()),
            _ => None,
        },
        _ => None,
    })
}

fn method_body_tokens_contain(method: &syn::ImplItemFn, expected: &str) -> bool {
    method
        .block
        .to_token_stream()
        .to_string()
        .contains(expected)
}

#[test]
fn test_single_variant_enum_zero_token_decode() {
    // Test that single-variant enums don't consume tokens in decode_prefix
    let schema = r#"{
        "type": "object",
        "properties": {
            "kind": {
                "type": "string",
                "enum": ["OnlyKind"]
            },
            "name": {
                "type": "string"
            }
        },
        "required": ["kind", "name"]
    }"#;

    let code = build_and_test_schema(schema, "single_variant_enum").expect("Should generate code");
    let generated_file = parse_generated_file(&code);

    // The generated decode_prefix for kind should NOT call tokens.next()
    let kind_decode_prefix =
        generated_inherent_method(&generated_file, "TestRootKind", "decode_prefix");
    assert_eq!(
        method_first_arg_name(kind_decode_prefix).as_deref(),
        Some("_tokens"),
        "Single-variant Kind enum should NOT use tokens parameter"
    );
}

#[test]
fn test_single_variant_enum_decode_prefix_runtime_alignment_contract() {
    let schema = r#"{
        "type": "object",
        "additionalProperties": false,
        "properties": {
            "kind": {
                "type": "string",
                "enum": ["OnlyKind"]
            },
            "name": {
                "type": "string"
            }
        },
        "required": ["kind", "name"]
    }"#;

    let code =
        build_and_test_schema(schema, "single_variant_enum_runtime").expect("Should generate code");
    run_generated_streaming_contract(
        "single-variant-enum-decode-prefix",
        &code,
        r#"
let original: TestRoot = serde_json::from_value(serde_json::json!({
    "kind": "OnlyKind",
    "name": "alpha"
}))
.map_err(|error| format!("generated TestRoot deserialization failed: {error}"))?;

let mut tokens = Vec::new();
original.encode(&mut tokens);
if tokens.first() != Some(&(b'a' as u32)) {
    return Err(format!("single-variant enum should emit no discriminator before name tokens: {tokens:?}"));
}

let mut full_iter = tokens.clone().into_iter();
let prefix = TestRoot::decode_prefix(&mut full_iter);
if !prefix.complete {
    return Err(format!("decode_prefix reported a complete zero-token enum object as partial: {:?}", prefix.partial_field));
}
if prefix.fields_decoded != 2 {
    return Err(format!("expected two root fields decoded, got {}", prefix.fields_decoded));
}
if prefix.partial_field.is_some() {
    return Err(format!("complete decode_prefix should not report a partial field: {:?}", prefix.partial_field));
}
if full_iter.next().is_some() {
    return Err("decode_prefix left surplus tokens after zero-token enum field".to_string());
}

let decoded_json = serde_json::to_value(&prefix.instance)
    .map_err(|error| format!("generated TestRoot serialization failed: {error}"))?;
let expected_json = serde_json::json!({
    "kind": "OnlyKind",
    "name": "alpha"
});
if decoded_json != expected_json {
    return Err(format!("decode_prefix changed zero-token enum object data: got {decoded_json}, expected {expected_json}"));
}

let mut truncated = tokens.clone();
let removed = truncated
    .pop()
    .ok_or_else(|| "expected encoded string to contain an end marker".to_string())?;
if removed != 256u32 {
    return Err(format!("expected to remove the name end marker, got {removed}"));
}
let mut truncated_iter = truncated.into_iter();
let partial = TestRoot::decode_prefix(&mut truncated_iter);
if partial.complete {
    return Err("decode_prefix treated a truncated string after a zero-token enum as complete".to_string());
}
let partial_field = partial
    .partial_field
    .as_ref()
    .ok_or_else(|| "truncated string should report partial field metadata".to_string())?;
if partial_field.field_name != "name" || partial_field.field_type != "String" {
    return Err(format!("unexpected partial field metadata: {:?}", partial_field));
}
if partial_field.items_decoded != 5 || partial_field.expected_length.is_some() {
    return Err(format!("unexpected truncated string progress metadata: {:?}", partial_field));
}

Ok(())
"#,
    );
}

#[test]
fn test_two_variant_enum_consumes_token() {
    // Test that two-variant enums DO consume a discriminator token
    let schema = r#"{
        "type": "object",
        "properties": {
            "kind": {
                "type": "string",
                "enum": ["KindA", "KindB"]
            },
            "name": {
                "type": "string"
            }
        },
        "required": ["kind", "name"]
    }"#;

    let code = build_and_test_schema(schema, "two_variant_enum").expect("Should generate code");
    let generated_file = parse_generated_file(&code);

    // Two-variant enums MUST consume a token for discriminator
    let kind_decode_prefix =
        generated_inherent_method(&generated_file, "TestRootKind", "decode_prefix");
    assert!(
        method_body_tokens_contain(kind_decode_prefix, "tokens . next"),
        "Two-variant Kind enum MUST consume discriminator token"
    );
}

#[test]
fn test_two_variant_enum_decode_prefix_runtime_alignment_contract() {
    let schema = r#"{
        "type": "object",
        "additionalProperties": false,
        "properties": {
            "kind": {
                "type": "string",
                "enum": ["KindA", "KindB"]
            },
            "name": {
                "type": "string"
            }
        },
        "required": ["kind", "name"]
    }"#;

    let code =
        build_and_test_schema(schema, "two_variant_enum_runtime").expect("Should generate code");
    run_generated_streaming_contract(
        "two-variant-enum-decode-prefix",
        &code,
        r#"
let original: TestRoot = serde_json::from_value(serde_json::json!({
    "kind": "KindB",
    "name": "bravo"
}))
.map_err(|error| format!("generated TestRoot deserialization failed: {error}"))?;

let mut tokens = Vec::new();
original.encode(&mut tokens);
if tokens.first() != Some(&1u32) {
    return Err(format!("two-variant enum should emit KindB discriminator before name tokens: {tokens:?}"));
}

let mut full_iter = tokens.clone().into_iter();
let prefix = TestRoot::decode_prefix(&mut full_iter);
if !prefix.complete {
    return Err(format!("decode_prefix reported a complete two-variant enum object as partial: {:?}", prefix.partial_field));
}
if prefix.fields_decoded != 2 {
    return Err(format!("expected two root fields decoded, got {}", prefix.fields_decoded));
}
if prefix.partial_field.is_some() {
    return Err(format!("complete decode_prefix should not report a partial field: {:?}", prefix.partial_field));
}
if full_iter.next().is_some() {
    return Err("decode_prefix left surplus tokens after two-variant enum field".to_string());
}

let decoded_json = serde_json::to_value(&prefix.instance)
    .map_err(|error| format!("generated TestRoot serialization failed: {error}"))?;
let expected_json = serde_json::json!({
    "kind": "KindB",
    "name": "bravo"
});
if decoded_json != expected_json {
    return Err(format!("decode_prefix changed two-variant enum object data: got {decoded_json}, expected {expected_json}"));
}

let mut truncated = tokens.clone();
let removed = truncated
    .pop()
    .ok_or_else(|| "expected encoded string to contain an end marker".to_string())?;
if removed != 256u32 {
    return Err(format!("expected to remove the name end marker, got {removed}"));
}
let mut truncated_iter = truncated.into_iter();
let partial = TestRoot::decode_prefix(&mut truncated_iter);
if partial.complete {
    return Err("decode_prefix treated a truncated string after a two-variant enum as complete".to_string());
}
let partial_field = partial
    .partial_field
    .as_ref()
    .ok_or_else(|| "truncated string should report partial field metadata".to_string())?;
if partial_field.field_name != "name" || partial_field.field_type != "String" {
    return Err(format!("unexpected partial field metadata: {:?}", partial_field));
}
if partial_field.items_decoded != 5 || partial_field.expected_length.is_some() {
    return Err(format!("unexpected truncated string progress metadata: {:?}", partial_field));
}

Ok(())
"#,
    );
}

#[test]
fn test_nested_struct_token_alignment() {
    // Test that nested structs maintain correct token alignment
    // This is the core bug we fixed - single-variant enums were
    // consuming tokens that belonged to nested structs
    let schema = r#"{
        "type": "object",
        "properties": {
            "apiVersion": {"type": "string"},
            "kind": {
                "type": "string",
                "enum": ["TestResource"]
            },
            "metadata": {
                "type": "object",
                "properties": {
                    "name": {"type": "string"},
                    "namespace": {"type": "string"}
                }
            }
        },
        "required": ["apiVersion", "kind"]
    }"#;

    let code = build_and_test_schema(schema, "nested_struct").expect("Should generate code");
    let generated_file = parse_generated_file(&code);

    // Verify the generated code has decode_prefix for both root and metadata
    assert!(
        generated_method_count(&generated_file, "decode_prefix") >= 2,
        "Should generate decode_prefix methods"
    );

    // The Kind enum (single variant "TestResource") should NOT consume tokens
    let kind_decode_prefix =
        generated_inherent_method(&generated_file, "TestRootKind", "decode_prefix");
    assert_eq!(
        method_first_arg_name(kind_decode_prefix).as_deref(),
        Some("_tokens"),
        "TestRootKind (single variant) should NOT use tokens parameter"
    );
}

#[test]
fn test_nested_struct_decode_prefix_runtime_alignment_contract() {
    let schema = r#"{
        "type": "object",
        "additionalProperties": false,
        "properties": {
            "apiVersion": {"type": "string"},
            "kind": {
                "type": "string",
                "enum": ["TestResource"]
            },
            "metadata": {
                "type": "object",
                "additionalProperties": false,
                "properties": {
                    "name": {"type": "string"},
                    "namespace": {"type": "string"}
                },
                "required": ["name", "namespace"]
            }
        },
        "required": ["apiVersion", "kind"]
    }"#;

    let code =
        build_and_test_schema(schema, "nested_struct_runtime").expect("Should generate code");
    run_generated_streaming_contract(
        "nested-struct-decode-prefix",
        &code,
        r#"
let original: TestRoot = serde_json::from_value(serde_json::json!({
    "apiVersion": "v1",
    "kind": "TestResource",
    "metadata": {
        "name": "pod-a",
        "namespace": "default"
    }
}))
.map_err(|error| format!("generated TestRoot deserialization failed: {error}"))?;

let mut tokens = Vec::new();
original.encode(&mut tokens);

let mut full_iter = tokens.clone().into_iter();
let prefix = TestRoot::decode_prefix(&mut full_iter);
if !prefix.complete {
    return Err(format!("decode_prefix reported a complete nested struct as partial: {:?}", prefix.partial_field));
}
if prefix.fields_decoded != 3 {
    return Err(format!("expected three root fields decoded, got {}", prefix.fields_decoded));
}
if prefix.partial_field.is_some() {
    return Err(format!("complete decode_prefix should not report a partial field: {:?}", prefix.partial_field));
}
if full_iter.next().is_some() {
    return Err("decode_prefix left surplus tokens after a complete nested struct".to_string());
}

let decoded_json = serde_json::to_value(&prefix.instance)
    .map_err(|error| format!("generated TestRoot serialization failed: {error}"))?;
let expected_json = serde_json::json!({
    "apiVersion": "v1",
    "kind": "TestResource",
    "metadata": {
        "name": "pod-a",
        "namespace": "default"
    }
});
if decoded_json != expected_json {
    return Err(format!("decode_prefix changed nested struct data: got {decoded_json}, expected {expected_json}"));
}

let mut truncated = tokens.clone();
let removed = truncated
    .pop()
    .ok_or_else(|| "expected encoded nested struct to contain a namespace end marker".to_string())?;
if removed != 256u32 {
    return Err(format!("expected to remove the namespace end marker, got {removed}"));
}
let mut truncated_iter = truncated.into_iter();
let partial = TestRoot::decode_prefix(&mut truncated_iter);
if partial.complete {
    return Err("decode_prefix treated a truncated nested metadata field as complete".to_string());
}
if partial.fields_decoded != 3 {
    return Err(format!("expected truncated nested struct to count three root fields, got {}", partial.fields_decoded));
}
let partial_field = partial
    .partial_field
    .as_ref()
    .ok_or_else(|| "truncated nested struct should report partial field metadata".to_string())?;
if partial_field.field_name != "metadata" || partial_field.field_type != "TestRootMetadata" {
    return Err(format!("unexpected partial field metadata: {:?}", partial_field));
}
if partial_field.items_decoded != 2 || partial_field.expected_length.is_some() {
    return Err(format!("unexpected nested struct progress metadata: {:?}", partial_field));
}

Ok(())
"#,
    );
}

#[test]
fn test_nullable_nested_struct_decode_prefix_runtime_metadata_contract() {
    let schema = r##"{
        "type": "object",
        "additionalProperties": false,
        "properties": {
            "apiVersion": {"type": "string"},
            "kind": {
                "type": "string",
                "enum": ["TestResource"]
            },
            "metadata": {
                "anyOf": [
                    {"$ref": "#/definitions/Metadata"},
                    {"type": "null"}
                ]
            }
        },
        "required": ["apiVersion", "kind"],
        "definitions": {
            "Metadata": {
                "type": "object",
                "additionalProperties": false,
                "properties": {
                    "name": {"type": "string"},
                    "namespace": {"type": "string"}
                },
                "required": ["name", "namespace"]
            }
        }
    }"##;

    let code = build_and_test_schema(schema, "nullable_nested_struct_runtime")
        .expect("Should generate code");
    run_generated_streaming_contract(
        "nullable-nested-struct-decode-prefix",
        &code,
        r#"
let original: TestRoot = serde_json::from_value(serde_json::json!({
    "apiVersion": "v1",
    "kind": "TestResource",
    "metadata": {
        "name": "pod-a",
        "namespace": "default"
    }
}))
.map_err(|error| format!("generated TestRoot deserialization failed: {error}"))?;

let mut tokens = Vec::new();
original.encode(&mut tokens);

let mut full_iter = tokens.clone().into_iter();
let prefix = TestRoot::decode_prefix(&mut full_iter);
if !prefix.complete {
    return Err(format!("decode_prefix reported a complete nullable nested struct as partial: {:?}", prefix.partial_field));
}
if prefix.fields_decoded != 3 {
    return Err(format!("expected three root fields decoded, got {}", prefix.fields_decoded));
}
if prefix.partial_field.is_some() {
    return Err(format!("complete decode_prefix should not report a partial field: {:?}", prefix.partial_field));
}
if full_iter.next().is_some() {
    return Err("decode_prefix left surplus tokens after a complete nullable nested struct".to_string());
}

let decoded_json = serde_json::to_value(&prefix.instance)
    .map_err(|error| format!("generated TestRoot serialization failed: {error}"))?;
let expected_json = serde_json::json!({
    "apiVersion": "v1",
    "kind": "TestResource",
    "metadata": {
        "name": "pod-a",
        "namespace": "default"
    }
});
if decoded_json != expected_json {
    return Err(format!("decode_prefix changed nullable nested struct data: got {decoded_json}, expected {expected_json}"));
}

let explicit_null: TestRoot = serde_json::from_value(serde_json::json!({
    "apiVersion": "v1",
    "kind": "TestResource",
    "metadata": null
}))
.map_err(|error| format!("generated TestRoot null deserialization failed: {error}"))?;
let mut null_tokens = Vec::new();
explicit_null.encode(&mut null_tokens);
let mut null_iter = null_tokens.into_iter();
let null_prefix = TestRoot::decode_prefix(&mut null_iter);
if !null_prefix.complete || null_prefix.partial_field.is_some() || null_iter.next().is_some() {
    return Err(format!("decode_prefix did not complete cleanly for explicit null metadata: {:?}", null_prefix.partial_field));
}
let null_json = serde_json::to_value(&null_prefix.instance)
    .map_err(|error| format!("generated TestRoot null serialization failed: {error}"))?;
let expected_null_json = serde_json::json!({
    "apiVersion": "v1",
    "kind": "TestResource",
    "metadata": null
});
if null_json != expected_null_json {
    return Err(format!("decode_prefix changed explicit null metadata: got {null_json}, expected {expected_null_json}"));
}

let mut truncated = tokens.clone();
let removed = truncated
    .pop()
    .ok_or_else(|| "expected encoded nullable nested struct to contain a namespace end marker".to_string())?;
if removed != 256u32 {
    return Err(format!("expected to remove the namespace end marker, got {removed}"));
}
let mut truncated_iter = truncated.into_iter();
let partial = TestRoot::decode_prefix(&mut truncated_iter);
if partial.complete {
    return Err("decode_prefix treated a truncated nullable nested metadata field as complete".to_string());
}
if partial.fields_decoded != 3 {
    return Err(format!("expected truncated nullable nested struct to count three root fields, got {}", partial.fields_decoded));
}
let partial_field = partial
    .partial_field
    .as_ref()
    .ok_or_else(|| "truncated nullable nested struct should report partial field metadata".to_string())?;
if partial_field.field_name != "metadata" || partial_field.field_type != "TestRootMetadataEnum" {
    return Err(format!("unexpected partial field metadata: {:?}", partial_field));
}
if partial_field.items_decoded != 2 || partial_field.expected_length.is_some() {
    return Err(format!("unexpected nullable nested struct progress metadata: {:?}", partial_field));
}

Ok(())
"#,
    );
}

#[test]
fn test_optional_nested_struct_with_single_variant_kind() {
    // This mimics Kubernetes resource structure:
    // - Required apiVersion (string)
    // - Required kind (single-variant enum)
    // - Optional metadata (nested struct)
    let schema = r#"{
        "type": "object",
        "properties": {
            "apiVersion": {"type": "string"},
            "kind": {
                "type": "string",
                "enum": ["Pod"]
            },
            "metadata": {
                "type": "object",
                "properties": {
                    "name": {"type": "string"},
                    "labels": {
                        "type": "object",
                        "additionalProperties": {"type": "string"}
                    }
                }
            },
            "spec": {
                "type": "object",
                "properties": {
                    "containers": {
                        "type": "array",
                        "items": {"type": "string"}
                    }
                }
            }
        },
        "required": ["apiVersion", "kind"]
    }"#;

    let code = build_and_test_schema(schema, "k8s_like_resource").expect("Should generate code");
    let generated_file = parse_generated_file(&code);

    // Verify decode_prefix exists
    assert!(
        generated_method_count(&generated_file, "decode_prefix") > 0,
        "Should generate decode_prefix"
    );

    // Count decode_prefix occurrences - should have multiple
    let decode_prefix_count = generated_method_count(&generated_file, "decode_prefix");
    assert!(
        decode_prefix_count >= 2,
        "Should have decode_prefix for multiple types, found {}",
        decode_prefix_count
    );
}

#[test]
fn test_deeply_nested_structs() {
    // Test multiple levels of nesting
    let schema = r#"{
        "type": "object",
        "properties": {
            "level1": {
                "type": "object",
                "properties": {
                    "level2": {
                        "type": "object",
                        "properties": {
                            "level3": {
                                "type": "object",
                                "properties": {
                                    "value": {"type": "string"}
                                }
                            }
                        }
                    }
                }
            }
        }
    }"#;

    let code = build_and_test_schema(schema, "deeply_nested").expect("Should generate code");
    let generated_file = parse_generated_file(&code);

    // Should generate decode_prefix for nested types
    assert!(
        generated_method_count(&generated_file, "decode_prefix") > 0,
        "Should generate decode_prefix for nested structs"
    );
}

#[test]
fn test_additional_properties_fallback() {
    // Test that additional_properties uses unwrap_or_default fallback
    let schema = r#"{
        "type": "object",
        "properties": {
            "name": {"type": "string"}
        },
        "additionalProperties": true
    }"#;

    let code = build_and_test_schema(schema, "additional_props").expect("Should generate code");
    let generated_file = parse_generated_file(&code);

    // Find decode_prefix and verify additional_properties handling
    let decode_prefix = generated_inherent_method(&generated_file, "TestRoot", "decode_prefix");

    // Should use unwrap_or_default for additional_properties
    if method_body_tokens_contain(decode_prefix, "additional_properties") {
        assert!(
            method_body_tokens_contain(decode_prefix, "unwrap_or_default"),
            "additional_properties should use unwrap_or_default fallback"
        );
    }
}

#[test]
fn test_mixed_required_optional_fields() {
    // Test struct with mix of required and optional fields
    let schema = r#"{
        "type": "object",
        "properties": {
            "required_field": {"type": "string"},
            "optional_field1": {"type": "string"},
            "optional_field2": {"type": "integer"},
            "optional_nested": {
                "type": "object",
                "properties": {
                    "inner": {"type": "string"}
                }
            }
        },
        "required": ["required_field"]
    }"#;

    let code = build_and_test_schema(schema, "mixed_fields").expect("Should generate code");
    let generated_file = parse_generated_file(&code);

    // Optional fields should be represented as Option<T>
    assert!(
        code.contains("Option<String>") && code.contains("Option<crate::tokenizer::JsonInteger>"),
        "Should generate Option<T> for optional fields"
    );

    // Optional fields should have skip_serializing_if for proper JSON serialization
    assert!(
        code.contains("skip_serializing_if"),
        "Optional fields should have skip_serializing_if attribute"
    );

    // decode_prefix should exist
    assert!(
        generated_method_count(&generated_file, "decode_prefix") > 0,
        "Should generate decode_prefix"
    );
}

#[test]
fn test_array_of_nested_structs() {
    // Test array containing nested structs
    let schema = r#"{
        "type": "object",
        "properties": {
            "items": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "name": {"type": "string"},
                        "value": {"type": "integer"}
                    },
                    "required": ["name"]
                }
            }
        }
    }"#;

    let code = build_and_test_schema(schema, "array_nested").expect("Should generate code");
    let generated_file = parse_generated_file(&code);

    // Should generate decode_prefix
    assert!(
        generated_method_count(&generated_file, "decode_prefix") > 0,
        "Should generate decode_prefix for array items"
    );
}

#[test]
fn test_required_array_of_nested_structs_decode_prefix_runtime_contract() {
    let schema = r#"{
        "type": "object",
        "additionalProperties": false,
        "properties": {
            "items": {
                "type": "array",
                "items": {
                    "type": "object",
                    "additionalProperties": false,
                    "properties": {
                        "name": {"type": "string"}
                    },
                    "required": ["name"]
                }
            }
        },
        "required": ["items"]
    }"#;

    let code = build_and_test_schema(schema, "required_array_nested_runtime")
        .expect("Should generate code");
    run_generated_streaming_contract(
        "required-array-nested-decode-prefix",
        &code,
        r#"
let original: TestRoot = serde_json::from_value(serde_json::json!({
    "items": [
        {"name": "alpha"},
        {"name": "beta"}
    ]
}))
.map_err(|error| format!("generated TestRoot deserialization failed: {error}"))?;

let mut tokens = Vec::new();
original.encode(&mut tokens);

let mut full_iter = tokens.clone().into_iter();
let prefix = TestRoot::decode_prefix(&mut full_iter);
if !prefix.complete {
    return Err(format!("decode_prefix reported a complete nested array as partial: {:?}", prefix.partial_field));
}
if prefix.fields_decoded != 1 {
    return Err(format!("expected one root field decoded, got {}", prefix.fields_decoded));
}
if prefix.partial_field.is_some() {
    return Err(format!("complete decode_prefix should not report a partial field: {:?}", prefix.partial_field));
}
if full_iter.next().is_some() {
    return Err("decode_prefix left surplus tokens after a complete nested array".to_string());
}

let decoded_json = serde_json::to_value(&prefix.instance)
    .map_err(|error| format!("generated TestRoot serialization failed: {error}"))?;
let expected_json = serde_json::json!({
    "items": [
        {"name": "alpha"},
        {"name": "beta"}
    ]
});
if decoded_json != expected_json {
    return Err(format!("decode_prefix changed nested array data: got {decoded_json}, expected {expected_json}"));
}

let mut truncated = tokens.clone();
let removed = truncated
    .pop()
    .ok_or_else(|| "expected encoded nested array to contain tokens".to_string())?;
if removed != 256u32 {
    return Err(format!("expected to remove the second name end marker, got {removed}"));
}
let mut truncated_iter = truncated.into_iter();
let partial = TestRoot::decode_prefix(&mut truncated_iter);
if partial.complete {
    return Err("decode_prefix treated a truncated nested array element as complete".to_string());
}
let partial_field = partial
    .partial_field
    .as_ref()
    .ok_or_else(|| "truncated nested array should report partial field metadata".to_string())?;
if partial_field.field_name != "items" || partial_field.field_type != "Vec" {
    return Err(format!("unexpected partial field metadata: {:?}", partial_field));
}
if partial_field.items_decoded != 2 || partial_field.expected_length != Some(2) {
    return Err(format!("unexpected nested array progress metadata: {:?}", partial_field));
}

Ok(())
"#,
    );
}

#[test]
fn test_root_array_of_nested_structs_decode_prefix_runtime_contract() {
    let schema = r##"{
        "type": "array",
        "items": {
            "$ref": "#/definitions/Subject"
        },
        "definitions": {
            "Subject": {
                "type": "object",
                "additionalProperties": false,
                "properties": {
                    "name": {"type": "string"}
                },
                "required": ["name"]
            }
        }
    }"##;

    let code =
        build_and_test_schema(schema, "root_array_nested_runtime").expect("Should generate code");
    run_generated_streaming_contract(
        "root-array-nested-decode-prefix",
        &code,
        r#"
let original: TestRoot = serde_json::from_value(serde_json::json!([
    {"name": "alpha"},
    {"name": "beta"}
]))
.map_err(|error| format!("generated TestRoot deserialization failed: {error}"))?;

let mut tokens = Vec::new();
original.encode(&mut tokens);

let mut full_iter = tokens.clone().into_iter();
let prefix = TestRoot::decode_prefix(&mut full_iter);
if !prefix.complete {
    return Err(format!("decode_prefix reported a complete root nested array as partial: {:?}", prefix.partial_field));
}
if prefix.fields_decoded != 2 {
    return Err(format!("expected two root array elements decoded, got {}", prefix.fields_decoded));
}
if prefix.partial_field.is_some() {
    return Err(format!("complete decode_prefix should not report a partial field: {:?}", prefix.partial_field));
}
if full_iter.next().is_some() {
    return Err("decode_prefix left surplus tokens after a complete root nested array".to_string());
}

let decoded_json = serde_json::to_value(&prefix.instance)
    .map_err(|error| format!("generated TestRoot serialization failed: {error}"))?;
let expected_json = serde_json::json!([
    {"name": "alpha"},
    {"name": "beta"}
]);
if decoded_json != expected_json {
    return Err(format!("decode_prefix changed root nested array data: got {decoded_json}, expected {expected_json}"));
}

let mut truncated = tokens.clone();
let removed = truncated
    .pop()
    .ok_or_else(|| "expected encoded root nested array to contain tokens".to_string())?;
if removed != 256u32 {
    return Err(format!("expected to remove the second name end marker, got {removed}"));
}
let mut truncated_iter = truncated.into_iter();
let partial = TestRoot::decode_prefix(&mut truncated_iter);
if partial.complete {
    return Err("decode_prefix treated a truncated root nested array element as complete".to_string());
}
let partial_field = partial
    .partial_field
    .as_ref()
    .ok_or_else(|| "truncated root nested array should report partial field metadata".to_string())?;
if partial_field.field_name != "elements" || partial_field.field_type != "Vec" {
    return Err(format!("unexpected partial field metadata: {:?}", partial_field));
}
if partial_field.items_decoded != 2 || partial_field.expected_length != Some(2) {
    return Err(format!("unexpected root nested array progress metadata: {:?}", partial_field));
}

Ok(())
"#,
    );
}

#[test]
fn test_root_array_of_const_refs_decode_prefix_runtime_contract() {
    let schema = r##"{
        "type": "array",
        "items": {
            "$ref": "#/definitions/ReadyState"
        },
        "definitions": {
            "ReadyState": {
                "const": "ready"
            }
        }
    }"##;

    let code =
        build_and_test_schema(schema, "root_array_const_runtime").expect("Should generate code");
    run_generated_streaming_contract(
        "root-array-const-ref-decode-prefix",
        &code,
        r#"
let original: TestRoot = serde_json::from_value(serde_json::json!(["ready", "ready"]))
    .map_err(|error| format!("generated TestRoot deserialization failed: {error}"))?;

let mut tokens = Vec::new();
original.encode(&mut tokens);

let mut full_iter = tokens.clone().into_iter();
let prefix = TestRoot::decode_prefix(&mut full_iter);
if !prefix.complete {
    return Err(format!("decode_prefix reported a complete root const array as partial: {:?}", prefix.partial_field));
}
if prefix.fields_decoded != 2 {
    return Err(format!("expected two root const array elements decoded, got {}", prefix.fields_decoded));
}
if prefix.partial_field.is_some() {
    return Err(format!("complete const array decode_prefix should not report a partial field: {:?}", prefix.partial_field));
}
if full_iter.next().is_some() {
    return Err("decode_prefix left surplus tokens after a complete root const array".to_string());
}

let decoded_json = serde_json::to_value(&prefix.instance)
    .map_err(|error| format!("generated TestRoot serialization failed: {error}"))?;
let expected_json = serde_json::json!(["ready", "ready"]);
if decoded_json != expected_json {
    return Err(format!("decode_prefix changed root const array data: got {decoded_json}, expected {expected_json}"));
}

Ok(())
"#,
    );
}

#[test]
fn test_required_array_of_strings_decode_prefix_preserves_partial_element_runtime_contract() {
    let schema = r#"{
        "type": "object",
        "additionalProperties": false,
        "properties": {
            "names": {
                "type": "array",
                "items": {"type": "string"}
            }
        },
        "required": ["names"]
    }"#;

    let code = build_and_test_schema(schema, "required_string_array_runtime")
        .expect("Should generate code");
    run_generated_streaming_contract(
        "required-string-array-decode-prefix",
        &code,
        r#"
let original: TestRoot = serde_json::from_value(serde_json::json!({
    "names": ["alpha", "beta"]
}))
.map_err(|error| format!("generated TestRoot deserialization failed: {error}"))?;

let mut tokens = Vec::new();
original.encode(&mut tokens);

let mut full_iter = tokens.clone().into_iter();
let prefix = TestRoot::decode_prefix(&mut full_iter);
if !prefix.complete {
    return Err(format!("decode_prefix reported a complete string array as partial: {:?}", prefix.partial_field));
}
if prefix.partial_field.is_some() {
    return Err(format!("complete string array decode_prefix should not report a partial field: {:?}", prefix.partial_field));
}
if full_iter.next().is_some() {
    return Err("decode_prefix left surplus tokens after a complete string array".to_string());
}

let mut truncated = tokens.clone();
let removed = truncated
    .pop()
    .ok_or_else(|| "expected encoded string array to contain tokens".to_string())?;
if removed != 256u32 {
    return Err(format!("expected to remove the second string end marker, got {removed}"));
}
let mut truncated_iter = truncated.into_iter();
let partial = TestRoot::decode_prefix(&mut truncated_iter);
if partial.complete {
    return Err("decode_prefix treated a truncated string array element as complete".to_string());
}
let partial_field = partial
    .partial_field
    .as_ref()
    .ok_or_else(|| "truncated string array should report partial field metadata".to_string())?;
if partial_field.field_name != "names" || partial_field.field_type != "Vec" {
    return Err(format!("unexpected partial field metadata: {:?}", partial_field));
}
if partial_field.items_decoded != 2 || partial_field.expected_length != Some(2) {
    return Err(format!("unexpected string array progress metadata: {:?}", partial_field));
}

let partial_json = serde_json::to_value(&partial.instance)
    .map_err(|error| format!("partial TestRoot serialization failed: {error}"))?;
let expected_partial_json = serde_json::json!({
    "names": ["alpha", "beta"]
});
if partial_json != expected_partial_json {
    return Err(format!("decode_prefix dropped the partial string array element: got {partial_json}, expected {expected_partial_json}"));
}

Ok(())
"#,
    );
}

#[test]
fn test_root_array_of_strings_decode_prefix_preserves_partial_element_runtime_contract() {
    let schema = r#"{
        "type": "array",
        "items": {"type": "string"}
    }"#;

    let code =
        build_and_test_schema(schema, "root_string_array_runtime").expect("Should generate code");
    run_generated_streaming_contract(
        "root-string-array-decode-prefix",
        &code,
        r#"
let original: TestRoot = serde_json::from_value(serde_json::json!(["alpha", "beta"]))
    .map_err(|error| format!("generated TestRoot deserialization failed: {error}"))?;

let mut tokens = Vec::new();
original.encode(&mut tokens);

let mut full_iter = tokens.clone().into_iter();
let prefix = TestRoot::decode_prefix(&mut full_iter);
if !prefix.complete {
    return Err(format!("decode_prefix reported a complete root string array as partial: {:?}", prefix.partial_field));
}
if prefix.fields_decoded != 2 {
    return Err(format!("expected two root string array elements decoded, got {}", prefix.fields_decoded));
}
if prefix.partial_field.is_some() {
    return Err(format!("complete root string array decode_prefix should not report a partial field: {:?}", prefix.partial_field));
}
if full_iter.next().is_some() {
    return Err("decode_prefix left surplus tokens after a complete root string array".to_string());
}

let mut truncated = tokens.clone();
let removed = truncated
    .pop()
    .ok_or_else(|| "expected encoded root string array to contain tokens".to_string())?;
if removed != 256u32 {
    return Err(format!("expected to remove the second string end marker, got {removed}"));
}
let mut truncated_iter = truncated.into_iter();
let partial = TestRoot::decode_prefix(&mut truncated_iter);
if partial.complete {
    return Err("decode_prefix treated a truncated root string array element as complete".to_string());
}
let partial_field = partial
    .partial_field
    .as_ref()
    .ok_or_else(|| "truncated root string array should report partial field metadata".to_string())?;
if partial_field.field_name != "elements" || partial_field.field_type != "Vec" {
    return Err(format!("unexpected partial field metadata: {:?}", partial_field));
}
if partial_field.items_decoded != 2 || partial_field.expected_length != Some(2) {
    return Err(format!("unexpected root string array progress metadata: {:?}", partial_field));
}

let partial_json = serde_json::to_value(&partial.instance)
    .map_err(|error| format!("partial TestRoot serialization failed: {error}"))?;
let expected_partial_json = serde_json::json!(["alpha", "beta"]);
if partial_json != expected_partial_json {
    return Err(format!("decode_prefix dropped the partial root string array element: got {partial_json}, expected {expected_partial_json}"));
}

Ok(())
"#,
    );
}

#[test]
fn test_nullable_array_of_strings_decode_prefix_preserves_partial_element_runtime_contract() {
    let schema = r#"{
        "type": "object",
        "additionalProperties": false,
        "properties": {
            "name": {"type": "string"},
            "names": {
                "anyOf": [
                    {
                        "type": "array",
                        "items": {"type": "string"}
                    },
                    {"type": "null"}
                ]
            }
        },
        "required": ["name"]
    }"#;

    let code = build_and_test_schema_with_optimization(
        schema,
        "nullable_string_array_runtime",
        tct::OptimizationLevel::O2,
    )
    .expect("Should generate code");
    run_generated_streaming_contract(
        "nullable-string-array-decode-prefix",
        &code,
        r#"
let original: TestRoot = serde_json::from_value(serde_json::json!({
    "name": "root",
    "names": ["alpha", "beta"]
}))
.map_err(|error| format!("generated TestRoot deserialization failed: {error}"))?;

let mut tokens = Vec::new();
original.encode(&mut tokens);

let mut full_iter = tokens.clone().into_iter();
let prefix = TestRoot::decode_prefix(&mut full_iter);
if !prefix.complete {
    return Err(format!("decode_prefix reported a complete nullable string array as partial: {:?}", prefix.partial_field));
}
if prefix.fields_decoded != 2 {
    return Err(format!("expected two root fields decoded, got {}", prefix.fields_decoded));
}
if prefix.partial_field.is_some() {
    return Err(format!("complete nullable string array decode_prefix should not report a partial field: {:?}", prefix.partial_field));
}
if full_iter.next().is_some() {
    return Err("decode_prefix left surplus tokens after a complete nullable string array".to_string());
}

let decoded_json = serde_json::to_value(&prefix.instance)
    .map_err(|error| format!("generated TestRoot serialization failed: {error}"))?;
let expected_json = serde_json::json!({
    "name": "root",
    "names": ["alpha", "beta"]
});
if decoded_json != expected_json {
    return Err(format!("decode_prefix changed nullable string array data: got {decoded_json}, expected {expected_json}"));
}

let explicit_null: TestRoot = serde_json::from_value(serde_json::json!({
    "name": "root",
    "names": null
}))
.map_err(|error| format!("generated TestRoot null deserialization failed: {error}"))?;
let mut null_tokens = Vec::new();
explicit_null.encode(&mut null_tokens);
let mut null_iter = null_tokens.into_iter();
let null_prefix = TestRoot::decode_prefix(&mut null_iter);
if !null_prefix.complete || null_prefix.partial_field.is_some() || null_iter.next().is_some() {
    return Err(format!("decode_prefix did not complete cleanly for explicit null nullable string array: {:?}", null_prefix.partial_field));
}
let null_json = serde_json::to_value(&null_prefix.instance)
    .map_err(|error| format!("generated TestRoot null serialization failed: {error}"))?;
let expected_null_json = serde_json::json!({
    "name": "root",
    "names": null
});
if null_json != expected_null_json {
    return Err(format!("decode_prefix changed explicit null nullable string array: got {null_json}, expected {expected_null_json}"));
}

let absent: TestRoot = serde_json::from_value(serde_json::json!({
    "name": "root"
}))
.map_err(|error| format!("generated TestRoot absent deserialization failed: {error}"))?;
let mut absent_tokens = Vec::new();
absent.encode(&mut absent_tokens);
let mut absent_iter = absent_tokens.into_iter();
let absent_prefix = TestRoot::decode_prefix(&mut absent_iter);
if !absent_prefix.complete || absent_prefix.partial_field.is_some() || absent_iter.next().is_some() {
    return Err(format!("decode_prefix did not complete cleanly for absent nullable string array: {:?}", absent_prefix.partial_field));
}
let absent_json = serde_json::to_value(&absent_prefix.instance)
    .map_err(|error| format!("generated TestRoot absent serialization failed: {error}"))?;
let expected_absent_json = serde_json::json!({
    "name": "root"
});
if absent_json != expected_absent_json {
    return Err(format!("decode_prefix changed absent nullable string array: got {absent_json}, expected {expected_absent_json}"));
}

let mut truncated = tokens.clone();
let removed = truncated
    .pop()
    .ok_or_else(|| "expected encoded nullable string array to contain tokens".to_string())?;
if removed != 256u32 {
    return Err(format!("expected to remove the second string end marker, got {removed}"));
}
let mut truncated_iter = truncated.into_iter();
let partial = TestRoot::decode_prefix(&mut truncated_iter);
if partial.complete {
    return Err("decode_prefix treated a truncated nullable string array element as complete".to_string());
}
let partial_field = partial
    .partial_field
    .as_ref()
    .ok_or_else(|| "truncated nullable string array should report partial field metadata".to_string())?;
if partial_field.field_name != "names" || partial_field.field_type != "Vec" {
    return Err(format!("unexpected partial field metadata: {:?}", partial_field));
}
if partial_field.items_decoded != 2 || partial_field.expected_length != Some(2) {
    return Err(format!("unexpected nullable string array progress metadata: {:?}", partial_field));
}

let partial_json = serde_json::to_value(&partial.instance)
    .map_err(|error| format!("partial TestRoot serialization failed: {error}"))?;
let expected_partial_json = serde_json::json!({
    "name": "root",
    "names": ["alpha", "beta"]
});
if partial_json != expected_partial_json {
    return Err(format!("decode_prefix dropped the partial nullable string array element: got {partial_json}, expected {expected_partial_json}"));
}

Ok(())
"#,
    );
}

#[test]
fn test_zero_token_optional_string_array_decode_prefix_runtime_contract() {
    let schema = r#"{
        "type": "object",
        "additionalProperties": false,
        "properties": {
            "kind": {"const": "Widget"},
            "names": {
                "type": "array",
                "items": {"type": "string"}
            },
            "optional_0": {"type": "string"},
            "optional_1": {"type": "string"},
            "optional_2": {"type": "string"}
        },
        "required": ["kind"]
    }"#;

    let code = build_and_test_schema(schema, "optional_string_array_zero_token_runtime")
        .expect("Should generate code");
    run_generated_streaming_contract(
        "optional-string-array-zero-token-decode-prefix",
        &code,
        r#"
let original: TestRoot = serde_json::from_value(serde_json::json!({
    "kind": "Widget",
    "names": ["alpha", "beta"]
}))
.map_err(|error| format!("generated TestRoot deserialization failed: {error}"))?;

let mut tokens = Vec::new();
original.encode(&mut tokens);

let mut full_iter = tokens.clone().into_iter();
let prefix = TestRoot::decode_prefix(&mut full_iter);
if !prefix.complete {
    return Err(format!("decode_prefix reported a complete optional string array as partial: {:?}", prefix.partial_field));
}
if prefix.partial_field.is_some() {
    return Err(format!("complete optional string array decode_prefix should not report a partial field: {:?}", prefix.partial_field));
}
if full_iter.next().is_some() {
    return Err("decode_prefix left surplus tokens after a complete optional string array".to_string());
}

let decoded_json = serde_json::to_value(&prefix.instance)
    .map_err(|error| format!("generated TestRoot serialization failed: {error}"))?;
let expected_json = serde_json::json!({
    "kind": "Widget",
    "names": ["alpha", "beta"]
});
if decoded_json != expected_json {
    return Err(format!("decode_prefix changed optional string array data: got {decoded_json}, expected {expected_json}"));
}

let absent: TestRoot = serde_json::from_value(serde_json::json!({
    "kind": "Widget"
}))
.map_err(|error| format!("generated TestRoot absent deserialization failed: {error}"))?;
let mut absent_tokens = Vec::new();
absent.encode(&mut absent_tokens);
let mut absent_iter = absent_tokens.into_iter();
let absent_prefix = TestRoot::decode_prefix(&mut absent_iter);
if !absent_prefix.complete || absent_prefix.partial_field.is_some() || absent_iter.next().is_some() {
    return Err(format!("decode_prefix did not complete cleanly for absent optional string array: {:?}", absent_prefix.partial_field));
}
let absent_json = serde_json::to_value(&absent_prefix.instance)
    .map_err(|error| format!("generated TestRoot absent serialization failed: {error}"))?;
let expected_absent_json = serde_json::json!({
    "kind": "Widget"
});
if absent_json != expected_absent_json {
    return Err(format!("decode_prefix changed absent optional string array: got {absent_json}, expected {expected_absent_json}"));
}

let mut truncated = tokens.clone();
let removed = truncated
    .pop()
    .ok_or_else(|| "expected encoded optional string array to contain tokens".to_string())?;
if removed != 256u32 {
    return Err(format!("expected to remove the second string end marker, got {removed}"));
}
let mut truncated_iter = truncated.into_iter();
let partial = TestRoot::decode_prefix(&mut truncated_iter);
if partial.complete {
    return Err("decode_prefix treated a truncated optional string array element as complete".to_string());
}
let partial_field = partial
    .partial_field
    .as_ref()
    .ok_or_else(|| "truncated optional string array should report partial field metadata".to_string())?;
if partial_field.field_name != "names" || partial_field.field_type != "Vec" {
    return Err(format!("unexpected partial field metadata: {:?}", partial_field));
}
if partial_field.items_decoded != 2 || partial_field.expected_length != Some(2) {
    return Err(format!("unexpected optional string array progress metadata: {:?}", partial_field));
}

let partial_json = serde_json::to_value(&partial.instance)
    .map_err(|error| format!("partial TestRoot serialization failed: {error}"))?;
let expected_partial_json = serde_json::json!({
    "kind": "Widget",
    "names": ["alpha", "beta"]
});
if partial_json != expected_partial_json {
    return Err(format!("decode_prefix dropped the partial optional string array element: got {partial_json}, expected {expected_partial_json}"));
}

let optional_original: TestRoot = serde_json::from_value(serde_json::json!({
    "kind": "Widget",
    "names": ["alpha"],
    "optional_0": "ready",
    "optional_1": "part"
}))
.map_err(|error| format!("generated TestRoot optional field deserialization failed: {error}"))?;
let mut optional_tokens = Vec::new();
optional_original.encode(&mut optional_tokens);
let removed_optional = optional_tokens
    .pop()
    .ok_or_else(|| "expected encoded optional string field to contain tokens".to_string())?;
if removed_optional != 256u32 {
    return Err(format!("expected to remove the optional string end marker, got {removed_optional}"));
}
let mut optional_iter = optional_tokens.into_iter();
let partial_optional = TestRoot::decode_prefix(&mut optional_iter);
if partial_optional.complete {
    return Err("decode_prefix treated a truncated later optional string as complete".to_string());
}
let optional_partial_field = partial_optional
    .partial_field
    .as_ref()
    .ok_or_else(|| "truncated later optional string should report partial field metadata".to_string())?;
if optional_partial_field.field_name != "optional_1"
    || optional_partial_field.field_type != "String"
    || optional_partial_field.items_decoded != 4
    || optional_partial_field.expected_length.is_some()
{
    return Err(format!("unexpected later optional string progress metadata: {:?}", optional_partial_field));
}
let partial_optional_json = serde_json::to_value(&partial_optional.instance)
    .map_err(|error| format!("partial optional TestRoot serialization failed: {error}"))?;
let expected_partial_optional_json = serde_json::json!({
    "kind": "Widget",
    "names": ["alpha"],
    "optional_0": "ready",
    "optional_1": "part"
});
if partial_optional_json != expected_partial_optional_json {
    return Err(format!("decode_prefix lost optional field state around truncated field: got {partial_optional_json}, expected {expected_partial_optional_json}"));
}

Ok(())
"#,
    );
}

#[test]
fn test_optional_array_of_nested_structs_decode_prefix_runtime_contract() {
    let schema = r#"{
        "type": "object",
        "additionalProperties": false,
        "properties": {
            "name": {"type": "string"},
            "subjects": {
                "type": "array",
                "items": {
                    "type": "object",
                    "additionalProperties": false,
                    "properties": {
                        "name": {"type": "string"}
                    },
                    "required": ["name"]
                }
            },
            "optional_0": {"type": "string"},
            "optional_1": {"type": "string"},
            "optional_2": {"type": "string"}
        },
        "required": ["name"]
    }"#;

    let code = build_and_test_schema(schema, "optional_array_nested_runtime")
        .expect("Should generate code");
    run_generated_streaming_contract(
        "optional-array-nested-decode-prefix",
        &code,
        r#"
let original: TestRoot = serde_json::from_value(serde_json::json!({
    "name": "root",
    "subjects": [
        {"name": "math"},
        {"name": "science"}
    ]
}))
.map_err(|error| format!("generated TestRoot deserialization failed: {error}"))?;

let mut tokens = Vec::new();
original.encode(&mut tokens);

let mut full_iter = tokens.clone().into_iter();
let prefix = TestRoot::decode_prefix(&mut full_iter);
if !prefix.complete {
    return Err(format!("decode_prefix reported a complete optional nested array as partial: {:?}", prefix.partial_field));
}
if prefix.partial_field.is_some() {
    return Err(format!("complete decode_prefix should not report a partial field: {:?}", prefix.partial_field));
}
if full_iter.next().is_some() {
    return Err("decode_prefix left surplus tokens after a complete optional nested array".to_string());
}

let decoded_json = serde_json::to_value(&prefix.instance)
    .map_err(|error| format!("generated TestRoot serialization failed: {error}"))?;
let expected_json = serde_json::json!({
    "name": "root",
    "subjects": [
        {"name": "math"},
        {"name": "science"}
    ]
});
if decoded_json != expected_json {
    return Err(format!("decode_prefix changed optional nested array data: got {decoded_json}, expected {expected_json}"));
}

let mut truncated = tokens.clone();
let removed = truncated
    .pop()
    .ok_or_else(|| "expected encoded optional nested array to contain tokens".to_string())?;
if removed != 256u32 {
    return Err(format!("expected to remove the second subject name end marker, got {removed}"));
}
let mut truncated_iter = truncated.into_iter();
let partial = TestRoot::decode_prefix(&mut truncated_iter);
if partial.complete {
    return Err("decode_prefix treated a truncated optional nested array element as complete".to_string());
}
let partial_field = partial
    .partial_field
    .as_ref()
    .ok_or_else(|| "truncated optional nested array should report partial field metadata".to_string())?;
if partial_field.field_name != "subjects" || partial_field.field_type != "Vec" {
    return Err(format!("unexpected partial field metadata: {:?}", partial_field));
}
if partial_field.items_decoded != 2 || partial_field.expected_length != Some(2) {
    return Err(format!("unexpected optional nested array progress metadata: {:?}", partial_field));
}

Ok(())
"#,
    );
}

#[test]
fn test_optional_array_of_nested_structs_standard_decode_prefix_runtime_contract() {
    let schema = r#"{
        "type": "object",
        "additionalProperties": false,
        "properties": {
            "name": {"type": "string"},
            "subjects": {
                "type": "array",
                "items": {
                    "type": "object",
                    "additionalProperties": false,
                    "properties": {
                        "name": {"type": "string"}
                    },
                    "required": ["name"]
                }
            }
        },
        "required": ["name"]
    }"#;

    let code = build_and_test_schema_with_optimization(
        schema,
        "optional_array_nested_standard_runtime",
        tct::OptimizationLevel::O2,
    )
    .expect("Should generate code");
    run_generated_streaming_contract(
        "optional-array-nested-standard-decode-prefix",
        &code,
        r#"
let original: TestRoot = serde_json::from_value(serde_json::json!({
    "name": "root",
    "subjects": [
        {"name": "math"},
        {"name": "science"}
    ]
}))
.map_err(|error| format!("generated TestRoot deserialization failed: {error}"))?;

let mut tokens = Vec::new();
original.encode(&mut tokens);

let mut full_iter = tokens.clone().into_iter();
let prefix = TestRoot::decode_prefix(&mut full_iter);
if !prefix.complete {
    return Err(format!("decode_prefix reported a complete standard optional nested array as partial: {:?}", prefix.partial_field));
}
if prefix.fields_decoded != 2 {
    return Err(format!("expected two root fields decoded, got {}", prefix.fields_decoded));
}
if prefix.partial_field.is_some() {
    return Err(format!("complete decode_prefix should not report a partial field: {:?}", prefix.partial_field));
}
if full_iter.next().is_some() {
    return Err("decode_prefix left surplus tokens after a complete standard optional nested array".to_string());
}

let decoded_json = serde_json::to_value(&prefix.instance)
    .map_err(|error| format!("generated TestRoot serialization failed: {error}"))?;
let expected_json = serde_json::json!({
    "name": "root",
    "subjects": [
        {"name": "math"},
        {"name": "science"}
    ]
});
if decoded_json != expected_json {
    return Err(format!("decode_prefix changed standard optional nested array data: got {decoded_json}, expected {expected_json}"));
}

let absent: TestRoot = serde_json::from_value(serde_json::json!({
    "name": "root"
}))
.map_err(|error| format!("generated TestRoot absent deserialization failed: {error}"))?;
let mut absent_tokens = Vec::new();
absent.encode(&mut absent_tokens);
let mut absent_iter = absent_tokens.into_iter();
let absent_prefix = TestRoot::decode_prefix(&mut absent_iter);
if !absent_prefix.complete || absent_prefix.partial_field.is_some() || absent_iter.next().is_some() {
    return Err(format!("decode_prefix did not complete cleanly for absent standard optional nested array: {:?}", absent_prefix.partial_field));
}
let absent_json = serde_json::to_value(&absent_prefix.instance)
    .map_err(|error| format!("generated TestRoot absent serialization failed: {error}"))?;
let expected_absent_json = serde_json::json!({
    "name": "root"
});
if absent_json != expected_absent_json {
    return Err(format!("decode_prefix changed absent standard optional nested array: got {absent_json}, expected {expected_absent_json}"));
}

let mut truncated = tokens.clone();
let removed = truncated
    .pop()
    .ok_or_else(|| "expected encoded standard optional nested array to contain tokens".to_string())?;
if removed != 256u32 {
    return Err(format!("expected to remove the second subject name end marker, got {removed}"));
}
let mut truncated_iter = truncated.into_iter();
let partial = TestRoot::decode_prefix(&mut truncated_iter);
if partial.complete {
    return Err("decode_prefix treated a truncated standard optional nested array element as complete".to_string());
}
let partial_field = partial
    .partial_field
    .as_ref()
    .ok_or_else(|| "truncated standard optional nested array should report partial field metadata".to_string())?;
if partial_field.field_name != "subjects" || partial_field.field_type != "Vec" {
    return Err(format!("unexpected partial field metadata: {:?}", partial_field));
}
if partial_field.items_decoded != 2 || partial_field.expected_length != Some(2) {
    return Err(format!("unexpected standard optional nested array progress metadata: {:?}", partial_field));
}

Ok(())
"#,
    );
}

#[test]
fn test_nullable_array_of_nested_structs_standard_decode_prefix_runtime_contract() {
    let schema = r#"{
        "type": "object",
        "additionalProperties": false,
        "properties": {
            "name": {"type": "string"},
            "subjects": {
                "anyOf": [
                    {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "additionalProperties": false,
                            "properties": {
                                "name": {"type": "string"}
                            },
                            "required": ["name"]
                        }
                    },
                    {"type": "null"}
                ]
            }
        },
        "required": ["name"]
    }"#;

    let code = build_and_test_schema_with_optimization(
        schema,
        "nullable_array_nested_standard_runtime",
        tct::OptimizationLevel::O2,
    )
    .expect("Should generate code");
    run_generated_streaming_contract(
        "nullable-array-nested-standard-decode-prefix",
        &code,
        r#"
let original: TestRoot = serde_json::from_value(serde_json::json!({
    "name": "root",
    "subjects": [
        {"name": "math"},
        {"name": "science"}
    ]
}))
.map_err(|error| format!("generated TestRoot deserialization failed: {error}"))?;

let mut tokens = Vec::new();
original.encode(&mut tokens);

let mut full_iter = tokens.clone().into_iter();
let prefix = TestRoot::decode_prefix(&mut full_iter);
if !prefix.complete {
    return Err(format!("decode_prefix reported a complete standard nullable nested array as partial: {:?}", prefix.partial_field));
}
if prefix.fields_decoded != 2 {
    return Err(format!("expected two root fields decoded, got {}", prefix.fields_decoded));
}
if prefix.partial_field.is_some() {
    return Err(format!("complete decode_prefix should not report a partial field: {:?}", prefix.partial_field));
}
if full_iter.next().is_some() {
    return Err("decode_prefix left surplus tokens after a complete standard nullable nested array".to_string());
}

let decoded_json = serde_json::to_value(&prefix.instance)
    .map_err(|error| format!("generated TestRoot serialization failed: {error}"))?;
let expected_json = serde_json::json!({
    "name": "root",
    "subjects": [
        {"name": "math"},
        {"name": "science"}
    ]
});
if decoded_json != expected_json {
    return Err(format!("decode_prefix changed standard nullable nested array data: got {decoded_json}, expected {expected_json}"));
}

let explicit_null: TestRoot = serde_json::from_value(serde_json::json!({
    "name": "root",
    "subjects": null
}))
.map_err(|error| format!("generated TestRoot null deserialization failed: {error}"))?;
let mut null_tokens = Vec::new();
explicit_null.encode(&mut null_tokens);
let mut null_iter = null_tokens.into_iter();
let null_prefix = TestRoot::decode_prefix(&mut null_iter);
if !null_prefix.complete || null_prefix.partial_field.is_some() || null_iter.next().is_some() {
    return Err(format!("decode_prefix did not complete cleanly for explicit null standard nullable nested array: {:?}", null_prefix.partial_field));
}
let null_json = serde_json::to_value(&null_prefix.instance)
    .map_err(|error| format!("generated TestRoot null serialization failed: {error}"))?;
let expected_null_json = serde_json::json!({
    "name": "root",
    "subjects": null
});
if null_json != expected_null_json {
    return Err(format!("decode_prefix changed explicit null standard nullable nested array: got {null_json}, expected {expected_null_json}"));
}

let absent: TestRoot = serde_json::from_value(serde_json::json!({
    "name": "root"
}))
.map_err(|error| format!("generated TestRoot absent deserialization failed: {error}"))?;
let mut absent_tokens = Vec::new();
absent.encode(&mut absent_tokens);
let mut absent_iter = absent_tokens.into_iter();
let absent_prefix = TestRoot::decode_prefix(&mut absent_iter);
if !absent_prefix.complete || absent_prefix.partial_field.is_some() || absent_iter.next().is_some() {
    return Err(format!("decode_prefix did not complete cleanly for absent standard nullable nested array: {:?}", absent_prefix.partial_field));
}
let absent_json = serde_json::to_value(&absent_prefix.instance)
    .map_err(|error| format!("generated TestRoot absent serialization failed: {error}"))?;
let expected_absent_json = serde_json::json!({
    "name": "root"
});
if absent_json != expected_absent_json {
    return Err(format!("decode_prefix changed absent standard nullable nested array: got {absent_json}, expected {expected_absent_json}"));
}

let mut truncated = tokens.clone();
let removed = truncated
    .pop()
    .ok_or_else(|| "expected encoded standard nullable nested array to contain tokens".to_string())?;
if removed != 256u32 {
    return Err(format!("expected to remove the second subject name end marker, got {removed}"));
}
let mut truncated_iter = truncated.into_iter();
let partial = TestRoot::decode_prefix(&mut truncated_iter);
if partial.complete {
    return Err("decode_prefix treated a truncated standard nullable nested array element as complete".to_string());
}
let partial_field = partial
    .partial_field
    .as_ref()
    .ok_or_else(|| "truncated standard nullable nested array should report partial field metadata".to_string())?;
if partial_field.field_name != "subjects" || partial_field.field_type != "Vec" {
    return Err(format!("unexpected partial field metadata: {:?}", partial_field));
}
if partial_field.items_decoded != 2 || partial_field.expected_length != Some(2) {
    return Err(format!("unexpected standard nullable nested array progress metadata: {:?}", partial_field));
}

Ok(())
"#,
    );
}

#[test]
fn test_enum_with_complex_variants() {
    // Test enum with different variant types
    let schema = r#"{
        "oneOf": [
            {"type": "string"},
            {"type": "integer"},
            {"type": "object", "properties": {"nested": {"type": "boolean"}}}
        ]
    }"#;

    let code = build_and_test_schema(schema, "complex_enum").expect("Should generate code");
    let generated_file = parse_generated_file(&code);

    // Should have decode_prefix with variant handling
    assert!(
        generated_method_count(&generated_file, "decode_prefix") > 0,
        "Should generate decode_prefix for complex enum"
    );

    let test_root_decode_prefix =
        generated_inherent_method(&generated_file, "TestRoot", "decode_prefix");

    // Multi-variant enum should read variant_index from tokens
    assert!(
        method_body_tokens_contain(test_root_decode_prefix, "variant_index")
            && method_body_tokens_contain(test_root_decode_prefix, "tokens . next"),
        "Multi-variant enum should consume discriminator token and use variant_index"
    );
}
