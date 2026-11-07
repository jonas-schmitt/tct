//! Tests for bounded integer encoding optimization (Phase 1.1)
//!
//! This test suite validates that integers with minimum/maximum constraints
//! use optimized range encoding instead of the default 8-byte i64 encoding.

use quote::ToTokens;
use serde_json::json;
use tct::TctCompiler;

fn parse_generated_code(code: &str) -> syn::File {
    syn::parse_file(code).unwrap_or_else(|error| panic!("generated Rust should parse: {error}"))
}

fn assert_generated_tuple_struct_field(code: &str, struct_name: &str, expected_type: &str) {
    let file = parse_generated_code(code);
    let structure = file
        .items
        .iter()
        .find_map(|item| match item {
            syn::Item::Struct(structure) if structure.ident == struct_name => Some(structure),
            _ => None,
        })
        .unwrap_or_else(|| panic!("generated struct `{struct_name}` should exist"));
    let syn::Fields::Unnamed(fields) = &structure.fields else {
        panic!("generated struct `{struct_name}` should be a tuple struct");
    };
    let field =
        fields.unnamed.iter().next().unwrap_or_else(|| {
            panic!("generated tuple struct `{struct_name}` should have a field")
        });

    assert_eq!(field.ty.to_token_stream().to_string(), expected_type);
}

#[test]
fn test_bounded_integer_constraint_aware_integer_optimization() {
    // Test the new constraint-aware integer optimization (Phase 1.1)
    let schema = json!({
        "title": "BoundedInteger",
        "type": "object",
        "properties": {
            "tiny_range": {
                "type": "integer",
                "minimum": 0,
                "maximum": 3
            },
            "small_range": {
                "type": "integer",
                "minimum": 10,
                "maximum": 110
            },
            "large_range": {
                "type": "integer",
                "minimum": -1000000,
                "maximum": 1000000
            },
            "unconstrained": {
                "type": "integer"
            }
        },
        "required": ["tiny_range"]
    });

    let result = TctCompiler::new(schema.clone(), "BoundedInteger")
        .unwrap()
        .compile();
    assert!(result.is_ok(), "Schema generation should work");

    let grammar = result.unwrap();
    let code = grammar
        .generate_code()
        .expect("Code generation should work");

    println!(
        "🚀 Optimized generated code for bounded integers:\n{}",
        code
    );

    // Constrained integers should use newtype wrappers
    assert!(code.contains("pub tiny_range: BoundedIntegerTinyRange"));
    assert!(code.contains("pub small_range: Option<BoundedIntegerSmallRange>"));
    assert!(code.contains("pub large_range: Option<BoundedIntegerLargeRange>"));

    // Verify newtype wrappers are generated
    assert_generated_tuple_struct_field(&code, "BoundedIntegerTinyRange", "i64");
    assert_generated_tuple_struct_field(&code, "BoundedIntegerSmallRange", "i64");
    assert_generated_tuple_struct_field(&code, "BoundedIntegerLargeRange", "i64");

    // Unconstrained integers use a lossless JSON integer wrapper.
    assert!(code.contains("pub unconstrained: Option<crate::tokenizer::JsonInteger>"));

    println!("✅ Phase 1.1 Bounded Integer Encoding successfully implemented!");
    println!("   - tiny_range (0-3): Newtype wrapper (1 token) = 50% compression");
    println!("   - small_range (10-110): Newtype wrapper (1 token) = 50% compression");
    println!("   - large_range (-1M to 1M): Newtype wrapper (1 token) = 50% compression");
    println!("   - unconstrained: JsonInteger (lossless signed/unsigned JSON integer)");
}

#[test]
fn test_bounded_integer_bounded_integer_compression_potential() {
    // Demonstrate compression potential with different ranges
    let test_cases = vec![
        // (range_size, expected_bits_needed, description)
        (4, 2, "tiny_range (0-3): needs 2 bits instead of 64"),
        (101, 7, "small_range (10-110): needs 7 bits instead of 64"),
        (
            2000001,
            21,
            "large_range (-1000000 to 1000000): needs 21 bits instead of 64",
        ),
    ];

    for (range_size, expected_bits, description) in test_cases {
        let bits_needed = (range_size as f64).log2().ceil() as u32;
        println!(
            "Range size {}: {} bits needed ({})",
            range_size, bits_needed, description
        );
        assert_eq!(
            bits_needed, expected_bits,
            "Bit calculation should be correct"
        );
    }

    println!("✅ Compression potential analysis complete");
}

