//! Vocabulary allocation results and savings calculation

use super::domain::TokenRange;
use rustc_hash::FxHashMap;

/// Complete vocabulary allocation result
pub struct VocabularyAllocation {
    /// Total vocabulary size (maximum token ID + 1)
    pub total_vocabulary_size: usize,
    /// Token range allocations for each field
    pub domain_allocations: FxHashMap<String, TokenRange>,
    /// Reverse lookup: token ID → field path (for debugging)
    pub token_to_field: FxHashMap<u32, String>,
}

impl VocabularyAllocation {
    /// Create an empty allocation
    pub fn new() -> Self {
        Self {
            total_vocabulary_size: 0,
            domain_allocations: FxHashMap::default(),
            token_to_field: FxHashMap::default(),
        }
    }

    /// Get the token range for a specific field
    pub fn get_range(&self, field_path: &str) -> Option<&TokenRange> {
        self.domain_allocations.get(field_path)
    }

    /// Calculate vocabulary size reduction compared to naive approach
    pub fn calculate_savings(&self, naive_vocab_size: usize) -> VocabularySavings {
        let reduction = naive_vocab_size.saturating_sub(self.total_vocabulary_size);
        let reduction_percentage = if naive_vocab_size > 0 {
            (reduction as f64 / naive_vocab_size as f64) * 100.0
        } else {
            0.0
        };

        VocabularySavings {
            naive_vocabulary_size: naive_vocab_size,
            distinct_structural_tokens_count: self.total_vocabulary_size,
            tokens_saved: reduction,
            reduction_percentage,
        }
    }
}

impl Default for VocabularyAllocation {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics about vocabulary size optimization
#[derive(Debug, Clone, PartialEq)]
pub struct VocabularySavings {
    pub naive_vocabulary_size: usize,
    pub distinct_structural_tokens_count: usize,
    pub tokens_saved: usize,
    pub reduction_percentage: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========== VocabularyAllocation Tests ==========

    #[test]
    fn test_new_creates_empty_allocation() {
        let alloc = VocabularyAllocation::new();
        assert_eq!(alloc.total_vocabulary_size, 0);
        assert!(alloc.domain_allocations.is_empty());
        assert!(alloc.token_to_field.is_empty());
    }

    #[test]
    fn test_default_creates_empty_allocation() {
        let alloc = VocabularyAllocation::default();
        assert_eq!(alloc.total_vocabulary_size, 0);
        assert!(alloc.domain_allocations.is_empty());
        assert!(alloc.token_to_field.is_empty());
    }

    #[test]
    fn test_get_range_existing_field() {
        let mut alloc = VocabularyAllocation::new();
        let range = TokenRange {
            start: 10,
            end: 20,
            cardinality: 10,
        };
        alloc
            .domain_allocations
            .insert("status".to_string(), range.clone());

        let retrieved = alloc.get_range("status");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().start, 10);
        assert_eq!(retrieved.unwrap().end, 20);
        assert_eq!(retrieved.unwrap().cardinality, 10);
    }

    #[test]
    fn test_get_range_nonexistent_field() {
        let alloc = VocabularyAllocation::new();
        assert!(alloc.get_range("nonexistent").is_none());
    }

    #[test]
    fn test_calculate_savings_with_reduction() {
        let mut alloc = VocabularyAllocation::new();
        alloc.total_vocabulary_size = 75;

        let savings = alloc.calculate_savings(100);
        assert_eq!(savings.naive_vocabulary_size, 100);
        assert_eq!(savings.distinct_structural_tokens_count, 75);
        assert_eq!(savings.tokens_saved, 25);
        assert_eq!(savings.reduction_percentage, 25.0);
    }

    #[test]
    fn test_calculate_savings_zero_naive_vocab() {
        let mut alloc = VocabularyAllocation::new();
        alloc.total_vocabulary_size = 50;

        let savings = alloc.calculate_savings(0);
        assert_eq!(savings.naive_vocabulary_size, 0);
        assert_eq!(savings.distinct_structural_tokens_count, 50);
        assert_eq!(savings.tokens_saved, 0); // saturating_sub returns 0
        assert_eq!(savings.reduction_percentage, 0.0);
    }

    #[test]
    fn test_calculate_savings_no_reduction() {
        let mut alloc = VocabularyAllocation::new();
        alloc.total_vocabulary_size = 100;

        let savings = alloc.calculate_savings(100);
        assert_eq!(savings.naive_vocabulary_size, 100);
        assert_eq!(savings.distinct_structural_tokens_count, 100);
        assert_eq!(savings.tokens_saved, 0);
        assert_eq!(savings.reduction_percentage, 0.0);
    }

    #[test]
    fn test_calculate_savings_optimized_larger_than_naive() {
        let mut alloc = VocabularyAllocation::new();
        alloc.total_vocabulary_size = 150;

        let savings = alloc.calculate_savings(100);
        assert_eq!(savings.naive_vocabulary_size, 100);
        assert_eq!(savings.distinct_structural_tokens_count, 150);
        assert_eq!(savings.tokens_saved, 0); // saturating_sub prevents underflow
        assert!(savings.reduction_percentage <= 0.0);
    }

    #[test]
    fn test_token_to_field_mapping() {
        let mut alloc = VocabularyAllocation::new();
        alloc.token_to_field.insert(10, "status".to_string());
        alloc.token_to_field.insert(20, "priority".to_string());

        assert_eq!(alloc.token_to_field.get(&10), Some(&"status".to_string()));
        assert_eq!(alloc.token_to_field.get(&20), Some(&"priority".to_string()));
        assert_eq!(alloc.token_to_field.get(&30), None);
    }

    // ========== VocabularySavings Tests ==========

    #[test]
    fn test_vocabulary_savings_clone() {
        let savings = VocabularySavings {
            naive_vocabulary_size: 200,
            distinct_structural_tokens_count: 150,
            tokens_saved: 50,
            reduction_percentage: 25.0,
        };

        let cloned = savings.clone();
        assert_eq!(cloned.naive_vocabulary_size, 200);
        assert_eq!(cloned.distinct_structural_tokens_count, 150);
        assert_eq!(cloned.tokens_saved, 50);
        assert_eq!(cloned.reduction_percentage, 25.0);
    }

    #[test]
    fn test_vocabulary_savings_partial_eq() {
        let savings1 = VocabularySavings {
            naive_vocabulary_size: 200,
            distinct_structural_tokens_count: 150,
            tokens_saved: 50,
            reduction_percentage: 25.0,
        };

        let savings2 = VocabularySavings {
            naive_vocabulary_size: 200,
            distinct_structural_tokens_count: 150,
            tokens_saved: 50,
            reduction_percentage: 25.0,
        };

        assert_eq!(savings1, savings2);
    }

    #[test]
    fn test_vocabulary_savings_debug() {
        let savings = VocabularySavings {
            naive_vocabulary_size: 100,
            distinct_structural_tokens_count: 75,
            tokens_saved: 25,
            reduction_percentage: 25.0,
        };

        let debug_str = format!("{:?}", savings);
        assert!(debug_str.contains("naive_vocabulary_size"));
        assert!(debug_str.contains("100"));
        assert!(debug_str.contains("25.0"));
    }
}
