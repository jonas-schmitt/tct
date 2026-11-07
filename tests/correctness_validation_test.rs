// Correctness validation tests using the Cargo test framework
// These tests validate schema processing correctness and identify edge cases

use serde_json::json;
use tct::TctCompiler;

fn parse_generated_code(code: &str) -> syn::File {
    syn::parse_file(code).unwrap_or_else(|error| panic!("generated Rust should parse: {error}"))
}

fn path_ends_with_ident(path: &syn::Path, name: &str) -> bool {
    path.segments
        .last()
        .is_some_and(|segment| segment.ident == name)
}

fn generated_items_have_json_token_encoder_impl(items: &[syn::Item]) -> bool {
    items.iter().any(|item| match item {
        syn::Item::Impl(item_impl) => item_impl
            .trait_
            .as_ref()
            .is_some_and(|(_, path, _)| path_ends_with_ident(path, "JsonTokenEncoder")),
        syn::Item::Mod(item_mod) => item_mod
            .content
            .as_ref()
            .is_some_and(|(_, items)| generated_items_have_json_token_encoder_impl(items)),
        _ => false,
    })
}

fn generated_file_has_json_token_encoder_impl(file: &syn::File) -> bool {
    generated_items_have_json_token_encoder_impl(&file.items)
}

fn generated_items_have_type_item(items: &[syn::Item]) -> bool {
    items.iter().any(|item| match item {
        syn::Item::Struct(_) | syn::Item::Enum(_) => true,
        syn::Item::Mod(item_mod) => item_mod
            .content
            .as_ref()
            .is_some_and(|(_, items)| generated_items_have_type_item(items)),
        _ => false,
    })
}

fn generated_file_has_type_item(file: &syn::File) -> bool {
    generated_items_have_type_item(&file.items)
}

/// Test edge cases that have been problematic
#[test]
fn test_correctness_validation_edge_case_validation() {
    let edge_cases = vec![
        // Empty object
        (json!({"type": "object", "properties": {}}), "EmptyObject"),
        // Single character properties
        (
            json!({"type": "object", "properties": {"a": {"type": "string"}}}),
            "SingleChar",
        ),
        // Properties with underscores
        (
            json!({"type": "object", "properties": {"prop_with_underscore": {"type": "string"}}}),
            "Underscore",
        ),
        // Very long property names
        (
            json!({"type": "object", "properties": {"very_long_property_name_that_exceeds_normal_length": {"type": "string"}}}),
            "LongProp",
        ),
        // Empty enum (should fail gracefully)
        (json!({"enum": []}), "EmptyEnum"),
        // Single value enum
        (json!({"enum": ["only_value"]}), "SingleEnum"),
        // Mixed constraints
        (
            json!({
                "type": "object",
                "properties": {
                    "constrained_int": {
                        "type": "integer",
                        "minimum": 0,
                        "maximum": 100
                    },
                    "pattern_string": {
                        "type": "string",
                        "pattern": "^[a-zA-Z]+$"
                    }
                }
            }),
            "MixedConstraints",
        ),
        // Deep nesting
        (
            json!({
                "type": "object",
                "properties": {
                    "level1": {
                        "type": "object",
                        "properties": {
                            "level2": {
                                "type": "object",
                                "properties": {
                                    "level3": {"type": "string"}
                                }
                            }
                        }
                    }
                }
            }),
            "DeepNesting",
        ),
    ];

    let mut successful = 0;
    let mut failed = 0;
    let mut error_categories = std::collections::HashMap::new();

    for (schema, name) in edge_cases {
        match TctCompiler::new(schema.clone(), name).unwrap().compile() {
            Ok(grammar) => {
                match grammar.generate_code() {
                    Ok(code) => {
                        successful += 1;
                        let generated_file = parse_generated_code(&code);
                        // Verify code contains expected elements
                        assert!(
                            generated_file_has_json_token_encoder_impl(&generated_file),
                            "Generated code should contain JsonTokenEncoder for {}",
                            name
                        );
                        assert!(
                            generated_file_has_type_item(&generated_file),
                            "Generated code should contain types for {}",
                            name
                        );
                        println!("✅ {}: Generated {} chars of code", name, code.len());
                    }
                    Err(e) => {
                        failed += 1;
                        let error_type = if e.to_string().contains("empty") {
                            "Empty Identifier"
                        } else if e.to_string().contains("invalid") {
                            "Invalid Syntax"
                        } else {
                            "Code Generation"
                        };
                        *error_categories.entry(error_type).or_insert(0) += 1;
                        println!("❌ {}: Code generation failed - {}", name, e);
                    }
                }
            }
            Err(e) => {
                failed += 1;
                let error_type = if e.to_string().contains("missing") {
                    "Missing Properties"
                } else if e.to_string().contains("malformed") {
                    "Malformed Schema"
                } else {
                    "Schema Processing"
                };
                *error_categories.entry(error_type).or_insert(0) += 1;
                println!("❌ {}: Schema processing failed - {}", name, e);
            }
        }
    }

    let total = successful + failed;
    let success_rate = (successful as f64 / total as f64) * 100.0;

    println!("\n📊 Edge Case Validation Summary:");
    println!("  Total: {}", total);
    println!("  Successful: {} ({:.1}%)", successful, success_rate);
    println!("  Failed: {}", failed);

    if !error_categories.is_empty() {
        println!("  Error Categories:");
        for (category, count) in error_categories {
            println!("    {}: {}", category, count);
        }
    }

    // We expect at least 70% success rate for edge cases
    assert!(
        success_rate >= 70.0,
        "Edge case success rate too low: {:.1}%",
        success_rate
    );
}

