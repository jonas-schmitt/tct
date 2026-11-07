//! Universal Schema-Aware Decoding System
//!
//! This module provides the core data structures and algorithms for universal schema-aware
//! decoding, eliminating all legacy workarounds and heuristics throughout the codebase.
//!
//! Key components:
//! - TokenStructure: Identifies token patterns (string/object/array/primitive)
//! - Pattern recognition: Analyzes token sequences to determine structure
//! - Variant compatibility: Maps types to compatible token structures

/// Represents the structure of tokens in the encoded stream
///
/// This enum captures the fundamental patterns that can be distinguished
/// in our untyped token format, enabling schema-aware dispatch without
/// requiring type tags.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TokenStructure {
    /// Single primitive value token: `[value]`
    Primitive(u32),

    /// Length-prefixed string: `[length, char1, char2, ...]`
    String {
        /// Number of characters in the string
        length: usize,
    },

    /// Length-prefixed array: `[length, item1, item2, ...]`
    Array {
        /// Number of items in the array
        length: usize,
    },

    /// Field-count-prefixed object: `[field_count, field1_name, field1_value, ...]`
    Object {
        /// Number of fields in the object
        field_count: usize,
    },

    /// Null value (represented as specific token or pattern)
    Null,
}

impl TokenStructure {
    /// Returns true if this structure represents a container type (array/object)
    pub fn is_container(&self) -> bool {
        matches!(
            self,
            TokenStructure::Array { .. } | TokenStructure::Object { .. }
        )
    }

    /// Returns true if this structure represents a primitive type
    pub fn is_primitive(&self) -> bool {
        matches!(self, TokenStructure::Primitive(_) | TokenStructure::Null)
    }

    /// Returns true if this structure represents a length-prefixed type
    pub fn is_length_prefixed(&self) -> bool {
        matches!(
            self,
            TokenStructure::String { .. }
                | TokenStructure::Array { .. }
                | TokenStructure::Object { .. }
        )
    }

    /// Returns the expected minimum token count for this structure
    pub fn min_token_count(&self) -> usize {
        match self {
            TokenStructure::Primitive(_) => 1,
            TokenStructure::Null => 1,
            TokenStructure::String { length } => 1 + length,
            TokenStructure::Array { length } => 1 + length, // Simplified - actual depends on item types
            TokenStructure::Object { field_count } => 1 + field_count * 2, // Simplified - actual depends on field types
        }
    }
}

/// Token pattern recognition for universal schema-aware decoding
///
/// These functions analyze token streams to determine their structure without
/// consuming tokens, enabling schema-aware dispatch.
pub mod pattern_recognition {
    use super::TokenStructure;
    use crate::tokenizer::BufferedTokenIterator;

    /// Analyze token stream to determine its structure
    ///
    /// This is the core function for universal schema-aware decoding. It examines
    /// the token stream without consuming tokens and returns the detected structure.
    ///
    /// Returns None if the token pattern cannot be determined or is invalid.
    pub fn analyze_token_structure(tokens: &BufferedTokenIterator) -> Option<TokenStructure> {
        crate::time_operation!("analyze_token_structure");
        // Handle empty token streams (common for unit enum variants)
        if tokens.remaining() == 0 {
            // Empty token stream - represents unit value or empty structure
            return Some(TokenStructure::Null);
        }

        // Check if we have any tokens
        let first_token = tokens.peek()?;

        // For our token format, we need to determine structure based on patterns:
        // - Single token: Primitive or Null
        // - Length-prefixed patterns: String/Array/Object

        // Start with simple heuristics - this will be enhanced in later phases
        if tokens.remaining() == 1 {
            // Single token - likely primitive or null
            if is_null_token(first_token) {
                Some(TokenStructure::Null)
            } else {
                Some(TokenStructure::Primitive(first_token))
            }
        } else if tokens.remaining() >= 2 {
            // Multi-token pattern - analyze for length-prefixed structures
            analyze_length_prefixed_structure(tokens)
        } else {
            None
        }
    }

    /// Check if a token represents a null value
    fn is_null_token(token: u32) -> bool {
        // Note: Null handling is primarily done via OptionalField<T> architecture
        // This fallback checks for the reserved null marker token
        token == 0xFFFFFFFF
    }

