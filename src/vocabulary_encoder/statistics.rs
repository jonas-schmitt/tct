//! Vocabulary Optimization Statistics
//!
//! This module provides detailed statistics and analysis for vocabulary
//! optimization effectiveness, including reduction percentages, token savings,
//! and domain-specific optimization breakdowns.

use rustc_hash::FxHashMap;

/// Comprehensive statistics for vocabulary optimization effectiveness
#[derive(Debug, Clone, PartialEq)]
pub struct VocabularyOptimizationStats {
    /// Number of fields that were optimized
    pub optimized_fields: usize,
    /// Total number of constrained values across all fields
    pub total_constrained_values: usize,
    /// Vocabulary size with naive encoding
    pub naive_vocabulary_size: usize,
    /// Vocabulary size with optimal encoding (COUNT of distinct structural tokens)
    pub distinct_structural_tokens_count: usize,
    /// Number of tokens saved
    pub tokens_saved: usize,
    /// Percentage reduction in vocabulary size
    pub reduction_percentage: f64,

    // Pattern optimization statistics (computed during encoding)
    /// Number of pattern domains optimized
    pub pattern_domains: usize,
    /// Vocabulary reduction from pattern optimization
    pub pattern_vocabulary_reduction: f64,
    /// Total tokens saved from pattern optimization
    pub pattern_tokens_saved: usize,
    /// Tokens allocated for pattern optimization
    pub pattern_tokens_allocated: usize,
    /// Total tokens allocated across all domains (MAX token ID + 1, used for BPE base_vocab)
    pub max_structural_token_id_plus_one: usize,
    /// Estimated tokens without optimization
    pub estimated_unoptimized_tokens: usize,
    /// Total vocabulary reduction across all optimizations
    pub total_vocabulary_reduction: f64,
    /// Number of enum domains optimized
    pub enum_domains: usize,
    /// Number of bounded integer domains optimized
    pub bounded_integer_domains: usize,
    /// Breakdown by optimization type
    pub optimization_breakdown: FxHashMap<String, usize>,
}

impl VocabularyOptimizationStats {
    /// Display optimization summary
    pub fn summary(&self) -> String {
        format!(
            "Vocabulary Optimization: {} → {} tokens ({:.1}% reduction)\n\
             - Optimized fields: {}\n\
             - Total constrained values: {}\n\
             - Tokens saved: {}",
            self.naive_vocabulary_size,
            self.distinct_structural_tokens_count,
            self.reduction_percentage,
            self.optimized_fields,
            self.total_constrained_values,
            self.tokens_saved
        )
    }

    /// Get detailed breakdown by optimization type
    pub fn detailed_breakdown(&self) -> String {
        let mut breakdown = String::new();
        breakdown.push_str(&self.summary());
        breakdown.push_str("\n\nOptimization Breakdown:\n");

        for (opt_type, count) in &self.optimization_breakdown {
            breakdown.push_str(&format!("  - {}: {} domains\n", opt_type, count));
        }

        if self.pattern_domains > 0 {
            breakdown.push_str(&format!(
                "  - Pattern domains: {}\n  - Pattern tokens saved: {}\n",
                self.pattern_domains, self.pattern_tokens_saved
            ));
        }

        breakdown
    }

    /// Calculate the efficiency score (0.0 - 1.0)
    pub fn efficiency_score(&self) -> f64 {
        if self.naive_vocabulary_size == 0 {
            return 0.0;
        }
        self.tokens_saved as f64 / self.naive_vocabulary_size as f64
    }

    /// Check if optimization was successful (achieved meaningful reduction)
    pub fn is_optimization_successful(&self) -> bool {
        self.reduction_percentage >= 10.0 // At least 10% reduction
    }
}

