//! Tests for synthetic schemas
//!
//! These tests ensure that our synthetic test schemas work correctly
//! with full round-trip compilation, encoding, and decoding.
//!
//! ⚠️ **IMPORTANT**: All generated-crate tests in this file are marked as `#[ignore]`.
//! They compile and run temporary Rust crates, so the first run is slower than ordinary
//! unit tests. Each schema/optimization-level pair is batched through one generated
//! binary so all fixtures share the same generated-crate build.
//!
//! **To run these tests locally**:
//! ```bash
//! # Run all synthetic schema tests (takes ~15-30 minutes)
//! cargo test --test synthetic_schema_test -- --ignored --nocapture
//!
//! # Run specific test
//! cargo test --test synthetic_schema_test test_simple_duplicate_round_trip -- --ignored --nocapture
//! ```
//!
//! **When to run**: Before releases, when modifying code generation, or when debugging schema compatibility issues.

use serde_json::Value;
use std::collections::HashSet;
use std::fs;
use tct::optimization_pass::OptimizationLevel;

mod round_trip_helpers;
use round_trip_helpers::{
    load_instances_for_schema, test_batch_round_trip_with_optimization_level,
};

#[test]
#[ignore]
fn test_simple_duplicate_round_trip() {
    test_synthetic_schema("simple_duplicate_test");
}

#[test]
#[ignore]
fn test_complex_dependencies_round_trip() {
    test_synthetic_schema("complex_dependencies");
}

#[test]
#[ignore]
fn test_scripts_config_round_trip() {
    test_synthetic_schema("scripts_config");
}

#[test]
#[ignore]
fn test_root_composition_round_trip() {
    test_synthetic_schema("root_composition_test");
}

#[test]
#[ignore]
fn test_simple_array_round_trip() {
    test_synthetic_schema("simple_array_test");
}

#[test]
#[ignore]
fn test_optional_field_enum_round_trip() {
    test_synthetic_schema("optional_field_enum");
}

#[test]
#[ignore]
fn test_primitive_enum_mismatch_round_trip() {
    test_synthetic_schema("primitive_enum_mismatch");
}

#[test]
#[ignore]
fn test_properties_ref_round_trip() {
    test_synthetic_schema("properties_ref");
}

#[test]
#[ignore]
fn test_nested_definitions_round_trip() {
    test_synthetic_schema("nested_definitions");
}

#[test]
#[ignore]
fn test_additional_properties_true_mixed_round_trip_preserves_o0_o3_semantics() {
    test_synthetic_schema_with_optimization_level(
        "additional_properties_true_mixed",
        OptimizationLevel::O0,
    );
    test_synthetic_schema_with_optimization_level(
        "additional_properties_true_mixed",
        OptimizationLevel::O3,
    );
}

#[test]
#[ignore]
fn test_allof_overlapping_properties_round_trip() {
    test_synthetic_schema_with_optimization_level(
        "allof_overlapping_properties",
        OptimizationLevel::O3,
    );
}

#[test]
#[ignore]
fn test_commitlint_rule_tuple_oneof_round_trip() {
    test_synthetic_schema_with_optimization_level(
        "commitlint_rule_tuple_oneof",
        OptimizationLevel::O3,
    );
}

fn test_synthetic_schema(schema_name: &str) {
    println!("\n🧪 Testing synthetic schema: {}", schema_name);

    // Load schema
    let schema_path = format!("schemas/synthetic/{}.json", schema_name);
    let schema_content = fs::read_to_string(&schema_path)
        .unwrap_or_else(|_| panic!("Failed to read schema: {}", schema_path));
    let schema: Value = serde_json::from_str(&schema_content)
        .unwrap_or_else(|_| panic!("Failed to parse schema: {}", schema_path));

    println!("✅ Schema loaded from {}", schema_path);

    // Load instances
    let mut instances = load_instances_for_schema(schema_name, "synthetic");
    if instances.is_empty() {
        println!("⚠️  No instances found for schema, skipping round-trip test");
        return;
    }
    instances.sort_by(|(left_name, _), (right_name, _)| left_name.cmp(right_name));

    let mut seen_instance_names = HashSet::new();
    for (instance_name, _) in &instances {
        assert!(
            seen_instance_names.insert(instance_name.clone()),
            "Duplicate synthetic round-trip instance id: {}",
            instance_name
        );
    }

    println!("✅ Found {} instances", instances.len());

    let summary = test_batch_round_trip_with_optimization_level(
        &schema,
        &instances,
        schema_name,
        OptimizationLevel::O0,
    )
    .unwrap_or_else(|e| panic!("Failed to run synthetic batch round-trip: {}", e));

    println!(
        "📊 Batch round-trip result: {:.1}% ({}/{})",
        summary.success_rate() * 100.0,
        summary.successful_instances,
        summary.total_instances
    );
    println!(
        "⏱️  schema={}ms codegen={}ms rust_build={}ms",
        summary.schema_compilation_time_ms,
        summary.code_generation_time_ms,
        summary.rust_compilation_time_ms
    );

    if !summary.failed_instances.is_empty() {
        panic!(
            "❌ {}/{} synthetic instances failed for {}:\n{}",
            summary.failed_instances.len(),
            instances.len(),
            schema_name,
            summary
                .failed_instances
                .iter()
                .map(|(instance_name, error)| format!("{}: {}", instance_name, error))
                .collect::<Vec<_>>()
                .join("\n")
        );
    }
}