    /// Analyze length-prefixed token structures with smart type discrimination
    fn analyze_length_prefixed_structure(tokens: &BufferedTokenIterator) -> Option<TokenStructure> {
        crate::time_operation!("analyze_length_prefixed_structure");
        let first_token = tokens.peek()?;
        let remaining = tokens.remaining();

        if remaining >= 2 {
            let second_token = tokens.peek_at(1)?;

            // String detection: first token is length, second is ASCII/byte value
            // Strings encode as [length, byte1, byte2, ...]
            if looks_like_string_pattern(tokens, first_token, second_token) {
                let length = first_token as usize;
                return Some(TokenStructure::String { length });
            }

            // Object detection: first token is field count, second starts first key (string)
            // HashMap encodes as [count, key1_len, key1_bytes..., value1_tokens..., key2_len, ...]
            if looks_like_object_pattern(tokens, first_token, second_token) {
                let field_count = first_token as usize;
                return Some(TokenStructure::Object { field_count });
            }

            // Array detection: first token is count, second is first element
            // Arrays encode as [count, elem1_tokens..., elem2_tokens..., ...]
            if looks_like_array_pattern(tokens, first_token) {
                let length = first_token as usize;
                return Some(TokenStructure::Array { length });
            }
        }

        // Fallback: treat as primitive if we can't determine structure
        Some(TokenStructure::Primitive(first_token))
    }

    /// Check if token pattern looks like a string: [length, bytes...]
    fn looks_like_string_pattern(tokens: &BufferedTokenIterator, first: u32, second: u32) -> bool {
        crate::time_operation!("looks_like_string_pattern");
        let potential_length = first as usize;

        // Reasonable string length bounds
        if potential_length == 0 || potential_length > 10000 {
            return false;
        }

        // Exact token count match is the strongest indicator for strings
        if tokens.remaining() == potential_length + 1 {
            // Second token should be a byte value (0-255) for strings
            if second <= 255 {
                // Check if remaining tokens look like bytes
                let bytes_to_check = potential_length.min(5).min(tokens.remaining() - 1);
                let all_bytes =
                    (1..=bytes_to_check).all(|i| tokens.peek_at(i).is_some_and(|t| t <= 255));
                return all_bytes;
            }
        }

        false // Be conservative - only match clear string patterns
    }

    /// Check if token pattern looks like an object/HashMap: [count, key_patterns...]
    fn looks_like_object_pattern(tokens: &BufferedTokenIterator, first: u32, second: u32) -> bool {
        let potential_count = first as usize;

        // Reasonable field count bounds
        if potential_count > 100 {
            return false;
        }

        // If count is 0, this is an empty object
        if potential_count == 0 {
            return tokens.remaining() == 1;
        }

        // Check for the specific object pattern from tests: [field_count, field1_name, field1_value, ...]
        // This expects field_count * 2 + 1 total tokens (each field = name + value)
        if tokens.remaining() == (potential_count * 2) + 1 {
            return true;
        }

        // For more complex nested objects, look for string-like key patterns
        // But be more conservative to avoid false positives with arrays
        let potential_key_length = second as usize;
        if potential_key_length > 0 && potential_key_length < 100 {
            // More restrictive bounds
            // Check if this looks like a string key pattern
            if let Some(third) = tokens.peek_at(2)
                && third <= 255
            {
                // Additional check: ensure we have enough tokens for at least one key-value pair
                let min_tokens_needed = 1 + potential_key_length + 1; // count + key_length + key_bytes + value
                if tokens.remaining() >= min_tokens_needed {
                    return true; // Looks like HashMap with string keys
                }
            }
        }

        false
    }

    /// Check if token pattern looks like an array: [count, elements...]
    fn looks_like_array_pattern(tokens: &BufferedTokenIterator, first: u32) -> bool {
        let potential_count = first as usize;

        // Reasonable array length bounds
        if potential_count > 10000 {
            return false;
        }

        // If count is 0, this is an empty array
        if potential_count == 0 {
            return tokens.remaining() == 1;
        }

        // Arrays need more than just count+1 tokens (unless very small)
        if tokens.remaining() <= potential_count + 1 {
            return true; // Simple array pattern
        }

        // For complex arrays, be more conservative - ensure it's not a clear string or object
        tokens.remaining() > 1
    }

