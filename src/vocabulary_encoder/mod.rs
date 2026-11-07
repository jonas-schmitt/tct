//! Dense Encoding/Decoding with Vocabulary-Aware Token Allocation
//!
//! This module provides the complete vocabulary-aware encoding system that combines
//! schema analysis, vocabulary allocation, and dense token ID mapping for optimal
//! LLM vocabulary usage.

// Sub-modules for organized functionality
pub mod encoding;
pub mod error;
pub mod statistics;

use crate::schema_analyzer::SchemaAnalyzer;
use crate::vocabulary_allocator::{
    TokenRange, ValueDomain, VocabularyAllocation, VocabularyAllocator,
};
use rustc_hash::FxHashMap;
use serde_json::Value;

// Re-export main types
pub use error::VocabularyEncodingError;
pub use statistics::VocabularyOptimizationStats;

use encoding::EncodingEngine;

/// Complete vocabulary-aware encoding system for a schema
pub struct VocabularyAwareEncoder {
    /// Vocabulary allocation mapping fields to token ranges
    allocation: VocabularyAllocation,
    /// Value domain information for each field
    domains: FxHashMap<String, ValueDomain>,
}

impl VocabularyAwareEncoder {
    /// Create a new vocabulary-aware encoder for a JSON schema
    pub fn new(schema: &Value) -> Result<Self, VocabularyEncodingError> {
        Self::new_with_constraint(schema, None)
    }

    /// Create a new vocabulary-aware encoder with vocabulary size constraint
    pub fn new_with_constraint(
        schema: &Value,
        max_vocabulary_size: Option<usize>,
    ) -> Result<Self, VocabularyEncodingError> {
        // Analyze schema to identify value domains
        let domains_list = SchemaAnalyzer::analyze_schema(schema)
            .map_err(VocabularyEncodingError::SchemaAnalysis)?;

        // Create vocabulary allocation with constraint
        let allocation = VocabularyAllocator::allocate_with_constraint(
            domains_list.clone(),
            max_vocabulary_size,
        );

        // Build domain lookup map
        let mut domains = FxHashMap::default();
        for domain in domains_list {
            domains.insert(domain.field_path.clone(), domain);
        }

        Ok(Self {
            allocation,
            domains,
        })
    }

    /// Get the total vocabulary size (number of tokens allocated)
    pub fn vocabulary_size(&self) -> usize {
        self.allocation.total_vocabulary_size
    }

