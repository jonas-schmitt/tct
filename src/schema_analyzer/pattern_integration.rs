//! Pattern Analysis Integration
//!
//! This module handles the integration of pattern analysis capabilities
//! with schema analysis for advanced string constraint optimization.

use crate::pattern_analyzer::PatternAnalysis;
use crate::vocabulary_allocator::{DomainType, ValueDomain};
use serde_json::Value;

use super::AnalysisError;

/// Trait for pattern-aware schema analysis
pub trait PatternIntegration<T: PatternAnalysis> {
    /// Get reference to the pattern analyzer
    fn pattern_analyzer(&self) -> &T;

    /// Get mutable reference to the pattern analyzer
    fn pattern_analyzer_mut(&mut self) -> &mut T;

    /// Analyze pattern constraints with advanced pattern detection
    fn analyze_advanced_pattern(
        &mut self,
        pattern: &str,
        field_path: &str,
    ) -> Result<Option<ValueDomain>, AnalysisError> {
        // Try finite enumeration detection first
        match self
            .pattern_analyzer_mut()
            .detect_finite_enumeration(pattern)
        {
            Ok(enumeration_values) => {
                if !enumeration_values.is_empty() && enumeration_values.len() <= 1000 {
                    let logical_values: Vec<Value> = enumeration_values
                        .iter()
                        .map(|s| Value::String(s.to_owned()))
                        .collect();
                    return Ok(Some(ValueDomain {
                        field_path: field_path.to_string(),
                        domain_type: DomainType::PatternEnumeration(enumeration_values),
                        cardinality: logical_values.len(),
                        logical_values,
                    }));
                }
            }
            Err(_) => {
                // Continue to bounded pattern analysis
            }
        }

        // Try bounded pattern detection
        match self.pattern_analyzer_mut().detect_bounded_pattern(pattern) {
            Ok(bounded_range) => {
                if bounded_range.total_values <= 10000 {
                    let logical_values = vec![Value::String("pattern_placeholder".to_string())];
                    let cardinality = bounded_range.total_values;
                    Ok(Some(ValueDomain {
                        field_path: field_path.to_string(),
                        domain_type: DomainType::PatternBounded(bounded_range),
                        cardinality,
                        logical_values,
                    }))
                } else {
                    Ok(None)
                }
            }
            Err(_) => Ok(None), // Pattern is not suitable for optimization
        }
    }

    /// Check if pattern analysis is enabled
    fn has_pattern_support(&self) -> bool {
        true // All implementations with PatternAnalysis have pattern support
    }

    /// Get pattern analysis capabilities summary
    fn pattern_capabilities(&self) -> PatternCapabilities {
        PatternCapabilities {
            finite_enumeration: true,
            bounded_patterns: true,
            unicode_support: true,
        }
    }
}

/// Summary of pattern analysis capabilities
#[derive(Debug, Clone, PartialEq)]
pub struct PatternCapabilities {
    /// Supports finite enumeration detection (e.g., (red|green|blue))
    pub finite_enumeration: bool,
    /// Supports bounded pattern analysis (e.g., [0-9]{3})
    pub bounded_patterns: bool,
    /// Supports Unicode character classes
    pub unicode_support: bool,
}

/// Enhanced schema analyzer with pattern integration
pub struct PatternAwareAnalyzer<T: PatternAnalysis> {
    pattern_analyzer: T,
}

impl<T: PatternAnalysis> PatternAwareAnalyzer<T> {
    pub fn new(pattern_analyzer: T) -> Self {
        Self { pattern_analyzer }
    }

    /// Analyze complex patterns with fallback strategies
    pub fn analyze_pattern_with_fallback(
        &mut self,
        pattern: &str,
        field_path: &str,
    ) -> Result<Option<ValueDomain>, AnalysisError> {
        // Primary analysis: Advanced pattern detection
        if let Some(domain) = self.analyze_advanced_pattern(pattern, field_path)? {
            return Ok(Some(domain));
        }

        // Fallback: Simple pattern classification
        if Self::is_simple_alternation(pattern)
            && let Ok(values) = self.extract_simple_alternation(pattern)
            && values.len() <= 100
        {
            let logical_values: Vec<Value> =
                values.iter().map(|s| Value::String(s.to_owned())).collect();
            return Ok(Some(ValueDomain {
                field_path: field_path.to_string(),
                domain_type: DomainType::PatternEnumeration(values),
                cardinality: logical_values.len(),
                logical_values,
            }));
        }

        Ok(None)
    }