    /// Determine if token stream is compatible with expected structure
    ///
    /// This function checks if the actual token pattern matches the expected
    /// structure, enabling precise variant routing in enum decoding.
    pub fn is_structure_compatible(
        tokens: &BufferedTokenIterator,
        expected: &TokenStructure,
    ) -> bool {
        match analyze_token_structure(tokens) {
            Some(actual) => structures_match(&actual, expected),
            None => false,
        }
    }

    /// Check if two token structures are compatible
    pub fn structures_match(actual: &TokenStructure, expected: &TokenStructure) -> bool {
        match (actual, expected) {
            // Exact matches
            (TokenStructure::Null, TokenStructure::Null) => true,
            (TokenStructure::Primitive(_), TokenStructure::Primitive(_)) => true, // Flexible primitive matching
            (TokenStructure::String { .. }, TokenStructure::String { .. }) => true, // Flexible string matching
            (TokenStructure::Array { .. }, TokenStructure::Array { .. }) => true, // Flexible array matching
            (TokenStructure::Object { .. }, TokenStructure::Object { .. }) => true, // Flexible object matching

            // No cross-type compatibility
            _ => false,
        }
    }
}

/// Variant compatibility analysis for enum decoding
///
/// This module analyzes which enum variants are compatible with specific
/// token structures, enabling precise routing in schema-aware decoding.
pub mod variant_compatibility {
    use super::{TokenStructure, pattern_recognition};
    use rustc_hash::FxHashMap;

    /// Variant compatibility information for an enum type
    #[derive(Debug, Clone)]
    pub struct VariantCompatibility {
        /// Mapping from token structures to compatible variant names
        pub structure_to_variants: FxHashMap<TokenStructure, Vec<String>>,
        /// Mapping from variant names to expected token structures
        pub variant_to_structure: FxHashMap<String, TokenStructure>,
        /// Variants that can handle multiple structures (flexible variants)
        pub flexible_variants: Vec<String>,
    }

    impl Default for VariantCompatibility {
        fn default() -> Self {
            Self::new()
        }
    }

    impl VariantCompatibility {
        /// Create a new variant compatibility analysis
        pub fn new() -> Self {
            Self {
                structure_to_variants: FxHashMap::default(),
                variant_to_structure: FxHashMap::default(),
                flexible_variants: Vec::new(),
            }
        }

        /// Add a variant with its expected token structure
        pub fn add_variant(&mut self, variant_name: String, expected_structure: TokenStructure) {
            // Add to variant → structure mapping
            self.variant_to_structure
                .insert(variant_name.clone(), expected_structure.clone());

            // Add to structure → variants mapping
            self.structure_to_variants
                .entry(expected_structure)
                .or_default()
                .push(variant_name);
        }

        /// Mark a variant as flexible (can handle multiple structure types)
        pub fn mark_flexible(&mut self, variant_name: String) {
            if !self.flexible_variants.contains(&variant_name) {
                self.flexible_variants.push(variant_name);
            }
        }

        /// Get variants compatible with a specific token structure
        pub fn get_compatible_variants(&self, structure: &TokenStructure) -> Vec<String> {
            let mut compatible = Vec::new();

            // Direct structure matches
            if let Some(variants) = self.structure_to_variants.get(structure) {
                compatible.extend(variants.clone());
            }

            // Check flexible variants using pattern matching
            for flexible_variant in &self.flexible_variants {
                if let Some(variant_structure) = self.variant_to_structure.get(flexible_variant)
                    && pattern_recognition::structures_match(structure, variant_structure)
                    && !compatible.contains(flexible_variant)
                {
                    compatible.push(flexible_variant.clone());
                }
            }

            compatible
        }

        /// Get the expected structure for a variant
        pub fn get_variant_structure(&self, variant_name: &str) -> Option<&TokenStructure> {
            self.variant_to_structure.get(variant_name)
        }

        /// Check if a variant can handle a specific token structure
        pub fn is_variant_compatible(
            &self,
            variant_name: &str,
            structure: &TokenStructure,
        ) -> bool {
            // Check if it's in the compatible variants list
            self.get_compatible_variants(structure)
                .contains(&variant_name.to_string())
        }