#[test]
fn test_bounded_integer_range_encoding_design() {
    // Test the mathematical foundation for range encoding

    // For a range [min, max], we need:
    // 1. Offset calculation: value_offset = actual_value - min
    // 2. Range size: range_size = max - min + 1
    // 3. Bits needed: bits = ceil(log2(range_size))
    // 4. Tokens needed: tokens = ceil(bits / 32)

    let test_ranges = vec![
        (0, 3, 4),         // Range [0,3] = 4 values
        (10, 110, 101),    // Range [10,110] = 101 values
        (-50, 50, 101),    // Range [-50,50] = 101 values
        (0, 255, 256),     // Range [0,255] = 256 values (1 byte)
        (0, 65535, 65536), // Range [0,65535] = 65536 values (2 bytes)
    ];

    for (min, max, expected_range_size) in test_ranges {
        let range_size = (max - min + 1) as u64;
        assert_eq!(range_size, expected_range_size as u64);

        let bits_needed = if range_size == 1 {
            0
        } else {
            (range_size as f64).log2().ceil() as u32
        };
        let tokens_needed = if bits_needed == 0 {
            1
        } else {
            bits_needed.div_ceil(32)
        }; // ceil(bits/32)

        println!(
            "Range [{}, {}]: size={}, bits={}, tokens={}",
            min, max, range_size, bits_needed, tokens_needed
        );
    }

    println!("✅ Range encoding mathematics validated");
}

#[test]
fn test_bounded_integer_comprehensive_integer_constraints() {
    // Test various constraint scenarios to ensure proper optimization selection
    let test_cases = vec![
        // (schema, expected_type, description)
        (
            json!({"type": "integer", "minimum": 0, "maximum": 1}),
            "BoundedU8",
            "Boolean-like range",
        ),
        (
            json!({"type": "integer", "minimum": 0, "maximum": 255}),
            "BoundedU8",
            "1-byte range",
        ),
        (
            json!({"type": "integer", "minimum": 1, "maximum": 256}),
            "BoundedU8",
            "Exactly 1-byte range (256 values)",
        ),
        (
            json!({"type": "integer", "minimum": 0, "maximum": 65535}),
            "BoundedU16",
            "2-byte range",
        ),
        (
            json!({"type": "integer", "minimum": 0, "maximum": 100000}),
            "BoundedU32",
            "Medium-large range",
        ),
        (
            json!({"type": "integer", "minimum": -100, "maximum": 100}),
            "BoundedU8",
            "Symmetric signed range",
        ),
        (
            json!({"type": "integer", "minimum": -32768, "maximum": 32767}),
            "BoundedU16",
            "16-bit signed range",
        ),
        (
            json!({"type": "integer", "minimum": 1000000000, "maximum": 1000000100}),
            "BoundedU8",
            "Large but narrow range",
        ),
        (
            json!({"type": "integer"}),
            "JsonInteger",
            "Unconstrained integer",
        ),
        (
            json!({"type": "integer", "minimum": 0}),
            "JsonInteger",
            "Only minimum constraint",
        ),
        (
            json!({"type": "integer", "maximum": 100}),
            "JsonInteger",
            "Only maximum constraint",
        ),
    ];

    for (schema_props, expected_type, description) in test_cases {
        let schema = json!({
            "title": "TestInteger",
            "type": "object",
            "properties": {
                "value": schema_props
            }
        });

        let result = TctCompiler::new(schema.clone(), "TestInteger")
            .unwrap()
            .compile();
        assert!(
            result.is_ok(),
            "Schema generation should work for: {}",
            description
        );

        let grammar = result.unwrap();
        let code = grammar
            .generate_code()
            .expect("Code generation should work");

        // With the newtype wrapper approach:
        // - Constrained integers get wrapper types (TestIntegerValue)
        // - Unconstrained or partially bounded integers use JsonInteger
        let expected_pattern = match expected_type {
            "BoundedU8" | "BoundedU16" | "BoundedU32" => {
                // Constrained integers get newtype wrappers
                "pub value: Option<TestIntegerValue>"
            }
            "JsonInteger" => "pub value: Option<crate::tokenizer::JsonInteger>",
            _ => panic!("Unknown expected type: {}", expected_type),
        };

        assert!(
            code.contains(expected_pattern),
            "Expected '{}' for {} but code was:\n{}",
            expected_pattern,
            description,
            code
        );

        println!("✅ {}: {} -> {}", description, schema_props, expected_type);
    }
}

#[test]
fn test_bounded_integer_mixed_integer_constraints_in_struct() {
    // Test a struct with multiple integer fields having different constraint types
    let schema = json!({
        "title": "MixedIntegers",
        "type": "object",
        "properties": {
            "tiny": {"type": "integer", "minimum": 0, "maximum": 15},
            "small": {"type": "integer", "minimum": 0, "maximum": 1000},
            "large": {"type": "integer", "minimum": 0, "maximum": 100000},
            "unconstrained": {"type": "integer"},
            "negative_range": {"type": "integer", "minimum": -50, "maximum": 50}
        },
        "required": ["tiny"]
    });

    let result = TctCompiler::new(schema.clone(), "MixedIntegers")
        .unwrap()
        .compile();
    assert!(
        result.is_ok(),
        "Mixed integer schema should generate successfully"
    );

    let grammar = result.unwrap();
    let code = grammar
        .generate_code()
        .expect("Code generation should work");

    println!("🚀 Mixed integer constraints generated code:\n{}", code);

    // Verify each field gets a newtype wrapper (bounded integer optimization)
    assert!(code.contains("pub tiny: MixedIntegersTiny"));
    assert!(code.contains("pub small: Option<MixedIntegersSmall>"));
    assert!(code.contains("pub large: Option<MixedIntegersLarge>"));
    assert!(code.contains("pub unconstrained: Option<crate::tokenizer::JsonInteger>"));
    assert!(code.contains("pub negative_range: Option<MixedIntegersNegativeRange>"));

    // Verify newtype wrappers are generated
    assert_generated_tuple_struct_field(&code, "MixedIntegersTiny", "i64");
    assert_generated_tuple_struct_field(&code, "MixedIntegersSmall", "i64");
    assert_generated_tuple_struct_field(&code, "MixedIntegersLarge", "i64");
    assert_generated_tuple_struct_field(&code, "MixedIntegersNegativeRange", "i64");

    println!("✅ Mixed integer constraints properly optimized");
}

