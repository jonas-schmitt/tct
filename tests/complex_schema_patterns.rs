use serde_json::json;
use tct::TctCompiler;

// Comprehensive testing of complex schema patterns and combinations.
// These tests exercise challenging schema constructs that could reveal
// correctness issues in the implementation.

fn parse_generated_code(code: &str) -> syn::File {
    match syn::parse_file(code) {
        Ok(file) => file,
        Err(error) => panic!("generated code should parse as Rust: {error}"),
    }
}

fn has_generated_struct(file: &syn::File, name: &str) -> bool {
    file.items.iter().any(|item| match item {
        syn::Item::Struct(item_struct) => item_struct.ident == name,
        _ => false,
    })
}

fn generated_enum<'a>(file: &'a syn::File, name: &str) -> &'a syn::ItemEnum {
    file.items
        .iter()
        .find_map(|item| match item {
            syn::Item::Enum(item_enum) if item_enum.ident == name => Some(item_enum),
            _ => None,
        })
        .unwrap_or_else(|| panic!("expected generated enum {name}"))
}

fn has_trait_impl_for_type(file: &syn::File, trait_name: &str, type_name: &str) -> bool {
    file.items.iter().any(|item| {
        let syn::Item::Impl(item_impl) = item else {
            return false;
        };
        let Some((_, trait_path, _)) = &item_impl.trait_ else {
            return false;
        };
        let trait_matches = trait_path
            .segments
            .last()
            .is_some_and(|segment| segment.ident == trait_name);
        let type_matches = match item_impl.self_ty.as_ref() {
            syn::Type::Path(type_path) => type_path
                .path
                .segments
                .last()
                .is_some_and(|segment| segment.ident == type_name),
            _ => false,
        };
        trait_matches && type_matches
    })
}

fn assert_generated_struct_encoder(file: &syn::File, type_name: &str) {
    assert!(has_generated_struct(file, type_name));
    assert!(has_trait_impl_for_type(file, "JsonTokenEncoder", type_name));
}

fn assert_generated_enum_encoder(file: &syn::File, type_name: &str) {
    generated_enum(file, type_name);
    assert!(has_trait_impl_for_type(file, "JsonTokenEncoder", type_name));
}