    /// Get detailed optimization statistics
    pub fn optimization_stats(&self) -> VocabularyOptimizationStats {
        let optimized_fields = self.domains.len();
        let total_constrained_values: usize = self.domains.values().map(|d| d.cardinality).sum();

        // Count domain types and calculate pattern-specific metrics first
        let mut enum_domains = 0;
        let mut bounded_integer_domains = 0;
        let mut pattern_domains = 0;
        let mut pattern_values = 0;
        let mut pattern_tokens_allocated = 0;
        let mut optimization_breakdown = FxHashMap::default();

        for domain in self.domains.values() {
            match &domain.domain_type {
                crate::vocabulary_allocator::DomainType::Enum { .. } => {
                    enum_domains += 1;
                    *optimization_breakdown
                        .entry("enum".to_string())
                        .or_insert(0) += 1;
                }
                crate::vocabulary_allocator::DomainType::BoundedInteger { .. } => {
                    bounded_integer_domains += 1;
                    *optimization_breakdown
                        .entry("bounded_integer".to_string())
                        .or_insert(0) += 1;
                }
                crate::vocabulary_allocator::DomainType::Boolean => {
                    *optimization_breakdown
                        .entry("boolean".to_string())
                        .or_insert(0) += 1;
                }
                crate::vocabulary_allocator::DomainType::PatternEnumeration(values) => {
                    pattern_domains += 1;
                    // For pattern enumeration, calculate naive token cost based on string lengths
                    let naive_tokens: usize = values.iter().map(|s| s.len()).sum();
                    pattern_values += naive_tokens;
                    pattern_tokens_allocated += domain.cardinality; // 1 token per enum value
                    *optimization_breakdown
                        .entry("pattern_enum".to_string())
                        .or_insert(0) += 1;
                }
                crate::vocabulary_allocator::DomainType::PatternBounded(range) => {
                    pattern_domains += 1;
                    // For pattern bounded, naive encoding would use string representation
                    // Estimate based on average string length for the range
                    let avg_string_length = (range.total_values as f64).log10().ceil() as usize + 1;
                    let naive_tokens = domain.cardinality * avg_string_length;
                    pattern_values += naive_tokens;
                    pattern_tokens_allocated += domain.cardinality;
                    *optimization_breakdown
                        .entry("pattern_bounded".to_string())
                        .or_insert(0) += 1;
                }
            }
        }

        // Calculate pattern-specific statistics
        let pattern_tokens_saved = pattern_values.saturating_sub(pattern_tokens_allocated);

        let pattern_vocabulary_reduction = if pattern_values > 0 {
            (pattern_tokens_saved as f64 / pattern_values as f64) * 100.0
        } else {
            0.0
        };

        // Now calculate overall vocabulary statistics including pattern optimizations
        let naive_vocabulary_size = total_constrained_values + pattern_tokens_saved;
        let distinct_structural_tokens_count = self.vocabulary_size();

        // Calculate max token ID that can appear in the stream
        // This is needed for BPE base_vocab to prevent collision
        // IMPORTANT: Use actual allocated token ranges, not assumed 0-based values
        let max_token_id = self
            .domains
            .iter()
            .filter_map(|(field_path, domain)| {
                // Get the actual allocated token range for this domain
                let token_range = self.allocation.domain_allocations.get(field_path)?;

                match &domain.domain_type {
                    crate::vocabulary_allocator::DomainType::Boolean => Some(1), // Uses token IDs 0, 1
                    crate::vocabulary_allocator::DomainType::BoundedInteger { min: _, max: _ } => {
                        // Use actual allocated token range, not the integer's max value
                        // The integer's value range doesn't determine which token IDs are used
                        Some(token_range.end - 1)
                    }
                    crate::vocabulary_allocator::DomainType::Enum { .. } => {
                        // For enums with numeric values, use the max numeric value if available
                        // Otherwise fall back to the allocated token range
                        domain
                            .logical_values
                            .iter()
                            .filter_map(|v| match v {
                                serde_json::Value::Number(n) => n.as_u64().map(|u| u as u32),
                                serde_json::Value::Bool(true) => Some(1),
                                serde_json::Value::Bool(false) => Some(0),
                                _ => None,
                            })
                            .max()
                            .or(Some(token_range.end - 1))
                    }
                    crate::vocabulary_allocator::DomainType::PatternEnumeration(_) => {
                        // Use actual allocated token range, not assumed 0-based cardinality
                        Some(token_range.end - 1)
                    }
                    crate::vocabulary_allocator::DomainType::PatternBounded(_) => {
                        // Use actual allocated token range, not assumed 0-based cardinality
                        Some(token_range.end - 1)
                    }
                }
            })
            .max()
            .unwrap_or(0);
        let max_structural_token_id_plus_one = (max_token_id + 1) as usize;

        let tokens_saved = naive_vocabulary_size.saturating_sub(distinct_structural_tokens_count);
        let reduction_percentage = if naive_vocabulary_size > 0 {
            (tokens_saved as f64 / naive_vocabulary_size as f64) * 100.0
        } else {
            0.0
        };

        VocabularyOptimizationStats {
            optimized_fields,
            total_constrained_values,
            naive_vocabulary_size,
            distinct_structural_tokens_count,
            tokens_saved,
            reduction_percentage,
            enum_domains,
            bounded_integer_domains,
            optimization_breakdown,
            // Pattern stats - Now implemented
            pattern_domains,
            pattern_vocabulary_reduction,
            pattern_tokens_saved,
            pattern_tokens_allocated,
            max_structural_token_id_plus_one,
            estimated_unoptimized_tokens: naive_vocabulary_size,
            total_vocabulary_reduction: reduction_percentage,
        }
    }

