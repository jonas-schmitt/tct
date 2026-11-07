//! Core vocabulary allocator logic

use super::allocation::VocabularyAllocation;
use super::domain::{DomainType, TokenRange, ValueDomain};
use serde_json::Value;

/// Core vocabulary allocator that optimizes token ID assignment
pub struct VocabularyAllocator;

impl VocabularyAllocator {
    /// Create optimal token allocation for a list of value domains
    pub fn allocate(domains: Vec<ValueDomain>) -> VocabularyAllocation {
        Self::allocate_with_constraint(domains, None)
    }

    /// Create token allocation with vocabulary size constraint
    pub fn allocate_with_constraint(
        domains: Vec<ValueDomain>,
        max_vocabulary_size: Option<usize>,
    ) -> VocabularyAllocation {
        let mut allocation = VocabularyAllocation::new();

        if domains.is_empty() {
            return allocation;
        }

        // Calculate total requirements for unconstrained allocation
        let total_required = domains.iter().map(|d| d.cardinality).sum::<usize>();

        // Check if we need to apply constraint
        if let Some(max_vocab) = max_vocabulary_size
            && total_required > max_vocab
        {
            // Use multi-token encoding with base-N constraint
            return Self::allocate_with_base_encoding(domains, max_vocab);
        }

        // Normal allocation when no constraint or constraint doesn't bind
        // Sort domains by cardinality (smallest first) for better packing
        let mut sorted_domains = domains;
        sorted_domains.sort_by_key(|d| d.cardinality);

        let mut current_token_id = 0u32;

        for domain in sorted_domains {
            let range = TokenRange::new(current_token_id, domain.cardinality);

            // Add to domain allocations
            allocation
                .domain_allocations
                .insert(domain.field_path.clone(), range.clone());

            // Add to reverse lookup for debugging
            for token_id in range.start..range.end {
                allocation
                    .token_to_field
                    .insert(token_id, domain.field_path.clone());
            }

            current_token_id = range.end;
        }

        allocation.total_vocabulary_size = current_token_id as usize;
        allocation
    }

    /// Allocate tokens using base-N encoding when vocabulary is constrained
    fn allocate_with_base_encoding(
        domains: Vec<ValueDomain>,
        max_vocab_size: usize,
    ) -> VocabularyAllocation {
        let mut allocation = VocabularyAllocation::new();

        // With base-N encoding, all fields use modulo operations with the constrained vocabulary size
        // This provides robust LLM behavior at the cost of potentially more tokens per value

        for domain in domains {
            let range = TokenRange::new(0, max_vocab_size); // All domains use the full vocabulary range

            // Add to domain allocations
            allocation
                .domain_allocations
                .insert(domain.field_path.clone(), range.clone());

            // Add to reverse lookup (all tokens can map to any field with base encoding)
            for token_id in 0..max_vocab_size as u32 {
                allocation
                    .token_to_field
                    .insert(token_id, domain.field_path.clone());
            }
        }

        allocation.total_vocabulary_size = max_vocab_size;
        allocation
    }

    /// Calculate what the naive vocabulary size would be (sum of all domain cardinalities)
    pub fn calculate_naive_vocabulary_size(domains: &[ValueDomain]) -> usize {
        // Use max token ID approach for any schema with explicit numeric enums
        // (whether string enums or numeric enums with actual integer values)
        let has_numeric_enums = domains.iter().any(|domain| {
            matches!(&domain.domain_type, DomainType::Enum { .. })
                && domain
                    .logical_values
                    .iter()
                    .any(|v| matches!(v, Value::Number(_)))
        });

        if has_numeric_enums {
            // Use max token ID approach: enums with numeric values would use those as token IDs
            let max_token_id = domains
                .iter()
                .filter_map(Self::calculate_domain_max_token_id)
                .max()
                .unwrap_or(0);
            (max_token_id + 1) as usize
        } else {
            // Use additive approach for schemas with only string enums and bounded integers
            domains.iter().map(|domain| domain.cardinality).sum()
        }
    }

    /// Calculate the maximum token ID a domain would use with naive encoding
    fn calculate_domain_max_token_id(domain: &ValueDomain) -> Option<u32> {
        match &domain.domain_type {
            DomainType::Boolean => Some(1), // Uses token IDs 0, 1
            DomainType::BoundedInteger { min: _, max } => Some(*max as u32),
            DomainType::Enum { values: _ } => {
                // For enums, naive encoding would use the actual values as token IDs
                // Find the maximum numeric value in the logical_values
                domain
                    .logical_values
                    .iter()
                    .filter_map(|v| match v {
                        Value::Number(n) => n.as_u64().map(|u| u as u32),
                        Value::Bool(true) => Some(1),
                        Value::Bool(false) => Some(0),
                        _ => None, // String enums would typically use sequential IDs starting from 0
                    })
                    .max()
                    .or_else(|| {
                        // If no numeric values, assume string enum uses 0-based indexing
                        if domain.logical_values.is_empty() {
                            None
                        } else {
                            Some((domain.logical_values.len() - 1) as u32)
                        }
                    })
            }
            // Pattern optimization cases
            DomainType::PatternEnumeration(values) => {
                // Pattern enumeration uses sequential token IDs
                Some((values.len() - 1) as u32)
            }
            DomainType::PatternBounded(range) => {
                // Pattern bounded uses the range size
                Some((range.total_values - 1) as u32)
            }
        }
    }
}

/// Value encoder that maps logical values to dense token IDs
pub struct VocabularyEncoder<'a> {
    domain: &'a ValueDomain,
    allocation: &'a TokenRange,
}

impl<'a> VocabularyEncoder<'a> {
    pub fn new(domain: &'a ValueDomain, allocation: &'a TokenRange) -> Self {
        assert_eq!(
            domain.cardinality, allocation.cardinality,
            "Domain cardinality {} doesn't match allocation cardinality {}",
            domain.cardinality, allocation.cardinality
        );
        Self { domain, allocation }
    }

    /// Encode a logical value to its corresponding token ID
    pub fn encode_value(&self, value: &Value) -> Option<u32> {
        let logical_index = self.domain.logical_values.iter().position(|v| v == value)?;
        Some(self.allocation.logical_index_to_token(logical_index))
    }

    /// Decode a token ID to its logical value (with modulo for LLM robustness)
    pub fn decode_token(&self, token_id: u32) -> Value {
        let logical_index = self.allocation.token_to_logical_index(token_id);
        self.domain.logical_values[logical_index].clone()
    }
}