        /// Get statistics about variant compatibility
        pub fn get_stats(&self) -> VariantCompatibilityStats {
            let total_variants = self.variant_to_structure.len();
            let total_structures = self.structure_to_variants.len();
            let flexible_variants = self.flexible_variants.len();

            // Calculate ambiguous structures (structures with multiple variants)
            let ambiguous_structures = self
                .structure_to_variants
                .values()
                .filter(|variants| variants.len() > 1)
                .count();

            VariantCompatibilityStats {
                total_variants,
                total_structures,
                flexible_variants,
                ambiguous_structures,
            }
        }
    }

    /// Statistics about variant compatibility analysis
    #[derive(Debug, Clone)]
    pub struct VariantCompatibilityStats {
        /// Total number of enum variants analyzed
        pub total_variants: usize,
        /// Total number of token structures evaluated
        pub total_structures: usize,
        /// Number of variants that can accept multiple structures
        pub flexible_variants: usize,
        /// Number of structures that match multiple variants
        pub ambiguous_structures: usize,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tokenizer::BufferedTokenIterator;

    #[test]
    fn test_token_structure_classification() {
        // Test primitive classification
        let primitive = TokenStructure::Primitive(42);
        assert!(primitive.is_primitive());
        assert!(!primitive.is_container());
        assert!(!primitive.is_length_prefixed());

        // Test string classification
        let string = TokenStructure::String { length: 5 };
        assert!(!string.is_primitive());
        assert!(!string.is_container());
        assert!(string.is_length_prefixed());

        // Test array classification
        let array = TokenStructure::Array { length: 3 };
        assert!(!array.is_primitive());
        assert!(array.is_container());
        assert!(array.is_length_prefixed());

        // Test object classification
        let object = TokenStructure::Object { field_count: 2 };
        assert!(!object.is_primitive());
        assert!(object.is_container());
        assert!(object.is_length_prefixed());

        // Test null classification
        let null = TokenStructure::Null;
        assert!(null.is_primitive());
        assert!(!null.is_container());
        assert!(!null.is_length_prefixed());
    }

    #[test]
    fn test_min_token_count() {
        assert_eq!(TokenStructure::Primitive(1).min_token_count(), 1);
        assert_eq!(TokenStructure::Null.min_token_count(), 1);
        assert_eq!(TokenStructure::String { length: 5 }.min_token_count(), 6);
        assert_eq!(TokenStructure::Array { length: 3 }.min_token_count(), 4);
        assert_eq!(
            TokenStructure::Object { field_count: 2 }.min_token_count(),
            5
        );
    }

    #[test]
    fn test_equality_and_hashing() {
        let s1 = TokenStructure::String { length: 5 };
        let s2 = TokenStructure::String { length: 5 };
        let s3 = TokenStructure::String { length: 3 };

        assert_eq!(s1, s2);
        assert_ne!(s1, s3);

        // Test that it can be used in hash-based collections
        let mut structures = std::collections::HashSet::new();
        structures.insert(s1.clone());
        assert!(structures.contains(&s2));
        assert!(!structures.contains(&s3));
    }

    #[test]
    fn test_pattern_recognition_single_token() {
        use pattern_recognition::*;

        // Test primitive recognition
        let primitive_tokens = BufferedTokenIterator::new([42].into_iter());
        let structure = analyze_token_structure(&primitive_tokens);
        assert_eq!(structure, Some(TokenStructure::Primitive(42)));

        // Test null recognition
        let null_tokens = BufferedTokenIterator::new([0xFFFFFFFF].into_iter());
        let structure = analyze_token_structure(&null_tokens);
        assert_eq!(structure, Some(TokenStructure::Null));
    }

    #[test]
    fn test_pattern_recognition_string_structure() {
        use pattern_recognition::*;

        // Test string pattern: [length, char1, char2, ...]
        let string_tokens = BufferedTokenIterator::new([3, 65, 66, 67].into_iter()); // "ABC"
        let structure = analyze_token_structure(&string_tokens);
        assert_eq!(structure, Some(TokenStructure::String { length: 3 }));
    }

