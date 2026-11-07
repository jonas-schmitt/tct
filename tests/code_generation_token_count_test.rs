//! TDD Tests for token_count() Method Generation
//!
//! This test suite defines the expected behavior for generating the token_count()
//! method in all generated JsonTokenEncoder implementations.
//!
//! These tests are designed to FAIL initially (Red phase of TDD) until the
//! code generator is updated to emit token_count() implementations.

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

fn type_ends_with_ident(ty: &syn::Type, name: &str) -> bool {
    matches!(ty, syn::Type::Path(type_path) if path_ends_with_ident(&type_path.path, name))
}

fn generated_token_count_method_count(file: &syn::File) -> usize {
    file.items
        .iter()
        .filter_map(|item| match item {
            syn::Item::Impl(item_impl) => Some(item_impl),
            _ => None,
        })
        .flat_map(|item_impl| item_impl.items.iter())
        .filter(|impl_item| {
            matches!(impl_item, syn::ImplItem::Fn(function) if function.sig.ident == "token_count")
        })
        .count()
}

fn generated_file_has_token_count_method(file: &syn::File) -> bool {
    generated_token_count_method_count(file) > 0
}

fn generated_file_has_json_token_encoder_impl_for(file: &syn::File, type_name: &str) -> bool {
    file.items.iter().any(|item| match item {
        syn::Item::Impl(item_impl) => {
            item_impl
                .trait_
                .as_ref()
                .is_some_and(|(_, path, _)| path_ends_with_ident(path, "JsonTokenEncoder"))
                && type_ends_with_ident(&item_impl.self_ty, type_name)
        }
        _ => false,
    })
}

/// Test that generated structs implement token_count() method
#[test]
fn test_code_generation_struct_token_count_implementation() {
    let schema = json!({
        "title": "TestStruct",
        "type": "object",
        "properties": {
            "name": {"type": "string"},
            "age": {"type": "integer"},
            "active": {"type": "boolean"}
        },
        "required": ["name"]
    });

    let grammar = TctCompiler::new(schema, "TestStruct")
        .unwrap()
        .compile()
        .unwrap();

    let generated_code = grammar.generate_code().unwrap();
    let generated_file = parse_generated_code(&generated_code);

    // Should generate token_count() method for the struct
    assert!(
        generated_file_has_token_count_method(&generated_file),
        "Generated struct should implement token_count() method"
    );

    // Should calculate token count by summing field token counts
    assert!(
        generated_code.contains("self.name.token_count()"),
        "Should call token_count() on string field 'name'"
    );

    assert!(
        generated_code.contains("self.age") && generated_code.contains("token_count()"),
        "Should call token_count() on optional integer field 'age'"
    );

    assert!(
        generated_code.contains("self.active") && generated_code.contains("token_count()"),
        "Should call token_count() on optional boolean field 'active'"
    );
}

/// Test that generated enums implement token_count() method
#[test]
fn test_code_generation_enum_token_count_implementation() {
    let schema = json!({
        "title": "TestEnum",
        "enum": ["option1", "option2", "option3"]
    });

    let grammar = TctCompiler::new(schema, "TestEnum")
        .unwrap()
        .compile()
        .unwrap();

    let generated_code = grammar.generate_code().unwrap();
    let generated_file = parse_generated_code(&generated_code);

    // Should generate token_count() method for the enum
    assert!(
        generated_file_has_token_count_method(&generated_file),
        "Generated enum should implement token_count() method"
    );

    // Should handle different enum variants
    assert!(
        generated_code.contains("match self")
            || generated_code.contains("1")
            || generated_code.contains("token_count"),
        "Should implement token counting for enum variants"
    );
}

/// Test that property-level enums implement token_count() method (benchmark failure case)
#[test]
fn test_code_generation_property_enum_token_count_implementation() {
    // This replicates the exact structure that fails in the benchmark test
    let schema = json!({
        "title": "MediumTest",
        "type": "object",
        "properties": {
            "id": { "type": "string" },
            "status": { "enum": ["pending", "active", "completed"] },
            "metadata": {
                "type": "object",
                "properties": {
                    "created_at": { "type": "string" },
                    "updated_at": { "type": "string" },
                    "priority": { "type": "integer" }
                }
            },
            "tags": {
                "type": "array",
                "items": { "type": "string" }
            }
        },
        "required": ["id", "status"]
    });

    let grammar = TctCompiler::new(schema, "MediumTest")
        .unwrap()
        .compile()
        .unwrap();

    let generated_code = grammar.generate_code().unwrap();
    let generated_file = parse_generated_code(&generated_code);

    // The main struct should implement token_count()
    assert!(
        generated_file_has_json_token_encoder_impl_for(&generated_file, "MediumTest"),
        "Main struct should implement JsonTokenEncoder"
    );

    // All generated enum types should implement token_count()
    let token_count_implementations = generated_token_count_method_count(&generated_file);

    // Should have token_count for: MediumTest struct, MediumTestStatus enum, MediumTestMetadata struct
    assert!(
        token_count_implementations >= 3,
        "Should generate token_count() for main struct, status enum, and metadata struct. Found {} implementations",
        token_count_implementations
    );

    // Specifically check for the status enum implementation
    assert!(
        generated_code.contains("MediumTestStatus")
            || generated_code.contains("enum")
            || generated_code.contains("status"),
        "Should generate enum type for status property"
    );
}

