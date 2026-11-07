//! Core encoding and decoding strategies
//!
//! This module provides the fundamental encoding and decoding operations
//! for converting between logical values and optimal token IDs.

use crate::vocabulary_allocator::{ValueDomain, VocabularyAllocation};
use rustc_hash::FxHashMap;
use serde_json::Value;

use super::error::VocabularyEncodingError;

/// Core encoding and decoding operations
pub struct EncodingEngine<'a> {
    /// Vocabulary allocation mapping fields to token ranges
    allocation: &'a VocabularyAllocation,
    /// Value domain information for each field
    domains: &'a FxHashMap<String, ValueDomain>,
}

impl<'a> EncodingEngine<'a> {
    /// Create a new encoding engine with allocation and domain data
    pub fn new(
        allocation: &'a VocabularyAllocation,
        domains: &'a FxHashMap<String, ValueDomain>,
    ) -> Self {
        Self {
            allocation,
            domains,
        }
    }

    /// Encode a specific field value to its optimal token ID
    pub fn encode_field(
        &self,
        field_path: &str,
        value: &Value,
    ) -> Result<u32, VocabularyEncodingError> {
        // Get the domain for this field
        let domain = self
            .domains
            .get(field_path)
            .ok_or_else(|| VocabularyEncodingError::UnknownField(field_path.to_string()))?;

        // Get the token range allocation
        let range = self
            .allocation
            .get_range(field_path)
            .ok_or_else(|| VocabularyEncodingError::NoAllocation(field_path.to_string()))?;

        // Find logical index of value using optimized strategy
        let logical_index = self.find_logical_index(domain, value, field_path)?;

        // Map to dense token ID using range allocation
        let token_id = range.logical_index_to_token(logical_index);
        // Release-visible invariant: allocation ranges must sit inside VOCAB_SIZE.
        assert!(
            (token_id as usize) < self.allocation.total_vocabulary_size,
            "encoded token ID {} for field '{}' exceeds vocabulary size {}",
            token_id,
            field_path,
            self.allocation.total_vocabulary_size
        );
        Ok(token_id)
    }

    /// Decode a token ID back to its logical value for a specific field
    pub fn decode_field(
        &self,
        field_path: &str,
        token_id: u32,
    ) -> Result<Value, VocabularyEncodingError> {
        // Get the domain for this field
        let domain = self
            .domains
            .get(field_path)
            .ok_or_else(|| VocabularyEncodingError::UnknownField(field_path.to_string()))?;

        // Get the token range allocation
        let range = self
            .allocation
            .get_range(field_path)
            .ok_or_else(|| VocabularyEncodingError::NoAllocation(field_path.to_string()))?;

        // Map token ID to logical index (with modulo for LLM robustness)
        let logical_index = range.token_to_logical_index(token_id);

        // Return the logical value using robust decoding
        self.get_logical_value(domain, logical_index)
    }

    /// Batch encode multiple field values efficiently
    pub fn encode_batch(
        &self,
        field_values: &[(String, Value)],
    ) -> Result<Vec<u32>, VocabularyEncodingError> {
        let mut results = Vec::with_capacity(field_values.len());

        for (field_path, value) in field_values {
            results.push(self.encode_field(field_path, value)?);
        }

        Ok(results)
    }

    /// Batch decode multiple token IDs efficiently
    pub fn decode_batch(
        &self,
        field_tokens: &[(String, u32)],
    ) -> Result<Vec<Value>, VocabularyEncodingError> {
        let mut results = Vec::with_capacity(field_tokens.len());

        for (field_path, token_id) in field_tokens {
            results.push(self.decode_field(field_path, *token_id)?);
        }

        Ok(results)
    }

    /// Find the logical index of a value within a domain
    fn find_logical_index(
        &self,
        domain: &ValueDomain,
        value: &Value,
        field_path: &str,
    ) -> Result<usize, VocabularyEncodingError> {
        // Use optimized search based on domain type
        match domain.logical_values.iter().position(|v| v == value) {
            Some(index) => Ok(index),
            None => Err(VocabularyEncodingError::InvalidValue(
                field_path.to_string(),
                value.clone(),
            )),
        }
    }

    /// Get logical value with bounds checking
    fn get_logical_value(
        &self,
        domain: &ValueDomain,
        logical_index: usize,
    ) -> Result<Value, VocabularyEncodingError> {
        domain
            .logical_values
            .get(logical_index)
            .cloned()
            .ok_or_else(|| {
                VocabularyEncodingError::InvalidValue(
                    "decoded_value".to_string(),
                    Value::Number(logical_index.into()),
                )
            })
    }
}

