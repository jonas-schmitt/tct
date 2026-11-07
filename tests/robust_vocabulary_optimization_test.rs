//! Robust Vocabulary Optimization Tests
//!
//! This test suite validates vocabulary optimization behavior using flexible,
//! behavior-focused assertions that remain stable when implementation details change.

use serde_json::json;
use tct::{
    schema_analyzer::SchemaAnalyzer, vocabulary_allocator::VocabularyAllocator,
    vocabulary_encoder::VocabularyAwareEncoder,
};

#[test]
fn test_robust_vocabulary_vocabulary_optimization_behavior() {
    // Test schema with clearly defined optimizable and non-optimizable fields
    let schema = json!({
        "title": "OptimizedProduct",
        "type": "object",
        "properties": {
            "id": {"type": "string"},  // Not optimizable - unbounded
            "name": {"type": "string"}, // Not optimizable - unbounded
            "is_featured": {"type": "boolean"}, // Optimizable: boolean
            "status": {
                "type": "string",
                "enum": ["draft", "active", "discontinued"] // Optimizable: enum
            },
            "priority": {
                "type": "integer",
                "minimum": 1,
                "maximum": 10 // Optimizable: bounded integer
            },
            "category": {
                "type": "string",
                "enum": ["electronics", "clothing", "books", "home", "sports"] // Optimizable: enum
            }
        },
        "required": ["id", "name", "status"]
    });

    // Test schema analysis - focus on behavior, not exact counts
    let domains = SchemaAnalyzer::analyze_schema(&schema)
        .expect("Schema analysis should succeed for valid schema");

    // Behavior: Should find all constrained fields (boolean, enums, bounded integers)
    assert!(
        domains.len() >= 3,
        "Should find at least 3 optimizable domains (boolean + 2 enums minimum)"
    );
    assert!(
        domains.len() <= 6,
        "Should not find unreasonably many domains"
    );

    // Behavior: Each domain should have reasonable cardinality
    for domain in &domains {
        assert!(
            domain.cardinality >= 2,
            "Each domain should have at least 2 values"
        );
        assert!(
            domain.cardinality <= 50,
            "No domain should have unreasonably high cardinality for this schema"
        );
    }

    // Test vocabulary allocation
    let allocation = VocabularyAllocator::allocate(domains.clone());
    let total_values: usize = domains.iter().map(|d| d.cardinality).sum();

    // Behavior: Total values should match sum of individual domain cardinalities
    assert_eq!(
        allocation.total_vocabulary_size, total_values,
        "Dense allocation should use exactly the sum of domain cardinalities"
    );

    // Behavior: Should provide meaningful optimization (more than just boolean)
    assert!(
        total_values >= 10,
        "Should have meaningful vocabulary size for optimization"
    );

    // Test vocabulary encoder
    let encoder = VocabularyAwareEncoder::new(&schema)
        .expect("VocabularyAwareEncoder creation should succeed");
    let stats = encoder.optimization_stats();

    // Behavior-focused assertions instead of exact values
    assert!(
        stats.optimized_fields >= 3,
        "Should optimize at least boolean, enum, and bounded integer fields"
    );
    assert!(
        stats.optimized_fields <= 6,
        "Should not find unreasonably many optimizable fields"
    );
    assert!(
        stats.total_constrained_values >= 10,
        "Should have meaningful number of constrained values"
    );
    assert!(
        stats.distinct_structural_tokens_count > 0,
        "Should produce some vocabulary optimization"
    );
    assert_eq!(
        stats.distinct_structural_tokens_count, stats.total_constrained_values,
        "Dense allocation should use all constrained values"
    );

    println!("✅ Vocabulary optimization behavior verified:");
    println!(
        "   - Optimized {} fields (reasonable range)",
        stats.optimized_fields
    );
    println!(
        "   - Vocabulary size: {} tokens (dense allocation)",
        stats.distinct_structural_tokens_count
    );
    println!("   - {}", stats.summary());
}

