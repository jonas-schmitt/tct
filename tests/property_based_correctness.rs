use proptest::prelude::*;
use quickcheck::{TestResult, quickcheck};
use serde_json::{Value, json};
use tct::TctCompiler;

const TOKENIZER_STRUCTS: &[&str] = &[
    "EOFContext",
    "BufferedTokenIterator",
    "GapInfo",
    "PartialEncoding",
    "PrefixResult",
    "VariantPresence",
];

fn parse_generated_code(code: &str) -> syn::File {
    syn::parse_file(code).unwrap_or_else(|error| panic!("generated Rust should parse: {error}"))
}

fn generated_user_struct_names(code: &str) -> Vec<String> {
    parse_generated_code(code)
        .items
        .iter()
        .filter_map(|item| match item {
            syn::Item::Struct(structure)
                if !TOKENIZER_STRUCTS.contains(&structure.ident.to_string().as_str()) =>
            {
                Some(structure.ident.to_string())
            }
            _ => None,
        })
        .collect()
}

fn generated_has_enum_named(code: &str, name: &str) -> bool {
    parse_generated_code(code)
        .items
        .iter()
        .any(|item| matches!(item, syn::Item::Enum(enumeration) if enumeration.ident == name))
}

fn generated_has_struct_named(code: &str, name: &str) -> bool {
    parse_generated_code(code)
        .items
        .iter()
        .any(|item| matches!(item, syn::Item::Struct(structure) if structure.ident == name))
}

fn generated_has_struct_or_enum(code: &str) -> bool {
    parse_generated_code(code)
        .items
        .iter()
        .any(|item| matches!(item, syn::Item::Struct(_) | syn::Item::Enum(_)))
}

fn generated_has_json_token_encoder_impl(code: &str) -> bool {
    parse_generated_code(code).items.iter().any(|item| {
        let syn::Item::Impl(item_impl) = item else {
            return false;
        };
        item_impl.trait_.as_ref().is_some_and(|(_, path, _)| {
            path.segments
                .last()
                .is_some_and(|segment| segment.ident == "JsonTokenEncoder")
        })
    })
}

/// Property-based testing for comprehensive schema correctness validation
/// This module uses both proptest and quickcheck to systematically test
/// schema generation and tokenization across the entire space of valid inputs.
// Simple schema generators for property-based testing
fn arb_simple_type() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("string".to_string()),
        Just("integer".to_string()),
        Just("boolean".to_string()),
        Just("number".to_string()),
    ]
}

fn arb_enum_values() -> impl Strategy<Value = Vec<String>> {
    // Generate more realistic enum values that won't cause identifier issues
    let realistic_enum_value = prop_oneof![
        // Common realistic enum patterns
        "[a-z]{3,10}",      // lowercase words
        "[A-Z]{3,10}",      // uppercase words
        "[a-z]+_[a-z]+",    // snake_case
        "[A-Z][a-z]+",      // PascalCase
        "[a-z]+[0-9]",      // word_with_number
        "VALUE_[0-9]{1,3}", // CONSTANT_STYLE
    ];

    prop::collection::vec(realistic_enum_value, 1..10)
}

fn arb_simple_schema() -> impl Strategy<Value = Value> {
    (arb_simple_type(), prop::option::of(arb_enum_values())).prop_map(|(type_name, enum_vals)| {
        if let Some(values) = enum_vals {
            json!({
                "title": "PropTestEnum",
                "enum": values
            })
        } else {
            json!({
                "title": "PropTestSimple",
                "type": type_name
            })
        }
    })
}

fn arb_object_schema() -> impl Strategy<Value = Value> {
    let properties = prop::collection::btree_map(
        "[a-zA-Z][a-zA-Z0-9_]*", // Valid property names
        arb_simple_schema(),
        1..5,
    );

    (
        properties,
        prop::collection::vec("[a-zA-Z][a-zA-Z0-9_]*", 0..3),
    )
        .prop_map(|(props, required)| {
            let mut schema = json!({
                "title": "PropTestObject",
                "type": "object",
                "properties": props
            });

            if !required.is_empty() {
                schema["required"] = json!(required);
            }

            schema
        })
}