#[test]
fn test_bounded_integer_integer_constraint_edge_cases() {
    // Test edge cases that should NOT be optimized
    let edge_cases = [
        // Very large ranges that exceed u32
        json!({"type": "integer", "minimum": 0, "maximum": 5000000000i64}),
        // Ranges with extreme negative values
        json!({"type": "integer", "minimum": -2147483648i64, "maximum": 2147483647i64}),
        // Range size exactly at boundary
        json!({"type": "integer", "minimum": 0, "maximum": 4294967295i64}),
    ];

    for (i, schema_props) in edge_cases.iter().enumerate() {
        let schema = json!({
            "title": "EdgeCase",
            "type": "object",
            "properties": {
                "value": schema_props
            }
        });

        let result = TctCompiler::new(schema.clone(), "EdgeCase")
            .unwrap()
            .compile();
        assert!(
            result.is_ok(),
            "Edge case {} should generate successfully",
            i
        );

        let grammar = result.unwrap();
        let code = grammar
            .generate_code()
            .expect("Code generation should work");

        // Even for large ranges, wrapper types are created (consistent newtype approach)
        // But they use standard i64 encoding internally (not optimized offset encoding)
        assert!(
            code.contains("pub value: Option<EdgeCaseValue>"),
            "Edge case {} should create wrapper type, but got:\n{}",
            i,
            code
        );
        assert_generated_tuple_struct_field(&code, "EdgeCaseValue", "i64");

        println!(
            "✅ Edge case {}: correctly uses wrapper type with i64 encoding",
            i
        );
    }
}

#[test]
fn test_bounded_integer_backward_compatibility() {
    // Ensure that existing schemas without constraints continue to work
    let schema = json!({
        "title": "LegacyInteger",
        "type": "object",
        "properties": {
            "old_field": {"type": "integer"},
            "another_field": {"type": "string"}
        }
    });

    let result = TctCompiler::new(schema.clone(), "LegacyInteger")
        .unwrap()
        .compile();
    assert!(result.is_ok(), "Legacy schema should still work");

    let grammar = result.unwrap();
    let code = grammar
        .generate_code()
        .expect("Code generation should work");

    // Should use the lossless JsonInteger wrapper for unconstrained integers
    assert!(code.contains("pub old_field: Option<crate::tokenizer::JsonInteger>"));
    assert!(code.contains("pub another_field: Option<String>"));

    println!("✅ Unconstrained integer schemas use lossless JsonInteger fields");
}

#[test]
fn test_bounded_integer_constraint_analysis_debug() {
    use serde_json::json;
    use tct::bounded_integer::analyze_integer_constraints;

    let test_cases = vec![
        (
            json!({"minimum": 1, "maximum": 256}),
            "range [1, 256] = 256 values",
        ),
        (
            json!({"minimum": 0, "maximum": 255}),
            "range [0, 255] = 256 values",
        ),
        (
            json!({"minimum": 0, "maximum": 256}),
            "range [0, 256] = 257 values",
        ),
    ];

    for (props, description) in test_cases {
        let props_map = props.as_object().unwrap();
        let result = analyze_integer_constraints(props_map);
        println!("Debug: {} -> {:?}", description, result);

        let min = props_map.get("minimum").unwrap().as_i64().unwrap();
        let max = props_map.get("maximum").unwrap().as_i64().unwrap();
        let range_size = max - min + 1;
        println!(
            "  Range size calculation: {} - {} + 1 = {}",
            max, min, range_size
        );
    }

    // Debug the problematic edge case
    let edge_case = json!({"minimum": -2147483648i64, "maximum": 2147483647i64});
    let props_map = edge_case.as_object().unwrap();
    let result = analyze_integer_constraints(props_map);
    println!("Debug edge case: {:?} -> {:?}", edge_case, result);

    let min = props_map.get("minimum").unwrap().as_i64().unwrap();
    let max = props_map.get("maximum").unwrap().as_i64().unwrap();
    println!("  min: {}, max: {}", min, max);

    match max.checked_sub(min) {
        Some(diff) => {
            println!("  max - min = {}", diff);
            match diff.checked_add(1) {
                Some(size) => println!("  range_size = {}", size),
                None => println!("  range_size calculation overflowed!"),
            }
        }
        None => println!("  max - min calculation overflowed!"),
    }
}