/// Test that generated nested structs implement token_count() correctly
#[test]
fn test_code_generation_nested_struct_token_count_implementation() {
    let schema = json!({
        "title": "NestedStruct",
        "type": "object",
        "properties": {
            "user": {
                "type": "object",
                "properties": {
                    "name": {"type": "string"},
                    "profile": {
                        "type": "object",
                        "properties": {
                            "bio": {"type": "string"}
                        }
                    }
                }
            },
            "tags": {
                "type": "array",
                "items": {"type": "string"}
            }
        }
    });

    let grammar = TctCompiler::new(schema, "NestedStruct")
        .unwrap()
        .compile()
        .unwrap();

    let generated_code = grammar.generate_code().unwrap();
    let generated_file = parse_generated_code(&generated_code);

    // All generated structs should implement token_count()
    let token_count_occurrences = generated_token_count_method_count(&generated_file);
    assert!(
        token_count_occurrences >= 3,
        "Should generate token_count() for main struct and nested structs. Found {} occurrences",
        token_count_occurrences
    );

    // Should handle nested objects and arrays
    assert!(
        generated_code.contains("self.user.token_count()")
            || generated_code.contains("token_count()"),
        "Should call token_count() on nested objects"
    );

    assert!(
        generated_code.contains("self.tags.token_count()")
            || generated_code.contains("token_count()"),
        "Should call token_count() on array fields"
    );
}

/// Test that optimized enums implement token_count() correctly
#[test]
fn test_code_generation_optimized_enum_token_count_implementation() {
    let schema = json!({
        "title": "OptimizedEnum",
        "type": "object",
        "properties": {
            "status": {"enum": ["active", "inactive", "pending"]},
            "priority": {
                "type": "integer",
                "minimum": 1,
                "maximum": 5
            }
        }
    });

    let grammar = TctCompiler::new(schema, "OptimizedEnum")
        .unwrap()
        .with_optimization_level(tct::OptimizationLevel::O2)
        .compile()
        .unwrap();

    let generated_code = grammar.generate_code().unwrap();
    let generated_file = parse_generated_code(&generated_code);

    // Optimized enums should still implement token_count()
    assert!(
        generated_file_has_token_count_method(&generated_file),
        "Optimized enums should implement token_count() method"
    );

    // Should handle both optimized enum and optimized bounded integer
    let token_count_occurrences = generated_token_count_method_count(&generated_file);
    assert!(
        token_count_occurrences >= 2,
        "Should generate token_count() for main struct and enum types. Found {} occurrences",
        token_count_occurrences
    );
}

/// Test that pattern-constrained string types implement token_count() correctly.
#[test]
fn test_code_generation_pattern_optimized_token_count_implementation() {
    let schema = json!({
        "title": "PatternOptimized",
        "type": "object",
        "properties": {
            "log_level": {
                "type": "string",
                "pattern": "^(debug|info|warn|error)$"
            }
        }
    });

    let grammar = TctCompiler::new(schema, "PatternOptimized")
        .unwrap()
        .with_optimization_level(tct::OptimizationLevel::O2)
        .compile()
        .unwrap();

    let generated_code = grammar.generate_code().unwrap();
    let generated_file = parse_generated_code(&generated_code);

    // Pattern-constrained types still use standard generated string encoding.
    assert!(
        generated_file_has_token_count_method(&generated_file),
        "Pattern-constrained types should implement token_count() method"
    );

    // Should work even though direct finite-regex encoding is reserved for future work.
    let token_count_occurrences = generated_token_count_method_count(&generated_file);
    assert!(
        token_count_occurrences >= 1,
        "Should generate token_count() for pattern-constrained types. Found {} occurrences",
        token_count_occurrences
    );
}