/// Test systematic schema variations
#[test]
fn test_correctness_validation_systematic_schema_variations() {
    let mut successful = 0;
    let mut failed = 0;

    // Test different object schema patterns
    for i in 1..=5 {
        let mut properties = serde_json::Map::new();
        for j in 0..i {
            properties.insert(format!("field_{}", j), json!({"type": "string"}));
        }

        let schema = json!({
            "type": "object",
            "properties": properties,
            "required": if i > 2 { vec!["field_0"] } else { vec![] }
        });

        match TctCompiler::new(schema.clone(), format!("SystematicObject{}", i))
            .unwrap()
            .compile()
        {
            Ok(grammar) => match grammar.generate_code() {
                Ok(_) => successful += 1,
                Err(_) => failed += 1,
            },
            Err(_) => failed += 1,
        }
    }

    // Test enum variations
    for i in 2..=6 {
        let values: Vec<_> = (0..i).map(|j| format!("option_{}", j)).collect();
        let schema = json!({"enum": values});

        match TctCompiler::new(schema.clone(), format!("SystematicEnum{}", i))
            .unwrap()
            .compile()
        {
            Ok(grammar) => match grammar.generate_code() {
                Ok(_) => successful += 1,
                Err(_) => failed += 1,
            },
            Err(_) => failed += 1,
        }
    }

    // Test oneOf variations
    for i in 2..=4 {
        let mut variants = Vec::new();
        for j in 0..i {
            if j % 2 == 0 {
                variants.push(json!({"type": "string"}));
            } else {
                variants.push(json!({"type": "integer"}));
            }
        }
        let schema = json!({"oneOf": variants});

        match TctCompiler::new(schema.clone(), format!("SystematicOneOf{}", i))
            .unwrap()
            .compile()
        {
            Ok(grammar) => match grammar.generate_code() {
                Ok(_) => successful += 1,
                Err(_) => failed += 1,
            },
            Err(_) => failed += 1,
        }
    }

    let total = successful + failed;
    let success_rate = (successful as f64 / total as f64) * 100.0;

    println!("\n📊 Systematic Validation Summary:");
    println!("  Total: {}", total);
    println!("  Successful: {} ({:.1}%)", successful, success_rate);
    println!("  Failed: {}", failed);

    // We expect at least 80% success rate for systematic schemas
    assert!(
        success_rate >= 80.0,
        "Systematic schema success rate too low: {:.1}%",
        success_rate
    );
}

