//! Domain Analysis Module
//!
//! This module contains specialized analysis for different constraint types:
//! integer constraints, string constraints, and pattern-based optimizations.

use crate::pattern_analyzer::PatternAnalysis;
use crate::schema_utils::{get_enum_values, get_schema_type};
use crate::vocabulary_allocator::{DomainType, ValueDomain};
use crate::warnings::{SchemaWarning, SchemaWarnings};
use serde_json::{Map, Value};

use super::AnalysisError;

fn inclusive_i64_range_size(minimum: i64, maximum: i64) -> Option<u128> {
    if minimum > maximum {
        return None;
    }

    Some((i128::from(maximum) - i128::from(minimum) + 1) as u128)
}

/// Trait for domain-specific analysis
pub trait DomainAnalysis {
    /// Analyze schema properties to extract value domains
    fn analyze_schema_properties(
        &mut self,
        schema_obj: &Map<String, Value>,
        field_path: &str,
        warnings: &mut SchemaWarnings,
    ) -> Result<Option<ValueDomain>, AnalysisError>;

    /// Analyze integer constraints (minimum, maximum)
    fn analyze_integer_constraints(
        &self,
        schema_obj: &Map<String, Value>,
        field_path: &str,
    ) -> Option<ValueDomain>;

    /// Analyze string constraints (enum, pattern)
    fn analyze_string_constraints(
        &mut self,
        schema_obj: &Map<String, Value>,
        field_path: &str,
        warnings: &mut SchemaWarnings,
    ) -> Result<Option<ValueDomain>, AnalysisError>;

    /// Analyze boolean constraints
    fn analyze_boolean_constraints(&self, field_path: &str) -> Option<ValueDomain>;
}

/// Standard implementation of domain analysis
pub struct StandardDomainAnalyzer<T: PatternAnalysis> {
    pattern_analyzer: T,
}

impl<T: PatternAnalysis> StandardDomainAnalyzer<T> {
    pub fn new(pattern_analyzer: T) -> Self {
        Self { pattern_analyzer }
    }
}

impl<T: PatternAnalysis> DomainAnalysis for StandardDomainAnalyzer<T> {
    fn analyze_schema_properties(
        &mut self,
        schema_obj: &Map<String, Value>,
        field_path: &str,
        warnings: &mut SchemaWarnings,
    ) -> Result<Option<ValueDomain>, AnalysisError> {
        // Check for enum constraints first (highest priority)
        if let Some(enum_values) = get_enum_values(schema_obj) {
            // Gracefully handle mixed enum types - collect only string values for domain analysis
            let string_values: Vec<String> = enum_values
                .iter()
                .filter_map(|v| v.as_str())
                .map(|s| s.to_string())
                .collect();

            // Only proceed if we have string values to analyze
            if string_values.is_empty() {
                // No string enum values found - skip domain analysis for this enum
                return Ok(None);
            }

            let logical_values: Vec<Value> = string_values
                .iter()
                .map(|s| Value::String(s.to_owned()))
                .collect();
            return Ok(Some(ValueDomain {
                field_path: field_path.to_string(),
                domain_type: DomainType::Enum {
                    values: string_values,
                },
                cardinality: logical_values.len(),
                logical_values,
            }));
        }

        // Check type-specific constraints
        match get_schema_type(schema_obj) {
            Some("integer") => Ok(self.analyze_integer_constraints(schema_obj, field_path)),
            Some("string") => self.analyze_string_constraints(schema_obj, field_path, warnings),
            Some("boolean") => Ok(self.analyze_boolean_constraints(field_path)),
            _ => Ok(None),
        }
    }

    fn analyze_integer_constraints(
        &self,
        schema_obj: &Map<String, Value>,
        field_path: &str,
    ) -> Option<ValueDomain> {
        let minimum = schema_obj.get("minimum")?.as_i64()?;
        let maximum = schema_obj.get("maximum")?.as_i64()?;

        let range_size = inclusive_i64_range_size(minimum, maximum)?;
        // Only create domain for reasonably sized ranges
        if range_size <= 10000 {
            let range_size = range_size as usize;
            // For large ranges, avoid pre-computing all values - generate on demand
            let logical_values = if range_size <= 100 {
                // Small ranges: pre-compute for better lookup performance
                (minimum..=maximum)
                    .map(|i| Value::Number(serde_json::Number::from(i)))
                    .collect()
            } else {
                // Large ranges: use placeholder that can be generated on demand
                vec![
                    Value::Number(serde_json::Number::from(minimum)),
                    Value::String(format!("..range({}-{})..", minimum, maximum)),
                    Value::Number(serde_json::Number::from(maximum)),
                ]
            };
            Some(ValueDomain {
                field_path: field_path.to_string(),
                domain_type: DomainType::BoundedInteger {
                    min: minimum,
                    max: maximum,
                },
                cardinality: range_size,
                logical_values,
            })
        } else {
            None
        }
    }

