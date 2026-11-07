//! Recommendation records derived from vocabulary analysis.

use super::{
    CompressionStrategy, DataType, OptimizationRecommendations, VocabularyAnalysis,
    VocabularyAnalyzer, VocabularyEfficiency,
};
use rustc_hash::FxHashMap;

impl VocabularyAnalyzer {
    /// Generate optimization recommendations based on analysis results
    pub fn generate_recommendations(
        &self,
        analysis: &VocabularyAnalysis,
        target_vocab_sizes: &[usize],
    ) -> OptimizationRecommendations {
        let mut recommendations = OptimizationRecommendations {
            optimal_vocab_size: None,
            compression_strategies: Vec::new(),
            efficiency_analysis: Vec::new(),
            warnings: Vec::new(),
        };

        // Analyze vocabulary efficiency across different sizes
        for &vocab_size in target_vocab_sizes {
            let efficiency = self.calculate_vocabulary_efficiency(analysis, vocab_size);
            recommendations.efficiency_analysis.push(efficiency);
        }

        // Find optimal vocabulary size
        recommendations.optimal_vocab_size =
            self.find_optimal_vocabulary_size(&recommendations.efficiency_analysis);

        // Generate compression strategies
        recommendations.compression_strategies = self.generate_compression_strategies(analysis);

        // Generate warnings for potential issues
        recommendations.warnings = self.generate_warnings(analysis);

        recommendations
    }

    /// Calculate vocabulary efficiency for a given vocabulary size
    pub(super) fn calculate_vocabulary_efficiency(
        &self,
        analysis: &VocabularyAnalysis,
        vocab_size: usize,
    ) -> VocabularyEfficiency {
        let max_required = analysis
            .position_analysis
            .values()
            .map(|pos| pos.vocabulary_size)
            .max()
            .unwrap_or(0);

        let total_positions = analysis.position_analysis.len();
        let constrained_positions = analysis
            .position_analysis
            .values()
            .filter(|pos| pos.vocabulary_size > vocab_size)
            .count();

        let compression_ratio = if max_required > 0 {
            (max_required as f64 - vocab_size as f64) / max_required as f64
        } else {
            0.0
        };

        let efficiency_score = if total_positions > 0 {
            ((total_positions - constrained_positions) as f64 / total_positions as f64) * 100.0
        } else {
            100.0
        };

        VocabularyEfficiency {
            vocab_size,
            max_required_vocab: max_required,
            constrained_positions,
            total_positions,
            compression_ratio,
            efficiency_score,
            multi_token_encoding_needed: constrained_positions > 0,
        }
    }

    /// Find the optimal vocabulary size based on efficiency analysis
    pub(super) fn find_optimal_vocabulary_size(
        &self,
        efficiency_analysis: &[VocabularyEfficiency],
    ) -> Option<usize> {
        efficiency_analysis
            .iter()
            .filter(|eff| eff.efficiency_score >= 80.0) // At least 80% efficiency
            .min_by_key(|eff| eff.vocab_size)
            .map(|eff| eff.vocab_size)
            .or_else(|| {
                // If no size meets 80% efficiency, find the best balance
                efficiency_analysis
                    .iter()
                    .max_by(|a, b| {
                        let score_a = a.efficiency_score - (a.vocab_size as f64 / 1000.0);
                        let score_b = b.efficiency_score - (b.vocab_size as f64 / 1000.0);
                        score_a
                            .partial_cmp(&score_b)
                            .unwrap_or(std::cmp::Ordering::Equal)
                    })
                    .map(|eff| eff.vocab_size)
            })
    }