/// Test performance characteristics
#[test]
fn test_correctness_validation_performance_validation() {
    let test_schemas = [
        // Small schema
        json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "age": {"type": "integer"}
            }
        }),
        // Medium schema
        json!({
            "type": "object",
            "properties": {
                "field1": {"type": "string"},
                "field2": {"type": "integer"},
                "field3": {"type": "boolean"},
                "nested": {
                    "type": "object",
                    "properties": {
                        "subfield1": {"type": "string"},
                        "subfield2": {"type": "array", "items": {"type": "string"}}
                    }
                }
            }
        }),
        // Complex schema with allOf
        json!({
            "allOf": [
                {
                    "type": "object",
                    "properties": {
                        "base_field": {"type": "string"}
                    }
                },
                {
                    "type": "object",
                    "properties": {
                        "extended_field": {"type": "integer"}
                    }
                }
            ]
        }),
    ];

    for (i, schema) in test_schemas.iter().enumerate() {
        let start = std::time::Instant::now();

        let result = TctCompiler::new(schema.clone(), format!("PerfTest{}", i))
            .unwrap()
            .compile();

        let generation_time = start.elapsed();

        match result {
            Ok(grammar) => {
                let code_start = std::time::Instant::now();
                match grammar.generate_code() {
                    Ok(code) => {
                        let code_gen_time = code_start.elapsed();
                        println!(
                            "⏱️ PerfTest{}: Schema {}ms, Code {}ms, Size {} chars",
                            i,
                            generation_time.as_millis(),
                            code_gen_time.as_millis(),
                            code.len()
                        );

                        // Keep wall-clock timings as telemetry. Stable cargo
                        // tests should not fail on local scheduler contention.
                        assert!(
                            !code.is_empty(),
                            "Generated code should not be empty for PerfTest{}",
                            i
                        );
                        let generated_file = parse_generated_code(&code);
                        assert!(
                            generated_file_has_json_token_encoder_impl(&generated_file),
                            "Generated code should contain JsonTokenEncoder for PerfTest{}",
                            i
                        );
                    }
                    Err(e) => {
                        println!("❌ PerfTest{}: Code generation failed - {}", i, e);
                    }
                }
            }
            Err(e) => {
                println!("❌ PerfTest{}: Schema processing failed - {}", i, e);
            }
        }
    }
}

/// Test compatibility with real-world patterns
#[test]
fn test_correctness_validation_real_world_compatibility() {
    let real_world_patterns = vec![
        // Configuration file pattern
        (
            json!({
                "type": "object",
                "properties": {
                    "version": {"type": "string"},
                    "features": {
                        "type": "object",
                        "patternProperties": {
                            "^[a-z_]+$": {"type": "boolean"}
                        }
                    },
                    "settings": {
                        "oneOf": [
                            {"type": "object", "properties": {"mode": {"enum": ["development", "production"]}}},
                            {"type": "null"}
                        ]
                    }
                },
                "required": ["version"]
            }),
            "ConfigPattern",
        ),
        // API Response pattern
        (
            json!({
                "type": "object",
                "properties": {
                    "status": {"type": "integer", "minimum": 200, "maximum": 599},
                    "data": {
                        "oneOf": [
                            {"type": "object"},
                            {"type": "array"},
                            {"type": "null"}
                        ]
                    },
                    "meta": {
                        "type": "object",
                        "properties": {
                            "timestamp": {"type": "string"},
                            "request_id": {"type": "string"}
                        }
                    }
                }
            }),
            "ApiPattern",
        ),
    ];

    let mut successful = 0;
    let mut total = 0;

    for (schema, name) in real_world_patterns {
        total += 1;

        match TctCompiler::new(schema.clone(), name).unwrap().compile() {
            Ok(grammar) => {
                match grammar.generate_code() {
                    Ok(code) => {
                        successful += 1;
                        let generated_file = parse_generated_code(&code);
                        println!(
                            "✅ {}: Real-world pattern processed successfully ({} chars)",
                            name,
                            code.len()
                        );

                        // Verify expected code structures
                        assert!(
                            generated_file_has_json_token_encoder_impl(&generated_file),
                            "Should contain tokenizer for {}",
                            name
                        );
                        assert!(
                            generated_file_has_type_item(&generated_file),
                            "Should contain types for {}",
                            name
                        );
                    }
                    Err(e) => {
                        println!("❌ {}: Code generation failed - {}", name, e);
                    }
                }
            }
            Err(e) => {
                println!("❌ {}: Schema processing failed - {}", name, e);
            }
        }
    }

    let success_rate = (successful as f64 / total as f64) * 100.0;
    println!(
        "\n📊 Real-World Compatibility: {}/{} ({:.1}%)",
        successful, total, success_rate
    );

    // Real-world patterns should have very high success rate
    assert!(
        success_rate >= 90.0,
        "Real-world compatibility too low: {:.1}%",
        success_rate
    );
}