    fn analyze_string_constraints(
        &mut self,
        schema_obj: &Map<String, Value>,
        field_path: &str,
        warnings: &mut SchemaWarnings,
    ) -> Result<Option<ValueDomain>, AnalysisError> {
        // Check for pattern constraints
        if let Some(pattern) = schema_obj.get("pattern").and_then(|p| p.as_str()) {
            return self.analyze_pattern_constraint(pattern, field_path, warnings);
        }

        // Check for format constraints
        if let Some(format) = schema_obj.get("format").and_then(|f| f.as_str()) {
            return Ok(self.analyze_format_constraint(format, field_path));
        }

        Ok(None)
    }

    fn analyze_boolean_constraints(&self, field_path: &str) -> Option<ValueDomain> {
        let logical_values = vec![Value::Bool(false), Value::Bool(true)];
        Some(ValueDomain {
            field_path: field_path.to_string(),
            domain_type: DomainType::Boolean,
            cardinality: 2, // true, false
            logical_values,
        })
    }
}

impl<T: PatternAnalysis> StandardDomainAnalyzer<T> {
    /// Analyze pattern constraints with finite enumeration detection (refactored for clarity)
    fn analyze_pattern_constraint(
        &mut self,
        pattern: &str,
        field_path: &str,
        warnings: &mut SchemaWarnings,
    ) -> Result<Option<ValueDomain>, AnalysisError> {
        // Try finite enumeration first
        if let Ok(enumeration_values) = self.pattern_analyzer.detect_finite_enumeration(pattern) {
            return self.handle_finite_enumeration(
                enumeration_values,
                field_path,
                pattern,
                warnings,
            );
        }

        // Finite enumeration failed, try bounded pattern
        self.handle_bounded_pattern_fallback(pattern, field_path, warnings)
    }

    /// Handle finite enumeration patterns (reduced nesting)
    fn handle_finite_enumeration(
        &self,
        enumeration_values: Vec<String>,
        field_path: &str,
        pattern: &str,
        warnings: &mut SchemaWarnings,
    ) -> Result<Option<ValueDomain>, AnalysisError> {
        // Check if enumeration is manageable
        if enumeration_values.is_empty() || enumeration_values.len() > 1000 {
            warnings.add(SchemaWarning::complex_pattern_optimization(
                field_path.to_string(),
                pattern.to_string(),
            ));
            return Ok(None);
        }

        // Pattern represents a finite, manageable set of values
        let logical_values: Vec<Value> = enumeration_values
            .iter()
            .map(|s| Value::String(s.to_owned()))
            .collect();

        Ok(Some(ValueDomain {
            field_path: field_path.to_string(),
            domain_type: DomainType::PatternEnumeration(enumeration_values),
            cardinality: logical_values.len(),
            logical_values,
        }))
    }

    /// Handle bounded pattern fallback when finite enumeration fails (reduced nesting)
    fn handle_bounded_pattern_fallback(
        &mut self,
        pattern: &str,
        field_path: &str,
        warnings: &mut SchemaWarnings,
    ) -> Result<Option<ValueDomain>, AnalysisError> {
        // Try bounded pattern detection
        match self.pattern_analyzer.detect_bounded_pattern(pattern) {
            Ok(bounded_range) => {
                self.handle_bounded_range(bounded_range, field_path, pattern, warnings)
            }
            Err(pattern_error) => {
                self.handle_pattern_error(&pattern_error, pattern, field_path, warnings);
                Ok(None)
            }
        }
    }