#[test]
fn test_robust_vocabulary_field_encoding_behavior() {
    // Test field encoding behavior without hard-coding exact token values
    let schema = json!({
        "type": "object",
        "properties": {
            "status": {
                "type": "string",
                "enum": ["pending", "processing", "completed", "failed"]
            },
            "priority": {
                "type": "integer",
                "minimum": 1,
                "maximum": 5
            },
            "is_urgent": {"type": "boolean"}
        }
    });

    let encoder = VocabularyAwareEncoder::new(&schema)
        .expect("VocabularyAwareEncoder creation should succeed");

    // Behavior: Boolean field should encode to exactly 2 different values
    let urgent_false = encoder.encode_field("is_urgent", &json!(false)).unwrap();
    let urgent_true = encoder.encode_field("is_urgent", &json!(true)).unwrap();
    assert_ne!(
        urgent_false, urgent_true,
        "Boolean values should encode to different tokens"
    );

    // Behavior: Values should be reasonable (not negative, not extremely large)
    assert!(urgent_false < 100, "Token values should be reasonable");
    assert!(urgent_true < 100, "Token values should be reasonable");

    // Behavior: Enum field should encode all values to different tokens
    let status_values = ["pending", "processing", "completed", "failed"];
    let mut status_tokens = Vec::new();

    for status in status_values {
        let token = encoder.encode_field("status", &json!(status)).unwrap();
        assert!(token < 100, "Status token should be reasonable");
        assert!(
            !status_tokens.contains(&token),
            "Each status should encode to a unique token"
        );
        status_tokens.push(token);
    }

    assert_eq!(status_tokens.len(), 4, "Should have 4 unique status tokens");

    // Behavior: Integer field should encode all values in range to different tokens
    let mut priority_tokens = Vec::new();
    for priority in 1..=5 {
        let token = encoder.encode_field("priority", &json!(priority)).unwrap();
        assert!(token < 100, "Priority token should be reasonable");
        assert!(
            !priority_tokens.contains(&token),
            "Each priority should encode to a unique token"
        );
        priority_tokens.push(token);
    }

    assert_eq!(
        priority_tokens.len(),
        5,
        "Should have 5 unique priority tokens"
    );

    println!("✅ Field encoding behavior verified:");
    println!("   - Boolean: {} distinct tokens", 2);
    println!("   - Status enum: {} distinct tokens", status_tokens.len());
    println!(
        "   - Priority integer: {} distinct tokens",
        priority_tokens.len()
    );
}

#[test]
fn test_robust_vocabulary_optimization_effectiveness_behavior() {
    // Test that optimization provides meaningful benefits without hard-coding exact ratios
    let no_optimization_schema = json!({
        "type": "object",
        "properties": {
            "description": {"type": "string"}, // Not optimizable
            "metadata": {"type": "object", "properties": {}} // Not optimizable
        }
    });

    let optimization_schema = json!({
        "type": "object",
        "properties": {
            "status": {"enum": ["active", "inactive"]},
            "priority": {"type": "integer", "minimum": 1, "maximum": 10},
            "category": {"enum": ["A", "B", "C", "D", "E"]}
        }
    });

    let encoder1 = VocabularyAwareEncoder::new(&no_optimization_schema).unwrap();
    let stats1 = encoder1.optimization_stats();

    let encoder2 = VocabularyAwareEncoder::new(&optimization_schema).unwrap();
    let stats2 = encoder2.optimization_stats();

    // Behavior: No optimization case
    assert_eq!(
        stats1.optimized_fields, 0,
        "Schema with no constraints should have no optimized fields"
    );
    assert_eq!(
        stats1.distinct_structural_tokens_count, 0,
        "No optimization should result in zero optimized vocabulary"
    );

    // Behavior: Optimization case
    assert!(
        stats2.optimized_fields > 0,
        "Schema with constraints should have optimized fields"
    );
    assert!(
        stats2.distinct_structural_tokens_count > 0,
        "Optimization should result in non-zero vocabulary"
    );
    assert!(
        stats2.optimized_fields >= 2,
        "Should optimize at least boolean and enum fields"
    );
    assert!(
        stats2.total_constrained_values > stats2.optimized_fields,
        "Total values should exceed number of fields"
    );

    println!("✅ Optimization effectiveness verified:");
    println!(
        "   - No constraints: {} fields optimized",
        stats1.optimized_fields
    );
    println!(
        "   - With constraints: {} fields optimized",
        stats2.optimized_fields
    );
    println!("   - Vocabulary reduction demonstrates meaningful optimization");
}

