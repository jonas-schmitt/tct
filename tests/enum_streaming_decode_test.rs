//! Integration tests for enum streaming decode (decode_prefix)
//!
//! Tests that the generated code for enum streaming decode compiles correctly
//! and handles Container (Vec<primitive>) and ComplexContainer (Vec<struct>) variants.

use quote::ToTokens;
use serde_json::json;
use std::process::Command;
use tct::TctCompiler;
use tct::generated_crate::{
    GeneratedCargoToml, configure_generated_cargo_command, copy_workspace_cargo_lock,
};

fn parse_generated_code(code: &str) -> syn::File {
    syn::parse_file(code).unwrap_or_else(|error| panic!("generated Rust should parse: {error}"))
}

fn type_ends_with_ident(ty: &syn::Type, name: &str) -> bool {
    matches!(
        ty,
        syn::Type::Path(type_path)
            if type_path
                .path
                .segments
                .last()
                .is_some_and(|segment| segment.ident == name)
    )
}

fn generated_impl_method<'a>(
    file: &'a syn::File,
    type_name: &str,
    method_name: &str,
) -> Option<&'a syn::ImplItemFn> {
    file.items.iter().find_map(|item| match item {
        syn::Item::Impl(item_impl) if type_ends_with_ident(&item_impl.self_ty, type_name) => {
            item_impl
                .items
                .iter()
                .find_map(|impl_item| match impl_item {
                    syn::ImplItem::Fn(function) if function.sig.ident == method_name => {
                        Some(function)
                    }
                    _ => None,
                })
        }
        _ => None,
    })
}

fn generated_file_has_impl_method(file: &syn::File, type_name: &str, method_name: &str) -> bool {
    generated_impl_method(file, type_name, method_name).is_some()
}

fn generated_impl_method_token_occurrences(
    file: &syn::File,
    type_name: &str,
    method_name: &str,
    expected: &str,
) -> usize {
    generated_impl_method(file, type_name, method_name)
        .unwrap_or_else(|| panic!("generated `{type_name}::{method_name}` method should exist"))
        .to_token_stream()
        .to_string()
        .matches(expected)
        .count()
}

fn generated_streaming_cargo_command() -> Command {
    let tct_path = std::env::current_dir().expect("failed to resolve repo root");
    let mut command = Command::new("cargo");
    configure_generated_cargo_command(&mut command, &tct_path);
    command
}

/// Test that streaming decode_prefix is generated for enums with Container variants
#[test]
fn test_enum_with_container_variant_compiles_with_streaming() {
    // Schema: oneOf with string and array of strings
    let schema = json!({
        "title": "ContainerEnum",
        "oneOf": [
            {"type": "string"},
            {"type": "array", "items": {"type": "string"}}
        ]
    });

    let compiler = TctCompiler::new(schema, "ContainerEnum")
        .expect("Should create compiler")
        .with_streaming_encode();

    let grammar = compiler.compile().expect("Compilation should succeed");
    let code = grammar
        .generate_code()
        .expect("Code generation should succeed");
    let generated_file = parse_generated_code(&code);

    // Verify decode_prefix is generated
    assert!(
        generated_file_has_impl_method(&generated_file, "ContainerEnum", "decode_prefix"),
        "Should generate decode_prefix function for streaming"
    );

    // Verify PrefixDecodeResult is used
    assert!(
        code.contains("PrefixDecodeResult"),
        "Should use PrefixDecodeResult type"
    );

    // Verify decode_vec_streaming is used for Container variant
    assert!(
        code.contains("decode_vec_streaming"),
        "Should use decode_vec_streaming for Vec<String> variant"
    );
}

/// Test that streaming decode_prefix is generated for enums with ComplexContainer variants
#[test]
fn test_enum_with_complex_container_variant_compiles_with_streaming() {
    // Schema: oneOf with object and array of objects
    let schema = json!({
        "title": "ComplexContainerEnum",
        "oneOf": [
            {
                "type": "object",
                "properties": {
                    "name": {"type": "string"}
                },
                "required": ["name"]
            },
            {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "id": {"type": "integer"}
                    },
                    "required": ["id"]
                }
            }
        ]
    });

    let compiler = TctCompiler::new(schema, "ComplexContainerEnum")
        .expect("Should create compiler")
        .with_streaming_encode();

    let grammar = compiler.compile().expect("Compilation should succeed");
    let code = grammar
        .generate_code()
        .expect("Code generation should succeed");
    let generated_file = parse_generated_code(&code);

    // Verify decode_prefix is generated
    assert!(
        generated_file_has_impl_method(&generated_file, "ComplexContainerEnum", "decode_prefix"),
        "Should generate decode_prefix function for streaming"
    );

    // The ComplexContainer variant should use per-element decode_prefix
    // This means we should see decode_prefix called on the inner type
    assert!(
        generated_impl_method_token_occurrences(
            &generated_file,
            "ComplexContainerEnum",
            "decode_prefix",
            "decode_prefix"
        ) >= 3,
        "Should use decode_prefix for Vec<struct> elements"
    );
}

