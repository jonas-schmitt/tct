//! Optimization testing utilities for behavior-focused assertions
//!
//! This module provides specialized assertion functions for testing optimization
//! behavior, vocabulary statistics, and code generation patterns. These assertions
//! focus on behavior rather than implementation details for robust testing.

use crate::vocabulary_encoder::VocabularyOptimizationStats;

/// Assert that optimization stats are within reasonable bounds for a given schema complexity
pub fn assert_reasonable_optimization_stats(
    stats: &VocabularyOptimizationStats,
    expected_field_range: (usize, usize),
    min_total_values: usize,
    description: &str,
) {
    let (min_fields, max_fields) = expected_field_range;

    assert!(
        stats.optimized_fields >= min_fields,
        "{}: Expected at least {} optimized fields, got {}. This suggests the optimization detection logic may have changed.",
        description,
        min_fields,
        stats.optimized_fields
    );

    assert!(
        stats.optimized_fields <= max_fields,
        "{}: Expected at most {} optimized fields, got {}. This might indicate new optimization detection or schema changes.",
        description,
        max_fields,
        stats.optimized_fields
    );

    assert!(
        stats.total_constrained_values >= min_total_values,
        "{}: Expected at least {} total constrained values, got {}",
        description,
        min_total_values,
        stats.total_constrained_values
    );

    // Vocabulary reduction should be meaningful if optimization occurred
    if stats.optimized_fields > 0 {
        assert!(
            stats.reduction_percentage >= 0.0,
            "{}: Vocabulary reduction should be non-negative, got {}%",
            description,
            stats.reduction_percentage
        );
    }
}

/// Assert that the optimization provides meaningful benefit (>5% vocabulary reduction)
pub fn assert_meaningful_optimization(stats: &VocabularyOptimizationStats, description: &str) {
    if stats.optimized_fields > 0 {
        assert!(
            stats.reduction_percentage >= 5.0,
            "{}: Expected meaningful optimization (>5%), got {}% vocabulary reduction",
            description,
            stats.reduction_percentage
        );
    }
}

/// Assert that optimization results are deterministic across multiple runs
pub fn assert_optimization_determinism(
    stats1: &VocabularyOptimizationStats,
    stats2: &VocabularyOptimizationStats,
    description: &str,
) {
    assert_eq!(
        stats1.optimized_fields, stats2.optimized_fields,
        "{}: Optimized fields count should be deterministic",
        description
    );
    assert_eq!(
        stats1.total_constrained_values, stats2.total_constrained_values,
        "{}: Total constrained values should be deterministic",
        description
    );
    assert_eq!(
        stats1.tokens_saved, stats2.tokens_saved,
        "{}: Token allocation should be deterministic",
        description
    );
    assert!(
        (stats1.reduction_percentage - stats2.reduction_percentage).abs() < 0.01,
        "{}: Vocabulary reduction should be deterministic (within 0.01%), got {}% vs {}%",
        description,
        stats1.reduction_percentage,
        stats2.reduction_percentage
    );
}

/// Assert that generated code contains expected optimization patterns
pub fn assert_contains_optimization_patterns(
    code: &str,
    expected_patterns: &[&str],
    description: &str,
) {
    for pattern in expected_patterns {
        assert!(
            code.contains(pattern),
            "{}: Generated code should contain pattern '{}'\nGenerated code:\n{}",
            description,
            pattern,
            code
        );
    }
}

/// Check for specific optimization artifacts in generated code
pub fn assert_contains_vocabulary_optimization(code: &str, description: &str) {
    // Look for vocabulary-related optimization patterns
    let vocab_patterns = ["vocabulary", "enum_values", "decode", "modulo"];

    let found_patterns: Vec<&str> = vocab_patterns
        .iter()
        .filter(|&&pattern| code.contains(pattern))
        .copied()
        .collect();

    assert!(
        !found_patterns.is_empty(),
        "{}: Expected vocabulary optimization patterns in generated code, but found none. Generated code:\n{}",
        description,
        code
    );
}

/// Assert that different optimization levels produce different results
pub fn assert_different_optimization_results(
    code_basic: &str,
    code_optimized: &str,
    description: &str,
) {
    assert_ne!(
        code_basic, code_optimized,
        "{}: Different optimization levels should produce different code",
        description
    );
}