proptest! {
    #[test]
    fn test_property_based_simple_schema_generation_always_succeeds(
        schema in arb_simple_schema(),
        name in "[A-Z][a-zA-Z0-9]*"
    ) {
        let result = TctCompiler::new(schema.clone(), &name).unwrap().compile();
        prop_assert!(result.is_ok(), "Schema generation failed for: {:?}", schema);

        if let Ok(grammar) = result {
            let code_result = grammar.generate_code();
            prop_assert!(code_result.is_ok(), "Code generation failed for schema: {:?}", schema);
        }
    }

    #[test]
    fn test_property_based_object_schema_generation_correctness(
        schema in arb_object_schema(),
        name in "[A-Z][A-Z0-9]*"  // Only uppercase letters and digits for valid PascalCase
    ) {
        let result = TctCompiler::new(schema.clone(), &name).unwrap().compile();
        prop_assert!(result.is_ok(), "Object schema generation failed for: {:?}", schema);

        if let Ok(grammar) = result {
            let code_result = grammar.generate_code();
            prop_assert!(code_result.is_ok(), "Object code generation failed for schema: {:?}", schema);

            if let Ok(code) = code_result {
                // Verify generated code contains expected structures
                // TCT sanitizes names to PascalCase, so we check for user-defined structs
                // (excluding tokenizer utility structs)
                let user_struct_names = generated_user_struct_names(&code);

                prop_assert!(!user_struct_names.is_empty(),
                           "Generated code missing user-defined struct. Schema: {:?}, Generated struct names: {:?}",
                           schema,
                           user_struct_names);
                prop_assert!(generated_has_json_token_encoder_impl(&code),
                           "Generated code missing JsonTokenEncoder implementation");
            }
        }
    }
}

/// QuickCheck-based tests for different testing patterns
#[test]
fn quickcheck_enum_schema_correctness() {
    fn prop(values: Vec<String>) -> TestResult {
        if values.is_empty() || values.len() > 50 {
            return TestResult::discard();
        }

        // Filter out invalid enum values that would cause identifier generation issues
        let valid_values: Vec<_> = values
            .into_iter()
            .filter(|v| {
                // Basic length and emptiness checks
                !v.is_empty() && v.len() < 100 &&
            // Filter out strings that would become empty after identifier sanitization
            !v.trim().is_empty() &&
            // Filter out strings with only control characters/null chars
            !v.chars().all(|c| c.is_control() || c == '\0') &&
            // Filter out strings that are only special characters (would become empty)
            v.chars().any(|c| c.is_alphanumeric())
            })
            .collect();

        if valid_values.is_empty() {
            return TestResult::discard();
        }

        let schema = json!({
            "title": "TestEnum",
            "type": "string",
            "enum": valid_values
        });

        let result = TctCompiler::new(schema.clone(), "TestEnum")
            .unwrap()
            .compile();
        if result.is_err() {
            return TestResult::failed();
        }

        let grammar = result.unwrap();
        let code_result = grammar.generate_code();

        match code_result {
            Ok(code) => {
                // Verify the code contains the expected enum structure
                // Note: For root-level enums, the name might be RootValue instead of TestEnum
                TestResult::from_bool(
                    (generated_has_enum_named(&code, "TestEnum")
                        || generated_has_enum_named(&code, "RootValue"))
                        && generated_has_json_token_encoder_impl(&code),
                )
            }
            Err(_) => TestResult::failed(),
        }
    }

    quickcheck(prop as fn(Vec<String>) -> TestResult);
}

#[test]
fn quickcheck_nested_object_correctness() {
    fn prop(depth: u8) -> TestResult {
        if depth == 0 || depth > 5 {
            return TestResult::discard();
        }

        // Generate a nested object schema
        let mut schema = json!({
            "type": "object",
            "properties": {
                "value": {"type": "string"}
            }
        });

        let mut current = &mut schema;
        for i in 1..depth {
            let next_level = json!({
                "type": "object",
                "properties": {
                    "value": {"type": "string"},
                    format!("nested_{}", i): {
                        "type": "object",
                        "properties": {
                            "inner_value": {"type": "integer"}
                        }
                    }
                }
            });

            current["properties"][format!("nested_{}", i)] = next_level;
            current = &mut current["properties"][format!("nested_{}", i)];
        }

        let result = TctCompiler::new(schema.clone(), "NestedTest")
            .unwrap()
            .compile();
        match result {
            Ok(grammar) => match grammar.generate_code() {
                Ok(code) => TestResult::from_bool(
                    generated_has_struct_named(&code, "NestedTest")
                        && generated_has_json_token_encoder_impl(&code),
                ),
                Err(_) => TestResult::failed(),
            },
            Err(_) => TestResult::failed(),
        }
    }

    quickcheck(prop as fn(u8) -> TestResult);
}

