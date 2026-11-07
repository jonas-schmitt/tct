//! Vocabulary Allocation System for LLM-Optimized Token Compression
//!
//! This module implements dense token ID allocation to minimize vocabulary size
//! for LLM efficiency. Instead of using logical values as token IDs, we assign
//! contiguous token ranges to minimize the total vocabulary size.

pub mod allocation;
pub mod allocator;
pub mod domain;

// Re-export main types for backwards compatibility
pub use allocation::{VocabularyAllocation, VocabularySavings};
pub use allocator::{VocabularyAllocator, VocabularyEncoder};
pub use domain::{DomainType, TokenRange, ValueDomain};

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn create_boolean_domain(field_path: &str) -> ValueDomain {
        ValueDomain {
            field_path: field_path.to_string(),
            domain_type: DomainType::Boolean,
            cardinality: 2,
            logical_values: vec![json!(false), json!(true)],
        }
    }

    fn create_enum_domain(field_path: &str, values: Vec<&str>) -> ValueDomain {
        let json_values: Vec<serde_json::Value> = values.iter().map(|s| json!(s)).collect();
        ValueDomain {
            field_path: field_path.to_string(),
            domain_type: DomainType::Enum {
                values: values.iter().map(|s| s.to_string()).collect(),
            },
            cardinality: values.len(),
            logical_values: json_values,
        }
    }

    fn create_bounded_integer_domain(field_path: &str, min: i64, max: i64) -> ValueDomain {
        let range = max - min + 1;
        let values: Vec<serde_json::Value> = (min..=max).map(|n| json!(n)).collect();
        ValueDomain {
            field_path: field_path.to_string(),
            domain_type: DomainType::BoundedInteger { min, max },
            cardinality: range as usize,
            logical_values: values,
        }
    }

    #[test]
    fn test_token_range_operations() {
        let range = TokenRange::new(10, 5); // tokens 10-14
        assert_eq!(range.start, 10);
        assert_eq!(range.end, 15);
        assert_eq!(range.cardinality, 5);

        assert!(range.contains(10));
        assert!(range.contains(14));
        assert!(!range.contains(9));
        assert!(!range.contains(15));

        assert_eq!(range.logical_index_to_token(0), 10);
        assert_eq!(range.logical_index_to_token(4), 14);

        assert_eq!(range.token_to_logical_index(10), 0);
        assert_eq!(range.token_to_logical_index(14), 4);
    }

    #[test]
    fn test_simple_allocation() {
        let domains = vec![
            create_boolean_domain("is_active"),
            create_enum_domain("status", vec!["pending", "approved", "rejected"]),
        ];

        let allocation = VocabularyAllocator::allocate(domains);

        assert_eq!(allocation.total_vocabulary_size, 5); // 2 + 3
        assert!(allocation.get_range("is_active").is_some());
        assert!(allocation.get_range("status").is_some());

        let bool_range = allocation.get_range("is_active").unwrap();
        let enum_range = allocation.get_range("status").unwrap();

        // Domains should be allocated contiguously with no gaps
        assert_eq!(bool_range.cardinality, 2);
        assert_eq!(enum_range.cardinality, 3);

        // Check that ranges don't overlap
        let bool_tokens: std::collections::HashSet<u32> =
            (bool_range.start..bool_range.end).collect();
        let enum_tokens: std::collections::HashSet<u32> =
            (enum_range.start..enum_range.end).collect();
        assert!(bool_tokens.is_disjoint(&enum_tokens));
    }

    #[test]
    fn test_vocabulary_encoder() {
        let domain = create_enum_domain("status", vec!["pending", "approved", "rejected"]);
        let range = TokenRange::new(100, 3); // tokens 100-102

        let encoder = VocabularyEncoder::new(&domain, &range);

        // Test encoding
        assert_eq!(encoder.encode_value(&json!("pending")), Some(100));
        assert_eq!(encoder.encode_value(&json!("approved")), Some(101));
        assert_eq!(encoder.encode_value(&json!("rejected")), Some(102));
        assert_eq!(encoder.encode_value(&json!("unknown")), None);

        // Test decoding
        assert_eq!(encoder.decode_token(100), json!("pending"));
        assert_eq!(encoder.decode_token(101), json!("approved"));
        assert_eq!(encoder.decode_token(102), json!("rejected"));
    }

    #[test]
    fn test_constrained_allocation() {
        let domains = vec![
            create_boolean_domain("is_active"),
            create_enum_domain("priority", vec!["low", "medium", "high", "urgent"]),
            create_bounded_integer_domain("score", 1, 10),
        ];

        // Total required: 2 + 4 + 10 = 16 tokens
        // Constrain to 8 tokens - should use base encoding
        let allocation = VocabularyAllocator::allocate_with_constraint(domains, Some(8));

        assert_eq!(allocation.total_vocabulary_size, 8);

        // All domains should use the full constrained vocabulary
        for field in ["is_active", "priority", "score"] {
            let range = allocation.get_range(field).unwrap();
            assert_eq!(range.start, 0);
            assert_eq!(range.cardinality, 8);
        }
    }

    #[test]
    fn test_vocabulary_savings_calculation() {
        let domains = vec![
            create_boolean_domain("is_active"),           // 2 tokens
            create_enum_domain("status", vec!["a", "b"]), // 2 tokens
        ];

        let allocation = VocabularyAllocator::allocate(domains.clone());
        let naive_size = VocabularyAllocator::calculate_naive_vocabulary_size(&domains);

        let savings = allocation.calculate_savings(naive_size);

        assert_eq!(savings.naive_vocabulary_size, 4);
        assert_eq!(savings.distinct_structural_tokens_count, 4);
        assert_eq!(savings.tokens_saved, 0);
        assert_eq!(savings.reduction_percentage, 0.0);
    }

    #[test]
    fn test_numeric_enum_naive_calculation() {
        let domains = vec![ValueDomain {
            field_path: "large_id".to_string(),
            domain_type: DomainType::Enum {
                values: vec!["100".to_string(), "200".to_string()],
            },
            cardinality: 2,
            logical_values: vec![json!(100), json!(200)], // Numeric values
        }];

        let naive_size = VocabularyAllocator::calculate_naive_vocabulary_size(&domains);

        // Should use max token ID approach: max(100, 200) + 1 = 201
        assert_eq!(naive_size, 201);
    }

    #[test]
    fn test_string_enum_naive_calculation() {
        let domains = vec![create_enum_domain("status", vec!["active", "inactive"])];

        let naive_size = VocabularyAllocator::calculate_naive_vocabulary_size(&domains);

        // Should use additive approach: 2 tokens
        assert_eq!(naive_size, 2);
    }
}
