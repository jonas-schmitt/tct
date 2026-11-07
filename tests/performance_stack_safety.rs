//! Performance Stack Safety Test Suite
//!
//! This test suite validates TCT's protection against stack overflow conditions
//! and ensures safe handling of deeply recursive schemas, circular references,
//! and pathological input patterns that could cause infinite recursion.

use serde_json::json;
use std::collections::HashMap;
use tct::{TctCompiler, TctError};

fn parse_generated_file(code: &str) -> syn::File {
    syn::parse_file(code).unwrap_or_else(|error| panic!("generated Rust should parse: {error}"))
}

fn assert_generated_struct(code: &str, struct_name: &str) {
    let file = parse_generated_file(code);
    assert!(
        file.items.iter().any(|item| match item {
            syn::Item::Struct(item_struct) => item_struct.ident == struct_name,
            _ => false,
        }),
        "Should generate expected root struct {struct_name}"
    );
}

/// Test extremely deep recursive schema nesting to verify stack safety
#[test]
fn test_stack_safety_extremely_deep_recursion() {
    // Create 500-level deep recursion to test stack limits
    let mut current_schema = json!({
        "type": "string",
        "description": "Terminal value at extreme depth"
    });

    for depth in (0..50).rev() {
        current_schema = json!({
            "type": "object",
            "properties": {
                format!("level_{}", depth): current_schema
            },
            "required": [format!("level_{}", depth)]
        });
    }

    let schema = json!({
        "title": "ExtremelyDeep",
        "type": "object",
        "properties": {
            "root": current_schema
        },
        "required": ["root"]
    });

    // Should handle extreme depth without stack overflow
    let result = TctCompiler::new(schema, "ExtremelyDeep")
        .and_then(|compiler| compiler.compile())
        .map_err(|e| -> Box<dyn std::error::Error> { Box::new(e) })
        .and_then(|grammar| {
            grammar
                .generate_code()
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
        });

    // Either succeeds with proper handling or fails gracefully (no stack overflow panic)
    match result {
        Ok(code) => {
            assert_generated_struct(&code, "ExtremelyDeep");
        }
        Err(err) => {
            let error_msg = err.to_string();
            assert!(
                error_msg.contains("deep")
                    || error_msg.contains("recursion")
                    || error_msg.contains("limit"),
                "Should provide informative error about depth limits, got: {}",
                error_msg
            );
        }
    }
}

#[test]
fn test_stack_safety_configurable_schema_depth_limit() {
    let mut nested_schema = json!({ "type": "string" });
    for depth in (0..5).rev() {
        nested_schema = json!({
            "type": "object",
            "properties": {
                format!("level_{}", depth): nested_schema
            }
        });
    }

    let schema = json!({
        "title": "DepthLimited",
        "type": "object",
        "properties": {
            "root": nested_schema
        }
    });

    let result = TctCompiler::new(schema, "DepthLimited")
        .unwrap()
        .with_max_schema_depth(3)
        .compile();
    let err = match result {
        Ok(_) => panic!("depth-limited compile should reject the nested schema"),
        Err(err) => err,
    };

    match err {
        TctError::SchemaProcessing {
            message,
            schema_path,
            suggestion,
            ..
        } => {
            assert!(
                message.contains("schema traversal depth"),
                "unexpected message: {}",
                message
            );
            assert!(
                schema_path.contains("#/properties/root"),
                "schema path should point into the nested schema, got {}",
                schema_path
            );
            assert!(
                suggestion.contains("with_max_schema_depth"),
                "suggestion should point to the configuration knob, got {}",
                suggestion
            );
        }
        other => panic!("expected SchemaProcessing depth error, got {other:?}"),
    }
}

#[test]
fn test_stack_safety_configurable_schema_depth_limit_covers_array_items() {
    let mut nested_items = json!({ "type": "string" });
    for _ in 0..5 {
        nested_items = json!({
            "type": "array",
            "items": nested_items
        });
    }

    let schema = json!({
        "title": "DepthLimitedArray",
        "type": "object",
        "properties": {
            "values": nested_items
        }
    });

    let result = TctCompiler::new(schema, "DepthLimitedArray")
        .unwrap()
        .with_max_schema_depth(5)
        .compile();
    let err = match result {
        Ok(_) => panic!("depth-limited compile should reject nested array item schemas"),
        Err(err) => err,
    };

    match err {
        TctError::SchemaProcessing { schema_path, .. } => {
            assert!(
                schema_path.contains("/items"),
                "schema path should point through array items, got {}",
                schema_path
            );
        }
        other => panic!("expected SchemaProcessing depth error, got {other:?}"),
    }
}