    /// Get information about a specific domain
    pub fn get_domain(&self, field_path: &str) -> Option<&ValueDomain> {
        self.domains.get(field_path)
    }

    /// Get all domain field paths
    pub fn get_domain_keys(&self) -> Vec<&String> {
        self.domains.keys().collect()
    }

    /// Encode a specific field value to its optimal token ID
    pub fn encode_field(
        &self,
        field_path: &str,
        value: &Value,
    ) -> Result<u32, VocabularyEncodingError> {
        let engine = EncodingEngine::new(&self.allocation, &self.domains);
        engine.encode_field(field_path, value)
    }

    /// Decode a token ID back to its logical value for a specific field
    pub fn decode_field(
        &self,
        field_path: &str,
        token_id: u32,
    ) -> Result<Value, VocabularyEncodingError> {
        let engine = EncodingEngine::new(&self.allocation, &self.domains);
        engine.decode_field(field_path, token_id)
    }

    /// Get the token range for a specific field
    pub fn get_field_range(&self, field_path: &str) -> Option<&TokenRange> {
        self.allocation.get_range(field_path)
    }

    /// Get list of all optimized field names
    pub fn get_optimized_fields(&self) -> Vec<&str> {
        self.domains.keys().map(|s| s.as_str()).collect()
    }

    /// Check if a field is optimized
    pub fn is_field_optimized(&self, field_path: &str) -> bool {
        self.domains.contains_key(field_path)
    }

    /// Create a batch encoder for efficient bulk operations
    pub fn batch_encoder(&self) -> BatchVocabularyEncoder<'_> {
        BatchVocabularyEncoder::new(self)
    }
}

/// Batch encoder for processing multiple values efficiently
pub struct BatchVocabularyEncoder<'a> {
    encoder: &'a VocabularyAwareEncoder,
}

impl<'a> BatchVocabularyEncoder<'a> {
    /// Create a new batch encoder
    pub fn new(encoder: &'a VocabularyAwareEncoder) -> Self {
        Self { encoder }
    }

    /// Encode multiple field-value pairs in a single operation
    pub fn encode_batch(
        &self,
        field_values: &[(String, Value)],
    ) -> Result<Vec<u32>, VocabularyEncodingError> {
        let engine = EncodingEngine::new(&self.encoder.allocation, &self.encoder.domains);
        engine.encode_batch(field_values)
    }

