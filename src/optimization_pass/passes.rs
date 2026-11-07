//! Legacy built-in optimization pass implementations.
//!
//! These pass structs remain available for compatibility with older
//! `tct::optimization_pass` callers. New optimizer behavior should be modeled
//! as structured metadata or `EncodingStrategy` decisions in the maintained
//! four-pass compiler pipeline, not as custom generated `TokenStream`
//! fragments returned from this legacy trait surface.

#![allow(deprecated)]

use super::core::{IntegerConstraints, OptimizationContext, OptimizationPass};
use proc_macro2::TokenStream;
use quote::format_ident;

/// Optimization pass that converts enum values to indices
#[derive(Debug)]
pub struct EnumIndexOptimizationPass {
    enabled: bool,
}

impl Default for EnumIndexOptimizationPass {
    fn default() -> Self {
        Self::new()
    }
}

impl EnumIndexOptimizationPass {
    /// Create a new enum index optimization pass
    pub fn new() -> Self {
        Self { enabled: true }
    }
}

impl OptimizationPass for EnumIndexOptimizationPass {
    fn name(&self) -> &'static str {
        "enum_index_optimization"
    }

    fn description(&self) -> &'static str {
        "Converts enum string values to compact index encoding (0, 1, 2, ...)"
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn optimize_enum_encoding(
        &self,
        _enum_name: &syn::Ident,
        enum_values: &[String],
        context: &OptimizationContext,
    ) -> Option<TokenStream> {
        if !self.enabled {
            return None;
        }

        // Generate index-based encoding (current behavior)
        let encode_arms = enum_values.iter().enumerate().map(|(i, value)| {
            let variant_name = format_ident!("{}", crate::utils::to_upper_camel_case(value));
            let index_literal = proc_macro2::Literal::u32_suffixed(i as u32);
            quote::quote! { Self::#variant_name => #index_literal }
        });

        let decode_arms = enum_values.iter().enumerate().map(|(i, value)| {
            let variant_name = format_ident!("{}", crate::utils::to_upper_camel_case(value));
            let index_literal = proc_macro2::Literal::u32_suffixed(i as u32);
            quote::quote! { #index_literal => Ok(Self::#variant_name) }
        });

        let enum_len = enum_values.len() as u32;

        // Use vocabulary constraint for modulo if available, otherwise use enum length
        let modulo_base = if let Some(max_vocab) = context.max_vocabulary_size {
            max_vocab as u32
        } else {
            enum_len
        };

        Some(quote::quote! {
            fn encode(&self, tokens: &mut Vec<u32>) {
                let token = match self {
                    #( #encode_arms ),*
                };
                tokens.push(token);
            }

            fn decode(tokens: &mut impl Iterator<Item = u32>) -> Result<Self, crate::tokenizer::DecodeError> {
                let token = tokens.next().ok_or(crate::tokenizer::DecodeError::UnexpectedEndOfStream)?;
                let normalized_token = token % #modulo_base;
                let enum_index = normalized_token % #enum_len;
                match enum_index {
                    #( #decode_arms ),*,
                    _ => unreachable!("Modulo operation should ensure valid token range")
                }
            }

            fn token_count(&self) -> usize {
                1
            }
        })
    }
}

/// Optimization pass for bounded integer encoding
#[derive(Debug)]
pub struct BoundedIntegerOptimizationPass {
    enabled: bool,
}

impl Default for BoundedIntegerOptimizationPass {
    fn default() -> Self {
        Self::new()
    }
}

impl BoundedIntegerOptimizationPass {
    /// Create a new bounded integer optimization pass
    pub fn new() -> Self {
        Self { enabled: true }
    }
}

impl OptimizationPass for BoundedIntegerOptimizationPass {
    fn name(&self) -> &'static str {
        "bounded_integer_optimization"
    }

    fn description(&self) -> &'static str {
        "Optimizes integers with min/max constraints to use smaller token ranges"
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn optimize_integer_encoding(
        &self,
        constraints: &IntegerConstraints,
        context: &OptimizationContext,
    ) -> Option<TokenStream> {
        if !self.enabled {
            return None;
        }

        // Check if we have bounded integer constraints
        if let (Some(min), Some(max)) = (constraints.minimum, constraints.maximum) {
            let range_size = max.checked_sub(min).and_then(|range| range.checked_add(1));
            if let Some(range_size) = range_size {
                if range_size <= 0 || range_size > u32::MAX as i64 {
                    return None;
                }
                let range_size = range_size as u32;

                // Use vocabulary constraint for modulo if available, otherwise use range size
                let modulo_base = if let Some(max_vocab) = context.max_vocabulary_size {
                    max_vocab as u32
                } else {
                    range_size
                };

                // Generate bounded integer encoding
                return Some(quote::quote! {
                    fn encode(&self, tokens: &mut Vec<u32>) {
                        let offset_value = *self - #min;
                        tokens.push(offset_value as u32);
                    }

                    fn decode(tokens: &mut impl Iterator<Item = u32>) -> Result<Self, crate::tokenizer::DecodeError> {
                        let token = tokens.next().ok_or(crate::tokenizer::DecodeError::UnexpectedEndOfStream)?;
                        let normalized_token = token % #modulo_base;
                        let range_index = normalized_token % #range_size;
                        Ok(range_index as i64 + #min)
                    }

                    fn token_count(&self) -> usize {
                        1
                    }
                });
            }
        }

        None
    }
}

/// Reserved placeholder for future string-pattern optimization.
///
/// Direct finite-regex encoding is not currently wired into the generated
/// tokenizer optimization pipeline. `patternProperties` support is handled by
/// schema discovery and generation, and vocabulary-analysis pattern extraction
/// lives outside this pass. The public CLI/API rejects `pattern_optimization`
/// until this pass has a real generated-code contract.
#[derive(Debug)]
pub struct PatternOptimizationPass;

impl Default for PatternOptimizationPass {
    fn default() -> Self {
        Self::new()
    }
}

impl PatternOptimizationPass {
    /// Create a new pattern optimization pass
    pub fn new() -> Self {
        Self
    }
}

impl OptimizationPass for PatternOptimizationPass {
    fn name(&self) -> &'static str {
        "pattern_optimization"
    }

    fn description(&self) -> &'static str {
        "Reserved placeholder for future finite-regex string encoding"
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn optimize_enum_encoding(
        &self,
        _enum_name: &syn::Ident,
        _enum_values: &[String],
        _context: &OptimizationContext,
    ) -> Option<proc_macro2::TokenStream> {
        // Pattern optimization is currently handled through the vocabulary optimization system
        // rather than through the OptimizationPass trait system. See class documentation above.
        //
        // This method returns None to indicate no enum-specific optimization is applied,
        // allowing the default enum encoding to proceed while pattern strings are optimized
        // separately through the vocabulary system.
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::optimization_pass::core::{AnalysisResult, OptimizationLevel};
    use rustc_hash::FxHashMap;

    fn context(max_vocabulary_size: Option<usize>) -> OptimizationContext {
        let mut analysis_results = FxHashMap::default();
        analysis_results.insert(
            "state".to_string(),
            AnalysisResult::VocabularyDomain { cardinality: 3 },
        );

        OptimizationContext {
            schema: serde_json::json!({"type": "object"}),
            optimization_level: OptimizationLevel::O2,
            max_vocabulary_size,
            analysis_results,
        }
    }

    fn compact(tokens: TokenStream) -> String {
        tokens.to_string().split_whitespace().collect::<String>()
    }

    #[test]
    fn enum_index_pass_generates_index_codec_with_default_and_vocab_modulo() {
        let pass = EnumIndexOptimizationPass::new();
        let enum_name = syn::parse_str::<syn::Ident>("AccessLevel").unwrap();
        let enum_values = vec![
            "read_only".to_string(),
            "write".to_string(),
            "admin".to_string(),
        ];

        assert_eq!(pass.name(), "enum_index_optimization");
        assert!(pass.description().contains("compact index encoding"));
        assert!(pass.as_any().is::<EnumIndexOptimizationPass>());

        let default_code = compact(
            pass.optimize_enum_encoding(&enum_name, &enum_values, &context(None))
                .expect("enum optimization should generate code"),
        );
        assert!(default_code.contains("Self::ReadOnly=>0u32"));
        assert!(default_code.contains("Self::Write=>1u32"));
        assert!(default_code.contains("Self::Admin=>2u32"));
        assert!(default_code.contains("token%3u32"));
        assert!(default_code.contains("normalized_token%3u32"));

        let vocab_code = compact(
            pass.optimize_enum_encoding(&enum_name, &enum_values, &context(Some(512)))
                .expect("enum optimization should generate code with vocab modulo"),
        );
        assert!(vocab_code.contains("token%512u32"));
        assert!(vocab_code.contains("normalized_token%3u32"));
    }

    #[test]
    fn enum_index_pass_can_be_disabled() {
        let pass = EnumIndexOptimizationPass { enabled: false };
        let enum_name = syn::parse_str::<syn::Ident>("Status").unwrap();
        let enum_values = vec!["ready".to_string()];

        assert!(
            pass.optimize_enum_encoding(&enum_name, &enum_values, &context(None))
                .is_none()
        );
    }

    #[test]
    fn bounded_integer_pass_uses_inclusive_range_and_vocab_modulo() {
        let pass = BoundedIntegerOptimizationPass::new();
        let constraints = IntegerConstraints {
            minimum: Some(10),
            maximum: Some(13),
            exclusive_minimum: None,
            exclusive_maximum: None,
        };

        assert_eq!(pass.name(), "bounded_integer_optimization");
        assert!(pass.description().contains("min/max constraints"));
        assert!(pass.as_any().is::<BoundedIntegerOptimizationPass>());

        let default_code = compact(
            pass.optimize_integer_encoding(&constraints, &context(None))
                .expect("bounded integer optimization should generate code"),
        );
        assert!(default_code.contains("*self-10i64"));
        assert!(default_code.contains("token%4u32"));
        assert!(default_code.contains("normalized_token%4u32"));
        assert!(default_code.contains("range_indexasi64+10i64"));

        let vocab_code = compact(
            pass.optimize_integer_encoding(&constraints, &context(Some(512)))
                .expect("bounded integer optimization should honor vocab modulo"),
        );
        assert!(vocab_code.contains("token%512u32"));
        assert!(vocab_code.contains("normalized_token%4u32"));
    }

    #[test]
    fn bounded_integer_pass_handles_single_value_range() {
        let pass = BoundedIntegerOptimizationPass::new();
        let constraints = IntegerConstraints {
            minimum: Some(42),
            maximum: Some(42),
            exclusive_minimum: None,
            exclusive_maximum: None,
        };

        let code = compact(
            pass.optimize_integer_encoding(&constraints, &context(None))
                .expect("single-value inclusive range should still be encodable"),
        );
        assert!(code.contains("token%1u32"));
        assert!(code.contains("normalized_token%1u32"));
        assert!(code.contains("+42i64"));
    }

    #[test]
    fn bounded_integer_pass_rejects_missing_invalid_and_overwide_ranges() {
        let pass = BoundedIntegerOptimizationPass::new();
        let invalid_cases = [
            IntegerConstraints {
                minimum: None,
                maximum: Some(3),
                exclusive_minimum: None,
                exclusive_maximum: None,
            },
            IntegerConstraints {
                minimum: Some(3),
                maximum: None,
                exclusive_minimum: None,
                exclusive_maximum: None,
            },
            IntegerConstraints {
                minimum: Some(10),
                maximum: Some(9),
                exclusive_minimum: None,
                exclusive_maximum: None,
            },
            IntegerConstraints {
                minimum: Some(i64::MIN),
                maximum: Some(i64::MAX),
                exclusive_minimum: None,
                exclusive_maximum: None,
            },
            IntegerConstraints {
                minimum: Some(0),
                maximum: Some(u32::MAX as i64),
                exclusive_minimum: None,
                exclusive_maximum: None,
            },
        ];

        for constraints in invalid_cases {
            assert!(
                pass.optimize_integer_encoding(&constraints, &context(None))
                    .is_none()
            );
        }
    }

    #[test]
    fn bounded_integer_pass_can_be_disabled() {
        let pass = BoundedIntegerOptimizationPass { enabled: false };
        let constraints = IntegerConstraints {
            minimum: Some(0),
            maximum: Some(1),
            exclusive_minimum: None,
            exclusive_maximum: None,
        };

        assert!(
            pass.optimize_integer_encoding(&constraints, &context(None))
                .is_none()
        );
    }

    #[test]
    fn pattern_pass_is_a_reserved_placeholder() {
        let pass = PatternOptimizationPass::new();
        let enum_name = syn::parse_str::<syn::Ident>("PatternCarrier").unwrap();
        let enum_values = vec!["value".to_string()];

        assert_eq!(pass.name(), "pattern_optimization");
        assert!(pass.description().contains("Reserved placeholder"));
        assert!(pass.as_any().is::<PatternOptimizationPass>());
        assert!(
            pass.optimize_enum_encoding(&enum_name, &enum_values, &context(None))
                .is_none()
        );
    }
}
