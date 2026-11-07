//! Test smart additionalProperties optimization
//!
//! Verify that the analyzer correctly detects type constraints and generates
//! optimized HashMap types instead of falling back to serde_json::Value.

use quote::ToTokens;
use serde_json::json;
use tct::{OptimizationLevel, TctCompiler};

fn parse_generated_code(code: &str) -> syn::File {
    syn::parse_file(code).unwrap_or_else(|error| panic!("generated Rust should parse: {error}"))
}

fn generated_struct_field_type_tokens(
    file: &syn::File,
    struct_name: &str,
    field_name: &str,
) -> Option<String> {
    file.items.iter().find_map(|item| match item {
        syn::Item::Struct(item_struct) if item_struct.ident == struct_name => {
            match &item_struct.fields {
                syn::Fields::Named(fields) => fields.named.iter().find_map(|field| {
                    field
                        .ident
                        .as_ref()
                        .filter(|ident| **ident == field_name)
                        .map(|_| field.ty.to_token_stream().to_string())
                }),
                _ => None,
            }
        }
        _ => None,
    })
}

fn generated_struct_field_type_contains(
    code: &str,
    struct_name: &str,
    field_name: &str,
    expected: &str,
) -> bool {
    let file = parse_generated_code(code);
    generated_struct_field_type_tokens(&file, struct_name, field_name)
        .is_some_and(|field_type| field_type.contains(expected))
}

#[test]
fn test_unconstrained_additional_properties_stays_value_at_o3() {
    let schema = json!({
        "type": "object",
        "properties": {
            "name": {"type": "string"}
        },
        "additionalProperties": true
    });

    let grammar = TctCompiler::new(schema, "ConfigSchema")
        .and_then(|compiler| {
            compiler
                .with_optimization_level(OptimizationLevel::O3)
                .compile()
        })
        .expect("Schema compilation should succeed");

    let generated_code = grammar
        .generate_code()
        .expect("Code generation should succeed");
    let code_str = generated_code.to_string();

    assert!(
        generated_struct_field_type_contains(
            &code_str,
            "ConfigSchema",
            "additional_properties",
            "rustc_hash :: FxHashMap < String , serde_json :: Value >"
        ),
        "O3 must not narrow unconstrained additionalProperties: true: {}",
        code_str
    );

    let instance = json!({
        "name": "demo",
        "extra_string": "ok",
        "extra_bool": true,
        "extra_number": 42
    });
    jsonschema::validate(
        &json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"}
            },
            "additionalProperties": true
        }),
        &instance,
    )
    .expect("Fixture should be valid under unconstrained additionalProperties");

    println!("✅ Unconstrained additionalProperties stays serde_json::Value at O3");
}

#[test]
fn test_missing_additional_properties_stays_value_at_o3() {
    let schema = json!({
        "type": "object",
        "properties": {
            "name": {"type": "string"}
        }
    });

    let grammar = TctCompiler::new(schema, "ImplicitAdditionalPropertiesSchema")
        .and_then(|compiler| {
            compiler
                .with_optimization_level(OptimizationLevel::O3)
                .compile()
        })
        .expect("Schema compilation should succeed");

    let generated_code = grammar
        .generate_code()
        .expect("Code generation should succeed");
    let code_str = generated_code.to_string();

    assert!(
        generated_struct_field_type_contains(
            &code_str,
            "ImplicitAdditionalPropertiesSchema",
            "additional_properties",
            "rustc_hash :: FxHashMap < String , serde_json :: Value >"
        ),
        "O3 must not narrow implicit additionalProperties: true: {}",
        code_str
    );

    println!("✅ Implicit additionalProperties stays serde_json::Value at O3");
}

#[test]
fn test_explicit_type_constraint() {
    let schema = json!({
        "type": "object",
        "properties": {
            "name": {"type": "string"}
        },
        "additionalProperties": {
            "type": "string",
            "pattern": "^[a-zA-Z]+$"
        }
    });

    let grammar = TctCompiler::new(schema, "ExplicitStringSchema")
        .and_then(|compiler| compiler.compile())
        .expect("Schema compilation should succeed");

    let generated_code = grammar
        .generate_code()
        .expect("Code generation should succeed");
    let code_str = generated_code.to_string();

    // Should use the explicit string constraint - TCT uses FxHashMap
    assert!(
        generated_struct_field_type_contains(
            &code_str,
            "ExplicitStringSchema",
            "additional_properties",
            "rustc_hash :: FxHashMap < String , String >"
        ),
        "Should respect explicit string constraint: {}",
        code_str
    );

    println!("✅ Explicit type constraint respected");
}

#[test]
fn test_package_json_context() {
    let schema = json!({
        "type": "object",
        "properties": {
            "name": {"type": "string"},
            "version": {"type": "string"},
            "dependencies": {
                "type": "object",
                "additionalProperties": true
            }
        }
    });

    let grammar = TctCompiler::new(schema, "PackageJsonSchema")
        .and_then(|compiler| compiler.compile())
        .expect("Schema compilation should succeed");

    let generated_code = grammar
        .generate_code()
        .expect("Code generation should succeed");
    let code_str = generated_code.to_string();

    // For dependencies in package.json, should optimize to strings
    // Note: This tests nested schema optimization
    println!(
        "Generated code for package.json dependencies:\n{}",
        code_str
    );

    println!("✅ Package.json dependencies schema compiled successfully");
}

#[test]
fn test_arbitrary_fallback() {
    let schema = json!({
        "type": "object",
        "properties": {
            "mixed1": {"type": "string"},
            "mixed2": {"type": "number"},
            "mixed3": {"type": "boolean"},
            "mixed4": {"type": "array"}
        },
        "additionalProperties": true
    });

    let grammar = TctCompiler::new(schema, "MixedTypesSchema")
        .and_then(|compiler| compiler.compile())
        .expect("Schema compilation should succeed");

    let generated_code = grammar
        .generate_code()
        .expect("Code generation should succeed");
    let code_str = generated_code.to_string();

    // With highly mixed types, should fall back to generic Value
    // TCT uses FxHashMap and serde_json::Value for arbitrary types
    assert!(
        generated_struct_field_type_contains(
            &code_str,
            "MixedTypesSchema",
            "additional_properties",
            "rustc_hash :: FxHashMap < String , serde_json :: Value >"
        ),
        "Should fall back to generic Value for mixed types: {}",
        code_str
    );

    println!("✅ Mixed types schema correctly falls back to generic Value");
}

#[test]
fn test_boolean_additional_properties_without_constraint_stays_value() {
    let schema = json!({
        "title": "Feature Flags",
        "type": "object",
        "properties": {
            "feature_a": {"type": "boolean"},
            "feature_b": {"type": "boolean"}
        },
        "additionalProperties": true
    });

    let grammar = TctCompiler::new(schema, "FlagsSchema")
        .and_then(|compiler| {
            compiler
                .with_optimization_level(OptimizationLevel::O3)
                .compile()
        })
        .expect("Schema compilation should succeed");

    let generated_code = grammar
        .generate_code()
        .expect("Code generation should succeed");
    let code_str = generated_code.to_string();

    assert!(
        generated_struct_field_type_contains(
            &code_str,
            "FlagsSchema",
            "additional_properties",
            "rustc_hash :: FxHashMap < String , serde_json :: Value >"
        ),
        "Schema hints must not narrow unconstrained additionalProperties: true: {}",
        code_str
    );

    println!("✅ Schema hints do not narrow unconstrained additionalProperties");
}