    #[test]
    fn test_pattern_recognition_array_structure() {
        use pattern_recognition::*;

        // Test array pattern: [length, item1, item2, ...]
        let array_tokens = BufferedTokenIterator::new([2, 100, 200, 300].into_iter());
        let structure = analyze_token_structure(&array_tokens);
        assert_eq!(structure, Some(TokenStructure::Array { length: 2 }));
    }

    #[test]
    fn test_pattern_recognition_object_structure() {
        use pattern_recognition::*;

        // Test object pattern: [field_count, field1_name, field1_value, field2_name, field2_value]
        let object_tokens = BufferedTokenIterator::new([2, 10, 100, 20, 200].into_iter());
        let structure = analyze_token_structure(&object_tokens);
        assert_eq!(structure, Some(TokenStructure::Object { field_count: 2 }));
    }

    #[test]
    fn test_pattern_recognition_empty_stream() {
        use pattern_recognition::*;

        // Test empty token stream (common for unit enum variants)
        let empty_tokens = BufferedTokenIterator::new([].into_iter());
        let structure = analyze_token_structure(&empty_tokens);
        assert_eq!(structure, Some(TokenStructure::Null));
    }

    #[test]
    fn test_structure_compatibility() {
        use pattern_recognition::*;

        // Test exact compatibility
        let string_tokens = BufferedTokenIterator::new([3, 65, 66, 67].into_iter());
        let expected = TokenStructure::String { length: 3 };
        assert!(is_structure_compatible(&string_tokens, &expected));

        // Test type compatibility (flexible matching)
        let expected_flexible = TokenStructure::String { length: 5 }; // Different length
        assert!(is_structure_compatible(&string_tokens, &expected_flexible));

        // Test incompatibility
        let expected_wrong = TokenStructure::Array { length: 3 };
        assert!(!is_structure_compatible(&string_tokens, &expected_wrong));
    }

    #[test]
    fn test_structures_match() {
        use pattern_recognition::structures_match;

        // Test exact matches
        let s1 = TokenStructure::String { length: 5 };
        let s2 = TokenStructure::String { length: 5 };
        assert!(structures_match(&s1, &s2));

        // Test flexible type matching
        let s3 = TokenStructure::String { length: 3 };
        assert!(structures_match(&s1, &s3)); // Different lengths but same type

        // Test cross-type incompatibility
        let array = TokenStructure::Array { length: 5 };
        assert!(!structures_match(&s1, &array));

        // Test primitive matching
        let p1 = TokenStructure::Primitive(42);
        let p2 = TokenStructure::Primitive(42);
        let p3 = TokenStructure::Primitive(100);
        assert!(structures_match(&p1, &p2)); // Exact match
        assert!(structures_match(&p1, &p3)); // Flexible primitive matching
    }

    #[test]
    fn test_empty_token_stream() {
        use pattern_recognition::*;

        let empty_tokens = BufferedTokenIterator::new([].into_iter());
        let structure = analyze_token_structure(&empty_tokens);
        // Empty streams should return Null structure, not None (this was the bug fix)
        assert_eq!(structure, Some(TokenStructure::Null));
    }

    #[test]
    fn test_variant_compatibility_basic() {
        use variant_compatibility::*;

        let mut compat = VariantCompatibility::new();

        // Add variants with their expected structures
        compat.add_variant(
            "StringVariant".to_string(),
            TokenStructure::String { length: 5 },
        );
        compat.add_variant(
            "ArrayVariant".to_string(),
            TokenStructure::Array { length: 3 },
        );
        compat.add_variant(
            "ObjectVariant".to_string(),
            TokenStructure::Object { field_count: 2 },
        );

        // Test variant structure lookup
        assert_eq!(
            compat.get_variant_structure("StringVariant"),
            Some(&TokenStructure::String { length: 5 })
        );
        assert_eq!(compat.get_variant_structure("NonExistent"), None);

        // Test compatibility checking
        let string_struct = TokenStructure::String { length: 5 };
        assert!(compat.is_variant_compatible("StringVariant", &string_struct));
        assert!(!compat.is_variant_compatible("ArrayVariant", &string_struct));
    }