fn test_synthetic_schema_with_optimization_level(
    schema_name: &str,
    optimization_level: OptimizationLevel,
) {
    println!(
        "\n🧪 Testing synthetic schema: {} at {:?}",
        schema_name, optimization_level
    );

    let schema_path = format!("schemas/synthetic/{}.json", schema_name);
    let schema_content = fs::read_to_string(&schema_path)
        .unwrap_or_else(|_| panic!("Failed to read schema: {}", schema_path));
    let schema: Value = serde_json::from_str(&schema_content)
        .unwrap_or_else(|_| panic!("Failed to parse schema: {}", schema_path));

    let mut instances = load_instances_for_schema(schema_name, "synthetic");
    assert!(
        !instances.is_empty(),
        "No instances found for schema: {}",
        schema_name
    );
    instances.sort_by(|(left_name, _), (right_name, _)| left_name.cmp(right_name));

    let summary = test_batch_round_trip_with_optimization_level(
        &schema,
        &instances,
        schema_name,
        optimization_level,
    )
    .unwrap_or_else(|e| {
        panic!(
            "Failed to run synthetic batch round-trip for {} at {:?}: {}",
            schema_name, optimization_level, e
        )
    });

    println!(
        "📊 Batch round-trip result at {:?}: {:.1}% ({}/{})",
        optimization_level,
        summary.success_rate() * 100.0,
        summary.successful_instances,
        summary.total_instances
    );

    if !summary.failed_instances.is_empty() {
        panic!(
            "❌ {}/{} synthetic instances failed for {} at {:?}:\n{}",
            summary.failed_instances.len(),
            instances.len(),
            schema_name,
            optimization_level,
            summary
                .failed_instances
                .iter()
                .map(|(instance_name, error)| format!("{}: {}", instance_name, error))
                .collect::<Vec<_>>()
                .join("\n")
        );
    }
}

#[test]
fn test_synthetic_instance_loader_discovers_nested_schema_fixtures() {
    let decoder_instances =
        load_instances_for_schema("decoder/string_vs_object_union", "synthetic");
    let decoder_names = decoder_instances
        .iter()
        .map(|(name, _)| name.as_str())
        .collect::<HashSet<_>>();

    assert_eq!(decoder_instances.len(), 2);
    assert!(decoder_names.contains("decoder/string_vs_object_union/object_version"));
    assert!(decoder_names.contains("decoder/string_vs_object_union/string_version"));

    let encoding_instances =
        load_instances_for_schema("encoding_patterns/fixed_size_types", "synthetic");
    let encoding_names = encoding_instances
        .iter()
        .map(|(name, _)| name.as_str())
        .collect::<HashSet<_>>();

    assert_eq!(encoding_instances.len(), 2);
    assert!(encoding_names.contains("synthetic/encoding_patterns/fixed_size_types/real/test1"));
    assert!(encoding_names.contains("synthetic/encoding_patterns/fixed_size_types/real/test2"));
    assert!(!encoding_names.contains("metadata"));
}

// Test specifically for exports/imports which has recursive references
#[test]
#[ignore] // Subprocess compilation timeout (~60s) - run manually with: cargo test test_exports_imports_round_trip -- --ignored --nocapture
fn test_exports_imports_round_trip() {
    test_synthetic_schema("exports_imports");
}

// Encoding pattern tests (bounded, length-prefixed, tagged, fixed-size)
#[test]
#[ignore]
fn test_encoding_patterns_fixed_size_types() {
    test_synthetic_schema("encoding_patterns/fixed_size_types");
}