    /// Handle successful bounded range detection (reduced nesting)
    fn handle_bounded_range(
        &self,
        bounded_range: crate::pattern_analyzer::BoundedRange,
        field_path: &str,
        pattern: &str,
        warnings: &mut SchemaWarnings,
    ) -> Result<Option<ValueDomain>, AnalysisError> {
        // Check if bounded range is manageable
        if bounded_range.total_values > 10000 {
            warnings.add(SchemaWarning::unbounded_pattern_optimization(
                field_path.to_string(),
                pattern.to_string(),
            ));
            return Ok(None);
        }

        // Generate logical values for bounded patterns (placeholder for now)
        let logical_values = vec![Value::String("pattern_placeholder".to_string())];
        let cardinality = bounded_range.total_values;

        Ok(Some(ValueDomain {
            field_path: field_path.to_string(),
            domain_type: DomainType::PatternBounded(bounded_range),
            cardinality,
            logical_values,
        }))
    }

    /// Handle pattern analysis errors with appropriate warnings (reduced nesting)
    fn handle_pattern_error(
        &self,
        pattern_error: &crate::pattern_analyzer::PatternError,
        pattern: &str,
        field_path: &str,
        warnings: &mut SchemaWarnings,
    ) {
        // Check if it's a syntax error or unbounded pattern
        if pattern.contains(".*")
            || pattern.contains(".+")
            || pattern.contains("*")
            || pattern.contains("+")
        {
            // Unbounded pattern
            warnings.add(SchemaWarning::unbounded_pattern_optimization(
                field_path.to_string(),
                pattern.to_string(),
            ));
        } else if pattern_error.to_string().contains("syntax")
            || pattern_error.to_string().contains("invalid")
        {
            // Invalid regex syntax
            warnings.add(SchemaWarning::invalid_pattern_syntax(
                field_path.to_string(),
                pattern.to_string(),
                pattern_error.to_string(),
            ));
        } else {
            // Complex pattern that cannot be optimized
            warnings.add(SchemaWarning::complex_pattern_optimization(
                field_path.to_string(),
                pattern.to_string(),
            ));
        }
    }