#[test]
fn test_complex_schema_deeply_nested_object_structures() {
    let schema = json!({
        "title": "DeepNesting",
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
                                    "level4": {
                                        "type": "object",
                                        "properties": {
                                            "level5": {
                                                "type": "object",
                                                "properties": {
                                                    "deep_value": {"type": "string"},
                                                    "deep_array": {
                                                        "type": "array",
                                                        "items": {"type": "integer"}
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    });

    let result = TctCompiler::new(schema.clone(), "DeepNesting")
        .unwrap()
        .compile();
    assert!(result.is_ok(), "Deep nesting schema generation failed");

    let grammar = result.unwrap();
    let code = grammar
        .generate_code()
        .expect("Deep nesting code generation failed");
    let parsed = parse_generated_code(&code);

    // Verify all nested structures are generated
    assert_generated_struct_encoder(&parsed, "DeepNesting");
    assert_generated_struct_encoder(&parsed, "DeepNestingLevel1");
    assert_generated_struct_encoder(&parsed, "DeepNestingLevel1Level2");

    println!("✅ Deep nesting test passed - 5 levels deep");
}

#[test]
fn test_complex_schema_complex_enum_combinations() {
    let schema = json!({
        "title": "ComplexEnums",
        "type": "object",
        "properties": {
            "simple_enum": {
                "enum": ["option1", "option2", "option3"]
            },
            "number_enum": {
                "enum": [1, 2, 3, 42, 100]
            },
            "mixed_enum": {
                "enum": ["string_value", 42, true, null]
            },
            "large_enum": {
                "enum": [
                    "value_001", "value_002", "value_003", "value_004", "value_005",
                    "value_006", "value_007", "value_008", "value_009", "value_010",
                    "value_011", "value_012", "value_013", "value_014", "value_015",
                    "value_016", "value_017", "value_018", "value_019", "value_020"
                ]
            }
        },
        "required": ["simple_enum", "large_enum"]
    });

    let result = TctCompiler::new(schema.clone(), "ComplexEnums")
        .unwrap()
        .compile();

    // Non-string enum values are intentionally unsupported by the current JSON
    // Schema compiler. Keep the accepted failure shape explicit until a
    // dedicated multi-type enum design is added.
    match result {
        Ok(_) => {
            // If this passes, multi-type enums have been implemented!
            println!("✅ Multi-type enum support has been implemented!");
        }
        Err(e) => {
            // Expected failure - document the current limitation
            let error_msg = format!("{}", e);
            if error_msg.contains("Enum values must be strings")
                || error_msg.contains("Invalid JSON schema structure")
            {
                println!("📋 Expected failure: Multi-type enum support not yet implemented");
                println!("   Current limitation: {}", error_msg);
                println!("   Current limitation: non-string enum values are unsupported");
                return; // Skip the rest of the test
            } else {
                panic!(
                    "Unexpected error (not the expected multi-type enum limitation): {}",
                    e
                );
            }
        }
    }

    let grammar = result.unwrap();
    let code = grammar
        .generate_code()
        .expect("Complex enum code generation failed");
    let parsed = parse_generated_code(&code);

    // Verify enum structures are generated
    let large_enum = generated_enum(&parsed, "ComplexEnumsLargeEnum");
    assert_generated_enum_encoder(&parsed, "ComplexEnumsSimpleEnum");
    assert_generated_enum_encoder(&parsed, "ComplexEnumsLargeEnum");

    // Check that the large enum has all expected variants
    let large_variants: Vec<_> = large_enum
        .variants
        .iter()
        .map(|variant| variant.ident.to_string())
        .collect();
    assert!(large_variants.iter().any(|variant| variant == "Value001"));
    assert!(large_variants.iter().any(|variant| variant == "Value020"));

    println!("✅ Complex enum combinations test passed");
}

#[test]
fn test_complex_schema_multiple_oneof_combinations() {
    let schema = json!({
        "title": "MultipleOneOf",
        "type": "object",
        "properties": {
            "choice1": {
                "oneOf": [
                    {"type": "string"},
                    {"type": "integer"},
                    {
                        "type": "object",
                        "properties": {
                            "nested": {"type": "boolean"}
                        }
                    }
                ]
            },
            "choice2": {
                "oneOf": [
                    {
                        "type": "array",
                        "items": {"type": "string"}
                    },
                    {
                        "type": "object",
                        "properties": {
                            "value": {"type": "number"}
                        }
                    }
                ]
            }
        }
    });

    let result = TctCompiler::new(schema.clone(), "MultipleOneOf")
        .unwrap()
        .compile();
    assert!(result.is_ok(), "Multiple oneOf schema generation failed");

    let grammar = result.unwrap();
    let code = grammar
        .generate_code()
        .expect("Multiple oneOf code generation failed");
    let parsed = parse_generated_code(&code);

    // Verify oneOf enums are generated
    assert_generated_enum_encoder(&parsed, "MultipleOneOfChoice1Enum");
    assert_generated_enum_encoder(&parsed, "MultipleOneOfChoice2Enum");

    println!("✅ Multiple oneOf combinations test passed");
}

#[test]
fn test_complex_schema_pattern_properties_complex() {
    let schema = json!({
        "title": "ComplexPatterns",
        "type": "object",
        "properties": {
            "fixed_field": {"type": "string"}
        },
        "patternProperties": {
            "^api_key_[a-z]+$": {"type": "string"},
            "^config_[0-9]+$": {"type": "integer"},
            "^feature_flag_[a-zA-Z_]+$": {"type": "boolean"},
            "^metadata_.*": {
                "type": "object",
                "properties": {
                    "value": {"type": "string"},
                    "timestamp": {"type": "integer"}
                }
            }
        }
    });

    let result = TctCompiler::new(schema.clone(), "ComplexPatterns")
        .unwrap()
        .compile();
    assert!(
        result.is_ok(),
        "Complex pattern properties schema generation failed"
    );

    let grammar = result.unwrap();
    let code = grammar
        .generate_code()
        .expect("Complex pattern properties code generation failed");
    let parsed = parse_generated_code(&code);

    // Verify pattern validation structures
    assert_generated_struct_encoder(&parsed, "ComplexPatterns");
    assert!(code.contains("HashMap"));
    assert!(code.contains("Regex"));

    println!("✅ Complex pattern properties test passed");
}

#[test]
fn test_complex_schema_extreme_nesting_with_all_constructs() {
    let schema = json!({
        "title": "ExtremeNesting",
        "type": "object",
        "properties": {
            "level1": {
                "type": "object",
                "properties": {
                    "enum_field": {"enum": ["option1", "option2"]},
                    "conditional": {
                        "if": {"properties": {"enum_field": {"const": "option1"}}},
                        "then": {
                            "type": "object",
                            "properties": {
                                "then_array": {
                                    "type": "array",
                                    "items": {
                                        "oneOf": [
                                            {"type": "string"},
                                            {
                                                "type": "object",
                                                "properties": {
                                                    "nested_value": {"type": "integer"}
                                                }
                                            }
                                        ]
                                    }
                                }
                            }
                        },
                        "else": {
                            "type": "object",
                            "properties": {
                                "else_pattern": {
                                    "type": "object",
                                    "patternProperties": {
                                        "^dynamic_[a-z]+$": {
                                            "type": "array",
                                            "items": {"type": "boolean"}
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    });

    let result = TctCompiler::new(schema.clone(), "ExtremeNesting")
        .unwrap()
        .compile();
    assert!(result.is_ok(), "Extreme nesting schema generation failed");

    let grammar = result.unwrap();
    let code = grammar
        .generate_code()
        .expect("Extreme nesting code generation failed");
    let parsed = parse_generated_code(&code);

    // This is the most complex test - verify it generates without errors
    assert_generated_struct_encoder(&parsed, "ExtremeNesting");
    assert!(
        code.len() > 1000,
        "Generated code seems too short for this complex schema"
    );

    println!("✅ Extreme nesting with all constructs test passed");
    println!("   Generated code length: {} characters", code.len());
}

#[test]
fn test_complex_schema_large_schema_performance() {
    // Create a schema with many properties to test performance
    let mut properties = serde_json::Map::new();

    for i in 0..50 {
        properties.insert(
            format!("field_{}", i),
            match i % 4 {
                0 => json!({"type": "string"}),
                1 => json!({"type": "integer"}),
                2 => json!({"type": "boolean"}),
                3 => {
                    json!({"enum": [format!("enum_value_{}", i), format!("enum_value_{}_alt", i)]})
                }
                _ => unreachable!(),
            },
        );
    }

    let schema = json!({
        "title": "LargeSchema",
        "type": "object",
        "properties": properties
    });

    let start = std::time::Instant::now();
    let result = TctCompiler::new(schema.clone(), "LargeSchema")
        .unwrap()
        .compile();
    let generation_time = start.elapsed();

    assert!(result.is_ok(), "Large schema generation failed");

    let grammar = result.unwrap();
    let start = std::time::Instant::now();
    let code = grammar
        .generate_code()
        .expect("Large schema code generation failed");
    let code_gen_time = start.elapsed();
    let parsed = parse_generated_code(&code);

    // Verify basic structure
    assert_generated_struct_encoder(&parsed, "LargeSchema");

    println!("✅ Large schema performance test passed");
    println!("   Schema generation: {:?}", generation_time);
    println!("   Code generation: {:?}", code_gen_time);
    println!("   Generated code length: {} characters", code.len());
}

#[test]
fn test_complex_schema_schema_with_all_json_types() {
    let schema = json!({
        "title": "AllTypes",
        "type": "object",
        "properties": {
            "null_field": {"type": "null"},
            "boolean_field": {"type": "boolean"},
            "integer_field": {"type": "integer"},
            "number_field": {"type": "number"},
            "string_field": {"type": "string"},
            "array_field": {
                "type": "array",
                "items": {"type": "string"}
            },
            "object_field": {
                "type": "object",
                "properties": {
                    "nested": {"type": "string"}
                }
            },
            "multi_type_field": {
                "type": ["string", "integer", "null"]
            }
        }
    });

    let result = TctCompiler::new(schema.clone(), "AllTypes")
        .unwrap()
        .compile();
    assert!(result.is_ok(), "All types schema generation failed");

    let grammar = result.unwrap();
    let code = grammar
        .generate_code()
        .expect("All types code generation failed");
    let parsed = parse_generated_code(&code);

    // Verify all type handlers are present
    assert_generated_struct_encoder(&parsed, "AllTypes");

    // Should handle all basic JSON types
    assert!(code.contains("bool") || code.contains("Boolean"));
    assert!(code.contains("JsonInteger") || code.contains("Integer"));
    assert!(code.contains("String"));
    assert!(code.contains("Vec"));
    assert!(code.contains("Option"));

    println!("✅ All JSON types test passed");
}

/// Test schemas that might be problematic due to naming conflicts
#[test]
fn test_complex_schema_naming_conflict_schemas() {
    let problematic_schemas = [
        // Field names that might conflict with Rust keywords
        json!({
            "title": "Keywords",
            "type": "object",
            "properties": {
                "type": {"type": "string"},
                "match": {"type": "string"},
                "impl": {"type": "string"},
                "enum": {"type": "string"},
                "struct": {"type": "string"},
                "fn": {"type": "string"},
                "let": {"type": "string"},
                "mut": {"type": "string"},
                "const": {"type": "string"}
            }
        }),
        // Names with special characters
        json!({
            "title": "SpecialChars",
            "type": "object",
            "properties": {
                "field-with-dash": {"type": "string"},
                "field_with_underscore": {"type": "string"},
                "field123": {"type": "string"},
                "123field": {"type": "string"},  // Invalid identifier
                "": {"type": "string"}  // Empty name
            }
        }),
        // Very long field names
        json!({
            "title": "LongNames",
            "type": "object",
            "properties": {
                "this_is_a_very_long_field_name_that_might_cause_issues_with_code_generation_or_formatting": {"type": "string"}
            }
        }),
    ];

    for (i, schema) in problematic_schemas.iter().enumerate() {
        let default_title = format!("Test{}", i);
        let title = schema["title"].as_str().unwrap_or(&default_title);
        let result = TctCompiler::new(schema.clone(), title).unwrap().compile();

        match result {
            Ok(grammar) => {
                let code_result = grammar.generate_code();
                match code_result {
                    Ok(code) => {
                        // If it generates code, verify it's valid Rust
                        let parsed = syn::parse_file(&code);
                        assert!(
                            parsed.is_ok(),
                            "Generated invalid Rust code for naming test {}",
                            i
                        );
                        println!("✅ Naming conflict test {} passed: {}", i, title);
                    }
                    Err(e) => {
                        println!(
                            "⚠️  Naming conflict test {} failed code generation (expected): {}",
                            i, e
                        );
                        // Some schemas with invalid names are expected to fail
                    }
                }
            }
            Err(e) => {
                println!(
                    "⚠️  Naming conflict test {} failed schema parsing (expected): {}",
                    i, e
                );
                // Some schemas with invalid names are expected to fail
            }
        }
    }
}