    /// Decode multiple field-token pairs in a single operation
    pub fn decode_batch(
        &self,
        field_tokens: &[(String, u32)],
    ) -> Result<Vec<Value>, VocabularyEncodingError> {
        let engine = EncodingEngine::new(&self.encoder.allocation, &self.encoder.domains);
        engine.decode_batch(field_tokens)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_encoder_creation() {
        let schema = json!({
            "type": "object",
            "properties": {
                "status": {
                    "type": "string",
                    "enum": ["active", "inactive", "pending"]
                }
            }
        });

        let encoder = VocabularyAwareEncoder::new(&schema).unwrap();
        assert_eq!(encoder.vocabulary_size(), 3);
        assert!(encoder.is_field_optimized("status"));
    }

    #[test]
    fn test_encode_decode_roundtrip() {
        let schema = json!({
            "type": "object",
            "properties": {
                "status": {
                    "type": "string",
                    "enum": ["active", "inactive"]
                }
            }
        });

        let encoder = VocabularyAwareEncoder::new(&schema).unwrap();

        let original_value = Value::String("active".to_string());
        let token_id = encoder.encode_field("status", &original_value).unwrap();
        let decoded_value = encoder.decode_field("status", token_id).unwrap();

        assert_eq!(original_value, decoded_value);
    }

    #[test]
    fn test_optimization_stats() {
        let schema = json!({
            "type": "object",
            "properties": {
                "status": {"type": "string", "enum": ["a", "b", "c"]},
                "flag": {"type": "boolean"}
            }
        });

        let encoder = VocabularyAwareEncoder::new(&schema).unwrap();
        let stats = encoder.optimization_stats();

        assert_eq!(stats.optimized_fields, 2);
        assert_eq!(stats.total_constrained_values, 5); // 3 enum + 2 boolean
        assert!(stats.reduction_percentage >= 0.0); // >= 0 since small schemas might not benefit from optimization
        assert!(stats.distinct_structural_tokens_count > 0);
    }

    #[test]
    fn test_max_structural_token_id_with_actual_allocation() {
        // This test exposes a bug where max_structural_token_id_plus_one
        // incorrectly assumes 0-based token allocation for certain domain types.
        //
        // Schema: enum (3 values) + bounded integer 0-500
        // The enum gets allocated first (e.g., tokens 0-2)
        // The bounded integer gets allocated after (e.g., tokens 3-503)
        // Bug: code assumes bounded integer uses tokens 0-500 (max_token_id = 500)
        // Correct: should use actual allocated range (max_token_id = 503)
        let schema = json!({
            "type": "object",
            "properties": {
                "status": {"type": "string", "enum": ["a", "b", "c"]},
                "count": {"type": "integer", "minimum": 0, "maximum": 500}
            },
            "required": ["status", "count"]
        });

        let encoder = VocabularyAwareEncoder::new(&schema).unwrap();
        let stats = encoder.optimization_stats();

        // Get actual token allocations to determine expected max_token_id
        let status_range = encoder.allocation.domain_allocations.get("status").unwrap();
        let count_range = encoder.allocation.domain_allocations.get("count").unwrap();

        let actual_max_token_id = status_range.end.max(count_range.end) - 1;
        let expected_max_plus_one = (actual_max_token_id + 1) as usize;

        assert_eq!(
            stats.max_structural_token_id_plus_one, expected_max_plus_one,
            "max_structural_token_id_plus_one should reflect actual allocated token ranges, not assumed 0-based values. \
            status_range: {:?}, count_range: {:?}",
            status_range, count_range
        );
    }

    #[test]
    fn test_max_structural_token_id_with_large_bounded_integer() {
        // Test with a bounded integer that has a large maximum value
        // This ensures the fix works correctly for integers with values >> cardinality
        let schema = json!({
            "type": "object",
            "properties": {
                "id": {"type": "integer", "minimum": 100, "maximum": 10000}
            }
        });

        let encoder = VocabularyAwareEncoder::new(&schema).unwrap();
        let stats = encoder.optimization_stats();

        let id_range = encoder.allocation.domain_allocations.get("id").unwrap();
        let expected_max_plus_one = id_range.end as usize;

        assert_eq!(
            stats.max_structural_token_id_plus_one, expected_max_plus_one,
            "max_structural_token_id_plus_one should use actual token range end, not integer max value. \
            id_range: {:?}",
            id_range
        );

        // Verify it's NOT using the integer's max value (10000)
        assert_ne!(
            stats.max_structural_token_id_plus_one,
            10001, // Would be 10001 if using integer max value + 1
            "Should not be using the integer's max value"
        );
    }

    #[test]
    fn test_max_structural_token_id_with_multiple_domains() {
        // Test with multiple domains of different types to ensure we find the true maximum
        let schema = json!({
            "type": "object",
            "properties": {
                "flag": {"type": "boolean"},
                "status": {"type": "string", "enum": ["a", "b"]},
                "priority": {"type": "integer", "minimum": 1, "maximum": 10},
                "category": {"type": "string", "enum": ["x", "y", "z", "w"]}
            }
        });

        let encoder = VocabularyAwareEncoder::new(&schema).unwrap();
        let stats = encoder.optimization_stats();

        // Find the actual maximum token ID across all domains
        let mut max_token_id = 0u32;
        for range in encoder.allocation.domain_allocations.values() {
            max_token_id = max_token_id.max(range.end - 1);
        }
        let expected_max_plus_one = (max_token_id + 1) as usize;

        assert_eq!(
            stats.max_structural_token_id_plus_one, expected_max_plus_one,
            "max_structural_token_id_plus_one should reflect the highest allocated token across all domains"
        );
    }

    #[test]
    fn test_max_structural_token_id_with_single_boolean() {
        // Edge case: single boolean field (simplest possible case)
        let schema = json!({
            "type": "object",
            "properties": {
                "flag": {"type": "boolean"}
            }
        });

        let encoder = VocabularyAwareEncoder::new(&schema).unwrap();
        let stats = encoder.optimization_stats();

        let flag_range = encoder.allocation.domain_allocations.get("flag").unwrap();
        // Boolean uses token IDs 0 and 1, so max_token_id = 1
        assert_eq!(flag_range.start, 0);
        assert_eq!(flag_range.end, 2); // Exclusive end
        assert_eq!(stats.max_structural_token_id_plus_one, 2);
    }

    #[test]
    fn test_batch_operations() {
        let schema = json!({
            "type": "object",
            "properties": {
                "status": {"type": "string", "enum": ["active", "inactive"]},
                "flag": {"type": "boolean"}
            }
        });

        let encoder = VocabularyAwareEncoder::new(&schema).unwrap();
        let batch_encoder = encoder.batch_encoder();

        // Test batch encoding
        let field_values = vec![
            ("status".to_string(), Value::String("active".to_string())),
            ("flag".to_string(), Value::Bool(true)),
        ];
        let encoded = batch_encoder.encode_batch(&field_values).unwrap();
        assert_eq!(encoded.len(), 2);

        // Test batch decoding roundtrip
        let field_tokens: Vec<(String, u32)> = vec![
            ("status".to_string(), encoded[0]),
            ("flag".to_string(), encoded[1]),
        ];
        let decoded = batch_encoder.decode_batch(&field_tokens).unwrap();

        assert_eq!(decoded[0], Value::String("active".to_string()));
        assert_eq!(decoded[1], Value::Bool(true));
    }

    // ========== Encoder Creation Tests ==========

    #[test]
    fn test_new_with_constraint() {
        let schema = json!({
            "type": "object",
            "properties": {
                "status": {"type": "string", "enum": ["a", "b", "c", "d", "e"]}
            }
        });

        let encoder = VocabularyAwareEncoder::new_with_constraint(&schema, Some(3)).unwrap();
        assert!(encoder.vocabulary_size() <= 3);
    }

    #[test]
    fn test_new_with_no_constraint() {
        let schema = json!({
            "type": "object",
            "properties": {
                "status": {"type": "string", "enum": ["a", "b", "c"]}
            }
        });

        let encoder = VocabularyAwareEncoder::new_with_constraint(&schema, None).unwrap();
        assert_eq!(encoder.vocabulary_size(), 3);
    }

    #[test]
    fn test_new_with_invalid_schema() {
        let schema = json!({
            "type": "invalid_type"
        });

        // Should handle schema analysis gracefully
        let result = VocabularyAwareEncoder::new(&schema);
        // Schema analyzer may return empty domains for unsupported types
        assert!(result.is_ok() || result.is_err());
    }

    // ========== Domain Query Tests ==========

    #[test]
    fn test_get_domain() {
        let schema = json!({
            "type": "object",
            "properties": {
                "status": {"type": "string", "enum": ["active", "inactive"]}
            }
        });

        let encoder = VocabularyAwareEncoder::new(&schema).unwrap();
        let domain = encoder.get_domain("status");
        assert!(domain.is_some());
        assert_eq!(domain.unwrap().cardinality, 2);
    }

    #[test]
    fn test_get_domain_not_found() {
        let schema = json!({
            "type": "object",
            "properties": {
                "status": {"type": "string", "enum": ["active", "inactive"]}
            }
        });

        let encoder = VocabularyAwareEncoder::new(&schema).unwrap();
        let domain = encoder.get_domain("unknown_field");
        assert!(domain.is_none());
    }

    #[test]
    fn test_get_domain_keys() {
        let schema = json!({
            "type": "object",
            "properties": {
                "status": {"type": "string", "enum": ["active"]},
                "flag": {"type": "boolean"}
            }
        });

        let encoder = VocabularyAwareEncoder::new(&schema).unwrap();
        let keys = encoder.get_domain_keys();
        assert_eq!(keys.len(), 2);
        assert!(keys.iter().any(|k| k.as_str() == "status"));
        assert!(keys.iter().any(|k| k.as_str() == "flag"));
    }

    #[test]
    fn test_get_field_range() {
        let schema = json!({
            "type": "object",
            "properties": {
                "status": {"type": "string", "enum": ["active", "inactive"]}
            }
        });

        let encoder = VocabularyAwareEncoder::new(&schema).unwrap();
        let range = encoder.get_field_range("status");
        assert!(range.is_some());
    }

    #[test]
    fn test_get_field_range_not_found() {
        let schema = json!({
            "type": "object",
            "properties": {
                "status": {"type": "string", "enum": ["active"]}
            }
        });

        let encoder = VocabularyAwareEncoder::new(&schema).unwrap();
        let range = encoder.get_field_range("unknown_field");
        assert!(range.is_none());
    }

    #[test]
    fn test_get_optimized_fields() {
        let schema = json!({
            "type": "object",
            "properties": {
                "status": {"type": "string", "enum": ["active"]},
                "priority": {"type": "integer", "minimum": 1, "maximum": 5}
            }
        });

        let encoder = VocabularyAwareEncoder::new(&schema).unwrap();
        let fields = encoder.get_optimized_fields();
        assert_eq!(fields.len(), 2);
        assert!(fields.contains(&"status"));
        assert!(fields.contains(&"priority"));
    }

    #[test]
    fn test_is_field_optimized_true() {
        let schema = json!({
            "type": "object",
            "properties": {
                "status": {"type": "string", "enum": ["active"]}
            }
        });

        let encoder = VocabularyAwareEncoder::new(&schema).unwrap();
        assert!(encoder.is_field_optimized("status"));
    }

    #[test]
    fn test_is_field_optimized_false() {
        let schema = json!({
            "type": "object",
            "properties": {
                "status": {"type": "string", "enum": ["active"]}
            }
        });

        let encoder = VocabularyAwareEncoder::new(&schema).unwrap();
        assert!(!encoder.is_field_optimized("unknown_field"));
    }

    // ========== Optimization Stats Tests ==========

    #[test]
    fn test_optimization_stats_with_bounded_integer() {
        let schema = json!({
            "type": "object",
            "properties": {
                "priority": {"type": "integer", "minimum": 1, "maximum": 10}
            }
        });

        let encoder = VocabularyAwareEncoder::new(&schema).unwrap();
        let stats = encoder.optimization_stats();

        assert_eq!(stats.optimized_fields, 1);
        assert_eq!(stats.bounded_integer_domains, 1);
        assert_eq!(stats.enum_domains, 0);
    }

    #[test]
    fn test_optimization_stats_with_boolean() {
        let schema = json!({
            "type": "object",
            "properties": {
                "flag": {"type": "boolean"}
            }
        });

        let encoder = VocabularyAwareEncoder::new(&schema).unwrap();
        let stats = encoder.optimization_stats();

        assert_eq!(stats.optimized_fields, 1);
        assert!(stats.optimization_breakdown.contains_key("boolean"));
    }

    #[test]
    fn test_optimization_stats_breakdown() {
        let schema = json!({
            "type": "object",
            "properties": {
                "status": {"type": "string", "enum": ["a", "b"]},
                "priority": {"type": "integer", "minimum": 1, "maximum": 5},
                "flag": {"type": "boolean"}
            }
        });

        let encoder = VocabularyAwareEncoder::new(&schema).unwrap();
        let stats = encoder.optimization_stats();

        assert_eq!(stats.enum_domains, 1);
        assert_eq!(stats.bounded_integer_domains, 1);
        assert!(stats.optimization_breakdown.contains_key("enum"));
        assert!(stats.optimization_breakdown.contains_key("bounded_integer"));
        assert!(stats.optimization_breakdown.contains_key("boolean"));
    }

    #[test]
    fn test_optimization_stats_empty_schema() {
        let schema = json!({
            "type": "object",
            "properties": {}
        });

        let encoder = VocabularyAwareEncoder::new(&schema).unwrap();
        let stats = encoder.optimization_stats();

        assert_eq!(stats.optimized_fields, 0);
        assert_eq!(stats.total_constrained_values, 0);
        assert_eq!(stats.reduction_percentage, 0.0);
    }

    // ========== Batch Encoder Tests ==========

    #[test]
    fn test_batch_encoder_new() {
        let schema = json!({
            "type": "object",
            "properties": {
                "status": {"type": "string", "enum": ["active"]}
            }
        });

        let encoder = VocabularyAwareEncoder::new(&schema).unwrap();
        let _batch_encoder = BatchVocabularyEncoder::new(&encoder);
    }

    #[test]
    fn test_batch_encoder_empty_batch() {
        let schema = json!({
            "type": "object",
            "properties": {
                "status": {"type": "string", "enum": ["active"]}
            }
        });

        let encoder = VocabularyAwareEncoder::new(&schema).unwrap();
        let batch_encoder = encoder.batch_encoder();

        let field_values: Vec<(String, Value)> = vec![];
        let encoded = batch_encoder.encode_batch(&field_values).unwrap();
        assert_eq!(encoded.len(), 0);

        let field_tokens: Vec<(String, u32)> = vec![];
        let decoded = batch_encoder.decode_batch(&field_tokens).unwrap();
        assert_eq!(decoded.len(), 0);
    }

    // ========== Encode/Decode Tests ==========

    #[test]
    fn test_encode_field_success() {
        let schema = json!({
            "type": "object",
            "properties": {
                "status": {"type": "string", "enum": ["active", "inactive"]}
            }
        });

        let encoder = VocabularyAwareEncoder::new(&schema).unwrap();
        let token = encoder.encode_field("status", &Value::String("active".to_string()));
        assert!(token.is_ok());
    }

    #[test]
    fn test_encode_field_invalid_value() {
        let schema = json!({
            "type": "object",
            "properties": {
                "status": {"type": "string", "enum": ["active"]}
            }
        });

        let encoder = VocabularyAwareEncoder::new(&schema).unwrap();
        let result = encoder.encode_field("status", &Value::String("invalid".to_string()));
        assert!(result.is_err());
    }

    #[test]
    fn test_decode_field_success() {
        let schema = json!({
            "type": "object",
            "properties": {
                "status": {"type": "string", "enum": ["active"]}
            }
        });

        let encoder = VocabularyAwareEncoder::new(&schema).unwrap();
        let result = encoder.decode_field("status", 0);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Value::String("active".to_string()));
    }

    #[test]
    fn test_vocabulary_size() {
        let schema = json!({
            "type": "object",
            "properties": {
                "status": {"type": "string", "enum": ["a", "b", "c"]}
            }
        });

        let encoder = VocabularyAwareEncoder::new(&schema).unwrap();
        assert_eq!(encoder.vocabulary_size(), 3);
    }
}