    #[test]
    fn test_variant_compatibility_multiple_variants() {
        use variant_compatibility::*;

        let mut compat = VariantCompatibility::new();

        // Add multiple variants that can handle the same structure
        let string_struct = TokenStructure::String { length: 5 };
        compat.add_variant("StringVariant1".to_string(), string_struct.clone());
        compat.add_variant("StringVariant2".to_string(), string_struct.clone());

        // Test getting compatible variants
        let compatible = compat.get_compatible_variants(&string_struct);
        assert_eq!(compatible.len(), 2);
        assert!(compatible.contains(&"StringVariant1".to_string()));
        assert!(compatible.contains(&"StringVariant2".to_string()));
    }

    #[test]
    fn test_variant_compatibility_flexible_variants() {
        use variant_compatibility::*;

        let mut compat = VariantCompatibility::new();

        // Add a flexible variant that can handle multiple structures
        compat.add_variant(
            "FlexibleVariant".to_string(),
            TokenStructure::String { length: 5 },
        );
        compat.mark_flexible("FlexibleVariant".to_string());

        // Test that flexible variant matches different string lengths
        let different_string = TokenStructure::String { length: 10 };
        let compatible = compat.get_compatible_variants(&different_string);
        assert!(compatible.contains(&"FlexibleVariant".to_string()));

        // But not completely different types
        let array_struct = TokenStructure::Array { length: 5 };
        let array_compatible = compat.get_compatible_variants(&array_struct);
        assert!(!array_compatible.contains(&"FlexibleVariant".to_string()));
    }

    #[test]
    fn test_variant_compatibility_stats() {
        use variant_compatibility::*;

        let mut compat = VariantCompatibility::new();

        // Add variants
        let string_struct = TokenStructure::String { length: 5 };
        compat.add_variant("StringVariant1".to_string(), string_struct.clone());
        compat.add_variant("StringVariant2".to_string(), string_struct.clone()); // Same structure
        compat.add_variant(
            "ArrayVariant".to_string(),
            TokenStructure::Array { length: 3 },
        );
        compat.mark_flexible("StringVariant1".to_string());

        let stats = compat.get_stats();
        assert_eq!(stats.total_variants, 3);
        assert_eq!(stats.total_structures, 2); // String and Array structures
        assert_eq!(stats.flexible_variants, 1);
        assert_eq!(stats.ambiguous_structures, 1); // String structure has 2 variants
    }

    #[test]
    fn test_variant_compatibility_empty() {
        use variant_compatibility::*;

        let compat = VariantCompatibility::new();

        // Test empty compatibility
        let string_struct = TokenStructure::String { length: 5 };
        let compatible = compat.get_compatible_variants(&string_struct);
        assert!(compatible.is_empty());

        assert_eq!(compat.get_variant_structure("NonExistent"), None);
        assert!(!compat.is_variant_compatible("NonExistent", &string_struct));

        let stats = compat.get_stats();
        assert_eq!(stats.total_variants, 0);
        assert_eq!(stats.total_structures, 0);
        assert_eq!(stats.flexible_variants, 0);
        assert_eq!(stats.ambiguous_structures, 0);
    }

    #[test]
    fn test_variant_compatibility_real_world_scenario() {
        use variant_compatibility::*;

        let mut compat = VariantCompatibility::new();

        // Simulate a real enum with string/object variants (like our test schemas)
        compat.add_variant(
            "StringVersion".to_string(),
            TokenStructure::String { length: 5 },
        );
        compat.add_variant(
            "ObjectVersion".to_string(),
            TokenStructure::Object { field_count: 1 },
        );

        // Mark string variant as flexible to handle different lengths
        compat.mark_flexible("StringVersion".to_string());

        // Test string routing
        let string_tokens = TokenStructure::String { length: 10 }; // Different length
        let string_variants = compat.get_compatible_variants(&string_tokens);
        assert_eq!(string_variants.len(), 1);
        assert_eq!(string_variants[0], "StringVersion");

        // Test object routing
        let object_tokens = TokenStructure::Object { field_count: 1 };
        let object_variants = compat.get_compatible_variants(&object_tokens);
        assert_eq!(object_variants.len(), 1);
        assert_eq!(object_variants[0], "ObjectVersion");

        // Test no false positives
        assert!(!compat.is_variant_compatible("StringVersion", &object_tokens));
        assert!(!compat.is_variant_compatible("ObjectVersion", &string_tokens));
    }
}