#[test]
#[ignore]
fn test_encoding_patterns_length_prefixed_types() {
    test_synthetic_schema("encoding_patterns/length_prefixed_types");
}

#[test]
#[ignore]
fn test_encoding_patterns_bounded_types() {
    test_synthetic_schema("encoding_patterns/bounded_types");
}

#[test]
#[ignore]
fn test_encoding_patterns_tagged_types() {
    test_synthetic_schema("encoding_patterns/tagged_types");
}

#[test]
#[ignore]
fn test_simple_untagged_enum() {
    test_synthetic_schema("simple_untagged_enum");
}

#[test]
#[ignore]
fn test_distinguishable_untagged_enum() {
    test_synthetic_schema("distinguishable_untagged_enum");
}

#[test]
#[ignore]
fn test_tuple_array() {
    test_synthetic_schema("tuple_array");
}

#[test]
#[ignore]
fn test_object_with_tuple_array_values() {
    test_synthetic_schema("object_with_tuple_array_values");
}

#[test]
#[ignore]
fn test_oneof_array_string() {
    test_synthetic_schema("oneof_array_string");
}

#[test]
#[ignore]
fn test_multiple_oneof_patterns() {
    test_synthetic_schema("multiple_oneof_patterns");
}

#[test]
#[ignore]
fn test_tuple_struct_in_enum() {
    test_synthetic_schema("tuple_struct_in_enum");
}

#[test]
#[ignore]
fn test_array_len_debug() {
    test_synthetic_schema("array_len_debug");
}

#[test]
#[ignore]
fn test_primitive_in_definition() {
    test_synthetic_schema("primitive_in_definition");
}

#[test]
#[ignore]
fn test_round_trip_precision() {
    test_synthetic_schema("round_trip_precision");
}

#[test]
#[ignore]
fn test_minimal_object() {
    test_synthetic_schema("minimal_object");
}

#[test]
#[ignore]
fn test_minimal_object_fix_test() {
    test_synthetic_schema("minimal_object_fix_test");
}

// Phase 2: Union Pattern Testing Schemas
#[test]
#[ignore]
fn test_union_variant_ambiguity() {
    test_synthetic_schema("union_variant_ambiguity");
}

#[test]
#[ignore]
fn test_union_nested() {
    test_synthetic_schema("union_nested");
}

#[test]
#[ignore]
fn test_union_optional_fields() {
    test_synthetic_schema("union_optional_fields");
}

#[test]
#[ignore]
fn test_union_mixed_types() {
    test_synthetic_schema("union_mixed_types");
}

#[test]
#[ignore]
fn test_union_field_ordering() {
    test_synthetic_schema("union_field_ordering");
}

#[test]
#[ignore]
fn test_github_workflow_exact() {
    test_synthetic_schema("github_workflow_exact");
}

// Decoder test schemas - These should FAIL with current system (TDD Red phase)
#[test]
#[ignore]
fn test_decoder_string_vs_object_union() {
    test_synthetic_schema("decoder/string_vs_object_union");
}

#[test]
#[ignore]
fn test_decoder_primitive_confusion() {
    test_synthetic_schema("decoder/primitive_confusion");
}

#[test]
#[ignore]
fn test_decoder_array_vs_object() {
    test_synthetic_schema("decoder/array_vs_object");
}

#[test]
#[ignore]
fn test_decoder_optional_struct_false_positive() {
    test_synthetic_schema("decoder/optional_struct_false_positive");
}

// Phase 5B: Enhanced synthetic schemas for newly identified patterns
#[test]
#[ignore]
fn test_object_vs_null() {
    test_synthetic_schema("object_vs_null");
}

#[test]
#[ignore]
fn test_serde_json_value_specificity() {
    test_synthetic_schema("serde_json_value_specificity");
}

#[test]
#[ignore]
fn test_multi_primitive_union() {
    test_synthetic_schema("multi_primitive_union");
}

// Test null type support
#[test]
#[ignore]
fn test_null_variant() {
    test_synthetic_schema("null_variant");
}

#[test]
#[ignore]
fn test_env_values() {
    test_synthetic_schema("env_values");
}

// Test format-optimized string encoding (O3 optimization)
#[test]
#[ignore]
fn test_format_optimization() {
    test_synthetic_schema("format_optimization_test");
}

// Test bit-packed optional-field presence encoding
#[test]
#[ignore]
fn test_bitpacked_optionals() {
    test_synthetic_schema("bitpacked_optionals");
}