    /// Check if pattern is a simple alternation like (a|b|c)
    fn is_simple_alternation(pattern: &str) -> bool {
        pattern.starts_with('(') && pattern.ends_with(')') && pattern.contains('|')
    }

    /// Extract values from simple alternation patterns
    fn extract_simple_alternation(&self, pattern: &str) -> Result<Vec<String>, AnalysisError> {
        if !Self::is_simple_alternation(pattern) {
            return Err(AnalysisError::InvalidSchema {
                path: "pattern".to_string(),
                reason: format!(
                    "Pattern '{}' is not a simple alternation (expected format: '(a|b|c)')",
                    pattern
                ),
            });
        }

        let inner = &pattern[1..pattern.len() - 1]; // Remove parentheses
        let values: Vec<String> = inner.split('|').map(|s| s.trim().to_string()).collect();

        if values.is_empty() {
            Err(AnalysisError::InvalidSchema {
                path: "pattern".to_string(),
                reason: format!(
                    "Empty alternation pattern '{}' contains no alternatives",
                    pattern
                ),
            })
        } else {
            Ok(values)
        }
    }
}

impl<T: PatternAnalysis> PatternIntegration<T> for PatternAwareAnalyzer<T> {
    fn pattern_analyzer(&self) -> &T {
        &self.pattern_analyzer
    }

    fn pattern_analyzer_mut(&mut self) -> &mut T {
        &mut self.pattern_analyzer
    }
}

/// Utilities for pattern-based optimization analysis
pub mod pattern_utils {
    /// Estimate the complexity of a regular expression pattern
    pub fn estimate_pattern_complexity(pattern: &str) -> PatternComplexity {
        let has_quantifiers = pattern.chars().any(|c| "?+*{}".contains(c));
        let has_char_classes = pattern.contains('[') && pattern.contains(']');
        let has_groups = pattern.contains('(') && pattern.contains(')');
        let has_anchors = pattern.starts_with('^') || pattern.ends_with('$');

        let complexity_score = [has_quantifiers, has_char_classes, has_groups, has_anchors]
            .iter()
            .map(|&b| if b { 1 } else { 0 })
            .sum();

        match complexity_score {
            0..=1 => PatternComplexity::Simple,
            2..=3 => PatternComplexity::Moderate,
            _ => PatternComplexity::Complex,
        }
    }

    /// Pattern complexity classification
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub enum PatternComplexity {
        Simple,   // Basic strings or simple alternations
        Moderate, // Character classes and basic quantifiers
        Complex,  // Advanced regex features
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pattern_analyzer::PatternAnalyzer;

    #[test]
    fn test_simple_alternation_detection() {
        assert!(PatternAwareAnalyzer::<PatternAnalyzer>::is_simple_alternation("(a|b|c)"));
        assert!(PatternAwareAnalyzer::<PatternAnalyzer>::is_simple_alternation("(red|green|blue)"));
        assert!(!PatternAwareAnalyzer::<PatternAnalyzer>::is_simple_alternation("abc"));
        assert!(!PatternAwareAnalyzer::<PatternAnalyzer>::is_simple_alternation("[abc]"));
    }

    #[test]
    fn test_simple_alternation_extraction() {
        let analyzer = PatternAwareAnalyzer::new(PatternAnalyzer::new());
        let values = analyzer.extract_simple_alternation("(a|b|c)").unwrap();
        assert_eq!(values, vec!["a", "b", "c"]);

        let values2 = analyzer
            .extract_simple_alternation("(red|green|blue)")
            .unwrap();
        assert_eq!(values2, vec!["red", "green", "blue"]);
    }

    #[test]
    fn test_pattern_capabilities() {
        let analyzer = PatternAwareAnalyzer::new(PatternAnalyzer::new());
        let capabilities = analyzer.pattern_capabilities();

        assert!(capabilities.finite_enumeration);
        assert!(capabilities.bounded_patterns);
        assert!(capabilities.unicode_support);
    }

    #[test]
    fn test_pattern_complexity_estimation() {
        use pattern_utils::*;

        assert_eq!(
            estimate_pattern_complexity("abc"),
            PatternComplexity::Simple
        );
        assert_eq!(
            estimate_pattern_complexity("(a|b|c)"),
            PatternComplexity::Simple
        );
        assert_eq!(
            estimate_pattern_complexity("[0-9]+"),
            PatternComplexity::Moderate
        );
        assert_eq!(
            estimate_pattern_complexity("^[a-zA-Z0-9]+$"),
            PatternComplexity::Moderate
        );
    }
}