    /// Analyze format constraints (placeholder for future enhancement)
    fn analyze_format_constraint(&self, _format: &str, _field_path: &str) -> Option<ValueDomain> {
        // Future enhancement: analyze JSON Schema format constraints
        // Examples: "date", "email", "uri", etc.
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pattern_analyzer::PatternAnalyzer;
    use serde_json::json;

    fn create_test_analyzer() -> StandardDomainAnalyzer<PatternAnalyzer> {
        StandardDomainAnalyzer::new(PatternAnalyzer::new())
    }

    #[test]
    fn test_enum_analysis() {
        let mut analyzer = create_test_analyzer();
        let schema = json!({
            "type": "string",
            "enum": ["red", "green", "blue"]
        })
        .as_object()
        .unwrap()
        .clone();

        let mut warnings = SchemaWarnings::new();
        let result = analyzer
            .analyze_schema_properties(&schema, "color", &mut warnings)
            .unwrap();
        assert!(result.is_some());

        let domain = result.unwrap();
        assert_eq!(domain.field_path, "color");
        assert_eq!(domain.cardinality, 3);
        match domain.domain_type {
            DomainType::Enum { values } => {
                assert_eq!(values, vec!["red", "green", "blue"]);
            }
            _ => panic!("Expected enumeration domain"),
        }
    }

    #[test]
    fn test_integer_analysis() {
        let analyzer = create_test_analyzer();
        let schema = json!({
            "type": "integer",
            "minimum": 1,
            "maximum": 10
        })
        .as_object()
        .unwrap()
        .clone();

        let result = analyzer.analyze_integer_constraints(&schema, "count");
        assert!(result.is_some());

        let domain = result.unwrap();
        assert_eq!(domain.field_path, "count");
        assert_eq!(domain.cardinality, 10);
        match domain.domain_type {
            DomainType::BoundedInteger { min, max } => {
                assert_eq!(min, 1);
                assert_eq!(max, 10);
            }
            _ => panic!("Expected bounded integer domain"),
        }
    }

    #[test]
    fn test_boolean_analysis() {
        let analyzer = create_test_analyzer();
        let result = analyzer.analyze_boolean_constraints("flag");
        assert!(result.is_some());

        let domain = result.unwrap();
        assert_eq!(domain.field_path, "flag");
        assert_eq!(domain.cardinality, 2);
        match domain.domain_type {
            DomainType::Boolean => {}
            _ => panic!("Expected boolean domain"),
        }
    }

    #[test]
    fn test_large_integer_range_rejected() {
        let analyzer = create_test_analyzer();
        let schema = json!({
            "type": "integer",
            "minimum": 1,
            "maximum": 50000
        })
        .as_object()
        .unwrap()
        .clone();

        let result = analyzer.analyze_integer_constraints(&schema, "large_count");
        assert!(result.is_none()); // Should be rejected due to size
    }

    // ========== Additional Coverage Tests ==========

    #[test]
    fn test_enum_with_mixed_types() {
        let mut analyzer = create_test_analyzer();
        let schema = json!({
            "enum": ["red", 42, true, null]
        })
        .as_object()
        .unwrap()
        .clone();

        let mut warnings = SchemaWarnings::new();
        let result = analyzer
            .analyze_schema_properties(&schema, "mixed", &mut warnings)
            .unwrap();

        // Should extract only string values
        assert!(result.is_some());
        let domain = result.unwrap();
        assert_eq!(domain.cardinality, 1); // Only "red" is a string
    }

    #[test]
    fn test_enum_with_no_strings() {
        let mut analyzer = create_test_analyzer();
        let schema = json!({
            "enum": [1, 2, 3, true, false]
        })
        .as_object()
        .unwrap()
        .clone();

        let mut warnings = SchemaWarnings::new();
        let result = analyzer
            .analyze_schema_properties(&schema, "numbers", &mut warnings)
            .unwrap();

        // Should return None since no string values
        assert!(result.is_none());
    }

    #[test]
    fn test_integer_missing_minimum() {
        let analyzer = create_test_analyzer();
        let schema = json!({
            "type": "integer",
            "maximum": 10
        })
        .as_object()
        .unwrap()
        .clone();

        let result = analyzer.analyze_integer_constraints(&schema, "count");
        assert!(result.is_none()); // Missing minimum
    }

    #[test]
    fn test_integer_missing_maximum() {
        let analyzer = create_test_analyzer();
        let schema = json!({
            "type": "integer",
            "minimum": 0
        })
        .as_object()
        .unwrap()
        .clone();

        let result = analyzer.analyze_integer_constraints(&schema, "count");
        assert!(result.is_none()); // Missing maximum
    }

    #[test]
    fn test_integer_invalid_range() {
        let analyzer = create_test_analyzer();
        let schema = json!({
            "type": "integer",
            "minimum": 100,
            "maximum": 10
        })
        .as_object()
        .unwrap()
        .clone();

        let result = analyzer.analyze_integer_constraints(&schema, "count");
        assert!(result.is_none()); // Invalid range (min > max)
    }

    #[test]
    fn test_integer_extreme_i64_range_does_not_overflow() {
        let analyzer = create_test_analyzer();
        let schema = json!({
            "type": "integer",
            "minimum": i64::MIN,
            "maximum": i64::MAX
        })
        .as_object()
        .unwrap()
        .clone();

        let result = analyzer.analyze_integer_constraints(&schema, "extreme");

        assert!(result.is_none());
    }

    #[test]
    fn test_integer_small_range_near_i64_min_is_analyzed() {
        let analyzer = create_test_analyzer();
        let schema = json!({
            "type": "integer",
            "minimum": i64::MIN,
            "maximum": i64::MIN + 2
        })
        .as_object()
        .unwrap()
        .clone();

        let domain = analyzer
            .analyze_integer_constraints(&schema, "near_min")
            .expect("small bounded range near i64::MIN should be analyzed");

        assert_eq!(domain.cardinality, 3);
        assert_eq!(
            domain.logical_values,
            vec![
                Value::Number(serde_json::Number::from(i64::MIN)),
                Value::Number(serde_json::Number::from(i64::MIN + 1)),
                Value::Number(serde_json::Number::from(i64::MIN + 2)),
            ]
        );
    }

    #[test]
    fn test_integer_small_range_precomputed() {
        let analyzer = create_test_analyzer();
        let schema = json!({
            "type": "integer",
            "minimum": 0,
            "maximum": 5
        })
        .as_object()
        .unwrap()
        .clone();

        let result = analyzer.analyze_integer_constraints(&schema, "small");
        assert!(result.is_some());

        let domain = result.unwrap();
        assert_eq!(domain.cardinality, 6);
        assert_eq!(domain.logical_values.len(), 6); // All values precomputed
    }

    #[test]
    fn test_integer_large_range_placeholder() {
        let analyzer = create_test_analyzer();
        let schema = json!({
            "type": "integer",
            "minimum": 0,
            "maximum": 500
        })
        .as_object()
        .unwrap()
        .clone();

        let result = analyzer.analyze_integer_constraints(&schema, "large");
        assert!(result.is_some());

        let domain = result.unwrap();
        assert_eq!(domain.cardinality, 501);
        assert_eq!(domain.logical_values.len(), 3); // Placeholder representation
    }

    #[test]
    fn test_analyze_schema_properties_unknown_type() {
        let mut analyzer = create_test_analyzer();
        let schema = json!({
            "type": "object"
        })
        .as_object()
        .unwrap()
        .clone();

        let mut warnings = SchemaWarnings::new();
        let result = analyzer
            .analyze_schema_properties(&schema, "obj", &mut warnings)
            .unwrap();

        assert!(result.is_none()); // Unknown type for domain analysis
    }

    #[test]
    fn test_analyze_schema_properties_no_type() {
        let mut analyzer = create_test_analyzer();
        let schema = json!({
            "description": "Just metadata"
        })
        .as_object()
        .unwrap()
        .clone();

        let mut warnings = SchemaWarnings::new();
        let result = analyzer
            .analyze_schema_properties(&schema, "field", &mut warnings)
            .unwrap();

        assert!(result.is_none()); // No type specified
    }

    #[test]
    fn test_string_with_pattern_finite_enumeration() {
        let mut analyzer = create_test_analyzer();
        let schema = json!({
            "type": "string",
            "pattern": "^(red|green|blue)$"
        })
        .as_object()
        .unwrap()
        .clone();

        let mut warnings = SchemaWarnings::new();
        let result = analyzer
            .analyze_schema_properties(&schema, "color", &mut warnings)
            .unwrap();

        assert!(result.is_some());
        let domain = result.unwrap();
        match domain.domain_type {
            DomainType::PatternEnumeration(values) => {
                assert_eq!(values.len(), 3);
            }
            _ => panic!("Expected pattern enumeration"),
        }
    }

    #[test]
    fn test_string_with_pattern_empty_enumeration() {
        let mut analyzer = create_test_analyzer();
        // Pattern that results in empty enumeration
        let schema = json!({
            "type": "string",
            "pattern": "^()$"
        })
        .as_object()
        .unwrap()
        .clone();

        let mut warnings = SchemaWarnings::new();
        let result = analyzer
            .analyze_schema_properties(&schema, "empty", &mut warnings)
            .unwrap();

        assert!(result.is_none());
        assert!(!warnings.is_empty());
    }

    #[test]
    fn test_string_with_pattern_large_enumeration() {
        let mut analyzer = create_test_analyzer();
        // Pattern that generates >1000 values
        let schema = json!({
            "type": "string",
            "pattern": "^[A-Z]{3}$"
        })
        .as_object()
        .unwrap()
        .clone();

        let mut warnings = SchemaWarnings::new();
        let result = analyzer
            .analyze_schema_properties(&schema, "large", &mut warnings)
            .unwrap();

        assert!(result.is_none());
        assert!(!warnings.is_empty());
    }

    #[test]
    fn test_string_with_bounded_pattern() {
        let mut analyzer = create_test_analyzer();
        let schema = json!({
            "type": "string",
            "pattern": "^\\d{4}$"
        })
        .as_object()
        .unwrap()
        .clone();

        let mut warnings = SchemaWarnings::new();
        let result = analyzer
            .analyze_schema_properties(&schema, "year", &mut warnings)
            .unwrap();

        assert!(result.is_some());
        let domain = result.unwrap();
        match domain.domain_type {
            DomainType::PatternBounded(_) => {}
            _ => panic!("Expected pattern bounded"),
        }
    }

    #[test]
    fn test_string_with_large_bounded_pattern() {
        let mut analyzer = create_test_analyzer();
        // Pattern with >10000 values
        let schema = json!({
            "type": "string",
            "pattern": "^[A-Z]{4}$"
        })
        .as_object()
        .unwrap()
        .clone();

        let mut warnings = SchemaWarnings::new();
        let result = analyzer
            .analyze_schema_properties(&schema, "large", &mut warnings)
            .unwrap();

        assert!(result.is_none());
        assert!(!warnings.is_empty());
    }

    #[test]
    fn test_string_with_unbounded_pattern_asterisk() {
        let mut analyzer = create_test_analyzer();
        let schema = json!({
            "type": "string",
            "pattern": "^abc.*$"
        })
        .as_object()
        .unwrap()
        .clone();

        let mut warnings = SchemaWarnings::new();
        let result = analyzer
            .analyze_schema_properties(&schema, "unbounded", &mut warnings)
            .unwrap();

        assert!(result.is_none());
        assert!(!warnings.is_empty());
    }

    #[test]
    fn test_string_with_unbounded_pattern_plus() {
        let mut analyzer = create_test_analyzer();
        let schema = json!({
            "type": "string",
            "pattern": "^abc.+$"
        })
        .as_object()
        .unwrap()
        .clone();

        let mut warnings = SchemaWarnings::new();
        let result = analyzer
            .analyze_schema_properties(&schema, "unbounded", &mut warnings)
            .unwrap();

        assert!(result.is_none());
        assert!(!warnings.is_empty());
    }

    #[test]
    fn test_string_with_unbounded_pattern_star() {
        let mut analyzer = create_test_analyzer();
        let schema = json!({
            "type": "string",
            "pattern": "^a*b$"
        })
        .as_object()
        .unwrap()
        .clone();

        let mut warnings = SchemaWarnings::new();
        let result = analyzer
            .analyze_schema_properties(&schema, "unbounded", &mut warnings)
            .unwrap();

        assert!(result.is_none());
        assert!(!warnings.is_empty());
    }

    #[test]
    fn test_string_with_invalid_pattern_syntax() {
        let mut analyzer = create_test_analyzer();
        let schema = json!({
            "type": "string",
            "pattern": "^[invalid"
        })
        .as_object()
        .unwrap()
        .clone();

        let mut warnings = SchemaWarnings::new();
        let result = analyzer
            .analyze_schema_properties(&schema, "invalid", &mut warnings)
            .unwrap();

        assert!(result.is_none());
        assert!(!warnings.is_empty());
    }

    #[test]
    fn test_string_with_complex_pattern() {
        let mut analyzer = create_test_analyzer();
        // Complex pattern that can't be optimized
        let schema = json!({
            "type": "string",
            "pattern": "^(?=.*[a-z])(?=.*[A-Z]).*$"
        })
        .as_object()
        .unwrap()
        .clone();

        let mut warnings = SchemaWarnings::new();
        let result = analyzer
            .analyze_schema_properties(&schema, "complex", &mut warnings)
            .unwrap();

        assert!(result.is_none());
        assert!(!warnings.is_empty());
    }

    #[test]
    fn test_string_with_format_constraint() {
        let mut analyzer = create_test_analyzer();
        let schema = json!({
            "type": "string",
            "format": "date"
        })
        .as_object()
        .unwrap()
        .clone();

        let mut warnings = SchemaWarnings::new();
        let result = analyzer
            .analyze_schema_properties(&schema, "date_field", &mut warnings)
            .unwrap();

        // Format constraints not yet supported
        assert!(result.is_none());
    }

    #[test]
    fn test_string_without_constraints() {
        let mut analyzer = create_test_analyzer();
        let schema = json!({
            "type": "string"
        })
        .as_object()
        .unwrap()
        .clone();

        let mut warnings = SchemaWarnings::new();
        let result = analyzer
            .analyze_schema_properties(&schema, "str", &mut warnings)
            .unwrap();

        assert!(result.is_none());
    }

    #[test]
    fn test_analyze_format_constraint_placeholder() {
        let analyzer = create_test_analyzer();
        let result = analyzer.analyze_format_constraint("email", "email_field");
        assert!(result.is_none()); // Not implemented yet
    }

    #[test]
    fn test_boolean_logical_values() {
        let analyzer = create_test_analyzer();
        let result = analyzer.analyze_boolean_constraints("flag");
        assert!(result.is_some());

        let domain = result.unwrap();
        assert_eq!(domain.logical_values.len(), 2);
        assert_eq!(domain.logical_values[0], Value::Bool(false));
        assert_eq!(domain.logical_values[1], Value::Bool(true));
    }

    #[test]
    fn test_enum_logical_values() {
        let mut analyzer = create_test_analyzer();
        let schema = json!({
            "enum": ["a", "b", "c"]
        })
        .as_object()
        .unwrap()
        .clone();

        let mut warnings = SchemaWarnings::new();
        let result = analyzer
            .analyze_schema_properties(&schema, "field", &mut warnings)
            .unwrap()
            .unwrap();

        assert_eq!(result.logical_values.len(), 3);
        assert_eq!(result.logical_values[0], Value::String("a".to_string()));
        assert_eq!(result.logical_values[1], Value::String("b".to_string()));
        assert_eq!(result.logical_values[2], Value::String("c".to_string()));
    }
}