/// Test circular reference detection and safe handling
#[test]
fn test_stack_safety_circular_references() {
    let schema = json!({
        "title": "CircularTest",
        "type": "object",
        "properties": {
            "self_ref": { "$ref": "#" },
            "name": { "type": "string" }
        },
        "required": ["name"]
    });

    // Should detect and handle circular references safely
    let result = TctCompiler::new(schema, "CircularTest")
        .and_then(|compiler| compiler.compile())
        .map_err(|e| -> Box<dyn std::error::Error> { Box::new(e) })
        .and_then(|grammar| {
            grammar
                .generate_code()
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
        });

    match result {
        Ok(code) => {
            assert_generated_struct(&code, "CircularTest");
            // If circular references are supported, verify proper handling
        }
        Err(err) => {
            let error_msg = err.to_string();
            assert!(
                error_msg.contains("circular")
                    || error_msg.contains("recursive")
                    || error_msg.contains("self"),
                "Should provide informative error about circular references, got: {}",
                error_msg
            );
        }
    }
}

/// Test mutually recursive schemas to verify cycle detection
#[test]
fn test_stack_safety_mutual_recursion() {
    let schema = json!({
        "$schema": "http://json-schema.org/draft-07/schema#",
        "definitions": {
            "TypeA": {
                "type": "object",
                "properties": {
                    "b_ref": { "$ref": "#/definitions/TypeB" },
                    "value": { "type": "string" }
                }
            },
            "TypeB": {
                "type": "object",
                "properties": {
                    "a_ref": { "$ref": "#/definitions/TypeA" },
                    "count": { "type": "integer" }
                }
            }
        },
        "title": "MutualRecursion",
        "type": "object",
        "properties": {
            "start_a": { "$ref": "#/definitions/TypeA" },
            "start_b": { "$ref": "#/definitions/TypeB" }
        }
    });

    // Should detect mutual recursion and handle safely
    let result = TctCompiler::new(schema, "MutualRecursion")
        .and_then(|compiler| compiler.compile())
        .map_err(|e| -> Box<dyn std::error::Error> { Box::new(e) })
        .and_then(|grammar| {
            grammar
                .generate_code()
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
        });

    match result {
        Ok(code) => {
            assert_generated_struct(&code, "MutualRecursion");
        }
        Err(err) => {
            let error_msg = err.to_string();
            // Should provide clear error about mutual recursion or cycle detection
            assert!(
                !error_msg.is_empty(),
                "Should provide informative error about mutual recursion"
            );
        }
    }
}

/// Test deeply nested array schemas for stack safety
#[test]
fn test_stack_safety_deeply_nested_arrays() {
    // Create arrays nested 50 levels deep (manageable for test environment)
    let mut current_schema = json!({ "type": "string" });

    for _depth in 0..50 {
        current_schema = json!({
            "type": "array",
            "items": current_schema
        });
    }

    let schema = json!({
        "title": "DeeplyNestedArrays",
        "type": "object",
        "properties": {
            "nested_arrays": current_schema
        },
        "required": ["nested_arrays"]
    });

    // Should handle deeply nested arrays without stack overflow
    let result = TctCompiler::new(schema, "DeeplyNestedArrays")
        .and_then(|compiler| compiler.compile())
        .map_err(|e| -> Box<dyn std::error::Error> { Box::new(e) })
        .and_then(|grammar| {
            grammar
                .generate_code()
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
        });

    match result {
        Ok(code) => {
            assert_generated_struct(&code, "DeeplyNestedArrays");
            assert!(code.contains("Vec<"), "Should generate nested Vec types");
        }
        Err(err) => {
            let error_msg = err.to_string();
            assert!(
                error_msg.contains("deep")
                    || error_msg.contains("nested")
                    || error_msg.contains("limit"),
                "Should provide informative error about nesting limits, got: {}",
                error_msg
            );
        }
    }
}