impl Default for VocabularyOptimizationStats {
    fn default() -> Self {
        Self {
            optimized_fields: 0,
            total_constrained_values: 0,
            naive_vocabulary_size: 0,
            distinct_structural_tokens_count: 0,
            tokens_saved: 0,
            reduction_percentage: 0.0,
            pattern_domains: 0,
            pattern_vocabulary_reduction: 0.0,
            pattern_tokens_saved: 0,
            pattern_tokens_allocated: 0,
            max_structural_token_id_plus_one: 0,
            estimated_unoptimized_tokens: 0,
            total_vocabulary_reduction: 0.0,
            enum_domains: 0,
            bounded_integer_domains: 0,
            optimization_breakdown: FxHashMap::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stats_summary() {
        let stats = VocabularyOptimizationStats {
            optimized_fields: 3,
            total_constrained_values: 100,
            naive_vocabulary_size: 200,
            distinct_structural_tokens_count: 150,
            tokens_saved: 50,
            reduction_percentage: 25.0,
            ..Default::default()
        };

        let summary = stats.summary();
        assert!(summary.contains("200 → 150 tokens (25.0% reduction)"));
        assert!(summary.contains("Optimized fields: 3"));
        assert!(summary.contains("Tokens saved: 50"));
    }

    #[test]
    fn test_efficiency_score() {
        let stats = VocabularyOptimizationStats {
            naive_vocabulary_size: 100,
            tokens_saved: 25,
            ..Default::default()
        };

        assert_eq!(stats.efficiency_score(), 0.25);
    }

    #[test]
    fn test_optimization_successful() {
        let successful_stats = VocabularyOptimizationStats {
            reduction_percentage: 15.0,
            ..Default::default()
        };
        assert!(successful_stats.is_optimization_successful());

        let unsuccessful_stats = VocabularyOptimizationStats {
            reduction_percentage: 5.0,
            ..Default::default()
        };
        assert!(!unsuccessful_stats.is_optimization_successful());
    }

    // ========== Detailed Breakdown Tests ==========

    #[test]
    fn test_detailed_breakdown_basic() {
        let mut breakdown = FxHashMap::default();
        breakdown.insert("enum".to_string(), 2);
        breakdown.insert("bounded_integer".to_string(), 1);

        let stats = VocabularyOptimizationStats {
            optimized_fields: 3,
            total_constrained_values: 50,
            naive_vocabulary_size: 100,
            distinct_structural_tokens_count: 75,
            tokens_saved: 25,
            reduction_percentage: 25.0,
            optimization_breakdown: breakdown,
            ..Default::default()
        };

        let detailed = stats.detailed_breakdown();
        assert!(detailed.contains("Optimization Breakdown:"));
        assert!(detailed.contains("enum: 2 domains"));
        assert!(detailed.contains("bounded_integer: 1 domains"));
    }

    #[test]
    fn test_detailed_breakdown_with_patterns() {
        let stats = VocabularyOptimizationStats {
            optimized_fields: 2,
            total_constrained_values: 30,
            naive_vocabulary_size: 80,
            distinct_structural_tokens_count: 60,
            tokens_saved: 20,
            reduction_percentage: 25.0,
            pattern_domains: 3,
            pattern_tokens_saved: 15,
            ..Default::default()
        };

        let detailed = stats.detailed_breakdown();
        assert!(detailed.contains("Pattern domains: 3"));
        assert!(detailed.contains("Pattern tokens saved: 15"));
    }

    #[test]
    fn test_detailed_breakdown_no_patterns() {
        let stats = VocabularyOptimizationStats {
            optimized_fields: 1,
            total_constrained_values: 10,
            naive_vocabulary_size: 50,
            distinct_structural_tokens_count: 40,
            tokens_saved: 10,
            reduction_percentage: 20.0,
            pattern_domains: 0,
            ..Default::default()
        };

        let detailed = stats.detailed_breakdown();
        assert!(!detailed.contains("Pattern domains:"));
        assert!(!detailed.contains("Pattern tokens saved:"));
    }

    // ========== Edge Case Tests ==========

    #[test]
    fn test_efficiency_score_zero_naive_vocabulary() {
        let stats = VocabularyOptimizationStats {
            naive_vocabulary_size: 0,
            tokens_saved: 0,
            ..Default::default()
        };

        assert_eq!(stats.efficiency_score(), 0.0);
    }

    #[test]
    fn test_efficiency_score_perfect_optimization() {
        let stats = VocabularyOptimizationStats {
            naive_vocabulary_size: 100,
            tokens_saved: 100,
            ..Default::default()
        };

        assert_eq!(stats.efficiency_score(), 1.0);
    }

    #[test]
    fn test_optimization_successful_boundary_10_percent() {
        let boundary_stats = VocabularyOptimizationStats {
            reduction_percentage: 10.0,
            ..Default::default()
        };
        assert!(boundary_stats.is_optimization_successful());
    }

    #[test]
    fn test_optimization_successful_just_below_threshold() {
        let below_threshold = VocabularyOptimizationStats {
            reduction_percentage: 9.9,
            ..Default::default()
        };
        assert!(!below_threshold.is_optimization_successful());
    }

    // ========== Full Constructor Tests ==========

    #[test]
    fn test_default_constructor() {
        let stats = VocabularyOptimizationStats::default();

        assert_eq!(stats.optimized_fields, 0);
        assert_eq!(stats.total_constrained_values, 0);
        assert_eq!(stats.naive_vocabulary_size, 0);
        assert_eq!(stats.distinct_structural_tokens_count, 0);
        assert_eq!(stats.tokens_saved, 0);
        assert_eq!(stats.reduction_percentage, 0.0);
        assert_eq!(stats.pattern_domains, 0);
        assert_eq!(stats.pattern_vocabulary_reduction, 0.0);
        assert_eq!(stats.pattern_tokens_saved, 0);
        assert_eq!(stats.pattern_tokens_allocated, 0);
        assert_eq!(stats.max_structural_token_id_plus_one, 0);
        assert_eq!(stats.estimated_unoptimized_tokens, 0);
        assert_eq!(stats.total_vocabulary_reduction, 0.0);
        assert_eq!(stats.enum_domains, 0);
        assert_eq!(stats.bounded_integer_domains, 0);
        assert!(stats.optimization_breakdown.is_empty());
    }

    #[test]
    fn test_full_stats_all_fields() {
        let mut breakdown = FxHashMap::default();
        breakdown.insert("enum".to_string(), 5);
        breakdown.insert("bounded_integer".to_string(), 3);
        breakdown.insert("boolean".to_string(), 2);

        let stats = VocabularyOptimizationStats {
            optimized_fields: 10,
            total_constrained_values: 200,
            naive_vocabulary_size: 500,
            distinct_structural_tokens_count: 300,
            tokens_saved: 200,
            reduction_percentage: 40.0,
            pattern_domains: 4,
            pattern_vocabulary_reduction: 15.5,
            pattern_tokens_saved: 50,
            pattern_tokens_allocated: 100,
            max_structural_token_id_plus_one: 350,
            estimated_unoptimized_tokens: 550,
            total_vocabulary_reduction: 45.5,
            enum_domains: 5,
            bounded_integer_domains: 3,
            optimization_breakdown: breakdown,
        };

        assert_eq!(stats.optimized_fields, 10);
        assert_eq!(stats.total_constrained_values, 200);
        assert_eq!(stats.naive_vocabulary_size, 500);
        assert_eq!(stats.distinct_structural_tokens_count, 300);
        assert_eq!(stats.tokens_saved, 200);
        assert_eq!(stats.reduction_percentage, 40.0);
        assert_eq!(stats.pattern_domains, 4);
        assert_eq!(stats.pattern_vocabulary_reduction, 15.5);
        assert_eq!(stats.pattern_tokens_saved, 50);
        assert_eq!(stats.pattern_tokens_allocated, 100);
        assert_eq!(stats.max_structural_token_id_plus_one, 350);
        assert_eq!(stats.estimated_unoptimized_tokens, 550);
        assert_eq!(stats.total_vocabulary_reduction, 45.5);
        assert_eq!(stats.enum_domains, 5);
        assert_eq!(stats.bounded_integer_domains, 3);
        assert_eq!(stats.optimization_breakdown.len(), 3);
        assert_eq!(stats.optimization_breakdown.get("enum"), Some(&5));
        assert_eq!(
            stats.optimization_breakdown.get("bounded_integer"),
            Some(&3)
        );
        assert_eq!(stats.optimization_breakdown.get("boolean"), Some(&2));
    }

    // ========== Clone and PartialEq Tests ==========

    #[test]
    fn test_clone_and_partial_eq() {
        let stats = VocabularyOptimizationStats {
            optimized_fields: 5,
            total_constrained_values: 100,
            naive_vocabulary_size: 200,
            distinct_structural_tokens_count: 150,
            tokens_saved: 50,
            reduction_percentage: 25.0,
            ..Default::default()
        };

        let cloned = stats.clone();
        assert_eq!(stats, cloned);
        assert_eq!(cloned.optimized_fields, 5);
        assert_eq!(cloned.reduction_percentage, 25.0);
    }
}