/// Strategy-specific encoding optimizations
pub mod strategies {
    use super::*;

    /// Optimized encoding for enumeration domains
    pub fn encode_enum_optimized(values: &[Value], target: &Value) -> Option<usize> {
        // For small enums, use linear search (cache-friendly)
        if values.len() <= 8 {
            values.iter().position(|v| v == target)
        } else {
            // For larger enums, could use hash map lookup in future
            values.iter().position(|v| v == target)
        }
    }

    /// Optimized encoding for bounded integer domains
    pub fn encode_integer_optimized(
        min_value: i64,
        max_value: i64,
        target_value: i64,
    ) -> Option<usize> {
        if target_value >= min_value && target_value <= max_value {
            Some((target_value - min_value) as usize)
        } else {
            None
        }
    }

    /// Optimized decoding for bounded integer domains
    pub fn decode_integer_optimized(min_value: i64, logical_index: usize) -> i64 {
        min_value + logical_index as i64
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vocabulary_allocator::{DomainType, VocabularyAllocation, VocabularyAllocator};

    fn create_test_domain() -> ValueDomain {
        ValueDomain {
            field_path: "status".to_string(),
            domain_type: DomainType::Enum {
                values: vec!["active".to_string(), "inactive".to_string()],
            },
            cardinality: 2,
            logical_values: vec![
                Value::String("active".to_string()),
                Value::String("inactive".to_string()),
            ],
        }
    }

    fn create_test_allocation() -> VocabularyAllocation {
        let domain = create_test_domain();
        VocabularyAllocator::allocate(vec![domain])
    }

    #[test]
    fn test_encode_field() {
        let domain = create_test_domain();
        let allocation = create_test_allocation();
        let mut domains = FxHashMap::default();
        domains.insert("status".to_string(), domain);

        let engine = EncodingEngine::new(&allocation, &domains);

        let result = engine.encode_field("status", &Value::String("active".to_string()));
        assert_eq!(result.unwrap(), 0);

        let result = engine.encode_field("status", &Value::String("inactive".to_string()));
        assert_eq!(result.unwrap(), 1);
    }

    #[test]
    fn test_decode_field() {
        let domain = create_test_domain();
        let allocation = create_test_allocation();
        let mut domains = FxHashMap::default();
        domains.insert("status".to_string(), domain);

        let engine = EncodingEngine::new(&allocation, &domains);

        let result = engine.decode_field("status", 0);
        assert_eq!(result.unwrap(), Value::String("active".to_string()));

        let result = engine.decode_field("status", 1);
        assert_eq!(result.unwrap(), Value::String("inactive".to_string()));
    }

    #[test]
    fn test_batch_operations() {
        let domain = create_test_domain();
        let allocation = create_test_allocation();
        let mut domains = FxHashMap::default();
        domains.insert("status".to_string(), domain);

        let engine = EncodingEngine::new(&allocation, &domains);

        // Test batch encoding
        let field_values = vec![
            ("status".to_string(), Value::String("active".to_string())),
            ("status".to_string(), Value::String("inactive".to_string())),
        ];
        let encoded = engine.encode_batch(&field_values).unwrap();
        assert_eq!(encoded, vec![0, 1]);

        // Test batch decoding
        let field_tokens = vec![("status".to_string(), 0), ("status".to_string(), 1)];
        let decoded = engine.decode_batch(&field_tokens).unwrap();
        assert_eq!(
            decoded,
            vec![
                Value::String("active".to_string()),
                Value::String("inactive".to_string())
            ]
        );
    }

    // ========== Error Handling Tests ==========

    #[test]
    fn test_encode_field_unknown_field_error() {
        let domain = create_test_domain();
        let allocation = create_test_allocation();
        let mut domains = FxHashMap::default();
        domains.insert("status".to_string(), domain);

        let engine = EncodingEngine::new(&allocation, &domains);

        let result = engine.encode_field("unknown_field", &Value::String("value".to_string()));
        assert!(result.is_err());
        match result.unwrap_err() {
            VocabularyEncodingError::UnknownField(field) => {
                assert_eq!(field, "unknown_field");
            }
            _ => panic!("Expected UnknownField error"),
        }
    }

    #[test]
    fn test_encode_field_invalid_value_error() {
        let domain = create_test_domain();
        let allocation = create_test_allocation();
        let mut domains = FxHashMap::default();
        domains.insert("status".to_string(), domain);

        let engine = EncodingEngine::new(&allocation, &domains);

        let result = engine.encode_field("status", &Value::String("invalid_value".to_string()));
        assert!(result.is_err());
        match result.unwrap_err() {
            VocabularyEncodingError::InvalidValue(field, value) => {
                assert_eq!(field, "status");
                assert_eq!(value, Value::String("invalid_value".to_string()));
            }
            _ => panic!("Expected InvalidValue error"),
        }
    }

    #[test]
    #[should_panic(expected = "encoded token ID")]
    fn test_encode_field_debug_asserts_token_below_vocabulary_size() {
        let domain = create_test_domain();
        let mut allocation = VocabularyAllocation::new();
        allocation.total_vocabulary_size = 1;
        allocation.domain_allocations.insert(
            "status".to_string(),
            crate::vocabulary_allocator::TokenRange::new(1, 2),
        );

        let mut domains = FxHashMap::default();
        domains.insert("status".to_string(), domain);

        let engine = EncodingEngine::new(&allocation, &domains);

        let _ = engine.encode_field("status", &Value::String("active".to_string()));
    }

    #[test]
    fn test_decode_field_unknown_field_error() {
        let domain = create_test_domain();
        let allocation = create_test_allocation();
        let mut domains = FxHashMap::default();
        domains.insert("status".to_string(), domain);

        let engine = EncodingEngine::new(&allocation, &domains);

        let result = engine.decode_field("unknown_field", 0);
        assert!(result.is_err());
        match result.unwrap_err() {
            VocabularyEncodingError::UnknownField(field) => {
                assert_eq!(field, "unknown_field");
            }
            _ => panic!("Expected UnknownField error"),
        }
    }

    #[test]
    fn test_encode_batch_with_error() {
        let domain = create_test_domain();
        let allocation = create_test_allocation();
        let mut domains = FxHashMap::default();
        domains.insert("status".to_string(), domain);

        let engine = EncodingEngine::new(&allocation, &domains);

        // Batch with invalid value
        let field_values = vec![
            ("status".to_string(), Value::String("active".to_string())),
            ("status".to_string(), Value::String("invalid".to_string())),
        ];
        let result = engine.encode_batch(&field_values);
        assert!(result.is_err());
    }

    #[test]
    fn test_decode_batch_with_error() {
        let domain = create_test_domain();
        let allocation = create_test_allocation();
        let mut domains = FxHashMap::default();
        domains.insert("status".to_string(), domain);

        let engine = EncodingEngine::new(&allocation, &domains);

        // Batch with unknown field
        let field_tokens = vec![("status".to_string(), 0), ("unknown_field".to_string(), 1)];
        let result = engine.decode_batch(&field_tokens);
        assert!(result.is_err());
    }

    #[test]
    fn test_encode_batch_empty() {
        let domain = create_test_domain();
        let allocation = create_test_allocation();
        let mut domains = FxHashMap::default();
        domains.insert("status".to_string(), domain);

        let engine = EncodingEngine::new(&allocation, &domains);

        let field_values: Vec<(String, Value)> = vec![];
        let encoded = engine.encode_batch(&field_values).unwrap();
        assert_eq!(encoded, Vec::<u32>::new());
    }

    #[test]
    fn test_decode_batch_empty() {
        let domain = create_test_domain();
        let allocation = create_test_allocation();
        let mut domains = FxHashMap::default();
        domains.insert("status".to_string(), domain);

        let engine = EncodingEngine::new(&allocation, &domains);

        let field_tokens: Vec<(String, u32)> = vec![];
        let decoded = engine.decode_batch(&field_tokens).unwrap();
        assert_eq!(decoded, Vec::<Value>::new());
    }

    // ========== Strategy Function Tests ==========

    #[test]
    fn test_encode_enum_optimized_small() {
        let values = vec![
            Value::String("a".to_string()),
            Value::String("b".to_string()),
            Value::String("c".to_string()),
        ];

        let result = strategies::encode_enum_optimized(&values, &Value::String("b".to_string()));
        assert_eq!(result, Some(1));

        let result = strategies::encode_enum_optimized(&values, &Value::String("d".to_string()));
        assert_eq!(result, None);
    }

    #[test]
    fn test_encode_enum_optimized_large() {
        let values: Vec<Value> = (0..10)
            .map(|i| Value::String(format!("value{}", i)))
            .collect();

        let result =
            strategies::encode_enum_optimized(&values, &Value::String("value5".to_string()));
        assert_eq!(result, Some(5));

        let result =
            strategies::encode_enum_optimized(&values, &Value::String("invalid".to_string()));
        assert_eq!(result, None);
    }

    #[test]
    fn test_encode_enum_optimized_exactly_8_values() {
        let values: Vec<Value> = (0..8).map(|i| Value::String(format!("v{}", i))).collect();

        let result = strategies::encode_enum_optimized(&values, &Value::String("v7".to_string()));
        assert_eq!(result, Some(7));
    }

    #[test]
    fn test_encode_integer_optimized_in_range() {
        let result = strategies::encode_integer_optimized(10, 20, 15);
        assert_eq!(result, Some(5)); // 15 - 10 = 5
    }

    #[test]
    fn test_encode_integer_optimized_at_min() {
        let result = strategies::encode_integer_optimized(10, 20, 10);
        assert_eq!(result, Some(0));
    }

    #[test]
    fn test_encode_integer_optimized_at_max() {
        let result = strategies::encode_integer_optimized(10, 20, 20);
        assert_eq!(result, Some(10));
    }

    #[test]
    fn test_encode_integer_optimized_below_range() {
        let result = strategies::encode_integer_optimized(10, 20, 5);
        assert_eq!(result, None);
    }

    #[test]
    fn test_encode_integer_optimized_above_range() {
        let result = strategies::encode_integer_optimized(10, 20, 25);
        assert_eq!(result, None);
    }

    #[test]
    fn test_decode_integer_optimized() {
        let result = strategies::decode_integer_optimized(10, 5);
        assert_eq!(result, 15); // 10 + 5 = 15
    }

    #[test]
    fn test_decode_integer_optimized_at_min() {
        let result = strategies::decode_integer_optimized(10, 0);
        assert_eq!(result, 10);
    }

    #[test]
    fn test_decode_integer_optimized_negative_min() {
        let result = strategies::decode_integer_optimized(-10, 5);
        assert_eq!(result, -5); // -10 + 5 = -5
    }

    // ========== Edge Cases ==========

    #[test]
    fn test_engine_new() {
        let domain = create_test_domain();
        let allocation = create_test_allocation();
        let mut domains = FxHashMap::default();
        domains.insert("status".to_string(), domain);

        let _engine = EncodingEngine::new(&allocation, &domains);
        // Test that constructor works correctly
    }

    #[test]
    fn test_find_logical_index_first_value() {
        let domain = create_test_domain();
        let allocation = create_test_allocation();
        let mut domains = FxHashMap::default();
        domains.insert("status".to_string(), domain.clone());

        let engine = EncodingEngine::new(&allocation, &domains);

        let result =
            engine.find_logical_index(&domain, &Value::String("active".to_string()), "status");
        assert_eq!(result.unwrap(), 0);
    }

    #[test]
    fn test_find_logical_index_last_value() {
        let domain = create_test_domain();
        let allocation = create_test_allocation();
        let mut domains = FxHashMap::default();
        domains.insert("status".to_string(), domain.clone());

        let engine = EncodingEngine::new(&allocation, &domains);

        let result =
            engine.find_logical_index(&domain, &Value::String("inactive".to_string()), "status");
        assert_eq!(result.unwrap(), 1);
    }

    #[test]
    fn test_get_logical_value_valid() {
        let domain = create_test_domain();
        let allocation = create_test_allocation();
        let mut domains = FxHashMap::default();
        domains.insert("status".to_string(), domain.clone());

        let engine = EncodingEngine::new(&allocation, &domains);

        let result = engine.get_logical_value(&domain, 0);
        assert_eq!(result.unwrap(), Value::String("active".to_string()));
    }

    #[test]
    fn test_get_logical_value_out_of_bounds() {
        let domain = create_test_domain();
        let allocation = create_test_allocation();
        let mut domains = FxHashMap::default();
        domains.insert("status".to_string(), domain.clone());

        let engine = EncodingEngine::new(&allocation, &domains);

        let result = engine.get_logical_value(&domain, 999);
        assert!(result.is_err());
        match result.unwrap_err() {
            VocabularyEncodingError::InvalidValue(field, value) => {
                assert_eq!(field, "decoded_value");
                assert_eq!(value, Value::Number(999.into()));
            }
            _ => panic!("Expected InvalidValue error"),
        }
    }
}