/// Test schema generation with various constraint combinations
#[test]
fn test_property_based_constraint_combination_correctness() {
    let test_cases = [
        // String with enum constraint
        json!({
            "title": "StringEnum",
            "enum": ["option1", "option2", "option3"]
        }),
        // Object with mixed property types
        json!({
            "title": "MixedObject",
            "type": "object",
            "properties": {
                "str_field": {"type": "string"},
                "int_field": {"type": "integer"},
                "bool_field": {"type": "boolean"},
                "enum_field": {"enum": ["a", "b", "c"]}
            },
            "required": ["str_field", "enum_field"]
        }),
        // Array of objects
        json!({
            "title": "ArrayObject",
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
        }),
        // oneOf with different types
        json!({
            "title": "OneOfTest",
            "oneOf": [
                {"type": "string"},
                {"type": "integer"},
                {
                    "type": "object",
                    "properties": {
                        "special": {"type": "boolean"}
                    }
                }
            ]
        }),
    ];

    for (i, schema) in test_cases.iter().enumerate() {
        let name = format!("TestCase{}", i);
        let result = TctCompiler::new(schema.clone(), &name).unwrap().compile();

        assert!(
            result.is_ok(),
            "Failed to generate schema for test case {}: {:?}",
            i,
            schema
        );

        let grammar = result.unwrap();
        let code_result = grammar.generate_code();

        assert!(
            code_result.is_ok(),
            "Failed to generate code for test case {}: {:?}",
            i,
            schema
        );

        let code = code_result.unwrap();

        // Verify basic structure is present
        assert!(
            generated_has_json_token_encoder_impl(&code),
            "Missing JsonTokenEncoder in test case {}",
            i
        );

        // Verify struct or enum is generated
        assert!(
            generated_has_struct_or_enum(&code),
            "Missing main type definition in test case {}",
            i
        );
    }
}

/// Debug test to understand struct naming
#[test]
fn test_debug_struct_naming() {
    let schema = json!({
        "title": "PropTestObject",
        "type": "object",
        "properties": {
            "A": {
                "title": "PropTestSimple",
                "type": "string"
            }
        }
    });

    let name = "AA";
    let result = TctCompiler::new(schema.clone(), name).unwrap().compile();
    let grammar = result.unwrap();
    let code = grammar.generate_code().unwrap();

    println!("=== Testing with name: {} ===", name);
    println!("=== ALL STRUCT/ENUM DEFINITIONS ===");
    for line in code.lines() {
        if line.trim().starts_with("pub struct ")
            || line.trim().starts_with("pub enum ")
            || line.trim().starts_with("struct ")
            || line.trim().starts_with("enum ")
        {
            println!("{}", line);
        }
    }

    println!("\n=== SEARCHING FOR ROOT STRUCT ===");
    // Check for various possible names
    let possible_names = vec![
        name.to_string(),
        "A0A".to_string(),
        "A_0a".to_string(),
        "A_0_a".to_string(),
        "RootValue".to_string(),
        format!("{}Value", name),
    ];

    for possible_name in possible_names {
        if generated_has_struct_named(&code, &possible_name) {
            println!("✅ Found: struct {}", possible_name);
        }
    }
}

/// Test edge cases that might cause issues
#[test]
fn test_property_based_edge_case_schema_correctness() {
    let edge_cases = [
        // Empty object
        json!({"title": "EmptyObject", "type": "object"}),
        // Object with no required fields
        json!({
            "title": "OptionalFields",
            "type": "object",
            "properties": {
                "optional1": {"type": "string"},
                "optional2": {"type": "integer"}
            }
        }),
        // Single enum value
        json!({
            "title": "SingleEnum",
            "enum": ["only_value"]
        }),
        // Deeply nested optional fields
        json!({
            "title": "DeepNested",
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
    ];

    for (i, schema) in edge_cases.iter().enumerate() {
        let name = format!("EdgeCase{}", i);
        let result = TctCompiler::new(schema.clone(), &name).unwrap().compile();

        assert!(
            result.is_ok(),
            "Edge case {} failed schema generation: {:?}",
            i,
            schema
        );

        let grammar = result.unwrap();
        let code_result = grammar.generate_code();

        assert!(
            code_result.is_ok(),
            "Edge case {} failed code generation: {:?}",
            i,
            schema
        );

        println!("✅ Edge case {} passed: {:?}", i, schema);
    }
}

/// Test that generated code can be parsed as valid Rust
#[test]
fn test_property_based_generated_code_syntax_validity() {
    let schemas = vec![
        json!({
            "title": "SyntaxTest1",
            "type": "object",
            "properties": {
                "field": {"type": "string"}
            }
        }),
        json!({
            "title": "SyntaxTest2",
            "type": "string",
            "enum": ["value1", "value2"]
        }),
        json!({
            "title": "SyntaxTest3",
            "oneOf": [
                {"type": "string"},
                {"type": "integer"}
            ]
        }),
    ];

    for schema in schemas {
        let title = schema["title"].as_str().unwrap_or("Test");
        let result = TctCompiler::new(schema.clone(), title).unwrap().compile();
        assert!(result.is_ok(), "Schema generation failed");

        let grammar = result.unwrap();
        let code = grammar.generate_code().expect("Code generation failed");

        // Verify the code can be parsed as valid Rust syntax
        let parsed = syn::parse_file(&code);
        assert!(
            parsed.is_ok(),
            "Generated code has invalid syntax: {}",
            code
        );

        println!("✅ Syntax validation passed for {}", title);
    }
}
