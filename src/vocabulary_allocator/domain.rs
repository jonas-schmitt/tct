//! Value domain types and token range structures

use serde_json::Value;

/// Represents a domain of possible values for a schema field
#[derive(Debug, Clone, PartialEq)]
pub struct ValueDomain {
    /// Field path (e.g., "user.age", "order.status")
    pub field_path: String,
    /// Type of value domain
    pub domain_type: DomainType,
    /// Number of possible values
    pub cardinality: usize,
    /// The actual values in logical order
    pub logical_values: Vec<Value>,
}

/// Types of value domains that can be optimized
#[derive(Debug, Clone, PartialEq)]
pub enum DomainType {
    Boolean,
    BoundedInteger {
        min: i64,
        max: i64,
    },
    Enum {
        values: Vec<String>,
    },
    /// Pattern-based enumeration (finite alternation patterns)
    PatternEnumeration(Vec<String>),
    /// Pattern-based bounded range (character class patterns)
    PatternBounded(crate::pattern_analyzer::BoundedRange),
}

/// Token ID range allocation for a value domain
#[derive(Debug, Clone, PartialEq)]
pub struct TokenRange {
    /// Starting token ID (inclusive)
    pub start: u32,
    /// Ending token ID (exclusive): [start, end)
    pub end: u32,
    /// Number of values (should equal end - start)
    pub cardinality: usize,
}

impl TokenRange {
    pub fn new(start: u32, cardinality: usize) -> Self {
        Self {
            start,
            end: start + cardinality as u32,
            cardinality,
        }
    }

    /// Check if a token ID is within this range
    pub fn contains(&self, token_id: u32) -> bool {
        token_id >= self.start && token_id < self.end
    }

    /// Convert logical index (0-based) to actual token ID
    /// This maps from domain position to token ID
    ///
    /// # Preconditions
    /// `logical_index` must be < `self.cardinality` (checked with `assert!` in
    /// all build profiles so release tests and invariant violations stay visible).
    pub fn logical_index_to_token(&self, logical_index: usize) -> u32 {
        assert!(
            logical_index < self.cardinality,
            "Logical index {} out of bounds for range with cardinality {}",
            logical_index,
            self.cardinality
        );
        self.start + logical_index as u32
    }

