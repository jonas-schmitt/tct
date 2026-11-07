//! Schema Processing Utilities
//!
//! This module provides SchemaTraversal implementations for analyzing
//! schema properties and collecting optimization statistics.

use crate::schema_utils::SchemaTraversal;
use serde_json::{Map, Value};

fn capped_inclusive_i64_range_size(min: i64, max: i64, cap: usize) -> Option<usize> {
    if min > max {
        return None;
    }

    let range_size = (i128::from(max) - i128::from(min) + 1) as u128;
    Some(range_size.min(cap as u128) as usize)
}

/// Schema processor for counting optimizable fields and constrained values
pub struct OptimizableFieldsProcessor {
    field_count: usize,
    value_count: usize,
}

impl Default for OptimizableFieldsProcessor {
    fn default() -> Self {
        Self::new()
    }
}

impl OptimizableFieldsProcessor {
    /// Create a new processor for counting optimizable fields
    pub fn new() -> Self {
        Self {
            field_count: 0,
            value_count: 0,
        }
    }

    /// Get the number of optimizable fields found during traversal
    pub fn field_count(&self) -> usize {
        self.field_count
    }

    /// Get the total number of constrained values found during traversal
    pub fn value_count(&self) -> usize {
        self.value_count
    }
}

impl SchemaTraversal for OptimizableFieldsProcessor {
    type Result = (usize, usize);

    fn process_schema(&mut self, schema: &Map<String, Value>, _path: &str) -> Self::Result {
        let mut local_field_count = 0;
        let mut local_value_count = 0;

        // Check for direct optimizable fields
        if let Some(enum_values) = schema.get("enum").and_then(|e| e.as_array()) {
            local_field_count += 1;
            local_value_count += enum_values.len();
        } else if schema.get("type").and_then(|t| t.as_str()) == Some("integer")
            && (schema.contains_key("minimum") || schema.contains_key("maximum"))
        {
            local_field_count += 1;
            let min = schema.get("minimum").and_then(|m| m.as_i64()).unwrap_or(0);
            let max = schema
                .get("maximum")
                .and_then(|m| m.as_i64())
                .unwrap_or_else(|| min.saturating_add(100));
            let range_size = capped_inclusive_i64_range_size(min, max, 10000).unwrap_or(0);
            local_value_count += range_size;
        } else if schema.get("type").and_then(|t| t.as_str()) == Some("boolean") {
            local_field_count += 1;
            local_value_count += 2; // true, false
        }

        self.field_count += local_field_count;
        self.value_count += local_value_count;

        (local_field_count, local_value_count)
    }

    fn combine_results(&self, results: Vec<Self::Result>) -> Self::Result {
        let total_fields: usize = results.iter().map(|(f, _)| f).sum();
        let total_values: usize = results.iter().map(|(_, v)| v).sum();
        (total_fields, total_values)
    }
}

/// Schema processor for counting domain types (enums vs integers)
pub struct DomainTypesProcessor {
    enum_count: usize,
    integer_count: usize,
}

impl Default for DomainTypesProcessor {
    fn default() -> Self {
        Self::new()
    }
}

impl DomainTypesProcessor {
    /// Create a new processor for counting domain types
    pub fn new() -> Self {
        Self {
            enum_count: 0,
            integer_count: 0,
        }
    }

    /// Get the number of enum domains found during traversal
    pub fn enum_count(&self) -> usize {
        self.enum_count
    }

    /// Get the number of integer domains found during traversal
    pub fn integer_count(&self) -> usize {
        self.integer_count
    }
}

impl SchemaTraversal for DomainTypesProcessor {
    type Result = (usize, usize);

    fn process_schema(&mut self, schema: &Map<String, Value>, _path: &str) -> Self::Result {
        let mut local_enum_count = 0;
        let mut local_integer_count = 0;

        // Count different types of optimizable domains
        if schema.get("enum").is_some() {
            local_enum_count += 1;
        } else if schema.get("type").and_then(|t| t.as_str()) == Some("integer")
            && (schema.contains_key("minimum") || schema.contains_key("maximum"))
        {
            local_integer_count += 1;
        }

        self.enum_count += local_enum_count;
        self.integer_count += local_integer_count;

        (local_enum_count, local_integer_count)
    }