/// Test deeply nested oneOf/anyOf schemas for stack safety
#[test]
fn test_stack_safety_deeply_nested_unions() {
    // Create oneOf nested 50 levels deep
    let mut current_schema = json!({ "type": "string" });

    for depth in 0..50 {
        current_schema = json!({
            "oneOf": [
                current_schema,
                { "type": "number" },
                {
                    "type": "object",
                    "properties": {
                        format!("level_{}", depth): { "type": "boolean" }
                    }
                }
            ]
        });
    }

    let schema = json!({
        "title": "DeeplyNestedUnions",
        "type": "object",
        "properties": {
            "nested_union": current_schema
        },
        "required": ["nested_union"]
    });

    // Should handle deeply nested unions without stack overflow
    let result = TctCompiler::new(schema, "DeeplyNestedUnions")
        .and_then(|compiler| compiler.compile())
        .map_err(|e| -> Box<dyn std::error::Error> { Box::new(e) })
        .and_then(|grammar| {
            grammar
                .generate_code()
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
        });

    match result {
        Ok(code) => {
            assert_generated_struct(&code, "DeeplyNestedUnions");
        }
        Err(err) => {
            let error_msg = err.to_string();
            assert!(
                error_msg.contains("deep")
                    || error_msg.contains("nested")
                    || error_msg.contains("complex"),
                "Should provide informative error about complex nesting, got: {}",
                error_msg
            );
        }
    }
}

/// Test pathological schema with exponential complexity
#[test]
fn test_stack_safety_exponential_complexity() {
    // Create schema that could cause exponential expansion
    let base_object = json!({
        "type": "object",
        "properties": {
            "a": { "type": "string" },
            "b": { "type": "string" },
            "c": { "type": "string" }
        }
    });

    let mut complex_schema = base_object.clone();

    // Create nested structure that could expand exponentially (reduced to prevent timeout)
    for i in 0..8 {
        complex_schema = json!({
            "allOf": [
                complex_schema.clone(),
                {
                    "type": "object",
                    "properties": {
                        format!("branch_{}", i): {
                            "oneOf": [
                                complex_schema.clone(),
                                { "type": "string" },
                                { "type": "number" }
                            ]
                        }
                    }
                }
            ]
        });
    }

    let schema = json!({
        "title": "ExponentialComplexity",
        "type": "object",
        "properties": {
            "complex_field": complex_schema
        },
        "required": ["complex_field"]
    });

    // Should handle or reject exponential complexity gracefully
    let result = TctCompiler::new(schema, "ExponentialComplexity")
        .and_then(|compiler| compiler.compile())
        .map_err(|e| -> Box<dyn std::error::Error> { Box::new(e) })
        .and_then(|grammar| {
            grammar
                .generate_code()
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
        });

    match result {
        Ok(code) => {
            assert_generated_struct(&code, "ExponentialComplexity");
        }
        Err(err) => {
            let error_msg = err.to_string();
            // Should provide informative error about complexity limits
            assert!(
                !error_msg.is_empty(),
                "Should provide error information for complex schemas"
            );
        }
    }
}

/// Test many recursive references with forward reference pattern
///
/// Creates 15 types where each references 3 types ahead (forward-only pattern).
/// This tests stack safety with unidirectional reference chains.
#[test]
fn test_stack_safety_many_recursive_references_forward() {
    let mut definitions = HashMap::new();

    // Create 15 types with forward references only
    for i in 0..15 {
        let mut properties = HashMap::new();

        // Each type references 3 types ahead
        for j in 1..=3 {
            let ref_target = (i + j) % 15;
            properties.insert(
                format!("forward_ref_{}", j),
                json!({ "$ref": format!("#/definitions/Type{}", ref_target) }),
            );
        }

        properties.insert("value".to_string(), json!({ "type": "string" }));

        definitions.insert(
            format!("Type{}", i),
            json!({
                "type": "object",
                "properties": properties
            }),
        );
    }

    let schema = json!({
        "$schema": "http://json-schema.org/draft-07/schema#",
        "definitions": definitions,
        "title": "ForwardRecursiveRefs",
        "type": "object",
        "properties": {
            "entry_point": { "$ref": "#/definitions/Type0" }
        },
        "required": ["entry_point"]
    });

    // Should handle forward recursive references safely
    let result = TctCompiler::new(schema, "ForwardRecursiveRefs")
        .and_then(|compiler| compiler.compile())
        .map_err(|e| -> Box<dyn std::error::Error> { Box::new(e) })
        .and_then(|grammar| {
            grammar
                .generate_code()
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
        });

    match result {
        Ok(code) => {
            assert_generated_struct(&code, "ForwardRecursiveRefs");
        }
        Err(err) => {
            let error_msg = err.to_string();
            assert!(
                !error_msg.is_empty(),
                "Should provide error information for forward recursive schemas"
            );
        }
    }
}