    /// Generate compression strategies based on analysis
    pub(super) fn generate_compression_strategies(
        &self,
        analysis: &VocabularyAnalysis,
    ) -> Vec<CompressionStrategy> {
        let mut strategies = Vec::new();

        // Analyze dominant data types
        let mut type_counts = FxHashMap::default();
        for position in analysis.position_analysis.values() {
            self.count_data_types(&position.data_type, &mut type_counts);
        }

        // Recommend strategies based on data type distribution
        for (data_type, count) in type_counts {
            if count >= 1 {
                // If type appears at least once (simple threshold for now)
                let strategy = match data_type {
                    DataType::Enum { .. } => CompressionStrategy {
                        strategy_type: "enum_optimization".to_string(),
                        description:
                            "Use dense token allocation for enum values with index-based encoding"
                                .to_string(),
                        expected_savings: "30-70% vocabulary reduction".to_string(),
                        complexity: "Low".to_string(),
                    },
                    DataType::Integer { .. } => CompressionStrategy {
                        strategy_type: "bounded_integer_optimization".to_string(),
                        description:
                            "Use range-based encoding for bounded integers with modulo operations"
                                .to_string(),
                        expected_savings: "20-50% vocabulary reduction".to_string(),
                        complexity: "Medium".to_string(),
                    },
                    DataType::String { .. } => CompressionStrategy {
                        strategy_type: "string_pattern_optimization".to_string(),
                        description:
                            "Use pattern-based encoding for strings with finite vocabularies"
                                .to_string(),
                        expected_savings: "10-40% vocabulary reduction".to_string(),
                        complexity: "High".to_string(),
                    },
                    _ => continue,
                };
                strategies.push(strategy);
            }
        }

        // Add structural optimization for complex schemas
        if analysis.complexity_metrics.max_depth > 3 {
            strategies.push(CompressionStrategy {
                strategy_type: "structural_optimization".to_string(),
                description: "Use hierarchical encoding for deeply nested structures".to_string(),
                expected_savings: "15-35% vocabulary reduction".to_string(),
                complexity: "High".to_string(),
            });
        }

        strategies
    }

    /// Generate warnings for potential issues
    pub(super) fn generate_warnings(&self, analysis: &VocabularyAnalysis) -> Vec<String> {
        let mut warnings = Vec::new();

        warnings.extend(self.reference_warnings.iter().cloned());

        // Warn about extremely large vocabulary requirements
        let max_vocab = analysis
            .position_analysis
            .values()
            .map(|pos| pos.vocabulary_size)
            .max()
            .unwrap_or(0);

        if max_vocab > 10000 {
            warnings.push(format!(
                "Very large vocabulary requirement detected ({}). Consider using multi-token encoding.",
                max_vocab
            ));
        }

        // Warn about deep nesting
        if analysis.complexity_metrics.max_depth > 10 {
            warnings.push(format!(
                "Deep nesting detected (depth {}). May cause stack overflow in recursive processing.",
                analysis.complexity_metrics.max_depth
            ));
        }

        // Warn about high complexity
        if analysis.complexity_metrics.complexity_score > 0.8 {
            warnings.push(
                "High schema complexity detected. Consider simplifying schema structure for better performance.".to_string()
            );
        }

        // Warn about large average vocabulary size
        if analysis.complexity_metrics.average_vocabulary_size > 1000.0 {
            warnings.push(format!(
                "Large average vocabulary size ({:.0}). Consider optimization strategies.",
                analysis.complexity_metrics.average_vocabulary_size
            ));
        }

        warnings
    }

    /// Recursively count data types, including those nested within objects
    #[allow(clippy::only_used_in_recursion)]
    pub(super) fn count_data_types(
        &self,
        data_type: &DataType,
        type_counts: &mut FxHashMap<DataType, usize>,
    ) {
        match data_type {
            DataType::Object { properties } => {
                // Count the object itself
                *type_counts
                    .entry(DataType::Object {
                        properties: FxHashMap::default(),
                    })
                    .or_insert(0) += 1;
                // Count all properties within the object
                for property_type in properties.values() {
                    self.count_data_types(property_type, type_counts);
                }
            }
            DataType::Array { item_type } => {
                *type_counts
                    .entry(DataType::Array {
                        item_type: Box::new(DataType::Unknown),
                    })
                    .or_insert(0) += 1;
                self.count_data_types(item_type, type_counts);
            }
            DataType::Union { variants } => {
                *type_counts
                    .entry(DataType::Union {
                        variants: Vec::new(),
                    })
                    .or_insert(0) += 1;
                for variant in variants {
                    self.count_data_types(variant, type_counts);
                }
            }
            other => {
                // Create a simplified version for counting (removing specific details)
                let count_key = match other {
                    DataType::String { .. } => DataType::String {
                        min_length: None,
                        max_length: None,
                    },
                    DataType::Integer { .. } => DataType::Integer {
                        minimum: None,
                        maximum: None,
                    },
                    DataType::Enum { .. } => DataType::Enum { values: Vec::new() },
                    simple => simple.clone(),
                };
                *type_counts.entry(count_key).or_insert(0) += 1;
            }
        }
    }
}