#[test]
fn test_robust_vocabulary_vocabulary_allocation_consistency() {
    // Test that vocabulary allocation is consistent and deterministic
    let schema = json!({
        "type": "object",
        "properties": {
            "status": {"enum": ["pending", "active", "completed"]},
            "is_urgent": {"type": "boolean"}
        }
    });

    // Run encoder creation multiple times
    let encoder1 = VocabularyAwareEncoder::new(&schema).unwrap();
    let encoder2 = VocabularyAwareEncoder::new(&schema).unwrap();

    let stats1 = encoder1.optimization_stats();
    let stats2 = encoder2.optimization_stats();

    // Behavior: Results should be deterministic
    assert_eq!(
        stats1.optimized_fields, stats2.optimized_fields,
        "Optimization should be deterministic"
    );
    assert_eq!(
        stats1.total_constrained_values, stats2.total_constrained_values,
        "Total values should be deterministic"
    );
    assert_eq!(
        stats1.distinct_structural_tokens_count, stats2.distinct_structural_tokens_count,
        "Vocabulary size should be deterministic"
    );

    // Behavior: Encoding should be consistent
    let token1_1 = encoder1.encode_field("status", &json!("pending")).unwrap();
    let token2_1 = encoder2.encode_field("status", &json!("pending")).unwrap();
    assert_eq!(
        token1_1, token2_1,
        "Same field/value should encode to same token"
    );

    let token1_2 = encoder1.encode_field("is_urgent", &json!(true)).unwrap();
    let token2_2 = encoder2.encode_field("is_urgent", &json!(true)).unwrap();
    assert_eq!(
        token1_2, token2_2,
        "Same field/value should encode to same token"
    );

    println!("✅ Vocabulary allocation consistency verified");
}

#[test]
fn test_robust_vocabulary_edge_cases_behavior() {
    // Test behavior with edge cases without hard-coding exact expectations
    let single_field_schema = json!({
        "type": "object",
        "properties": {
            "flag": {"type": "boolean"}
        }
    });

    let large_enum_schema = json!({
        "type": "object",
        "properties": {
            "choice": {
                "enum": (0..50).map(|i| format!("option_{}", i)).collect::<Vec<_>>()
            }
        }
    });

    // Single field case
    let encoder_single = VocabularyAwareEncoder::new(&single_field_schema).unwrap();
    let stats_single = encoder_single.optimization_stats();

    assert_eq!(
        stats_single.optimized_fields, 1,
        "Single boolean field should be optimized"
    );
    assert_eq!(
        stats_single.total_constrained_values, 2,
        "Boolean should have exactly 2 values"
    );
    assert_eq!(
        stats_single.distinct_structural_tokens_count, 2,
        "Should allocate exactly 2 tokens"
    );

    // Large enum case
    let encoder_large = VocabularyAwareEncoder::new(&large_enum_schema).unwrap();
    let stats_large = encoder_large.optimization_stats();

    assert_eq!(
        stats_large.optimized_fields, 1,
        "Single enum field should be optimized"
    );
    assert_eq!(
        stats_large.total_constrained_values, 50,
        "Should count all enum values"
    );
    assert_eq!(
        stats_large.distinct_structural_tokens_count, 50,
        "Should allocate tokens for all values"
    );

    // Behavior: Large enum should still be manageable
    assert!(
        stats_large.distinct_structural_tokens_count <= 100,
        "Large enum should still be reasonably sized"
    );

    println!("✅ Edge cases behavior verified:");
    println!(
        "   - Single field: {} tokens allocated",
        stats_single.distinct_structural_tokens_count
    );
    println!(
        "   - Large enum: {} tokens allocated",
        stats_large.distinct_structural_tokens_count
    );
}