/// Test streaming disabled - should not generate decode_prefix
#[test]
fn test_enum_streaming_disabled_no_decode_prefix() {
    let schema = json!({
        "title": "SimpleEnum",
        "oneOf": [
            {"type": "string"},
            {"type": "integer"}
        ]
    });

    // Streaming disabled (default - don't call with_streaming_encode)
    let compiler = TctCompiler::new(schema, "SimpleEnum").expect("Should create compiler");

    let grammar = compiler.compile().expect("Compilation should succeed");
    let code = grammar
        .generate_code()
        .expect("Code generation should succeed");
    let generated_file = parse_generated_code(&code);

    // Should NOT have decode_prefix when streaming is disabled
    assert!(
        !generated_file_has_impl_method(&generated_file, "SimpleEnum", "decode_prefix"),
        "Should NOT generate decode_prefix when streaming is disabled"
    );
}

/// Test mixed enum with multiple variant types
#[test]
fn test_mixed_enum_all_variant_types_streaming() {
    // Schema with primitive, container, and reference variants
    let schema = json!({
        "title": "MixedEnum",
        "definitions": {
            "InnerStruct": {
                "type": "object",
                "properties": {
                    "value": {"type": "string"}
                },
                "required": ["value"]
            }
        },
        "oneOf": [
            {"type": "string"},
            {"type": "array", "items": {"type": "integer"}},
            {"$ref": "#/definitions/InnerStruct"}
        ]
    });

    let compiler = TctCompiler::new(schema, "MixedEnum")
        .expect("Should create compiler")
        .with_streaming_encode();

    let grammar = compiler.compile().expect("Compilation should succeed");
    let code = grammar
        .generate_code()
        .expect("Code generation should succeed");
    let generated_file = parse_generated_code(&code);

    // Verify decode_prefix is generated
    assert!(
        generated_file_has_impl_method(&generated_file, "MixedEnum", "decode_prefix"),
        "Should generate decode_prefix function"
    );

    // Verify different decode strategies are used
    assert!(
        code.contains("decode_vec_streaming"),
        "Should use decode_vec_streaming for Vec<i64> variant"
    );
}

/// Test that generated code with streaming enum can be compiled by rustc
#[test]
#[ignore] // Run manually: cargo test --test enum_streaming_decode_test test_streaming_enum_code_compiles -- --ignored --nocapture
fn test_streaming_enum_code_compiles() {
    use std::fs;

    let schema = json!({
        "title": "StreamingTestEnum",
        "oneOf": [
            {"type": "string"},
            {"type": "array", "items": {"type": "string"}},
            {"type": "array", "items": {"type": "integer"}}
        ]
    });

    let compiler = TctCompiler::new(schema, "StreamingTestEnum")
        .expect("Should create compiler")
        .with_streaming_encode();

    let grammar = compiler.compile().expect("Compilation should succeed");
    let code = grammar
        .generate_code()
        .expect("Code generation should succeed");

    // Write to temp file
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let lib_path = temp_dir.path().join("lib.rs");
    fs::write(&lib_path, &code).expect("Failed to write generated code");

    // Create Cargo.toml
    let cargo_toml = GeneratedCargoToml::new("streaming-enum-test")
        .with_chrono()
        .with_base64()
        .with_regex()
        .with_lazy_static()
        .with_rustc_hash()
        .with_jsonschema()
        .render();
    fs::write(temp_dir.path().join("Cargo.toml"), cargo_toml).expect("Failed to write Cargo.toml");
    let tct_path = std::env::current_dir().expect("failed to resolve repo root");
    copy_workspace_cargo_lock(&tct_path, temp_dir.path())
        .expect("failed to copy workspace Cargo.lock into generated streaming enum crate");

    // Create src directory and move lib.rs
    let src_dir = temp_dir.path().join("src");
    fs::create_dir(&src_dir).expect("Failed to create src dir");
    fs::rename(&lib_path, src_dir.join("lib.rs")).expect("Failed to move lib.rs");

    // Try to compile
    let output = generated_streaming_cargo_command()
        .args(["check", "--lib"])
        .current_dir(temp_dir.path())
        .output()
        .expect("Failed to execute cargo check");

    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "Generated code should compile successfully. Errors:\n{}",
        stderr
    );

    eprintln!("✅ Streaming enum code compiles successfully!");
}