    fn combine_results(&self, results: Vec<Self::Result>) -> Self::Result {
        let total_enums: usize = results.iter().map(|(e, _)| e).sum();
        let total_integers: usize = results.iter().map(|(_, i)| i).sum();
        (total_enums, total_integers)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // ========== OptimizableFieldsProcessor Tests ==========

    #[test]
    fn test_optimizable_fields_processor_new() {
        let processor = OptimizableFieldsProcessor::new();
        assert_eq!(processor.field_count(), 0);
        assert_eq!(processor.value_count(), 0);
    }

    #[test]
    fn test_optimizable_fields_processor_default() {
        let processor = OptimizableFieldsProcessor::default();
        assert_eq!(processor.field_count(), 0);
        assert_eq!(processor.value_count(), 0);
    }

    #[test]
    fn test_optimizable_fields_process_enum() {
        let mut processor = OptimizableFieldsProcessor::new();
        let schema = json!({
            "enum": ["red", "green", "blue"]
        })
        .as_object()
        .unwrap()
        .clone();

        let (fields, values) = processor.process_schema(&schema, "#/");

        assert_eq!(fields, 1);
        assert_eq!(values, 3);
        assert_eq!(processor.field_count(), 1);
        assert_eq!(processor.value_count(), 3);
    }

    #[test]
    fn test_optimizable_fields_process_bounded_integer_with_min_and_max() {
        let mut processor = OptimizableFieldsProcessor::new();
        let schema = json!({
            "type": "integer",
            "minimum": 0,
            "maximum": 10
        })
        .as_object()
        .unwrap()
        .clone();

        let (fields, values) = processor.process_schema(&schema, "#/");

        assert_eq!(fields, 1);
        assert_eq!(values, 11); // 0 to 10 inclusive
        assert_eq!(processor.field_count(), 1);
        assert_eq!(processor.value_count(), 11);
    }

    #[test]
    fn test_optimizable_fields_process_integer_with_only_minimum() {
        let mut processor = OptimizableFieldsProcessor::new();
        let schema = json!({
            "type": "integer",
            "minimum": 50
        })
        .as_object()
        .unwrap()
        .clone();

        let (fields, values) = processor.process_schema(&schema, "#/");

        assert_eq!(fields, 1);
        assert_eq!(values, 101); // Default range: 50 to 150
    }

    #[test]
    fn test_optimizable_fields_process_integer_with_only_maximum() {
        let mut processor = OptimizableFieldsProcessor::new();
        let schema = json!({
            "type": "integer",
            "maximum": 25
        })
        .as_object()
        .unwrap()
        .clone();

        let (fields, values) = processor.process_schema(&schema, "#/");

        assert_eq!(fields, 1);
        assert_eq!(values, 26); // 0 (default min) to 25
    }

    #[test]
    fn test_optimizable_fields_process_large_range_capped() {
        let mut processor = OptimizableFieldsProcessor::new();
        let schema = json!({
            "type": "integer",
            "minimum": 0,
            "maximum": 50000
        })
        .as_object()
        .unwrap()
        .clone();

        let (fields, values) = processor.process_schema(&schema, "#/");

        assert_eq!(fields, 1);
        assert_eq!(values, 10000); // Capped at max 10000
    }

    #[test]
    fn test_optimizable_fields_process_extreme_integer_bounds_without_overflow() {
        let mut processor = OptimizableFieldsProcessor::new();
        let schema = json!({
            "type": "integer",
            "minimum": i64::MIN,
            "maximum": i64::MAX
        })
        .as_object()
        .unwrap()
        .clone();

        let (fields, values) = processor.process_schema(&schema, "#/");

        assert_eq!(fields, 1);
        assert_eq!(values, 10000); // Capped at max 10000 without overflowing i64 math
    }

    #[test]
    fn test_optimizable_fields_process_minimum_near_i64_max_without_overflow() {
        let mut processor = OptimizableFieldsProcessor::new();
        let schema = json!({
            "type": "integer",
            "minimum": i64::MAX
        })
        .as_object()
        .unwrap()
        .clone();

        let (fields, values) = processor.process_schema(&schema, "#/");

        assert_eq!(fields, 1);
        assert_eq!(values, 1);
    }

    #[test]
    fn test_optimizable_fields_process_boolean() {
        let mut processor = OptimizableFieldsProcessor::new();
        let schema = json!({
            "type": "boolean"
        })
        .as_object()
        .unwrap()
        .clone();

        let (fields, values) = processor.process_schema(&schema, "#/");

        assert_eq!(fields, 1);
        assert_eq!(values, 2); // true, false
    }

    #[test]
    fn test_optimizable_fields_process_string_not_optimizable() {
        let mut processor = OptimizableFieldsProcessor::new();
        let schema = json!({
            "type": "string"
        })
        .as_object()
        .unwrap()
        .clone();

        let (fields, values) = processor.process_schema(&schema, "#/");

        assert_eq!(fields, 0);
        assert_eq!(values, 0);
    }

    #[test]
    fn test_optimizable_fields_process_integer_without_bounds_not_optimizable() {
        let mut processor = OptimizableFieldsProcessor::new();
        let schema = json!({
            "type": "integer"
        })
        .as_object()
        .unwrap()
        .clone();

        let (fields, values) = processor.process_schema(&schema, "#/");

        assert_eq!(fields, 0);
        assert_eq!(values, 0);
    }

    #[test]
    fn test_optimizable_fields_combine_results_empty() {
        let processor = OptimizableFieldsProcessor::new();
        let combined = processor.combine_results(vec![]);
        assert_eq!(combined, (0, 0));
    }

    #[test]
    fn test_optimizable_fields_combine_results_single() {
        let processor = OptimizableFieldsProcessor::new();
        let combined = processor.combine_results(vec![(2, 10)]);
        assert_eq!(combined, (2, 10));
    }

    #[test]
    fn test_optimizable_fields_combine_results_multiple() {
        let processor = OptimizableFieldsProcessor::new();
        let combined = processor.combine_results(vec![(1, 5), (2, 10), (1, 3)]);
        assert_eq!(combined, (4, 18));
    }

    // ========== DomainTypesProcessor Tests ==========

    #[test]
    fn test_domain_types_processor_new() {
        let processor = DomainTypesProcessor::new();
        assert_eq!(processor.enum_count(), 0);
        assert_eq!(processor.integer_count(), 0);
    }

    #[test]
    fn test_domain_types_processor_default() {
        let processor = DomainTypesProcessor::default();
        assert_eq!(processor.enum_count(), 0);
        assert_eq!(processor.integer_count(), 0);
    }

    #[test]
    fn test_domain_types_process_enum() {
        let mut processor = DomainTypesProcessor::new();
        let schema = json!({
            "enum": ["a", "b", "c"]
        })
        .as_object()
        .unwrap()
        .clone();

        let (enums, integers) = processor.process_schema(&schema, "#/");

        assert_eq!(enums, 1);
        assert_eq!(integers, 0);
        assert_eq!(processor.enum_count(), 1);
        assert_eq!(processor.integer_count(), 0);
    }

    #[test]
    fn test_domain_types_process_bounded_integer() {
        let mut processor = DomainTypesProcessor::new();
        let schema = json!({
            "type": "integer",
            "minimum": 0,
            "maximum": 100
        })
        .as_object()
        .unwrap()
        .clone();

        let (enums, integers) = processor.process_schema(&schema, "#/");

        assert_eq!(enums, 0);
        assert_eq!(integers, 1);
        assert_eq!(processor.enum_count(), 0);
        assert_eq!(processor.integer_count(), 1);
    }

    #[test]
    fn test_domain_types_process_integer_with_only_minimum() {
        let mut processor = DomainTypesProcessor::new();
        let schema = json!({
            "type": "integer",
            "minimum": 5
        })
        .as_object()
        .unwrap()
        .clone();

        let (enums, integers) = processor.process_schema(&schema, "#/");

        assert_eq!(enums, 0);
        assert_eq!(integers, 1);
    }

    #[test]
    fn test_domain_types_process_integer_with_only_maximum() {
        let mut processor = DomainTypesProcessor::new();
        let schema = json!({
            "type": "integer",
            "maximum": 50
        })
        .as_object()
        .unwrap()
        .clone();

        let (enums, integers) = processor.process_schema(&schema, "#/");

        assert_eq!(enums, 0);
        assert_eq!(integers, 1);
    }

    #[test]
    fn test_domain_types_process_string_not_domain() {
        let mut processor = DomainTypesProcessor::new();
        let schema = json!({
            "type": "string"
        })
        .as_object()
        .unwrap()
        .clone();

        let (enums, integers) = processor.process_schema(&schema, "#/");

        assert_eq!(enums, 0);
        assert_eq!(integers, 0);
    }

    #[test]
    fn test_domain_types_process_integer_without_bounds_not_domain() {
        let mut processor = DomainTypesProcessor::new();
        let schema = json!({
            "type": "integer"
        })
        .as_object()
        .unwrap()
        .clone();

        let (enums, integers) = processor.process_schema(&schema, "#/");

        assert_eq!(enums, 0);
        assert_eq!(integers, 0);
    }

    #[test]
    fn test_domain_types_combine_results_empty() {
        let processor = DomainTypesProcessor::new();
        let combined = processor.combine_results(vec![]);
        assert_eq!(combined, (0, 0));
    }

    #[test]
    fn test_domain_types_combine_results_single() {
        let processor = DomainTypesProcessor::new();
        let combined = processor.combine_results(vec![(3, 2)]);
        assert_eq!(combined, (3, 2));
    }

    #[test]
    fn test_domain_types_combine_results_multiple() {
        let processor = DomainTypesProcessor::new();
        let combined = processor.combine_results(vec![(2, 1), (1, 3), (4, 0)]);
        assert_eq!(combined, (7, 4));
    }
}