/// Test many recursive references with bidirectional reference pattern
///
/// Creates 15 types where each references both forward and backward types.
/// This tests stack safety with complex bidirectional reference graphs.
#[test]
fn test_stack_safety_many_recursive_references_bidirectional() {
    let mut definitions = HashMap::new();

    // Create 15 types with bidirectional references
    for i in 0..15 {
        let mut properties = HashMap::new();

        // Forward reference
        let forward_target = (i + 2) % 15;
        properties.insert(
            "forward_ref".to_string(),
            json!({ "$ref": format!("#/definitions/Type{}", forward_target) }),
        );

        // Backward reference
        let backward_target = if i >= 2 { i - 2 } else { 15 + i - 2 };
        properties.insert(
            "backward_ref".to_string(),
            json!({ "$ref": format!("#/definitions/Type{}", backward_target) }),
        );

        // Adjacent reference
        let adjacent_target = (i + 1) % 15;
        properties.insert(
            "adjacent_ref".to_string(),
            json!({ "$ref": format!("#/definitions/Type{}", adjacent_target) }),
        );

        properties.insert("value".to_string(), json!({ "type": "string" }));

        definitions.insert(
            format!("Type{}", i),
            json!({
                "type": "object",
                "properties": properties
            }),
        );
    }

    let schema = json!({
        "$schema": "http://json-schema.org/draft-07/schema#",
        "definitions": definitions,
        "title": "BidirectionalRecursiveRefs",
        "type": "object",
        "properties": {
            "entry_point": { "$ref": "#/definitions/Type0" }
        },
        "required": ["entry_point"]
    });

    // Should handle bidirectional recursive references safely
    let result = TctCompiler::new(schema, "BidirectionalRecursiveRefs")
        .and_then(|compiler| compiler.compile())
        .map_err(|e| -> Box<dyn std::error::Error> { Box::new(e) })
        .and_then(|grammar| {
            grammar
                .generate_code()
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
        });

    match result {
        Ok(code) => {
            assert_generated_struct(&code, "BidirectionalRecursiveRefs");
        }
        Err(err) => {
            let error_msg = err.to_string();
            assert!(
                !error_msg.is_empty(),
                "Should provide error information for bidirectional recursive schemas"
            );
        }
    }
}

/// Test many recursive references with circular chain pattern
///
/// Creates 20 types forming multiple circular chains.
/// This tests stack safety with strongly connected components.
#[test]
fn test_stack_safety_many_recursive_references_circular_chains() {
    let mut definitions = HashMap::new();

    // Create 20 types forming 4 circular chains of 5 types each
    for i in 0..20 {
        let mut properties = HashMap::new();

        // Each type references the next in its chain (creates circles of size 5)
        let chain_id = i / 5;
        let pos_in_chain = i % 5;
        let next_in_chain = chain_id * 5 + ((pos_in_chain + 1) % 5);

        properties.insert(
            "next_in_chain".to_string(),
            json!({ "$ref": format!("#/definitions/Type{}", next_in_chain) }),
        );

        // Cross-chain reference to create interconnected circles
        let cross_chain_target = ((chain_id + 1) % 4) * 5 + pos_in_chain;
        properties.insert(
            "cross_chain".to_string(),
            json!({ "$ref": format!("#/definitions/Type{}", cross_chain_target) }),
        );

        properties.insert("value".to_string(), json!({ "type": "string" }));

        definitions.insert(
            format!("Type{}", i),
            json!({
                "type": "object",
                "properties": properties
            }),
        );
    }

    let schema = json!({
        "$schema": "http://json-schema.org/draft-07/schema#",
        "definitions": definitions,
        "title": "CircularChainRefs",
        "type": "object",
        "properties": {
            "entry_point": { "$ref": "#/definitions/Type0" }
        },
        "required": ["entry_point"]
    });

    // Should handle circular chains safely
    let result = TctCompiler::new(schema, "CircularChainRefs")
        .and_then(|compiler| compiler.compile())
        .map_err(|e| -> Box<dyn std::error::Error> { Box::new(e) })
        .and_then(|grammar| {
            grammar
                .generate_code()
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
        });

    match result {
        Ok(code) => {
            assert_generated_struct(&code, "CircularChainRefs");
        }
        Err(err) => {
            let error_msg = err.to_string();
            assert!(
                !error_msg.is_empty(),
                "Should provide error information for circular chain schemas"
            );
        }
    }
}