    /// Convert token ID to logical index within this domain
    /// Returns 0-based index for accessing logical_values array
    ///
    /// # Preconditions
    /// `token_id` must be in range [`self.start`, `self.end`) (checked with
    /// `assert!` in all build profiles).
    pub fn token_to_logical_index(&self, token_id: u32) -> usize {
        assert!(
            self.contains(token_id),
            "Token ID {} not in range [{}, {})",
            token_id,
            self.start,
            self.end
        );
        (token_id - self.start) as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========== TokenRange Tests ==========

    #[test]
    fn test_token_range_new_normal() {
        let range = TokenRange::new(100, 50);
        assert_eq!(range.start, 100);
        assert_eq!(range.end, 150);
        assert_eq!(range.cardinality, 50);
    }

    #[test]
    fn test_token_range_new_zero_cardinality() {
        let range = TokenRange::new(10, 0);
        assert_eq!(range.start, 10);
        assert_eq!(range.end, 10);
        assert_eq!(range.cardinality, 0);
    }

    #[test]
    fn test_token_range_new_large_cardinality() {
        let range = TokenRange::new(0, 10000);
        assert_eq!(range.start, 0);
        assert_eq!(range.end, 10000);
        assert_eq!(range.cardinality, 10000);
    }

    #[test]
    fn test_token_range_contains_within_range() {
        let range = TokenRange::new(100, 50);

        assert!(range.contains(100)); // Start (inclusive)
        assert!(range.contains(125)); // Middle
        assert!(range.contains(149)); // Last valid token
    }

    #[test]
    fn test_token_range_contains_outside_range() {
        let range = TokenRange::new(100, 50);

        assert!(!range.contains(99)); // Before start
        assert!(!range.contains(150)); // At end (exclusive)
        assert!(!range.contains(200)); // After end
    }

    #[test]
    fn test_token_range_contains_zero_cardinality() {
        let range = TokenRange::new(10, 0);

        assert!(!range.contains(10)); // Even start is not contained for empty range
        assert!(!range.contains(9));
        assert!(!range.contains(11));
    }

    #[test]
    fn test_logical_index_to_token_valid() {
        let range = TokenRange::new(100, 50);

        assert_eq!(range.logical_index_to_token(0), 100); // First
        assert_eq!(range.logical_index_to_token(25), 125); // Middle
        assert_eq!(range.logical_index_to_token(49), 149); // Last
    }

    #[test]
    #[should_panic(expected = "Logical index 50 out of bounds for range with cardinality 50")]
    fn test_logical_index_to_token_out_of_bounds() {
        let range = TokenRange::new(100, 50);
        range.logical_index_to_token(50); // Exactly at cardinality
    }

    #[test]
    #[should_panic(expected = "out of bounds")]
    fn test_logical_index_to_token_far_out_of_bounds() {
        let range = TokenRange::new(100, 50);
        range.logical_index_to_token(100); // Far beyond cardinality
    }

    #[test]
    fn test_token_to_logical_index_valid() {
        let range = TokenRange::new(100, 50);

        assert_eq!(range.token_to_logical_index(100), 0); // First
        assert_eq!(range.token_to_logical_index(125), 25); // Middle
        assert_eq!(range.token_to_logical_index(149), 49); // Last
    }

    #[test]
    #[should_panic(expected = "Token ID 99 not in range [100, 150)")]
    fn test_token_to_logical_index_below_range() {
        let range = TokenRange::new(100, 50);
        range.token_to_logical_index(99);
    }

    #[test]
    #[should_panic(expected = "Token ID 150 not in range [100, 150)")]
    fn test_token_to_logical_index_at_end() {
        let range = TokenRange::new(100, 50);
        range.token_to_logical_index(150); // End is exclusive
    }

    #[test]
    #[should_panic(expected = "not in range")]
    fn test_token_to_logical_index_above_range() {
        let range = TokenRange::new(100, 50);
        range.token_to_logical_index(200);
    }

    #[test]
    fn test_round_trip_index_to_token_to_index() {
        let range = TokenRange::new(100, 50);

        for logical_index in 0..50 {
            let token_id = range.logical_index_to_token(logical_index);
            let recovered_index = range.token_to_logical_index(token_id);
            assert_eq!(recovered_index, logical_index);
        }
    }

    #[test]
    fn test_round_trip_token_to_index_to_token() {
        let range = TokenRange::new(100, 50);

        for token_id in 100..150 {
            let logical_index = range.token_to_logical_index(token_id);
            let recovered_token = range.logical_index_to_token(logical_index);
            assert_eq!(recovered_token, token_id);
        }
    }

    #[test]
    fn test_token_range_clone() {
        let range1 = TokenRange::new(100, 50);
        let range2 = range1.clone();

        assert_eq!(range1, range2);
        assert_eq!(range2.start, 100);
        assert_eq!(range2.end, 150);
        assert_eq!(range2.cardinality, 50);
    }

    #[test]
    fn test_token_range_partial_eq() {
        let range1 = TokenRange::new(100, 50);
        let range2 = TokenRange::new(100, 50);
        let range3 = TokenRange::new(200, 50);
        let range4 = TokenRange::new(100, 25);

        assert_eq!(range1, range2);
        assert_ne!(range1, range3); // Different start
        assert_ne!(range1, range4); // Different cardinality
    }

    #[test]
    fn test_token_range_debug() {
        let range = TokenRange::new(100, 50);
        let debug_str = format!("{:?}", range);

        assert!(debug_str.contains("TokenRange"));
        assert!(debug_str.contains("100"));
        assert!(debug_str.contains("150"));
        assert!(debug_str.contains("50"));
    }

    // ========== ValueDomain Tests ==========

    #[test]
    fn test_value_domain_creation() {
        let domain = ValueDomain {
            field_path: "user.status".to_string(),
            domain_type: DomainType::Enum {
                values: vec!["active".to_string(), "inactive".to_string()],
            },
            cardinality: 2,
            logical_values: vec![
                Value::String("active".to_string()),
                Value::String("inactive".to_string()),
            ],
        };

        assert_eq!(domain.field_path, "user.status");
        assert_eq!(domain.cardinality, 2);
        assert_eq!(domain.logical_values.len(), 2);
    }

    #[test]
    fn test_value_domain_clone() {
        let domain1 = ValueDomain {
            field_path: "user.age".to_string(),
            domain_type: DomainType::BoundedInteger { min: 0, max: 120 },
            cardinality: 121,
            logical_values: vec![],
        };

        let domain2 = domain1.clone();
        assert_eq!(domain1, domain2);
        assert_eq!(domain2.field_path, "user.age");
        assert_eq!(domain2.cardinality, 121);
    }

    #[test]
    fn test_value_domain_partial_eq() {
        let domain1 = ValueDomain {
            field_path: "status".to_string(),
            domain_type: DomainType::Boolean,
            cardinality: 2,
            logical_values: vec![Value::Bool(false), Value::Bool(true)],
        };

        let domain2 = ValueDomain {
            field_path: "status".to_string(),
            domain_type: DomainType::Boolean,
            cardinality: 2,
            logical_values: vec![Value::Bool(false), Value::Bool(true)],
        };

        let domain3 = ValueDomain {
            field_path: "enabled".to_string(),
            domain_type: DomainType::Boolean,
            cardinality: 2,
            logical_values: vec![Value::Bool(false), Value::Bool(true)],
        };

        assert_eq!(domain1, domain2);
        assert_ne!(domain1, domain3); // Different field_path
    }

    #[test]
    fn test_value_domain_debug() {
        let domain = ValueDomain {
            field_path: "user.status".to_string(),
            domain_type: DomainType::Boolean,
            cardinality: 2,
            logical_values: vec![],
        };

        let debug_str = format!("{:?}", domain);
        assert!(debug_str.contains("ValueDomain"));
        assert!(debug_str.contains("user.status"));
    }

    // ========== DomainType Tests ==========

    #[test]
    fn test_domain_type_boolean() {
        let domain_type = DomainType::Boolean;
        let debug_str = format!("{:?}", domain_type);
        assert!(debug_str.contains("Boolean"));
    }

    #[test]
    fn test_domain_type_bounded_integer() {
        let domain_type = DomainType::BoundedInteger { min: 0, max: 100 };

        if let DomainType::BoundedInteger { min, max } = domain_type {
            assert_eq!(min, 0);
            assert_eq!(max, 100);
        } else {
            panic!("Expected BoundedInteger variant");
        }
    }

    #[test]
    fn test_domain_type_enum() {
        let values = vec!["red".to_string(), "green".to_string(), "blue".to_string()];
        let domain_type = DomainType::Enum {
            values: values.clone(),
        };

        if let DomainType::Enum {
            values: enum_values,
        } = domain_type
        {
            assert_eq!(enum_values.len(), 3);
            assert_eq!(enum_values[0], "red");
            assert_eq!(enum_values[1], "green");
            assert_eq!(enum_values[2], "blue");
        } else {
            panic!("Expected Enum variant");
        }
    }

    #[test]
    fn test_domain_type_pattern_enumeration() {
        let patterns = vec!["v1".to_string(), "v2".to_string()];
        let domain_type = DomainType::PatternEnumeration(patterns.clone());

        if let DomainType::PatternEnumeration(pattern_values) = domain_type {
            assert_eq!(pattern_values.len(), 2);
            assert_eq!(pattern_values[0], "v1");
            assert_eq!(pattern_values[1], "v2");
        } else {
            panic!("Expected PatternEnumeration variant");
        }
    }

    #[test]
    fn test_domain_type_clone() {
        let domain_type1 = DomainType::Enum {
            values: vec!["a".to_string(), "b".to_string()],
        };
        let domain_type2 = domain_type1.clone();

        assert_eq!(domain_type1, domain_type2);
    }

    #[test]
    fn test_domain_type_partial_eq() {
        let dt1 = DomainType::Boolean;
        let dt2 = DomainType::Boolean;
        let dt3 = DomainType::BoundedInteger { min: 0, max: 10 };

        assert_eq!(dt1, dt2);
        assert_ne!(dt1, dt3);
    }

    #[test]
    fn test_domain_type_debug() {
        let domain_type = DomainType::Enum {
            values: vec!["active".to_string()],
        };
        let debug_str = format!("{:?}", domain_type);

        assert!(debug_str.contains("Enum"));
        assert!(debug_str.contains("active"));
    }
}