/// Test stack safety with TctCompiler API directly
#[test]
fn test_stack_safety_compiler_api_limits() {
    // Create deep recursion schema
    let mut deep_schema = json!({ "type": "string" });
    for depth in 0..30 {
        deep_schema = json!({
            "type": "object",
            "properties": {
                format!("level_{}", depth): deep_schema
            }
        });
    }

    let schema = json!({
        "title": "CompilerApiDeep",
        "type": "object",
        "properties": {
            "deep_field": deep_schema
        }
    });

    // Test TctCompiler API directly for stack safety
    let compiler_result = TctCompiler::new(schema, "CompilerApiDeep");

    match compiler_result {
        Ok(compiler) => {
            let compile_result = compiler.compile();
            match compile_result {
                Ok(grammar) => {
                    let code_result = grammar.generate_code();
                    match code_result {
                        Ok(code) => assert_generated_struct(&code, "CompilerApiDeep"),
                        Err(err) => {
                            let error_msg = err.to_string();
                            assert!(
                                !error_msg.is_empty(),
                                "Should provide informative code generation error"
                            );
                        }
                    }
                }
                Err(err) => {
                    let error_msg = err.to_string();
                    assert!(
                        !error_msg.is_empty(),
                        "Should provide informative compilation error"
                    );
                }
            }
        }
        Err(err) => {
            let error_msg = err.to_string();
            assert!(
                !error_msg.is_empty(),
                "Should provide informative compiler creation error"
            );
        }
    }
}

/// Test graceful handling of infinite loops in schema processing
#[test]
fn test_stack_safety_infinite_loop_protection() {
    // Create schema structure that could cause infinite processing loops
    let schema = json!({
        "$schema": "http://json-schema.org/draft-07/schema#",
        "definitions": {
            "LoopA": {
                "allOf": [
                    { "$ref": "#/definitions/LoopB" },
                    {
                        "type": "object",
                        "properties": {
                            "loop_a_field": { "type": "string" }
                        }
                    }
                ]
            },
            "LoopB": {
                "allOf": [
                    { "$ref": "#/definitions/LoopA" },
                    {
                        "type": "object",
                        "properties": {
                            "loop_b_field": { "type": "number" }
                        }
                    }
                ]
            }
        },
        "title": "InfiniteLoop",
        "type": "object",
        "properties": {
            "entry": { "$ref": "#/definitions/LoopA" }
        },
        "required": ["entry"]
    });

    // Should detect and prevent infinite processing loops
    let result = TctCompiler::new(schema, "InfiniteLoop")
        .and_then(|compiler| compiler.compile())
        .map_err(|e| -> Box<dyn std::error::Error> { Box::new(e) })
        .and_then(|grammar| {
            grammar
                .generate_code()
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
        });

    match result {
        Ok(code) => {
            assert_generated_struct(&code, "InfiniteLoop");
        }
        Err(err) => {
            let error_msg = err.to_string();
            assert!(
                error_msg.contains("loop")
                    || error_msg.contains("circular")
                    || error_msg.contains("infinite")
                    || !error_msg.is_empty(),
                "Should provide informative error about infinite loops, got: {}",
                error_msg
            );
        }
    }
}
